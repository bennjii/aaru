//! Iterator over all primitive entities in the structure
//! ignoring header blocks

use std::path::PathBuf;
use rayon::iter::{ParallelIterator};

use crate::codec::block::iterator::BlockIterator;
use crate::codec::element::item::Element;
use crate::codec::error::CodecError;
use crate::element::item::ProcessedElement;
use crate::parallel::Parallel;

pub struct ProcessedElementIterator {
    iter: BlockIterator,
}

impl ProcessedElementIterator {
    pub fn new(path: PathBuf) -> Result<ProcessedElementIterator, CodecError> {
        Ok(ProcessedElementIterator {
            iter: BlockIterator::new(path)?,
        })
    }
}

impl Parallel for ProcessedElementIterator {
    type Item<'a> = ProcessedElement;

    fn for_each<F>(mut self, f: F) -> ()
        where
            F: Fn(ProcessedElement) + Send + Sync,
    {
        self.iter.par_iter().for_each(|mut block| {
            block.par_iter().for_each(&f);
        })
    }

    fn map_red<Map, Reduce, Identity, T>(mut self, map_op: Map, red_op: Reduce, ident: Identity) -> T
        where
            Map: Fn(ProcessedElement) -> T + Send + Sync,
            Reduce: Fn(T, T) -> T + Send + Sync,
            Identity: Fn() -> T + Send + Sync,
            T: Send
    {
        self.iter
            .par_iter().map(|mut block| {
            block.par_iter().map(&map_op).reduce(&ident, &red_op)
        })
            .reduce(
                &ident,
                &red_op,
            )
    }

    fn par_red<Reduce, Identity, Combine, T>(mut self, fold_op: Reduce, ident: Identity, combine: Combine) -> T
        where
            Reduce: Fn(T, ProcessedElement) -> T + Send + Sync,
            Identity: Fn() -> T + Send + Sync,
            Combine: Fn(T, T) -> T + Send + Sync,
            T: Send
    {
        self.iter
            .par_iter().map(|mut block| {
            block.par_iter().fold(&ident, &fold_op).reduce(&ident, &combine)
        })
            .reduce(&ident, &combine)
    }
}
//
// impl Parallel<Element<'_>> for ElementIterator {
//     fn for_each<F>(mut self, f: F) -> ()
//         where
//             F: Fn(Element) + Send + Sync,
//     {
//         self.iter.par_iter().for_each(|mut block| {
//             block.raw_par_iter().for_each(&f);
//         })
//     }
//
//     fn map_red<Map, Reduce, Identity, T>(
//         mut self,
//         map_op: Map,
//         red_op: Reduce,
//         ident: Identity
//     ) -> T
//         where
//             Map: Fn(Element) -> T + Send + Sync,
//             Reduce: Fn(T, T) -> T + Send + Sync,
//             Identity: Fn() -> T + Send + Sync,
//             T: Send
//     {
//         self.iter
//             .par_iter().map(|mut block| {
//             block.raw_par_iter().map(&map_op).reduce(&ident, &red_op)
//         })
//             .reduce(
//                 &ident,
//                 &red_op,
//             )
//     }
//
//     fn par_red<Reduce, Identity, Combine, T>(
//         mut self,
//         fold_op: Reduce,
//         ident: Identity,
//         combine: Combine
//     ) -> T
//         where
//             Reduce: Fn(T, Element) -> T + Send + Sync,
//             Identity: Fn() -> T + Send + Sync,
//             Combine: Fn(T, T) -> T + Send + Sync,
//             T: Send
//     {
//         self.iter
//             .par_iter().map(|mut block| {
//             block.raw_par_iter().fold(&ident, &fold_op).reduce(&ident, &combine)
//         })
//             .reduce(&ident, &combine)
//     }
// }