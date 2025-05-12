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
    /// It may return a match error which is encountered for various reasons.
    /// This may be due to insufficient candidates for a given node in the sequence,
    /// or due to blown-out costings. There are other reasons this may occur given
    /// the functionality is statistical and therefore prone to out-of-bound failures
    /// which are less deterministic than a brute-force model.
    fn solve<E, T>(&self, transition: Transition<E, T>) -> Result<Collapse, MatchError>
    where
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy + Send + Sync;
}
