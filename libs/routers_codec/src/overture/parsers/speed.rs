use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use super::heading::Heading;
use super::travel_mode::TravelMode;

/// The unit a raw speed value is expressed in.
///
/// Overture's schema permits only `km/h` and `mph`.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr,
)]
pub enum SpeedUnit {
    #[strum(to_string = "km/h", serialize = "kmh", serialize = "kph")]
    Kmh,
    #[strum(to_string = "mph")]
    Mph,
}

/// A speed value with its unit.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Speed {
    pub value: u32,
    pub unit: SpeedUnit,
}

impl Speed {
    #[inline]
    pub fn new(value: u32, unit: SpeedUnit) -> Self {
        Self { value, unit }
    }

    /// The speed normalised to whole km/h.
    #[inline]
    pub fn in_kmh(&self) -> u32 {
        match self.unit {
            SpeedUnit::Kmh => self.value,
            SpeedUnit::Mph => (self.value as f64 * 1.609_344).round() as u32,
        }
    }
}

/// A single speed-limit rule from a segment's `speed_limits` array.
///
/// `between`-style linear-referenced scoping is deliberately omitted for
/// now; a rule is treated as applying to the whole segment.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SpeedLimit {
    pub max_speed: Option<Speed>,
    pub min_speed: Option<Speed>,
    /// Directional scope; `None` applies both ways.
    pub heading: Option<Heading>,
    /// Modes this limit applies to; empty means all.
    pub mode: Vec<TravelMode>,
    /// Whether the rule's `when` scope carries conditions beyond
    /// heading/mode (time window, purpose, status, vehicle dimensions).
    /// Conditional limits hold only under circumstances a consumer must
    /// evaluate itself.
    pub conditional: bool,
}
