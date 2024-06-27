#![doc = include_str!("../../docs/geo.md")]

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