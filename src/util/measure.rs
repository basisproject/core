//! Some helpful utilities for dealing with om2::Measure objects in the context
//! of event processing.

use crate::error::{Error, Result};
use om2::{Measure, NumericUnion};
use rust_decimal::prelude::*;

/// Decrement a Measure by some other Measure.
///
/// This will fail if the Measure being decremented falls below zero or if the
/// two Measures have units that don't match.
///
/// Returns true if the first Measure was modified.
pub fn dec_measure(measure: &mut Measure, dec_by: &Measure) -> Result<bool> {
    if measure.has_unit() != dec_by.has_unit() {
        Err(Error::MeasureUnitsMismatched)?;
    }
    let from_quantity = measure.has_numerical_value().clone();
    let dec_quantity = dec_by.has_numerical_value().clone();
    if dec_quantity.is_zero() {
        return Ok(false);
    }
    if dec_quantity.is_negative() {
        Err(Error::NegativeMeasurement)?;
    }
    let remaining = from_quantity
        .clone()
        .sub(dec_quantity.clone())
        .map_err(|e| Error::NumericUnionOpError(e))?;
    if remaining.is_negative() {
        Err(Error::NegativeMeasurement)?;
    }
    measure.set_has_numerical_value(remaining);
    Ok(true)
}

/// Increment a Measure by some other Measure.
///
/// This will fail if the Measure being decremented falls below zero or if the
/// two Measures have units that don't match.
///
/// Returns true if the first Measure was modified.
pub fn inc_measure(measure: &mut Measure, inc_by: &Measure) -> Result<bool> {
    if measure.has_unit() != inc_by.has_unit() {
        Err(Error::MeasureUnitsMismatched)?;
    }
    let from_quantity = measure.has_numerical_value().clone();
    let inc_quantity = inc_by.has_numerical_value().clone();
    if inc_quantity.is_zero() {
        return Ok(false);
    }
    if inc_quantity.is_negative() {
        Err(Error::NegativeMeasurement)?;
    }
    let added = from_quantity
        .clone()
        .add(inc_quantity.clone())
        .map_err(|e| Error::NumericUnionOpError(e))?;
    if added.is_negative() {
        Err(Error::NegativeMeasurement)?;
    }
    measure.set_has_numerical_value(added);
    Ok(true)
}

/// Either use the given `measure` if it exists, or create a measure of 0 and
/// return it using the same units/numeric types as `default`.
pub fn unwrap_or_zero(measure: &Option<Measure>, default: &Measure) -> Measure {
    measure.clone().unwrap_or_else(|| {
        let unit = default.has_unit().clone();
        let numeric = match default.has_numerical_value().clone() {
            NumericUnion::Decimal(_) => NumericUnion::Decimal(Decimal::zero()),
            NumericUnion::Double(_) => NumericUnion::Double(f64::zero()),
            NumericUnion::Float(_) => NumericUnion::Float(f32::zero()),
            NumericUnion::Integer(_) => NumericUnion::Integer(i64::zero()),
        };
        Measure::new(numeric, unit)
    })
}

/// Set a Measure's count to zero (preserves Unit and NumericUnion types).
pub fn set_zero(measure: &mut Measure) {
    let num = match measure.has_numerical_value() {
        NumericUnion::Decimal(_) => NumericUnion::Decimal(Zero::zero()),
        NumericUnion::Double(_) => NumericUnion::Double(Zero::zero()),
        NumericUnion::Float(_) => NumericUnion::Float(Zero::zero()),
        NumericUnion::Integer(_) => NumericUnion::Integer(Zero::zero()),
    };
    measure.set_has_numerical_value(num);
}
