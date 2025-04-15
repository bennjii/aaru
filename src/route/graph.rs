use super::transition::graph::MatchError;
use crate::codec::element::item::ProcessedElement;
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::element::variants::common::OsmEntryId;
use crate::codec::element::variants::Node;
use crate::codec::parallel::Parallel;
use crate::route::error::RouteError;
use crate::route::transition::candidate::Collapse;
use crate::route::transition::graph::Transition;
use crate::route::transition::{CostingStrategies, DirectionAwareEdgeId, SelectiveForwardSolver};
use crate::route::Scan;

use geo::{LineString, Point};
use log::{debug, info};
use petgraph::prelude::DiGraphMap;
use petgraph::visit::EdgeRef;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rstar::RTree;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use std::time::Instant;
#[cfg(feature = "tracing")]
use tracing::Level;

pub type Weight = u32;

// TODO: Convert `type X = Y` to `struct X(Y)` for type enforcement. (TypeName pattern)
pub type NodeIx = OsmEntryId;
pub type EdgeIx = OsmEntryId;

pub type Edge<'a> = (NodeIx, NodeIx, &'a Weight);

pub type GraphStructure = DiGraphMap<NodeIx, (Weight, DirectionAwareEdgeId)>;

const MAX_WEIGHT: Weight = 255 as Weight;

/// Routing graph, can be ingested from an `.osm.pbf` file,
/// and can be actioned upon using `route(start, end)`.
pub struct Graph {
    pub(crate) graph: GraphStructure,
    pub(crate) index: RTree<Node>,
    pub(crate) hash: RwLock<HashMap<NodeIx, Node>>,
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            graph: GraphStructure::default(),
            index: RTree::default(),
            hash: RwLock::new(HashMap::new()),
        }
    }
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph with Nodes: {}", self.hash.read().unwrap().len())
    }
}

impl Graph {
    pub fn index(&self) -> &RTree<Node> {
        &self.index
    }

    pub fn size(&self) -> usize {
        self.hash.read().unwrap().len()
    }

    #[inline]
    pub fn get_position(&self, node_index: &NodeIx) -> Option<Point<f64>> {
        self.hash
            .read()
            .ok()?
            .get(node_index)
            .map(|point| point.position)
    }

    pub fn resolve_line(&self, node_index: &[NodeIx]) -> Vec<Point<f64>> {
        node_index
            .iter()
            .filter_map(|node| self.get_position(node))
            .collect::<Vec<_>>()
    }

    /// The weighting mapping of node keys to weight.
    pub fn weights<'a>() -> Result<HashMap<&'a str, Weight>, RouteError> {
        let mut weights: HashMap<&str, Weight> = HashMap::new();

        // TODO: Base this dynamically on geospacial properties and roading shape

        weights.insert("motorway", 1);
        weights.insert("motorway_link", 2);
        weights.insert("trunk", 3);
        weights.insert("trunk_link", 4);
        weights.insert("primary", 5);
        weights.insert("primary_link", 6);
        weights.insert("secondary", 7);
        weights.insert("secondary_link", 8);
        weights.insert("tertiary", 9);
        weights.insert("tertiary_link", 10);
        weights.insert("unclassified", 11);
        weights.insert("residential", 12);
        weights.insert("living_street", 13);

        Ok(weights)
    }

    /// Creates a graph from a `.osm.pbf` file, using the `ProcessedElementIterator`
    pub fn new(filename: std::ffi::OsString) -> crate::Result<Graph> {
        let mut start_time = Instant::now();
        let fixed_start_time = Instant::now();

        let path = PathBuf::from(filename);

        let reader = ProcessedElementIterator::new(path)?;
        let weights = Graph::weights()?;

        debug!("Iterator warming took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        info!("Ingesting...");

        let global_graph = Mutex::new(GraphStructure::new());
        let index: Vec<Node> = reader.par_red(
            |mut tree: Vec<Node>, element: ProcessedElement| {
                match element {
                    ProcessedElement::Way(way) => {
                        // If way is not traversable (/ is not road)
                        if way.tags().road_tag().is_none() {
                            return tree;
                        }

                        // Get the weight from the weight table
                        let weight = match way.tags().road_tag() {
                            Some(weight) => weights.get(weight).copied().unwrap_or(MAX_WEIGHT),
                            None => MAX_WEIGHT,
                        };

                        let bidirectional = !way.tags().unidirectional();

                        // Update with all adjacent nodes
                        way.refs().windows(2).for_each(|edge| {
                            if let [a, b] = edge {
                                let direction_aware = DirectionAwareEdgeId::new(way.id());
                                let mut lock = global_graph.lock().unwrap();

                                lock.add_edge(a.id, b.id, (weight, direction_aware.forward()));

                                // If way is bidi, add opposite edge with a DirAw backward.
                                if bidirectional {
                                    lock.add_edge(b.id, a.id, (weight, direction_aware.backward()));
                                }
                            } else {
                                debug!("Edge windowing produced odd-sized entry: {:?}", edge);
                            }
                        });
                    }
                    ProcessedElement::Node(node) => {
                        // Add the node to the graph
                        tree.push(node);
                    }
                    _ => {}
                }

                tree
            },
            |mut a_tree, b_tree| {
                a_tree.extend(b_tree);
                a_tree
            },
            Vec::new,
        );

        let graph = global_graph.into_inner().unwrap();

        debug!("Graphical ingestion took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        let hash = RwLock::new(HashMap::new());
        let filtered = index
            .to_owned()
            .into_par_iter()
            .filter(|v| graph.contains_node(v.id))
            .inspect(|e| {
                hash.write().unwrap().insert(e.id, *e);
            })
            .collect();

        debug!("HashMap creation took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        let tree = RTree::bulk_load(filtered);
        debug!("RTree bulk load took: {:?}", start_time.elapsed());

        info!(
            "Finished. Ingested {:?} nodes from {:?} nodes total in {}ms",
            tree.size(),
            index.len(),
            fixed_start_time.elapsed().as_millis()
        );

        Ok(Graph {
            graph,
            index: tree,
            hash,
        })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    pub fn map_match(&self, linestring: LineString) -> Result<Collapse, MatchError> {
        info!("Finding matched route for {} positions", linestring.0.len());

        let costing = CostingStrategies::default();

        // Create our hidden markov model solver
        let transition = Transition::new(self, linestring, costing);

        // Yield the transition layers of each level
        // & Collapse the layers into a final vector
        transition.solve(SelectiveForwardSolver::default())
    }

    pub(crate) fn route_nodes(
        &self,
        start_node: NodeIx,
        finish_node: NodeIx,
    ) -> Option<(Weight, Vec<Node>)> {
        debug!("Routing {:?} -> {:?}", start_node, finish_node);

        let (score, path) = petgraph::algo::astar(
            &self.graph,
            start_node,
            |finish| finish == finish_node,
            |e| e.weight().0,
            |_| 0 as Weight,
        )?;

        let hashmap = self.hash.read().ok()?;
        let route = path
            .iter()
            .filter_map(|v| hashmap.get(v).copied())
            .collect();

        Some((score, route))
    }

    /// Finds the optimal route between a start and end point.
    /// Returns the weight and routing node vector.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    pub fn route(&self, start: Point, finish: Point) -> Option<(Weight, Vec<Node>)> {
        let start_node = self.nearest_node(start)?;
        let finish_node = self.nearest_node(finish)?;
        self.route_nodes(start_node.id, finish_node.id)
    }
}
