use getset::{Getters, CopyGetters};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::ops::{Add, Sub, Mul, Div};

#[derive(Clone, Debug, Default, PartialEq, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Costs {
    products: HashMap<String, f64>,
    labor: HashMap<String, f64>,
}

impl Costs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_labor(ty: &str, labor: f64) -> Self {
        let mut costs = Self::new();
        costs.track_labor(ty, labor);
        costs
    }

    pub fn new_with_product(prod: &str, val: f64) -> Self {
        let mut costs = Self::new();
        costs.track(prod, val);
        costs
    }

    pub fn track(&mut self, prod: &str, val: f64) {
        if val < 0.0 {
            panic!("Costs::track() -- given value must be >= 0.0")
        }
        let entry = self.products.entry(prod.to_string()).or_insert(0.0);
        *entry += val;
    }

    pub fn track_labor(&mut self, ty: &str, val: f64) {
        if val < 0.0 {
            panic!("Costs::track_labor() -- given value must be >= 0.0")
        }
        let entry = self.labor.entry(ty.to_string()).or_insert(0.0);
        *entry += val;
    }

    #[allow(dead_code)]
    pub fn get(&self, product: &str) -> f64 {
        *self.products.get(product).unwrap_or(&0.0)
    }

    #[allow(dead_code)]
    pub fn get_labor(&self, ty: &str) -> f64 {
        *self.labor.get(ty).unwrap_or(&0.0)
    }

    /// Test if we have an empty cost set
    pub fn is_zero(&self) -> bool {
        for (_, val) in self.labor.iter() {
            if val > &0.0 {
                return false;
            }
        }
        for (_, val) in self.products.iter() {
            if val > &0.0 {
                return false;
            }
        }
        true
    }

    /// given a set of costs, subtract them from our current costs, but only if
    /// the result is >= 0 for each cost tracked. then, return a costs object
    /// showing exactly how much was taken
    pub fn take(&mut self, costs: &Costs) -> Costs {
        let mut new_costs = Costs::new();
        for (k, lval) in self.labor.iter_mut() {
            let mut rval = costs.labor().get(k).unwrap_or(&0.0) + 0.0;
            let val = if lval > &mut rval { rval } else { lval.clone() };
            *lval -= val;
            new_costs.track_labor(k, val.clone());
        }
        for (k, lval) in self.products.iter_mut() {
            let mut rval = costs.products().get(k).unwrap_or(&0.0) + 0.0;
            let val = if lval > &mut rval { rval } else { lval.clone() };
            *lval -= val;
            new_costs.track(k, val.clone());
        }
        new_costs
    }
}

impl Add for Costs {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        for k in other.labor().keys() {
            let entry = self.labor.entry(k.to_owned()).or_insert(0.0);
            *entry += other.labor().get(k).unwrap();
        }
        for k in other.products().keys() {
            let entry = self.products.entry(k.to_owned()).or_insert(0.0);
            *entry += other.products().get(k).unwrap();
        }
        self
    }
}

impl Sub for Costs {
    type Output = Self;

    fn sub(mut self, other: Self) -> Self {
        for k in other.labor().keys() {
            let entry = self.labor.entry(k.to_owned()).or_insert(0.0);
            *entry -= other.labor().get(k).unwrap();
        }
        for k in other.products().keys() {
            let entry = self.products.entry(k.to_owned()).or_insert(0.0);
            *entry -= other.products().get(k).unwrap();
        }
        self
    }
}

impl Mul for Costs {
    type Output = Self;

    fn mul(mut self, rhs: Self) -> Self {
        for (k, val) in self.labor.iter_mut() {
            *val *= rhs.labor().get(k).unwrap_or(&0.0);
        }
        for (k, val) in self.products.iter_mut() {
            *val *= rhs.products().get(k).unwrap_or(&0.0);
        }
        self
    }
}

impl Mul<f64> for Costs {
    type Output = Self;

    fn mul(mut self, rhs: f64) -> Self {
        for (_, val) in self.labor.iter_mut() {
            *val *= rhs;
        }
        for (_, val) in self.products.iter_mut() {
            *val *= rhs;
        }
        self
    }
}

impl Div for Costs {
    type Output = Self;

