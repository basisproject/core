use crate::{
    models::{
        resource_spec::ResourceSpecID,
        resource_group::ResourceGroupID,
    },
};
basis_model! {
    pub struct ResourceGroupLink {
        /// The ID of the resource group.
        group_id: ResourceGroupID,
        /// The ID of the product we're linking to the group.
        product_id: ResourceSpecID,
        // TODO: at some point, store meta information about the resource
        // quantity/renewal/depletion/etc
    }
    ResourceGroupLinkID
    ResourceGroupLinkBuilder
}

