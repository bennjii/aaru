//! The realtime (streaming) lifecycle: `begin` → `push` → `solve` per tick,
//! `finish` at the end — asserted equivalent to the one-shot batch match, and
//! resumable across serialization.

use geo::{LineString, Point, point, wkt};
use routers_network::mock::{MockEntryId, MockNetwork, MockNetworkBuilder};
use routers_transition::candidate::CollapsedPath;
use routers_transition::costing::{CostingStrategies, DefaultEmissionCost, DefaultTransitionCost};
use routers_transition::layer::generation::StandardGenerator;
use routers_transition::matcher::Trip;
use routers_transition::weigh::AllCompute;
use routers_transition::{Continuation, MatchError, Matcher, Origin};

type Costing = CostingStrategies<DefaultEmissionCost, DefaultTransitionCost, MockEntryId>;

/// A staircase road: west, then south, then west again.
fn bent_road() -> MockNetwork {
    MockNetworkBuilder::new()
        .node(1, point!(x: -118.15, y: 34.15))
        .node(2, point!(x: -118.16, y: 34.15))
        .node(3, point!(x: -118.17, y: 34.15))
        .node(4, point!(x: -118.17, y: 34.14))
        .node(5, point!(x: -118.18, y: 34.14))
        .edge(1, 2)
        .edge(2, 3)
        .edge(3, 4)
        .edge(4, 5)
        .build()
}

fn trajectory() -> LineString {
    wkt! {
        LINESTRING(
            -118.151 34.1503, -118.155 34.1503, -118.165 34.1503,
            -118.170 34.1490, -118.172 34.1403, -118.179 34.1403
        )
    }
}

/// Index-stamped observations over a shared timeline. Reconcile compares
/// timestamps, so every slice of one trajectory must agree on them — stamp
/// once, then slice.
fn origins_of(points: &[Point]) -> Vec<Origin> {
    points
        .iter()
        .enumerate()
        .map(|(index, &point)| Origin::new(point, index as i64))
        .collect()
}

fn assert_same_match(a: &CollapsedPath<MockEntryId>, b: &CollapsedPath<MockEntryId>) {
    assert_eq!(a.cost, b.cost, "costs must agree");
    assert_eq!(a.route, b.route, "chosen candidates must agree");
    assert_eq!(a.collapsed(), b.collapsed(), "matched positions must agree");
    assert_eq!(
        a.interpolated.len(),
        b.interpolated.len(),
        "hop counts must agree"
    );
    for (x, y) in a.interpolated.iter().zip(&b.interpolated) {
        assert_eq!(x.path, y.path, "re-derived hop geometry must agree");
    }
}

/// Pushing and re-solving one position at a time must land on exactly the
/// match the batch pipeline finds.
#[test]
fn streaming_equals_batch() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let batch = m.r#match(trajectory()).expect("batch match must succeed");

    let mut trip = m.begin();
    for origin in origins_of(&trajectory().into_points()) {
        m.push(&mut trip, origin).expect("push must anchor");
        m.solve(&mut trip).expect("every prefix must solve");
    }
    let streamed = m.snapshot(&mut trip).expect("snapshot must succeed");

    assert_same_match(&streamed, &batch);
}

/// A trip serialized mid-stream and resumed in a "new process" (fresh matcher,
/// fresh caches) must complete to the same match.
#[test]
fn trip_serde_round_trip_resumes() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = || StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator(), AllCompute::default(), &());

    let origins = origins_of(&trajectory().into_points());
    let (head, tail) = origins.split_at(3);

    let mut trip = m.begin();
    for &origin in head {
        m.push(&mut trip, origin).expect("push must anchor");
    }
    m.solve(&mut trip).expect("prefix must solve");

    // Tick boundary: persist, drop everything, resume elsewhere.
    let stored = serde_json::to_string(&trip).expect("trip must serialize");
    drop(trip);

    let mut resumed: Trip<MockEntryId> =
        serde_json::from_str(&stored).expect("trip must deserialize");
    assert!(resumed.is_solved(), "solved state must survive the trip");

    let m2 = Matcher::new(&net, &costing, generator(), AllCompute::default(), &());
    for &origin in tail {
        m2.push(&mut resumed, origin).expect("push must anchor");
        m2.solve(&mut resumed).expect("every prefix must solve");
    }
    let streamed = m2.snapshot(&mut resumed).expect("snapshot must succeed");

    let batch = m.r#match(trajectory()).expect("batch match must succeed");
    assert_same_match(&streamed, &batch);
}

