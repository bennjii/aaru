use crate::codec::element::variants::OsmEntryId;
use crate::route::transition::graph::{MatchError, Transition};
use crate::route::transition::*;

#[derive(Debug, Default, Copy, Clone)]
pub enum ResolutionMethod {
    #[default]
    Standard,
    DistanceOnly,
}

#[derive(Clone)]
pub struct Reachable {
    pub source: CandidateId,
    pub target: CandidateId,
    pub path: Vec<OsmEntryId>,

    pub(crate) resolution_method: ResolutionMethod,
}

impl Reachable {
    pub fn new(source: CandidateId, target: CandidateId, path: Vec<OsmEntryId>) -> Self {
        Self {
            source,
            target,
            path,
            resolution_method: Default::default(),
        }
    }

    pub fn distance_only(self) -> Self {
        Self {
            resolution_method: ResolutionMethod::DistanceOnly,
            ..self
        }
    }

    pub fn hash(&self) -> (usize, usize) {
        (self.source.index(), self.target.index())
    }
}

pub trait Solver {
    /// Refines a single node within an initial layer to all nodes in the
    /// following layer with their respective emission and transition
    /// probabilities in the hidden markov model.
    ///
    /// Based on the method used in FMM / MM2
    fn solve<E, T>(&self, transition: Transition<E, T>) -> Result<Collapse, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync;
}
