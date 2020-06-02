use crate::{
    costs::{Costs, CostMover},
    models::{
        company::CompanyID,
        process_spec::ProcessSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Process {
        id: <<ProcessID>>,
        /// The inner VF process
        inner: vf::Process<ProcessSpecID, Url, CompanyID, (), ()>,
        /// Our costs tally for this process
        costs: Costs,
    }
    ProcessBuilder
}

impl Process {
    pub fn track_costs(&mut self, costs: Costs) {
        self.set_costs(self.costs().clone() + costs);
    }
}

impl CostMover for Process {
    fn costs(&self) -> &Costs {
        self.costs()
    }

    fn set_costs(&mut self, costs: Costs) {
        self.set_costs(costs);
    }
}

