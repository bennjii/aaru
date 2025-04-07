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

pub struct TransitionLengths {
    /// The great circle distance between source and target
    pub straightline_distance: f64,

    /// The path of the optimal route between candidates
    pub route_length: f64,
}

impl TransitionLengths {
    /// Calculates the deviance in straightline distance to the length
    /// of the entire route. Returns values between 0 and 1. Where values
    /// closer to 1 represent more optimal distances, whilst those closer
    /// to 0 represent less optimal distances.
    ///
    /// The route length is defined as the cumulative distance between
    /// nodes in the optimal transition path, plus the offsets into the
    /// edges by which the candidates live.
    ///
    /// The straightline distance is defined as the haversine (great circle)
    /// distance between the two candidates.
    ///
    /// Therefore, our deviance is defined as the ratio of straightline
    /// distance to the route length, which measures how much farther
    /// the actual route was than a virtual path directly between the candidates.
    ///
    /// For example:
    /// -   If two candidates were `100m` apart, but had a most optimal route
    ///     between them of `130m`, the deviance would be `~0.77`.
    /// -   If two alternate candidates were `100m` apart but instead had an
    ///     optimal route between them of `250m`, the deviance is `0.4`.
    ///
    /// Note that a lower deviance score means the values are less aligned.
    pub fn deviance(&self) -> f64 {
        (self.straightline_distance / self.route_length).clamp(0.0, 1.0)
    }
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

    pub fn lengths(&self) -> Option<TransitionLengths> {
        let (source, target) = self.candidates();
        let offset = self.total_offset(&source, &target)?;

        let route_length = self.optimal_path.length() + offset;
        let straightline_distance = Haversine::distance(source.position, target.position);

        Some(TransitionLengths {
            straightline_distance,
            route_length,
        })
    }
}
