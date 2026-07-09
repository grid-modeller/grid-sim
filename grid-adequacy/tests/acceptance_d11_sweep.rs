//! D11 tier-2 SWEEP acceptance (docs/notes/d11-priced-dispatch.md
//! rules 2 and 4): the Module 1 wind-capacity sweep on `run_multi`
//! over the committed 5-zone scenario, so imports respond
//! **endogenously** to the swept GB fleet — the tier-2 fix of the
//! frozen-imports-under-sweep deviation. The design intended the
//! frozen-imports deviation to move from "bracketed" (Package B,
//! docs/notes/package-b-imports-bracket-review.md) to "resolved, with
//! the bracket as the error bar" — the MEASUREMENT REFUSED that
//! framing: see THE PRE-REGISTERED FINDING BRANCH below.
//!
//! # The central estimate runs the SCARCITY rule (BINDING ruling)
//!
//! Per the d11-engine-review.md §G ruling (do not relitigate): rule 2's
//! purpose is endogeneity, which either flow signal delivers; the
//! priced ladder fails A1/A2a/A2b/A3/A4-BE at the 2024 anchor while the
//! scarcity rule passes all of them, and on 2024 prices the ladder's
//! both-gas flow directions are convention noise
//! (docs/notes/d11-a2a-mismatch-characterisation.md §3/§5). So the
//! tier-2 CENTRAL estimate here runs the committed `scarcity` default,
//! and the priced ladder is pinned as a NAMED SENSITIVITY only.
//!
//! # Conventions
//!
//! - Only the GB wind fleet scales (onshore + offshore, proportionally
//!   from the committed 14.4/14.7 split — the Module 1 convention).
//!   External zones' fleets, demand, traces and budgets stay at their
//!   committed 2024 basis: external fleets are NOT projected.
//! - Metric definitions match the pinned single-zone Module 1 sweep
//!   exactly (`grid_adequacy::sweep::MultiZoneWindPoint` docs), so the
//!   60 GW numbers are comparable against the Package B bracket pins
//!   (grid-cli/tests/regression_imports_bracket_2024.rs).
//! - Self-validation before trusting the new number (the
//!   acceptance_b4_lp discipline): at the 2024 anchor (the unswept
//!   committed fleet) the sweep must reproduce the committed run
//!   bit-for-bit and sit inside the Stage 5 A1 import band.
//!
//! # THE PRE-REGISTERED FINDING BRANCH FIRED (2026-07-05) — framing
//! # pending supervisor/reviewer adjudication
//!
//! The work order's rule-4 assertion was "the central value falls
//! INSIDE the 0.535–0.611 bracket; if it falls outside, that is a
//! FINDING to surface loudly, not a re-pin". MEASURED: the 60 GW
//! delivered-capture central estimate is **0.6975 — ABOVE the whole
//! Package B band** (0.6977 pre-R7-fix; docs/08 R7), and the
//! companion quantities escape the tier-1 bracket on every axis:
//!
//! - delivered capture 0.6975 > frozen 0.6106 (the bracket's top);
//! - curtailment 3.98 TWh (4.01 pre-R7-fix) < export-in-surplus
//!   5.33 TWh (the bracket's floor; frozen read 21.85);
//! - gas 40.67 TWh > frozen 33.21 TWh (tier 1 had NO gas bracket —
//!   degenerate by construction).
//!
//! Direction, pre-registered: this is exactly the Package B review's
//! §4(b)(iii) caveat — all three tier-1 conventions act only in
//! £0-priced surplus periods and lack the export/withdrawal price
//! channel, "so the entire bracket likely UNDERSTATES real high-wind
//! capture". Tier 2 adds that channel (endogenous exports displace
//! external thermal and endogenous imports withdraw when GB is long),
//! keeps GB gas-marginal in 64.2 % of periods at 60 GW (vs 46.5 %
//! single-zone frozen) and mean SMP at £51.24 (vs £37.14) — hence
//! capture ABOVE, curtailment and its £0-flooding BELOW, the tier-1
//! envelope. The tier-1 bracket is therefore NOT an error bar around
//! this central estimate; it is a one-sided (understating) bound on
//! the capture axis. Anchor discipline held before any of this was
//! trusted: the anchor point reproduces the committed Stage 5 record
//! bit-for-bit (+36.03 TWh, 71.70 TWh — the A1 PASS row;
//! +35.94/71.80 pre-R7-fix).
//!
//! Model boundary carried with the finding: the multi-zone capture
//! axis is a NEW quantity (the committed 5-zone scenario deliberately
//! carries no Stage 2 [pricing] block), and at the ANCHOR it reads
//! 0.8950 delivered vs the single-zone reference's 0.9413 — a −0.046
//! anchor wedge from modelled-vs-observed import timing (the A2
//! residual class). That wedge points DOWN, so it cannot explain the
//! 60 GW result sitting ABOVE the bracket — it strengthens the
//! direction, but the magnitude carries scarcity-rule fidelity, not
//! only the price channel.
//!
//! Per the work order these pins are the measured FINDING record (the
//! D11 rule-4 conversion precedent, acceptance_d11_priced_ladder.rs);
//! the "resolved, with the bracket as the error bar" run-report
//! framing is WITHHELD until the finding is adjudicated.
//!
//! Data-gated (fetched/derived, never committed): the 2024 GB pack,
//! the ENTSO-E 2024 pack and the cf-eu pack. FAILS LOUDLY (no
//! `#[ignore]`) with build instructions if any is missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::OnceLock;

