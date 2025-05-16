use codec::consts::DISTRICT_OF_COLUMBIA;
use codec::element::ProcessedElement;
use codec::{Parallel, ProcessedElementIterator};

use criterion::criterion_main;
use log::info;

use routers::route::Graph;
use std::path::{Path, PathBuf};
use tokio::time::Instant;

fn ingest_as_full_graph() {
    let path = Path::new(DISTRICT_OF_COLUMBIA)
        .as_os_str()
        .to_ascii_lowercase();
    let graph = Graph::new(path).expect("Could not generate graph");
    info!("Graph generated, size: {}", graph.size());
}

fn ingestion_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("ingestion_benchmark");
    group.significance_level(0.1).sample_size(30);

    group.bench_function("ingest_as_full_graph", |b| {
        b.iter(|| ingest_as_full_graph())
    });
    group.finish();
}

criterion::criterion_group!(standard_benches, ingestion_benchmark);
criterion_main!(standard_benches);
