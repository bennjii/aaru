use crate::Graph;
use crate::transition::Reachable;
use crate::transition::candidate::*;
use codec::Entry;
use geo::LineString;

/// A route representing the parsed output from a function
/// passed through the transition graph.
pub struct Route<E>
where
    E: Entry,
{
    /// The solved cost of the collapsed route.
    /// This value is not actionable by the consumer but rather indicative of how confident
    /// the system is in the route chosen.
    pub cost: u32,

    /// The route as a vector of [`CandidateId`]s.
    /// To obtain the list of [`Candidate`]s, use [`Collapse::matched`]
    pub route: Vec<CandidateId>,

    /// The interpolated nodes of the collapsed route.
    /// This exists as a vector of [`Reachable`] nodes which represent each layer transition.
    /// Each node contains the interpolated path between the candidates in those layers.
    ///
    /// To obtain the geographic representation of this interpolation,
    /// use the [`Collapse::interpolated`] method.
    pub interpolated: Vec<Reachable<E>>,

    candidates: Candidates<E>,
}
