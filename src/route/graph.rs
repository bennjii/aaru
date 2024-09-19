use log::{debug, info};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::time::Instant;
use geo::{coord, line_string, point, LineInterpolatePoint, LineLocatePoint};
use petgraph::Direction;
use petgraph::prelude::{DiGraphMap};
use petgraph::visit::{EdgeRef};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
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
type Edge<'a> = (NodeIx, NodeIx, &'a Weight);

type GraphStructure = DiGraphMap<NodeIx, Weight>;
// type GraphStructure = Csr<(), u32, Directed, usize>; - Doesn't implement IntoEdgesDirected yet.

const MAX_WEIGHT: Weight = 255;


/// Routing graph, can be ingested from an `.osm.pbf` file,
/// and can be actioned upon using `route(start, end)`.
pub struct Graph {
    graph: GraphStructure,
    index: RTree<Node>,
    hash: scc::HashMap<usize, Node>,
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
        let mut start_time = Instant::now();
        let fixed_start_time = Instant::now();

        let path = PathBuf::from(filename);

        let reader = ProcessedElementIterator::new(path)?;
        let weights = Graph::weights()?;

        debug!("Iterator warming took: {:?}", start_time.elapsed());
        start_time = Instant::now();

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
                // a_graph.extend(b_graph.all_edges());
                for (source, target, weight ) in b_graph.all_edges() {
                    a_graph.add_edge(source, target, *weight);
                }

                a_tree.extend(b_tree);
                (a_graph, a_tree)
            },
            || (GraphStructure::default(), Vec::new()),
        );

        debug!("Graphical ingestion took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        let hash = scc::HashMap::new();
        let filtered = index
            .to_owned()
            .into_par_iter()
            .filter(|v| graph.contains_node(v.id))
            .inspect(|e| { hash.insert(e.id as usize, *e); })
            .collect();

        debug!("HashMap creation took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        let tree = RTree::bulk_load(filtered);
        debug!("RTree bulk load took: {:?}", start_time.elapsed());

        info!(
            "Finished. Ingested {:?} nodes from {:?} nodes total in {}ms",
            tree.size(), index.len(), fixed_start_time.elapsed().as_millis()
        );

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
    pub fn nearest_edges(&self, lat_lng: &LatLng, distance: i64) -> impl Iterator<Item=(NodeIx, NodeIx, &Weight)> {
        self.index.locate_within_distance(lat_lng.as_node(), distance)
            .flat_map(|node|
                // Find all outgoing edges for the given node
                self.graph.edges_directed(node.id, Direction::Outgoing)
            )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn nearest_projected_nodes(&self, lat_lng: &LatLng, distance: i64) -> impl Iterator<Item=(LatLng, Edge)> + '_ {
        let (x, y) = lat_lng.expand();
        let initial_point = point! { x: x, y: y };

        self.nearest_edges(&lat_lng, distance)
            .filter_map(|edge| {
                let src = self.hash.get(&(edge.source() as usize))?;
                let trg = self.hash.get(&(edge.target() as usize))?;

                let (x1, y1) = src.position.expand();
                let (x2, y2) = trg.position.expand();

                Some((line_string! [ coord! { x: x1, y: y1 }, coord! { x: x2, y: y2 } ], edge))
            })
            .filter_map(move |(linestring, edge)| {
                // We locate the point upon the linestring,
                // and then project that fractional (%)
                // upon the linestring to obtain a point
                linestring.line_locate_point(&initial_point)
                    .and_then(|frac| linestring.line_interpolate_point(frac))
                    .and_then(|point| {
                        let (lng, lat) = point.0.x_y();

                        LatLng::from_degree(lat, lng)
                            .ok()
                            .and_then(|pos| Some((pos, edge)))
                    })
            })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn map_match(&self, coordinates: Vec<LatLng>, distance: i64) -> Vec<LatLng> {
        coordinates
            .par_iter()
            // Perform a candidate search (CS) for nearby projected nodes
            .map(|coordinate| self.nearest_projected_nodes(coordinate, distance))
            // Now we use a hidden markov model to predict the most
            // efficient route chaining given a gaussian emission
            // model and a transition probability
            .filter_map(|e| {
                // This is where we'll implement HMM
                // in order to select the best of the
                // given
                e.take(1).next()
            })
            // We can use the edges brought through so
            // that we can reconstruct (using routing)
            // an interpolated route that would have
            // occurred.
            .map(|(position, _)| {
                // NOTE: Could use `.windows()` over a fixed slice to route between
                position
            })
            .collect()
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
                    .map(|v| v.to(&finish_node).as_m() as u8)
                    .unwrap_or(0)
            },
        )?;

        let route = path
            .iter()
            .filter_map(|v| self.hash.get(&(*v as usize)))
            .map(|e| e.get().clone())
            .collect();

        Some((score, route))
    }
}
