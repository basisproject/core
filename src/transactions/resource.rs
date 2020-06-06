use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        resource::{Resource, ResourceID},
        resource_spec::ResourceSpecID,
        user::User,
    },
};
use om2::Unit;
use url::Url;
use vf_rs::{vf, dfc};

/// Create a new resource
pub fn create(caller: &User, member: &CompanyMember, company: &Company, id: ResourceID, spec_id: ResourceSpecID, lot: Option<dfc::ProductBatch>, name: Option<String>, tracking_id: Option<String>, classifications: Vec<Url>, note: Option<String>, unit_of_effort: Option<Unit>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResources)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceCreate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    let model = Resource::builder()
        .id(id)
        .inner(
            vf::EconomicResource::builder()
                .classified_as(classifications)
                .conforms_to(spec_id)
                .lot(lot)
                .name(name)
                .note(note)
                .primary_accountable(Some(company.id().clone().into()))
                .tracking_identifier(tracking_id)
                .unit_of_effort(unit_of_effort)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .in_custody_of(company.id().clone())
        .costs(Costs::new())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a resource
pub fn update(caller: &User, member: &CompanyMember, company: &Company, mut subject: Resource, lot: Option<dfc::ProductBatch>, name: Option<String>, tracking_id: Option<String>, classifications: Option<Vec<Url>>, note: Option<String>, unit_of_effort: Option<Unit>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResources)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceUpdate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    if lot.is_some() {
        subject.inner_mut().set_lot(lot);
    }
    if name.is_some() {
        subject.inner_mut().set_name(name);
    }
    if tracking_id.is_some() {
        subject.inner_mut().set_tracking_identifier(tracking_id);
    }
    if let Some(classifications) = classifications {
        subject.inner_mut().set_classified_as(classifications);
    }
    if note.is_some() {
        subject.inner_mut().set_note(note);
    }
    if unit_of_effort.is_some() {
        subject.inner_mut().set_unit_of_effort(unit_of_effort);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a resource
pub fn delete(caller: &User, member: &CompanyMember, company: &Company, mut subject: Resource, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResources)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceDelete)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    if !subject.costs().is_zero() {
        Err(Error::CannotEraseCosts)?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::{CompanyID, CompanyType},
            company_member::CompanyMemberID,
            occupation::OccupationID,
            resource_spec::ResourceSpecID,
            testutils::{make_user, make_company, make_member, make_resource_spec},
            user::UserID,
        },
        util,
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = ResourceID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ResourceCreate], &now);
        let spec = make_resource_spec(&ResourceSpecID::create(), company.id(), "widgets, baby", &now);
        let lot = dfc::ProductBatch::builder()
            .batch_number("123")
            .build().unwrap();

        let mods = create(&user, &member, &company, id.clone(), spec.id().clone(), Some(lot.clone()), Some("widget batch".into()), None, vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()], Some("niceee".into()), Some(Unit::Hour), true, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Create).unwrap();
        assert_eq!(resource.id(), &id);
        assert_eq!(resource.inner().name(), &Some("widget batch".into()));
        assert_eq!(resource.inner().lot(), &Some(lot.clone()));
        assert_eq!(resource.inner().classified_as(), &vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()]);
        assert_eq!(resource.inner().primary_accountable(), &Some(company.id().clone().into()));
        assert_eq!(resource.inner().tracking_identifier(), &None);
        assert_eq!(resource.inner().note(), &Some("niceee".into()));
        assert_eq!(resource.inner().unit_of_effort(), &Some(Unit::Hour));
        assert_eq!(resource.in_custody_of(), &company.id().clone().into());
        assert!(resource.costs().is_zero());
        assert_eq!(resource.active(), &true);
        assert_eq!(resource.created(), &now);
        assert_eq!(resource.updated(), &now);
        assert_eq!(resource.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::ResourceDelete]);
        let res = create(&user, &member2, &company, id.clone(), spec.id().clone(), Some(lot.clone()), Some("widget batch".into()), None, vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()], Some("niceee".into()), Some(Unit::Hour), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = create(&user2, &member, &company, id.clone(), spec.id().clone(), Some(lot.clone()), Some("widget batch".into()), None, vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()], Some("niceee".into()), Some(Unit::Hour), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now.clone()));
        let res = create(&user, &member, &company2, id.clone(), spec.id().clone(), Some(lot.clone()), Some("widget batch".into()), None, vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()], Some("niceee".into()), Some(Unit::Hour), true, &now);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = ResourceID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let mut member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ResourceCreate], &now);
        let spec = make_resource_spec(&ResourceSpecID::create(), company.id(), "widgets, baby", &now);
        let lot = dfc::ProductBatch::builder()
            .batch_number("123")
            .build().unwrap();
        let mods = create(&user, &member, &company, id.clone(), spec.id().clone(), Some(lot.clone()), Some("widget batch".into()), None, vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()], Some("niceee".into()), Some(Unit::Hour), true, &now).unwrap().into_modifications();
        let resource = mods[0].clone().expect_op::<Resource>(Op::Create).unwrap();

        let res = update(&user, &member, &company, resource.clone(), None, Some("better widgets".into()), Some("444-computers-and-equipment".into()), None, None, Some(Unit::WattHour), Some(false), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        member.set_permissions(vec![CompanyPermission::ResourceUpdate]);
        let now2 = util::time::now();
        let mods = update(&user, &member, &company, resource.clone(), None, Some("better widgets".into()), Some("444-computers-and-equipment".into()), None, None, Some(Unit::WattHour), Some(false), &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let resource2 = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.id(), &id);
        assert_eq!(resource2.inner().name(), &Some("better widgets".into()));
        assert_eq!(resource2.inner().lot(), &Some(lot.clone()));
        assert_eq!(resource2.inner().classified_as(), &vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()]);
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.id().clone().into()));
        assert_eq!(resource2.inner().tracking_identifier(), &Some("444-computers-and-equipment".into()));
        assert_eq!(resource2.inner().note(), &Some("niceee".into()));
        assert_eq!(resource2.inner().unit_of_effort(), &Some(Unit::WattHour));
        assert_eq!(resource2.in_custody_of(), &company.id().clone().into());
        assert_eq!(resource2.active(), &false);
        assert_eq!(resource2.created(), &now);
        assert_eq!(resource2.updated(), &now);
        assert_eq!(resource2.deleted(), &None);

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = update(&user2, &member, &company, resource.clone(), None, Some("better widgets".into()), Some("444-computers-and-equipment".into()), None, None, Some(Unit::WattHour), Some(false), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = update(&user, &member, &company2, resource.clone(), None, Some("better widgets".into()), Some("444-computers-and-equipment".into()), None, None, Some(Unit::WattHour), Some(false), &now);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = ResourceID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let mut member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ResourceCreate], &now);
        let spec = make_resource_spec(&ResourceSpecID::create(), company.id(), "widgets, baby", &now);
        let lot = dfc::ProductBatch::builder()
            .batch_number("123")
            .build().unwrap();
        let mods = create(&user, &member, &company, id.clone(), spec.id().clone(), Some(lot.clone()), Some("widget batch".into()), None, vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()], Some("niceee".into()), Some(Unit::Hour), true, &now).unwrap().into_modifications();
        let resource = mods[0].clone().expect_op::<Resource>(Op::Create).unwrap();

        let now2 = util::time::now();
        let res = delete(&user, &member, &company, resource.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        member.set_permissions(vec![CompanyPermission::ResourceDelete]);
        let mods = delete(&user, &member, &company, resource.clone(), &now2).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let resource2 = mods[0].clone().expect_op::<Resource>(Op::Delete).unwrap();
        assert_eq!(resource2.id(), &id);
        assert_eq!(resource2.inner().name(), &Some("widget batch".into()));
        assert_eq!(resource2.inner().lot(), &Some(lot.clone()));
        assert_eq!(resource2.inner().classified_as(), &vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()]);
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.id().clone().into()));
        assert_eq!(resource2.inner().tracking_identifier(), &None);
        assert_eq!(resource2.inner().note(), &Some("niceee".into()));
        assert_eq!(resource2.inner().unit_of_effort(), &Some(Unit::Hour));
        assert_eq!(resource2.in_custody_of(), &company.id().clone().into());
        assert_eq!(resource2.active(), &true);
        assert_eq!(resource2.created(), &now);
        assert_eq!(resource2.updated(), &now);
        assert_eq!(resource2.deleted(), &Some(now2.clone()));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = delete(&user2, &member, &company, resource.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = delete(&user, &member, &company2, resource.clone(), &now2);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }
}

