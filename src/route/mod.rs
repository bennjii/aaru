#![doc = include_str!("../../docs/route.md")]

#[doc(hidden)]
pub mod graph;
#[doc(hidden)]
mod test;
#[doc(hidden)]
pub mod error;
mod mapmatch;

#[doc(inline)]
pub use graph::Graph;
