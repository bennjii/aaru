use alloc::collections::{BTreeMap, VecDeque};
use core::time::Duration;
use std::collections::HashMap;
use web_time::Instant;

use geo::Point;
use routers_realtime::event::{MatchedDiff, MatchedEvent, VehicleId};

use crate::E;

/// A vehicle's matched history, merged from diff emissions: one geometry
/// segment per observation timestamp. Overlapping emissions supersede per
/// layer — re-emission is convergence, not conflict — so the trace heals as
/// later solves refine earlier layers.
pub struct VehicleTrace {
    /// Observation timestamp → the geometry driven into that observation
    /// (its inbound road path, then its matched position).
    layers: BTreeMap<i64, Vec<Point>>,
    pub last_seen: Instant,
}

impl VehicleTrace {
    fn new() -> Self {
        Self {
            layers: BTreeMap::new(),
            last_seen: Instant::now(),
        }
    }

    fn merge(&mut self, diff: &MatchedDiff<E>, capacity: usize) {
        for layer in &diff.layers {
            let mut segment = layer.path.clone();
            segment.push(layer.position);
            self.layers.insert(layer.timestamp, segment);
        }

        // Bound by observation count, trimming the oldest.
        while self.layers.len() > capacity {
            self.layers.pop_first();
        }

        self.last_seen = Instant::now();
    }

    /// The full tail as one point sequence, oldest observation first.
    pub fn flattened(&self) -> Vec<Point> {
        self.layers.values().flatten().copied().collect()
    }
}

pub struct StoreStats {
    pub vehicle_count: usize,
    pub events_per_sec: usize,
    pub total_events: u64,
}

/// Merged matched history per vehicle. Memory is bounded on both axes:
/// each vehicle retains at most `capacity` observations, and vehicles that
/// go quiet for longer than `idle_ttl` are evicted entirely.
pub struct TraceStore {
    capacity: usize,
    idle_ttl: Duration,
    pub traces: HashMap<VehicleId, VehicleTrace>,
    event_bucket: VecDeque<Instant>,
    total_events: u64,
}

impl TraceStore {
    pub fn new(capacity: usize, idle_ttl: Duration) -> Self {
        Self {
            capacity,
            idle_ttl,
            traces: HashMap::new(),
            event_bucket: VecDeque::new(),
            total_events: 0,
        }
    }

    pub fn ingest(&mut self, result: MatchedEvent<E>) {
        let now = Instant::now();

        self.event_bucket.push_back(now);
        self.total_events += 1;

        if result.diff.layers.is_empty() {
            return;
        }

        // Layers merge by observation timestamp, so the newest one is the
        // vehicle's current position, which the plugin marks with the head
        // dot.
        self.traces
            .entry(result.vehicle_id)
            .or_insert_with(VehicleTrace::new)
            .merge(&result.diff, self.capacity);
    }

    pub fn evict_idle(&mut self) {
        let now = Instant::now();

        self.traces
            .retain(|_, trace| now.duration_since(trace.last_seen) < self.idle_ttl);

        while self
            .event_bucket
            .front()
            .is_some_and(|t| now.duration_since(*t).as_secs() >= 2)
        {
            self.event_bucket.pop_front();
        }
    }

    pub fn stats(&self) -> StoreStats {
        let now = Instant::now();
        let recent = self
            .event_bucket
            .iter()
            .filter(|t| now.duration_since(**t) < Duration::from_secs(1))
            .count();

        StoreStats {
            vehicle_count: self.traces.len(),
            events_per_sec: recent,
            total_events: self.total_events,
        }
    }
}