    fn div(mut self, rhs: Self) -> Self::Output {
        for (k, v) in self.labor.iter_mut() {
            let div = rhs.labor().get(k).unwrap_or(&0.0);
            #[cfg(feature = "panic-div0")]
            {
                if *div == 0.0 {
                    panic!("Costs::div() -- divide by zero for {:?}", k);
                }
            }
            *v /= div;
        }
        for (k, _) in rhs.labor().iter() {
            match self.labor.get(k) {
                None => {
                    self.labor.insert(k.clone(), 0.0);
                }
                _ => {}
            }
        }
        for (k, v) in self.products.iter_mut() {
            let div = rhs.products().get(k).unwrap_or(&0.0);
            #[cfg(feature = "panic-div0")]
            {
                if *div == 0.0 {
                    panic!("Costs::div() -- divide by zero for {:?}", k);
                }
            }
            *v /= div;
        }
        for (k, _) in rhs.products().iter() {
            match self.products.get(k) {
                None => {
                    self.products.insert(k.clone(), 0.0);
                }
                _ => {}
            }
        }
        self
    }
}

impl Div<f64> for Costs {
    type Output = Self;

    fn div(mut self, rhs: f64) -> Self::Output {
        #[cfg(feature = "panic-div0")]
        {
            if rhs == 0.0 {
                panic!("Costs::div() -- divide by zero");
            }
        }
        for (_, v) in self.labor.iter_mut() {
            *v /= rhs
        }
        for (_, v) in self.products.iter_mut() {
            *v /= rhs
        }
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Getters, CopyGetters, Serialize, Deserialize)]
pub struct CostsTally {
    #[getset(get = "pub")]
    costs: Costs,
    #[getset(get_copy = "pub")]
    len: u64,
}

impl CostsTally {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an entry to the cost bucket
    pub fn add(&mut self, costs: &Costs) {
        self.costs = self.costs.clone() + costs.clone();
        self.len += 1;
    }

    /// Subtract an entry from the cost bucket
    pub fn subtract(&mut self, costs: &Costs) {
        self.costs = self.costs.clone() - costs.clone();
        self.len -= 1;
    }

    pub fn add_single(&mut self, val: f64) {
        self.costs.track("_single", val);
        self.len += 1;
    }

    pub fn subtract_single(&mut self, val: f64) {
        let mut tmp_costs = Costs::new();
        tmp_costs.track("_single", val);
        self.costs = self.costs.clone() - tmp_costs;
        self.len -= 1;
    }

    pub fn total(&self) -> Costs {
        self.costs.clone()
    }

    pub fn total_single(&self) -> f64 {
        self.costs.get("_single")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostsTallyMap {
    map: HashMap<String, CostsTally>,
}

impl CostsTallyMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, key: &str, costs: &Costs) {
        let entry = self.map.entry(key.to_owned()).or_insert(CostsTally::new());
        entry.add(costs);
    }

    pub fn subtract(&mut self, key: &str, costs: &Costs) {
        let entry = self.map.entry(key.to_owned()).or_insert(CostsTally::new());
        entry.subtract(costs);
    }

    pub fn add_map(&mut self, map: &HashMap<String, Costs>) {
        for (key, val) in map.iter() {
            self.add(key, val);
        }
    }

    pub fn subtract_map(&mut self, map: &HashMap<String, Costs>) {
        for (key, val) in map.iter() {
            self.subtract(key, val);
        }
    }

    pub fn get(&self, key: &str) -> CostsTally {
        self.map.get(key).map(|x| x.clone()).unwrap_or(CostsTally::new())
    }

