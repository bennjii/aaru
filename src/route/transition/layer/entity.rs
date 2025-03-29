use geo::Point;

use crate::route::transition::candidate::CandidateId;

pub struct Layer {
    pub nodes: Vec<CandidateId>,
    pub origin: Point,
}
