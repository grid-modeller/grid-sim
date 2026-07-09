//! D13 rule 4 / adjudication B(i): the **loss-as-waste** term of the
//! MinCurtailment objective, proven on hand-computable lossy fixtures
//! (the red-first condition (d) of the adopted design,
//! `docs/notes/d13-composed-boundary-trade.md`).
//!
//! # The bias this term removes
//!
//! The MinCurtailment objective prices curtailment at 1 and (before
//! D13) carried no link-flow or loss term. Exports leave a zone at full
//! sending-end power while the receiving end gets `sent × (1 − loss)`,
//! so shipping surplus `x` into a neighbour that is ITSELF curtailing
//! changed the objective by `−x·loss`: the LP **strictly preferred
//! burning surplus as link loss** over counting it as curtailment — a
//! loss-disposal channel that inflates gross trade and shaves measured
//! curtailment on any lossy-link scenario.
//!
//! # The adopted fix (D13, four conditions)
//!
//! Link-loss energy joins the MinCurtailment waste terms at weight 1
//! (`loss × sent`, both directions) — the exact analogue of the
//! committed storage round-trip-loss term (d12-mincurtailment term 3:
//! disposal costs exactly what curtailment costs; genuine use still
//! nets a gain). Conditions, all held here or by the committed suite:
//!
//! - (a) **MinCurtailment only** — the MinUnserved oracle objective is
//!   byte-for-byte unchanged (its committed tests and digests gate it);
//! - (b) the term is **structurally skipped at `loss == 0.0`**, so the
//!   committed lossless family's objective is byte-identical
//!   (`acceptance_b4_lp` re-run green is the gate);
//! - (c) unmoved-pins gate: the full suite re-run green;
//! - (d) THIS FILE — red-first on a hand-computable lossy fixture.
//!
//! # Precision (adjudication B(i), mandatory)
//!
//! The term converts the strict `−x·loss` preference into an exact
//! **indifference**: the export-into-curtailing-neighbour class joins
//! the ordinary degeneracy classes and stays under the [floor, point]
//! band discipline. The disposal fixture's zero-flow outcome below is
//! the solver's deterministic resolution of that indifference from a
//! cold start (zero reduced cost — simplex has no reason to pivot the
//! flow in), NOT a model-determined guarantee; what IS model-determined
//! is that disposal no longer pays (without the term the strictly
//! optimal vertex was flow-at-cap, measured curtailment strictly below
//! the physical waste).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{MultiZoneInputs, RunInputs, ZoneInputs, run_multi_lp_min_curtailment};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, LinkSpec, Scenario, TechId,
    WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{PerUnit, Power};

const START: &str = "2024-01-01T00:00:00Z";

fn start() -> UtcInstant {
    UtcInstant::parse(START).unwrap()
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
        capacity_factor_trace: Some(format!("synthetic/{tech}.parquet").into()),
        ..thermal(tech, capacity_gw)
    }
}

fn zone(id: &str, fleet: Vec<FleetEntry>) -> ZoneSpec {
    ZoneSpec {
        pricing: None,
        id: ZoneId::new(id),
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
        storage: vec![],
    }
}

fn lossy_link(from: &str, to: &str, cap: f64, loss: f64) -> LinkSpec {
    LinkSpec {
        name: Some("L".to_owned()),
        from: ZoneId::new(from),
        to: ZoneId::new(to),
        capacity_gw: Power::gigawatts(cap),
        reverse_capacity_gw: None,
        capability_trace: None,
        availability: PerUnit::new(1.0),
        loss: PerUnit::new(loss),
    }
}

