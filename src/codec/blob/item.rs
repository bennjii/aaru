//! Describes the `BlobItem`, which holds the file reference for an `Element`

use std::ops::Range;
use crate::codec::osm::BlobHeader;

pub struct BlobItem {
    pub(crate) range: Range<usize>,
    pub header: BlobHeader,
}

impl BlobItem {
    #[inline]
    pub(crate) fn new(start: usize, header: BlobHeader) -> Option<BlobItem> {
        let end = start + (header.datasize as u64) as usize;

        Some(BlobItem {
            range: start..end,
            header,
        })
    }
}
