use crate::route::transition::candidate::{Candidate, CandidateId, CandidateRef, Candidates};
use crate::route::transition::layer::Layer;
use crate::route::transition::{
    Costing, CostingStrategies, EmissionContext, EmissionStrategy, TransitionStrategy,
};
use crate::route::{Graph, Scan};

#[cfg(debug_assertions)]
use crate::route::transition::CandidateLocation;

use geo::{Distance, Haversine, MultiPoint, Point};
use log::info;
use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use rayon::prelude::{FromParallelIterator, IntoParallelIterator};
use wkt::ToWkt;

#[derive(Default)]
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

impl FromParallelIterator<Layer> for Layers {
    fn from_par_iter<I>(layers: I) -> Self
    where
        I: IntoParallelIterator<Item = Layer>,
    {
        let layers = layers.into_par_iter().collect::<Vec<Layer>>();
        Self { layers }
    }
}

const DEFAULT_SEARCH_DISTANCE: f64 = 3000.0;
const DEFAULT_FILTER_DISTANCE: f64 = 50.0;

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
    /// The maximum distance by which the generator will search for nodes,
    /// allowing it to find edges which may be comprised of distant nodes.
    search_distance: f64,

    /// The maximum distance by which matched candidates will be found,
    /// this directly minimises the cost to compute since it impacts the
    /// quantity of candidates found.
    ///
    /// A high search distance may take longer to compute but will give
    /// more accurate candidates as it can find edges who's comprising nodes
    /// are far apart.
    filter_distance: f64,

    heuristics: &'a CostingStrategies<E, T>,

    map: &'a Graph,
}

impl<'a, E, T> LayerGenerator<'a, E, T>
where
    E: EmissionStrategy + Send + Sync,
    T: TransitionStrategy + Send + Sync,
{
    pub fn new(map: &'a Graph, heuristics: &'a CostingStrategies<E, T>) -> Self {
        LayerGenerator {
            map,
            heuristics,

            search_distance: DEFAULT_SEARCH_DISTANCE,
            filter_distance: DEFAULT_FILTER_DISTANCE,
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

                let mut projected = self
                    .map
                    // We'll do a best-effort search (square) radius
                    .nearest_projected_nodes(&origin, self.search_distance)
                    .collect::<Vec<_>>();

                // TODO: Formalize take over filter
                projected.sort_by(|(a, _), (b, _)| {
                    Haversine::distance(*a, origin).total_cmp(&Haversine::distance(*b, origin))
                });

                let nodes = projected
                    .into_iter()
                    .take(10)
                    .take_while(|(p, _)| Haversine::distance(*p, origin) < self.filter_distance)
                    .enumerate()
                    .map(|(node_id, (position, edge))| {
                        // We have the actual projected position, and it's associated edge.
                        // Therefore, we can use the Emission costing function to calculate
                        // the associated emission cost of this candidate.
                        let emission = self
                            .heuristics
                            // TODO: This will calculate the distance between TWICE since we do it above.
                            //    => Investigate if we can save this value and supply it to the ctx.
                            .emission(EmissionContext::new(&position, &origin));

                        #[cfg(debug_assertions)]
                        let location = CandidateLocation { layer_id, node_id };
                        let candidate = Candidate::new(
                            edge,
                            position,
                            emission,
                            #[cfg(debug_assertions)]
                            location,
                        );

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

        // let mut points = vec![];
        // candidates.lookup.scan(|_, candidate| {
        //     points.push(candidate.position);
        // });
        //
        // let mp = points.into_iter().collect::<MultiPoint>();
        // info!("All Candidates ({}): {}", mp.len(), mp.wkt_string());

        (layers, candidates)
    }
}
