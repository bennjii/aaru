use crate::osm::element::Tags;
use crate::osm::primitives::{Directionality, TransportMode};
use std::num::NonZeroU8;

pub mod primitives;
pub mod speed_limit;

pub use speed_limit::SpeedLimit;

pub struct TraversalConditions {
    pub transport_mode: TransportMode,
    pub directionality: Directionality,
    pub lane: Option<NonZeroU8>,
}

pub trait Parser: Sized {
    fn parse(tags: &Tags) -> Option<Self>;
}
