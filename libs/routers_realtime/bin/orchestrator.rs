use std::time::Duration;

use routers_realtime::event::VehicleId;
use scc::HashCache;
use scc::hash_cache::Entry;

use routers_codec::osm::OsmEntryId;
use routers_realtime::{
    bus::{NATSSink, NATSStream, Wire},
    event::{MatchContext, MatchReply, MatchedEvent, Payload, RawEvent},
    store::RedisStore,
};
use routers_transition::matcher::Trip;
use routers_transition::{Continuation, Origin};

use anyhow::{Context, Result};
use async_nats::{ConnectOptions, ServerAddr};
use clap::Parser;
use futures::{SinkExt, StreamExt};
use geo::{Distance, Haversine};
use log::{debug, error, info};
use tokio::sync::mpsc;
use tracing::{Instrument, field, info_span, warn};
use url::Url;

type E = OsmEntryId;

/// A [`Payload`] handed to a worker, tagged with the wall-clock stamps the
/// dispatch loop captured: when it was queued (for channel-residency timing)
/// and its wire send time (for end-to-end timing).
struct Dispatch {
    queued_at: web_time::SystemTime,
    sent_at: Option<web_time::SystemTime>,
    payload: Payload,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// URL of the NATS server
    #[arg(short, env, long)]
    nats: Url,

    /// Valkey primaries, comma-separated. Vehicles are spread across them by
    /// rendezvous hash, so the order carries no meaning and every binary that
    /// touches the history must be given the same set.
    #[arg(short, env, long, value_delimiter = ',')]
    redis: Vec<Url>,

    /// The NATS subject to use to source raw events from.
    /// For example, `events.position.{cell}.{rest}`, the shard's geohash split
    /// across two tokens.
    #[arg(short, long = "in", env)]
    inbound_subject: String,

    /// The NATS subject the shard's matchers serve requests on.
    /// For example, `events.match.{shard}`.
    #[arg(short, long = "out", env)]
    outbound_subject: String,

    /// The NATS subject matched emissions publish into, for the reconciler
    /// and any observer. For example, `events.matched.{shard}`.
    #[arg(long = "matched", env)]
    matched_subject: String,

    /// How long to wait for a matcher's reply before re-driving the request.
    #[arg(long, env, value_parser = humantime::parse_duration, default_value = "5s")]
    solve_timeout: Duration,

    /// Re-drives after the first attempt before the event is abandoned.
    /// Replies are idempotent downstream (layers merge by timestamp and
    /// resolve by revision), so a duplicate solve from a late reply is
    /// convergence, not conflict.
    #[arg(long, env, default_value_t = 3)]
    solve_retries: usize,

    /// The number of context entries to retrieve from Redis for each vehicle.
    #[arg(short, long = "context-window", env, default_value = "10")]
    context_window: usize,

    /// Points older than this will be discarded from history, regardless
    /// of if it's within the KV store, or not.
    #[arg(long, env, value_parser = humantime::parse_duration, default_value = "120s")]
    gap: Duration,

    /// Consecutive points further away than this will be treated as a "teleport",
    /// and dropped along with everything older.
    #[arg(long, env, default_value = "2000")]
    jump_distance: f64,

    /// How many workers to fan vehicles across. Each vehicle is pinned to one
    /// by hash, so its events stay ordered on a worker that owns their trip
    /// state outright — the maps need no locks. A worker now holds its lane
    /// for a whole solve round trip, so this is the pod's in-flight solve
    /// bound; workers are tokio tasks, priced accordingly.
    #[arg(short, env, long, default_value = "64")]
    workers: usize,

    /// Vehicles each worker keeps trip state for, before the least recently
    /// used is evicted.
    ///
    /// Vehicles are hash-spread across workers, so the fleet-wide bound is this
    /// times `workers`. Size it above the concurrent vehicles a shard carries:
    /// evicting a vehicle that is still moving forces its next event to restart
    /// the trip rather than resume it. Rounded up to a power of two.
    ///
    /// Unbounded is not an option here. Trip state is per vehicle and vehicles
    /// leave a shard permanently, so an unbounded map grows for the life of the
    /// pod.
    #[arg(long, env, default_value_t = 1024)]
    vehicle_cache: usize,
}

