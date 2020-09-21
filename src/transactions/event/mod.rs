//! Events are what move costs/resources through the system.
//!
//! See the [event model.][1]
//!
//! [1]: ../../models/event/index.html

use crate::{
    models::{
        resource::{ResourceID, Resource},
    },
};
#[cfg(feature = "with_serde")]
use serde::{Serialize, Deserialize};

/// Helps us signify whether we want an operation that moves a resource from one
/// place to another to a) create a new resource copied from the original or b)
/// update a pre-existing resource.
///
/// This is used mainly for the move, transfer, transfer-all-rights, and
/// transfer-custody events.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(Serialize, Deserialize))]
pub enum ResourceMover {
    /// Create a new resource using the given ID
    Create(ResourceID),
    /// Update the given resource
    Update(Resource),
}

pub mod accounting;
pub mod delivery;
pub mod production;
pub mod modification;
pub mod service;
pub mod transfer;
pub mod work;

