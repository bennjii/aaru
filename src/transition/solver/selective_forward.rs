use crate::transition::*;

use log::{debug, info};

use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use codec::Entry;
use geo::{Distance, Haversine};
use itertools::Itertools;
use pathfinding::num_traits::Zero;
use pathfinding::prelude::*;
use petgraph::Direction;
use petgraph::prelude::EdgeRef;

/// A Upper-Bounded Dijkstra (UBD) algorithm.
///
/// TODO: Docs
pub struct SelectiveForwardSolver<Ent>
where
    Ent: Entry,
{
    // Internally holds a successors cache
    predicate: Arc<Mutex<PredicateCache<Ent>>>,
    reachable_hash: RefCell<FxHashMap<(usize, usize), Reachable<Ent>>>,
}

impl<Ent> Default for SelectiveForwardSolver<Ent>
where
    Ent: Entry,
{
    fn default() -> Self {
        Self {
            predicate: Arc::new(Mutex::new(PredicateCache::default())),
            reachable_hash: RefCell::new(FxHashMap::default()),
        }
    }
}

impl<Ent> SelectiveForwardSolver<Ent>
where
    Ent: Entry,
{
    pub fn use_cache(self, cache: Arc<Mutex<PredicateCache<Ent>>>) -> Self {
        Self {
            predicate: cache,
            ..self
        }
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
        transition: &'b Transition<'b, E, T, Ent>,
        context: RoutingContext<'b, Ent>,
        (start, end): (CandidateId, CandidateId),
        source: &CandidateId,
    ) -> Vec<(CandidateId, CandidateEdge)>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy<Ent> + Send + Sync,
        'b: 'a,
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
            debug!("End-Successors: {successors:?}");
            return vec![(end, CandidateEdge::zero())];
        }

        // Note: `reachable` ~= free, `reach` ~= 0.1ms (some overhead- how?)

        self.reachable(context, source, successors.as_slice())
            .unwrap_or_default()
            .into_iter()
            .filter_map(move |reachable| {
                let source_layer = context.candidate(&reachable.source)?.location.layer_id;
                let target_layer = context.candidate(&reachable.target)?.location.layer_id;

                let sl = transition.layers.layers.get(source_layer)?;
                let tl = transition.layers.layers.get(target_layer)?;

                let path_vec = reachable.path_nodes().collect_vec();

                let layer_width = Haversine.distance(sl.origin, tl.origin);
                let optimal_path = Trip::new_with_map(transition.map, &path_vec);

                let transition_cost = transition.heuristics.transition(TransitionContext {
                    map_path: &path_vec,
                    requested_resolution_method: reachable.resolution_method,

                    source_candidate: &reachable.source,
                    target_candidate: &reachable.target,
                    routing_context: context,

                    layer_width,
                    optimal_path,
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
            })
            .collect::<Vec<_>>()
    }

    /// Derives which candidates are reachable by the source candidate.
    ///
    /// Provides a slice of target candidate IDs, `targets`. The solver
    /// will use these to procure all candidates which are reachable,
    /// and the path of routable entries ([`OsmEntryId`]) which are used
    /// to reach the target.
    fn reachable<'a>(
        &self,
        ctx: RoutingContext<'a, Ent>,
        source: &CandidateId,
        targets: &'a [CandidateId],
    ) -> Option<Vec<Reachable<Ent>>> {
        let source_candidate = ctx.candidate(source)?;

        // Upper-Bounded reachable map containing a Child:Parent relation
        // Note: Parent is OsmEntryId::NULL, which will not be within the map,
        //       indicating the root element.
        let predicate_map = {
            self.predicate
                .lock()
                .unwrap()
                .query(&ctx, source_candidate.edge.target)
                .clone()
        };

        let reachable = {
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

                    let path = path_to_target
                        .windows(2)
                        .filter_map(|pair| {
                            if let [a, b] = pair {
                                return ctx.edge(a, b);
                            }

                            None
                        })
                        .collect::<Vec<_>>();

                    Some(Reachable::new(*source, *target, path))
                })
                .collect::<Vec<_>>()
        };

        Some(reachable)
    }
}

impl<Ent> Solver<Ent> for SelectiveForwardSolver<Ent>
where
    Ent: Entry,
{
    fn solve<E, T>(
        &self,
        mut transition: Transition<E, T, Ent>,
    ) -> Result<Collapse<Ent>, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy<Ent> + Send + Sync,
    {
        let (start, end) = {
            // Compute cost ~= free
            transition
                .candidates
                .attach_ends(&transition.layers)
                .map_err(MatchError::EndAttachFailure)?
        };

        debug!("Attached Ends");
        transition.candidates.weave(&transition.layers);
        debug!("Weaved all candidate layers.");

        info!("Solving: Start={start:?}. End={end:?}. ");
        let context = transition.context();

        // Note: For every candidate, generate their reachable elements, then run the solver overtop.
        //       This means we can do it in parallel, which is more efficient - however will have to
        //       compute for *every* candidate, not just the likely ones, which will lead to poor
        //       scalability for really long-routes.
        //
        //       This behaviour can be implemented using the `AllForwardSolver` going forward.

        let Some((path, cost)) = astar(
            &start,
            |source| self.reach(&transition, context, (start, end), source),
            |_| CandidateEdge::zero(),
            |node| *node == end,
        ) else {
            return Err(MatchError::CollapseFailure(CollapseError::NoPathFound));
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
