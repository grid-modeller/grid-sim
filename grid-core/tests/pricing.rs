//! Stage 2 pricing layer (ADR-9): SRMC arithmetic, the carbon step
//! series, the system-marginal-price conventions, revenue/capture
//! accounting, emissions totals, and the reported realism statistics.
//!
//! The SRMC hand-check reproduces the 2024 price-pack cross-check
//! (docs/notes/2024-price-pack-report.md §3): mean 2024 inputs — gas SAP
//! £28.67/MWh_th HHV, UKA step-series mean £37.13/tCO2 + CPS £18, DUKES
//! CCGT η 0.4893 HHV, EF 0.18253 tCO2/MWh_th HHV — give a CCGT SRMC of
//! £79.16/MWh_e.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use grid_core::GridError;
use grid_core::pricing::{
    CarbonAuction, PricedSeries, capture_price, capture_ratio, carbon_price_step_series,
    median_ratio, monthly_mean_correlation, price_setting_share, revenue, srmc_series,
    system_marginal_price, time_weighted_mean_price, total_emissions,
};
use grid_core::scenario::TechId;
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{CarbonPrice, Emissions, EmissionsRate, Money, PerUnit, Power, Price};

fn t0() -> UtcInstant {
    UtcInstant::parse("2024-01-01T00:00:00Z").unwrap()
}

fn price_trace(values: &[f64]) -> Trace<Price> {
    Trace::from_parts(
        t0(),
        values
            .iter()
            .map(|&v| Price::pounds_per_megawatt_hour(v))
            .collect(),
    )
    .unwrap()
}

fn carbon_trace(values: &[f64]) -> Trace<CarbonPrice> {
    Trace::from_parts(
        t0(),
        values
            .iter()
            .map(|&v| CarbonPrice::pounds_per_tonne_co2(v))
            .collect(),
    )
    .unwrap()
}

fn gw(values: &[f64]) -> Vec<Power> {
    values.iter().map(|&v| Power::gigawatts(v)).collect()
}

fn pounds_per_mwh(values: &[f64]) -> Vec<Price> {
    values
        .iter()
        .map(|&v| Price::pounds_per_megawatt_hour(v))
        .collect()
}

// ---------------------------------------------------------------------
// SRMC arithmetic.
// ---------------------------------------------------------------------

#[test]
fn srmc_series_reproduces_the_2024_mean_hand_check() {
    // Mean 2024 inputs → £79.16/MWh_e (price-pack report §3).
    let fuel = price_trace(&[28.67, 28.67]);
    let carbon = carbon_trace(&[55.13, 55.13]); // UKA mean 37.13 + CPS 18
    let srmc = srmc_series(
        &fuel,
        &carbon,
        PerUnit::new(0.4893),
        EmissionsRate::tonnes_per_megawatt_hour(0.18253),
    )
    .unwrap();
    for value in srmc.values() {
        assert!(
            (value.as_pounds_per_megawatt_hour() - 79.16).abs() < 0.005,
            "SRMC {} != 79.16",
            value.as_pounds_per_megawatt_hour()
        );
    }
}

#[test]
fn srmc_series_rejects_invalid_efficiencies() {
    let fuel = price_trace(&[30.0]);
    let carbon = carbon_trace(&[50.0]);
    for eta in [0.0, -0.5, 1.01, f64::NAN] {
        let err = srmc_series(
            &fuel,
            &carbon,
            PerUnit::new(eta),
            EmissionsRate::tonnes_per_megawatt_hour(0.18),
        )
        .unwrap_err();
        assert!(
            matches!(err, GridError::InvalidPricing { .. }),
            "eta {eta}: {err:?}"
        );
    }
}

#[test]
fn srmc_series_rejects_misaligned_traces() {
    let fuel = price_trace(&[30.0, 30.0]);
    let carbon = carbon_trace(&[50.0]); // one period short
    let err = srmc_series(
        &fuel,
        &carbon,
        PerUnit::new(0.5),
        EmissionsRate::tonnes_per_megawatt_hour(0.18),
    )
    .unwrap_err();
    assert!(matches!(err, GridError::InvalidPricing { .. }), "{err:?}");
}

