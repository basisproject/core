//! Models are the "atom" datatypes for Basis. They represent the objects in the
//! system and their relationships to each other (via IDs). Each model has a
//! struct (ie `User`) and an ID object (ie `UserID`). The id object allows
//! models to link to each other without having to embed the graph into the
//! model data itself.
//!
//! Models are read-only and can only be created or updated using
//! [transactions].
//!
//! In some cases models contain business logic (like [Event]) that define
//! various interactions. For the most part though, models define data structure
//! and relationships.
//!
//! This module also contains some utilities for enumerating changes to models
//! (like [Modifications]) and the classes that support them.
//!
//! Note that because this crate relies heavily on the [ValueFlows ontology][vf]
//! that many of the models have an `inner` field which represents the
//! corresponding ValueFlows type associated with the model. Composition is used
//! as the default pattern here, which offers a fairly clean implementation but
//! with the small sacrifice of having to sometimes to `model.inner().val()`
//! instead of just `model.val()`. The tradeoff is that the VF types are cleanly
//! separated from the Basis models.
//!
//! [transactions]: ../transactions
//! [Event]: event/struct.Event.html
//! [Modifications]: struct.Modifications.html
//! [vf]: https://valueflo.ws/

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[macro_use]
pub(crate) mod lib;

pub use lib::agent::{Agent, AgentID};

// load all of our pub mod <model>; ... lines
load_models! { pub mod }

// create an enum that contains all of our model types
load_models! { pub enum Model }

/// A type for determining if a model should be created, updated, or deleted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Op {
    /// Create a model
    Create,
    /// Update a model
    Update,
    /// Delete a model
    Delete,
}

/// Documents a modification to a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modification {
    /// The type of modification
    op: Op,
    /// The model we're modifying
    model: Model,
}

impl Modification {
    /// Create a new modification
    pub(crate) fn new(op: Op, model: Model) -> Self {
        Self { op, model }
    }

    /// Turn this modification into a pair. Good for implementing saving logic:
    ///
    /// ```rust
    /// use basis_core::{
    ///     models::{
    ///         Model,
    ///         Modification,
    ///         Op,
    ///         user::{User, UserID},
    ///     },
    ///     transactions,
    /// };
    /// use chrono::Utc;
    ///
    /// fn save_mod(modification: Modification) -> Result<(), String> {
    ///     match modification.into_pair() {
    ///         (Op::Create, Model::User(user)) => { /* create a user in your db ... */ }
    ///         (Op::Update, Model::Process(process)) => { /* update a process in your db ... */ }
    ///         (Op::Delete, Model::Resource(resource)) => { /* delete a resource in your db ... */ }
    ///         _ => {}
    ///     }
    ///     Ok(())
    /// }
    ///
    /// let mods = transactions::user::create(UserID::create(), "andrew@lyonbros.com", "andrew", true, &Utc::now()).unwrap();
    /// for modification in mods {
    ///     save_mod(modification).unwrap();
    /// }
    /// ```
    pub fn into_pair(self) -> (Op, Model) {
        (self.op, self.model)
    }

    /// Consume this modification, and verify that the `Op` matches the one
    /// passed in, then return the *unwrapped* model (ie, not `Model::User(user)`
    /// but `user as User`).
    ///
    /// Very handy for testing:
    /// ```rust
    /// use basis_core::{
    ///     models::{
    ///         Op,
    ///         user::{User, UserID},
    ///     },
    ///     transactions,
    /// };
    /// use chrono::Utc;
    ///
    /// let mods = transactions::user::create(UserID::create(), "andrew@lyonbros.com", "andrew", true, &Utc::now()).unwrap().into_vec();
    /// // verifies that the first modification is User Create, and returns the
    /// // User model.
    /// let user = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
    /// assert_eq!(user.name(), "andrew");
    /// ```
    pub fn expect_op<T: TryFrom<Model>>(self, verify_op: Op) -> Result<T> {
        let (op, model) = self.into_pair();
        if op != verify_op {
            Err(Error::OpMismatch)?;
        }
        // NOTE: I do not know why I have to map this error. Seems dumb.
        Ok(T::try_from(model).map_err(|_| Error::WrongModelType)?)
    }
}

/// A set of modifications we want to make to any number of models.
///
/// This is passed back by successfully run transactions. You can use a set of
/// modifications either by converting into a vec (`into_vec()`), or using an
/// iterator.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modifications {
    /// The model modifications we're making
    modifications: Vec<Modification>,
}

impl Modifications {
    /// Create a new modification set
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Create a new modification set with a single mod
    pub(crate) fn new_single<T: Into<Model>>(op: Op, model: T) -> Self {
        let mut mods = Self::new();
        mods.push(op, model);
        mods
    }

    /// Consume the modification set and return the list of modifications
    pub fn into_vec(self) -> Vec<Modification> {
        self.modifications
    }

    /// Push a raw modification object into the mods list.
    pub(crate) fn push_raw(&mut self, modification: Modification) {
        self.modifications.push(modification);
    }

    /// Push a modification into the list with a `Op` and `Model` (bypasses
    /// having to create a `Modification` by hand)
    pub(crate) fn push<T: Into<Model>>(&mut self, op: Op, model: T) {
        self.push_raw(Modification::new(op, model.into()));
    }
}

impl IntoIterator for Modifications {
    type Item = Modification;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.modifications.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            process::Process,
            user::{User, UserID},
        },
        util::{self, test::*},
    };

    #[test]
    fn modifications() {
        let now = util::time::now();
        let user = make_user(&UserID::new("slappy"), None, &now);
        let mut modifications = Modifications::new_single(Op::Create, user.clone());
        modifications.push(Op::Update, user);

        for modi in modifications.clone() {
            match modi.into_pair() {
                (_, Model::User(_)) => {}
                _ => panic!("modification mismatch"),
            }
        }

        let mods = modifications.into_vec();
        let user = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
        assert_eq!(user.id(), &UserID::new("slappy"));
        let user = mods[1].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(user.id(), &UserID::new("slappy"));
        let res = mods[0].clone().expect_op::<Process>(Op::Create);
        assert_eq!(res, Err(Error::WrongModelType));
        let res = mods[1].clone().expect_op::<Process>(Op::Update);
        assert_eq!(res, Err(Error::WrongModelType));
        let res = mods[0].clone().expect_op::<User>(Op::Update);
        assert_eq!(res, Err(Error::OpMismatch));
        let res = mods[1].clone().expect_op::<User>(Op::Create);
        assert_eq!(res, Err(Error::OpMismatch));
        let res = mods[0].clone().expect_op::<Process>(Op::Update);
        assert_eq!(res, Err(Error::OpMismatch));
    }
}
