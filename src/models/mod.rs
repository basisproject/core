use getset::Getters;
use serde::{Serialize, Deserialize};

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
#[derive(Debug, Clone, PartialEq, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
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

