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
use pathfinding::num_traits::Zero;
use pathfinding::prelude::{dijkstra, dijkstra_reach, DijkstraReachableItem};
use petgraph::prelude::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

const DEFAULT_THRESHOLD: f64 = 200_000f64; // 2km in cm

type ProcessedReachable = (CandidateId, Reachable);

/// A Upper-Bounded Dijkstra (UBD) algorithm.
///
/// TODO: Docs
pub struct SelectiveForwardSolver {
    /// The threshold by which the solver is bounded, in centimeters.
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
    /// Returns all the nodes reachable by the solver in an iterator, measured in distance
    fn reachable_iterator<'a>(
        ctx: RoutingContext<'a>,
        start: &'a NodeIx,
    ) -> impl Iterator<Item = DijkstraReachableItem<NodeIx, u32>> + use<'a> {
        dijkstra_reach(start, |node| {
            // Calc. once
            let source = ctx.map.get_position(node).unwrap();

            ctx.map
                .graph
                .edges_directed(*node, Direction::Outgoing)
                .map(|(_, next, _w)| {
                    (
                        next,
                        if *node != next {
                            let target = ctx.map.get_position(&next).unwrap();

                            // In centimeters (1m = 100cm)
                            (Haversine::distance(source, target) * 100f64) as u32
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
        start: &'a NodeIx,
    ) -> impl Iterator<Item = DijkstraReachableItem<NodeIx, u32>> + use<'a, 'b> {
        Self::reachable_iterator(ctx, start).take_while(move |p| {
            // Bounded by the threshold distance (centimeters)
            (p.total_cost as f64) < self.threshold_distance
        })
    }

    /// Creates a path from the source up the parent map until no more parents
    /// are found. This assumes there is only one relation between parent and children.
    ///
    /// Returns in the order `[target, ..., source]`.
    ///
    /// If the target is not found by the builder, `None` is returned.
    #[inline]
    pub(crate) fn path_builder<N, C>(
        source: &N,
        target: &N,
        parents: &HashMap<N, (N, C)>,
    ) -> Option<Vec<N>>
    where
        N: Eq + Hash + Copy,
    {
        let mut rev = vec![*source];
        let mut next = source;

        while let Some((parent, _)) = parents.get(next) {
            // Located the target
            if *next == *target {
                rev.reverse();
                return Some(rev);
            }

            rev.push(*parent);
            next = parent;
        }

        None
    }

    fn reach<'a, E, T>(
        &self,
        transition: &Transition<E, T>,
        context: RoutingContext<'a>,
        (start, end): (CandidateId, CandidateId),
        reachable_hash: &mut HashMap<(usize, usize), Reachable>,
        source: &CandidateId,
    ) -> Vec<(CandidateId, CandidateEdge)>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        let graph_ref = Arc::clone(&transition.candidates.graph);
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

        // Fast-track to the finish line
        if successors.contains(&end) {
            debug!("End-Successors: {:?}", successors);
            return vec![(end, CandidateEdge::zero())];
        }

        let reached = self
            // TODO: Supply context to the reachability function in order to reuse routes
            //       already made. Plus, consider working as contraction hierarchies
            .reachable(context, source, successors.as_slice())
            .unwrap_or_default()
            .into_iter()
            .map(|reachable| {
                let trip = Trip::new_with_map(transition.map, reachable.path.as_slice());

                let source = context.candidate(&reachable.source);
                let target = context.candidate(&reachable.target);

                let source_layer = source.unwrap().location.layer_id;
                let target_layer = target.unwrap().location.layer_id;

                let sl = transition.layers.layers.get(source_layer).unwrap();
                let tl = transition.layers.layers.get(target_layer).unwrap();

                let layer_width = Haversine::distance(sl.origin, tl.origin);

                let transition_cost = transition.heuristics.transition(TransitionContext {
                    // TODO: Remove clone after debugging.
                    optimal_path: trip.clone(),
                    map_path: reachable.path.clone(),

                    source_candidate: &reachable.source,
                    target_candidate: &reachable.target,
                    routing_context: context,

                    layer_width,
                });

                let emission_cost = transition
                    .candidates
                    .candidate(&reachable.target)
                    .map_or(u32::MAX, |v| v.emission);

                let transition = (transition_cost as f64 * 0.8) as u32;
                let emission = (emission_cost as f64 * 0.2) as u32;

                let return_value = (
                    reachable.target,
                    CandidateEdge::new(emission.saturating_add(transition)),
                );

                reachable_hash.insert(reachable.hash(), reachable);
                return_value
            })
            .collect::<Vec<_>>();

        reached
    }
}

impl Solver for SelectiveForwardSolver {
    fn reachable<'a>(
        &self,
        ctx: RoutingContext<'a>,
        source: &CandidateId,
        targets: &'a [CandidateId],
    ) -> Option<Vec<Reachable>> {
        let source_candidate = ctx.candidate(source)?;

        // Upper-Bounded reachable map containing a Child:Parent relation
        // Note: Parent is OsmEntryId::NULL, which will not be within the map,
        //       indicating the root element.
        let predicate_map = self
            .bounded_iterator(ctx, &source_candidate.edge.target)
            .map(|predicate| {
                let parent = predicate.parent.unwrap_or(OsmEntryId::null());
                (predicate.node, (parent, predicate.total_cost))
            })
            .collect::<HashMap<OsmEntryId, (OsmEntryId, u32)>>();

        let reachable = targets
            .iter()
            .filter_map(|target| {
                // Get the candidate information of the target found
                let candidate = ctx.candidate(target)?;

                // Generate the path to this target using the predicate map
                let path_to_target = Self::path_builder(
                    &candidate.edge.source,
                    &source_candidate.edge.target,
                    &predicate_map,
                )?;

                Some(Reachable::new(*source, *target, path_to_target))
            })
            .collect::<Vec<_>>();

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

        let mut reachable_hash: HashMap<(usize, usize), Reachable> = HashMap::new();
        let Some((path, cost)) = dijkstra(
            &start,
            |source| {
                self.reach(
                    &transition,
                    context,
                    (start, end),
                    &mut reachable_hash,
                    source,
                )
            },
            // |_| CandidateEdge::zero(),
            |node| *node == end,
        ) else {
            return Err(MatchError::CollapseFailure);
        };

        debug!("Total cost of solve: {}", cost.weight);

        let reached = path
            .windows(2)
            .filter_map(|nodes| {
                if let [a, b] = nodes {
                    reachable_hash.get(&(a.index(), b.index())).cloned()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(Collapse::new(
            cost.weight,
            reached,
            path,
            transition.candidates,
        ))
    }
}
