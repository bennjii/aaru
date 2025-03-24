pub mod emission {
    use crate::route::transition::*;
    use geo::{Distance, Haversine};

    /// Calculates the emission cost of a candidate relative
    /// to its source node.
    ///
    /// ## Calculation
    ///
    /// ```math
    ///
    /// ```
    pub struct DefaultEmissionCost;

    impl<'a> Strategy<EmissionContext<'a>> for DefaultEmissionCost {
        type Cost = f64;

        const ZETA: f64 = 1.0;
        const BETA: f64 = -100.0;

        fn calculate(&self, context: EmissionContext<'a>) -> Self::Cost {
            Haversine::distance(*context.source_position, *context.candidate_position).powi(2)
        }
    }
}

pub mod transition {
    use crate::route::transition::*;
    use geo::{Distance, Haversine};

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
        const BETA: f64 = -50.0;

        fn calculate(&self, context: TransitionContext<'a>) -> Self::Cost {
            // Value in range [0, 180].
            let turn_cost = { context.optimal_path.immediate_angle().abs() };

            // Value in range [0, inf)
            let deviance = {
                let shortest_distance = Haversine::distance(
                    context.source_candidate().position,
                    context.target_candidate().position,
                );

                let path_length = context.optimal_path.length();
                let deviance = path_length / shortest_distance;

                deviance.clamp(0.0, 1.0)
            };

            turn_cost + deviance
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
            CostingStrategies::new(DefaultEmissionCost, DefaultTransitionCost)
        }
    }

    impl<E, T> Costing<E, T> for CostingStrategies<E, T>
    where
        T: TransitionStrategy,
        E: EmissionStrategy,
    {
        fn emission(&self, context: EmissionContext) -> f64 {
            self.emission.cost(context)
        }

        fn transition(&self, context: TransitionContext) -> f64 {
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
