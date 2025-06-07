pub mod collection;
pub mod limit;
pub mod restriction;
#[cfg(test)]
mod test;

use crate::osm::element::Tags;
use crate::osm::speed_limit::limit::PossiblyConditionalSpeedLimit;
use crate::osm::{Parser, TraversalConditions};
pub use collection::SpeedLimitCollection;

pub(super) mod subtypes {
    pub const LANES: &str = "lanes";

    pub const CONDITION_PATTERN: &str = r"\(([^)]+)\)";
    pub const VALUE_PATTERN: &str = r"^\s*(\d+)(?:\s*([^\s(]+))?";
}

pub trait SpeedLimit {
    fn speed_limit(&self) -> Option<SpeedLimitCollection>;
}

impl SpeedLimit for Tags {
    fn speed_limit(&self) -> Option<SpeedLimitCollection> {
        SpeedLimitCollection::parse(self)
    }
}

pub trait SpeedLimitExt {
    fn relevant_limits(
        &self,
        traversal_conditions: TraversalConditions,
    ) -> Vec<PossiblyConditionalSpeedLimit>;
}
