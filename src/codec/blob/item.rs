//! Describes the `BlobItem`, which holds the file reference for an `Element`

use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use log::trace;
use crate::osm::BlobHeader;

pub(crate) struct BlobItem {
    index: u64,
    pub(crate) start: u64,
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

    #[cfg(not(feature = "mmap"))]
    pub(crate) fn data(&self, file: &mut File) -> Option<Vec<u8>> {
        file.seek(SeekFrom::Start(self.start)).ok()?;
        let mut blob_buffer = vec![0; self.item.datasize as usize];
        file.read_exact(blob_buffer.as_mut_slice()).ok()?;
        Some(blob_buffer)
    }

    #[cfg(feature = "mmap")]
    pub(crate) fn data<'a>(&self, map: &'a memmap2::Mmap) -> Option<&'a [u8]> {
        let start = self.start as usize;
        let end = min(start + self.item.datasize as usize, map.len());
        let blob_buffer = (*map).get(start..end)?;

        Some(blob_buffer)
    }
}
