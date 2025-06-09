use std::fmt::Debug;
use std::hash::Hash;

pub mod edge;
pub mod node;

pub use edge::Edge;
pub use node::Node;

pub trait Entry:
    Default + Copy + Clone + PartialEq + Eq + Ord + Hash + Debug + Send + Sync
{
    fn identifier(&self) -> i64;
}

pub trait Metadata: Clone + Debug + Send + Sync {
    type Raw<'a>
    where
        Self: 'a;

    type RuntimeRouting: Debug;

    fn pick(raw: Self::Raw<'_>) -> Self;
    fn runtime() -> Self::RuntimeRouting;

    fn accessible(&self, access: &Self::RuntimeRouting) -> bool;
}
