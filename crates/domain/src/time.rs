use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch (UTC).
///
/// The domain never reads the clock itself; the application layer passes the current time in
/// (see `Clock` in `starter-application`). That keeps the domain deterministic and testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub const fn as_millis(self) -> i64 {
        self.0
    }

    /// Formats as RFC 3339 in UTC with second precision, e.g. `2026-10-11T08:15:00Z`.
    pub fn to_rfc3339(self) -> String {
        let secs = self.0.div_euclid(1000);
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            (rem % 3600) / 60,
            rem % 60
        )
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_rfc3339())
    }
}

/// Converts days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_epoch() {
        assert_eq!(Timestamp::from_millis(0).to_rfc3339(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn formats_leap_day() {
        // 2024-02-29T12:34:56Z
        assert_eq!(
            Timestamp::from_millis(1_709_210_096_000).to_rfc3339(),
            "2024-02-29T12:34:56Z"
        );
    }

    #[test]
    fn formats_before_epoch() {
        assert_eq!(Timestamp::from_millis(-1000).to_rfc3339(), "1969-12-31T23:59:59Z");
    }
}
