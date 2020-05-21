basis_model! {
    /// Acts as a group for various products classified as resources.
    ///
    /// For instance, a group might be "iron", and all the iron produced by iron
    /// mines might link to the group.
    pub struct ResourceGroup {
        /// The name of the group, generally will be some easily-identifiable
        /// resource name like "iron" or "silicon" or "fresh water"
        name: String,
        /// The globally-decided cost (in credits) for products under this group.
        credit_cost_per_unit: f64,
    }
    ResourceGroupID
    ResourceGroupBuilder
}

