use std::collections::HashMap as StandardHashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Div, Sub};
use std::sync::{Arc, RwLock};

use geo::{Bearing, Distance, Haversine, LineString, Point};
use geohash::Direction::SE;
use log::{debug, info, warn};

use pathfinding::prelude::dijkstra_reach;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Direction, Graph};

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSlice;

use scc::HashMap;
use wkt::ToWkt;

use crate::codec::element::variants::common::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::costing::emission::EmissionStrategy;
use crate::route::transition::costing::transition::TransitionStrategy;
use crate::route::transition::node::TransitionCandidate;
use crate::route::transition::trip::Trip;
use crate::route::transition::{
    Costing, CostingStrategies, DefaultEmissionCost, DefaultTransitionCost, EmissionContext,
    Strategy,
};
use crate::route::Graph as RouteGraph;

const DEFAULT_ERROR: f64 = 10f64;
const TRANSITION_LOGARITHM_BASE: f64 = 10.0;

type LayerId = usize;
type NodeId = usize;

#[derive(Clone, Copy, Debug)]
pub struct TransitionPair<K>
where
    K: Into<f64> + Div<Output = K> + PartialOrd,
{
    shortest_distance: K,
    path_length: K,
}

type CandidateEdge = (f64, Vec<NodeIx>);
type CandidatePoint = (Point, f64);

type ArcGraph<A, B> = Arc<RwLock<Graph<A, B, Directed>>>;

pub struct Transition<'a, E, T>
where
    E: EmissionStrategy,
    T: TransitionStrategy,
{
    map: &'a RouteGraph,

    // keep in mind we dont need layerid and nodeid, but theyre useful for debugging so we'll keep for now.
    graph: ArcGraph<CandidatePoint, CandidateEdge>,
    candidates: HashMap<NodeIndex, TransitionCandidate>,
    error: Option<f64>,

    layers: Vec<Vec<NodeIndex>>,
    points: Vec<Point>,

    heuristics: CostingStrategies<E, T>,
}

#[derive(Debug)]
pub enum MatchError {
    CollapseFailure,
    NoPointsProvided,
}

struct InterpolatedNodes {
    pub node_idx: Vec<NodeIx>,
}

pub struct Match {
    pub cost: f64,

    /// Direct matches for each individual point in the initial trajectory.
    /// These are the new points, with associated routing information to
    /// aid in information recovery.
    pub matched: Vec<TransitionCandidate>,

    pub interpolated: LineString, // Vec<InterpolatedNodes>
}

