// use std::io::{Read, Write};
// use std::sync::Mutex;
//
// use osmpbf::{Element, Way};
// use petgraph::data::Build;
// use petgraph::Directed;
// use petgraph::graphmap::GraphMap;
// use petgraph::prelude::NodeIndex;
// use rstar::{Point, RTree};
// use scc::HashMap;
// use serde::{Deserialize, Serialize};
//
// use crate::Shard;
//
// fn predicate(object: &Way) -> bool {
//     let mut tags = object.tags();
//     tags.find(|tag| tag.0 == "highway").is_some()
//     // object
//     //     && (tags.contains("highway", "motorway")
//     //         || tags.contains("highway", "motorway_link")
//     //         || tags.contains("highway", "trunk")
//     //         || tags.contains("highway", "trunk_link")
//     //         || tags.contains("highway", "primary")
//     //         || tags.contains("highway", "primary_link")
//     //         || tags.contains("highway", "secondary")
//     //         || tags.contains("highway", "secondary_link")
//     //         || tags.contains("highway", "tertiary")
//     //         || tags.contains("highway", "tertiary_link")
//     //         || tags.contains("highway", "unclassified")
//     //         || tags.contains("highway", "residential")
//     //         || tags.contains("highway", "living_street"))
// }
//
// fn read_osmpbf(filename: std::ffi::OsString) -> osmpbfreader::OsmPbfReader<std::fs::File> {
//     let path = std::path::Path::new(&filename);
//     let file = std::fs::File::open(&path).unwrap();
//     osmpbfreader::OsmPbfReader::new(file)
// }
//
// #[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, PartialOrd)]
// pub struct Node {
//     pub id: i64,
//     pub index: NodeIndex,
//     pub lon: f64,
//     pub lat: f64,
// }
//
// impl Point for Node {
//     type Scalar = f64;
//     const DIMENSIONS: usize = 2;
//
//     fn generate(mut generator: impl FnMut(usize) -> Self::Scalar) -> Self {
//         Node {
//             id: 0,
//             index: NodeIndex::new(0),
//             lon: generator(0),
//             lat: generator(1),
//         }
//     }
//
//     fn nth(&self, index: usize) -> Self::Scalar {
//         match index {
//             0 => self.lon,
//             1 => self.lat,
//             _ => unreachable!(),
//         }
//     }
//
//     fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
//         match index {
//             0 => &mut self.lon,
//             1 => &mut self.lat,
//             _ => unreachable!(),
//         }
//     }
// }
//
// pub struct Graph {
//     data: Mutex<GraphMap<Node, i32, Directed>>,
//     index: rstar::RTree<Node>,
// }
//
// impl Graph {
//     pub fn new(filename: std::ffi::OsString) -> Graph {
//         // let f2 = File::open(filename.clone()).expect("Could not open file 2");
//         let mut reader = osmpbf::ElementReader::from_path(filename).expect("");
//
//         let mut weights = HashMap::new();
//
//         weights.insert("motorway", 1);
//         weights.insert("motorway_link", 2);
//         weights.insert("trunk", 3);
//         weights.insert("trunk_link", 4);
//         weights.insert("primary", 5);
//         weights.insert("primary_link", 6);
//         weights.insert("secondary", 7);
//         weights.insert("secondary_link", 8);
//         weights.insert("tertiary", 9);
//         weights.insert("tertiary_link", 10);
//         weights.insert("unclassified", 11);
//         weights.insert("residential", 12);
//         weights.insert("living_street", 13);
//         weights.insert("footway", 100);
//
//         let mut nodes = HashMap::new();
//         let mut graph = Mutex::new(GraphMap::new());
//         // let mut index = RTree::new();
//
//         println!("Starting reader...");
//
//         let mut ways: Vec<(String, Vec<i64>)> = vec![];
//
//         reader.par_map_reduce(
//             |element| match element {
//                 Element::Node(n) => {
//                     let mut node = Node {
//                         id: n.id(),
//                         index: NodeIndex::new(0),
//                         lat: n.lat(),
//                         lon: n.lon(),
//                     };
//
//                     graph.lock().expect("Couldn't lock").add_node(node);
//                 }
//                 Element::DenseNode(n) => {
//                     let mut node = Node {
//                         id: n.id(),
//                         index: NodeIndex::new(0),
//                         lat: n.lat(),
//                         lon: n.lon(),
//                     };
//
//                     graph.lock().unwrap().add_node(node);
//                 }
//                 Element::Way(way) => {
//                     ways.push((way.tags().find(|tag| tag.0 == "highway").unwrap().1.to_string(), way.refs().collect(), ));
//                 }
//                 _ => {}
//             },
//                 || (),
//                 |a, b| ()
//             )
//             .expect("Failed to read file");
//
//         for way in ways {
//             for node in way.1.windows(2) {
//                 let node1_id: i64 = node[0];
//                 let node2_id: i64 = node[1];
//
//                 let node1_index = nodes.get(&node1_id).unwrap();
//                 let node2_index = nodes.get(&node2_id).unwrap();
//
//                 // Computation performed twice, minimize it.
//                 if let Some(weight) = weights.get(way.0.as_str()) {
//                     graph.lock().unwrap().add_edge(*node1_index, *node2_index, *weight);
//                 }
//             }
//         }
//
//         Graph {
//             data: graph,
//             index: RTree::new(),
//         }
//     }
//
//     pub fn nearest_node(&self, lonlat: &[f64]) -> Option<&Node> {
//         let node = Node {
//             id: 0,
//             index: NodeIndex::new(0),
//             lon: lonlat[0],
//             lat: lonlat[1],
//         };
//
//         self.index.nearest_neighbor(&node)
//     }
//
//     pub fn route(&self, start: &[f64], finish: &[f64]) -> (i32, Vec<Vec<f64>>) {
//         let start_node = self.nearest_node(start).unwrap();
//         let finish_node = self.nearest_node(finish).unwrap();
//
//         // let start_index = start_node.index;
//         // let finish_index = finish_node.index;
//
//         // println!("Starting at {}, ending at {}.", start_index.index(), finish_index.index());
//
//         let graph = self.data.lock().unwrap();
//         let (score, path) = petgraph::algo::astar(
//             &self.data.lock().unwrap(),
//             start_node,
//             |finish| finish == finish_node,
//             |e| *e.weight(),
//             |_| 0,
//         )
//         .unwrap();
//
//         let mut route = vec![];
//         let nodes = self.data.raw_nodes();
//         for node_index in path {
//             let node = nodes.get(node_index.index()).unwrap();
//             let node_weight = &node.weight;
//             route.push(vec![node_weight.lon, node_weight.lat]);
//         }
//
//         (score, route)
//     }
// }
