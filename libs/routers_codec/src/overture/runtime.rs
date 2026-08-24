//! Runtime trip configuration for Overture routing.
//!
//! The `Runtime` associated type of
//! [`OvertureEdgeMetadata`](crate::overture::meta::OvertureEdgeMetadata):
//! the conditions `accessible` is evaluated against, expressed in the
//! [`TravelMode`] vocabulary.

use crate::overture::parsers::TravelMode;

/// The mode and permissions a traversal is evaluated under.
#[derive(Debug, Clone, PartialEq)]
pub struct OvertureTripConfiguration {
    /// The mode of travel; access restrictions are filtered against it.
    ///
    /// Default is [`TravelMode::Vehicle`] (any vehicle).
    pub travel_mode: TravelMode,

    /// Whether privately-signed roads may be traversed. Default `false`.
    pub allow_private_roads: bool,
}

impl Default for OvertureTripConfiguration {
    #[inline]
    fn default() -> Self {
        Self {
            travel_mode: TravelMode::Vehicle,
            allow_private_roads: false,
        }
    }
}
