use fixtures::{
    LAX_LYNWOOD_MATCHED, LAX_LYNWOOD_TRIP, LOS_ANGELES, VENTURA_MATCHED, VENTURA_TRIP, ZURICH,
    fixture,
};

use routers::Graph;
use routers::transition::*;

use criterion::{black_box, criterion_main};
use geo::{LineString, coord};
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
            MapMatchScenario {
                name: "LAX_LYNWOOD",
                input_linestring: LAX_LYNWOOD_TRIP,
                expected_linestring: LAX_LYNWOOD_MATCHED,
            },
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
        let path = Path::new(fixture!(ga.source_file))
            .as_os_str()
            .to_ascii_lowercase();
        let graph = Graph::new(path).expect("Graph must be created");

        let costing = CostingStrategies::default();

        ga.matches.iter().for_each(|sc| {
            let coordinates: LineString<f64> = LineString::try_from_wkt_str(sc.input_linestring)
                .expect("Linestring must parse successfully.");

            let _ = graph
                .map_match(
                    black_box(coordinates.clone()),
                    black_box(Arc::clone(&lookup)),
                )
                .expect("Match must complete successfully");

            group.bench_function(format!("layer-gen: {}", sc.name), |b| {
                let points = coordinates.clone().into_points();
                let generator = LayerGenerator::new(&graph, &costing);

                b.iter(|| {
                    let (layers, _) = generator.with_points(&points);
                    assert_eq!(layers.layers.len(), points.len())
                })
            });

            group.bench_function(format!("match: {}", sc.name), |b| {
                b.iter(|| {
                    let result = graph
                        .map_match(coordinates.clone(), Arc::clone(&lookup))
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
                    assert_eq!(as_wkt_string, sc.expected_linestring);
                })
            });
        });
    });

    group.finish();
}

criterion::criterion_group!(targeted_benches, target_benchmark);
criterion_main!(targeted_benches);
