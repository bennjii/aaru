use std::path::PathBuf;
use std::sync::Mutex;
use log::{debug, info};

use petgraph::data::Build;
use petgraph::Directed;
use petgraph::graphmap::{DiGraphMap, GraphMap};
use petgraph::prelude::NodeIndex;
use rstar::{RTree};
use scc::HashMap;

use crate::coord::latlng::LatLng;
use crate::element::item::{Element, ProcessedElement};
use crate::element::iterator::ElementIterator;
use crate::element::processed_iterator::ProcessedElementIterator;
use crate::element::variants::Node;
use crate::parallel::Parallel;
use crate::route::error::RouteError;

const MAX_WEIGHT: i32 = 9e9 as i32;

pub struct Graph {
    graph: DiGraphMap<i64, i64>,
    index: RTree<Node>
}

impl Graph {
    pub fn weights<'a>() -> Result<HashMap<&'a str, i32>, RouteError> {
        let mut weights = HashMap::new();

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

        let mut reader = ProcessedElementIterator::new(path)?;
        let weights = Graph::weights()?;

        info!("Ingesting...");

        let (graph, index): (DiGraphMap<i64, i64>, Vec<Node>) = reader.par_red(
            |(mut graph, mut tree): (DiGraphMap<i64, i64>, Vec<Node>), element: ProcessedElement| {
                match element {
                    ProcessedElement::Way(way) => {
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
                                    graph.add_edge(*a, *b, weight as i64);
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

        let tree = RTree::bulk_load(index);

        info!("Ingested.");
        Ok(Graph { graph, index: tree })
    }

    pub fn nearest_node(&self, node: Node) -> Option<&Node> {
        self.index.nearest_neighbor(&node)
    }

    // pub fn route(&self, start: &[f64], finish: &[f64]) -> (i32, Vec<Vec<f64>>) {
    //     let start_node = self.nearest_node(start).unwrap();
    //     let finish_node = self.nearest_node(finish).unwrap();
    //
    //     // let start_index = start_node.index;
    //     // let finish_index = finish_node.index;
    //
    //     // println!("Starting at {}, ending at {}.", start_index.index(), finish_index.index());
    //
    //     let graph = self.data.lock().unwrap();
    //     let (score, path) = petgraph::algo::astar(
    //         &self.data.lock().unwrap(),
    //         start_node,
    //         |finish| finish == finish_node,
    //         |e| *e.weight(),
    //         |_| 0,
    //     )
    //     .unwrap();
    //
    //     let mut route = vec![];
    //     let nodes = self.data.raw_nodes();
    //     for node_index in path {
    //         let node = nodes.get(node_index.index()).unwrap();
    //         let node_weight = &node.weight;
    //         route.push(vec![node_weight.lon, node_weight.lat]);
    //     }
    //
    //     (score, route)
    // }
}
