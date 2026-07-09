//! Stage 7 acceptance tests for the per-scenario LP-vs-rule-based
//! storage GAP REPORT (docs/04 Stage 7 acceptance line: "LP storage
//! requirement ≤ rule-based on every scenario (sanity invariant); gap
//! reported per scenario").
//!
//! The invariant machinery exists from D12 2b
//! (`min_storage_for_zero_unserved_lp`, `docs/notes/
//! d12-perfect-foresight-lp.md` rule 4); this file pins the REPORT
//! surface: both requirements measured by the same bisection on the
//! same designation, the gap as a first-class artefact value, the
//! invariant asserted structurally (a violation is an engine defect,
//! not a finding), and the publication stamps (quarantine → refuse;
//! the HiGHS cross-machine floating-point caveat travels with every
//! published LP number — the 2b review obligation).
//!
//! The strict-gap scenario is the committed A—B—C wheeling fixture
//! from lp_solve.rs (LP wheels the full 3 GW path so C's store needs
//! ~1.0 GWh; the rule-based single-pass equal-depth flow strands 1 GW
//! of A's surplus so the store needs ~1.5 GWh) — the pinned example of
//! a strictly positive gap.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{
    MultiZoneInputs, RunInputs, SolveOptions, ZoneInputs, check_storage_gap_invariant,
    storage_gap_report,
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
        name: "synthetic-gap-report".to_owned(),
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

/// The committed strict-gap fixture (lp_solve.rs test 3): A—B—C line,
/// links capped at 3 GW, one period; A has 4 GW surplus, C a 5 GW
/// deficit served by C's store (starts full). LP requirement ~1.0 GWh;
/// rule-based ~1.5 GWh.
fn wheeling_scenario() -> (Scenario, MultiZoneInputs) {
    let s = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 5.0)], vec![]),
            zone("B", vec![], vec![]),
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

/// A single-zone fixture where the rule-based greedy dispatch is
/// already optimal, so the gap is ~zero.
fn equality_scenario() -> (Scenario, MultiZoneInputs) {
    let s = scenario(
        vec![zone(
            "Z",
            vec![renewable("onshore_wind", 4.0)],
            vec![battery(10.0, 1.0, 1.0, 0.0)],
        )],
        vec![],
        2,
    );
    let inputs = multi(vec![zone_inputs(
        "Z",
        &[2.0, 2.0],
        &[("onshore_wind", &[1.0, 0.0])],
    )]);
    (s, inputs)
}

// ---------------------------------------------------------------------
// The pinned strict-gap example: gap reported per scenario, nonzero on
// the A—B—C wheel.
// ---------------------------------------------------------------------

#[test]
fn gap_report_pins_the_strict_wheeling_gap() {
    let (s, inputs) = wheeling_scenario();
    let report = storage_gap_report(&s, &inputs, 2, 0, &SolveOptions::default(), &[]).unwrap();

    assert_eq!(report.scenario_name, "synthetic-gap-report");
    assert_eq!(report.store_label, "battery");

    let rule = report.rule_based_requirement.as_gigawatt_hours();
    let lp = report.lp_requirement.as_gigawatt_hours();
    let gap = report.gap.as_gigawatt_hours();
    // Pinned example values (hand calc, lp_solve.rs test 3): the
    // rule-based single pass needs ~1.5 GWh, the LP ~1.0 GWh.
    assert!((1.4..=1.6).contains(&rule), "rule-based {rule} GWh");
    assert!((0.9..=1.1).contains(&lp), "LP {lp} GWh");
    assert!((0.3..=0.7).contains(&gap), "gap {gap} GWh");
    // The gap is strictly positive beyond the bisection slack — the
    // wheeling advantage is real, not tolerance dust.
    assert!(
        report.gap > report.invariant_slack,
        "gap {gap} GWh must exceed the slack {} GWh",
        report.invariant_slack.as_gigawatt_hours()
    );
    // gap = rule-based − LP, exactly.
    assert_eq!(
        report.gap,
        report.rule_based_requirement - report.lp_requirement
    );
    // The full bisection traces travel with the artefact (ADR-10: keep
    // the whole response).
    assert!(!report.rule_based.naive.iterations.is_empty());
    assert!(!report.lp.naive.iterations.is_empty());
}

