use crate::route::graph::Weight;
use pathfinding::num_traits::Zero;
use std::ops::Add;

/// A fractional value, broken into a stored numerator and denominator.
/// It supports the [`Add`] trait wherein the combination of numerator
/// and denominator is supported.
///
/// The fractional value can be derived by the [`CumulativeFraction::value`] function.
#[derive(Copy, Clone, Hash, Debug)]
pub struct CumulativeFraction {
    pub numerator: Weight,
    pub denominator: u32,
}

impl CumulativeFraction {
    /// Returns the fractional value (num / denom) of the cumulative fraction.
    pub(crate) fn value(&self) -> Weight {
        if self.denominator == 0 {
            return 0;
        }

        self.numerator / self.denominator
    }
}

impl Zero for CumulativeFraction {
    fn zero() -> Self {
        CumulativeFraction {
            numerator: 0,
            denominator: 0,
        }
    }

    fn is_zero(&self) -> bool {
        self.value() == 0
    }
}

impl Add<CumulativeFraction> for CumulativeFraction {
    type Output = CumulativeFraction;

    fn add(self, rhs: CumulativeFraction) -> Self::Output {
        CumulativeFraction {
            numerator: self.numerator + rhs.numerator,
            denominator: self.denominator + rhs.denominator,
        }
    }
}
