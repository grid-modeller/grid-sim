//! Stage 3 storage mechanics: the D4 acceptance hooks
//! (docs/notes/d4-rule-based-dispatch.md §"Acceptance hooks") plus
//! hand-checkable rule-by-rule cases.
//!
//! - SoC conservation property test: every period,
//!   `ΔSoC = charge×√η×Δt − discharge×Δt/√η` (to f64 rounding);
//!   SoC ∈ [0, capacity].
//! - Charge/curtailment and discharge/unserved exclusivity.
//! - Duplicate `dispatch_order` within a zone → validation error.
//! - Policy-boundary test: the `SystemState` passed to `RuleBased`
//!   contains no data from periods beyond the current one.
//! - DSR stores are rejected at run time (Q6 work; schema shape only).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;

use grid_adequacy::{
    DispatchDecision, DispatchPolicy, ExogenousSupply, PolicyContract, RuleBased, RunInputs,
    RunResult, StoreAction, SystemState, run, run_with_policy,
};
use grid_core::GridError;
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, Scenario, StorageKind,
    StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, PerUnit, Power};
use proptest::prelude::*;

const START: &str = "2024-01-01T00:00:00Z";

/// A single-zone scenario with the given fleet and storage, one demand
/// period per entry of the caller's traces.
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
        name: "synthetic".to_owned(),
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

fn scenario(fleet: Vec<FleetEntry>, storage: Vec<StorageSpec>, periods: usize) -> Scenario {
    scenario_at(START, fleet, storage, periods)
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

/// A store spec with the Stage 3 fields.
fn store(kind: StorageKind, power: f64, energy: f64, rte: f64, order: u8) -> StorageSpec {
    StorageSpec {
        kind,
        power_gw: Power::gigawatts(power),
        energy_gwh: Energy::gigawatt_hours(energy),
        round_trip_efficiency: PerUnit::new(rte),
        dispatch_order: order,
        initial_soc: None,
        shift_duration: None,
        daily_volume_limit: None,
    }
}

fn store_with_soc(
    kind: StorageKind,
    power: f64,
    energy: f64,
    rte: f64,
    order: u8,
    initial_soc: f64,
) -> StorageSpec {
    StorageSpec {
        initial_soc: Some(PerUnit::new(initial_soc)),
        ..store(kind, power, energy, rte, order)
    }
}

fn power_trace_at(start: &str, values: &[f64]) -> Trace<Power> {
    Trace::from_parts(
        UtcInstant::parse(start).unwrap(),
        values.iter().map(|&v| Power::gigawatts(v)).collect(),
    )
    .unwrap()
}

fn cf_trace_at(start: &str, values: &[f64]) -> Trace<PerUnit> {
    Trace::from_parts(
        UtcInstant::parse(start).unwrap(),
        values.iter().map(|&v| PerUnit::new(v)).collect(),
    )
    .unwrap()
}

fn inputs_at(
    start: &str,
    demand_gw: &[f64],
    cf: &[(&str, &[f64])],
    exogenous: Vec<ExogenousSupply>,
) -> RunInputs {
    RunInputs {
        demand: power_trace_at(start, demand_gw),
        capacity_factors: cf
            .iter()
            .map(|(tech, values)| (TechId::new(*tech), cf_trace_at(start, values)))
            .collect::<BTreeMap<_, _>>(),
        exogenous,
        availability: BTreeMap::new(),
        heating: None,
    }
}

fn inputs(demand_gw: &[f64], cf: &[(&str, &[f64])]) -> RunInputs {
    inputs_at(START, demand_gw, cf, vec![])
}

fn store_series<'r>(result: &'r RunResult, label: &str) -> &'r grid_adequacy::StoreSeries {
    result
        .stores
        .iter()
        .find(|s| s.label == label)
        .unwrap_or_else(|| panic!("no store series {label}"))
}

fn gwh(e: Energy) -> f64 {
    e.as_gigawatt_hours()
}

// ---------------------------------------------------------------------
// Hand-checkable rule-by-rule cases (D4 rules 1–4).
// ---------------------------------------------------------------------

