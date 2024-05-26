use crate::osm;

#[derive(Clone)]
pub struct Way {
    id: i64,
    refs: Vec<i64>
}

impl From<&osm::Way> for Way {
    fn from(value: &osm::Way) -> Self {
        Way {
            id: value.id,
            refs: value.refs.iter().map(|v| v.clone()).collect()
        }
    }
}