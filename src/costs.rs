//! Costs are a way to model disaggregate tracking of labor and resources while
//! treating the result like any number that can be added, subtracted,
//! multiplied, or divided.
//!
//! ```rust
//! use basis_core::costs::Costs;
//! use rust_decimal::prelude::*;
//!
//! let mut costs = Costs::new();
//! costs.track_resource("gasoline", 0.4);
//! costs.track_resource("iron", 2.2);
//! costs.track_labor("ceo", 42.0);
//! costs.track_labor("machinist", 122.0);
//! costs.track_labor_hours("ceo", 2.0);
//! costs.track_labor_hours("machinist", 8.0);
//! costs.track_currency("usd", Decimal::new(4200, 2));
//!
//! let costs2 = costs * 2.5;
//! assert_eq!(costs2.get_resource("gasoline"), 0.4 * 2.5);
//! assert_eq!(costs2.get_resource("iron"), 2.2 * 2.5);
//! assert_eq!(costs2.get_labor("ceo"), 42.0 * 2.5);
//! assert_eq!(costs2.get_labor("machinist"), 122.0 * 2.5);
//! assert_eq!(costs2.get_labor_hours("ceo"), 2.0 * 2.5);
//! assert_eq!(costs2.get_labor_hours("machinist"), 8.0 * 2.5);
//! assert_eq!(costs2.get_currency("usd"), Decimal::new(4200, 2) * Decimal::from_f64(2.5).unwrap());
//!
//! let costs3 = costs2 / 3.2;
//! assert_eq!(costs3.get_resource("gasoline"), (0.4 * 2.5) / 3.2);
//! assert_eq!(costs3.get_resource("iron"), (2.2 * 2.5) / 3.2);
//! assert_eq!(costs3.get_labor("ceo"), (42.0 * 2.5) / 3.2);
//! assert_eq!(costs3.get_labor("machinist"), (122.0 * 2.5) / 3.2);
//! assert_eq!(costs3.get_labor_hours("ceo"), (2.0 * 2.5) / 3.2);
//! assert_eq!(costs3.get_labor_hours("machinist"), (8.0 * 2.5) / 3.2);
//! assert_eq!(costs3.get_currency("usd"), (Decimal::new(4200, 2) * Decimal::from_f64(2.5).unwrap()) / Decimal::from_f64(3.2).unwrap());
//! ```
//!
//! In effect, Costs are an abstraction around Basis' view of production. While
//! costs themselves can be derived via walking the graph, traversing the total
//! economic graph is almost an infinite traversal, even for the most simple
//! items.
//!
//! Take the classic douchey capitalist pencil example. We can reduce it to
//! graphite, wood, the machines used to process both, the labor to extract the
//! wood/graphite, and the process of shipping the materials to their various
//! destinations along the supply chain. Or can we? The axe that cuts down the
//! tree used to make the wood in the pencil has its own supply chain story. The
//! truck that ships the pencil to the art store has a vast number of hops on
//! its own supply chain. The axe was needed to make the pencil, and in a sense
//! the axe's costs are imbued in the pencil's. So in a well-functioning cost
//! tracking system, the pencil would show the iron/steel content of that axe,
//! albeit a small amount. Now, maybe the truck that ships the pencil uses tires
//! from a company that processes rubber. Maybe that company uses pencils in
//! their daily activities. Uh oh, an infinite circular reference.
//!
//! Costs cannot be effectively "walked" because the graph is too vast and in
//! some cases, recursively infinite. Instead what we do is aggregate the costs
//! at the output of each economic node (company-product pair). When another
//! company orders that product, those costs are added to theirs and move
//! through until *they* have an output, to which costs are attributed.
//!
//! So in a sense, companies are aggregators (on the input side) and dividers
//! (on the output side) of costs.
//!
//! The best way we can represent this without having enormous tree structures
//! that are the size of the economy itself is through the Costs object which
//! aggregates costs on the level of three hash objects:
//!
//! - **labor-occupation-wage** (`labor`) -- How much total cost *in wages* it
//! took to make something, per-occupation.
//! - **labor-occupation-hours** (`labor_hours`) -- How many *total hours* it
//! took to make something, per-occupation.
//! - **resource-unit** (`resource`) -- The amount of each resource, measured
//! in a standard unit, it took to make something.
//! - **currency** (`currency`) -- The amount of currency that went into
//! purchasing inputs, useful for pricing either within or without the network.
//!
//! Labor hours are not used for cost/price value when charging consumers for
//! end products (we use the wage value), but are there to track the actual cost
//! of human labor time outside of the negotiations and fluctiations of wages.
//! This also makes it so in a future society where all wages are zero
//! (communism) we can *still track labor costs in units of hours*.
//!
//! Resources are interesting, because the ultimate goal is to track them *as
//! close to raw materials as possible* while still being useful. For instance,
//! crude oil in itself is good to track as a resource, but it might also be
//! just as useful to track gasoline, jet fuel, kerosene, etc. Thus we make it
//! possible to have standard resource transformations, applied on a limited
//! basis, in order to account for not just raw materials but semi-raw
//! materials. That said, tracking the widget-content of some product isn't
//! especially useful, nor the yards of linen imbued in it (sorry, Marx). The
//! ultimate goal is to track resources such that we're more globally aware of
//! our depletion rates of resources vs their renewal/recycle rates. This mostly
//! involves tracking the contituent resources, not the higher-level products.
//! What products are defined as raw/semi-raw materials (aka "resources") is a
//! systemwide, collective decision. It will be a function of governance, not
//! code.

