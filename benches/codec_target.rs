#[cfg(not(feature = "mmap"))]
use std::fs::File;
use std::path::PathBuf;
use criterion::criterion_main;
use log::{error};

use rayon::iter::ParallelIterator;
use aaru::codec::{BlockItem, BlockIterator};
use aaru::codec::consts::DISTRICT_OF_COLUMBIA;

fn iterate_blocks_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iterator = BlockIterator::new(path.clone());

    let mut primitive_blocks = 0;
    let mut header_blocks = 0;

    match iterator {
        Ok(iter) => {
            for block in iter {
                match block {
                    BlockItem::HeaderBlock(_) => header_blocks += 1,
                    BlockItem::PrimitiveBlock(_) => primitive_blocks += 1
                }
            }
        },
        Err(err) => {
            error!("Failed to load file, {:?}. Got error: {err}", path.as_os_str().to_str());
        }
    }

    assert_eq!(header_blocks, 1);
    assert_eq!(primitive_blocks, 237);
}

fn parallel_iterate_blocks_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);

    let mut block_iter = BlockIterator::new(path).unwrap();

    let elements = block_iter.par_iter()
        .map(|block| {
            match block {
                BlockItem::HeaderBlock(_) => (0, 1),
                BlockItem::PrimitiveBlock(_) => (1, 0)
            }
        })
        .reduce(
            || (0, 0),
            |a, b| (a.0 + b.0, a.1 + b.1)
        );

    assert_eq!(elements, (237, 1));
}

fn compare_to_osmpbf() {
    use osmpbf::{BlobReader, BlobType};

    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let reader = BlobReader::from_path(path).unwrap();

    let mut primitive_blocks = 0;
    let mut header_blocks = 0;

    for block in reader {
        match block {
            Ok(b) => {
                match b.get_type() {
                    BlobType::OsmHeader => {
                        if b.to_headerblock().is_ok() {
                            header_blocks += 1;
                        }
                    },
                    BlobType::OsmData => {
                        if b.to_primitiveblock().is_ok() {
                            primitive_blocks += 1;
                        }
                    }
                    _ => {}
                }
            },
            Err(_) => {}
        }
    }

    assert_eq!(header_blocks, 1);
    assert_eq!(primitive_blocks, 237);
}

fn target_benchmark(c: &mut criterion::Criterion) {
    c.bench_function("iterate_blocks_each", |b| b.iter(|| iterate_blocks_each()));
    c.bench_function("parallel_iterate_blocks_each", |b| b.iter(|| parallel_iterate_blocks_each()));
    c.bench_function("compared_to_osmpbf", |b| b.iter(|| compare_to_osmpbf()));
}

criterion::criterion_group!(targeted_benches, target_benchmark);
criterion_main!(targeted_benches);