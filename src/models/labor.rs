use chrono::{DateTime, Utc};
use crate::{
    models::company_member::CompanyMemberID,
    models::cost_tag::{CostTagLink, Costable},
    models::costs::Costs,
};

basis_model! {
    pub struct Labor {
        member_id: CompanyMemberID,
        occupation: String,
        wage: f64,
        #[builder(default)]
        cost_tags: Vec<CostTagLink>,
        #[builder(default)]
        start: Option<DateTime<Utc>>,
        #[builder(default)]
        end: Option<DateTime<Utc>>,
    }
    LaborID
    LaborBuilder
}

impl Labor {
    /// lets us know if we have both a start and end data
    pub fn is_finalized(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }

    /// Get the number of hours this labor entry encompasses
    pub fn hours(&self) -> Option<f64> {
        if !self.is_finalized() { return None; }
        let duration = self.end.unwrap() - self.start.unwrap();
        Some(duration.num_milliseconds() as f64 / (60.0 * 60.0 * 1000.0))
    }

    /// Gets the adjusted hours (by wage) for this labor record
    pub fn wage_hours(&self) -> Option<f64> {
        self.hours().map(|x| x * self.wage)
    }
}

impl Costable for Labor {
    fn get_costs(&self) -> Costs {
        Costs::new_with_labor(&self.occupation, self.wage_hours().unwrap_or(0.0))
    }

    fn get_cost_tags(&self) -> Vec<CostTagLink> {
        self.cost_tags.clone()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use crate::{
        models::cost_tag::CostTagLink,
        util,
    };

    fn make_labor() -> Labor {
        Labor::builder()
            .id("1234")
            .member_id("JOE THE PLUMBER MAKES 100K/YEAR SO HE SHOULDN'T BE TAXED BECAUSE THAT'S SOCIALISM")
            .occupation("whiner")
            .wage(42)
            .cost_tags(vec![CostTagLink::new("complainy buttface expenses", 4)])
            .created(util::time::now())
            .updated(util::time::now())
            .build().unwrap()
    }

    #[test]
    fn hours() {
        let labor = make_labor();
        let start: DateTime<Utc> = "2018-01-01T15:32:59.033Z".parse().unwrap();
        let end: DateTime<Utc> = "2018-01-02T03:17:11.573Z".parse().unwrap();
        let mut labor2 = labor.clone();
        labor2
            .set_start(Some(start))
            .set_end(Some(end))
            .set_updated(util::time::now());
        // long day...
        assert_eq!(labor2.hours(), Some(11.736816666666666));
        assert_eq!(labor2.wage_hours(), Some(11.736816666666666 * 42.0));
    }

    #[test]
    fn empty() {
        let labor = make_labor();
        assert!(!labor.is_finalized());
        let start: DateTime<Utc> = "2018-01-01T15:32:59.033Z".parse().unwrap();
        let end: DateTime<Utc> = "2018-01-02T03:17:11.573Z".parse().unwrap();
        let mut labor2 = labor.clone();
        labor2
            .set_start(Some(start))
            .set_end(Some(end))
            .set_updated(util::time::now());
        assert!(labor2.is_finalized());
    }
}

