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

