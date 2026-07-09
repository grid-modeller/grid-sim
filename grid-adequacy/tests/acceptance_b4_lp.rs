//! D12 step 3 — the Scotland–England B4 congestion RE-MEASUREMENT under
//! the perfect-foresight LP (`docs/notes/d12-perfect-foresight-lp.md`
//! rule 4; decision record `docs/notes/d12-mincurtailment-decision.md`;
//! findings `docs/notes/b4-lp-findings.md`). The rule-based dispatch
//! under-wheels northern surplus, so B4 barely binds (~1.96%); the LP
//! wheels it optimally as far as B4/B6 allow, and B4's binding frequency
//! rises to the BAND pinned below — roughly 12–14× the myopic dispatcher,
//! direction southward. This is the QUALITATIVE finding ("the optimiser
//! binds far more than the myopic one"); it is NOT a convergence-to-
//! observed story. Observed ~35.86% is a DAY-AHEAD SCHEDULED position
//! that sits ABOVE the physically-optimal LP (the DA flow exceeds the
//! posted limit in 32.9% of masked periods), so the honest bracket is
//! rule-based << LP band < observed-DA; and the LP band itself still
//! OVERSTATES physical binding (the §3/§6 biases below).
//!
//! # A BAND, not a point (Richard's ruling 2026-07-05)
//!
//! The MinCurtailment objective carries NO link-flow term and both links
//! have loss = 0.0, so the B4 flow is OBJECTIVE-DEGENERATE in periods
//! where the LP is indifferent — shifting spill (or costless thermal
//! backing) across the link changes nothing, and the reported flow is a
//! HiGHS solver-vertex artifact (deterministic per ADR-5, but not
//! model-determined). One class is quantified in-test: binding periods in
//! which a DOWNSTREAM zone (SSCO or RGB) is itself curtailing — the spill
//! could equally have been left north of B4, so the binding there is not
//! physics. The test pins BOTH the point value (the regression pin) and
//! that physics FLOOR; quote the band `[floor, point]` only.
//!
//! # Mask convention — NOT the committed rule-based test's
//!
//! Binding = flow ≥ 99% of the per-period observed DA limit, over the
//! observed flow mask. UNLIKE the committed `acceptance_b4_3zone.rs`
//! convention, this test's mask DROPS the 42 zero-limit sentinel rows
//! from the DENOMINATOR as well as the numerator (the committed
//! convention keeps them in the denominator, excluding them only from the
//! numerator): a sentinel row posts no real limit to bind against, so
//! this test excludes it symmetrically from both flow series being
//! compared. Denominators: 17,235 here vs 17,277 committed; the same
//! 6,181 observed binding periods therefore read 35.86% here vs 35.78%
//! committed. Both circulate as "the" anchor; the difference is pure
//! convention, stated once here and in the findings note.
//!
//! The validated metric (per `docs/notes/three-zone-scottish-data-report.md`
//! design-review item 4) is DIRECTION + BINDING FREQUENCY, NOT the DA
//! net-flow magnitude (15.78 TWh — a wedge budget, no outturn anchor). So
//! this test pins the binding band and compares to observed, never tuning
//! to it (rule 4). The method is self-validated below: it must reproduce
//! observed ~35.86% and rule-based ~1.96% before the LP figure is trusted.
//!
//! Requires the fetched 3-zone packs (never committed); FAILS LOUDLY with
//! fetch instructions if they are absent, like the sibling B4/B6
//! acceptance tests (`require_packs`). This is the SOLE test on the
//! MinCurtailment objective and its pinned band — a silent skip would let
//! a broken objective ride through a packless test run.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{
    MultiZoneRunResult, load_multi_zone_inputs, run_multi, run_multi_lp_min_curtailment,
};
use grid_core::scenario::{Scenario, StorageKind};
use grid_core::time::{HALF_HOUR_MICROS, UtcInstant};
use grid_core::trace::load_sparse_power_trace_mw;
use grid_core::units::Power;

