//! Describes the minimal `Way` structure.
//! Has methods for accessing appropriate
//! tags for graph representation.

use std::borrow::Cow;
use petgraph::visit::Walker;

use crate::osm;
use crate::osm::PrimitiveBlock;

#[derive(Clone)]
pub struct Way {
    id: i64,
    road_tag: Option<String>,
    refs: Vec<i64>
}

impl Way {
    pub fn refs(&self) -> &Vec<i64> {
        &self.refs
    }

    pub fn r#type(&self) -> &Option<String> {
        &self.road_tag
    }

    pub fn from_raw(value: &osm::Way, block: &PrimitiveBlock) -> Self {
        Way {
            id: value.id,
            refs: value.refs.iter().map(|v| v.clone()).collect(),
            road_tag: value.road_tag(block)
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

impl osm::Way {
    pub fn road_tag(&self, block: &PrimitiveBlock) -> Option<String> {
        self.keys
            .iter()
            .zip(self.vals.iter())
            .map(|(&k, &v)| (make_string(k as usize, block), make_string(v as usize, block)))
            .find(|(key, value)| key == "highway")
            .map(|(key, value)| value)
    }
}
