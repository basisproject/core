//! A commitment is a promise to deliver on some future economic event.
//!
//! Commitments are effectively line items in a larger order (see `agreement`)
//! such as a line on a receipt or one item in an online shopping cart. They are
//! restricted to a few types of actions [defined in OrderAction][1].
//!
//! See the [commitment model.][2]
//!
//! [1]: ../enum.OrderAction.html
//! [2]: ../../models/commitment/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        agreement::Agreement,
        commitment::{Commitment, CommitmentID},
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        lib::{
            agent::{Agent, AgentID},
            basis_model::Deletable,
        },
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
        user::User,
    },
    transactions::OrderAction,
};
use om2::Measure;
use url::Url;
use vf_rs::{vf, geo::SpatialThing};

/// Create a new commitment
pub fn create(caller: &User, member: &CompanyMember, company: &Company, agreement: &Agreement, id: CommitmentID, move_costs: Costs, action: OrderAction, agreed_in: Option<Url>, at_location: Option<SpatialThing>, created: Option<DateTime<Utc>>, due: Option<DateTime<Utc>>, effort_quantity: Option<Measure>, finished: Option<bool>, has_beginning: Option<DateTime<Utc>>, has_end: Option<DateTime<Utc>>, has_point_in_time: Option<DateTime<Utc>>, in_scope_of: Vec<AgentID>, input_of: Option<ProcessID>, name: Option<String>, note: Option<String>, output_of: Option<ProcessID>, provider: AgentID, receiver: AgentID, resource_conforms_to: Option<ResourceSpecID>, resource_inventoried_as: Option<ResourceID>, resource_quantity: Option<Measure>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateCommitments)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::CommitmentCreate)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    let company_agent_id: AgentID = company.agent_id();
    if company_agent_id != provider && company_agent_id != receiver {
        // can't create a commitment for a company you aren't a member of DUUUHHH
        Err(Error::InsufficientPrivileges)?;
    }
    if !agreement.has_participant(&provider) || !agreement.has_participant(&receiver) {
        // can't create a commitment for an agreement you are not party to
        Err(Error::InsufficientPrivileges)?;
    }
    let event_action = match action {
        OrderAction::DeliverService => vf::Action::DeliverService,
        OrderAction::Transfer => vf::Action::Transfer,
        OrderAction::TransferCustody => vf::Action::TransferCustody,
    };
    let model = Commitment::builder()
        .id(id)
        .inner(
            vf::Commitment::builder()
                .action(event_action)
                .agreed_in(agreed_in)
                .at_location(at_location)
                .clause_of(Some(agreement.id().clone()))
                .created(created)
                .due(due)
                .effort_quantity(effort_quantity)
                .finished(finished)
                .has_beginning(has_beginning)
                .has_end(has_end)
                .has_point_in_time(has_point_in_time)
                .in_scope_of(in_scope_of)
                .input_of(input_of)
                .name(name)
                .note(note)
                .output_of(output_of)
                .provider(provider)
                .receiver(receiver)
                .resource_conforms_to(resource_conforms_to)
                .resource_inventoried_as(resource_inventoried_as)
                .resource_quantity(resource_quantity)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(move_costs)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a commitment
pub fn update(caller: &User, member: &CompanyMember, company: &Company, mut subject: Commitment, move_costs: Option<Costs>, action: Option<OrderAction>, agreed_in: Option<Option<Url>>, at_location: Option<Option<SpatialThing>>, created: Option<Option<DateTime<Utc>>>, due: Option<Option<DateTime<Utc>>>, effort_quantity: Option<Option<Measure>>, finished: Option<Option<bool>>, has_beginning: Option<Option<DateTime<Utc>>>, has_end: Option<Option<DateTime<Utc>>>, has_point_in_time: Option<Option<DateTime<Utc>>>, in_scope_of: Option<Vec<AgentID>>, input_of: Option<Option<ProcessID>>, name: Option<Option<String>>, note: Option<Option<String>>, output_of: Option<Option<ProcessID>>, resource_conforms_to: Option<Option<ResourceSpecID>>, resource_inventoried_as: Option<Option<ResourceID>>, resource_quantity: Option<Option<Measure>>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateCommitments)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::CommitmentUpdate)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    let event_action = action.map(|x| {
        match x {
            OrderAction::DeliverService => vf::Action::DeliverService,
            OrderAction::Transfer => vf::Action::Transfer,
            OrderAction::TransferCustody => vf::Action::TransferCustody,
        }
    });

    if let Some(move_costs) = move_costs {
        subject.set_move_costs(move_costs);
    }
    if let Some(event_action) = event_action {
        subject.inner_mut().set_action(event_action);
    }
    if let Some(agreed_in) = agreed_in {
        subject.inner_mut().set_agreed_in(agreed_in);
    }
    if let Some(at_location) = at_location {
        subject.inner_mut().set_at_location(at_location);
    }
    if let Some(created) = created {
        subject.inner_mut().set_created(created);
    }
    if let Some(due) = due {
        subject.inner_mut().set_due(due);
    }
    if let Some(effort_quantity) = effort_quantity {
        subject.inner_mut().set_effort_quantity(effort_quantity);
    }
    if let Some(finished) = finished {
        subject.inner_mut().set_finished(finished);
    }
    if let Some(has_beginning) = has_beginning {
        subject.inner_mut().set_has_beginning(has_beginning);
    }
    if let Some(has_end) = has_end {
        subject.inner_mut().set_has_end(has_end);
    }
    if let Some(has_point_in_time) = has_point_in_time {
        subject.inner_mut().set_has_point_in_time(has_point_in_time);
    }
    if let Some(in_scope_of) = in_scope_of {
        subject.inner_mut().set_in_scope_of(in_scope_of);
    }
    if let Some(input_of) = input_of {
        subject.inner_mut().set_input_of(input_of);
    }
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(note) = note {
        subject.inner_mut().set_note(note);
    }
    if let Some(output_of) = output_of {
        subject.inner_mut().set_output_of(output_of);
    }
    if let Some(resource_conforms_to) = resource_conforms_to {
        subject.inner_mut().set_resource_conforms_to(resource_conforms_to);
    }
    if let Some(resource_inventoried_as) = resource_inventoried_as {
        subject.inner_mut().set_resource_inventoried_as(resource_inventoried_as);
    }
    if let Some(resource_quantity) = resource_quantity {
        subject.inner_mut().set_resource_quantity(resource_quantity);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a commitment
pub fn delete(caller: &User, member: &CompanyMember, company: &Company, mut subject: Commitment, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateCommitments)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::CommitmentDelete)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            agreement::AgreementID,
            company::CompanyID,
            company_member::CompanyMemberID,
            occupation::OccupationID,
            testutils::{make_agreement, make_user, make_company, make_member_worker, make_resource},
            user::UserID,
        },
        util,
    };
    use om2::Unit;
    use rust_decimal_macros::*;

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = CommitmentID::create();
        let company_from = make_company(&CompanyID::create(), "bridget's widgets", &now);
        let company_to = make_company(&CompanyID::create(), "larry's chairs", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 111222", "UwU big order of widgetzzz", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member_worker(&CompanyMemberID::create(), user.id(), company_to.id(), &OccupationID::create(), vec![CompanyPermission::CommitmentCreate], &now);
        let costs = Costs::new_with_labor("widgetmaker", 42);
        let resource = make_resource(&ResourceID::new("widget1"), company_from.id(), &Measure::new(dec!(30), Unit::One), &Costs::new_with_labor("widgetmaker", dec!(50)), &now);
        let loc = SpatialThing::builder()
            .mappable_address(Some("444 Checkmate lane, LOGIC and FACTS, MN, 33133".into()))
            .build().unwrap();

        let mods = create(&user, &member, &company_to, &agreement, id.clone(), costs.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let commitment = mods[0].clone().expect_op::<Commitment>(Op::Create).unwrap();
        assert_eq!(commitment.id(), &id);
        assert_eq!(commitment.move_costs(), &costs.clone());
        assert_eq!(commitment.inner().action(), &vf::Action::Transfer);
        assert_eq!(commitment.inner().agreed_in(), &None);
        assert_eq!(commitment.inner().at_location(), &Some(loc.clone()));
        assert_eq!(commitment.inner().created(), &Some(now.clone()));
        assert_eq!(commitment.inner().due(), &None);
        assert_eq!(commitment.inner().effort_quantity(), &None);
        assert_eq!(commitment.inner().finished(), &Some(false));
        assert_eq!(commitment.inner().has_beginning(), &None);
        assert_eq!(commitment.inner().has_end(), &None);
        assert_eq!(commitment.inner().has_point_in_time(), &None);
        assert_eq!(commitment.inner().in_scope_of(), &vec![]);
        assert_eq!(commitment.inner().input_of(), &None);
        assert_eq!(commitment.inner().name(), &Some("widgetzz".into()));
        assert_eq!(commitment.inner().note(), &Some("sending widgets to larry".into()));
        assert_eq!(commitment.inner().output_of(), &None);
        assert_eq!(commitment.inner().provider(), &company_from.agent_id());
        assert_eq!(commitment.inner().receiver(), &company_to.agent_id());
        assert_eq!(commitment.inner().resource_conforms_to(), &None);
        assert_eq!(commitment.inner().resource_inventoried_as(), &Some(ResourceID::new("widget1")));
        assert_eq!(commitment.inner().resource_quantity(), &Some(Measure::new(dec!(10), Unit::One)));
        assert_eq!(commitment.active(), &true);
        assert_eq!(commitment.created(), &now);
        assert_eq!(commitment.updated(), &now);
        assert_eq!(commitment.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::ProcessDelete]);
        let res = create(&user, &member2, &company_to, &agreement, id.clone(), costs.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = create(&user2, &member, &company_to, &agreement, id.clone(), costs.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company_to2 = company_to.clone();
        company_to2.set_deleted(Some(now.clone()));
        let res = create(&user, &member, &company_to2, &agreement, id.clone(), costs.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));

        let mut company3 = company_to.clone();
        let mut company4 = company_to.clone();
        company3.set_id(CompanyID::new("bill's zingers, get your premium zings here. got a friend who constantly pranks you? turn the tables and zing that doofus in front of everyone!!"));
        company4.set_id(CompanyID::new("jill's zingers, get the best zings here. turn that lame party into a laugh fest with some classic zingers. don't buy at bill's, he sucks."));
        let res = create(&user, &member, &company_to, &agreement, id.clone(), costs.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company3.agent_id(), company4.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![]);
        let res = create(&user, &member, &company_to, &agreement2, id.clone(), costs.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = CommitmentID::create();
        let company_from = make_company(&CompanyID::create(), "bridget's widgets", &now);
        let company_to = make_company(&CompanyID::create(), "larry's chairs", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 111222", "UwU big order of widgetzzz", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member_worker(&CompanyMemberID::create(), user.id(), company_to.id(), &OccupationID::create(), vec![CompanyPermission::CommitmentCreate, CompanyPermission::CommitmentUpdate], &now);
        let costs1 = Costs::new_with_labor("widgetmaker", 42);
        let costs2 = Costs::new_with_labor("widgetmaker", 31);
        let resource = make_resource(&ResourceID::new("widget1"), company_from.id(), &Measure::new(dec!(30), Unit::One), &Costs::new_with_labor("widgetmaker", dec!(50)), &now);
        let agreement_url: Url = "http://legalzoom.com/standard-widget-shopping-cart-agreement".parse().unwrap();
        let loc = SpatialThing::builder()
            .mappable_address(Some("444 Checkmate lane, LOGIC and FACTS, MN, 33133".into()))
            .build().unwrap();

        let mods = create(&user, &member, &company_to, &agreement, id.clone(), costs1.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now).unwrap().into_vec();
        let commitment1 = mods[0].clone().expect_op::<Commitment>(Op::Create).unwrap();
        let now2 = util::time::now();
        let mods = update(&user, &member, &company_to, commitment1.clone(), Some(costs2.clone()), None, Some(Some(agreement_url.clone())), None, Some(Some(now2.clone())), None, None, Some(Some(true)), Some(Some(now.clone())), None, None, Some(vec![company_from.agent_id()]), None, None, Some(Some("here, larry".into())), None, None, None, Some(Some(Measure::new(dec!(50), Unit::One))), None, &now2).unwrap().into_vec();
        let commitment2 = mods[0].clone().expect_op::<Commitment>(Op::Update).unwrap();

        assert_eq!(commitment2.id(), commitment1.id());
        assert_eq!(commitment2.move_costs(), &costs2);
        assert_eq!(commitment2.inner().action(), commitment1.inner().action());
        assert_eq!(commitment2.inner().agreed_in(), &Some(agreement_url.clone()));
        assert_eq!(commitment2.inner().at_location(), commitment1.inner().at_location());
        assert_eq!(commitment2.inner().clause_of(), commitment1.inner().clause_of());
        assert_eq!(commitment2.inner().created(), &Some(now2.clone()));
        assert_eq!(commitment2.inner().due(), commitment1.inner().due());
        assert_eq!(commitment2.inner().effort_quantity(), commitment1.inner().effort_quantity());
        assert_eq!(commitment2.inner().finished(), &Some(true));
        assert_eq!(commitment2.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(commitment2.inner().has_end(), commitment1.inner().has_end());
        assert_eq!(commitment2.inner().has_point_in_time(), commitment1.inner().has_point_in_time());
        assert_eq!(commitment2.inner().in_scope_of(), &vec![company_from.agent_id()]);
        assert_eq!(commitment2.inner().input_of(), commitment1.inner().input_of());
        assert_eq!(commitment2.inner().name(), commitment1.inner().name());
        assert_eq!(commitment2.inner().note(), &Some("here, larry".into()));
        assert_eq!(commitment2.inner().output_of(), commitment1.inner().output_of());
        assert_eq!(commitment2.inner().provider(), commitment1.inner().provider());
        assert_eq!(commitment2.inner().receiver(), commitment1.inner().receiver());
        assert_eq!(commitment2.inner().resource_conforms_to(), commitment1.inner().resource_conforms_to());
        assert_eq!(commitment2.inner().resource_inventoried_as(), commitment1.inner().resource_inventoried_as());
        assert_eq!(commitment2.inner().resource_quantity(), &Some(Measure::new(dec!(50), Unit::One)));
        assert_eq!(commitment2.active(), &true);
        assert_eq!(commitment2.created(), &now);
        assert_eq!(commitment2.updated(), &now2);
        assert_eq!(commitment2.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::ProcessDelete]);
        let res = update(&user, &member2, &company_to, commitment1.clone(), Some(costs2.clone()), None, Some(Some(agreement_url.clone())), None, Some(Some(now2.clone())), None, None, Some(Some(true)), Some(Some(now.clone())), None, None, Some(vec![company_from.agent_id()]), None, None, Some(Some("here, larry".into())), None, None, None, Some(Some(Measure::new(dec!(50), Unit::One))), None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = update(&user2, &member, &company_to, commitment1.clone(), Some(costs2.clone()), None, Some(Some(agreement_url.clone())), None, Some(Some(now2.clone())), None, None, Some(Some(true)), Some(Some(now.clone())), None, None, Some(vec![company_from.agent_id()]), None, None, Some(Some("here, larry".into())), None, None, None, Some(Some(Measure::new(dec!(50), Unit::One))), None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company_to2 = company_to.clone();
        company_to2.set_deleted(Some(now.clone()));
        let res = update(&user, &member, &company_to2, commitment1.clone(), Some(costs2.clone()), None, Some(Some(agreement_url.clone())), None, Some(Some(now2.clone())), None, None, Some(Some(true)), Some(Some(now.clone())), None, None, Some(vec![company_from.agent_id()]), None, None, Some(Some("here, larry".into())), None, None, None, Some(Some(Measure::new(dec!(50), Unit::One))), None, &now2);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = CommitmentID::create();
        let company_from = make_company(&CompanyID::create(), "bridget's widgets", &now);
        let company_to = make_company(&CompanyID::create(), "larry's dairies (outdoor outdoor. shutup parker. thank you parker, shutup. thank you.)", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 111222", "UwU big order of widgetzzz", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member_worker(&CompanyMemberID::create(), user.id(), company_to.id(), &OccupationID::create(), vec![CompanyPermission::CommitmentCreate, CompanyPermission::CommitmentDelete], &now);
        let resource = make_resource(&ResourceID::new("widget1"), company_from.id(), &Measure::new(dec!(30), Unit::One), &Costs::new_with_labor("widgetmaker", dec!(50)), &now);
        let costs1 = Costs::new_with_labor("widgetmaker", 42);
        let loc = SpatialThing::builder()
            .mappable_address(Some("444 Checkmate lane, LOGIC and FACTS, MN, 33133".into()))
            .build().unwrap();

        let mods = create(&user, &member, &company_to, &agreement, id.clone(), costs1.clone(), OrderAction::Transfer, None, Some(loc.clone()), Some(now.clone()), None, None, Some(false), None, None, None, vec![], None, Some("widgetzz".into()), Some("sending widgets to larry".into()), None, company_from.agent_id(), company_to.agent_id(), None, Some(resource.id().clone()), Some(Measure::new(dec!(10), Unit::One)), true, &now).unwrap().into_vec();
        let commitment1 = mods[0].clone().expect_op::<Commitment>(Op::Create).unwrap();

        let now2 = util::time::now();
        let mods = delete(&user, &member, &company_to, commitment1.clone(), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let commitment2 = mods[0].clone().expect_op::<Commitment>(Op::Delete).unwrap();
        assert_eq!(commitment2.id(), commitment1.id());
        assert_eq!(commitment2.move_costs(), commitment1.move_costs());
        assert_eq!(commitment2.inner().action(), commitment1.inner().action());
        assert_eq!(commitment2.inner().agreed_in(), commitment1.inner().agreed_in());
        assert_eq!(commitment2.inner().at_location(), commitment1.inner().at_location());
        assert_eq!(commitment2.inner().clause_of(), commitment1.inner().clause_of());
        assert_eq!(commitment2.inner().created(), commitment1.inner().created());
        assert_eq!(commitment2.inner().due(), commitment1.inner().due());
        assert_eq!(commitment2.inner().effort_quantity(), commitment1.inner().effort_quantity());
        assert_eq!(commitment2.inner().finished(), commitment1.inner().finished());
        assert_eq!(commitment2.inner().has_beginning(), commitment1.inner().has_beginning());
        assert_eq!(commitment2.inner().has_end(), commitment1.inner().has_end());
        assert_eq!(commitment2.inner().has_point_in_time(), commitment1.inner().has_point_in_time());
        assert_eq!(commitment2.inner().in_scope_of(), commitment1.inner().in_scope_of());
        assert_eq!(commitment2.inner().input_of(), commitment1.inner().input_of());
        assert_eq!(commitment2.inner().name(), commitment1.inner().name());
        assert_eq!(commitment2.inner().note(), commitment1.inner().note());
        assert_eq!(commitment2.inner().output_of(), commitment1.inner().output_of());
        assert_eq!(commitment2.inner().provider(), commitment1.inner().provider());
        assert_eq!(commitment2.inner().receiver(), commitment1.inner().receiver());
        assert_eq!(commitment2.inner().resource_conforms_to(), commitment1.inner().resource_conforms_to());
        assert_eq!(commitment2.inner().resource_inventoried_as(), commitment1.inner().resource_inventoried_as());
        assert_eq!(commitment2.inner().resource_quantity(), commitment1.inner().resource_quantity());
        assert_eq!(commitment2.active(), commitment1.active());
        assert_eq!(commitment2.created(), commitment1.created());
        assert_eq!(commitment2.updated(), commitment1.updated());
        assert_eq!(commitment2.deleted(), &Some(now2.clone()));

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::ProcessDelete]);
        let res = delete(&user, &member2, &company_to, commitment1.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = delete(&user2, &member, &company_to, commitment1.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company_to2 = company_to.clone();
        company_to2.set_deleted(Some(now2.clone()));
        let res = delete(&user, &member, &company_to2, commitment1.clone(), &now2);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));
    }
}

