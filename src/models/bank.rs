//! A bank is a region-owned company that facilitates transactions with the
//! outside market and also allows planned investment in internal companies
//! based on democratically-selected criteria.

use rust_decimal::prelude::*;

basis_model! {
    /// The bank model. This is a stub.
    pub struct Bank {
        id: <<BankID>>,
        /// How many active credits are in circulation in this region. This is
        /// basically a measure of `printed - spent`.
        active_credits: Decimal,
        /// How much currency is in the capital pool backing the credits. If
        /// this number is 0, regional credits are worth 0 of the local currency
        /// but if this number is the same as `active_credits` then each credit
        /// is worth one currency denomination (for example, $1 USD).
        currency_backing_credits: Decimal,
    }
    BankBuilder
}


