//! Three-zone (N-Scotland / S-Scotland / E+W) B4 + B6 boundary gates —
//! the culmination of the Scottish-group package (scenarios/
//! gb-2024-3zone.toml). Extends the committed B6 two-zone precedent
//! (acceptance_b6_2zone.rs) with the intra-Scottish B4 boundary that
//! throttles the ~7 GW northern wind pool.
//!
//! THE SIX BINDING OBLIGATIONS (scottish-group-boundary-scoping.md
//! adjudication banner) and the rulings (scottish-group-boundary-design-
//! review.md items 3/5) are carried on every quote. The two most
//! load-bearing:
//!  - (obligation 1) the N/S demand split and the CF sub-cluster
//!    partition are pinned PRE-RUN and NEVER tuned to the B4 DA series
//!    (15.78 TWh / 35.8% binding) — B4 has NO annual-outturn cross-anchor.
//!  - (obligation 2 / item 5) the model may quote DIRECTION + PINNED
//!    TOTALS under stated conventions ONLY. NO "B4 effect proper" %, NO
//!    B4-vs-B6 decomposition. B4 is quotable for DIRECTION + binding
//!    frequency; its net magnitude carries "DA-only, no outturn anchor".
//!
//! # THE FINDING OF RECORD (measured 2026-07-04; at full prominence,
//! # exactly as the B6 es=0.03 inversion was — design-review item 5
//! # PREDICTED this and the measurement CONFIRMS it)
//!
//! The single-pass rule-based flow rule ([`grid_adequacy::flow`] rules
//! 1/3, run across TWO hub-sharing borders) COMPOUNDS the equal-depth
//! artefact. Measured consequences, all pinned below as DIAGNOSTICS:
//!
//!  1. **The B4 effect is largely PRE-EMPTED by the flow convention.**
//!     At the reference fleet, copper-plate (unbounded B4) already
//!     strands 6.90 TWh of surplus in N-Scotland (the equal-depth rule
//!     equalises N/S surplus DEPTH rather than wheeling N's surplus
//!     through S to E+W in a single pass); adding the finite B4 gate
//!     raises N curtailment by only +0.04 TWh. The "dominant B4 term"
//!     the three-zone geometry was built to capture is therefore mostly
//!     invisible UNDER THIS DISPATCH CONVENTION.
//!  2. **The model's B4 binding frequency (1.95%) falls FAR below the
//!     observed 35.8%.** The observed DA series pushes the northern
//!     surplus (including scheduled thermal) to the ~1.8 GW B4 wall; the
//!     rule-based flow only UNDER-wheels it (equal-depth signal, single
//!     pass — it partially wheels via B6 reading the post-B4 position but
//!     runs no second sweep), so B4 barely binds in the model. The
//!     35.8% observed anchor reproduces exactly (pack arithmetic) but is
//!     NOT reproduced by the model — this contradicts the design
//!     review's pre-run expectation that B4 binding frequency would be
//!     "honest to quote as validated". Reported, not tuned out.
//!  3. **The three-zone B6 EXIT (10.26 TWh) is LOWER than the two-zone
//!     (15.79 TWh) and the 17 TWh Energy Trends outturn**, because the
//!     stranded northern surplus never reaches the B6 exit. The design
//!     review's expectation that "all Scottish export exits via B6" so
//!     B6 still anchors at ~17 TWh is CONTRADICTED as measured. B6
//!     binding likewise drops (3.35% vs the two-zone 23.6%).
//!
//! WHAT SURVIVES, SAFELY QUOTABLE (design-review item 5, verbatim):
//!  - the raw total-delta DIRECTION (Scottish curtailment UP vs the
//!    two-zone B6-only and copper-plate baselines; further-tightened
//!    lower bound);
//!  - PINNED TOTALS under fully stated conventions;
//!  - the OBSERVED B4/B6 binding-frequency anchors (pack arithmetic).
//!
//! WHAT MAY NOT BE QUOTED: any "B4 effect proper" %, any B4-vs-B6 or
//! boundary-vs-dispatch decomposition. The Stage-7 LP is the resolver.
//!
//! QUOTE DUTY: this three-zone model REMAINS a LOWER BOUND on the
//! Scottish constraint phenomenon (B5 folded into S-Scotland copper-plate
//! -> under-states; design-review item 1 failure-mode A). It TIGHTENS the
//! two-zone B6-only bound; it does not discharge the lower-bound duty.
//!
//! Requires the locally built 2024 + cf-gb2 + cf-gb3 + b4 data packs;
//! fails loudly with build instructions if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{LinkFlowSeries, MultiZoneRunResult, load_multi_zone_inputs, run_multi};
use grid_core::scenario::Scenario;
use grid_core::time::UtcInstant;
use grid_core::trace::load_sparse_power_trace_mw;
use grid_core::units::Power;

