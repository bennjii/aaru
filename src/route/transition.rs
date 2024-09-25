use std::cell::RefCell;
use std::cmp::Ordering;
use std::f64::consts::E;
use std::ops::{Div, Mul};

use geo::{HaversineDistance, HaversineLength, LineString, Point};
use log::debug;
use wkt::ToWkt;
use std::borrow::BorrowMut;

use crate::codec::element::variants::Node;
use crate::route::graph::{Edge, NodeIx};
use crate::route::Graph;

const DEFAULT_ERROR: f64 = 20f64;

pub struct Transition<'a> {
    // The original linestring
    linestring: LineString,
    graph: &'a Graph,
    error: Option<f64>,
}

#[derive(Clone)]
struct TransitionNode<'a> {
    candidate: &'a TransitionCandidate<'a>,
    prev_best: Option<&'a RefCell<TransitionNode<'a>>>,
    current_path: Box<Vec<Node>>,
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
    #[inline]
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
            debug!("Outward routing from {} to {} nodes", node.borrow().candidate.index, layer_b.1.len());

            // Iterate for each sub-node to a node
            layer_b.1.iter()
                .filter(|b| node.borrow().candidate.index != b.borrow().candidate.index)
                .for_each(|alt| {
                    debug!("Transition is routing between {} and {}",
                        node.borrow().candidate.index,
                        alt.borrow().candidate.index
                    );
                    
                    debug!("WKT {} ==> {}", node.borrow().candidate.position.wkt_string(), alt.borrow().candidate.position.wkt_string());
                    debug!("IDT {} ==> {}", node.borrow().candidate.edge.0, alt.borrow().candidate.edge.0);
                    
                    let start = node.borrow().candidate.edge.0;
                    let end = alt.borrow().candidate.edge.0;

                    match self.graph.route_raw(start, end) {
                        Some((weight, nodes)) => {
                            let direct_distance = node.borrow().candidate.position.haversine_distance(&alt.borrow().candidate.position);

                            // TODO: Consider doing this by default on route
                            // TODO: Consider returning these nodes to "interpolate" the route
                            let travel_distance = nodes
                                .iter()
                                .map(|node| node.position)
                                .collect::<LineString>()
                                .haversine_length();

                            // Compare actual distance with straight-line-distance
                            let transition_probability = Transition::transition_probability(
                                travel_distance,
                                direct_distance
                                // layer_a.0.segment.length,
                            );

                            let net_probability = node.borrow().cumulative_probability
                                + transition_probability.log(E)
                                + alt.borrow().emission_probability.log(E);

                            debug!(
                                "Transition Probability: {}, Travel Distance: {}, Segment Length: {}",
                                transition_probability,
                                travel_distance,
                                direct_distance
                            );

                            debug!(
                                "=> Ln(This.Transition)={}, Ln(Alt.Emission)={}, Node.Cumulative={}",
                                transition_probability.log(E), alt.borrow().emission_probability.log(E),
                                node.borrow().cumulative_probability
                            );

                            // Only one-such borrow must exist at one time,
                            // simply do not borrow again in this scope.
                            let mut mutable_ptr = alt.borrow_mut();

                            // Only if it probabilistic to route do we make the change
                            if net_probability >= mutable_ptr.cumulative_probability {
                                debug!("Committing changes, cumulative probability reached.");

                                mutable_ptr.cumulative_probability = net_probability;
                                mutable_ptr.transition_probability = transition_probability;
                                mutable_ptr.prev_best = Some(node);
                                *mutable_ptr.current_path.borrow_mut() = nodes;
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
            if let Some(best_node) = nodes.into_iter().max_by(|a, b| {
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
                .flat_map(|candidate|
                    // TODO: Slow implementation..
                    candidate.borrow().current_path
                        .iter()
                        .rev()
                        .map(|item| item.position)
                        .collect::<Vec<_>>()
                )
                // .map(|candidate| candidate.borrow().candidate.position)
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
                    .nearest_projected_nodes(segment.source, distance)
                    .inspect(|(pos, _)| debug!("=> Node={}", pos.wkt_string()))
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

                if candidates.is_empty() {
                    return None
                }
                
                Some(TransitionLayer {
                    candidates,
                    segment,
                })
            })
            .collect::<Vec<_>>();

        debug!("Formed {} transition layers.", layers.len());
        let mut layer_index = 0;

        // We need to keep this in the outer-scope
        let node_layers = layers
            .iter()
            // Only used for debug indexing
            .map(|layer| {
                let val = (layer, layer_index);
                layer_index += 1;
                val
            })
            .filter_map(|(layer, layer_index)| {
                let nodes = layer
                    .candidates
                    .iter()
                    .map(|candidate| {
                        let emission_probability = Transition::emission_probability(
                            layer.segment.length,
                            self.error.unwrap_or(DEFAULT_ERROR),
                        );

                        debug!("LayerIndex={}", layer_index);

                        TransitionNode {
                            candidate,
                            prev_best: None,
                            current_path: Box::new(vec![]),
                            emission_probability,
                            transition_probability: 0.0f64,
                            cumulative_probability: if layer_index == 0 { emission_probability.log(E) } else { f64::NEG_INFINITY },
                        }
                    })
                    .map(RefCell::new)
                    .collect::<Vec<_>>();

                if nodes.is_empty() {
                    debug!("Layer with {} candidates lacks sufficient nodes, rejecting.", layer.candidates.len());
                    return None
                }

                debug!("Have {} nodes for layer {}.", nodes.len(), layer_index);
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
