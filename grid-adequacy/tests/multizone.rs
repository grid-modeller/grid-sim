//! Stage 5 multi-zone engine unit and property tests on synthetic
//! scenarios (docs/04 Stage 5): the scarcity-equalising flow rule, link
//! capacity and loss accounting, the seasonal-budget dispatch
//! constraint, per-zone conservation, determinism, and — the hard
//! constraint of the work order — single-zone inertness (`run_multi` on
//! one zone is bit-identical to the frozen Stage 1–4 `run` path; the
//! pinned 2024 digest in grid-cli/tests/regression_2024.rs is the
//! data-backed half of the same proof).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{
    BudgetSchedule, MultiZoneInputs, RunInputs, ZoneInputs, run, run_multi, run_multi_lp,
};
use grid_core::GridError;
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, EnergyBudgetSpec, FleetEntry, Horizon, LinkSpec,
    Scenario, StorageKind, StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, PerUnit, Power};
use proptest::prelude::*;

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

/// A dispatchable entry with an energy budget (the schema names a trace;
/// synthetic runs supply the windows directly via [`BudgetSchedule`]).
fn budgeted(tech: &str, capacity_gw: f64, window_periods: usize) -> FleetEntry {
    FleetEntry {
        energy_budget: Some(EnergyBudgetSpec {
            trace: "synthetic/budget.parquet".into(),
            columns: vec!["mw".to_owned()],
            window_periods,
        }),
        ..thermal(tech, capacity_gw)
    }
}

fn zone(id: &str, fleet: Vec<FleetEntry>) -> ZoneSpec {
    ZoneSpec {
        pricing: None,
        id: ZoneId::new(id),
        demand: demand_spec(),
        exogenous_supply: vec![],
        fleet,
        storage: vec![],
    }
}

fn link(name: &str, from: &str, to: &str, cap: f64, avail: f64, loss: f64) -> LinkSpec {
    LinkSpec {
        name: Some(name.to_owned()),
        from: ZoneId::new(from),
        to: ZoneId::new(to),
        capacity_gw: Power::gigawatts(cap),
        reverse_capacity_gw: None,
        capability_trace: None,
        availability: PerUnit::new(avail),
        loss: PerUnit::new(loss),
    }
}