use grid_adequacy::{
    Execution, MultiZoneWindPoint, MultiZoneWindSweep, load_multi_zone_inputs, run_multi,
    wind_capacity_sweep_multi,
};
use grid_core::scenario::{FlowSignal, Scenario};
use grid_core::units::{Energy, Power};

const SCENARIO: &str = "scenarios/gb-2024-5zone.toml";

/// Observed 2024 GB net imports, TWh (NESO; the Stage 5 A1 reference
/// and its ±10 % band — the anchor self-validation gate).
const IMPORTS_ACTUAL_TWH: f64 = 33.30;
/// Observed 2024 annual gas generation, TWh (A1 reference, ±5 %).
const GAS_ACTUAL_TWH: f64 = 72.79;

/// The design's Package B band (d11-priced-dispatch.md rule 4, quoted
/// verbatim: "the 0.535–0.611 delivered-capture bracket becomes a
/// central value with the bracket as the band"). The tier-1
/// delivered-capture convention range proper is 0.5514 (export) to
/// 0.6106 (frozen); 0.535 is the (convention-invariant) potential-basis
/// capture — the design's band is the wider envelope of the two pinned
/// Package B capture columns at 60 GW.
const PACKAGE_B_BAND_LO: f64 = 0.535;
const PACKAGE_B_BAND_HI: f64 = 0.611;
/// The Package B 60 GW pins the central estimate is quoted against
/// (regression_imports_bracket_2024.rs, first measured 2026-07-03).
const P60_FROZEN_DELIVERED: f64 = 0.6106059846371504;
const P60_ZERO_DELIVERED: f64 = 0.5952510429390278;
const P60_EXPORT_DELIVERED: f64 = 0.5514484407085398;
const P60_POTENTIAL: f64 = 0.5347799945293277;
const P60_FROZEN_CURTAILMENT_TWH: f64 = 21.845913344574633;
const P60_EXPORT_CURTAILMENT_TWH: f64 = 5.3280243997597205;

// ---------------------------------------------------------------------
// PINNED central estimate at 60 GW (scarcity rule, endogenous imports;
// first measured 2026-07-05). THE FINDING RECORD (module docs): the
// delivered capture sits ABOVE the whole Package B band and the
// curtailment BELOW its export floor — pinned as measured, framing
// withheld pending adjudication.
// ---------------------------------------------------------------------
// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7).
// The movement matches the sweep review's own §B.4 bound exactly
// (curtailment fell 0.0247 ≤ 0.025 TWh; delivered capture fell
// 0.000195 ≤ 0.0002; both AGAINST the above-band finding, which
// stands). Old values: delivered 0.6976839505365661 / potential
// 0.6816365528136847 / curtailment 4.007462807827 / gas
// 40.695234239837 / net imports -6.456015207006 / price-setting
// 64.247495446266 / SMP 51.241226229505 / unserved 0.0.
const PIN_60_DELIVERED_CAPTURE: f64 = 0.6974892015334265;
const PIN_60_POTENTIAL_CAPTURE: f64 = 0.6815454419892222;
const PIN_60_CURTAILMENT_TWH: f64 = 3.982736889304;
const PIN_60_GAS_TWH: f64 = 40.670313638111;
const PIN_60_NET_IMPORTS_TWH: f64 = -6.462858359731;
const PIN_60_GAS_PRICE_SETTING_PCT: f64 = 64.190573770492;
const PIN_60_MEAN_SMP: f64 = 51.200623094653;
const PIN_60_UNSERVED_GWH: f64 = 0.0;

