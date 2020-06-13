use chrono::{DateTime, Utc};

/// Determine if a year (eg `2017`) is a leap year.
#[allow(dead_code)]
pub fn is_leap_year(year: i64) -> bool {
    ((year % 4) == 0 && (year % 100) != 0) || (year % 400) == 0
}

/// Get the current time. Mainly for testing.
#[allow(dead_code)]
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

