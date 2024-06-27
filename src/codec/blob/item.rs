//! Describes the `BlobItem`, which holds the file reference for an `Element`

#[cfg(feature = "mmap")]
use std::cmp::min;
#[cfg(not(feature = "mmap"))]
use std::fs::File;
#[cfg(not(feature = "mmap"))]
use std::io::{Seek, SeekFrom, Read};

use crate::codec::osm::BlobHeader;

pub struct BlobItem {
    pub(crate) start: u64,
    pub item: BlobHeader,
}

impl BlobItem {
    pub(crate) fn new(start: u64, item: BlobHeader) -> Self {
        BlobItem {
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