const SCENARIO: &str = "scenarios/gb-2024-3zone.toml";
const PERIODS_2024: usize = 17_568;
const B4_TRACE: &str = "data/packs/b6/processed/b4_da_flows_limits.parquet";
const B6_TRACE: &str = "data/packs/b6/processed/b6_da_flows_limits.parquet";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Loud pack-presence check with build instructions (trace-test
/// precedent): the 2024, cf-gb2/cf-gb3 zonal, and b4 packs are
/// fetched-and-built, never committed.
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

// ---------------------------------------------------------------------
// Observed DA anchors (shared with the B6 precedent's alignment logic).
// ---------------------------------------------------------------------

struct Observed {
    flow_mw: Vec<Option<f64>>,
    limit_mw: Vec<Option<f64>>,
}

fn observed(rel: &str) -> Observed {
    let path = repo_root().join(rel);
    let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
    let align = |points: Vec<(UtcInstant, Option<Power>)>| -> Vec<Option<f64>> {
        let mut out = vec![None; PERIODS_2024];
        for (t, v) in points {
            let offset = t.unix_micros() - start.unix_micros();
            if offset < 0 || offset % grid_core::time::HALF_HOUR_MICROS != 0 {
                continue;
            }
            let index = (offset / grid_core::time::HALF_HOUR_MICROS) as usize;
            if index < PERIODS_2024 {
                out[index] = v.map(|p| p.as_gigawatts() * 1000.0);
            }
        }
        out
    };
    Observed {
        flow_mw: align(load_sparse_power_trace_mw(&path, "flow_mw").unwrap()),
        limit_mw: align(load_sparse_power_trace_mw(&path, "limit_mw").unwrap()),
    }
}

fn flow_mask(obs: &Observed) -> Vec<bool> {
    (0..PERIODS_2024)
        .map(|t| obs.flow_mw[t].is_some() && obs.limit_mw[t].is_some())
        .collect()
}

// ---------------------------------------------------------------------
// Model helpers.
// ---------------------------------------------------------------------

fn link<'a>(result: &'a MultiZoneRunResult, name: &str) -> &'a LinkFlowSeries {
    result.links.iter().find(|l| l.name == name).unwrap()
}

/// Model southward (`from → to`) sending-end flow, GW per period: the
/// home end is the `from` zone, so southward = −home_end (loss = 0).
fn southward_gw(result: &MultiZoneRunResult, name: &str) -> Vec<f64> {
    link(result, name)
        .home_end
        .iter()
        .map(|p| -p.as_gigawatts())
        .collect()
}

fn net_twh(southward_gw: &[f64], mask: Option<&[bool]>) -> f64 {
    southward_gw
        .iter()
        .enumerate()
        .filter(|(t, _)| mask.is_none_or(|m| m[*t]))
        .map(|(_, gw)| gw * 0.5)
        .sum::<f64>()
        / 1000.0
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let (va, vb): (f64, f64) = (
        a.iter().map(|x| (x - ma).powi(2)).sum(),
        b.iter().map(|y| (y - mb).powi(2)).sum(),
    );
    cov / (va.sqrt() * vb.sqrt())
}