// ---------------------------------------------------------------------
// The invariant holds at equality where rule-based is already optimal.
// ---------------------------------------------------------------------

#[test]
fn gap_report_holds_at_equality_where_rule_based_is_optimal() {
    let (s, inputs) = equality_scenario();
    let report = storage_gap_report(&s, &inputs, 0, 0, &SolveOptions::default(), &[]).unwrap();
    let gap = report.gap.as_gigawatt_hours();
    assert!(
        gap.abs() <= report.invariant_slack.as_gigawatt_hours(),
        "gap {gap} GWh should be within the bisection slack on an already-optimal scenario"
    );
}

// ---------------------------------------------------------------------
// The sanity invariant is structural: a measured LP > rule-based beyond
// the bisection slack is an engine defect and errors loudly. (The full
// report path cannot produce one by construction, so the check itself
// is exercised directly.)
// ---------------------------------------------------------------------

#[test]
fn gap_invariant_violation_is_a_structured_error() {
    let err = check_storage_gap_invariant(
        Energy::gigawatt_hours(1.0),
        Energy::gigawatt_hours(2.0),
        Energy::gigawatt_hours(0.2),
    )
    .unwrap_err();
    match err {
        GridError::SanityInvariantViolated { reason } => {
            assert!(reason.contains("LP"), "reason: {reason}");
        }
        other => panic!("expected SanityInvariantViolated, got {other:?}"),
    }
    // Within slack, or genuinely below: no violation.
    check_storage_gap_invariant(
        Energy::gigawatt_hours(1.0),
        Energy::gigawatt_hours(1.1),
        Energy::gigawatt_hours(0.2),
    )
    .unwrap();
    check_storage_gap_invariant(
        Energy::gigawatt_hours(1.5),
        Energy::gigawatt_hours(1.0),
        Energy::gigawatt_hours(0.2),
    )
    .unwrap();
}

// ---------------------------------------------------------------------
// Publication stamps: quarantine propagates and the publish path
// refuses; the HiGHS floating-point caveat always travels.
// ---------------------------------------------------------------------

#[test]
fn quarantined_inputs_stamp_the_report_non_quotable() {
    let (s, inputs) = wheeling_scenario();
    let report = storage_gap_report(
        &s,
        &inputs,
        2,
        0,
        &SolveOptions::default(),
        &["storage.battery_li_ion".to_owned()],
    )
    .unwrap();
    assert!(!report.quotable);
    match report.ensure_publishable() {
        Err(GridError::NonQuotableResult { reason }) => {
            assert!(
                reason.contains("storage.battery_li_ion"),
                "reason: {reason}"
            );
        }
        other => panic!("expected NonQuotableResult, got {other:?}"),
    }
}

#[test]
fn clean_report_is_publishable_and_carries_the_higgs_caveat() {
    let (s, inputs) = wheeling_scenario();
    let report = storage_gap_report(&s, &inputs, 2, 0, &SolveOptions::default(), &[]).unwrap();
    assert!(report.quotable);
    report.ensure_publishable().unwrap();
    // The 2b review obligation: the HiGHS cross-machine floating-point
    // caveat travels with every published LP number.
    assert!(
        report.caveats.iter().any(|c| c.contains("HiGHS")),
        "caveats: {:?}",
        report.caveats
    );
    // The ADR-6 framing: the gap is a reported finding.
    assert!(
        report.caveats.iter().any(|c| c.contains("finding")),
        "caveats: {:?}",
        report.caveats
    );
}

// ---------------------------------------------------------------------
// Determinism (ADR-5).
// ---------------------------------------------------------------------

#[test]
fn gap_report_is_deterministic() {
    let (s, inputs) = wheeling_scenario();
    let first = storage_gap_report(&s, &inputs, 2, 0, &SolveOptions::default(), &[]).unwrap();
    let second = storage_gap_report(&s, &inputs, 2, 0, &SolveOptions::default(), &[]).unwrap();
    assert!(first == second, "gap report differs between reruns");
}
