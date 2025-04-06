use crate::route::graph::{EdgeIx, NodeIx, Weight};
use crate::route::Graph;
use geo::{Distance, Haversine, Point};
use pathfinding::num_traits::Zero;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Add;

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub source: NodeIx,
    pub target: NodeIx,

    pub weight: Weight,
    pub id: EdgeIx,
}

impl Edge {
    pub fn new(source: NodeIx, target: NodeIx, weight: Weight, id: EdgeIx) -> Self {
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

        Some(Haversine::distance(source_position, target_position))
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

    #[cfg(debug_assertions)]
    pub location: CandidateLocation,
}

impl Candidate {
    pub fn new(
        edge: Edge,
        position: Point,
        emission: u32,
        #[cfg(debug_assertions)] location: CandidateLocation,
    ) -> Self {
        Self {
            edge,
            position,
            emission,
            #[cfg(debug_assertions)]
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