impl<'a, E, T> Transition<'a, E, T>
where
    E: EmissionStrategy + Send + Sync,
    T: TransitionStrategy + Send + Sync,
{
    pub fn new(
        map: &'a RouteGraph,
        linestring: LineString,
        costing: CostingStrategies<E, T>,
    ) -> Transition<'a, E, T> {
        let points = linestring.into_points();

        Transition {
            map,
            points,

            graph: Arc::new(RwLock::new(Graph::new())),
            candidates: HashMap::new(),
            error: None,
            layers: vec![],
            heuristics: costing,
        }
    }

    pub fn set_error(self, error: f64) -> Self {
        Transition {
            error: Some(error),
            ..self
        }
    }

    /// Cost(v) = 1/Z * exp(-1 * (v / beta))
    /// Note: V should be positive, consider abs value.
    #[inline(always)]
    pub(crate) fn cost_decay(value: f64, z: f64, beta: f64) -> f64 {
        ((1.0 / z) * (-1.0 * (value / beta)).exp()).abs()
    }

    /// Calculates the emission probability of `dist` (the GPS error from
    /// the observed point and matched point) with a given GPS `error`
    #[inline]
    pub(crate) fn emission_cost<
        K: Into<f64> + Sub<Output = K> + Div<Output = K> + PartialOrd + Copy,
    >(
        dist: K,
        error: K,
    ) -> f64 {
        debug_assert!(error.into() > 0.0, "Error must be positive");

        // To match to create a continuous fn with dist < err case.
        const Z: f64 = 1.0;
        const BETA: f64 = -100.0;

        // let abs_diff = Into::<f64>::into(dist - error).powi(2);
        Self::cost_decay(dist.into().powi(2), Z, BETA)
    }

    #[inline]
    pub fn transition_cost<
        K: Into<f64> + Div<Output = K> + Sub<Output = K> + PartialOrd + Debug,
    >(
        pair: TransitionPair<K>,
    ) -> Option<f64> {
        const Z: f64 = 1.0;
        const BETA: f64 = -50.0;

        if pair.path_length < pair.shortest_distance {
            // Somehow we got a shorter or equal path to the
            // shortest distance, regardless there is no cost in this.
            debug!("I'm shorter");
            return None;
        }

        let abs_diff = Into::<f64>::into(pair.shortest_distance - pair.path_length).powi(2);
        Some(Self::cost_decay(abs_diff, Z, BETA))
    }

    pub fn turn_cost(&self, path: &[OsmEntryId]) -> f64 {
        const Z: f64 = 1.0;
        const BETA: f64 = -50.0;

        let hashmap = self.map.hash.read().unwrap();

        let locations = path
            .iter()
            .flat_map(|id| hashmap.get(id))
            .collect::<Vec<_>>();

        let angle = Trip::from(locations.as_slice()).total_angle();
        Self::cost_decay(angle, Z, BETA)
    }

    /// May return None if a cycle is detected.
    pub(crate) fn pathbuilder<N, C>(target: &N, parents: &StandardHashMap<N, (N, C)>) -> Vec<N>
    where
        N: Eq + Hash + Copy,
    {
        let mut rev = vec![*target];
        let mut next = target;
        while let Some((parent, _)) = parents.get(next) {
            rev.push(*parent);
            next = parent;
        }
        rev.reverse();
        rev
    }

    /// Generates the layers of the transition graph, where each layer
    /// represents a point in the linestring, and each node in the layer
    /// represents a candidate transition point, within the `distance`
    /// search radius of the linestring point, which was found by the
    /// projection of the linestring point upon the closest edges within this radius.
    pub fn generate_layers(&self, distance: f64) -> Vec<Vec<NodeIndex>> {
        // Create the base transition graph
        let candidates = self
            .points
            .par_iter()
            .enumerate()
            .map(|(layer_id, point)| {
                info!(
                    "Generating layer {} (Point={})",
                    layer_id,
                    point.wkt_string()
                );

                self.map
                    // We'll do a best-effort search (square) radius
                    .nearest_projected_nodes(point, distance)
                    .enumerate()
                    .map(|(node_id, (position, map_edge))| {
                        // We have the actual projected position, and it's associated edge.
                        let emission = self
                            .heuristics
                            .emission(EmissionContext::new(&position, point));

                        let candidate = TransitionCandidate {
                            map_edge: (map_edge.0, map_edge.1),
                            position,
                            layer_id,
                            node_id,
                            emission,
                        };

                        candidate
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut write_lock = self.graph.write().unwrap();

        candidates
            .into_iter()
            .map(|candidate| {
                candidate
                    .into_iter()
                    .map(|node| {
                        let node_index = write_lock.add_node((node.position, node.emission));
                        let _ = self.candidates.insert(node_index, node);
                        node_index
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// Refines a single node within an initial layer to all nodes in the
    /// following layer with their respective emission and transition
    /// probabilities in the hidden markov model.
    ///
    /// Based on the method used in FMM / MM2
    #[inline]
    fn refine_candidates(
        &self,
        left_ix: NodeIndex,
        right_ixs: &[NodeIndex],
    ) -> Vec<(NodeIndex, f64, Vec<OsmEntryId>)> {
        let left_candidate = *self.candidates.get(&left_ix).unwrap();

        // debug!(
        //     "Routing from Layer::{}::{} to Layer::{}::*.",
        //     left_candidate.layer_id,
        //     left_candidate.node_id,
        //     left_candidate.layer_id + 1,
        // );

        let (start, end) = left_candidate.map_edge;
        let end_position = self.map.get_position(&end).unwrap();
        let threshold_distance = 2_000f64; // 1km

        // The distance remaining in the edge to travel
        let distance_to_end_of_edge = Haversine::distance(
            left_candidate.position,
            self.map.get_position(&end).unwrap(),
        );

        let reach = dijkstra_reach(&end, |node, _| {
            self.map
                .graph
                .edges_directed(*node, Direction::Outgoing)
                .map(|(_, next, _w)| {
                    (
                        next,
                        if *node != next {
                            let source = self.map.get_position(node).unwrap();
                            let target = self.map.get_position(&next).unwrap();
                            // In centimeters
                            (Haversine::distance(source, target) * 1_000f64) as u32
                        } else {
                            0u32
                        }, // Total accrued distance
                    )
                })
                .collect::<Vec<_>>()
        });

        let probabilities = reach
            .take_while(|p| {
                (distance_to_end_of_edge + (p.total_cost as f64 / 1_000f64)) < threshold_distance
            })
            .map(|predicate| {
                (
                    predicate.clone(),
                    Haversine::distance(
                        left_candidate.position,
                        self.map.get_position(&predicate.node).unwrap(),
                    ),
                )
            })
            // .take_while(|(_, distance)| *distance < threshold_distance)
            .map(|(k, shortest_distance)| {
                (
                    k.node,
                    (
                        // Invalid position so the build_path function
                        // will terminate as the found call will return None
                        k.parent.unwrap_or(OsmEntryId::null()),
                        TransitionPair {
                            shortest_distance,
                            path_length: distance_to_end_of_edge + (k.total_cost as f64 / 1_000f64),
                        },
                    ),
                )
            })
            .collect::<StandardHashMap<OsmEntryId, (OsmEntryId, TransitionPair<f64>)>>();

        let paths = right_ixs
            .iter()
            .filter_map(|key| {
                self.candidates.get(key).and_then(|candidate| {
                    let start_to_inner_position = Haversine::distance(
                        self.map.get_position(&candidate.map_edge.0).unwrap(),
                        candidate.position,
                    );

                    probabilities
                        .get(&candidate.map_edge.0)
                        .map(|(_parent, prob)| {
                            // Refactor but this is to add the cost of entering into the edge to trial
                            return (
                                key,
                                (
                                    candidate.map_edge.0,
                                    TransitionPair {
                                        shortest_distance: prob.shortest_distance,
                                        path_length: prob.path_length + start_to_inner_position,
                                    },
                                ),
                            );
                        })
                })
            })
            .map(|(key, (to, pair))| (key, to, pair, Self::pathbuilder(&to, &probabilities)))
            .map(|(right, target, lengths, path)| {
                let cost = Self::transition_cost(lengths);
                if cost.is_none() {
                    let hashmap = self.map.hash.read().unwrap();

                    let as_linestring = path
                        .iter()
                        .filter_map(|index| hashmap.get(index))
                        .map(|node| node.position)
                        .collect::<LineString>();

                    warn!(
                        "Transition cost is None for {:?} \nPath=GEOMETRYCOLLECTION({}, {}, {})",
                        lengths,
                        left_candidate.position.wkt_string(),
                        hashmap.get(&target).unwrap().position.wkt_string(),
                        as_linestring.wkt_string()
                    );
                }
                // if *cost.deref() < 1.0 {
                //     debug!("Transition cost: {:?} {}", lengths, cost);
                // }

                (*right, cost.unwrap_or(0.0), path)
            })
            .collect::<Vec<(NodeIndex, f64, Vec<OsmEntryId>)>>();

        // debug!(
        //     "TIMING: Full={} ({} -> *)",
        //     time.elapsed().as_micros(),
        //     left_candidate.position.wkt_string(),
        // );

        paths
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, start: NodeIndex, end: NodeIndex) -> Option<(f64, Vec<NodeIndex>)> {
        // There should be exclusive read-access by the time collapse is called.
        let graph = self.graph.read().unwrap();

        // // -- DEBUG
        // let max_weight = graph.raw_nodes()
        //     .iter()
        //     .max_by(|a, b| a.weight.1.total_cmp(&b.weight.1));
        //
        // debug!("Maximum node weight: {:?}", max_weight);
        // // -- DEBUG

        // let finding = pathfinding::prelude::dfs_reach(
        //     start,
        //     |a| {
        //     graph.edges_directed(*a, Direction::Outgoing).map(|v| v.target())
        // }).collect::<Vec<_>>();
        //
        // if finding.contains(&end) {
        //     debug!("Origin found.");
        // } else {
        //     debug!("No origin found, walking backwards..");
        //     for (layer, item) in self.layers.iter().enumerate() {
        //         for index in item {
        //             if finding.contains(index) {
        //                 debug!("Back-part layer located in {layer} @ {:?}", index);
        //             }
        //         }
        //     }
        // }

        petgraph::algo::astar(
            &*graph,
            start,
            |node| node == end,
            |e| {
                // Decaying Transition Cost
                let transition_cost = e.weight().0;
                let turn_cost = self.turn_cost(&e.weight().1);

                // Loosely-Decaying Emission Cost
                let emission_cost = graph.node_weight(e.target()).map_or(f64::INFINITY, |v| v.1);

                transition_cost + emission_cost + turn_cost
            },
            |_| 0.0,
        )
    }

    pub fn generate_probabilities(mut self, distance: f64) -> Self {
        // Deconstruct the trajectory into individual segments
        self.layers = self.generate_layers(distance);

        info!("Layer Generation Complete!");

        // let mut collection: Vec<String> = vec![];
        // self.layers.iter().for_each(|layer| {
        //     layer.iter().for_each(|node| {
        //         let id = OsmEntryId::as_node(node.index() as i64);
        //         let graph = self.graph.read().unwrap();
        //         let position = graph.node_weight(*node);
        //         // .get_position(&OsmEntryId::as_node(node.index() as i64));

        //         if let Some((point, _)) = position {
        //             collection.push(point.wkt_string());
        //         } else {
        //             warn!("Could not resolve location for entry at point {:?}", id);
        //         }
        //     });
        // });

        // debug!("GEOMETRYCOLLECTION ( {} )", collection.join(", "));

        // Declaring all the pairs of indices that need to be refined.
        let transition_probabilities = self
            .layers
            .par_windows(2)
            .enumerate()
            .inspect(|(index, pair)| {
                debug!("Unpacking ({index}) {:?} and {:?}...", pair[0].len(), pair[1].len());
            })
            .flat_map(|(index, vectors)| {
                // Taking all forward pairs of (left, [...right]) such that
                // ...
                let output = vectors[0]
                    .iter()
                    .map(|&a| (a, vectors[1].as_slice()))
                    .collect::<Vec<_>>();

                if output.is_empty() {
                    warn!("Could not find any routing for layer transition ({index}) from {} to {} nodes", vectors[0].len(), vectors[1].len())
                }

                output
            })
            .map(|(left, right)| (left, self.refine_candidates(left, right)))
            // .into_par_iter()
            // .map(|(left, right)| {
            //     let candidates = ;
            //     if candidates.is_empty() {
            //         warn!("No candidates found for {:?}", left);
            //     }
            //
            //     (left, candidates)
            // })
            .collect::<Vec<_>>();

        let mut write_access = self.graph.write().unwrap();

        for (left, weights) in transition_probabilities {
            for (right, weight, path) in weights {
                // Transition Probabilities are of the range {0, 1}, such that
                // -log_N(x) yields a value which is infinitely large when x
                // is close to 0, and 0 when x is close to 1. This is the desired
                // behaviour since a value with a Transition Probability of 1,
                // represents a transition which is most desirable. Therefore, has
                // "zero" cost to move through. Whereas, one with a Transition
                // Probability of 0, is a transition we want to discourage, thus
                // has a +inf cost to traverse.
                //
                // t_p(x) = -log_N(x) in the range { 0 <= x <= 1 }
                // let transition_cost = -weight.deref().log(TRANSITION_LOGARITHM_BASE);

                // debug!("TP {} for path {}", weight, path);
                write_access.add_edge(left, right, (weight, path));
            }
        }

        // let mut output = File::create("./output.viz").expect("Missing output file");
        // write!(output, "{:?}", Dot::with_config(&write_access.deref(), &[Config::EdgeNoLabel])).expect("Could not write");

        drop(write_access);

        self
    }

    /// Backtracks the HMM from the most appropriate final point to
    /// its prior most appropriate points
    ///
    /// Consumes the graph.
    pub fn backtrack(self) -> Result<Match, MatchError> {
        let start = *self.points.first().ok_or(MatchError::NoPointsProvided)?;
        let end = *self.points.last().ok_or(MatchError::NoPointsProvided)?;

        // Add in the start and end points to the graph
        let (source, target) = {
            let mut graph = self.graph.write().unwrap();
            let source = graph.add_node((start, 0.0));
            self.layers.first().unwrap().iter().for_each(|node| {
                graph.add_edge(source, *node, (0.0f64, vec![]));
            });

            let target = graph.add_node((end, 0.0));
            self.layers.last().unwrap().iter().for_each(|node| {
                graph.add_edge(*node, target, (0.0f64, vec![]));
            });

            drop(graph);
            (source, target)
        };

        // Now we refine the candidates
        let (cost, collapsed) = self
            .collapse(source, target)
            .ok_or_else(|| MatchError::CollapseFailure)?;

        debug!("Collapsed with final cost: {}", cost);

        let get_edge = |a: &NodeIndex, b: &NodeIndex| -> Option<(f64, Vec<NodeIx>)> {
            let reader = self.graph.read().ok()?;
            let edge_index = reader.find_edge(*a, *b)?;
            reader.edge_weight(edge_index).cloned()
        };

        let interpolated = collapsed
            .windows(2)
            .filter_map(|candidate| {
                if let [a, b] = candidate {
                    return get_edge(a, b).or_else(|| get_edge(b, a)).map(|pp| {
                        let hashmap = self.map.hash.read().unwrap();

                        pp.1.iter()
                            .filter_map(|index| hashmap.get(index))
                            .map(|node| node.position)
                            .collect::<Vec<_>>()
                    });
                }

                None
            })
            .flatten()
            .collect::<LineString>();

        let matched = collapsed
            .iter()
            .filter_map(|node| self.candidates.get(node))
            .map(|can| *can)
            .collect::<Vec<_>>();

        Ok(Match {
            interpolated,
            matched,
            cost,
        })
    }
}
