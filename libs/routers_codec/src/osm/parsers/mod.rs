pub mod primitives;
pub mod speed_limit;

pub use speed_limit::SpeedLimit;

pub trait Parser: Sized {
    fn parse(tags: &crate::osm::Tags) -> Option<Self>;
}
