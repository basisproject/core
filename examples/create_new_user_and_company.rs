use basis_core::{
    error::Result,
    models::{
        company::{Company, CompanyID},
        member::{Member, MemberClass, MemberID, MemberWorker},
        occupation::{Occupation, OccupationID},
        user::{User, UserID},
        Op,
    },
    system::vote::Vote,
    transactions::{company, occupation, user},
};
use chrono::Utc;

/// Normally our system would be seeded with occupation data already, but we're
/// starting with a blank slate so we need to add an occupation.
fn create_voted_occupation(label: &str) -> Result<Occupation> {
    let voter = Vote::systemic(UserID::create(), &Utc::now())?;
    let mods = occupation::create(
        voter.user(),
        OccupationID::create(),
        label.into(),
        "Adding our first occupation",
        true,
        &Utc::now(),
    )?
    .into_vec();
    mods[0].clone().expect_op::<Occupation>(Op::Create)
}

fn example() -> Result<(User, Member, Company)> {
    // run the user creation transaction and grab our user from the modification
    // list.
    //
    // transactions don't pass back models, but rather modifications on models
    // (aka, "add User" or "Update company" or "delete Member" etc).
    let mods = user::create(
        UserID::create(),
        "user-8171287127nnx78.233b2c@basisproject.net",
        "Jerry",
        true,
        &Utc::now(),
    )?
    .into_vec();
    let user = mods[0].clone().expect_op::<User>(Op::Create)?;

    // create our first occupation (by democratic vote)
    let occupation = create_voted_occupation("President")?;

    // now create our company, which also creates a member record that links the
    // calling user to the company as a worker
    let founder = company::Founder::new(
        MemberID::create(),
        MemberClass::Worker(MemberWorker::new(occupation.id().clone(), None)),
        true,
    );
    let mods = company::create(
        &user,
        CompanyID::create(),
        "Widget Extravaganza",
        "info@widgetextravaganza.com",
        true,
        founder,
        &Utc::now(),
    )?
    .into_vec();
    let company = mods[0].clone().expect_op::<Company>(Op::Create)?;
    let member = mods[1].clone().expect_op::<Member>(Op::Create)?;
    Ok((user, member, company))
}

fn main() {
    let (user, member, company) = example().unwrap();
    println!(
        "Hi, {}, founder of {} (member {}), I'm Dad!",
        user.name(),
        company.inner().name(),
        member.id().as_str()
    );
}