/// The anchor point's endogenous imports (must equal the committed
/// 5-zone run's A1 quantity bit-for-bit). Re-pinned 2026-07-06 with
/// the R7 stall fix alongside the 5-zone digests themselves
/// (regression_5zone.rs) — the pre-fix record was +35.94 TWh /
/// 71.80 TWh (35.935152502942 / 71.797411264632); both A1 gates still
/// pass their bands (36.03 in 33.30 ± 10 %, 71.70 in 72.79 ± 5 %).
const PIN_ANCHOR_NET_IMPORTS_TWH: f64 = 36.025896904243;
const PIN_ANCHOR_GAS_TWH: f64 = 71.700788341640;

// ---------------------------------------------------------------------
// PINNED priced-ladder SENSITIVITY at 60 GW (named sensitivity, NOT
// the headline; first measured 2026-07-05). Caveat carried from the
// characterisation note §3/§5: on 2024 prices the ladder's both-gas
// flow directions are convention noise (a sub-noise, sign-flipping
// carbon wedge decides them), and the ladder fails the Stage 5 A-gates
// at the anchor — these numbers are a dispatch-convention sensitivity,
// not a second central estimate. Note the curtailment is IDENTICAL to
// the scarcity central to 12 dp: curtailment lives in £0-surplus
// periods where the ladder degrades to the scarcity rule BY
// CONSTRUCTION (the graceful-degradation property) — a corroborating
// observation, asserted below.
// ---------------------------------------------------------------------
// Re-pinned 2026-07-06 (R7 stall fix; the recovery regime preserves
// the graceful-degradation guarantee, so the ladder curtailment stays
// identical to the scarcity central to 12 dp). Old values: delivered
// 0.6784115295781239 / potential 0.6628074159582596 / curtailment
// 4.007462807827 / gas 40.030291928817 / net imports -5.601225528878.
const PIN_60_LADDER_DELIVERED_CAPTURE: f64 = 0.6781816865247511;
const PIN_60_LADDER_POTENTIAL_CAPTURE: f64 = 0.6626792734214919;
const PIN_60_LADDER_CURTAILMENT_TWH: f64 = 3.982736889304;
const PIN_60_LADDER_GAS_TWH: f64 = 40.022600573858;
const PIN_60_LADDER_NET_IMPORTS_TWH: f64 = -5.626390888512;

