use std::fmt::Debug;

use crate::route::graph::NodeIx;
use crate::route::transition::candidate::{Candidate, Candidates, Collapse};
use crate::route::transition::costing::emission::EmissionStrategy;
use crate::route::transition::costing::transition::TransitionStrategy;
use crate::route::transition::layer::{LayerGenerator, Layers};
use crate::route::transition::{CostingStrategies, Solver};
use crate::route::Graph;
use geo::LineString;
use log::debug;
use measure_time::debug_time;

type LayerId = usize;
type NodeId = usize;

pub struct Transition<'a, E, T>
where
    E: EmissionStrategy,
    T: TransitionStrategy,
{
    pub(crate) map: &'a Graph,
    pub(crate) heuristics: CostingStrategies<E, T>,

    pub(crate) candidates: Candidates,
    pub(crate) layers: Layers,
}

#[derive(Debug)]
pub enum MatchError {
    CollapseFailure,
    NoPointsProvided,
}

struct InterpolatedNodes {
    pub node_idx: Vec<NodeIx>,
}

pub struct Match {
    pub cost: f64,

    /// Direct matches for each individual point in the initial trajectory.
    /// These are the new points, with associated routing information to
    /// aid in information recovery.
    pub matched: Vec<Candidate>,
    pub interpolated: LineString,
}

impl<'a, E, T> Transition<'a, E, T>
where
    E: EmissionStrategy + Send + Sync,
    T: TransitionStrategy + Send + Sync,
{
    pub fn new(
        map: &'a Graph,
        linestring: LineString,
        heuristics: CostingStrategies<E, T>,
    ) -> Transition<'a, E, T> {
        debug_time!("transition graph generation");

        let points = linestring.into_points();
        let generator = LayerGenerator::new(map, &heuristics);

        // Generate the layers and candidates.
        let (layers, candidates) = generator.with_points(&points);

        Transition {
            map,
            candidates,
            layers,
            heuristics,
        }
    }

    pub fn solve(self, solver: impl Solver) -> Result<Collapse, MatchError> {
        debug_time!("transition solve");
        // Indirection to call.
        solver.solve(self)
    }

    /// Backtracks the [HMM] from the most appropriate final point to
    /// its prior most appropriate points
    ///
    /// Consumes the transition structure in doing so.
    /// This is because it makes irreversible modifications
    /// to the candidate graph that put it in a collapsable position.
    ///
    /// [HMM]: Hidden Markov Model
    pub(crate) fn collapse(self) -> Result<Collapse, MatchError> {
        // Use the candidates to collapse the graph into a single route.
        let collapse = self
            .candidates
            .collapse()
            .ok_or(MatchError::CollapseFailure)?;

        debug!("Collapsed with final cost: {}", collapse.cost);
        Ok(collapse)
    }
}