// ---------------------------------------------------------------------
// The UKA + CPS carbon step series (reference-file convention: forward-
// fill from the most recent auction; before the first auction, use the
// first auction's price; CPS added throughout).
// ---------------------------------------------------------------------

#[test]
fn carbon_step_series_forward_fills_from_auction_dates() {
    let auctions = vec![
        CarbonAuction {
            date: UtcInstant::parse("2024-01-10T00:00:00Z").unwrap(),
            clearing_price: CarbonPrice::pounds_per_tonne_co2(37.02),
        },
        CarbonAuction {
            date: UtcInstant::parse("2024-01-24T00:00:00Z").unwrap(),
            clearing_price: CarbonPrice::pounds_per_tonne_co2(32.61),
        },
    ];
    // 48 half-hours per day; start 2024-01-01, run 25 days.
    let periods = 25 * 48;
    let series = carbon_price_step_series(
        &auctions,
        CarbonPrice::pounds_per_tonne_co2(18.0),
        t0(),
        periods,
    )
    .unwrap();
    assert_eq!(series.len(), periods);
    let v = |i: usize| series.values()[i].as_pounds_per_tonne_co2();
    // Before the first auction: first auction's price + CPS.
    assert_eq!(v(0), 37.02 + 18.0);
    // On the first auction day (from 00:00 UTC).
    assert_eq!(v(9 * 48), 37.02 + 18.0);
    // Last period before the second auction.
    assert_eq!(v(23 * 48 - 1), 37.02 + 18.0);
    // From the second auction date onward.
    assert_eq!(v(23 * 48), 32.61 + 18.0);
    assert_eq!(v(periods - 1), 32.61 + 18.0);
}

#[test]
fn carbon_step_series_accepts_unsorted_auctions() {
    let auctions = vec![
        CarbonAuction {
            date: UtcInstant::parse("2024-01-24T00:00:00Z").unwrap(),
            clearing_price: CarbonPrice::pounds_per_tonne_co2(32.61),
        },
        CarbonAuction {
            date: UtcInstant::parse("2024-01-10T00:00:00Z").unwrap(),
            clearing_price: CarbonPrice::pounds_per_tonne_co2(37.02),
        },
    ];
    let series =
        carbon_price_step_series(&auctions, CarbonPrice::pounds_per_tonne_co2(0.0), t0(), 48)
            .unwrap();
    assert_eq!(series.values()[0].as_pounds_per_tonne_co2(), 37.02);
}

#[test]
fn carbon_step_series_rejects_empty_auctions() {
    let err = carbon_price_step_series(&[], CarbonPrice::pounds_per_tonne_co2(18.0), t0(), 48)
        .unwrap_err();
    assert!(matches!(err, GridError::InvalidPricing { .. }), "{err:?}");
}

// ---------------------------------------------------------------------
// System marginal price conventions (documented in the pricing module):
// the most expensive *dispatched* SRMC-bearing technology sets the
// price; must-take-only periods price at zero with no setter; unserved
// periods price at the fleet's SRMC ceiling.
// ---------------------------------------------------------------------

fn tech(name: &str) -> TechId {
    TechId::new(name)
}

