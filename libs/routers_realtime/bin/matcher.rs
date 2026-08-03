use std::path::PathBuf;
use std::sync::Arc;

use routers_codec::osm::{OsmEdgeMetadata, OsmEntryId};
use routers_network::Metadata;
use routers_realtime::{
    bus::{self, Wire},
    event::{MatchContext, MatchReply, MatchedDiff},
};
use routers_shard::{FileFetcher, Geohash, ShardLoader, ShardedNetwork};
use routers_transition::{
    Continuation, MatchError, Matcher,
    costing::{CostingStrategies, DefaultEmissionCost, DefaultTransitionCost},
    layer::generation::StandardGenerator,
    primitives::PredicateCache,
    weigh::AllCompute,
};

use anyhow::Context;
use async_nats::{ConnectOptions, ServerAddr};
use clap::Parser;
use futures::StreamExt;
use log::{debug, error, info, warn};
use tracing::{field, info_span};
use url::Url;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// URL of the NATS server
    #[arg(short, env, long)]
    nats: Url,

    /// The directory of stored shard files
    #[arg(short, env, long)]
    directory: PathBuf,

    /// The shard precision the system is configured to
    #[arg(short, env, long)]
    precision: usize,

    // The configured "owned" shard.
    #[arg(short, env, long)]
    shard: Geohash,

    // The inbound NATS subject to serve match requests from.
    #[arg(short, env, long)]
    inbound_subject: String,

    /// The queue group shared by this shard's matchers: NATS delivers each
    /// request to exactly one member, so replicas divide the load instead of
    /// duplicating it.
    #[arg(short, env, long, default_value = "matchers")]
    queue_group: String,

    /// The search distance to use for matching
    #[arg(long, env)]
    search_distance: Option<f64>,

    /// How many contexts to solve concurrently. Solving is CPU-bound and each
    /// context is self-contained, so contexts fan out across a blocking pool
    /// with no shared state to serialise on.
    #[arg(short, env, long, default_value = "5")]
    workers: usize,
}

type E = OsmEntryId;
type M = OsmEdgeMetadata;
type Net = ShardedNetwork<E, M, Geohash>;

/// Everything a solve needs, owned so the service can be shared (`Arc`) across
/// the concurrent solves without leaking or juggling `'static` borrows. The
/// network's spatial index and the predicate cache are the only heavy state,
/// and both are shared; a per-solve [`Matcher`] is just a bundle of borrows
/// into this and is free to build.
struct Matching {
    network: Arc<Net>,
    runtime: <M as Metadata>::Runtime,
    costing: CostingStrategies<DefaultEmissionCost, DefaultTransitionCost, E>,
    cache: Arc<PredicateCache<Net>>,
    search_distance: Option<f64>,
}

impl Matching {
    /// Solve one context, recording its outcome onto a fresh `match_event`
    /// span. Returns the reply to send: the emission and resume state, or
    /// [`MatchReply::NoMatch`] when there is nothing to emit (no anchor, or a
    /// nominal/fatal solve failure).
    fn solve(
        &self,
        MatchContext {
            vehicle_id,
            continuation,
        }: MatchContext<E>,
    ) -> MatchReply<E> {
        let mut generator = StandardGenerator::new(self.network.as_ref(), &self.costing.emission);
        if let Some(distance) = self.search_distance {
            generator = generator.with_search_distance(distance);
        }

        let weigher = AllCompute::default().use_cache(self.cache.clone());
        let matcher = Matcher::new(
            self.network.as_ref(),
            &self.costing,
            generator,
            weigher,
            &self.runtime,
        );

        let span = info_span!(
            "match_event",
            outcome = field::Empty,
            severity = field::Empty,
            continuation = field::Empty,
            converged = field::Empty,
            emitted = field::Empty,
        );
        let _entered = span.enter();

        let (mut trip, fresh) = match continuation {
            Continuation::Resume { trip, fresh } => {
                span.record("continuation", "resume");
                (trip, fresh)
            }
            Continuation::Restart { fresh } => {
                span.record("continuation", "restart");
                (matcher.begin(), fresh)
            }
        };

        info_span!("push", points = fresh.len()).in_scope(|| {
            for origin in fresh {
                match matcher.push(&mut trip, origin) {
                    Ok(_) => {}
                    Err(MatchError::Unanchored(err)) => {
                        info_span!("point_drop", reason = "unanchored")
                            .in_scope(|| debug!("{vehicle_id}: dropped off-network point ({err})"));
                    }
                    Err(err) => {
                        info_span!("point_drop", reason = "push_error")
                            .in_scope(|| error!("{vehicle_id}: could not push point: {err}"));
                    }
                }
            }
        });

        if trip.is_empty() {
            span.record("outcome", "no_anchor");
            span.record("severity", "nominal");
            warn!("{vehicle_id}: no anchored layers to solve");
            return MatchReply::NoMatch;
        }

        if let Err(err) = info_span!("solve").in_scope(|| matcher.solve(&mut trip)) {
            let (outcome, severity) = classify(err);
            span.record("outcome", outcome);
            span.record("severity", severity);
            error!("{vehicle_id}: unable to solve trip");
            return MatchReply::NoMatch;
        }

        // Copied out: the snapshot's borrow spans the whole trip mutably.
        let origins = trip.origins().to_vec();

        let solution = match info_span!("snapshot").in_scope(|| matcher.snapshot(&mut trip)) {
            Ok(solution) => solution,
            Err(err) => {
                let (outcome, severity) = classify(err);
                span.record("outcome", outcome);
                span.record("severity", severity);
                return MatchReply::NoMatch;
            }
        };

        // Emit everything a future solve could still change — the whole trip
        // since its last cut. Revision 0 until durable ingest supplies stream
        // sequences.
        let diff = info_span!("emit")
            .in_scope(|| MatchedDiff::new(&solution, &origins, self.network.as_ref(), 0));
        drop(solution);
        span.record("emitted", diff.layers.len());

        // Cut behind the convergence point: those layers are final, already
        // emitted, and only cost wire from here on. The convergence layer
        // itself stays as the resume anchor. An unfused trip stays whole —
        // the orchestrator's context window bounds its growth.
        match matcher.convergence(&trip) {
            Ok(Some(layer)) => {
                span.record("converged", layer.index() as u64);
                trip.tail(trip.layers() - layer.index());
            }
            Ok(None) => {}
            Err(err) => error!("{vehicle_id}: convergence query failed: {err}"),
        }

        span.record("outcome", "success");
        span.record("severity", "ok");

        MatchReply::Solved { diff, trip }
    }
}

