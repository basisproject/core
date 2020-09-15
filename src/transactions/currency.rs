//! Currencies track real-world market currencies in the cost tracking system.
//!
//! This set of transactions deals with creating currencies tracked by Basis,
//! such as USD, EUR, etc.
//!
//! See the [currency model.][1]
//!
//! [1]: ../../models/currency/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        currency::{Currency, CurrencyID},
        lib::basis_model::Model,
        user::User,
    },
};

/// Create a new `Currency`.
pub fn create<T: Into<String>>(caller: &User, id: CurrencyID, name: T, decimal_places: u8, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CurrencyCreate)?;
    let model = Currency::builder()
        .id(id)
        .name(name.into())
        .decimal_places(decimal_places)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update an existing `Currency`
pub fn update(caller: &User, mut subject: Currency, name: Option<String>, decimal_places: Option<u8>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CurrencyUpdate)?;
    if let Some(name) = name {
        subject.set_name(name);
    }
    if let Some(decimal_places) = decimal_places {
        subject.set_decimal_places(decimal_places);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a `Currency`
pub fn delete(caller: &User, mut subject: Currency, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CurrencyDelete)?;
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("currency".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
            Op,

            currency::Currency,
        },
        util::{self, test::{self, *}},
    };

    #[test]
    fn can_create() {
        let id = CurrencyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.user_mut().set_roles(vec![Role::SuperAdmin]);

        let testfn = |state: &TestState<Currency, Currency>| {
            create(state.user(), id.clone(), "usd", 2, true, &now)
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let currency = mods[0].clone().expect_op::<Currency>(Op::Create).unwrap();
        assert_eq!(currency.id(), &id);
        assert_eq!(currency.name(), "usd");
        assert_eq!(currency.decimal_places(), &2);
        assert_eq!(currency.active(), &true);
        assert_eq!(currency.created(), &now);
        assert_eq!(currency.updated(), &now);
        assert_eq!(currency.deleted(), &None);

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let id = CurrencyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.user_mut().set_roles(vec![Role::SuperAdmin]);
        let mods = create(state.user(), id.clone(), "usdz", 1, false, &now).unwrap().into_vec();
        let currency = mods[0].clone().expect_op::<Currency>(Op::Create).unwrap();
        state.model = Some(currency);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Currency, Currency>| {
            update(state.user(), state.model().clone(), Some("usd".into()), Some(2), Some(true), &now2)
        };

        // not truly an update but ok
        let mods = testfn(&state).unwrap().into_vec();
        let currency2 = mods[0].clone().expect_op::<Currency>(Op::Update).unwrap();
        assert_eq!(currency2.id(), state.model().id());
        assert_eq!(currency2.name(), "usd");
        assert_eq!(currency2.decimal_places(), &2);
        assert_eq!(currency2.active(), &true);
        assert_eq!(currency2.created(), state.model().created());
        assert_eq!(currency2.created(), &now);
        assert_eq!(currency2.updated(), &now2);
        assert_eq!(currency2.deleted(), &None);

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = CurrencyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.user_mut().set_roles(vec![Role::SuperAdmin]);
        let mods = create(state.user(), id.clone(), "usd", 2, true, &now).unwrap().into_vec();
        let currency = mods[0].clone().expect_op::<Currency>(Op::Create).unwrap();
        state.model = Some(currency);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Currency, Currency>| {
            delete(state.user(), state.model().clone(), &now2)
        };
        test::double_deleted_tester(&state, "currency", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let currency2 = mods[0].clone().expect_op::<Currency>(Op::Delete).unwrap();
        assert_eq!(currency2.id(), state.model().id());
        assert_eq!(currency2.name(), "usd");
        assert_eq!(currency2.decimal_places(), &2);
        assert_eq!(currency2.active(), &true);
        assert_eq!(currency2.created(), state.model().created());
        assert_eq!(currency2.updated(), state.model().updated());
        assert_eq!(currency2.deleted(), &Some(now2));

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

