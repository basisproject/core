//! The vote module allows creating user objects that effectively have network
//! super powers. The idea is that this user can be wielded by whatever system
//! implements the core to represent votes either systemically or within
//! specific companies.
//!
//! The idea here is to provide an interface for democracy without the core
//! needing to know the implementation details.
//!
//! ```rust
//! use basis_core::{
//!     access::Role,
//!     models::{
//!         Agent,
//!         company::{CompanyID, Permission as CompanyPermission},
//!     },
//!     system::vote::Vote,
//! };
//! use chrono::Utc;
//!
//! let systemic_voter = Vote::systemic(&Utc::now()).unwrap();
//! assert_eq!(systemic_voter.user().roles(), &vec![Role::SuperAdmin]);
//! assert_eq!(systemic_voter.member(), &None);
//!
//! let company_id = CompanyID::new("hairy larry's scrumptious dairies");
//! let company_voter = Vote::company(&company_id, &Utc::now()).unwrap();
//! assert_eq!(company_voter.user().roles(), &vec![Role::User]);
//! assert_eq!(company_voter.member().as_ref().unwrap().inner().subject(), &company_voter.user().agent_id());
//! assert_eq!(company_voter.member().as_ref().unwrap().inner().object(), &company_id.clone().into());
//! assert_eq!(company_voter.member().as_ref().unwrap().permissions(), &vec![CompanyPermission::All]);
//! ```

use chrono::{DateTime, Utc};
use crate::{
    access::Role,
    error::{Error, Result},
    models::{
        company::{CompanyID, Permission as CompanyPermission},
        lib::agent::{Agent, AgentID},
        member::*,
        user::{User, UserID},
    },
};
use getset::Getters;
use vf_rs::vf;

/// An object that holds information about a voting user as well as any extra
/// information we might need. For instance, we might only need a user object if
/// voting on something systemic, but if a vote occurs within a specific company
/// then we need both a user and a member object created for us.
#[derive(Clone, Debug, PartialEq, Getters)]
#[getset(get = "pub")]
pub struct Vote {
    /// Holds our voting user
    user: User,
    /// Holds our voting member, if we have one
    member: Option<Member>,
}

impl Vote {
    /// Utility function to make a new user with a given role.
    fn make_voter(role: Role, now: &DateTime<Utc>) -> Result<User> {
        let id = UserID::create();
        User::builder()
            .id(id.clone())
            .roles(vec![role])
            .email(format!("vote-{}@basisproject.net", id.as_str()))
            .name(format!("Vote {}", id.as_str()))
            .active(true)
            .created(now.clone())
            .updated(now.clone())
            .build()
            .map_err(|e| Error::BuilderFailed(e))
    }

    /// Create a new systemic voting user.
    ///
    /// This can be used for systemic changes that don't need a company or a
    /// member object. For instance, this might be used to adjust costs of
    /// various tracked resources or manage occupation data. This user is given
    /// super admin abilities.
    ///
    /// If you want to vote to run a transaction for a specific company, see
    /// the `Vote::company()` method.
    pub fn systemic(now: &DateTime<Utc>) -> Result<Self> {
        let user = Self::make_voter(Role::SuperAdmin, now)?;
        Ok(Self {
            user,
            member: None,
        })
    }

    /// Create a new voting company member.
    ///
    /// This is specifically for voting to run a transaction internal to a
    /// company. This member is given company-wide admin abilities.
    pub fn company(company_id: &CompanyID, now: &DateTime<Utc>) -> Result<Self> {
        let user = Self::make_voter(Role::User, now)?;
        let id = MemberID::create();
        let company_agent_id: AgentID = company_id.clone().into();
        let member = Member::builder()
            .id(id)
            .inner(
                vf::AgentRelationship::builder()
                    .subject(user.agent_id())
                    .object(company_agent_id)
                    .relationship(())
                    .build()
                    .map_err(|e| Error::BuilderFailed(e))?
            )
            .class(MemberClass::User(MemberUser::new()))
            .permissions(vec![CompanyPermission::All])
            .agreement(None)
            .active(true)
            .created(now.clone())
            .updated(now.clone())
            .build()
            .map_err(|e| Error::BuilderFailed(e))?;
        Ok(Self {
            user,
            member: Some(member),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        util,
    };

    #[test]
    fn systemic() {
        let now = util::time::now();
        let voter = Vote::systemic(&now).unwrap();
        assert_eq!(voter.user().roles(), &vec![Role::SuperAdmin]);
        assert_eq!(voter.user().active(), &true);
        assert_eq!(voter.user().created(), &now);
        assert_eq!(voter.user().updated(), &now);
        assert_eq!(voter.member(), &None);
    }

    #[test]
    fn company() {
        let now = util::time::now();
        let company_id = CompanyID::new("hairy larry's scrumptious dairies");
        let voter = Vote::company(&company_id, &now).unwrap();
        let user = voter.user().clone();
        assert_eq!(user.roles(), &vec![Role::User]);
        assert_eq!(user.active(), &true);
        assert_eq!(user.created(), &now);
        assert_eq!(user.updated(), &now);

        let member = voter.member().clone().unwrap();
        assert_eq!(member.inner().subject(), &user.agent_id());
        assert_eq!(member.inner().object(), &company_id.clone().into());
        match member.class() {
            MemberClass::User(_) => {}
            _ => panic!("voter::tests::company() -- bad class"),
        }
        assert_eq!(member.permissions(), &vec![CompanyPermission::All]);
        assert_eq!(member.active(), &true);
        assert_eq!(member.created(), &now);
        assert_eq!(member.updated(), &now);
    }
}

