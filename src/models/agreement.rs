//! Agreements respresent a larger transaction between two agents. Think of an
//! agreement like an order, and that order can be made up of multiple
//! deliverables, modeled as `Commitment`s and `EconomicEvent`s.

use crate::{
    models::{
        lib::agent::AgentID,
    },
};
use vf_rs::vf;

basis_model! {
    /// An agreement between two or more parties. This model is a very thin
    /// wrapper around the [ValueFlows Agreement][vfagreement] object. It has no
    /// concept of trying to parse or contain agreement text or clauses, but
    /// rather acts as a simple pointer to *some agreement somewhere* that the
    /// affected parties have shared access to.
    ///
    /// [vfagreement]: https://valueflo.ws/introduction/exchanges.html#agreements
    pub struct Agreement {
        id: <<AgreementID>>,
        /// The inner vf Agreement object
        inner: vf::Agreement,
        /// A list of the participants in the agreement. This allows quickly
        /// checking to see if an event or commitment is part of an agreement.
        /// Note that this might also allow the storage layer to have a list of
        /// signatures needed in order to materially change the agreement.
        participants: Vec<AgentID>,
        /// Whether this agreement has been approved by all parties
        finalized: bool,
    }
    AgreementBuilder
}

impl Agreement {
    /// Determines if the given agent is a participant in this agreement.
    pub fn has_participant(&self, agent_id: &AgentID) -> bool {
        self.participants().contains(agent_id)
    }

    /// Determines if our agreement has been finalized
    pub fn is_finalized(&self) -> bool {
        // for now, just read the bit. later on, we might have a more in-depth
        // check.
        *self.finalized()
    }
}

