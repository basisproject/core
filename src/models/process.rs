//! Processes are aggregators of costs via their inputs, labor and resources,
//! and dividers/subtractors of costs via their outputs, resources and services.

use crate::{
    costs::{Costs, CostMover},
    models::{
        company::CompanyID,
        lib::agent::AgentID,
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
        inner: vf::Process<ProcessSpecID, Url, AgentID, (), ()>,
        /// The company this process belongs to
        company_id: CompanyID,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::CompanyID,
        },
        util::{self, test::*},
    };
    use rust_decimal_macros::*;

    #[test]
    fn compare() {
        let now = util::time::now();
        let id1 = ProcessID::new("widget1");
        let id2 = ProcessID::new("widget2");
        let company_id1 = CompanyID::new("jerry's widgets");
        let company_id2 = CompanyID::new("frank's widgets");
        let costs = Costs::new_with_labor("machinist", dec!(23.2));
        let mut costs2 = costs.clone();
        costs2.track_labor("mayor", dec!(2.4));
        let process1 = make_process(&id1, &company_id1, "make widgets", &costs, &now);
        let process2 = make_process(&id2, &company_id2, "burn widgets", &costs, &now);

        assert!(process1 == process1);
        assert!(process2 == process2);
        assert!(process1.clone() == process1.clone());
        assert!(process2.clone() == process2.clone());
        assert!(process1 != process2);
        let mut process3 = process2.clone();
        assert!(process1 != process3);
        process3.set_id(id1.clone());
        assert!(process1 != process3);
        process3.inner_mut().set_name("make widgets".into());
        assert!(process1 != process3);
        process3.set_company_id(company_id1.clone().into());
        assert!(process1 == process3);
        process3.set_costs(costs2);
        assert!(process1 != process3);
        process3.set_costs(Costs::new_with_labor("machinist", dec!(23.2)));
        assert!(process1 == process3);
    }
}

