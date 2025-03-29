use crate::route::transition::candidate::{Candidate, CandidateId, CandidateRef, Candidates};
use crate::route::transition::layer::Layer;
use crate::route::transition::{
    Costing, CostingStrategies, EmissionContext, EmissionStrategy, TransitionStrategy,
};
use crate::route::Graph;
use geo::Point;
use log::info;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::prelude::{FromParallelIterator, IntoParallelIterator};
use wkt::ToWkt;

pub struct Layers {
    pub layers: Vec<Layer>,
}

impl Layers {
    pub fn last(&self) -> Option<&Layer> {
        self.layers.last()
    }

    pub fn first(&self) -> Option<&Layer> {
        self.layers.first()
    }
}

impl Default for Layers {
    fn default() -> Self {
        Layers { layers: vec![] }
    }
}

impl FromParallelIterator<Layer> for Layers {
    fn from_par_iter<I>(layers: I) -> Self
    where
        I: IntoParallelIterator<Item = Layer>,
    {
        let layers = layers.into_par_iter().collect::<Vec<Layer>>();
        Self { layers }
    }
}

/// Generates the layers within the transition graph.
///
/// Generates the layers of the transition graph, where each layer
/// represents a point in the linestring, and each node in the layer
/// represents a candidate transition point, within the `distance`
/// search radius of the linestring point, which was found by the
/// projection of the linestring point upon the closest edges within this radius.
pub struct LayerGenerator<'a, E, T>
where
    E: EmissionStrategy,
    T: TransitionStrategy,
{
    distance: f64,
    heuristics: &'a CostingStrategies<E, T>,

    map: &'a Graph,
}

impl<'a, E, T> LayerGenerator<'a, E, T>
where
    E: EmissionStrategy + Send + Sync,
    T: TransitionStrategy + Send + Sync,
{
    pub fn new(map: &'a Graph, heuristics: &'a CostingStrategies<E, T>, distance: f64) -> Self {
        LayerGenerator {
            map,
            distance,
            heuristics,
        }
    }

    /// TODO: Docs
    ///
    /// Takes a projection distance (`distance`), for which
    /// to search for projected nodes within said radius from
    /// the position on the input point.
    pub fn with_points(&self, input: Vec<Point>) -> (Layers, Candidates) {
        let candidates = Candidates::default();

        // In parallel, create each layer, and collect into a single structure.
        let layers = input
            .into_par_iter()
            .enumerate()
            .map(|(layer_id, origin)| {
                // Generate an individual layer
                info!(
                    "Generating layer {} (Point={})",
                    layer_id,
                    origin.wkt_string()
                );

                let nodes = self
                    .map
                    // We'll do a best-effort search (square) radius
                    .nearest_projected_nodes(&origin, self.distance)
                    .enumerate()
                    .map(|(node_id, (position, map_edge))| {
                        // We have the actual projected position, and it's associated edge.
                        // Therefore, we can use the Emission costing function to calculate
                        // the associated emission cost of this candidate.
                        let emission = self
                            .heuristics
                            .emission(EmissionContext::new(&position, &origin));

                        let candidate = Candidate {
                            map_edge: (map_edge.0, map_edge.1),
                            position,
                            layer_id,
                            node_id,
                            emission,
                        };

                        let candidate_reference = CandidateRef::new(emission);
                        (candidate, candidate_reference)
                    })
                    .collect::<Vec<(Candidate, CandidateRef)>>();

                // Inner-Scope for the graph, dropped on close.
                let nodes = {
                    let mut graph = candidates.graph.write().unwrap();
                    nodes
                        .into_iter()
                        .map(|(candidate, candidate_ref)| {
                            let node_index = graph.add_node(candidate_ref);
                            let _ = candidates.lookup.insert(node_index, candidate);

                            node_index as CandidateId
                        })
                        .collect::<Vec<CandidateId>>()
                };

                Layer { nodes, origin }
            })
            .collect::<Layers>();

        (layers, candidates)
    }
}
