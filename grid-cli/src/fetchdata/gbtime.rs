//! Europe/London local midnight → UTC, at the I/O edge (ADR-3).
//!
//! The NESO demand file keys rows by *settlement date* — a Europe/London
//! clock day whose period 1 starts at local midnight (46 periods on the
//! short spring day, 50 on the long autumn day). Converting to the
//! internal UTC index needs exactly one fact per date: whether local
//! midnight is GMT (UTC+0) or BST (UTC+1).
//!
//! GB summer time follows the EU rule: BST starts at 01:00 UTC on the
//! last Sunday of March and ends at 01:00 UTC on the last Sunday of
//! October. Both transitions happen *after* local midnight, so local
//! midnight of date `d` is BST iff `lastSunMarch < d <= lastSunOctober`.
//! This rule is exact for 1996 onwards (the harmonised EU rule; GB's
//! start rule matches back to 1981 but the end rule differed before
//! 1996) — the pinned sources are per-year NESO files from 2024, well
//! inside that range.

use grid_core::GridError;
use grid_core::time::UtcInstant;

const HOUR_MICROS: i64 = 3_600 * 1_000_000;
const DAY_MICROS: i64 = 24 * HOUR_MICROS;

/// UTC instant of Europe/London local midnight on the given civil date.
///
/// Errors only if the date itself is invalid (delegated to the strict
/// [`UtcInstant`] parser).
pub fn london_midnight_utc(year: i64, month: u8, day: u8) -> Result<UtcInstant, GridError> {
    let utc_midnight = UtcInstant::parse(&format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))?;
    let offset = if midnight_is_bst(year, month, day)? {
        HOUR_MICROS
    } else {
        0
    };
    Ok(UtcInstant::from_unix_micros(
        utc_midnight.unix_micros() - offset,
    ))
}

/// Whether local midnight of the given date falls in British Summer Time.
fn midnight_is_bst(year: i64, month: u8, day: u8) -> Result<bool, GridError> {
    let start = last_sunday(year, 3)?; // BST begins 01:00 UTC this day
    let end = last_sunday(year, 10)?; // BST ends 01:00 UTC this day
    Ok(((month, day) > (3, start)) && ((month, day) <= (10, end)))
}

/// Day-of-month of the last Sunday of March or October — the GB
/// clock-change dates the validator cross-checks.
pub fn last_sunday_of(year: i64, month: u8) -> Result<u8, GridError> {
    last_sunday(year, month)
}

/// Day-of-month of the last Sunday of the given month.
fn last_sunday(year: i64, month: u8) -> Result<u8, GridError> {
    debug_assert!(month == 3 || month == 10);
    let last_day: u8 = 31; // March and October both have 31 days
    let instant = UtcInstant::parse(&format!("{year:04}-{month:02}-{last_day:02}T00:00:00Z"))?;
    let days_since_epoch = instant.unix_micros().div_euclid(DAY_MICROS);
    // 1970-01-01 was a Thursday; Sunday == 0 in this convention.
    let weekday_sun0 = (days_since_epoch + 4).rem_euclid(7) as u8;
    Ok(last_day - weekday_sun0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn midnight(y: i64, m: u8, d: u8) -> String {
        london_midnight_utc(y, m, d).unwrap().to_string()
    }

    #[test]
    fn gb_2024_clock_change_dates() {
        assert_eq!(last_sunday(2024, 3).unwrap(), 31);
        assert_eq!(last_sunday(2024, 10).unwrap(), 27);
    }

    #[test]
    fn winter_midnight_is_utc_midnight() {
        assert_eq!(midnight(2024, 1, 1), "2024-01-01T00:00:00Z");
        assert_eq!(midnight(2024, 12, 31), "2024-12-31T00:00:00Z");
        // The short day itself still *starts* in GMT (transition at 01:00 UTC).
        assert_eq!(midnight(2024, 3, 31), "2024-03-31T00:00:00Z");
        // The day after the long day is back to GMT.
        assert_eq!(midnight(2024, 10, 28), "2024-10-28T00:00:00Z");
    }

    #[test]
    fn summer_midnight_is_2300_utc_previous_day() {
        // First full BST day.
        assert_eq!(midnight(2024, 4, 1), "2024-03-31T23:00:00Z");
        assert_eq!(midnight(2024, 7, 15), "2024-07-14T23:00:00Z");
        // The long day itself starts in BST (transition at 01:00 UTC).
        assert_eq!(midnight(2024, 10, 27), "2024-10-26T23:00:00Z");
    }

    #[test]
    fn other_years_match_the_published_rule() {
        // 2023: 26 March / 29 October; 2025: 30 March / 26 October.
        assert_eq!(last_sunday(2023, 3).unwrap(), 26);
        assert_eq!(last_sunday(2023, 10).unwrap(), 29);
        assert_eq!(last_sunday(2025, 3).unwrap(), 30);
        assert_eq!(last_sunday(2025, 10).unwrap(), 26);
    }
}
