use std::collections::HashMap as StandardHashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Div, Sub};
use std::sync::{Arc, RwLock};

use geo::{Distance, Haversine, LineString, Point};
use log::{debug, info, warn};

use pathfinding::prelude::dijkstra_reach;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Direction};

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use rayon::slice::ParallelSlice;

use scc::HashMap;
use wkt::ToWkt;

use crate::codec::element::variants::common::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::candidate::{Candidate, Candidates, Collapse};
use crate::route::transition::costing::emission::EmissionStrategy;
use crate::route::transition::costing::transition::TransitionStrategy;
use crate::route::transition::layer::{LayerGenerator, Layers};
use crate::route::transition::trip::Trip;
use crate::route::transition::{Costing, CostingStrategies, EmissionContext, TransitionContext};
use crate::route::Graph;

const DEFAULT_ERROR: f64 = 10f64;
const TRANSITION_LOGARITHM_BASE: f64 = 10.0;

type LayerId = usize;
type NodeId = usize;

pub struct Transition<'a, E, T>
where
    E: EmissionStrategy,
    T: TransitionStrategy,
{
    map: &'a Graph,

    candidates: Candidates,
    layers: Layers,

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
    pub matched: Vec<Candidate>,
    pub interpolated: LineString,
}

impl<'a, E, T> Transition<'a, E, T>
where
    E: EmissionStrategy + Send + Sync,
    T: TransitionStrategy + Send + Sync,
{
    pub fn new(
        map: &'a Graph,
        linestring: LineString,
        heuristics: CostingStrategies<E, T>,
    ) -> Transition<'a, E, T> {
        let points = linestring.into_points();
        let generator = LayerGenerator::new(map, &heuristics, DEFAULT_ERROR);

        // Generate the layers and candidates.
        let (layers, candidates) = generator.with_points(points);

        Transition {
            map,
            candidates,
            layers,
            heuristics,
        }
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

    /// Refines a single node within an initial layer to all nodes in the
    /// following layer with their respective emission and transition
    /// probabilities in the hidden markov model.
    ///
    /// Based on the method used in FMM / MM2
    // #[inline]
    // fn refine_candidates(
    //     &self,
    //     left_ix: NodeIndex,
    //     right_ixs: &[NodeIndex],
    // ) -> Vec<(NodeIndex, f64, Vec<OsmEntryId>)> {
    //     let left_candidate = *self.candidates.get(&left_ix).unwrap();
    //
    //     // debug!(
    //     //     "Routing from Layer::{}::{} to Layer::{}::*.",
    //     //     left_candidate.layer_id,
    //     //     left_candidate.node_id,
    //     //     left_candidate.layer_id + 1,
    //     // );
    //
    //     let (start, end) = left_candidate.map_edge;
    //     let end_position = self.map.get_position(&end).unwrap();
    //     let threshold_distance = 2_000f64; // 1km
    //
    //     // The distance remaining in the edge to travel
    //     let distance_to_end_of_edge = Haversine::distance(
    //         left_candidate.position,
    //         self.map.get_position(&end).unwrap(),
    //     );
    //
    //     let reach = dijkstra_reach(&end, |node, _| {
    //         self.map
    //             .graph
    //             .edges_directed(*node, Direction::Outgoing)
    //             .map(|(_, next, _w)| {
    //                 (
    //                     next,
    //                     if *node != next {
    //                         let source = self.map.get_position(node).unwrap();
    //                         let target = self.map.get_position(&next).unwrap();
    //                         // In centimeters
    //                         (Haversine::distance(source, target) * 1_000f64) as u32
    //                     } else {
    //                         0u32
    //                     }, // Total accrued distance
    //                 )
    //             })
    //             .collect::<Vec<_>>()
    //     });
    //
    //     let probabilities = reach
    //         .take_while(|p| {
    //             (distance_to_end_of_edge + (p.total_cost as f64 / 1_000f64)) < threshold_distance
    //         })
    //         .map(|predicate| {
    //             (
    //                 predicate.clone(),
    //                 Haversine::distance(
    //                     left_candidate.position,
    //                     self.map.get_position(&predicate.node).unwrap(),
    //                 ),
    //             )
    //         })
    //         // .take_while(|(_, distance)| *distance < threshold_distance)
    //         .map(|(k, shortest_distance)| {
    //             (
    //                 k.node,
    //                 (
    //                     // Invalid position so the build_path function
    //                     // will terminate as the found call will return None
    //                     k.parent.unwrap_or(OsmEntryId::null()),
    //                     TransitionPair {
    //                         shortest_distance,
    //                         path_length: distance_to_end_of_edge + (k.total_cost as f64 / 1_000f64),
    //                     },
    //                 ),
    //             )
    //         })
    //         .collect::<StandardHashMap<OsmEntryId, (OsmEntryId, TransitionPair<f64>)>>();
    //
    //     let paths = right_ixs
    //         .iter()
    //         .filter_map(|source| {
    //             self.candidates.get(source).and_then(|candidate| {
    //                 let start_to_inner_position = Haversine::distance(
    //                     self.map.get_position(&candidate.map_edge.0).unwrap(),
    //                     candidate.position,
    //                 );
    //
    //                 probabilities
    //                     .get(&candidate.map_edge.0)
    //                     .map(|(_parent, prob)| {
    //                         // Refactor but this is to add the cost of entering into the edge to trial
    //                         return (
    //                             source,
    //                             (
    //                                 candidate.map_edge.0,
    //                                 TransitionPair {
    //                                     shortest_distance: prob.shortest_distance,
    //                                     path_length: prob.path_length + start_to_inner_position,
    //                                 },
    //                             ),
    //                         );
    //                     })
    //             })
    //         })
    //         .map(|(key, (to, pair))| (key, to, pair, Self::pathbuilder(&to, &probabilities)))
    //         .map(|(right, target, lengths, path)| {
    //             let trip = self.map.resolve_line(path.as_slice());
    //
    //             let cost = self.heuristics.transition(TransitionContext {
    //                 optimal_path: Trip::from(trip),
    //                 target_candidate: target,
    //                 source_candidate: right,
    //                 routing_context: todo!(),
    //             });
    //
    //             (*right, cost, path)
    //         })
    //         .collect::<Vec<(NodeIndex, f64, Vec<OsmEntryId>)>>();
    //
    //     // debug!(
    //     //     "TIMING: Full={} ({} -> *)",
    //     //     time.elapsed().as_micros(),
    //     //     left_candidate.position.wkt_string(),
    //     // );
    //
    //     paths
    // }
    //
    // pub fn generate_probabilities(mut self, distance: f64) -> Self {
    //     // Deconstruct the trajectory into individual segments
    //     self.layers = self.generate_layers(distance);
    //
    //     info!("Layer Generation Complete!");
    //
    //     // let mut collection: Vec<String> = vec![];
    //     // self.layers.iter().for_each(|layer| {
    //     //     layer.iter().for_each(|node| {
    //     //         let id = OsmEntryId::as_node(node.index() as i64);
    //     //         let graph = self.graph.read().unwrap();
    //     //         let position = graph.node_weight(*node);
    //     //         // .get_position(&OsmEntryId::as_node(node.index() as i64));
    //
    //     //         if let Some((point, _)) = position {
    //     //             collection.push(point.wkt_string());
    //     //         } else {
    //     //             warn!("Could not resolve location for entry at point {:?}", id);
    //     //         }
    //     //     });
    //     // });
    //
    //     // debug!("GEOMETRYCOLLECTION ( {} )", collection.join(", "));
    //
    //     // Declaring all the pairs of indices that need to be refined.
    //     let transition_probabilities = self
    //         .layers
    //         .par_windows(2)
    //         .enumerate()
    //         .inspect(|(index, pair)| {
    //             debug!("Unpacking ({index}) {:?} and {:?}...", pair[0].len(), pair[1].len());
    //         })
    //         .flat_map(|(index, vectors)| {
    //             // Taking all forward pairs of (left, [...right]) such that
    //             // ...
    //             let output = vectors[0]
    //                 .iter()
    //                 .map(|&a| (a, vectors[1].as_slice()))
    //                 .collect::<Vec<_>>();
    //
    //             if output.is_empty() {
    //                 warn!("Could not find any routing for layer transition ({index}) from {} to {} nodes", vectors[0].len(), vectors[1].len())
    //             }
    //
    //             output
    //         })
    //         .map(|(left, right)| (left, self.refine_candidates(left, right)))
    //         // .into_par_iter()
    //         // .map(|(left, right)| {
    //         //     let candidates = ;
    //         //     if candidates.is_empty() {
    //         //         warn!("No candidates found for {:?}", left);
    //         //     }
    //         //
    //         //     (left, candidates)
    //         // })
    //         .collect::<Vec<_>>();
    //
    //     let mut write_access = self.graph.write().unwrap();
    //
    //     for (left, weights) in transition_probabilities {
    //         for (right, weight, path) in weights {
    //             // Transition Probabilities are of the range {0, 1}, such that
    //             // -log_N(x) yields a value which is infinitely large when x
    //             // is close to 0, and 0 when x is close to 1. This is the desired
    //             // behaviour since a value with a Transition Probability of 1,
    //             // represents a transition which is most desirable. Therefore, has
    //             // "zero" cost to move through. Whereas, one with a Transition
    //             // Probability of 0, is a transition we want to discourage, thus
    //             // has a +inf cost to traverse.
    //             //
    //             // t_p(x) = -log_N(x) in the range { 0 <= x <= 1 }
    //             // let transition_cost = -weight.deref().log(TRANSITION_LOGARITHM_BASE);
    //
    //             // debug!("TP {} for path {}", weight, path);
    //             write_access.add_edge(left, right, (weight, path));
    //         }
    //     }
    //
    //     // let mut output = File::create("./output.viz").expect("Missing output file");
    //     // write!(output, "{:?}", Dot::with_config(&write_access.deref(), &[Config::EdgeNoLabel])).expect("Could not write");
    //
    //     drop(write_access);
    //
    //     self
    // }

    /// Backtracks the [HMM] from the most appropriate final point to
    /// its prior most appropriate points
    ///
    /// Consumes the transition structure in doing so.
    /// This is because it makes irreversible modifications
    /// to the candidate graph that put it in a collapsable position.
    ///
    /// [HMM]: Hidden Markov Model
    pub fn collapse(self) -> Result<Collapse, MatchError> {
        // Use the candidates to collapse the graph into a single route.
        let collapse = self
            .candidates
            .collapse(&self.layers)
            .ok_or_else(|| MatchError::CollapseFailure)?;

        debug!("Collapsed with final cost: {}", collapse.cost);
        Ok(collapse)
    }
}