/// A point with no nearby road is rejected and leaves the trip untouched, so
/// the stream can drop it and continue.
#[test]
fn unanchored_push_leaves_trip_unchanged() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let mut trip = m.begin();
    m.push(&mut trip, Origin::new(point!(x: -118.151, y: 34.1503), 0))
        .expect("on-road point must anchor");

    let off_network = Origin::new(point!(x: 0.0, y: 0.0), 1);
    let err = m.push(&mut trip, off_network).expect_err("must reject");
    assert!(matches!(err, MatchError::Unanchored(_)));
    assert_eq!(trip.layers(), 1, "rejected push must not grow the trip");

    m.push(&mut trip, Origin::new(point!(x: -118.155, y: 34.1503), 2))
        .expect("stream continues after a dropped point");
    let path = m.solve(&mut trip).expect("solve must succeed");
    assert_eq!(path.nodes.len(), 2);
}

/// Matching is deterministic: identical inputs give identical outputs, and the
/// collapse-time geometry re-derivation reproduces itself run over run.
#[test]
fn repeated_matches_reproduce_geometry() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = || StandardGenerator::new(&net, &costing.emission);

    let a = Matcher::new(&net, &costing, generator(), AllCompute::default(), &())
        .r#match(trajectory())
        .expect("match must succeed");
    let b = Matcher::new(&net, &costing, generator(), AllCompute::default(), &())
        .r#match(trajectory())
        .expect("match must succeed");

    assert_same_match(&a, &b);
}

/// Trimming a trip to its last `n` layers must leave a consistent,
/// re-solvable state whose solution equals a fresh batch match of the same
/// suffix — candidates re-stamped, trellis cut, resolved boundaries kept.
#[test]
fn tail_matches_batch_over_suffix() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = || StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator(), AllCompute::default(), &());

    let points = trajectory().into_points();
    let origins = origins_of(&points);

    let mut trip = m.begin();
    for &origin in &origins {
        m.push(&mut trip, origin).expect("push must anchor");
    }
    m.solve(&mut trip).expect("full trip must solve");

    trip.tail(3);
    assert_eq!(trip.layers(), 3, "trip must hold exactly the suffix");
    assert_eq!(trip.origins(), &origins[3..], "origins must be the suffix");
    assert!(!trip.is_solved(), "a cut certificate must reopen");

    let streamed = m.snapshot(&mut trip).expect("trimmed trip must re-solve");
    let batch = m
        .r#match(LineString::from(points[3..].to_vec()))
        .expect("batch match must succeed");
    assert_same_match(&streamed, &batch);
}

/// `tail` is windowing, not surgery: asking for at least the current size
/// changes nothing, and asking for zero empties the trip.
#[test]
fn tail_bounds_are_noop_and_empty() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let mut trip = m.begin();
    for origin in origins_of(&trajectory().into_points()) {
        m.push(&mut trip, origin).expect("push must anchor");
    }
    m.solve(&mut trip).expect("trip must solve");

    trip.tail(usize::MAX);
    assert_eq!(trip.layers(), 6, "oversized tail must be a no-op");
    assert!(trip.is_solved(), "a no-op tail must keep the certificate");

    trip.tail(0);
    assert!(trip.is_empty(), "tail(0) must empty the trip");
}

/// A persisted trip whose origins overlap the committed history resumes:
/// trimmed to the overlap, with only the unseen points left to push — and the
/// resumed stream lands on the batch match of the history.
#[test]
fn reconcile_resumes_and_trims_to_overlap() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let points = trajectory().into_points();
    let origins = origins_of(&points);

    // "Yesterday's" persisted trip: the first four observations.
    let mut persisted = m.begin();
    for &origin in &origins[..4] {
        m.push(&mut persisted, origin).expect("push must anchor");
    }
    m.solve(&mut persisted).expect("prefix must solve");

    // Today's committed history: the window slid past the first point and
    // two new points arrived.
    let history = &origins[1..];

    let Continuation::Resume { mut trip, fresh } =
        Continuation::reconcile(Some(persisted), history)
    else {
        panic!("overlapping history must resume");
    };
    assert_eq!(trip.layers(), 3, "trip must trim to the overlap");
    assert_eq!(trip.origins(), &origins[1..4]);
    assert_eq!(fresh, origins[4..].to_vec(), "unseen points must be fresh");

    for origin in fresh {
        m.push(&mut trip, origin).expect("push must anchor");
    }
    let streamed = m.snapshot(&mut trip).expect("resumed trip must solve");
    let batch = m
        .r#match(LineString::from(points[1..].to_vec()))
        .expect("batch match must succeed");
    assert_same_match(&streamed, &batch);
}

