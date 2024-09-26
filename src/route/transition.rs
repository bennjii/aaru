use std::cell::{RefCell};
use std::cmp::Ordering;
use std::f64::consts::E;
use std::ops::{Div, Mul};

use geo::{HaversineDistance, HaversineLength, LineString, Point};
use log::debug;
use wkt::ToWkt;
use std::borrow::BorrowMut;
use rayon::iter::IndexedParallelIterator;
use rayon::prelude::ParallelSlice;
use rayon::iter::ParallelIterator;
use tokio::time::Instant;

use crate::codec::element::variants::Node;
use crate::route::graph::{Edge, NodeIx};
use crate::route::Graph;

const DEFAULT_ERROR: f64 = 20f64;

pub struct Transition<'a> {
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
    segment: TrajectorySegment<'a>,
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

type ImbuedLayer<'t> = Vec<RefCell<TransitionNode<'t>>>;

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
    pub fn new(graph: &'a Graph) -> Self {
        Transition {
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
        layer_a
            .iter()
            .for_each(|node| {
            debug!("Outward routing from {} to {} nodes", node.borrow().candidate.index, layer_b.len());

            // Iterate for each sub-node to a node
            layer_b
                .iter()
                // .filter(|b| node.borrow().candidate.index != b.borrow().candidate.index)
                .for_each(|alt| {
                    let mut time = Instant::now();

                    debug!("Transition is routing between {} and {}",
                        node.borrow().candidate.index,
                        alt.borrow().candidate.index
                    );
                    
                    let start = node.borrow().candidate.edge.0;
                    let end = alt.borrow().candidate.edge.0;
                    println!("TIMING: Obtain=@{}", time.elapsed().as_micros());
                    time = Instant::now();

                    match self.graph.route_raw(start, end) {
                        Some((weight, nodes)) => {
                            println!("TIMING: Route=@{}", time.elapsed().as_micros());

                            let direct_distance = node.borrow().candidate.position
                                .haversine_distance(&alt.borrow().candidate.position);

                            println!("TIMING: Distance=@{}", time.elapsed().as_micros());

                            // TODO: Consider doing this by default on route
                            // TODO: Consider returning these nodes to "interpolate" the route
                            let travel_distance = nodes
                                .iter()
                                .map(|node| node.position)
                                .collect::<LineString>()
                                .haversine_length();

                            println!("TIMING: TravelDistance=@{}", time.elapsed().as_micros());

                            // Compare actual distance with straight-line-distance
                            let transition_probability = Transition::transition_probability(
                                travel_distance,
                                direct_distance
                            );

                            let net_probability = node.borrow().cumulative_probability
                                + transition_probability.log(E)
                                + alt.borrow().emission_probability.log(E);

                            println!("TIMING: Probabilities=@{}", time.elapsed().as_micros());

                            // Only one-such borrow must exist at one time,
                            // simply do not borrow again in this scope.
                            let mut mutable_ptr = alt.borrow_mut();

                            println!("TIMING: GettingMutBorrow=@{}", time.elapsed().as_micros());

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

                            println!("TIMING: CommitChanges=@{}", time.elapsed().as_micros());
                        }
                        None => debug!("Found no route between nodes.")
                    }

                    println!("TIMING: Full={}", time.elapsed().as_micros());
                });
        });
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, layers: &Vec<ImbuedLayer>) -> Vec<Point> {
        if let Some(nodes) = layers.last() {
            // debug!(
            //     "Finding best node on the last layer: (Source={}, NoCandidates={})",
            //     last_layer.segment.source.to_wkt().to_string(),
            //     last_layer.candidates.len()
            // );

            // Find the optimal candidate in last_layer.candidates
            if let Some(best_node) = nodes.into_iter()
                .max_by(|a, b| {
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
    pub fn backtrack(&'a self, line_string: LineString, distance: f64) -> Vec<Point> {
        let time = Instant::now();
        debug!("BACKTRACK::TIMING:: Start=@{}", time.elapsed().as_micros());

        // Deconstruct the trajectory into individual segments
        let points = line_string.into_points();
        debug!("BACKTRACK::TIMING:: GeneratePoints=@{}", time.elapsed().as_micros());

        let layers = points
            .as_parallel_slice()
            .par_windows(2)
            .map(|window| TrajectorySegment::new(&window[0], &window[1]))
            .enumerate()
            .filter_map(|(segment_index, segment)| {
                debug!("Looking for nodes adjacent to {}, within distance {}.", segment.source.wkt_string(), distance);

                let candidates = self
                    .graph
                    // Get all relevant (projected) nodes within Nm
                    .nearest_projected_nodes(segment.source, distance)
                    // .take(5) // Lazily consume only what is required
                    .enumerate()
                    .map(|(index, (position, edge))| {
                        TransitionCandidate {
                            index: index as i64,
                            edge,
                            position,
                        }
                    })
                    .collect::<Vec<_>>();

                debug!("Obtained {} candidates for segment {}", candidates.len(), segment_index);
                if candidates.is_empty() {
                    return None
                }
                
                Some(TransitionLayer {
                    candidates,
                    segment,
                })
            })
            .collect::<Vec<_>>();

        // ^^
        debug!("BACKTRACK::TIMING:: GenerateLayers=@{}", time.elapsed().as_micros()); // 7486074us - 1147023
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

                if candidates.is_empty() {
                    return None
                }

                return Some(candidates)
            })
            .collect::<Vec<Vec<_>>>();

        debug!("BACKTRACK::TIMING:: GenerateAllNodes=@{}", time.elapsed().as_micros());
        debug!("All {} layer nodes generated", node_layers.len());

        // Refine Step
        let mut layer_index = 0;
        node_layers
            .windows(2)
            .for_each(|layers| {
                debug!("Moving onto layers {}", layer_index);
                self.refine_candidates(&layers[0], &layers[1]);
                layer_index += 1;
            });

        debug!("BACKTRACK::TIMING:: RefineLayers=@{}", time.elapsed().as_micros());
        debug!("Refined all layers, collapsing...");

        // Now we refine the candidates
        let collapsed = self.collapse(&node_layers);
        debug!("BACKTRACK::TIMING:: Collapsed=@{}", time.elapsed().as_micros());

        collapsed
    }
}
