use std::fs::File;
use std::path::PathBuf;
use log::{error, info, warn};

use rayon::iter::{IntoParallelRefIterator, ParallelBridge};
use rayon::iter::ParallelIterator;

use crate::blob::item::BlobItem;
use crate::blob::iterator::BlobIterator;
use crate::codec::block::iterator::BlockIterator;
use crate::codec::block::item::BlockItem;
use crate::codec::consts::DISTRICT_OF_COLUMBIA;

#[test]
fn iterate_blobs_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iterator = BlobIterator::new(path.clone());

    match iterator {
        Ok(iter) => {
            for blob in iter {
                info!("Have blob: {}. Type: {}", blob.item.datasize, blob.item.r#type);
            }
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
            for block in iter {
                match block {
                    BlockItem::HeaderBlock(header) => header_blocks += 1,
                    BlockItem::PrimitiveBlock(primitive) => primitive_blocks += 1
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

#[test_log::test]
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
