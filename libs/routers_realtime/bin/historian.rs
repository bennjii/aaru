use anyhow::Context;
use async_nats::{ConnectOptions, ServerAddr};
use clap::Parser;
use futures::StreamExt;
use log::{error, info};
use std::time::Duration;
use tokio::time::{Instant, timeout_at};
use tracing::{Instrument, info_span};
use url::Url;

use routers_realtime::{
    bus::NATSStream,
    event::{Payload, RawEvent},
    store::RedisStore,
};

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

    /// The subject to use for the NATS events stream.
    /// For example, "events.position.9q.*" to consume a whole cell.
    #[arg(short, long, env)]
    subject: String,

    /// NATS queue group to join. Members of a group share the subject's
    /// deliveries instead of each receiving every message, which is what lets a
    /// cell run more than one historian. Unset, this is a plain subscription
    /// and a second replica would archive every event a second time.
    #[arg(long, env)]
    queue_group: Option<String>,

    /// The number of events to keep in the Redis history
    #[arg(long, env, default_value_t = 25)]
    history: usize,

    /// Batch size for Redis publishing
    #[arg(long, env, default_value_t = 1024)]
    batch_size: usize,

    /// Batch timeout for Redis publishing
    #[arg(long, env, value_parser = humantime::parse_duration, default_value = "100ms")]
    batch_timeout: Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _telemetry = routers_realtime::telemetry::init("routers-historian");
    let args = Args::parse();

    info!("historian starting: {:?}", args);

    let nats_url = ServerAddr::from_url(args.nats).context("could not create NATS url")?;

    let client = ConnectOptions::new()
        .name("HistorianService")
        .connect(nats_url)
        .await
        .context("could not connect to NATS")?;
    let subscriber = match args.queue_group {
        Some(group) => client.queue_subscribe(args.subject, group).await,
        None => client.subscribe(args.subject).await,
    }
    .context("could not subscribe to NATS subject")?;

    let mut nats = NATSStream::<Payload>::new(subscriber);

    let mut kv = RedisStore::<RawEvent>::new(&args.redis)
        .await
        .context("could not connect to redis store")?;
    let mut batch: Vec<RawEvent> = Vec::with_capacity(args.batch_size);

    loop {
        batch.clear();
        let deadline = Instant::now() + args.batch_timeout;

        while batch.len() < batch.capacity() {
            match timeout_at(deadline, nats.next()).await {
                Ok(Some(e)) => batch.push(e.as_event()),
                _ => break,
            }
        }

        if batch.is_empty() {
            continue;
        }

        if let Err(e) = kv
            .write_many(&batch, args.history)
            .instrument(info_span!("archive", events = batch.len()))
            .await
        {
            error!("write error: {e}");
        } else {
            info!("archived {} event(s)", batch.len());
        }
    }
}
