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
        user::{User, UserID},
    },
};

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
        .ubi(false)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        util::{self, test::{self, *}},
    };
    use rust_decimal_macros::*;

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
        assert_eq!(account.balance(), &dec!(0));
        assert_eq!(account.ubi(), &false);
        assert_eq!(account.active(), &true);
        assert_eq!(account.created(), &now);
        assert_eq!(account.updated(), &now);
        assert_eq!(account.deleted(), &None);
    }
}

