//! Q5/Q11 heating-overlay engine acceptance (D9 rule 5, tests 2 and 4;
//! the grid-core suite carries tests 1 and 3):
//!
//! - **No-heating byte-path** (test 2): the migrated v5 reference
//!   scenario loads with `inputs.heating = None` and its demand trace
//!   is bit-identical to the hand-computed
//!   `base × annual_scale + extra_demand_gw` — the engine byte-path is
//!   untouched by the v5 migration. (The dispatch DIGEST re-verification
//!   lives in grid-cli/tests/regression_2024.rs — 779d7444… — and the
//!   new 5-zone GB-zone digest pin there; the Stage 5 A2 pinned counts
//!   guard the 5-zone external dispatch.)
//! - **Demand addition** (D9 rule 1): with a heating block, per-period
//!   demand equals the no-heating demand plus the overlay's electrical
//!   total, exactly — nothing else changes.
//! - **Characterisation pin** (test 4): the D9 0.70/0.20/0.10 mix on
//!   the Royal-Society 37+-year fleet — the 40-year storage requirement
//!   delta and the peak-residual-demand delta vs the no-heating
//!   baseline, pinned.
//!
//! These tests need the fetched data packs (2024 pack; per-year
//! 1985–2024 pack; the pinned GB t2m trace). They FAIL LOUDLY if any
//! is missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{SolveOptions, load_run_inputs, min_storage_for_zero_unserved, run};
use grid_core::analysis::residual_load;
use grid_core::scenario::{HeatingEntry, HeatingKind, HeatingSpec, Scenario, TraceRefSpec};
use grid_core::units::{Energy, PerUnit, Power};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn require(rel: &str) {
    let path = repo_root().join(rel);
    assert!(
        path.exists(),
        "required data is missing ({}) — build the pack first (scripts/fetch-2024, \
         scripts/era5-cf; the t2m trace: derive_t2m_gb.py)",
        path.display()
    );
}

/// The D9 rule-2 heating block used throughout: the reviewed reference
/// quantum (410.5 TWh record-mean delivered heat, GB), electrified
/// share 0.5 (the D9 rule-2 illustrative level), DHW fraction 0.170
/// (the reviewed basis), the pinned t2m trace, and the D9 three-way
/// mix 0.70 ASHP / 0.20 GSHP / 0.10 district.
fn d9_heating_block() -> HeatingSpec {
    let entry = |kind: HeatingKind, share: f64| HeatingEntry {
        kind,
        share: PerUnit::new(share),
        cop_curve: None,
        correction_factor: None,
        rhpp_derating: None,
        cop_const: None,
        resource_depth_m: None,
    };
    HeatingSpec {
        delivered_heat_twh: Energy::gigawatt_hours(410_500.0),
        electrified_share: PerUnit::new(0.5),
        dhw_fraction: PerUnit::new(0.170),
        temperature_trace: TraceRefSpec {
            path: "data/weather/gb_t2m_pop.parquet".to_owned(),
            column: "t2m_pop".to_owned(),
        },
        entries: vec![
            entry(HeatingKind::Ashp, 0.70),
            entry(HeatingKind::Gshp, 0.20),
            entry(HeatingKind::DistrictGeothermal, 0.10),
        ],
    }
}

/// No-heating byte-path (D9 rule 5 test 2, input side): the migrated
/// v5 reference scenario carries no heating block; the loaded inputs
/// carry no overlay and the demand trace is exactly
/// `base × scale + extra` — bit-identical to the pre-v5 arithmetic.
#[test]
fn migrated_reference_scenario_loads_with_untouched_demand() {
    require("data/packs/2024/processed/demand_2024.parquet");
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    assert_eq!(scenario.schema_version, 8);
    assert!(scenario.zones[0].demand.heating.is_none());

    let inputs = load_run_inputs(&scenario, &root).unwrap();
    assert!(inputs.heating.is_none());

    // Reconstruct the pre-v5 demand arithmetic by hand.
    let base = grid_core::trace::load_power_trace_mw(
        &root.join("data/packs/2024/processed/demand_2024.parquet"),
        "underlying_demand",
        17_568,
    )
    .unwrap();
    let scale = scenario.zones[0].demand.annual_scale;
    let extra = scenario.zones[0].demand.extra_demand_gw;
    for (loaded, &b) in inputs.demand.values().iter().zip(base.values()) {
        assert_eq!(*loaded, b * scale + extra);
    }
}

