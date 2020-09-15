use basis_core::{
    error::Result,
    models::{
        Op,

        account::AccountID,
        company::{Company, CompanyID},
        member::{Member, MemberID, MemberClass, MemberWorker},
        occupation::{Occupation, OccupationID},
        user::{User, UserID},
    },
    transactions::{
        company,
        occupation,
        user,
    },
    system::vote::Vote,
};
use chrono::Utc;

/// Normally our system would be seeded with occupation data already, but we're
/// starting with a blank slate so we need to add an occupation.
fn create_voted_occupation(label: &str) -> Result<Occupation> {
    let voter = Vote::systemic(UserID::new("f8636701-2ec0-46e9-bff3-5cff3d7f97cf"), &Utc::now())?;
    let mods = occupation::create(voter.user(), OccupationID::new("e8677b3c-e125-4fb2-8cf1-04bcdae162b7"), label.into(), "Adding our first occupation", true, &Utc::now())?.into_vec();
    mods[0].clone().expect_op::<Occupation>(Op::Create)
}

fn example() -> Result<(User, Member, Company)> {
    // run the user creation transaction and grab our user from the modification
    // list.
    //
    // transactions don't pass back models, but rather modifications on models
    // (aka, "add User" or "Update company" or "delete Member" etc).
    let mods = user::create(UserID::new("389f9613-1ac6-435d-9d73-e96118e0ea71"), "user-8171287127nnx78.233b2c@basisproject.net", "Jerry", AccountID::new("9b7c9b40-b759-4a15-8615-4012be92f06a"), true, &Utc::now())?.into_vec();
    let user = mods[0].clone().expect_op::<User>(Op::Create)?;

    // create our first occupation (by democratic vote)
    let occupation = create_voted_occupation("President")?;

    // now create our company, which also creates a member record that links the
    // calling user to the company as a worker
    let founder = company::Founder::new(MemberID::new("7100be67-2ac1-4b83-b7af-6fec9294c4ee"), MemberClass::Worker(MemberWorker::new(occupation.id().clone(), None)), true);
    let mods = company::create(&user, CompanyID::new("89e1aff5-84c7-4e84-b3d6-f7f5924d0e53"), "Widget Extravaganza", "info@widgetextravaganza.com", true, founder, &Utc::now())?.into_vec();
    let company = mods[0].clone().expect_op::<Company>(Op::Create)?;
    let member = mods[1].clone().expect_op::<Member>(Op::Create)?;
    Ok((user, member, company))
}

fn main() {
    let (user, member, company) = example().unwrap();
    println!("Hi, {}, founder of {} (member {}), I'm Dad!", user.name(), company.inner().name(), member.id().as_str());
}

