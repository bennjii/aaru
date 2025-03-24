use geo::Point;

use crate::route::transition::candidate::{Candidate, CandidateId};

pub struct Layer {
    pub nodes: Vec<CandidateId>,
    pub origin: Point,
}
