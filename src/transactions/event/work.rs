//! Work events record labor in the system. They are how labor costs (both waged
//! and hourly labor) get recorded in the system, and also act as the systemic
//! marker for paying company members. Record labor, get paid.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        event::{Event, EventID, EventProcessState, LaborType},
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        process::Process,
        user::User,
    },
};
use vf_rs::vf;

pub fn work_wage_and_hours(caller: &User, member: &CompanyMember, company: &Company, id: EventID, process: Process, costs: Costs, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Work)?;
    let state = EventProcessState::builder()
        .input_of(process)
        .provider(member.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .agreed_in(member.agreement().clone())
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(costs))
        .labor_type(LaborType::WageAndHours)
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let evmods = event.process(state, now)?.modifications();
    let mut mods = Modifications::new();
    mods.push(Op::Create, event);
    for evmod in evmods {
        mods.push_raw(evmod);
    }
    Ok(mods)
}

#[cfg(test)]
mod tests {
}

