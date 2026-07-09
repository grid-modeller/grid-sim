//! Acceptance tests for the D12 rolling-horizon LP wrapper (package 2b;
//! `docs/notes/d12-perfect-foresight-lp.md` rule 3, and the tractability
//! finding `docs/notes/d12-lp-tractability.md`: a single full-40-year LP
//! is not viable — HiGHS crashes past ~5 years — so the horizon is solved
//! in overlapping windows, each window carrying storage state forward.
//!
//! The correctness contract pinned here:
//!
//! 1. **Pass-through identity:** a rolling run whose window spans the whole
//!    horizon is exactly one LP solve, so it must equal `run_multi_lp`
//!    bit-for-bit (the wrapper adds nothing when it does not window).
//! 2. **Determinism:** two rolling runs of identical inputs are identical.
//! 3. **Stitching:** a genuine multi-window run conserves energy every
//!    period and preserves full horizon length, proving the committed
//!    segments are stitched in order.
//! 4. **Carry is load-bearing:** a storage-binding scenario where a later
//!    window's deficit can be served ONLY by state of charge banked in an
//!    earlier, already-committed window — so a broken carry (reset, wrong
//!    index, or off-by-one) strands load. The correct carry reaches zero
//!    unserved and reproduces the whole-horizon SoC trajectory. This is
//!    the discriminating test the earlier three lacked (their fixtures
//!    were not storage-binding across windows, so a wrong carry would
//!    still pass) — the property that will underwrite the step-3 storage
//!    magnitudes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{MultiZoneInputs, RunInputs, ZoneInputs, run_multi_lp, run_multi_lp_rolling};
use grid_core::GridError;
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
        name: "lp-rolling-synthetic".to_owned(),
        description: None,
        horizon: horizon(periods),
        zones,
        links,
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::PerfectForesight,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

fn cf_trace(values: &[f64]) -> Trace<PerUnit> {
    Trace::from_parts(start(), values.iter().map(|&v| PerUnit::new(v)).collect()).unwrap()
}

fn demand_trace(values: &[f64]) -> Trace<Power> {
    Trace::from_parts(
        start(),
        values.iter().map(|&v| Power::gigawatts(v)).collect(),
    )
    .unwrap()
}

fn zone_inputs(id: &str, demand: &[f64], cf: &[(&str, &[f64])]) -> ZoneInputs {
    ZoneInputs {
        pricing: None,
        id: ZoneId::new(id),
        inputs: RunInputs {
            demand: demand_trace(demand),
            capacity_factors: cf
                .iter()
                .map(|(t, v)| (TechId::new(*t), cf_trace(v)))
                .collect::<BTreeMap<_, _>>(),
            exogenous: vec![],
            availability: BTreeMap::new(),
            heating: None,
        },
        budgets: BTreeMap::new(),
    }
}

/// A 2-zone scenario over `n` periods: a windy northern zone (wind +
/// store) joined by one link to a southern zone with a thermal backstop.
/// Chronologically varying so storage genuinely matters across periods.
fn two_zone(n: usize) -> (Scenario, MultiZoneInputs) {
    let s = scenario(
        vec![
            zone(
                "north",
                vec![renewable("onshore_wind", 20.0)],
                vec![battery(5.0, 40.0, 0.81, 0.5)],
            ),
            zone("south", vec![thermal("ccgt", 15.0)], vec![]),
        ],
        vec![link("N-S", "north", "south", 8.0)],
        n,
    );
    // Wind swings high/low; southern demand steady. Storage in the north
    // must time-shift surplus, and the link must wheel it south.
    let wind: Vec<f64> = (0..n).map(|t| if t % 4 < 2 { 0.9 } else { 0.1 }).collect();
    let north_demand = vec![6.0; n];
    let south_demand = vec![10.0; n];
    let inputs = MultiZoneInputs {
        zones: vec![
            zone_inputs("north", &north_demand, &[("onshore_wind", &wind)]),
            zone_inputs("south", &south_demand, &[]),
        ],
        link_capabilities: vec![],
    };
    (s, inputs)
}

