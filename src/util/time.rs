use chrono::{DateTime, Utc, NaiveDateTime};

pub fn now() -> DateTime<Utc> {
    Utc::now()
}

pub fn is_current(time: &DateTime<Utc>) -> bool {
    return (now() - time.clone()).num_seconds() < 10;
}

pub fn from_timestamp(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(ts, 0), Utc)
}

pub fn default_time() -> DateTime<Utc> {
    from_timestamp(0)
}

