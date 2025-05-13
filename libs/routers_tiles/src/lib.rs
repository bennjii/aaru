pub mod mvt {
    //! MapboxVectorTile Protobuf Definitions
    include!(concat!(env!("OUT_DIR"), "/mvt.rs"));
}

pub mod datasource;
pub mod fragment;
pub mod layer;
pub mod repositories;

#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod vector_tile;

#[doc(inline)]
pub use datasource::query::Query;
#[doc(inline)]
pub use datasource::query::TileQuery;
#[doc(inline)]
pub use repositories::RepositorySet;
