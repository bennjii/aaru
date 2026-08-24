//! Describes the minimal `Segment` structure: an Overture linestring
//! referencing two or more connectors, each at a fractional position
//! (`at` ∈ `0..=1`) along its length. Splitting the segment at its sorted
//! connector positions yields its chain of graph edges, with the linestring's
//! interior vertices materialised between them so edge geometry follows the
//! road shape.

use geo::{Coord, LineString};
use routers_network::edge::Weight;

use crate::overture::id::OvertureEntryId;
use crate::overture::parsers::{AccessRestriction, Heading, RoadClass, SpeedLimit};

/// A reference from a segment to one of its connectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentConnector {
    pub id: OvertureEntryId,
    /// Fractional position along the segment, `0..=1`.
    pub at: f64,
}

/// A parsed, routable-candidate segment.
///
/// `connectors` are stored sorted ascending by `at` (see [`Segment::new`]),
/// so consecutive pairs form the edge chain directly.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub id: OvertureEntryId,
    /// The segment's linestring. May be empty, in which case edges between
    /// connectors are straight chords.
    pub geometry: LineString,
    pub connectors: Vec<SegmentConnector>,
    pub road_class: Option<RoadClass>,
    /// `subclass: link` — a ramp/slip road; adds a small weight penalty.
    pub is_link: bool,
    pub speed_limits: Vec<SpeedLimit>,
    pub access: Vec<AccessRestriction>,
}

impl Segment {
    /// Builds a segment, sorting its connectors by `at`. `NaN` positions
    /// keep their relative order but should not occur in valid data.
    pub fn new(
        id: OvertureEntryId,
        geometry: LineString,
        mut connectors: Vec<SegmentConnector>,
        road_class: Option<RoadClass>,
        is_link: bool,
        speed_limits: Vec<SpeedLimit>,
        access: Vec<AccessRestriction>,
    ) -> Self {
        connectors
            .sort_by(|a, b| a.at.partial_cmp(&b.at).unwrap_or(core::cmp::Ordering::Equal));
        Segment {
            id,
            geometry,
            connectors,
            road_class,
            is_link,
            speed_limits,
            access,
        }
    }

    /// Whether this segment should be added to the routable graph.
    #[inline]
    pub fn navigable(&self) -> bool {
        self.road_class.is_some_and(|c| c.navigable()) && self.connectors.len() >= 2
    }

    /// The routing weight for every sub-edge of this segment: the road-class
    /// base weight, plus one for link (ramp) segments so ramps are marginally
    /// less preferred than the road they join.
    #[inline]
    pub fn weight(&self) -> Weight {
        let base = self.road_class.map_or(Weight::MAX, |c| c.weighting());
        if self.is_link { base.saturating_add(1) } else { base }
    }

    /// Whether driving is permitted in the given heading.
    ///
    /// Open by default; a `denied` access restriction scoped to the heading
    /// closes that direction, yielding a one-way segment.
    #[inline]
    pub fn open(&self, heading: Heading) -> bool {
        !self.access.iter().any(|a| a.denies_driving(heading))
    }

    /// The linestring's interior vertices strictly between the fractional
    /// positions `from` and `to` (each `0..=1` of the segment's length), in
    /// travel order.
    ///
    /// These become synthetic graph nodes between two connectors, so edge
    /// geometry follows the road shape instead of cutting the corner. An
    /// empty geometry yields no vertices. Vertices coincident with either
    /// bound (within a hundredth of the segment's length) are skipped — the
    /// connectors themselves already sit there.
    pub fn interior_vertices(&self, from: f64, to: f64) -> impl Iterator<Item = Coord> + '_ {
        // Cumulative length at each vertex, with longitude compressed by
        // cos(latitude) so the fractions track the segment-length
        // parameterisation `at` is defined over.
        let compression = self
            .geometry
            .0
            .first()
            .map_or(1.0, |c| c.y.to_radians().cos());

        let mut cumulative = 0.0;
        let lengths: Vec<f64> = core::iter::once(0.0)
            .chain(self.geometry.lines().map(|line| {
                cumulative += ((line.dx() * compression).powi(2) + line.dy().powi(2)).sqrt();
                cumulative
            }))
            .collect();

        let total = lengths.last().copied().unwrap_or(0.0);
        let epsilon = total * 1e-3;
        let (lo, hi) = (
            from.min(to) * total + epsilon,
            from.max(to) * total - epsilon,
        );

        self.geometry
            .coords()
            .copied()
            .zip(lengths)
            .filter(move |(_, at)| *at > lo && *at < hi)
            .map(|(coord, _)| coord)
    }
}
