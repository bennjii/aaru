//! Iterates over `BlockItem`s in the file

use std::io;
use std::path::PathBuf;
use lending_iterator::HKT;
use lending_iterator::LendingIterator;
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::prelude::ParallelBridge;
use std::borrow::BorrowMut;
use std::sync::Arc;
use crossbeam::channel;
use rayon::iter::plumbing::{Consumer, UnindexedConsumer};
use crate::codec::blob::iterator::BlobIterator;
use crate::codec::BlobItem;
use crate::codec::block::item::BlockItem;

pub struct BlockIterator {
    iter: BlobIterator,
    index: usize,
}

impl BlockIterator {
    pub fn new(path: PathBuf) -> Result<BlockIterator, io::Error> {
        let iter = BlobIterator::new(path)?;

        Ok(BlockIterator {
            index: 0,
            iter,
        })
    }
}

impl BlockIterator {
    pub fn iter<'a>(mut self) -> impl Iterator<Item=BlockItem> + 'a {
        self.iter
            .filter_map_into_iter::<_, BlockItem>(|blob| BlockItem::from_blob_item(&blob))
    }
}

impl ParallelIterator for BlockIterator {
    type Item = BlockItem;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<BlockItem> + Consumer<BlockItem>,
    {
        let (sender, receiver) = channel::unbounded();

        // Spawn a thread to feed items into the channel
        std::thread::spawn(move || {
            let mut raw = self.iter();

            for item in raw.by_ref() {
                if sender.send(item).is_err() {
                    break;
                }
            }
        });

        receiver.into_iter().par_bridge().drive_unindexed(consumer)
    }
}

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