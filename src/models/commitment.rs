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
    error::{Error, Result},
    costs::Costs,
    models::{
        agreement::AgreementID,
        event::Event,
        lib::agent::AgentID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    }
};
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
        inner: vf::Commitment<AgreementID, AgreementID, AgentID, (), ProcessID, AgentID, (), ResourceSpecID, ResourceID>,
        /// The amount of costs committed to be moved. One could think of this
        /// somewhat like a negotiated price in the current system.
        move_costs: Option<Costs>,
    }
    CommitmentBuilder
}

impl Commitment {
    /// Given an event, make sure the event either
    ///
    /// - matches the current commitment (meaning the companies, processes, and
    /// resources in the event are the same as they are in the commitment)
    /// - does not require a commitment (ie, an internal transfer or something)
    pub fn validate_event(&self, event: &Event) -> Result<()> {
        match event.inner().action() {
            // inter-organizational actions actions need validation
            vf::Action::DeliverService | vf::Action::Transfer | vf::Action::TransferAllRights | vf::Action::TransferCustody => {}
            // intra-organizational actions do not need validation
            _ => { return Ok(()); }
        }
        if self.inner().provider() != event.inner().provider() ||
            self.inner().receiver() != event.inner().receiver() ||
            self.inner().input_of() != event.inner().input_of() ||
            self.inner().output_of() != event.inner().output_of() ||
            self.inner().resource_inventoried_as() != event.inner().resource_inventoried_as() ||
            self.inner().resource_quantity() != event.inner().resource_quantity() ||
            self.move_costs() != event.move_costs()
        {
            Err(Error::CommitmentInvalid)?;
        }
        Ok(())
    }
}

