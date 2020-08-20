//! An agreement represents a grouping of commitments and events betwixt two
//! agents.
//!
//! In other words, an agreement is basically an order.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        agreement::{Agreement, AgreementID},
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        user::User,
    },
};
use vf_rs::vf;

/// Create a new agreement/order
pub fn create<T: Into<String>>(caller: &User, member: &CompanyMember, company: &Company, id: AgreementID, name: T, note: T, created: Option<DateTime<Utc>>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateAgreements)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::AgreementCreate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    let model = Agreement::builder()
        .id(id)
        .inner(
            vf::Agreement::builder()
                .created(created)
                .name(Some(name.into()))
                .note(Some(note.into()))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .finalized(false)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update an agreement (mainly just name/note, everything else is commitment/
/// event management).
pub fn update(caller: &User, member: &CompanyMember, company: &Company, mut subject: Agreement, name: Option<String>, note: Option<String>, created: Option<Option<DateTime<Utc>>>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateAgreements)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::AgreementUpdate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    if *subject.finalized() {
        Err(Error::ObjectIsReadOnly("agreement".into()))?;
    }
    if let Some(created) = created {
        subject.inner_mut().set_created(created);
    }
    if let Some(name) = name {
        subject.inner_mut().set_name(Some(name));
    }
    if let Some(note) = note {
        subject.inner_mut().set_note(Some(note));
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Finalize an agreement
pub fn finalize(caller: &User, member: &CompanyMember, company: &Company, mut subject: Agreement, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateAgreements)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::AgreementUpdate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    if *subject.finalized() {
        Err(Error::ObjectIsReadOnly("agreement".into()))?;
    }
    subject.set_finalized(true);
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

