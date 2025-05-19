use crate::route::Graph;
use crate::route::graph::{EdgeIx, NodeIx, Weight};
use crate::route::transition::RoutingContext;

use codec::osm::element::variants::Node;
use geo::{Distance, Haversine, LineLocatePoint, LineString, Point};
use pathfinding::num_traits::Zero;
use petgraph::Direction;
use rstar::AABB;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Add;

/// Represents an edge within the system, along with the directionality of the edge.
///
/// Since the transition graph is a directed graph, it does not support bidirectional edges.
/// Meaning, any edge which is bidirectional must therefore be converted into two edges, each
/// with a different direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionAwareEdgeId {
    id: EdgeIx,
    direction: Direction,
}

impl DirectionAwareEdgeId {
    pub fn new(id: EdgeIx) -> Self {
        Self {
            id,
            direction: Direction::Outgoing,
        }
    }

    /// The [`EdgeIx`] of the direction-aware edge.
    pub fn index(&self) -> EdgeIx {
        self.id
    }

    /// If the direction-aware edge is forward-facing.
    pub fn forward(self) -> Self {
        DirectionAwareEdgeId {
            direction: Direction::Outgoing,
            ..self
        }
    }

    /// If the direction-aware edge is rear/backward-facing.
    pub fn backward(self) -> Self {
        DirectionAwareEdgeId {
            direction: Direction::Incoming,
            ..self
        }
    }
}

impl Ord for DirectionAwareEdgeId {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.id.cmp(&other.id) {
            Ordering::Equal => self.direction.cmp(&other.direction),
            ord => ord,
        }
    }
}

impl PartialOrd for DirectionAwareEdgeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A [flyweight] representation of an edge within the system.
///
/// The non-backed alternative is the [`FatEdge`] which contains node information directly,
/// instead of by indices.
///
/// Every edge has a [source](#field.source) and [target](#field.target).
/// They also contain a [weight](#field.weight) to represent how "heavy" the edge is to traverse,
/// and an identifier, [id](#field.id) which is direction-aware.
///
/// [flyweight]: https://refactoring.guru/design-patterns/flyweight
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub source: NodeIx,
    pub target: NodeIx,

    pub weight: Weight,
    pub id: DirectionAwareEdgeId,
}

impl<'a> From<(NodeIx, NodeIx, &'a (Weight, DirectionAwareEdgeId))> for Edge {
    #[inline]
    fn from((source, target, edge): (NodeIx, NodeIx, &'a (Weight, DirectionAwareEdgeId))) -> Self {
        Edge {
            source,
            target,
            weight: edge.0,
            id: edge.1,
        }
    }
}

/// Represents a fat edge within the system.
///
/// A [`FatEdge`], unlike an [`Edge`] contains source/target information inside the structure
/// itself, instead of through [`NodeIx`] indirection. This makes the structure "fat" since
/// the [`Node`] struct is large.
///
/// A helper method, [`FatEdge::thin`] is provided to downsize to an [`Edge`]. Note this process
/// is lossy if no data source containing the original node is present.
///
/// ### Note
///
/// As it is large, this should only be used transitively
/// like in [`Scan::nearest_edges`](crate::route::Scan::nearest_edges).
pub struct FatEdge {
    pub source: Node,
    pub target: Node,

    pub weight: Weight,
    pub id: DirectionAwareEdgeId,
}

impl FatEdge {
    /// Downsizes a [`FatEdge`] to an [`Edge`].
    #[inline]
    pub fn thin(&self) -> Edge {
        Edge {
            source: self.source.id,
            target: self.target.id,
            id: self.id,
            weight: self.weight,
        }
    }
}

impl rstar::RTreeObject for FatEdge {
    type Envelope = AABB<Point>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(self.target.position, self.source.position)
    }
}

/// The location of a candidate within a solution.
/// This identifies which layer the candidate came from, and which node in the layer it was.
///
/// This is useful for debugging purposes to understand a node without requiring further context.
#[derive(Clone, Copy, Debug)]
pub struct CandidateLocation {
    pub layer_id: usize,
    pub node_id: usize,
}