/// The realtime dissemination loop, as the orchestrator + matcher run it:
/// each tick reconciles the previously-committed trip against the committed
/// history, pushes only the fresh points, solves, snapshots, and commits the
/// (serde round-tripped) trip for the next tick.
///
/// When the history window covers the whole trip, every tick's snapshot must
/// span the *entire* trajectory so far — a consumer that replaces its trace
/// with each result (the realtime viewer) sees the full tail.
#[test]
fn ticked_resume_snapshots_full_history() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = || StandardGenerator::new(&net, &costing.emission);

    let points = trajectory().into_points();
    let origins = origins_of(&points);
    let mut committed: Option<Trip<MockEntryId>> = None;

    for tick in 1..=points.len() {
        let history = &origins[..tick];
        let m = Matcher::new(&net, &costing, generator(), AllCompute::default(), &());

        let (mut trip, fresh) = match Continuation::reconcile(committed.take(), history) {
            Continuation::Resume { trip, fresh } => (trip, fresh),
            Continuation::Restart { fresh } => (m.begin(), fresh),
        };

        assert!(
            trip.layers() + fresh.len() >= history.len(),
            "tick {tick}: reconcile lost context: {} retained + {} fresh < {} history",
            trip.layers(),
            fresh.len(),
            history.len()
        );

        for origin in fresh {
            m.push(&mut trip, origin).expect("push must anchor");
        }

        let streamed = m.snapshot(&mut trip).expect("tick must solve");
        let batch = m
            .r#match(LineString::from(points[..tick].to_vec()))
            .expect("batch match must succeed");
        assert_same_match(&streamed, &batch);

        // Tick boundary: the trip travels matcher → orchestrator over the bus.
        let wire = serde_json::to_string(&trip).expect("trip must serialize");
        committed = Some(serde_json::from_str(&wire).expect("trip must deserialize"));
    }
}

/// The same loop, but the committed history is a sliding window (the
/// orchestrator's `--context-window`). Reconcile trims the resumed trip to
/// the window overlap, so the emitted snapshot can never span more points
/// than the window holds — a supersede-style consumer's trace is bounded by
/// the orchestrator window, *not* by the trip's true length.
#[test]
fn windowed_history_bounds_the_emitted_trace() {
    const WINDOW: usize = 3;

    let net = bent_road();
    let costing = Costing::default();
    let generator = || StandardGenerator::new(&net, &costing.emission);

    let points = trajectory().into_points();
    let origins = origins_of(&points);
    let mut committed: Option<Trip<MockEntryId>> = None;

    for tick in 1..=origins.len() {
        let history = &origins[tick.saturating_sub(WINDOW)..tick];
        let m = Matcher::new(&net, &costing, generator(), AllCompute::default(), &());

        let (mut trip, fresh) = match Continuation::reconcile(committed.take(), history) {
            Continuation::Resume { trip, fresh } => (trip, fresh),
            Continuation::Restart { fresh } => (m.begin(), fresh),
        };
        for origin in fresh {
            m.push(&mut trip, origin).expect("push must anchor");
        }
        m.solve(&mut trip).expect("tick must solve");
        committed = Some(trip);
    }

    let finished = committed.expect("loop ran");
    assert_eq!(
        finished.layers(),
        WINDOW,
        "the trip is windowed to the history overlap, so per-tick results \
         cannot carry the full trace"
    );
    assert_eq!(finished.origins(), &origins[origins.len() - WINDOW..]);
}

