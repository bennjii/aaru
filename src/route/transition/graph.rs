use std::collections::HashMap as StandardHashMap;
use std::hash::Hash;
use std::ops::{Deref, Div, Mul};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use geo::{Distance, Haversine, LineString, Point};
use log::debug;
use pathfinding::prelude::dijkstra_reach;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Direction, Graph};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};
use rayon::slice::ParallelSlice;
use scc::HashMap;
use wkt::ToWkt;

use crate::codec::element::variants::common::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::node::TransitionCandidate;
use crate::route::Graph as RouteGraph;

const DEFAULT_ERROR: f64 = 20f64;
const TRANSITION_LOGARITHM_BASE: f64 = 1.2;

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

/// A 0.0-1.0 probability of transitioning from one node to another
#[derive(Clone, Copy, Debug)]
pub struct TransitionProbability(pub(crate) f64);

impl Deref for TransitionProbability {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

type CandidateEdge = (f64, Vec<NodeIx>);
type CandidatePoint = (Point, f64);

type ArcGraph<A, B> = Arc<RwLock<Graph<A, B, Directed>>>;

pub struct Transition<'a> {
    map: &'a RouteGraph,
    // keep in mind we dont need layerid and nodeid, but theyre useful for debugging so we'll keep for now.
    graph: ArcGraph<CandidatePoint, CandidateEdge>,
    candidates: HashMap<NodeIndex, TransitionCandidate>,
    error: Option<f64>,

    layers: Vec<Vec<NodeIndex>>,
    points: Vec<Point>,
}

#[derive(Debug)]
pub enum MatchError {
    CollapseFailure,
}

pub struct Match {
    pub cost: f64,
    pub interpolated: LineString,
    pub matched: Vec<TransitionCandidate>,
}

