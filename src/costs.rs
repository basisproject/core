//! This library holds the algorithm that costs products and services.

use std::collections::HashMap;
use error::{BResult, BError};
use models::{
    costs::Costs,
    cost_tag::{CostTagEntry, Costable},
    order::Order,
    amortization::Amortization,
    product::Product,
    labor::Labor,
};

/// Takes two sets of orders: a company's incoming orders ("sales" in the
/// current vernacular) and outgoing orders ("purchases").
///
/// The orders *must* be filtered such that both sets are a particular window
/// in time (ex, the last 365 days) and must be ordered from oldest to newest.
pub fn calculate_costs(orders_incoming: &Vec<Order>, orders_outgoing: &Vec<Order>, labor: &Vec<Labor>, _wamortization: &HashMap<String, Amortization>, products: &HashMap<String, Product>) -> BResult<HashMap<String, Costs>> {
    // holds a mapping for cost_tag -> sum costs for all of our cost tags
    let mut sum_costs: HashMap<String, Costs> = HashMap::new();
    // maps product_id -> number produced over order period
    let mut sum_produced: HashMap<String, f64> = HashMap::new();

    // add our labor costs into the totals
    for entry in labor {
        entry.tally_tagged_costs(&mut sum_costs);
    }

    // add all outgoing orders into the cost totals
    for order in orders_outgoing {
        order.tally_tagged_costs(&mut sum_costs);
    }

    // sum how many of each product we have produced
    for order in orders_incoming {
        for prod in &order.products {
            let current = sum_produced.entry(prod.product_id.clone()).or_insert(Default::default());
            *current += prod.quantity;
        }
    }

    calculate_costs_with_aggregates(products, &sum_costs, &sum_produced)
}

pub fn calculate_costs_with_aggregates(products: &HashMap<String, Product>, sum_costs: &HashMap<String, Costs>, sum_produced: &HashMap<String, f64>) -> BResult<HashMap<String, Costs>> {
    let mut tag_tracker: HashMap<String, bool> = HashMap::new();
    let mut final_costs: HashMap<String, Costs> = HashMap::new();
    let mut product_tag_totals: HashMap<String, u64> = HashMap::new();
    let mut products = products.clone();

    // track which cost tags are present in the products
    for (prod_id, product) in products.iter() {
        if sum_produced.get(prod_id).unwrap_or(&0.0) == &0.0 {
            // products that were not ordered/produced won't get costs
            continue;
        }
        for tag in &product.cost_tags {
            if tag.weight == 0 {
                continue;
            }
            tag_tracker.insert(tag.id.clone(), true);
        }
    }
    // check for missing tags.
    //
    // if we have costs assigned to a tag, but no products are assigned that tag
    // then potentially we'd have unaccounted for costs. so what we do is find
    // tags that HAVE costs but are NOT assigned and assign them to each product
    // equally.
    for (tag_id, _costs) in sum_costs.iter() {
        if tag_tracker.contains_key(tag_id) {
            continue;
        }
        // if a tag with costs associated to it is missing from the product
        // assignments, assign that cost tag to all products equally
        for (_prod_id, product) in products.iter_mut() {
            product.cost_tags.push(CostTagEntry::new(&tag_id, 1));
        }
    }
    // for each product, tally up the sum of the cost tags (bucketed by cost
    // tag)
    for (_, product) in products.iter() {
        for tag in &product.cost_tags {
            let current = product_tag_totals.entry(tag.id.clone()).or_insert(0);
            *current = tag.weight;
        }
    }
    // for each product, divvy up the costs of each of its cost tags via the
    // tag ratio (as compared to other products) and then divide by the amount
    // produced.
    //
    // this gives a per-unit cost to each product based on the flow of costs
    // through the cost tags.
    for (prod_id, product) in products.iter() {
        let num_produced: f64 = sum_produced.get(prod_id).unwrap_or(&0.0).clone();
        if num_produced == 0.0 {
            // products that were not produced have no cost (and will not have
            // their cost tags tallied in the totals, meaning they will not
            // "steal" costs from products that were actively produced)
            final_costs.insert(prod_id.clone(), Costs::new());
        } else {
            let mut prod_cost_sum = Costs::new();
            for tag in &product.cost_tags {
                let total: f64 = product_tag_totals.get(&tag.id).ok_or_else(|| BError::CostMissingTag)?.clone() as f64;
                let tag_ratio: f64 = tag.weight as f64 / total;
                let tag_costs = sum_costs.get(&tag.id).map(|x| x.clone()).unwrap_or(Costs::new());
                prod_cost_sum = prod_cost_sum + (tag_costs * tag_ratio);
            }
            final_costs.insert(prod_id.clone(), prod_cost_sum / num_produced);
        }
    }
    Ok(final_costs)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use exonum::crypto::Hash;
    use std::collections::HashMap;
    use models::order::{Order, ProcessStatus, ProductEntry};
    use models::labor::Labor;
    use models::product::{Product, Unit, Dimensions};
    use util::time;

    fn make_hash() -> Hash {
        Hash::new([1, 28, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4])
    }

    #[test]
    fn calculates() {
        let orders_incoming = test_orders_incoming();
        let orders_outgoing = test_orders_outgoing();
        let labor = test_labor();
        let amortization = HashMap::new();
        let products = test_products();
        let costs = calculate_costs(&orders_incoming, &orders_outgoing, &labor, &amortization, &products).expect("costs failed");
        println!(">>> final costs: {:?}", costs);
    }

    fn test_orders_incoming() -> Vec<Order> {
        let fakehash = make_hash();
        vec![
        ]
    }

    fn test_orders_outgoing() -> Vec<Order> {
        let fakehash = make_hash();
        vec![
        ]
    }

    fn test_labor() -> Vec<Labor> {
        let fakehash = make_hash();
        vec![
        ]
    }

    fn test_products() -> HashMap<String, Product> {
        let fakehash = make_hash();
        let mut products = HashMap::new();
        products
    }
}

