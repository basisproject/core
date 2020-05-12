use crate::{
    models::costs::Costs,
};
use getset::{Getters, CopyGetters};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

basis_model!{
    pub struct CostTag {
        company_id: String,
        name: String,
    }
    CostTagBuilder
}

/// Allows an object to link to a cost tag by id, using a weight value.
#[derive(Clone, Debug, PartialEq, Getters, CopyGetters, Serialize, Deserialize)]
pub struct CostTagLink {
    #[getset(get = "pub")]
    pub cost_tag_id: String,
    #[getset(get_copy = "pub")]
    pub weight: u64,
}

impl CostTagLink {
    pub fn new<T>(cost_tag_id: T, weight: u64) -> Self
        where T: Into<String>
    {
        Self {
            cost_tag_id: cost_tag_id.into(),
            weight,
        }
    }
}

pub trait Costable {
    /// Get the costs for this object
    fn get_costs(&self) -> Costs;

    /// Get the cost tags for this object
    fn get_cost_tags(&self) -> Vec<CostTagLink>;

    /// Add this object's tagged costs to an existing hash (that contains tagged
    /// costs)
    fn tally_tagged_costs(&self, cost_collection: &mut HashMap<String, Costs>) {
        let object_costs = self.get_costs();
        let object_cost_tags = self.get_cost_tags();
        let cost_tags = if object_cost_tags.len() > 0 {
            object_cost_tags
        } else {
            vec![CostTagLink::new("_uncategorized", 1)]
        };
        let cost_tag_sum = cost_tags.iter().fold(0, |acc, x| acc + x.weight) as f64;
        for cost_tag in &cost_tags {
            let ratio = (cost_tag.weight as f64) / cost_tag_sum;
            let current = cost_collection.entry(cost_tag.cost_tag_id.clone()).or_insert(Default::default());
            *current = current.clone() + (object_costs.clone() * ratio);
        }
    }

    /// Create a new hash that contains the tagged costs of this object
    fn get_tagged_costs(&self) -> HashMap<String, Costs> {
        let mut final_costs = HashMap::new();
        self.tally_tagged_costs(&mut final_costs);
        final_costs
    }
}

