#![doc = include_str!("../../docs/codec.md")]

// Exposed modules
pub mod element;
pub mod blob;
pub mod block;

// Hidden modules
#[doc(hidden)]
pub mod parallel;
#[doc(hidden)]
pub mod test;
#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod consts;

// Inlined structs
#[doc(inline)]
pub use element::processed_iterator::ProcessedElementIterator;
#[doc(inline)]
pub use element::iterator::ElementIterator;
#[doc(inline)]
pub use block::iterator::BlockIterator;
#[doc(inline)]
pub use blob::iterator::BlobIterator;

// Doc-Linking
#[doc(hidden)]
pub use osm::*;
#[doc(hidden)]
pub use blob::item::BlobItem;
#[doc(hidden)]
pub use block::item::BlockItem;
#[doc(hidden)]
pub use element::item::Element;
#[doc(hidden)]
pub use crate::geo::coord::latlng::LatLng;
#[doc(inline)]
pub use parallel::Parallel;

// Protocol Buffer Includes
pub mod osm {
    //! OpenStreetMaps Protobuf Definitions
    include!(concat!(env!("OUT_DIR"), "/osmpbf.rs"));
}

pub mod mvt {
    //! MapboxVectorTile Protobuf Definitions
    include!(concat!(env!("OUT_DIR"), "/mvt.rs"));
}
