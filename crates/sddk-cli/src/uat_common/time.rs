//! Time utilities for UAT components.

#![allow(dead_code)]

/// Generate RFC3339 timestamp (UTC).
#[allow(clippy::manual_is_multiple_of)]
pub fn now_rfc3339() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let mins = (remaining % 3600) / 60;
    let secs = remaining % 60;
    let is_leap = |y: u64| (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let days_in_year = |y: u64| if is_leap(y) { 366 } else { 365 };
    let mut dy = days;
    let mut y = 1970u64;
    while dy >= days_in_year(y) {
        dy -= days_in_year(y);
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0u64;
    for (i, dm) in month_days.iter().enumerate() {
        if dy < *dm {
            m = i as u64 + 1;
            break;
        }
        dy -= *dm;
    }
    let d = dy + 1;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, mins, secs
    )
}
