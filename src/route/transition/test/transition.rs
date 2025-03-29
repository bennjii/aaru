// use std::{
//     path::Path,
//     sync::{Arc, LazyLock, Mutex},
//     time::Instant,
// };
//
// use geo::{wkt, Destination, Distance, Geodesic, Haversine, LineString};
// use petgraph::{graph::NodeIndex, Directed, Graph as Petgraph};
// use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
// use rstar::AABB;
// use wkt::ToWkt;
//
// use crate::{codec::consts::SYDNEY, route::Graph};
// use crate::route::transition::CostingStrategies;
// use crate::route::transition::graph::Transition;
//
// static GLOBAL_GRAPH: LazyLock<Graph> = LazyLock::new(|| {
//     let path = Path::new(SYDNEY);
//     Graph::new(path.as_os_str().to_ascii_lowercase()).expect("Couldn't create graph.")
// });
//
// #[tokio::test]
// async fn test_transition() {
//     env_logger::try_init().ok();
//     println!("Graph Created. Transitioning...");
//
//     let linestring = wkt! {
//         LINESTRING (151.19462 -33.885309, 151.193783 -33.887126,151.189685 -33.890243, 151.185329 -33.892915, 151.179487 -33.896864, 151.178023 -33.898694, 151.179283 -33.902523, 151.181273 -33.906295, 151.184916 -33.907203, 151.187641 -33.906851, 151.189315 -33.905061, 151.192024 -33.902145, 151.19432 -33.899576, 151.194438 -33.898957, 151.19425 -33.898556)
//     };
//
//     let now = Instant::now();
//     let costing = CostingStrategies::default();
//     let transition = Transition::new(&GLOBAL_GRAPH, linestring, costing);
//
//     println!(
//         "[TRANSITION] Init. Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
//
//     let now = Instant::now();
//     let mres = transition.generate_probabilities(50.0).backtrack().unwrap();
//
//     println!(
//         "[TRANSITION] Backtracked. Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
//
//     println!(
//         "[TRANSITION] Backtracked. {}",
//         mres.matched
//             .iter()
//             .map(|v| v.position)
//             .collect::<LineString>()
//             .wkt_string()
//     );
//
//     assert!(mres.matched.len() > 1);
// }
//
// #[test]
// fn nearest_nodes() {
//     // Using nearest edges
//     let now = Instant::now();
//     let point = wkt! { POINT(151.183886 -33.885197) };
//
//     println!("Point: {:?}", point);
//
//     // 100m2 search radius
//
//     let bottom_right = Geodesic::destination(point, 135.0, 100.0);
//     let top_left = Geodesic::destination(point, 315.0, 100.0);
//     let bbox = AABB::from_corners(top_left, bottom_right);
//
//     println!("BBox: {:?}", bbox);
//
//     let nearest = GLOBAL_GRAPH
//         .index()
//         .locate_in_envelope(&bbox)
//         .collect::<Vec<_>>();
//     assert_eq!(nearest.len(), 9);
//
//     println!(
//         "[NEAREST_DISTANCE_RADIUS_100(2)] Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
//
//     // Using nearest edges
//     let now = Instant::now();
//     let point = wkt! { POINT(151.183886 -33.885197) };
//     let nearest = GLOBAL_GRAPH
//         .nearest_edges(&point, 100.0)
//         .collect::<Vec<_>>();
//     assert!(nearest.len() > 1);
//
//     println!(
//         "[NEAREST_EDGES] Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
//
//     // Using nearest projected
//     let now = Instant::now();
//     let point = wkt! { POINT(151.183886 -33.885197) };
//     let nearest = GLOBAL_GRAPH.nearest_projected_nodes(&point, 100.0);
//     assert_eq!(nearest.collect::<Vec<_>>().len(), 16);
//
//     println!(
//         "[NEAREST_PROJECTED] Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
// }
//
// #[test]
// fn test_par_layers() {
//     type LayerId = usize;
//     type NodeId = usize;
//
//     let path = Path::new(SYDNEY);
//     let graph = Graph::new(path.as_os_str().to_ascii_lowercase()).expect("Couldn't create graph.");
//
//     let petgraph: Arc<Mutex<Petgraph<(LayerId, NodeId, TransitionCandidate), i32, Directed>>> =
//         Arc::new(Mutex::new(Petgraph::new()));
//
//     let distance = 100.0;
//     let linestring = wkt! {
//         LINESTRING (151.19462 -33.885309, 151.193783 -33.887126, 151.189685 -33.890243, 151.185329 -33.892915, 151.179487 -33.896864, 151.178023 -33.898694, 151.179283 -33.902523, 151.181273 -33.906295, 151.184916 -33.907203, 151.187641 -33.906851, 151.189315 -33.905061, 151.192024 -33.902145, 151.19432 -33.899576, 151.194438 -33.898957, 151.19425 -33.898556)
//     };
//
//     let now = Instant::now();
//     let layers = linestring
//         .into_points()
//         .par_iter()
//         .enumerate()
//         .map(|(layer_id, point)| {
//             graph
//                 // We'll do a best-effort 100m2 search (square) radius
//                 .nearest_projected_nodes(point, distance)
//                 .enumerate()
//                 .map(|(node_id, (position, map_edge))| {
//                     // We have the actual projected position, and it's associated edge.
//                     let _distance = Haversine::distance(position, *point);
//                     let emission = DefaultEmissionCost.calculate(EmissionContext {
//                         candidate_position: &position,
//                         source_position: &point,
//                     }) as f64;
//
//                     let candidate = TransitionCandidate {
//                         map_edge: (map_edge.0, map_edge.1),
//                         emission,
//                         position,
//                         layer_id,
//                         node_id,
//                     };
//
//                     let node_index = petgraph
//                         .lock()
//                         .unwrap()
//                         .add_node((layer_id, node_id, candidate));
//
//                     node_index
//                 })
//                 .collect::<Vec<NodeIndex>>()
//         })
//         .collect::<Vec<_>>();
//
//     println!(
//         "[LAYERS] Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
//     assert_eq!(layers.len(), 15);
// }
//
// #[test]
// fn test_series_layers() {
//     type LayerId = usize;
//     type NodeId = usize;
//
//     let path = Path::new(SYDNEY);
//     let graph = Graph::new(path.as_os_str().to_ascii_lowercase()).expect("Couldn't create graph.");
//
//     let mut petgraph: Petgraph<(LayerId, NodeId, TransitionCandidate), i32, Directed> =
//         Petgraph::new();
//
//     let distance = 100.0;
//     let linestring = wkt! {
//         LINESTRING (151.19462 -33.885309, 151.193783 -33.887126, 151.189685 -33.890243, 151.185329 -33.892915, 151.179487 -33.896864, 151.178023 -33.898694, 151.179283 -33.902523, 151.181273 -33.906295, 151.184916 -33.907203, 151.187641 -33.906851, 151.189315 -33.905061, 151.192024 -33.902145, 151.19432 -33.899576, 151.194438 -33.898957, 151.19425 -33.898556)
//     };
//
//     let now = Instant::now();
//     let layers = linestring
//         .into_points()
//         .iter()
//         .enumerate()
//         .map(|(layer_id, point)| {
//             graph
//                 // We'll do a best-effort 100m2 search (square) radius
//                 .nearest_projected_nodes(point, distance)
//                 .enumerate()
//                 .map(|(node_id, (position, map_edge))| {
//                     // We have the actual projected position, and it's associated edge.
//                     let _distance = Haversine::distance(position, *point);
//                     let emission = DefaultEmissionCost.calculate(EmissionContext {
//                         candidate_position: &position,
//                         source_position: &point,
//                     }) as f64;
//
//                     let candidate = TransitionCandidate {
//                         map_edge: (map_edge.0, map_edge.1),
//                         emission,
//                         position,
//                         layer_id,
//                         node_id,
//                     };
//
//                     let node_index = petgraph.add_node((layer_id, node_id, candidate));
//
//                     node_index
//                 })
//                 .collect::<Vec<NodeIndex>>()
//         })
//         .collect::<Vec<_>>();
//
//     println!(
//         "[LAYERS] Elapsed: {}us (us = 0.001 ms)",
//         now.elapsed().as_micros()
//     );
//     assert_eq!(layers.len(), 15);
// }
