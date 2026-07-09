//! Stage 6 acceptance: Σ(H × MVA) consistency (docs/04 Stage 6, third
//! acceptance test) plus the Stage 6 Module 6 first-cut pins.
//!
//! The engine's per-period system inertia over the 2024 reference run
//! must equal the hand sum over dispatched synchronous plant, under the
//! documented convention (see `grid_stability::inertia`):
//!
//! - synchronous technologies: nuclear, biomass, hydro, coal, CCGT,
//!   OCGT; non-synchronous: wind, solar, battery, interconnector
//!   imports (inverter/HVDC-coupled);
//! - pumped storage is synchronous while RUNNING (either direction —
//!   pumping hours count);
//! - synchronised MVA of an aggregated technology at period t =
//!   dispatched GW at t ÷ the documented power factor (0.9).
//!
//! These tests need the locally built 2024 data pack (git-ignored;
//! fetched, not committed) and fail loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::OnceLock;

use grid_adequacy::{RunResult, load_run_inputs, run};
use grid_core::inertia::DEFAULT_POWER_FACTOR;
use grid_core::scenario::Scenario;
use grid_stability::{InertiaTable, inertia_series, system_inertia};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first (scripts/fetch-2024, \
         scripts/era5-cf)",
        probe.display()
    );
}

/// The 2024 reference run and its scenario, computed once per process.
fn reference_run() -> &'static (Scenario, RunResult) {
    static RUN: OnceLock<(Scenario, RunResult)> = OnceLock::new();
    RUN.get_or_init(|| {
        require_pack();
        let root = repo_root();
        let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
        let inputs = load_run_inputs(&scenario, &root).unwrap();
        let result = run(&scenario, &inputs).unwrap();
        (scenario, result)
    })
}

/// The acceptance property: engine inertia at EVERY period equals the
/// hand sum over dispatched synchronous plant.
#[test]
fn engine_inertia_equals_hand_sum_over_dispatched_synchronous_plant() {
    let (scenario, result) = reference_run();
    let table = InertiaTable::from_scenario(scenario).unwrap();
    let series = inertia_series(result, &table).unwrap();
    assert_eq!(series.len(), result.periods());

    // Hand sum, from the scenario's own effective metadata: H × MVA
    // over the synchronous technologies with nonzero dispatch, MVA =
    // GW / 0.9. The 2024 reference stores are provably inert (D4), so
    // storage contributes nothing here; PS-while-running is exercised
    // in `pumped_storage_counts_while_running` below.
    let pf = DEFAULT_POWER_FACTOR.value();
    let zone = &scenario.zones[0];
    for (t, &engine) in series.iter().enumerate() {
        let mut hand = 0.0;
        for tech_series in result.renewables.iter().chain(&result.thermal) {
            let entry = zone
                .fleet
                .iter()
                .find(|e| e.technology == tech_series.tech)
                .unwrap();
            if !entry.effective_synchronous() {
                continue;
            }
            let gw = tech_series.power[t].as_gigawatts();
            if gw > 0.0 {
                let h = entry.effective_inertia_h().unwrap().as_seconds();
                hand += h * gw / pf;
            }
        }
        for store in &result.stores {
            let active =
                store.charge[t].as_gigawatts().abs() + store.discharge[t].as_gigawatts().abs();
            assert_eq!(active, 0.0, "2024 reference stores must be inert (D4)");
        }
        let engine = engine.as_gigavolt_ampere_seconds();
        assert!(
            (engine - hand).abs() <= 1e-9 * hand.max(1.0),
            "period {t}: engine {engine} GVA·s != hand sum {hand} GVA·s"
        );
        // Same number through the single-period entry point.
        let single = system_inertia(result, &table, t).unwrap();
        assert_eq!(single.as_gigavolt_ampere_seconds(), engine);
    }
}

/// The synchronous/non-synchronous roster of docs/04 Stage 6, checked
/// against the scenario's derived defaults.
#[test]
fn derived_synchronous_roster_matches_docs04() {
    let (scenario, _) = reference_run();
    let zone = &scenario.zones[0];
    let sync = ["nuclear", "biomass", "hydro", "coal", "ccgt", "ocgt"];
    let non_sync = ["offshore_wind", "onshore_wind", "solar"];
    for entry in &zone.fleet {
        let expected = sync.contains(&entry.technology.as_str());
        assert_eq!(
            entry.effective_synchronous(),
            expected,
            "technology {}: derived synchronous flag",
            entry.technology
        );
        assert!(!non_sync.contains(&entry.technology.as_str()) || !entry.effective_synchronous());
    }
}