    pub fn map_ref<'a>(&'a self) -> &'a HashMap<String, CostsTally> {
        &self.map
    }

    pub fn into_map(self) -> HashMap<String, CostsTally> {
        let CostsTallyMap { map } = self;
        map
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
        costs1.track("widget", 3.1);
        costs1.track("iron", 8.5);
        costs2.track_labor("miner", 2.0);
        costs2.track_labor("widgetmaker", 3.0);
        costs2.track("widget", 1.8);
        costs2.track("oil", 5.6);

        let costs = costs1 + costs2;
        assert_eq!(costs.get_labor("miner"), 6.0 + 2.0);
        assert_eq!(costs.get_labor("widgetmaker"), 3.0);
        assert_eq!(costs.get_labor("joker"), 0.0);
        assert_eq!(costs.get("widget"), 3.1 + 1.8);
        assert_eq!(costs.get("iron"), 8.5 + 0.0);
        assert_eq!(costs.get("oil"), 5.6 + 0.0);
    }

    #[test]
    fn mul() {
        let mut costs1 = Costs::new();
        costs1.track_labor("miner", 6.0);
        costs1.track_labor("widgetmaker", 3.0);
        costs1.track("widget", 3.1);
        costs1.track("iron", 8.5);

        let costs = costs1 * 5.2;
        assert_eq!(costs.get_labor("miner"), 6.0 * 5.2);
        assert_eq!(costs.get_labor("widgetmaker"), 3.0 * 5.2);
        assert_eq!(costs.get("widget"), 3.1 * 5.2);
        assert_eq!(costs.get("iron"), 8.5 * 5.2);

        let mut costs1 = Costs::new();
        let mut costs2 = Costs::new();
        costs1.track_labor("miner", 1.3);
        costs1.track("widget", 8.7);
        costs2.track_labor("miner", 6.0);
        costs2.track_labor("widgetmaker", 5.0);
        costs2.track("widget", 3.1);
        costs2.track("iron", 8.5);

        let costs = costs1 * costs2;
        assert_eq!(costs.get_labor("miner"), 1.3 * 6.0);
        assert_eq!(costs.get_labor("widgetmaker"), 0.0 * 5.0);
        assert_eq!(costs.get("widget"), 8.7 * 3.1);
        assert_eq!(costs.get("iron"), 0.0 * 8.5);
    }

    #[test]
    fn div_costs() {
        let mut costs1 = Costs::new();
        let mut costs2 = Costs::new();

        costs1.track_labor("miner", 6.0);
        costs1.track_labor("singer", 2.0);
        costs1.track("widget", 3.1);
        costs2.track_labor("miner", 2.0);
        costs2.track_labor("singer", 6.0);
        costs2.track("widget", 1.8);
        costs2.track("oil", 5.6);

        let costs = costs1 / costs2;
        assert_eq!(costs.get_labor("miner"), 6.0 / 2.0);
        assert_eq!(costs.get_labor("singer"), 2.0 / 6.0);
        assert_eq!(costs.get("widget"), 3.1 / 1.8);
        assert_eq!(costs.get("oil"), 0.0 / 5.6);
    }

    #[test]
    fn div_f64() {
        let mut costs1 = Costs::new();

        costs1.track_labor("widgetmaker", 6.0);
        costs1.track("widget", 3.1);
        costs1.track("oil", 5.6);

        let costs = costs1 / 1.3;
        assert_eq!(costs.get_labor("widgetmaker"), 6.0 / 1.3);
        assert_eq!(costs.get("widget"), 3.1 / 1.3);
        assert_eq!(costs.get("oil"), 5.6 / 1.3);
    }

    #[cfg(feature = "panic-div0")]
    #[test]
    #[should_panic]
    fn div_by_0() {
        let mut costs1 = Costs::new();
        let costs2 = Costs::new();

        costs1.track("iron", 8.5);

        let costs = costs1 / costs2;
        assert_eq!(costs.get("iron"), 8.5 / 0.0);
    }

    #[cfg(not(feature = "panic-div0"))]
    #[test]
    fn div_by_0() {
        let mut costs1 = Costs::new();
        let costs2 = Costs::new();

        costs1.track("iron", 8.5);

        let costs = costs1 / costs2;
        assert_eq!(costs.get("iron"), 8.5 / 0.0);
    }

    #[cfg(feature = "panic-div0")]
    #[test]
    #[should_panic]
    fn div_f64_by_0() {
        let mut costs1 = Costs::new();

        costs1.track_labor("dancer", 6.0);
        costs1.track("widget", 3.1);
        costs1.track("oil", 5.6);

        let costs = costs1 / 0.0;
        assert_eq!(costs.get_labor("dancer"), 6.0 / 0.0);
        assert_eq!(costs.get("widget"), 3.1 / 0.0);
        assert_eq!(costs.get("oil"), 5.6 / 0.0);
    }

    #[cfg(not(feature = "panic-div0"))]
    #[test]
    fn div_f64_by_0() {
        let mut costs1 = Costs::new();

        costs1.track_labor("dancer", 6.0);
        costs1.track("widget", 3.1);
        costs1.track("oil", 5.6);

        let costs = costs1 / 0.0;
        assert_eq!(costs.get_labor("dancer"), 6.0 / 0.0);
        assert_eq!(costs.get("widget"), 3.1 / 0.0);
        assert_eq!(costs.get("oil"), 5.6 / 0.0);
    }

    #[test]
    fn is_zero() {
        let mut costs = Costs::new();
        assert!(costs.is_zero());
        costs.track("widget", 5.0);
        assert!(!costs.is_zero());
        assert!(!Costs::new_with_labor("dictator", 4.0).is_zero());
    }

    #[test]
    fn cost_buckets() {
        let mut bucket = CostsTally::new();
        assert_eq!(bucket.costs, Costs::new());
        assert_eq!(bucket.len(), 0);

        let mut costs = Costs::new();
        costs.track("widget", 69.0);
        bucket.add(&costs);

        assert_eq!(bucket.total().get("widget"), 69.0);
        assert_eq!(bucket.len(), 1);

        let mut costs = Costs::new();
        costs.track("widget", 42.0);
        bucket.add(&costs);

        assert_eq!(bucket.total().get("widget"), 69.0 + 42.0);
        assert_eq!(bucket.len(), 2);

        let mut costs = Costs::new();
        costs.track("widget", 69.0);
        bucket.subtract(&costs);

        assert_eq!(bucket.total().get("widget"), 42.0);
        assert_eq!(bucket.len(), 1);

        let mut single_bucket = CostsTally::new();
        single_bucket.add_single(64.5);
        single_bucket.add_single(12.0);

        assert_eq!(single_bucket.total_single(), 64.5 + 12.0);
        assert_eq!(single_bucket.len(), 2);

        single_bucket.subtract_single(64.5);

        assert_eq!(single_bucket.total_single(), 12.0);
        assert_eq!(single_bucket.len(), 1);

        let mut bucketmap = CostsTallyMap::new();
        bucketmap.add("inventory", &Costs::new_with_product("UNOBTAINIUM", 244.0));
        bucketmap.add("inventory", &Costs::new_with_product("UNOBTAINIUM", 198.0));
        bucketmap.add("operating", &Costs::new_with_product("BULLDOZER", 20.0));
        bucketmap.add("operating", &Costs::new_with_product("BULLDOZER", 2.0));

        let map = bucketmap.into_map();
        let inv = map.get("inventory").unwrap();
        let op = map.get("operating").unwrap();
        assert_eq!(inv.total(), Costs::new_with_product("UNOBTAINIUM", 244.0 + 198.0));
        assert_eq!(inv.len(), 2);
        assert_eq!(op.total(), Costs::new_with_product("BULLDOZER", 20.0 + 2.0));
        assert_eq!(op.len(), 2);

        let mut bucketmap = CostsTallyMap::new();
        let mut map = HashMap::new();
        {
            let inv = map.entry("inventory".to_owned()).or_insert(Costs::new());
            *inv = inv.clone() + Costs::new_with_product("UNOBTAINIUM", 244.0);
            *inv = inv.clone() + Costs::new_with_product("UNOBTAINIUM", 198.0);
        }
        {
            let op = map.entry("operating".to_owned()).or_insert(Costs::new());
            *op = op.clone() + Costs::new_with_product("BULLDOZER", 20.0);
            *op = op.clone() + Costs::new_with_product("BULLDOZER", 2.0);
        }
        bucketmap.add_map(&map);

        let map = bucketmap.into_map();
        let inv = map.get("inventory").unwrap();
        let op = map.get("operating").unwrap();
        assert_eq!(inv.total(), Costs::new_with_product("UNOBTAINIUM", 244.0 + 198.0));
        assert_eq!(inv.len(), 1);
        assert_eq!(op.total(), Costs::new_with_product("BULLDOZER", 20.0 + 2.0));
        assert_eq!(op.len(), 1);
    }
}

