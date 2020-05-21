use crate::{
    costs::Costs,
    models::{
        company::CompanyID,
        process_spec::ProcessSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Process {
        process: vf::Process<ProcessSpecID, Url, CompanyID, (), ()>,
        costs: Costs,
    }
    ProcessID
    ProcessBuilder
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cost_tracking() {
        let costs = Costs::new();

        // TODO: take the following:
        //
        // - inputs
        // - outputs
        // - cost tags ...or something similar?
        // - amortization
        //
        // assign the costs of the inputs to the outputs either equally or in
        // proportion to the cost tags.
    }
}

