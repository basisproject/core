//! The `Event` model is *the* core piece of the system that really ties the
//! room together. Processing events is what allows moving `Costs` through the
//! system.
//!
//! It's important to note that in many REA systems, only events are recorded
//! and all the other objects are willed into existence from there. For our
//! purposes, events will operate on and mutate *known* objects. The reason is
//! that we want to build a picture of ongoing state as it happens to avoid the
//! need to walk back our event tree on every operation or do graph traversal on
//! our economic network. Most REA systems, to my understanding, have a
//! recording/observation process, and afterwards an analysis process. Because
//! the economic graph is so vast and complex, we don't have the luxury of doing
//! and analysis process: observation and analysis must happen at the same time!

use chrono::{DateTime, Utc};
use crate::{
    costs::{Costs, CostMover},
    error::{Error, Result},
    models::{
        agent::AgentID,
        agreement::AgreementID,
        company::CompanyID,
        company_member::CompanyMember,
        process::{Process, ProcessID},
        resource::{Resource, ResourceID},
        resource_spec::ResourceSpecID,
    },
    util::measure,
};
use derive_builder::Builder;
use om2::{Measure, NumericUnion, Unit};
use serde::{Serialize, Deserialize};
use std::convert::TryInto;
use vf_rs::vf::{self, Action};

/// When creating a `work` event, we need to know if that event corresponds to
/// wages, labor hours, or both. This lets our event processor know.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum LaborType {
    /// This event signifies a wage cost.
    ///
    /// We would use this if we want to record a wage cost (ie, `Costs.labor`)
    /// but not a labor hour cost. For instance, if a worker is on salary but
    /// they also record their hours (ie via clocking in/out) separately, they
    /// would create two separate work events with `LaborType::Wage` and
    /// `LaborType::Hours` to represent both.
    Wage,
    /// This event signifies a labor hour cost
    ///
    /// We would use this if we want to record a labor our cost (ie
    /// `Costs.labor_hours`) but not a labor age cost.
    Hours,
    /// This event should be counted toward both wages and hours.
    ///
    /// This can be used for an hourly worker that records hours worked and
    /// hourly wage at the same time, or it can be used for a salaried worker
    /// that doesn't track their hours at all and wants to create automatic
    /// entries for their labor hours that also signify their labor wage.
    WageAndHours,
}

/// When creating a `transfer` event, we need to know if that event transfers
/// costs internally between processes or if it transfers resources between
/// different agents.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TransferType {
    /// This is an internal cost transfer
    InternalCostTransfer,
    /// This transfers a resource to another company (the original intended
    /// purpose of the vf::Action::Transfer type)
    ResourceTransfer,
}

basis_model! {
    /// Events really tie the room together.
    ///
    /// They are the piece that represent the flow of resources and costs both
    /// through and between economic entities.
    pub struct Event {
        /// The event's core VF type
        inner: vf::EconomicEvent<AgreementID, AgentID, ProcessID, AgentID, AgreementID, (), ResourceSpecID, ResourceID, EventID>,
        /// If this event is an output of a process, move some fixed amount of
        /// the process' costs and transfer them either into another Process or
        /// into a Resource
        move_costs: Option<Costs>,
        /// When recording a work event, this lets us know if it should apply to
        /// the `labor` and/or `labor_hours` buckets of our Costs object.
        labor_type: Option<LaborType>,
        /// The type of transfer (if using `Action::Transfer`). Can be internal
        /// (exclusively for moving costs between processes) or external (giving
        /// all rights and custody of a resource to another company).
        ///
        /// We could just measure whether or not the event has a ResourceID, but
        /// being explicit is probably a better choice, especially when going
        /// outside of the intended purpose of the `Transfer` event. It also
        /// makes things more clear when creating the event whether it should be
        /// allowed or not.
        transfer_type: Option<TransferType>,
    }
    EventID
    EventBuilder
}

/// A container type for objects we want to save while processing an event. For
/// our purposes, saving means either updating or creating.
#[derive(Debug)]
pub enum Saver {
    CreateEvent(Event),
    CreateResource(Resource),
    ModifyProcess(Process),
    ModifyResource(Resource),
}

/// A set of data that the event processor needs to do its thing. Generally this
/// acts as a container for objects that the event only has *references* to and
/// wouldn't otherwise be able to access.
#[derive(Debug, Default, Builder)]
#[builder(pattern = "owned", setter(into, strip_option), default)]
pub struct EventProcessState {
    /// The process this event is an output of
    output_of: Option<Process>,
    /// The process this event is an input of
    input_of: Option<Process>,
    /// The resource this event operates on (Consume/Produce/etc)
    resource: Option<Resource>,
    /// The provider (if a member) performing an action (generally `Work`)
    provider: Option<CompanyMember>,
}