/// A worker's view of the shared configuration and its own trip state, borrowed
/// per event for [`try_create_context`](App::try_create_context).
struct App<'a> {
    gap: chrono::TimeDelta,
    jump_distance: f64,
    context_window: usize,
    trips: &'a HashCache<VehicleId, Trip<E>>,
    kv: &'a mut RedisStore<RawEvent>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = routers_realtime::telemetry::init("routers-orchestrator");

    let args = Args::parse();
    info!("orchestrator started: {:?}", args);

    let nats_url = ServerAddr::from_url(args.nats).context("could not create NATS url")?;

    let client = ConnectOptions::new()
        .name("OrchestratorService")
        .connect(nats_url)
        .await
        .context("could not connect to NATS")?;

    let mut source = NATSStream::<Payload>::new(
        client
            .subscribe(args.inbound_subject)
            .await
            .context("could not subscribe to NATS event subject")?,
    );

    let gap = chrono::Duration::from_std(args.gap).context("gap out of range")?;

    // Connected once, then cloned per worker: the clone shares the multiplexed
    // sockets, so the pod holds one connection per primary rather than one per
    // primary per worker.
    let store = RedisStore::<RawEvent>::new(&args.redis)
        .await
        .context("could not connect to redis store")?;

    let mut handles = Vec::with_capacity(args.workers);
    let mut txs = Vec::with_capacity(args.workers);

    for _ in 0..args.workers {
        let (tx, mut rx) = mpsc::channel::<Dispatch>(1024);
        txs.push(tx);

        let mut kv = store.clone();
        let client = client.clone();

        let request_subject = args.outbound_subject.clone();
        let matched_subject = args.matched_subject.clone();
        let mut sink =
            NATSSink::<MatchedEvent<E>>::new(client.clone(), move |_| matched_subject.clone());

        let context_window = args.context_window;
        let jump_distance = args.jump_distance;
        let vehicle_cache = args.vehicle_cache;
        let solve_timeout = args.solve_timeout;
        let solve_retries = args.solve_retries;

        handles.push(tokio::spawn(async move {
            let trips: HashCache<VehicleId, Trip<E>> = HashCache::with_capacity(0, vehicle_cache);

            while let Some(Dispatch {
                queued_at,
                sent_at,
                payload,
            }) = rx.recv().await
            {
                routers_realtime::bus::span_between(
                    "worker_wait",
                    queued_at,
                    routers_realtime::bus::wallclock(),
                );

                let vehicle_id = payload.vehicle_id;

                let span = info_span!(
                    "orchestrate",
                    continuation = field::Empty,
                    fresh = field::Empty,
                    cut = field::Empty,
                    attempts = field::Empty,
                );

                let mut app = App {
                    gap,
                    context_window,
                    jump_distance,
                    trips: &trips,
                    kv: &mut kv,
                };

                let context = match app
                    .try_create_context(payload)
                    .instrument(span.clone())
                    .await
                {
                    Ok(context) => context,
                    Err(err) => {
                        warn!("could not create match context: {err}");
                        continue;
                    }
                };

                // The vehicle's lane holds through the round trip: its next
                // event cannot overtake this one, so a stale solve can never
                // overwrite a fresh trip. Other vehicles overlap on other
                // workers.
                let Some(reply) = solve(
                    &client,
                    &request_subject,
                    &context,
                    solve_timeout,
                    solve_retries,
                    &span,
                )
                .instrument(span.clone())
                .await
                else {
                    continue;
                };

                if let MatchReply::Solved { diff, trip } = reply {
                    info_span!(parent: &span, "commit_result", layers = trip.layers()).in_scope(
                        || {
                            if let Some(sent_at) = sent_at {
                                routers_realtime::bus::span_between(
                                    "event_to_match",
                                    sent_at,
                                    routers_realtime::bus::wallclock(),
                                );
                            }

                            match trips.entry(vehicle_id) {
                                Entry::Occupied(mut entry) => {
                                    entry.put(trip);
                                }
                                Entry::Vacant(entry) => {
                                    entry.put_entry(trip);
                                }
                            }
                        },
                    );

                    if let Err(err) = sink
                        .send(MatchedEvent { vehicle_id, diff })
                        .instrument(info_span!(parent: &span, "publish_matched"))
                        .await
                    {
                        error!("could not publish matched event: {err:#}");
                    }
                }
            }
        }));
    }

    while let Some(payload) = source.next().await {
        // The wire stamp is only valid while this message is the stream's newest
        // yield, so capture it here rather than in the worker.
        let sent_at = routers_realtime::bus::last_sent_at();

        // The stable path (not `DefaultHasher`): worker pinning is the same
        // per-vehicle spread the fleet's partition scheme derives, so the two
        // must never disagree on a Rust release boundary.
        let worker = routers_realtime::partition::mix(payload.vehicle_id.0) as usize % args.workers;

        txs[worker]
            .send(Dispatch {
                queued_at: routers_realtime::bus::wallclock(),
                sent_at,
                payload,
            })
            .await
            .map_err(|_| anyhow::anyhow!("worker {worker} channel closed"))?;
    }

    // Dropping the senders drains each worker before the process exits.
    drop(txs);
    for handle in handles {
        handle.await.ok();
    }

    Ok(())
}

