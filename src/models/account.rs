//! Accounts are a place to hold credits earned through labor. Think of them
//! like a bank account or crypto wallet.

use crate::{
    models:: {
        user::UserID,
    }
};

basis_model! {
    pub struct Account {
        id: <<AccountID>>,
        user_id: UserID,
        name: String,
        description: String,
        balance: f64,
    }
    AccountBuilder
}

