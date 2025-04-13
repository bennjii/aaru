use aaru::codec::consts::{LOS_ANGELES, VENTURA_MATCHED, VENTURA_TRIP};
use aaru::route::Graph;
use criterion::criterion_main;
use geo::{coord, LineString};
use std::path::Path;
use wkt::{ToWkt, TryFromWkt};

struct MapMatchScenario {
    name: &'static str,
    source_file: &'static str,

    input_linestring: &'static str,
    expected_linestring: &'static str,
}

const MATCH_CASES: [MapMatchScenario; 1] = [MapMatchScenario {
    name: "VENTURA_HWY",
    source_file: LOS_ANGELES,

    input_linestring: VENTURA_TRIP,
    expected_linestring: VENTURA_MATCHED,
}];

fn target_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("match");
    group.significance_level(0.1).sample_size(30);

    MATCH_CASES.into_iter().for_each(|sc| {
        let path = Path::new(sc.source_file).as_os_str().to_ascii_lowercase();
        let graph = Graph::new(path).expect("Graph must be created");

        group.bench_function(format!("match: {}", sc.name), |b| {
            b.iter(|| {
                let coordinates: LineString<f64> =
                    LineString::try_from_wkt_str(sc.input_linestring)
                        .expect("Linestring must parse successfully.");

                let result = graph
                    .map_match(coordinates)
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

    group.finish();
}

criterion::criterion_group!(targeted_benches, target_benchmark);
criterion_main!(targeted_benches);