fn scenario(zones: Vec<ZoneSpec>, links: Vec<LinkSpec>, periods: usize) -> Scenario {
    Scenario {
        schema_version: 6,
        name: "synthetic-multizone".to_owned(),
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

fn power_trace(values: &[f64]) -> Trace<Power> {
    Trace::from_parts(
        start(),
        values.iter().map(|&v| Power::gigawatts(v)).collect(),
    )
    .unwrap()
}

fn cf_trace(values: &[f64]) -> Trace<PerUnit> {
    Trace::from_parts(start(), values.iter().map(|&v| PerUnit::new(v)).collect()).unwrap()
}

/// Zone inputs with the given constant-or-listed demand and optional CF
/// traces / budget windows.
fn zone_inputs(
    id: &str,
    demand_gw: &[f64],
    cf: &[(&str, &[f64])],
    budgets: &[(&str, usize, &[f64])],
) -> ZoneInputs {
    ZoneInputs {
        pricing: None,
        id: ZoneId::new(id),
        inputs: RunInputs {
            demand: power_trace(demand_gw),
            capacity_factors: cf
                .iter()
                .map(|(tech, values)| (TechId::new(*tech), cf_trace(values)))
                .collect::<BTreeMap<_, _>>(),
            exogenous: vec![],
            availability: BTreeMap::new(),
            heating: None,
        },
        budgets: budgets
            .iter()
            .map(|(tech, window_periods, windows_gwh)| {
                (
                    TechId::new(*tech),
                    BudgetSchedule {
                        window_periods: *window_periods,
                        windows: windows_gwh
                            .iter()
                            .map(|&g| Energy::gigawatt_hours(g))
                            .collect(),
                    },
                )
            })
            .collect(),
    }
}

fn multi(zones: Vec<ZoneInputs>) -> MultiZoneInputs {
    MultiZoneInputs {
        zones,
        link_capabilities: vec![],
    }
}

fn gw(series: &[Power]) -> Vec<f64> {
    series.iter().map(|p| p.as_gigawatts()).collect()
}

// ---------------------------------------------------------------------
// The frozen multi-zone merit ladder (Stage 7 two-ladder split,
// flow.rs FLOW_MERIT_ORDER docs): a multi-zone scenario naming a
// DISPATCH-LADDER-ONLY rung — valid single-zone since Stage 7 — is
// rejected with the structured error by BOTH multi-zone engines, which
// share the frozen six-rung lookup (multizone.rs and lp.rs). The
// ladder stays frozen pending a knowing signal-convention re-pin of
// the multi-zone digest family (the scarcity signal is numerically
// index-based).
// ---------------------------------------------------------------------

#[test]
fn extended_dispatch_rungs_are_rejected_by_both_multi_zone_engines() {
    for rung in ["beccs", "hydrogen_turbine"] {
        let s = scenario(
            vec![
                zone("A", vec![thermal(rung, 5.0)]),
                zone("B", vec![thermal("ccgt", 5.0)]),
            ],
            vec![link("AB", "A", "B", 1.0, 1.0, 0.0)],
            2,
        );
        let inputs = || {
            multi(vec![
                zone_inputs("A", &[1.0, 1.0], &[], &[]),
                zone_inputs("B", &[1.0, 1.0], &[], &[]),
            ])
        };
        // run_multi (the rule-based multi-zone engine).
        match run_multi(&s, &inputs()) {
            Err(GridError::UnknownThermalTechnology { tech }) => assert_eq!(tech, rung),
            other => {
                panic!("run_multi with {rung}: expected UnknownThermalTechnology, got {other:?}")
            }
        }
        // run_multi_lp (the perfect-foresight LP shares the frozen
        // FLOW_MERIT_ORDER lookup in lp.rs).
        match run_multi_lp(&s, &inputs()) {
            Err(GridError::UnknownThermalTechnology { tech }) => assert_eq!(tech, rung),
            other => {
                panic!("run_multi_lp with {rung}: expected UnknownThermalTechnology, got {other:?}")
            }
        }
        // The same rung is ACCEPTED by the single-zone path (the
        // engine.rs Stage 7 tests pin dispatch; this guards the
        // contrast that makes the rejection a ladder-split property,
        // not a bad id).
        let single = scenario(vec![zone("A", vec![thermal(rung, 5.0)])], vec![], 2);
        let single_inputs = RunInputs {
            demand: power_trace(&[1.0, 1.0]),
            capacity_factors: BTreeMap::new(),
            exogenous: vec![],
            availability: BTreeMap::new(),
            heating: None,
        };
        run(&single, &single_inputs)
            .unwrap_or_else(|e| panic!("single-zone {rung} must stay dispatchable: {e}"));
    }
}

// ---------------------------------------------------------------------
// Single-zone inertness: the multi-zone engine on one zone is
// bit-identical to the frozen single-zone path.
// ---------------------------------------------------------------------

#[test]
fn single_zone_run_multi_is_bit_identical_to_run() {
    let scenario = scenario(
        vec![ZoneSpec {
            storage: vec![StorageSpec {
                kind: StorageKind::Battery,
                power_gw: Power::gigawatts(2.0),
                energy_gwh: Energy::gigawatt_hours(4.0),
                round_trip_efficiency: PerUnit::new(0.81),
                dispatch_order: 1,
                initial_soc: Some(PerUnit::new(0.5)),
                shift_duration: None,
                daily_volume_limit: None,
            }],
            ..zone(
                "GB",
                vec![thermal("ccgt", 10.0), renewable("onshore_wind", 8.0)],
            )
        }],
        // Links to an undeclared external zone stay legal — and inert —
        // in a single-zone scenario (the GB reference pattern).
        vec![link("IFA", "GB", "FR", 2.0, 0.95, 0.02)],
        4,
    );
    let demand = [5.0, 1.0, 9.0, 3.0];
    let wind = [0.2, 0.9, 0.1, 0.4];

    let single = RunInputs {
        demand: power_trace(&demand),
        capacity_factors: BTreeMap::from([(TechId::new("onshore_wind"), cf_trace(&wind))]),
        exogenous: vec![],
        availability: BTreeMap::new(),
        heating: None,
    };
    let reference = run(&scenario, &single).unwrap();

    let result = run_multi(
        &scenario,
        &multi(vec![zone_inputs(
            "GB",
            &demand,
            &[("onshore_wind", &wind)],
            &[],
        )]),
    )
    .unwrap();

    assert_eq!(result.zones.len(), 1);
    assert_eq!(result.zones[0].id.as_str(), "GB");
    // Bit-identical, not approximately equal (the work-order hard
    // constraint): the multi-zone path is provably inert on one zone.
    assert!(result.zones[0].result == reference);
    // And the links never flowed.
    for series in &result.links {
        assert!(series.home_end.iter().all(|p| *p == Power::gigawatts(0.0)));
        assert!(series.away_end.iter().all(|p| *p == Power::gigawatts(0.0)));
    }
}

// ---------------------------------------------------------------------
// The flow rule: relative scarcity moves energy, bounded by capacity,
// availability, the exporter's stack, and signal equalisation.
// ---------------------------------------------------------------------

#[test]
fn flow_emerges_from_relative_scarcity_and_respects_the_capacity_cap() {
    // A is nuclear-marginal (cheap), B is ccgt-marginal (expensive):
    // energy flows A -> B at the link cap x availability.
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("nuclear", 10.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![link("AB", "A", "B", 3.0, 0.5, 0.0)],
        2,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[5.0, 5.0], &[], &[]),
            zone_inputs("B", &[5.0, 5.0], &[], &[]),
        ]),
    )
    .unwrap();

    let ab = &result.links[0];
    // Positive at the B (away) end: B imports 1.5 GW = 3.0 x 0.5.
    assert_eq!(gw(&ab.away_end), vec![1.5, 1.5]);
    assert_eq!(gw(&ab.home_end), vec![-1.5, -1.5]);
    // A's nuclear serves its own 5 plus the export; B's ccgt drops.
    let a_nuclear = &result.zones[0].result.thermal[0];
    let b_ccgt = &result.zones[1].result.thermal[0];
    assert_eq!(gw(&a_nuclear.power), vec![6.5, 6.5]);
    assert_eq!(gw(&b_ccgt.power), vec![3.5, 3.5]);
    // No unserved, no curtailment anywhere.
    for zone in &result.zones {
        assert_eq!(
            zone.result.total_unserved(),
            Energy::gigawatt_hours(0.0),
            "{}",
            zone.id
        );
        assert_eq!(zone.result.total_curtailment(), Energy::gigawatt_hours(0.0));
    }
}

