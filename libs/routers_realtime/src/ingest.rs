//! The durable ingest topology: how raw events partition across JetStream.
//!
//! Like [`partition`](crate::partition), everything here is wire law.
//! Producers choose a vehicle's subject, and every revision downstream is a
//! stream sequence — so the partition→subject and partition→stream mappings
//! must never move without a coordinated migration. In particular, moving a
//! partition to a different stream resets its sequence domain and breaks
//! every revision comparison across the boundary: the stream count is fixed
//! fleet-wide config, not a tunable.
//!
//! Raw streams are work queues (a message deletes on ack, so disk holds
//! exactly the backlog) split across several streams because one stream is
//! one raft leader. Consumers filter one partition subject each, which
//! requires the non-overlapping filtered consumers NATS server 2.10 added.

use core::time::Duration;

use anyhow::Context as _;
use async_nats::jetstream;

use crate::partition::PARTITIONS;

/// Subject prefix for partitioned raw events: a vehicle's events publish to
/// `events.raw.p.<splitmix64(vehicle_id) % PARTITIONS>`.
pub const RAW_PREFIX: &str = "events.raw.p";

/// Subject prefix for matched emissions, partitioned identically.
pub const MATCHED_PREFIX: &str = "events.matched.p";

/// The single stream retaining matched emissions for the reconciler (and any
/// late observer) to read.
pub const MATCHED_STREAM: &str = "EVENTS-MATCHED";

/// The raw-event subject of one partition.
pub fn raw_subject(partition: u64) -> String {
    format!("{RAW_PREFIX}.{partition}")
}

/// The matched-emission subject of one partition.
pub fn matched_subject(partition: u64) -> String {
    format!("{MATCHED_PREFIX}.{partition}")
}

/// Which raw stream holds `partition`, out of `streams` total: contiguous
/// ranges, so a stream's subjects read as one block.
pub fn stream_index(partition: u64, streams: u64) -> u64 {
    partition / PARTITIONS.div_ceil(streams)
}

/// The name of one raw stream.
pub fn stream_name(index: u64) -> String {
    format!("EVENTS-RAW-{index}")
}

/// Idempotently create (or bind) the raw stream `index` of `streams`.
pub async fn raw_stream(
    context: &jetstream::Context,
    index: u64,
    streams: u64,
) -> anyhow::Result<jetstream::stream::Stream> {
    let chunk = PARTITIONS.div_ceil(streams);
    let partitions = (index * chunk)..(((index + 1) * chunk).min(PARTITIONS));

    context
        .get_or_create_stream(jetstream::stream::Config {
            name: stream_name(index),
            subjects: partitions.map(raw_subject).collect(),
            retention: jetstream::stream::RetentionPolicy::WorkQueue,
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        })
        .await
        .with_context(|| format!("could not create raw stream {index}"))
}

/// Idempotently create (or bind) the matched stream. Time-limited retention:
/// nothing consumes it destructively, and the reconciler may lag.
pub async fn matched_stream(
    context: &jetstream::Context,
    max_age: Duration,
) -> anyhow::Result<jetstream::stream::Stream> {
    context
        .get_or_create_stream(jetstream::stream::Config {
            name: MATCHED_STREAM.into(),
            subjects: vec![format!("{MATCHED_PREFIX}.>")],
            retention: jetstream::stream::RetentionPolicy::Limits,
            storage: jetstream::stream::StorageType::File,
            max_age,
            ..Default::default()
        })
        .await
        .context("could not create matched stream")
}

/// The durable consumer owning one partition's raw events. Redelivery after
/// `ack_wait` is the crash-recovery path; `max_ack_pending` bounds how far a
/// slow owner lets its unacked window grow — the backlog knob.
pub async fn partition_consumer(
    stream: &jetstream::stream::Stream,
    partition: u64,
    max_ack_pending: i64,
    ack_wait: Duration,
) -> anyhow::Result<jetstream::consumer::PullConsumer> {
    let name = format!("orchestrator-p{partition}");

    stream
        .get_or_create_consumer(
            &name,
            jetstream::consumer::pull::Config {
                durable_name: Some(name.clone()),
                filter_subject: raw_subject(partition),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                max_ack_pending,
                ack_wait,
                ..Default::default()
            },
        )
        .await
        .with_context(|| format!("could not create consumer for partition {partition}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every partition maps into exactly one stream, streams cover contiguous
    /// blocks, and the whole space is covered — for any stream count.
    #[test]
    fn streams_partition_the_partition_space() {
        for streams in [1u64, 2, 3, 4, 8, 32, 1024] {
            let mut last = 0;
            for partition in 0..PARTITIONS {
                let index = stream_index(partition, streams);
                assert!(index < streams, "index {index} escapes {streams} streams");
                assert!(index >= last, "stream blocks must be contiguous");
                last = index;
            }
        }
    }

    #[test]
    fn subjects_are_distinct_per_partition() {
        assert_eq!(raw_subject(0), "events.raw.p.0");
        assert_eq!(raw_subject(1023), "events.raw.p.1023");
        assert_eq!(matched_subject(485), "events.matched.p.485");
    }
}
