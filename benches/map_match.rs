use criterion::criterion_main;

fn target_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("match");
    group.significance_level(0.1).sample_size(30);

    group.bench_function("noop", |b| b.iter(|| {}));

    group.finish();
}

criterion::criterion_group!(targeted_benches, target_benchmark);
criterion_main!(targeted_benches);