const RATIO_TOL: f64 = 1e-7; // bit-deterministic engine (ADR-5)
const TWH_TOL: f64 = 1e-6;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Loud pack-presence check (the Stage 5 / D11 discipline; full build
/// instructions in acceptance_stage5_2024.rs).
fn require_packs() {
    let root = repo_root();
    for rel in [
        "data/packs/2024/processed/demand_2024.parquet",
        "data/packs/2024/processed/gas_sap_daily_2024.parquet",
        "data/packs/entsoe-2024/processed/load_fr_2024.parquet",
        "data/packs/entsoe-2024/processed/flows_gb_entsoe_2024.parquet",
        "data/packs/cf-eu-1985-2024.sha256",
    ] {
        assert!(
            root.join(rel).exists(),
            "data pack file missing: {rel} — build the 2024 / entsoe-2024 / cf-eu packs \
             first (scripts/fetch-2024, scripts/era5-cf, scripts/fetch-entsoe). These D11 \
             sweep acceptance tests stay RED until the packs exist."
        );
    }
}

fn twh(energy: Energy) -> f64 {
    energy.as_gigawatt_hours() / 1000.0
}

/// The committed GB wind reference capacity (14.7 + 14.4 GW), summed
/// from the loaded scenario so the anchor scale factor is EXACTLY 1.0
/// (x/x = 1.0 in IEEE arithmetic) and the anchor dispatch is
/// bit-identical to the committed run.
fn anchor_wind_capacity(scenario: &Scenario) -> Power {
    let gb = scenario
        .zones
        .iter()
        .find(|z| z.id.as_str() == "GB")
        .unwrap();
    Power::gigawatts(
        gb.fleet
            .iter()
            .filter(|e| matches!(e.technology.as_str(), "offshore_wind" | "onshore_wind"))
            .map(|e| e.capacity_gw.as_gigawatts())
            .sum(),
    )
}

/// The shared CENTRAL-ESTIMATE sweep: [anchor, 60 GW] on the committed
/// scenario (scarcity flow signal), rayon execution.
fn central_sweep() -> &'static MultiZoneWindSweep {
    static SWEEP: OnceLock<MultiZoneWindSweep> = OnceLock::new();
    SWEEP.get_or_init(|| {
        require_packs();
        let root = repo_root();
        let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
        let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
        wind_capacity_sweep_multi(
            &scenario,
            &inputs,
            "GB",
            &[anchor_wind_capacity(&scenario), Power::gigawatts(60.0)],
            Execution::Parallel,
        )
        .unwrap()
    })
}

fn assert_pinned(what: &str, measured: f64, pinned: f64, tolerance: f64) {
    assert!(
        (measured - pinned).abs() <= tolerance,
        "{what}: measured {measured:.12} vs pinned {pinned:.12} (±{tolerance:e}) — if the \
         change is intentional, update this pin and the D11 sweep finding record (module \
         docs; the run-report note once the finding is adjudicated) together"
    );
}

fn report_point(label: &str, point: &MultiZoneWindPoint) {
    eprintln!(
        "{label}: wind {:.1} GW | delivered capture {:?} | potential capture {:?} | \
         curtailment {:.12} TWh | gas {:.12} TWh | net imports {:+.12} TWh | \
         gas price-setting {:.12} % | mean SMP £{:.12}/MWh | unserved {:.12} GWh",
        point.wind_capacity.as_gigawatts(),
        point.wind_capture_ratio_delivered,
        point.wind_capture_ratio,
        twh(point.curtailment),
        twh(point.gas),
        twh(point.net_imports),
        100.0 * point.gas_price_setting_share,
        point.mean_smp.as_pounds_per_megawatt_hour(),
        point.unserved.as_gigawatt_hours(),
    );
}

// ---------------------------------------------------------------------
// Anchor self-validation (the acceptance_b4_lp discipline: reproduce
// the known anchors before trusting the new number).
// ---------------------------------------------------------------------

#[test]
fn anchor_point_reproduces_the_committed_run_and_the_a1_bands() {
    let sweep = central_sweep();
    let anchor = &sweep.points[0];
    report_point("ANCHOR (committed fleet)", anchor);

    // (a) Bit-identity with the committed 5-zone run: the anchor scale
    // factor is exactly 1.0, so the sweep's dispatch at the anchor IS
    // the committed dispatch — the physical metrics must be EQUAL, not
    // merely close.
    let root = repo_root();
    let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let committed = run_multi(&scenario, &inputs).unwrap();
    let gb = committed.zone("GB").unwrap();
    assert!(
        anchor.net_imports == gb.net_imports_energy(),
        "anchor imports differ from the committed run"
    );
    assert!(
        anchor.curtailment == gb.total_curtailment(),
        "anchor curtailment differs from the committed run"
    );
    assert!(
        anchor.unserved == gb.total_unserved(),
        "anchor unserved differs from the committed run"
    );
    let gas = gb.thermal_energy("ccgt").unwrap() + gb.thermal_energy("ocgt").unwrap();
    assert!(
        anchor.gas == gas,
        "anchor gas differs from the committed run"
    );

    // (b) The Stage 5 A1 gates at the anchor (the sweep's
    // self-validation: modelled imports ±10 % of the NESO actual,
    // modelled gas ±5 %). A committed-gate change would already fail
    // acceptance_stage5_2024; this re-asserts it THROUGH the sweep
    // path before any swept number is trusted.
    let imports = twh(anchor.net_imports);
    let error_percent = 100.0 * (imports - IMPORTS_ACTUAL_TWH) / IMPORTS_ACTUAL_TWH;
    assert!(
        error_percent.abs() <= 10.0,
        "anchor imports {imports:.2} TWh vs actual {IMPORTS_ACTUAL_TWH} TWh: \
         {error_percent:+.2} % outside the A1 ±10 % band"
    );
    let gas_twh = twh(anchor.gas);
    let gas_error = 100.0 * (gas_twh - GAS_ACTUAL_TWH) / GAS_ACTUAL_TWH;
    assert!(
        gas_error.abs() <= 5.0,
        "anchor gas {gas_twh:.2} TWh vs actual {GAS_ACTUAL_TWH} TWh: \
         {gas_error:+.2} % outside the A1 ±5 % band"
    );

    // (c) Pinned (the run report's anchor row).
    assert_pinned(
        "anchor net imports (TWh)",
        imports,
        PIN_ANCHOR_NET_IMPORTS_TWH,
        TWH_TOL,
    );
    assert_pinned("anchor gas (TWh)", gas_twh, PIN_ANCHOR_GAS_TWH, TWH_TOL);
}

