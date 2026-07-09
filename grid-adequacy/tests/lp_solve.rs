//! Acceptance tests for the D12 perfect-foresight LP bisection ORACLE
//! (package 2b PHASE 2; `docs/notes/d12-perfect-foresight-lp.md` rules
//! 2 & 4). `min_storage_for_zero_unserved_lp` mirrors the rule-based
//! `min_storage_for_zero_unserved` bisection but uses `run_multi_lp` as
//! the inner feasibility oracle over `MultiZoneInputs`.
//!
//! Coverage:
//! 1. the LP bisection recovers a hand-computed single-zone requirement;
//! 2. the sanity invariant (rule 4): LP requirement ≤ RuleBased
//!    requirement on a general single-zone scenario (equality where the
//!    rule-based dispatch is already optimal);
//! 3. the STRICT case: on the A—B—C wheeling topology the rule-based
//!    single-pass strands surplus, so the LP needs materially less
//!    storage — shown by running the rule-based multi-zone dispatch at
//!    the LP requirement and finding it still unserved;
//! 4. determinism (ADR-5): the LP bisection is bit-reproducible.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{
    MultiZoneInputs, RunInputs, SolveOptions, ZoneInputs, min_storage_for_zero_unserved,
    min_storage_for_zero_unserved_lp, run_multi,
};
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

fn battery(power_gw: f64, energy_gwh: f64, rte: f64, initial_soc: f64) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Battery,
        power_gw: Power::gigawatts(power_gw),
        energy_gwh: Energy::gigawatt_hours(energy_gwh),
        round_trip_efficiency: PerUnit::new(rte),
        dispatch_order: 1,
        initial_soc: Some(PerUnit::new(initial_soc)),
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
        name: "synthetic-lp-solve".to_owned(),
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

fn run_inputs(demand_gw: &[f64], cf: &[(&str, &[f64])]) -> RunInputs {
    RunInputs {
        demand: power_trace(demand_gw),
        capacity_factors: cf
            .iter()
            .map(|(tech, values)| (TechId::new(*tech), cf_trace(values)))
            .collect::<BTreeMap<_, _>>(),
        exogenous: vec![],
        availability: BTreeMap::new(),
        heating: None,
    }
}

fn zone_inputs(id: &str, demand_gw: &[f64], cf: &[(&str, &[f64])]) -> ZoneInputs {
    ZoneInputs {
        pricing: None,
        id: ZoneId::new(id),
        inputs: run_inputs(demand_gw, cf),
        budgets: BTreeMap::new(),
    }
}

fn multi(zones: Vec<ZoneInputs>) -> MultiZoneInputs {
    MultiZoneInputs {
        zones,
        link_capabilities: vec![],
    }
}

// ---------------------------------------------------------------------
// Test 1 — the LP bisection recovers a hand-computed single-zone
// requirement.
//
// One lossless zone, wind [4, 0] GW against demand [2, 2], battery
// starting empty (power 10 GW, non-binding). P0 banks the 2 GW surplus,
// P1 needs 1.0 GWh delivered → minimum store ENERGY is exactly 1.0 GWh
// (the 2a hand calc). The bisection must converge to 1.0 within tol.
// ---------------------------------------------------------------------

fn storage_scenario() -> Scenario {
    scenario(
        vec![zone(
            "Z",
            vec![renewable("onshore_wind", 4.0)],
            // Placeholder energy; the bisection scales it.
            vec![battery(10.0, 1.0, 1.0, 0.0)],
        )],
        vec![],
        2,
    )
}

#[test]
fn lp_bisection_recovers_the_single_zone_requirement() {
    let s = storage_scenario();
    let inputs = multi(vec![zone_inputs(
        "Z",
        &[2.0, 2.0],
        &[("onshore_wind", &[1.0, 0.0])],
    )]);
    let out =
        min_storage_for_zero_unserved_lp(&s, &inputs, 0, 0, &SolveOptions::default()).unwrap();
    let req = out.naive.requirement.as_gigawatt_hours();
    assert!(
        (req - 1.0).abs() <= 0.1,
        "LP bisection requirement {req} GWh should be ~1.0 (hand calc)"
    );
    assert_eq!(out.store_label, "battery");
    // Feasible at/above, and the trace shows an infeasible point below.
    assert!(out.naive.iterations.iter().any(|it| !it.feasible));
    assert!(out.naive.iterations.iter().any(|it| it.feasible));
}

// ---------------------------------------------------------------------
// Test 2 — the sanity invariant (rule 4): LP requirement ≤ RuleBased.
//
// On this single-zone scenario the rule-based greedy dispatch is already
// optimal (a single store, charge from surplus, discharge on deficit),
// so the invariant holds at EQUALITY. Both bisections must return the
// same requirement within tolerance.
// ---------------------------------------------------------------------

