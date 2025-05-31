use crate::Scan;
use crate::graph::traits::util::init_graph;
use routers_fixtures::DISTRICT_OF_COLUMBIA;

use geo::{Point, wkt};

#[test]
fn projected_distance_check() {
    const DISTANCE: f64 = 100.0; // 100m search radius
    let graph = init_graph(DISTRICT_OF_COLUMBIA).expect("Could not produce graph");

    let points = wkt! {
        LINESTRING (-77.000347 38.887621, -76.998931 38.887638, -76.996978 38.887651, -76.994832 38.88763, -76.993067 38.887659)
    };

    for point in &points {
        let point = Point(*point);
        let nodes = graph
            .scan_nodes_projected(&point, DISTANCE)
            .collect::<Vec<_>>();

        assert!(
            !nodes.is_empty(),
            "Expected nodes to be non-empty at {DISTANCE}m. Could not find candidate for {point:?}"
        );
    }
}
