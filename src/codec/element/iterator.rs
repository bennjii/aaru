//! Iterator over all primitive entities in the structure
//! ignoring header blocks

use std::array::from_fn;
use std::{io, iter};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::slice::Iter;
use either::Either;
use log::{info, warn};
use osmpbfreader::osmformat::PrimitiveGroup;
use rayon::iter::ParallelIterator;

use crate::codec::block::iterator::BlockIterator;
use crate::codec::element::item::Element;

pub struct ElementIterator {
    iter: BlockIterator,
    index: i32
}

impl ElementIterator {
    pub fn new(path: PathBuf) -> Result<ElementIterator, io::Error> {
        Ok(ElementIterator {
            iter: BlockIterator::new(path)?,
            index: 0,
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
}



