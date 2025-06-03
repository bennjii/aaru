use crate::Entry;
use std::fmt::Debug;
use crate::osm::primitives::SpeedValue;
use crate::osm::TraversalConditions;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Edge<E>
where
    E: Entry,
{
    pub source: E,
    pub target: E,
}