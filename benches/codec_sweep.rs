use aaru::codec::consts::DISTRICT_OF_COLUMBIA;
use aaru::codec::element::ProcessedElement;
use aaru::codec::{BlockIterator, Element, ElementIterator, Parallel, ProcessedElementIterator};
use criterion::criterion_main;
use log::info;
use rayon::iter::ParallelIterator;
use std::any::Any;
use std::path::PathBuf;
use tokio::runtime::{Handle, Runtime};
use tokio::task::spawn_blocking;

async fn block_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let mut iter = BlockIterator::new(path)
        .await
        .expect("Could not create iterator");

    iter.par_iter().for_each(|item| {
        info!("Block: {:?}", item.type_id());
    });
}

async fn element_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ElementIterator::new(path)
        .await
        .expect("Could not create iterator");

    let nodes = iter.map_red(
        |item| match item {
            Element::Way(_) => 0,
            Element::Node(_) => 1,
            Element::Relation(_) => 0,
            Element::DenseNodes(_) => 0,
        },
        |a, b| a + b,
        || 0,
    );

    info!("There are {nodes} nodes");
}

async fn processed_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ProcessedElementIterator::new(path)
        .await
        .expect("Could not create iterator");

    let nodes = iter.map_red(
        |item| match item {
            ProcessedElement::Way(_) => 0,
            ProcessedElement::Node(_) => 1,
        },
        |a, b| a + b,
        || 0,
    );

    info!("There are {nodes} nodes");
}

fn sweep_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("iterator_sweep");
    group.significance_level(0.1).sample_size(30);

    group.bench_function("block_iter_count", |b| {
        b.to_async(Runtime::new().expect("Must have runtime"))
            .iter(|| block_iter_count())
    });
    group.bench_function("element_iter_count", |b| {
        b.to_async(Runtime::new().expect("Must have runtime"))
            .iter(|| element_iter_count())
    });
    group.bench_function("processed_iter_count", |b| {
        b.to_async(Runtime::new().expect("Must have runtime"))
            .iter(|| processed_iter_count())
    });
    group.finish();
}

criterion::criterion_group!(standard_benches, sweep_benchmark);
criterion_main!(standard_benches);