use costs_derive::Costs;
use crate::{
    models::{
        currency::CurrencyID,
        occupation::OccupationID,
        resource_spec::ResourceSpecID,
    },
};
use getset::{Getters, MutGetters};
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::ops::{Add, Sub, Mul, Div};

/// A struct that acts as a container for the various types of disaggregate
/// costs we want to store and track.
///
/// The majority of this struct's implementation is under the `#[derive(Costs)]`
/// macro. This implements a number of utility functions that would otherwise be
/// a huge pain to type out over and over. It also implements some math for our
/// Cost (Add, Sub, Mul, Div).
///
/// Note that if this type were somehow iterable, a proc macro wouldn't even be
/// needed, but the types would then be more difficult to look at and
/// immediately recognize what we're trying to do, and littering generics all
/// over the place isn't my cup of tea for an object that's supposed to be
/// conceptually and operationally simple.
#[derive(Costs, Clone, Debug, Default, PartialEq, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub", get_mut)]
pub struct Costs {
    /// Stores resource content. Resources are ResourceSpec instances that have
    /// a resource tracking information attached, so we link to them via their
    /// ResourceSpecID
    resource: HashMap<ResourceSpecID, f64>,
    /// Stores labor *as is has been paid in credits* per-occupation. In other
    /// words, we don't track raw hours here, but rather the social labor value
    /// as negotiated between workers and their companies.
    labor: HashMap<OccupationID, f64>,
    /// Stores raw labor hours per-occupation. This information might be more
    /// useful in the future, as it's a measure of the occupation-time that went
    /// into building something, as opposed to the credits paid out. Cases where
    /// this might be handy is a system where all wages are 0, but we still want
    /// to track labor content.
    labor_hours: HashMap<OccupationID, f64>,
    /// Stores currency values of products. This is a strange one to have in a
    /// moneyless system, but supports the banking process of the system by
    /// tracking how much money it cost to purchase some asset from the larger
    /// market. This allows the system to know how much currency is needed to
    /// recoup the expenses on some item when selling it back into the market
    /// (or how many credits to destroy if being purchased internally). The idea
    /// is that in a hopeful future, this bucket will be obsolete and always
    /// empty as currency-based markets are phased out.
    currency: HashMap<CurrencyID, Decimal>,
}

