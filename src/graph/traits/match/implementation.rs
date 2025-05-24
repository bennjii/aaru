use crate::Graph;
use crate::r#match::definition::Match;
use crate::transition::*;

use codec::Entry;
use geo::LineString;
use log::info;
use std::sync::Arc;

impl<E> Match<E> for Graph<E>
where
    E: Entry,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    fn map_match(&self, linestring: LineString) -> Result<Collapse<E>, MatchError> {
        info!("Finding matched route for {} positions", linestring.0.len());

        let costing = CostingStrategies::default();

        // Create our hidden markov model solver
        let transition = Transition::new(self, linestring, costing);

        // Yield the transition layers of each level
        // & Collapse the layers into a final vector
        let cache = Arc::clone(&self.cache);
        transition.solve(SelectiveForwardSolver::default().use_cache(cache))
    }
}
