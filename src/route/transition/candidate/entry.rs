use crate::route::graph::{EdgeIx, NodeIx, Weight};
use crate::route::transition::RoutingContext;
use crate::route::Graph;

use crate::codec::element::variants::Node;
use geo::{Distance, Haversine, LineLocatePoint, LineString, Point};
use pathfinding::num_traits::Zero;
use petgraph::Direction;
use rstar::AABB;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Add;

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

    pub fn index(&self) -> EdgeIx {
        self.id
    }

    pub fn forward(self) -> Self {
        DirectionAwareEdgeId {
            direction: Direction::Outgoing,
            ..self
        }
    }

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
        Edge::new(source, target, edge.0, edge.1)
    }
}

pub struct FatEdge {
    pub source: Node,
    pub target: Node,

    pub weight: Weight,
    pub id: DirectionAwareEdgeId,
}

impl FatEdge {
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

impl Edge {
    pub fn new(source: NodeIx, target: NodeIx, weight: Weight, id: DirectionAwareEdgeId) -> Self {
        Self {
            source,
            target,
            weight,
            id,
        }
    }

    pub fn length(self, graph: &Graph) -> Option<f64> {
        let Edge { source, target, .. } = self;

        let source_position = graph.get_position(&source)?;
        let target_position = graph.get_position(&target)?;

        Some(Haversine.distance(source_position, target_position))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CandidateLocation {
    pub layer_id: usize,
    pub node_id: usize,
}

#[derive(Clone, Copy, Debug)]
/// Represents the candidate selected within a layer.
///
/// This value holds a reference to the edge on the
/// underlying routing structure it is sourced from,
/// along with it's true position, emission cost,
/// and the layer and node ids associated with its selection.
///
/// TODO: Complete
pub struct Candidate {
    /// Refers to the points within the map graph (Underlying routing structure)
    pub edge: Edge,
    pub position: Point,
    pub emission: u32,

    pub location: CandidateLocation,
}

/// Calculates offset distances for the virtualized candidate position.
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
    /// TODO: Docs
    ///
    /// Returns the percentage of the distance through the edge,
    /// relative to the position upon the linestring by which it lies,
    /// considering the line to start at the Source and end at the Target node.
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

/// Represents the edge of this candidate within
/// the [`Candidate`] graph.
///
/// TODO: Complete
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
