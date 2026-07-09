//! Delivered-basis revenue and capture accounting (Package A,
//! ratified 2026-07-03): tests for `delivered_renewable_power` — the
//! pro-rata allocation of the pooled curtailment across the
//! weather-driven renewables — and for the delivered-basis fields of
//! [`grid_adequacy::TechPricing`], computed ALONGSIDE the unchanged
//! potential-basis convention.
//!
//! Definitions under test (full prose at the code site,
//! `grid_adequacy::pricing`):
//!
//! - **Potential basis (Stage 2, unchanged):** renewable revenue,
//!   energy and capture are computed on potential (pre-curtailment)
//!   output.
//! - **Delivered basis (new):** per period, the pooled curtailment is
//!   attributed to the renewables pro-rata by potential output, capped
//!   at the renewable pool (spill from exogenous must-take beyond the
//!   renewable pool stays unattributed); delivered = potential − that
//!   share. Revenue/energy/capture recomputed on delivered output.
//!
//! Direction-of-divergence note (worked out before asserting, per the
//! work order): under the model's own SMP conventions curtailment
//! occurs only in surplus periods, which price at £0 (must-take-only,
//! grid-core pricing convention 2). Curtailed energy therefore earns
//! £0 on the potential basis too, so the two REVENUE totals are
//! identical in-model, while the delivered ENERGY is strictly smaller
//! whenever curtailment is nonzero. Consequently the delivered capture
//! price/ratio is **≥** the potential one — the naive expectation
//! "delivered capture ≤ potential capture" INVERTS. The true
//! invariants tested here are:
//!
//! 1. delivered energy ≤ potential energy (per technology, per period
//!    and per aggregate), equality exactly when curtailment is zero;
//! 2. delivered revenue ≤ potential revenue under non-negative prices,
//!    with equality when curtailment is zero (and also — the in-model
//!    case — whenever curtailment falls only in £0 periods);
//! 3. delivered capture price/ratio ≥ potential capture price/ratio
//!    whenever curtailed energy is priced at or below the potential
//!    capture price (always true in-model, where it is priced £0).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::pricing::delivered_renewable_power;
use grid_adequacy::{PricingInputs, RunResult, TechSeries, price_run};
use grid_core::pricing::{capture_price, capture_ratio, revenue};
use grid_core::scenario::{Reliability, TechId};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, Power, Price};
use proptest::prelude::*;

const START: &str = "2024-01-01T00:00:00Z";

fn start() -> UtcInstant {
    UtcInstant::parse(START).unwrap()
}

fn renewable(tech: &str, power_gw: &[f64]) -> TechSeries {
    TechSeries {
        tech: TechId::new(tech),
        reliability: Reliability::Variable,
        reliability_overridden: false,
        power: power_gw.iter().map(|&p| Power::gigawatts(p)).collect(),
    }
}

fn thermal(tech: &str, power_gw: &[f64]) -> TechSeries {
    TechSeries {
        tech: TechId::new(tech),
        reliability: Reliability::Firm,
        reliability_overridden: false,
        power: power_gw.iter().map(|&p| Power::gigawatts(p)).collect(),
    }
}

/// A hand-built single-zone run result: two renewables, one thermal
/// technology, a pooled curtailment series, no unserved energy.
fn synthetic_result(
    wind_gw: &[f64],
    solar_gw: &[f64],
    ccgt_gw: &[f64],
    curtailment_gw: &[f64],
) -> RunResult {
    let periods = wind_gw.len();
    assert_eq!(solar_gw.len(), periods);
    assert_eq!(ccgt_gw.len(), periods);
    assert_eq!(curtailment_gw.len(), periods);
    RunResult {
        start: start(),
        demand: vec![Power::gigawatts(10.0); periods],
        renewables: vec![renewable("wind", wind_gw), renewable("solar", solar_gw)],
        exogenous: vec![],
        thermal: vec![thermal("ccgt", ccgt_gw)],
        stores: vec![],
        curtailment: curtailment_gw
            .iter()
            .map(|&c| Power::gigawatts(c))
            .collect(),
        unserved: vec![Power::gigawatts(0.0); periods],
    }
}

