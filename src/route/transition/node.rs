use crate::route::graph::NodeIx;

// TODO: Refactor this.
// Transition probability is kept
// on the node's edge toward the next/prev
// in the graph from [`crate::transition`].
#[derive(Clone, Copy, Debug)]
pub struct TransitionCandidate {
    pub map_edge: (NodeIx, NodeIx),
    pub position: geo::Point,
    pub emission: f64,

    pub layer_id: usize,
    pub node_id: usize,
}
