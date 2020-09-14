//! Services are processes that require labor but don't create resources.
//!
//! For instance, delivering a package, providing healthcare, or providing legal
//! advice are all services.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        agreement::Agreement,
        event::{Event, EventID, EventProcessState},
        company::{Company, Permission as CompanyPermission},
        member::Member,
        lib::{
            agent::Agent,
            basis_model::Model,
        },
        process::Process,
        user::User,
    },
};
use url::Url;
use vf_rs::vf;

/// Provide a service to another agent, moving costs along the way.
pub fn deliver_service(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, process_from: Process, process_to: Process, move_costs: Costs, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company_from.id(), CompanyPermission::DeliverService)?;
    if !company_from.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !company_to.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !agreement.has_participant(&company_from.agent_id()) || !agreement.has_participant(&company_from.agent_id()) {
        // can't create an event for an agreement you are not party to
        Err(Error::InsufficientPrivileges)?;
    }

    let process_from_id = process_from.id().clone();
    let process_to_id = process_to.id().clone();

    let state = EventProcessState::builder()
        .output_of(process_from)
        .input_of(process_to)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::DeliverService)
                .agreed_in(agreed_in)
                .has_point_in_time(now.clone())
                .input_of(Some(process_to_id))
                .note(note)
                .provider(company_from.id().clone())
                .realization_of(Some(agreement.id().clone()))
                .receiver(company_to.id().clone())
                .output_of(Some(process_from_id))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(move_costs))
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;

    let evmods = event.process(state, now)?.into_vec();
    let mut mods = Modifications::new();
    mods.push(Op::Create, event);
    for evmod in evmods {
        mods.push_raw(evmod);
    }
    Ok(mods)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            agreement::AgreementID,
            company::CompanyID,
            event::{EventID, EventError},
            lib::agent::Agent,
            occupation::OccupationID,
            process::{Process, ProcessID},
        },
        util::{self, test::{self, *}},
    };
    use rust_decimal_macros::*;

    #[test]
    fn can_deliver_service() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::DeliverService], &now);
        let company_from = state.company().clone();
        let company_to = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 1234", "gotta make some planks", &now);
        let agreed_in: Url = "https://legalzoom.com/my-dad-is-suing-your-dad-the-agreement".parse().unwrap();
        let occupation_id = OccupationID::new("lawyer");
        let process_from = make_process(&ProcessID::create(), company_from.id(), "various lawyerings", &Costs::new_with_labor(occupation_id.clone(), dec!(177.25)), &now);
        let process_to = make_process(&ProcessID::create(), company_to.id(), "employee legal agreement drafting", &Costs::new_with_labor(occupation_id.clone(), dec!(804)), &now);
        let costs_to_move = process_from.costs().clone() * dec!(0.777777777);
        state.model = Some(process_from);
        state.model2 = Some(process_to);

        let testfn_inner = |state: &TestState<Process, Process>, company_from: &Company, company_to: &Company, agreement: &Agreement| {
            deliver_service(state.user(), state.member(), company_from, company_to, agreement, id.clone(), state.model().clone(), state.model2().clone(), costs_to_move.clone(), Some(agreed_in.clone()), Some("making planks lol".into()), &now)
        };
        let testfn_from = |state: &TestState<Process, Process>| {
            testfn_inner(state, state.company(), &company_to, &agreement)
        };
        let testfn_to = |state: &TestState<Process, Process>| {
            testfn_inner(state, &company_from, state.company(), &agreement)
        };
        test::standard_transaction_tests(&state, &testfn_from);

        let mods = testfn_from(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let process_from2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        let process_to2 = mods[2].clone().expect_op::<Process>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("making planks lol".into()));
        assert_eq!(event.inner().output_of(), &Some(state.model().id().clone()));
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &None);
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(process_from2.id(), state.model().id());
        assert_eq!(process_from2.company_id(), company_from.id());
        assert_eq!(process_from2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(process_to2.id(), state.model2().id());
        assert_eq!(process_to2.company_id(), company_to.id());
        assert_eq!(process_to2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // can't move costs from a process you don't own
        let mut state2 = state.clone();
        state2.model_mut().set_company_id(CompanyID::new("zing").into());
        let res = testfn_from(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // can't move costs into a process company_to doesnt own
        let mut state3 = state.clone();
        state3.model2_mut().set_company_id(CompanyID::new("zing").into());
        let res = testfn_from(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company_to.agent_id()]);
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state5 = state.clone();
        state5.company = Some(company_to.clone());
        test::deleted_company_tester(&state5, &testfn_to);
    }
}