/// Model binding share on the gate-(iii) DA-flow-mask denominator (the
/// B6 precedent's convention): periods where the modelled southward flow
/// reaches 99% of the capability the model dispatched against, over the
/// OBSERVED flow mask, zero-limit sentinels excluded from the numerator.
fn model_binding_share(
    result: &MultiZoneRunResult,
    name: &str,
    mask: &[bool],
    mask_count: usize,
) -> f64 {
    let model = southward_gw(result, name);
    let cap = link(result, name).capability.as_ref().unwrap();
    let binding = (0..PERIODS_2024)
        .filter(|&t| mask[t] && cap.forward_observed[t])
        .filter(|&t| model[t] >= 0.99 * cap.forward[t].as_gigawatts())
        .count();
    binding as f64 / mask_count as f64
}

fn load_scenario() -> Scenario {
    Scenario::load(&repo_root().join(SCENARIO)).unwrap()
}

fn run(scenario: &Scenario) -> MultiZoneRunResult {
    let inputs = load_multi_zone_inputs(scenario, &repo_root()).unwrap();
    run_multi(scenario, &inputs).unwrap()
}

/// The full three-zone validation run (both links constrained by their
/// 2024 DA traces).
fn validation_run() -> MultiZoneRunResult {
    run(&load_scenario())
}

/// Copper-plate: both internal links unbounded — the model's
/// pre-constraint boundary flows.
fn copper_plate_run() -> MultiZoneRunResult {
    let mut scenario = load_scenario();
    for l in &mut scenario.links {
        l.capability_trace = None;
        l.capacity_gw = Power::gigawatts(1000.0);
        l.reverse_capacity_gw = Some(Power::gigawatts(1000.0));
    }
    run(&scenario)
}

fn zone_curt(result: &MultiZoneRunResult, id: &str) -> f64 {
    result
        .zone(id)
        .unwrap()
        .total_curtailment()
        .as_gigawatt_hours()
        / 1000.0
}

// =====================================================================
// Anchor reproduction: pack arithmetic must land exactly on the reviewed
// b4/b6 statistics before any model comparison (B6 precedent).
// =====================================================================

#[test]
fn b4_observed_anchor_reproduces_the_reviewed_pack_statistics() {
    require_packs();
    let obs = observed(B4_TRACE);
    let mask = flow_mask(&obs);
    let mask_count = mask.iter().filter(|&&m| m).count();
    // 17,280 present rows − 3 NaN = 17,277 (b4_report.json).
    assert_eq!(mask_count, 17_277, "the 2024 B4 flow mask");

    let net: f64 = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| obs.flow_mw[t].unwrap() * 0.5)
        .sum::<f64>()
        / 1e6;
    assert!(
        (net - 15.7818555).abs() < 1e-6,
        "observed B4 net DA flow {net} TWh (anchor 15.7818555)"
    );

    let binding = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .filter(|&t| {
            let limit = obs.limit_mw[t].unwrap();
            limit > 0.0 && obs.flow_mw[t].unwrap() >= 0.99 * limit
        })
        .count();
    let share = binding as f64 / mask_count as f64;
    // b4_report.json share_periods_flow_ge_99pct_limit.
    assert!(
        (share - 0.35775887017422003).abs() < 1e-12,
        "observed B4 binding share {share} (anchor 0.35775887…)"
    );
}

#[test]
fn b6_observed_anchor_reproduces_the_reviewed_pack_statistics() {
    require_packs();
    let obs = observed(B6_TRACE);
    let mask = flow_mask(&obs);
    let mask_count = mask.iter().filter(|&&m| m).count();
    assert_eq!(mask_count, 17_211, "the 2024 B6 flow mask (unchanged)");
    let net: f64 = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| obs.flow_mw[t].unwrap() * 0.5)
        .sum::<f64>()
        / 1e6;
    assert!(
        (net - 22.627189).abs() < 1e-6,
        "observed B6 net DA flow {net} TWh (anchor 22.627189, unchanged from two-zone)"
    );
}

