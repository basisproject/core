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

use crate::{
    error::{Error, Result},
};
use serde::{Serialize, Deserialize};
use std::convert::TryFrom;

#[macro_use]
mod lib;

// load all of our pub mod <model>; ... lines
load_models!{ pub mod }

// create an enum that contains all of our model types
load_models!{ pub enum Model }

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
    pub fn new(op: Op, model: Model) -> Self {
        Self { op, model }
    }

    /// Turn this modification into a pair
    pub fn into_pair(self) -> (Op, Model) {
        (self.op, self.model)
    }

    /// Consume this modification, and verify that the `Op` matches the one
    /// passed in, then return the *unwraped* model (ie, not `Model::User(user)`
    /// but `user as User`).
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
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modifications {
    /// The model modifications we're making
    modifications: Vec<Modification>,
}

impl Modifications {
    /// Create a new modification set
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new modification set with a single mod
    pub fn new_single<T: Into<Model>>(op: Op, model: T) -> Self {
        let mut mods = Self::new();
        mods.push(op, model);
        mods
    }

    /// Consume the modification set and return the list of modifications
    pub fn into_modifications(self) -> Vec<Modification> {
        self.modifications
    }

    /// Push a modification into the list with a `Op` and `Model` (bypasses
    /// having to create a `Modification` by hand)
    pub fn push<T: Into<Model>>(&mut self, op: Op, model: T) {
        self.modifications.push(Modification::new(op, model.into()));
    }
}

