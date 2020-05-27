use crate::{
    models::{
        company::CompanyID,
    },
};
use getset::{Getters, Setters};
use serde::{Serialize, Deserialize};
use url::Url;
use vf_rs::vf;

#[derive(Clone, Debug, Default, PartialEq, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub")]
pub struct Dimensions {
    width: f64,
    height: f64,
    length: f64,
}

impl Dimensions {
    pub fn new(width: f64, height: f64, length: f64) -> Self {
        Self {
            width,
            height,
            length,
        }
    }
}

basis_model! {
    pub struct ResourceSpec {
        inner: vf::ResourceSpecification<Url>,
        /// products are namespaced by company_id. we have no interest in trying
        /// to classify some chair as a Chair that anyone can build, but rather
        /// only as a chair built by a specific company.
        ///
        /// if we want to group products together, we certainly can, but this is
        /// not the place for it.
        company_id: CompanyID,
    }
    ResourceSpecID
    ResourceSpecBuilder
}

