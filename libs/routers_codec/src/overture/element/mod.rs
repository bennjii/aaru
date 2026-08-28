//! Overture feature elements: [`Connector`] (node) and [`Segment`] (edge chain).

pub mod connector;
pub mod segment;

pub use connector::Connector;
pub use segment::{Segment, SegmentConnector};
