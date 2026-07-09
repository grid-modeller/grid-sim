//! Stage 3 bisection solver (`min_storage_for_zero_unserved`, ADR-10):
//! hand-checkable requirements on synthetic systems, the full-trace
//! reporting contract, infeasibility, and the D4 initial-SoC guard
//! (year-1 flag + burn-in re-run).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{RunInputs, SolveOptions, min_storage_for_zero_unserved};
use grid_core::GridError;
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, Scenario, StorageKind,
    StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, PerUnit, Power};

fn scenario_at(
    start: &str,
    fleet: Vec<FleetEntry>,
    storage: Vec<StorageSpec>,
    periods: usize,
) -> Scenario {
    let start_instant = UtcInstant::parse(start).unwrap();
    let end = start_instant.plus_periods(periods as i64 - 1);
    Scenario {
        schema_version: 5,
        name: "solver-synthetic".to_owned(),
        description: None,
        horizon: Horizon {
            start: start.to_owned(),
            end: end.to_string(),
            weather_years: WeatherYears::Years(vec![2024]),
        },
        zones: vec![ZoneSpec {
            pricing: None,
            id: ZoneId::new("GB"),
            demand: DemandSpec {
                base_profile: "unused-in-synthetic-runs".into(),
                column: "underlying_demand".to_owned(),
                extra_profiles: vec![],
                annual_scale: 1.0,
                extra_demand_gw: Power::gigawatts(0.0),
                heating: None,
            },
            exogenous_supply: vec![],
            fleet,
            storage,
        }],
        links: vec![],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

fn renewable(tech: &str, capacity_gw: f64) -> FleetEntry {
    FleetEntry {
        technology: TechId::new(tech),
        capacity_gw: Power::gigawatts(capacity_gw),
        capacity_factor_trace: Some(format!("synthetic/{tech}.parquet").into()),
        availability: None,
        reliability: None,
        inertia_h: None,
        synchronous: None,
        energy_budget: None,
    }
}

fn store_starting_empty(power: f64, energy: f64, rte: f64) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Battery,
        power_gw: Power::gigawatts(power),
        energy_gwh: Energy::gigawatt_hours(energy),
        round_trip_efficiency: PerUnit::new(rte),
        dispatch_order: 1,
        initial_soc: Some(PerUnit::new(0.0)),
        shift_duration: None,
        daily_volume_limit: None,
    }
}

fn inputs_at(start: &str, demand_gw: &[f64], cf: &[(&str, &[f64])]) -> RunInputs {
    let start = UtcInstant::parse(start).unwrap();
    RunInputs {
        demand: Trace::from_parts(
            start,
            demand_gw.iter().map(|&v| Power::gigawatts(v)).collect(),
        )
        .unwrap(),
        capacity_factors: cf
            .iter()
            .map(|(tech, values)| {
                (
                    TechId::new(*tech),
                    Trace::from_parts(start, values.iter().map(|&v| PerUnit::new(v)).collect())
                        .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
        exogenous: vec![],
        availability: BTreeMap::new(),
        heating: None,
    }
}

const START: &str = "2024-01-01T00:00:00Z";

/// Surplus then deficit, no thermal stack, η = 1, store starts empty:
/// the store must carry exactly the deficit energy, so the requirement
/// converges onto it from above within tolerance.
#[test]
fn bisection_finds_the_hand_checkable_requirement() {
    // 4 periods: wind 14, 14, 0, 0 GW vs demand 10 GW flat.
    // Surplus phase banks up to 4 GWh (2 × 4 GW × 0.5 h);
    // deficit phase needs 10 GWh... that exceeds the bank, so scale:
    // deficit needs 2 × 10 GW × 0.5 h = 10 GWh > 4 GWh available.
    // Use demand 10, wind 14 for 6 periods then 0 for 2:
    // bank 6 × 4 × 0.5 = 12 GWh ≥ deficit 2 × 10 × 0.5 = 10 GWh.
    let demand = [10.0; 8];
    let wind_cf = [0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.0, 0.0];
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 20.0)],
        vec![store_starting_empty(20.0, 1.0, 1.0)],
        8,
    );
    let inputs = inputs_at(START, &demand, &[("onshore_wind", &wind_cf)]);
    let result = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap();

    // Analytic requirement: 10 GWh (η = 1, charge power ample, deficit
    // 10 GW × 1 h). The reported value is the smallest known-feasible
    // capacity, so it sits within tolerance ABOVE 10 GWh.
    let requirement = result.naive.requirement.as_gigawatt_hours();
    assert!(
        (10.0..=10.11).contains(&requirement),
        "requirement {requirement} GWh (expected 10 GWh + tolerance)"
    );
    // Full trace reported: every iterate carries its unserved energy and
    // verdict, and the requirement is a feasible iterate.
    assert!(result.naive.iterations.len() >= 5);
    assert!(
        result
            .naive
            .iterations
            .iter()
            .any(|it| it.feasible && it.energy == result.naive.requirement)
    );
    let first = &result.naive.iterations[0];
    assert_eq!(first.energy, Energy::gigawatt_hours(0.0));
    assert!(!first.feasible);
    assert!((first.unserved.as_gigawatt_hours() - 10.0).abs() < 1e-9);
    // Store starts EMPTY here, so the year-1 guard flag must be raised
    // (min SoC is at the very first period) — and with a single-year
    // horizon the burn-in re-run is skipped with a reason.
    assert!(result.initial_condition_sensitive);
    assert!(result.burn_in.is_none());
    let reason = result.burn_in_skipped.unwrap();
    assert!(reason.contains("first weather year"), "reason: {reason}");
}