const SCENARIO: &str = "scenarios/gb-2024-3zone.toml";
const B4_TRACE: &str = "data/packs/b6/processed/b4_da_flows_limits.parquet";
const PERIODS_2024: usize = 17_568;

/// Observed B4 binding-frequency anchor under THIS test's convention
/// (sentinel rows dropped from the denominator — module docs): 6,181 /
/// 17,235 = 0.358631. The committed convention reads 0.357759.
const OBSERVED_BINDING: f64 = 0.3586;
/// Committed rule-based B4 binding (acceptance_b4_3zone.rs / three-zone
/// engine review): ~1.95–1.96%. Reproduced here as the method self-check
/// (re-run on the de-duplicated scenario — the dropped stores are inert
/// under rule-based dispatch, so the value is unchanged).
const RULE_BASED_BINDING: f64 = 0.0195;

/// THE BAND (quote `[floor, point]`, never the point alone).
///
/// Point: perfect-foresight LP B4 binding on the DE-DUPLICATED scenario —
/// the pumped-hydro stores dropped from BOTH NSCO (Cruachan+Foyers,
/// 740 MW) and RGB (Dinorwig+Ffestiniog, 2.088 GW), since the exogenous
/// `pumped_storage_net` traces already carry every observed 2024 PS
/// action GB-wide (the earlier 0.3175 pin was REJECTED for the NSCO
/// double-count; 0.2820 carried the same defect class in RGB — the RGB
/// de-dup moved it 0.281984 → 0.281578, −0.04pp). Measured 2026-07-05:
/// 4,853/17,235 = 0.281578. This is a REGRESSION pin, not a validated
/// magnitude: the data report's onshore split (§3, ~+31%/unit north of
/// B4) and offshore-commissioning wedge (§6, ~19%) both bias B4 binding
/// UP, with no outturn cross-anchor. Tolerance ±0.01 covers HiGHS
/// cross-machine floating-point variation at degenerate vertices.
const PIN_B4_LP_BINDING_POINT: f64 = 0.2816;
/// Floor: the point MINUS binding periods in which a downstream zone
/// (SSCO or RGB) is itself curtailing — there the LP was indifferent to
/// where the spill sat, so the binding is a solver-vertex artifact, not
/// physics (module docs). Computed in-test from the same LP result.
/// Measured 2026-07-05: 4,044/17,235 = 0.234639.
const PIN_B4_LP_BINDING_FLOOR: f64 = 0.2346;

