//! Intents are a way to signal either a want or an offer, and often lead to
//! a `Commitment` which can be thought of as part an order between two agents.
//!
//! For instance, if you made a widget and you want someone to purchase it, you
//! would create and publish an intent to `transfer` that widget.
//!
//! See the [intent model.][1]
//!
//! [1]: ../../models/intent/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, Permission as CompanyPermission},
        member::Member,
        lib::{
            agent::{Agent, AgentID},
            basis_model::Model,
        },
        intent::{Intent, IntentID},
        resource::ResourceID,
        resource_spec::ResourceSpecID,
        user::User,
    },
    transactions::OrderAction,
};
use om2::Measure;
use url::Url;
use vf_rs::{vf, geo::SpatialThing};

/// Create a new intent
pub fn create(caller: &User, member: &Member, company: &Company, id: IntentID, move_costs: Option<Costs>, action: OrderAction, agreed_in: Option<Url>, at_location: Option<SpatialThing>, available_quantity: Option<Measure>, due: Option<DateTime<Utc>>, effort_quantity: Option<Measure>, finished: Option<bool>, has_beginning: Option<DateTime<Utc>>, has_end: Option<DateTime<Utc>>, has_point_in_time: Option<DateTime<Utc>>, in_scope_of: Vec<AgentID>, name: Option<String>, note: Option<String>, provider: Option<AgentID>, receiver: Option<AgentID>, resource_conforms_to: Option<ResourceSpecID>, resource_inventoried_as: Option<ResourceID>, resource_quantity: Option<Measure>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateIntents)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::IntentCreate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    let company_agent_id = company.agent_id();
    if provider.is_none() && receiver.is_none() {
        // an intent must have a provider or receiver
        Err(Error::MissingFields(vec!["provider".into(), "receiver".into()]))?;
    }
    if (provider.is_some() && Some(&company_agent_id) != provider.as_ref()) || (receiver.is_some() && Some(&company_agent_id) != receiver.as_ref()) {
        // can't create an intent for a company you aren't a member of DUUUHHH
        Err(Error::InsufficientPrivileges)?;
    }
    let event_action = match action {
        OrderAction::DeliverService => vf::Action::DeliverService,
        OrderAction::Transfer => vf::Action::Transfer,
        OrderAction::TransferCustody => vf::Action::TransferCustody,
    };
    let model = Intent::builder()
        .id(id)
        .inner(
            vf::Intent::builder()
                .action(event_action)
                .agreed_in(agreed_in)
                .at_location(at_location)
                .available_quantity(available_quantity)
                .due(due)
                .effort_quantity(effort_quantity)
                .finished(finished)
                .has_beginning(has_beginning)
                .has_end(has_end)
                .has_point_in_time(has_point_in_time)
                .in_scope_of(in_scope_of)
                .name(name)
                .note(note)
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

/// Update an intent
pub fn update(caller: &User, member: &Member, company: &Company, mut subject: Intent, move_costs: Option<Option<Costs>>, action: Option<OrderAction>, agreed_in: Option<Option<Url>>, at_location: Option<Option<SpatialThing>>, available_quantity: Option<Option<Measure>>, due: Option<Option<DateTime<Utc>>>, effort_quantity: Option<Option<Measure>>, finished: Option<Option<bool>>, has_beginning: Option<Option<DateTime<Utc>>>, has_end: Option<Option<DateTime<Utc>>>, has_point_in_time: Option<Option<DateTime<Utc>>>, in_scope_of: Option<Vec<AgentID>>, name: Option<Option<String>>, note: Option<Option<String>>, provider: Option<Option<AgentID>>, receiver: Option<Option<AgentID>>, resource_conforms_to: Option<Option<ResourceSpecID>>, resource_inventoried_as: Option<Option<ResourceID>>, resource_quantity: Option<Option<Measure>>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateIntents)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::IntentUpdate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    let company_agent_id = company.agent_id();
    if provider == Some(None) && receiver == Some(None) {
        // an intent must have a provider or receiver
        Err(Error::MissingFields(vec!["provider".into(), "receiver".into()]))?;
    }
    if let Some(provider) = provider {
        if provider.is_some() && Some(&company_agent_id) != provider.as_ref() {
            // can't create an intent for a company you aren't a member of DUUUHHH
            Err(Error::InsufficientPrivileges)?;
        }
        subject.inner_mut().set_provider(provider);
    }
    if let Some(receiver) = receiver {
        if receiver.is_some() && Some(&company_agent_id) != receiver.as_ref() {
            // can't create an intent for a company you aren't a member of DUUUHHH
            Err(Error::InsufficientPrivileges)?;
        }
        subject.inner_mut().set_receiver(receiver);
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
    if let Some(available_quantity) = available_quantity {
        subject.inner_mut().set_available_quantity(available_quantity);
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
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(note) = note {
        subject.inner_mut().set_note(note);
    }
    // provider/receiver are set above in their respective perm check
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

/// Delete an intent
pub fn delete(caller: &User, member: &Member, company: &Company, mut subject: Intent, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateIntents)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::IntentDelete)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("intent".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::CompanyID,
        },
        util::{self, test::{self, *}},
    };
    use om2::Unit;

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = IntentID::create();
        let state = TestState::standard(vec![CompanyPermission::IntentCreate], &now);
        let costs = Costs::new_with_labor("widgetmaker", 42);

        let testfn_inner = |state: &TestState<Intent, Intent>, provider: Option<AgentID>, receiver: Option<AgentID>| {
            create(state.user(), state.member(), state.company(), id.clone(), Some(costs.clone()), OrderAction::Transfer, None, Some(state.loc().clone()), Some(Measure::new(10, Unit::One)), None, None, Some(false), Some(now.clone()), None, None, vec![state.company().agent_id()], Some("buy my widget".into()), Some("gee willickers i hope someone buys my widget".into()), provider, receiver, None, Some(ResourceID::new("widget1")), None, true, &now)
        };
        let testfn = |state: &TestState<Intent, Intent>| {
            testfn_inner(state, Some(state.company().agent_id()), None)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let intent = mods[0].clone().expect_op::<Intent>(Op::Create).unwrap();
        assert_eq!(intent.id(), &id);
        assert_eq!(intent.move_costs(), &Some(costs.clone()));
        assert_eq!(intent.inner().action(), &vf::Action::Transfer);
        assert_eq!(intent.inner().agreed_in(), &None);
        assert_eq!(intent.inner().at_location(), &Some(state.loc().clone()));
        assert_eq!(intent.inner().available_quantity(), &Some(Measure::new(10, Unit::One)));
        assert_eq!(intent.inner().due(), &None);
        assert_eq!(intent.inner().effort_quantity(), &None);
        assert_eq!(intent.inner().finished(), &Some(false));
        assert_eq!(intent.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(intent.inner().has_end(), &None);
        assert_eq!(intent.inner().has_point_in_time(), &None);
        assert_eq!(intent.inner().in_scope_of(), &vec![state.company().agent_id()]);
        assert_eq!(intent.inner().name(), &Some("buy my widget".into()));
        assert_eq!(intent.inner().note(), &Some("gee willickers i hope someone buys my widget".into()));
        assert_eq!(intent.inner().provider(), &Some(state.company().agent_id()));
        assert_eq!(intent.inner().receiver(), &None);
        assert_eq!(intent.inner().resource_conforms_to(), &None);
        assert_eq!(intent.inner().resource_inventoried_as(), &Some(ResourceID::new("widget1")));
        assert_eq!(intent.inner().resource_quantity(), &None);
        assert_eq!(intent.active(), &true);
        assert_eq!(intent.created(), &now);
        assert_eq!(intent.updated(), &now);
        assert_eq!(intent.deleted(), &None);

        let mut state2 = state.clone();
        state2.company_mut().set_id(CompanyID::new("bill's company"));
        let res = testfn_inner(&state2, Some(state.company().agent_id()), None);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        let res = testfn_inner(&state2, None, Some(state.company().agent_id()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let res = testfn_inner(&state, None, None);
        assert_eq!(res, Err(Error::MissingFields(vec!["provider".into(), "receiver".into()])));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = IntentID::create();
        let mut state = TestState::standard(vec![CompanyPermission::IntentCreate, CompanyPermission::IntentUpdate], &now);
        let costs1 = Costs::new_with_labor("widgetmaker", 42);
        let costs2 = Costs::new_with_labor("widgetmaker", 41);

        let mods = create(state.user(), state.member(), state.company(), id.clone(), Some(costs1.clone()), OrderAction::Transfer, None, Some(state.loc().clone()), Some(Measure::new(10, Unit::One)), None, None, Some(false), Some(now.clone()), None, None, vec![state.company().agent_id()], Some("buy my widget".into()), Some("gee willickers i hope someone buys my widget".into()), Some(state.company().agent_id()), None, None, Some(ResourceID::new("widget1")), None, true, &now).unwrap().into_vec();
        let intent = mods[0].clone().expect_op::<Intent>(Op::Create).unwrap();
        state.model = Some(intent);

        let now2 = util::time::now();
        let testfn_inner = |state: &TestState<Intent, Intent>, provider: Option<Option<AgentID>>, receiver: Option<Option<AgentID>>| {
            update(state.user(), state.member(), state.company(), state.model().clone(), Some(Some(costs2.clone())), None, None, Some(None), None, None, None, None, None, None, None, Some(vec![]), Some(Some("buy widget".into())), None, provider, receiver, None, None, None, Some(false), &now2)
        };
        let testfn = |state: &TestState<Intent, Intent>| {
            testfn_inner(state, None, None)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        let intent2 = mods[0].clone().expect_op::<Intent>(Op::Update).unwrap();

        assert_eq!(intent2.id(), state.model().id());
        assert_eq!(intent2.move_costs(), &Some(costs2.clone()));
        assert_eq!(intent2.inner().action(), state.model().inner().action());
        assert_eq!(intent2.inner().agreed_in(), intent2.inner().agreed_in());
        assert_eq!(intent2.inner().at_location(), &None);
        assert_eq!(intent2.inner().available_quantity(), state.model().inner().available_quantity());
        assert_eq!(intent2.inner().due(), state.model().inner().due());
        assert_eq!(intent2.inner().effort_quantity(), state.model().inner().effort_quantity());
        assert_eq!(intent2.inner().finished(), state.model().inner().finished());
        assert_eq!(intent2.inner().has_beginning(), state.model().inner().has_beginning());
        assert_eq!(intent2.inner().has_end(), state.model().inner().has_end());
        assert_eq!(intent2.inner().has_point_in_time(), state.model().inner().has_point_in_time());
        assert_eq!(intent2.inner().in_scope_of(), &vec![]);
        assert_eq!(intent2.inner().name(), &Some("buy widget".into()));
        assert_eq!(intent2.inner().note(), state.model().inner().note());
        assert_eq!(intent2.inner().provider(), state.model().inner().provider());
        assert_eq!(intent2.inner().receiver(), state.model().inner().receiver());
        assert_eq!(intent2.inner().resource_conforms_to(), state.model().inner().resource_conforms_to());
        assert_eq!(intent2.inner().resource_inventoried_as(), state.model().inner().resource_inventoried_as());
        assert_eq!(intent2.inner().resource_quantity(), state.model().inner().resource_quantity());
        assert_eq!(intent2.active(), &false);
        assert_eq!(intent2.created(), &now);
        assert_eq!(intent2.updated(), &now2);
        assert_eq!(intent2.deleted(), &None);

        let mut state2 = state.clone();
        state2.company_mut().set_id(CompanyID::new("bill's company"));
        let res = testfn_inner(&state2, Some(Some(CompanyID::new("widgetzzz plus").into())), None);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        let res = testfn_inner(&state2, None, Some(Some(CompanyID::new("widgetzzz plus").into())));
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let res = testfn_inner(&state, Some(None), Some(None));
        assert_eq!(res, Err(Error::MissingFields(vec!["provider".into(), "receiver".into()])));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = IntentID::create();
        let mut state = TestState::standard(vec![CompanyPermission::IntentCreate, CompanyPermission::IntentDelete], &now);
        let costs = Costs::new_with_labor("widgetmaker", 42);

        let mods = create(state.user(), state.member(), state.company(), id.clone(), Some(costs.clone()), OrderAction::Transfer, None, Some(state.loc().clone()), Some(Measure::new(10, Unit::One)), None, None, Some(false), Some(now.clone()), None, None, vec![state.company().agent_id()], Some("buy my widget".into()), Some("gee willickers i hope someone buys my widget".into()), Some(state.company().agent_id()), None, None, Some(ResourceID::new("widget1")), None, true, &now).unwrap().into_vec();
        let intent = mods[0].clone().expect_op::<Intent>(Op::Create).unwrap();
        state.model = Some(intent);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Intent, Intent>| {
            delete(state.user(), state.member(), state.company(), state.model().clone(), &now2)
        };
        test::standard_transaction_tests(&state, &testfn);
        test::double_deleted_tester(&state, "intent", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let intent2 = mods[0].clone().expect_op::<Intent>(Op::Delete).unwrap();
        assert_eq!(intent2.id(), state.model().id());
        assert_eq!(intent2.move_costs(), state.model().move_costs());
        assert_eq!(intent2.inner(), state.model().inner());
        assert_eq!(intent2.active(), state.model().active());
        assert_eq!(intent2.created(), state.model().created());
        assert_eq!(intent2.updated(), state.model().updated());
        assert_eq!(intent2.deleted(), &Some(now2));
    }
}

