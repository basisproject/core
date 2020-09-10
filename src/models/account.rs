//! Accounts are a place to hold credits earned through labor. Think of them
//! like a bank account or crypto wallet.

use crate::{
    error::{Error, Result},
    models:: {
        user::UserID,
    }
};
use getset::{Getters, Setters};
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};

/// Describes a multi-signature relationship to an account, allowing the owners
/// of the account to decide how they may manage funds as a group. For instance,
/// a transaction might need 2-of-3 owners' signatures to be validated.
///
/// This can be used to model things like beneficiaries or set up joint accounts
/// for families.
#[derive(Clone, Debug, PartialEq, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct Multisig {
    /// Requires at least N signatures to complete transactions
    signatures_required: u64,
}

impl Multisig {
    /// Create a new multisig obj
    pub(crate) fn new(signatures_required: u64) -> Self {
        Self {
            signatures_required,
        }
    }
}

basis_model! {
    pub struct Account {
        id: <<AccountID>>,
        /// The user ids of the account owners
        user_ids: Vec<UserID>,
        /// The multisig capabilities of this account
        multisig: Vec<Multisig>,
        /// The account's name for identification purposes
        name: String,
        /// A description of the account
        description: String,
        /// The account's balance
        balance: Decimal,
        /// Whether or not this is a UBI account
        ubi: bool,
    }
    AccountBuilder
}

impl Account {
    /// Adjust the account's balance. Can be positive or negative. The balance
    /// cannot go below zero. Returns the updated balance on success.
    pub(crate) fn adjust_balance<T: Into<Decimal>>(&mut self, amount: T) -> Result<&Decimal> {
        let new_amount = self.balance().clone() + amount.into();
        if new_amount < Decimal::zero() {
            Err(Error::NegativeAccountBalance)?;
        }
        self.set_balance(new_amount);
        Ok(self.balance())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        util::{self, test::*},
    };
    use rust_decimal_macros::*;

    #[test]
    fn account_cannot_go_negative() {
        let now = util::time::now();
        let mut account = make_account(&AccountID::create(), &UserID::create(), dec!(50.0), "my account", &now);
        let amount = account.adjust_balance(dec!(-49)).unwrap();
        assert_eq!(amount, &dec!(1));
        assert_eq!(account.balance(), &dec!(1));
        let amount = account.adjust_balance(dec!(-0.6)).unwrap();
        assert_eq!(amount, &dec!(0.4));
        assert_eq!(account.balance(), &dec!(0.4));
        let amount = account.adjust_balance(dec!(-0.4)).unwrap();
        assert_eq!(amount, &dec!(0));
        assert_eq!(account.balance(), &dec!(0));
        let res = account.adjust_balance(dec!(-0.1));
        assert_eq!(res, Err(Error::NegativeAccountBalance));
    }
}

