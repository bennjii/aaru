use log::{debug, info};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use petgraph::{Directed, Direction};
use petgraph::csr::Csr;
use petgraph::graph::{DiGraph, EdgeReference};
use petgraph::prelude::{NodeIndex};
use petgraph::visit::{EdgeRef, IntoEdgesDirected, IntoNodeReferences};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;
use scc::HashMap;

use crate::codec::element::item::ProcessedElement;
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::element::variants::Node;
use crate::codec::parallel::Parallel;
use crate::geo::coord::latlng::LatLng;
use crate::route::error::RouteError;

const MAX_WEIGHT: u32 = 999;

// type GraphStructure = Csr<(), u32, Directed, usize>; - Doesn't implement IntoEdgesDirected yet.
type GraphStructure = DiGraph<i64, u32, usize>;

/// Routing graph, can be ingested from an `.osm.pbf` file,
/// and can be actioned upon using `route(start, end)`.
pub struct Graph {
    graph: GraphStructure,
    index: RTree<Node>,
    hash: std::collections::HashMap<usize, Node>,
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph with Nodes: {}", self.hash.len())
    }
}

struct Vector(LatLng);

impl Graph {
    /// The weighting mapping of node keys to weight.
    pub fn weights<'a>() -> Result<HashMap<&'a str, u32>, RouteError> {
        let weights = HashMap::new();

        weights.insert("motorway", 1)?;
        weights.insert("motorway_link", 2)?;
        weights.insert("trunk", 3)?;
        weights.insert("trunk_link", 4)?;
        weights.insert("primary", 5)?;
        weights.insert("primary_link", 6)?;
        weights.insert("secondary", 7)?;
        weights.insert("secondary_link", 8)?;
        weights.insert("tertiary", 9)?;
        weights.insert("tertiary_link", 10)?;
        weights.insert("unclassified", 11)?;
        weights.insert("residential", 12)?;
        weights.insert("living_street", 13)?;

        Ok(weights)
    }

    /// Creates a graph from a `.osm.pbf` file, using the `ProcessedElementIterator`
    pub fn new(filename: std::ffi::OsString) -> crate::Result<Graph> {
        let start_time = std::time::Instant::now();
        let path = PathBuf::from(filename);

        let reader = ProcessedElementIterator::new(path)?;
        let weights = Graph::weights()?;

        info!("Ingesting...");

        let (graph, index): (GraphStructure, Vec<Node>) = reader.par_red(
            |(mut graph, mut tree): (GraphStructure, Vec<Node>),
             element: ProcessedElement| {
                match element {
                    ProcessedElement::Way(way) => {
                        if !way.is_road() {
                            return (graph, tree);
                        }

                        // Get the weight from the weight table
                        let weight = match way.r#type() {
                            Some(weight) => weights
                                .get(weight.as_str())
                                .map(|v| v.get().clone())
                                .unwrap_or(MAX_WEIGHT),
                            None => MAX_WEIGHT,
                        };

                        // Update with all adjacent nodes
                        way.refs().windows(2).for_each(|edge| {
                            if let [a, b] = edge {
                                debug!("Edge Index: {}:{}", a, b);
                                graph.add_edge(NodeIndex::from(*a as usize), NodeIndex::from(*b as usize), weight);
                            } else {
                                debug!("Edge windowing produced odd-sized entry: {:?}", edge);
                            }
                        });
                    }
                    ProcessedElement::Node(node) => {
                        // Add the node to the graph
                        tree.push(node);
                    }
                }

                (graph, tree)
            },
            |(mut a_graph, mut a_tree), (b_graph, b_tree)| {
                // TODO: Add `Graph` merge optimisations
                a_graph.extend_with_edges(
                    b_graph.raw_edges().iter().map(|e| (e.source(), e.target(), e.weight))
                );
                // for (node, _) in b_graph.node_references() {
                //     for petgraph::csr::EdgeReference::<'_, u32, Directed, usize> { source, target, weight, .. } in b_graph.edges(node) {
                //         a_graph.add_edge(source, target, *weight);
                //     }
                // }

                a_tree.extend(b_tree);
                (a_graph, a_tree)
            },
            || (GraphStructure::default(), Vec::new()),
        );

        let filtered = index
            .iter()
            // .filter(|v| graph.contains_node(v.id))
            .map(|v| v.clone())
            .collect::<Vec<Node>>();

        let mut hash = std::collections::HashMap::new();
        for item in &filtered {
            // Add referenced node instead
            hash.insert(item.id as usize, item.clone());
        }

        let tree = RTree::bulk_load(filtered.clone());
        let time_passed = start_time.elapsed().as_millis();

        info!("Ingested {:?} nodes in {}ms", tree.size(), time_passed);
        Ok(Graph {
            graph,
            index: tree,
            hash,
        })
    }

    fn as_node(lat_lng: LatLng) -> Node {
        Node::new(lat_lng, 0i64)
    }

    /// Finds the nearest node to a lat/lng position
    pub fn nearest_node(&self, lat_lng: LatLng) -> Option<&Node> {
        self.index.nearest_neighbor(&Self::as_node(lat_lng))
    }

    pub fn nearest_edges(&self, lat_lng: LatLng, distance: i64) -> impl Iterator<Item=EdgeReference<u32, usize>>
    {
        // Get all nearby nodes
        self.index.locate_within_distance(Self::as_node(lat_lng), distance)
            .flat_map(|node|
                // Find all outgoing edges for the given node
                self.graph.edges_directed(NodeIndex::from(node.id as usize), Direction::Outgoing)
            )
        // TODO: Filter the above function to consider edges which cross the bounary
        //       as possibly invalid
        // TODO: use u32 instead of i64 as node-id yeah?
    }

    pub fn nearest_projected_nodes(&self, lat_lng: LatLng, distance: i64) -> impl Iterator<Item=LatLng> + '_
    {
        let location = |node_index: NodeIndex<usize>| {
            self.hash.get(&node_index.index()).unwrap().position
        };

        self.nearest_edges(lat_lng, distance)
           .map(move |edge| {
               let source = location(edge.source()).as_vec();
               let target = location(edge.target()).as_vec();

               // We need to project the lat-lng upon this virtual edge,
               // visualised as a 'straight' geodesic line between the
               // source and the target.

               let to_source = lat_lng.as_vec().to(source);
               let to_target = lat_lng.as_vec().to(target);

               let scalar = (to_source.dot(&to_target) / to_target.dot(&to_target))
                   .clamp(0, 1);
               let out_x = source.x + (scalar * (target.x - source.x)); // Longitude
               let out_y = source.y + (scalar * (target.y - source.y)); // Latitude
               LatLng::from((&out_y, &out_x))
           })
    }

    /// Finds the optimal route between a start and end point.
    /// Returns the weight and routing node vector.
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn route(&self, start: LatLng, finish: LatLng) -> Option<(u32, Vec<Node>)> {
        let start_node = self.nearest_node(start)?;
        let finish_node = self.nearest_node(finish)?;

        let (score, path) = petgraph::algo::astar(
            &self.graph,
            NodeIndex::from(start_node.id as usize),
            |finish| finish == NodeIndex::from(finish_node.id as usize),
            |e| *e.weight(),
            |v| {
                self.hash
                    .get(&v.index())
                    .map(|v| v.to(finish_node).as_m())
                    .unwrap_or(0)
            },
        )?;

        let route = path
            .par_iter()
            .map(|v| self.hash.get(&v.index()))
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .cloned()
            .collect();

        Some((score, route))
    }
}
