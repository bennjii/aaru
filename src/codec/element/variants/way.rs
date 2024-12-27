//! Describes the minimal `Way` structure.
//! Has methods for accessing appropriate
//! tags for graph representation.

use crate::codec::osm::PrimitiveBlock;
use crate::codec::{osm, relation::MemberType};

use super::common::{
    OsmEntryId, Reference, ReferenceKey, References, Referentiable, Tagable, Tags,
};

#[derive(Clone, Debug)]
pub struct Way {
    // TODO: Use this in routing so attributes like roadnames, etc. can be used when recollecting and returning a response metric
    id: i64,
    refs: References,
    tags: Tags,
}

impl Way {
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
        let refs: Vec<Reference> = value
            .refs
            .iter()
            .fold(vec![], |mut prior, current| {
                let index = current + prior.last().unwrap_or(&0i64);
                prior.push(index);
                prior
            })
            .into_iter()
            // All nodes in a Way are `Node` types, therefore navigable.
            .map(|index| Reference::without_role(OsmEntryId::as_node(index)))
            .collect::<Vec<_>>();

        Way {
            id: value.id,
            refs: References::from(refs),
            tags: value.tags(block),
        }
    }
}

impl Tagable for osm::Way {
    fn indicies<'a>(&'a self) -> impl Iterator<Item = (&'a u32, &'a u32)> {
        self.keys.iter().zip(self.vals.iter())
    }
}

impl Referentiable for osm::Way {
    fn indicies<'a>(&'a self) -> impl Iterator<Item = ReferenceKey<'a>> {
        self.refs
            .iter()
            .map(|id| (&-1i32, id, &(MemberType::Node as i32)))
    }
}
