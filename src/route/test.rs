#![cfg(test)]

use std::{path::Path, time::Instant};

use crate::route::Graph;
use crate::codec::consts::{BADEN_WUERTTEMBERG, DISTRICT_OF_COLUMBIA, SYDNEY};
use crate::{geo::coord::latlng::LatLng, codec::element::variants::Node};

fn generate_linestring(route: Vec<Node>) -> String {
    format!("LINESTRING({})",
        route
            .iter()
            .map(|loc| format!("{:.10} {:.10}", loc.position.lng(), loc.position.lat()))
            .collect::<Vec<String>>()
            .join(",")
    )
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

    let start = LatLng::from_degree(38.91261500917026, -77.02343850496823)?;
    let end = LatLng::from_degree(38.91772552535467, -77.03456230592386)?;

    let (weight, route) = graph.route(start, end).expect("Could not produce route");
    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 268, "Incorrect Route Weighting");

    Ok(())
}

#[test]
fn stutgard_mapping() -> crate::Result<()> {
    let graph = init_graph(BADEN_WUERTTEMBERG)?;
    let start = LatLng::from_degree(48.773585361, 9.186777765)?;
    let end = LatLng::from_degree(48.779354943, 9.170598155)?;

    let time = Instant::now();
    let (weight, route) = graph.route(start, end).expect("Could not produce route");
    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 0, "Incorrect Route Weighting");

    Ok(())
}

#[test]
fn sydney_mapping() -> crate::Result<()> {
    let graph = init_graph(SYDNEY)?;

    let start = LatLng::from_degree(-33.883572, 151.180025)?;
    let end = LatLng::from_degree(-33.890029, 151.201438)?;

    println!("Start: {:?}", graph.nearest_node(start));

    let time = Instant::now();
    let (weight, route) = graph.route(start, end).expect("Could not produce route");
    println!("Took: {:?}", time.elapsed());

    println!("{}", generate_linestring(route));
    assert_eq!(weight, 0, "Incorrect Route Weighting");
    Ok(())
}
