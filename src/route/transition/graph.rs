use crate::route::Graph;
use crate::route::transition::*;

use geo::LineString;

type LayerId = usize;
type NodeId = usize;

/// A map-specific transition graph based on the Hidden-Markov-Model structure.
///
/// This is the orchestration point for solving transition graphs for making
/// map-matching requests. It requires a [map](Graph) on instantiation, as well as
/// a [route](LineString) to solve for.
///
/// ### Example
///
/// Below is an example that can interpolate a trip using map-matching. To
/// see all the available ways to interpret the resultant solution, see
/// the [`Collapse`] structure.
///
/// ```rust
/// use geo::LineString;
/// use routers::route::{Graph, Transition};
/// use routers::route::transition::{CostingStrategies, SelectiveForwardSolver};
///
/// // An example function to find the interpolated path of a trip.
/// fn match_trip(map: &Graph, route: LineString) -> Option<LineString> {
///     // Use the default costing strategies
///     let costing = CostingStrategies::default();
///
///     // Create our transition graph, supplying our map for context,
///     // and the route we wish to load as the layer data.
///     let transition = Transition::new(&map, route, costing);
///
///     // For example, let's choose the selective-forward solver.
///     let solver = SelectiveForwardSolver::default();
///
///     // Now.. we simply solve the transition graph using the solver
///     let solution = transition.solve(solver)?;
///
///     // Now we can return the interpolated path, just like that!
///     Some(solution.interpolated(map))
/// }
/// ```
pub struct Transition<'a, E, T>
where
    E: EmissionStrategy,
    T: TransitionStrategy,
{
    pub(crate) map: &'a Graph,
    pub(crate) heuristics: CostingStrategies<E, T>,

    pub(crate) candidates: Candidates,
    pub(crate) layers: Layers,
}

impl<'a, E, T> Transition<'a, E, T>
where
    E: EmissionStrategy + Send + Sync,
    T: TransitionStrategy + Send + Sync,
{
    /// Creates a new transition graph from the input linestring and heuristics.
    ///
    /// ### Warning
    ///
    /// This function is expensive. Unlike many other `::new(..)` functions, this
    /// function calls out to the [`LayerGenerator`]. This may take significant time
    /// in some circumstances, particularly in longer (>1000 pt) input paths.
    ///
    /// Therefore, this function may be more expensive than intended for some cases,
    /// plan accordingly.
    pub fn new(
        map: &'a Graph,
        linestring: LineString,
        heuristics: CostingStrategies<E, T>,
    ) -> Transition<'a, E, T> {
        let points = linestring.into_points();
        let generator = LayerGenerator::new(map, &heuristics);

        // Generate the layers and candidates.
        let (layers, candidates) = generator.with_points(&points);

        Transition {
            map,
            candidates,
            layers,
            heuristics,
        }
    }

    /// Converts the transition graph into a [`RoutingContext`].
    pub fn context(&self) -> RoutingContext {
        RoutingContext {
            candidates: &self.candidates,
            map: self.map,
        }
    }

    /// Solves the transition graph, using the provided [`Solver`].
    pub fn solve(self, solver: impl Solver) -> Result<Collapse, MatchError> {
        // Indirection to call.
        solver.solve(self)
    }

    /// Collapses the Hidden Markov Model (See [HMM]) into a
    /// [`Collapse`] result (solve).
    ///
    /// Consumes the transition structure in doing so.
    /// This is because it makes irreversible modifications
    /// to the candidate graph that put it in a collapsable
    /// position, and therefore breaks atomicity, and should
    /// not be re-used.
    ///
    /// [HMM]: https://en.wikipedia.org/wiki/Hidden_Markov_model
    pub(crate) fn collapse(self) -> Result<Collapse, MatchError> {
        // Use the candidates to collapse the graph into a single route.
        self.candidates
            .collapse()
            .ok_or(MatchError::CollapseFailure)
    }
}
