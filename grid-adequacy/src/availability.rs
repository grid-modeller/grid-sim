//! Availability models for thermal technologies (docs/04 Stage 1: flat
//! plus a monthly profile).
//!
//! An availability model derates a technology's nameplate capacity per
//! settlement period: the dispatchable ceiling at instant `t` is
//! `capacity × factor_at(t)`. Stage 1 supports:
//!
//! - **Flat** — one factor for every period.
//! - **Monthly** — one factor per UTC calendar month (January first).
//!   This expresses both the 2024 nuclear AGR outage pattern and windowed
//!   capacity such as the coal fleet (Ratcliffe closed 2024-09-30: a
//!   monthly profile with zeros from October).
//!
//! Factors are validated into `0.0..=1.0` at construction, so a held
//! `AvailabilityModel` is always physically meaningful.

use grid_core::GridError;
use grid_core::time::UtcInstant;
use grid_core::units::PerUnit;

/// A validated per-period capacity derating (see module docs).
#[derive(Debug, Clone, PartialEq)]
pub struct AvailabilityModel(Repr);

#[derive(Debug, Clone, PartialEq)]
enum Repr {
    Flat(PerUnit),
    Monthly([PerUnit; 12]),
}

fn check_factor(factor: PerUnit, context: &str) -> Result<(), GridError> {
    let value = factor.value();
    if !(0.0..=1.0).contains(&value) || value.is_nan() {
        return Err(GridError::InvalidRunInputs {
            reason: format!("{context}: availability factor {value} is outside 0.0..=1.0"),
        });
    }
    Ok(())
}

impl AvailabilityModel {
    /// A flat (constant) availability factor.
    ///
    /// Errors with [`GridError::InvalidRunInputs`] if the factor is
    /// outside `0.0..=1.0`.
    pub fn flat(factor: PerUnit) -> Result<Self, GridError> {
        check_factor(factor, "flat availability")?;
        Ok(Self(Repr::Flat(factor)))
    }

    /// A per-calendar-month availability profile, January first.
    ///
    /// Errors with [`GridError::InvalidRunInputs`] if any factor is
    /// outside `0.0..=1.0`.
    pub fn monthly(factors: [PerUnit; 12]) -> Result<Self, GridError> {
        for (index, factor) in factors.iter().enumerate() {
            check_factor(
                *factor,
                &format!("monthly availability, month {}", index + 1),
            )?;
        }
        Ok(Self(Repr::Monthly(factors)))
    }

    /// The availability factor applying at `instant` (for the monthly
    /// model, the factor of the instant's UTC calendar month).
    #[must_use]
    pub fn factor_at(&self, instant: UtcInstant) -> PerUnit {
        match &self.0 {
            Repr::Flat(factor) => *factor,
            Repr::Monthly(factors) => {
                let (_, month, _) = instant.civil_date();
                factors[usize::from(month) - 1]
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn flat_factor_applies_everywhere() {
        let model = AvailabilityModel::flat(PerUnit::new(0.75)).unwrap();
        let t = UtcInstant::parse("2024-06-15T12:00:00Z").unwrap();
        assert_eq!(model.factor_at(t), PerUnit::new(0.75));
    }

    #[test]
    fn monthly_factor_follows_the_utc_calendar_month() {
        let mut factors = [PerUnit::new(0.5); 12];
        factors[8] = PerUnit::new(0.9); // September
        factors[9] = PerUnit::new(0.0); // October
        let model = AvailabilityModel::monthly(factors).unwrap();
        let september = UtcInstant::parse("2024-09-30T23:30:00Z").unwrap();
        let october = UtcInstant::parse("2024-10-01T00:00:00Z").unwrap();
        assert_eq!(model.factor_at(september), PerUnit::new(0.9));
        assert_eq!(model.factor_at(october), PerUnit::new(0.0));
    }

    #[test]
    fn out_of_range_factors_are_rejected() {
        assert!(AvailabilityModel::flat(PerUnit::new(-0.01)).is_err());
        assert!(AvailabilityModel::flat(PerUnit::new(1.01)).is_err());
        assert!(AvailabilityModel::flat(PerUnit::new(f64::NAN)).is_err());
        assert!(AvailabilityModel::flat(PerUnit::new(0.0)).is_ok());
        assert!(AvailabilityModel::flat(PerUnit::new(1.0)).is_ok());
    }
}
