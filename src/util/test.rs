use crate::{
    error::{Error, Result},
    models::{
        Modifications,

        company::Company,
        member::*,
        user::User,
    },
    util,
};

pub fn deleted_company_tester<F>(user: User, member: Member, company: Company, testfn: F)
    where F: Fn(User, Member, Company) -> Result<Modifications>
{
    let now = util::time::now();

    let mut company1 = company.clone();
    company1.set_deleted(None);
    company1.set_active(true);
    let res = testfn(user.clone(), member.clone(), company1);
    assert!(res.is_ok());

    let mut company2 = company.clone();
    company2.set_deleted(Some(now.clone()));
    company2.set_active(true);
    let res = testfn(user.clone(), member.clone(), company2);
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));

    let mut company3 = company.clone();
    company3.set_deleted(None);
    company3.set_active(false);
    let res = testfn(user.clone(), member.clone(), company3);
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));

    let mut company4 = company.clone();
    company4.set_deleted(Some(now.clone()));
    company4.set_active(false);
    let res = testfn(user.clone(), member.clone(), company4);
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));
}

pub fn permissions_checks<F>(user: User, member: Member, company: Company, testfn: F)
    where F: Fn(User, Member, Company) -> Result<Modifications>
{
    let mut member2 = member.clone();
    member2.set_permissions(vec![]);
    let res = testfn(user.clone(), member2, company.clone());
    assert_eq!(res, Err(Error::InsufficientPrivileges));

    let mut user2 = user.clone();
    user2.set_roles(vec![]);
    let res = testfn(user2, member.clone(), company.clone());
    assert_eq!(res, Err(Error::InsufficientPrivileges));
}

pub fn standard_transaction_tests<F>(user: User, member: Member, company: Company, testfn: F)
    where F: Fn(User, Member, Company) -> Result<Modifications> + Clone
{
    deleted_company_tester(user.clone(), member.clone(), company.clone(), testfn.clone());
    permissions_checks(user.clone(), member.clone(), company.clone(), testfn.clone());
}


