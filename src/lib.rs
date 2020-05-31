/// A macro that standardizes including, exporting, and creating wrapper type(s)
/// for our heroic models.
macro_rules! load_models {
    (
        @pub use
        $( ($path:ident, $model:ident, $($extratypes:ident),*), )*
    ) => {
        pub use models::{
            $( $path::{$model, $($extratypes),*}, )*
        };
    };

    (
        @pub mod
        $( ($path:ident, $($_rest:tt)*), )*
    ) => {
        $(
            pub mod $path;
        )*
    };

    // create an enum that wraps our models in CUD
    (
        @pub enum $enumname:ident
        $( ($path:ident, $model:ident, $($_extratypes:ident),*), )*
    ) => {
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        pub enum $enumname {
            $(
                $model(crate::models::$path::$model),
            )*
        }
    };

    // entry point
    ($($load_type:tt)*) => {
        load_models! {
            @$($load_type)*
            // kind of trying to load based on dependency order here, but it's not perfect.
            (region, Region, RegionID),
            (user, User, UserID),
            (occupation, Occupation, OccupationID),
            (currency, Currency, CurrencyID),
            (company, Company, CompanyID),
            (process_spec, ProcessSpec, ProcessSpecID),
            (process, Process, ProcessID),
            (event, Event, EventID),
            (company_member, CompanyMember, CompanyMemberID),
            (agreement, Agreement, AgreementID),
            (account, Account, AccountID),
            (resource_spec, ResourceSpec, ResourceSpecID, Dimensions),
            (resource, Resource, ResourceID),
            (commitment, Commitment, CommitmentID),
            //(resource_group, ResourceGroup, ResourceGroupID),
            //(resource_group_link, ResourceGroupLink, ResourceGroupLinkID),
        }
    };
}

pub mod error;
mod util;
#[macro_use]
mod access;
mod models;
pub mod costs;
pub mod transactions;

load_models!{ pub use }

