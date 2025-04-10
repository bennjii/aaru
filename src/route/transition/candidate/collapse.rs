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
    pub fn interpolated(&self, map: &Graph) -> LineString {
        self.interpolated
            .iter()
            .enumerate()
            .flat_map(|(index, reachable)| {
                let source = self.candidates.candidate(&reachable.source).unwrap();
                let target = self.candidates.candidate(&reachable.target).unwrap();

                let path = reachable
                    .path
                    .iter()
                    .filter_map(|node| map.get_position(node));

                std::iter::repeat_n(source.position, if index == 0 { 1 } else { 0 })
                    .chain(path)
                    .chain(std::iter::once(target.position))
            })
            .collect::<LineString>()
    }
}
