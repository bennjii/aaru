use crate::Graph;
use std::{fmt::format, path::Path, time::Instant};
use crate::codec::test::{AUSTRALIA, BADEN_WUERTTEMBERG, DISTRICT_OF_COLUMBIA};
use crate::coord::latlng::LatLng;

#[test]
fn columbia_mapping() -> crate::Result<()> {
    let time = Instant::now();

    let path = Path::new(DISTRICT_OF_COLUMBIA);
    let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

    println!("Graph Init Took: {:?}", time.elapsed());

    let time = Instant::now();

    // LON;LAT
    // -771210058
    let start = LatLng::new_raw(-77.02343850496823, 38.91261500917026);
    let end = LatLng::new_raw(-77.03456230592386, 38.91772552535467);

    let node = graph.nearest_node(start);
    if let Some(node) = node {
        println!(
            "Nearest node to start is: {:?}",
            node.position
        );
    }

    let end_node = graph.nearest_node(end);
    if let Some(node) = end_node {
        println!(
            "Nearest node to end is: {:?}",
            node.position
        );
    }

    let route = graph.route(start, end);
    println!("Took: {:?}", time.elapsed());

    let linestring = route
        .1
        .iter()
        .map(|loc| format!("{:.10} {:.10}", loc.position.lat, loc.position.lng))
        .collect::<Vec<String>>()
        .join(",");

    println!("LINESTRING({})", linestring);
    assert_eq!(route.0, 15, "Incorrect Route Weighting");

    Ok(())
}

// #[test]
fn stutgard_mapping() -> crate::Result<()> {
    let time = Instant::now();

    let path = Path::new(BADEN_WUERTTEMBERG);
    let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

    println!("Graph Init Took: {:?}", time.elapsed());

    let time = Instant::now();

    let start = vec![9.186777765, 48.773585361];
    let end = vec![9.170598155, 48.779354943];

    Ok(())

    // let node = graph.nearest_node(&start);
    // if let Some(node) = node {
    //     println!(
    //         "Nearest node to start is: lat:{}, lon:{}",
    //         node.position.lat, node.position.lng
    //     );
    // }
    //
    // let route = graph.route(&start, &end);
    // println!("Took: {:?}", time.elapsed());
    //
    // let linestring = route
    //     .1
    //     .iter()
    //     .map(|loc| format!("{} {}", loc[0], loc[1]))
    //     .collect::<Vec<String>>()
    //     .join(",");
    //
    // println!("LINESTRING({})", linestring);
    //
    // for item in route.1 {
    //     println!("{}:{}", item[0], item[1]);
    // }
    //
    // assert_eq!(route.0, 0, "Incorrect Route Weighting");
}

// #[test]
fn test_sydney() -> crate::Result<()> {
    let time = Instant::now();

    let path = Path::new(AUSTRALIA);
    let graph = Graph::new(path.as_os_str().to_ascii_lowercase())?;

    println!("Graph Init Took: {:?}", time.elapsed());

    Ok(())

    // let time = Instant::now();

    // let start = vec![151.183154, -33.885424];
    // let end = vec![151.202487, -33.883972];
    //
    // let route = graph.route(&start, &end);
    // println!("Took: {:?}", time.elapsed());
    //
    // let linestring = route
    //     .1
    //     .iter()
    //     .map(|loc| format!("{} {}", loc[0], loc[1]))
    //     .collect::<Vec<String>>()
    //     .join(",");
    //
    // println!("LINESTRING({})", linestring);
    //
    // for item in route.1 {
    //     println!("{}:{}", item[0], item[1]);
    // }
    //
    // assert_eq!(route.0, 0, "Incorrect Route Weighting");
}
