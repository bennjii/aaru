use routers_network::Direction;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

/// A directional constraint relative to a segment's linestring order.
///
/// Overture's `when.heading` enum has exactly these two values; the absence
/// of a heading on a rule means it applies both ways.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum Heading {
    /// Along the segment: first connector → last connector.
    Forward,
    /// Against the segment: last connector → first connector.
    Backward,
}

impl Heading {
    /// Maps a heading onto the graph's edge [`Direction`]: forward edges are
    /// `Outgoing` (source → target in linestring order), backward edges are
    /// `Incoming`, matching how the builder tags each directed edge.
    #[inline]
    pub const fn direction(&self) -> Direction {
        match self {
            Heading::Forward => Direction::Outgoing,
            Heading::Backward => Direction::Incoming,
        }
    }
}
