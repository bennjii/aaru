use geo::{
    coord, line_string, Destination, Geodesic, LineInterpolatePoint, LineLocatePoint, LineString,
    Point,
};
use log::{debug, error, info};
use petgraph::prelude::DiGraphMap;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rstar::{RTree, AABB};
use scc::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
#[cfg(feature = "tracing")]
use tracing::Level;

use crate::codec::element::item::ProcessedElement;
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::element::variants::common::OsmEntryId;
use crate::codec::element::variants::Node;
use crate::codec::parallel::Parallel;
use crate::route::error::RouteError;
use crate::route::transition::graph::Transition;

pub type Weight = u32;
pub type NodeIx = OsmEntryId;
pub type Edge<'a> = (NodeIx, NodeIx, &'a Weight);

pub type GraphStructure = DiGraphMap<NodeIx, Weight>;

const MAX_WEIGHT: Weight = 255 as Weight;

/// Routing graph, can be ingested from an `.osm.pbf` file,
/// and can be actioned upon using `route(start, end)`.
pub struct Graph {
    pub(crate) graph: GraphStructure,
    pub(crate) index: RTree<Node>,
    pub(crate) hash: HashMap<NodeIx, Node>,
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph with Nodes: {}", self.hash.len())
    }
}

impl Graph {
    pub fn index(&self) -> &RTree<Node> {
        &self.index
    }

    pub fn size(&self) -> usize {
        self.hash.len()
    }

    #[inline]
    pub fn get_position(&self, node_index: &NodeIx) -> Option<Point<f64>> {
        self.hash.get(node_index).map(|point| point.position)
    }

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
                            Some(weight) => {
                                weights.get(weight).map(|v| *v.get()).unwrap_or(MAX_WEIGHT)
                            }
                            None => MAX_WEIGHT,
                        };

                        // Update with all adjacent nodes
                        way.refs().windows(2).for_each(|edge| {
                            if let [a, b] = edge {
                                let mut lock = global_graph.lock().unwrap();
                                lock.add_edge(a.id, b.id, weight);
                                if !way.tags().one_way() && !way.tags().roundabout() {
                                    lock.add_edge(b.id, a.id, weight);
                                }
                                drop(lock);
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

        let hash = scc::HashMap::new();
        let filtered = index
            .to_owned()
            .into_par_iter()
            .filter(|v| graph.contains_node(v.id))
            .inspect(|e| {
                if let Err((index, node)) = hash.insert(e.id, *e) {
                    error!(
                        "Unable to insert node, index {:?} already taken. Node: {:?}",
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

    #[inline]
    pub fn square_scan(&self, point: &Point, distance: f64) -> Vec<&Node> {
        let bottom_right = Geodesic::destination(*point, 135.0, distance);
        let top_left = Geodesic::destination(*point, 315.0, distance);
        let bbox = AABB::from_corners(top_left, bottom_right);

        self.index().locate_in_envelope(&bbox).collect::<Vec<_>>()
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO))]
    #[inline]
    pub fn nearest_edges(
        &self,
        point: &Point,
        distance: f64,
    ) -> impl Iterator<Item = (NodeIx, NodeIx, &Weight)> {
        self.square_scan(point, distance)
            .into_iter()
            .flat_map(|node| self.graph.edges_directed(node.id, Direction::Outgoing))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::INFO))]
    #[inline]
    pub fn nearest_projected_nodes<'a>(
        &'a self,
        point: &'a Point,
        distance: f64,
    ) -> impl Iterator<Item = (Point, Edge<'a>)> + 'a {
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
                    .line_locate_point(point)
                    .and_then(|frac| linestring.line_interpolate_point(frac))
                    .map(|point| (point, edge))
            })
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    pub fn map_match(&self, linestring: LineString, distance: f64) -> LineString {
        info!("Finding matched route for {} positions", linestring.0.len());

        // Create our hidden markov model solver
        let transition = Transition::new(self, linestring);

        // Yield the transition layers of each level
        // & Collapse the layers into a final vector
        let match_result = transition.generate_probabilities(distance).backtrack();

        match_result
            .matched
            .iter()
            .map(|coord| {
                let (lng, lat) = coord.position.x_y();
                coord! { x: lng, y: lat }
            })
            .collect::<LineString>()
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
            |e| *e.weight(),
            |_| 0 as Weight,
        )?;

        let route = path
            .iter()
            .filter_map(|v| self.hash.get(v))
            .map(|e| *e.get())
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
