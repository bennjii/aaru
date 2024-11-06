use geo::{GeodesicBearing, HaversineDistance, Point};
use log::debug;

pub struct TrajectorySegment<'a> {
    pub source: &'a Point,
    pub target: &'a Point,

    // The Euclidean length of the segment
    // ahead of the current node.
    //
    //  This + ----------‐ + Next
    //           ^ Euclidean length of line
    //
    pub length: f64,
    pub bearing: f64,
}

impl<'a> TrajectorySegment<'a> {
    #[inline]
    pub fn new(a: &'a Point, b: &'a Point) -> Self {
        debug!(
            "Segment length {} between {:?} and {:?}",
            a.haversine_distance(b),
            a,
            b
        );
        TrajectorySegment {
            source: a,
            target: b,
            length: a.haversine_distance(b),
            bearing: a.geodesic_bearing(*b),
        }
    }
}
