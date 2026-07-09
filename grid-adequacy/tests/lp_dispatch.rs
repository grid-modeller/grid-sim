//! Acceptance tests for the D12 perfect-foresight LP dispatch CORE
//! (package 2a; `docs/notes/d12-perfect-foresight-lp.md` rules 2/3/5).
//!
//! The LP is a whole-horizon linear program used as the bisection
//! FEASIBILITY ORACLE (rule 2): its objective is min total unserved, so a
//! feasible zero-unserved dispatch exists iff the minimised unserved is
//! ~0. These tests prove the core on small, hand-checkable scenarios:
//!
//! 1. single-zone storage feasibility with a hand-computed minimum store
//!    energy (the LP hits zero unserved at/above it, >0 below);
//! 2. the dispositive three-zone WHEELING case (A—B—C line) — the
//!    rule-based single-pass equal-depth flow under-wheels and strands
//!    C's deficit, while the LP wheels A→B→C in one period to zero
//!    unserved (the quotable proof the LP resolves the B4 finding);
//! 3. determinism (two runs bit-identical); and
//! 4. the per-period energy-conservation identity in the LP result.
//!
//! Where a test pins a number it is an exact-value assertion (engine
//! mechanics, not a published GB result).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{MultiZoneInputs, RunInputs, ZoneInputs, run_multi, run_multi_lp};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, LinkSpec, Scenario, StorageKind,
    StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, PerUnit, Power};

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
        name: "synthetic-lp".to_owned(),
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