fn scenario(zones: Vec<ZoneSpec>, links: Vec<LinkSpec>, periods: usize) -> Scenario {
    Scenario {
        schema_version: 8,
        name: "synthetic-lp-loss-waste".to_owned(),
        description: None,
        horizon: Horizon {
            start: START.to_owned(),
            end: start().plus_periods(periods as i64 - 1).to_string(),
            weather_years: WeatherYears::Years(vec![2024]),
        },
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

fn zone_inputs(id: &str, demand_gw: &[f64], cf: &[(&str, &[f64])]) -> ZoneInputs {
    ZoneInputs {
        pricing: None,
        id: ZoneId::new(id),
        inputs: RunInputs {
            demand: Trace::from_parts(
                start(),
                demand_gw.iter().map(|&v| Power::gigawatts(v)).collect(),
            )
            .unwrap(),
            capacity_factors: cf
                .iter()
                .map(|(tech, values)| {
                    (
                        TechId::new(*tech),
                        Trace::from_parts(
                            start(),
                            values.iter().map(|&v| PerUnit::new(v)).collect::<Vec<_>>(),
                        )
                        .unwrap(),
                    )
                })
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

fn total_curtailment_gwh(result: &grid_adequacy::MultiZoneRunResult) -> f64 {
    result
        .zones
        .iter()
        .map(|z| z.result.total_curtailment().as_gigawatt_hours())
        .sum()
}

// ---------------------------------------------------------------------
// Fixture 1 — DISPOSAL: shipping into a curtailing neighbour must not
// pay. One period, dt = 0.5 h.
//
//   A: wind 4 GW (cf 1), demand 1 → surplus 3 GW (curtailing).
//   B: wind 3 GW (cf 1), demand 1 → surplus 2 GW (curtailing).
//   Link A→B: 2 GW, loss 0.1.
//
// Physical waste (generation − served demand) = 7 − 2 = 5 GW
// → 2.5 GWh. WITHOUT the loss-as-waste term, shipping x costs the
// objective −0.1·x (A curtails x less, B curtails 0.9·x more), so the
// strictly optimal vertex ships the full 2 GW and reports measured
// curtailment (3−2) + (2+1.8) = 4.8 GW → 2.4 GWh — 0.1 GWh of surplus
// silently burned as link loss instead of counted as curtailment.
// WITH the term the disposal nets exactly zero, and the measured
// curtailment equals the physical waste.
// ---------------------------------------------------------------------

#[test]
fn min_curtailment_does_not_prefer_burning_surplus_as_link_loss() {
    let s = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 4.0)]),
            zone("B", vec![renewable("onshore_wind", 3.0)]),
        ],
        vec![lossy_link("A", "B", 2.0, 0.1)],
        1,
    );
    let inputs = multi(vec![
        zone_inputs("A", &[1.0], &[("onshore_wind", &[1.0])]),
        zone_inputs("B", &[1.0], &[("onshore_wind", &[1.0])]),
    ]);
    let lp = run_multi_lp_min_curtailment(&s, &inputs).unwrap();

    let curtailment = total_curtailment_gwh(&lp);
    let sent_gw = -lp.links[0].home_end[0].as_gigawatts();
    eprintln!(
        "disposal fixture: measured curtailment {curtailment} GWh (physical waste 2.5), \
         A→B sending-end flow {sent_gw} GW"
    );

    // THE BIAS ASSERTION (red before the loss-as-waste term existed):
    // measured curtailment must equal the physical waste — the LP must
    // not report less curtailment by burning the difference in the
    // link. Without the term this reads 2.4 GWh (flow forced to cap).
    assert!(
        (curtailment - 2.5).abs() < 1e-9,
        "the LP shaved measured curtailment below the physical waste by loss-disposal: \
         {curtailment} GWh vs 2.5 GWh physical (flow {sent_gw} GW)"
    );
    // The deterministic resolution of the now-indifferent disposal
    // class from a cold start is zero flow (module docs: a solver
    // characterisation under the [floor, point] band discipline, not a
    // model-determined guarantee — but its move would signal a solver
    // or objective change worth investigating).
    assert!(
        sent_gw.abs() < 1e-9,
        "disposal flow expected zero at the cold-start vertex, got {sent_gw} GW"
    );
}

// ---------------------------------------------------------------------
// Fixture 2 — GENUINE USE: wheeling that serves load (displacing
// thermal) must STILL be strictly favoured with the term. One period.
//
//   A: wind 4 GW (cf 1), demand 1 → surplus 3 GW.
//   B: no wind, demand 2, ccgt 5 GW.
//   Link A→B: 2 GW, loss 0.1.
//
// Shipping x reduces A's curtailment by x while B's received
// 0.9·x displaces thermal (curtailment unchanged at B), so the
// objective falls by x·(1 − loss) — strictly favoured, term or no
// term. Optimal: ship the full 2 GW; A curtails 1 GW (0.5 GWh);
// B receives 1.8 GW and runs 0.2 GW of ccgt.
// ---------------------------------------------------------------------

#[test]
fn min_curtailment_still_wheels_genuinely_useful_surplus_over_a_lossy_link() {
    let s = scenario(
        vec![
            zone("A", vec![renewable("onshore_wind", 4.0)]),
            zone("B", vec![thermal("ccgt", 5.0)]),
        ],
        vec![lossy_link("A", "B", 2.0, 0.1)],
        1,
    );
    let inputs = multi(vec![
        zone_inputs("A", &[1.0], &[("onshore_wind", &[1.0])]),
        zone_inputs("B", &[2.0], &[]),
    ]);
    let lp = run_multi_lp_min_curtailment(&s, &inputs).unwrap();

    let sent_gw = -lp.links[0].home_end[0].as_gigawatts();
    let received_gw = lp.links[0].away_end[0].as_gigawatts();
    let a_curt = lp
        .zone("A")
        .unwrap()
        .total_curtailment()
        .as_gigawatt_hours();
    let b_ccgt = lp.zone("B").unwrap().thermal[0].power[0].as_gigawatts();
    eprintln!(
        "genuine-use fixture: sent {sent_gw} GW, received {received_gw} GW, \
         A curtailment {a_curt} GWh, B ccgt {b_ccgt} GW"
    );

    assert!(
        (sent_gw - 2.0).abs() < 1e-9,
        "genuine thermal-displacing wheeling must run at the 2 GW cap, got {sent_gw}"
    );
    assert!((received_gw - 1.8).abs() < 1e-9, "received = sent × 0.9");
    assert!(
        (a_curt - 0.5).abs() < 1e-9,
        "A curtails exactly its unwheelable 1 GW (0.5 GWh), got {a_curt}"
    );
    assert!(
        (b_ccgt - 0.2).abs() < 1e-9,
        "B's thermal covers exactly the loss-shrunk residual 0.2 GW, got {b_ccgt}"
    );
}