impl<'a> Transition<'a> {
    pub fn new(map: &'a RouteGraph, linestring: LineString) -> Self {
        let points = linestring.into_points();

        Transition {
            map,
            points,

            graph: Arc::new(RwLock::new(Graph::new())),
            candidates: HashMap::new(),
            error: None,
            layers: vec![],
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
    pub(crate) fn emission_probability<K: Into<f64>, T: Div<Output = K> + Mul>(
        dist: T,
        error: T,
    ) -> f64 {
        let alpha: f64 = (dist / error).into();
        0.1f64 * alpha.powi(2)
    }

    /// Calculates the transition probability of between two candidates
    /// given the `shortest_dist` between them, and the `euclidean_dist`
    /// of the segment we are transitioning over
    pub fn transition_probability<K: Into<f64> + Div<Output = K> + PartialOrd>(
        pair: TransitionPair<K>,
    ) -> TransitionProbability {
        match pair.shortest_distance >= pair.path_length {
            true => TransitionProbability(1.0f64),
            false => TransitionProbability((pair.shortest_distance / pair.path_length).into()),
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

    /// Generates the layers of the transition graph, where each layer
    /// represents a point in the linestring, and each node in the layer
    /// represents a candidate transition point, within the `distance`
    /// search radius of the linestring point, which was found by the
    /// projection of the linestring point upon the closest edges within this radius.
    pub fn generate_layers(&self, distance: f64) -> Vec<Vec<NodeIndex>> {
        // Create the base transition graph
        self.points
            .par_iter()
            .enumerate()
            .map(|(layer_id, point)| {
                self.map
                    // We'll do a best-effort search (square) radius
                    .nearest_projected_nodes(point, distance)
                    .enumerate()
                    .map(|(node_id, (position, map_edge))| {
                        // We have the actual projected position, and it's associated edge.
                        let distance = Haversine::distance(position, *point);
                        let emission_probability = Transition::emission_probability(
                            distance,
                            self.error.unwrap_or(DEFAULT_ERROR),
                        );

                        let candidate = TransitionCandidate {
                            map_edge: (map_edge.0, map_edge.1),
                            position,
                            layer_id,
                            node_id,
                        };

                        let node_index = self
                            .graph
                            .write()
                            .unwrap()
                            .add_node((position, emission_probability));
                        let _ = self.candidates.insert(node_index, candidate);

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
    ) -> Vec<(NodeIndex, TransitionProbability, Vec<OsmEntryId>)> {
        let left_candidate = *self.candidates.get(&left_ix).unwrap();

        debug!(
            "Routing from Layer::{}::{} to Layer::{}::*.",
            left_candidate.layer_id,
            left_candidate.node_id,
            left_candidate.layer_id + 1,
        );

        let time = Instant::now();
        let start = left_candidate.map_edge.0;
        let threshold_distance = 1_000;

        let reach = dijkstra_reach(&start, |node, _| {
            self.map
                .graph
                .edges_directed(*node, Direction::Outgoing)
                .map(|(_, next, _w)| {
                    (
                        next,
                        if *node != next {
                            let source = self.map.get_position(node).unwrap();
                            let target = self.map.get_position(&next).unwrap();
                            Haversine::distance(source, target) as u32
                        } else {
                            0
                        }, // Total accrued distance
                    )
                })
                .collect::<Vec<_>>()
        });

        let probabilities = reach
            .map(|predicate| {
                (
                    predicate.clone(),
                    Haversine::distance(
                        left_candidate.position,
                        self.map.get_position(&predicate.node).unwrap(),
                    ) as u32,
                )
            })
            .take_while(|(_, distance)| *distance < threshold_distance)
            .map(|(k, j)| {
                (
                    k.node,
                    (
                        // Invalid position so the build_path function
                        // will terminate as the found call will return None
                        k.parent.unwrap_or(OsmEntryId::null()),
                        TransitionPair {
                            shortest_distance: j as f64,
                            path_length: k.total_cost as f64,
                        },
                    ),
                )
            })
            .collect::<StandardHashMap<OsmEntryId, (OsmEntryId, TransitionPair<f64>)>>();

        debug!(
            "Generated {} parent possibilities to pair with.",
            probabilities.len()
        );

        let paths = right_ixs
            .iter()
            .filter_map(|key| {
                self.candidates.get(key).and_then(|candidate| {
                    probabilities
                        .get(&candidate.map_edge.0)
                        .map(|(_parent, prob)| (key, (candidate.map_edge.0, prob)))
                })
            })
            .map(|(key, (to, pair))| (key, pair, Transition::pathbuilder(&to, &probabilities)))
            .map(|(right, lengths, path)| {
                (*right, Transition::transition_probability(*lengths), path)
            })
            .collect::<Vec<(NodeIndex, TransitionProbability, Vec<OsmEntryId>)>>();

        debug!(
            "TIMING: Full={} ({} -> *)",
            time.elapsed().as_micros(),
            left_candidate.position.wkt_string(),
        );

        paths
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, start: NodeIndex, end: NodeIndex) -> Option<(f64, Vec<NodeIndex>)> {
        // There should be exclusive read-access by the time collapse is called.
        let graph = self.graph.read().unwrap();
        petgraph::algo::astar(
            &*graph,
            start,
            |node| node == end,
            |e| {
                // Decaying Transition Cost
                let transition_cost = e.weight().0;

                // Loosely-Decaying Emission Cost
                let emission_cost = graph.node_weight(e.source()).map_or(0.0, |v| v.1);

                transition_cost + emission_cost
            },
            |_| 0.0,
        )
    }

    pub fn generate_probabilities(mut self, distance: f64) -> Self {
        // Deconstruct the trajectory into individual segments
        self.layers = self.generate_layers(distance);

        // Declaring all the pairs of indicies that need to be refined.
        let transition_probabilities = self
            .layers
            .par_windows(2)
            .inspect(|pair| {
                debug!("Unpacking {:?} and {:?}...", pair[0].len(), pair[1].len());
            })
            .flat_map(|vectors| {
                vectors[0]
                    .iter()
                    .map(|&a| (a, vectors[1].as_slice()))
                    .collect::<Vec<_>>()
            })
            .into_par_iter()
            .map(|(left, right)| (left, self.refine_candidates(left, right)))
            .collect::<Vec<_>>();

        let mut write_access = self.graph.write().unwrap();

        for (left, weights) in transition_probabilities {
            for (right, weight, path) in weights {
                // Transition Probabilities are of the range {0, 1}, such that
                // -log_N(x) yields a value which is infinately large when x
                // is close to 0, and 0 when x is close to 1. This is the desired
                // behaviour since a value with a Transition Probability of 1,
                // represents a transition which is most desirable. Therefore, has
                // "zero" cost to move through. Whereas, one with a Transition
                // Probability of 0, is a transition we want to discourage, thus
                // has a +inf cost to traverse.
                //
                // t_p(x) = -log_N(x) in the range { 0 <= x <= 1 }
                let transition_cost = -weight.deref().log(TRANSITION_LOGARITHM_BASE);
                write_access.add_edge(left, right, (transition_cost, path));
            }
        }

        drop(write_access);

        self
    }

    /// Backtracks the HMM from the most appropriate final point to
    /// its prior most appropriate points
    ///
    /// Consumes the graph.
    pub fn backtrack(self) -> Result<Match, MatchError> {
        let start = *self.points.first().unwrap();
        let end = *self.points.last().unwrap();

        // Add in the start and end points to the graph
        let (source, target) = {
            let mut graph = self.graph.write().unwrap();
            let source = graph.add_node((start, 0.0));
            self.layers.first().unwrap().iter().for_each(|node| {
                // TODO: Add the starting NodeIx in
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
                        pp.1.iter()
                            .filter_map(|index| self.map.hash.get(index))
                            .map(|node| node.position)
                            .collect::<Vec<_>>()
                    });
                }

                None
            })
            .flatten()
            .collect::<LineString>();

        debug!("Total Route: {}", interpolated.wkt_string());

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
