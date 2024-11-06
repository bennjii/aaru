#![doc = include_str!("../../docs/route.md")]

#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod graph;
#[doc(hidden)]
mod test;
pub mod transition;
#[doc(inline)]
pub use graph::Graph;
