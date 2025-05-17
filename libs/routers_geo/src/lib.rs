pub const MVT_EXTENT: u32 = 4096;
pub const MVT_VERSION: u32 = 2;

pub const MEAN_EARTH_RADIUS: f64 = 6371008.8;
pub const SRID3857_MAX_LNG: u32 = 20026377;

pub mod cluster;
#[doc(hidden)]
pub mod coord;
#[doc(hidden)]
pub mod error;
pub mod project;

#[doc(inline)]
pub use coord::point::TileItem;
#[doc(inline)]
pub use project::Project;
