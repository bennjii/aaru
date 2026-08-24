use serde::{Deserialize, Serialize};

/// A linear-referenced range (`between: [start, end]`), normalised so that
/// `start <= end` with both endpoints in `0..=1`.
///
/// Many segment attributes apply only to a sub-range of the segment, using
/// the same parameterisation as a connector's `at`. Attribute scoping by
/// `between` is not yet applied during graph assembly; the type carries the
/// range through for that follow-up.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Between {
    pub start: f64,
    pub end: f64,
}

impl Between {
    /// Builds a range, clamping to `0..=1` and ordering the endpoints — the
    /// schema does not enforce `start < end`.
    #[inline]
    pub fn new(a: f64, b: f64) -> Self {
        let a = a.clamp(0.0, 1.0);
        let b = b.clamp(0.0, 1.0);
        Between {
            start: a.min(b),
            end: a.max(b),
        }
    }

    #[inline]
    pub fn overlaps(&self, other: &Between) -> bool {
        self.start < other.end && other.start < self.end
    }
}