/// D4 rule 2 + √η mechanics: surplus charges the store; SoC gains
/// `power × Δt × √η`.
#[test]
fn surplus_charges_the_store_with_sqrt_eta_gain() {
    // Wind 12 GW vs demand 10 GW: 2 GW surplus. Battery power 5 GW,
    // η = 0.81 (√η = 0.9), starts empty.
    let s = scenario(
        vec![renewable("onshore_wind", 20.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            0.81,
            1,
            0.0,
        )],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[("onshore_wind", &[0.6])])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.charge, vec![Power::gigawatts(2.0)]);
    assert_eq!(battery.discharge, vec![Power::gigawatts(0.0)]);
    // ΔSoC = 2 GW × 0.5 h × 0.9 = 0.9 GWh.
    assert!(
        (gwh(battery.soc[0]) - 0.9).abs() < 1e-12,
        "soc {:?}",
        battery.soc
    );
    // The absorbed surplus is no longer curtailment.
    assert_eq!(result.curtailment, vec![Power::gigawatts(0.0)]);
}

/// D4 rule 2: surplus no store can absorb is curtailment (power cap).
#[test]
fn surplus_beyond_store_power_is_curtailed() {
    let s = scenario(
        vec![renewable("onshore_wind", 20.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            1.5,
            100.0,
            1.0,
            1,
            0.0,
        )],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[("onshore_wind", &[0.6])])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.charge, vec![Power::gigawatts(1.5)]);
    assert_eq!(result.curtailment, vec![Power::gigawatts(0.5)]);
}

/// D4 rule 2: a full store cannot absorb surplus (headroom limit; the
/// initial-SoC default is FULL).
#[test]
fn a_full_store_absorbs_nothing_and_initial_soc_defaults_to_full() {
    let s = scenario(
        vec![renewable("onshore_wind", 20.0)],
        vec![store(StorageKind::Battery, 5.0, 100.0, 1.0, 1)],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[("onshore_wind", &[0.6])])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.charge, vec![Power::gigawatts(0.0)]);
    assert_eq!(battery.soc, vec![Energy::gigawatt_hours(100.0)]);
    assert_eq!(result.curtailment, vec![Power::gigawatts(2.0)]);
}

/// D4 rule 3 + √η mechanics: deficit discharges the store after the
/// stack; SoC loses `power × Δt / √η`.
#[test]
fn deficit_discharges_after_the_stack_with_sqrt_eta_loss() {
    // Demand 10 GW, ccgt 6 GW: post-stack deficit 4 GW. Battery power
    // 5 GW, η = 0.81 (√η = 0.9), starts full at 100 GWh.
    let s = scenario(
        vec![thermal("ccgt", 6.0)],
        vec![store(StorageKind::Battery, 5.0, 100.0, 0.81, 1)],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.discharge, vec![Power::gigawatts(4.0)]);
    // The stack ran first, in full.
    let ccgt = result
        .thermal
        .iter()
        .find(|t| t.tech.as_str() == "ccgt")
        .unwrap();
    assert_eq!(ccgt.power, vec![Power::gigawatts(6.0)]);
    // ΔSoC = −4 GW × 0.5 h / 0.9 = −2.2222… GWh.
    assert!((gwh(battery.soc[0]) - (100.0 - 4.0 * 0.5 / 0.9)).abs() < 1e-9);
    assert_eq!(result.unserved, vec![Power::gigawatts(0.0)]);
}

/// D4 rule 3: deficit no store can cover is unserved energy.
#[test]
fn deficit_beyond_stores_is_unserved() {
    let s = scenario(
        vec![thermal("ccgt", 6.0)],
        vec![store(StorageKind::Battery, 1.0, 100.0, 1.0, 1)],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[])).unwrap();
    assert_eq!(result.unserved, vec![Power::gigawatts(3.0)]);
}