impl EventProcessState {
    /// Create a state builder
    pub fn builder() -> EventProcessStateBuilder {
        EventProcessStateBuilder::default()
    }
}

/// A standard result set our event processor can return, including the items
/// we wish to be saved/updated.
#[derive(Debug)]
pub struct EventProcessResult {
    /// The ID of the current event we're processing the result for
    event_id: EventID,
    /// The time we're processing the event
    process_time: DateTime<Utc>,
    /// The items we're saving/updating as a result of processing this event
    modifications: Vec<Saver>,
}

impl EventProcessResult {
    /// Create a new result
    pub fn new(event_id: &EventID, process_time: &DateTime<Utc>) -> Self {
        Self {
            event_id: event_id.clone(),
            process_time: process_time.clone(),
            modifications: Default::default(),
        }
    }

    /// Consume the result and return the modification list
    pub fn modifications(self) -> Vec<Saver> {
        self.modifications
    }

    /// Push an event to create into the result set
    pub fn create_event(&mut self, mut event: Event) {
        event.inner_mut().set_triggered_by(Some(self.event_id.clone()));
        event.set_created(self.process_time.clone());
        event.set_updated(self.process_time.clone());
        self.modifications.push(Saver::CreateEvent(event));
    }

    /// Push a resource to create into the result set
    pub fn create_resource(&mut self, mut resource: Resource) {
        resource.set_created(self.process_time.clone());
        resource.set_updated(self.process_time.clone());
        self.modifications.push(Saver::CreateResource(resource));
    }

    /// Push a process to modify into the result set
    pub fn modify_process(&mut self, mut process: Process) {
        process.set_updated(self.process_time.clone());
        self.modifications.push(Saver::ModifyProcess(process));
    }

    /// Push a resource to modify into the result set
    pub fn modify_resource(&mut self, mut resource: Resource) {
        resource.set_updated(self.process_time.clone());
        self.modifications.push(Saver::ModifyResource(resource));
    }
}

