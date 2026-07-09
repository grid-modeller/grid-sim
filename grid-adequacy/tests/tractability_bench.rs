//! PHASE 1 tractability benchmark for the D12 perfect-foresight LP
//! (package 2b; `docs/notes/d12-perfect-foresight-lp.md` rule 3). NOT a
//! correctness acceptance test — these are `#[ignore]`d timing probes that
//! answer the open horizon/tractability question with evidence: can HiGHS
//! solve a full-horizon 40-year × multi-zone LP in acceptable time and
//! memory, or is a rolling-horizon window needed?
//!
//! Run one size at a time under a memory profiler to get clean per-size
//! peak RSS, e.g.:
//!   cargo test -p grid-adequacy --release --test tractability_bench \
//!       bench_lp_1yr -- --ignored --nocapture
//! then wrap the built binary in `/usr/bin/time -l` for peak RSS.
//!
//! The scenario is a representative 3-zone LINE topology (A—B—C, the
//! wheeling case that D12 rule 3 says matters), each zone with a wind
//! renewable + a hydrogen store, plus a thermal backstop in the southern
//! zone, and two finite links. Traces are synthetic but chronologically
//! varying (seasonal + diurnal) so the LP is a genuine multi-period
//! recharge problem, not a degenerate one.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::time::Instant;

use grid_adequacy::{MultiZoneInputs, RunInputs, ZoneInputs, run_multi_lp};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, LinkSpec, Scenario, StorageKind,
    StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, PerUnit, Power};

const START: &str = "1985-01-01T00:00:00Z";

fn start() -> UtcInstant {
    UtcInstant::parse(START).unwrap()
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

fn hydrogen(power_gw: f64, energy_gwh: f64) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Hydrogen,
        power_gw: Power::gigawatts(power_gw),
        energy_gwh: Energy::gigawatt_hours(energy_gwh),
        round_trip_efficiency: PerUnit::new(0.40),
        dispatch_order: 1,
        initial_soc: Some(PerUnit::new(0.5)),
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
        loss: PerUnit::new(0.02),
    }
}

fn horizon(periods: usize) -> Horizon {
    Horizon {
        start: START.to_owned(),
        end: start().plus_periods(periods as i64 - 1).to_string(),
        weather_years: WeatherYears::All,
    }
}

