//! The credit converter module provides an interface for converting standard
//! resource units (like 5kg iron) and currencies (like $4.95) into credit
//! values.
//!
//! The purpose of this is to be able to reduce a set of [Costs][0] down to a
//! single value which can be compared with other values (like a price).
//!
//! [0]: ../../costs/index.html

use crate::{
    costs::Costs,
    error::Result,
    models::{
        currency::CurrencyID,
        resource_spec::ResourceSpecID,
    },
};
use rust_decimal::prelude::*;

/// A struct that aids in reducing a set of costs into a singular `Decimal`
/// value which can be easily compared with another value.
pub struct CreditConverter {
}

impl CreditConverter {
    /// Create a new credit converter
    pub fn new() -> Self {
        // TODO: need information on resource and currency conversion, obvis
        Self {}
    }

    /// Convert a currency id/value pair into a credit decimal value.
    fn currency(&self, id: &CurrencyID, value: &Decimal) -> Result<Decimal> {
        drop(id);
        drop(value);
        Ok(Decimal::zero())
    }

    /// Convert a resource id/ pair into a credit decimal value.
    fn resource(&self, id: &ResourceSpecID, value: &Decimal) -> Result<Decimal> {
        drop(id);
        drop(value);
        Ok(Decimal::zero())
    }

    /// Convert a `Costs` object into a `Decimal`
    pub fn reduce(&self, costs: &Costs) -> Result<Decimal> {
        let mut credit_value = Decimal::zero();
        for (currency_id, val) in costs.currency() {
            // convert currency into a credit value
            credit_value += self.currency(&currency_id, &val)?;
        }
        for (resource_id, val) in costs.resource() {
            // convert standard resource units into a credit value
            credit_value += self.resource(&resource_id, &val)?;
        }
        for (_, val) in costs.labor() {
            // no conversion necessary, labor costs ARE credit costs
            credit_value += val;
        }
        Ok(credit_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::*;

    #[test]
    fn reduces() {
        let converter = CreditConverter::new();

        let costs1 = Costs::new_with_labor("mechanic", dec!(24.333));
        let reduced1 = converter.reduce(&costs1).unwrap();
        assert_eq!(reduced1, dec!(24.333));

        let mut costs2 = Costs::new();
        costs2.track_labor("machinist", dec!(18.444));
        costs2.track_labor("machinist", dec!(12.3));
        costs2.track_labor("ceo", dec!(42.91));
        let reduced2 = converter.reduce(&costs2).unwrap();
        assert_eq!(reduced2, dec!(18.444) + dec!(12.3) + dec!(42.91));

        let mut costs3 = Costs::new();
        costs3.track_labor("ice cream vendor", dec!(80));
        costs3.track_currency("usd", dec!(140.55));
        costs3.track_currency("cad", dec!(13.42));
        costs3.track_resource("gasoline", dec!(0.4));
        costs3.track_resource("iron", dec!(1.82));
        let reduced3 = converter.reduce(&costs3).unwrap();
        // NOTE: this will break when we implement currency/resource tracking
        assert_eq!(reduced3, dec!(80));
    }
}

