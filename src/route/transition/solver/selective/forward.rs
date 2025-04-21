use crate::codec::element::variants::OsmEntryId;
use crate::route::graph::NodeIx;
use crate::route::transition::graph::{MatchError, Transition};
use crate::route::transition::primitives::{Dijkstra, DijkstraReachableItem};
use crate::route::transition::solver::selective::successors::SuccessorsLookupTable;
use crate::route::transition::solver::selective::weight_and_distance::WeightAndDistance;
use crate::route::transition::{
    CandidateEdge, CandidateId, Collapse, Costing, EmissionStrategy, Reachable, RoutingContext,
    Solver, TransitionContext, TransitionStrategy, Trip,
};
use geo::{Distance, Haversine};
use log::{debug, info};
use measure_time::debug_time;
use pathfinding::num_traits::Zero;
use pathfinding::prelude::astar;
use petgraph::prelude::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::hash::Hash;
use std::num::NonZeroI64;
use std::sync::{Arc, Mutex};

const DEFAULT_THRESHOLD: f64 = 200_000f64; // 2km in cm

type ProcessedReachable = (CandidateId, Reachable);

/// A Upper-Bounded Dijkstra (UBD) algorithm.
///
/// TODO: Docs
pub struct SelectiveForwardSolver {
    /// The threshold by which the solver is bounded, in centimeters.
    threshold_distance: f64,

    successors_lookup_table: Arc<Mutex<SuccessorsLookupTable>>,
    reachable_hash: RefCell<FxHashMap<(usize, usize), Reachable>>,
}

impl Default for SelectiveForwardSolver {
    fn default() -> Self {
        Self {
            threshold_distance: DEFAULT_THRESHOLD,
            successors_lookup_table: Arc::new(Mutex::new(SuccessorsLookupTable::new())),
            reachable_hash: RefCell::new(FxHashMap::default()),
        }
    }
}

impl SelectiveForwardSolver {
    pub fn use_cache(self, cache: Arc<Mutex<SuccessorsLookupTable>>) -> Self {
        Self {
            successors_lookup_table: cache,
            ..self
        }
    }

