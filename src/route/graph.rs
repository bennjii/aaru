use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use log::{debug, info};

use petgraph::graphmap::{DiGraphMap};
use petgraph::prelude::{EdgeRef};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rstar::{RTree};
use scc::HashMap;

use crate::geo::coord::latlng::LatLng;
use crate::codec::element::item::{ProcessedElement};
use crate::codec::element::processed_iterator::ProcessedElementIterator;
use crate::codec::element::variants::Node;
use crate::codec::parallel::Parallel;
use crate::route::error::RouteError;

const MAX_WEIGHT: u32 = 999;

pub struct Graph {
    graph: DiGraphMap<i64, u32>,
    index: RTree<Node>,
    hash: std::collections::HashMap<i64, Node>
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph with Nodes: {}", self.hash.len())
    }
}

impl Graph {
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

    pub fn new(filename: std::ffi::OsString) -> crate::Result<Graph> {
        let path = PathBuf::from(filename);

        let reader = ProcessedElementIterator::new(path)?;
        let weights = Graph::weights()?;

        info!("Ingesting...");

        let (graph, index): (DiGraphMap<i64, u32>, Vec<Node>) = reader.par_red(
            |(mut graph, mut tree): (DiGraphMap<i64, u32>, Vec<Node>), element: ProcessedElement| {
                match element {
                    ProcessedElement::Way(way) => {
                        if !way.is_road() {
                            return (graph, tree);
                        }

                        // Get the weight from the weight table
                        let weight = match way.r#type() {
                            Some(weight) =>
                                weights.get(weight.as_str())
                                    .map(|v| v.get().clone())
                                    .unwrap_or(MAX_WEIGHT),
                            None => MAX_WEIGHT
                        };

                        // Update with all adjacent nodes
                        way.refs()
                            .windows(2)
                            .for_each(|edge| {
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
            || (DiGraphMap::new(), Vec::new()),
            |(mut a_graph, mut a_tree), (b_graph, b_tree)| {
                // TODO: Add `Graph` merge optimisations
                for (start, end, weight) in b_graph.all_edges() {
                    a_graph.add_edge(start, end, weight.clone());
                }

                a_tree.extend(b_tree);
                (a_graph, a_tree)
            },
        );

        let filtered = index
            .iter()
            .filter(|v| graph.contains_node(v.id))
            .map(|v| v.clone())
            .collect::<Vec<Node>>();

        let mut hash = std::collections::HashMap::new();
        for item in &filtered {
            // Add referenced node instead
            hash.insert(item.id, item.clone());
        }

        let tree = RTree::bulk_load(filtered.clone());

        info!("Ingested {:?} nodes.", tree.size());
        Ok(Graph { graph, index: tree, hash })
    }

    pub fn nearest_node(&self, lat_lng: LatLng) -> Option<&Node> {
        self.index.nearest_neighbor(&Node::new(lat_lng, &0i64))
    }

    #[tracing::instrument]
    pub fn route(&self, start: LatLng, finish: LatLng) -> Option<(u32, Vec<Node>)> {
        let start_node = self.nearest_node(start).unwrap();
        let finish_node = self.nearest_node(finish).unwrap();

        let (score, path) = petgraph::algo::astar(
            &self.graph,
            start_node.id,
            |finish| finish == finish_node.id,
            |e| *e.weight(),
            |v| {
                self.hash
                    .get(&v)
                    .map(|v| v.to(finish_node).as_m())
                    .unwrap_or(0)
            },
        )?;

        let route = path
            .par_iter()
            .map(|v| self.hash.get(v))
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .cloned()
            .collect();

        Some((score, route))
    }
}