#[test]
fn equalisation_stops_at_equal_marginal_stress() {
    // Both zones ccgt-marginal: flow equalises the fractional
    // utilisation of the marginal technology (the price proxy).
    // A at 2/10, B at 8/10 -> q = 3 equalises both at 5/10.
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("ccgt", 10.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![link("AB", "A", "B", 10.0, 1.0, 0.0)],
        1,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[2.0], &[], &[]),
            zone_inputs("B", &[8.0], &[], &[]),
        ]),
    )
    .unwrap();
    let ab = &result.links[0];
    assert!((ab.away_end[0].as_gigawatts() - 3.0).abs() < 1e-9);
    let a_ccgt = &result.zones[0].result.thermal[0].power;
    let b_ccgt = &result.zones[1].result.thermal[0].power;
    assert!((a_ccgt[0].as_gigawatts() - 5.0).abs() < 1e-9);
    assert!((b_ccgt[0].as_gigawatts() - 5.0).abs() < 1e-9);
}

#[test]
fn identical_zones_exchange_nothing() {
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("ccgt", 10.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![link("AB", "A", "B", 10.0, 1.0, 0.0)],
        3,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[4.0, 6.0, 8.0], &[], &[]),
            zone_inputs("B", &[4.0, 6.0, 8.0], &[], &[]),
        ]),
    )
    .unwrap();
    for series in &result.links {
        assert!(series.home_end.iter().all(|p| *p == Power::gigawatts(0.0)));
    }
}

#[test]
fn losses_land_between_the_two_ends() {
    // 10 % loss: the importer receives sent x 0.9; the wedge is the
    // link loss, per period (the conservation identity of the work
    // order: energy in = energy out + losses, per link, per period).
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("nuclear", 20.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![link("AB", "A", "B", 2.0, 1.0, 0.1)],
        1,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[5.0], &[], &[]),
            zone_inputs("B", &[5.0], &[], &[]),
        ]),
    )
    .unwrap();
    let ab = &result.links[0];
    let sent = -ab.home_end[0].as_gigawatts();
    let received = ab.away_end[0].as_gigawatts();
    assert!(sent > 0.0, "A exports");
    assert!((received - sent * 0.9).abs() < 1e-12);
    // The exporter generates the sending-end power on top of its load.
    let a_nuclear = result.zones[0].result.thermal[0].power[0].as_gigawatts();
    assert!((a_nuclear - (5.0 + sent)).abs() < 1e-9);
    let b_ccgt = result.zones[1].result.thermal[0].power[0].as_gigawatts();
    assert!((b_ccgt - (5.0 - received)).abs() < 1e-9);
}

#[test]
fn exports_never_exceed_the_exporters_stack() {
    // B is unserved-deep; A's whole stack is 3 GW against 2 GW of its
    // own load: exports are capped at the 1 GW of spare stack — a zone
    // never exports into its own unserved region.
    let scenario = scenario(
        vec![zone("A", vec![thermal("nuclear", 3.0)]), zone("B", vec![])],
        vec![link("AB", "A", "B", 5.0, 1.0, 0.0)],
        1,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[2.0], &[], &[]),
            zone_inputs("B", &[4.0], &[], &[]),
        ]),
    )
    .unwrap();
    let ab = &result.links[0];
    assert!((ab.away_end[0].as_gigawatts() - 1.0).abs() < 1e-9);
    // A fully dispatched, zero unserved; B unserved 3 GW after the
    // 1 GW import (the empty-zone case works, structured and finite).
    assert_eq!(
        result.zones[0].result.total_unserved(),
        Energy::gigawatt_hours(0.0)
    );
    let b_unserved = &result.zones[1].result.unserved;
    assert!((b_unserved[0].as_gigawatts() - 3.0).abs() < 1e-9);
}

#[test]
fn surplus_exports_reduce_curtailment_before_the_importers_stack() {
    // A has 4 GW of surplus wind; B is ccgt-marginal. The surplus flows
    // to B (cap 3), displacing B's gas; A curtails only the remainder.
    let scenario = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 10.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![link("AB", "A", "B", 3.0, 1.0, 0.0)],
        1,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[2.0], &[("onshore_wind", &[0.6])], &[]),
            zone_inputs("B", &[5.0], &[], &[]),
        ]),
    )
    .unwrap();
    let ab = &result.links[0];
    assert!((ab.away_end[0].as_gigawatts() - 3.0).abs() < 1e-9);
    // A: 6 wind - 2 load - 3 export = 1 curtailed.
    assert!((result.zones[0].result.curtailment[0].as_gigawatts() - 1.0).abs() < 1e-9);
    // B: 5 load - 3 import = 2 gas.
    assert!((result.zones[1].result.thermal[0].power[0].as_gigawatts() - 2.0).abs() < 1e-9);
}

