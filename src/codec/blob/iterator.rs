//! The file blob iterator
//! Supports `mmap` reading through the optional feature

use std::fs::File;
use std::io;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;
use prost::Message;
use crate::blob::item::BlobItem;
use crate::osm::BlobHeader;

pub(crate) struct BlobIterator {
    #[cfg(not(feature = "mmap"))]
    file: File,
    #[cfg(feature = "mmap")]
    map: memmap2::Mmap,
    offset: u64,
    index: u64,
}

impl BlobIterator {
    pub fn new(path: PathBuf) -> Result<BlobIterator, io::Error> {
        let file = File::open(path)?;

        #[cfg(feature = "mmap")]
        let map = unsafe { memmap2::Mmap::map(&file)? };

        Ok(BlobIterator {
            #[cfg(not(feature = "mmap"))]
            file,
            #[cfg(feature = "mmap")]
            map,
            offset: 0,
            index: 0,
        })
    }
}

impl Iterator for BlobIterator {
    type Item = BlobItem;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(feature = "mmap")]
        self.map.advise(memmap2::Advice::Sequential)?;

        // Move to the location of the item
        #[cfg(not(feature = "mmap"))]
        self.file.seek(SeekFrom::Start(self.offset)).ok()?;

        // Create a `Header` length buffer
        #[cfg(not(feature = "mmap"))]
        let mut header_len_buffer = [0_u8; 4];
        #[cfg(not(feature = "mmap"))]
        self.file.read_exact(&mut header_len_buffer).ok()?;
        #[cfg(feature = "mmap")]
        let mut header_len_buffer = self.map[self.offset..4];
        self.offset += 4;

        // Translate to i32 (Big Endian)
        let blob_header_length = i32::from_be_bytes(header_len_buffer);

        // Create the actual header buffer
        #[cfg(not(feature = "mmap"))]
        let mut blob_header_buffer = vec![0; blob_header_length as usize];
        #[cfg(not(feature = "mmap"))]
        self.file.read_exact(&mut blob_header_buffer).ok()?;
        #[cfg(feature = "mmap")]
        let mut blob_header_buffer = self.map[self.offset..(blob_header_length as u64)];
        self.offset += blob_header_length as u64;

        let header = BlobHeader::decode(&mut Cursor::new(blob_header_buffer)).ok()?;
        self.offset += header.datasize as u64;

        let blob = BlobItem::new(self.index, self.offset, header);
        self.index += 1;

        Some(blob)
    }
}