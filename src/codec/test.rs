#![cfg(test)]

#[cfg(not(feature = "mmap"))]
use std::fs::File;
use std::path::PathBuf;
use lending_iterator::LendingIterator;
use log::{error, info};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::codec::blob::iterator::BlobIterator;
use crate::codec::block::iterator::BlockIterator;
use crate::codec::block::item::BlockItem;
use crate::codec::consts::DISTRICT_OF_COLUMBIA;

#[test]
fn iterate_blobs_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iterator = BlobIterator::new(path.clone());

    match iterator {
        Ok(iter) => {
            iter.filter_map_into_iter::<_, String>(|blob| {
                Some(format!("Have blob: {}. Type: {}", blob.item.datasize, blob.item.r#type))
            }).map(|v| println!("{}", v)).collect()
        },
        Err(err) => {
            error!("Failed to load file, {:?}. Got error: {err}", path.as_os_str().to_str());
        }
    }

    info!("Test Complete.");
}

#[test]
fn iterate_blocks_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iterator = BlockIterator::new(path.clone());

    let mut primitive_blocks = 0;
    let mut header_blocks = 0;

    match iterator {
        Ok(iter) => {
            for block in iter.iter() {
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

#[test]
fn parallel_iterate_blocks_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);

    let mut block_iter = BlockIterator::new(path).unwrap();

    let elements = block_iter.into_par_iter()
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

#[test]
fn compare_to_osmpbf() {
    use osmpbf::{BlobReader, BlobType};

    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let reader = BlobReader::from_path(path).unwrap();

    println!("Counting...");

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
