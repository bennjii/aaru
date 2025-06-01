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
