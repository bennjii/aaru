use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::f64::consts::E;
use std::ops::{Div, Mul};
use std::time::Instant;

use geo::{HaversineDistance, HaversineLength, LineString, Point};
use log::debug;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::prelude::ParallelSlice;
use wkt::ToWkt;

use crate::route::transition::node::{
    ImbuedLayer, TransitionCandidate, TransitionLayer, TransitionNode,
};
use crate::route::transition::segment::TrajectorySegment;
use crate::route::Graph;

const DEFAULT_ERROR: f64 = 20f64;

pub struct Transition<'a> {
    graph: &'a Graph,
    error: Option<f64>,
}

impl<'a> Transition<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Transition { graph, error: None }
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
        layer_a.iter().for_each(|node| {
            debug!(
                "Outward routing from {} to {} nodes",
                node.borrow().candidate.index,
                layer_b.len()
            );

            // Iterate for each sub-node to a node
            layer_b
                .iter()
                // .filter(|b| node.borrow().candidate.index != b.borrow().candidate.index)
                .for_each(|alt| {
                    let mut time = Instant::now();

                    debug!(
                        "Transition is routing between {} and {}",
                        node.borrow().candidate.index,
                        alt.borrow().candidate.index
                    );
                    let start = node.borrow().candidate.edge.0;
                    let end = alt.borrow().candidate.edge.0;
                    debug!("TIMING: Obtain=@{}", time.elapsed().as_micros());
                    time = Instant::now();

                    match self.graph.route_raw(start, end) {
                        Some((_, nodes)) => {
                            debug!("TIMING: Route=@{}", time.elapsed().as_micros());

                            let direct_distance = node
                                .borrow()
                                .candidate
                                .position
                                .haversine_distance(&alt.borrow().candidate.position);

                            debug!("TIMING: Distance=@{}", time.elapsed().as_micros());

                            // TODO: Consider doing this by default on route
                            // TODO: Consider returning these nodes to "interpolate" the route
                            let travel_distance = nodes
                                .iter()
                                .map(|node| node.position)
                                .collect::<LineString>()
                                .haversine_length();

                            debug!("TIMING: TravelDistance=@{}", time.elapsed().as_micros());

                            // Compare actual distance with straight-line-distance
                            let transition_probability = Transition::transition_probability(
                                travel_distance,
                                direct_distance,
                            );

                            let net_probability = node.borrow().cumulative_probability
                                + transition_probability.log(E)
                                + alt.borrow().emission_probability.log(E);

                            debug!("TIMING: Probabilities=@{}", time.elapsed().as_micros());

                            // Only one-such borrow must exist at one time,
                            // simply do not borrow again in this scope.
                            let mut mutable_ptr = alt.borrow_mut();

                            debug!("TIMING: GettingMutBorrow=@{}", time.elapsed().as_micros());

                            // Only if it probabilistic to route do we make the change
                            if net_probability >= mutable_ptr.cumulative_probability {
                                debug!("Committing changes, cumulative probability reached.");

                                // Extension...
                                //
                                // NodeRelation {
                                //     node,
                                //     // Storing other information is irrelevant,
                                //     // as it means we loose paralellism, since
                                //     // the information is relative to the node.
                                //     transition_probability,
                                //     path: nodes
                                // }

                                mutable_ptr.cumulative_probability = net_probability;
                                mutable_ptr.transition_probability = transition_probability;
                                mutable_ptr.prev_best = Some(node);
                                *mutable_ptr.current_path.borrow_mut() = nodes;
                            } else {
                                debug!(
                                    "Insufficient, cannot make change. Cumulative was: {}",
                                    mutable_ptr.cumulative_probability
                                );
                            }

                            debug!("TIMING: CommitChanges=@{}", time.elapsed().as_micros());
                        }
                        None => debug!("Found no route between nodes."),
                    }

                    debug!("TIMING: Full={}", time.elapsed().as_micros());
                });
        });
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, layers: &Vec<ImbuedLayer>) -> Vec<Point> {
        if let Some(nodes) = layers.last() {
            // All nodes with a possible route, sorted by best probability
            // TODO: A Dijkstra reverse search would yield better results as to route-depth and partial patching
            let searchable = nodes
                .iter()
                .filter(|node| node.borrow().prev_best.is_some())
                .max_by(|a, b| {
                    a.borrow()
                        .cumulative_probability
                        .partial_cmp(&b.borrow().cumulative_probability)
                        .unwrap_or(Ordering::Equal)
                });

            // Find the optimal candidate in last_layer.candidates
            if let Some(best_node) = searchable {
                return std::iter::from_fn({
                    let mut previous_best = Some(best_node);
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
                            .collect::<Vec<_>>())
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

    /// Backtracks the HMM from the most appropriate final point to
    /// its prior most appropriate points
    pub fn backtrack(&'a self, line_string: LineString, distance: f64) -> Vec<Point> {
        let time = Instant::now();
        debug!("BACKTRACK::TIMING:: Start=@{}", time.elapsed().as_micros());

        // Deconstruct the trajectory into individual segments
        let points = line_string.into_points();
        debug!(
            "BACKTRACK::TIMING:: GeneratePoints=@{}",
            time.elapsed().as_micros()
        );

        let layers = points
            .as_parallel_slice()
            .par_windows(2)
            .map(|window| TrajectorySegment::new(&window[0], &window[1]))
            .enumerate()
            .filter_map(|(segment_index, segment)| {
                debug!(
                    "Looking for nodes adjacent to {}, within distance {}.",
                    segment.source.wkt_string(),
                    distance
                );

                let candidates = self
                    .graph
                    // We want exactly 10 candidates to search with
                    .nearest_projected_nodes(segment.source, 5)
                    .enumerate()
                    .map(|(index, (position, edge))| TransitionCandidate {
                        index: index as i64,
                        edge,
                        position,
                    })
                    .collect::<Vec<_>>();

                debug!(
                    "Obtained {} candidates for segment {}",
                    candidates.len(),
                    segment_index
                );
                if candidates.is_empty() {
                    return None;
                }

                Some(TransitionLayer {
                    candidates,
                    segment,
                })
            })
            .collect::<Vec<_>>();

        // If we use the bridging technique
        // from the above function, then this
        // can be one iterator meaning we
        // dont have to collect and re-create,
        // thus can be paralellized completely.
        debug!(
            "BACKTRACK::TIMING:: GenerateLayers=@{}",
            time.elapsed().as_micros()
        );
        debug!("Formed {} transition layers.", layers.len());

        // We need to keep this in the outer-scope
        let node_layers = layers
            .iter()
            // Only used for debug indexing
            .enumerate()
            .filter_map(|(layer_index, layer)| {
                let candidates = layer
                    .candidates
                    .iter()
                    .map(|candidate| {
                        let emission_probability = Transition::emission_probability(
                            candidate.position.haversine_distance(layer.segment.source),
                            self.error.unwrap_or(DEFAULT_ERROR),
                        );

                        TransitionNode {
                            candidate,
                            prev_best: None,
                            current_path: Box::new(vec![]),
                            emission_probability,
                            transition_probability: 0.0f64,
                            cumulative_probability: if layer_index == 0 {
                                emission_probability.log(E)
                            } else {
                                f64::NEG_INFINITY
                            },
                        }
                    })
                    .map(RefCell::new)
                    .collect::<Vec<_>>();

                if candidates.is_empty() {
                    return None;
                }

                return Some(candidates);
            })
            .collect::<Vec<_>>();

        debug!(
            "BACKTRACK::TIMING:: GenerateAllNodes=@{}",
            time.elapsed().as_micros()
        );
        debug!("All {} layer nodes generated", node_layers.len());

        // Refine Step
        let mut layer_index = 0;
        node_layers.windows(2).for_each(|layers| {
            debug!("Moving onto layers {}", layer_index);
            self.refine_candidates(&layers[0], &layers[1]);
            layer_index += 1;
        });

        debug!(
            "BACKTRACK::TIMING:: RefineLayers=@{}",
            time.elapsed().as_micros()
        );
        debug!("Refined all layers, collapsing...");

        // Now we refine the candidates
        let collapsed = self.collapse(&node_layers);
        debug!(
            "BACKTRACK::TIMING:: Collapsed=@{}",
            time.elapsed().as_micros()
        );

        collapsed
    }
}
