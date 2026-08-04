use geo::Point;
use serde::{Deserialize, Serialize};

/// One observed position and when it was observed.
///
/// The timestamp is microseconds since the Unix epoch, minted by the supplier
/// at the ingest boundary — the per-vehicle ordering and identity key for
/// everything derived from the observation. This crate never interprets it
/// beyond equality; it rides along so every trip layer stays addressable by
/// the observation that created it.
///
/// Equality is exact on both fields: two observations sharing a timestamp but
/// not a position contradict each other, and a resume must refuse the pair.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    pub point: Point,

    /// Microseconds since the Unix epoch.
    pub timestamp: i64,
}

impl Origin {
    pub fn new(point: Point, timestamp: i64) -> Self {
        Self { point, timestamp }
    }
}
