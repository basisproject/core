//! Resource specifications are meta descriptions of `Resource` objects. If you
//! buy a "Zenith Men's 96.0529.4035/51.M Defy Xtreme Tourbillon Titanium
//! Chronograph Watch" on Wamazon, the watch you get in the mail is the resource
//! and the *resource specification* is the Wamazon product description page.

use crate::{
    models::{
        company::CompanyID,
    },
};
use getset::Getters;
use serde::{Serialize, Deserialize};
use url::Url;
use vf_rs::vf;

/// Physical dimensions for this resource.
#[derive(Clone, Debug, Default, PartialEq, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Dimensions {
    /// I refuse to
    width: f64,
    /// document these
    height: f64,
    /// struct fields.
    length: f64,
}

impl Dimensions {
    /// Create a new dimension set.
    pub fn new(width: f64, height: f64, length: f64) -> Self {
        Self {
            width,
            height,
            length,
        }
    }
}

basis_model! {
    /// The `ResourceSpec` model wraps our heroic [vf::ResourceSpecification][vfresource]
    /// object, with one addition: we add a `CompanyID`, which effectively acts
    /// to namespace resource specifications on a per-company basis.
    ///
    /// [vfresource]: https://valueflo.ws/introduction/resources.html
    pub struct ResourceSpec {
        id: <<ResourceSpecID>>,
        inner: vf::ResourceSpecification<Url>,
        /// products are namespaced by company_id. we have no interest in trying
        /// to classify some chair as a Chair that anyone can build, but rather
        /// only as a chair built by a specific company.
        ///
        /// if we want to group products together, we certainly can, but this is
        /// not the place for it.
        company_id: CompanyID,
    }
    ResourceSpecBuilder
}

