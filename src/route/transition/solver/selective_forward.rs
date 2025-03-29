use crate::codec::element::variants::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::candidate::CandidateId;
use crate::route::transition::graph::{MatchError, Transition};
use crate::route::transition::solver::methods::{Reachable, Solver};
use crate::route::transition::{
    CandidateEdge, Collapse, Costing, EmissionStrategy, RoutingContext, TransitionContext,
    TransitionStrategy, Trip,
};

use geo::{Distance, Haversine};
use log::{debug, info};
use pathfinding::prelude::{astar, dijkstra_reach, DijkstraReachableItem};
use petgraph::prelude::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
#[cfg(feature = "tracing")]
use tracing::Level;

const DEFAULT_THRESHOLD: f64 = 2_000f64;

type ProcessedReachable = (CandidateId, Reachable);

/// A Upper-Bounded Dijkstra (UBD) algorithm.
///
/// TODO: Docs
pub struct SelectiveForwardSolver {
    /// The threshold by which the solver is bounded, in meters.
    threshold_distance: f64,
}

impl Default for SelectiveForwardSolver {
    fn default() -> Self {
        Self {
            threshold_distance: DEFAULT_THRESHOLD,
        }
    }
}

impl SelectiveForwardSolver {
    // Returns all the nodes reachable by the solver in an iterator, measured in distance
    fn reachable_iterator<'a>(
        ctx: RoutingContext<'a>,
        end: &'a NodeIx,
    ) -> impl Iterator<Item = DijkstraReachableItem<NodeIx, u32>> + use<'a> {
        dijkstra_reach(end, |node, _| {
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
    #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::DEBUG, skip(self)))]
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
}

impl Solver for SelectiveForwardSolver {
    // #[cfg_attr(feature = "tracing", tracing::instrument(level = Level::DEBUG, skip(self, ctx)))]
    fn reachable<'a>(
        &self,
        ctx: RoutingContext<'a>,
        source: &CandidateId,
        targets: &'a [CandidateId],
    ) -> Option<Vec<Reachable>> {
        // debug!("Searching for {} reachable nodes from candidate {:?}", targets.len(), source);
        let left = ctx.candidate(source)?;
        // debug!("Left candidate: {:?}", left);

        // The distance remaining in the edge to travel
        // TODO: Explain why this is necessary.
        let end_node = left.map_edge.1;

        // debug!("End node (map node index): {:?}", end_node);
        let end_position = ctx.map.get_position(&end_node)?;
        // debug!("End position (from map): {:?}", end_position);

        let edge_offset = Haversine::distance(left.position, end_position);
        // debug!("Looking for {end_node:?} (from {:?}) at {end_position:?}, at offset {edge_offset:?}", left.map_edge.0);

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

        // debug!("Generated a predicate map of size {}", predicate_map.len());

        let reachable = targets
            .iter()
            .filter_map(|target| {
                // debug!("({source:?}) Target={:?}", target);

                // Get the candidate information of the target found
                let candidate = ctx.candidate(target)?;
                // debug!("({source:?}) Reachable Candidate={:?}", candidate);

                // Generate the path to this target using the predicate map
                // TODO: Validate why the source of the edge in docs.
                let path_to_target = Self::path_builder(&candidate.map_edge.0, &predicate_map);
                Some(Reachable::new(*target, path_to_target))
            })
            .collect::<Vec<_>>();

        // debug!("Found {} reachable targets of {} targets.", reachable.len(), targets.len());
        Some(reachable)
    }

    fn solve<E, T>(&self, mut transition: Transition<E, T>) -> Result<Collapse, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        info!("Solving...");

        let (start, end) = transition
            .candidates
            .attach_ends(&transition.layers)
            .ok_or(MatchError::CollapseFailure)?;

        debug!(
            "Start={start:?}. End={end:?}. Candidates: {:?}",
            transition.candidates
        );

        transition.candidates.weave(&transition.layers);

        debug!("Linked / Weaved all layers!");

        let context = RoutingContext {
            candidates: &transition.candidates,
            map: transition.map,
        };

        let graph_ref = Arc::clone(&transition.candidates.graph);
        let Some((path, cost)) = astar(
            &start,
            |source| {
                let successors = graph_ref
                    .read()
                    .unwrap()
                    .edges_directed(*source, Direction::Outgoing)
                    .map(|edge| edge.target())
                    .collect::<Vec<CandidateId>>();

                // #[cold]
                if *source == start {
                    // No cost to reach a first node.
                    return successors
                        .into_iter()
                        .map(|candidate| (candidate, CandidateEdge::zero()))
                        .collect::<Vec<_>>();
                }

                if successors.contains(&end) {
                    // Fast-track to the finish line
                    return vec![(end, CandidateEdge::zero())];
                }

                let source_owned = *source;
                let reached = self
                    .reachable(context, source, successors.as_slice())
                    .unwrap_or_default()
                    .into_iter()
                    .map(|reachable| {
                        let cost = transition.heuristics.transition(TransitionContext {
                            optimal_path: Trip::new_with_map(
                                transition.map,
                                reachable.path.as_slice(),
                            ),
                            source_candidate: &source_owned,
                            target_candidate: &reachable.candidate,
                            routing_context: context,
                        });

                        (
                            reachable.candidate,
                            CandidateEdge::new(cost, &reachable.path),
                        )
                    })
                    .collect::<Vec<_>>();

                // debug!("Reachable from {source:?}: {reached:?}");

                reached
            },
            |_| CandidateEdge::zero(),
            |node| *node == end,
        ) else {
            return Err(MatchError::CollapseFailure);
        };

        Ok(Collapse::new(
            cost.weight,
            cost.nodes().to_vec(),
            path,
            transition.candidates,
        ))
    }
}
