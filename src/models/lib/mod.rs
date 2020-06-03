/// A macro that standardizes including, exporting, and creating wrapper type(s)
/// for our heroic models.
macro_rules! load_models {
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
        /// An enum that allows returning *any* model type. This is mainly used
        /// along with [Op](enum.Op.html) to specify modifications (ie
        /// `[Op::Create, User]`).
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
            (account, Account, AccountID),
            (agreement, Agreement, AgreementID),
            (commitment, Commitment, CommitmentID),
            (company, Company, CompanyID),
            (company_member, CompanyMember, CompanyMemberID),
            (currency, Currency, CurrencyID),
            (event, Event, EventID),
            (intent, Intent, IntentID),
            (occupation, Occupation, OccupationID),
            (process, Process, ProcessID),
            (process_spec, ProcessSpec, ProcessSpecID),
            (region, Region, RegionID),
            (resource, Resource, ResourceID),
            (resource_spec, ResourceSpec, ResourceSpecID, Dimensions),
            (user, User, UserID),

            //(resource_group, ResourceGroup, ResourceGroupID),
            //(resource_group_link, ResourceGroupLink, ResourceGroupLinkID),
        }
    };
}

#[macro_use]
pub mod basis_model;
pub mod agent;