impl Costs {
    /// Creates an empty cost object.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        let mut costs1 = Costs::new();
        let mut costs2 = Costs::new();

        costs1.track_labor("miner", 6.0);
        costs1.track_resource("widget", 3.1);
        costs1.track_resource("iron", 8.5);
        costs1.track_labor_hours("miner", 0.5);
        costs1.track_currency("usd", Decimal::new(500, 2));
        costs2.track_currency("eur", Decimal::new(230, 2));
        costs2.track_labor("miner", 2.0);
        costs2.track_labor("widgetmaker", 3.0);
        costs2.track_resource("widget", 1.8);
        costs2.track_resource("oil", 5.6);
        costs2.track_labor_hours("miner", 0.7);
        costs2.track_labor_hours("birthday clown", 0.3);
        costs2.track_currency("usd", Decimal::new(1490, 2));
        costs2.track_currency("cny", Decimal::new(3000, 0));

        let costs = costs1 + costs2;
        assert_eq!(costs.get_labor("miner"), 6.0 + 2.0);
        assert_eq!(costs.get_labor("widgetmaker"), 3.0);
        assert_eq!(costs.get_labor("joker"), 0.0);
        assert_eq!(costs.get_labor_hours("miner"), 0.5 + 0.7);
        assert_eq!(costs.get_labor_hours("birthday clown"), 0.3);
        assert_eq!(costs.get_labor_hours("magical wish pony"), 0.0);
        assert_eq!(costs.get_resource("widget"), 3.1 + 1.8);
        assert_eq!(costs.get_resource("iron"), 8.5 + 0.0);
        assert_eq!(costs.get_resource("oil"), 5.6 + 0.0);
        assert_eq!(costs.get_currency("usd"), Decimal::new(500, 2) + Decimal::new(1490, 2));
        assert_eq!(costs.get_currency("eur"), Decimal::new(230, 2));
        assert_eq!(costs.get_currency("cny"), Decimal::new(3000, 0));
        assert_eq!(costs.get_currency("btc"), Zero::zero());
    }

    #[test]
    fn mul() {
        let mut costs1 = Costs::new();
        costs1.track_labor("miner", 6.0);
        costs1.track_labor("widgetmaker", 3.0);
        costs1.track_resource("widget", 3.1);
        costs1.track_resource("iron", 8.5);
        costs1.track_labor_hours("miner", 3.0);
        costs1.track_currency("cny", Decimal::new(140000, 2));

        let costs = costs1 * 5.2;
        assert_eq!(costs.get_labor("miner"), 6.0 * 5.2);
        assert_eq!(costs.get_labor("widgetmaker"), 3.0 * 5.2);
        assert_eq!(costs.get_resource("widget"), 3.1 * 5.2);
        assert_eq!(costs.get_resource("iron"), 8.5 * 5.2);
        assert_eq!(costs.get_labor_hours("miner"), 3.0 * 5.2);
        assert_eq!(costs.get_currency("cny"), Decimal::new(140000, 2) * Decimal::from_f64(5.2).unwrap());

        let mut costs1 = Costs::new();
        let mut costs2 = Costs::new();
        costs1.track_labor("miner", 1.3);
        costs1.track_resource("widget", 8.7);
        costs1.track_labor_hours("miner", 42.0);
        costs1.track_currency("usd", Decimal::new(1300, 2));
        costs1.track_currency("eur", Decimal::new(6900, 2));
        costs2.track_labor("miner", 6.0);
        costs2.track_labor("widgetmaker", 5.0);
        costs2.track_resource("widget", 3.1);
        costs2.track_resource("iron", 8.5);
        costs2.track_labor_hours("miner", 3.0);
        costs2.track_labor_hours("axe murdererer", 3.0);
        costs2.track_currency("usd", Decimal::new(4200, 2));

        let costs = costs1 * costs2;
        assert_eq!(costs.get_labor("miner"), 1.3 * 6.0);
        assert_eq!(costs.get_labor("widgetmaker"), 0.0 * 5.0);
        assert_eq!(costs.get_resource("widget"), 8.7 * 3.1);
        assert_eq!(costs.get_resource("iron"), 0.0 * 8.5);
        assert_eq!(costs.get_labor_hours("miner"), 42.0 * 3.0);
        assert_eq!(costs.get_labor_hours("axe murdererer"), 0.0);
        assert_eq!(costs.get_currency("usd"), Decimal::new(1300, 2) * Decimal::new(4200, 2));
        assert_eq!(costs.get_currency("eur"), Zero::zero());
    }

    #[test]
    fn div_costs() {
        let mut costs1 = Costs::new();
        let mut costs2 = Costs::new();

        costs1.track_labor("miner", 6.0);
        costs1.track_labor("singer", 2.0);
        costs1.track_resource("widget", 3.1);
        costs1.track_labor_hours("dog walker", 5.2);
        costs1.track_currency("usd", Decimal::new(7800, 2));
        costs2.track_labor("miner", 2.0);
        costs2.track_labor("singer", 6.0);
        costs2.track_resource("widget", 1.8);
        costs2.track_resource("oil", 5.6);
        costs2.track_labor_hours("dog walker", 2.2);
        costs2.track_currency("usd", Decimal::new(1200, 2));

        let costs = costs1 / costs2;
        assert_eq!(costs.get_labor("miner"), 6.0 / 2.0);
        assert_eq!(costs.get_labor("singer"), 2.0 / 6.0);
        assert_eq!(costs.get_resource("widget"), 3.1 / 1.8);
        assert_eq!(costs.get_resource("oil"), 0.0 / 5.6);
        assert_eq!(costs.get_labor_hours("dog walker"), 5.2 / 2.2);
        assert_eq!(costs.get_currency("usd"), Decimal::new(7800, 2) / Decimal::new(1200, 2));
    }

    #[test]
    fn div_f64() {
        let mut costs1 = Costs::new();

        costs1.track_labor("widgetmaker", 6.0);
        costs1.track_resource("widget", 3.1);
        costs1.track_resource("oil", 5.6);
        costs1.track_labor_hours("doctor", 14.0);
        costs1.track_currency("eur", Decimal::new(43301, 2));

        let costs = costs1 / 1.3;
        assert_eq!(costs.get_labor("widgetmaker"), 6.0 / 1.3);
        assert_eq!(costs.get_resource("widget"), 3.1 / 1.3);
        assert_eq!(costs.get_resource("oil"), 5.6 / 1.3);
        assert_eq!(costs.get_labor_hours("doctor"), 14.0 / 1.3);
        assert_eq!(costs.get_currency("eur"), Decimal::new(43301, 2) / Decimal::from_f64(1.3).unwrap());
    }

    #[test]
    fn div_0_by_0() {
        let costs1 = Costs::new_with_labor("clown", 0.0);
        let costs2 = Costs::new();

        let costs = costs1 / costs2;
        assert_eq!(costs.get_labor("clown"), 0.0);
    }

    #[test]
    fn is_div_0() {
        let costs1 = Costs::new_with_labor("clown", 0.0);
        let costs2 = Costs::new();
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), false);

        let costs1 = Costs::new_with_labor("clown", 0.0);
        let costs2 = Costs::new_with_labor("clown", 0.0);
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), false);

        let costs1 = Costs::new_with_labor("violinist", 5.2);
        let costs2 = Costs::new();
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), true);

        let costs1 = Costs::new_with_labor("violinist", 5.2);
        let costs2 = Costs::new_with_labor("violinist", 0.0);
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), true);

        let mut costs1 = Costs::new();
        costs1.track_resource("iron", 4.2);
        costs1.track_labor("clown", 69.0);
        costs1.track_labor_hours("clown", 1.1);
        costs1.track_currency("usd", Decimal::new(1300, 2));
        let mut costs2 = Costs::new();
        costs2.track_resource("iron", 4.2);
        costs2.track_labor("clown", 69.0);
        costs2.track_labor_hours("clown", 1.1);
        costs2.track_currency("usd", Decimal::new(1300, 2));
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), false);

        let mut costs1 = Costs::new();
        costs1.track_resource("iron", 4.2);
        costs1.track_labor("clown", 69.0);
        costs1.track_labor_hours("clown", 0.0);
        costs1.track_currency("usd", Decimal::new(1300, 2));
        let mut costs2 = Costs::new();
        costs2.track_resource("iron", 4.2);
        costs2.track_labor("clown", 69.0);
        costs2.track_labor_hours("clown", 0.0);
        costs2.track_currency("usd", Decimal::new(1200, 2));
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), false);

        let mut costs1 = Costs::new();
        costs1.track_resource("iron", 4.2);
        costs1.track_labor("clown", 69.0);
        costs1.track_labor_hours("clown", 1.1);
        costs1.track_currency("usd", Decimal::new(1300, 2));
        costs1.track_currency("cny", Decimal::new(1, 2));
        let mut costs2 = Costs::new();
        costs2.track_resource("iron", 4.2);
        costs2.track_labor("clown", 69.0);
        costs2.track_labor_hours("clown", 1.1);
        costs2.track_currency("usd", Decimal::new(1300, 2));
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), true);

        let mut costs1 = Costs::new();
        costs1.track_resource("iron", 4.2);
        costs1.track_labor("clown", 69.0);
        costs1.track_labor_hours("clown", 1.1);
        costs1.track_currency("usd", Decimal::new(1300, 2));
        costs1.track_currency("cny", Decimal::new(1, 2));
        let mut costs2 = Costs::new();
        costs2.track_resource("iron", 4.2);
        costs2.track_labor("clown", 69.0);
        costs2.track_labor_hours("clown", 1.1);
        costs2.track_currency("usd", Decimal::new(0, 2));
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), true);

        let mut costs1 = Costs::new();
        costs1.track_resource("iron", 4.2);
        costs1.track_labor("clown", 69.0);
        costs1.track_labor_hours("clown", 1.1);
        costs1.track_currency("usd", Decimal::new(1300, 2));
        costs1.track_currency("cny", Decimal::new(1, 2));
        let mut costs2 = Costs::new();
        costs2.track_resource("iron", 4.2);
        costs2.track_labor("clown", 69.0);
        costs2.track_labor_hours("clown", 0.0);
        costs2.track_currency("usd", Decimal::new(1200, 2));
        assert_eq!(Costs::is_div_by_0(&costs1, &costs2), true);
    }

    #[test]
    #[should_panic]
    fn div_by_0() {
        let mut costs1 = Costs::new();
        let costs2 = Costs::new();

        costs1.track_resource("iron", 8.5);

        let costs = costs1 / costs2;
        assert_eq!(costs.get_resource("iron"), 8.5 / 0.0);
    }

    #[test]
    #[should_panic]
    fn div_f64_by_0() {
        let mut costs1 = Costs::new();

        costs1.track_labor("dancer", 6.0);
        costs1.track_resource("widget", 3.1);
        costs1.track_resource("oil", 5.6);

        let costs = costs1 / 0.0;
        assert_eq!(costs.get_labor("dancer"), 6.0 / 0.0);
        assert_eq!(costs.get_resource("widget"), 3.1 / 0.0);
        assert_eq!(costs.get_resource("oil"), 5.6 / 0.0);
    }

    #[test]
    fn is_zero() {
        let mut costs = Costs::new();
        assert!(costs.is_zero());
        costs.track_resource("widget", 5.0);
        assert!(!costs.is_zero());
        assert!(!Costs::new_with_labor("dictator", 4.0).is_zero());
    }
}

