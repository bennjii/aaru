use crate::codec::element::variants::OsmEntryId;
use crate::route::transition::graph::{MatchError, Transition};
use crate::route::transition::*;

pub struct Reachable {
    pub candidate: CandidateId,
    pub path: Vec<OsmEntryId>,
}

impl Reachable {
    pub fn new(candidate: CandidateId, path: Vec<OsmEntryId>) -> Self {
        Self { candidate, path }
    }
}

pub trait Solver {
    /// Derives which candidates are reachable by the source candidate.
    ///
    /// Provides a slice of target candidate IDs, `targets`. The solver
    /// will use these to procure all candidates which are reachable,
    /// and the path of routable entries ([`OsmEntryId`]) which are used
    /// to reach the target.
    fn reachable<'a>(
        &self,
        ctx: RoutingContext<'a>,
        source: &CandidateId,
        targets: &'a [CandidateId],
    ) -> Option<Vec<Reachable>>;

    fn prework<E, T>(&self, transition: &Transition<E, T>)
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync;

    /// Refines a single node within an initial layer to all nodes in the
    /// following layer with their respective emission and transition
    /// probabilities in the hidden markov model.
    ///
    /// Based on the method used in FMM / MM2
    fn solve<E, T>(&self, transition: Transition<E, T>) -> Result<Collapse, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync,
    {
        self.prework(&transition);
        transition.collapse()
    }
}