/// Flatten a whole multi-zone result to plain numbers for exact
/// comparison (every physical series at both ends of every link and every
/// per-zone series).
fn flatten(r: &grid_adequacy::MultiZoneRunResult) -> Vec<f64> {
    let mut v = Vec::new();
    for z in &r.zones {
        let rr = &z.result;
        v.extend(rr.demand.iter().map(|p| p.as_gigawatts()));
        for ts in &rr.renewables {
            v.extend(ts.power.iter().map(|p| p.as_gigawatts()));
        }
        for ts in &rr.thermal {
            v.extend(ts.power.iter().map(|p| p.as_gigawatts()));
        }
        for ss in &rr.stores {
            v.extend(ss.charge.iter().map(|p| p.as_gigawatts()));
            v.extend(ss.discharge.iter().map(|p| p.as_gigawatts()));
            v.extend(ss.soc.iter().map(|e| e.as_gigawatt_hours()));
        }
        v.extend(rr.curtailment.iter().map(|p| p.as_gigawatts()));
        v.extend(rr.unserved.iter().map(|p| p.as_gigawatts()));
    }
    for l in &r.links {
        v.extend(l.home_end.iter().map(|p| p.as_gigawatts()));
        v.extend(l.away_end.iter().map(|p| p.as_gigawatts()));
    }
    v
}

/// Contract 1: a rolling run whose window spans the whole horizon is a
/// single LP solve, identical to `run_multi_lp`.
#[test]
fn rolling_with_full_window_equals_whole_horizon_lp() {
    let n = 8;
    let (s, inputs) = two_zone(n);
    let whole = run_multi_lp(&s, &inputs).unwrap();
    let rolled = run_multi_lp_rolling(&s, &inputs, n, n).unwrap();
    assert_eq!(
        flatten(&rolled),
        flatten(&whole),
        "a full-window rolling run must equal the whole-horizon LP"
    );
}

/// Contract 2: determinism.
#[test]
fn rolling_is_deterministic() {
    let n = 12;
    let (s, inputs) = two_zone(n);
    let a = run_multi_lp_rolling(&s, &inputs, 6, 3).unwrap();
    let b = run_multi_lp_rolling(&s, &inputs, 6, 3).unwrap();
    assert_eq!(flatten(&a), flatten(&b));
}

/// Contract 3: a genuine multi-window run stitches the committed segments
/// and carries SoC across boundaries — energy is conserved every period
/// in every zone, and feasibility is preserved (this scenario's thermal
/// backstop covers demand, so no window should strand load).
#[test]
fn multi_window_run_conserves_energy_and_stitches_full_length() {
    let n = 12;
    let (s, inputs) = two_zone(n);
    let r = run_multi_lp_rolling(&s, &inputs, 6, 3).unwrap();

    // Full length preserved across the stitched windows.
    for z in &r.zones {
        assert_eq!(z.result.demand.len(), n, "zone {} length", z.id);
        assert_eq!(z.result.unserved.len(), n);
    }

    // Per-zone, per-period energy conservation (must_take from renewables
    // + link net + stack + discharge + unserved == demand + charge +
    // curtailment). Link net at the home end is already in the balance via
    // the link series; we check the zone-local identity the engine emits.
    let dt = Duration::half_hour();
    for z in &r.zones {
        let rr = &z.result;
        for t in 0..n {
            let mut supply = Power::gigawatts(0.0);
            for ts in &rr.renewables {
                supply = supply + ts.power[t];
            }
            for ts in &rr.thermal {
                supply = supply + ts.power[t];
            }
            for ss in &rr.stores {
                supply = supply + ss.discharge[t];
            }
            for ls in &rr.exogenous {
                supply = supply + ls.power[t];
            }
            supply = supply + rr.unserved[t];
            let mut load = rr.demand[t] + rr.curtailment[t];
            for ss in &rr.stores {
                load = load + ss.charge[t];
            }
            let residual = (supply - load).as_gigawatts().abs();
            assert!(
                residual < 1e-6,
                "zone {} period {t}: energy not conserved (residual {residual} GW)",
                z.id
            );
            // SoC bounds hold across the carry.
            for ss in &rr.stores {
                let soc = ss.soc[t].as_gigawatt_hours();
                assert!(
                    (-1e-9..=40.0 + 1e-9).contains(&soc),
                    "zone {} store {} period {t}: SoC {soc} out of bounds",
                    z.id,
                    ss.label
                );
            }
            let _ = dt;
        }
    }
}

