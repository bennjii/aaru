//! Iterates over `FileBlock`s in the file

use std::fs::File;
use std::io;
use std::path::PathBuf;
use log::warn;
use rayon::iter::ParallelIterator;
use crate::codec::blob::iterator::BlobIterator;
use crate::codec::block::item::FileBlock;

pub struct BlockIterator {
    blob_iterator: BlobIterator,
    #[cfg(feature = "mmap")]
    map: memmap2::Mmap
}

impl BlockIterator {
    pub fn new(path: PathBuf) -> Result<BlockIterator, io::Error> {
        let iter = BlobIterator::new(path)?;

        #[cfg(feature = "mmap")]
        let map = unsafe { memmap2::Mmap::map(&iter.file)? };

        #[cfg(feature = "mmap")]
        if let Err(err) = map.advise(memmap2::Advice::Sequential) {
            warn!("Could not advise memory. Encountered: {}", err);
        }

        Ok(BlockIterator {
            blob_iterator: iter,
            #[cfg(feature = "mmap")]
            map
        })
    }
}

impl Iterator for BlockIterator {
    type Item = FileBlock;

    #[cfg(feature = "mmap")]
    fn next(&mut self) -> Option<Self::Item> {
        let blob_desc = self.blob_iterator.next()?;
        FileBlock::from_blob_item(&blob_desc, &self.map)
    }

    #[cfg(not(feature = "mmap"))]
    fn next(&mut self) -> Option<Self::Item> {
        let blob_desc = self.blob_iterator.next()?;
        FileBlock::from_blob_item(&blob_desc, &mut self.blob_iterator.file)
    }
}


// impl ParallelIterator for BlockIterator {
//     type Item = FileBlock;
//
//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//         where
//             C: UnindexedConsumer<Self::Item>,
//     {
//         bridge_unindexed(BlockIterator {
//             blob_iterator: self.blob_iterator,
//         }, consumer)
//     }
// }

// impl UnindexedConsumer<I> for BlockIterator {
//     fn to_reducer(&self) -> Self::Reducer {
//         todo!()
//     }
//
//     fn split_off_left(&self) -> Self {
//         todo!()
//     }
// }