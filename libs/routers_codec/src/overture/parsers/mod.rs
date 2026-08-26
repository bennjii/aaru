//! Overture attribute parsers and domain types.
//!
//! Overture data is structured, so these types model its typed attributes
//! (`class`, `speed_limits`, `access_restrictions`, …) directly. String
//! values parse via [`FromStr`](core::str::FromStr) (strum).

pub mod access;
pub mod heading;
pub mod linear_ref;
pub mod road_class;
pub mod speed;
pub mod travel_mode;

pub use access::{AccessRestriction, AccessType};
pub use heading::Heading;
pub use linear_ref::Between;
pub use road_class::RoadClass;
pub use speed::{Speed, SpeedLimit, SpeedUnit};
pub use travel_mode::TravelMode;
