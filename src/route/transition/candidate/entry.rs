use crate::route::graph::NodeIx;
use crate::route::Graph;
use geo::{Distance, Haversine, Point};
use pathfinding::num_traits::Zero;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::ops::Add;

#[derive(Clone, Copy, Debug)]
pub struct MapEdge {
    pub start: NodeIx,
    pub end: NodeIx,
}

impl MapEdge {
    pub fn new(start: NodeIx, end: NodeIx) -> Self {
        Self { start, end }
    }

    pub fn length(self, graph: &Graph) -> Option<f64> {
        let MapEdge { start, end } = self;

        let start_position = graph.get_position(&start)?;
        let end_position = graph.get_position(&end)?;

        Some(Haversine::distance(start_position, end_position))
    }
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
    pub map_edge: MapEdge,
    pub position: Point,
    pub emission: u32,

    pub layer_id: usize,
    pub node_id: usize,
}

/// Represents the edge of this candidate within
/// the [`Candidate`] graph.
///
/// TODO: Complete
#[derive(Clone, Copy)]
pub struct CandidateEdge {
    pub weight: u32,

    // TODO: Document this, meaning forgotten.
    pub flyweight: (usize, usize),
}

impl Debug for CandidateEdge {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CandidateEdge {{ weight: {}, nodes: [...] }}",
            self.weight
        )
    }
}

impl Eq for CandidateEdge {}

impl PartialEq<Self> for CandidateEdge {
    fn eq(&self, other: &Self) -> bool {
        self.weight.eq(&other.weight)
    }
}

impl PartialOrd<Self> for CandidateEdge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.weight.partial_cmp(&other.weight)
    }
}

impl Ord for CandidateEdge {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}

impl Zero for CandidateEdge {
    fn zero() -> Self {
        CandidateEdge::zero()
    }

    fn is_zero(&self) -> bool {
        self.weight.is_zero()
    }
}

impl Add<Self> for CandidateEdge {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        CandidateEdge {
            flyweight: rhs.flyweight,
            weight: self.weight + rhs.weight,
        }
    }
}

impl CandidateEdge {
    pub fn new(weight: u32, flyweight: (usize, usize)) -> Self {
        Self { weight, flyweight }
    }

    pub fn zero() -> Self {
        CandidateEdge {
            weight: 0,
            flyweight: (0, 0),
        }
    }
}
