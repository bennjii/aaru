use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use routers_network::edge::Weight;

/// The functional class of an Overture road segment.
///
/// Ramps and slip roads are not distinct classes: a motorway ramp is
/// `class: motorway` + `subclass: link`, carried separately on
/// [`Segment::is_link`](crate::overture::Segment).
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[repr(u8)]
pub enum RoadClass {
    /// A restricted access major divided highway. Equivalent to the
    /// Freeway, Autobahn, etc..
    Motorway,

    /// The most important roads in a country's system that aren't motorways.
    Trunk,

    /// The next most important roads in a country's system.
    /// (Often link larger towns.)
    Primary,

    /// The next most important roads in a country's system.
    /// (Often link towns.)
    Secondary,

    /// The next most important roads in a country's system.
    /// (Often link smaller towns and villages)
    Tertiary,

    /// The least important through roads in a country's system: minor roads
    /// of a lower classification than tertiary, but which serve a purpose
    /// other than access to properties.
    Unclassified,

    /// Roads which serve as access to housing, without function of
    /// connecting settlements. Often lined with housing.
    Residential,

    /// Residential streets where pedestrians have legal priority over cars,
    /// and speeds are kept very low.
    LivingStreet,

    /// Access roads to, or within an industrial estate, camp site, business
    /// park, car park, alleys, etc.
    Service,

    /// Roads used mainly or exclusively by pedestrians.
    Pedestrian,

    /// A minor pathway used mainly by pedestrians.
    Footway,

    /// Flights of steps on footpaths.
    Steps,

    /// An unspecified or shared-use path.
    Path,

    /// Roads for mostly agricultural or forestry uses.
    Track,

    /// A separated way for cyclists.
    Cycleway,

    /// A way intended for horse riders.
    Bridleway,

    /// A road of unknown classification.
    Unknown,
}

impl RoadClass {
    /// Whether a segment of this class belongs in the routable graph.
    /// Only motor-vehicle roadways are navigable.
    #[inline]
    pub const fn navigable(&self) -> bool {
        use RoadClass::*;
        matches!(
            self,
            Motorway
                | Trunk
                | Primary
                | Secondary
                | Tertiary
                | Unclassified
                | Residential
                | LivingStreet
                | Service
        )
    }

    /// Base routing weight for the class; lower is preferred.
    #[inline]
    pub const fn weighting(&self) -> Weight {
        use RoadClass::*;
        match self {
            Motorway => 1,
            Trunk => 3,
            Primary => 5,
            Secondary => 7,
            Tertiary => 9,
            Unclassified => 10,
            Residential => 10,
            LivingStreet => 50,
            Service => 50,

            // Non-navigable; never enters the graph, defined for completeness.
            Pedestrian | Footway | Steps | Path | Track | Cycleway | Bridleway | Unknown => 100,
        }
    }
}
