//! Iterates over `BlockItem`s in the file

use std::io;
use std::path::PathBuf;
use lending_iterator::HKT;
use lending_iterator::LendingIterator;
use log::info;
use rayon::iter::{ParallelIterator};
use rayon::prelude::ParallelBridge;
use std::borrow::BorrowMut;

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

// impl Parallel for BlockIterator {
//     type Item<'a> = BlockItem;
//
//     fn for_each<F>(self, f: F) -> ()
//     where
//         F: for<'a> Fn(Self::Item<'_>) + Send + Sync,
//     {
//         self.iter
//             .for_each(|blob| {
//                 if let Some(block) = self.take_blob(&blob) {
//                     f(block)
//                 }
//             })
//     }
//
//     fn map_red<Map, Reduce, Identity, T>(self, map_op: Map, red_op: Reduce, ident: Identity) -> T
//     where
//         Map: for<'a> Fn(Self::Item<'_>) -> T + Send + Sync,
//         Reduce: Fn(T, T) -> T + Send + Sync,
//         Identity: Fn() -> T + Send + Sync,
//         T: Send,
//     {
//         self.iter
//             .into_iter()
//             .par_bridge()
//             .filter_map(|blob| self.take_blob(&blob))
//             .map(|mut block| {
//                 block.raw_par_iter().map(&map_op).reduce(&ident, &red_op)
//             })
//             .reduce(
//                 &ident,
//                 &red_op,
//             )
//     }
//
//     fn par_red<Reduce, Identity, Combine, T>(self, fold_op: Reduce, combine: Combine, ident: Identity) -> T
//     where
//         Reduce: for<'a> Fn(T, Self::Item<'_>) -> T + Send + Sync,
//         Identity: Fn() -> T + Send + Sync,
//         Combine: Fn(T, T) -> T + Send + Sync,
//         T: Send,
//     {
//         self.iter
//             .into_iter()
//             .par_bridge()
//             .filter_map(|blob| self.take_blob(&blob))
//             .map(|mut block| {
//                 block.raw_par_iter().fold(&ident, &fold_op).reduce(&ident, &combine)
//             })
//             .reduce(&ident, &combine)
//     }
// }


impl BlockIterator {
    pub fn par_iter<'a>(mut self) -> impl ParallelIterator<Item=BlockItem> + 'a {
        let mut lended = self.iter;

        while let Some(next) = lended.next() {
            let block = BlockItem::from_blob_item(&next);
            info!("block::{}", block.is_some());
        }

        vec![].into_iter().par_bridge()

        // self.iter // let iterator: Vec<BlockItem> =
         //    .lend()
         //    .filter_map::<HKT!(BlockItem), _>(
         //        |_, blob| {
         //            BlockItem::from_blob_item(blob)
         //        }
         //    )
         //    .into_iter()
         //    .par_bridge()
    }
}

impl<'a> Iterator for BlockIterator {
    type Item = BlockItem;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(blob) => BlockItem::from_blob_item(&blob),
            None => None
        }
    }

    // #[cfg(not(feature = "mmap"))]
    // fn next(&mut self) -> Option<Self::Item> {
    //     let blob_desc = &self.blobs[self.index];
    //     self.index += 1;
    //     BlockItem::from_blob_item(blob_desc, &mut self.file)
    // }
}