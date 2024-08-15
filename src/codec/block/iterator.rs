//! Iterates over `BlockItem`s in the file

use std::io;
use std::path::PathBuf;
use rayon::iter::ParallelIterator;
use crate::codec::blob::iterator::BlobIterator;
use crate::codec::BlobItem;
use crate::codec::block::item::BlockItem;

pub struct BlockIterator {
    iter: BlobIterator,
    // blobs: Vec<BlobItem>,
    index: usize,
}

impl BlockIterator {
    pub fn new(path: PathBuf) -> Result<BlockIterator, io::Error> {
        let iter = BlobIterator::new(path)?;
        // let blobs = iter.collect::<Vec<_>>();

        Ok(BlockIterator {
            index: 0,
            iter,
            // blobs
        })
    }
}

// impl BlockIterator {
//     pub fn iter<'a>(self) -> impl Iterator<Item=BlockItem> + 'a {
//         self.iter
//             .filter_map_into_iter::<_, BlockItem>(|blob|
//                 BlockItem::from_blob_item(&blob, &self.iter.buf)
//             )
//     }
// }

impl Iterator for BlockIterator {
    type Item = BlockItem;

    fn next(&mut self) -> Option<Self::Item> {
        BlockItem::from_blob_item(&self.iter.next()?, &self.iter.buf)
    }
}

// impl ParallelIterator for BlockIterator {
//     fn next(&mut self) -> Option<Self::Item> {
//
//     }
// }

// impl ParallelIterator for BlockIterator {
//     type Item = BlockItem;
//
//     fn drive_unindexed<C>(self, consumer: C) -> C::Result
//     where
//         C: UnindexedConsumer<BlockItem> + Consumer<BlockItem>,
//     {
//         let (sender, receiver) = channel::unbounded();
//
//         // Spawn a thread to feed items into the channel
//         std::thread::spawn(move || {
//             let mut raw = self.into_iter();
//
//             for item in raw.by_ref() {
//                 if sender.send(item).is_err() {
//                     break;
//                 }
//             }
//         });
//
//         receiver.into_iter().par_bridge().drive_unindexed(consumer)
//     }
// }

// impl<'a> Iterator for BlockIterator {
//     type Item = BlockItem;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         match self.iter.next() {
//             Some(blob) => BlockItem::from_blob_item(&blob),
//             None => None
//         }
//     }
//
//     // #[cfg(not(feature = "mmap"))]
//     // fn next(&mut self) -> Option<Self::Item> {
//     //     let blob_desc = &self.blobs[self.index];
//     //     self.index += 1;
//     //     BlockItem::from_blob_item(blob_desc, &mut self.file)
//     // }
// }