/// Represents the candidate selected within a layer.
///
/// This value holds the [edge](#field.edge) on the underlying routing structure it is sourced
/// from, along with the candidate position, [position](#field.position).
///
/// It further contains the emission cost [emission](#field.emission) associated with choosing this
/// candidate and the candidate's location within the solution, [location](#field.location).
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    /// Refers to the points within the map graph (Underlying routing structure)
    pub edge: Edge,
    pub position: Point,
    pub emission: u32,

    pub location: CandidateLocation,
}

/// A virtual tail is a representation of the distance from some intermediary point
/// on a candidate edge to the edge's end. This is used to resolve routing decisions
/// within short distances, in which case we need to understand the distance between
/// our intermediary projected position and some end of the edge.
///
/// If the candidates were on the same edge, we would instead utilise the
/// [ResolutionMethod] option.
///
/// The below diagram images the virtual tail for intermediate candidate position.
/// For example, the [`VirtualTail::ToSource`] variant can be seen to measure the
/// distance from this intermediate, to the source of the edge, and vice versa for
/// the target.
///
///                 Candidate
///          ToSource   |   ToTarget
///        +------------|------------+
///      Source                    Target
pub enum VirtualTail {
    /// The distance from the edge's source to the virtual candidate position.
    ToSource,

    /// The distance from the virtual candidate position to the edge target.
    ToTarget,
}

impl Candidate {
    /// Returns the percentage of the distance through the edge, relative to the position
    /// upon the linestring by which it lies.
    ///
    /// The below diagram visualises this percentage. Note that `0%` represents
    /// an intermediate which is equivalent to the source of the edge, whilst `100%`
    /// represents an intermediate equivalent to the target.
    ///
    ///                Edge Percentages
    ///     Source                         Target
    ///       +---------|----------------|---+
    ///                0.4              0.9
    ///               (40%)            (90%)
    ///
    pub fn percentage(&self, graph: &Graph) -> Option<f64> {
        let edge = graph
            .resolve_line(&[self.edge.source, self.edge.target])
            .into_iter()
            .collect::<LineString>();

        edge.line_locate_point(&self.position)
    }

    /// Calculates the offset, in meters, of the candidate to it's edge by the [`VirtualTail`].
    pub fn offset(&self, ctx: &RoutingContext, variant: VirtualTail) -> Option<f64> {
        match variant {
            VirtualTail::ToSource => {
                let source = ctx.map.get_position(&self.edge.source)?;
                Some(Haversine.distance(source, self.position))
            }
            VirtualTail::ToTarget => {
                let target = ctx.map.get_position(&self.edge.target)?;
                Some(Haversine.distance(self.position, target))
            }
        }
    }

    pub fn new(edge: Edge, position: Point, emission: u32, location: CandidateLocation) -> Self {
        Self {
            edge,
            position,
            emission,
            location,
        }
    }
}

/// Represents the edge of this candidate within the candidate graph.
///
/// This is distinct from [`Edge`] since it exists within the candidate graph
/// of the [`Transition`](crate::route::graph::Transition), not of [`Graph`].
///
/// This edge stores the weight associated with traversing this edge.
///
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct CandidateEdge {
    pub weight: u32,
}

impl Eq for CandidateEdge {}

impl PartialEq<Self> for CandidateEdge {
    fn eq(&self, other: &Self) -> bool {
        self.weight.eq(&other.weight)
    }
}

impl PartialOrd<Self> for CandidateEdge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CandidateEdge {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}

impl Zero for CandidateEdge {
    fn zero() -> Self {
        CandidateEdge::default()
    }

    fn is_zero(&self) -> bool {
        self.weight.is_zero()
    }
}

impl Add<Self> for CandidateEdge {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        CandidateEdge {
            weight: self.weight.saturating_add(rhs.weight),
        }
    }
}

impl CandidateEdge {
    #[inline]
    pub fn new(weight: u32) -> Self {
        Self { weight }
    }
}
