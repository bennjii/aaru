#![doc = include_str!("../../docs/tile.md")]

pub mod datasource;
pub mod fragment;
pub mod layer;
pub mod repositories;

#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod mvt;

#[doc(inline)]
pub use datasource::query::Query;
#[doc(inline)]
pub use datasource::query::TileQuery;
#[doc(inline)]
pub use repositories::RepositorySet;
