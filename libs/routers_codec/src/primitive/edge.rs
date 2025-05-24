use std::fmt::Debug;

use crate::Entry;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Edge<E>
where
    E: Entry,
{
    pub source: E,
    pub target: E,
}
