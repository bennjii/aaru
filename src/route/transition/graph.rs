use std::collections::HashMap as StandardHashMap;
use std::f64::consts::E;
use std::fs::File;
use std::hash::Hash;
use std::io::Write;
use std::ops::{Deref, Div, Mul};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use geo::{
    point, Distance, Euclidean, GeometryCollection, HasDimensions, Haversine, HaversineDistance,
    Length, LineString, MultiLineString, MultiPoint, Point, Relate, Simplify, SimplifyVw,
};
use log::{debug, error, info};
use pathfinding::prelude::{
    build_path, dijkstra_partial, dijkstra_reach, DijkstraReachable, DijkstraReachableItem,
};
use petgraph::graph::NodeIndex;
use petgraph::{Directed, Direction, Graph};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
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
struct TransitionPair<K>
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

pub struct Transition<'a> {
    map: &'a RouteGraph,
    // keep in mind we dont need layerid and nodeid, but theyre useful for debugging so we'll keep for now.
    graph: Arc<RwLock<Graph<Point, f64, Directed>>>,
    candidates: HashMap<NodeIndex, (LayerId, NodeId, TransitionCandidate)>,
    error: Option<f64>,
}

impl<'a> Transition<'a> {
    pub fn new(map: &'a RouteGraph) -> Self {
        Transition {
            map,
            graph: Arc::new(RwLock::new(Graph::new())),
            candidates: HashMap::new(),
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
    pub(crate) fn context_aware_pathbuilder<N, C>(
        target: &N,
        parents: &StandardHashMap<N, (N, C)>,
    ) -> Option<Vec<N>>
    where
        N: Eq + Hash + Copy,
    {
        let mut rev = vec![*target];
        let mut next = target;
        while let Some((parent, _)) = parents.get(next) {
            if rev.contains(parent) {
                return None;
            }

            rev.push(*parent);
            next = parent;
        }
        rev.reverse();
        Some(rev)
    }

    /// Generates the layers of the transition graph, where each layer
    /// represents a point in the linestring, and each node in the layer
    /// represents a candidate transition point, within the `distance`
    /// search radius of the linestring point, which was found by the
    /// projection of the linestring point upon the closest edges within this radius.
    pub fn generate_layers(&self, linestring: Vec<Point>, distance: f64) -> Vec<Vec<NodeIndex>> {
        // Create the base transition graph
        linestring
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

                        let node_index = self.graph.write().unwrap().add_node(position);
                        let _ = self
                            .candidates
                            .insert(node_index, (layer_id, node_id, candidate));

                        node_index
                    })
                    .collect::<Vec<NodeIndex>>()
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
        let left_candidate = self.candidates.get(&left_ix).unwrap().clone();
        let left = left_candidate.2;

        debug!(
            "Routing from Layer::{}::{} to Layer::{}::*.",
            left_candidate.0,
            left_candidate.1,
            left_candidate.0 + 1,
        );

        let time = Instant::now();
        let start = left.map_edge.0;
        let threshold_distance = 3_000;

        let reach = dijkstra_reach(&start, |node, a| {
            self.map
                .graph
                .edges_directed(*node, Direction::Outgoing)
                .map(|(_, next, w)| {
                    (
                        next,
                        // (a as i64) + ...
                        *w, // Haversine::distance(
                           //     self.map.hash.get(node).unwrap().position,
                           //     self.map.hash.get(&next).unwrap().position,
                           // ) as u32, // Total accrued distance
                    )
                })
                .collect::<Vec<_>>()
        });

        debug!("Created reach.");

        // WHEN NO LONGER NEEDING PATHS, THIS CAN BE GREATLY SHORTENED (INLINED)
        //

        let probabilities = reach
            .map(|predicate| {
                (
                    predicate.clone(),
                    Haversine::distance(
                        left.position,
                        self.map.hash.get(&predicate.node).unwrap().position,
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

        let mut vector = vec![];

        vector.push((
            self.map.hash.get(&left.map_edge.0).unwrap().position,
            vec![],
        ));

        probabilities.iter().for_each(|(_, (key, _))| {
            vector.push((
                self.map
                    .hash
                    .get(key)
                    .map(|e| e.position)
                    .unwrap_or(point! {x: 0.0, y: 0.0}),
                build_path(key, &probabilities),
            ));
        });

        // END OF DEBUG: START PROCESSING...

        let paths = right_ixs
            .into_iter()
            .filter_map(|key| {
                self.candidates.get(key).and_then(|candidate| {
                    probabilities
                        .get(&candidate.2.map_edge.0)
                        .map(|(_parent, prob)| (key, (candidate.2.map_edge.0, prob)))
                })
            })
            .filter_map(|(key, (to, pair))| {
                let path = Transition::context_aware_pathbuilder(&to, &probabilities);
                Some((key, pair, path?))
            })
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

        // debug!(
        //     "TIMING: DijkstraPartial=@{} (Size={})",
        //     time.elapsed().as_micros(),
        //     parents.len()
        // );

        // let tp = right_ixs
        //     .iter()
        //     .map(|target| {
        //         (
        //             *target,
        //             self.candidates.get(target).map(|candidate| {
        //                 let target = candidate.2;
        //                 if !parents.contains_key(&target.map_edge.0) {
        //                     // debug!("Doesnt contain the key.");
        //                     return (0.0f64, vec![]);
        //                 }

        //                 // debug!("DOES contain the key.");

        //                 let time = Instant::now();
        //                 let path = build_path(&target.map_edge.0, &parents);
        //                 // debug!(
        //                 //     "TIMING: Route::{}=@{}",
        //                 //     candidate.1,
        //                 //     time.elapsed().as_micros()
        //                 // );

        //                 let direct_distance =
        //                     Haversine::distance(left.position, candidate.2.position);
        //                 let travel_distance = path
        //                     .iter()
        //                     .filter_map(|index| self.map.hash.get(index))
        //                     .map(|node| node.position)
        //                     .collect::<LineString>()
        //                     .length::<Haversine>();

        //                 // debug!(
        //                 //     "TIMING: Distance::{}=@{}",
        //                 //     candidate.1,
        //                 //     time.elapsed().as_micros()
        //                 // );
        //                 //
        //                 // debug!(
        //                 // "Direct={}. Travel={}. StartNode={start}",
        //                 // direct_distance, travel_distance
        //                 // );
        //                 let p =
        //                     Transition::transition_probability(travel_distance, direct_distance);

        //                 (p, path)
        //             }),
        //         )
        //     })
        //     // .inspect(|element| debug!("=> {:?}", element))
        //     .collect::<Vec<_>>();

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

        let graph = self.graph.read().unwrap();
        if let Some((score, path)) =
            petgraph::algo::astar(&*graph, start, |node| node == end, |e| *e.weight(), |_| 0.0)
        {
            debug!("Minimal trip found with score of {score}.");
            return path;
        }

        // Return no points by default
        debug!("Insufficient layers or no optimal candidate, empty vec.");
        vec![]
    }

    /// Backtracks the HMM from the most appropriate final point to
    /// its prior most appropriate points
    ///
    /// Consumes the graph.
    pub fn backtrack(self, linestring: LineString, distance: f64) -> Vec<TransitionCandidate> {
        let time = Instant::now();
        debug!("BACKTRACK::TIMING:: Start=@{}", time.elapsed().as_micros());

        let as_points = linestring.into_points();
        let start = *as_points.first().unwrap();
        let end = *as_points.last().unwrap();

        // Deconstruct the trajectory into individual segments
        let layers = self.generate_layers(as_points, distance);
        debug!(
            "BACKTRACK::TIMING:: GeneratedPoints=@{}",
            time.elapsed().as_micros()
        );

        let all_points = layers
            .iter()
            .flat_map(|candidates| {
                candidates.iter().filter_map(|candidate| {
                    self.candidates.get(candidate).map(|node| node.2.position)
                })
            })
            .collect::<MultiPoint>();

        info!("WKT(PTS)={}", all_points.wkt_string());

        // Add in the start and end points to the graph
        let (source, target) = {
            let mut graph = self.graph.write().unwrap();
            let source = graph.add_node(start);
            layers.first().unwrap().iter().for_each(|node| {
                graph.add_edge(source, *node, 0.0f64);
            });

            let target = graph.add_node(end);
            layers.last().unwrap().iter().for_each(|node| {
                graph.add_edge(*node, target, 0.0f64);
            });

            drop(graph);
            (source, target)
        };

        info!("Identified {} layers to backtrack.", layers.len());
        let total_comp = layers
            .windows(2)
            .fold(0, |acc, layers| acc + (layers[0].len() * layers[1].len()));

        info!(
            "Performing a total of {} routes (Avg. 300us = {}us = {}ms - In series). Breakdown:",
            total_comp,
            total_comp * 300,
            total_comp * 300 / 1000
        );
        for (index, layer) in layers.iter().enumerate() {
            info!("Layer::{} has {} nodes.", index, layer.len());
        }

        // Declaring all the pairs of indicies that need to be refined.
        //
        // TODO: Consider how to parallise it too...
        let transition_probabilities = layers
            .par_windows(2)
            .inspect(|pair| {
                debug!("Unpacking {:?} and {:?}...", pair[0].len(), pair[1].len());
            })
            .flat_map(|vectors| {
                vectors[0]
                    .iter()
                    .map(|&a| (a, vectors[1].as_slice()))
                    // .flat_map(|&a| vectors[1].iter().map(move |&b| (a, b)))
                    .collect::<Vec<_>>()
            })
            .map(|(left, right)| {
                // debug!("Refining between {:?} and {:?}...", left, right);
                (left, self.refine_candidates(left, right))
            })
            .collect::<Vec<_>>();

        let all_lines = transition_probabilities
            .iter()
            .flat_map(|(candidate, vec)| {
                vec.iter()
                    .map(|(_, _, nix)| {
                        nix.iter()
                            .filter_map(|n| {
                                self.map.hash.get(n).map(|k| k.position).or_else(|| {
                                    debug!("!!: Can't find key n={n}");
                                    None
                                })
                            })
                            .collect::<LineString>()
                    })
                    .filter(|line| !line.is_empty())
            })
            .collect::<MultiLineString>();

        // File::create("output.txt")
        //     .unwrap()
        //     .write_all(all_lines.wkt_string().as_bytes())
        //     .unwrap();

        info!("WKT(LS)={}", all_lines.wkt_string());

        let route_size = transition_probabilities.len();
        let mut map: HashMap<(NodeIndex, NodeIndex), (TransitionProbability, Vec<NodeIx>)> =
            HashMap::new();
        for (left, weights) in transition_probabilities {
            for (right, weight, path) in weights {
                map.insert((left, right), (weight, path))
                    .expect("TODO: panic message");
                // debug!(
                //     "Refined transition between {:?} and {:?} with TP(Weight)={:?}",
                //     left, right, weight
                // );

                self.graph
                    .write()
                    .unwrap()
                    .add_edge(left, right, 1f64 - weight.deref());
            }
        }
        info!("Made compute for {} total routings.", route_size);

        debug!(
            "BACKTRACK::TIMING:: RefinedLayers=@{}",
            time.elapsed().as_micros()
        );

        debug!("Refined all layers, collapsing...");

        // Now we refine the candidates
        let collapsed = self.collapse(source, target);
        debug!(
            "BACKTRACK::TIMING:: Collapsed=@{}",
            time.elapsed().as_micros()
        );

        let total_linestring = collapsed
            .windows(2)
            .filter_map(|candidate| {
                if let [a, b] = candidate {
                    return map.get(&(*a, *b)).or_else(|| map.get(&(*b, *a))).map(|pp| {
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

        debug!("Total Route: {}", total_linestring.wkt_string());

        collapsed
            .iter()
            .filter_map(|node| self.candidates.get(node))
            .map(|can| can.2)
            .collect::<Vec<_>>()
    }
}
