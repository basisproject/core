use vf_rs::vf;

basis_model! {
    pub struct ProcessSpec {
        process_spec: vf::ProcessSpecification,
        // TODO: process specs will have quanties of known inputs/outputs mainly
        // used for resource transformations
    }
    ProcessSpecID
    ProcessSpecBuilder
}

