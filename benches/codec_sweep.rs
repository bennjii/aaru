use std::any::Any;
use std::path::PathBuf;
use criterion::criterion_main;
use log::info;

use rayon::iter::ParallelIterator;
use aaru::codec::consts::DISTRICT_OF_COLUMBIA;
use aaru::codec::element::ProcessedElement;
use aaru::codec::{BlockIterator, Element, ElementIterator, Parallel, ProcessedElementIterator};

fn block_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let mut iter = BlockIterator::new(path).expect("Could not create iterator");

    iter.for_each(|item| {
        info!("Block: {:?}", item.type_id());
    });
}

fn element_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ElementIterator::new(path).expect("Could not create iterator");

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

fn processed_iter_count() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iter = ProcessedElementIterator::new(path).expect("Could not create iterator");

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
    c.bench_function("block_iter_count", |b| b.iter(|| block_iter_count()));
    c.bench_function("element_iter_count", |b| b.iter(|| element_iter_count()));
    c.bench_function("processed_iter_count", |b| b.iter(|| processed_iter_count()));
}

criterion::criterion_group!(standard_benches, sweep_benchmark);
criterion_main!(standard_benches);