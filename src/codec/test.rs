use std::path::PathBuf;
use log::{error, info};
use crate::blob::item::BlobItem;
use crate::blob::iterator::BlobIterator;

const DISTRICT_OF_COLUMBIA: &str = "./resources/district-of-columbia.osm.pbf";
const BADEN_WUERTTEMBERG: &str = "./resources/baden-wuerttemberg-latest.osm.pbf";
const AUSTRALIA: &str = "./resources/australia-latest.osm.pbf";

#[test_log::test]
fn iterate_each() {
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
fn continuity() {
    info!("LOGGING-OUT");
}
