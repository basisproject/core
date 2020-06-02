use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, CompanyID, CompanyType, Permission as CompanyPermission},
        company_member::{CompanyMember, CompanyMemberID},
        occupation::OccupationID,
        user::User,
    },
};
use vf_rs::vf;

/// Creates a new private company
pub fn create_private<T: Into<String>>(caller: &User, id: CompanyID, company_name: T, company_email: T, founder_id: CompanyMemberID, founder_occupation_id: OccupationID, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::CompanyCreatePrivate)?;
    let company = Company::builder()
        .id(id.clone())
        .ty(CompanyType::Private)
        .inner(
            vf::Agent::builder()
                .name(company_name)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .email(company_email)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let founder = CompanyMember::builder()
        .id(founder_id)
        .inner(
            vf::AgentRelationship::builder()
                .subject(caller.id().clone())
                .object(id.clone())
                .relationship(founder_occupation_id)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .permissions(vec![CompanyPermission::All])
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let mut mods = Modifications::new();
    mods.push(Op::Create, company);
    mods.push(Op::Create, founder);
    Ok(mods)
}

/// Update a private company
pub fn update_private(caller: &User, member: &CompanyMember, mut subject: Company, name: Option<String>, email: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::CompanyAdminUpdate)
        .or_else(|_| access_check!(member, CompanyPermission::CompanyUpdate))?;
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(email) = email {
        subject.set_email(email);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new())
}

/// Delete a private company
pub fn delete_private(caller: &User, member: &CompanyMember, mut subject: Company, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::CompanyAdminDelete)
        .or_else(|_| access_check!(member, CompanyPermission::CompanyDelete))?;
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
}

