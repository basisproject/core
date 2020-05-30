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
use std::convert::TryInto;

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

impl Into<AgentID> for CompanyID {
    fn into(self) -> AgentID {
        AgentID::CompanyID(self)
    }
}
impl Into<AgentID> for CompanyMemberID {
    fn into(self) -> AgentID {
        AgentID::MemberID(self)
    }
}
impl Into<AgentID> for RegionID {
    fn into(self) -> AgentID {
        AgentID::RegionID(self)
    }
}
impl Into<AgentID> for UserID {
    fn into(self) -> AgentID {
        AgentID::UserID(self)
    }
}

impl TryInto<CompanyID> for AgentID {
    type Error = Error;

    fn try_into(self) -> Result<CompanyID> {
        Ok(match self {
            AgentID::CompanyID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}
impl TryInto<CompanyMemberID> for AgentID {
    type Error = Error;

    fn try_into(self) -> Result<CompanyMemberID> {
        Ok(match self {
            AgentID::MemberID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}
impl TryInto<RegionID> for AgentID {
    type Error = Error;

    fn try_into(self) -> Result<RegionID> {
        Ok(match self {
            AgentID::RegionID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}
impl TryInto<UserID> for AgentID {
    type Error = Error;

    fn try_into(self) -> Result<UserID> {
        Ok(match self {
            AgentID::UserID(id) => id,
            _ => Err(Error::WrongAgentIDType)?,
        })
    }
}

