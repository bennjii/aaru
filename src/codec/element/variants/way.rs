//! Describes the minimal `Way` structure.
//! Has methods for accessing appropriate
//! tags for graph representation.

use std::borrow::Cow;
use petgraph::visit::Walker;

use crate::codec::osm;
use crate::codec::osm::PrimitiveBlock;

#[derive(Clone)]
pub struct Way {
    id: i64,
    road_tag: Option<String>,
    refs: Vec<i64>,
}

impl Way {
    pub fn is_road(&self) -> bool {
        self.road_tag.is_some()
    }

    pub fn refs(&self) -> &Vec<i64> {
        &self.refs
    }

    pub fn r#type(&self) -> &Option<String> {
        &self.road_tag
    }

    pub fn from_raw(value: &osm::Way, block: &PrimitiveBlock) -> Self {
        Way {
            id: value.id,
            refs: value.refs.iter().fold(vec![], |mut prior, current| {
                prior.push(current + prior.last().unwrap_or(&0i64));
                prior
            }),
            road_tag: value.road_tag(block),
        }
    }
}

fn make_string(k: usize, block: &PrimitiveBlock) -> String {
    let cow = String::from_utf8_lossy(&*block.stringtable.s[k]);

    match cow {
        Cow::Borrowed(s) => String::from(s),
        Cow::Owned(s) => String::from(s),
    }
}

const VALID_ROADWAYS: [&str; 12] = [
    "motorway",
    "motorway_link",
    "trunk",
    "trunk_link",
    "primary",
    "primary_link",
    "secondary",
    "secondary_link",
    "tertiary",
    "tertiary_link",
    "residential",
    "living_street",
];

impl osm::Way {
    pub fn road_tag(&self, block: &PrimitiveBlock) -> Option<String> {
        self.keys
            .iter()
            .zip(self.vals.iter())
            .map(|(&k, &v)| (make_string(k as usize, block), make_string(v as usize, block)))
            .find(|(key, value)| key == "highway" && VALID_ROADWAYS.contains(&value.as_str()))
            .map(|(_, value)| value)
    }
}