/// D4 rule 2 (surplus-only charging): storage never charges from the
/// thermal stack — in a deficit period the store sits idle even with
/// spare stack headroom.
#[test]
fn charging_draws_from_surplus_only_never_from_the_stack() {
    // Demand 10 GW, ccgt 30 GW (20 GW headroom), battery empty: no
    // charging happens.
    let s = scenario(
        vec![thermal("ccgt", 30.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        2,
    );
    let result = run(&s, &inputs(&[10.0, 10.0], &[])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.charge, vec![Power::gigawatts(0.0); 2]);
    assert_eq!(battery.soc, vec![Energy::gigawatt_hours(0.0); 2]);
}

/// D4 rule 1: `net = 0` → no storage action.
#[test]
fn exact_balance_means_no_storage_action() {
    let s = scenario(
        vec![renewable("onshore_wind", 20.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.5,
        )],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[("onshore_wind", &[0.5])])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.charge, vec![Power::gigawatts(0.0)]);
    assert_eq!(battery.discharge, vec![Power::gigawatts(0.0)]);
}

/// D4 rule 2: stores charge in ascending dispatch_order; the same order
/// applies to discharge (rule 3).
#[test]
fn stores_charge_and_discharge_in_ascending_dispatch_order() {
    // Surplus 3 GW: order-1 battery (2 GW) absorbs first, order-2
    // hydrogen takes the remaining 1 GW.
    let s = scenario(
        vec![renewable("onshore_wind", 20.0)],
        vec![
            store_with_soc(StorageKind::Hydrogen, 5.0, 1000.0, 1.0, 2, 0.0),
            store_with_soc(StorageKind::Battery, 2.0, 100.0, 1.0, 1, 0.0),
        ],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[("onshore_wind", &[0.65])])).unwrap();
    assert_eq!(
        store_series(&result, "battery").charge,
        vec![Power::gigawatts(2.0)]
    );
    assert_eq!(
        store_series(&result, "hydrogen").charge,
        vec![Power::gigawatts(1.0)]
    );

    // Deficit 3 GW, no stack: battery (order 1) discharges first.
    let s = scenario(
        vec![],
        vec![
            store(StorageKind::Hydrogen, 5.0, 1000.0, 1.0, 2),
            store(StorageKind::Battery, 2.0, 100.0, 1.0, 1),
        ],
        1,
    );
    let result = run(&s, &inputs(&[3.0], &[])).unwrap();
    assert_eq!(
        store_series(&result, "battery").discharge,
        vec![Power::gigawatts(2.0)]
    );
    assert_eq!(
        store_series(&result, "hydrogen").discharge,
        vec![Power::gigawatts(1.0)]
    );
}

/// D4 rule 4 (no reserve holding): the store discharges for today's
/// small deficit even though tomorrow's bigger one will then go
/// unserved. Greedy means greedy.
#[test]
fn no_reserve_holding_greedy_discharges_today() {
    // Store holds 1 GWh (η = 1). Period 1: deficit 1 GW (0.5 GWh);
    // period 2: deficit 4 GW. Greedy serves period 1 fully, leaving only
    // 0.5 GWh (1 GW for a half-hour) for period 2.
    let s = scenario(
        vec![],
        vec![store_with_soc(StorageKind::Battery, 10.0, 1.0, 1.0, 1, 1.0)],
        2,
    );
    let result = run(&s, &inputs(&[1.0, 4.0], &[])).unwrap();
    let battery = store_series(&result, "battery");
    assert_eq!(battery.discharge[0], Power::gigawatts(1.0));
    assert_eq!(result.unserved[0], Power::gigawatts(0.0));
    assert_eq!(battery.discharge[1], Power::gigawatts(1.0));
    assert_eq!(result.unserved[1], Power::gigawatts(3.0));
}

