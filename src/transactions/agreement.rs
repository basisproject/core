//! An agreement represents a grouping of commitments and events betwixt two
//! agents.
//!
//! In other words, an agreement is basically an order.
//!
//! See the [agreement model.][1]
//!
//! [1]: ../../models/agreement/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        lib::{
            agent::AgentID,
            basis_model::Model,
        },
        agreement::{Agreement, AgreementID},
        company::{Company, Permission as CompanyPermission},
        member::Member,
        user::User,
    },
};
use vf_rs::vf;

/// Create a new agreement/order.
///
/// When updating data connected to an agreement, only agents that are in the
/// agreement's `participants` list will be allowed to complete updates. This
/// makes it so only those involved in the agreement can modify it or any of its
/// data in any way.
pub fn create<T: Into<String>>(caller: &User, member: &Member, company: &Company, id: AgreementID, participants: Vec<AgentID>, name: T, note: T, created: Option<DateTime<Utc>>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateAgreements)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::AgreementCreate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
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
        .participants(participants)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update an agreement, including the participant list.
pub fn update(caller: &User, member: &Member, company: &Company, mut subject: Agreement, participants: Option<Vec<AgentID>>, name: Option<String>, note: Option<String>, created: Option<Option<DateTime<Utc>>>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateAgreements)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::AgreementUpdate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if let Some(participants) = participants {
        subject.set_participants(participants);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            lib::agent::Agent,
            company::CompanyID,
            member::MemberID,
            occupation::OccupationID,
            testutils::{deleted_company_tester, make_user, make_company, make_member_worker},
            user::UserID,
        },
        util,
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = AgreementID::create();
        let company_to = make_company(&CompanyID::create(), "sam's widgets", &now);
        let company_from = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member_worker(&MemberID::create(), user.id(), company_to.id(), &OccupationID::create(), vec![CompanyPermission::AgreementCreate], &now);
        let participants = vec![company_to.agent_id(), company_from.agent_id()];

        let mods = create(&user, &member, &company_to, id.clone(), participants.clone(), "order 1234141", "hi i'm jerry. just going to order some widgets. don't mind me, just ordering widgets.", Some(now.clone()), true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let agreement = mods[0].clone().expect_op::<Agreement>(Op::Create).unwrap();
        assert_eq!(agreement.id(), &id);
        assert_eq!(agreement.inner().created(), &Some(now.clone()));
        assert_eq!(agreement.inner().name(), &Some("order 1234141".into()));
        assert_eq!(agreement.inner().note(), &Some("hi i'm jerry. just going to order some widgets. don't mind me, just ordering widgets.".into()));
        assert_eq!(agreement.participants(), &participants);
        assert_eq!(agreement.active(), &true);
        assert_eq!(agreement.created(), &now);
        assert_eq!(agreement.updated(), &now);
        assert_eq!(agreement.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::AgreementUpdate]);
        let res = create(&user, &member2, &company_to, id.clone(), participants.clone(), "order 1234141", "hi i'm jerry. just going to order some widgets. don't mind me, just ordering widgets.", Some(now.clone()), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = create(&user2, &member, &company_to, id.clone(), participants.clone(), "order 1234141", "hi i'm jerry. just going to order some widgets. don't mind me, just ordering widgets.", Some(now.clone()), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company_to.clone(), &now, |company: Company| {
            create(&user, &member, &company, id.clone(), participants.clone(), "order 1234141", "hi i'm jerry. just going to order some widgets. don't mind me, just ordering widgets.", Some(now.clone()), true, &now)
        });
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = AgreementID::create();
        let company_to = make_company(&CompanyID::create(), "sam's widgets", &now);
        let company_from = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member_worker(&MemberID::create(), user.id(), company_to.id(), &OccupationID::create(), vec![CompanyPermission::AgreementCreate, CompanyPermission::AgreementUpdate], &now);
        let participants = vec![company_to.agent_id(), company_from.agent_id()];

        let mods = create(&user, &member, &company_to, id.clone(), participants.clone(), "order 1234141", "hi i'm jerry. just going to order some widgets. don't mind me, just ordering widgets.", Some(now.clone()), true, &now).unwrap().into_vec();
        let agreement1 = mods[0].clone().expect_op::<Agreement>(Op::Create).unwrap();
        let now2 = util::time::now();
        let mods = update(&user, &member, &company_to, agreement1.clone(), Some(vec![company_from.agent_id()]), Some("order 1111222".into()), Some("jerry's long-winded order".into()), None, None, &now2).unwrap().into_vec();
        let agreement2 = mods[0].clone().expect_op::<Agreement>(Op::Update).unwrap();

        assert_eq!(agreement2.id(), agreement1.id());
        assert_eq!(agreement2.inner().created(), agreement1.inner().created());
        assert_eq!(agreement2.inner().name(), &Some("order 1111222".into()));
        assert_eq!(agreement2.inner().note(), &Some("jerry's long-winded order".into()));
        assert_eq!(agreement2.participants(), &vec![company_from.agent_id()]);
        assert_eq!(agreement2.active(), agreement1.active());
        assert_eq!(agreement2.created(), agreement1.created());
        assert_eq!(agreement2.updated(), &now2);
        assert_eq!(agreement2.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::AgreementCreate]);
        let res = update(&user, &member2, &company_to, agreement1.clone(), None, Some("order 1111222".into()), Some("jerry's long-winded order".into()), None, None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = update(&user2, &member, &company_to, agreement1.clone(), None, Some("order 1111222".into()), Some("jerry's long-winded order".into()), None, None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company_to.clone(), &now2, |company: Company| {
            update(&user, &member, &company, agreement1.clone(), None, Some("order 1111222".into()), Some("jerry's long-winded order".into()), None, None, &now2)
        });
    }
}

