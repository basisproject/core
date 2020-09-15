//! An occupation represents a type of job that we desire to track. Occupations
//! are tracked by id in the cost tracking system, allowing only a set amount of
//! job types to be accounted for (as opposed to using freeform entry).
//!
//! Note that occupations require global systemic management.

use vf_rs::vf;

basis_model! {
    /// The occupation model assigns an `OccupationID` to a job title and allows
    /// future-proof cost tracking of that job type.
    pub struct Occupation {
        id: <<OccupationID>>,
        /// The inner VF type which holds our `role_label` field used to hold
        /// the occupation name.
        inner: vf::AgentRelationshipRole,
    }
    OccupationBuilder
}