// =====================================================================
// B4 GATE: DIRECTION (southward) is validated; the net magnitude and the
// binding frequency are DIAGNOSTICS carrying the finding that the
// rule-based flow pre-empts the B4 effect (never a validated match).
// =====================================================================

/// B4 copper-plate net over the DA mask (DIAGNOSTIC, first pass 2026-07-04).
/// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7 —
/// the pre-fix walk silently cap-truncated boundary-sliver stalls,
/// stranding northern surplus; findings 1–3 unchanged in shape). Was
/// net 6.13413709138561 / r 0.2328541398969653 / constrained
/// 6.1881052641925525 / binding 0.01950570122127684 (337/17,277).
const PIN_B4_COPPER_NET_TWH: f64 = 6.174674742763343;
const PIN_B4_COPPER_R: f64 = 0.23282220323347905;
/// B4 constrained net over all periods (DIAGNOSTIC).
const PIN_B4_CONSTRAINED_NET_TWH: f64 = 6.228187380781606;
/// B4 model binding share (gate-(iii) mask denominator, 347/17,277) —
/// FAR below the observed 0.35775887 (finding 2). DIAGNOSTIC.
const PIN_B4_CONSTRAINED_BINDING: f64 = 0.020084505411819182;

#[test]
fn b4_gate_direction_holds_and_the_flow_artefact_is_pinned() {
    require_packs();
    let obs = observed(B4_TRACE);
    let mask = flow_mask(&obs);
    let mask_count = mask.iter().filter(|&&m| m).count();

    let copper = southward_gw(&copper_plate_run(), "B4");
    let net_copper = net_twh(&copper, Some(&mask));
    let model_masked: Vec<f64> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| copper[t])
        .collect();
    let obs_masked: Vec<f64> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| obs.flow_mw[t].unwrap() / 1000.0)
        .collect();
    let r = pearson(&model_masked, &obs_masked);

    let validation = validation_run();
    let net_constrained = net_twh(&southward_gw(&validation, "B4"), None);
    let binding = model_binding_share(&validation, "B4", &mask, mask_count);
    eprintln!(
        "B4 gate: copper net {net_copper:.4} TWh (r={r:.4}) vs DA anchor 15.7818555 — a ~9.6 TWh \
         miss ('DA-only, no outturn anchor'; the ~19% offshore wedge lands here, but the DOMINANT \
         gap is the equal-depth flow artefact stranding northern surplus, NOT a data wedge). \
         Model B4 binding {binding:.4} vs observed 0.35775887 — FINDING: the flow convention \
         pre-empts the B4 effect (direction + pinned totals only; LP is the Stage-7 resolver)"
    );

    // VALIDATED: B4 flow is southward (net > 0) — the direction the DA
    // series shows (94% of Scottish offshore is north of B4).
    assert!(
        net_copper > 0.0 && net_constrained > 0.0,
        "B4 modelled flow must be net southward (N→S), like the DA series"
    );

    // DIAGNOSTICS (measured-then-pinned; deterministic ADR-5). These are
    // NOT validated magnitudes — they carry finding 1/2 (the flow
    // convention, not a data wedge, is the dominant gap). A move is a
    // knowing re-pin with the record.
    assert!(
        (net_copper - PIN_B4_COPPER_NET_TWH).abs() < 1e-6,
        "PINNED B4 copper net moved: {net_copper}"
    );
    assert!(
        (r - PIN_B4_COPPER_R).abs() < 1e-9,
        "PINNED B4 copper r moved: {r}"
    );
    assert!(
        (net_constrained - PIN_B4_CONSTRAINED_NET_TWH).abs() < 1e-6,
        "PINNED B4 constrained net moved: {net_constrained}"
    );
    assert!(
        (binding - PIN_B4_CONSTRAINED_BINDING).abs() < 1e-12,
        "PINNED B4 binding moved: {binding}"
    );

    // The finding, as an assert: the model's B4 binding is far below the
    // observed 35.8% (the flow convention pre-empts the boundary).
    assert!(
        binding < 0.10,
        "the model B4 binding should fall far below the observed 0.358 (the flow-convention \
         pre-emption finding): {binding}"
    );
}

