use geo::Point;
use std::cell::RefCell;

use crate::codec::element::variants::Node;
use crate::route::graph::NodeIx;
use crate::route::transition::segment::TrajectorySegment;

pub type ImbuedLayer<'t> = Vec<RefCell<TransitionNode<'t>>>;

#[derive(Clone)]
pub struct TransitionNode<'a> {
    pub candidate: &'a TransitionCandidate,
    pub prev_best: Option<&'a RefCell<TransitionNode<'a>>>,
    pub current_path: Box<Vec<Node>>,
    pub emission_probability: f64,
    pub transition_probability: f64,
    pub cumulative_probability: f64,
}

pub struct RefinedTransitionLayer<'a> {
    pub nodes: Vec<TransitionNode<'a>>,
    pub segment: &'a TrajectorySegment<'a>,
}

pub struct TransitionLayer<'a> {
    pub candidates: Vec<TransitionCandidate>,
    pub segment: TrajectorySegment<'a>,
}

// Transition probability is kept
// on the node's edge toward the next/prev
// in the graph from [`crate::transition`].
#[derive(Clone, Copy, Debug)]
pub struct TransitionCandidate {
    pub map_edge: (NodeIx, NodeIx),
    pub position: Point,

    pub layer_id: usize,
    pub node_id: usize,
}