/// D9 rule 1: heating demand ADDS to zone demand before dispatch —
/// per-period, demand-with-heating equals demand-without plus the
/// overlay's electrical total, exactly; and the overlay's series and
/// constants ride on the loaded inputs for the output layer.
#[test]
fn heating_block_adds_the_overlay_to_demand_exactly() {
    require("data/packs/2024/processed/demand_2024.parquet");
    require("data/weather/gb_t2m_pop.parquet");
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let baseline = load_run_inputs(&scenario, &root).unwrap();

    scenario.zones[0].demand.heating = Some(d9_heating_block());
    let heated = load_run_inputs(&scenario, &root).unwrap();
    let overlay = heated.heating.as_ref().expect("overlay must be loaded");

    assert_eq!(heated.demand.len(), baseline.demand.len());
    for t in 0..heated.demand.len() {
        let expected = baseline.demand.values()[t] + overlay.electrical_total[t];
        assert_eq!(
            heated.demand.values()[t],
            expected,
            "period {t}: demand-with-heating must be demand + overlay exactly"
        );
    }
    // Everything else is untouched (heating changes demand only).
    assert_eq!(heated.capacity_factors, baseline.capacity_factors);
    assert_eq!(heated.exogenous, baseline.exogenous);
    assert_eq!(heated.availability, baseline.availability);
    // The echoed constants are present for the output layer.
    assert!(overlay.constants.k.as_gigawatts_per_kelvin() > 0.0);
    assert_eq!(overlay.entries.len(), 3);
}

