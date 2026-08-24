use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use super::heading::Heading;
use super::travel_mode::TravelMode;

/// The kind of access a restriction grants or removes.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum AccessType {
    /// Travel is legally permitted.
    Allowed,
    /// Travel is legally prohibited.
    Denied,
    /// The way is designated for the scoped modes.
    Designated,
}

/// A single rule from a segment's `access_restrictions` array.
///
/// Overture has **no `oneway` boolean**: directionality is expressed here,
/// as a `Denied` restriction scoped to a [`Heading`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AccessRestriction {
    pub access_type: AccessType,
    /// Directional scope; `None` applies both ways.
    pub heading: Option<Heading>,
    /// Modes this restriction applies to; empty means all.
    pub mode: Vec<TravelMode>,
    /// Whether the rule's `when` scope carries conditions beyond
    /// heading/mode — a time window (`during`), purpose (`using`),
    /// recognised status (`recognized`) or vehicle dimensions (`vehicle`).
    ///
    /// Conditional rules are never applied as blanket rules: a denial such
    /// as "denied to vehicles over 4.2m" must not close a road to all
    /// traffic. Evaluating the conditions themselves is a follow-up.
    pub conditional: bool,
}

impl AccessRestriction {
    /// Whether this restriction denies motor-vehicle travel in the given
    /// heading — the condition that makes a segment one-way for routing.
    ///
    /// Conditional denials do not close a direction; they hold only under
    /// circumstances the graph cannot assume.
    pub fn denies_driving(&self, heading: Heading) -> bool {
        !self.conditional
            && self.access_type == AccessType::Denied
            && self.heading == Some(heading)
            && (self.mode.is_empty() || self.mode.iter().any(TravelMode::is_motor_traffic))
    }
}
