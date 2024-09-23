use std::cell::RefCell;
use std::f64::consts::E;
use std::ops::{Div, Mul};

use geo::{EuclideanDistance, LineString, Point};
use log::debug;
use wkt::ToWkt;

use crate::route::graph::{Edge, NodeIx};
use crate::route::Graph;

const DEFAULT_ERROR: f64 = 20f64;

pub struct Transition<'a> {
    // The original linestring
    linestring: LineString,
    graph: &'a Graph,
    error: Option<f64>,
}

#[derive(Clone, Copy)]
struct TransitionNode<'a> {
    candidate: &'a TransitionCandidate<'a>,
    prev_best: Option<&'a RefCell<TransitionNode<'a>>>,
    emission_probability: f64,
    transition_probability: f64,
    cumulative_probability: f64,
}

struct RefinedTransitionLayer<'a> {
    nodes: Vec<TransitionNode<'a>>,
    segment: &'a TrajectorySegment<'a>,
}

struct TransitionLayer<'a> {
    candidates: Vec<TransitionCandidate<'a>>,
    segment: &'a TrajectorySegment<'a>,
}

struct TransitionCandidate<'a> {
    index: NodeIx,
    edge: Edge<'a>,
    position: Point,
}

struct TrajectorySegment<'a> {
    source: &'a Point,
    target: &'a Point,

    // The Euclidean length of the segment
    // ahead of the current node.
    //
    //  This + ----------‐ + Next
    //           ^ Euclidean length of line
    //
    length: f64,
}

type ImbuedLayer<'t> = (&'t TransitionLayer<'t>, Vec<RefCell<TransitionNode<'t>>>);

impl<'a> TrajectorySegment<'a> {
    pub fn new(a: &'a Point, b: &'a Point) -> Self {
        TrajectorySegment {
            source: a,
            target: b,
            length: a.euclidean_distance(b),
        }
    }
}

impl<'a> Transition<'a> {
    pub fn new(linestring: LineString, graph: &'a Graph) -> Self {
        Transition {
            linestring,
            graph,
            error: None,
        }
    }

    pub fn set_error(self, error: f64) -> Self {
        Transition {
            error: Some(error),
            ..self
        }
    }

    /// Calculates the emission probability of `dist` (the GPS error from
    /// the observed point and matched point) with a given GPS `error`
    fn emission_probability<K: Into<f64>, T: Div<Output = K> + Mul>(dist: T, error: T) -> f64 {
        let alpha: f64 = (dist / error).into();
        E.powf(-0.5f64 * alpha.powi(2))
    }

    /// Calculates the transition probability of between two candidates
    /// given the `shortest_dist` between them, and the `euclidean_dist`
    /// of the segment we are transitioning over
    fn transition_probability<K: Into<f64>, T: Div<Output = K> + PartialOrd>(
        shortest_dist: T,
        euclidean_dist: T,
    ) -> f64 {
        match euclidean_dist >= shortest_dist {
            true => 1.0f64,
            false => (euclidean_dist / shortest_dist).into(),
        }
    }

    /// Refines the candidates into TransitionNodes with their respective
    /// emission and transition probabilities in the hidden markov model.
    ///
    /// Based on the method used in FMM
    fn refine_candidates<'t>(&'t self, layer_a: &'t ImbuedLayer<'t>, layer_b: &'t ImbuedLayer<'t>) {
        // Now we modify the nodes to refine them
        layer_a.1
            .iter()
            .for_each(|node| {
                // Iterate for each sub-node to a node
                layer_b.1
                    .iter()
                    .for_each(|alt| {
                        match self.graph.route(node.borrow().candidate.position, alt.borrow().candidate.position) {
                            Some((weight, _)) => {
                                let transition_probability =
                                    Transition::transition_probability(weight as f64, layer_a.0.segment.length);

                                let net_probability = node.borrow().cumulative_probability
                                    + transition_probability.log(E)
                                    + alt.borrow().emission_probability.log(E);

                                let mut mutable_ptr = alt.borrow_mut();

                                mutable_ptr.cumulative_probability = net_probability;
                                mutable_ptr.prev_best = Some(node);
                                mutable_ptr.transition_probability = transition_probability;
                            }
                            None => {},
                        }
                    });
            });
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, layers: &Vec<ImbuedLayer>) -> Vec<Point> {
        if let Some((last_layer, nodes)) = layers.last() {
            // Find the optimal candidate in last_layer.candidates
            // TODO: Actually pick the best
            let optimal_node = nodes.last().unwrap();

            return std::iter::from_fn({
                let mut previous_best = optimal_node.borrow().prev_best;
                move || {
                    // Perform rollup on the candidates to walk-back the path
                    previous_best.take().map(|prev| {
                        previous_best = prev.borrow().prev_best;
                        prev
                    })
                }
            })
            .fuse()
            .inspect(|node| {
                debug!(
                    "Candidate {:?} ({}) selected.",
                    node.borrow().candidate.index,
                    node.borrow().candidate.position.wkt_string()
                )
            })
            .map(|candidate| candidate.borrow().candidate.position)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        }

        // Return no points by default
        vec![]
    }

    // Try move to `impl Iterator<Item = RefinedTransitionLayer> + '_`
    /// Backtracks the HMM from the most appropriate final point to
    /// its prior most appropriate points
    pub fn backtrack(&'a self, distance: f64) -> Vec<Point> {
        // Deconstruct the trajectory into individual segments
        let as_coordinates = self.linestring.clone().into_points();
        let segments = as_coordinates
            .windows(2)
            .map(|window| TrajectorySegment::new(&window[0], &window[1]))
            .collect::<Vec<_>>();

        // TODO: Merge ˄ and ˅ into one iterator?
        // TODO: Check if index is required later.
        let mut index = 0;

        // Create transition layers from each segment
        let layers = segments
            .iter()
            .filter_map(|segment| {
                let candidates = self
                    .graph
                    // Get all relevant (projected) nodes within Nm
                    .nearest_projected_nodes(&segment.source, distance)
                    .map(|(position, edge)| {
                        index = index + 1; // Incremental index

                        TransitionCandidate {
                            index,
                            edge,
                            position,
                        }
                    })
                    .collect::<Vec<_>>();

                Some(TransitionLayer {
                    candidates,
                    segment,
                })
            })
            .collect::<Vec<_>>();

        // We need to keep this in the outer-scope
        let node_layers = layers
            .iter()
            .map(|layer| {
                let nodes = layer
                    .candidates
                    .iter()
                    .map(|candidate| {
                        let distance = candidate.position.euclidean_distance(layer.segment.source);
                        let emission_probability =
                            Transition::emission_probability(distance, self.error.unwrap_or(DEFAULT_ERROR));

                        TransitionNode {
                            candidate,
                            prev_best: None,
                            emission_probability,
                            transition_probability: 0.0f64,
                            cumulative_probability: 0.0f64,
                        }
                    })
                    .map(RefCell::new)
                    .collect::<Vec<_>>();

                (layer, nodes)
            })
            .collect::<Vec<_>>();

        // Refine Step
        node_layers
            .windows(2)
            .for_each(|layers| {
                self.refine_candidates(&layers[0], &layers[1]);
            });

        // Now we refine the candidates
        self.collapse(&node_layers)
    }
}
