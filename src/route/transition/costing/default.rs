pub mod emission {
    use crate::route::transition::*;
    use geo::{Distance, Haversine};

    /// 10 meters (85th% GPS error)
    const DEFAULT_EMISSION_ERROR: f64 = 10.0;

    /// Calculates the emission cost of a candidate relative
    /// to its source node.
    ///
    /// ## Calculation
    ///
    /// The emission cost is defined by the  relative distance
    /// from the source position, given some "free" zone, known
    /// as the [`emission_error`](#field.emission_error). Within this error, any
    /// candidate is considered a no-cost transition as the likelihood
    /// the position is within the boundary is equal within this radius.
    ///
    /// The relative calculation is given simply below, where `distance`
    /// defines the haversine distancing function between two Lat/Lng positions.
    ///
    /// ```math
    /// relative(source, candidate) = err / distance(source, candidate)
    /// ```
    ///
    /// The cost derived is given as the square root of the reciprocal of
    /// the relative distance.
    ///
    /// ```math
    /// cost(source, candidate) = sqrt(1 / relative(source, candidate))
    /// ```
    ///
    /// There exist values within the strategy
    /// implementation which define how "aggressive" the falloff is.
    /// These hyperparameters may need to be tuned in order to calculate for nodes
    /// which have large error. Alternatively, providing your own emission error
    /// is possible too.
    pub struct DefaultEmissionCost {
        /// The free radius around which emissions cost the same, to provide
        /// equal opportunity to nodes within the expected GPS error.
        ///
        /// Default: [`DEFAULT_EMISSION_ERROR`]
        pub emission_error: f64,
    }

    impl Default for DefaultEmissionCost {
        fn default() -> Self {
            DefaultEmissionCost {
                emission_error: DEFAULT_EMISSION_ERROR,
            }
        }
    }

    impl<'a> Strategy<EmissionContext<'a>> for DefaultEmissionCost {
        type Cost = f64;

        const ZETA: f64 = 1.0;
        const BETA: f64 = -10.0;

        fn calculate(&self, context: EmissionContext<'a>) -> Option<Self::Cost> {
            let distance =
                Haversine::distance(*context.source_position, *context.candidate_position);

            // Value in range [0, 1] (1=Low Cost, 0=High Cost)
            let relative_to_error = (DEFAULT_EMISSION_ERROR / distance).clamp(0.0, 1.0);

            Some(relative_to_error.recip().sqrt())
        }
    }
}

pub mod transition {
    use crate::route::transition::*;

    /// Calculates the transition cost between two candidates.
    ///
    /// Involves the following "sub-heuristics" used to quantify
    /// the trip "complexity" and travel "likelihood".
    ///
    /// # Calculation
    ///
    /// Using turn-costing, we calculate immediate and summative
    /// angular rotation, and with deviance we determine a travel likelihood.
    ///
    /// ## Turn Cost
    /// We describe the summative angle, seen in the [`Trip::total_angle`]
    /// function, as the total angular rotation exhibited by a trip.
    /// We assume a high degree of rotation is not preferable, and trips
    /// are assumed to take the most optimal path with the most reasonable
    /// changes in trajectory, meaning many turns where few are possible
    /// is discouraged.
    ///
    /// We may then [amortize] this cost to calculate the immediately
    /// exhibited angle. Or, alternatively expressed as the average angle
    /// experienced
    ///
    /// ```math
    /// sum_angle(trip) = ∑(angles(trip))
    /// imm_angle(trip) = sum_angle(trip) / len(trip)
    ///
    /// turn_cost(trip) = imm_angle(trip)
    /// ```
    ///
    /// ## Deviance
    /// Defines the variability between the trip length (in meters)
    /// and the shortest great-circle distance between the two candidates.
    ///
    /// This cost is low in segments which follow an optimal path, i.e. in
    /// a highway, as it discourages alternate paths which may appear quicker
    /// to traverse.
    ///
    /// ```math
    /// length(trip) = ∑(distance(segment))
    /// deviance(trip, source, target) = length(trip) - distance(source, target)
    /// ```
    ///
    /// ### Total Cost
    /// The total cost is combined as such.
    ///
    /// ```math
    /// cost(trip, s, t) = deviance(trip, s, t) + turn_cost(trip)
    /// ```
    ///
    /// [amortize]: https://en.wikipedia.org/wiki/Amortized_analysis
    pub struct DefaultTransitionCost;

    impl<'a> Strategy<TransitionContext<'a>> for DefaultTransitionCost {
        type Cost = f64;

        const ZETA: f64 = 1.0;
        const BETA: f64 = -1.0;

        fn calculate(&self, context: TransitionContext<'a>) -> Option<Self::Cost> {
            // Find the transition lengths (shortest path, trip length)
            let lengths = context.lengths()?;

            // Value in range [0, 1] (1=Low Cost, 0=High Cost)
            let deviance = lengths.deviance();

            let avg_weight = context
                .map_path
                .windows(2)
                .filter_map(|node| match node {
                    [a, b] => context.routing_context.edge(a, b),
                    _ => None,
                })
                .map(|(weight, _)| *weight as f64)
                .sum::<f64>()
                / context.map_path.iter().len() as f64;

            // Value in range [0, 1] (1=Low Cost, 0=High Cost)
            // Defines the no. edges traversed (fewer distinct edge id's, the better)
            let distinct_cost = (1.0 / avg_weight).sqrt().clamp(0.0, 1.0);

            // Value in range [0, 1] (1=Low Cost, 0=High Cost)
            let turn_cost = context
                .optimal_path
                .angular_complexity(context.layer_width)
                .clamp(0.0, 1.0);

            // Value in range [0, 1] (1=Low Cost, 0=High Cost)
            //  Weighted: 30% Edge Distinction, 30% Turn Difficulty, 30% Distance Deviance
            //      Note: Weights must sum to 100%
            let avg_cost = (0.3 * distinct_cost) + (0.3 * turn_cost) + (0.3 * deviance);

            // Take the inverse to "span" values
            Some(avg_cost.recip())
        }
    }
}

pub mod costing {
    use super::{DefaultEmissionCost, DefaultTransitionCost};
    use crate::route::transition::*;

    pub struct CostingStrategies<E, T>
    where
        E: EmissionStrategy,
        T: TransitionStrategy,
    {
        emission: E,
        transition: T,
    }

    impl<E, T> CostingStrategies<E, T>
    where
        E: EmissionStrategy,
        T: TransitionStrategy,
    {
        pub fn new(emission: E, transition: T) -> Self {
            Self {
                emission,
                transition,
            }
        }
    }

    impl Default for CostingStrategies<DefaultEmissionCost, DefaultTransitionCost> {
        fn default() -> Self {
            CostingStrategies::new(DefaultEmissionCost::default(), DefaultTransitionCost)
        }
    }

    impl<E, T> Costing<E, T> for CostingStrategies<E, T>
    where
        T: TransitionStrategy,
        E: EmissionStrategy,
    {
        fn emission(&self, context: EmissionContext) -> u32 {
            self.emission.cost(context)
        }

        fn transition(&self, context: TransitionContext) -> u32 {
            self.transition.cost(context)
        }
    }
}

#[doc(hidden)]
pub use costing::*;
#[doc(hidden)]
pub use emission::*;
#[doc(hidden)]
pub use transition::*;