/// Ask a matcher for one context's solve, re-driving on timeout or transport
/// error. `None` when every attempt failed and the event is abandoned —
/// until durable ingest lands, this path is exactly as lossy as the bus
/// beneath it, just louder.
async fn solve(
    client: &async_nats::Client,
    subject: &str,
    context: &MatchContext<E>,
    timeout: Duration,
    retries: usize,
    span: &tracing::Span,
) -> Option<MatchReply<E>> {
    let payload = match context.encode() {
        Ok(payload) => payload,
        Err(err) => {
            error!("could not encode match context: {err:#}");
            return None;
        }
    };

    for attempt in 0..=retries {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(250) * attempt as u32).await;
        }

        let request = client.request_with_headers(
            subject.to_string(),
            routers_realtime::bus::outbound(),
            payload.clone().into(),
        );

        match tokio::time::timeout(timeout, request).await {
            Ok(Ok(message)) => {
                span.record("attempts", attempt as u64 + 1);

                match MatchReply::<E>::decode(&message.payload) {
                    Ok(reply) => return Some(reply),
                    // A decode failure is a version skew, not a transient:
                    // re-driving it would only re-fail.
                    Err(err) => {
                        error!("undecodable reply: {err:#}");
                        return None;
                    }
                }
            }
            Ok(Err(err)) => warn!("solve request failed (attempt {attempt}): {err}"),
            Err(_) => warn!("solve request timed out (attempt {attempt})"),
        }
    }

    error!("abandoning event after {} attempts", retries + 1);
    None
}

impl App<'_> {
    async fn try_create_context(
        &mut self,
        Payload {
            vehicle_id,
            timestamp,
            point,
            ..
        }: Payload,
    ) -> Result<MatchContext<E>> {
        let mut entries = self
            .kv
            .get_many(&vehicle_id, self.context_window * 3)
            .instrument(info_span!("context_fetch"))
            .await
            .context("could not get entries from redis store")?;

        entries.retain(|event| event.timestamp <= timestamp);
        entries.sort_by_key(|event| std::cmp::Reverse(event.timestamp));
        entries.truncate(self.context_window);

        let fetched = entries.len();
        let context = entries
            .into_iter()
            .inspect(|v| debug!("event: {:?}", v))
            .scan((point, timestamp), |(prev_p, prev_ts), event: RawEvent| {
                let duration = (*prev_ts - event.timestamp).abs();
                let distance = Haversine.distance(*prev_p, event.point);

                if duration <= self.gap && distance <= self.jump_distance {
                    *prev_p = event.point;
                    *prev_ts = event.timestamp;
                    Some(event)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let cut = fetched - context.len();
        if cut > 0 {
            info_span!("history_cut", reason = "gap_or_teleport").in_scope(|| {});
        }

        let mut history: Vec<RawEvent> = std::iter::once(RawEvent {
            vehicle_id: vehicle_id.clone(),
            point,
            timestamp,
        })
        .chain(context)
        .collect();

        history.sort_by_key(|event| event.timestamp);
        history.dedup_by_key(|event| event.timestamp);

        let origins = history
            .into_iter()
            .map(|event| Origin::new(event.point, event.timestamp.timestamp_micros()))
            .collect::<Vec<_>>();

        let previous = self.trips.get(&vehicle_id).map(|trip| trip.get().clone());
        let continuation =
            info_span!("reconcile").in_scope(|| Continuation::reconcile(previous, &origins));

        let span = tracing::Span::current();
        span.record("cut", cut);
        match &continuation {
            Continuation::Resume { fresh, .. } => {
                span.record("continuation", "resume");
                span.record("fresh", fresh.len());
            }
            Continuation::Restart { fresh } => {
                span.record("continuation", "restart");
                span.record("fresh", fresh.len());
            }
        }

        Ok(MatchContext {
            vehicle_id,
            continuation,
        })
    }
}
