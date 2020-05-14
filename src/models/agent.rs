use crate::{
    models::{
        company::CompanyID,
        user::UserID,
    },
};
use serde::{Serialize, Deserialize};

/// A union defined because the AGENT generic in AgentRelationship (et al)
/// applies to both the subject and object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AgentID {
    #[serde(rename = "user")]
    UserID(UserID),
    #[serde(rename = "company")]
    CompanyID(CompanyID),
}
impl From<UserID> for AgentID {
    fn from(id: UserID) -> Self {
        Self::UserID(id)
    }
}
impl From<CompanyID> for AgentID {
    fn from(id: CompanyID) -> Self {
        Self::CompanyID(id)
    }
}

