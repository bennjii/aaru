use crate::route::graph::{NodeIx, Weight};
use crate::route::transition::candidate::{Candidate, CandidateId, Candidates};
use crate::route::Graph;
use crate::route::transition::DirectionAwareEdgeId;

/// A base context provided to costing methods, allowing costing methods
/// to access to further information within the current routing
/// progress at the discretion of the implementer.
#[derive(Clone, Copy, Debug)]
pub struct RoutingContext<'a> {
    pub candidates: &'a Candidates,
    pub map: &'a Graph,
}

impl RoutingContext<'_> {
    /// TODO: Docs
    pub fn candidate(&self, candidate: &CandidateId) -> Option<Candidate> {
        self.candidates.candidate(candidate)
    }

    pub fn edge(&self, a: &NodeIx, b: &NodeIx) -> Option<&(Weight, DirectionAwareEdgeId)> {
        self.map.graph.edge_weight(*a, *b)
    }
}
