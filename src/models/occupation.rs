use vf_rs::vf;

basis_model! {
    pub struct Occupation {
        inner: vf::AgentRelationshipRole,
    }
    OccupationID
    OccupationBuilder
}

