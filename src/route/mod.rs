#![doc = include_str!("../../docs/route.md")]

#[doc(hidden)]
pub mod graph;
#[doc(hidden)]
mod test;
#[doc(hidden)]
pub mod error;
pub mod transition;
#[doc(inline)]
pub use graph::Graph;