// =====================================================================
// B6 GATE (design-review item 4). The two-zone gate expected B6 to be
// "unchanged" (~22.6 TWh copper / 23.6% binding). In the three-zone
// geometry under rule-based flow it is NOT: the B6 exit DROPS because
// northern surplus is stranded at B4/N-Scotland (finding 3). Pinned as
// DIAGNOSTICS with that finding.
// =====================================================================

// Re-pinned 2026-07-06 (R7 stall fix; finding 3 unchanged in shape).
// Was copper 10.362549874727046 / r 0.7209369300335855 / constrained
// 10.258225220910827; the binding share (577/17,211) did not move.
const PIN_B6_COPPER_NET_TWH: f64 = 10.425323654285018;
const PIN_B6_COPPER_R: f64 = 0.7218638379273287;
const PIN_B6_CONSTRAINED_NET_TWH: f64 = 10.319574549012335;
const PIN_B6_CONSTRAINED_BINDING: f64 = 0.03352507117541107;

#[test]
fn b6_gate_exit_drops_in_the_three_zone_geometry_and_is_pinned() {
    require_packs();
    let obs = observed(B6_TRACE);
    let mask = flow_mask(&obs);
    let mask_count = mask.iter().filter(|&&m| m).count();

    let copper = southward_gw(&copper_plate_run(), "B6");
    let net_copper = net_twh(&copper, Some(&mask));
    let model_masked: Vec<f64> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| copper[t])
        .collect();
    let obs_masked: Vec<f64> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| obs.flow_mw[t].unwrap() / 1000.0)
        .collect();
    let r = pearson(&model_masked, &obs_masked);

    let validation = validation_run();
    let net_constrained = net_twh(&southward_gw(&validation, "B6"), None);
    let binding = model_binding_share(&validation, "B6", &mask, mask_count);
    eprintln!(
        "B6 gate: three-zone copper B6 exit {net_copper:.4} TWh vs the two-zone 19.898 (DA anchor \
         22.627); constrained {net_constrained:.4} TWh vs the 17 TWh Energy Trends outturn; \
         binding {binding:.4} vs the two-zone 0.2502 — FINDING 3: the B6 exit DROPS in the \
         three-zone geometry (northern surplus stranded at B4/N-Scotland), contradicting the \
         design review's 'all Scottish export exits via B6, ~17 TWh' expectation. Direction \
         (southward) holds; magnitude is convention-conditioned, quoted only as a pinned total."
    );

    // VALIDATED: B6 flow is southward (net > 0).
    assert!(
        net_copper > 0.0 && net_constrained > 0.0,
        "B6 flow must be net southward"
    );
    // The finding: three-zone B6 exit is materially BELOW the two-zone.
    assert!(
        net_constrained < 15.0,
        "three-zone B6 exit should drop below the two-zone 15.79 TWh (surplus stranding): {net_constrained}"
    );

    // DIAGNOSTICS (measured-then-pinned).
    assert!(
        (net_copper - PIN_B6_COPPER_NET_TWH).abs() < 1e-6,
        "PINNED B6 copper net moved: {net_copper}"
    );
    assert!(
        (r - PIN_B6_COPPER_R).abs() < 1e-9,
        "PINNED B6 copper r moved: {r}"
    );
    assert!(
        (net_constrained - PIN_B6_CONSTRAINED_NET_TWH).abs() < 1e-6,
        "PINNED B6 constrained net moved: {net_constrained}"
    );
    assert!(
        (binding - PIN_B6_CONSTRAINED_BINDING).abs() < 1e-12,
        "PINNED B6 binding moved: {binding}"
    );
}

// =====================================================================
// Q2/Q10: three-zone Scottish curtailment at the reference fleet and at
// 60 GW wind, vs the two-zone B6-only numbers. QUOTE DUTY: still a LOWER
// BOUND (B5 folded -> under-states); DIRECTION ONLY for the increment;
// NO "B4 effect proper" %, NO B4-vs-B6 decomposition (obligation 2).
// =====================================================================

