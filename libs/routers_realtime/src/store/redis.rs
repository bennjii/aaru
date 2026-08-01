use std::collections::HashMap;

use futures::future::try_join_all;
use redis::aio::MultiplexedConnection;
use redis::streams::StreamRangeReply;
use thiserror::Error;
use url::Url;

use crate::store::Storable;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("redis: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("serialisation: {0}")]
    Serialisation(#[from] postcard::Error),
    #[error("no redis endpoints supplied")]
    NoEndpoints,
}

type Result<T> = std::result::Result<T, StoreError>;

/// FNV-1a. Stable by construction, which `DefaultHasher` is not: its algorithm
/// may change between Rust releases, and every binary in the fleet has to agree
/// on where a vehicle lives.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// splitmix64 finaliser. FNV-1a avalanches poorly on its own, and rendezvous
/// hashing compares scores between nodes, so weak mixing would skew placement.
fn mix(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Which primary owns a key. Held apart from the connections so the mapping —
/// the part that has to stay stable for history to survive — is testable
/// without a server.
#[derive(Clone)]
struct Placement {
    /// One hash per endpoint URL. A primary's identity is its URL rather than
    /// its position, so reordering the fleet moves no vehicle.
    seeds: Vec<u64>,
}

impl Placement {
    fn new(urls: &[Url]) -> Self {
        Self {
            seeds: urls.iter().map(|url| fnv1a(url.as_str().as_bytes())).collect(),
        }
    }

    /// Rendezvous (highest-random-weight) placement. Plain modulo would remap
    /// nearly every vehicle when the fleet changes size, discarding the history
    /// that keeps trips continuous; this remaps about 1/N of them.
    fn index_for(&self, key: &str) -> usize {
        let hash = fnv1a(key.as_bytes());

        self.seeds
            .iter()
            .enumerate()
            // The seed breaks ties, so the winner never depends on list order.
            .max_by_key(|(_, seed)| (mix(hash ^ **seed), **seed))
            .map(|(index, _)| index)
            .expect("fleet is non-empty, checked in RedisStore::new")
    }
}

/// A fleet of independent Valkey primaries, addressed by vehicle.
///
/// The keyspace is `vehicle:<id>:positions`. A vehicle must reach the same
/// primary wherever it drives, because the shared history is what lets a trip
/// continue across a shard boundary — so placement is by vehicle and never by
/// geography.
///
/// Primaries are independent, so this needs no Redis Cluster protocol: no slot
/// map, no `MOVED` handling, no cluster bus. Cloning shares the underlying
/// sockets, so a worker pool clones one store rather than opening its own.
#[derive(Clone)]
pub struct RedisStore<T: Storable> {
    conns: Vec<MultiplexedConnection>,
    placement: Placement,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Storable> RedisStore<T> {
    pub async fn new(urls: &[Url]) -> Result<Self> {
        if urls.is_empty() {
            return Err(StoreError::NoEndpoints);
        }

        let conns = try_join_all(urls.iter().map(|url| async move {
            redis::Client::open(url.clone())?
                .get_multiplexed_async_connection()
                .await
        }))
        .await?;

        Ok(Self {
            conns,
            placement: Placement::new(urls),
            _phantom: std::marker::PhantomData,
        })
    }

    pub async fn get_many(&mut self, vehicle_id: &str, len: usize) -> Result<Vec<T>> {
        let key = format!("vehicle:{}:positions", vehicle_id);
        let node = self.placement.index_for(&key);

        let reply: StreamRangeReply = redis::cmd("XREVRANGE")
            .arg(&key)
            .arg("+")
            .arg("-")
            .arg("COUNT")
            .arg(len)
            .query_async(&mut self.conns[node])
            .await?;

        let mut entries = Vec::with_capacity(reply.ids.len());

        for stream_id in &reply.ids {
            let value = match stream_id.map.get("val") {
                Some(redis::Value::BulkString(b)) => b.as_slice(),
                _ => continue,
            };

            let entry: T = postcard::from_bytes(value)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    pub async fn write_many(&mut self, batch: &[T], limit: usize) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        // A batch spans vehicles and vehicles spread across the fleet, so this
        // is a scatter: one pipeline per primary the batch actually touches.
        let mut pipelines: HashMap<usize, redis::Pipeline> = HashMap::new();

        for item in batch {
            let key = format!("vehicle:{}:positions", item.key());
            let value = postcard::to_allocvec(item)?;

            pipelines
                .entry(self.placement.index_for(&key))
                .or_insert_with(redis::pipe)
                .cmd("XADD")
                .arg(key)
                .arg("MAXLEN")
                .arg("~")
                .arg(limit)
                .arg("*")
                .arg("shard")
                .arg(item.shard_id().to_string())
                .arg("val")
                .arg(value)
                .ignore();
        }

        // Concurrently. Awaiting each primary in turn would multiply the flush
        // latency by the number of primaries the batch reached.
        try_join_all(pipelines.into_iter().map(|(node, pipeline)| {
            let mut conn = self.conns[node].clone();
            async move { pipeline.query_async::<()>(&mut conn).await }
        }))
        .await?;

        Ok(())
    }
}

pub struct CachedRedisStore<T: Storable> {
    store: RedisStore<T>,
    cache: HashMap<String, Vec<T>>,
}

impl<T: Storable> CachedRedisStore<T> {
    pub fn new(store: RedisStore<T>) -> Self {
        Self {
            store,
            cache: HashMap::new(),
        }
    }

    pub async fn get_many(&mut self, vehicle_id: &str, len: usize) -> Result<Vec<T>> {
        if let Some(cached) = self.cache.get(vehicle_id).cloned() {
            return Ok(cached);
        }

        let entries = self.store.get_many(vehicle_id, len).await?;
        self.cache.insert(vehicle_id.to_string(), entries.clone());
        Ok(entries)
    }

    /// Roll an observed item into the cached window (newest-first), so
    /// subsequent reads see it without re-querying the backing store. Without
    /// this the cache is a frozen snapshot of the first read — which, for a
    /// consumer racing the writer on a vehicle's first event, is empty
    /// forever.
    pub fn push(&mut self, key: &str, item: T, len: usize) {
        let entries = self.cache.entry(key.to_string()).or_default();
        entries.insert(0, item);
        entries.truncate(len);
    }

    pub async fn write_many(&mut self, batch: &[T], limit: usize) -> Result<()> {
        self.store.write_many(batch, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::Placement;
    use url::Url;

    fn fleet(n: usize) -> Vec<Url> {
        (0..n)
            .map(|i| Url::parse(&format!("redis://valkey-{i:03}:6379")).unwrap())
            .collect()
    }

    fn keys(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("vehicle:{i}:positions")).collect()
    }

    #[test]
    fn placement_is_deterministic() {
        let urls = fleet(20);
        let (a, b) = (Placement::new(&urls), Placement::new(&urls));

        for key in keys(1000) {
            assert_eq!(a.index_for(&key), b.index_for(&key));
        }
    }

    /// The fleet is a set, not a list. Two binaries handed the same primaries in
    /// different orders must still agree on where a vehicle lives, or each would
    /// see only part of its history.
    #[test]
    fn placement_ignores_url_order() {
        let urls = fleet(20);
        let mut shuffled = urls.clone();
        shuffled.reverse();

        let (direct, reversed) = (Placement::new(&urls), Placement::new(&shuffled));

        for key in keys(1000) {
            let expected = &urls[direct.index_for(&key)];
            let actual = &shuffled[reversed.index_for(&key)];
            assert_eq!(
                expected, actual,
                "{} moved when the list was reordered",
                key
            );
        }
    }

    /// Rendezvous hashing's reason for being. Modulo would move ~19/20 of the
    /// keyspace here; anything near that would discard the history that keeps
    /// trips continuous across a shard boundary.
    #[test]
    fn growing_the_fleet_moves_about_one_nth_of_keys() {
        let before = Placement::new(&fleet(20));
        let after = Placement::new(&fleet(21));

        let sample = keys(10_000);
        let moved = sample
            .iter()
            .filter(|key| {
                // Indices are comparable because fleet(21) extends fleet(20).
                before.index_for(key) != after.index_for(key)
            })
            .count();

        // The ideal is 1/21 ≈ 4.8%. Allow slack for hash noise at this sample
        // size, but stay far below the ~95% modulo would produce.
        let ratio = moved as f64 / sample.len() as f64;
        assert!(ratio < 0.10, "{:.1}% of keys moved, expected ~4.8%", ratio * 100.0);
    }

    #[test]
    fn placement_spreads_across_the_fleet() {
        let size = 20;
        let placement = Placement::new(&fleet(size));

        let mut counts = vec![0usize; size];
        for key in keys(20_000) {
            counts[placement.index_for(&key)] += 1;
        }

        // 1000 expected per primary. A primary taking nothing, or several times
        // its share, would mean the mixing step is not doing its job.
        for (index, count) in counts.iter().enumerate() {
            assert!(
                (500..2000).contains(count),
                "primary {} took {} of 20000 keys",
                index,
                count
            );
        }
    }
}