/// Characterisation pin (D9 rule 5 test 4): the 0.70/0.20/0.10 mix on
/// the RS 37+-year fleet (scenarios/royal-society-37y.toml, horizon
/// 1985–2024, ~570 TWh/yr non-heat demand tiled from 2024).
///
/// Assumptions stated: quantum 410.5 TWh (record-mean delivered heat,
/// GB), electrified_share 0.5, DHW 0.170 — the electrified quantum is
/// 205.25 TWh/yr on top of ~570 TWh/yr, all reference COP parameters.
/// Standing caveats apply (no behavioural profile ⇒ deltas are lower
/// bounds; 2024 non-heat demand tiling; climate-stationary intensity).
///
/// STORE-POWER CONVENTION (stated, applied to BOTH endpoints): the
/// committed scenario's 100 GW hydrogen rating was chosen so the
/// ENERGY requirement, not the power rating, binds the solve (its own
/// header). Under the heating overlay that no longer holds — the FIRST
/// FINDING pinned below is that at 100 GW the heated solve is
/// POWER-BOUND INFEASIBLE (no energy size achieves zero unserved: the
/// heated peak residual exceeds what fleet + 100 GW of discharge can
/// serve). The requirement DELTA is therefore measured with the rating
/// raised to 200 GW on both endpoints, restoring the scenario's stated
/// energy-binding design; the peak-residual delta is power-independent.
///
/// PINNED (first pass, 2026-07-03, measured):
/// - peak residual demand: baseline 92.23871490574456 GW → heated
///   113.4466987983204 GW (delta +21.208 GW — the heated peak exceeds
///   the committed 100 GW rating, which is why the 100 GW solve is
///   infeasible);
/// - at 200 GW store power: baseline requirement 23,872 GWh (equal to
///   the Stage 3 100 GW pin — the rating never bound the baseline) →
///   heated 40,224 GWh (delta +16,352 GWh, ×1.69);
/// - at the committed 100 GW: heated solve infeasible (the finding).
#[test]
fn d9_mix_on_the_rs_fleet_pins_storage_and_peak_residual_deltas() {
    require("data/weather/gb_t2m_pop.parquet");
    for year in 1985..=2024 {
        require(&format!("data/packs/demand-tiled/demand_{year}.parquet"));
        require(&format!("data/packs/cf/gb_offshore_cf_{year}.parquet"));
    }
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/royal-society-37y.toml")).unwrap();
    let mut heated_scenario = scenario.clone();
    heated_scenario.zones[0].demand.heating = Some(d9_heating_block());

    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let heated_inputs = load_run_inputs(&heated_scenario, &root).unwrap();

    // Peak residual demand (power-independent; fixed 100 TWh store is
    // irrelevant to the residual, which precedes storage).
    let baseline_peak = peak_residual(&run(&scenario, &inputs).unwrap());
    let heated_peak = peak_residual(&run(&heated_scenario, &heated_inputs).unwrap());

    // Finding 1: at the committed 100 GW rating the heated solve is
    // power-bound infeasible.
    let infeasible = min_storage_for_zero_unserved(
        &heated_scenario,
        &heated_inputs,
        0,
        &SolveOptions::default(),
    )
    .unwrap_err();
    assert!(
        matches!(infeasible, grid_core::GridError::SolveInfeasible { .. }),
        "expected the 100 GW heated solve to be power-bound infeasible; got: {infeasible}"
    );

    // Finding 2: the requirement delta at the stated 200 GW convention.
    let at_200gw = |scenario: &Scenario, inputs: &grid_adequacy::RunInputs| -> Energy {
        let mut wide = scenario.clone();
        wide.zones[0].storage[0].power_gw = Power::gigawatts(200.0);
        min_storage_for_zero_unserved(&wide, inputs, 0, &SolveOptions::default())
            .unwrap()
            .naive
            .requirement
    };
    let baseline_requirement = at_200gw(&scenario, &inputs);
    let heated_requirement = at_200gw(&heated_scenario, &heated_inputs);

    eprintln!(
        "RS 37y + D9 0.70/0.20/0.10 mix: peak residual {baseline_peak} -> {heated_peak} GW \
         (delta {:+.3}); storage requirement at 200 GW {} -> {} GWh (delta {:+})",
        heated_peak - baseline_peak,
        baseline_requirement.as_gigawatt_hours(),
        heated_requirement.as_gigawatt_hours(),
        heated_requirement.as_gigawatt_hours() - baseline_requirement.as_gigawatt_hours(),
    );

    // The pins (deterministic — ADR-5; a deliberate engine/pack/
    // reference change is a knowing re-pin with the record).
    assert!(
        (baseline_peak - 92.238_714_905_744_56).abs() < 1e-9,
        "PINNED baseline peak residual moved: {baseline_peak} GW"
    );
    assert!(
        (heated_peak - 113.446_698_798_320_4).abs() < 1e-9,
        "PINNED heated peak residual moved: {heated_peak} GW"
    );
    assert!(
        (baseline_requirement.as_gigawatt_hours() - 23_872.0).abs() < 1e-9,
        "PINNED 200 GW baseline requirement moved: {} GWh (the 100 GW Stage 3 pin is \
         23,872 — the rating never bound the baseline solve)",
        baseline_requirement.as_gigawatt_hours()
    );
    assert!(
        (heated_requirement.as_gigawatt_hours() - 40_224.0).abs() < 1e-9,
        "PINNED heated storage requirement moved: {} GWh",
        heated_requirement.as_gigawatt_hours()
    );
    // Direction sanity (not the pin): electrified heat only adds demand.
    assert!(heated_requirement > baseline_requirement);
    assert!(heated_peak > baseline_peak);
}

/// Peak residual demand of a run, GW: max over periods of
/// `demand − Σ must-take` (the grid-core analysis definition; this
/// fleet has no exogenous supply, so must-take = the renewables).
fn peak_residual(result: &grid_adequacy::RunResult) -> f64 {
    let must_take: Vec<&[Power]> = result
        .renewables
        .iter()
        .map(|s| s.power.as_slice())
        .chain(result.exogenous.iter().map(|s| s.power.as_slice()))
        .collect();
    residual_load(&result.demand, &must_take)
        .unwrap()
        .iter()
        .map(|p| p.as_gigawatts())
        .fold(f64::NEG_INFINITY, f64::max)
}
