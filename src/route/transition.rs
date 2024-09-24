use std::cell::RefCell;
use std::cmp::Ordering;
use std::f64::consts::E;
use std::ops::{Div, Mul};

use geo::{EuclideanDistance, HaversineDistance, HaversineLength, LineString, Point};
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
        debug!("Segment length {} between {:?} and {:?}", a.haversine_distance(b), a, b);
        TrajectorySegment {
            source: a,
            target: b,
            length: a.haversine_distance(b),
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
        // TODO: Refactor how this works to it can be paralleled.
        // Now we modify the nodes to refine them
        layer_a.1.iter().for_each(|node| {
            // Required to meet borrowing rules
            let node_position = node.borrow().candidate.position.clone();

            // Iterate for each sub-node to a node
            layer_b.1.iter()
                .filter(|b| node.borrow().candidate.index != b.borrow().candidate.index)
                .for_each(|alt| {
                    let alt_position = alt.borrow().candidate.position.clone();

                    debug!("Routing between {} and {}",
                        node.borrow().candidate.index,
                        alt.borrow().candidate.index
                    );

                    match self.graph.route(node_position, alt_position) {
                        Some((weight, nodes)) => {
                            // TODO: Consider doing this by default on route
                            let travel_distance = nodes
                                .iter()
                                .map(|node| node.position)
                                .collect::<LineString>()
                                .haversine_length();

                            // Compare actual distance with straight-line-distance
                            let transition_probability = Transition::transition_probability(
                                travel_distance,
                                layer_a.0.segment.length,
                            );

                            debug!(
                                "Transition Probability: {}, Segment Length: {}",
                                transition_probability,
                                layer_a.0.segment.length
                            );

                            let net_probability = node.borrow().cumulative_probability
                                + transition_probability.log(E)
                                + alt.borrow().emission_probability.log(E);

                            debug!(
                                "Route found! Total weight: {}, Net Probability: {}",
                                weight, net_probability,
                            );

                            debug!(
                                "=> Ln(This.Transition)={}, Ln(Alt.Emission)={}",
                                transition_probability.log(E), alt.borrow().emission_probability.log(E)
                            );

                            // Only one-such borrow must exist at one time,
                            // simply do not borrow again in this scope.
                            let mut mutable_ptr = alt.borrow_mut();

                            // Only if it probabilistic to route do we make the change
                            if net_probability > mutable_ptr.cumulative_probability {
                                debug!("Committing changes, cumulative probability reached.");

                                mutable_ptr.cumulative_probability = net_probability;
                                mutable_ptr.transition_probability = transition_probability;
                                mutable_ptr.prev_best = Some(node);
                            } else {
                                debug!("Insufficient, cannot make change. Cumulative was: {}", mutable_ptr.cumulative_probability);
                            }
                        }
                        None => debug!("Found no route between nodes.")
                    }
                });
        });
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, layers: &Vec<ImbuedLayer>) -> Vec<Point> {
        if let Some((last_layer, nodes)) = layers.last() {
            debug!(
                "Finding best node on the last layer: (Source={}, NoCandidates={})",
                last_layer.segment.source.to_wkt().to_string(),
                last_layer.candidates.len()
            );

            // Find the optimal candidate in last_layer.candidates
            if let Some(best_node) = nodes.iter().max_by(|a, b| {
                a.borrow()
                    .cumulative_probability
                    .partial_cmp(&b.borrow().cumulative_probability)
                    .unwrap_or(Ordering::Equal)
            }) {
                return std::iter::from_fn({
                    let mut previous_best = best_node.borrow().prev_best;
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
        }

        // Return no points by default
        debug!("Insufficient layers or no optimal candidate, empty vec.");
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
            .inspect(|segment| debug!("SegmentLength={}", segment.length))
            .collect::<Vec<_>>();

        debug!("Obtained {} trajectory segments.", segments.len());

        // TODO: Merge ˄ and ˅ into one iterator?
        // TODO: Check if index is required later.
        let mut index = 0;
        let mut segment_index = 0;

        // Create transition layers from each segment
        let layers = segments
            .iter()
            .filter_map(|segment| {
                debug!("Looking for nodes adjacent to {}, within distance {}.", segment.source.wkt_string(), distance);

                let candidates = self
                    .graph
                    // Get all relevant (projected) nodes within Nm
                    .nearest_projected_nodes(&segment.source, distance)
                    .map(|(position, edge)| {
                        index += 1; // Incremental index

                        TransitionCandidate {
                            index,
                            edge,
                            position,
                        }
                    })
                    .collect::<Vec<_>>();

                debug!("Obtained {} candidates for segment {}", candidates.len(), segment_index);
                segment_index += 1;

                Some(TransitionLayer {
                    candidates,
                    segment,
                })
            })
            .collect::<Vec<_>>();

        debug!("Formed {} transition layers.", layers.len());

        // We need to keep this in the outer-scope
        let node_layers = layers
            .iter()
            .filter_map(|layer| {
                let nodes = layer
                    .candidates
                    .iter()
                    .map(|candidate| {
                        let distance = candidate.position.haversine_distance(layer.segment.source);
                        let emission_probability = Transition::emission_probability(
                            distance,
                            self.error.unwrap_or(DEFAULT_ERROR),
                        );

                        TransitionNode {
                            candidate,
                            prev_best: None,
                            emission_probability,
                            transition_probability: 0.0f64,
                            cumulative_probability: f64::NEG_INFINITY,
                        }
                    })
                    .map(RefCell::new)
                    .collect::<Vec<_>>();

                if nodes.len() == 0 {
                    debug!("Layer with {} candidates lacks sufficient nodes, rejecting.", layer.candidates.len());
                    return None
                }

                debug!("Have {} nodes for layer", nodes.len());
                Some((layer, nodes))
            })
            .collect::<Vec<_>>();

        debug!("All {} layer nodes generated", node_layers.len());

        // Refine Step
        let mut layer_index = 0;
        node_layers.windows(2).for_each(|layers| {
            debug!("Moving onto layers {}", layer_index);
            self.refine_candidates(&layers[0], &layers[1]);
            layer_index += 1;
        });

        debug!("Refined all layers, collapsing...");

        // Now we refine the candidates
        self.collapse(&node_layers)
    }
}
