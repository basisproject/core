//! Process specifications group `Process` models, such that the process spec
//! might be "Make coat" and the process itself would be an *instance* of "Make
//! coat" that happened at a specific time with a specific set of inputs and
//! outputs which are used for cost tracking.
//!
//! Process specifications can also contain resource transformations (such as
//! turning iron into steel). In effect, the transformation acts to *consume*
//! the input resource, whereas in most cases processes just move resources.

use vf_rs::vf;

basis_model! {
    /// The `ProcessSpec` model 
    pub struct ProcessSpec {
        id: <<ProcessSpecID>>,
        /// Our VF process object.
        inner: vf::ProcessSpecification,
        // TODO: implement some concept of a known transformation (ie, refining
        // crude oil)
        //resource_transform: Option<ResourceTransformProcessID>,
    }
    ProcessSpecBuilder
}

