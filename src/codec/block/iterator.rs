//! Iterates over `BlockItem`s in the file

use std::io;
use std::path::PathBuf;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use rayon::prelude::ParallelBridge;
use crate::codec::blob::item::BlobItem;
use crate::codec::blob::iterator::BlobIterator;
use crate::codec::block::item::BlockItem;
use crate::codec::Parallel;

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

impl Parallel for BlockIterator {
    type Item<'a> = BlockItem;

    fn for_each<F>(self, f: F) -> ()
    where
        F: for<'a> Fn(Self::Item<'_>) + Send + Sync,
    {
        self.iter
            .into_iter()
            .for_each(|blob| {
                if let Some(block) = self.take_blob(&blob) {
                    f(block)
                }
            })
    }

    fn map_red<Map, Reduce, Identity, T>(self, map_op: Map, red_op: Reduce, ident: Identity) -> T
    where
        Map: for<'a> Fn(Self::Item<'_>) -> T + Send + Sync,
        Reduce: Fn(T, T) -> T + Send + Sync,
        Identity: Fn() -> T + Send + Sync,
        T: Send,
    {
        self.iter
            .into_iter()
            .par_bridge()
            .filter_map(|blob| self.take_blob(&blob))
            .map(|mut block| {
                block.raw_par_iter().map(&map_op).reduce(&ident, &red_op)
            })
            .reduce(
                &ident,
                &red_op,
            )
    }

    fn par_red<Reduce, Identity, Combine, T>(self, fold_op: Reduce, combine: Combine, ident: Identity) -> T
    where
        Reduce: for<'a> Fn(T, Self::Item<'_>) -> T + Send + Sync,
        Identity: Fn() -> T + Send + Sync,
        Combine: Fn(T, T) -> T + Send + Sync,
        T: Send,
    {
        self.iter
            .into_iter()
            .par_bridge()
            .filter_map(|blob| self.take_blob(&blob))
            .map(|mut block| {
                block.raw_par_iter().fold(&ident, &fold_op).reduce(&ident, &combine)
            })
            .reduce(&ident, &combine)
    }
}

impl BlockIterator {
    pub fn take_blob(&self, blob: &BlobItem) -> Option<BlockItem> {
        #[cfg(feature = "mmap")]
        let block = BlockItem::from_blob_item(blob, &self.iter.map);
        #[cfg(not(feature = "mmap"))]
        let block = BlockItem::from_blob_item(blob, &self.iter.file);

        return block
    }
}

impl BlockIterator {
    pub fn par_iter(mut self) -> impl ParallelIterator<Item=BlockItem> + '_ {
        self.iter
            .into_iter()
            .par_bridge()
            .map(|blob| self.take_blob(&blob))
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
    }
}

impl Iterator for BlockIterator {
    type Item = BlockItem;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(blob) => self.take_blob(&blob),
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