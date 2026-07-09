//! UTC time for half-hourly settlement periods (ADR-3).
//!
//! Internal time is UTC with a monotonic half-hourly index; local time and
//! clock changes are handled only at I/O edges. There are no naive
//! datetimes: the only accepted textual form is strict RFC 3339 with a
//! literal `Z` offset and whole seconds (`YYYY-MM-DDTHH:MM:SSZ`).
//!
//! Represented as microseconds since the Unix epoch (matching the data
//! pack's Parquet `timestamp[us, tz=UTC]` index), proleptic Gregorian
//! calendar, leap seconds ignored (Unix time convention). Calendar
//! conversions use Howard Hinnant's `days_from_civil` / `civil_from_days`
//! algorithms.

use crate::GridError;

/// Microseconds in one half-hourly settlement period.
pub const HALF_HOUR_MICROS: i64 = 30 * 60 * 1_000_000;

const MICROS_PER_SEC: i64 = 1_000_000;
const SECS_PER_DAY: i64 = 86_400;

/// An instant in UTC, at microsecond resolution.
///
/// Ordering and equality follow the underlying timeline. The raw `i64` is
/// a time index, not a physical quantity in the ADR-4 sense; it is exposed
/// via [`UtcInstant::unix_micros`] for interop with Parquet trace indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcInstant(i64);

impl UtcInstant {
    /// Instant from microseconds since the Unix epoch (UTC).
    #[must_use]
    pub const fn from_unix_micros(micros: i64) -> Self {
        Self(micros)
    }

    /// Microseconds since the Unix epoch (UTC).
    #[must_use]
    pub const fn unix_micros(self) -> i64 {
        self.0
    }

    /// Parse strict RFC 3339 UTC: `YYYY-MM-DDTHH:MM:SSZ`.
    ///
    /// Anything else — naive datetimes, non-`Z` offsets, fractional
    /// seconds, invalid calendar dates — is rejected (ADR-3).
    pub fn parse(text: &str) -> Result<Self, GridError> {
        let err = |reason: &str| GridError::InvalidTimestamp {
            value: text.to_owned(),
            reason: reason.to_owned(),
        };

        let bytes = text.as_bytes();
        if bytes.len() != 20 {
            return Err(err("expected the exact form YYYY-MM-DDTHH:MM:SSZ"));
        }
        if bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes[10] != b'T'
            || bytes[13] != b':'
            || bytes[16] != b':'
            || bytes[19] != b'Z'
        {
            return Err(err("expected the exact form YYYY-MM-DDTHH:MM:SSZ"));
        }

        let digits = |range: core::ops::Range<usize>| -> Result<i64, GridError> {
            let field = &text[range];
            if !field.bytes().all(|b| b.is_ascii_digit()) {
                return Err(err("non-digit character in a numeric field"));
            }
            field
                .parse::<i64>()
                .map_err(|_| err("numeric field out of range"))
        };

        let year = digits(0..4)?;
        let month = digits(5..7)?;
        let day = digits(8..10)?;
        let hour = digits(11..13)?;
        let minute = digits(14..16)?;
        let second = digits(17..19)?;

        if !(1..=12).contains(&month) {
            return Err(err("month must be 01-12"));
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(err("day out of range for the given month"));
        }
        if hour > 23 {
            return Err(err("hour must be 00-23"));
        }
        if minute > 59 {
            return Err(err("minute must be 00-59"));
        }
        if second > 59 {
            return Err(err("second must be 00-59 (leap seconds not represented)"));
        }

        let days = days_from_civil(year, month, day);
        let secs = days * SECS_PER_DAY + hour * 3_600 + minute * 60 + second;
        Ok(Self(secs * MICROS_PER_SEC))
    }

    /// The instant `n` half-hourly settlement periods after this one.
    #[must_use]
    pub const fn plus_periods(self, n: i64) -> Self {
        Self(self.0 + n * HALF_HOUR_MICROS)
    }

    /// The proleptic-Gregorian civil date `(year, month, day)` of this
    /// instant (UTC), month and day 1-based. Used for calendar-month
    /// aggregation and monthly availability profiles (Stage 1).
    #[must_use]
    pub const fn civil_date(self) -> (i64, u8, u8) {
        let secs = self.0.div_euclid(MICROS_PER_SEC);
        let days = secs.div_euclid(SECS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        (year, month as u8, day as u8)
    }

    /// Number of half-hourly periods from `self` to `end`, counting both
    /// endpoints as period starts (so `start..=start` is one period —
    /// settlement-period convention: a horizon `start`/`end` names the
    /// first and last period *starts*).
    ///
    /// Errors if `end` precedes `self` or the two instants are not a whole
    /// number of half-hours apart.
    pub fn periods_until_inclusive(self, end: Self) -> Result<usize, GridError> {
        let span = end.0 - self.0;
        if span < 0 {
            return Err(GridError::InvalidHorizon {
                reason: format!("end {end} precedes start {self}"),
            });
        }
        if span % HALF_HOUR_MICROS != 0 {
            return Err(GridError::InvalidHorizon {
                reason: format!(
                    "start {self} and end {end} are not a whole number of half-hours apart"
                ),
            });
        }
        Ok((span / HALF_HOUR_MICROS) as usize + 1)
    }
}

impl core::fmt::Display for UtcInstant {
    /// Formats as strict RFC 3339 UTC (`YYYY-MM-DDTHH:MM:SSZ`), truncating
    /// sub-second microseconds (trace indices are always on whole
    /// half-hours).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let secs = self.0.div_euclid(MICROS_PER_SEC);
        let days = secs.div_euclid(SECS_PER_DAY);
        let time_of_day = secs.rem_euclid(SECS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        write!(
            f,
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            time_of_day / 3_600,
            (time_of_day / 60) % 60,
            time_of_day % 60
        )
    }
}

const fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
    }
}

