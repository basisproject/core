use serde::{Serialize, Deserialize};
use vf_rs::vf::Agent;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompanyType {
    Public,
    Syndicate,
    Private,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    All,
    AllBut(Vec<Permission>),

    CompanyUpdate,
    CompanyDelete,

    MemberCreate,
    MemberSetRoles,
    MemberDelete,

    LaborSetClock,
    LaborTagCost,
    LaborSetWage,

    ProductCreate,
    ProductUpdate,
    ProductDelete,
    ProductTagCost,

    OrderCreate,
    OrderUpdateProcessStatus,
    OrderUpdateCostTags,
    OrderUpdateShipping,
    OrderUpdateShippingDates,
    OrderCancel,
    OrderTagCost,

    CostTagCreate,
    CostTagUpdate,
    CostTagDelete,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Role {
    Owner,
    Admin,
    MemberAdmin,
    LaborAdmin,
    ProductAdmin,
    CostTagAdmin,
    CostTaggerAdmin,
    CostTaggerLabor,
    CostTaggerProduct,
    CostTaggerOrder,
    Purchaser,
    Supplier,
}

impl Role {
    pub fn permissions(&self) -> Vec<Permission> {
        match *self {
            Role::Owner => {
                vec![Permission::All]
            }
            Role::Admin => {
                vec![
                    Permission::AllBut(vec![Permission::CompanyDelete]),
                ]
            }
            Role::MemberAdmin => {
                vec![
                    Permission::MemberCreate,
                    Permission::MemberSetRoles,
                    Permission::MemberDelete,
                ]
            }
            Role::LaborAdmin => {
                vec![
                    Permission::LaborSetClock,
                    Permission::LaborSetWage,
                ]
            }
            Role::ProductAdmin => {
                vec![
                    Permission::ProductCreate,
                    Permission::ProductUpdate,
                    Permission::ProductDelete,
                ]
            }
            Role::CostTagAdmin => {
                vec![
                    Permission::CostTagCreate,
                    Permission::CostTagUpdate,
                    Permission::CostTagDelete,
                ]
            }
            Role::CostTaggerAdmin => {
                vec![
                    Permission::LaborTagCost,
                    Permission::ProductTagCost,
                    Permission::OrderTagCost,
                ]
            }
            Role::CostTaggerLabor => {
                vec![
                    Permission::LaborTagCost,
                ]
            }
            Role::CostTaggerProduct => {
                vec![
                    Permission::ProductTagCost,
                ]
            }
            Role::CostTaggerOrder => {
                vec![
                    Permission::OrderTagCost,
                ]
            }
            Role::Purchaser => {
                vec![
                    Permission::OrderCreate,
                    Permission::OrderUpdateCostTags,
                    Permission::OrderCancel,
                ]
            }
            Role::Supplier => {
                vec![
                    Permission::OrderUpdateProcessStatus,
                    Permission::OrderCancel,
                ]
            }
        }
    }

    pub fn can(&self, perm: &Permission) -> bool {
        for p in &self.permissions() {
            match p {
                Permission::All => {
                    return true;
                }
                Permission::AllBut(x) => {
                    if x.contains(perm) {
                        return false;
                    }
                    return true;
                }
                _ => {
                    if p == perm {
                        return true;
                    }
                }
            }
        }
        false
    }
}

basis_model! {
    pub struct Company {
        agent: Agent,
        ty: CompanyType,
        #[builder(setter(strip_option), default)]
        region_id: Option<String>,
        email: String,
    }
    CompanyBuilder
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn permissions_work() {
        let owner = Role::Owner;
        assert!(owner.can(&Permission::All));
        assert!(owner.can(&Permission::CompanyUpdate));
        assert!(owner.can(&Permission::CompanyDelete));
        assert!(owner.can(&Permission::MemberCreate));
        assert!(owner.can(&Permission::MemberSetRoles));
        assert!(owner.can(&Permission::MemberDelete));
        assert!(owner.can(&Permission::ProductCreate));
        assert!(owner.can(&Permission::ProductUpdate));
        assert!(owner.can(&Permission::ProductDelete));
        assert!(owner.can(&Permission::OrderCreate));
        assert!(owner.can(&Permission::OrderUpdateProcessStatus));
        assert!(owner.can(&Permission::OrderUpdateCostTags));
        assert!(owner.can(&Permission::OrderCancel));

        let admin = Role::Admin;
        assert!(admin.can(&Permission::CompanyUpdate));
        assert!(!admin.can(&Permission::CompanyDelete));
        assert!(admin.can(&Permission::MemberCreate));
        assert!(admin.can(&Permission::MemberSetRoles));
        assert!(admin.can(&Permission::MemberDelete));
        assert!(admin.can(&Permission::ProductCreate));
        assert!(admin.can(&Permission::ProductUpdate));
        assert!(admin.can(&Permission::ProductDelete));
        assert!(admin.can(&Permission::OrderCreate));
        assert!(admin.can(&Permission::OrderUpdateProcessStatus));
        assert!(admin.can(&Permission::OrderUpdateCostTags));
        assert!(admin.can(&Permission::OrderCancel));

        let member_admin = Role::MemberAdmin;
        assert!(!member_admin.can(&Permission::CompanyUpdate));
        assert!(!member_admin.can(&Permission::CompanyDelete));
        assert!(member_admin.can(&Permission::MemberCreate));
        assert!(member_admin.can(&Permission::MemberSetRoles));
        assert!(member_admin.can(&Permission::MemberDelete));
        assert!(!member_admin.can(&Permission::ProductCreate));
        assert!(!member_admin.can(&Permission::ProductUpdate));
        assert!(!member_admin.can(&Permission::ProductDelete));
        assert!(!member_admin.can(&Permission::OrderCreate));
        assert!(!member_admin.can(&Permission::OrderUpdateProcessStatus));
        assert!(!member_admin.can(&Permission::OrderUpdateCostTags));
        assert!(!member_admin.can(&Permission::OrderCancel));
    }
}

