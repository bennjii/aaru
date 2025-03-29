use crate::route::transition::candidate::collapse::Collapse;
use crate::route::transition::candidate::{Candidate, CandidateEdge, CandidateId, CandidateRef};
use crate::route::transition::layer::Layers;
use petgraph::prelude::EdgeRef;
use petgraph::{Directed, Graph};
use scc::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

type LockedGraph<A, B> = Arc<RwLock<Graph<A, B, Directed>>>;

pub struct Candidates {
    /// The locked graph structure storing the candidates
    /// in their layers, connected piecewise.
    ///
    /// The associated node information in the graph can be
    /// used to look up the candidate from the flyweight.
    pub(crate) graph: LockedGraph<CandidateRef, CandidateEdge>,

    /// Candidate flyweight
    pub(crate) lookup: HashMap<CandidateId, Candidate>,

    ends: Option<(CandidateId, CandidateId)>,
}

impl Debug for Candidates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let entries = self.lookup.len();
        write!(
            f,
            "Candidates {{ graph: <locked>, lookup: \"{entries} Entries\" }}"
        )
    }
}

impl Candidates {
    pub fn attach_ends(&mut self, layers: &Layers) -> Option<(CandidateId, CandidateId)> {
        if self.ends.is_some() {
            return None;
        }

        let mut graph = self.graph.write().unwrap();
        let source = graph.add_node(CandidateRef::butt());
        let target = graph.add_node(CandidateRef::butt());

        // We need to bind the first and last layers to a singular
        // source and target value so we can route toward this given
        // target, from our own source.
        //
        //                   Layer     Layer
        //                     0         N
        //
        //               __/---+   ...   +---\__
        //              /                       \
        //   SOURCE    +-------+   ...   +-------+  TARGET
        //              \                       /
        //               ‾‾\---+   ...   +---/‾‾
        //
        // So, we need to attach each entry within the first/initial layer
        // to this source, and every entry within the last/final layer to
        // the target.

        // Attach the initial layer
        layers.first()?.nodes.iter().for_each(|node| {
            graph.add_edge(source, *node, CandidateEdge::zero());
        });

        // Attach to the final layer
        layers.last()?.nodes.iter().for_each(|node| {
            graph.add_edge(*node, target, CandidateEdge::zero());
        });

        drop(graph);
        let ends = Some((source, target));
        self.ends = ends;

        ends
    }

    /// Collapses transition layers, `layers`, into a single vector of
    /// the finalised points. This is useful for solvers which will
    /// mutate the candidates, and require an external method to solve
    /// based on the calculated edge weights. Iterative solvers which
    /// do not produce a candidate solution do not require this function.
    ///
    /// Takes an owned value to indicate the structure is [terminal].
    ///
    /// [terminal]: Cannot be used again
    pub fn collapse(self) -> Option<Collapse> {
        let (source, target) = self.ends?;

        // There should be exclusive read-access by the time collapse is called.
        // This will block access to any other client using this candidate structure,
        // however this function
        let graph = self.graph.read().unwrap();

        let Some((cost, route)) = petgraph::algo::astar(
            &*graph,
            source,
            |node| node == target,
            |e| {
                // Decaying Transition Cost
                let transition_cost = e.weight().weight;

                // Loosely-Decaying Emission Cost
                let emission_cost = graph
                    .node_weight(e.target())
                    .map_or(f64::INFINITY, |v| v.weight());

                transition_cost + emission_cost
            },
            |_| 0.0,
        ) else {
            return None;
        };

        drop(graph);
        Some(Collapse::new(cost, route, self))
    }

    /// TODO: Provide docs
    pub fn edge(&self, a: &CandidateId, b: &CandidateId) -> Option<CandidateEdge> {
        let reader = self.graph.read().ok()?;

        let edge_index = reader.find_edge(*a, *b)?;

        // TODO: Can we make this operation cheaper? We're cloning vectors internally.
        reader.edge_weight(edge_index).cloned()
    }

    /// TODO: Provide docs
    pub fn candidate(&self, a: &CandidateId) -> Option<Candidate> {
        self.lookup.get(a).map(|c| *c)
    }
}

impl Default for Candidates {
    fn default() -> Self {
        let graph = Arc::new(RwLock::new(Graph::new()));
        let lookup = HashMap::new();

        Candidates {
            graph,
            lookup,
            ends: None,
        }
    }
}
