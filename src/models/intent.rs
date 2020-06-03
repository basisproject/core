//! An intent represents one economic entity's *desire* for some future event to
//! happen (an `Event`).
//!
//! - An `Intent` represents "we want something to happen"
//! - A [Commitment] represents "we agree that something will happen"
//! - An [Event] represents "something did happen"
//!
//! [Commitment]: ../commitment/struct.Commitment.html
//! [Event]: ../event/struct.Event.html

use crate::{
    models::{
        agreement::AgreementID,
        lib::agent::AgentID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    }
};
use vf_rs::vf;

basis_model! {
    /// The `Intent` model is a wrapper around the [ValueFlows intent][vfintent]
    /// object. It is effectively what an [Event] looks like *before the event
    /// has been commited to*.
    ///
    /// [Event]: ../event/struct.Event.html
    /// [vfintent]: https://valueflo.ws/introduction/flows.html#intent
    pub struct Intent {
        id: <<IntentID>>,
        inner: vf::Intent<AgreementID, AgentID, ProcessID, AgentID, (), ResourceSpecID, ResourceID>,
    }
    IntentBuilder
}

