use vf_rs::vf;

basis_model! {
    pub struct ProcessSpec {
        process_spec: vf::ProcessSpecification,
        // TODO: implement some concept of a known transformation (ie, refining
        // crude oil)
        //resource_transform: Option<ResourceTransformProcessID>,
    }
    ProcessSpecID
    ProcessSpecBuilder
}

