//! Describes the `BlobItem`, which holds the file reference for an `Element`

use crate::osm::BlobHeader;

pub(crate) struct BlobItem {
    index: u64,
    start: u64,
    pub item: BlobHeader,
}

impl BlobItem {
    pub(crate) fn new(index: u64, start: u64, item: BlobHeader) -> Self {
        BlobItem {
            index,
            start,
            item
        }
    }
}
