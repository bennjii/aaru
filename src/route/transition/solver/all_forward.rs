use crate::codec::element::variants::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::candidate::{CandidateEdge, CandidateId};
use crate::route::transition::graph::{MatchError, Transition};
use crate::route::transition::solver::methods::{Reachable, Solver};
use crate::route::transition::{
    Collapse, Costing, EmissionStrategy, RoutingContext, TransitionContext, TransitionStrategy,
    Trip,
};

use geo::{Distance, Haversine};
use pathfinding::prelude::{dijkstra_reach, DijkstraReachableItem};
use petgraph::Direction;
use rayon::iter::IntoParallelIterator;
use rayon::{iter::ParallelIterator, slice::ParallelSlice};
use std::collections::HashMap;
use std::hash::Hash;

const DEFAULT_THRESHOLD: f64 = 5_000f64;

type ProcessedReachable = (CandidateId, Reachable);

/// A Upper-Bounded Dijkstra (UBD) algorithm.
///
/// TODO: Docs
pub struct AllForwardSolver {
    /// The threshold by which the solver is bounded, in meters.
    threshold_distance: f64,
}

impl Default for AllForwardSolver {
    fn default() -> Self {
        Self {
            threshold_distance: DEFAULT_THRESHOLD,
        }
    }
}

impl AllForwardSolver {
    // Returns all the nodes reachable by the solver in an iterator, measured in distance
    fn reachable_iterator<'a>(
        ctx: RoutingContext<'a>,
        end: &'a NodeIx,
    ) -> impl Iterator<Item = DijkstraReachableItem<NodeIx, u32>> + use<'a> {
        dijkstra_reach(end, |node| {
            ctx.map
                .graph
                .edges_directed(*node, Direction::Outgoing)
                .map(|(_, next, _w)| {
                    (
                        next,
                        if *node != next {
                            let source = ctx.map.get_position(node).unwrap();
                            let target = ctx.map.get_position(&next).unwrap();

                            // In centimeters
                            (Haversine::distance(source, target) * 1_000f64) as u32
                        } else {
                            // Total accrued distance
                            0u32
                        },
                    )
                })
                .collect::<Vec<_>>()
        })
    }

    /// TODO: Docs
    ///
    /// Supplies an offset, which represents the initial distance
    /// taken in travelling initial edges, in meters.
    fn bounded_iterator<'a, 'b>(
        &'b self,
        ctx: RoutingContext<'a>,
        offset: f64,
        end: &'a NodeIx,
    ) -> impl Iterator<Item = DijkstraReachableItem<NodeIx, u32>> + use<'a, 'b> {
        Self::reachable_iterator(ctx, end).take_while(move |p| {
            let distance_in_meters = p.total_cost as f64 / 1_000f64;
            let total_cost = distance_in_meters + offset;

            // Bounded by the threshold distance (meters)
            total_cost < self.threshold_distance
        })
    }

    /// May return None if a cycle is detected.
    #[inline]
    pub(crate) fn path_builder<N, C>(target: &N, parents: &HashMap<N, (N, C)>) -> Vec<N>
    where
        N: Eq + Hash + Copy,
    {
        let mut rev = vec![*target];
        let mut next = target;
        while let Some((parent, _)) = parents.get(next) {
            rev.push(*parent);
            next = parent;
        }
        rev.reverse();
        rev
    }

    fn flatten<'a, E, T>(
        transition: &'a Transition<'a, E, T>,
    ) -> impl IntoParallelIterator<Item = (CandidateId, Vec<CandidateId>)> + use<'a, E, T>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        transition.layers.layers.par_windows(2).flat_map(|vectors| {
            // Taking all forward pairs of (left, [...right])
            let output = vectors[0]
                .nodes
                .iter()
                .map(|&a| (a, vectors[1].nodes.clone()))
                .collect::<Vec<_>>();

            output
        })
    }

    fn all_reachable<'a, E, T>(
        &'a self,
        context: RoutingContext<'a>,
        transition: &'a Transition<E, T>,
    ) -> impl ParallelIterator<Item = ProcessedReachable> + 'a
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        Self::flatten(transition)
            .into_par_iter()
            .map(move |(left, right)| (left, self.reachable(context, &left, right.as_slice())))
            .flat_map(move |(left, right)| {
                right
                    .unwrap_or_default()
                    .into_par_iter()
                    .map(move |reachable| (left, reachable))
            })
    }
}

impl Solver for AllForwardSolver {
    fn reachable<'a>(
        &self,
        ctx: RoutingContext<'a>,
        source: &CandidateId,
        targets: &'a [CandidateId],
    ) -> Option<Vec<Reachable>> {
        let left = ctx.candidate(source)?;

        // The distance remaining in the edge to travel
        // TODO: Explain why this is necessary.
        let end_node = left.edge.target;
        let end_position = ctx.map.get_position(&end_node)?;
        let edge_offset = Haversine::distance(left.position, end_position);

        // Upper-Bounded reachable map containing a Child:Parent relation
        // Note: Parent is OsmEntryId::NULL, which will not be within the map, indicating the root element.
        let predicate_map = self
            .bounded_iterator(ctx, edge_offset, &end_node)
            .map(|predicate| {
                (
                    predicate.node,
                    (
                        predicate.parent.unwrap_or(OsmEntryId::null()),
                        predicate.total_cost,
                    ),
                )
            })
            .collect::<HashMap<OsmEntryId, (OsmEntryId, u32)>>();

        let reachable = targets
            .iter()
            .filter_map(|target| {
                // Get the candidate information of the target found
                let candidate = ctx.candidate(target)?;
                // Generate the path to this target using the predicate map
                // TODO: Validate why the source of the edge in docs.
                let path_to_target = Self::path_builder(&candidate.edge.source, &predicate_map);

                Some(Reachable::new(*source, *target, path_to_target))
            })
            .collect::<Vec<_>>();

        Some(reachable)
    }

    fn solve<E, T>(&self, transition: Transition<E, T>) -> Result<Collapse, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        let context = RoutingContext {
            candidates: &transition.candidates,
            map: transition.map,
        };

        // Declaring all the pairs of indices that need to be refined.
        let transition_probabilities = self
            .all_reachable(context, &transition)
            .map(|(source, reachable)| {
                // Derive the transition cost of reaching this candidate
                let cost = transition.heuristics.transition(TransitionContext {
                    optimal_path: Trip::new_with_map(transition.map, reachable.path.as_slice()),
                    source_candidate: &source,
                    target_candidate: &reachable.target,
                    routing_context: context,
                });

                let edge = CandidateEdge::new(cost);
                (source, reachable.target, edge)
            })
            .collect::<Vec<_>>();

        // Scoped exclusive access to the graph.
        {
            // TODO: Can we bulk-add these / provide utility?
            let mut write_access = transition.candidates.graph.write().unwrap();
            for (source, target, edge) in transition_probabilities {
                write_access.add_edge(source, target, edge);
            }
        }

        transition.collapse()
    }
}