/// SoC carries across the year boundary: no annual reset (D4 mechanics;
/// docs/04 Stage 3 multi-year continuous). Charge on 31 December,
/// discharge on 1 January.
#[test]
fn soc_carries_across_the_year_boundary_without_reset() {
    // Two periods: 2024-12-31T23:30Z (surplus) then 2025-01-01T00:00Z
    // (deficit).
    let s = scenario_at(
        "2024-12-31T23:30:00Z",
        vec![renewable("onshore_wind", 20.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        2,
    );
    let result = run(
        &s,
        &inputs_at(
            "2024-12-31T23:30:00Z",
            &[10.0, 10.0],
            &[("onshore_wind", &[0.7, 0.0])],
            vec![],
        ),
    )
    .unwrap();
    let battery = store_series(&result, "battery");
    // Year 1: 4 GW surplus charged → SoC 2 GWh at the year's last period.
    assert_eq!(battery.charge[0], Power::gigawatts(4.0));
    assert!((gwh(battery.soc[0]) - 2.0).abs() < 1e-12);
    // Year 2 opens with a 10 GW deficit: the store discharges from the
    // carried SoC (no reset to empty or full).
    assert_eq!(battery.discharge[1], Power::gigawatts(4.0));
    assert!((gwh(battery.soc[1]) - 0.0).abs() < 1e-12);
    assert_eq!(result.unserved[1], Power::gigawatts(6.0));
}

/// `initial_soc` is honoured when given (fraction of energy capacity).
#[test]
fn initial_soc_fraction_is_honoured() {
    let s = scenario(
        vec![],
        vec![store_with_soc(
            StorageKind::Battery,
            10.0,
            8.0,
            1.0,
            1,
            0.25,
        )],
        1,
    );
    let result = run(&s, &inputs(&[2.0], &[])).unwrap();
    let battery = store_series(&result, "battery");
    // Started at 2 GWh; discharged 2 GW × 0.5 h = 1 GWh.
    assert!((gwh(battery.soc[0]) - 1.0).abs() < 1e-12);
}

// ---------------------------------------------------------------------
// Validation and rejection paths.
// ---------------------------------------------------------------------

/// D4 rule 2 / acceptance hook: duplicate dispatch_order within a zone
/// is a scenario validation error, surfaced by the engine.
#[test]
fn duplicate_dispatch_order_is_rejected_by_the_engine() {
    let s = scenario(
        vec![thermal("ccgt", 10.0)],
        vec![
            store(StorageKind::Battery, 1.0, 1.0, 1.0, 1),
            store(StorageKind::PumpedHydro, 1.0, 1.0, 1.0, 1),
        ],
        1,
    );
    let err = run(&s, &inputs(&[5.0], &[])).unwrap_err();
    match err {
        GridError::DuplicateDispatchOrder { ref zone, order } => {
            assert_eq!(zone, "GB");
            assert_eq!(order, 1);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

/// DSR pseudo-storage is schema shape only until Q6: the engine rejects
/// it with a clear error naming the Q6 work.
#[test]
fn dsr_stores_are_rejected_at_run_time_pending_q6() {
    let mut dsr = store(StorageKind::Dsr, 2.0, 8.0, 1.0, 1);
    dsr.shift_duration = Some(Duration::hours(4.0));
    dsr.daily_volume_limit = Some(Energy::gigawatt_hours(10.0));
    let s = scenario(vec![thermal("ccgt", 10.0)], vec![dsr], 1);
    let err = run(&s, &inputs(&[5.0], &[])).unwrap_err();
    let msg = err.to_string();
    assert!(
        matches!(err, GridError::UnsupportedFeature { .. }),
        "unexpected error: {err:?}"
    );
    assert!(msg.contains("Q6"), "message should name the Q6 work: {msg}");
    assert!(msg.contains("dsr") || msg.contains("DSR"), "message: {msg}");
}

/// Two stores of the same kind get disambiguated output labels.
#[test]
fn duplicate_store_kinds_get_disambiguated_labels() {
    let s = scenario(
        vec![thermal("ccgt", 10.0)],
        vec![
            store(StorageKind::Battery, 1.0, 1.0, 1.0, 1),
            store(StorageKind::Battery, 1.0, 1.0, 1.0, 2),
        ],
        1,
    );
    let result = run(&s, &inputs(&[5.0], &[])).unwrap();
    let labels: Vec<&str> = result.stores.iter().map(|s| s.label.as_str()).collect();
    assert_eq!(labels, ["battery_1", "battery_2"]);
}

// ---------------------------------------------------------------------
// Policy boundary (D4 design stance): the SystemState passed to the
// policy is a pure function of the current period — no data from
// periods beyond it. Verified behaviourally: two runs whose inputs
// differ only AFTER period t must present identical SystemStates up to
// and including t. (Documented invariant, asserted here — not a
// trait-shape guarantee; the Horizon argument carries calendar metadata
// only.)
// ---------------------------------------------------------------------

/// A recording wrapper around `RuleBased`: captures every SystemState
/// it is shown (flattened to plain numbers), then delegates.
struct RecordingPolicy {
    inner: RuleBased,
    seen: RefCell<Vec<Vec<f64>>>,
}

impl RecordingPolicy {
    fn new() -> Self {
        Self {
            inner: RuleBased,
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl DispatchPolicy for RecordingPolicy {
    fn dispatch(
        &self,
        state: &SystemState<'_>,
        horizon: &grid_core::scenario::Horizon,
    ) -> DispatchDecision {
        let mut row = vec![
            state.instant.unix_micros() as f64,
            state.demand.as_gigawatts(),
            state.must_take.as_gigawatts(),
            state.stack_available.as_gigawatts(),
        ];
        for s in state.stores {
            row.push(s.soc.as_gigawatt_hours());
            row.push(s.power.as_gigawatts());
            row.push(s.energy.as_gigawatt_hours());
        }
        self.seen.borrow_mut().push(row);
        self.inner.dispatch(state, horizon)
    }
}

#[test]
fn policy_sees_no_data_from_future_periods() {
    let fleet = || vec![renewable("onshore_wind", 20.0), thermal("ccgt", 5.0)];
    let stores = || {
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            10.0,
            0.81,
            1,
            0.5,
        )]
    };
    let n = 8;
    let split = 4; // inputs identical up to and including period 4

    let demand_a = [10.0, 12.0, 9.0, 11.0, 10.0, 10.0, 10.0, 10.0];
    let demand_b = [10.0, 12.0, 9.0, 11.0, 10.0, 30.0, 2.0, 40.0];
    let wind_a = [0.7, 0.1, 0.9, 0.2, 0.5, 0.5, 0.5, 0.5];
    let wind_b = [0.7, 0.1, 0.9, 0.2, 0.5, 0.0, 1.0, 0.0];

    let run_recorded = |demand: &[f64], wind: &[f64]| -> Vec<Vec<f64>> {
        let s = scenario(fleet(), stores(), n);
        let policy = RecordingPolicy::new();
        run_with_policy(&s, &inputs(demand, &[("onshore_wind", wind)]), &policy).unwrap();
        policy.seen.into_inner()
    };

    let seen_a = run_recorded(&demand_a, &wind_a);
    let seen_b = run_recorded(&demand_b, &wind_b);
    assert_eq!(seen_a.len(), n);
    assert_eq!(seen_b.len(), n);
    // Identical inputs up to `split` → identical SystemStates up to
    // `split`, bit for bit: nothing the policy is shown depends on any
    // later period.
    for t in 0..=split {
        assert_eq!(
            seen_a[t], seen_b[t],
            "period {t}: SystemState leaked future data"
        );
    }
    // Sanity: after the split the states really do diverge.
    assert_ne!(seen_a[split + 1], seen_b[split + 1]);
}

/// The engine validates policy decisions instead of trusting them: a
/// policy that violates the surplus-only charging rule (or any physical
/// bound) is a structured error, not silent corruption.
struct OvereagerPolicy;

impl DispatchPolicy for OvereagerPolicy {
    fn dispatch(
        &self,
        state: &SystemState<'_>,
        _horizon: &grid_core::scenario::Horizon,
    ) -> DispatchDecision {
        // Charge at full power regardless of surplus.
        DispatchDecision {
            actions: state
                .stores
                .iter()
                .map(|s| StoreAction {
                    charge: s.power,
                    discharge: Power::gigawatts(0.0),
                })
                .collect(),
        }
    }
}

#[test]
fn an_infeasible_policy_decision_is_a_structured_error() {
    let s = scenario(
        vec![thermal("ccgt", 30.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        1,
    );
    // Deficit period (no surplus): charging is infeasible under D4.
    let err = run_with_policy(&s, &inputs(&[10.0], &[]), &OvereagerPolicy).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidDispatchDecision { .. }),
        "unexpected error: {err:?}"
    );
}

/// A policy that discharges at full power regardless of the system
/// state — in a surplus period the energy would have nowhere to go.
struct SurplusDischarger;

impl DispatchPolicy for SurplusDischarger {
    fn dispatch(
        &self,
        state: &SystemState<'_>,
        _horizon: &grid_core::scenario::Horizon,
    ) -> DispatchDecision {
        DispatchDecision {
            actions: state
                .stores
                .iter()
                .map(|s| StoreAction {
                    charge: Power::gigawatts(0.0),
                    discharge: s.power,
                })
                .collect(),
        }
    }
}

/// Reviewer finding (2026-07-02): a policy discharging during a surplus
/// period passed the per-store bounds and its energy silently vanished
/// from the accounting. It must be a structured error — "never silent
/// corruption" (dispatch module docs).
#[test]
fn discharging_during_a_surplus_period_is_a_structured_error() {
    // Wind 12 GW vs demand 10 GW: surplus, no post-stack deficit. The
    // store is full (default), so per-store discharge bounds alone
    // would pass.
    let s = scenario(
        vec![renewable("onshore_wind", 20.0)],
        vec![store(StorageKind::Battery, 5.0, 100.0, 1.0, 1)],
        1,
    );
    let err = run_with_policy(
        &s,
        &inputs(&[10.0], &[("onshore_wind", &[0.6])]),
        &SurplusDischarger,
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidDispatchDecision { .. }),
        "unexpected error: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("surplus"), "message: {msg}");
}

// ---------------------------------------------------------------------
// D12 rule 1 — the policy-contract split. The engine's per-period
// validation runs a physical tier (laws, every policy) and a policy
// tier (the RuleBased conventions, only when the contract declares
// them). A policy that RELAXES surplus-only charging may pre-charge a
// store from the thermal stack in a surplus-free period — rejected
// today at the surplus-only check, accepted after the split — while the
// safety net still rejects the same decision under the rule-based
// contract.
// ---------------------------------------------------------------------

/// Charges every store at full feasible power each period, regardless of
/// surplus — a foresight-style pre-charge from the stack. Its contract
/// is parameterised so one policy exercises both the relaxed and the
/// rule-based case (the only difference between them is the contract).
struct PreCharger {
    obey_surplus_only: bool,
}

impl DispatchPolicy for PreCharger {
    fn dispatch(
        &self,
        state: &SystemState<'_>,
        _horizon: &grid_core::scenario::Horizon,
    ) -> DispatchDecision {
        let dt = Duration::half_hour();
        DispatchDecision {
            actions: state
                .stores
                .iter()
                .map(|s| StoreAction {
                    charge: s.max_charge(dt),
                    discharge: Power::gigawatts(0.0),
                })
                .collect(),
        }
    }

    fn contract(&self) -> PolicyContract {
        PolicyContract {
            charge_from_surplus_only: self.obey_surplus_only,
            discharge_after_stack_only: true,
        }
    }
}

/// GREEN (post-split): a policy that declares
/// `charge_from_surplus_only = false` may pre-charge a store from the
/// thermal stack in a surplus-free period. The physical tier alone
/// governs: the stack serves the deficit PLUS the charge load, energy is
/// conserved, and the store's SoC rises. (RED before the split: the same
/// run errored at the unconditional surplus-only check — reproduced by
/// the safety-net test below with the obligation declared.)
#[test]
fn a_relaxed_contract_may_precharge_a_store_from_the_stack() {
    // Deficit period: demand 10 GW, no renewables, so net = −10 GW. The
    // battery is empty; ccgt has 30 GW, enough for the 10 GW deficit plus
    // a 5 GW charge (η = 1, √η = 1).
    let s = scenario(
        vec![thermal("ccgt", 30.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        1,
    );
    let result = run_with_policy(
        &s,
        &inputs(&[10.0], &[]),
        &PreCharger {
            obey_surplus_only: false,
        },
    )
    .unwrap();
    let battery = store_series(&result, "battery");
    // Charged at full power despite no surplus.
    assert_eq!(battery.charge, vec![Power::gigawatts(5.0)]);
    // SoC rose from empty: ΔSoC = 5 GW × 0.5 h × 1.0 = 2.5 GWh.
    assert!(gwh(battery.soc[0]) > 0.0, "soc {:?}", battery.soc);
    assert!(
        (gwh(battery.soc[0]) - 2.5).abs() < 1e-12,
        "soc {:?}",
        battery.soc
    );
    // The stack served demand + charge (10 + 5 = 15 GW); nothing unserved,
    // nothing curtailed — conservation held (the engine asserts it).
    let ccgt = result
        .thermal
        .iter()
        .find(|t| t.tech.as_str() == "ccgt")
        .unwrap();
    assert_eq!(ccgt.power, vec![Power::gigawatts(15.0)]);
    assert_eq!(result.unserved, vec![Power::gigawatts(0.0)]);
    assert_eq!(result.curtailment, vec![Power::gigawatts(0.0)]);
}

/// The safety net survives: the SAME pre-charge decision under a policy
/// that KEEPS the rule-based contract (`charge_from_surplus_only = true`)
/// is still rejected at the surplus-only check. This is exactly the
/// pre-split behaviour — the policy-tier check fires iff the contract
/// declares the obligation.
#[test]
fn the_surplus_only_safety_net_survives_under_the_rule_based_contract() {
    let s = scenario(
        vec![thermal("ccgt", 30.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        1,
    );
    let err = run_with_policy(
        &s,
        &inputs(&[10.0], &[]),
        &PreCharger {
            obey_surplus_only: true,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidDispatchDecision { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("surplus"), "message: {err}");
}

/// The physical tier holds even with the contract relaxed: a policy that
/// charges BEYOND the surplus in a surplus period would drive curtailment
/// negative — energy conjured from nowhere. With `charge_from_surplus_only
/// = false` the policy-tier surplus check no longer fires, but the
/// physical non-negativity guard does, so the corruption is rejected, not
/// silently recorded. (RED before that guard: the conservation check alone
/// passed it, because curtailment is the identity's residual plug
/// variable.)
#[test]
fn a_relaxed_contract_may_not_conjure_energy_by_overcharging_in_surplus() {
    // Surplus period: 30 GW wind at CF 0.5 = 15 GW must-take vs 5 GW
    // demand → 10 GW surplus. The empty 30 GW battery's max_charge is
    // 30 GW (η = 1), so PreCharger tries to charge 30 GW — 20 GW beyond
    // the surplus, which can only come from nowhere.
    let s = scenario(
        vec![renewable("onshore_wind", 30.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            30.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        1,
    );
    let err = run_with_policy(
        &s,
        &inputs(&[5.0], &[("onshore_wind", &[0.5])]),
        &PreCharger {
            obey_surplus_only: false,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidDispatchDecision { .. }),
        "unexpected error: {err:?}"
    );
    assert!(
        err.to_string().contains("negative curtailment"),
        "message: {err}"
    );
}

/// The physical tier must ALSO hold in a DEFICIT period, not just a
/// surplus one. With `charge_from_surplus_only` relaxed, a policy that
/// charges a store when the thermal stack cannot back the charge would
/// otherwise raise the store's SoC (energy delivered) while the same
/// charge is folded into `unserved` (energy NOT delivered) — the store
/// gains energy the grid never generated. The two derived series stay
/// non-negative and the conservation identity still balances (unserved
/// is its plug variable), so only a dedicated guard catches it: charging
/// and unserved energy are mutually exclusive within a period. (RED
/// before the guard: run_with_policy returned Ok with the battery's SoC
/// risen from empty and unserved > demand.)
#[test]
fn a_relaxed_contract_may_not_conjure_energy_by_charging_in_a_deficit() {
    // Deficit period: demand 10 GW, ccgt 0 GW (the stack supplies
    // nothing), an empty battery. PreCharger tries to charge 5 GW with no
    // supply to back it.
    let s = scenario(
        vec![thermal("ccgt", 0.0)],
        vec![store_with_soc(
            StorageKind::Battery,
            5.0,
            100.0,
            1.0,
            1,
            0.0,
        )],
        1,
    );
    let err = run_with_policy(
        &s,
        &inputs(&[10.0], &[]),
        &PreCharger {
            obey_surplus_only: false,
        },
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidDispatchDecision { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("unserved"), "message: {err}");
}

// ---------------------------------------------------------------------
// Property tests (D4 acceptance hooks).
// ---------------------------------------------------------------------

/// Strategy: a small random system with two stores over two days.
#[allow(clippy::type_complexity)]
fn arb_system() -> impl Strategy<
    Value = (
        Vec<f64>,             // demand GW
        Vec<f64>,             // wind cf
        f64,                  // wind capacity
        f64,                  // ccgt capacity
        (f64, f64, f64, f64), // store 1: power, energy, rte, initial soc
        (f64, f64, f64, f64), // store 2: power, energy, rte, initial soc
    ),
> {
    let n = 96usize;
    let arb_store = || (0.0f64..15.0, 0.0f64..40.0, 0.05f64..=1.0, 0.0f64..=1.0);
    (
        prop::collection::vec(0.0f64..80.0, n),
        prop::collection::vec(0.0f64..=1.0, n),
        0.0f64..90.0,
        0.0f64..30.0,
        arb_store(),
        arb_store(),
    )
}

fn arb_run(
    demand: &[f64],
    wind_cf: &[f64],
    wind_cap: f64,
    ccgt_cap: f64,
    s1: (f64, f64, f64, f64),
    s2: (f64, f64, f64, f64),
) -> RunResult {
    let n = demand.len();
    let s = scenario(
        vec![
            renewable("onshore_wind", wind_cap),
            thermal("ccgt", ccgt_cap),
        ],
        vec![
            store_with_soc(StorageKind::Battery, s1.0, s1.1, s1.2, 1, s1.3),
            store_with_soc(StorageKind::Hydrogen, s2.0, s2.1, s2.2, 2, s2.3),
        ],
        n,
    );
    run(&s, &inputs(demand, &[("onshore_wind", wind_cf)])).unwrap()
}

proptest! {
    /// SoC conservation (D4 acceptance hook): every period,
    /// ΔSoC = charge×√η×Δt − discharge×Δt/√η (to f64 rounding), and
    /// SoC stays within [0, capacity].
    #[test]
    fn soc_conservation_holds_every_period(
        (demand, wind_cf, wind_cap, ccgt_cap, s1, s2) in arb_system()
    ) {
        let result = arb_run(&demand, &wind_cf, wind_cap, ccgt_cap, s1, s2);
        let dt = 0.5;
        for (series, params) in result.stores.iter().zip([s1, s2]) {
            let (_, energy, rte, initial) = params;
            let sqrt_eta = rte.sqrt();
            let mut previous = initial * energy;
            for t in 0..demand.len() {
                let charge = series.charge[t].as_gigawatts();
                let discharge = series.discharge[t].as_gigawatts();
                let expected = previous + charge * sqrt_eta * dt - discharge * dt / sqrt_eta;
                let soc = series.soc[t].as_gigawatt_hours();
                prop_assert!(
                    (soc - expected).abs() <= 1e-9 * energy.max(1.0),
                    "period {t}, {}: soc {soc} != expected {expected}", series.label
                );
                prop_assert!(soc >= 0.0, "period {t}: soc {soc} < 0");
                prop_assert!(
                    soc <= energy * (1.0 + 1e-12) + 1e-12,
                    "period {t}: soc {soc} > capacity {energy}"
                );
                // A store never charges and discharges in the same period.
                prop_assert!(
                    charge == 0.0 || discharge == 0.0,
                    "period {t}: simultaneous charge and discharge"
                );
                previous = soc;
            }
        }
    }

    /// Charge/curtailment exclusivity (D4 acceptance hook): curtailment
    /// only when every store is full or power-saturated.
    #[test]
    fn curtailment_only_when_every_store_is_full_or_power_saturated(
        (demand, wind_cf, wind_cap, ccgt_cap, s1, s2) in arb_system()
    ) {
        let result = arb_run(&demand, &wind_cf, wind_cap, ccgt_cap, s1, s2);
        let tol = 1e-9;
        for t in 0..demand.len() {
            if result.curtailment[t].as_gigawatts() <= tol {
                continue;
            }
            for (series, params) in result.stores.iter().zip([s1, s2]) {
                let (power, energy, _, _) = params;
                let charge = series.charge[t].as_gigawatts();
                let soc = series.soc[t].as_gigawatt_hours();
                let full = soc >= energy - 1e-9 * energy.max(1.0);
                let power_saturated = charge >= power - tol;
                prop_assert!(
                    full || power_saturated,
                    "period {t}, {}: curtailment {} GW while store has headroom \
                     (soc {soc}/{energy} GWh, charging {charge}/{power} GW)",
                    series.label, result.curtailment[t].as_gigawatts()
                );
            }
        }
    }

    /// Discharge/unserved exclusivity (D4 acceptance hook): unserved
    /// energy only when every store is empty or power-saturated.
    #[test]
    fn unserved_only_when_every_store_is_empty_or_power_saturated(
        (demand, wind_cf, wind_cap, ccgt_cap, s1, s2) in arb_system()
    ) {
        let result = arb_run(&demand, &wind_cf, wind_cap, ccgt_cap, s1, s2);
        let tol = 1e-9;
        for t in 0..demand.len() {
            if result.unserved[t].as_gigawatts() <= tol {
                continue;
            }
            for (series, params) in result.stores.iter().zip([s1, s2]) {
                let (power, energy, _, _) = params;
                let discharge = series.discharge[t].as_gigawatts();
                let soc = series.soc[t].as_gigawatt_hours();
                let empty = soc <= 1e-9 * energy.max(1.0);
                let power_saturated = discharge >= power - tol;
                prop_assert!(
                    empty || power_saturated,
                    "period {t}, {}: unserved {} GW while store holds energy \
                     (soc {soc} GWh, discharging {discharge}/{power} GW)",
                    series.label, result.unserved[t].as_gigawatts()
                );
            }
        }
    }

    /// The Stage 1 per-period energy balance, extended for storage:
    /// must-take + thermal + discharge + unserved =
    /// demand + charge + curtailment.
    #[test]
    fn energy_balance_holds_with_storage(
        (demand, wind_cf, wind_cap, ccgt_cap, s1, s2) in arb_system()
    ) {
        let result = arb_run(&demand, &wind_cf, wind_cap, ccgt_cap, s1, s2);
        for t in 0..demand.len() {
            let must_take: f64 = result.renewables.iter().map(|r| r.power[t].as_gigawatts()).sum();
            let thermal: f64 = result.thermal.iter().map(|s| s.power[t].as_gigawatts()).sum();
            let discharge: f64 = result.stores.iter().map(|s| s.discharge[t].as_gigawatts()).sum();
            let charge: f64 = result.stores.iter().map(|s| s.charge[t].as_gigawatts()).sum();
            let lhs = must_take + thermal + discharge + result.unserved[t].as_gigawatts();
            let rhs = result.demand[t].as_gigawatts()
                + charge
                + result.curtailment[t].as_gigawatts();
            prop_assert!(
                (lhs - rhs).abs() <= 1e-9 * rhs.abs().max(1.0),
                "period {t}: supply {lhs} != use {rhs}"
            );
        }
    }
}
