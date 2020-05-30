use vf_rs::vf;

basis_model! {
    pub struct Occupation {
        id: <<OccupationID>>,
        inner: vf::AgentRelationshipRole,
    }
    OccupationBuilder
}

