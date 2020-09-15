//! Defines systemic parameters for the Basis UBI, such as how much is paid
//! over time and the upper ceiling on UBI accounts (to prevent endless
//! accumulation).

use getset::{Getters, Setters};
use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};

/// Holds systemic UBI parameters.
#[derive(Clone, Default, Debug, PartialEq, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct UBIParameters {
    /// The maximum balance a UBI account can hold
    ceiling: Decimal,
    /// How much UBI we get over time
    balance_per_day: Decimal,
}

impl UBIParameters {
    /// Create a new empty params object
    pub fn new() -> Self {
        Default::default()
    }
}