fn zone_inputs(id: &str, demand_gw: &[f64], cf: &[(&str, &[f64])]) -> ZoneInputs {
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
        budgets: BTreeMap::new(),
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

/// The physical energy-conservation identity the engine's physical tier
/// enforces (`multizone.rs`), restated over a `RunResult` (link flows are
/// folded into the zone's exogenous series):
/// `renewables + exogenous + thermal + discharge + unserved
///    == demand + charge + curtailment`, every period. Non-negative
/// curtailment and unserved (step-1 physical law).
fn assert_conservation(result: &grid_adequacy::MultiZoneRunResult) {
    for zr in &result.zones {
        let r = &zr.result;
        for t in 0..r.periods() {
            let supply: f64 = r
                .renewables
                .iter()
                .chain(&r.thermal)
                .map(|s| s.power[t].as_gigawatts())
                .sum::<f64>()
                + r.exogenous
                    .iter()
                    .map(|s| s.power[t].as_gigawatts())
                    .sum::<f64>()
                + r.stores
                    .iter()
                    .map(|s| s.discharge[t].as_gigawatts())
                    .sum::<f64>();
            let uses = r.demand[t].as_gigawatts() - r.unserved[t].as_gigawatts()
                + r.stores
                    .iter()
                    .map(|s| s.charge[t].as_gigawatts())
                    .sum::<f64>()
                + r.curtailment[t].as_gigawatts();
            assert!(
                (supply - uses).abs() < 1e-6,
                "zone {} period {t}: supply {supply} != uses {uses}",
                zr.id
            );
            assert!(
                r.curtailment[t].as_gigawatts() >= -1e-9,
                "zone {} period {t}: negative curtailment",
                zr.id
            );
            assert!(
                r.unserved[t].as_gigawatts() >= -1e-9,
                "zone {} period {t}: negative unserved",
                zr.id
            );
            // No simultaneous charge and discharge in the emitted result.
            for s in &r.stores {
                assert!(
                    s.charge[t].as_gigawatts() < 1e-9 || s.discharge[t].as_gigawatts() < 1e-9,
                    "zone {} period {t}: store {} charges and discharges at once",
                    zr.id,
                    s.label
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// Test 1 — hand-checkable single-zone storage feasibility.
//
// One zone, no thermal, wind must-take [4, 0] GW against demand [2, 2],
// one lossless battery (rte = 1, initial SoC empty), power rating 10 GW
// (non-binding). dt = 0.5 h.
//
//   P0: surplus 2 GW → the store can bank at most 2 GW × 0.5 h = 1.0 GWh
//       (also headroom-limited by the energy capacity).
//   P1: deficit 2 GW → the store must deliver 2 GW × 0.5 h = 1.0 GWh to
//       reach zero unserved.
//
// So the minimum store ENERGY for zero unserved is exactly 1.0 GWh.
// Below it the store banks only `energy` GWh in P0 and P1 is short:
//   at energy = 0.9 GWh, P1 discharge = 0.9 GWh (1.8 GW) leaves
//   0.2 GW × 0.5 h = 0.1 GWh unserved.
// ---------------------------------------------------------------------

fn feasibility_scenario(energy_gwh: f64) -> (Scenario, MultiZoneInputs) {
    let s = scenario(
        vec![zone(
            "Z",
            vec![renewable("onshore_wind", 4.0)],
            vec![battery(10.0, energy_gwh, 1.0, 0.0)],
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

#[test]
fn lp_storage_feasibility_matches_the_hand_computed_minimum() {
    // At the minimum (1.0 GWh): zero unserved.
    let (s, inputs) = feasibility_scenario(1.0);
    let at_min = run_multi_lp(&s, &inputs).unwrap();
    assert!(
        at_min.zones[0].result.total_unserved().as_gigawatt_hours() <= 1e-9,
        "at the 1.0 GWh minimum the LP must reach zero unserved, got {:?}",
        at_min.zones[0].result.total_unserved()
    );
    assert_conservation(&at_min);

    // Above the minimum (2.0 GWh): still zero unserved.
    let (s, inputs) = feasibility_scenario(2.0);
    let above = run_multi_lp(&s, &inputs).unwrap();
    assert!(above.zones[0].result.total_unserved().as_gigawatt_hours() <= 1e-9);

    // Below the minimum (0.9 GWh): exactly 0.1 GWh unserved (hand calc).
    let (s, inputs) = feasibility_scenario(0.9);
    let below = run_multi_lp(&s, &inputs).unwrap();
    let unserved = below.zones[0].result.total_unserved().as_gigawatt_hours();
    assert!(
        (unserved - 0.1).abs() < 1e-9,
        "below the minimum the LP is short by exactly 0.1 GWh, got {unserved}"
    );
    assert_conservation(&below);
}

// ---------------------------------------------------------------------
// √η SoC convention: where the dispatch is FORCED to be identical
// (charging is the only way to reduce unserved, so the LP charges fully —
// exactly what the rule-based greedy policy does), the LP's storage SoC
// series must match the rule-based engine's `StoreState::apply` exactly.
// This pins the √η per-leg split (charge adds ·dt·√η, discharge removes
// ·dt/√η) to the engine convention (D12 rule 4 — like-for-like).
//
//   rte = 0.81 (√η = 0.9), initial SoC empty, no thermal.
//   P0: wind 4, demand 2 → surplus 2, the store banks 2 × 0.5 × 0.9 =
//       0.9 GWh. P1: wind 0, demand 4 → deficit 4; the store delivers
//       0.9 × 0.9 = 0.81 GWh (1.62 GW), 2.38 GW unserved. Both engines
//       must produce SoC = [0.9, 0.0].
// ---------------------------------------------------------------------

#[test]
fn lp_soc_convention_matches_rule_based_when_dispatch_is_forced() {
    let s = scenario(
        vec![zone(
            "Z",
            vec![renewable("onshore_wind", 4.0)],
            vec![battery(10.0, 10.0, 0.81, 0.0)],
        )],
        vec![],
        2,
    );
    let inputs = multi(vec![zone_inputs(
        "Z",
        &[2.0, 4.0],
        &[("onshore_wind", &[1.0, 0.0])],
    )]);
    let rule = run_multi(&s, &inputs).unwrap();
    let lp = run_multi_lp(&s, &inputs).unwrap();

    let rule_soc = &rule.zones[0].result.stores[0].soc;
    let lp_soc = &lp.zones[0].result.stores[0].soc;
    for (t, (a, b)) in rule_soc.iter().zip(lp_soc).enumerate() {
        assert!(
            (a.as_gigawatt_hours() - b.as_gigawatt_hours()).abs() < 1e-9,
            "period {t}: rule-based SoC {a:?} != LP SoC {b:?}"
        );
    }
    // The hand-computed SoC path.
    assert!((lp_soc[0].as_gigawatt_hours() - 0.9).abs() < 1e-9);
    assert!(lp_soc[1].as_gigawatt_hours().abs() < 1e-9);
    assert_conservation(&lp);
}

// ---------------------------------------------------------------------
// Test 2 — the dispositive three-zone WHEELING case (the B4 point).
//
// LINE topology A—B—C: north zone A has surplus wind, south zone C has a
// deficit reachable ONLY by routing through middle zone B (links A–B and
// B–C; NO direct A–C). Finite caps (3 GW each) still admit the wheel.
//
//   A: wind 5 GW at CF 1, demand 1 → surplus 4 GW.
//   B: no fleet, demand 0.
//   C: no fleet, demand 3 → deficit 3 GW.
//
// The LP wheels 3 GW A→B→C in one period: C fully served, A curtails 1.
// The rule-based single-pass equal-depth flow CANNOT: border (A,B)
// equalises at 2 GW into B, then border (B,C) can pass only B's 2 GW on
// to C — C is left 1 GW short. The comparison is the quotable proof.
// ---------------------------------------------------------------------

fn wheeling_scenario() -> (Scenario, MultiZoneInputs) {
    let s = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 5.0)], vec![]),
            zone("B", vec![], vec![]),
            zone("C", vec![], vec![]),
        ],
        vec![link("AB", "A", "B", 3.0), link("BC", "B", "C", 3.0)],
        1,
    );
    let inputs = multi(vec![
        zone_inputs("A", &[1.0], &[("onshore_wind", &[1.0])]),
        zone_inputs("B", &[0.0], &[]),
        zone_inputs("C", &[3.0], &[]),
    ]);
    (s, inputs)
}

#[test]
fn rule_based_single_pass_under_wheels_and_strands_the_southern_deficit() {
    let (s, inputs) = wheeling_scenario();
    let rule = run_multi(&s, &inputs).unwrap();
    // The whole point: the rule-based flow leaves C unserved (it cannot
    // wheel A→B→C in one pass).
    let total_unserved: f64 = rule
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum();
    assert!(
        total_unserved > 1e-6,
        "the rule-based single-pass flow should strand the southern deficit, \
         got {total_unserved} GWh unserved"
    );
    let c_unserved = rule.zone("C").unwrap().total_unserved().as_gigawatt_hours();
    assert!(
        c_unserved > 1e-6,
        "C left unserved by rule-based: {c_unserved}"
    );
}

#[test]
fn lp_wheels_north_through_middle_to_south_for_zero_unserved() {
    let (s, inputs) = wheeling_scenario();
    let lp = run_multi_lp(&s, &inputs).unwrap();

    let total_unserved: f64 = lp
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum();
    assert!(
        total_unserved <= 1e-9,
        "the LP must wheel A→B→C to zero unserved, got {total_unserved} GWh"
    );

    // The wheel: 3 GW sent A→B and 3 GW sent B→C (sending-end power).
    let ab = lp.links.iter().find(|l| l.name == "AB").unwrap();
    let bc = lp.links.iter().find(|l| l.name == "BC").unwrap();
    // home_end at A is negative (A exports); away_end at C positive.
    assert!(
        (gw(&ab.away_end)[0] - 3.0).abs() < 1e-9,
        "A→B: {:?}",
        gw(&ab.away_end)
    );
    assert!(
        (gw(&bc.away_end)[0] - 3.0).abs() < 1e-9,
        "B→C: {:?}",
        gw(&bc.away_end)
    );

    // A curtails its remaining 1 GW of surplus (4 surplus − 3 wheeled).
    let a = lp.zone("A").unwrap();
    assert!(
        (a.total_curtailment().as_gigawatt_hours() - 0.5).abs() < 1e-9,
        "A curtailment: {:?}",
        a.total_curtailment()
    );
    // C fully served.
    assert!(lp.zone("C").unwrap().total_unserved().as_gigawatt_hours() <= 1e-9);

    assert_conservation(&lp);
}

// ---------------------------------------------------------------------
// Test 3 — determinism (ADR-5): two runs of identical input are
// bit-identical.
// ---------------------------------------------------------------------

#[test]
fn lp_runs_are_deterministic() {
    let (s, inputs) = wheeling_scenario();
    let first = run_multi_lp(&s, &inputs).unwrap();
    let second = run_multi_lp(&s, &inputs).unwrap();
    assert!(first == second, "LP runs differ between reruns");

    // And the storage scenario too (exercises the SoC path).
    let (s, inputs) = feasibility_scenario(1.0);
    let first = run_multi_lp(&s, &inputs).unwrap();
    let second = run_multi_lp(&s, &inputs).unwrap();
    assert!(first == second, "LP storage runs differ between reruns");
}

// ---------------------------------------------------------------------
// Test 4 — the conservation identity is exercised inside tests 1 and 2
// (via `assert_conservation`), and once more here on a mixed scenario
// with both a lossy link and storage, so the identity spans every term.
// ---------------------------------------------------------------------

#[test]
fn lp_result_satisfies_the_conservation_identity() {
    let mut s = scenario(
        vec![
            zone(
                "A",
                vec![renewable("onshore_wind", 6.0), thermal("ccgt", 4.0)],
                vec![battery(2.0, 4.0, 0.81, 1.0)],
            ),
            zone("B", vec![thermal("ccgt", 5.0)], vec![]),
        ],
        vec![link("AB", "A", "B", 2.0)],
        4,
    );
    s.links[0].loss = PerUnit::new(0.05);
    let inputs = multi(vec![
        zone_inputs(
            "A",
            &[3.0, 5.0, 2.0, 6.0],
            &[("onshore_wind", &[0.9, 0.1, 1.0, 0.2])],
        ),
        zone_inputs("B", &[4.0, 3.0, 5.0, 2.0], &[]),
    ]);
    let lp = run_multi_lp(&s, &inputs).unwrap();
    assert_conservation(&lp);

    // Per-link loss identity: received = sent × (1 − loss).
    let ab = &lp.links[0];
    for t in 0..4 {
        let home = ab.home_end[t].as_gigawatts();
        let away = ab.away_end[t].as_gigawatts();
        let (sent, received) = if home < 0.0 {
            (-home, away)
        } else {
            (-away, home)
        };
        assert!(sent >= -1e-9);
        assert!(
            (received - sent * 0.95).abs() < 1e-9,
            "period {t}: sent {sent} received {received}"
        );
    }
}

// Duration import kept meaningful (the hand calcs are stated in dt units).
const _: Duration = Duration::half_hour();
