use crate::{
    error::{Error, Result},
    models::{
        Modifications,

        company::Company,
        lib::basis_model::Model,
        member::*,
        user::User,
    },
    util,
};

pub fn deleted_company_tester<T, F>(user: User, member: Member, company: Company, subject: Option<T>, testfn: F)
    where T: Model,
          F: Fn(User, Member, Company, Option<T>) -> Result<Modifications> + Clone
{
    let now = util::time::now();

    let mut company1 = company.clone();
    company1.set_deleted(None);
    company1.set_active(true);
    let res = testfn(user.clone(), member.clone(), company1, subject.clone());
    assert!(res.is_ok());

    let mut company2 = company.clone();
    company2.set_deleted(Some(now.clone()));
    company2.set_active(true);
    let res = testfn(user.clone(), member.clone(), company2, subject.clone());
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));

    let mut company3 = company.clone();
    company3.set_deleted(None);
    company3.set_active(false);
    let res = testfn(user.clone(), member.clone(), company3, subject.clone());
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));

    let mut company4 = company.clone();
    company4.set_deleted(Some(now.clone()));
    company4.set_active(false);
    let res = testfn(user.clone(), member.clone(), company4, subject.clone());
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));
}

pub fn permissions_checks<T, F>(user: User, member: Member, company: Company, subject: Option<T>, testfn: F)
    where T: Model,
          F: Fn(User, Member, Company, Option<T>) -> Result<Modifications> + Clone
{
    let mut member2 = member.clone();
    member2.set_permissions(vec![]);
    let res = testfn(user.clone(), member2, company.clone(), subject.clone());
    assert_eq!(res, Err(Error::InsufficientPrivileges));

    let mut user2 = user.clone();
    user2.set_roles(vec![]);
    let res = testfn(user2, member.clone(), company.clone(), subject.clone());
    assert_eq!(res, Err(Error::InsufficientPrivileges));
}

pub fn double_deleted_tester<T, F, S>(user: User, member: Member, company: Company, mut subject: T, tystr: S, testfn: F)
    where T: Model,
          F: Fn(User, Member, Company, Option<T>) -> Result<Modifications> + Clone,
          S: Into<String>
{
    subject.set_deleted(Some(util::time::now()));
    let res: Result<Modifications> = testfn(user.clone(), member.clone(), company.clone(), Some(subject));
    assert_eq!(res, Err(Error::ObjectIsDeleted(tystr.into())));
}

pub fn standard_transaction_tests<T, F>(user: User, member: Member, company: Company, subject: Option<T>, testfn: F)
    where T: Model,
          F: Fn(User, Member, Company, Option<T>) -> Result<Modifications> + Clone
{
    deleted_company_tester(user.clone(), member.clone(), company.clone(), subject.clone(), testfn.clone());
    permissions_checks(user.clone(), member.clone(), company.clone(), subject.clone(), testfn.clone());
}


