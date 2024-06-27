#![doc = include_str!("../../docs/geo.md")]

pub const MVT_EXTENT: u32 = 4096;
pub const MVT_VERSION: u32 = 2;

#[doc(hidden)]
pub mod coord;
#[doc(hidden)]
pub mod error;
pub mod project;

#[doc(inline)]
pub use coord::latlng::LatLng;
#[doc(inline)]
pub use coord::point::Point;
#[doc(inline)]
pub use project::Project;