impl Event {
    /// Our event processor. This method is responsible for mutating the objects
    /// the event operates on (like subtracting costs from one resource/process
    /// and adding them to another resource/process).
    ///
    /// This method returns an array of events that should be created as a
    /// result of processing this event.
    ///
    /// Note that this method *assumes the event is legitimate* and doesn't do
    /// any kind of checking as to whether or not the event should exist. That
    /// should happen when the event is created.
    pub fn process(&self, state: EventProcessState, now: &DateTime<Utc>) -> Result<EventProcessResult> {
        // some low-hanging fruit error checking. hackers HATE him!!1
        if self.inner().output_of().as_ref() != state.output_of.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedOutputProcessID)?;
        }
        if self.inner().input_of().as_ref() != state.input_of.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedInputProcessID)?;
        }
        if self.inner().resource_inventoried_as().as_ref() != state.resource.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedResourceID)?;
        }
        if let Some(provider) = state.provider.as_ref() {
            if self.inner().provider() != &provider.id().clone().into() {
                Err(Error::EventMismatchedProviderID)?;
            }
        }

        // create our result set
        let mut res = EventProcessResult::new(&self.id, now);

        match self.inner().action() {
            Action::Accept => {}
            Action::Cite => {}
            Action::Consume => {
                let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                let mut resource_measure = resource.inner().accounting_quantity().clone()
                    .ok_or(Error::EventMissingResourceQuantity)?;
                let event_measure = self.inner().resource_quantity().clone()
                    .ok_or(Error::EventMissingResourceQuantity)?;

                let mut should_save_resource = false;
                if measure::dec_measure(&mut resource_measure, &event_measure)? {
                    resource.inner_mut().set_accounting_quantity(Some(resource_measure));
                    should_save_resource = true;
                }
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut input_process = state.input_of.clone().ok_or(Error::EventMissingInputProcess)?;
                    if resource.move_costs_to(&mut input_process, &move_costs)? {
                        res.modify_process(input_process);
                        should_save_resource = true;
                    }
                }
                if should_save_resource {
                    res.modify_resource(resource);
                }
            }
            Action::DeliverService => {
                let mut output_process = state.output_of.clone().ok_or(Error::EventMissingOutputProcess)?;
                let mut input_process = state.input_of.clone().ok_or(Error::EventMissingInputProcess)?;
                let move_costs = self.move_costs.clone().ok_or(Error::EventMissingCosts)?;
                if output_process.move_costs_to(&mut input_process, &move_costs)? {
                    res.modify_process(output_process);
                    res.modify_process(input_process);
                }
            }
            Action::Dropoff => {}
            Action::Lower => {}
            Action::Modify => {
                let mut output_process = state.output_of.clone().ok_or(Error::EventMissingOutputProcess)?;
                let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                let move_costs = self.move_costs.clone().ok_or(Error::EventMissingCosts)?;
                if output_process.move_costs_to(&mut resource, &move_costs)? {
                    res.modify_process(output_process);
                    res.modify_resource(resource);
                }
            }
            Action::Move => {}
            Action::Pickup => {}
            Action::Produce => {
                let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                // grab the resource's current accounting quantity and
                // add the event's quantity to it. if the resource
                // doesn't have a quantity, then just default to using
                // the event's quantity.
                let event_measure = self.inner().resource_quantity().clone()
                    .ok_or(Error::EventMissingResourceQuantity)?;
                let new_resource_measure = match resource.inner().accounting_quantity() {
                    Some(resource_measure) => {
                        let mut resource_measure = resource_measure.clone();
                        measure::inc_measure(&mut resource_measure, &event_measure)?;
                        resource_measure
                    }
                    None => event_measure,
                };
                let company_id = self.inner().provider().clone();
                resource.inner_mut().set_accounting_quantity(Some(new_resource_measure));
                resource.inner_mut().set_primary_accountable(Some(company_id.clone().into()));
                resource.set_in_custody_of(company_id.into());
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut output_process = state.output_of.clone().ok_or(Error::EventMissingOutputProcess)?;
                    if output_process.move_costs_to(&mut resource, move_costs)? {
                        res.modify_process(output_process);
                    }
                }
                res.modify_resource(resource);
            }
            Action::Raise => {}
            Action::Transfer => {
                // transfer is interesting because we can use it to move a
                // particular resource between entities, but we can also use it
                // to move costs internally between processes.
                match self.transfer_type.clone().ok_or(Error::EventMissingTransferType)? {
                    TransferType::InternalCostTransfer => {
                        let mut output_process = state.output_of.clone().ok_or(Error::EventMissingOutputProcess)?;
                        let mut input_process = state.input_of.clone().ok_or(Error::EventMissingInputProcess)?;
                        let move_costs = self.move_costs.clone().ok_or(Error::EventMissingCosts)?;
                        if output_process.move_costs_to(&mut input_process, &move_costs)? {
                            res.modify_process(output_process);
                            res.modify_process(input_process);
                        }
                    }
                    TransferType::ResourceTransfer => {
                        let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                        let company_id: CompanyID = self.inner().receiver().clone().try_into()?;
                        resource.inner_mut().set_primary_accountable(Some(company_id.clone().into()));
                        resource.set_in_custody_of(company_id.into());
                        res.modify_resource(resource);
                    }
                }
            }
            Action::TransferAllRights => {
                let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                let company_id: CompanyID = self.inner().receiver().clone().try_into()?;
                resource.inner_mut().set_primary_accountable(Some(company_id.into()));
                res.modify_resource(resource);
            }
            Action::TransferCustody => {
                let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                let company_id: CompanyID = self.inner().receiver().clone().try_into()?;
                resource.set_in_custody_of(company_id.into());
                res.modify_resource(resource);
            }
            Action::Use => {
                let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut input_process = state.input_of.clone().ok_or(Error::EventMissingInputProcess)?;
                    if resource.move_costs_to(&mut input_process, &move_costs)? {
                        res.modify_resource(resource);
                        res.modify_process(input_process);
                    }
                }
            }
            Action::Work => {
                let mut input_process = state.input_of.clone().ok_or(Error::EventMissingInputProcess)?;
                let member = state.provider.clone().ok_or(Error::EventMissingProvider)?;
                let labor_type = self.labor_type.clone().ok_or(Error::EventMissingLaborType)?;
                let occupation_id = member.inner().relationship().clone();
                let get_hours = || -> Result<f64> {
                    match self.inner().effort_quantity() {
                        Some(Measure { has_unit: Unit::Hour, has_numerical_value: hours }) => {
                            let num_hours = NumericUnion::Double(0.0).add(hours.clone())
                                .map_err(|e| Error::NumericUnionOpError(e))?;
                            match num_hours {
                                NumericUnion::Double(val) => Ok(val),
                                _ => Err(Error::NumericUnionOpError(format!("error converting to f64: {:?}", num_hours)))?,
                            }
                        }
                        _ => Err(Error::EventLaborMustBeHours)?,
                    }
                };
                match labor_type {
                    // for wage costs, we effectively use `Event.move_costs` 
                    LaborType::Wage => {
                        let costs = self.move_costs.clone().ok_or(Error::EventMissingCosts)?;
                        input_process.receive_costs(&costs)?;
                    }
                    LaborType::Hours => {
                        let hours = get_hours()?;
                        input_process.receive_costs(&Costs::new_with_labor_hours(occupation_id, hours))?;
                    }
                    LaborType::WageAndHours => {
                        let hours = get_hours()?;
                        let mut costs = self.move_costs.clone().ok_or(Error::EventMissingCosts)?;
                        costs.track_labor_hours(occupation_id, hours);
                        input_process.receive_costs(&costs)?;
                    }
                }
                res.modify_process(input_process);
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        costs::Costs,
        models::{
            company::{CompanyID, Role},
            company_member::{Compensation, CompanyMember},
            process::Process,
            resource::Resource,
            user::UserID,
        },
        util,
    };
    use om2::{Measure, NumericUnion, Unit};
    use rust_decimal::prelude::*;
    use vf_rs::vf;

    fn test_init(company_id: &CompanyID, now: &DateTime<Utc>) -> (Process, Process, Resource, CompanyMember) {
        let process_from = Process::builder()
            .id("1111")
            .inner(vf::Process::builder().name("Make widgets").build().unwrap())
            .costs(Costs::new_with_labor("machinist", 100.0))
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let process_to = Process::builder()
            .id("1112")
            .inner(vf::Process::builder().name("Check widgets").build().unwrap())
            .costs(Costs::default())
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let resource = Resource::builder()
            .id("4444")
            .inner(
                vf::EconomicResource::builder()
                    .accounting_quantity(Measure::new(NumericUnion::Integer(10), Unit::One))
                    .conforms_to("3330")
                    .build().unwrap()
            )
            .in_custody_of(company_id.clone())
            .costs(Costs::default())
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let member = CompanyMember::builder()
            .id("5555")
            .inner(
                vf::AgentRelationship::builder()
                    .subject(UserID::from("jerry"))
                    .object(CompanyID::from("jerry's widgets ultd"))
                    .relationship("CEO")
                    .build().unwrap()
            )
            .active(true)
            .roles(vec![Role::MemberAdmin])
            .compensation(Compensation::new_hourly(0.0, "12345"))
            .process_spec_id("1234444")
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        (process_from, process_to, resource, member)
    }

    #[test]
    fn consume() {
    }

    #[test]
    fn deliver_service() {
    }

    #[test]
    fn produce() {
        let now = util::time::now();
        let company_id: CompanyID = "6969".into();
        let (process_from, process_to, resource, _) = test_init(&company_id, &now);

        let event = Event::builder()
            .id(EventID::create())
            .inner(
                vf::EconomicEvent::builder()
                    .action(vf::Action::Produce)
                    .has_beginning(util::time::now())
                    .input_of(process_to.id().clone())
                    .output_of(process_from.id().clone())
                    .provider(company_id.clone())
                    .receiver(company_id.clone())
                    .resource_inventoried_as(resource.id().clone())
                    .resource_quantity(Measure::new(NumericUnion::Decimal(Decimal::new(5, 0)), Unit::One))
                    .build().unwrap()
            )
            .move_costs(Costs::new_with_labor("machinist", 42.0))
            .labor_type(None)
            .transfer_type(None)
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let state = EventProcessState::builder()
            .output_of(process_from.clone())
            .input_of(process_to.clone())
            .resource(resource.clone())
            .build().unwrap();
        let res = event.process(state, &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                let mut process = process.clone();
                assert_eq!(process.costs().clone(), Costs::new_with_labor("machinist", 100.0) - Costs::new_with_labor("machinist", 42.0));
                process.set_costs(Costs::new());
                let mut process_from2 = process_from.clone();
                process_from2.set_costs(Costs::new());
                let proc_ser = serde_json::to_string_pretty(&process).unwrap();
                let proc2_ser = serde_json::to_string_pretty(&process_from2).unwrap();
                // only process.costs should be changed
                assert_eq!(proc_ser, proc2_ser);
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.inner().accounting_quantity().clone().unwrap(), Measure::new(NumericUnion::Integer(15), Unit::One));
                assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
                assert_eq!(resource.in_custody_of(), &company_id.clone().into());
                assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", 42.0));
                println!("{:?}", resource);
            }
            _ => panic!("unexpected result"),
        }

        let event = Event::builder()
            .id(EventID::create())
            .inner(
                vf::EconomicEvent::builder()
                    .action(vf::Action::Produce)
                    .has_beginning(util::time::now())
                    .input_of(process_to.id().clone())
                    .output_of(process_from.id().clone())
                    .provider(company_id.clone())
                    .receiver(company_id.clone())
                    .resource_inventoried_as(resource.id().clone())
                    .resource_quantity(Measure::new(NumericUnion::Decimal(Decimal::new(5, 0)), Unit::One))
                    .build().unwrap()
            )
            .move_costs(Costs::new_with_labor("machinist", 100.000001))
            .labor_type(None)
            .transfer_type(None)
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let state = EventProcessState::builder()
            .output_of(process_from.clone())
            .input_of(process_to.clone())
            .resource(resource.clone())
            .build().unwrap();
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn transfer() {
    }

    #[test]
    fn transfer_all_rights() {
    }

    #[test]
    fn transfer_custody() {
    }

    #[test]
    fn r#use() {
    }

    #[test]
    fn work() {
    }
}

