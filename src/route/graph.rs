use geo::{coord, line_string, HaversineDistance, LineInterpolatePoint, LineLocatePoint, LineString, Point};
use log::{debug, error, info};
use petgraph::prelude::DiGraphMap;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rstar::{RTree};
use scc::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::time::Instant;
use wkt::ToWkt;

#[cfg(feature = "tracing")]
use tracing::{Level, field::debug};

use crate::codec::element::item::ProcessedElement;
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::element::variants::Node;
use crate::codec::parallel::Parallel;
use crate::geo::coord::latlng::LatLng;
use crate::route::error::RouteError;
use crate::route::transition::Transition;

pub type Weight = u32;
pub type NodeIx = i64;
pub type Edge<'a> = (NodeIx, NodeIx, &'a Weight);

pub type GraphStructure = DiGraphMap<NodeIx, Weight>;
// type GraphStructure = Csr<(), u32, Directed, usize>; - Doesn't implement IntoEdgesDirected yet.

const MAX_WEIGHT: Weight = 255 as Weight;

/// Routing graph, can be ingested from an `.osm.pbf` file,
/// and can be actioned upon using `route(start, end)`.
pub struct Graph {
    graph: GraphStructure,
    index: RTree<Node>,
    hash: HashMap<NodeIx, Node>,
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph with Nodes: {}", self.hash.len())
    }
}

struct Vector(LatLng);

impl Graph {
    /// The weighting mapping of node keys to weight.
    pub fn weights<'a>() -> Result<HashMap<&'a str, Weight>, RouteError> {
        let weights: HashMap<&str, Weight> = HashMap::new();

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
            |(mut graph, mut tree): (GraphStructure, Vec<Node>), element: ProcessedElement| {
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
                for (source, target, weight) in b_graph.all_edges() {
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
            .inspect(|e| {
                if let Err((index, node)) = hash.insert(e.id, *e) {
                    error!(
                        "Unable to insert node, index {} already taken. Node: {:?}",
                        index, node
                    );
                }
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

    /// Finds the nearest node to a lat/lng position
    pub fn nearest_node(&self, point: Point) -> Option<&Node> {
        self.index.nearest_neighbor(&point)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO))]
    pub fn nearest_edges(
        &self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (NodeIx, NodeIx, &Weight)> {
        self.index
            .locate_within_distance(*point, distance)
            .inspect(|v| debug!("Found node: {}", v.position.wkt_string()))
            .flat_map(|node|
                // Find all outgoing edges for the given node
                self.graph.edges_directed(node.id, Direction::Outgoing)
            )
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO))]
    pub fn nearest_projected_nodes<'a>(
        &'a self,
        point: &'a Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, Edge)> + 'a {
        self.nearest_edges(point, distance)
            .filter_map(|edge| {
                let src = self.hash.get(&edge.source())?;
                let trg = self.hash.get(&edge.target())?;

                Some((line_string![src.position.0, trg.position.0], edge))
            })
            .filter_map(move |(linestring, edge)| {
                // We locate the point upon the linestring,
                // and then project that fractional (%)
                // upon the linestring to obtain a point
                linestring
                    .line_locate_point(&point)
                    .and_then(|frac| linestring.line_interpolate_point(frac))
                    .and_then(|point| Some((point, edge)))
            })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    pub fn map_match(&self, coordinates: Vec<LatLng>, distance: f64) -> Vec<Point> {
        let linestring: LineString = coordinates
            .iter()
            .map(|coord| {
                let (lng, lat) = coord.expand();
                coord! { x: lng, y: lat }
            })
            .collect();

        info!("Finding matched route for {} positions", linestring.0.len());

        // Create our hidden markov model solver
        let transition = Transition::new(linestring, self);

        // Yield the transition layers of each level
        // & Collapse the layers into a final vector
        transition.backtrack(distance)
    }

    /// Finds the optimal route between a start and end point.
    /// Returns the weight and routing node vector.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    pub fn route(&self, start: Point, finish: Point) -> Option<(Weight, Vec<Node>)> {
        let start_node = self.nearest_node(start)?;
        let finish_node = self.nearest_node(finish)?;

        debug!("Distance between selected nodes: {}m",  start_node.position.haversine_distance(&finish_node.position));
        debug!(
            "Lazy-Snapped Routing between {} {} and {} {}",
            start_node.id, start_node.position.wkt_string(),
            finish_node.id, finish_node.position.wkt_string()
        );

        let (score, path) = petgraph::algo::astar(
            &self.graph,
            start_node.id,
            |finish| finish == finish_node.id,
            |e| *e.weight(),
            |v| {
                self.hash
                    .get(&v)
                    .map(|v|
                        v.position.haversine_distance(&finish_node.position) as Weight
                    )
                    .unwrap_or(0 as Weight)
            },
        )?;

        debug!("Route Obtained. Score={}, PathLength={}", score, path.len());

        let route = path
            .iter()
            .filter_map(|v| self.hash.get(v))
            .map(|e| e.get().clone())
            .collect();

        Some((score, route))
    }
}
