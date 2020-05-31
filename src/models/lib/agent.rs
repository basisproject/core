use crate::{
    error::{Result, Error},
    models::{
        company::CompanyID,
        company_member::CompanyMemberID,
        region::RegionID,
        user::UserID,
    },
};
use serde::{Serialize, Deserialize};
use std::convert::TryFrom;

/// VF (correctly) assumes different types of actors in the economic network
/// that have "agency" so here we define the objects that have agency within the
/// Basis system. This lets us use a more generic `AgentID` object that fulfills
/// VF's model while still constraining ourselves to a limited set of actors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AgentID {
    #[serde(rename = "company")]
    CompanyID(CompanyID),
    #[serde(rename = "member")]
    MemberID(CompanyMemberID),
    #[serde(rename = "region")]
    RegionID(RegionID),
    #[serde(rename = "user")]
    UserID(UserID),
}

impl From<CompanyID> for AgentID {
    fn from(val: CompanyID) -> Self {
        AgentID::CompanyID(val)
    }
}
impl From<CompanyMemberID> for AgentID {
    fn from(val: CompanyMemberID) -> Self {
        AgentID::MemberID(val)
    }
}
impl From<RegionID> for AgentID {
    fn from(val: RegionID) -> Self {
        AgentID::RegionID(val)
    }
}
impl From<UserID> for AgentID {
    fn from(val: UserID) -> Self {
        AgentID::UserID(val)
    }
}

impl TryFrom<AgentID> for CompanyID {
    type Error = Error;

    fn try_from(val: AgentID) -> Result<Self> {
        Ok(match val {
            AgentID::CompanyID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}
impl TryFrom<AgentID> for CompanyMemberID {
    type Error = Error;

    fn try_from(val: AgentID) -> Result<Self> {
        Ok(match val {
            AgentID::MemberID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}
impl TryFrom<AgentID> for RegionID {
    type Error = Error;

    fn try_from(val: AgentID) -> Result<Self> {
        Ok(match val {
            AgentID::RegionID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}
impl TryFrom<AgentID> for UserID {
    type Error = Error;

    fn try_from(val: AgentID) -> Result<Self> {
        Ok(match val {
            AgentID::UserID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}

