#![cfg(test)]

use crate::codec::consts::{BADEN_WUERTTEMBERG, DISTRICT_OF_COLUMBIA, SYDNEY};
use crate::codec::element::variants::Node;
use crate::route::Graph;
use geo::{coord, LineString, Point};
use std::{path::Path, time::Instant};
use wkt::ToWkt;

fn generate_linestring(route: Vec<Node>) -> String {
    route
        .iter()
        .map(|node| node.position)
        .collect::<LineString>()
        .wkt_string()
}

fn init_graph(file: &str) -> crate::Result<Graph> {
    let time = Instant::now();

    let path = Path::new(file);
    let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

    println!("Graph Init Took: {:?}", time.elapsed());
    Ok(graph)
}

#[test]
fn columbia_mapping() -> crate::Result<()> {
    let graph = init_graph(DISTRICT_OF_COLUMBIA)?;
    let time = Instant::now();

    let start = coord! { x: -77.02343850496823, y: 38.91261500917026 };
    let end = coord! { x: -77.03456230592386, y: 38.91772552535467 };

    let (weight, route) = graph
        .route(Point(start), Point(end))
        .expect("Could not produce route");

    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 342, "Incorrect Route Weighting");

    Ok(())
}

#[test]
fn stutgard_mapping() -> crate::Result<()> {
    let graph = init_graph(BADEN_WUERTTEMBERG)?;
    let start = coord! { x: 9.186777765, y: 48.773585361 };
    let end = coord! { x: 9.170598155, y: 48.779354943 };

    let time = Instant::now();
    let (weight, route) = graph
        .route(Point(start), Point(end))
        .expect("Could not produce route");

    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 676, "Incorrect Route Weighting");

    Ok(())
}

#[test]
fn sydney_mapping() -> crate::Result<()> {
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
    assert_eq!(weight, 627, "Incorrect Route Weighting");
    Ok(())
}
