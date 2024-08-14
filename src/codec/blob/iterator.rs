//! The file blob iterator
//! Supports `mmap` reading through the optional feature

use std::fs::File;
use std::io;
use std::io::{BufReader, Cursor};
use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use ::lending_iterator::prelude::*;
use prost::Message;

#[cfg(feature = "mmap")]
use log::warn;
use crate::codec::blob::item::BlobItem;
use crate::codec::BlockItem;
use crate::codec::osm::BlobHeader;

const HEADER_LEN_SIZE: usize = 4;

pub struct BlobIterator {
    pub(crate) buf: Vec<u8>,

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

        #[cfg(feature = "mmap")]
        if let Err(err) = map.advise(memmap2::Advice::WillNeed) {
            warn!("Could not advise memory. Encountered: {}", err);
        }

        let mut buf = vec![0; file.metadata()?.size() as usize];
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut buf)?;

        Ok(BlobIterator {
            #[cfg(feature = "mmap")]
            map,
            buf,
            offset: 0,
            index: 0,
        })
    }

    pub fn make_block(&self, blob: &BlobItem) -> Option<BlockItem> {
        #[cfg(feature = "mmap")]
        let block = BlockItem::from_blob_item(blob);
        #[cfg(not(feature = "mmap"))]
        let block = BlockItem::from_blob_item(blob);

        return block
    }
}

#[gat]
impl LendingIterator for BlobIterator {
    type Item<'next> where Self: 'next = BlobItem<'next>;

    fn next(self: &mut Self) -> Option<Item<'_, Self>> {
        if self.buf.len() < self.offset as usize + HEADER_LEN_SIZE {
            return None;
        }

        let header_len_buffer = &self.buf[self.offset as usize..self.offset as usize + HEADER_LEN_SIZE];
        self.offset += HEADER_LEN_SIZE as u64;

        // Translate to i32 (Big Endian)
        let blob_header_length = i32::from_be_bytes(header_len_buffer.try_into().unwrap()) as usize;

        if self.buf.len() < self.offset as usize + blob_header_length {
            return None;
        }

        let blob_header_buffer = &self.buf[self.offset as usize..self.offset as usize + blob_header_length];
        self.offset += blob_header_length as u64;

        let start = self.offset;
        let header = BlobHeader::decode(&mut Cursor::new(blob_header_buffer)).ok()?;
        self.offset += header.datasize as u64;

        let blob = BlobItem::new(start, header, &self.buf)?;
        self.index += 1;

        Some(blob)
    }
}


impl BlobIterator {
    #[cfg(feature = "mmap")]
    pub fn _next(&mut self) -> Option<BlobItem> {
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

        let blob = BlobItem::new(start, header, &self.buf)?;
        self.index += 1;

        Some(blob)
    }

    #[cfg(not(feature = "mmap"))]
    pub fn _next(&mut self) -> Option<BlobItem> {
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

        let blob = BlobItem::new(start, header, &mut self.file)?;
        self.index += 1;

        Some(blob)
    }
}