use std::fmt::Debug;
use std::hash::Hash;

pub mod edge;
pub mod node;

pub use edge::Edge;
pub use node::Node;

// TODO: No domain-specific in primitive.
use crate::osm::element::Tags;

pub trait Entry:
    Default + Copy + Clone + PartialEq + Eq + Ord + Hash + Debug + Send + Sync
{
    fn identifier(&self) -> i64;
}

pub trait Metadata: Clone + Debug + Send + Sync {
    fn pick(&self, tags: &Tags) -> Self;
}