    /// TODO: Docs
    ///
    /// Supplies an offset, which represents the initial distance
    /// taken in travelling initial edges, in meters.
    fn bounded_iterator<'a, 'b>(
        &'b self,
        ctx: RoutingContext<'a>,
        start: &'a NodeIx,
    ) -> impl Iterator<Item = DijkstraReachableItem> + use<'a, 'b> {
        let index = &NonZeroI64::new(start.identifier).unwrap();

        Dijkstra
            .reach(index, move |node| {
                self.successors_lookup_table
                    .lock()
                    .unwrap()
                    .lookup(ctx, node)
                    .to_vec()
            })
            .take_while(move |p| {
                // Bounded by the threshold distance (centimeters)
                (p.total_cost.1 as f64) < self.threshold_distance
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
        parents: &FxHashMap<N, (N, C)>,
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

    fn reach<'a, 'b, E, T>(
        &'b self,
        transition: &'b Transition<'b, E, T>,
        context: RoutingContext<'b>,
        (start, end): (CandidateId, CandidateId),
        source: &CandidateId,
    ) -> Box<dyn Iterator<Item = (CandidateId, CandidateEdge)> + 'b>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
        'b: 'a,
    {
        debug_time!("SelectiveForwardSolver::reach (mother-function)");

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
            return Box::new(
                successors
                    .into_iter()
                    .map(|candidate| (candidate, CandidateEdge::zero())),
            );
        }

        // Fast-track to the finish line
        if successors.contains(&end) {
            debug!("End-Successors: {:?}", successors);
            return Box::new(std::iter::once((end, CandidateEdge::zero())));
        }

        Box::new(
            self
                // TODO: Supply context to the reachability function in order to reuse routes
                //       already made. Plus, consider working as contraction hierarchies
                .reachable(context, source, successors.as_slice())
                .unwrap_or_default()
                .into_iter()
                .filter_map(move |reachable| {
                    let source_layer = context.candidate(&reachable.source)?.location.layer_id;
                    let target_layer = context.candidate(&reachable.target)?.location.layer_id;

                    let sl = transition.layers.layers.get(source_layer)?;
                    let tl = transition.layers.layers.get(target_layer)?;

                    let layer_width = Haversine.distance(sl.origin, tl.origin);

                    let transition_cost = transition.heuristics.transition(TransitionContext {
                        optimal_path: Trip::new_with_map(transition.map, &reachable.path),
                        map_path: &reachable.path,
                        requested_resolution_method: reachable.resolution_method,

                        source_candidate: &reachable.source,
                        target_candidate: &reachable.target,
                        routing_context: context,

                        layer_width,
                    });

                    let emission_cost = transition
                        .candidates
                        .candidate(&reachable.target)
                        .map_or(u32::MAX, |v| v.emission);

                    let transition = (transition_cost as f64 * 0.6) as u32;
                    let emission = (emission_cost as f64 * 0.4) as u32;

                    let return_value = (
                        reachable.target,
                        CandidateEdge::new(emission.saturating_add(transition)),
                    );

                    self.reachable_hash
                        .borrow_mut()
                        .insert(reachable.hash(), reachable);
                    Some(return_value)
                }),
        )
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
        let predicate_map = {
            debug_time!("reachability predicate map");

            self.bounded_iterator(ctx, &source_candidate.edge.target)
                .map(|predicate| {
                    let parent = predicate.parent.map_or(0, |v| v.get());
                    (
                        OsmEntryId::as_node(predicate.node.get()),
                        (OsmEntryId::as_node(parent), predicate.total_cost),
                    )
                })
                .collect::<FxHashMap<OsmEntryId, (OsmEntryId, WeightAndDistance)>>()
        };

        let reachable = {
            debug_time!("reachable creation");

            targets
                .iter()
                .filter_map(|target| {
                    // Get the candidate information of the target found
                    let candidate = ctx.candidate(target)?;

                    // Both candidates are on the same edge
                    'stmt: {
                        if candidate.edge.id.index() == source_candidate.edge.id.index() {
                            let common_source =
                                candidate.edge.source == source_candidate.edge.source;
                            let common_target =
                                candidate.edge.target == source_candidate.edge.target;

                            let tracking_forward = common_source && common_target;

                            let source_percentage = source_candidate.percentage(ctx.map)?;
                            let target_percentage = candidate.percentage(ctx.map)?;

                            // debug!(
                            //     "Found movement with {source_percentage} to {target_percentage} which goes {}.",
                            //     if tracking_forward { "Forward" } else { "Backward" },
                            // );
                            // debug!("=> {} : CandidateSource={:?}, CandidateTarget={:?}",
                            //     candidate.position.wkt_string(), candidate.edge.source, candidate.edge.target
                            // );
                            // debug!("=> {} : SourceCandidateSource={:?}, SourceCandidateTarget={:?}",
                            //     source_candidate.position.wkt_string(), source_candidate.edge.source, source_candidate.edge.target
                            // );

                            return if tracking_forward && source_percentage <= target_percentage {
                                // We are moving forward, it is simply the distance between the nodes
                                Some(Reachable::new(*source, *target, vec![]).distance_only())
                            } else {
                                // We are going "backwards", behaviour becomes dependent on
                                // the directionality of the edge. However, to return across the
                                // node is an independent transition, and is not covered.
                                break 'stmt;
                            };
                        }
                    }

                    // Generate the path to this target using the predicate map
                    let path_to_target = Self::path_builder(
                        &candidate.edge.source,
                        &source_candidate.edge.target,
                        &predicate_map,
                    )?;

                    Some(Reachable::new(*source, *target, path_to_target))
                })
                .collect::<Vec<_>>()
        };

        Some(reachable)
    }

    fn solve<E, T>(&self, mut transition: Transition<E, T>) -> Result<Collapse, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        info!("Solving...");

        let (start, end) = {
            debug_time!("attach candidate ends");

            transition
                .candidates
                .attach_ends(&transition.layers)
                .ok_or(MatchError::CollapseFailure)?
        };

        debug!(
            "Start={start:?}. End={end:?}. Candidates: {:?}",
            transition.candidates
        );

        {
            debug_time!("candidate weave");
            transition.candidates.weave(&transition.layers);
        }

        debug!("Linked / Weaved all layers!");

        let context = RoutingContext {
            candidates: &transition.candidates,
            map: transition.map,
        };

        // TLDR: For every candidate, generate their reachable elements, then run the solver overtop.
        //       This means we can do it in parallel, which is more efficient - however will have to
        //       compute for *every* candidate, not just the likely ones, which will lead to poor
        //       scalability for really long-routes.
        //
        // => As a side note, being able to compute the reachable for candidates and then somehow
        //    cache them will lead to incredible performance when revisiting a segment if you were
        //    to route a trip in real-time, since it would re-use the hot-cache for all nodes that
        //    were common to the last trip, meaning you'd only have to calculate one more layer,
        //    which means the overhead is more reliable and consistent.

        let Some((path, cost)) = astar(
            &start,
            |source| self.reach(&transition, context, (start, end), source),
            |_| CandidateEdge::zero(),
            |node| *node == end,
        ) else {
            return Err(MatchError::CollapseFailure);
        };

        info!("Total cost of solve: {}", cost.weight);

        let reached = path
            .windows(2)
            .filter_map(|nodes| {
                if let [a, b] = nodes {
                    self.reachable_hash
                        .borrow()
                        .get(&(a.index(), b.index()))
                        .cloned()
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