#[test]
fn parallel_links_between_the_same_zones_split_pro_rata() {
    // Two links joining the same pair are dispatched as one border and
    // split by capacity x availability share (the CONT-NW convention:
    // Nemo and BritNed carry the same differential sign by design).
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("nuclear", 20.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![
            link("L2", "A", "B", 2.0, 1.0, 0.0),
            link("L1", "A", "B", 1.0, 1.0, 0.0),
        ],
        1,
    );
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[5.0], &[], &[]),
            zone_inputs("B", &[9.0], &[], &[]),
        ]),
    )
    .unwrap();
    // The border saturates (differential holds to the cap): 3 GW total,
    // split 2:1.
    let l2 = result.links.iter().find(|l| l.name == "L2").unwrap();
    let l1 = result.links.iter().find(|l| l.name == "L1").unwrap();
    assert!((l2.away_end[0].as_gigawatts() - 2.0).abs() < 1e-9);
    assert!((l1.away_end[0].as_gigawatts() - 1.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------
// The seasonal-budget dispatch constraint (NO2 reservoir hydro model).
// ---------------------------------------------------------------------

#[test]
fn budgeted_dispatch_is_capped_by_the_window_budget() {
    // One zone, hydro 10 GW with a 2-period window budget of 2 GWh then
    // 0: the first window serves the 2 GW load in full (2 x 1 GWh), the
    // second has no allowance left and the load goes unserved.
    let scenario = scenario(vec![zone("A", vec![budgeted("hydro", 10.0, 2)])], vec![], 4);
    let result = run_multi(
        &scenario,
        &multi(vec![zone_inputs(
            "A",
            &[2.0, 2.0, 2.0, 2.0],
            &[],
            &[("hydro", 2, &[2.0, 0.0])],
        )]),
    )
    .unwrap();
    let hydro = &result.zones[0].result.thermal[0].power;
    assert_eq!(gw(hydro), vec![2.0, 2.0, 0.0, 0.0]);
    assert_eq!(
        gw(&result.zones[0].result.unserved),
        vec![0.0, 0.0, 2.0, 2.0]
    );
}

#[test]
fn unused_budget_allowance_carries_forward_across_windows() {
    // Water stays in the reservoir: a half-used first window leaves
    // allowance for the zero-budget second window.
    let scenario = scenario(vec![zone("A", vec![budgeted("hydro", 10.0, 2)])], vec![], 4);
    let result = run_multi(
        &scenario,
        &multi(vec![zone_inputs(
            "A",
            &[1.0, 1.0, 1.0, 1.0],
            &[],
            &[("hydro", 2, &[2.0, 0.0])],
        )]),
    )
    .unwrap();
    let hydro = &result.zones[0].result.thermal[0].power;
    assert_eq!(gw(hydro), vec![1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        result.zones[0].result.total_unserved(),
        Energy::gigawatt_hours(0.0)
    );
}

#[test]
fn budget_exhaustion_limits_exports_and_the_scarcity_signal_follows() {
    // A's budgeted hydro exports to B while allowance lasts; when the
    // budget runs out mid-window, A stops exporting (and B falls back
    // to unserved) — the budget is visible to the flow rule.
    let scenario = scenario(
        vec![
            zone("A", vec![budgeted("hydro", 10.0, 4)]),
            zone("B", vec![]),
        ],
        vec![link("AB", "A", "B", 2.0, 1.0, 0.0)],
        4,
    );
    // Budget 3 GWh for the whole horizon; A load 1 GW, B load 2 GW ->
    // demand on the budget is 1.5 GWh/period; it lasts two periods.
    let result = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[1.0, 1.0, 1.0, 1.0], &[], &[("hydro", 4, &[3.0])]),
            zone_inputs("B", &[2.0, 2.0, 2.0, 2.0], &[], &[]),
        ]),
    )
    .unwrap();
    let hydro = &result.zones[0].result.thermal[0].power;
    let ab = &result.links[0];
    // Periods 0-1: full service, 3 GW hydro (1 own + 2 export).
    assert_eq!(gw(&ab.away_end)[0..2], [2.0, 2.0]);
    assert_eq!(gw(hydro)[0..2], [3.0, 3.0]);
    // Then the allowance (3 GWh - 2 x 1.5 GWh) is gone: nothing runs.
    assert_eq!(gw(hydro)[2..4], [0.0, 0.0]);
    assert_eq!(gw(&ab.away_end)[2..4], [0.0, 0.0]);
}

// ---------------------------------------------------------------------
// Structured errors.
// ---------------------------------------------------------------------

