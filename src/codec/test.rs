use std::path::PathBuf;
use log::{error, info};
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