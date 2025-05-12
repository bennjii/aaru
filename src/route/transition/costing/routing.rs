use crate::route::graph::NodeIx;
use crate::route::transition::*;
use crate::route::Graph;

/// A base context provided to costing methods.
///
/// Allows costing methods to access to further information
/// within the current routing progress at the discretion
/// of the call site.
///
/// Provides access to the base map [`map`](#field.map).
/// It also provides a reference to the [`candidates`](#field.candidates) chosen in prior stages.
#[derive(Clone, Copy, Debug)]
pub struct RoutingContext<'a> {
    pub candidates: &'a Candidates,
    pub map: &'a Graph,
}

impl RoutingContext<'_> {
    /// Obtain a [candidate](Candidate), should it exist, by its [identifier](CandidateId).
    pub fn candidate(&self, candidate: &CandidateId) -> Option<Candidate> {
        self.candidates.candidate(candidate)
    }

    /// Obtain the [edge](Edge), should it exist, between two [nodes](NodeIx) (specified as ids)
    pub fn edge(&self, a: &NodeIx, b: &NodeIx) -> Option<Edge> {
        let edge = self.map.graph.edge_weight(*a, *b)?;
        Some(Edge::from((*a, *b, edge)))
    }
}