/// A misaligned NON-FIRST zone trace must come back as a structured
/// [`GridError::InvalidRunInputs`], never a panic (the library-crate
/// no-panic rule). `run_multi_lp_rolling` computes `periods` from zone 0
/// alone; before the fix its multi-window path sliced every zone's traces
/// (`slice_trace`) BEFORE any cross-zone length validation, so a shorter
/// second-zone demand trace indexed out of bounds and panicked — while the
/// identical input to `run_multi_lp` returns `InvalidRunInputs`. The window
/// here is smaller than the horizon so the slicing path genuinely engages.
#[test]
fn rolling_rejects_misaligned_non_first_zone_trace_without_panicking() {
    let n = 12;
    let (s, mut inputs) = two_zone(n);
    // Zone 1's demand covers only 5 of zone 0's 12 periods.
    inputs.zones[1].inputs.demand = demand_trace(&[10.0; 5]);

    let result = run_multi_lp_rolling(&s, &inputs, 6, 3);
    assert!(
        matches!(result, Err(GridError::InvalidRunInputs { .. })),
        "a misaligned non-first-zone trace must be a structured \
         InvalidRunInputs, got {result:?}"
    );
}

/// Contract 4 (closes reviewer finding B1): the SoC carry across window
/// boundaries is load-bearing. One zone, no thermal, an EMPTY battery. The
/// only surplus is period 0 (wind 12 GW, demand 0 → 6 GWh banked at
/// 12 GW × 0.5 h); periods 1–3 are a deficit (demand 4 GW, no wind, no
/// thermal) totalling exactly the 6 GWh banked; periods 4–5 are idle. The
/// windows are short (commit 1 < window 4 < horizon 6), so the windows
/// that serve periods 2–3 begin AFTER the single charging period and
/// cannot re-derive the charge — the drought is served only if SoC is
/// genuinely carried forward. A broken carry (reset/wrong-index/off-by-one)
/// leaves a later window empty and strands load.
#[test]
fn multi_window_carry_is_load_bearing_across_windows() {
    let s = scenario(
        vec![zone(
            "Z",
            vec![renewable("onshore_wind", 12.0)],
            vec![battery(12.0, 6.0, 1.0, 0.0)],
        )],
        vec![],
        6,
    );
    let inputs = MultiZoneInputs {
        zones: vec![zone_inputs(
            "Z",
            &[0.0, 4.0, 4.0, 4.0, 0.0, 0.0],
            &[("onshore_wind", &[1.0, 0.0, 0.0, 0.0, 0.0, 0.0])],
        )],
        link_capabilities: vec![],
    };

    // Genuine windowing: commit 1 < window 4 < horizon 6.
    let rolled = run_multi_lp_rolling(&s, &inputs, 4, 1).unwrap();
    let unserved = rolled.zones[0].result.total_unserved().as_gigawatt_hours();
    assert!(
        unserved <= 1e-9,
        "the drought is served only by SoC banked in period 0 and carried \
         across windows; a broken carry strands load. got {unserved} GWh"
    );

    // The carried trajectory reproduces the whole-horizon optimum exactly:
    // bank 6 GWh in period 0, draw down to empty over the drought →
    // SoC = [6, 4, 2, 0, 0, 0].
    let whole = run_multi_lp(&s, &inputs).unwrap();
    let rolled_soc: Vec<f64> = rolled.zones[0].result.stores[0]
        .soc
        .iter()
        .map(|e| e.as_gigawatt_hours())
        .collect();
    let whole_soc: Vec<f64> = whole.zones[0].result.stores[0]
        .soc
        .iter()
        .map(|e| e.as_gigawatt_hours())
        .collect();
    for (t, (r, w)) in rolled_soc.iter().zip(&whole_soc).enumerate() {
        assert!(
            (r - w).abs() < 1e-9,
            "period {t}: rolling SoC {r} != whole-horizon SoC {w}"
        );
    }
    assert!(
        (rolled_soc[0] - 6.0).abs() < 1e-9,
        "expected 6 GWh banked in period 0, got {}",
        rolled_soc[0]
    );
    assert!(
        rolled_soc[3].abs() < 1e-9,
        "expected the store empty after the drought, got {}",
        rolled_soc[3]
    );
}
