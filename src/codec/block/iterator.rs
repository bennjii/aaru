//! Iterates over `FileBlock`s in the file

use std::io;
use std::path::PathBuf;
use crate::codec::blob::iterator::BlobIterator;
use crate::codec::block::item::FileBlock;

pub struct BlockIterator {
    blob_iterator: BlobIterator,
}

impl BlockIterator {
    pub fn new(path: PathBuf) -> Result<BlockIterator, io::Error> {
        let iter = BlobIterator::new(path)?;

        Ok(BlockIterator {
            blob_iterator: iter
        })
    }
}

impl Iterator for BlockIterator {
    type Item = FileBlock;

    #[cfg(feature = "mmap")]
    fn next(&mut self) -> Option<Self::Item> {
        let blob_desc = self.blob_iterator.next()?;
        FileBlock::from_blob_item(blob_desc, &mut self.blob_iterator.map)
    }

    #[cfg(not(feature = "mmap"))]
    fn next(&mut self) -> Option<Self::Item> {
        let blob_desc = self.blob_iterator.next()?;
        FileBlock::from_blob_item(blob_desc, &mut self.blob_iterator.file)
    }
}