/// A match attempt's `outcome`/`severity` labels for the success-ratio series.
/// Nominal failures are the data's fault (a point off every road, a trace the
/// network cannot bridge) and expected in healthy operation; fatal ones are ours.
fn classify(err: MatchError) -> (&'static str, &'static str) {
    match err {
        MatchError::Unanchored(_) => ("unanchored", "nominal"),
        MatchError::Disconnected(_) => ("disconnected", "nominal"),
        MatchError::TrellisError(_) | MatchError::SolveError(_) => ("internal", "fatal"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = routers_realtime::telemetry::init("routers-matcher");

    let args = Args::parse();
    info!("matcher started: {:?}", args);

    let fetcher = FileFetcher::new(args.directory);
    let mut loader = ShardLoader::<E, M, Geohash, _, _>::new(fetcher, |key: &Geohash| {
        format!("{}.shard.rt", key)
    });

    let network = loader
        .load(&args.shard)
        .await
        .context("could not find shard in cache")?;

    let nats_url = ServerAddr::from_url(args.nats).context("could not create NATS url")?;

    let client = ConnectOptions::new()
        .name("MatcherService")
        .connect(nats_url)
        .await
        .context("could not connect to NATS")?;

    // The queue group makes replicas additive: each request lands on exactly
    // one member, so scaling a shard's matchers divides the load.
    let subscriber = client
        .queue_subscribe(args.inbound_subject, args.queue_group)
        .await
        .context("could not subscribe to NATS subject")?;

    let matching = Arc::new(Matching {
        network,
        runtime: OsmEdgeMetadata::runtime(None),
        costing: CostingStrategies::default(),
        cache: Arc::new(PredicateCache::default()),
        search_distance: args.search_distance,
    });

    // Each context is solved on the blocking pool (solving is synchronous and
    // CPU-bound); `for_each_concurrent` keeps `workers` in flight at once.
    // Every request is answered on its reply inbox — a NoMatch is still an
    // answer, so the asking orchestrator never waits out a timeout for an
    // event that solved to nothing.
    subscriber
        .for_each_concurrent(args.workers, |message| {
            let matching = Arc::clone(&matching);
            let client = client.clone();

            async move {
                bus::inbound(message.subject.as_str(), message.headers.as_ref());

                let Some(inbox) = message.reply else {
                    warn!("dropping request without a reply inbox — not a request?");
                    return;
                };

                let context = match MatchContext::<E>::decode(&message.payload) {
                    Ok(context) => context,
                    Err(err) => {
                        warn!("skipping undecodable context: {err}");
                        return;
                    }
                };

                let reply = tokio::task::spawn_blocking(move || matching.solve(context))
                    .await
                    .unwrap_or_else(|err| {
                        error!("solve task panicked: {err}");
                        MatchReply::NoMatch
                    });

                let payload = match reply.encode() {
                    Ok(payload) => payload,
                    Err(err) => {
                        error!("could not encode reply: {err:#}");
                        return;
                    }
                };

                if let Err(err) = client
                    .publish_with_headers(inbox, bus::outbound(), payload.into())
                    .await
                {
                    error!("could not send reply: {err:#}");
                }
            }
        })
        .await;

    error!("source terminated");
    Ok(())
}
