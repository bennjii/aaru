//! The file blob iterator
//! Supports `mmap` reading through the optional feature

use std::fs::File;
use std::io;
use std::io::Cursor;
#[cfg(not(feature = "mmap"))]
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use log::{trace, warn};
use prost::Message;
use crate::codec::blob::item::BlobItem;
use crate::codec::osm::BlobHeader;

const HEADER_LEN_SIZE: usize = 4;

pub(crate) struct BlobIterator {
    pub(crate) file: File,

    #[cfg(feature = "mmap")]
    pub(crate) map: memmap2::Mmap,

    pub(crate) index: u64,
    offset: u64,
}

impl BlobIterator {
    pub fn new(path: PathBuf) -> Result<BlobIterator, io::Error> {
        let file = File::open(path)?;

        #[cfg(feature = "mmap")]
        let map = unsafe { memmap2::Mmap::map(&file)? };

        #[cfg(feature = "mmap")]
        if let Err(err) = map.advise(memmap2::Advice::Sequential) {
            warn!("Could not advise memory. Encountered: {}", err);
        }

        Ok(BlobIterator {
            #[cfg(feature = "mmap")]
            map,
            file,
            offset: 0,
            index: 0,
        })
    }
}

impl Iterator for BlobIterator {
    type Item = BlobItem;

    #[cfg(feature = "mmap")]
    fn next(&mut self) -> Option<Self::Item> {
        if self.map.len() < self.offset as usize + HEADER_LEN_SIZE {
            return None;
        }

        let header_len_buffer = &self.map[self.offset as usize..self.offset as usize + HEADER_LEN_SIZE];
        self.offset += HEADER_LEN_SIZE as u64;

        // Translate to i32 (Big Endian)
        let blob_header_length = i32::from_be_bytes(header_len_buffer.try_into().unwrap()) as usize;

        if self.map.len() < self.offset as usize + blob_header_length {
            return None;
        }

        let blob_header_buffer = &self.map[self.offset as usize..self.offset as usize + blob_header_length];
        self.offset += blob_header_length as u64;

        let start = self.offset;
        let header = BlobHeader::decode(&mut Cursor::new(blob_header_buffer)).ok()?;
        self.offset += header.datasize as u64;

        let blob = BlobItem::new(start, header);
        self.index += 1;

        Some(blob)
    }

    #[cfg(not(feature = "mmap"))]
    fn next(&mut self) -> Option<Self::Item> {
        // Move to the location of the item
        self.file.seek(SeekFrom::Start(self.offset)).ok()?;

        // Create a `Header` length buffer
        let mut header_len_buffer = [0_u8; 4];
        self.file.read_exact(&mut header_len_buffer).ok()?;
        self.offset += 4;

        // Translate to i32 (Big Endian)
        let blob_header_length = i32::from_be_bytes(header_len_buffer);

        // Create the actual header buffer
        let mut blob_header_buffer = vec![0; blob_header_length as usize];
        self.file.read_exact(&mut blob_header_buffer).ok()?;

        self.offset += blob_header_length as u64;

        let start = self.offset;
        let header = BlobHeader::decode(&mut Cursor::new(blob_header_buffer)).ok()?;
        self.offset += header.datasize as u64;

        let blob = BlobItem::new(self.index, start, header);
        self.index += 1;

        Some(blob)
    }
}