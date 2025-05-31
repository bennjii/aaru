use crate::Graph;
use crate::Match;
use crate::transition::*;

use codec::{Entry, Metadata};
use geo::LineString;
use log::info;
use std::sync::Arc;

impl<E, M> Match<E, M> for Graph<E, M>
where
    E: Entry,
    M: Metadata,
{
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    fn r#match(&self, linestring: LineString) -> Result<RoutedPath<E, M>, MatchError> {
        info!("Finding matched route for {} positions", linestring.0.len());
        let costing = CostingStrategies::default();

        // Create our hidden markov model solver
        let transition = Transition::new(self, linestring, costing);

        // Yield the transition layers of each level
        // & Collapse the layers into a final vector
        let cache = Arc::clone(&self.cache);
        let solver = SelectiveForwardSolver::default().use_cache(cache);

        transition
            .solve(solver)
            .map(|collapsed| RoutedPath::new(collapsed, self))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = Level::INFO))]
    fn snap(&self, _linestring: LineString) -> Result<RoutedPath<E, M>, MatchError> {
        unimplemented!()
    }
}
