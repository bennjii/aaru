use crate::route::transition::{Strategy, TransitionCandidate, Trip};

pub trait TransitionStrategy: for<'a> Strategy<TransitionContext<'a>> {}
impl<T> TransitionStrategy for T where T: for<'a> Strategy<TransitionContext<'a>> {}

#[derive(Clone, Copy, Debug)]
pub struct TransitionContext<'a> {
    /// The optimal path travelled between the
    /// source candidate and target candidate, used
    /// to determine trip complexity (and therefore
    /// cost) often through heuristics such as
    /// immediate and summative angular rotation.
    pub optimal_path: Trip<'a>,

    /// The source candidate indicating the edge and
    /// position for which the path begins at.
    pub source_candidate: &'a TransitionCandidate,

    /// The target candidate indicating the edge and
    /// position for which the path ends at.
    pub target_candidate: &'a TransitionCandidate,
}
