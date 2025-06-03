use crate::Entry;
use std::fmt::Debug;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Edge<E>
where
    E: Entry,
{
    pub source: E,
    pub target: E,
}