#[test]
fn lp_requirement_is_at_most_rule_based_requirement() {
    let s = storage_scenario();
    let rule_inputs = run_inputs(&[2.0, 2.0], &[("onshore_wind", &[1.0, 0.0])]);
    let lp_inputs = multi(vec![zone_inputs(
        "Z",
        &[2.0, 2.0],
        &[("onshore_wind", &[1.0, 0.0])],
    )]);

    let rule =
        min_storage_for_zero_unserved(&s, &rule_inputs, 0, &SolveOptions::default()).unwrap();
    let lp =
        min_storage_for_zero_unserved_lp(&s, &lp_inputs, 0, 0, &SolveOptions::default()).unwrap();

    let rule_req = rule.naive.requirement.as_gigawatt_hours();
    let lp_req = lp.naive.requirement.as_gigawatt_hours();
    // The invariant: LP ≤ RuleBased (within one tolerance step).
    assert!(
        lp_req <= rule_req + 0.11,
        "LP requirement {lp_req} must not exceed rule-based {rule_req}"
    );
    // And here it is EQUAL (rule-based is already optimal).
    assert!(
        (lp_req - rule_req).abs() <= 0.11,
        "LP {lp_req} and rule-based {rule_req} should tie on this scenario"
    );
}

// ---------------------------------------------------------------------
// Test 3 — the STRICT case (rule 4): wheeling makes the LP need less.
//
// LINE topology A—B—C, all lossless, links AB and BC capped at 3 GW. A
// single period. A has surplus wind (5 GW at CF 1, demand 1 → surplus
// 4); C has a 5 GW deficit (no fleet); the store lives in C and starts
// full (only its DISCHARGE matters this period).
//
//   LP: wheels the full BC cap (3 GW) A→B→C, leaving C 2 GW short →
//       store must deliver 2 GW × 0.5 h = 1.0 GWh. Requirement = 1.0.
//   RuleBased: the single-pass equal-depth flow equalises border (A,B)
//       at 2 GW (A surplus only 4, meets B at depth 2), so only 2 GW
//       reaches C — C is 3 GW short → store must deliver 1.5 GWh.
//
// So the LP requirement (1.0) is strictly below the rule-based one
// (1.5). Proven by running the rule-based dispatch AT the LP
// requirement and finding it still unserved.
// ---------------------------------------------------------------------

fn wheeling_sizing_scenario() -> (Scenario, MultiZoneInputs) {
    let s = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 5.0)], vec![]),
            zone("B", vec![], vec![]),
            // Store in C, starts FULL; placeholder energy (bisection scales it).
            zone("C", vec![], vec![battery(10.0, 1.0, 1.0, 1.0)]),
        ],
        vec![link("AB", "A", "B", 3.0), link("BC", "B", "C", 3.0)],
        1,
    );
    let inputs = multi(vec![
        zone_inputs("A", &[1.0], &[("onshore_wind", &[1.0])]),
        zone_inputs("B", &[0.0], &[]),
        zone_inputs("C", &[5.0], &[]),
    ]);
    (s, inputs)
}

#[test]
fn lp_needs_strictly_less_storage_than_rule_based_when_wheeling_helps() {
    let (s, inputs) = wheeling_sizing_scenario();

    // The LP bisection sizes C's store (zone index 2, store index 0).
    let lp = min_storage_for_zero_unserved_lp(&s, &inputs, 2, 0, &SolveOptions::default()).unwrap();
    let lp_req = lp.naive.requirement.as_gigawatt_hours();
    assert!(
        (lp_req - 1.0).abs() <= 0.1,
        "LP requirement {lp_req} GWh should be ~1.0 (wheels 3 GW, store covers 2)"
    );

    // Now run the RULE-BASED multi-zone dispatch at the LP requirement:
    // it must STILL leave unserved energy (the single pass strands 1 GW
    // of A's surplus, so 1.5 GWh of store is really needed). This proves
    // rule-based requirement > LP requirement — strictly.
    let mut at_lp = s.clone();
    at_lp.zones[2].storage[0].energy_gwh = lp.naive.requirement;
    let rule = run_multi(&at_lp, &inputs).unwrap();
    let unserved: f64 = rule
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum();
    assert!(
        unserved > 0.1,
        "rule-based dispatch at the LP requirement ({lp_req} GWh) must still be short — \
         got {unserved} GWh unserved (LP is strictly better)"
    );
}

// ---------------------------------------------------------------------
// Test 4 — determinism (ADR-5): the LP bisection is bit-reproducible.
// ---------------------------------------------------------------------

#[test]
fn lp_bisection_is_deterministic() {
    let s = storage_scenario();
    let inputs = multi(vec![zone_inputs(
        "Z",
        &[2.0, 2.0],
        &[("onshore_wind", &[1.0, 0.0])],
    )]);
    let first =
        min_storage_for_zero_unserved_lp(&s, &inputs, 0, 0, &SolveOptions::default()).unwrap();
    let second =
        min_storage_for_zero_unserved_lp(&s, &inputs, 0, 0, &SolveOptions::default()).unwrap();
    assert!(first == second, "LP bisection differs between reruns");
}

// ---------------------------------------------------------------------
// Guard rails: bad store designation is a structured error.
// ---------------------------------------------------------------------

#[test]
fn out_of_range_store_designation_is_rejected() {
    let s = storage_scenario();
    let inputs = multi(vec![zone_inputs(
        "Z",
        &[2.0, 2.0],
        &[("onshore_wind", &[1.0, 0.0])],
    )]);
    // Zone index out of range.
    assert!(min_storage_for_zero_unserved_lp(&s, &inputs, 3, 0, &SolveOptions::default()).is_err());
    // Store index out of range.
    assert!(min_storage_for_zero_unserved_lp(&s, &inputs, 0, 5, &SolveOptions::default()).is_err());
}
