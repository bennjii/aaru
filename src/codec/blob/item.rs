//! Describes the `BlobItem`, which holds the file reference for an `Element`

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
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

    #[cfg(not(feature = "mmap"))]
    pub(crate) fn data(&self, file: &mut File) -> Option<Vec<u8>> {
        file.seek(SeekFrom::Start(self.start)).ok()?;
        let mut blob_buffer = vec![0; self.item.datasize as usize];
        file.read_exact(blob_buffer.as_mut_slice()).ok()?;
        Some(blob_buffer)
    }

    #[cfg(feature = "mmap")]
    pub(crate) fn data(&self, map: &mut memmap2::Mmap) -> Option<&[u8]> {
        let blob_buffer = &map[
            self.start as usize..self.start as usize + self.item.datasize as usize
        ];
        Some(blob_buffer)
    }
}
