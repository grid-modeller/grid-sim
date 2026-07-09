//! Robustness-guard tests for the D12 perfect-foresight LP (package 2b
//! PHASE 2; `docs/notes/d12-lp-tractability.md` — the two robustness
//! findings). Both guards are ADDITIVE validation at the entry of
//! `run_multi_lp`; valid small 2a scenarios are unaffected (the 2a
//! `lp_dispatch` tests still pass unchanged).
//!
//! 1. **RTE floor guard** — a store whose round-trip efficiency sits
//!    strictly below the safe floor is rejected with a structured error
//!    BEFORE the LP is built (η ≥ 1e-3 — the floor value included — is
//!    the accepted region; the cycling-penalty soundness argument needs
//!    η well above 1e-6).
//! 2. **LP size guard** — an oversized LP is rejected with a structured
//!    error BEFORE HiGHS is handed a problem large enough to risk the
//!    uncaught C++ `std::length_error` process abort measured in the
//!    tractability benchmark. A binding-window-sized problem (a few
//!    years) is permitted.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{
    LP_RTE_FLOOR, LP_VARIABLE_CAP, MultiZoneInputs, RunInputs, ZoneInputs, estimate_lp_variables,
    run_multi_lp,
};
use grid_core::GridError;
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, LinkSpec, Scenario, StorageKind,
    StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, PerUnit, Power};

const START: &str = "2024-01-01T00:00:00Z";

fn start() -> UtcInstant {
    UtcInstant::parse(START).unwrap()
}

fn horizon(periods: usize) -> Horizon {
    Horizon {
        start: START.to_owned(),
        end: start().plus_periods(periods as i64 - 1).to_string(),
        weather_years: WeatherYears::Years(vec![2024]),
    }
}

fn demand_spec() -> DemandSpec {
    DemandSpec {
        base_profile: "unused-in-synthetic-runs".into(),
        column: "underlying_demand".to_owned(),
        extra_profiles: vec![],
        annual_scale: 1.0,
        extra_demand_gw: Power::gigawatts(0.0),
        heating: None,
    }
}

fn thermal(tech: &str, capacity_gw: f64) -> FleetEntry {
    FleetEntry {
        technology: TechId::new(tech),
        capacity_gw: Power::gigawatts(capacity_gw),
        capacity_factor_trace: None,
        availability: None,
        reliability: None,
        inertia_h: None,
        synchronous: None,
        energy_budget: None,
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

fn battery(power_gw: f64, energy_gwh: f64, rte: f64, order: u8) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Battery,
        power_gw: Power::gigawatts(power_gw),
        energy_gwh: Energy::gigawatt_hours(energy_gwh),
        round_trip_efficiency: PerUnit::new(rte),
        dispatch_order: order,
        initial_soc: Some(PerUnit::new(0.0)),
        shift_duration: None,
        daily_volume_limit: None,
    }
}

fn zone(id: &str, fleet: Vec<FleetEntry>, storage: Vec<StorageSpec>) -> ZoneSpec {
    ZoneSpec {
        pricing: None,
        id: ZoneId::new(id),
        demand: demand_spec(),
        exogenous_supply: vec![],
        fleet,
        storage,
    }
}

fn link(name: &str, from: &str, to: &str, cap: f64) -> LinkSpec {
    LinkSpec {
        name: Some(name.to_owned()),
        from: ZoneId::new(from),
        to: ZoneId::new(to),
        capacity_gw: Power::gigawatts(cap),
        reverse_capacity_gw: None,
        capability_trace: None,
        availability: PerUnit::new(1.0),
        loss: PerUnit::new(0.0),
    }
}

