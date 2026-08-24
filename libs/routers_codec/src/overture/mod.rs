#![doc = include_str!("../../docs/overture.md")]

// Exposed modules
pub mod element;
pub mod graph;
pub mod id;
pub mod meta;
pub mod parsers;
pub mod runtime;

#[cfg(all(feature = "overture", not(target_arch = "wasm32")))]
pub mod reader;

// Hidden modules
#[doc(hidden)]
pub mod builder;
#[doc(hidden)]
pub mod error;

#[cfg(test)]
mod tests;

// Inlined structs
#[doc(inline)]
pub use element::{Connector, Segment, SegmentConnector};
#[doc(inline)]
pub use graph::OvertureNetwork;
#[doc(inline)]
pub use id::{Interner, OvertureEntryId};
#[doc(inline)]
pub use meta::OvertureEdgeMetadata;
#[doc(inline)]
pub use parsers::*;
#[doc(inline)]
pub use runtime::OvertureTripConfiguration;
