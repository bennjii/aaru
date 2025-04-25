use aaru::codec::consts::{
    LAX_LYNWOOD_MATCHED, LAX_LYNWOOD_TRIP, LOS_ANGELES, VENTURA_MATCHED, VENTURA_TRIP, ZURICH,
};
use aaru::route::transition::{PredicateCache, SuccessorsLookupTable};
use aaru::route::Graph;
use criterion::criterion_main;
use geo::{coord, LineString};
use std::path::Path;
use std::sync::{Arc, Mutex};
use wkt::{ToWkt, TryFromWkt};

struct MapMatchScenario {
    name: &'static str,
    input_linestring: &'static str,
    expected_linestring: &'static str,
}

struct GraphArea {
    source_file: &'static str,
    matches: &'static [MapMatchScenario],
}

const MATCH_CASES: [GraphArea; 2] = [
    GraphArea {
        source_file: LOS_ANGELES,
        matches: &[
            MapMatchScenario {
                name: "VENTURA_HWY",
                input_linestring: VENTURA_TRIP,
                expected_linestring: VENTURA_MATCHED,
            },
            // MapMatchScenario {
            //     name: "LAX_LYNWOOD",
            //     input_linestring: LAX_LYNWOOD_TRIP,
            //     expected_linestring: LAX_LYNWOOD_MATCHED,
            // },
        ],
    },
    GraphArea {
        source_file: ZURICH,
        matches: &[],
    },
];

fn target_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("match");
    group.significance_level(0.1).sample_size(30);

    let lookup = Arc::new(Mutex::new(PredicateCache::default()));

    MATCH_CASES.into_iter().for_each(|ga| {
        let path = Path::new(ga.source_file).as_os_str().to_ascii_lowercase();
        let graph = Graph::new(path).expect("Graph must be created");

        ga.matches.iter().for_each(|sc| {
            group.bench_function(format!("match: {}", sc.name), |b| {
                b.iter(|| {
                    let coordinates: LineString<f64> =
                        LineString::try_from_wkt_str(sc.input_linestring)
                            .expect("Linestring must parse successfully.");

                    let result = graph
                        .map_match(coordinates, Arc::clone(&lookup))
                        .expect("Match must complete successfully");

                    let linestring = result
                        .matched()
                        .iter()
                        .map(|node| {
                            coord! {
                                x: node.position.x(),
                                y: node.position.y(),
                            }
                        })
                        .collect::<LineString>();

                    let as_wkt_string = linestring.wkt_string();
                    // assert_eq!(as_wkt_string, sc.expected_linestring);
                })
            });
        });
    });

    group.finish();
}

criterion::criterion_group!(targeted_benches, target_benchmark);
criterion_main!(targeted_benches);
