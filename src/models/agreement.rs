//! Agreements represent a legal commitment between two parties and may or may
//! not be involved in a set of commitments and/or events.

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
        inner: vf::Agreement,
    }
    AgreementBuilder
}

