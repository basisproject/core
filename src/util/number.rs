//! A set of utilities for working with numbers in the Basis costs system.

use crate::{
    costs::Costs,
    error::{Error, Result},
};
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
use std::ops::Mul;

/// Create a number used in the costing system. Internal use only.
///
/// This is mostly a wrapper around a standard number type that makes it easier
/// to swap out test values/Costs types project-wide without having to change
/// each instance by hand, but can also be used by callers of the core to create
/// numbers more seamlessly.
///
/// ```rust
/// use basis_core::{
///     costs::Costs,
///     models::occupation::OccupationID,
///     num
/// };
/// let costs = Costs::new_with_labor(OccupationID::new("plumber"), num!(45.8));
/// ```
///
/// Right now, this wraps `rust_decimal::Decimal`'s `dec!()` macro.
macro_rules! num {
    ($val:expr) => {
        rust_decimal_macros::dec!($val)
    }
}

/// Represents a ratio: a value such that `0 <= v <= 1`.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ratio {
    /// The inner ratio value.
    inner: Decimal,
}

impl Ratio {
    /// Create a new ratio from a Decimal.
    pub fn new<T: Into<Decimal>>(ratio_val: T) -> Result<Self> {
        let ratio: Decimal = ratio_val.into();
        if ratio < Decimal::zero() || ratio > num!(1) {
            Err(Error::InvalidRatio(ratio))?;
        }
        Ok(Self {
            inner: ratio,
        })
    }

    /// Grab this ratio's inner value
    pub fn inner(&self) -> &Decimal {
        &self.inner
    }
}

impl Mul<Costs> for Ratio {
    type Output = Costs;

    fn mul(self, rhs: Costs) -> Costs {
        rhs * self.inner().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ratio() {
        Ratio::new(0).unwrap();
        Ratio::new(1).unwrap();
        Ratio::new(num!(0.999999999999999999999999)).unwrap();
        Ratio::new(num!(0.000000000000000000000001)).unwrap();
        Ratio::new(num!(0.5050)).unwrap();
        assert_eq!(Ratio::new(2), Err(Error::InvalidRatio(num!(2))));
        assert_eq!(Ratio::new(-1), Err(Error::InvalidRatio(num!(-1))));
        let val = num!(1.0000000000000000000000001);
        assert_eq!(Ratio::new(val.clone()), Err(Error::InvalidRatio(val)));
        let val = num!(-0.0000000000000000000000001);
        assert_eq!(Ratio::new(val.clone()), Err(Error::InvalidRatio(val)));
    }

    #[test]
    fn can_multiply_ratio() {
        let ratio = Ratio::new(num!(0.5)).unwrap();
        let mut costs = Costs::new();
        costs.track_labor("machinist", num!(16.8));
        costs.track_resource("steel", num!(5000), num!(0.004));
        let mut costs2 = Costs::new();
        costs2.track_labor("machinist", num!(8.4));
        costs2.track_resource("steel", num!(2500), num!(0.004));
        assert_eq!(ratio * costs, costs2);

        let ratio = Ratio::new(num!(0.833912)).unwrap();
        let mut costs = Costs::new();
        costs.track_labor("machinist", num!(73.99));
        costs.track_resource("steel", num!(8773), num!(0.003));
        let mut costs2 = Costs::new();
        costs2.track_labor("machinist", num!(73.99) * num!(0.833912));
        costs2.track_resource("steel", num!(8773) * num!(0.833912), num!(0.003));
        assert_eq!(costs * ratio, costs2);
    }
}

