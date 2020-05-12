use exonum::crypto::Hash;
use chrono::{DateTime, Utc};
use crate::{
    proto,
    cost_tag::{CostTagEntry, Costable},
    costs::Costs,
};

#[derive(Clone, Debug, ProtobufConvert)]
#[exonum(pb = "proto::labor::Labor", serde_pb_convert)]
pub struct Labor {
    pub id: String,
    pub company_id: String,
    pub user_id: String,
    pub occupation: String,
    pub wage: f64,
    pub cost_tags: Vec<CostTagEntry>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub history_len: u64,
    pub history_hash: Hash,
}

impl Labor {
    pub fn new(id: &str, company_id: &str, user_id: &str, occupation: &str, wage: f64, cost_tags: &Vec<CostTagEntry>, start: Option<&DateTime<Utc>>, end: Option<&DateTime<Utc>>, created: &DateTime<Utc>, updated: &DateTime<Utc>, history_len: u64, history_hash: &Hash) -> Self {
        Self {
            id: id.to_owned(),
            company_id: company_id.to_owned(),
            user_id: user_id.to_owned(),
            occupation: occupation.to_owned(),
            wage,
            cost_tags: cost_tags.clone(),
            start: start.unwrap_or(&util::time::default_time()).clone(),
            end: end.unwrap_or(&util::time::default_time()).clone(),
            created: created.clone(),
            updated: updated.clone(),
            history_len,
            history_hash: history_hash.clone(),
        }
    }

    pub fn update(&self, cost_tags: Option<&Vec<CostTagEntry>>, start: Option<&DateTime<Utc>>, end: Option<&DateTime<Utc>>, updated: &DateTime<Utc>, history_hash: &Hash) -> Self {
        Self::new(
            &self.id,
            &self.company_id,
            &self.user_id,
            &self.occupation,
            self.wage,
            cost_tags.unwrap_or(&self.cost_tags),
            Some(start.unwrap_or(&self.start)),
            Some(end.unwrap_or(&self.end)),
            &self.created,
            updated,
            self.history_len + 1,
            history_hash
        )
    }

    pub fn set_wage(&self, wage: f64, updated: &DateTime<Utc>, history_hash: &Hash) -> Self {
        Self::new(
            &self.id,
            &self.company_id,
            &self.user_id,
            &self.occupation,
            wage,
            &self.cost_tags,
            Some(&self.start),
            Some(&self.end),
            &self.created,
            updated,
            self.history_len + 1,
            history_hash
        )
    }

    /// lets us know if we have both a start and end data
    pub fn is_finalized(&self) -> bool {
        let empty = util::time::default_time();
        self.start != empty && self.end != empty
    }

    /// Get the number of hours this labor entry encompasses
    pub fn hours(&self) -> f64 {
        let duration = self.end - self.start;
        duration.num_milliseconds() as f64 / (60.0 * 60.0 * 1000.0)
    }

    /// Gets the adjusted hours (by wage) for this labor record
    pub fn wage_hours(&self) -> f64 {
        self.hours() * self.wage
    }
}

impl Costable for Labor {
    fn get_costs(&self) -> Costs {
        Costs::new_with_labor(&self.occupation, self.wage_hours())
    }

