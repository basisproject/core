//! The `Event` model is *the* core piece of the system that really ties the
//! room together. Processing events is what allows moving `Costs` through the
//! system.
//!
//! It's important to note that many REA systems, to my understanding, have a
//! recording/observation process, and afterwards an analysis process. Because
//! the economic graph is so vast and complex, we don't have the luxury of doing
//! an analysis process: observation and analysis must happen at the same time!
//! This means we have to forego some of the niceties we might get in other REA
//! systems.
//!
//! For clarity on events and how they tie in with intents and commitments:
//!
//! - An [Intent] represents "we want something to happen"
//! - A [Commitment] represents "we agree that something will happen"
//! - An `Event` represents "something did happen"
//!
//! [Intent]: ../intent/struct.Intent.html
//! [Commitment]: ../commitment/struct.Commitment.html

use chrono::{DateTime, Utc};
use crate::{
    costs::{Costs, CostMover},
    error::{Error, Result},
    models::{
        Op,
        Modifications,

        agreement::AgreementID,
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
use thiserror::Error;
use vf_rs::vf::{self, Action, InputOutput, ResourceEffect};

/// An error type for when event processing goes awry.
#[derive(Error, Debug, PartialEq)]
pub enum EventError {
    /// An event's end date must be after its begin date
    #[error("end time must be after begin time")]
    DateEndBeforeBegin,
    /// We're trying to set `has_end` with `has_beginning` being blank. This
    /// does not make sense and I'm afraid I cannot allow this to happen.
    #[error("cannot specify an end date without a begin date")]
    DateEndMustHaveBegin,
    /// We're trying to add inputs to a deleted process. No. Bad.
    #[error("cannot add inputs to a deleted process")]
    InputOnDeletedProcess,
    /// We expected an InputOutput value but didn't find one
    #[error("missing InputOutput designation")]
    InvalidInputOutput,
    /// A labor event was recorded with some effort unit other than hours
    #[error("labor effort must be recorded in hours")]
    LaborMustBeHours,
    /// The given input_of process does not match the event's input process id
    #[error("the given input_of process does not match the event's input process id")]
    MismatchedInputProcessID,
    /// The given `output_of` process does not match the event's output process id
    #[error("the given `output_of` process does not match the event's output process id")]
    MismatchedOutputProcessID,
    /// The given provider does not match the event's provider id
    #[error("the given provider does not match the event's provider id")]
    MismatchedProviderID,
    /// The given resource does not match the event's resource id
    #[error("the given resource does not match the event's resource id")]
    MismatchedResourceID,
    /// The given to_resource does not match the event's to resource id
    #[error("the given to_resource does not match the event's to_resource id")]
    MismatchedResourceToID,
    /// The event is missing the `move_costs` field
    #[error("event requires the `move_costs` field")]
    MissingCosts,
    /// The event is missing the `effort_quantity` field
    #[error("event requires the `effort_quantity` field")]
    MissingEffortQuantity,
    /// The event is missing the `resource_quantity` field
    #[error("event requires the `resource_quantity` field")]
    MissingEventMeasure,
    /// The event is missing the `input_of` object
    #[error("this event requires the `input_of` process")]
    MissingInputProcess,
    /// The event is missing the `move_type` field
    #[error("event requires the `move_type` field")]
    MissingMoveType,
    /// The event is missing the `output_of` object
    #[error("this event requires the `output_of` process")]
    MissingOutputProcess,
    /// The event is missing the `provider` object
    #[error("this event requires the `provider` object")]
    MissingProvider,
    /// The event is missing the `resource` object
    #[error("this event requires the `resource` object")]
    MissingResource,
    /// The event is missing the `resource_to` object
    #[error("this event requires the `resource_to` object")]
    MissingResourceTo,
    /// When we try to run an operation on a process we don't own
    #[error("operation on a resource you don't own")]
    ProcessOwnerMismatch,
    /// The resource's accounting quantity cannot be zero if the resource has
    /// non-zero `costs`
    #[error("event's resource cannot have an accounting quantity == 0 with costs > 0")]
    ResourceCostQuantityMismatch,
    /// When performing an operation on a resource that isn't in your custody
    #[error("operation on a resource you don't have custody of")]
    ResourceCustodyMismatch,
    /// When performing an operation on a resource that doesn't belong to you
    #[error("operation on a resource you don't own")]
    ResourceOwnerMismatch,
}

/// When creating a `transfer` event, we need to know if that event transfers
/// costs internally between processes or if it transfers resources between
/// different agents.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum MoveType {
    /// This is an internal cost transfer *between two processes*.
    ProcessCosts,
    /// This moves a resource internally in the company (this is the original
    /// indended purpose of the `move` action in VF)
    Resource,
}

basis_model! {
    /// The event model, which is the glue that moves costs between objects.
    ///
    /// This model wraps the [ValueFlows event][vfevent] object. It effectively
    /// describes things that have already happened in the economic network,
    /// and it can fulfill commitments.
    ///
    /// [vfevent]: https://valueflo.ws/introduction/flows.html#economic-events
    pub struct Event {
        id: <<EventID>>,
        /// The event's core VF type
        inner: vf::EconomicEvent<AgreementID, AgentID, ProcessID, AgentID, AgreementID, (), ResourceSpecID, ResourceID, EventID>,
        /// If this event is an input/output of a process or resource, move some
        /// fixed amount of costs between the two objects.
        move_costs: Option<Costs>,
        /// The type of move (if using `Action::Move`). Can be cost-based
        /// (exclusively for moving costs between resources and processes) or
        /// resource-based (moving a resource internally in a company).
        ///
        /// We could just measure whether or not the event has a ResourceID, but
        /// being explicit is probably a better choice, especially when going
        /// outside of the intended purpose of the `Move` event. It also makes
        /// things more clear when creating the event whether it should be
        /// allowed or not.
        move_type: Option<MoveType>,
    }
    EventBuilder
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
    /// The secondary resource we're operating on (Transfer/Move/etc)
    to_resource: Option<Resource>,
}

impl EventProcessState {
    /// Create a state builder
    pub fn builder() -> EventProcessStateBuilder {
        EventProcessStateBuilder::default()
    }
}

/// A standard result set our event processor can return, including the items
/// we wish to be saved/updated.
#[derive(Debug, PartialEq)]
pub struct EventProcessResult {
    /// The ID of the current event we're processing the result for
    event_id: EventID,
    /// The time we're processing the event
    process_time: DateTime<Utc>,
    /// The items we're saving/updating as a result of processing this event
    modifications: Modifications,
}

impl EventProcessResult {
    /// Create a new result
    pub fn new(event_id: &EventID, process_time: &DateTime<Utc>) -> Self {
        Self {
            event_id: event_id.clone(),
            process_time: process_time.clone(),
            modifications: Modifications::new(),
        }
    }

    /// Consume the result and return the modification list
    pub fn into_modifications(self) -> Modifications {
        self.modifications
    }

    /// Push an event to create into the result set
    #[allow(dead_code)]
    fn create_event(&mut self, mut event: Event) {
        event.inner_mut().set_triggered_by(Some(self.event_id.clone()));
        event.set_created(self.process_time.clone());
        event.set_updated(self.process_time.clone());
        self.modifications.push(Op::Create, event);
    }

    /// Push a resource to create into the result set
    #[allow(dead_code)]
    fn create_resource(&mut self, mut resource: Resource) {
        resource.set_created(self.process_time.clone());
        resource.set_updated(self.process_time.clone());
        self.modifications.push(Op::Create, resource);
    }

    /// Push a process to modify into the result set
    #[allow(dead_code)]
    fn modify_process(&mut self, mut process: Process) {
        process.set_updated(self.process_time.clone());
        self.modifications.push(Op::Update, process);
    }

    /// Push a resource to modify into the result set
    #[allow(dead_code)]
    fn modify_resource(&mut self, mut resource: Resource) {
        resource.set_updated(self.process_time.clone());
        self.modifications.push(Op::Update, resource);
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
    pub fn process(&self, state: EventProcessState, now: &DateTime<Utc>) -> Result<Modifications> {
        // some low-hanging fruit error checking. basically make sure that if we
        // pass in a process/resource that it's id matches the one we have in
        // the event's data.
        if state.output_of.is_some() && self.inner().output_of().as_ref() != state.output_of.as_ref().map(|x| x.id()) {
            Err(EventError::MismatchedOutputProcessID)?;
        }
        if state.input_of.is_some() && self.inner().input_of().as_ref() != state.input_of.as_ref().map(|x| x.id()) {
            Err(EventError::MismatchedInputProcessID)?;
        }
        if state.resource.is_some() && self.inner().resource_inventoried_as().as_ref() != state.resource.as_ref().map(|x| x.id()) {
            Err(EventError::MismatchedResourceID)?;
        }
        if state.to_resource.is_some() && self.inner().to_resource_inventoried_as().as_ref() != state.to_resource.as_ref().map(|x| x.id()) {
            Err(EventError::MismatchedResourceToID)?;
        }
        if let Some(provider) = state.provider.as_ref() {
            if self.inner().provider() != &provider.id().clone().into() {
                Err(EventError::MismatchedProviderID)?;
            }
        }
        if self.inner().has_beginning().is_none() && self.inner().has_end().is_some() {
            Err(EventError::DateEndMustHaveBegin)?;
        }
        match (self.inner().has_beginning().as_ref(), self.inner().has_end().as_ref()) {
            (Some(begin), Some(end)) => {
                if end < begin {
                    Err(EventError::DateEndBeforeBegin)?;
                }
            }
            _ => {}
        }

        // create our result set.
        let mut res = EventProcessResult::new(self.id(), now);

        // this event is started but not completed, so it's pending and we don't
        // apply it yet.
        if self.inner().has_beginning().is_some() && self.inner().has_end().is_none() {
            return Ok(res.into_modifications());
        }

        // grab our action and some values from it
        let action = self.inner().action();
        let accounting_effect = Some(action.resource_effect()).and_then(|x| if x == ResourceEffect::NoEffect { None } else { Some(x) });
        let onhand_effect = Some(action.onhand_effect()).and_then(|x| if x == ResourceEffect::NoEffect { None } else { Some(x) });
        // note that the following line only works because accounting/onhand
        // effects are paired in concert with each other. if there was every a
        // case where accounting increased and onhand decreased in the same
        // action, much of the logic using bundle_effect would need to be
        // rewritten.
        let bundle_effect = accounting_effect.clone().or(onhand_effect.clone());

        // attempt to grab our primary and (if applicable) secondary process and
        // resource.
        let mut process: Option<Process> = match action.input_output() {
            Some(InputOutput::Input) => {
                let process = state.input_of.clone().ok_or(EventError::MissingInputProcess)?;
                // make sure the receiver owns the process we're inputting into
                if self.inner().receiver() != &process.company_id().clone().into() {
                    Err(EventError::ProcessOwnerMismatch)?;
                }
                Some(process)
            }
            Some(InputOutput::Output) => {
                let process = state.output_of.clone().ok_or(EventError::MissingOutputProcess)?;
                // make sure the provider owns the process we're outputting from
                if self.inner().provider() != &process.company_id().clone().into() {
                    Err(EventError::ProcessOwnerMismatch)?;
                }
                Some(process)
            }
            None => None,
        };
        // we fill these in either by hand in the action matcher or using our
        // default functions defined just under here
        let mut process2: Option<Process> = None;
        let mut resource: Option<Resource> = None;
        let mut resource2: Option<Resource> = None;
        let mut resource2_is_create = false;
        let mut resource_owner_must_match = true;
        let mut move_costs: Option<Costs> = None;

        // tries to guess if we *need* a primary resource, and if so, grabs it
        // from the state
        let mut default_resource = || -> Result<()> {
            resource = match &bundle_effect {
                &Some(_) => Some(state.resource.clone().ok_or(EventError::MissingResource)?.clone()),
                &None => None,
            };
            Ok(())
        };
        // tries to guess if we *need* a secondary resource, and if so, grabs it
        // from the state. however, if we specify a to_resource id in the event
        // data but neglect to send in a to resource in our state, this means we
        // want to *create* a new resource copied from the primary resource.
        let mut default_resource2 = |resource1: &Option<Resource>| -> Result<()> {
            resource2 = match &bundle_effect {
                &Some(ResourceEffect::DecrementIncrement) => {
                    match (state.to_resource.as_ref(), resource1.as_ref(), self.inner().to_resource_inventoried_as().as_ref()) {
                        (Some(resource), _, _) => {
                            Some(resource.clone())
                        }
                        (None, Some(primary_resource), Some(resource_id)) => {
                            let mut res_tmp = primary_resource.clone();
                            res_tmp.set_id(resource_id.clone());
                            res_tmp.set_costs(Costs::new());
                            res_tmp.zero_measures();
                            resource2_is_create = true;
                            Some(res_tmp)
                        }
                        _ => Err(EventError::MissingResourceTo)?,
                    }
                }
                _ => None,
            };
            Ok(())
        };
        // tries to guess if we *need* move costs, and if so, grabs them from
        // the event data
        let mut default_move_costs = || -> Result<()> {
            move_costs = match (action.input_output(), &bundle_effect) {
                (Some(_), _) | (_, &Some(ResourceEffect::DecrementIncrement)) => {
                    Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?)
                }
                _ => None,
            };
            Ok(())
        };

        // most of our actions will use the same processing logic, but we also
        // have cases where overrides are necessary because input_output() and
        // resource_effect() don't cover all cases.
        //
        // that said, we can still use defaults where applicable.
        match action {
            // needed because we can't determine the resource from the action
            // resource effects
            Action::Cite => {
                move_costs = Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?);
                resource = Some(state.resource.clone().ok_or(EventError::MissingResource)?);
            }
            Action::DeliverService => {
                move_costs = Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?);
                process2 = Some(state.input_of.clone().ok_or(EventError::MissingInputProcess)?);
            }
            Action::Dropoff => {
                move_costs = Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?);
                resource = Some(state.resource.clone().ok_or(EventError::MissingResource)?);
                resource_owner_must_match = false;
            }
            Action::Move => {
                move_costs = Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?);
                match self.move_type() {
                    Some(MoveType::ProcessCosts) => {
                        process = Some(state.output_of.clone().ok_or(EventError::MissingOutputProcess)?);
                        process2 = Some(state.input_of.clone().ok_or(EventError::MissingInputProcess)?);
                    }
                    Some(MoveType::Resource) => {
                        default_resource()?;
                        default_resource2(&resource)?;
                    }
                    None => Err(EventError::MissingMoveType)?,
                }
            }
            Action::Pickup => {
                move_costs = Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?);
                resource = Some(state.resource.clone().ok_or(EventError::MissingResource)?);
                resource_owner_must_match = false;
            }
            // needed because we can't determine the resource from the action
            // resource effects
            Action::Use => {
                move_costs = Some(self.move_costs().clone().ok_or(EventError::MissingCosts)?);
                resource = Some(state.resource.clone().ok_or(EventError::MissingResource)?);
            }
            Action::Work => {
                let mut input_process = state.input_of.clone().ok_or(EventError::MissingInputProcess)?;
                let member = state.provider.clone().ok_or(EventError::MissingProvider)?;
                let occupation_id = member.inner().relationship().clone();
                let move_costs = self.move_costs().as_ref().ok_or(EventError::MissingCosts)?;

                // grab JUST this occupation's costs from the event. in other
                // words, we only accept costs specific to this occupation. it
                // would be stupid for the transaction creating this event to
                // pass in any costs that weren't just relating to this
                // occupation, but better safe than sorry.
                let occupation_costs = move_costs.get_labor(occupation_id.clone());
                let hours = match self.inner().effort_quantity() {
                    Some(Measure { has_unit: Unit::Hour, has_numerical_value: hours }) => {
                        let num_hours = NumericUnion::Decimal(Decimal::zero()).add(hours.clone())
                            .map_err(|e| Error::NumericUnionOpError(e))?;
                        match num_hours {
                            NumericUnion::Decimal(val) => val,
                            _ => Err(Error::NumericUnionOpError(format!("error converting to Decimal: {:?}", num_hours)))?,
                        }
                    }
                    None => Err(EventError::MissingEffortQuantity)?,
                    _ => Err(EventError::LaborMustBeHours)?,
                };
                let mut costs = Costs::new();
                costs.track_labor(occupation_id.clone(), occupation_costs);
                costs.track_labor_hours(occupation_id, hours);
                input_process.receive_costs(&costs)?;
                res.modify_process(input_process);
            }
            _ => {
                default_resource()?;
                default_resource2(&resource)?;
                match (action.input_output(), &bundle_effect) {
                    (Some(_), _) | (_, &Some(ResourceEffect::DecrementIncrement)) => {
                        default_move_costs()?;
                    }
                    _ => {}
                }
            }
        }

        match (process.as_ref(), process2.as_ref()) {
            (Some(process1), Some(process2)) => {
                if self.inner().provider() != &process1.company_id().clone().into() {
                    Err(EventError::ProcessOwnerMismatch)?;
                }
                if self.inner().receiver() != &process2.company_id().clone().into() {
                    Err(EventError::ProcessOwnerMismatch)?;
                }
                if process2.is_deleted() {
                    Err(EventError::InputOnDeletedProcess)?;
                }
            }
            (Some(process), None) => {
                if action.input_output() == Some(InputOutput::Output) && self.inner().provider() != &process.company_id().clone().into() {
                    Err(EventError::ProcessOwnerMismatch)?;
                }
                if action.input_output() == Some(InputOutput::Input) && self.inner().receiver() != &process.company_id().clone().into() {
                    Err(EventError::ProcessOwnerMismatch)?;
                }
                if action.input_output() == Some(InputOutput::Input) && process.is_deleted() {
                    Err(EventError::InputOnDeletedProcess)?;
                }
            }
            _ => {}
        }

        // make sure the primary resource is being acted on by its owner and/or
        // custodian
        match resource.as_ref() {
            Some(resource) => {
                if (accounting_effect == Some(ResourceEffect::Decrement) || accounting_effect == Some(ResourceEffect::DecrementIncrement)) && resource.inner().primary_accountable().as_ref() != Some(self.inner().provider()) {
                    Err(EventError::ResourceOwnerMismatch)?;
                }
                if accounting_effect == Some(ResourceEffect::Increment) && resource.inner().primary_accountable().as_ref() != Some(self.inner().receiver()) {
                    Err(EventError::ResourceOwnerMismatch)?;
                }
                if self.inner().provider() == self.inner().receiver() && resource.inner().primary_accountable().as_ref() != Some(self.inner().provider()) {
                    if resource_owner_must_match {
                        Err(EventError::ResourceOwnerMismatch)?;
                    }
                }
                if (onhand_effect == Some(ResourceEffect::Decrement) || onhand_effect == Some(ResourceEffect::DecrementIncrement)) && resource.in_custody_of() != self.inner().provider() {
                    Err(EventError::ResourceCustodyMismatch)?;
                }
                if onhand_effect == Some(ResourceEffect::Increment) && resource.in_custody_of() != self.inner().receiver() {
                    Err(EventError::ResourceCustodyMismatch)?;
                }
                if self.inner().provider() == self.inner().receiver() && resource.in_custody_of() != self.inner().provider() {
                    Err(EventError::ResourceCustodyMismatch)?;
                }
            }
            _ => {}
        }

        // make sure the secondary resource is being acted on by its owner
        // and/or custodian
        if !resource2_is_create {
            match resource2.as_ref() {
                Some(resource) => {
                    if resource.inner().primary_accountable().as_ref() != Some(self.inner().receiver()) {
                        Err(EventError::ResourceOwnerMismatch)?;
                    }
                    if resource.in_custody_of() != self.inner().receiver() {
                        Err(EventError::ResourceCustodyMismatch)?;
                    }
                }
                _ => {}
            }
        }

        // save these so we can test for changes in our final step
        let process_clone = process.clone();
        let process2_clone = process2.clone();
        let resource_clone = resource.clone();
        let resource2_clone = resource2.clone();

        // cost moving logic
        if process.is_some() && process2.is_some() {
            let move_costs = move_costs.ok_or(EventError::MissingCosts)?;
            let process_output = process.as_mut().unwrap();
            let process_input = process2.as_mut().unwrap();
            process_output.move_costs_to(process_input, &move_costs)?;
        } else if resource.is_some() && resource2.is_some() {
            let move_costs = move_costs.ok_or(EventError::MissingCosts)?;
            let resource_output = resource.as_mut().unwrap();
            let resource_input = resource2.as_mut().unwrap();
            resource_output.move_costs_to(resource_input, &move_costs)?;
        } else if resource.is_some() && process.is_some() {
            let move_costs = move_costs.ok_or(EventError::MissingCosts)?;
            let resource_inner = resource.as_mut().unwrap();
            let process_inner = process.as_mut().unwrap();
            match action.input_output() {
                Some(InputOutput::Input) => {
                    resource_inner.move_costs_to(process_inner, &move_costs)?;
                }
                Some(InputOutput::Output) => {
                    process_inner.move_costs_to(resource_inner, &move_costs)?;
                }
                None => { Err(EventError::InvalidInputOutput)?; }
            }
        }

        // accounting/offhand quantity adjustments
        macro_rules! incdec_builder_primary {
            ($effect:expr, $resource:ident, $fn_get:ident, $fn_set:ident, $res:ident, $measure:ident, $($extra:tt)*) => {
                match ($effect, $resource.as_mut()) {
                    (Some(effect), Some($res)) => {
                        let event_measure = self.inner().resource_quantity().clone()
                            .ok_or(EventError::MissingEventMeasure)?;
                        let mut $measure = measure::unwrap_or_zero($res.inner().$fn_get(), &event_measure);
                        match effect {
                            ResourceEffect::Decrement | ResourceEffect::DecrementIncrement => {
                                measure::dec_measure(&mut $measure, &event_measure)?;
                                $($extra)*
                                $res.inner_mut().$fn_set(Some($measure));
                            }
                            ResourceEffect::Increment => {
                                measure::inc_measure(&mut $measure, &event_measure)?;
                                $res.inner_mut().$fn_set(Some($measure));
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        macro_rules! incdec_builder_secondary {
            ($effect:expr, $resource:ident, $fn_get:ident, $fn_set:ident) => {
                match ($effect, $resource.as_mut()) {
                    (Some(effect), Some(res)) => {
                        let event_measure = self.inner().resource_quantity().clone()
                            .ok_or(EventError::MissingEventMeasure)?;
                        let mut resource_measure = measure::unwrap_or_zero(res.inner().$fn_get(), &event_measure);
                        if effect == ResourceEffect::DecrementIncrement {
                            measure::inc_measure(&mut resource_measure, &event_measure)?;
                            res.inner_mut().$fn_set(Some(resource_measure));
                        }
                    }
                    _ => {}
                }
            };
        }
        incdec_builder_primary! {
            accounting_effect.clone(), resource, accounting_quantity, set_accounting_quantity, res, resource_measure, {
                if resource_measure.has_numerical_value().is_zero() && res.costs().is_gt_0() {
                    Err(EventError::ResourceCostQuantityMismatch)?;
                }
            }
        }
        incdec_builder_secondary! { accounting_effect, resource2, accounting_quantity, set_accounting_quantity }
        incdec_builder_primary! { onhand_effect.clone(), resource, onhand_quantity, set_onhand_quantity, res, resource_measure, {} }
        incdec_builder_secondary! { onhand_effect, resource2, onhand_quantity, set_onhand_quantity }

        // set resource custody/ownership
        if let Some(res) = resource.as_mut() {
            if action.resource_effect() == ResourceEffect::Increment {
                res.inner_mut().set_primary_accountable(Some(self.inner().receiver().clone()));
            }
            if action.onhand_effect() == ResourceEffect::Increment {
                res.set_in_custody_of(self.inner().receiver().clone());
            }
        }
        if let Some(res) = resource2.as_mut() {
            if action.resource_effect() == ResourceEffect::DecrementIncrement {
                res.inner_mut().set_primary_accountable(Some(self.inner().receiver().clone()));
            }
            if action.onhand_effect() == ResourceEffect::DecrementIncrement {
                res.set_in_custody_of(self.inner().receiver().clone());
            }
        }

        // save any resource modifications
        if let Some(location) = self.inner().at_location().as_ref() {
            if resource2.is_some() {
                resource2.as_mut().map(|res| { res.inner_mut().set_current_location(Some(location.clone())); });
            } else if resource.is_some() {
                resource.as_mut().map(|res| { res.inner_mut().set_current_location(Some(location.clone())); });
            }
        }

        // save our changes, if we have them
        if process != process_clone { res.modify_process(process.unwrap()); }
        if process2 != process2_clone { res.modify_process(process2.unwrap()); }
        if resource != resource_clone { res.modify_resource(resource.unwrap()); }
        if resource2_is_create {
            res.create_resource(resource2.unwrap());
        } else if resource2 != resource2_clone {
            res.modify_resource(resource2.unwrap());
        }

        Ok(res.into_modifications())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        costs::Costs,
        models::{
            company::{CompanyID, Permission},
            company_member::{Compensation},
            process::Process,
            resource::Resource,
            user::UserID,
        },
        util,
    };
    use om2::{Measure, NumericUnion, Unit};
    use rust_decimal_macros::*;
    use vf_rs::vf;

    fn required_fields(event: &Event, state: &EventProcessState) -> (Vec<&'static str>, Vec<&'static str>) {
        let mut event_fields = vec![];
        let mut state_fields = vec![];
        let action = event.inner().action();
        let accounting_effect = Some(action.resource_effect()).and_then(|x| if x == ResourceEffect::NoEffect { None } else { Some(x) });
        let onhand_effect = Some(action.onhand_effect()).and_then(|x| if x == ResourceEffect::NoEffect { None } else { Some(x) });
        let bundle_effect = accounting_effect.clone().or(onhand_effect.clone());
        match action {
            Action::Move => {
                event_fields.push("move_type");
                event_fields.push("move_costs");
                match event.move_type() {
                    Some(MoveType::Resource) => {
                        event_fields.push("resource_quantity");
                        event_fields.push("to_resource_inventoried_as");
                    }
                    _ => {}
                }
            }
            Action::Work => {
                event_fields.push("move_costs");
                event_fields.push("effort_quantity");
            }
            _ => {
                match (action.input_output(), bundle_effect.clone()) {
                    (Some(_), _) | (_, Some(ResourceEffect::DecrementIncrement)) => {
                        event_fields.push("move_costs");
                    }
                    _ => {}
                }
                if bundle_effect == Some(ResourceEffect::DecrementIncrement) {
                    event_fields.push("to_resource_inventoried_as");
                }
                match bundle_effect {
                    Some(_) => {
                        event_fields.push("resource_quantity");
                    }
                    _ => {}
                }
            }
        }
        match action {
            Action::DeliverService => {
                state_fields.push("input_of");
                state_fields.push("output_of");
            }
            Action::Move => {
                match event.move_type() {
                    Some(MoveType::ProcessCosts) => {
                        state_fields.push("input_of");
                        state_fields.push("output_of");
                    }
                    Some(MoveType::Resource) => {
                        state_fields.push("resource");
                        // to_resource not required because we can create the
                        // resource via to_resource_inventoried_as
                    }
                    _ => {}
                }
            }
            Action::Use => {
                state_fields.push("resource");
                state_fields.push("input_of");
            }
            Action::Work => {
                state_fields.push("input_of");
                state_fields.push("provider");
            }
            _ => {
                match action.input_output() {
                    Some(InputOutput::Input) => {
                        state_fields.push("input_of");
                    }
                    Some(InputOutput::Output) => {
                        state_fields.push("output_of");
                    }
                    _ => {}
                }
                if bundle_effect.is_some() {
                    state_fields.push("resource");
                }
            }
        }
        if state.to_resource.is_some() && !event_fields.contains(&"to_resource_inventoried_as") {
            event_fields.push("to_resource_inventoried_as");
        }
        event_fields.sort();
        state_fields.sort();
        (event_fields, state_fields)
    }

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
                "to_resource" => {
                    if state.to_resource.is_some() {
                        builder = builder.to_resource(state.to_resource.clone().unwrap());
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
        let all_state_combos = generate_combinations(&vec!["input_of", "output_of", "provider", "resource", "to_resource"]);
        let all_event_combos = generate_combinations(&vec!["move_costs", "move_type", "resource_quantity", "effort_quantity", "to_resource_inventoried_as"]);
        for evfields in &all_event_combos {
            let mut event2 = event.clone();
            event2.set_move_costs(None);
            event2.set_move_type(None);
            event2.inner_mut().set_resource_quantity(None);
            event2.inner_mut().set_effort_quantity(None);
            event2.inner_mut().set_to_resource_inventoried_as(None);
            for evfield in evfields {
                match *evfield {
                    "move_costs" => { event2.set_move_costs(event.move_costs().clone()); }
                    "move_type" => { event2.set_move_type(event.move_type().clone()); }
                    "resource_quantity" => { event2.inner_mut().set_resource_quantity(event.inner().resource_quantity().clone()); }
                    "effort_quantity" => { event2.inner_mut().set_effort_quantity(event.inner().effort_quantity().clone()); }
                    "to_resource_inventoried_as" => { event2.inner_mut().set_to_resource_inventoried_as(event.inner().to_resource_inventoried_as().clone()); }
                    _ => {}
                }
            }
            for fieldset in &all_state_combos {
                let state = state_with_fields(&state, fieldset.clone());
                let (must_event_fields, must_state_fields) = required_fields(&event2, &state);
                let has_event_fields = must_event_fields.iter().fold(true, |acc, x| acc && evfields.contains(x));
                let has_state_fields = must_state_fields.iter().fold(true, |acc, x| acc && fieldset.contains(x));
                let should_pass = has_event_fields && has_state_fields;
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
    fn make_state(company_id: &CompanyID, company_to: &CompanyID, provider_is_company: bool, now: &DateTime<Utc>) -> EventProcessState {
        let mut builder = EventProcessState::builder();
        let process_from = Process::builder()
            .id("1111")
            .inner(vf::Process::builder().name("Make widgets").build().unwrap())
            .company_id(company_id.clone())
            .costs(Costs::new_with_labor("machinist", dec!(100.0)))
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let process_to = Process::builder()
            .id("1112")
            .inner(vf::Process::builder().name("Check widgets").build().unwrap())
            .company_id(company_to.clone())
            .costs(Costs::default())
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let resource = Resource::builder()
            .id("4444")
            .inner(
                vf::EconomicResource::builder()
                    .primary_accountable(Some(company_id.clone().into()))
                    .accounting_quantity(Measure::new(NumericUnion::Integer(10), Unit::One))
                    .onhand_quantity(Measure::new(NumericUnion::Integer(11), Unit::One))
                    .conforms_to("3330")
                    .build().unwrap()
            )
            .in_custody_of(company_id.clone())
            .costs(Costs::new_with_labor("machinist", dec!(34.91)))
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
        let resource_to = Resource::builder()
            .id("4445")
            .inner(
                vf::EconomicResource::builder()
                    .primary_accountable(Some(company_to.clone().into()))
                    .accounting_quantity(Measure::new(NumericUnion::Integer(1), Unit::One))
                    .onhand_quantity(Measure::new(NumericUnion::Integer(0), Unit::One))
                    .conforms_to("3330")
                    .build().unwrap()
            )
            .in_custody_of(company_to.clone())
            .costs(Costs::new_with_labor("trucker", dec!(29.8)))
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
                .permissions(vec![Permission::MemberCreate, Permission::MemberSetPermissions, Permission::MemberDelete])
                .compensation(Some(Compensation::new_hourly(dec!(0.0), "12345")))
                .created(now.clone())
                .updated(now.clone())
                .build().unwrap();
            builder = builder.provider(member);
        }
        builder
            .output_of(process_from)
            .input_of(process_to)
            .resource(resource)
            .to_resource(resource_to)
            .build().unwrap()
    }

    /// Create a test event. Change it how you want after the fact. Or don't. I
    /// don't care.
    fn make_event(action: vf::Action, company_id: &CompanyID, company_to: &CompanyID, state: &EventProcessState, now: &DateTime<Utc>) -> Event {
        Event::builder()
            .id(EventID::create())
            .inner(
                vf::EconomicEvent::builder()
                    .action(action)
                    .has_beginning(now.clone())
                    .has_end(now.clone())
                    .input_of(state.input_of.as_ref().unwrap().id().clone())
                    .output_of(state.output_of.as_ref().unwrap().id().clone())
                    .provider(company_id.clone())
                    .receiver(company_to.clone())
                    .resource_inventoried_as(state.resource.as_ref().unwrap().id().clone())
                    .to_resource_inventoried_as(state.to_resource.as_ref().unwrap().id().clone())
                    .resource_quantity(Measure::new(NumericUnion::Decimal(dec!(6)), Unit::One))
                    .build().unwrap()
            )
            .move_costs(Costs::new_with_labor("machinist", dec!(30.0)))
            .move_type(None)
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
                    "onhand_quantity" => { resource.inner_mut().set_onhand_quantity(None); }
                    "primary_accountable" => { resource.inner_mut().set_primary_accountable(None); }
                    "current_location" => { resource.inner_mut().set_current_location(None); }
                    // TODO: all other event-editable resource fields
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
    fn validate_begin_end() {
        let now: DateTime<Utc> = "2018-06-06T00:00:00Z".parse().unwrap();
        let now2: DateTime<Utc> = "2018-06-06T06:52:00Z".parse().unwrap();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);
        let event = make_event(vf::Action::Consume, &company_id, &company_id, &state, &now);

        let res = event.process(state.clone(), &now);
        assert!(res.is_ok());

        let mut event2 = event.clone();
        event2.inner_mut().set_has_end(Some(now2.clone()));
        let res = event2.process(state.clone(), &now);
        assert!(res.is_ok());

        // end with no begin
        let mut event3 = event.clone();
        event3.inner_mut().set_has_beginning(None);
        event3.inner_mut().set_has_end(Some(now.clone()));
        let res = event3.process(state.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::DateEndMustHaveBegin)));

        // begin after end
        let mut event4 = event.clone();
        event4.inner_mut().set_has_beginning(Some(now2.clone()));
        event4.inner_mut().set_has_end(Some(now.clone()));
        let res = event4.process(state.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::DateEndBeforeBegin)));
    }

    // -------------------------------------------------------------------------

    #[test]
    fn consume() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let event = make_event(vf::Action::Consume, &company_id, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", dec!(30.0)));
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let resource = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().accounting_quantity().clone().unwrap(), Measure::new(NumericUnion::Integer(4), Unit::One));
        assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(4.91)));
        check_resource_mods(vec!["costs", "accounting_quantity", "onhand_quantity"], &resource, state.resource.as_ref().unwrap());

        let mut event = make_event(vf::Action::Consume, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        let res = event.process(state.clone(), &now);
        assert_eq!(res, Err(Error::NegativeCosts));

        let mut state2 = state.clone();
        state2.input_of.as_mut().unwrap().set_deleted(Some(now.clone()));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::InputOnDeletedProcess)));

        let mut state3 = state.clone();
        state3.input_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn deliver_service() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-consulting");
        let company2_id = CompanyID::new("bills-widgets");
        let state = make_state(&company_id, &company2_id, true, &now);

        let event = make_event(vf::Action::DeliverService, &company_id, &company2_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 70));
        check_process_mods(vec!["costs"], &process, state.output_of.as_ref().unwrap());

        let process = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 30));
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let mut event = make_event(vf::Action::DeliverService, &company_id, &company2_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        let res = event.process(state.clone(), &now);
        assert_eq!(res, Err(Error::NegativeCosts));

        let mut state2 = state.clone();
        state2.input_of.as_mut().unwrap().set_deleted(Some(now.clone()));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::InputOnDeletedProcess)));
    }

    #[test]
    fn lower() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let event = make_event(vf::Action::Lower, &company_id, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 1);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().accounting_quantity().as_ref().unwrap(), &Measure::new(4 as i64, Unit::One));
        assert_eq!(resource.inner().onhand_quantity().as_ref().unwrap(), &Measure::new(5 as i64, Unit::One));
        check_resource_mods(vec!["accounting_quantity", "onhand_quantity"], &resource, state.resource.as_ref().unwrap());

        let mut event = make_event(vf::Action::Lower, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(15)), Unit::One)));
        let res = event.process(state.clone(), &now);
        assert_eq!(res, Err(Error::NegativeMeasurement));

        let mut event = make_event(vf::Action::Lower, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(10)), Unit::One)));
        let res = event.process(state.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCostQuantityMismatch)));

        let mut event = make_event(vf::Action::Lower, &company_id, &company_id, &state, &now);
        let mut state2 = state.clone();
        state2.resource.as_mut().map(|x| x.set_costs(Costs::new()));
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(10)), Unit::One)));
        let mods = event.process(state2, &now).unwrap().into_vec();
        let resource2 = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.inner().accounting_quantity().as_ref().unwrap(), &Measure::new(0 as i64, Unit::One));
        assert_eq!(resource2.inner().onhand_quantity().as_ref().unwrap(), &Measure::new(1 as i64, Unit::One));
    }

    #[test]
    fn modify() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let event = make_event(vf::Action::Modify, &company_id, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 70));
        check_process_mods(vec!["costs"], &process, state.output_of.as_ref().unwrap());

        let resource = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(64.91)));
        check_resource_mods(vec!["costs", "onhand_quantity"], &resource, state.resource.as_ref().unwrap());

        let mut event = make_event(vf::Action::Modify, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        let res = event.process(state, &now);
        assert_eq!(res, Err(Error::NegativeCosts));
    }

    #[test]
    fn move_process_costs() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let mut event = make_event(vf::Action::Move, &company_id, &company_id, &state, &now);
        event.set_move_type(Some(MoveType::ProcessCosts));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 70));
        check_process_mods(vec!["costs"], &process, state.output_of.as_ref().unwrap());

        let process = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 30));
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.input_of.as_mut().unwrap().set_deleted(Some(now.clone()));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::InputOnDeletedProcess)));

        let mut state3 = state.clone();
        state3.output_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        let mut state4 = state.clone();
        state4.input_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state4.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn move_resource() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);
        let location = vf_rs::geo::SpatialThing::builder()
            .lat(Some(71.665519))
            .long(Some(129.019811))
            .alt(Some(500.0))
            .build().unwrap();
        let mut event = make_event(vf::Action::Move, &company_id, &company_id, &state, &now);
        event.set_move_type(Some(MoveType::Resource));
        event.inner_mut().set_at_location(Some(location));
        fuzz_state(event.clone(), state.clone(), &now);

        // going to move costs and counts
        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let mut costs = Costs::new();
        costs.track_labor("machinist", dec!(34.91) - dec!(30.0));
        let resource = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.costs(), &costs);
        assert_eq!(resource.inner().accounting_quantity(), &Some(Measure::new(10 - 6, Unit::One)));
        assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource.inner().current_location(), &None);
        assert_eq!(resource.in_custody_of(), &company_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "primary_accountable", "accounting_quantity", "onhand_quantity"], &resource, state.resource.as_ref().unwrap());

        let mut costs = Costs::new();
        costs.track_labor("trucker", dec!(29.8));
        costs.track_labor("machinist", dec!(30.0));
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.costs(), &costs);
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(1 + 6, Unit::One)));
        assert_eq!(resource2.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource2.inner().current_location().as_ref().unwrap().lat(), &Some(71.665519));
        assert_eq!(resource2.in_custody_of(), &company_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "primary_accountable", "accounting_quantity", "onhand_quantity", "current_location"], &resource2, state.to_resource.as_ref().unwrap());

        // going to move just costs (set count to 0 lol)
        let mut event2 = event.clone();
        event2.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(13.2))));
        event2.inner_mut().set_resource_quantity(Some(Measure::new(dec!(0), Unit::One)));
        let mods = event2.process(state.clone(), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 2);

        let mut costs = Costs::new();
        costs.track_labor("machinist", dec!(34.91) - dec!(13.2));
        let resource3 = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource3.costs(), &costs);
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(10, Unit::One)));
        assert_eq!(resource3.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource3.inner().current_location(), &None);
        assert_eq!(resource3.in_custody_of(), &company_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "primary_accountable", "accounting_quantity", "onhand_quantity"], &resource3, state.resource.as_ref().unwrap());

        let mut costs = Costs::new();
        costs.track_labor("machinist", dec!(13.2));
        costs.track_labor("trucker", dec!(29.8));
        let resource4 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource4.costs(), &costs);
        assert_eq!(resource4.inner().accounting_quantity(), &Some(Measure::new(1, Unit::One)));
        assert_eq!(resource4.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource4.inner().current_location().as_ref().unwrap().lat(), &Some(71.665519));
        assert_eq!(resource4.in_custody_of(), &company_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "primary_accountable", "accounting_quantity", "onhand_quantity", "current_location"], &resource4, state.to_resource.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.resource.as_mut().map(|x| x.inner_mut().set_primary_accountable(Some(CompanyID::new("bliv").into())));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        let mut state3 = state.clone();
        state3.resource.as_mut().map(|x| x.set_in_custody_of(CompanyID::new("bliv").into()));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        let now4 = util::time::now();
        let mut state4 = state.clone();
        state4.to_resource = None;
        let mods = event.process(state4.clone(), &now4).unwrap().into_vec();
        let resource5 = mods[1].clone().expect_op::<Resource>(Op::Create).unwrap();
        let mut resource2_clone = resource2.clone();
        resource2_clone.inner_mut().accounting_quantity_mut().as_mut().map(|x| x.set_has_numerical_value(NumericUnion::Integer(6)));
        resource2_clone.set_costs(Costs::new_with_labor("machinist", dec!(30.0)));
        resource2_clone.set_created(now4.clone());
        resource2_clone.set_updated(now4.clone());
        assert_eq!(resource5.id(), event.inner().to_resource_inventoried_as().as_ref().unwrap());
        assert_eq!(resource5, resource2_clone);
    }

    #[test]
    fn produce() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let mut event = make_event(vf::Action::Produce, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(42.0))));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 58));
        check_process_mods(vec!["costs"], &process, state.output_of.as_ref().unwrap());

        let resource = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().accounting_quantity().clone().unwrap(), Measure::new(NumericUnion::Integer(15), Unit::One));
        assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource.in_custody_of(), &company_id.clone().into());
        assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(76.91)));
        check_resource_mods(vec!["costs", "in_custody_of", "accounting_quantity", "onhand_quantity", "primary_accountable"], &resource, state.resource.as_ref().unwrap());

        let mut event = make_event(vf::Action::Produce, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(5)), Unit::One)));
        event.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        let res = event.process(state.clone(), &now);
        assert_eq!(res, Err(Error::NegativeCosts));

        let mut state2 = state.clone();
        state2.output_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn raise() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let event = make_event(vf::Action::Raise, &company_id, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 1);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().accounting_quantity().as_ref().unwrap(), &Measure::new(16 as i64, Unit::One));
        assert_eq!(resource.inner().onhand_quantity().as_ref().unwrap(), &Measure::new(17 as i64, Unit::One));
        check_resource_mods(vec!["accounting_quantity", "onhand_quantity", "primary_accountable"], &resource, state.resource.as_ref().unwrap());

        let mut event = make_event(vf::Action::Raise, &company_id, &company_id, &state, &now);
        event.inner_mut().set_resource_quantity(Some(Measure::new(NumericUnion::Decimal(dec!(-15)), Unit::One)));
        let res = event.process(state.clone(), &now);
        assert_eq!(res, Err(Error::NegativeMeasurement));
    }

    #[test]
    fn transfer() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let company2_id = CompanyID::new("alejandro's-fine-chairs");
        let state = make_state(&company_id, &company2_id, true, &now);

        let mut event = make_event(vf::Action::Transfer, &company_id, &company2_id, &state, &now);
        event.set_move_type(Some(MoveType::Resource));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource.in_custody_of(), &company_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "primary_accountable", "accounting_quantity", "onhand_quantity"], &resource, state.resource.as_ref().unwrap());

        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.inner().primary_accountable().clone().unwrap(), company2_id.clone().into());
        assert_eq!(resource2.in_custody_of(), &company2_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "primary_accountable", "accounting_quantity", "onhand_quantity"], &resource2, state.to_resource.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.resource.as_mut().map(|x| x.inner_mut().set_primary_accountable(Some(CompanyID::new("bliv").into())));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        let mut state3 = state.clone();
        state3.resource.as_mut().map(|x| x.set_in_custody_of(CompanyID::new("bliv").into()));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        let now4 = util::time::now();
        let mut state4 = state.clone();
        state4.to_resource = None;
        let mods = event.process(state4.clone(), &now4).unwrap().into_vec();
        let resource5 = mods[1].clone().expect_op::<Resource>(Op::Create).unwrap();
        let mut resource2_clone = resource2.clone();
        resource2_clone.inner_mut().accounting_quantity_mut().as_mut().map(|x| x.set_has_numerical_value(NumericUnion::Integer(6)));
        resource2_clone.set_costs(Costs::new_with_labor("machinist", dec!(30.0)));
        resource2_clone.set_created(now4.clone());
        resource2_clone.set_updated(now4.clone());
        assert_eq!(resource5.id(), event.inner().to_resource_inventoried_as().as_ref().unwrap());
        assert_eq!(resource5, resource2_clone);
    }

    #[test]
    fn transfer_all_rights() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let company2_id = CompanyID::new("alejandro's-fine-chairs");
        let state = make_state(&company_id, &company2_id, true, &now);

        let event = make_event(vf::Action::TransferAllRights, &company_id, &company2_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().primary_accountable().clone().unwrap(), company_id.clone().into());
        assert_eq!(resource.in_custody_of().clone(), company_id.clone().into());
        check_resource_mods(vec!["costs", "primary_accountable", "accounting_quantity"], &resource, state.resource.as_ref().unwrap());

        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.inner().primary_accountable().clone().unwrap(), company2_id.clone().into());
        assert_eq!(resource.in_custody_of().clone(), company_id.clone().into());
        check_resource_mods(vec!["costs", "primary_accountable", "accounting_quantity"], &resource2, state.to_resource.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.resource.as_mut().map(|x| x.inner_mut().set_primary_accountable(Some(CompanyID::new("bliv").into())));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        let mut state3 = state.clone();
        state3.resource.as_mut().map(|x| x.set_in_custody_of(CompanyID::new("bliv").into()));
        let res = event.process(state3.clone(), &now);
        assert!(res.is_ok());

        let now4 = util::time::now();
        let mut state4 = state.clone();
        state4.to_resource = None;
        let mods = event.process(state4.clone(), &now4).unwrap().into_vec();
        let resource5 = mods[1].clone().expect_op::<Resource>(Op::Create).unwrap();
        let mut resource2_clone = resource2.clone();
        resource2_clone.inner_mut().accounting_quantity_mut().as_mut().map(|x| x.set_has_numerical_value(NumericUnion::Integer(6)));
        resource2_clone.inner_mut().onhand_quantity_mut().as_mut().map(|x| x.set_has_numerical_value(NumericUnion::Integer(0)));
        resource2_clone.set_in_custody_of(company_id.clone().into());
        resource2_clone.inner_mut().set_primary_accountable(Some(company2_id.clone().into()));
        resource2_clone.set_costs(Costs::new_with_labor("machinist", dec!(30.0)));
        resource2_clone.set_created(now4.clone());
        resource2_clone.set_updated(now4.clone());
        assert_eq!(resource5.id(), event.inner().to_resource_inventoried_as().as_ref().unwrap());
        assert_eq!(resource5, resource2_clone);
    }

    #[test]
    fn transfer_custody() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let company2_id = CompanyID::new("alejandro's-fine-chairs");
        let state = make_state(&company_id, &company2_id, true, &now);

        let event = make_event(vf::Action::TransferCustody, &company_id, &company2_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.inner().primary_accountable().clone(), Some(company_id.clone().into()));
        assert_eq!(resource.in_custody_of().clone(), company_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "onhand_quantity"], &resource, state.resource.as_ref().unwrap());

        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.in_custody_of().clone(), company2_id.clone().into());
        check_resource_mods(vec!["costs", "in_custody_of", "onhand_quantity"], &resource2, state.to_resource.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.resource.as_mut().map(|x| x.inner_mut().set_primary_accountable(Some(CompanyID::new("bliv").into())));
        let res = event.process(state2.clone(), &now);
        assert!(res.is_ok());

        let mut state3 = state.clone();
        state3.resource.as_mut().map(|x| x.set_in_custody_of(CompanyID::new("bliv").into()));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        let now4 = util::time::now();
        let mut state4 = state.clone();
        state4.to_resource = None;
        let mods = event.process(state4.clone(), &now4).unwrap().into_vec();
        let resource5 = mods[1].clone().expect_op::<Resource>(Op::Create).unwrap();
        let mut resource2_clone = resource2.clone();
        resource2_clone.inner_mut().accounting_quantity_mut().as_mut().map(|x| x.set_has_numerical_value(NumericUnion::Integer(0)));
        resource2_clone.inner_mut().onhand_quantity_mut().as_mut().map(|x| x.set_has_numerical_value(NumericUnion::Integer(6)));
        resource2_clone.set_in_custody_of(company2_id.clone().into());
        resource2_clone.inner_mut().set_primary_accountable(Some(company_id.clone().into()));
        resource2_clone.set_costs(Costs::new_with_labor("machinist", dec!(30.0)));
        resource2_clone.set_created(now4.clone());
        resource2_clone.set_updated(now4.clone());
        assert_eq!(resource5.id(), event.inner().to_resource_inventoried_as().as_ref().unwrap());
        assert_eq!(resource5, resource2_clone);
    }

    #[test]
    fn useeee() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, true, &now);

        let event = make_event(vf::Action::Use, &company_id, &company_id, &state, &now);
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 2);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("machinist", 30));
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let resource = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource.in_custody_of().clone(), company_id.clone().into());
        assert_eq!(resource.costs(), &Costs::new_with_labor("machinist", dec!(4.91)));
        check_resource_mods(vec!["costs", "in_custody_of"], &resource, state.resource.as_ref().unwrap());

        let mut event2 = make_event(vf::Action::Use, &company_id, &company_id, &state, &now);
        event2.set_move_costs(Some(Costs::new_with_labor("machinist", dec!(100.000001))));
        let res = event2.process(state.clone(), &now);
        assert_eq!(res, Err(Error::NegativeCosts));

        let mut state2 = state.clone();
        state2.input_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        let mut state3 = state.clone();
        state3.resource.as_mut().map(|x| x.inner_mut().set_primary_accountable(Some(CompanyID::new("bliv").into())));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        let mut state4 = state.clone();
        state4.resource.as_mut().map(|x| x.set_in_custody_of(CompanyID::new("bliv").into()));
        let res = event.process(state4.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn work_wage() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, false, &now);

        let mut event = make_event(vf::Action::Work, &company_id, &company_id, &state, &now);
        let mut costs = Costs::new();
        costs.track_labor("CEO", 69);   // should be tracked, our member is CEO
        costs.track_labor("machinist", 42);    // should not be tracked
        event.set_move_costs(Some(costs));
        event.inner_mut().set_provider(state.provider.as_ref().unwrap().id().clone().into());
        event.inner_mut().set_effort_quantity(Some(Measure::new(dec!(0), Unit::Hour)));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 1);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor("CEO", 69));
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.input_of.as_mut().unwrap().set_deleted(Some(now.clone()));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::InputOnDeletedProcess)));

        let mut state3 = state.clone();
        state3.input_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn work_hours() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, false, &now);

        let mut event = make_event(vf::Action::Work, &company_id, &company_id, &state, &now);
        event.set_move_costs(Some(Costs::new()));
        event.inner_mut().set_provider(state.provider.as_ref().unwrap().id().clone().into());
        event.inner_mut().set_effort_quantity(Some(Measure::new(dec!(5.4), Unit::Hour)));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 1);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process.costs(), &Costs::new_with_labor_hours("CEO", dec!(5.4)));
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.input_of.as_mut().unwrap().set_deleted(Some(now.clone()));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::InputOnDeletedProcess)));

        let mut state3 = state.clone();
        state3.input_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn work_wage_and_hours() {
        let now = util::time::now();
        let company_id = CompanyID::new("jerry's-widgets-1212");
        let state = make_state(&company_id, &company_id, false, &now);

        let mut event = make_event(vf::Action::Work, &company_id, &company_id, &state, &now);
        let mut costs = Costs::new();
        costs.track_labor("CEO", 69);   // should be tracked, our member is CEO
        costs.track_labor("gerrymandering", 42);    // should not be tracked
        event.set_move_costs(Some(costs));
        event.inner_mut().set_provider(state.provider.as_ref().unwrap().id().clone().into());
        event.inner_mut().set_effort_quantity(Some(Measure::new(NumericUnion::Integer(12), Unit::Hour)));
        fuzz_state(event.clone(), state.clone(), &now);

        let res = event.process(state.clone(), &now).unwrap();
        let mods = res.into_vec();
        assert_eq!(mods.len(), 1);

        let process = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        let mut costs = Costs::new();
        costs.track_labor("CEO", 69);
        costs.track_labor_hours("CEO", 12);
        assert_eq!(process.costs(), &costs);
        check_process_mods(vec!["costs"], &process, state.input_of.as_ref().unwrap());

        let mut state2 = state.clone();
        state2.input_of.as_mut().unwrap().set_deleted(Some(now.clone()));
        let res = event.process(state2.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::InputOnDeletedProcess)));

        let mut state3 = state.clone();
        state3.input_of.as_mut().map(|x| x.set_company_id(CompanyID::new("bliv")));
        let res = event.process(state3.clone(), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }
}