/// Pumped storage contributes inertia while running — in BOTH modes
/// (pumping is demand, but the machine is synchronised) — and batteries
/// never do. Exercised on a synthetic run because the 2024 reference
/// stores are inert.
#[test]
fn pumped_storage_counts_while_running_battery_never() {
    use grid_adequacy::StoreSeries;
    use grid_core::scenario::StorageKind;
    use grid_core::time::UtcInstant;
    use grid_core::units::{Energy, Power};

    let (scenario, _) = reference_run();
    let table = InertiaTable::from_scenario(scenario).unwrap();
    let zero = Power::gigawatts(0.0);
    // A minimal synthetic result: no fleet dispatch, one PS store and
    // one battery store, three periods: idle / discharging / pumping.
    let synthetic = RunResult {
        start: UtcInstant::parse("2024-01-01T00:00:00Z").unwrap(),
        demand: vec![Power::gigawatts(30.0); 3],
        renewables: vec![],
        exogenous: vec![],
        thermal: vec![],
        stores: vec![
            StoreSeries {
                label: "pumped_hydro".to_owned(),
                kind: StorageKind::PumpedHydro,
                charge: vec![zero, zero, Power::gigawatts(1.8)],
                discharge: vec![zero, Power::gigawatts(2.7), zero],
                soc: vec![Energy::gigawatt_hours(10.0); 3],
            },
            StoreSeries {
                label: "battery".to_owned(),
                kind: StorageKind::Battery,
                charge: vec![zero, zero, Power::gigawatts(1.0)],
                discharge: vec![zero, Power::gigawatts(1.0), zero],
                soc: vec![Energy::gigawatt_hours(3.0); 3],
            },
        ],
        curtailment: vec![zero; 3],
        unserved: vec![zero; 3],
    };
    let series = inertia_series(&synthetic, &table).unwrap();
    let gva_s: Vec<f64> = series
        .iter()
        .map(|i| i.as_gigavolt_ampere_seconds())
        .collect();
    // Idle: nothing synchronised.
    assert_eq!(gva_s[0], 0.0);
    // Discharging 2.7 GW at H = 4.5 s, pf 0.9: 4.5 × 2.7/0.9 = 13.5.
    // The battery's 1 GW discharge adds nothing.
    assert!((gva_s[1] - 13.5).abs() < 1e-12);
    // Pumping 1.8 GW: 4.5 × 1.8/0.9 = 9.0 — pumping hours count.
    assert!((gva_s[2] - 9.0).abs() < 1e-12);
}

// ---------------------------------------------------------------------
// Module 6 first cut (docs/04 Stage 6 demo artefact): pinned findings
// for the 2024 reference run, first measured 2026-07-03 (this run is
// the record; every published number gets a pinned regression test,
// CLAUDE.md).
//
// READ BEFORE QUOTING: these are MODEL-dispatch numbers, UNCONSTRAINED
// by any inertia product — the adequacy engine has no must-run floor,
// no minimum-stable generation and no NESO stability actions, so it
// dispatches the entire synchronous stack to zero in the windiest
// periods (2 periods of literally zero inertia in 2024). Real GB
// inertia stayed within ~110–350 GVA·s (KRA22) because NESO *pays* to
// hold the floor. The Module 6 finding is exactly this gap: what the
// energy market alone would provide vs what stability requires. Do not
// quote the counts as "GB was below its inertia floor 85 % of 2024".
// ---------------------------------------------------------------------

/// Pinned minimum-inertia period of the 2024 reference run: ZERO — a
/// windy spring midday where the model dispatches no synchronous plant
/// at all (renewables + imports + "other" cover demand).
const PINNED_MIN_INERTIA_GVA_S: f64 = 0.0;
const PINNED_MIN_INERTIA_AT: &str = "2024-04-06T11:30:00Z";
const PINNED_ZERO_INERTIA_PERIODS: usize = 2;

/// Pinned counts of half-hour periods below the FRCR floors
/// (120 GVA·s = NESO minimum requirement from 2024-06-19; 102 GVA·s =
/// the 2025 ambition — `data/reference/inertia-constants.toml`):
/// 15,020 periods = 7,510 h/yr and 13,335 periods = 6,667.5 h/yr of
/// the 17,568-period year.
const PINNED_PERIODS_BELOW_120: usize = 15_020;
const PINNED_PERIODS_BELOW_102: usize = 13_335;

