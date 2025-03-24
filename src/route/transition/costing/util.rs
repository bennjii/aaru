use crate::route::transition::*;

pub trait Strategy<Ctx> {
    /// A calculable cost which can be any required
    /// type, so long as it is castable into a 64-bit float.
    type Cost: Into<f64>;

    /// The zeta (ζ) value in the decay function.
    const ZETA: f64;

    /// The beta (β) value in the decay function.
    const BETA: f64;

    /// The calculation cost you must implement
    fn calculate(&self, context: Ctx) -> Self::Cost;

    /// An optimal decay-based costing heuristic which accepts
    /// the input value and transforms it using the associated
    /// constants `ZETA` and `BETA` to calculate the resultant output
    /// cost using the `decay` method.
    ///
    /// ### Formula
    /// The scalar is given by `1 / ζ`. Therefore, if `ζ` is `1`, no
    /// scaling is applied. The exponential component is the negative
    /// value divided by `β`. The absolute value of the resultant is taken.
    ///
    /// ```math
    /// decay(value) = |(1 / ζ) * e^(-1 * value / β)|
    /// ```
    #[inline(always)]
    fn cost(&self, ctx: Ctx) -> f64 {
        ((1.0 / Self::ZETA) * (-1.0 * (self.calculate(ctx).into() / Self::BETA)).exp()).abs()
    }
}

pub trait Costing<Emission, Transition>
where
    Transition: TransitionStrategy,
    Emission: EmissionStrategy,
{
    fn emission(&self, context: EmissionContext) -> f64;
    fn transition(&self, context: TransitionContext) -> f64;
}
