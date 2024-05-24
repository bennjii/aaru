use std::fs::File;
use std::io::Read;
use either::Either;
use flate2::read::ZlibDecoder;
use log::{info, trace, warn};
use prost::Message;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use crate::codec::blob::item::BlobItem;
use crate::codec::block::iterator::BlockIterator;
use crate::codec::element::item::Element;
use crate::osm::{Blob, HeaderBlock, PrimitiveBlock};
use crate::osm::blob::Data;

pub enum BlockItem {
    HeaderBlock(HeaderBlock),
    PrimitiveBlock(PrimitiveBlock)
}

impl BlockItem {
    #[cfg(feature = "mmap")]
    #[inline]
    pub(crate) fn from_blob_item(blob: &BlobItem, mmap: &memmap2::Mmap) -> Option<Self> {
        trace!("Decoding blob: {}. Size: {}", blob.start, blob.item.datasize);
        let block_data = blob.data(mmap)?;
        BlockItem::from_raw(block_data, &blob)
    }

    #[cfg(not(feature = "mmap"))]
    #[inline]
    pub(crate) fn from_blob_item(blob: &BlobItem, file: &mut File) -> Option<Self> {
        trace!("Decoding blob: {}. Size: {}", blob.start, blob.item.datasize);
        let block_data = blob.data(file)?;
        BlockItem::from_raw(block_data.as_slice(), &blob)
    }

    #[inline]
    fn from_raw(data: &[u8], blob_item: &BlobItem) -> Option<Self> {
        trace!("Partial Block: {:?}", data[0..5].to_vec());
        let blob = Blob::decode(data).expect("Parse Failed");

        // Convert raw into actual. Handles ZLIB encoding.
        let data = BlockItem::from_blob(blob)?;
        BlockItem::from_data(data.as_slice(), blob_item)
    }

    #[inline]
    fn from_blob<'a>(blob: Blob) -> Option<Vec<u8>> {
        if let Some(Data::ZlibData(data)) = blob.data {
            return BlockItem::zlib_decode(data, blob.raw_size.unwrap_or(0) as usize)
        }

        warn!("Type {:?} not yet supported.", blob.data);
        None
    }

    #[inline]
    fn from_data(data: &[u8], blob: &BlobItem) -> Option<Self> {
        match blob.item.r#type.as_str() {
            "OSMData" => Some(BlockItem::PrimitiveBlock(PrimitiveBlock::decode(data).ok()?)),
            "OSMHeader" => Some(BlockItem::HeaderBlock(HeaderBlock::decode(data).ok()?)),
            _ => None
        }
    }

    #[inline]
    fn zlib_decode<'a>(data: Vec<u8>, raw_size: usize) -> Option<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(data.as_slice());
        let mut decoded = vec![0_u8; raw_size];
        decoder.read_exact(&mut decoded).ok()?;
        Some(decoded)
    }

    pub fn element_iter(&self) -> impl Iterator<Item=Element> {
         match self {
             BlockItem::PrimitiveBlock(primitive) => {
                 Either::Left(primitive.primitivegroup
                     .iter()
                     .flat_map(|group| Element::from_group(group)))
             }
             BlockItem::HeaderBlock(_) => Either::Right(std::iter::empty())
         }
    }

    pub fn par_iter(&mut self) -> impl ParallelIterator<Item=Element> + '_ {
        match self {
            BlockItem::PrimitiveBlock(primitive) => {
                Either::Left(primitive.primitivegroup
                    .par_iter()
                    .flat_map(|group| Element::from_group(group)))
            }
            BlockItem::HeaderBlock(_) => Either::Right(rayon::iter::empty())
        }
    }
}