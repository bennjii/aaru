#![doc = include_str!("../../docs/osm.md")]

// Exposed modules
pub mod blob;
pub mod block;
pub mod element;

pub mod parsers;

// Hidden modules
#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod parallel;
#[doc(hidden)]
pub mod test;

// Inlined structs
#[doc(inline)]
pub use blob::iterator::BlobIterator;
#[doc(inline)]
pub use block::iterator::BlockIterator;
#[doc(inline)]
pub use element::OsmEntryId;
#[doc(inline)]
pub use element::iterator::ElementIterator;
#[doc(inline)]
pub use element::processed_iterator::ProcessedElementIterator;

// Doc-Linking
#[doc(inline)]
pub use parallel::Parallel;

#[doc(hidden)]
pub use element::variants::common::*;
#[doc(hidden)]
pub use model::*;
#[doc(inline)]
pub use parsers::*;

#[doc(hidden)]
pub use blob::item::BlobItem;
#[doc(hidden)]
pub use block::item::BlockItem;
#[doc(hidden)]
pub use element::item::Element;

#[doc(inline)]
pub use meta::OsmEdgeMetadata;
#[doc(inline)]
pub use runtime::RuntimeTraversalConfig;

// Protocol Buffer Includes
pub mod model {
    //! OpenStreetMaps Protobuf Definitions
    include!(concat!(env!("OUT_DIR"), "/osmpbf.rs"));
}

pub mod meta {
    use crate::Metadata;
    use crate::osm::access_tag::AccessTag;
    use crate::osm::access_tag::access::AccessValue;
    use crate::osm::element::{TagString, Tags};
    use crate::osm::primitives::*;
    use crate::osm::speed_limit::SpeedLimitCollection;
    use crate::osm::{Access, RuntimeTraversalConfig, SpeedLimit};
    use crate::primitive::edge::Direction;
    use itertools::Itertools;
    use std::num::NonZeroU8;

    #[derive(Debug, Clone, Default)]
    pub struct OsmEdgeMetadata {
        pub lane_count: Option<NonZeroU8>,
        pub speed_limit: Option<SpeedLimitCollection>,
        pub access: Vec<AccessTag>,
        pub road_class: Option<RoadClass>,
    }

    impl Metadata for OsmEdgeMetadata {
        type Raw<'a> = &'a Tags;
        type RuntimeRouting = RuntimeTraversalConfig;

        fn pick(raw: Self::Raw<'_>) -> Self {
            Self {
                road_class: raw.r#as::<RoadClass>(TagString::HIGHWAY),
                lane_count: raw.r#as::<NonZeroU8>(TagString::LANES),
                speed_limit: raw.speed_limit(),
                access: raw.access(),
            }
        }

        fn runtime() -> Self::RuntimeRouting {
            RuntimeTraversalConfig::default()
        }

        #[inline]
        fn accessible(&self, conditions: &Self::RuntimeRouting, direction: Direction) -> bool {
            // Computes the negative-filter access restriction, assuming accessible by default.
            // If any access conditions match the input, it will be rejected.
            self.access
                .iter()
                .filter(|AccessTag { restriction, .. }| {
                    // Only consider access methods which are applicable
                    conditions
                        .transport_mode
                        .is_restricted_by(restriction.transport_mode)
                })
                .filter(
                    |AccessTag { restriction, .. }| match restriction.directionality {
                        Directionality::Forward => direction == Direction::Outgoing,
                        Directionality::Backward => direction == Direction::Incoming,
                        Directionality::BothWays => true,
                        _ => false,
                    },
                )
                .sorted_by_key(|AccessTag { restriction, .. }| {
                    // Sort by specificity such that we consider the most specific
                    // filter first, and the least specific last.
                    restriction.transport_mode.specificity_level()
                })
                .next()
                .map(|AccessTag { access, .. }| {
                    // We default to `true`, since a roadway is considered accessible
                    // unless otherwise specified. If any access tag disallows access
                    // up the specificity hierarchy, we will return `false`.
                    match access {
                        AccessValue::Yes => true,
                        AccessValue::Private => conditions.allow_private_roads,
                        _ => false,
                    }
                })
                .unwrap_or(true)
        }
    }
}

pub mod runtime {
    use crate::osm::primitives::TransportMode;
    use crate::osm::primitives::condition::VehicleProperty;
    use crate::osm::primitives::opening_hours::TimeOfWeek;

    #[derive(Debug, Clone)]
    pub struct RuntimeTraversalConfig {
        /// The transport mode by which a vehicle is travelling.
        /// This is used in order to validate access to ways,
        /// as well as for collecting metadata in order to produce
        /// an output which is relevant for the traversal.
        ///
        /// Default is `Vehicle`
        pub transport_mode: TransportMode,

        /// Properties of the travelling vehicle, allows filtering
        /// for accurate routing between regions based on conditions
        /// such as vehicle weight, length, number of wheels, etc.
        ///
        /// Default is `None`.
        pub vehicle_properties: Option<Vec<(VehicleProperty, f64)>>,

        /// An optionally specifiable time of week at which the
        /// traversal occurs. This allows filtering for conditions
        /// which specify specific hours for which access is permitted,
        /// or allowing for accurate metadata.
        ///
        /// Default is `None`.
        pub time_of_week: Option<TimeOfWeek>,

        /// Describes if the solver should consider private
        /// roadways. These often require the owners permission,
        /// and should be considered for transports which frequently
        /// visit private roadways or require routing within mapped
        /// private residences.
        ///
        /// Default is `false`.
        pub allow_private_roads: bool,
    }

    impl Default for RuntimeTraversalConfig {
        #[inline]
        fn default() -> Self {
            RuntimeTraversalConfig {
                transport_mode: TransportMode::Vehicle,
                vehicle_properties: None,
                time_of_week: None,
                allow_private_roads: false,
            }
        }
    }
}
