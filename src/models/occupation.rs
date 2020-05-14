use vf_rs::vf::AgentRelationshipRole;

basis_model! {
    pub struct Occupation {
        agent_relationship_role: AgentRelationshipRole,
    }
    OccupationID
    OccupationBuilder
}

