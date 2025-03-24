use crate::route::graph::NodeIx;
use crate::route::transition::candidate::CandidateId;
use geo::Point;

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
    pub emission: f64,

    pub layer_id: usize,
    pub node_id: usize,
}

/// Represents the edge of this candidate within
/// the [`Candidate`] graph.
///
/// TODO: Complete
#[derive(Default, Clone)]
pub struct CandidateEdge {
    pub weight: f64,
    pub nodes: Vec<NodeIx>,
}

impl CandidateEdge {
    pub fn zero() -> Self {
        Self::default()
    }
}
