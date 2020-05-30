use chrono::{DateTime, Utc};

#[allow(dead_code)]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

