use geo::Point;
use std::cell::RefCell;

use crate::codec::element::variants::Node;
use crate::route::graph::{Edge, NodeIx};
use crate::route::transition::segment::TrajectorySegment;

pub type ImbuedLayer<'t> = Vec<RefCell<TransitionNode<'t>>>;

#[derive(Clone)]
pub struct TransitionNode<'a> {
    pub candidate: &'a TransitionCandidate<'a>,
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
    pub candidates: Vec<TransitionCandidate<'a>>,
    pub segment: TrajectorySegment<'a>,
}

pub struct TransitionCandidate<'a> {
    pub index: NodeIx,
    pub edge: Edge<'a>,
    pub position: Point,
}
