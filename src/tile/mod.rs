#![doc = include_str!("../../docs/tile.md")]

pub mod datasource;
pub mod repositories;
pub mod fragment;
pub mod layer;

#[doc(hidden)]
pub mod mvt;
#[doc(hidden)]
pub mod error;

#[doc(inline)]
pub use repositories::RepositorySet;
#[doc(inline)]
pub use datasource::query::Query;
#[doc(inline)]
pub use datasource::query::TileQuery;