#[test]
fn multi_zone_link_to_an_undeclared_zone_is_a_structured_error() {
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("ccgt", 10.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![link("AX", "A", "X", 1.0, 1.0, 0.0)],
        1,
    );
    let err = run_multi(
        &scenario,
        &multi(vec![
            zone_inputs("A", &[1.0], &[], &[]),
            zone_inputs("B", &[1.0], &[], &[]),
        ]),
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidScenario { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains('X'), "err: {err}");
}

#[test]
fn budget_windows_must_cover_the_horizon() {
    let scenario = scenario(vec![zone("A", vec![budgeted("hydro", 10.0, 2)])], vec![], 4);
    // 4 periods at window 2 need 2 windows; give 1.
    let err = run_multi(
        &scenario,
        &multi(vec![zone_inputs(
            "A",
            &[1.0; 4],
            &[],
            &[("hydro", 2, &[2.0])],
        )]),
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidRunInputs { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn zone_inputs_must_match_the_scenario_zones() {
    let scenario = scenario(
        vec![
            zone("A", vec![thermal("ccgt", 10.0)]),
            zone("B", vec![thermal("ccgt", 10.0)]),
        ],
        vec![],
        1,
    );
    let err = run_multi(&scenario, &multi(vec![zone_inputs("A", &[1.0], &[], &[])])).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidRunInputs { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn single_zone_run_rejects_energy_budgets_loudly() {
    // The frozen single-zone path does not implement budgets; it must
    // say so rather than silently ignoring the field.
    let scenario = scenario(vec![zone("A", vec![budgeted("hydro", 10.0, 2)])], vec![], 2);
    let inputs = RunInputs {
        demand: power_trace(&[1.0, 1.0]),
        capacity_factors: BTreeMap::new(),
        exogenous: vec![],
        availability: BTreeMap::new(),
        heating: None,
    };
    let err = run(&scenario, &inputs).unwrap_err();
    assert!(
        matches!(err, GridError::UnsupportedFeature { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("energy_budget"), "err: {err}");
}

// ---------------------------------------------------------------------
// Determinism and conservation properties.
// ---------------------------------------------------------------------

#[test]
fn two_runs_of_the_same_multizone_inputs_are_bit_identical() {
    let scenario = scenario(
        vec![
            zone(
                "A",
                vec![thermal("ccgt", 10.0), renewable("onshore_wind", 6.0)],
            ),
            zone("B", vec![thermal("ccgt", 8.0)]),
        ],
        vec![link("AB", "A", "B", 2.0, 0.95, 0.03)],
        6,
    );
    let inputs = || {
        multi(vec![
            zone_inputs(
                "A",
                &[4.0, 5.0, 6.0, 3.0, 8.0, 2.0],
                &[("onshore_wind", &[0.1, 0.9, 0.5, 0.8, 0.0, 1.0])],
                &[],
            ),
            zone_inputs("B", &[5.0, 4.0, 3.0, 6.0, 7.0, 2.0], &[], &[]),
        ])
    };
    let first = run_multi(&scenario, &inputs()).unwrap();
    let second = run_multi(&scenario, &inputs()).unwrap();
    assert!(first == second, "multi-zone runs differ between reruns");
}

proptest! {
    /// Per-zone, per-period conservation with a lossy link in play:
    /// must-take + link net + thermal + discharge
    ///   = demand − unserved + charge + curtailment,
    /// and per-link, per-period: received = sent × (1 − loss), with
    /// |flow| never above capacity × availability.
    #[test]
    fn conservation_holds_per_zone_and_per_link(
        demand_a in prop::collection::vec(0.0f64..12.0, 8),
        demand_b in prop::collection::vec(0.0f64..12.0, 8),
        wind in prop::collection::vec(0.0f64..1.0, 8),
    ) {
        let scenario = scenario(
            vec![
                zone("A", vec![thermal("ccgt", 6.0), renewable("onshore_wind", 8.0)]),
                zone("B", vec![thermal("nuclear", 5.0)]),
            ],
            vec![link("AB", "A", "B", 2.0, 0.9, 0.05)],
            8,
        );
        let result = run_multi(
            &scenario,
            &multi(vec![
                zone_inputs("A", &demand_a, &[("onshore_wind", &wind)], &[]),
                zone_inputs("B", &demand_b, &[], &[]),
            ]),
        )
        .unwrap();

        let cap = 2.0 * 0.9;
        let ab = &result.links[0];
        for t in 0..8 {
            let home = ab.home_end[t].as_gigawatts();
            let away = ab.away_end[t].as_gigawatts();
            // Capacity (sending-end) and loss identity.
            prop_assert!(home.abs() <= cap + 1e-9);
            prop_assert!(away.abs() <= cap + 1e-9);
            let (sent, received) = if home < 0.0 { (-home, away) } else { (-away, home) };
            prop_assert!(sent >= -1e-12);
            prop_assert!((received - sent * 0.95).abs() < 1e-9,
                "period {t}: sent {sent} received {received}");
        }

        // Conservation per zone (link flows are folded into the zone's
        // exogenous series by the engine, so the RunResult identity is
        // the single-zone one).
        for zone_result in result.zones.iter().map(|z| &z.result) {
            for t in 0..8 {
                let supply: f64 = zone_result.renewables.iter()
                    .chain(&zone_result.thermal)
                    .map(|s| s.power[t].as_gigawatts())
                    .sum::<f64>()
                    + zone_result.exogenous.iter().map(|s| s.power[t].as_gigawatts()).sum::<f64>()
                    + zone_result.stores.iter().map(|s| s.discharge[t].as_gigawatts()).sum::<f64>();
                let uses = zone_result.demand[t].as_gigawatts()
                    - zone_result.unserved[t].as_gigawatts()
                    + zone_result.stores.iter().map(|s| s.charge[t].as_gigawatts()).sum::<f64>()
                    + zone_result.curtailment[t].as_gigawatts();
                prop_assert!((supply - uses).abs() < 1e-9,
                    "period {t}: supply {supply} != uses {uses}");
            }
        }
    }
}

// ---------------------------------------------------------------------
// Schema v6: per-direction and per-period link capability (the B6
// link-convention ruling — docs/notes/b6-two-zone-data-review.md §6a:
// export and import capabilities differ, and the 2024 validation
// configuration takes the observed half-hourly DA limit series as the
// forward capability).
// ---------------------------------------------------------------------

use grid_adequacy::LinkCapability;

/// A link with the v6 direction split (forward `from → to` at
/// `capacity_gw`, reverse at `reverse_capacity_gw`).
fn asymmetric_link(name: &str, from: &str, to: &str, fwd: f64, rev: f64) -> LinkSpec {
    LinkSpec {
        reverse_capacity_gw: Some(Power::gigawatts(rev)),
        ..link(name, from, to, fwd, 1.0, 0.0)
    }
}

#[test]
fn reverse_capacity_bounds_the_reverse_direction_only() {
    // A (the link's `from`) has cheap nuclear headroom; B (the `to`)
    // is unserved-deep: forward flow runs to the FORWARD capability.
    let fwd_scenario = scenario(
        vec![zone("A", vec![thermal("nuclear", 10.0)]), zone("B", vec![])],
        vec![asymmetric_link("B6", "A", "B", 4.1, 3.5)],
        1,
    );
    let inputs = || {
        multi(vec![
            zone_inputs("A", &[1.0], &[], &[]),
            zone_inputs("B", &[6.0], &[], &[]),
        ])
    };
    let result = run_multi(&fwd_scenario, &inputs()).unwrap();
    let b6 = &result.links[0];
    assert!(
        (b6.away_end[0].as_gigawatts() - 4.1).abs() < 1e-12,
        "forward (A→B) flow should hit the 4.1 GW forward capability, got {:?}",
        b6.away_end[0]
    );

    // Same physics, link declared the other way round: the flow now
    // runs in the link's REVERSE direction and must stop at 3.5 GW.
    let rev_scenario = scenario(
        vec![zone("A", vec![thermal("nuclear", 10.0)]), zone("B", vec![])],
        vec![asymmetric_link("B6", "B", "A", 4.1, 3.5)],
        1,
    );
    let result = run_multi(&rev_scenario, &inputs()).unwrap();
    let b6 = &result.links[0];
    assert!(
        (b6.home_end[0].as_gigawatts() - 3.5).abs() < 1e-12,
        "reverse (A→B against the link's declared direction) flow should stop at the \
         3.5 GW reverse capability, got {:?}",
        b6.home_end[0]
    );
}

#[test]
fn absent_reverse_capacity_keeps_the_symmetric_pre_v6_semantics() {
    // No reverse_capacity_gw: both directions bound by capacity_gw —
    // the pre-v6 behaviour, byte-for-byte.
    let s = scenario(
        vec![zone("A", vec![thermal("nuclear", 10.0)]), zone("B", vec![])],
        vec![link("L", "B", "A", 2.0, 1.0, 0.0)],
        1,
    );
    let result = run_multi(
        &s,
        &multi(vec![
            zone_inputs("A", &[1.0], &[], &[]),
            zone_inputs("B", &[6.0], &[], &[]),
        ]),
    )
    .unwrap();
    assert!(
        (result.links[0].home_end[0].as_gigawatts() - 2.0).abs() < 1e-12,
        "symmetric link must still carry capacity_gw in the reverse direction"
    );
    // And no capability series is recorded (pre-v6 output shape).
    assert!(result.links[0].capability.is_none());
}

#[test]
fn capability_trace_supersedes_the_forward_capacity_per_period() {
    // Forward capability per period: [2.0, 0.5, 6.0] GW — the engine
    // must clamp the same physical differential differently each
    // period. Reverse stays flat at 3.5 (unused here).
    let mut s = scenario(
        vec![zone("A", vec![thermal("nuclear", 10.0)]), zone("B", vec![])],
        vec![asymmetric_link("B6", "A", "B", 4.1, 3.5)],
        3,
    );
    s.links[0].capability_trace = Some(grid_core::scenario::LinkCapabilityTraceSpec {
        path: "synthetic/b6.parquet".to_owned(),
        column: "limit_mw".to_owned(),
        sentinel_high_mw: Power::megawatts(9999.0),
        upper_bound_gw: Power::gigawatts(6.7),
        masked_fill_gw: Power::gigawatts(4.1),
    });
    let mut inputs = multi(vec![
        zone_inputs("A", &[1.0, 1.0, 1.0], &[], &[]),
        zone_inputs("B", &[6.0, 6.0, 6.0], &[], &[]),
    ]);
    inputs.link_capabilities = vec![Some(LinkCapability {
        forward: vec![
            Power::gigawatts(2.0),
            Power::gigawatts(0.5),
            Power::gigawatts(6.0),
        ],
        observed: vec![true, false, true],
    })];
    let result = run_multi(&s, &inputs).unwrap();
    let b6 = &result.links[0];
    let sent: Vec<f64> = b6.away_end.iter().map(|p| p.as_gigawatts()).collect();
    assert!((sent[0] - 2.0).abs() < 1e-12, "period 0: {sent:?}");
    assert!((sent[1] - 0.5).abs() < 1e-12, "period 1: {sent:?}");
    // Period 2: capability 6 GW exceeds B's 6 GW deficit — the flow
    // equalises at B fully served (A has 10 GW of stack against 1 GW
    // of load; both signals meet inside A's nuclear rung).
    assert!(sent[2] > 4.1 && sent[2] <= 6.0, "period 2: {sent:?}");

    // The applied capabilities are recorded for the output layer (the
    // B6 binding columns), with the observation mask carried through.
    let capability = b6.capability.as_ref().unwrap();
    let fwd: Vec<f64> = capability
        .forward
        .iter()
        .map(|p| p.as_gigawatts())
        .collect();
    assert_eq!(fwd, vec![2.0, 0.5, 6.0]);
    assert_eq!(capability.forward_observed, vec![true, false, true]);
    let rev: Vec<f64> = capability
        .reverse
        .iter()
        .map(|p| p.as_gigawatts())
        .collect();
    assert_eq!(rev, vec![3.5, 3.5, 3.5]);
}

#[test]
fn a_trace_declaring_link_without_capability_inputs_is_a_structured_error() {
    let mut s = scenario(
        vec![zone("A", vec![thermal("nuclear", 10.0)]), zone("B", vec![])],
        vec![asymmetric_link("B6", "A", "B", 4.1, 3.5)],
        1,
    );
    s.links[0].capability_trace = Some(grid_core::scenario::LinkCapabilityTraceSpec {
        path: "synthetic/b6.parquet".to_owned(),
        column: "limit_mw".to_owned(),
        sentinel_high_mw: Power::megawatts(9999.0),
        upper_bound_gw: Power::gigawatts(6.7),
        masked_fill_gw: Power::gigawatts(4.1),
    });
    let err = run_multi(
        &s,
        &multi(vec![
            zone_inputs("A", &[1.0], &[], &[]),
            zone_inputs("B", &[6.0], &[], &[]),
        ]),
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("B6"), "err: {msg}");
    assert!(msg.contains("capability"), "err: {msg}");
}

#[test]
fn availability_derates_both_directional_capabilities() {
    // availability 0.5 on a 4.1/3.5 link: forward binds at 2.05.
    let mut s = scenario(
        vec![zone("A", vec![thermal("nuclear", 10.0)]), zone("B", vec![])],
        vec![asymmetric_link("B6", "A", "B", 4.1, 3.5)],
        1,
    );
    s.links[0].availability = PerUnit::new(0.5);
    let result = run_multi(
        &s,
        &multi(vec![
            zone_inputs("A", &[1.0], &[], &[]),
            zone_inputs("B", &[6.0], &[], &[]),
        ]),
    )
    .unwrap();
    assert!(
        (result.links[0].away_end[0].as_gigawatts() - 2.05).abs() < 1e-12,
        "forward capability must be derated by availability"
    );
}

// ---------------------------------------------------------------------
// Schema v7 (D11): the priced-ladder flow signal.
// ---------------------------------------------------------------------

/// A minimal [zones.pricing] spec naming a gas SRMC recipe for `ccgt`
/// (scenario-level coherence only; synthetic runs supply the loaded
/// SRMC traces directly on `ZoneInputs.pricing`).
fn zone_pricing_spec() -> grid_core::scenario::ZonePricingSpec {
    grid_core::scenario::ZonePricingSpec {
        reference: "data/reference/prices-2024.toml".to_owned(),
        carbon_flat_gbp_per_tco2: None,
        fuel_price: [(
            "gas".to_owned(),
            grid_core::scenario::TraceRefSpec {
                path: "synthetic/gas.parquet".to_owned(),
                column: "gas".to_owned(),
            },
        )]
        .into_iter()
        .collect(),
        srmc: [(
            "ccgt".to_owned(),
            grid_core::scenario::SrmcRecipeSpec {
                fuel: "gas".to_owned(),
                efficiency: "ccgt".to_owned(),
            },
        )]
        .into_iter()
        .collect(),
    }
}

fn price_trace(values: &[f64]) -> Trace<grid_core::units::Price> {
    Trace::from_parts(
        start(),
        values
            .iter()
            .map(|&v| grid_core::units::Price::pounds_per_megawatt_hour(v))
            .collect(),
    )
    .unwrap()
}

/// Attach loaded SRMC inputs (ccgt at a flat level) to zone inputs.
fn with_srmc(mut inputs: ZoneInputs, ccgt_srmc: &[f64]) -> ZoneInputs {
    inputs.pricing = Some(grid_adequacy::ZonePricingInputs {
        srmc: [(TechId::new("ccgt"), price_trace(ccgt_srmc))]
            .into_iter()
            .collect(),
    });
    inputs
}

/// A two-zone both-gas-marginal system: A has the big gas fleet at low
/// fractional utilisation, B the small fleet at high utilisation.
fn both_gas_scenario(flow_signal: grid_core::scenario::FlowSignal) -> Scenario {
    let mut a = zone("A", vec![thermal("ccgt", 30.0)]);
    a.pricing = Some(zone_pricing_spec());
    let mut b = zone("B", vec![thermal("ccgt", 10.0)]);
    b.pricing = Some(zone_pricing_spec());
    let mut s = scenario(vec![a, b], vec![link("AB", "A", "B", 2.0, 1.0, 0.0)], 2);
    s.dispatch.flow_signal = flow_signal;
    s
}

/// THE MECHANISM TEST: in a both-gas-marginal period the scarcity rule
/// trades toward the proportionally more-stressed gas fleet, blind to
/// price; the priced ladder follows the SRMC wedge instead, and a
/// wedge favouring the opposite direction flips the flow (bang-bang to
/// the rung edge or the cap — flow.rs prose rule 1b).
#[test]
fn priced_ladder_follows_the_srmc_wedge_where_the_scarcity_rule_follows_stress() {
    use grid_core::scenario::FlowSignal;
    // Demand: A 6 GW of 30 (0.2 utilisation), B 8 GW of 10 (0.8).
    let demand_a = [6.0, 6.0];
    let demand_b = [8.0, 8.0];

    // Scarcity rule: flow A → B toward equal utilisation.
    let s = both_gas_scenario(FlowSignal::Scarcity);
    let inputs = multi(vec![
        with_srmc(zone_inputs("A", &demand_a, &[], &[]), &[80.0, 80.0]),
        with_srmc(zone_inputs("B", &demand_b, &[], &[]), &[81.0, 81.0]),
    ]);
    let scarcity = run_multi(&s, &inputs).unwrap();
    let flow = gw(&scarcity.links[0].home_end);
    assert!(flow[0] < 0.0, "scarcity: A exports to B, got {flow:?}");

    // Priced ladder, same SRMC wedge direction (B dearer): still A → B,
    // but bang-bang to the 2 GW cap instead of the equalising 1.5 GW.
    let s = both_gas_scenario(FlowSignal::PricedLadder);
    let priced = run_multi(&s, &inputs).unwrap();
    let flow = gw(&priced.links[0].home_end);
    assert!(
        (flow[0] + 2.0).abs() < 1e-12,
        "priced ladder with B dearer: A exports at the cap, got {flow:?}"
    );

    // Priced ladder, wedge REVERSED (A dearer): the direction flips
    // against the stress gradient — the scarcity rule could never
    // produce this flow.
    let inputs = multi(vec![
        with_srmc(zone_inputs("A", &demand_a, &[], &[]), &[81.0, 81.0]),
        with_srmc(zone_inputs("B", &demand_b, &[], &[]), &[80.0, 80.0]),
    ]);
    let priced = run_multi(&s, &inputs).unwrap();
    let flow = gw(&priced.links[0].home_end);
    assert!(
        flow[0] > 0.0,
        "priced ladder with A dearer: B exports to A, got {flow:?}"
    );
}

/// PROPERTY (a) OF THE WORK ORDER — the graceful-degradation
/// guarantee: with EQUAL per-zone SRMCs the priced ladder's flows are
/// byte-identical to the scarcity rule's, across a varied multi-period
/// system (surplus, shared-rung, ladder-gap and deficit periods).
#[test]
fn priced_ladder_with_equal_per_zone_srmcs_is_byte_identical_to_the_scarcity_rule() {
    use grid_core::scenario::FlowSignal;
    let build = |flow_signal| {
        let mut a = zone(
            "A",
            vec![
                thermal("nuclear", 3.0),
                thermal("ccgt", 12.0),
                renewable("wind", 10.0),
            ],
        );
        a.pricing = Some(zone_pricing_spec());
        let mut b = zone("B", vec![thermal("ccgt", 8.0)]);
        b.pricing = Some(zone_pricing_spec());
        let mut c = zone("C", vec![thermal("hydro", 4.0)]);
        // C has no priced technology: an empty (but present) pricing
        // block — its rungs price at the £0 must-take floor.
        c.pricing = Some(grid_core::scenario::ZonePricingSpec {
            reference: "data/reference/prices-2024.toml".to_owned(),
            carbon_flat_gbp_per_tco2: None,
            fuel_price: BTreeMap::new(),
            srmc: BTreeMap::new(),
        });
        let mut s = scenario(
            vec![a, b, c],
            vec![
                link("AB", "A", "B", 2.0, 1.0, 0.021),
                link("BC", "B", "C", 1.0, 1.0, 0.0),
                link("AC", "A", "C", 1.5, 0.9, 0.05),
            ],
            6,
        );
        s.dispatch.flow_signal = flow_signal;
        s
    };
    // Equal SRMC series in every zone (the same gas price + carbon).
    let srmc = [79.0, 81.5, 90.0, 60.0, 75.0, 82.0];
    let wind = [0.9, 0.1, 0.0, 1.0, 0.5, 0.2];
    let make_inputs = || {
        multi(vec![
            with_srmc(
                zone_inputs(
                    "A",
                    &[4.0, 12.0, 14.0, 2.0, 8.0, 15.9],
                    &[("wind", &wind)],
                    &[],
                ),
                &srmc,
            ),
            with_srmc(
                zone_inputs("B", &[6.0, 2.0, 7.9, 5.0, 4.0, 8.5], &[], &[]),
                &srmc,
            ),
            {
                let mut c = zone_inputs("C", &[1.0, 3.0, 4.5, 0.5, 2.0, 5.0], &[], &[]);
                c.pricing = Some(grid_adequacy::ZonePricingInputs {
                    srmc: BTreeMap::new(),
                });
                c
            },
        ])
    };
    let scarcity = run_multi(&build(FlowSignal::Scarcity), &make_inputs()).unwrap();
    let priced = run_multi(&build(FlowSignal::PricedLadder), &make_inputs()).unwrap();
    assert!(
        scarcity.links == priced.links,
        "equal per-zone SRMCs must reproduce the scarcity flows byte-for-byte"
    );
    assert!(
        scarcity.zones == priced.zones,
        "equal per-zone SRMCs must reproduce the zone results byte-for-byte"
    );
}

/// PROPERTY (b) OF THE WORK ORDER — determinism: two priced-ladder
/// runs of identical inputs are bit-identical (ADR-5).
#[test]
fn priced_ladder_reruns_are_bit_identical() {
    use grid_core::scenario::FlowSignal;
    let s = both_gas_scenario(FlowSignal::PricedLadder);
    let make_inputs = || {
        multi(vec![
            with_srmc(zone_inputs("A", &[6.0, 20.0], &[], &[]), &[80.0, 85.0]),
            with_srmc(zone_inputs("B", &[8.0, 3.0], &[], &[]), &[81.0, 79.0]),
        ])
    };
    let first = run_multi(&s, &make_inputs()).unwrap();
    let second = run_multi(&s, &make_inputs()).unwrap();
    assert!(first == second, "priced-ladder reruns differ (ADR-5)");
}

/// The engine refuses a priced-ladder run whose zone inputs carry no
/// loaded pricing (the scenario-level requirement has an input-level
/// counterpart — inputs may be constructed programmatically).
#[test]
fn priced_ladder_without_loaded_pricing_inputs_is_a_structured_error() {
    use grid_core::scenario::FlowSignal;
    let s = both_gas_scenario(FlowSignal::PricedLadder);
    let inputs = multi(vec![
        with_srmc(zone_inputs("A", &[6.0, 6.0], &[], &[]), &[80.0, 80.0]),
        zone_inputs("B", &[8.0, 8.0], &[], &[]), // no pricing loaded
    ]);
    let err = run_multi(&s, &inputs).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidRunInputs { .. }),
        "unexpected: {err:?}"
    );
    assert!(err.to_string().contains("priced_ladder"), "err: {err}");
}

/// A priced-ladder run with NO priced technology anywhere has no
/// fleet-SRMC ceiling to price unserved periods (Stage 2 convention 3
/// analogue) — a structured error, never a silent £0.
#[test]
fn priced_ladder_with_no_priced_technology_anywhere_is_a_structured_error() {
    use grid_core::scenario::FlowSignal;
    let s = both_gas_scenario(FlowSignal::PricedLadder);
    let strip = |mut z: ZoneInputs| {
        z.pricing = Some(grid_adequacy::ZonePricingInputs {
            srmc: BTreeMap::new(),
        });
        z
    };
    let inputs = multi(vec![
        strip(zone_inputs("A", &[6.0, 6.0], &[], &[])),
        strip(zone_inputs("B", &[8.0, 8.0], &[], &[])),
    ]);
    let err = run_multi(&s, &inputs).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidRunInputs { .. }),
        "unexpected: {err:?}"
    );
    assert!(err.to_string().contains("ceiling"), "err: {err}");
}
