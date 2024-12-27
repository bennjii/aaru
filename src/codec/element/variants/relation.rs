use crate::codec::osm;

use super::common::{ReferenceKey, References, Referentiable, Tagable, Tags};

#[derive(Clone, Debug)]
pub struct Relation {
    pub id: i64,
    pub tags: Tags,
    pub refs: References,
}

impl Relation {
    pub fn from_raw(relation: &osm::Relation, block: &osm::PrimitiveBlock) -> Self {
        Self {
            id: relation.id,
            tags: relation.tags(block),
            refs: relation.references(block),
        }
    }
}

impl Tagable for osm::Relation {
    fn indicies<'a>(&'a self) -> impl Iterator<Item = (&'a u32, &'a u32)> {
        self.keys.iter().zip(self.vals.iter())
    }
}

impl Referentiable for osm::Relation {
    fn indicies<'a>(&'a self) -> impl Iterator<Item = ReferenceKey<'a>> {
        self.roles_sid
            .iter()
            .zip(self.memids.iter())
            .zip(self.types.iter())
            .map(|(e, v)| (e.0, e.1, v))
    }
}
