use crate::graph::item::{Graph, GraphStructure, Weight};

use codec::Node;
use codec::osm::element::ProcessedElement;
use codec::osm::element::variants::OsmEntryId;
use codec::osm::{Parallel, ProcessedElementIterator};

use log::{debug, info};
use rstar::RTree;
use rustc_hash::FxHashMap;

use crate::{DirectionAwareEdgeId, Edge, FatEdge, PredicateCache};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

impl Graph<OsmEntryId> {
    /// The weighting mapping of node keys to weight.
    pub fn weights<'a>() -> Result<HashMap<&'a str, Weight>, Box<dyn Error>> {
        let mut weights: HashMap<&str, Weight> = HashMap::new();

        // TODO: Base this dynamically on geospacial properties and roading shape

        // Primary roadways
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

        // Residential
        weights.insert("residential", 11);
        weights.insert("unclassified", 12);

        // Misc / Service. (Shouldn't be impossible to traverse, just difficult.)
        weights.insert("living_street", 50);
        weights.insert("service", 51);
        weights.insert("busway", 52);
        weights.insert("road", 53);

        Ok(weights)
    }

    /// Creates a graph from a `.osm.pbf` file, using the `ProcessedElementIterator`
    pub fn new(filename: std::ffi::OsString) -> Result<Self, Box<dyn Error>> {
        let mut start_time = Instant::now();
        let fixed_start_time = Instant::now();

        let path = PathBuf::from(filename);

        let reader = ProcessedElementIterator::new(path).map_err(|err| format!("{err:?}"))?;
        let weights = Graph::weights()?;

        debug!("Iterator warming took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        info!("Ingesting...");

        let global_graph = Mutex::new(GraphStructure::new());
        let (nodes, edges): (Vec<Node<OsmEntryId>>, Vec<Edge<OsmEntryId>>) = reader.par_red(
            |mut trees: (Vec<Node<OsmEntryId>>, Vec<Edge<OsmEntryId>>),
             element: ProcessedElement| {
                match element {
                    ProcessedElement::Way(way) => {
                        // If way is not traversable (/ is not road)
                        if way.tags().road_tag().is_none() {
                            return trees;
                        }

                        // Get the weight from the weight table
                        let weight = match way.tags().road_tag() {
                            Some(weight) => weights
                                .get(weight)
                                .copied()
                                .unwrap_or(crate::graph::MAX_WEIGHT),
                            None => crate::graph::MAX_WEIGHT,
                        };

                        let bidirectional = !way.tags().unidirectional();

                        // Update with all adjacent nodes
                        way.refs().windows(2).for_each(|edge| {
                            if let [a, b] = edge {
                                let direction_aware = DirectionAwareEdgeId::new(way.id());
                                let mut lock = global_graph.lock().unwrap();

                                let w = (weight, direction_aware.forward());
                                trees.1.push(Edge::from((a.id, b.id, &w)));
                                lock.add_edge(a.id, b.id, w);

                                // If way is bidi, add opposite edge with a DirAw backward.
                                if bidirectional {
                                    let w = (weight, direction_aware.backward());
                                    trees.1.push(Edge::from((b.id, a.id, &w)));
                                    lock.add_edge(b.id, a.id, w);
                                }
                            } else {
                                debug!("Edge windowing produced odd-sized entry: {edge:?}");
                            }
                        });
                    }
                    ProcessedElement::Node(node) => {
                        // Add the node to the graph
                        trees.0.push(node);
                    }
                    _ => {}
                }

                trees
            },
            |mut a_tree, b_tree| {
                a_tree.0.extend(b_tree.0);
                a_tree.1.extend(b_tree.1);
                a_tree
            },
            || (Vec::new(), Vec::new()),
        );

        let graph = global_graph.into_inner().unwrap();

        debug!("Graphical ingestion took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        let mut hash = FxHashMap::default();
        let filtered = {
            nodes
                .iter()
                .copied()
                .filter(|v| graph.contains_node(v.id))
                .inspect(|e| {
                    hash.insert(e.id, *e);
                })
                .collect()
        };

        let fat = {
            edges
                .iter()
                .flat_map(|edge| {
                    Some(FatEdge {
                        source: *hash.get(&edge.source)?,
                        target: *hash.get(&edge.target)?,
                        id: edge.id,
                        weight: edge.weight,
                    })
                })
                .collect()
        };

        debug!("HashMap creation took: {:?}", start_time.elapsed());
        start_time = Instant::now();

        let tree = RTree::bulk_load(filtered);
        let tree_edge = RTree::bulk_load(fat);
        debug!("RTree bulk load took: {:?}", start_time.elapsed());

        info!(
            "Finished. Ingested {:?} nodes from {:?} nodes total in {}ms",
            tree.size(),
            nodes.len(),
            fixed_start_time.elapsed().as_millis()
        );

        Ok(Graph {
            graph,
            hash,

            index: tree,
            index_edge: tree_edge,

            cache: Arc::new(Mutex::new(PredicateCache::default())),
        })
    }
}