    fn get_cost_tags(&self) -> Vec<CostTagEntry> {
        self.cost_tags.clone()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use util;

    fn make_hash() -> Hash {
        Hash::new([1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4])
    }

    fn make_labor() -> Labor {
        let date = util::time::now();
        Labor::new(
            "9fd8cdc6-04a8-4a35-9cd8-9dc6073a2d10",
            "df874abc-5583-4740-9f4e-3236530bcc1e",
            "7de177ba-d589-4f7b-94e0-96d2b0752460",
            "tremendous president. the best president. everyone says so.",
            1000.0, // a good wage. tremendous wage.
            &vec![CostTagEntry::new("113", 4)],
            Some(&date),
            None,
            &date,
            &date,
            0,
            &make_hash()
        )
    }

    #[test]
    fn update() {
        let labor = make_labor();
        util::sleep(100);
        let date2 = util::time::now();
        let hash2 = Hash::new([1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 233, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4]);
        let labor2 = labor.update(None, Some(&date2), None, &date2, &hash2);
        assert_eq!(labor.id, labor2.id);
        assert_eq!(labor.company_id, labor2.company_id);
        assert_eq!(labor.user_id, labor2.user_id);
        assert_eq!(labor.occupation, labor2.occupation);
        assert_eq!(labor.wage, labor2.wage);
        assert_eq!(labor.cost_tags, labor2.cost_tags);
        assert!(labor.start != labor2.start);
        assert_eq!(labor2.start, date2);
        assert_eq!(labor.end, util::time::default_time());
        assert_eq!(labor2.end, util::time::default_time());
        assert_eq!(labor.created, labor2.created);
        assert_eq!(labor.history_len, 0);
        assert_eq!(labor2.history_len, 1);
        assert_eq!(labor2.history_hash, hash2);
        util::sleep(100);
        let date3 = util::time::now();
        let hash3 = Hash::new([1, 37, 6, 4, 1, 37, 6, 4, 1, 37, 6, 133, 1, 37, 6, 4, 1, 37, 6, 4, 1, 37, 6, 4, 1, 37, 6, 4, 1, 37, 6, 4]);
        let cost_tags3 = vec![CostTagEntry::new("4242", 17)];
        let labor3 = labor2.update(Some(&cost_tags3), None, Some(&date3), &date3, &hash3);
        assert_eq!(labor2.id, labor3.id);
        assert_eq!(labor2.company_id, labor3.company_id);
        assert_eq!(labor2.user_id, labor3.user_id);
        assert_eq!(labor2.occupation, labor3.occupation);
        assert_eq!(labor2.wage, labor3.wage);
        assert_eq!(labor2.cost_tags[0].id, "113");
        assert_eq!(labor3.cost_tags[0].id, "4242");
        assert_eq!(labor2.start, labor3.start);
        assert_eq!(labor3.start, date2);
        assert_eq!(labor2.end, util::time::default_time());
        assert_eq!(labor3.end, date3);
        assert_eq!(labor2.created, labor3.created);
        assert_eq!(labor2.history_len, 1);
        assert_eq!(labor3.history_len, 2);
        assert_eq!(labor3.history_hash, hash3);
    }

    #[test]
    fn set_wage() {
        let labor = make_labor();
        let date2 = util::time::now();
        let hash2 = Hash::new([1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 233, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4]);
        // terrible wage. unpresidented.
        let labor2 = labor.set_wage(0.00001, &date2, &hash2);
        assert!(labor.wage != labor2.wage);
    }

    #[test]
    fn hours() {
        let labor = make_labor();
        let start: DateTime<Utc> = "2018-01-01T15:32:59.033Z".parse().unwrap();
        let end: DateTime<Utc> = "2018-01-02T03:17:11.573Z".parse().unwrap();
        let hash2 = Hash::new([1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 233, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4]);
        let labor2 = labor.update(None, Some(&start), Some(&end), &end, &hash2);
        // long day...
        assert_eq!(labor2.hours(), 11.736816666666666);
        assert_eq!(labor2.wage_hours(), 11.736816666666666 * 1000.0);
    }

    #[test]
    fn empty() {
        let labor = make_labor();
        assert!(!labor.is_finalized());
        let start: DateTime<Utc> = "2018-01-01T15:32:59.033Z".parse().unwrap();
        let end: DateTime<Utc> = "2018-01-02T03:17:11.573Z".parse().unwrap();
        let hash2 = Hash::new([1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 233, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4]);
        let labor2 = labor.update(None, Some(&start), Some(&end), &end, &hash2);
        assert!(labor2.is_finalized());
    }
}