// ---------------------------------------------------------------------
// The tier-2 central estimate at 60 GW (rule 4): pinned, and quoted
// against the Package B bracket. The design's expectation was "the
// 0.535–0.611 delivered-capture bracket becomes a central value with
// the bracket as the band"; the measurement landed ABOVE the band
// (module docs — the finding, pinned in that shape).
// ---------------------------------------------------------------------

#[test]
fn sixty_gw_central_estimate_is_pinned_and_sits_above_the_package_b_band() {
    let sweep = central_sweep();
    let point = &sweep.points[1];
    assert!((point.wind_capacity.as_gigawatts() - 60.0).abs() < 1e-12);
    report_point(
        "CENTRAL ESTIMATE (60 GW, scarcity, endogenous imports)",
        point,
    );
    eprintln!(
        "vs Package B 60 GW pins: frozen delivered {P60_FROZEN_DELIVERED:.4} / zero \
         {P60_ZERO_DELIVERED:.4} / export {P60_EXPORT_DELIVERED:.4}; potential \
         {P60_POTENTIAL:.4}; curtailment frozen {P60_FROZEN_CURTAILMENT_TWH:.2} / export \
         {P60_EXPORT_CURTAILMENT_TWH:.2} TWh"
    );

    let delivered = point
        .wind_capture_ratio_delivered
        .expect("60 GW wind output cannot be zero");
    // The work order's rule-4 assertion was "central INSIDE the
    // 0.535–0.611 band". The pre-registered miss branch FIRED (module
    // docs): the central sits ABOVE the whole band, in the exact
    // direction the Package B review's §4(b)(iii) missing-export-
    // price-channel caveat predicted. The finding's SHAPE is asserted
    // so the record cannot silently rot — if the central ever moves
    // back inside the band, that is a re-adjudication event, not a
    // green light.
    assert!(
        delivered > PACKAGE_B_BAND_HI,
        "the 60 GW delivered-capture central estimate {delivered:.6} no longer sits ABOVE \
         the Package B band [{PACKAGE_B_BAND_LO}, {PACKAGE_B_BAND_HI}] — the pinned D11 \
         sweep FINDING has changed shape: re-adjudicate (do not silently re-frame)"
    );
    // Companion axes of the same finding: curtailment BELOW the tier-1
    // export floor; gas ABOVE the frozen value tier 1 could not move.
    assert!(
        twh(point.curtailment) < P60_EXPORT_CURTAILMENT_TWH,
        "60 GW curtailment {} TWh no longer below the tier-1 export floor \
         {P60_EXPORT_CURTAILMENT_TWH} TWh — re-adjudicate the finding",
        twh(point.curtailment)
    );

    assert_pinned(
        "60 GW delivered capture",
        delivered,
        PIN_60_DELIVERED_CAPTURE,
        RATIO_TOL,
    );
    assert_pinned(
        "60 GW potential capture",
        point.wind_capture_ratio.unwrap(),
        PIN_60_POTENTIAL_CAPTURE,
        RATIO_TOL,
    );
    assert_pinned(
        "60 GW curtailment (TWh)",
        twh(point.curtailment),
        PIN_60_CURTAILMENT_TWH,
        TWH_TOL,
    );
    assert_pinned("60 GW gas (TWh)", twh(point.gas), PIN_60_GAS_TWH, TWH_TOL);
    assert_pinned(
        "60 GW net imports (TWh)",
        twh(point.net_imports),
        PIN_60_NET_IMPORTS_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "60 GW gas price-setting (%)",
        100.0 * point.gas_price_setting_share,
        PIN_60_GAS_PRICE_SETTING_PCT,
        1e-6,
    );
    assert_pinned(
        "60 GW mean SMP (£/MWh)",
        point.mean_smp.as_pounds_per_megawatt_hour(),
        PIN_60_MEAN_SMP,
        1e-6,
    );
    assert_pinned(
        "60 GW unserved (GWh)",
        point.unserved.as_gigawatt_hours(),
        PIN_60_UNSERVED_GWH,
        1e-6,
    );
}

