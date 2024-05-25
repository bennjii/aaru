//! Iterator over all primitive entities in the structure
//! ignoring header blocks

use std::path::PathBuf;
use rayon::iter::{ParallelIterator};

use crate::codec::block::iterator::BlockIterator;
use crate::codec::element::item::Element;
use crate::codec::error::CodecError;

pub struct ElementIterator {
    iter: BlockIterator,
}

impl ElementIterator {
    pub fn new(path: PathBuf) -> Result<ElementIterator, CodecError> {
        Ok(ElementIterator {
            iter: BlockIterator::new(path)?,
        })
    }

    pub fn for_each<F>(mut self, f: F) -> ()
        where
            F: for<'a> Fn(Element<'a>) + Send + Sync,
    {
        self.iter.par_iter().for_each(|mut block| {
            block.par_iter().for_each(&f);
        })
    }

    pub fn map_red<Map, Reduce, Identity, T>(mut self, map_op: Map, red_op: Reduce, ident: Identity) -> T
        where
            Map: for<'a> Fn(Element<'a>) -> T + Send + Sync,
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
                &red_op
            )
    }
}



