use std::fs::File;
use std::io::Read;
use flate2::read::ZlibDecoder;
use log::{info, trace, warn};
use prost::Message;
use crate::codec::blob::item::BlobItem;
use crate::osm::{Blob, HeaderBlock, PrimitiveBlock};
use crate::osm::blob::Data;

pub enum FileBlock {
    HeaderBlock(HeaderBlock),
    PrimitiveBlock(PrimitiveBlock)
}

impl FileBlock {
    #[cfg(feature = "mmap")]
    pub(crate) fn from_blob_item(blob: BlobItem, mmap: &mut memmap2::Mmap) -> Option<Self> {
        let block_data = blob.data(mmap)?;
        FileBlock::from_raw(block_data, blob)
    }

    #[cfg(not(feature = "mmap"))]
    pub(crate) fn from_blob_item(blob: BlobItem, file: &mut File) -> Option<Self> {
        let block_data = blob.data(file)?;
        FileBlock::from_raw(block_data.as_slice(), blob)
    }

    fn from_raw(data: &[u8], blob_item: BlobItem) -> Option<Self> {
        let blob = Blob::decode(data).expect("Parse Failed");

        // Convert raw into actual. Handles ZLIB encoding.
        let data = FileBlock::from_blob(blob)?;
        FileBlock::from_data(data.as_slice(), blob_item)
    }

    fn from_blob<'a>(blob: Blob) -> Option<Vec<u8>> {
        if let Some(Data::ZlibData(data)) = blob.data {
            return FileBlock::zlib_decode(data, blob.raw_size.unwrap_or(0) as usize)
        }

        warn!("Type {:?} not yet supported.", blob.data);
        None
    }

    fn from_data(data: &[u8], blob: BlobItem) -> Option<Self> {
        match blob.item.r#type.as_str() {
            "OSMData" => Some(FileBlock::PrimitiveBlock(PrimitiveBlock::decode(data).ok()?)),
            "OSMHeader" => Some(FileBlock::HeaderBlock(HeaderBlock::decode(data).ok()?)),
            _ => None
        }
    }

    fn zlib_decode<'a>(data: Vec<u8>, raw_size: usize) -> Option<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(data.as_slice());
        let mut decoded = vec![0_u8; raw_size];
        decoder.read_exact(&mut decoded).ok()?;
        Some(decoded)
    }
}