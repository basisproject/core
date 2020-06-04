//! A region is a geographical container of shared assets.

basis_model! {
    /// The region model. Pretty much a stub for now.
    pub struct Region {
        id: <<RegionID>>,
        /// This region's name.
        name: String,
    }
    RegionBuilder
}

