use crate::route::transition::Strategy;

pub trait EmissionStrategy: for<'a> Strategy<EmissionContext<'a>> {}
impl<T> EmissionStrategy for T where T: for<'a> Strategy<EmissionContext<'a>> {}

#[derive(Clone, Copy, Debug)]
pub struct EmissionContext<'a> {
    /// The proposed (candidate) position to be matched onto.
    ///
    /// This belongs to the network, and is not provided
    /// as input to the match query.
    pub candidate_position: &'a geo::Point,

    /// The position the costing method is matching.
    ///
    /// This belongs to the un-matched trip, as the position
    /// which must be matched upon the network.
    pub source_position: &'a geo::Point,
}

impl<'a> EmissionContext<'a> {
    pub fn new(candidate: &'a geo::Point, source: &'a geo::Point) -> Self {
        Self {
            candidate_position: candidate,
            source_position: source,
        }
    }
}
