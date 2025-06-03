use crate::Entry;
use std::fmt::Debug;
use std::num::{NonZeroU8, NonZeroU16};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Edge<E>
where
    E: Entry,
{
    pub source: E,
    pub target: E,
}

#[derive(Default)]
pub struct GenericMetadata {
    pub lane_count: Option<NonZeroU8>,
    pub speed_limit: Option<NonZeroU16>,

    pub road_class: Option<String>,
}
