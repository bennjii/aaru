use geo::Point;

use crate::route::transition::candidate::CandidateId;

pub struct Layer {
    /// All the candidates detected within the layer, as
    /// positions the [origin](#field.origin) could be matched to.
    pub nodes: Vec<CandidateId>,

    /// TODO: Docs
    ///
    /// The raw, possibly GPS-derived point being matched
    pub origin: Point,
}
