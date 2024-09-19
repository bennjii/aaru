use log::{debug, info};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use geo::{coord, line_string, point, LineInterpolatePoint, LineLocatePoint};
use petgraph::Direction;
use petgraph::prelude::{DiGraphMap};
use petgraph::visit::{EdgeRef};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;
use scc::HashMap;

use crate::codec::element::item::ProcessedElement;
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::element::variants::Node;
use crate::codec::parallel::Parallel;
use crate::geo::coord::latlng::LatLng;
use crate::route::error::RouteError;

type Weight = u8;
type NodeIx = i64;

type GraphStructure = DiGraphMap<NodeIx, Weight>;
// type GraphStructure = Csr<(), u32, Directed, usize>; - Doesn't implement IntoEdgesDirected yet.

const MAX_WEIGHT: Weight = 255;


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
    pub fn weights<'a>() -> Result<HashMap<&'a str, u8>, RouteError> {
        let weights: HashMap<&str, u8> = HashMap::new();

        // TODO: Base this dynamically on geospacial properties and roading shape

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
                                graph.add_edge(*a, *b, weight);
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
                // a_graph.extend_with_edges(
                //     b_graph.raw_edges().iter().map(|e| (e.source(), e.target(), e.weight))
                // );
                // for (node, _) in b_graph.node_references() {
                //     for petgraph::csr::EdgeReference::<'_, u32, Directed, usize> { source, target, weight, .. } in b_graph.edges(node) {
                //         a_graph.add_edge(source, target, *weight);
                //     }
                // }

                for (source, target, weight ) in b_graph.all_edges() {
                    a_graph.add_edge(source, target, *weight);
                }

                a_tree.extend(b_tree);
                (a_graph, a_tree)
            },
            || (GraphStructure::default(), Vec::new()),
        );

        let filtered = index
            .iter()
            .filter(|v| graph.contains_node(v.id))
            .map(|v| v.clone())
            .collect::<Vec<Node>>();

        let mut hash = std::collections::HashMap::new();
        for item in &filtered {
            // Add referenced node instead
            hash.insert(item.id as usize, item.clone());
        }

        let tree = RTree::bulk_load(filtered.clone());
        let time_passed = start_time.elapsed().as_millis();

        info!("Ingested {:?} nodes from {:?} nodes total in {}ms", tree.size(), index.len(), time_passed);
        Ok(Graph {
            graph,
            index: tree,
            hash,
        })
    }

    /// Finds the nearest node to a lat/lng position
    pub fn nearest_node(&self, lat_lng: LatLng) -> Option<&Node> {
        self.index.nearest_neighbor(&lat_lng.as_node())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn nearest_edges(&self, lat_lng: LatLng, distance: i64) -> impl Iterator<Item=(NodeIx, NodeIx, &Weight)> {
        self.index.locate_within_distance(lat_lng.as_node(), distance)
            .flat_map(|node|
                // Find all outgoing edges for the given node
                self.graph.edges_directed(node.id, Direction::Outgoing)
            )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn nearest_projected_nodes(&self, lat_lng: LatLng, distance: i64) -> impl Iterator<Item=LatLng> + '_ {
        let (x, y) = lat_lng.expand();
        let initial_point = point! { x: x, y: y };

        self.nearest_edges(lat_lng, distance)
            .filter_map(|edge| {
                let src = self.hash.get(&(edge.source() as usize));
                let trg = self.hash.get(&(edge.target() as usize));

                let (x1, y1) = src?.position.expand();
                let (x2, y2) = trg?.position.expand();

                Some(line_string! [ coord! { x: x1, y: y1 }, coord! { x: x2, y: y2 } ])
            })
            .filter_map(move |linestring| {
                linestring.line_locate_point(&initial_point)
                    .and_then(|frac| linestring.line_interpolate_point(frac))
                    .and_then(|point| {
                        let (lng, lat) = point.0.x_y();
                        LatLng::from_degree(lat, lng).ok()
                    })
            })
    }

    /// Finds the optimal route between a start and end point.
    /// Returns the weight and routing node vector.
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn route(&self, start: LatLng, finish: LatLng) -> Option<(Weight, Vec<Node>)> {
        let start_node = self.nearest_node(start)?;
        let finish_node = self.nearest_node(finish)?;

        let (score, path) = petgraph::algo::astar(
            &self.graph,
            start_node.id,
            |finish| finish == finish_node.id,
            |e| *e.weight(),
            |v| {
                self.hash
                    .get(&(v as usize))
                    .map(|v| v.to(finish_node).as_m() as u8)
                    .unwrap_or(0)
            },
        )?;

        let route = path
            .par_iter()
            .map(|v| self.hash.get(&(*v as usize)))
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .cloned()
            .collect();

        Some((score, route))
    }
}
