use crate::osm::element::Tags;

pub mod primitives;
pub mod speed_limit;

pub trait Parser<Return>: Sized {
    fn parse(tags: Tags) -> Option<Return>;
}