/// A downstream zone counts as "curtailing" above this power (GW): far
/// above the LP's solution-dust clamp (1e-9 GW), far below any physical
/// curtailment event.
const CURTAILMENT_TOL_GW: f64 = 1e-6;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Loud pack-presence check with build instructions (the sibling
/// acceptance_b4_3zone.rs pattern): the 2024, cf-gb2/cf-gb3 zonal, and b4
/// packs are fetched-and-built, never committed.
fn require_packs() {
    let root = repo_root();
    for (rel, hint) in [
        (
            "data/packs/2024/processed/demand_2024.parquet",
            "scripts/fetch-2024 (fetch.py, build.py)",
        ),
        (
            "data/packs/cf-gb2/nsco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb3zone.py; verify data/packs/cf-gb3-1985-2024.sha256",
        ),
        (
            "data/packs/cf-gb2/ssco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb3zone.py",
        ),
        (
            B4_TRACE,
            "scripts/fetch-b6 (build.py --three-zone); verify data/packs/b4.sha256",
        ),
    ] {
        let path = root.join(rel);
        assert!(
            path.exists(),
            "data pack file missing: {} — build it first: {hint}",
            path.display()
        );
    }
}

fn align(points: Vec<(UtcInstant, Option<Power>)>) -> Vec<Option<f64>> {
    let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
    let mut out = vec![None; PERIODS_2024];
    for (t, v) in points {
        let offset = t.unix_micros() - start.unix_micros();
        if offset < 0 || offset % HALF_HOUR_MICROS != 0 {
            continue;
        }
        let index = (offset / HALF_HOUR_MICROS) as usize;
        if index < PERIODS_2024 {
            out[index] = v.map(|p| p.as_gigawatts());
        }
    }
    out
}

/// Model southward (from→to) flow GW = −home_end.
fn southward_gw(r: &MultiZoneRunResult, name: &str) -> Vec<f64> {
    r.links
        .iter()
        .find(|l| l.name == name)
        .unwrap()
        .home_end
        .iter()
        .map(|p| -p.as_gigawatts())
        .collect()
}

/// This test's mask: observed flow present AND a real (non-sentinel)
/// limit posted, i.e. limit strictly inside (0.001, 9.0) GW — drops B4's
/// 42 zero-limit sentinel rows from BOTH numerator and denominator (the
/// convention difference in the module docs; B4 carries no high-side
/// no-constraint sentinels).
fn binding_mask(flow: &[Option<f64>], limit: &[Option<f64>]) -> Vec<bool> {
    (0..PERIODS_2024)
        .map(|t| flow[t].is_some() && limit[t].is_some_and(|l| l > 0.001 && l < 9.0))
        .collect()
}

/// The masked periods where `model_gw` reaches 99% of the posted limit.
fn binding_periods(model_gw: &[f64], mask: &[bool], limit: &[Option<f64>]) -> Vec<usize> {
    (0..PERIODS_2024)
        .filter(|&t| mask[t] && t < model_gw.len())
        .filter(|&t| model_gw[t] >= 0.99 * limit[t].unwrap())
        .collect()
}

fn zone_curtailment_gw(r: &MultiZoneRunResult, id: &str) -> Vec<f64> {
    r.zones
        .iter()
        .find(|z| z.id.as_str() == id)
        .unwrap()
        .result
        .curtailment
        .iter()
        .map(|p| p.as_gigawatts())
        .collect()
}

fn total_unserved_twh(r: &MultiZoneRunResult) -> f64 {
    r.zones
        .iter()
        .map(|z| {
            z.result
                .unserved
                .iter()
                .map(|p| p.as_gigawatts() * 0.5)
                .sum::<f64>()
        })
        .sum::<f64>()
        / 1000.0
}

#[test]
fn b4_binding_frequency_lp_resolves_the_under_wheeling() {
    require_packs();

    let path = repo_root().join(B4_TRACE);
    let flow = align(load_sparse_power_trace_mw(&path, "flow_mw").unwrap());
    let limit = align(load_sparse_power_trace_mw(&path, "limit_mw").unwrap());
    let mask = binding_mask(&flow, &limit);
    let mask_count = mask.iter().filter(|&&m| m).count();
    assert_eq!(
        mask_count, 17_235,
        "this test's mask (17,277 committed minus the 42 sentinel rows)"
    );

    // Method self-check: reproduce the observed anchor from the trace.
    let obs_gw: Vec<f64> = (0..PERIODS_2024).map(|t| flow[t].unwrap_or(0.0)).collect();
    let obs = binding_periods(&obs_gw, &mask, &limit).len() as f64 / mask_count as f64;
    assert!(
        (obs - OBSERVED_BINDING).abs() <= 0.01,
        "observed B4 binding {obs:.4} != anchor {OBSERVED_BINDING} — method or pack changed"
    );

    // De-duplicate GB's pumped storage for this LP re-measurement.
    // Cruachan+Foyers (NSCO, 740 MW) and Dinorwig+Ffestiniog (RGB,
    // 2.088 GW) each appear BOTH as the exogenous `pumped_storage_net`
    // trace (the observed 2024 output, split 0.2617 / 0.7383) AND as a
    // dispatchable `pumped_hydro` store. The rule-based engine leaves the
    // stores INERT (so the committed base scenario, its pinned digest and
    // the Cruachan N/S sensitivity all keep them), but the min-curtailment
    // LP WAKES them, so the same physical assets would act twice. We drop
    // the dispatchable pumped-hydro store from EVERY zone here so the LP
    // sees GB's PS exactly once (as history). The base scenario file is
    // deliberately left unchanged — it is the pinned rule-based reference;
    // only this LP measurement de-duplicates.
    let mut scenario = Scenario::load(&repo_root().join(SCENARIO)).unwrap();
    for zone in &mut scenario.zones {
        zone.storage.retain(|s| s.kind != StorageKind::PumpedHydro);
    }
    let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();

    // Method self-check: reproduce the committed rule-based binding. The
    // dropped stores were inert under rule-based, so this is unchanged
    // (~1.96%).
    let rb = run_multi(&scenario, &inputs).unwrap();
    let rb_bind =
        binding_periods(&southward_gw(&rb, "B4"), &mask, &limit).len() as f64 / mask_count as f64;
    assert!(
        (rb_bind - RULE_BASED_BINDING).abs() <= 0.005,
        "rule-based B4 binding {rb_bind:.4} != committed {RULE_BASED_BINDING}"
    );

    // The D12 result: the LP resolves the under-wheeling.
    let lp = run_multi_lp_min_curtailment(&scenario, &inputs).unwrap();
    assert!(
        total_unserved_twh(&lp) < 1e-6,
        "LP dispatch must be feasible (zero unserved)"
    );
    let lp_south = southward_gw(&lp, "B4");
    let bind_t = binding_periods(&lp_south, &mask, &limit);
    let lp_bind = bind_t.len() as f64 / mask_count as f64;

    // The physics floor: exclude binding periods where a downstream zone
    // (SSCO or RGB) is itself curtailing — there the objective was
    // indifferent to where the spill sat (no link-flow term, loss = 0),
    // so the B4 binding is a solver-vertex artifact, not physics.
    let ssco_curt = zone_curtailment_gw(&lp, "SSCO");
    let rgb_curt = zone_curtailment_gw(&lp, "RGB");
    let floor_count = bind_t
        .iter()
        .filter(|&&t| ssco_curt[t] <= CURTAILMENT_TOL_GW && rgb_curt[t] <= CURTAILMENT_TOL_GW)
        .count();
    let lp_floor = floor_count as f64 / mask_count as f64;

    eprintln!(
        "B4 LP band on the de-duplicated (NSCO+RGB) scenario: \
         point {lp_bind:.6} ({}/{mask_count}), floor {lp_floor:.6} ({floor_count}/{mask_count}); \
         rule-based {rb_bind:.6}, observed {obs:.6} (this convention)",
        bind_t.len()
    );

    // THE PINNED BAND (regression guards; quote [floor, point] only).
    assert!(
        (lp_bind - PIN_B4_LP_BINDING_POINT).abs() <= 0.01,
        "LP B4 binding point {lp_bind:.4} moved from pinned {PIN_B4_LP_BINDING_POINT}"
    );
    assert!(
        (lp_floor - PIN_B4_LP_BINDING_FLOOR).abs() <= 0.01,
        "LP B4 binding floor {lp_floor:.4} moved from pinned {PIN_B4_LP_BINDING_FLOOR}"
    );

    // The finding: even the degeneracy-purged FLOOR binds far more than
    // the myopic rule-based dispatch (~12×), direction southward —
    // COMPARED, never tuned (rule 4). NOT a convergence-to-observed claim
    // (see the module docs).
    assert!(
        lp_floor > 10.0 * rb_bind,
        "the LP floor must bind far more than rule-based: floor {lp_floor:.4} vs rb {rb_bind:.4}"
    );
    assert!(
        lp_bind > 10.0 * rb_bind,
        "LP must bind far more than rule-based"
    );
    assert!(
        (lp_bind - OBSERVED_BINDING).abs() < (rb_bind - OBSERVED_BINDING).abs(),
        "LP must be closer to observed than the rule-based dispatch"
    );
}