/// The 3-zone line scenario (A—B—C) sized for the benchmark.
fn bench_scenario(periods: usize) -> Scenario {
    Scenario {
        schema_version: 6,
        name: "tractability-bench".to_owned(),
        description: None,
        horizon: horizon(periods),
        zones: vec![
            // North: wind-rich, small demand, big store.
            zone(
                "A",
                vec![renewable("onshore_wind", 30.0)],
                vec![hydrogen(10.0, 5_000.0)],
            ),
            // Middle: modest wind, middling demand.
            zone(
                "B",
                vec![renewable("onshore_wind", 10.0)],
                vec![hydrogen(10.0, 5_000.0)],
            ),
            // South: demand-heavy, has the thermal backstop + store.
            zone(
                "C",
                vec![renewable("offshore_wind", 8.0), thermal("ccgt", 20.0)],
                vec![hydrogen(10.0, 5_000.0)],
            ),
        ],
        links: vec![link("AB", "A", "B", 10.0), link("BC", "B", "C", 10.0)],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

/// A chronologically-varying CF trace: seasonal (annual) × diurnal, clamped
/// to [0, 1]. `phase` shifts zones apart so wheeling is exercised.
fn cf_series(periods: usize, base: f64, amp: f64, phase: f64) -> Trace<PerUnit> {
    let year = 17_520.0;
    let day = 48.0;
    let values = (0..periods)
        .map(|t| {
            let tf = t as f64;
            let seasonal = (2.0 * PI * tf / year + phase).sin();
            let diurnal = (2.0 * PI * tf / day).sin();
            let v = base + amp * (0.6 * seasonal + 0.4 * diurnal);
            PerUnit::new(v.clamp(0.0, 1.0))
        })
        .collect();
    Trace::from_parts(start(), values).unwrap()
}

/// A demand trace: seasonal (winter-peaking) × diurnal, in GW.
fn demand_series(periods: usize, base: f64, amp: f64) -> Trace<Power> {
    let year = 17_520.0;
    let day = 48.0;
    let values = (0..periods)
        .map(|t| {
            let tf = t as f64;
            // winter-peaking: cosine so t=0 (Jan) is high.
            let seasonal = (2.0 * PI * tf / year).cos();
            let diurnal = -(2.0 * PI * tf / day).cos();
            Power::gigawatts(base + amp * (0.6 * seasonal + 0.4 * diurnal))
        })
        .collect();
    Trace::from_parts(start(), values).unwrap()
}

fn zone_inputs(id: &str, demand: Trace<Power>, cf: Vec<(&str, Trace<PerUnit>)>) -> ZoneInputs {
    ZoneInputs {
        pricing: None,
        id: ZoneId::new(id),
        inputs: RunInputs {
            demand,
            capacity_factors: cf
                .into_iter()
                .map(|(tech, tr)| (TechId::new(tech), tr))
                .collect::<BTreeMap<_, _>>(),
            exogenous: vec![],
            availability: BTreeMap::new(),
            heating: None,
        },
        budgets: BTreeMap::new(),
    }
}

fn bench_inputs(periods: usize) -> MultiZoneInputs {
    MultiZoneInputs {
        zones: vec![
            zone_inputs(
                "A",
                demand_series(periods, 8.0, 3.0),
                vec![("onshore_wind", cf_series(periods, 0.40, 0.35, 0.0))],
            ),
            zone_inputs(
                "B",
                demand_series(periods, 15.0, 5.0),
                vec![("onshore_wind", cf_series(periods, 0.35, 0.30, 1.0))],
            ),
            zone_inputs(
                "C",
                demand_series(periods, 25.0, 8.0),
                vec![("offshore_wind", cf_series(periods, 0.45, 0.30, 2.0))],
            ),
        ],
        link_capabilities: vec![],
    }
}

/// Run the benchmark at `periods` and print the size, build and solve time.
fn run_bench(label: &str, periods: usize) {
    let built = Instant::now();
    let scenario = bench_scenario(periods);
    let inputs = bench_inputs(periods);
    let build_elapsed = built.elapsed();

    let solve_started = Instant::now();
    let result = run_multi_lp(&scenario, &inputs);
    let solve_elapsed = solve_started.elapsed();

    match result {
        Ok(r) => {
            let unserved: f64 = r
                .zones
                .iter()
                .map(|z| z.result.total_unserved().as_gigawatt_hours())
                .sum();
            eprintln!(
                "[{label}] periods={periods} zones=3 links=2 | input-build {build_elapsed:?} | \
                 LP solve {solve_elapsed:?} | total unserved {unserved:.3} GWh"
            );
        }
        Err(e) => {
            eprintln!(
                "[{label}] periods={periods} | input-build {build_elapsed:?} | \
                 LP solve FAILED after {solve_elapsed:?}: {e}"
            );
            panic!("LP solve failed: {e}");
        }
    }
}

#[test]
#[ignore = "PHASE 1 tractability benchmark; run explicitly under /usr/bin/time -l"]
fn bench_lp_1yr() {
    run_bench("1yr", 17_520);
}

#[test]
#[ignore = "PHASE 1 tractability benchmark; run explicitly under /usr/bin/time -l"]
fn bench_lp_5yr() {
    run_bench("5yr", 5 * 17_520);
}

#[test]
#[ignore = "PHASE 1 tractability benchmark; run explicitly under /usr/bin/time -l"]
fn bench_lp_10yr() {
    run_bench("10yr", 10 * 17_520);
}

#[test]
#[ignore = "PHASE 1 tractability benchmark; the full 40-year attempt — may OOM/not finish"]
fn bench_lp_40yr() {
    run_bench("40yr", 701_280);
}
