use geo::{Bearing, Distance, Geodesic, Haversine, Point};

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
        TrajectorySegment {
            source: a,
            target: b,
            length: Haversine::distance(*a, *b),
            bearing: Geodesic::bearing(*a, *b),
        }
    }
}