// ---------------------------------------------------------------------
// The priced-ladder SENSITIVITY at 60 GW: in-memory flow-signal flip
// (the established B4-LP / D11 precedent — the committed scenario file
// stays on the scarcity default). NOT the headline (module docs).
// ---------------------------------------------------------------------

#[test]
fn sixty_gw_priced_ladder_sensitivity_is_pinned() {
    require_packs();
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    scenario.dispatch.flow_signal = FlowSignal::PricedLadder;
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let sweep = wind_capacity_sweep_multi(
        &scenario,
        &inputs,
        "GB",
        &[Power::gigawatts(60.0)],
        Execution::Serial,
    )
    .unwrap();
    let point = &sweep.points[0];
    report_point("SENSITIVITY (60 GW, priced ladder)", point);

    assert_pinned(
        "60 GW ladder delivered capture",
        point.wind_capture_ratio_delivered.unwrap(),
        PIN_60_LADDER_DELIVERED_CAPTURE,
        RATIO_TOL,
    );
    assert_pinned(
        "60 GW ladder potential capture",
        point.wind_capture_ratio.unwrap(),
        PIN_60_LADDER_POTENTIAL_CAPTURE,
        RATIO_TOL,
    );
    assert_pinned(
        "60 GW ladder curtailment (TWh)",
        twh(point.curtailment),
        PIN_60_LADDER_CURTAILMENT_TWH,
        TWH_TOL,
    );
    // Corroboration of the engine's graceful-degradation property:
    // curtailment lives in £0-surplus periods, where the priced ladder
    // degrades to the scarcity rule by construction — so the two
    // signals' 60 GW curtailment agree (pin equality).
    assert!(
        (twh(point.curtailment) - PIN_60_CURTAILMENT_TWH).abs() <= TWH_TOL,
        "ladder curtailment {} TWh diverged from the scarcity central \
         {PIN_60_CURTAILMENT_TWH} TWh — the £0-surplus degradation property no longer \
         covers the curtailment set",
        twh(point.curtailment)
    );
    assert_pinned(
        "60 GW ladder gas (TWh)",
        twh(point.gas),
        PIN_60_LADDER_GAS_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "60 GW ladder net imports (TWh)",
        twh(point.net_imports),
        PIN_60_LADDER_NET_IMPORTS_TWH,
        TWH_TOL,
    );
}

// ---------------------------------------------------------------------
// Determinism at acceptance scale (ADR-5): a second, independent,
// SERIAL sweep is bit-identical to the shared rayon sweep — one
// assertion covering both rerun stability and parallel/serial
// equivalence on the full 5-zone scenario.
// ---------------------------------------------------------------------

#[test]
fn serial_rerun_is_bit_identical_to_the_shared_parallel_sweep() {
    let shared = central_sweep();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let serial = wind_capacity_sweep_multi(
        &scenario,
        &inputs,
        "GB",
        &[anchor_wind_capacity(&scenario), Power::gigawatts(60.0)],
        Execution::Serial,
    )
    .unwrap();
    assert!(
        serial == *shared,
        "serial multi-zone sweep differs from the rayon sweep (ADR-5)"
    );
}