/// Days since 1970-01-01 for a proleptic-Gregorian civil date
/// (Hinnant, `days_from_civil`).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let mp = (month + 9) % 12; // Mar=0 .. Feb=11
    let doy = (153 * mp + 2) / 5 + day - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Civil date (year, month, day) from days since 1970-01-01
/// (Hinnant, `civil_from_days`).
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // Reference instants, computed independently:
    // 2024-01-01T00:00:00Z = 1_704_067_200 s since the Unix epoch.
    const T_2024_01_01: i64 = 1_704_067_200 * 1_000_000;
    // 2024-12-31T23:30:00Z = 1_704_067_200 + (366*86400 - 1800) s.
    const T_2024_12_31_2330: i64 = (1_704_067_200 + 366 * 86_400 - 1_800) * 1_000_000;

    #[test]
    fn parses_strict_rfc3339_utc() {
        let t = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        assert_eq!(t.unix_micros(), T_2024_01_01);
        let t = UtcInstant::parse("2024-12-31T23:30:00Z").unwrap();
        assert_eq!(t.unix_micros(), T_2024_12_31_2330);
        // Pre-epoch dates (the 1985+ weather record is post-epoch, but the
        // representation must not be surprised by earlier ones).
        let t = UtcInstant::parse("1969-12-31T23:59:59Z").unwrap();
        assert_eq!(t.unix_micros(), -1_000_000);
    }

    #[test]
    fn rejects_malformed_and_non_utc_timestamps() {
        for bad in [
            "2024-01-01 00:00:00",       // missing T/Z
            "2024-01-01T00:00:00+01:00", // not UTC — local offsets are I/O-edge only
            "2024-01-01T00:00:00",       // naive datetime (ADR-3: none anywhere)
            "2024-13-01T00:00:00Z",      // no month 13
            "2024-02-30T00:00:00Z",      // no 30 Feb
            "2023-02-29T00:00:00Z",      // not a leap year
            "2024-01-01T24:00:00Z",      // no hour 24
            "not a date",
            "",
        ] {
            assert!(
                UtcInstant::parse(bad).is_err(),
                "should have rejected {bad:?}"
            );
        }
    }

    #[test]
    fn displays_as_rfc3339_utc_round_trip() {
        for s in [
            "2024-01-01T00:00:00Z",
            "2024-12-31T23:30:00Z",
            "2024-02-29T12:30:00Z", // leap day
            "1985-01-01T00:00:00Z",
            "1969-12-31T23:59:59Z",
        ] {
            let t = UtcInstant::parse(s).unwrap();
            assert_eq!(t.to_string(), s);
        }
    }

    #[test]
    fn civil_date_returns_utc_calendar_components() {
        for (text, expected) in [
            ("2024-01-01T00:00:00Z", (2024, 1, 1)),
            ("2024-02-29T12:30:00Z", (2024, 2, 29)), // leap day
            ("2024-09-30T23:30:00Z", (2024, 9, 30)), // coal-closure boundary
            ("2024-10-01T00:00:00Z", (2024, 10, 1)),
            ("2024-12-31T23:30:00Z", (2024, 12, 31)),
            ("1985-06-15T06:00:00Z", (1985, 6, 15)),
        ] {
            assert_eq!(UtcInstant::parse(text).unwrap().civil_date(), expected);
        }
    }

    #[test]
    fn half_hourly_period_arithmetic() {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        assert_eq!(start.plus_periods(0), start);
        assert_eq!(
            start.plus_periods(1),
            UtcInstant::parse("2024-01-01T00:30:00Z").unwrap()
        );
        // 17,568 half-hours after the start of leap-year 2024 is the start
        // of 2025; the final period of 2024 begins one period earlier.
        assert_eq!(
            start.plus_periods(17_567),
            UtcInstant::parse("2024-12-31T23:30:00Z").unwrap()
        );
    }

    #[test]
    fn periods_between_counts_inclusive_half_hours() {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        let end = UtcInstant::parse("2024-12-31T23:30:00Z").unwrap();
        // Inclusive of both endpoints: the leap-year period count.
        assert_eq!(start.periods_until_inclusive(end).unwrap(), 17_568);
        assert_eq!(start.periods_until_inclusive(start).unwrap(), 1);
    }

    #[test]
    fn periods_between_rejects_misaligned_or_reversed_bounds() {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        let end = UtcInstant::parse("2024-12-31T23:30:00Z").unwrap();
        // Reversed bounds.
        assert!(end.periods_until_inclusive(start).is_err());
        // Not a whole number of half-hours apart.
        let offset = UtcInstant::parse("2024-01-01T00:59:59Z").unwrap();
        assert!(start.periods_until_inclusive(offset).is_err());
    }
}