fn scenario(zones: Vec<ZoneSpec>, links: Vec<LinkSpec>, periods: usize) -> Scenario {
    Scenario {
        schema_version: 6,
        name: "synthetic-lp-guard".to_owned(),
        description: None,
        horizon: horizon(periods),
        zones,
        links,
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

fn constant_power(value: f64, periods: usize) -> Trace<Power> {
    Trace::from_parts(start(), vec![Power::gigawatts(value); periods]).unwrap()
}

fn zone_inputs(id: &str, demand: Trace<Power>) -> ZoneInputs {
    ZoneInputs {
        pricing: None,
        id: ZoneId::new(id),
        inputs: RunInputs {
            demand,
            capacity_factors: BTreeMap::new(),
            exogenous: vec![],
            availability: BTreeMap::new(),
            heating: None,
        },
        budgets: BTreeMap::new(),
    }
}

// ---------------------------------------------------------------------
// Guard 1 — the RTE floor.
// ---------------------------------------------------------------------

#[test]
fn below_floor_round_trip_efficiency_is_rejected_structurally() {
    // A store at η = 1e-4 (below the 1e-3 floor). Scenario validation
    // accepts it (rte ∈ (0, 1]); the LP guard must reject it.
    let s = scenario(
        vec![zone(
            "Z",
            vec![thermal("ccgt", 5.0)],
            vec![battery(1.0, 1.0, 1e-4, 1)],
        )],
        vec![],
        2,
    );
    let inputs = MultiZoneInputs {
        zones: vec![zone_inputs("Z", constant_power(1.0, 2))],
        link_capabilities: vec![],
    };
    let err = run_multi_lp(&s, &inputs).unwrap_err();
    match err {
        GridError::StorageEfficiencyBelowFloor {
            efficiency, floor, ..
        } => {
            assert!((efficiency - 1e-4).abs() < 1e-12, "efficiency {efficiency}");
            assert!((floor - LP_RTE_FLOOR).abs() < 1e-12, "floor {floor}");
        }
        other => panic!("expected StorageEfficiencyBelowFloor, got {other:?}"),
    }
}

#[test]
fn at_or_above_the_floor_is_accepted() {
    // η exactly at the floor is accepted (the floor is η ≥ 1e-3).
    let s = scenario(
        vec![zone(
            "Z",
            vec![thermal("ccgt", 5.0)],
            vec![battery(1.0, 1.0, LP_RTE_FLOOR, 1)],
        )],
        vec![],
        2,
    );
    let inputs = MultiZoneInputs {
        zones: vec![zone_inputs("Z", constant_power(1.0, 2))],
        link_capabilities: vec![],
    };
    // Thermal (5 GW) covers the 1 GW demand, so this solves to zero
    // unserved — the point is only that the guard does not fire.
    assert!(run_multi_lp(&s, &inputs).is_ok());
}

// ---------------------------------------------------------------------
// Guard 2 — the LP size cap.
//
// The tractability benchmark (docs/notes/d12-lp-tractability.md): a
// 5-year/3-zone LP (~1.75 M vars) solved, the 10-year (~3.5 M vars)
// ABORTED the process. The cap sits safely between.
// ---------------------------------------------------------------------

#[test]
fn oversized_lp_errors_cleanly_instead_of_aborting_the_process() {
    // One zone, a thermal, and eight stores over a 100 000-period
    // horizon: estimated variables = 100 000 × (1 + 8×3 + 2) = 2.7 M,
    // above the 2.5 M cap. The guard must fire BEFORE any LP is built
    // (no HiGHS abort), returning a structured error.
    let periods = 100_000;
    let stores: Vec<StorageSpec> = (0..8).map(|i| battery(1.0, 1.0, 0.9, i + 1)).collect();
    let s = scenario(
        vec![zone("Z", vec![thermal("ccgt", 5.0)], stores)],
        vec![],
        periods,
    );
    let inputs = MultiZoneInputs {
        zones: vec![zone_inputs("Z", constant_power(1.0, periods))],
        link_capabilities: vec![],
    };
    let err = run_multi_lp(&s, &inputs).unwrap_err();
    match err {
        GridError::LpProblemTooLarge {
            estimated_variables,
            cap,
            ..
        } => {
            assert_eq!(cap, LP_VARIABLE_CAP);
            assert!(
                estimated_variables > cap,
                "estimate {estimated_variables} should exceed cap {cap}"
            );
        }
        other => panic!("expected LpProblemTooLarge, got {other:?}"),
    }
}

#[test]
fn binding_window_sized_lp_is_within_the_cap() {
    // A 3-year, 3-zone binding-window slice (the tractability note's
    // recommended window) is inside the cap; a 10-year one is not. Uses
    // the estimator directly so no multi-year LP is actually solved.
    // Matches the benchmark fleet: each zone a renewable + a store, a
    // thermal in the south, two links — ~20 variables per period.
    let store = || vec![battery(10.0, 5_000.0, 0.4, 1)];
    let three_zone = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 30.0)], store()),
            zone("B", vec![renewable("onshore_wind", 10.0)], store()),
            zone(
                "C",
                vec![renewable("offshore_wind", 8.0), thermal("ccgt", 20.0)],
                store(),
            ),
        ],
        vec![link("AB", "A", "B", 10.0), link("BC", "B", "C", 10.0)],
        3 * 17_520,
    );
    let three_year = estimate_lp_variables(&three_zone, 3 * 17_520);
    assert!(
        three_year < LP_VARIABLE_CAP,
        "3-year binding window ({three_year} vars) must be within the cap {LP_VARIABLE_CAP}"
    );
    // The 10-year danger zone the benchmark aborted on is rejected.
    let ten_year = estimate_lp_variables(&three_zone, 10 * 17_520);
    assert!(
        ten_year > LP_VARIABLE_CAP,
        "10-year ({ten_year} vars) must exceed the cap {LP_VARIABLE_CAP}"
    );
}
