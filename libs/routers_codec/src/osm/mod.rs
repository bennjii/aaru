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

use std::num::{NonZeroU16, NonZeroU8};
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
#[doc(hidden)]
pub use blob::item::BlobItem;
#[doc(hidden)]
pub use block::item::BlockItem;
#[doc(hidden)]
pub use element::item::Element;
#[doc(hidden)]
pub use model::*;
#[doc(inline)]
pub use parallel::Parallel;
#[doc(inline)]
pub use parsers::*;

// Protocol Buffer Includes
pub mod model {
    //! OpenStreetMaps Protobuf Definitions
    include!(concat!(env!("OUT_DIR"), "/osmpbf.rs"));
}

pub mod meta {
    use std::num::NonZeroU8;

    use crate::Metadata;
    use crate::osm::element::{TagString, Tags};
    use crate::osm::primitives::{AccessTag, Directionality, SpeedValue, TransportMode};
    use crate::osm::{SpeedLimit, TraversalConditions};

    // TODO: Rename
    #[derive(Debug, Clone, Default)]
    pub struct OsmEdgeMetadata {
        pub lane_count: Option<NonZeroU8>,
        pub speed_limit: Option<SpeedValue>,
        pub access_tag: Option<AccessTag>,
        pub road_class: Option<String>,
    }

    impl Metadata for OsmEdgeMetadata {
        fn pick(&self, tags: &Tags) -> Self {
            Self {
                lane_count: tags.r#as::<NonZeroU8>(TagString::LANES),
                speed_limit: tags
                    .speed_limit(TraversalConditions {
                        directionality: Directionality::BothWays,
                        transport_mode: TransportMode::MotorVehicle,
                        lane: None,
                    }),
                ..Self::default()
            }
        }
    }
}