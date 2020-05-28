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
        company::CompanyID,
        company_member::CompanyMember,
        process::{Process, ProcessID},
        resource::{Resource, ResourceID},
        resource_spec::ResourceSpecID,
    },
    util::measure,
};
use serde::{Serialize, Deserialize};
use std::convert::TryInto;
use vf_rs::vf::{self, Action};

/// When creating a `work` event, we need to know if that event corresponds to
/// wages, labor hours, or both. This lets our event processor know.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum LaborType {
    /// This event signifies a wage cost
    Wage,
    /// This event signifies a labor hour cost
    Hours,
    /// This event should be counted toward both wages and hours
    WageAndHours,
}

basis_model! {
    pub struct Event {
        /// The event's core VF type
        inner: vf::EconomicEvent<(), CompanyID, ProcessID, AgentID, (), (), ResourceSpecID, ResourceID, EventID>,
        /// If this event is an output of a process, move some fixed amount of
        /// the process' costs and transfer them either into another Process or
        /// into a Resource
        move_costs: Option<Costs>,
        /// When recording a work event, this lets us know if it should apply to
        /// the `labor` and/or `labor_hours` buckets of our Costs object.
        labor_type: Option<LaborType>,
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
    pub fn process(&self, output_of: Option<Process>, input_of: Option<Process>, resource: Option<Resource>, provider: Option<&CompanyMember>, updated: &DateTime<Utc>) -> Result<EventProcessResult> {
        // some low-hanging fruit error checking
        if self.inner().output_of().as_ref() != output_of.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedOutputProcessID)?;
        }
        if self.inner().input_of().as_ref() != input_of.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedInputProcessID)?;
        }
        if self.inner().resource_inventoried_as().as_ref() != resource.as_ref().map(|x| x.id()) {
            Err(Error::EventMismatchedResourceID)?;
        }
        if let Some(provider) = provider.as_ref() {
            if self.inner().provider() != &provider.id().clone().into() {
                Err(Error::EventMismatchedProviderID)?;
            }
        }

        // create our result set
        let mut res = EventProcessResult::new(&self.id, updated);

        match self.inner().action() {
            Action::Accept => {}
            Action::Cite => {}
            Action::Consume => {
                let resource = resource.ok_or(Error::EventMissingResource)?;
                let mut resource_measure = resource.inner().accounting_quantity().clone()
                    .ok_or(Error::EventMissingResourceQuantity)?;
                let event_measure = self.inner().resource_quantity().clone()
                    .ok_or(Error::EventMissingResourceQuantity)?;

                let mut should_save_resource = false;
                let mut resource_mod = resource.clone();
                if measure::dec_measure(&mut resource_measure, &event_measure)? {
                    resource_mod.inner_mut().set_accounting_quantity(Some(resource_measure));
                    should_save_resource = true;
                }
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut input_process = input_of.ok_or(Error::EventMissingInputProcess)?.clone();
                    if resource_mod.move_costs_to(&mut input_process, &move_costs)? {
                        res.modify_process(input_process);
                        should_save_resource = true;
                    }
                }
                if should_save_resource {
                    res.modify_resource(resource_mod);
                }
            }
            Action::DeliverService => {
            }
            Action::Dropoff => {}
            Action::Lower => {}
            Action::Modify => {}
            Action::Move => {}
            Action::Pickup => {}
            Action::Produce => {
                let resource = resource.ok_or(Error::EventMissingResource)?;
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
                let mut resource_mod = resource.clone();
                resource_mod.inner_mut().set_accounting_quantity(Some(new_resource_measure));
                resource_mod.inner_mut().set_primary_accountable(Some(company_id.clone().into()));
                resource_mod.set_in_custody_of(company_id.into());
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut output_process = output_of.ok_or(Error::EventMissingOutputProcess)?.clone();
                    if output_process.move_costs_to(&mut resource_mod, move_costs)? {
                        res.modify_process(output_process);
                    }
                }
                res.modify_resource(resource_mod);
            }
            Action::Raise => {}
            Action::Transfer => {
                let resource = resource.ok_or(Error::EventMissingResource)?;
                let company_id: CompanyID = self.inner().receiver().clone().try_into()?;
                let mut new_resource = resource.clone();
                new_resource.inner_mut().set_primary_accountable(Some(company_id.clone().into()));
                new_resource.set_in_custody_of(company_id.into());
                res.modify_resource(new_resource);
            }
            Action::TransferAllRights => {
                let resource = resource.ok_or(Error::EventMissingResource)?;
                let company_id: CompanyID = self.inner().receiver().clone().try_into()?;
                let mut new_resource = resource.clone();
                new_resource.inner_mut().set_primary_accountable(Some(company_id.into()));
                res.modify_resource(new_resource);
            }
            Action::TransferCustody => {
                let resource = resource.ok_or(Error::EventMissingResource)?;
                let company_id: CompanyID = self.inner().receiver().clone().try_into()?;
                let mut new_resource = resource.clone();
                new_resource.set_in_custody_of(company_id.into());
                res.modify_resource(new_resource);
            }
            Action::Use => {
                let resource = resource.ok_or(Error::EventMissingResource)?;
                let mut resource_mod = resource.clone();
                if let Some(move_costs) = self.move_costs().as_ref() {
                    let mut input_process = input_of.ok_or(Error::EventMissingInputProcess)?.clone();
                    if resource_mod.move_costs_to(&mut input_process, &move_costs)? {
                        res.modify_resource(resource_mod);
                        res.modify_process(input_process);
                    }
                }
            }
            Action::Work => {
                //let mut input_process = input_of.ok_or(Error::EventMissingInputProcess)?.clone();
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
            company::CompanyID,
            process::Process,
            resource::Resource,
        },
        util,
    };
    use om2::{Measure, NumericUnion, Unit};
    use rust_decimal::prelude::*;
    use vf_rs::vf;

    #[test]
    fn consume() {
    }

    #[test]
    fn deliver_service() {
    }

    #[test]
    fn produce() {
        let company_id: CompanyID = "6969".into();
        let now = util::time::now();
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
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let res = event.process(Some(process_from.clone()), Some(process_to.clone()), Some(resource.clone()), None, &now).unwrap();
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
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        match event.process(Some(process_from.clone()), Some(process_to.clone()), Some(resource.clone()), None, &now) {
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

