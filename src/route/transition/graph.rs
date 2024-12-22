use std::collections::HashMap as StandardHashMap;
use std::f64::consts::E;
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

use crate::route::graph::NodeIx;
use crate::route::transition::node::TransitionCandidate;
use crate::route::Graph as RouteGraph;

const DEFAULT_ERROR: f64 = 20f64;

// NEW API:
//
// + === + === +
//   \        /
//    \     /
//       +
//
// We need a way to represent the graph as a whole.
// We will use petgraph's DiGraph for this,
// since it will be a directed graph, built in reverse, tracking backwards.
//
// We must first add all the nodes into the graph,
// then we can add the edges with their weights in
// the transition.
//
// Finally, to backtrack, we can use the provided
// algorithms to perform dijkstra on it to backtrack.
// possibly just using the astar algo since the
// api is easier to use.
//

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

// TODO: Move LayerID and NodeID into TransitionCandidate.

pub struct Transition<'a> {
    map: &'a RouteGraph,
    // keep in mind we dont need layerid and nodeid, but theyre useful for debugging so we'll keep for now.
    graph: ArcGraph<CandidatePoint, CandidateEdge>,
    candidates: HashMap<NodeIndex, (LayerId, NodeId, TransitionCandidate)>,
    error: Option<f64>,

    layers: Vec<Vec<NodeIndex>>,
    points: Vec<Point>,
}

pub struct MatchResult {
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
        E.powf(-0.5f64 * alpha.powi(2))
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
                            emission_probability,
                        };

                        let node_index = self
                            .graph
                            .write()
                            .unwrap()
                            .add_node((position, emission_probability));
                        let _ = self
                            .candidates
                            .insert(node_index, (layer_id, node_id, candidate));

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
    ) -> Vec<(NodeIndex, TransitionProbability, Vec<i64>)> {
        let left_candidate = *self.candidates.get(&left_ix).unwrap();
        let left = left_candidate.2;

        debug!(
            "Routing from Layer::{}::{} to Layer::{}::*.",
            left_candidate.0,
            left_candidate.1,
            left_candidate.0 + 1,
        );

        let time = Instant::now();
        let start = left.map_edge.0;
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
                        left.position,
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
                        k.parent.unwrap_or(-1),
                        TransitionPair {
                            shortest_distance: j as f64,
                            path_length: k.total_cost as f64,
                        },
                    ),
                )
            })
            .collect::<StandardHashMap<i64, (i64, TransitionPair<f64>)>>();

        debug!(
            "Generated {} parent possibilities to pair with.",
            probabilities.len()
        );

        // let mut vector = vec![];

        // vector.push((
        //     self.map.hash.get(&left.map_edge.0).unwrap().position,
        //     vec![],
        // ));

        // probabilities.iter().for_each(|(_, (key, _))| {
        //     vector.push((
        //         self.map
        //             .hash
        //             .get(key)
        //             .map(|e| e.position)
        //             .unwrap_or(point! {x: 0.0, y: 0.0}),
        //         build_path(key, &probabilities),
        //     ));
        // });

        // END OF DEBUG: START PROCESSING...

        let paths = right_ixs
            .iter()
            .filter_map(|key| {
                self.candidates.get(key).and_then(|candidate| {
                    probabilities
                        .get(&candidate.2.map_edge.0)
                        .map(|(_parent, prob)| (key, (candidate.2.map_edge.0, prob)))
                })
            })
            .map(|(key, (to, pair))| (key, pair, Transition::pathbuilder(&to, &probabilities)))
            .map(|(right, lengths, path)| {
                (*right, Transition::transition_probability(*lengths), path)
            })
            .collect::<Vec<(NodeIndex, TransitionProbability, Vec<i64>)>>();

        // let (points, lines): (Vec<Point>, Vec<Vec<i64>>) = vector.into_iter().unzip();
        // debug!(
        //     "OutgoingLines=\nGEOMETRYCOLLECTION({}, {})",
        //     points.into_iter().collect::<MultiPoint>().wkt_string(),
        //     lines
        //         .iter()
        //         .map(|path| {
        //             path.into_iter()
        //                 .filter_map(|index| self.map.hash.get(index).map(|p| p.position))
        //                 .filter(|line| !line.is_empty())
        //                 .collect::<LineString>()
        //         })
        //         .collect::<MultiLineString>()
        //         .wkt_string()
        // );

        debug!(
            "Success rate of {}/{}={}",
            paths.len(),
            probabilities.len(),
            paths.len() as f64 / probabilities.len() as f64
        );

        debug!(
            "TIMING: Full={} ({} -> *)",
            time.elapsed().as_micros(),
            left.position.wkt_string(),
        );

        paths
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points
    fn collapse(&self, start: NodeIndex, end: NodeIndex) -> Vec<NodeIndex> {
        // Use the internal graph to backtrack using dijkstras.
        // Make sure to add the start and end positions which are used
        // to trace between as the target end-position. We're essentially
        // looking for the shortest path through the graph, using the
        // edge weighting as a consideration of the transition probability,
        // determined in `refine_candidates`.
        //

        // There should be exclusive read-access by the time collapse is called.
        let graph = self.graph.read().unwrap();
        if let Some((score, path)) = petgraph::algo::astar(
            &*graph,
            start,
            |node| node == end,
            |e| e.weight().0 + graph.node_weight(e.target()).map_or(0.0, |v| v.1),
            |_| 0.0,
        ) {
            debug!("Minimal trip found with score of {score}.");
            return path;
        }

        // Return no points by default
        debug!("Insufficient layers or no optimal candidate, empty vec.");
        vec![]
    }

    pub fn generate_probabilities(self, distance: f64) -> Self {
        // Deconstruct the trajectory into individual segments
        let layers = self.generate_layers(distance);

        // Declaring all the pairs of indicies that need to be refined.
        let transition_probabilities = layers
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

        for (left, weights) in transition_probabilities {
            for (right, weight, path) in weights {
                let normalised_ep = self
                    .candidates
                    .get(&left)
                    .unwrap()
                    .2
                    .emission_probability
                    .log10();

                let normalised_tp = -weight.deref().log10();

                self.graph.write().unwrap().add_edge(
                    left,
                    right,
                    (normalised_ep + normalised_tp, path),
                );
            }
        }

        self
    }

    /// Backtracks the HMM from the most appropriate final point to
    /// its prior most appropriate points
    ///
    /// Consumes the graph.
    pub fn backtrack(self) -> MatchResult {
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
        let collapsed = self.collapse(source, target);

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
            .map(|can| can.2)
            .collect::<Vec<_>>();

        MatchResult {
            interpolated,
            matched,
        }
    }
}