// Reference-fleet pins (first pass 2026-07-04).
// Re-pinned 2026-07-06 (R7 stall fix): was NSCO constrained
// 6.946826258169218 / NSCO copper 6.904630615779606; SSCO/RGB unmoved.
const PIN_REF_NSCO_CONSTRAINED: f64 = 6.913762587626429;
const PIN_REF_SSCO_CONSTRAINED: f64 = 0.060931165659654896;
const PIN_REF_RGB_CONSTRAINED: f64 = 0.0;
const PIN_REF_NSCO_COPPER: f64 = 6.870827868362257;
const PIN_REF_SSCO_COPPER: f64 = 0.0;
const PIN_REF_RGB_COPPER: f64 = 0.0;

/// The two-zone B6-only reference-fleet SCO constrained curtailment
/// (acceptance_b6_2zone.rs PIN_REF_SCO_CONSTRAINED) — the bound the
/// three-zone number tightens. Quoting copy re-pinned 2026-07-06 with
/// its source (R7 stall fix): was 1.684001587134434.
const TWO_ZONE_REF_SCO_CONSTRAINED: f64 = 1.678730303423035;

#[test]
fn q2_q10_reference_fleet_three_zone_curtailment_is_pinned() {
    require_packs();
    let constrained = validation_run();
    let copper = copper_plate_run();

    let (n_c, s_c, e_c) = (
        zone_curt(&constrained, "NSCO"),
        zone_curt(&constrained, "SSCO"),
        zone_curt(&constrained, "RGB"),
    );
    let (n_k, s_k, e_k) = (
        zone_curt(&copper, "NSCO"),
        zone_curt(&copper, "SSCO"),
        zone_curt(&copper, "RGB"),
    );
    let sco_constrained = n_c + s_c;
    eprintln!(
        "Q2/Q10 ref fleet three-zone: constrained NSCO {n_c} / SSCO {s_c} / RGB {e_c} \
         (Scottish total {sco_constrained} TWh) vs copper NSCO {n_k} / SSCO {s_k} / RGB {e_k}. \
         DIRECTION only: Scottish curtailment > the two-zone B6-only {TWO_ZONE_REF_SCO_CONSTRAINED} \
         and > copper — a further-tightened LOWER BOUND (B5 folded; no B4-vs-B6 decomposition)"
    );

    // DIRECTION (obligation 2): the three-zone Scottish curtailment
    // exceeds the two-zone B6-only bound and its own copper-plate.
    assert!(
        sco_constrained > TWO_ZONE_REF_SCO_CONSTRAINED,
        "three-zone Scottish curtailment must exceed the two-zone B6-only bound: {sco_constrained}"
    );
    assert!(
        sco_constrained >= n_k + s_k - 1e-9,
        "the boundaries must not reduce Scottish curtailment vs copper"
    );

    // PINNED totals (measured-then-pinned).
    for (what, measured, pinned) in [
        ("NSCO constrained", n_c, PIN_REF_NSCO_CONSTRAINED),
        ("SSCO constrained", s_c, PIN_REF_SSCO_CONSTRAINED),
        ("RGB constrained", e_c, PIN_REF_RGB_CONSTRAINED),
        ("NSCO copper", n_k, PIN_REF_NSCO_COPPER),
        ("SSCO copper", s_k, PIN_REF_SSCO_COPPER),
        ("RGB copper", e_k, PIN_REF_RGB_COPPER),
    ] {
        assert!(
            (measured - pinned).abs() < 1e-6,
            "PINNED Q2/Q10 ref {what} moved: measured {measured}, pinned {pinned}"
        );
    }
}

