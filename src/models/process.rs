//! Processes are aggregators of costs via their inputs, labor and resources,
//! and dividers/subtractors of costs via their outputs, resources and services.

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
    /// The `Process` model wraps the [vf::Process][vfprocess] object and adds
    /// cost tracking in. Processes are the places where inputs are transformed
    /// into outputs.
    ///
    /// Processes must reference a [ProcessSpec], which acts as a grouping of
    /// processes of similar type, but can also (in special cases) act as a
    /// transformer of various tracked raw resources (ie crude oil -> diesel,
    /// jet fuel, etc).
    ///
    /// [vfprocess]: https://valueflo.ws/introduction/processes.html
    /// [ProcessSpec]: ../process_spec/struct.ProcessSpec.html
    pub struct Process {
        id: <<ProcessID>>,
        /// The inner VF process
        inner: vf::Process<ProcessSpecID, Url, CompanyID, (), ()>,
        /// Our costs tally for this process
        costs: Costs,
    }
    ProcessBuilder
}

impl CostMover for Process {
    fn costs(&self) -> &Costs {
        self.costs()
    }

    fn set_costs(&mut self, costs: Costs) {
        self.set_costs(costs);
    }
}

