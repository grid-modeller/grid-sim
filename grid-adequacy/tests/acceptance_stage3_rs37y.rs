//! Stage 3 part 2 acceptance tests (docs/04 Stage 3; docs/07 Module 3):
//! the Royal-Society-style 37+-year storage runs, first passed against
//! the complete real 1985–2024 record on 2026-07-03 and PINNED (docs/05
//! pin discipline: every published number gets a pinned regression).
//!
//! - (a) `min_storage_for_zero_unserved` on the RS-style scenario
//!   (wind + solar + hydrogen only, 1985–2024) lands in the published
//!   order of magnitude — tens of TWh — and is pinned exactly;
//! - (b) SoC carries across every year boundary (no annual resets — D4
//!   mechanics: resets are exactly how "a few days of storage" errors
//!   happen);
//! - (c) the benign-vs-37-year contrast: the 12 h battery that passes
//!   the single benign 2024 year (acceptance_stage3_2024.rs) fails
//!   catastrophically on the full record — Module 3(a) vs (b), pinned;
//! - (d) the OVERBUILD CURVE (Stage 3 finding): the requirement is
//!   violently sensitive to supply overbuild — pinned points at 0.70×
//!   (scenarios/royal-society-37y-lean.toml) and 0.85× capacities
//!   (scenarios/royal-society-37y-mid.toml; quoted by the stage-3 run
//!   report, hence pinned per the published-number rule), and an
//!   infeasibility cliff at 0.60×. Measured 2026-07-03 (all three
//!   capacities scaled): 0.60× → infeasible at the 10⁶ GWh cap;
//!   0.70× → 58,432 GWh; 0.85× → 28,336 GWh; 1.00× → 23,872 GWh.
//!   The curve, not any single point, is the finding. RS-comparability
//!   (verified against the published report, "Large-scale electricity
//!   storage", 2023 — anchors in the stage-3 run report): the RS
//!   report sized wind + solar supply at 1.23–1.40× annual demand, so
//!   the lean 0.70× point (~1.35×) is the RS-COMPARABLE one; mid
//!   (~1.64×) and headline (~1.92×) sit above the RS range. Store-side
//!   GWh here are not directly comparable to RS-quoted TWh (asymmetric
//!   legs, different accounting basis — unit table in the run report
//!   §3).
//!
//! Determinism basis for the exact pins:
//! `results = f(scenario, data-pack checksums, engine)` (ADR-5). The
//! pinned values hold bit-for-bit while the scenario files, the
//! per-year pack (data/packs/cf, data/packs/demand-tiled, manifested)
//! and the engine are unchanged; a deliberate change to any of them is
//! a knowing re-pin, recorded in the stage run report.
//!
//! These tests need the per-year 1985–2024 data pack (fetched/derived,
//! not committed). They FAIL LOUDLY (no `#[ignore]`) if it is missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use grid_adequacy::{SolveOptions, load_run_inputs, min_storage_for_zero_unserved, run};
use grid_core::scenario::{Scenario, TraceFiles};
use grid_core::units::Energy;

const RS_SCENARIO: &str = "scenarios/royal-society-37y.toml";
const LEAN_SCENARIO: &str = "scenarios/royal-society-37y-lean.toml";
const MID_SCENARIO: &str = "scenarios/royal-society-37y-mid.toml";
const BENIGN_SCENARIO: &str = "scenarios/gb-2024-benign-battery.toml";

/// The full 1985–2024 record: 30 × 365 + 10 × 366 days, half-hourly.
const FULL_RECORD_PERIODS: usize = 701_280;

