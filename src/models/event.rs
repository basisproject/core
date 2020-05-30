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
        agreement::AgreementID,
        company::CompanyID,
        company_member::CompanyMember,
        lib::agent::AgentID,
        process::{Process, ProcessID},
        resource::{Resource, ResourceID},
        resource_spec::ResourceSpecID,
    },
    util::measure,
};
use derive_builder::Builder;
use om2::{Measure, NumericUnion, Unit};
use rust_decimal::prelude::*;
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
    /// The event model, which is the glue that moves costs between objects.
    pub struct Event {
        id: <<EventID>>,
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
#[derive(Debug, Default, Clone, Builder)]
#[builder(pattern = "owned", setter(into, strip_option), default)]
pub struct EventProcessState {
    /// The process this event is an input of
    input_of: Option<Process>,
    /// The process this event is an output of
    output_of: Option<Process>,
    /// The provider (if a member) performing an action (generally `Work`)
    provider: Option<CompanyMember>,
    /// The resource this event operates on (Consume/Produce/etc)
    resource: Option<Resource>,
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
        if state.output_of.is_some() && self.inner().output_of().as_ref() != state.output_of.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedOutputProcessID)?;
        }
        if state.input_of.is_some() && self.inner().input_of().as_ref() != state.input_of.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedInputProcessID)?;
        }
        if state.resource.is_some() && self.inner().resource_inventoried_as().as_ref() != state.resource.as_ref().map(|x| x.id()) {
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
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut resource = state.resource.clone().ok_or(Error::EventMissingResource)?;
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
                let get_hours = || -> Result<Decimal> {
                    match self.inner().effort_quantity() {
                        Some(Measure { has_unit: Unit::Hour, has_numerical_value: hours }) => {
                            let num_hours = NumericUnion::Decimal(Decimal::zero()).add(hours.clone())
                                .map_err(|e| Error::NumericUnionOpError(e))?;
                            match num_hours {
                                NumericUnion::Decimal(val) => Ok(val),
                                _ => Err(Error::NumericUnionOpError(format!("error converting to Decimal: {:?}", num_hours)))?,
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

    /// Return a list of fields that the `Event` object needs to have to
    /// successfully process itself.
    pub fn required_event_fields(&self) -> Vec<&'static str> {
        let mut fields = vec![];
        match self.inner().action() {
            Action::Accept => {}
            Action::Cite => {}
            Action::Consume => {
                fields.push("resource_quantity");
            }
            Action::DeliverService => {
                fields.push("move_costs");
            }
            Action::Dropoff => {}
            Action::Lower => {}
            Action::Modify => {
                fields.push("move_costs");
            }
            Action::Move => {}
            Action::Pickup => {}
            Action::Produce => {
                fields.push("resource_quantity");
            }
            Action::Raise => {}
            Action::Transfer => {
                fields.push("transfer_type");
                match self.transfer_type {
                    Some(TransferType::InternalCostTransfer) => {
                        fields.push("move_costs");
                    }
                    _ => {}
                }
            }
            Action::TransferAllRights => {}
            Action::TransferCustody => {}
            Action::Use => {}
            Action::Work => {
                fields.push("labor_type");
                match self.labor_type {
                    Some(LaborType::Wage) | Some(LaborType::WageAndHours) => {
                        fields.push("move_costs");
                    }
                    _ => {}
                }
            }
        }
        fields.sort();
        fields
    }

    /// Return a list of fields that the `EventProcessState` object needs to
    /// have to successfully process this event.
    pub fn required_state_fields(&self) -> Vec<&'static str> {
        let mut fields = vec![];
        match self.inner().action() {
            Action::Accept => {}
            Action::Cite => {}
            Action::Consume => {
                fields.push("resource");
                if self.move_costs().is_some() {
                    fields.push("input_of");
                }
            }
            Action::DeliverService => {
                fields.push("output_of");
                fields.push("input_of");
            }
            Action::Dropoff => {}
            Action::Lower => {}
            Action::Modify => {
                fields.push("output_of");
                fields.push("resource");
            }
            Action::Move => {}
            Action::Pickup => {}
            Action::Produce => {
                fields.push("resource");
                if self.move_costs().is_some() {
                    fields.push("output_of");
                }
            }
            Action::Raise => {}
            Action::Transfer => {
                match self.transfer_type {
                    Some(TransferType::InternalCostTransfer) => {
                        fields.push("input_of");
                        fields.push("output_of");
                    }
                    Some(TransferType::ResourceTransfer) => {
                        fields.push("resource");
                    }
                    _ => {}
                }
            }
            Action::TransferAllRights => {
                fields.push("resource");
            }
            Action::TransferCustody => {
                fields.push("resource");
            }
            Action::Use => {
                if self.move_costs().is_some() {
                    fields.push("resource");
                    fields.push("input_of");
                }
            }
            Action::Work => {
                fields.push("input_of");
                fields.push("provider");
            }
        }
        fields.sort();
        fields
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
    use rust_decimal_macros::*;
    use vf_rs::vf;

    fn state_with_fields(state: &EventProcessState, fields: Vec<&'static str>) -> EventProcessState {
        let mut builder = EventProcessState::builder();
        for field in fields {
            match field {
                "output_of" => {
                    if state.output_of.is_some() {
                        builder = builder.output_of(state.output_of.clone().unwrap());
                    }
                }
                "input_of" => {
                    if state.input_of.is_some() {
                        builder = builder.input_of(state.input_of.clone().unwrap());
                    }
                }
                "resource" => {
                    if state.resource.is_some() {
                        builder = builder.resource(state.resource.clone().unwrap());
                    }
                }
                "provider" => {
                    if state.provider.is_some() {
                        builder = builder.provider(state.provider.clone().unwrap());
                    }
                }
                _ => panic!("unknown field {}", field)
            }
        }
        builder.build().unwrap()
    }

    /// Given a set of values, find all combinations of those values as present
    /// or absent in a vec.
    fn generate_combinations<T: Clone>(vals: &Vec<T>) -> Vec<Vec<T>> {
        // we use binary counting here to accomplish the combination finding.
        // this might seem obtuse, but i have 4 hours of sleep and this seems
        // like the quickest way to get it done.
        let mut out = vec![];
        let combos = 2u32.pow(vals.len() as u32);
        for i in 0..combos {
            let mut combo = vec![];
            let mut bits = i;
            for idx in 0..vals.len() {
                if bits & 1 > 0 {
                    combo.push(vals[idx].clone());
                }
                bits = bits >> 1;
            }
            out.push(combo);
        }
        out
    }

    /// Takes a valid event/state and tries a bunch of different possible
    /// combinations of state and event fields being None, trying to find
    /// permutations that break expectations.
    ///
    /// In other words, we test actual results against the methods
    /// `required_event_fields` and `required_state_fields`.
    fn fuzz_state(event: Event, state: EventProcessState, now: &DateTime<Utc>) {
        let all_field_combos = generate_combinations(&vec!["input_of", "output_of", "provider", "resource"]);
        let all_event_combos = generate_combinations(&vec!["move_costs", "transfer_type", "labor_type", "resource_quantity"]);
        for evfields in &all_event_combos {
            let mut event2 = event.clone();
            event2.set_move_costs(None);
            event2.set_transfer_type(None);
            event2.set_labor_type(None);
            event2.inner_mut().set_resource_quantity(None);
            for evfield in evfields {
                match *evfield {
                    "move_costs" => { event2.set_move_costs(event.move_costs().clone()); }
                    "transfer_type" => { event2.set_transfer_type(event.transfer_type().clone()); }
                    "labor_type" => { event2.set_labor_type(event.labor_type().clone()); }
                    "resource_quantity" => { event2.inner_mut().set_resource_quantity(event.inner().resource_quantity().clone()); }
                    _ => {}
                }
            }
            let must_event_fields = event2.required_event_fields();
            let must_state_fields = event2.required_state_fields();
            for fieldset in &all_field_combos {
                let has_event_fields = must_event_fields.iter().fold(true, |acc, x| acc && evfields.contains(x));
                let has_state_fields = must_state_fields.iter().fold(true, |acc, x| acc && fieldset.contains(x));
                let should_pass =has_event_fields && has_state_fields;
                let state = state_with_fields(&state, fieldset.clone());
                match event2.process(state, now) {
                    Ok(_) => {
                        if !should_pass {
                            panic!("event state fuzzer: passed but should have failed: {:?} {:?}", evfields, fieldset);
                        }
                    }
                    Err(e) => {
                        if should_pass {
                            panic!("event state fuzzer: failed but should have passed: {:?} {:?} {}", evfields, fieldset, e);
                        }
                    }
                }
            }
        }
    }

    /// Creates an EventProcessState object with some easy defaults. Saves
    /// having to copy and paste this stuff over and over.
    fn make_state(company_id: &CompanyID, provider_is_company: bool, now: &DateTime<Utc>) -> EventProcessState {
        let mut builder = EventProcessState::builder();
        let process_from = Process::builder()
            .id("1111")
            .inner(vf::Process::builder().name("Make widgets").build().unwrap())
            .costs(Costs::new_with_labor("machinist", dec!(100.0)))
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
            .costs(Costs::new_with_labor("machinist", dec!(34.91)))
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        if !provider_is_company {
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
                .compensation(Compensation::new_hourly(dec!(0.0), "12345"))
                .process_spec_id("1234444")
                .created(now.clone())
                .updated(now.clone())
                .build().unwrap();
            builder = builder.provider(member);
        }
        builder
            .output_of(process_from)
            .input_of(process_to)
            .resource(resource)
            .build().unwrap()
    }

    /// Create a test event. Change it how you want after the fact.
    fn make_event(action: vf::Action, company_id: &CompanyID, state: &EventProcessState, now: &DateTime<Utc>) -> Event {
        Event::builder()
            .id(EventID::create())
            .inner(
                vf::EconomicEvent::builder()
                    .action(action)
                    .has_beginning(now.clone())
                    .input_of(state.input_of.as_ref().unwrap().id().clone())
                    .output_of(state.output_of.as_ref().unwrap().id().clone())
                    .provider(company_id.clone())
                    .receiver(company_id.clone())
                    .resource_inventoried_as(state.resource.as_ref().unwrap().id().clone())
                    .resource_quantity(Measure::new(NumericUnion::Decimal(dec!(6)), Unit::One))
                    .build().unwrap()
            )
            .move_costs(Costs::new_with_labor("machinist", dec!(30.0)))
            .labor_type(None)
            .transfer_type(None)
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap()
    }

    /// Check that a given process has the give costs, but also that its pre-
    /// event-process is the exact same as the new version (besides the process
    /// costs).
    fn check_process_mods(fields_allowed_to_change: Vec<&'static str>, process_new: &Process, process_previous: &Process) {
        let mod_process = |mut process: Process| {
            for field in &fields_allowed_to_change {
                match *field {
                    "costs" => { process.set_costs(Costs::new()); }
                    _ => {}
                }
            }
            process
        };
        let process = mod_process(process_new.clone());
        let process_prev = mod_process(process_previous.clone());
        let proc_ser = serde_json::to_string(&process).unwrap();
        let proc2_ser = serde_json::to_string(&process_prev).unwrap();
        // only process.costs should be changed
        assert_eq!(proc_ser, proc2_ser);
    }

    /// Check that a resource, compared against its previous version, has only
    /// changed its costs, custody, owner, and quantity. All other mods should
    /// be marked as test failures (or this expectation should be changed and an
    /// exception added in this function).
    fn check_resource_mods(fields_allowed_to_change: Vec<&'static str>, resource_new: &Resource, resource_previous: &Resource) {
        let mod_resource = |mut resource: Resource| {
            for field in &fields_allowed_to_change {
                match *field {
                    "costs" => { resource.set_costs(Costs::new()); }
                    "in_custody_of" => { resource.set_in_custody_of(CompanyID::new("<testlol>").into()); }
                    "accounting_quantity" => { resource.inner_mut().set_accounting_quantity(None); }
                    "primary_accountable" => { resource.inner_mut().set_primary_accountable(None); }
                    _ => {}
                }
            }
            resource
        };
        let resource = mod_resource(resource_new.clone());
        let resource_prev = mod_resource(resource_previous.clone());
        let res_ser = serde_json::to_string(&resource).unwrap();
        let res2_ser = serde_json::to_string(&resource_prev).unwrap();
        // only resource.costs/custody/quantity/accountable should be changed
        assert_eq!(res_ser, res2_ser);
    }

    // -------------------------------------------------------------------------

    #[test]
    fn consume() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let event = make_event(vf::Action::Consume, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", dec!(30.0)));
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.inner().accounting_quantity().clone().unwrap(), Measure::new(NumericUnion::Integer(4), Unit::One));
                assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(4.91)));
                check_resource_mods(vec!["costs", "accounting_quantity"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }

        let mut event = make_event(vf::Action::Consume, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn deliver_service() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let event = make_event(vf::Action::DeliverService, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 70));
                check_process_mods(vec!["costs"], process, state.output_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 30));
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }

        let mut event = make_event(vf::Action::DeliverService, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn modify() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let event = make_event(vf::Action::Modify, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 70));
                check_process_mods(vec!["costs"], process, state.output_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(64.91)));
                check_resource_mods(vec!["costs", "accounting_quantity"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }

        let mut event = make_event(vf::Action::Modify, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn produce() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let mut event = make_event(vf::Action::Produce, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(42.0))));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 58));
                check_process_mods(vec!["costs"], process, state.output_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.inner().accounting_quantity().clone().unwrap(), Measure::new(NumericUnion::Integer(15), Unit::One));
                assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
                assert_eq!(resource.in_custody_of(), &company_id.clone().into());
                assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(76.91)));
                check_resource_mods(vec!["costs", "in_custody_of", "accounting_quantity", "primary_accountable"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }

        let mut event = make_event(vf::Action::Produce, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn transfer_internal() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let mut event = make_event(vf::Action::Transfer, &company_id, &state, &now);
        event.set_transfer_type(Some(TransferType::InternalCostTransfer));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", 59)));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 41));
                check_process_mods(vec!["costs"], process, state.output_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 59));
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }

        let mut event = make_event(vf::Action::Produce, &company_id, &state, &now);
        event.set_transfer_type(Some(TransferType::InternalCostTransfer));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn transfer_resource() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let mut event = make_event(vf::Action::Transfer, &company_id, &state, &now);
        event.set_transfer_type(Some(TransferType::ResourceTransfer));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 1);
        match &mods[0] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
                assert_eq!(resource.in_custody_of(), &company_id.clone().into());
                check_resource_mods(vec!["in_custody_of", "primary_accountable"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn transfer_all_rights() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let event = make_event(vf::Action::TransferAllRights, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 1);
        match &mods[0] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
                check_resource_mods(vec!["primary_accountable"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn transfer_custody() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let event = make_event(vf::Action::TransferCustody, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 1);
        match &mods[0] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.in_custody_of().clone(), company_id.clone().into());
                check_resource_mods(vec!["in_custody_of"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn r#use() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, true, &now);

        let event = make_event(vf::Action::Use, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 2);
        match &mods[0] {
            Saver::ModifyResource(resource) => {
                assert_eq!(resource.in_custody_of().clone(), company_id.clone().into());
                assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(4.91)));
                check_resource_mods(vec!["costs", "in_custody_of"], resource, state.resource.as_ref().unwrap());
            }
            _ => panic!("unexpected result"),
        }
        match &mods[1] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 30));
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }

        let mut event = make_event(vf::Action::Use, &company_id, &state, &now);
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        match event.process(state, &now) {
            Err(Error::NegativeCosts) => {}
            _ => panic!("should have overflowed move_costs"),
        }
    }

    #[test]
    fn work_wage() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, false, &now);

        let mut event = make_event(vf::Action::Work, &company_id, &state, &now);
        event.set_labor_type(Some(LaborType::Wage));
        event.inner_mut().set_provider(state.provider.as_ref().unwrap().id().clone().into());
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 1);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 30));
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn work_hours() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, false, &now);

        let mut event = make_event(vf::Action::Work, &company_id, &state, &now);
        event.set_labor_type(Some(LaborType::Hours));
        event.set_move_costs(None);
        event.inner_mut().set_provider(state.provider.as_ref().unwrap().id().clone().into());
        event.inner_mut().set_effort_quantity(Some(Measure::new(NumericUnion::Integer(5), Unit::Hour)));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 1);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                assert_eq!(process.costs(), &Costs::new_with_labor_hours("CEO", 5));
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn work_wage_and_hours() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, false, &now);

        let mut event = make_event(vf::Action::Work, &company_id, &state, &now);
        event.set_labor_type(Some(LaborType::WageAndHours));
        event.set_move_costs(Some(Costs::new_with_labor("CEO", 69)));
        event.inner_mut().set_provider(state.provider.as_ref().unwrap().id().clone().into());
        event.inner_mut().set_effort_quantity(Some(Measure::new(NumericUnion::Integer(12), Unit::Hour)));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.modifications();
        assert_eq!(mods.len(), 1);
        match &mods[0] {
            Saver::ModifyProcess(process) => {
                let mut costs = Costs::new();
                costs.track_labor("CEO", 69);
                costs.track_labor_hours("CEO", 12);
                assert_eq!(process.costs(), &costs);
                check_process_mods(vec!["costs"], process, state.input_of.as_ref().unwrap())
            }
            _ => panic!("unexpected result"),
        }
    }
}

