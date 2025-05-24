use crate::transition::*;
use codec::primitive::Entry;

#[derive(Debug, Default, Copy, Clone)]
pub enum ResolutionMethod {
    #[default]
    Standard,
    DistanceOnly,
}

/// Defines a [target](#field.target) element reachable from some given
/// [source](#field.source) through a known [path](#field.path).
///
/// It requests itself to be resolved in the heuristic-layer by a given
/// [resolution_method](#field.resolution_method).
#[derive(Clone)]
pub struct Reachable<E>
where
    E: Entry,
{
    pub source: CandidateId,
    pub target: CandidateId,
    pub path: Vec<E>,

    pub(crate) resolution_method: ResolutionMethod,
}

impl<E> Reachable<E>
where
    E: Entry,
{
    /// Creates a new reachable element, supplied a source, target and path.
    ///
    /// This assumes the default resolution method.
    pub fn new(source: CandidateId, target: CandidateId, path: Vec<E>) -> Self {
        Self {
            source,
            target,
            path,
            resolution_method: Default::default(),
        }
    }

    /// Consumes and modifies a reachable element to request the
    /// [`DistanceOnly`](ResolutionMethod::DistanceOnly) option.
    pub fn distance_only(self) -> Self {
        Self {
            resolution_method: ResolutionMethod::DistanceOnly,
            ..self
        }
    }

    /// Converts a reachable element into a (source, target) index pair
    /// used for hashing the structure as a path lookup between the
    /// source and target.
    pub fn hash(&self) -> (usize, usize) {
        (self.source.index(), self.target.index())
    }
}

/// Defines a structure which can be supplied to the [`Transition::solve`] function
/// in order to solve the transition graph.
///
/// Functionality is implemented using the [`Solver::solve`] method.
pub trait Solver<Ent> {
    /// Refines a single node within an initial layer to all nodes in the
    /// following layer with their respective emission and transition
    /// probabilities in the hidden markov model.
    ///
    /// It may return a match error which is encountered for various reasons.
    /// This may be due to insufficient candidates for a given node in the sequence,
    /// or due to blown-out costings. There are other reasons this may occur given
    /// the functionality is statistical and therefore prone to out-of-bound failures
    /// which are less deterministic than a brute-force model.
    fn solve<E, T>(&self, transition: Transition<E, T, Ent>) -> Result<Collapse<Ent>, MatchError>
    where
        Ent: Entry,
        E: EmissionStrategy + Send + Sync,
        T: TransitionStrategy<Ent> + Send + Sync;
}
