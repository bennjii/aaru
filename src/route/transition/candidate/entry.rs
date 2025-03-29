use crate::route::graph::NodeIx;
use geo::Point;
use pathfinding::num_traits::Zero;
use std::cmp::Ordering;
use std::ops::Add;
use std::sync::Arc;

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
    pub map_edge: (NodeIx, NodeIx),
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
    pub nodes: *const [NodeIx],
}

impl CandidateEdge {
    pub fn nodes(&self) -> &[NodeIx] {
        unsafe { &*self.nodes }
    }
}

unsafe impl Send for CandidateEdge {}
unsafe impl Sync for CandidateEdge {}

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
        let vec = self
            .nodes()
            .to_vec()
            .into_iter()
            .chain(rhs.nodes().to_vec())
            .collect::<Vec<_>>();

        let data = Arc::from(vec);

        CandidateEdge {
            nodes: &*data as *const [NodeIx],
            weight: self.weight + rhs.weight,
        }
    }
}

impl CandidateEdge {
    pub fn new(weight: u32, nodes: &[NodeIx]) -> Self {
        let data: Arc<[NodeIx]> = Arc::from(nodes);
        Self {
            weight,
            nodes: &*data as *const [NodeIx],
        }
    }

    pub fn zero() -> Self {
        CandidateEdge {
            weight: 0,
            nodes: &[],
        }
    }
}
