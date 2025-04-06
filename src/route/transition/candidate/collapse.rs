use crate::route::transition::candidate::*;
use crate::route::transition::Reachable;
use crate::route::Graph;
use geo::LineString;

pub struct Collapse {
    pub cost: u32,
    pub route: Vec<CandidateId>,
    pub interpolated: Vec<Reachable>,

    candidates: Candidates,
}

impl Collapse {
    pub(crate) fn new(
        cost: u32,
        interpolated: Vec<Reachable>,
        route: Vec<CandidateId>,
        candidates: Candidates,
    ) -> Self {
        Self {
            cost,
            interpolated,
            route,
            candidates,
        }
    }

    fn edge_omni(&self, a: &CandidateId, b: &CandidateId) -> Option<CandidateEdge> {
        self.candidates
            .edge(a, b)
            .or_else(|| self.candidates.edge(b, a))
    }

    /// TODO: Docs
    pub fn matched(&self) -> Vec<Candidate> {
        self.route
            .iter()
            .filter_map(|node| self.candidates.lookup.get(node))
            .map(|can| *can)
            .collect::<Vec<_>>()
    }

    /// Returns the interpolated route from the collapse as a linestring
    pub fn interpolated(&self, graph: &Graph) -> LineString {
        self.interpolated
            .iter()
            .flat_map(|reachable| reachable.path.clone())
            .filter_map(|node| graph.get_position(&node))
            .collect::<LineString>()
    }
}