// 60 GW high-wind pins (first pass 2026-07-04).
// Re-pinned 2026-07-06 (R7 stall fix): were NSCO 24.56326876740292 /
// SSCO 7.103028971336957 / RGB 2.6807213859690617 constrained; NSCO
// 23.613921736790875 / SSCO 4.225967159810115 / RGB
// 3.6990859260946127 copper. Post-fix the copper-plate SSCO/RGB split
// lands at near-exact equal surplus depth.
const PIN_60_NSCO_CONSTRAINED: f64 = 24.537021747975718;
const PIN_60_SSCO_CONSTRAINED: f64 = 7.030181653205421;
const PIN_60_RGB_CONSTRAINED: f64 = 2.680339572534873;
const PIN_60_NSCO_COPPER: f64 = 23.546549373273045;
const PIN_60_SSCO_COPPER: f64 = 3.70796534613293;
const PIN_60_RGB_COPPER: f64 = 3.70796534613293;

/// The two-zone B6-only 60 GW SCO constrained curtailment
/// (acceptance_b6_2zone.rs PIN_60GW_SCO_CONSTRAINED) — the bound the
/// three-zone number tightens. Quoting copy re-pinned 2026-07-06 with
/// its source (R7 stall fix): was 27.13942924577101.
const TWO_ZONE_60_SCO_CONSTRAINED: f64 = 27.02512748182898;

#[test]
fn q2_q10_sixty_gw_three_zone_curtailment_is_pinned() {
    require_packs();
    let root = repo_root();
    let factor = 60.0 / 29.1;

    let mut scenario = load_scenario();
    for zone in &mut scenario.zones {
        for entry in &mut zone.fleet {
            let t = entry.technology.as_str();
            if t == "onshore_wind" || t == "offshore_wind" {
                entry.capacity_gw = entry.capacity_gw * factor;
            }
        }
    }
    // A scaled (hypothetical) fleet takes the flat central capabilities
    // (no DA limit series exists for a hypothetical fleet).
    for l in &mut scenario.links {
        l.capability_trace = None;
    }
    let constrained = run_multi(
        &scenario,
        &load_multi_zone_inputs(&scenario, &root).unwrap(),
    )
    .unwrap();

    let mut copper = scenario.clone();
    for l in &mut copper.links {
        l.capacity_gw = Power::gigawatts(1000.0);
        l.reverse_capacity_gw = Some(Power::gigawatts(1000.0));
    }
    let copper = run_multi(&copper, &load_multi_zone_inputs(&copper, &root).unwrap()).unwrap();

    let (n_c, s_c, e_c) = (
        zone_curt(&constrained, "NSCO"),
        zone_curt(&constrained, "SSCO"),
        zone_curt(&constrained, "RGB"),
    );
    let (n_k, s_k, e_k) = (
        zone_curt(&copper, "NSCO"),
        zone_curt(&copper, "SSCO"),
        zone_curt(&copper, "RGB"),
    );
    let sco_constrained = n_c + s_c;
    eprintln!(
        "Q2/Q10 60 GW three-zone: constrained NSCO {n_c} / SSCO {s_c} / RGB {e_c} \
         (Scottish total {sco_constrained} TWh) vs copper NSCO {n_k} / SSCO {s_k} / RGB {e_k}. \
         DIRECTION only: Scottish curtailment > the two-zone B6-only {TWO_ZONE_60_SCO_CONSTRAINED} \
         TWh — the further-tightened LOWER BOUND (B5 folded -> under-states)"
    );

    assert!(
        sco_constrained > TWO_ZONE_60_SCO_CONSTRAINED,
        "three-zone 60 GW Scottish curtailment must exceed the two-zone 27.03 TWh: {sco_constrained}"
    );

    for (what, measured, pinned) in [
        ("NSCO constrained", n_c, PIN_60_NSCO_CONSTRAINED),
        ("SSCO constrained", s_c, PIN_60_SSCO_CONSTRAINED),
        ("RGB constrained", e_c, PIN_60_RGB_CONSTRAINED),
        ("NSCO copper", n_k, PIN_60_NSCO_COPPER),
        ("SSCO copper", s_k, PIN_60_SSCO_COPPER),
        ("RGB copper", e_k, PIN_60_RGB_COPPER),
    ] {
        assert!(
            (measured - pinned).abs() < 1e-6,
            "PINNED Q2/Q10 60 GW {what} moved: measured {measured}, pinned {pinned}"
        );
    }
}