#[test]
fn pinned_2024_min_inertia_hour_and_floor_counts() {
    use grid_stability::{min_inertia, periods_below};

    let (scenario, result) = reference_run();
    let table = InertiaTable::from_scenario(scenario).unwrap();
    let series = inertia_series(result, &table).unwrap();

    let (min_index, min) = min_inertia(&series).unwrap();
    assert!(
        (min.as_gigavolt_ampere_seconds() - PINNED_MIN_INERTIA_GVA_S).abs() < 1e-9,
        "min inertia moved: {} GVA·s vs pinned {PINNED_MIN_INERTIA_GVA_S}",
        min.as_gigavolt_ampere_seconds()
    );
    assert_eq!(
        result.timestamp_at(min_index).to_string(),
        PINNED_MIN_INERTIA_AT
    );
    let zero_periods = series
        .iter()
        .filter(|i| i.as_gigavolt_ampere_seconds() == 0.0)
        .count();
    assert_eq!(zero_periods, PINNED_ZERO_INERTIA_PERIODS);

    let below = |floor: f64| {
        periods_below(
            &series,
            grid_core::units::Inertia::gigavolt_ampere_seconds(floor),
        )
    };
    assert_eq!(below(120.0), PINNED_PERIODS_BELOW_120);
    assert_eq!(below(102.0), PINNED_PERIODS_BELOW_102);
}

/// The Royal-Society-style finding, pinned at the mechanism level: an
/// all-variable fleet (wind + solar + battery + hydrogen storage) has
/// ZERO system inertia at every hour — there is no synchronous plant to
/// dispatch, so without synthetic provision the stability question is
/// not "how much margin" but "undefined". Needs no data pack.
#[test]
fn all_variable_fleet_has_zero_synchronous_inertia() {
    use grid_adequacy::{StoreSeries, TechSeries};
    use grid_core::scenario::{Reliability, StorageKind, TechId};
    use grid_core::time::UtcInstant;
    use grid_core::units::{Energy, Power};

    let scenario = Scenario::from_toml_str(
        r#"
schema_version = 8
name = "rs-style-zero-sync"

[horizon]
start = "2024-01-01T00:00:00Z"
end = "2024-01-01T01:30:00Z"
weather_years = [2024]

[[zones]]
id = "GB"
[zones.demand]
base_profile = "unused.parquet"
annual_scale = 1.0

[[zones.fleet]]
technology = "offshore_wind"
capacity_gw = 240.0
capacity_factor_trace = "unused.parquet"

[[zones.fleet]]
technology = "solar"
capacity_gw = 80.0
capacity_factor_trace = "unused.parquet"

[[zones.storage]]
kind = "battery"
power_gw = 10.0
energy_gwh = 40.0
round_trip_efficiency = 0.88
dispatch_order = 1

[[zones.storage]]
kind = "hydrogen"
power_gw = 20.0
energy_gwh = 60000.0
round_trip_efficiency = 0.38
dispatch_order = 2

[dispatch]
policy = "rule_based"
"#,
    )
    .unwrap();
    let table = InertiaTable::from_scenario(&scenario).unwrap();

    let gw = |v: f64| Power::gigawatts(v);
    let synthetic = RunResult {
        start: UtcInstant::parse("2024-01-01T00:00:00Z").unwrap(),
        demand: vec![gw(40.0); 4],
        renewables: vec![
            TechSeries {
                tech: TechId::new("offshore_wind"),
                reliability: Reliability::Variable,
                reliability_overridden: false,
                power: vec![gw(90.0); 4],
            },
            TechSeries {
                tech: TechId::new("solar"),
                reliability: Reliability::Variable,
                reliability_overridden: false,
                power: vec![gw(20.0); 4],
            },
        ],
        exogenous: vec![],
        thermal: vec![],
        stores: vec![
            StoreSeries {
                label: "battery".to_owned(),
                kind: StorageKind::Battery,
                charge: vec![gw(10.0); 4],
                discharge: vec![gw(0.0); 4],
                soc: vec![Energy::gigawatt_hours(20.0); 4],
            },
            StoreSeries {
                label: "hydrogen".to_owned(),
                kind: StorageKind::Hydrogen,
                charge: vec![gw(20.0); 4],
                discharge: vec![gw(0.0); 4],
                soc: vec![Energy::gigawatt_hours(3000.0); 4],
            },
        ],
        curtailment: vec![gw(60.0); 4],
        unserved: vec![gw(0.0); 4],
    };
    let series = inertia_series(&synthetic, &table).unwrap();
    assert!(
        series.iter().all(|i| i.as_gigavolt_ampere_seconds() == 0.0),
        "an all-variable fleet must have exactly zero synchronous inertia"
    );
}