/// The rendering signature of a gap-cut restart: a trip holding a single
/// point still emits a non-empty interpolated path — its matched edge's
/// source node plus the matched position. A supersede-style consumer
/// replaces the vehicle's whole trace with this short stub.
#[test]
fn single_point_restart_emits_a_stub_segment() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let mut trip = m.begin();
    m.push(&mut trip, Origin::new(point!(x: -118.155, y: 34.1503), 0))
        .expect("push must anchor");

    let snapshot = m.snapshot(&mut trip).expect("single point must solve");
    assert_eq!(snapshot.route.len(), 1, "one layer, one chosen candidate");
    assert!(
        snapshot.interpolated.is_empty(),
        "no hop exists, so nothing to interpolate"
    );

    let routed = routers_transition::candidate::RoutedPath::new(snapshot, &net);
    assert!(
        !routed.interpolated.is_empty(),
        "the routed view still emits a stub (edge source + matched point)"
    );
    assert!(
        routed.interpolated.len() <= 2,
        "the stub is at most two elements, not a trace"
    );
}

/// A history the trip's origins never overlap (a teleport cut everything the
/// trellis knew) — and the absence of any trip at all — both restart.
#[test]
fn reconcile_restarts_on_divergence() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let origins = origins_of(&trajectory().into_points());

    let mut persisted = m.begin();
    for &origin in &origins[..3] {
        m.push(&mut persisted, origin).expect("push must anchor");
    }

    // Post-teleport: the orchestrator discarded everything the trip has seen.
    let history = origins[3..].to_vec();

    match Continuation::reconcile(Some(persisted), &history) {
        Continuation::Restart { fresh } => assert_eq!(fresh, history),
        Continuation::Resume { .. } => panic!("disjoint history must restart"),
    }

    match Continuation::<MockEntryId>::reconcile(None, &history) {
        Continuation::Restart { fresh } => assert_eq!(fresh, history),
        Continuation::Resume { .. } => panic!("no trip must restart"),
    }
}

/// Overlap is the whole observation, not just the position: a history that
/// re-times the same points was not what the trip was solved against, and a
/// history that re-places the same timestamps contradicts it. Both restart.
#[test]
fn reconcile_restarts_when_observations_disagree() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = || StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator(), AllCompute::default(), &());

    let origins = origins_of(&trajectory().into_points());

    let persist = || {
        let mut persisted = m.begin();
        for &origin in &origins[..3] {
            m.push(&mut persisted, origin).expect("push must anchor");
        }
        persisted
    };

    // Same positions, shifted timestamps.
    let retimed: Vec<Origin> = origins[..3]
        .iter()
        .map(|o| Origin::new(o.point, o.timestamp + 100))
        .collect();
    assert!(
        matches!(
            Continuation::reconcile(Some(persist()), &retimed),
            Continuation::Restart { .. }
        ),
        "re-timed history must restart"
    );

    // Same timestamps, one contradicted position.
    let mut replaced = origins[..3].to_vec();
    replaced[2].point = point!(x: -118.180, y: 34.1403);
    assert!(
        matches!(
            Continuation::reconcile(Some(persist()), &replaced),
            Continuation::Restart { .. }
        ),
        "a contradicted position must restart"
    );
}

/// `LayerId` indexes everything on a trip: origins, candidate layers, trellis.
#[test]
fn trip_accessors_are_layer_indexed() {
    let net = bent_road();
    let costing = Costing::default();
    let generator = StandardGenerator::new(&net, &costing.emission);
    let m = Matcher::new(&net, &costing, generator, AllCompute::default(), &());

    let mut trip = m.begin();
    assert!(trip.is_empty() && trip.last_id().is_none());

    let position: Point = point!(x: -118.151, y: 34.1503);
    let id = m
        .push(&mut trip, Origin::new(position, 7))
        .expect("push must anchor");

    assert_eq!(trip.last_id(), Some(id));
    assert_eq!(trip.point(id), Some(position));

    let layer = trip.layer(id).expect("layer must exist");
    assert!(!layer.is_empty());
    for (n, candidate) in layer.iter().enumerate() {
        assert_eq!(candidate.location.layer, id);
        assert_eq!(candidate.location.node.index(), n);
        assert_eq!(
            trip.candidate(&candidate.location).map(|c| c.position),
            Some(candidate.position)
        );
    }

    let trellis = trip.trellis().expect("trellis exists after first layer");
    assert_eq!(trellis.widths(), &[layer.len() as u32]);
}