/// The docs/04 Stage 3 acceptance band, store-side GWh: "tens of TWh".
/// Deliberately generous: the Royal Society quotes ~60–100 TWh
/// store-side under ASYMMETRIC per-leg efficiencies (electrolysis
/// ≈ 0.7, reconversion ≈ 0.5); our symmetric √η split shifts
/// store-side numbers by up to ~15–20 % (D4 comparison convention,
/// owned — docs/notes/d4-rule-based-dispatch.md §Mechanics), and the
/// fleet/demand assumptions are RS-comparable, not RS-identical (see
/// the scenario header). The band checks the ORDER OF MAGNITUDE; the
/// pinned exact values (in the tests below, since the 2026-07-03 first
/// pass) are the regressions.
const BAND_LO_GWH: f64 = 20_000.0;
const BAND_HI_GWH: f64 = 200_000.0;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Half-hourly periods in calendar `year`.
fn periods_in_year(year: i32) -> usize {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    if leap { 17_568 } else { 17_520 }
}

/// Fail loudly (the acceptance tests are meant to be red, not skipped,
/// until the concurrent data package finishes the 1985–2024 per-year
/// derivation) if any per-year file is missing.
fn require_full_record_pack() {
    let root = repo_root();
    let mut missing: Vec<String> = Vec::new();
    for year in 1985..=2024 {
        for rel in [
            format!("data/packs/demand-tiled/demand_{year}.parquet"),
            format!("data/packs/cf/gb_onshore_cf_{year}.parquet"),
            format!("data/packs/cf/gb_offshore_cf_{year}.parquet"),
            format!("data/packs/cf/gb_solar_cf_{year}.parquet"),
        ] {
            if !root.join(&rel).exists() {
                missing.push(rel);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the per-year 1985–2024 data pack is incomplete: {} file(s) missing, first {} — \
         it is being produced by the concurrent data package (ERA5 Phase B derivation, \
         scripts/era5-cf; tiled demand, scripts/fetch-2024). These Stage 3 part 2 \
         acceptance tests stay RED until the record is complete.",
        missing.len(),
        missing[0]
    );
}

/// Acceptance (a): the bisection solver on the RS-style scenario finds
/// a hydrogen energy requirement of the published order of magnitude —
/// tens of TWh, band [20, 200] TWh store-side (see `BAND_LO_GWH`).
/// The D4 initial-SoC guard is applied: when the naive solve leans on
/// the initial-full store within the first weather year, the one-year
/// burn-in figure must ALSO sit in the band, and both are reported.
///
/// PINNED (first real-record pass, 2026-07-03): requirement exactly
/// 23,872 GWh — exact because every bisection candidate is
/// doubling/halving arithmetic on whole GWh (binary-exact f64) and the
/// run is deterministic (module docs). Min SoC at that size:
/// 8.870000113328198 GWh at 1989-12-13T02:00:00Z (the 1989 winter
/// nadir — NOT in the first weather year, so the D4 guard does not
/// flag and no burn-in re-run happens). NOTE (reviewer pin-time
/// condition): 23.9 TWh sits BELOW the ~[40, 150] TWh quotable band
/// for the RS comparison — this point alone is not quotable as "the
/// RS number"; the finding is the overbuild curve (module docs; the
/// 0.70× point in `lean_rs_scenario_pins_the_overbuild_curve` is the
/// one inside the quotable band).
#[test]
fn rs_scenario_min_storage_is_tens_of_twh() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(RS_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let result =
        min_storage_for_zero_unserved(&scenario, &inputs, 0, &SolveOptions::default()).unwrap();

    let naive_gwh = result.naive.requirement.as_gigawatt_hours();
    eprintln!(
        "RS 37+-year solve: naive requirement {naive_gwh} GWh; min SoC \
         {} GWh at {}; initial-condition-sensitive: {}",
        result.min_soc.as_gigawatt_hours(),
        result.min_soc_at,
        result.initial_condition_sensitive
    );
    assert!(
        (BAND_LO_GWH..=BAND_HI_GWH).contains(&naive_gwh),
        "naive hydrogen requirement {naive_gwh:.1} GWh is outside the published-order band \
         [{BAND_LO_GWH}, {BAND_HI_GWH}] GWh (tens of TWh — docs/04 Stage 3)"
    );

    if let Some(burn_in) = &result.burn_in {
        let burn_in_gwh = burn_in.requirement.as_gigawatt_hours();
        eprintln!("RS 37+-year solve: one-year burn-in requirement {burn_in_gwh:.1} GWh");
        assert!(
            (BAND_LO_GWH..=BAND_HI_GWH).contains(&burn_in_gwh),
            "burn-in hydrogen requirement {burn_in_gwh:.1} GWh is outside the \
             published-order band [{BAND_LO_GWH}, {BAND_HI_GWH}] GWh"
        );
    }
    // A 40-year horizon always extends beyond year 1, so the guard can
    // never be silently skipped here.
    assert!(
        result.burn_in_skipped.is_none(),
        "the burn-in re-run must never be skipped on a 40-year horizon: {:?}",
        result.burn_in_skipped
    );

    // The regression pins (doc comment above; deterministic, ADR-5).
    assert!(
        (naive_gwh - 23_872.0).abs() < 1e-9,
        "PINNED requirement moved: measured {naive_gwh} GWh, pinned 23,872 GWh — a \
         deliberate engine/pack/scenario change requires a knowing re-pin"
    );
    assert_eq!(
        result.min_soc_at.to_string(),
        "1989-12-13T02:00:00Z",
        "PINNED min-SoC instant moved (the 1989 winter nadir)"
    );
    assert!(
        (result.min_soc.as_gigawatt_hours() - 8.870000113328198).abs() < 1e-9,
        "PINNED min SoC moved: measured {} GWh",
        result.min_soc.as_gigawatt_hours()
    );
    assert!(
        !result.initial_condition_sensitive && result.burn_in.is_none(),
        "PINNED guard outcome moved: the 1989 nadir is not in the first weather year, \
         so no burn-in re-run is expected"
    );
}

/// Acceptance (b): SoC is continuous across all 39 year boundaries —
/// the change over every boundary period is exactly the period's flows
/// under the √η split (D4 mechanics), never a re-initialisation.
#[test]
fn rs_scenario_soc_is_continuous_across_all_year_boundaries() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(RS_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    // The scenario as written: the fixed 100 TWh hydrogen store.
    let result = run(&scenario, &inputs).unwrap();
    assert_eq!(result.periods(), FULL_RECORD_PERIODS);
    let store = &result.stores[0];
    let sqrt_eta = scenario.zones[0].storage[0]
        .round_trip_efficiency
        .value()
        .sqrt();
    let dt_h = 0.5;

    let mut boundary = 0usize;
    for year in 1985..=2023 {
        boundary += periods_in_year(year);
        // First period of the following year: ΔSoC must equal that
        // period's flows exactly — carried state, no reset.
        let delta =
            store.soc[boundary].as_gigawatt_hours() - store.soc[boundary - 1].as_gigawatt_hours();
        let expected = store.charge[boundary].as_gigawatts() * dt_h * sqrt_eta
            - store.discharge[boundary].as_gigawatts() * dt_h / sqrt_eta;
        assert!(
            (delta - expected).abs() < 1e-6,
            "SoC step into {} (period {boundary}) was {delta} GWh but the period's flows \
             give {expected} GWh — an annual reset, not carried state",
            year + 1
        );
    }

    // And the store genuinely works across the record (the continuity
    // assertion must not pass vacuously on an inert store).
    let charged = grid_adequacy::RunResult::total_energy(&store.charge);
    let discharged = grid_adequacy::RunResult::total_energy(&store.discharge);
    assert!(
        charged.as_gigawatt_hours() > 0.0 && discharged.as_gigawatt_hours() > 0.0,
        "the hydrogen store never cycled (charged {charged:?}, discharged {discharged:?})"
    );
}

/// Acceptance (c) — Module 3(a) vs (b) in one test: the 12 h battery
/// that gives ZERO unserved on the single benign 2024 year
/// (acceptance_stage3_2024.rs) fails CATASTROPHICALLY on the full
/// 1985–2024 record: same fleet, same battery, same 2024 demand profile
/// (tiled by calendar date), 40 winters instead of one.
///
/// "Catastrophically" means: total unserved energy at least an order
/// of magnitude above the benign year's bare-fleet shortfall
/// (19.1 GWh — i.e. > 191 GWh, battery present), spread across at
/// least five distinct calendar years (a structural, recurring
/// failure, not one freak winter). Those coarse assertions are kept as
/// the meaning of the test; the exact measured values are PINNED
/// (first real-record pass, 2026-07-03): 557.4285217177783 GWh
/// unserved over 574 periods in 33 distinct calendar years.
#[test]
fn benign_twelve_hour_battery_fails_catastrophically_on_the_full_record() {
    require_full_record_pack();
    let root = repo_root();

    // The benign scenario, re-pointed at the full record: horizon
    // 1985–2024, tiled demand (same 2024 profile every year), per-year
    // CF traces for the same three weather-driven technologies. Fleet
    // and battery are byte-identical to the benign-year test's.
    let mut scenario = Scenario::load(&root.join(BENIGN_SCENARIO)).unwrap();
    scenario.horizon.start = "1985-01-01T00:00:00Z".to_owned();
    scenario.horizon.end = "2024-12-31T23:30:00Z".to_owned();
    scenario.horizon.weather_years = grid_core::scenario::WeatherYears::All;
    let per_year = |pattern: &dyn Fn(i32) -> String| -> TraceFiles {
        TraceFiles::from_paths((1985..=2024).map(pattern).collect())
    };
    scenario.zones[0].demand.base_profile =
        per_year(&|y| format!("data/packs/demand-tiled/demand_{y}.parquet"));
    for entry in &mut scenario.zones[0].fleet {
        if entry.capacity_factor_trace.is_some() {
            let file_tech = match entry.technology.as_str() {
                "offshore_wind" => "offshore",
                "onshore_wind" => "onshore",
                "solar" => "solar",
                other => panic!("unexpected weather-driven technology {other}"),
            };
            entry.capacity_factor_trace = Some(per_year(&|y| {
                format!("data/packs/cf/gb_{file_tech}_cf_{y}.parquet")
            }));
        }
    }

    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let result = run(&scenario, &inputs).unwrap();
    assert_eq!(result.periods(), FULL_RECORD_PERIODS);

    let unserved_gwh = result.total_unserved().as_gigawatt_hours();
    let unserved_periods = result
        .unserved
        .iter()
        .filter(|p| p.as_gigawatts() > 0.0)
        .count();
    let years_with_unserved: BTreeSet<i64> = result
        .unserved
        .iter()
        .enumerate()
        .filter(|(_, p)| p.as_gigawatts() > 0.0)
        .map(|(t, _)| result.timestamp_at(t).civil_date().0)
        .collect();
    eprintln!(
        "benign fleet + 12 h battery on 1985–2024: {unserved_gwh} GWh unserved over \
         {unserved_periods} periods in {} distinct years: {years_with_unserved:?}",
        years_with_unserved.len()
    );

    assert!(
        unserved_gwh > 191.0,
        "expected catastrophic failure (> 191 GWh, an order of magnitude above the benign \
         year's 19.1 GWh bare shortfall); measured {unserved_gwh:.1} GWh — Module 3(b)"
    );
    assert!(
        years_with_unserved.len() >= 5,
        "expected a structural, recurring failure across ≥ 5 calendar years; got {}: \
         {years_with_unserved:?}",
        years_with_unserved.len()
    );

    // The regression pins (doc comment above; deterministic, ADR-5).
    assert!(
        (unserved_gwh - 557.4285217177783).abs() < 1e-6,
        "PINNED unserved energy moved: measured {unserved_gwh} GWh, pinned \
         557.4285217177783 GWh — a deliberate engine/pack/scenario change requires a \
         knowing re-pin"
    );
    assert_eq!(unserved_periods, 574, "PINNED unserved period count moved");
    assert_eq!(
        years_with_unserved.len(),
        33,
        "PINNED distinct-year count moved: {years_with_unserved:?}"
    );

    // The battery is not the fix: it drains and stays irrelevant at the
    // multi-week timescale (36 GWh ≈ 70 minutes of average GB demand).
    let battery = &result.stores[0];
    let (_, min_soc) = battery.min_soc().unwrap();
    assert_eq!(
        min_soc,
        Energy::gigawatt_hours(0.0),
        "the 12 h battery should run completely dry somewhere in 40 winters"
    );
}

// ---------------------------------------------------------------------
// (d) The overbuild curve (module docs): pinned points at 0.70× and
// 0.85× supply, and the infeasibility cliff at 0.60×.
// ---------------------------------------------------------------------

/// The lean-supply (0.70×) variant: offshore 168 / onshore 56 /
/// solar 140 GW (~1.35× annual demand of wind + solar potential),
/// everything else identical to the headline scenario.
///
/// PINNED (first real-record pass, 2026-07-03): requirement exactly
/// 58,432 GWh (58.4 TWh — inside the reviewer's ~[40, 150] TWh
/// quotable band for the RS comparison, unlike the 1.00× point); min
/// SoC 21.737747689484188 GWh at 2011-04-25T07:30:00Z (not year-1, no
/// burn-in). This ~1.35× supply sizing sits INSIDE the RS report's
/// verified 1.23–1.40× range — the RS-COMPARABLE point on the
/// overbuild curve (scenario header; store-side vs RS-quoted unit
/// table in the stage-3 run report §3).
#[test]
fn lean_rs_scenario_pins_the_overbuild_curve() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(LEAN_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let result =
        min_storage_for_zero_unserved(&scenario, &inputs, 0, &SolveOptions::default()).unwrap();

    let naive_gwh = result.naive.requirement.as_gigawatt_hours();
    eprintln!(
        "RS lean (0.70×) solve: naive requirement {naive_gwh} GWh; min SoC {} GWh at {}; \
         initial-condition-sensitive: {}",
        result.min_soc.as_gigawatt_hours(),
        result.min_soc_at,
        result.initial_condition_sensitive
    );

    // Still tens of TWh (the docs/04 band) …
    assert!(
        (BAND_LO_GWH..=BAND_HI_GWH).contains(&naive_gwh),
        "lean requirement {naive_gwh} GWh is outside [{BAND_LO_GWH}, {BAND_HI_GWH}] GWh"
    );
    // … and pinned exactly (doc comment above; deterministic, ADR-5).
    assert!(
        (naive_gwh - 58_432.0).abs() < 1e-9,
        "PINNED lean requirement moved: measured {naive_gwh} GWh, pinned 58,432 GWh — a \
         deliberate engine/pack/scenario change requires a knowing re-pin"
    );
    assert_eq!(
        result.min_soc_at.to_string(),
        "2011-04-25T07:30:00Z",
        "PINNED lean min-SoC instant moved"
    );
    assert!(
        (result.min_soc.as_gigawatt_hours() - 21.737747689484188).abs() < 1e-9,
        "PINNED lean min SoC moved: measured {} GWh",
        result.min_soc.as_gigawatt_hours()
    );
    assert!(
        !result.initial_condition_sensitive && result.burn_in.is_none(),
        "PINNED lean guard outcome moved (nadir 2011, not year-1)"
    );
}

/// The mid-supply (0.85×) variant: offshore 204 / onshore 68 /
/// solar 170 GW (~1.64× annual demand of wind + solar potential),
/// everything else identical to the headline scenario. Pinned because
/// the stage-3 run report quotes this point (the published-number
/// rule, docs/05).
///
/// PINNED (first real-record pass, 2026-07-03): requirement exactly
/// 28,336 GWh; min SoC 13.38767961262123 GWh at 1997-02-01T01:00:00Z
/// (the February 1997 nadir — not year-1, no burn-in). This ~1.64×
/// supply sizing is ABOVE the RS report's verified 1.23–1.40× range:
/// an intermediate point on the overbuild curve, not an RS-comparable
/// case (scenario header; the RS-comparable point is the lean ~1.35×).
#[test]
fn mid_rs_scenario_pins_the_overbuild_curve() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(MID_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let result =
        min_storage_for_zero_unserved(&scenario, &inputs, 0, &SolveOptions::default()).unwrap();

    let naive_gwh = result.naive.requirement.as_gigawatt_hours();
    eprintln!(
        "RS mid (0.85×) solve: naive requirement {naive_gwh} GWh; min SoC {} GWh at {}; \
         initial-condition-sensitive: {}",
        result.min_soc.as_gigawatt_hours(),
        result.min_soc_at,
        result.initial_condition_sensitive
    );

    // Still tens of TWh (the docs/04 band) …
    assert!(
        (BAND_LO_GWH..=BAND_HI_GWH).contains(&naive_gwh),
        "mid requirement {naive_gwh} GWh is outside [{BAND_LO_GWH}, {BAND_HI_GWH}] GWh"
    );
    // … and pinned exactly (doc comment above; deterministic, ADR-5).
    assert!(
        (naive_gwh - 28_336.0).abs() < 1e-9,
        "PINNED mid requirement moved: measured {naive_gwh} GWh, pinned 28,336 GWh — a \
         deliberate engine/pack/scenario change requires a knowing re-pin"
    );
    assert_eq!(
        result.min_soc_at.to_string(),
        "1997-02-01T01:00:00Z",
        "PINNED mid min-SoC instant moved"
    );
    assert!(
        (result.min_soc.as_gigawatt_hours() - 13.38767961262123).abs() < 1e-9,
        "PINNED mid min SoC moved: measured {} GWh",
        result.min_soc.as_gigawatt_hours()
    );
    assert!(
        !result.initial_condition_sensitive && result.burn_in.is_none(),
        "PINNED mid guard outcome moved (nadir February 1997, not year-1)"
    );
}

/// The cliff (a Stage 3 FINDING, documented as such): at 0.60× supply
/// (offshore 144 / onshore 48 / solar 120 GW) NO storage size achieves
/// zero unserved — annual wind + solar supply no longer covers annual
/// demand once conversion losses are paid, so the solver's doubling
/// search hits its 10⁶ GWh cap and returns the structured
/// `SolveInfeasible` (CLI exit 1). Storage moves energy through time;
/// it cannot create it. Between 0.60× and 0.70× the requirement goes
/// from infinite to 58 TWh — the overbuild curve's left edge.
#[test]
fn overbuild_below_the_cliff_is_infeasible() {
    require_full_record_pack();
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join(RS_SCENARIO)).unwrap();
    // 0.60× of the headline capacities, everything else untouched.
    for entry in &mut scenario.zones[0].fleet {
        entry.capacity_gw = entry.capacity_gw * 0.6;
    }
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let err =
        min_storage_for_zero_unserved(&scenario, &inputs, 0, &SolveOptions::default()).unwrap_err();
    assert!(
        matches!(err, grid_core::GridError::SolveInfeasible { .. }),
        "0.60× supply must be structurally infeasible (the cliff); got: {err}"
    );
}

/// The Stage 3 demo-artefact numbers (docs/04: "multi-week drawdowns
/// and multi-year recharge"), pinned because the stage-3 run report
/// quotes them: the LEAN scenario run at its pinned requirement
/// (58,432 GWh) exhibits genuinely multi-year store dynamics.
///
/// Definitions (normative for these pins): the store is "at full" in a
/// period iff its end-of-period SoC equals the capacity exactly (the
/// engine snaps SoC at the bounds); a "below-full episode" is a
/// maximal consecutive run of not-at-full periods; episode length in
/// days is periods × 0.5 h / 24.
///
/// PINNED (first real-record pass, 2026-07-03):
/// - at full in only 106,324 of 701,280 periods (15.16 %);
/// - longest below-full episode: 34,587 periods = 720.5625 days,
///   2009-12-09T13:30:00Z → 2011-11-30T02:30:00Z, nadir
///   21.737747689484188 GWh at 2011-04-25T07:30:00Z (the same instant
///   the solve pin found — and 2009–2011 is the RS report's own
///   binding window);
/// - runner-up episodes: 21,231 periods = 442.3125 days,
///   1996-12-05T01:00:00Z → 1998-02-20T08:00:00Z; and 19,527 periods
///   = 406.8125 days, 1987-01-07T02:30:00Z → 1988-02-17T21:30:00Z.
///
/// (The HEADLINE 1.00× scenario at ITS requirement shows nothing of
/// the kind — longest below-full episode 57 days, at full 55 % of
/// periods — which is why the stage artefact is the lean run.)
#[test]
fn lean_at_requirement_below_full_episodes_are_pinned() {
    require_full_record_pack();
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join(LEAN_SCENARIO)).unwrap();
    // The lean scenario at its pinned requirement (the demo artefact,
    // runs/rs-37y-lean-at-requirement/).
    let capacity = Energy::gigawatt_hours(58_432.0);
    scenario.zones[0].storage[0].energy_gwh = capacity;
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let result = run(&scenario, &inputs).unwrap();
    assert_eq!(result.periods(), FULL_RECORD_PERIODS);
    assert_eq!(
        result.total_unserved(),
        Energy::gigawatt_hours(0.0),
        "at the pinned requirement the run must have zero unserved"
    );

    let soc = &result.stores[0].soc;
    let at_full = soc.iter().filter(|&&s| s == capacity).count();

    // Maximal consecutive below-full episodes as (start, end) indices.
    let mut episodes: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (index, &s) in soc.iter().enumerate() {
        if s < capacity {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            episodes.push((begin, index - 1));
        }
    }
    if let Some(begin) = start {
        episodes.push((begin, FULL_RECORD_PERIODS - 1));
    }
    episodes.sort_by_key(|(begin, end)| std::cmp::Reverse(end - begin + 1));

    let describe = |(begin, end): (usize, usize)| {
        let periods = end - begin + 1;
        format!(
            "{periods} periods = {} days, {} .. {}",
            periods as f64 * 0.5 / 24.0,
            result.timestamp_at(begin),
            result.timestamp_at(end)
        )
    };
    eprintln!(
        "lean at requirement: at full {at_full}/{FULL_RECORD_PERIODS} periods \
         ({:.2} %); top episodes: [{}], [{}], [{}]",
        100.0 * at_full as f64 / FULL_RECORD_PERIODS as f64,
        describe(episodes[0]),
        describe(episodes[1]),
        describe(episodes[2]),
    );

    // The regression pins (doc comment above; deterministic, ADR-5).
    assert_eq!(at_full, 106_324, "PINNED at-full period count moved");

    let pin_episode = |which: usize, periods: usize, begin: &str, end: &str| {
        let (b, e) = episodes[which];
        assert_eq!(
            e - b + 1,
            periods,
            "PINNED episode {which} length moved: {}",
            describe(episodes[which])
        );
        assert_eq!(
            result.timestamp_at(b).to_string(),
            begin,
            "PINNED episode {which} start moved"
        );
        assert_eq!(
            result.timestamp_at(e).to_string(),
            end,
            "PINNED episode {which} end moved"
        );
    };
    // Longest: 720.5625 days across two winters — the multi-year
    // drawdown/recharge the docs/04 demo artefact must show.
    pin_episode(0, 34_587, "2009-12-09T13:30:00Z", "2011-11-30T02:30:00Z");
    // Runners-up: 442.3125 and 406.8125 days.
    pin_episode(1, 21_231, "1996-12-05T01:00:00Z", "1998-02-20T08:00:00Z");
    pin_episode(2, 19_527, "1987-01-07T02:30:00Z", "1988-02-17T21:30:00Z");

    // The longest episode's nadir is the solve pin's min SoC, at the
    // same instant.
    let (b, e) = episodes[0];
    let nadir = (b..=e)
        .min_by(|&i, &j| {
            soc[i]
                .as_gigawatt_hours()
                .partial_cmp(&soc[j].as_gigawatt_hours())
                .unwrap()
        })
        .unwrap();
    assert_eq!(
        result.timestamp_at(nadir).to_string(),
        "2011-04-25T07:30:00Z",
        "PINNED longest-episode nadir instant moved"
    );
    assert!(
        (soc[nadir].as_gigawatt_hours() - 21.737747689484188).abs() < 1e-9,
        "PINNED longest-episode nadir moved: measured {} GWh",
        soc[nadir].as_gigawatt_hours()
    );
}