/// A fleet that never runs short needs zero storage: the solver reports
/// 0 GWh after a single evaluation.
#[test]
fn zero_requirement_when_the_fleet_never_runs_short() {
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 20.0)],
        vec![store_starting_empty(5.0, 100.0, 1.0)],
        4,
    );
    let inputs = inputs_at(START, &[10.0; 4], &[("onshore_wind", &[0.6; 4])]);
    let result = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap();
    assert_eq!(result.naive.requirement, Energy::gigawatt_hours(0.0));
    assert_eq!(result.naive.iterations.len(), 1);
}

/// No surplus ever arrives, the store starts empty: no capacity can
/// help — a structured infeasibility (CLI exit 1), not a hang.
#[test]
fn infeasible_solves_are_a_structured_error() {
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 20.0)],
        vec![store_starting_empty(20.0, 1.0, 1.0)],
        4,
    );
    let inputs = inputs_at(START, &[10.0; 4], &[("onshore_wind", &[0.0; 4])]);
    let err = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap_err();
    assert!(
        matches!(err, GridError::SolveInfeasible { .. }),
        "unexpected error: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("battery"), "message: {msg}");
}

/// The store's power rating also caps discharge: even with unlimited
/// energy a 1 GW store cannot cover a 2 GW deficit → infeasible.
#[test]
fn power_capped_deficits_are_infeasible_regardless_of_energy() {
    let demand = [10.0, 10.0];
    let wind_cf = [1.0, 0.0]; // 20 GW surplus then 10 GW deficit
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 20.0)],
        vec![store_starting_empty(1.0, 1.0, 1.0)],
        2,
    );
    let inputs = inputs_at(START, &demand, &[("onshore_wind", &wind_cf)]);
    let err = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap_err();
    assert!(matches!(err, GridError::SolveInfeasible { .. }));
}

/// The √η split enters the requirement: at η = 0.25 (√η = 0.5) the
/// store must hold deficit/0.5 = twice the delivered energy.
#[test]
fn efficiency_enters_the_requirement_via_the_sqrt_split() {
    let demand = [10.0; 8];
    let wind_cf = [0.7, 0.7, 0.7, 0.7, 0.7, 0.7, 0.0, 0.0];
    // Delivered deficit 10 GWh needs 10/0.5 = 20 GWh of SoC. At 20 GW
    // wind the surplus phase offers 4 GW × 6 × 0.5 h = 12 GWh
    // grid-side → SoC gain 12 × 0.5 = 6 GWh < 20 GWh: infeasible at
    // ANY capacity — the √η charge loss shows up as an energy-sourcing
    // limit, not just a size requirement.
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 20.0)],
        vec![store_starting_empty(20.0, 1.0, 0.25)],
        8,
    );
    let inputs = inputs_at(START, &demand, &[("onshore_wind", &wind_cf)]);
    let err = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap_err();
    assert!(matches!(err, GridError::SolveInfeasible { .. }));
    // With 100 GW wind the surplus is power-capped at the store's
    // 20 GW: intake 20 GW × 6 × 0.5 h × 0.5 = 30 GWh of SoC ≥ 20 GWh,
    // so the requirement is exactly the discharge-side 20 GWh.
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 100.0)],
        vec![store_starting_empty(20.0, 1.0, 0.25)],
        8,
    );
    let inputs = inputs_at(START, &demand, &[("onshore_wind", &wind_cf)]);
    let result = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap();
    let requirement = result.naive.requirement.as_gigawatt_hours();
    assert!(
        (20.0..=20.21).contains(&requirement),
        "requirement {requirement} GWh (expected 20 GWh + tolerance)"
    );
}

