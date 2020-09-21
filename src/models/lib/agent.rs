use crate::{
    error::{Result, Error},
    models::{
        company::CompanyID,
        member::MemberID,
        lib::basis_model::Model,
        user::UserID,
    },
};
#[cfg(feature = "with_serde")]
use serde::{Serialize, Deserialize};
use std::convert::TryFrom;

/// A trait that holds common agent functionality, generally applied to models
/// with ID types implemented in `AgentID`.
pub trait Agent: Model {
    /// Convert the model's ID to and AgentID.
    fn agent_id(&self) -> AgentID;
}

/// VF (correctly) assumes different types of actors in the economic network
/// that have "agency" so here we define the objects that have agency within the
/// Basis system. This lets us use a more generic `AgentID` object that fulfills
/// VF's model while still constraining ourselves to a limited set of actors.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(Serialize, Deserialize))]
pub enum AgentID {
    #[cfg_attr(feature = "with_serde", serde(rename = "company"))]
    CompanyID(CompanyID),
    #[cfg_attr(feature = "with_serde", serde(rename = "member"))]
    MemberID(MemberID),
    #[cfg_attr(feature = "with_serde", serde(rename = "user"))]
    UserID(UserID),
}

/// Implements `From<ModelID> for AgentID` and also `TryFrom<AgentID> for ModelID`
macro_rules! impl_agent_for_model_id {
    ($idty:ident) => {
        impl From<$idty> for AgentID {
            fn from(val: $idty) -> Self {
                AgentID::$idty(val)
            }
        }

        impl TryFrom<AgentID> for $idty {
            type Error = Error;

            fn try_from(val: AgentID) -> Result<Self> {
                Ok(match val {
                    AgentID::$idty(id) => id,
                    _ => Err(Error::WrongAgentIDType)?,
                })
            }
        }
    };
}

impl_agent_for_model_id! { CompanyID }
impl_agent_for_model_id! { MemberID }
impl_agent_for_model_id! { UserID }

