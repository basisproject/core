//! Accounts (named after "bank accounts") hold credits that are created through
//! labor.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        account::{Account, AccountID, Multisig},
        lib::basis_model::Model,
        user::{User, UserID},
    },
    system::ubi::UBIParameters,
};
use rust_decimal::prelude::*;

/// Create a new account
pub fn create<T: Into<String>>(caller: &User, id: AccountID, user_ids: Vec<UserID>, multisig: Vec<Multisig>, name: T, description: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::AccountCreate)?;
    if !user_ids.contains(caller.id()) {
        Err(Error::InsufficientPrivileges)?;
    }
    let model = Account::builder()
        .id(id)
        .user_ids(user_ids)
        .multisig(multisig)
        .name(name)
        .description(description)
        .balance(0)
        .ubi(None)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update some basic info about an account
pub fn update(caller: &User, mut subject: Account, name: Option<String>, description: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::AccountUpdate)?;
    if !subject.user_ids().contains(caller.id()) {
        Err(Error::InsufficientPrivileges)?;
    }
    if let Some(name) = name {
        subject.set_name(name);
    }
    if let Some(description) = description {
        subject.set_description(description);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Set the owners and multisig of an account.
pub fn set_owners(caller: &User, mut subject: Account, user_ids: Option<Vec<UserID>>, multisig: Option<Vec<Multisig>>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::AccountSetOwners)?;
    if !subject.user_ids().contains(caller.id()) {
        Err(Error::InsufficientPrivileges)?;
    }
    if subject.ubi().is_some() {
        Err(Error::UBIAccountError)?;
    }
    if let Some(user_ids) = user_ids {
        subject.set_user_ids(user_ids);
    }
    if let Some(multisig) = multisig {
        subject.set_multisig(multisig);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Transfer credits from one account to another.
pub fn transfer(caller: &User, mut subject: Account, mut to_account: Account, amount: Decimal, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::AccountTransfer)?;
    if !subject.user_ids().contains(caller.id()) {
        Err(Error::InsufficientPrivileges)?;
    }
    if subject.ubi().is_some() {
        Err(Error::UBIAccountError)?;
    }
    subject.adjust_balance(-amount)?;
    subject.set_updated(now.clone());
    to_account.adjust_balance(amount)?;
    to_account.set_updated(now.clone());
    let mut mods = Modifications::new();
    mods.push(Op::Update, subject);
    mods.push(Op::Update, to_account);
    Ok(mods)
}

/// Claim UBI. This reads the difference between now and the last time UBI was
/// claimed and puts the appropriate amount into the account (with an upper
/// ceiling).
///
/// Takes a systemic `UBIParameters` object which tells us how the UBI behaves.
pub fn claim_ubi(caller: &User, mut subject: Account, ubi_params: &UBIParameters, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::AccountUBIClaim)?;
    if !subject.user_ids().contains(caller.id()) {
        Err(Error::InsufficientPrivileges)?;
    }
    let ubi = if let Some(ubi) = subject.ubi().clone() {
        ubi
    } else {
        return Err(Error::UBIAccountRequired);
    };
    let days_elapsed = Decimal::from(now.timestamp() - ubi.last_claim().timestamp()) / num!(86400);
    let balance_adjustment = days_elapsed * ubi_params.balance_per_day().clone();
    subject.adjust_balance(balance_adjustment)?;
    subject.set_balance(subject.balance().clone().min(ubi_params.ceiling().clone()).normalize());
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete an account. Must have a 0 balance.
pub fn delete(caller: &User, mut subject: Account, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::AccountDelete)?;
    if !subject.user_ids().contains(caller.id()) {
        Err(Error::InsufficientPrivileges)?;
    }
    if subject.ubi().is_some() {
        Err(Error::UBIAccountError)?;
    }
    if subject.balance() != &Zero::zero() {
        Err(Error::CannotEraseCredits)?;
    }
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("account".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            account::Ubi,
        },
        util::{self, test::{self, *}},
    };

    #[test]
    fn can_create() {
        let id = AccountID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.company = None;
        state.member = None;
        let multisig = vec![Multisig::new(1)];

        let testfn = |state: &TestState<Account, Account>| {
            create(state.user(), id.clone(), vec![state.user().id().clone()], multisig.clone(), "Jerry's account", "Hi I'm Jerry", true, &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let account = mods[0].clone().expect_op::<Account>(Op::Create).unwrap();
        assert_eq!(account.id(), &id);
        assert_eq!(account.user_ids(), &vec![state.user().id().clone()]);
        assert_eq!(account.multisig(), &multisig);
        assert_eq!(account.name(), "Jerry's account");
        assert_eq!(account.description(), "Hi I'm Jerry");
        assert_eq!(account.balance(), &num!(0));
        assert_eq!(account.ubi(), &None);
        assert_eq!(account.active(), &true);
        assert_eq!(account.created(), &now);
        assert_eq!(account.updated(), &now);
        assert_eq!(account.deleted(), &None);
    }

    #[test]
    fn can_update() {
        let id = AccountID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let multisig = vec![Multisig::new(1)];
        let mods = create(state.user(), id.clone(), vec![state.user().id().clone()], multisig.clone(), "Jerry's account", "Hi I'm Jerry", true, &now).unwrap().into_vec();
        let account = mods[0].clone().expect_op::<Account>(Op::Create).unwrap();
        state.company = None;
        state.member = None;
        state.model = Some(account);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Account, Account>| {
            update(state.user(), state.model().clone(), Some("Jerry's great account".into()), Some("The best account".into()), Some(true), &now2)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let account2 = mods[0].clone().expect_op::<Account>(Op::Update).unwrap();
        assert_eq!(account2.id(), state.model().id());
        assert_eq!(account2.user_ids(), state.model().user_ids());
        assert_eq!(account2.multisig(), state.model().multisig());
        assert_eq!(account2.name(), "Jerry's great account");
        assert_eq!(account2.description(), "The best account");
        assert_eq!(account2.balance(), state.model().balance());
        assert_eq!(account2.ubi(), state.model().ubi());
        assert_eq!(account2.created(), state.model().created());
        assert_eq!(account2.updated(), &now2);
        assert_eq!(account2.deleted(), &None);

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_set_owners() {
        let id = AccountID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let multisig = vec![Multisig::new(1)];
        let mods = create(state.user(), id.clone(), vec![state.user().id().clone()], multisig.clone(), "Jerry's account", "Hi I'm Jerry", true, &now).unwrap().into_vec();
        let account = mods[0].clone().expect_op::<Account>(Op::Create).unwrap();
        state.company = None;
        state.member = None;
        state.model = Some(account);

        let now2 = util::time::now();
        let user_ids = vec![state.user().id().clone(), UserID::create()];
        let multisig2 = vec![Multisig::new(2)];
        let testfn = |state: &TestState<Account, Account>| {
            set_owners(state.user(), state.model().clone(), Some(user_ids.clone()), Some(multisig2.clone()), &now2)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let account2 = mods[0].clone().expect_op::<Account>(Op::Update).unwrap();
        assert_eq!(account2.id(), state.model().id());
        assert_eq!(account2.user_ids(), &user_ids);
        assert_eq!(account2.multisig(), &multisig2);
        assert_eq!(account2.name(), state.model().name());
        assert_eq!(account2.description(), state.model().description());
        assert_eq!(account2.balance(), state.model().balance());
        assert_eq!(account2.ubi(), state.model().ubi());
        assert_eq!(account2.created(), state.model().created());
        assert_eq!(account2.updated(), &now2);
        assert_eq!(account2.deleted(), &None);

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_transfer() {
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let account1 = make_account(&AccountID::create(), state.user().id(), num!(50), "Jerry's account", &now);
        let account2 = make_account(&AccountID::create(), &UserID::create(), num!(0), "Larry's account", &now);
        state.company = None;
        state.member = None;
        state.model = Some(account1);
        state.model2 = Some(account2);

        let now2 = util::time::now();
        let testfn_inner = |state: &TestState<Account, Account>, amount: Decimal| {
            transfer(state.user(), state.model().clone(), state.model2().clone(), amount, &now2)
        };
        let testfn = |state: &TestState<Account, Account>| {
            testfn_inner(state, num!(10))
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let account3 = mods[0].clone().expect_op::<Account>(Op::Update).unwrap();
        assert_eq!(account3.balance(), &num!(40));
        assert_eq!(account3.id(), state.model().id());
        assert_eq!(account3.user_ids(), state.model().user_ids());
        assert_eq!(account3.multisig(), state.model().multisig());
        assert_eq!(account3.name(), state.model().name());
        assert_eq!(account3.description(), state.model().description());
        assert_eq!(account3.ubi(), state.model().ubi());
        assert_eq!(account3.created(), state.model().created());
        assert_eq!(account3.updated(), &now2);
        assert_eq!(account3.deleted(), &None);
        let account4 = mods[1].clone().expect_op::<Account>(Op::Update).unwrap();
        assert_eq!(account4.balance(), &num!(10));
        assert_eq!(account4.id(), state.model2().id());
        assert_eq!(account4.user_ids(), state.model2().user_ids());
        assert_eq!(account4.multisig(), state.model2().multisig());
        assert_eq!(account4.name(), state.model2().name());
        assert_eq!(account4.description(), state.model2().description());
        assert_eq!(account4.ubi(), state.model2().ubi());
        assert_eq!(account4.created(), state.model2().created());
        assert_eq!(account4.updated(), &now2);
        assert_eq!(account4.deleted(), &None);

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        state2.user_mut().set_id(state.model2().user_ids()[0].clone());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let res = testfn_inner(&state, num!(56));
        assert_eq!(res, Err(Error::NegativeAccountBalance));
    }

    #[test]
    fn can_claim_ubi() {
        let id = AccountID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let multisig = vec![Multisig::new(1)];
        let mods = create(state.user(), id.clone(), vec![state.user().id().clone()], multisig.clone(), "UBI", "UBI", true, &now).unwrap().into_vec();
        let mut account = mods[0].clone().expect_op::<Account>(Op::Create).unwrap();
        let mut ubi_params = UBIParameters::new();
        ubi_params.set_ceiling(num!(100));
        ubi_params.set_balance_per_day(num!(900) / num!(30));
        account.set_ubi(Some(Ubi::new("2020-01-01T00:00:00Z".parse().unwrap())));
        state.company = None;
        state.member = None;
        state.model = Some(account);

        let now2 = "2020-01-06T12:30:00Z".parse().unwrap();
        let testfn_inner = |state: &TestState<Account, Account>, ubi_params: &UBIParameters| {
            claim_ubi(state.user(), state.model().clone(), ubi_params, &now2)
        };
        let testfn = |state: &TestState<Account, Account>| {
            testfn_inner(state, &ubi_params)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let account2 = mods[0].clone().expect_op::<Account>(Op::Update).unwrap();
        assert_eq!(account2.id(), state.model().id());
        assert_eq!(account2.balance(), &num!(100));
        assert_eq!(account2.created(), state.model().created());
        assert_eq!(account2.updated(), &now2);
        assert_eq!(account2.deleted(), &None);

        let mut ubi_params2 = ubi_params.clone();
        ubi_params2.set_ceiling(num!(5000));
        let mods = testfn_inner(&state, &ubi_params2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let account3 = mods[0].clone().expect_op::<Account>(Op::Update).unwrap();
        assert_eq!(account3.id(), state.model().id());
        assert_eq!(account3.balance(), &num!(165.625));
        assert_eq!(account3.created(), state.model().created());
        assert_eq!(account3.updated(), &now2);

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = AccountID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let multisig = vec![Multisig::new(1)];
        let mods = create(state.user(), id.clone(), vec![state.user().id().clone()], multisig.clone(), "Jerry's account", "Hi I'm Jerry", true, &now).unwrap().into_vec();
        let account = mods[0].clone().expect_op::<Account>(Op::Create).unwrap();
        state.company = None;
        state.member = None;
        state.model = Some(account);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Account, Account>| {
            delete(state.user(), state.model().clone(), &now2)
        };
        test::standard_transaction_tests(&state, &testfn);
        test::double_deleted_tester(&state, "account", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let account2 = mods[0].clone().expect_op::<Account>(Op::Delete).unwrap();
        assert_eq!(account2.id(), state.model().id());
        assert_eq!(account2.user_ids(), state.model().user_ids());
        assert_eq!(account2.multisig(), state.model().multisig());
        assert_eq!(account2.name(), state.model().name());
        assert_eq!(account2.description(), state.model().description());
        assert_eq!(account2.balance(), state.model().balance());
        assert_eq!(account2.ubi(), state.model().ubi());
        assert_eq!(account2.created(), state.model().created());
        assert_eq!(account2.updated(), state.model().updated());
        assert_eq!(account2.deleted(), &Some(now2.clone()));

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state3 = state.clone();
        state3.model_mut().set_balance(num!(21.55));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::CannotEraseCredits));
    }
}

