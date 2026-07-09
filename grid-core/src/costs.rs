//! Stage 7 annualisation arithmetic and WACC banding (D8 rule 4,
//! `docs/notes/d8-lcoe-methods.md`).
//!
//! The capital recovery factor is the pinned formula
//!
//! ```text
//! CRF = r (1 + r)^n / ((1 + r)^n − 1)
//! ```
//!
//! with `r` the real WACC and `n` the cited asset life in years; a
//! technology's annuity is its **overnight** capex × CRF. Every
//! headline cost is quoted at the three pinned WACCs (4.5 / 7.5 /
//! 10.0 % real, uniform across technologies — docs/04 Stage 7 pin);
//! [`WaccBand`] is the carrier type that makes a single-WACC output
//! structurally impossible.
//!
//! **Documented limitation (basis stamp):** the annuity here is
//! computed on the raw overnight capex, NOT escalated over the
//! source's build phasing at the WACC (interest during construction).
//! The phasing arrays are parsed and available on every
//! [`crate::costs_reference`] row (review condition 11), but the IDC
//! escalation is explicitly out of scope for this package; using the
//! overnight number under-costs long-build technologies (nuclear
//! 8-year build, offshore 3-year) relative to the source's own method
//! (evidence note `docs/notes/stage7-cost-inputs-report.md` §1). Every
//! cost-stack artefact carries this limitation in its metadata until
//! the escalation lands.

use crate::GridError;
use crate::units::{
    AnnualCapacityCost, AnnualEnergyCapacityCost, CapacityCost, EnergyCapacityCost, PerUnit,
};

/// One value per pinned WACC (low / central / high) — every Stage 7
/// cost output is carried in this shape, so no single-WACC number can
/// leave the engine (D8 rule 4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaccBand<T> {
    /// Value at the low WACC (4.5 % real).
    pub low: T,
    /// Value at the central WACC (7.5 % real).
    pub central: T,
    /// Value at the high WACC (10.0 % real).
    pub high: T,
}

impl<T> WaccBand<T> {
    /// Apply a fallible function at each band point.
    pub fn try_map<U>(
        &self,
        mut f: impl FnMut(&T) -> Result<U, GridError>,
    ) -> Result<WaccBand<U>, GridError> {
        Ok(WaccBand {
            low: f(&self.low)?,
            central: f(&self.central)?,
            high: f(&self.high)?,
        })
    }

    /// Apply a function at each band point.
    pub fn map<U>(&self, mut f: impl FnMut(&T) -> U) -> WaccBand<U> {
        WaccBand {
            low: f(&self.low),
            central: f(&self.central),
            high: f(&self.high),
        }
    }
}

/// Validate an annualisation input pair: a real WACC strictly inside
/// (0, 1) and a positive asset life.
fn check_inputs(rate: PerUnit, life_years: u32) -> Result<(), GridError> {
    let r = rate.value();
    if !(r > 0.0 && r < 1.0) {
        return Err(GridError::InvalidCostInputs {
            reason: format!("real WACC {r} is outside (0, 1) — rates are fractions, not percent"),
        });
    }
    if life_years == 0 {
        return Err(GridError::InvalidCostInputs {
            reason: "asset life of 0 years cannot be annualised".to_owned(),
        });
    }
    Ok(())
}

/// The capital recovery factor `r(1+r)^n / ((1+r)^n − 1)` as a per-year
/// fraction of capex (D8 rule 4).
///
/// Errors with [`GridError::InvalidCostInputs`] on a rate outside
/// (0, 1) or a zero life.
pub fn capital_recovery_factor(rate: PerUnit, life_years: u32) -> Result<PerUnit, GridError> {
    check_inputs(rate, life_years)?;
    let r = rate.value();
    let x = (1.0 + r).powi(life_years as i32);
    Ok(PerUnit::new(r * x / (x - 1.0)))
}

/// The rule-4 annuity of an overnight capacity capex: £/kW × CRF,
/// returned in the canonical £/MW/yr (the 10³ kW→MW factor is applied
/// here). See the module docs for the overnight-basis limitation.
pub fn annuity_per_mw(
    capex: CapacityCost,
    rate: PerUnit,
    life_years: u32,
) -> Result<AnnualCapacityCost, GridError> {
    let crf = capital_recovery_factor(rate, life_years)?;
    Ok(AnnualCapacityCost::pounds_per_megawatt_year(
        capex.as_pounds_per_kilowatt() * 1.0e3 * crf.value(),
    ))
}

/// The rule-4 annuity of an overnight energy-capacity capex (storage
/// energy leg): £/kWh × CRF = £/kWh/yr.
pub fn annuity_per_kwh(
    capex: EnergyCapacityCost,
    rate: PerUnit,
    life_years: u32,
) -> Result<AnnualEnergyCapacityCost, GridError> {
    let crf = capital_recovery_factor(rate, life_years)?;
    Ok(AnnualEnergyCapacityCost::pounds_per_kilowatt_hour_year(
        capex.as_pounds_per_kilowatt_hour() * crf.value(),
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn crf_of_a_one_year_life_repays_principal_plus_interest() {
        let crf = capital_recovery_factor(PerUnit::new(0.10), 1).unwrap();
        assert!((crf.value() - 1.10).abs() < 1e-12);
    }

    #[test]
    fn crf_falls_with_longer_life_and_rises_with_rate() {
        let short = capital_recovery_factor(PerUnit::new(0.075), 15).unwrap();
        let long = capital_recovery_factor(PerUnit::new(0.075), 60).unwrap();
        assert!(long < short);
        let low = capital_recovery_factor(PerUnit::new(0.045), 25).unwrap();
        let high = capital_recovery_factor(PerUnit::new(0.100), 25).unwrap();
        assert!(low < high);
    }

    #[test]
    fn annuity_applies_the_kw_to_mw_factor() {
        // £1,000/kW at CRF ≈ 1.1 (one year, 10 %) is £1.1m/MW/yr.
        let annuity = annuity_per_mw(
            CapacityCost::pounds_per_kilowatt(1000.0),
            PerUnit::new(0.10),
            1,
        )
        .unwrap();
        assert!((annuity.as_pounds_per_megawatt_year() - 1.1e6).abs() < 1e-6);
    }

    #[test]
    fn energy_annuity_stays_per_kwh() {
        let annuity = annuity_per_kwh(
            EnergyCapacityCost::pounds_per_kilowatt_hour(100.0),
            PerUnit::new(0.10),
            1,
        )
        .unwrap();
        assert!((annuity.as_pounds_per_kilowatt_hour_year() - 110.0).abs() < 1e-9);
    }

    #[test]
    fn degenerate_inputs_are_structured_errors() {
        for (rate, life) in [(0.0, 25), (-0.05, 25), (1.0, 25), (7.5, 25), (0.075, 0)] {
            let result = capital_recovery_factor(PerUnit::new(rate), life);
            assert!(
                matches!(result, Err(GridError::InvalidCostInputs { .. })),
                "rate {rate}, life {life}"
            );
        }
    }

    #[test]
    fn wacc_band_maps_pointwise() {
        let band = WaccBand {
            low: 1.0,
            central: 2.0,
            high: 3.0,
        };
        let doubled = band.map(|v| v * 2.0);
        assert_eq!(doubled.low, 2.0);
        assert_eq!(doubled.central, 4.0);
        assert_eq!(doubled.high, 6.0);
    }
}
