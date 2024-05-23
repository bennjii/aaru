use std::fs::File;
use std::path::PathBuf;
use log::{error, info, warn};
use memmap2::Advice;
use osmpbf::{BlobReader, BlobType};
use rayon::iter::{IntoParallelRefIterator, ParallelBridge};
use rayon::iter::ParallelIterator;

use crate::blob::item::BlobItem;
use crate::blob::iterator::BlobIterator;
use crate::codec::block::iterator::BlockIterator;
use crate::codec::block::item::FileBlock;

const DISTRICT_OF_COLUMBIA: &str = "./resources/district-of-columbia.osm.pbf";
const BADEN_WUERTTEMBERG: &str = "./resources/baden-wuerttemberg-latest.osm.pbf";
const AUSTRALIA: &str = "./resources/australia-latest.osm.pbf";

#[test_log::test]
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

#[test_log::test]
fn iterate_blocks_each() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);
    let iterator = BlockIterator::new(path.clone());

    let mut primitive_blocks = 0;
    let mut header_blocks = 0;

    match iterator {
        Ok(iter) => {
            for block in iter {
                match block {
                    FileBlock::HeaderBlock(header) => header_blocks += 1,
                    FileBlock::PrimitiveBlock(primitive) => primitive_blocks += 1
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
fn iterate_blocks_each_parallel() {
    let path = PathBuf::from(DISTRICT_OF_COLUMBIA);

    let blob_iter = BlobIterator::new(path.clone());
    let blobs: Vec<BlobItem> = blob_iter.unwrap().collect();

    let file = File::open(path).expect("Couldn't open file");
    let mut map = unsafe { memmap2::Mmap::map(&file).expect("Couldn't open file") };
    if let Err(err) = map.advise(memmap2::Advice::WillNeed) {
        warn!("Could not advise memory. Encountered: {}", err);
    }

    if let Err(err) = map.advise(memmap2::Advice::Random) {
        warn!("Could not advise memory. Encountered: {}", err);
    }

    let elements = blobs.par_iter()
        .map(|blob| {
            let block = FileBlock::from_blob_item(&blob, &map);

            match block {
                Some(blk) => {
                    match blk {
                        FileBlock::HeaderBlock(_) => (0, 1),
                        FileBlock::PrimitiveBlock(_) => (1, 0)
                    }
                }
                None => (0, 0)
            }
        })
        .reduce(
            || (0, 0),
            |a, b| (a.0 + b.0, a.1 + b.1)
        );

    assert_eq!(elements.1, 1);
    assert_eq!(elements.0, 237);
}

#[test_log::test]
fn compare_to_osmpbf() {
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
