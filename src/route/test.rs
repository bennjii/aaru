use crate::route::{Graph, Scan};

use codec::osm::element::variants::Node;
use routers_fixtures::{DISTRICT_OF_COLUMBIA, fixture_path};

use crate::error::RouteError;
use geo::{LineString, Point, coord, wkt};
use std::{path::Path, time::Instant};
use wkt::ToWkt;

fn generate_linestring(route: Vec<Node>) -> String {
    route
        .iter()
        .map(|node| node.position)
        .collect::<LineString>()
        .wkt_string()
}

fn init_graph(file: &str) -> Result<Graph, RouteError> {
    let time = Instant::now();

    let fixture = fixture_path(file);
    let path = Path::new(&fixture);
    let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

    println!("Graph Init Took: {:?}", time.elapsed());
    Ok(graph)
}

#[test]
fn columbia_mapping() -> Result<(), RouteError> {
    let graph = init_graph(DISTRICT_OF_COLUMBIA)?;
    let time = Instant::now();

    let start = coord! { x: -77.02343850496823, y: 38.91261500917026 };
    let end = coord! { x: -77.03456230592386, y: 38.91772552535467 };

    let (weight, route) = graph
        .route(Point(start), Point(end))
        .expect("Could not produce route");

    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 216, "Incorrect Route Weighting");

    Ok(())
}

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
            .nearest_projected_nodes(&point, DISTANCE)
            .collect::<Vec<_>>();

        assert!(
            !nodes.is_empty(),
            "Expected nodes to be non-empty at {DISTANCE}m. Could not find candidate for {point:?}"
        );
    }
}

#[test]
#[cfg(any())]
fn stutgard_mapping() -> Result<(), RouteError> {
    let graph = init_graph(BADEN_WUERTTEMBERG)?;
    let start = coord! { x: 9.186777765, y: 48.773585361 };
    let end = coord! { x: 9.170598155, y: 48.779354943 };

    let time = Instant::now();
    let (weight, route) = graph
        .route(Point(start), Point(end))
        .expect("Could not produce route");

    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 658, "Incorrect Route Weighting");

    Ok(())
}

#[test]
#[cfg(any())]
fn sydney_mapping() -> Result<(), RouteError> {
    let graph = init_graph(SYDNEY)?;

    let start = coord! { x: 151.180025, y: -33.883572 };
    let end = coord! { x: 151.201438, y: -33.890029 };

    println!("Start: {:?}", graph.nearest_node(Point(start)));

    let time = Instant::now();
    let (weight, route) = graph
        .route(Point(start), Point(end))
        .expect("Could not produce route");

    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 450, "Incorrect Route Weighting");
    Ok(())
}