/// The D4 initial-SoC guard over a two-year horizon: a default-full
/// store whose minimum SoC lands in year 1 flags the result and re-runs
/// with a one-year burn-in; both figures are reported and the burn-in
/// figure is the honest (larger-or-equal... here smaller) one measured
/// without year 1's transient.
#[test]
fn year_one_min_soc_flags_and_reruns_with_burn_in() {
    // Horizon spans 2024-12-31 (year 1: one deficit period, initial-SoC
    // subsidised) and 2025-01-01 onward (year 2: surplus, then a small
    // deficit).
    let start = "2024-12-31T23:00:00Z";
    let periods = 6;
    // demand 10; wind: 0, 0 (year-1 deficits), then 14, 14 (surplus),
    // then 0, 10 (deficit, balance).
    let demand = [10.0; 6];
    let wind_cf = [0.0, 0.0, 0.7, 0.7, 0.0, 0.5];
    let store = StorageSpec {
        initial_soc: None, // default FULL (D4)
        ..store_starting_empty(20.0, 1.0, 1.0)
    };
    let s = scenario_at(
        start,
        vec![renewable("onshore_wind", 20.0)],
        vec![store],
        periods,
    );
    let inputs = inputs_at(start, &demand, &[("onshore_wind", &wind_cf)]);
    let result = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap();

    // Naive: year 1 has 10 GWh of deficit (2 periods × 10 GW × 0.5 h)
    // wait — year 1 here is only the two 2024 periods: 23:00 and 23:30,
    // both deficit → 10 GWh drawn from the initial-full store; year 2
    // deficits: period 4 (10 GW × 0.5 h = 5 GWh) + period 5 balanced by
    // 4 GW... demand 10, wind 10 → net 0. So the naive requirement must
    // cover max cumulative draw: 10 GWh (year 1) discharged, then 4 GWh
    // recharged (surplus 4 GW × 2 × 0.5), then 5 GWh (year 2 deficit):
    // needs capacity ≥ 11 GWh (draw 10, bank 4 → need soc0 = full ≥
    // 10 + (5 − 4) = 11). Min SoC occurs... during year 2 actually.
    // The assertion below is therefore on the flag semantics, not a
    // particular number: whichever year the min lands in, the reported
    // figures must be self-consistent.
    let naive = result.naive.requirement.as_gigawatt_hours();
    assert!(naive > 0.0);
    if result.initial_condition_sensitive {
        // Flag raised → multi-year horizon → burn-in re-run present.
        let burn_in = result.burn_in.as_ref().expect("burn-in re-run");
        assert!(burn_in.requirement.as_gigawatt_hours() >= 0.0);
        assert!(!burn_in.iterations.is_empty());
    } else {
        assert!(result.burn_in.is_none());
        assert!(result.burn_in_skipped.is_none());
    }

    // Force the year-1 case deterministically: make year 2 all surplus,
    // so the min SoC can only be in year 1.
    let wind_cf = [0.0, 0.0, 0.7, 0.7, 0.7, 0.7];
    let inputs = inputs_at(start, &demand, &[("onshore_wind", &wind_cf)]);
    let result = min_storage_for_zero_unserved(&s, &inputs, 0, &SolveOptions::default()).unwrap();
    assert!(result.initial_condition_sensitive, "min SoC is in year 1");
    let (min_year, _, _) = result.min_soc_at.civil_date();
    assert_eq!(min_year, 2024);
    let burn_in = result
        .burn_in
        .expect("burn-in re-run for a 2024–25 horizon");
    // With year 1 excluded from the feasibility test, zero capacity is
    // enough: year 2 never runs short.
    assert_eq!(burn_in.requirement, Energy::gigawatt_hours(0.0));
    // And both figures are available for reporting (naive > burn-in).
    assert!(result.naive.requirement > burn_in.requirement);
}

/// An out-of-range designated store index is a structured error.
#[test]
fn designating_a_missing_store_is_an_error() {
    let s = scenario_at(
        START,
        vec![renewable("onshore_wind", 20.0)],
        vec![store_starting_empty(5.0, 1.0, 1.0)],
        2,
    );
    let inputs = inputs_at(START, &[10.0; 2], &[("onshore_wind", &[0.6; 2])]);
    let err = min_storage_for_zero_unserved(&s, &inputs, 7, &SolveOptions::default()).unwrap_err();
    assert!(matches!(err, GridError::InvalidScenario { .. }));
    assert!(err.to_string().contains('7'), "err: {err}");
}
