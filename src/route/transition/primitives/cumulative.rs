use crate::route::graph::Weight;
use pathfinding::num_traits::Zero;
use std::ops::Add;

#[derive(Copy, Clone, Hash, Debug)]
pub struct CumulativeFraction {
    pub numerator: Weight,
    pub denominator: u32,
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

impl CumulativeFraction {
    pub(crate) fn value(&self) -> Weight {
        if self.denominator == 0 {
            return 0;
        }

        self.numerator / self.denominator
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
