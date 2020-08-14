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
    costs::Costs,
    models::{
        lib::agent::AgentID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    }
};
use url::Url;
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
        /// Our inner VF intent type
        inner: vf::Intent<Url, AgentID, ProcessID, AgentID, (), ResourceSpecID, ResourceID>,
        /// If this event is an input/output of a process or resource, move some
        /// fixed amount of costs between the two objects.
        move_costs: Option<Costs>,
    }
    IntentBuilder
}

