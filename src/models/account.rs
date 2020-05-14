use crate::{
    models:: {
        user::UserID,
    }
};

basis_model! {
    pub struct Account {
        user_id: UserID,
        name: String,
        description: String,
        balance: f64,
    }
    AccountID
    AccountBuilder
}

