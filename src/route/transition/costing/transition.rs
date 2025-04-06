use crate::route::transition::candidate::{Candidate, CandidateId};
use crate::route::transition::{RoutingContext, Strategy, Trip};

pub trait TransitionStrategy: for<'a> Strategy<TransitionContext<'a>> {}
impl<T> TransitionStrategy for T where T: for<'a> Strategy<TransitionContext<'a>> {}

#[derive(Clone, Debug)]
pub struct TransitionContext<'a> {
    /// The optimal path travelled between the
    /// source candidate and target candidate, used
    /// to determine trip complexity (and therefore
    /// cost) often through heuristics such as
    /// immediate and summative angular rotation.
    pub optimal_path: Trip,

    /// The source candidate indicating the edge and
    /// position for which the path begins at.
    pub source_candidate: &'a CandidateId,

    /// The target candidate indicating the edge and
    /// position for which the path ends at.
    pub target_candidate: &'a CandidateId,

    /// Further context to provide access to determine routing information,
    /// such as node positions upon the map, and referencing other candidates.
    pub routing_context: RoutingContext<'a>,
}

impl TransitionContext<'_> {
    /// TODO: Docs
    pub fn source_candidate(&self) -> Candidate {
        self.routing_context
            .candidate(self.source_candidate)
            .expect("source candidate not found in routing context")
    }

    /// TODO: Docs
    pub fn target_candidate(&self) -> Candidate {
        self.routing_context
            .candidate(self.target_candidate)
            .expect("target candidate not found in routing context")
    }
}