fn energy_of(series: &[Power]) -> Energy {
    series
        .iter()
        .map(|&p| p * Duration::half_hour())
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

// ---------------------------------------------------------------------
// Deterministic allocation tests.
// ---------------------------------------------------------------------

/// Pro-rata sharing: potential [3, 1] GW with 2 GW pooled curtailment
/// delivers [1.5, 0.5] GW — a uniform curtailment fraction across the
/// renewables in the period.
#[test]
fn curtailment_is_shared_pro_rata_by_potential_output() {
    let result = synthetic_result(&[3.0], &[1.0], &[0.0], &[2.0]);
    let delivered = delivered_renewable_power(&result).unwrap();
    assert_eq!(delivered.len(), 2);
    assert!((delivered[0][0].as_gigawatts() - 1.5).abs() < 1e-12);
    assert!((delivered[1][0].as_gigawatts() - 0.5).abs() < 1e-12);
}

/// Pooled curtailment can exceed the renewable pool (exogenous
/// must-take spill); only the renewable pool's worth is attributed —
/// delivered output clamps at zero, never negative.
#[test]
fn curtailment_beyond_the_renewable_pool_clamps_delivered_at_zero() {
    let result = synthetic_result(&[1.5], &[0.5], &[0.0], &[5.0]);
    let delivered = delivered_renewable_power(&result).unwrap();
    assert_eq!(delivered[0][0], Power::gigawatts(0.0));
    assert_eq!(delivered[1][0], Power::gigawatts(0.0));
}

/// Zero curtailment leaves the delivered series bit-identical to the
/// potential series.
#[test]
fn zero_curtailment_gives_bit_identical_delivered_series() {
    let result = synthetic_result(&[3.25, 0.0, 7.5], &[1.0, 0.5, 0.0], &[5.0; 3], &[0.0; 3]);
    let delivered = delivered_renewable_power(&result).unwrap();
    for (series, out) in result.renewables.iter().zip(&delivered) {
        assert_eq!(&series.power, out);
    }
}

/// A curtailment period with zero renewable potential (pure exogenous
/// spill) attributes nothing and produces no NaN.
#[test]
fn zero_potential_period_with_curtailment_attributes_nothing() {
    let result = synthetic_result(&[0.0], &[0.0], &[0.0], &[3.0]);
    let delivered = delivered_renewable_power(&result).unwrap();
    assert_eq!(delivered[0][0], Power::gigawatts(0.0));
    assert_eq!(delivered[1][0], Power::gigawatts(0.0));
    assert!(delivered[0][0].as_gigawatts().is_finite());
}

/// Misaligned series are refused, not truncated.
#[test]
fn misaligned_series_are_an_error() {
    let mut result = synthetic_result(&[1.0, 2.0], &[1.0, 1.0], &[0.0, 0.0], &[0.5, 0.5]);
    result.renewables[0].power.pop();
    assert!(delivered_renewable_power(&result).is_err());
}

// ---------------------------------------------------------------------
// Property tests: the work-order invariants (a) and (b).
// ---------------------------------------------------------------------

proptest! {
    /// (b) Delivered energy ≤ potential energy, per technology and per
    /// period; equality (bit-exact) exactly when the period's
    /// curtailment is zero and the pool is nonzero.
    #[test]
    fn delivered_energy_never_exceeds_potential_energy(
        wind in prop::collection::vec(0.0f64..50.0, 1..48),
        solar_seed in prop::collection::vec(0.0f64..30.0, 48),
        // Curtailment as a fraction of the pool, allowed to overshoot
        // (up to 1.5× the pool) to exercise the exogenous-spill clamp.
        curtail_frac in prop::collection::vec(0.0f64..1.5, 48),
    ) {
        let n = wind.len();
        let solar = &solar_seed[..n];
        let curtailment: Vec<f64> = (0..n)
            .map(|t| (wind[t] + solar[t]) * curtail_frac[t])
            .collect();
        let result = synthetic_result(&wind, solar, &vec![0.0; n], &curtailment);
        let delivered = delivered_renewable_power(&result).unwrap();
        for (series, out) in result.renewables.iter().zip(&delivered) {
            for t in 0..n {
                prop_assert!(
                    out[t].as_gigawatts() <= series.power[t].as_gigawatts() + 1e-12,
                    "period {t}: delivered {} > potential {}",
                    out[t].as_gigawatts(),
                    series.power[t].as_gigawatts()
                );
                prop_assert!(out[t].as_gigawatts() >= 0.0);
                if curtailment[t] == 0.0 {
                    // Equality is exact when nothing is curtailed.
                    prop_assert_eq!(out[t], series.power[t]);
                }
            }
            prop_assert!(
                energy_of(out).as_gigawatt_hours()
                    <= energy_of(&series.power).as_gigawatt_hours() + 1e-9
            );
        }
        // Aggregate: pool minus attributed curtailment, exactly.
        let potential_pool = energy_of(&result.renewables[0].power)
            + energy_of(&result.renewables[1].power);
        let delivered_pool = energy_of(&delivered[0]) + energy_of(&delivered[1]);
        prop_assert!(
            delivered_pool.as_gigawatt_hours() <= potential_pool.as_gigawatt_hours() + 1e-9
        );
    }

    /// (a) Delivered revenue ≤ potential revenue under non-negative
    /// prices, per technology and per aggregate; equality when
    /// curtailment is zero everywhere.
    #[test]
    fn delivered_revenue_bounded_by_potential_revenue_under_nonnegative_prices(
        wind in prop::collection::vec(0.0f64..50.0, 1..48),
        solar_seed in prop::collection::vec(0.0f64..30.0, 48),
        curtail_frac in prop::collection::vec(0.0f64..1.0, 48),
        price_seed in prop::collection::vec(0.0f64..200.0, 48),
        curtail_on in prop::bool::ANY,
    ) {
        let n = wind.len();
        let solar = &solar_seed[..n];
        let curtailment: Vec<f64> = (0..n)
            .map(|t| {
                if curtail_on {
                    (wind[t] + solar[t]) * curtail_frac[t]
                } else {
                    0.0
                }
            })
            .collect();
        let smp: Vec<Price> = price_seed[..n]
            .iter()
            .map(|&p| Price::pounds_per_megawatt_hour(p))
            .collect();
        let result = synthetic_result(&wind, solar, &vec![0.0; n], &curtailment);
        let delivered = delivered_renewable_power(&result).unwrap();

        let mut potential_total = 0.0;
        let mut delivered_total = 0.0;
        for (series, out) in result.renewables.iter().zip(&delivered) {
            let potential = revenue(&series.power, &smp).unwrap().as_pounds();
            let delivered_rev = revenue(out, &smp).unwrap().as_pounds();
            prop_assert!(
                delivered_rev <= potential + 1e-6 * potential.abs().max(1.0),
                "delivered £{delivered_rev} > potential £{potential}"
            );
            potential_total += potential;
            delivered_total += delivered_rev;
        }
        if !curtail_on {
            // Zero curtailment: bit-identical series, identical revenue.
            prop_assert_eq!(delivered_total, potential_total);
        }
    }
}

// ---------------------------------------------------------------------
// (d) High-curtailment divergence, model-convention shaped: SMP is £0
// exactly in the curtailment (surplus) periods — the only shape the
// dispatch + SMP conventions can produce — and positive elsewhere.
// ---------------------------------------------------------------------

/// The two bases diverge visibly at high curtailment, in the direction
/// the definitions imply: revenue IDENTICAL (curtailed energy is priced
/// £0 on the potential basis too), delivered energy much smaller, so
/// delivered capture price and capture ratio are HIGHER than the
/// potential-basis ones. The naive "delivered capture ≤ potential
/// capture" inverts — see the module docs of this test file and the
/// prose at `grid_adequacy::pricing`.
#[test]
fn high_curtailment_diverges_with_delivered_capture_above_potential() {
    // Period 2 is a deep-surplus period: renewables 12 GW, demand
    // covered without thermal, 10 GW curtailed, SMP £0 (must-take-only,
    // grid-core convention 2). All other periods price at £100.
    let result = synthetic_result(
        &[4.0, 6.0, 8.0, 2.0],
        &[2.0, 2.0, 4.0, 0.0],
        &[4.0, 2.0, 0.0, 8.0],
        &[0.0, 0.0, 10.0, 0.0],
    );
    let smp = vec![
        Price::pounds_per_megawatt_hour(100.0),
        Price::pounds_per_megawatt_hour(100.0),
        Price::pounds_per_megawatt_hour(0.0),
        Price::pounds_per_megawatt_hour(100.0),
    ];
    let delivered = delivered_renewable_power(&result).unwrap();
    let wind_potential = &result.renewables[0].power;
    let wind_delivered = &delivered[0];

    // Revenue identical: curtailment lives only in the £0 period.
    let rev_potential = revenue(wind_potential, &smp).unwrap().as_pounds();
    let rev_delivered = revenue(wind_delivered, &smp).unwrap().as_pounds();
    assert_eq!(rev_delivered, rev_potential);
    assert!(rev_potential > 0.0);

    // Delivered energy strictly smaller.
    let e_potential = energy_of(wind_potential).as_gigawatt_hours();
    let e_delivered = energy_of(wind_delivered).as_gigawatt_hours();
    assert!(e_delivered < e_potential);

    // Capture diverges visibly, delivered ABOVE potential.
    let cp_potential = capture_price(wind_potential, &smp)
        .unwrap()
        .unwrap()
        .as_pounds_per_megawatt_hour();
    let cp_delivered = capture_price(wind_delivered, &smp)
        .unwrap()
        .unwrap()
        .as_pounds_per_megawatt_hour();
    assert!(
        cp_delivered > cp_potential * 1.2,
        "expected visible divergence: delivered £{cp_delivered}/MWh vs potential \
         £{cp_potential}/MWh"
    );
    let cr_potential = capture_ratio(wind_potential, &smp).unwrap().unwrap();
    let cr_delivered = capture_ratio(wind_delivered, &smp).unwrap().unwrap();
    assert!(
        cr_delivered > cr_potential,
        "delivered capture ratio {cr_delivered} not above potential {cr_potential}"
    );

    // Worked numbers (wind: 4+6+2 GW at £100, half-hour periods):
    // potential energy 10 GWh, delivered 10 − 8×(10/12)/2 = 6.667 GWh;
    // revenue £600k; capture £60/MWh potential vs £90/MWh delivered.
    assert!((cp_potential - 60.0).abs() < 1e-9);
    assert!((cp_delivered - 90.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------
// price_run carries both bases per technology.
// ---------------------------------------------------------------------

/// `price_run` populates the delivered-basis fields alongside the
/// unchanged potential-basis ones: renewables get the allocated
/// delivered series; thermal dispatch IS delivered output, so its two
/// bases coincide by construction.
#[test]
fn price_run_populates_both_bases_per_technology() {
    let result = synthetic_result(
        &[4.0, 6.0, 8.0, 2.0],
        &[2.0, 2.0, 4.0, 0.0],
        &[4.0, 2.0, 0.0, 8.0],
        &[0.0, 0.0, 10.0, 0.0],
    );
    let srmc = Trace::from_parts(start(), vec![Price::pounds_per_megawatt_hour(100.0); 4]).unwrap();
    let mut srmc_map = BTreeMap::new();
    srmc_map.insert(TechId::new("ccgt"), srmc);
    let inputs = PricingInputs {
        srmc: srmc_map,
        ef_electric_co2: BTreeMap::new(),
        ef_electric_co2e: BTreeMap::new(),
        observed_price: None,
    };
    let priced = price_run(&result, &inputs).unwrap();

    let tech = |name: &str| {
        priced
            .technologies
            .iter()
            .find(|t| t.tech.as_str() == name)
            .unwrap_or_else(|| panic!("no pricing entry for {name}"))
    };

    // Wind (renewable): potential basis unchanged, delivered basis
    // smaller in energy, equal in revenue (curtailment at £0 only),
    // higher in capture.
    let wind = tech("wind");
    assert!((wind.energy.as_gigawatt_hours() - 10.0).abs() < 1e-9);
    assert!(
        (wind.energy_delivered.as_gigawatt_hours() - (10.0 - 8.0 * (10.0 / 12.0) / 2.0)).abs()
            < 1e-9
    );
    assert_eq!(wind.revenue_delivered.as_pounds(), wind.revenue.as_pounds());
    let cp = wind.capture_price.unwrap().as_pounds_per_megawatt_hour();
    let cp_delivered = wind
        .capture_price_delivered
        .unwrap()
        .as_pounds_per_megawatt_hour();
    assert!(cp_delivered > cp);
    assert!(wind.capture_ratio_delivered.unwrap() > wind.capture_ratio.unwrap());

    // Thermal: the two bases coincide exactly.
    let ccgt = tech("ccgt");
    assert_eq!(ccgt.energy_delivered, ccgt.energy);
    assert_eq!(ccgt.revenue_delivered.as_pounds(), ccgt.revenue.as_pounds());
    assert_eq!(ccgt.capture_price_delivered, ccgt.capture_price);
    assert_eq!(ccgt.capture_ratio_delivered, ccgt.capture_ratio);
}