#[test]
fn smp_is_the_srmc_of_the_most_expensive_dispatched_priced_technology() {
    let ccgt_power = gw(&[10.0, 10.0, 0.0]);
    let ocgt_power = gw(&[0.5, 0.0, 0.0]);
    let nuclear_power = gw(&[5.0, 5.0, 5.0]);
    let ccgt_srmc = pounds_per_mwh(&[80.0, 80.0, 80.0]);
    let ocgt_srmc = pounds_per_mwh(&[110.0, 110.0, 110.0]);
    let series = [
        PricedSeries {
            tech: tech("nuclear"),
            power: &nuclear_power,
            srmc: None,
        },
        PricedSeries {
            tech: tech("ccgt"),
            power: &ccgt_power,
            srmc: Some(&ccgt_srmc),
        },
        PricedSeries {
            tech: tech("ocgt"),
            power: &ocgt_power,
            srmc: Some(&ocgt_srmc),
        },
    ];
    let unserved = gw(&[0.0, 0.0, 0.0]);
    let prices = system_marginal_price(&series, &unserved).unwrap();

    // Period 0: OCGT dispatched — the most expensive dispatched sets it.
    assert_eq!(prices.smp[0], Price::pounds_per_megawatt_hour(110.0));
    assert_eq!(prices.setter[0], Some(tech("ocgt")));
    // Period 1: CCGT only.
    assert_eq!(prices.smp[1], Price::pounds_per_megawatt_hour(80.0));
    assert_eq!(prices.setter[1], Some(tech("ccgt")));
    // Period 2: no priced technology dispatched → zero, no setter.
    assert_eq!(prices.smp[2], Price::pounds_per_megawatt_hour(0.0));
    assert_eq!(prices.setter[2], None);
}

#[test]
fn unserved_periods_price_at_the_fleet_srmc_ceiling() {
    let ccgt_power = gw(&[10.0]);
    let ccgt_srmc = pounds_per_mwh(&[80.0]);
    let ocgt_power = gw(&[0.0]); // exhausted elsewhere / unavailable
    let ocgt_srmc = pounds_per_mwh(&[110.0]);
    let series = [
        PricedSeries {
            tech: tech("ccgt"),
            power: &ccgt_power,
            srmc: Some(&ccgt_srmc),
        },
        PricedSeries {
            tech: tech("ocgt"),
            power: &ocgt_power,
            srmc: Some(&ocgt_srmc),
        },
    ];
    let unserved = gw(&[0.2]);
    let prices = system_marginal_price(&series, &unserved).unwrap();
    // The ceiling technology prices the unserved period even though it is
    // not dispatched (a documented floor on the true scarcity price).
    assert_eq!(prices.smp[0], Price::pounds_per_megawatt_hour(110.0));
    assert_eq!(prices.setter[0], Some(tech("ocgt")));
}

#[test]
fn unserved_periods_without_any_priced_technology_are_an_error() {
    let nuclear_power = gw(&[5.0]);
    let series = [PricedSeries {
        tech: tech("nuclear"),
        power: &nuclear_power,
        srmc: None,
    }];
    let unserved = gw(&[1.0]);
    let err = system_marginal_price(&series, &unserved).unwrap_err();
    assert!(matches!(err, GridError::InvalidPricing { .. }), "{err:?}");
}

#[test]
fn smp_rejects_misaligned_series() {
    let power = gw(&[1.0, 1.0]);
    let srmc = pounds_per_mwh(&[80.0]); // shorter than power
    let series = [PricedSeries {
        tech: tech("ccgt"),
        power: &power,
        srmc: Some(&srmc),
    }];
    let unserved = gw(&[0.0, 0.0]);
    let err = system_marginal_price(&series, &unserved).unwrap_err();
    assert!(matches!(err, GridError::InvalidPricing { .. }), "{err:?}");
}

// ---------------------------------------------------------------------
// Revenue, capture price, capture ratio.
// ---------------------------------------------------------------------

#[test]
fn revenue_and_capture_price_hand_check() {
    // 2 GW then 0 GW over two half-hours at £100 then £50:
    // revenue = 1 GWh × £100/MWh = £100,000; energy 1 GWh; capture £100.
    let power = gw(&[2.0, 0.0]);
    let smp = pounds_per_mwh(&[100.0, 50.0]);
    assert_eq!(revenue(&power, &smp).unwrap(), Money::pounds(100_000.0));
    assert_eq!(
        capture_price(&power, &smp).unwrap(),
        Some(Price::pounds_per_megawatt_hour(100.0))
    );
}

#[test]
fn capture_price_of_zero_output_is_none_not_nan() {
    let power = gw(&[0.0, 0.0]);
    let smp = pounds_per_mwh(&[100.0, 50.0]);
    assert_eq!(capture_price(&power, &smp).unwrap(), None);
    assert_eq!(capture_ratio(&power, &smp).unwrap(), None);
}

