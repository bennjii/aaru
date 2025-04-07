use crate::route::transition::candidate::{Candidate, CandidateId};
use crate::route::transition::{OffsetVariant, RoutingContext, Strategy, Trip};
use geo::{Distance, Haversine};

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

    /// Returns the (Source, Target) candidate
    pub fn candidates(&self) -> (Candidate, Candidate) {
        (self.source_candidate(), self.target_candidate())
    }

    /// Calculates the total offset, of both source and target positions within the context
    pub fn total_offset(&self, source: &Candidate, target: &Candidate) -> Option<f64> {
        let inner_offset = source.offset(&self.routing_context, OffsetVariant::Inner)?;
        let outer_offset = target.offset(&self.routing_context, OffsetVariant::Outer)?;

        Some(inner_offset + outer_offset)
    }

    pub fn total_distance(&self) -> Option<f64> {
        let (source, target) = self.candidates();

        let offset = self.total_offset(&source, &target)?;
        let distance = Haversine::distance(source.position, target.position);

        Some(distance + offset)
    }
}
