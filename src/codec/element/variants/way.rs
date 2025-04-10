//! Describes the minimal `Way` structure.
//! Has methods for accessing appropriate
//! tags for graph representation.

use crate::codec::osm::PrimitiveBlock;
use crate::codec::{osm, relation::MemberType};

use super::common::{OsmEntryId, ReferenceKey, References, Referential, Tagable, Tags};

#[derive(Clone, Debug)]
pub struct Way {
    // TODO: Use this in routing so attributes like roadnames, etc. can be used when recollecting and returning a response metric
    id: OsmEntryId,
    refs: References,
    tags: Tags,
}

impl Way {
    pub fn id(&self) -> OsmEntryId {
        self.id
    }

    #[inline]
    pub fn tags(&self) -> &Tags {
        &self.tags
    }

    #[inline]
    pub fn refs(&self) -> &References {
        &self.refs
    }

    #[inline]
    pub fn from_raw(value: &osm::Way, block: &PrimitiveBlock) -> Self {
        Way {
            id: OsmEntryId::as_way(value.id),
            refs: value.references(block),
            tags: value.tags(block),
        }
    }
}

impl Tagable for osm::Way {
    fn indices(&self) -> impl Iterator<Item = (&u32, &u32)> {
        self.keys.iter().zip(self.vals.iter())
    }
}

impl Referential for osm::Way {
    fn indices(&self) -> impl Iterator<Item = ReferenceKey> {
        self.refs
            .iter()
            .map(|id| (&-1i32, id, &(MemberType::Node as i32)))
    }
}