#[test]
fn capture_ratio_hand_check() {
    // Time-weighted mean SMP = £75/MWh; capture £100 → ratio 4/3.
    let power = gw(&[2.0, 0.0]);
    let smp = pounds_per_mwh(&[100.0, 50.0]);
    assert_eq!(
        time_weighted_mean_price(&smp),
        Some(Price::pounds_per_megawatt_hour(75.0))
    );
    let ratio = capture_ratio(&power, &smp).unwrap().unwrap();
    assert!((ratio - 100.0 / 75.0).abs() < 1e-12, "ratio {ratio}");
}

#[test]
fn revenue_rejects_misaligned_series() {
    let power = gw(&[1.0]);
    let smp = pounds_per_mwh(&[100.0, 50.0]);
    assert!(matches!(
        revenue(&power, &smp).unwrap_err(),
        GridError::InvalidPricing { .. }
    ));
}

// ---------------------------------------------------------------------
// Emissions accounting.
// ---------------------------------------------------------------------

#[test]
fn total_emissions_hand_check() {
    // 10 GW for two half-hours = 10 GWh = 10,000 MWh at 0.373 tCO2/MWh_e
    // = 3,730 tCO2.
    let power = gw(&[10.0, 10.0]);
    let emissions = total_emissions(&power, EmissionsRate::tonnes_per_megawatt_hour(0.373));
    assert_eq!(emissions, Emissions::tonnes_co2(3730.0));
}

// ---------------------------------------------------------------------
// Price-setting share and the reported realism statistics.
// ---------------------------------------------------------------------

#[test]
fn price_setting_share_counts_the_named_technologies() {
    let setter = vec![
        Some(tech("ccgt")),
        Some(tech("ocgt")),
        Some(tech("coal")),
        None,
    ];
    let share = price_setting_share(&setter, &["ccgt", "ocgt"]);
    assert!((share - 0.5).abs() < 1e-12, "share {share}");
}

#[test]
fn median_ratio_is_the_median_of_per_period_ratios() {
    let model = pounds_per_mwh(&[100.0, 90.0, 300.0]);
    let observed = pounds_per_mwh(&[100.0, 100.0, 100.0]);
    // Ratios 1.0, 0.9, 3.0 → median 1.0.
    let median = median_ratio(&model, &observed).unwrap();
    assert!((median - 1.0).abs() < 1e-12, "median {median}");
}

#[test]
fn monthly_mean_correlation_is_pearson_over_calendar_month_means() {
    // Two calendar months (Jan + Feb 2024), model = 2 × observed →
    // perfect correlation.
    let periods = (31 + 29) * 48;
    // A rising trend so the two monthly means differ (a flat series has
    // zero across-month variance and no defined correlation).
    let observed: Vec<f64> = (0..periods)
        .map(|i| 50.0 + (i % 48) as f64 + i as f64 / 100.0)
        .collect();
    let model: Vec<f64> = observed.iter().map(|v| v * 2.0).collect();
    let r = monthly_mean_correlation(t0(), &pounds_per_mwh(&model), &pounds_per_mwh(&observed))
        .unwrap();
    assert!((r - 1.0).abs() < 1e-9, "r {r}");
}

#[test]
fn monthly_mean_correlation_needs_at_least_two_months() {
    let model = pounds_per_mwh(&[80.0; 48]);
    let observed = pounds_per_mwh(&[70.0; 48]);
    assert!(matches!(
        monthly_mean_correlation(t0(), &model, &observed).unwrap_err(),
        GridError::InvalidPricing { .. }
    ));
}

// ---------------------------------------------------------------------
// Property test: revenue conservation — the sum of per-technology
// revenues equals the SMP-weighted total dispatch (Σ_tech Σ_t p×Δt×SMP =
// Σ_t SMP×Δt×Σ_tech p).
// ---------------------------------------------------------------------

