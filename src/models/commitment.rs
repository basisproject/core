//! A commitment represents one economic entity's *commitment* to perform some
//! future action (an `Event`).
//!
//! - An [Intent] represents "we want something to happen"
//! - A `Commitment` represents "we agree that something will happen"
//! - An [Event] represents "something did happen"
//!
//! [Intent]: ../intent/struct.Intent.html
//! [Event]: ../event/struct.Event.html

use crate::{
    costs::Costs,
    models::{
        agreement::AgreementID,
        lib::agent::AgentID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    }
};
use url::Url;
use vf_rs::vf;

basis_model! {
    /// The `Commitment` model is a wrapper around the [ValueFlows commitment][vfcomm]
    /// object. It is effectively what an [Event] looks like *before the event
    /// happens*.
    ///
    /// [Event]: ../event/struct.Event.html
    /// [vfcomm]: https://valueflo.ws/introduction/flows.html#commitment
    pub struct Commitment {
        id: <<CommitmentID>>,
        /// The commitments's core VF type
        inner: vf::Commitment<Url, AgreementID, AgentID, (), ProcessID, AgentID, (), ResourceSpecID, ResourceID>,
        /// The amount of costs committed to be moved. One could think of this
        /// somewhat like a negotiated price in the current system.
        move_costs: Costs,
    }
    CommitmentBuilder
}

