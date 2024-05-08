mod test {
    use crate::Graph;
    use std::{fmt::format, path::Path, time::Instant};

    const DISTRICT_OF_COLUMBIA: &str = "./resources/district-of-columbia.osm.pbf";
    const BADEN_WUERTTEMBERG: &str = "./resources/baden-wuerttemberg-latest.osm.pbf";

    #[test]
    fn columnbia_mapping() {
        let path = Path::new(DISTRICT_OF_COLUMBIA);
        let graph = Graph::new(path.as_os_str().to_ascii_lowercase());

        let time = Instant::now();

        // LON;LAT
        let start = vec![-77.028125505, 38.908258345];
        let end = vec![-77.02799012990586, 38.9592498916808];

        let node = graph.nearest_node(&start);
        if let Some(node) = node {
            println!(
                "Nearest node to start is: lat:{}, lon:{}",
                node.lat, node.lon
            );
        }

        let route = graph.route(&start, &end);
        println!("Took: {:?}", time.elapsed());

        let linestring = route
            .1
            .iter()
            .map(|loc| format!("{} {}", loc[1], loc[0]))
            .collect::<Vec<String>>()
            .join(",");
        println!("LINESTRING({})", linestring);

        for item in route.1 {
            println!("{}:{}", item[0], item[1]);
        }

        assert_eq!(route.0, 15, "Incorrect Route Weighting");
    }

    // #[test]
    fn stutgard_mapping() {
        let path = Path::new(BADEN_WUERTTEMBERG);
        let graph = Graph::new(path.as_os_str().to_ascii_lowercase());

        let time = Instant::now();

        let start = vec![9.186777765, 48.773585361];
        let end = vec![9.170598155, 48.779354943];

        let node = graph.nearest_node(&start);
        if let Some(node) = node {
            println!(
                "Nearest node to start is: lat:{}, lon:{}",
                node.lat, node.lon
            );
        }

        let route = graph.route(&start, &end);
        println!("Took: {:?}", time.elapsed());

        for item in route.1 {
            println!("{}:{}", item[0], item[1]);
        }

        assert_eq!(route.0, 15, "Incorrect Route Weighting");
    }
}