mod revenue_conservation {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn per_tech_revenues_sum_to_smp_weighted_total_dispatch(
            powers in proptest::collection::vec(
                proptest::collection::vec(0.0f64..40.0, 24), 1..5),
            prices in proptest::collection::vec(-100.0f64..500.0, 24),
        ) {
            let smp = pounds_per_mwh(&prices);
            let mut sum_revenue = 0.0;
            let mut total = vec![0.0f64; 24];
            for series in &powers {
                let power = gw(series);
                sum_revenue += revenue(&power, &smp).unwrap().as_pounds();
                for (acc, p) in total.iter_mut().zip(series) {
                    *acc += p;
                }
            }
            let direct = revenue(&gw(&total), &smp).unwrap().as_pounds();
            let scale = sum_revenue.abs().max(direct.abs()).max(1.0);
            prop_assert!(
                (sum_revenue - direct).abs() / scale < 1e-9,
                "Σ per-tech {} != direct {}", sum_revenue, direct
            );
        }
    }
}

// ---------------------------------------------------------------------
// The committed 2024 prices-reference file parses and matches its own
// documented cross-checks (characterisation of
// data/reference/prices-2024.toml).
// ---------------------------------------------------------------------

mod prices_reference {
    use super::*;
    use grid_core::prices_reference::PricesReference;
    use std::path::PathBuf;

    fn reference_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("data/reference/prices-2024.toml")
    }

    #[test]
    fn committed_reference_file_parses_with_documented_cross_checks() {
        let reference = PricesReference::load(&reference_path()).unwrap();
        assert_eq!(reference.year, 2024);

        // 25 UKA auctions; mean £37.18 (reference file cross-check).
        assert_eq!(reference.uka_auctions.len(), 25);
        let mean = reference
            .uka_auctions
            .iter()
            .map(|a| a.clearing_price.as_pounds_per_tonne_co2())
            .sum::<f64>()
            / 25.0;
        assert!((mean - 37.18).abs() < 0.005, "UKA mean {mean}");
        // Auctions dated within 2024, ascending.
        let first = reference.uka_auctions.first().unwrap().date;
        assert_eq!(first, UtcInstant::parse("2024-01-10T00:00:00Z").unwrap());

        // CPS £18/tCO2 (frozen since 2016).
        assert_eq!(reference.cps, CarbonPrice::pounds_per_tonne_co2(18.0));

        // Emission factors: CO2-only for pricing, CO2e for accounting.
        assert_eq!(
            reference.ef_co2_thermal,
            EmissionsRate::tonnes_per_megawatt_hour(0.18253)
        );
        assert_eq!(
            reference.ef_co2e_thermal,
            EmissionsRate::tonnes_per_megawatt_hour(0.18290)
        );

        // Fleet efficiencies, HHV basis.
        assert_eq!(
            reference.efficiency_hhv.get("ccgt"),
            Some(&PerUnit::new(0.4893))
        );
        assert_eq!(
            reference.efficiency_hhv.get("ocgt"),
            Some(&PerUnit::new(0.349))
        );

        // Monthly SAP: 12 months; unweighted mean £28.64 (file cross-check).
        assert_eq!(reference.gas_monthly_sap.len(), 12);
        let sap_mean = reference
            .gas_monthly_sap
            .iter()
            .map(|(_, p)| p.as_pounds_per_megawatt_hour())
            .sum::<f64>()
            / 12.0;
        assert!((sap_mean - 28.64).abs() < 0.005, "SAP mean {sap_mean}");
    }

    #[test]
    fn wrong_schema_string_is_rejected() {
        let err = PricesReference::from_toml_str("schema = \"prices-reference-v2\"\nyear = 2024\n")
            .unwrap_err();
        assert!(
            matches!(err, GridError::InvalidPricesReference { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let reference = std::fs::read_to_string(reference_path()).unwrap();
        let with_extra = format!("{reference}\n[frobnicate]\nx = 1\n");
        let err = PricesReference::from_toml_str(&with_extra).unwrap_err();
        assert!(err.to_string().contains("frobnicate"), "{err}");
    }
}
