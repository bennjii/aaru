//! Describes the `BlobItem`, which holds the file reference for an `Element`

#[cfg(feature = "mmap")]
use std::cmp::min;
#[cfg(not(feature = "mmap"))]
use std::fs::File;
#[cfg(not(feature = "mmap"))]
use std::io::{Seek, SeekFrom, Read};
use crate::codec::osm::BlobHeader;

pub struct BlobItem<'a> {
    pub(crate) start: u64,
    pub(crate) data: &'a [u8],
    pub item: BlobHeader,
}

impl BlobItem<'_> {
    #[cfg(not(feature = "mmap"))]
    #[inline]
    pub(crate) fn new(start: u64, item: BlobHeader, file: &mut File) -> Option<BlobItem> {
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut blob_buffer = vec![0; item.datasize as usize];
        file.read_exact(blob_buffer.as_mut_slice()).ok()?;

        Some(BlobItem {
            start,
            item,
            data: blob_buffer.as_slice()
        })
    }

    #[cfg(feature = "mmap")]
    #[inline]
    pub(crate) fn new<'a>(start: u64, item: BlobHeader, map: &'a Vec<u8>) -> Option<BlobItem> {
        let u_start = start as usize;
        let end = min(u_start + item.datasize as usize, map.len());
        let blob_buffer = (*map).get(u_start..end)?;

        Some(BlobItem {
            start,
            item,
            data: blob_buffer
        })
    }
}
