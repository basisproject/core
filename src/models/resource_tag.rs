use exonum::crypto::Hash;
use chrono::{DateTime, Utc};
use util;
use crate::proto;

#[derive(Clone, Debug, ProtobufConvert)]
#[exonum(pb = "proto::resource_tag::ResourceTag", serde_pb_convert)]
pub struct ResourceTag {
    pub id: String,
    pub product_id: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub deleted: DateTime<Utc>,
    pub history_len: u64,
    pub history_hash: Hash,
}

impl ResourceTag {
    pub fn new(id: &str, product_id: &str, created: &DateTime<Utc>, updated: &DateTime<Utc>, deleted: Option<&DateTime<Utc>>, history_len: u64, history_hash: &Hash) -> Self {
        Self {
            id: id.to_owned(),
            product_id: product_id.to_owned(),
            created: created.clone(),
            updated: updated.clone(),
            deleted: deleted.unwrap_or(&util::time::default_time()).clone(),
            history_len,
            history_hash: history_hash.clone(),
        }
    }

    pub fn delete(&self, deleted: &DateTime<Utc>, history_hash: &Hash) -> Self {
        Self::new(
            &self.id,
            &self.product_id,
            &self.created,
            &self.updated,
            Some(deleted),
            self.history_len + 1,
            history_hash
        )
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted != util::time::default_time()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use util;

    fn make_hash() -> Hash {
        Hash::new([1, 234, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4])
    }

    fn make_resource_tag() -> ResourceTag {
        let date = util::time::now();
        ResourceTag::new(
            "c5a94565-6037-452c-ab33-82c79bd0e42b",
            "2496b7f7-d041-4129-910e-5f9c07754336",
            &date,
            &date,
            None,
            0,
            &make_hash()
        )
    }

    #[test]
    fn deletes() {
        let resource_tag = make_resource_tag();
        util::sleep(100);
        let date2 = util::time::now();
        let hash2 = Hash::new([1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 233, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4, 1, 27, 6, 4]);
        let resource_tag2 = resource_tag.delete(&date2, &hash2);

        assert_eq!(resource_tag.id, resource_tag2.id);
        assert_eq!(resource_tag.product_id, resource_tag2.product_id);
        assert_eq!(resource_tag.created, resource_tag2.created);
        assert_eq!(resource_tag.updated, resource_tag2.updated);
        assert_eq!(resource_tag.deleted, util::time::default_time());
        assert_eq!(resource_tag2.deleted, date2);
        assert_eq!(resource_tag2.history_hash, hash2);
        assert!(resource_tag.history_hash != resource_tag2.history_hash);
        assert_eq!(resource_tag.history_len, resource_tag2.history_len - 1);

        assert!(!resource_tag.is_deleted());
        assert!(resource_tag2.is_deleted());
    }
}


