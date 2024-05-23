//! Iterates over `FileBlock`s in the file

use std::fs::File;
use std::io;
use std::path::PathBuf;
use log::warn;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use crate::codec::blob::item::BlobItem;
use crate::codec::blob::iterator::BlobIterator;
use crate::codec::block::item::FileBlock;

pub struct BlockIterator {
    blobs: Vec<BlobItem>,
    index: usize,
    #[cfg(feature = "mmap")]
    map: memmap2::Mmap,
    file: File
}

impl BlockIterator {
    pub fn new(path: PathBuf) -> Result<BlockIterator, io::Error> {
        let iter = BlobIterator::new(path)?;

        let file = iter.file.try_clone().expect("");

        #[cfg(feature = "mmap")]
        let map = unsafe { memmap2::Mmap::map(&file)? };

        #[cfg(feature = "mmap")]
        if let Err(err) = map.advise(memmap2::Advice::WillNeed) {
            warn!("Could not advise memory. Encountered: {}", err);
        }

        #[cfg(feature = "mmap")]
        if let Err(err) = map.advise(memmap2::Advice::Random) {
            warn!("Could not advise memory. Encountered: {}", err);
        }

        let blobs: Vec<BlobItem> = iter.collect();

        Ok(BlockIterator {
            blobs,
            index: 0,
            #[cfg(feature = "mmap")]
            map,
            file,
        })
    }
}

impl BlockIterator {
    pub fn par_iter(&mut self) -> impl ParallelIterator<Item=Option<FileBlock>> + '_ {
        self.blobs
            .par_iter()
            .map(|blob| {
                #[cfg(feature = "mmap")]
                return FileBlock::from_blob_item(blob, &self.map);
                #[cfg(not(feature = "mmap"))]
                return FileBlock::from_blob_item(blob, &mut self.file);
            })
    }
}

impl Iterator for BlockIterator {
    type Item = FileBlock;

    #[cfg(feature = "mmap")]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.blobs.len() {
            return None;
        }

        let blob_desc = &self.blobs[self.index];
        self.index += 1;
        FileBlock::from_blob_item(blob_desc, &self.map)
    }

    #[cfg(not(feature = "mmap"))]
    fn next(&mut self) -> Option<Self::Item> {
        let blob_desc = self.blobs[self.index];
        self.index += 1;
        FileBlock::from_blob_item(&blob_desc, &mut self.file)
    }
}