use crate::route::graph::NodeIx;
use crate::route::transition::candidate::*;
use crate::route::Graph;
use geo::LineString;

pub struct Collapse {
    pub cost: u32,
    pub route: Vec<CandidateId>,
    pub interpolated: Vec<NodeIx>,

    candidates: Candidates,
}

impl Collapse {
    pub(crate) fn new(
        cost: u32,
        interpolated: Vec<NodeIx>,
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
            .map(|can| can)
            .collect::<Vec<_>>()
    }

    /// Returns the interpolated route from the collapse as a linestring
    pub fn interpolated(&self, graph: &Graph) -> LineString {
        self.interpolated
            .iter()
            .filter_map(|node| graph.get_position(node))
            .collect::<LineString>()

        // self.route
        //     .windows(2)
        //     .filter_map(|candidate| {
        //         let [a, b] = candidate else {
        //             return None;
        //         };
        //
        //         let edge = self.edge_omni(a, b)?;
        //         let hashmap = graph.hash.read().ok()?;
        //
        //         Some(
        //             edge.nodes
        //                 .iter()
        //                 .filter_map(|index| hashmap.get(index))
        //                 .map(|node| node.position)
        //                 .collect::<Vec<_>>(),
        //         )
        //     })
        //     .flatten()
        //     .collect::<LineString>()
    }
}
