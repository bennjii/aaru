use aaru::codec::consts::DISTRICT_OF_COLUMBIA;
use aaru::codec::element::ProcessedElement;
use aaru::codec::{Element, ElementIterator, Parallel, ProcessedElementIterator};

use criterion::criterion_main;
use log::info;

use aaru::route::Graph;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use tokio::time::Instant;

async fn ingest_and_count() {
    let timer = Instant::now();
    let path = PathBuf::from(Path::new(DISTRICT_OF_COLUMBIA));
    let reader = ProcessedElementIterator::new(path).await.expect("!");

    let (ways, nodes) = reader.par_red(
        |(ways, nodes), element| match element {
            ProcessedElement::Way(_) => (ways + 1, nodes),
            ProcessedElement::Node(_) => (ways, nodes + 1),
        },
        |(ways, nodes), (ways2, nodes2)| (ways + ways2, nodes + nodes2),
        || (0u64, 0u64),
    );

    info!(
        "Got {} ways and {} nodes in {}ms",
        ways,
        nodes,
        timer.elapsed().as_millis()
    );
}

async fn ingest_as_full_graph() {
    let path = Path::new(DISTRICT_OF_COLUMBIA)
        .as_os_str()
        .to_ascii_lowercase();
    let graph = Graph::new(path).await.expect("Could not generate graph");
    info!("Graph generated, size: {}", graph.size());
}

fn ingestion_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("ingestion_benchmark");
    group.significance_level(0.1).sample_size(30);

    group.bench_function("ingest_and_count", |b| {
        b.to_async(Runtime::new().expect("Must have runtime"))
            .iter(|| ingest_and_count())
    });
    group.bench_function("ingest_as_full_graph", |b| {
        b.to_async(Runtime::new().expect("Must have runtime"))
            .iter(|| ingest_as_full_graph())
    });
    group.finish();
}

criterion::criterion_group!(standard_benches, ingestion_benchmark);
criterion_main!(standard_benches);
