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
pub use runtime::TraversalConditions;

// Protocol Buffer Includes
pub mod model {
    //! OpenStreetMaps Protobuf Definitions
    include!(concat!(env!("OUT_DIR"), "/osmpbf.rs"));
}

pub mod meta {
    use std::num::NonZeroU8;

    use crate::Metadata;
    use crate::osm::SpeedLimit;
    use crate::osm::element::{TagString, Tags};
    use crate::osm::primitives::*;
    use crate::osm::speed_limit::SpeedLimitCollection;

    #[derive(Debug, Clone, Default)]
    pub struct OsmEdgeMetadata {
        pub lane_count: Option<NonZeroU8>,
        pub speed_limit: Option<SpeedLimitCollection>,
        pub access_tag: Option<AccessTag>,
        pub road_class: Option<String>,
    }

    impl Metadata for OsmEdgeMetadata {
        type Raw<'a> = &'a Tags;

        fn pick(raw: Self::Raw<'_>) -> Self {
            Self {
                lane_count: raw.r#as::<NonZeroU8>(TagString::LANES),
                speed_limit: raw.speed_limit(),
                ..Self::default()
            }
        }
    }
}

pub mod runtime {
    use crate::osm::primitives::{Directionality, TransportMode};
    use std::num::NonZeroU8;

    // TODO: Internalise
    #[derive(Debug, Clone)]
    pub struct TraversalConditions {
        pub transport_mode: TransportMode,
        pub directionality: Directionality,
        pub lane: Option<NonZeroU8>,
    }
}
