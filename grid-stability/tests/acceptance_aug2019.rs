//! Stage 6 acceptance: reproduce the 9 August 2019 GB frequency event
//! (docs/04 Stage 6; tolerances pinned 2026-07-03 from
//! `docs/notes/stage-6-evidence-report.md`).
//!
//! Inputs are the published record only — the committed event spec
//! `scenarios/events/gb-2019-08-09.toml` is built from
//! `data/reference/stability-2019-event.toml` (loss sequence, response
//! holdings × measured delivery factors, stage-1 LFDD 931 MW with a
//! 0.2–0.5 s action delay, 29 GW demand) — and the official inertia is
//! tested at BOTH published bounds (210 GVA·s, ESO report Table 4;
//! 219.632 GVA·s, appendices Appendix M Q42; the ~5 % self-disagreement
//! is itself tolerance evidence).
//!
//! Anchors (docs/04, pinned):
//! - **T1 (nadir)**: 48.75 < f_min ≤ 48.80 Hz — the LFDD protection
//!   band (stage 1 at 48.80 operated, stage 2 at 48.75 did not);
//!   measured 48.787. Precondition: stage-1 LFDD explicitly modelled.
//! - **T2 (first arrest)**: 49.10 ± 0.10 Hz (measured 49.083) — the
//!   genuine swing-physics discriminator.
//! - **T3 (initial RoCoF)**: within ±25 % of 0.144 Hz/s over the pinned
//!   2-s window starting 0.51 s after the fault (the reference TOML's
//!   window convention: the first whole-second sample after the fault).
//! - **T4 (binary)**: a 1,000 MW loss under identical conditions stays
//!   ≥ 49.5 Hz (ESO's own published simulation + the 1 July 2019
//!   outturn).
//!
//! Fault-to-LFDD time and the recovery trajectory are diagnostic only
//! (input circularity documented in the evidence report) — asserted
//! nowhere here.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use grid_core::units::{Duration, Inertia, Power};
use grid_stability::{EventResult, EventSpec, simulate};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn spec_at(inertia_gva_s: f64) -> EventSpec {
    let mut spec =
        EventSpec::load(&repo_root().join("scenarios/events/gb-2019-08-09.toml")).unwrap();
    spec.inertia = Inertia::gigavolt_ampere_seconds(inertia_gva_s);
    spec
}

/// The two published inertia bounds (GVA·s).
const INERTIA_BOUNDS: [f64; 2] = [210.0, 219.632];

fn run_at(inertia_gva_s: f64) -> EventResult {
    simulate(&spec_at(inertia_gva_s)).unwrap()
}

#[test]
fn t1_nadir_within_lfdd_protection_band_at_both_inertia_bounds() {
    for bound in INERTIA_BOUNDS {
        let result = run_at(bound);
        let nadir = result.nadir.as_hertz();
        assert!(
            48.75 < nadir && nadir <= 48.80,
            "T1 FAIL at {bound} GVA·s: nadir {nadir} Hz outside (48.75, 48.80]"
        );
        // The precondition that makes T1 meaningful: stage-1 LFDD
        // actually operated in the model (evidence report §4 — the
        // nadir is an LFDD interception, not a free swing minimum) and
        // stage 2 did not.
        let stages: Vec<u32> = result.lfdd_actions.iter().map(|a| a.stage).collect();
        assert_eq!(
            stages,
            vec![1],
            "at {bound} GVA·s: expected exactly LFDD stage 1 to operate, got {stages:?}"
        );
    }
}

#[test]
fn t2_first_arrest_within_pinned_band_at_both_inertia_bounds() {
    for bound in INERTIA_BOUNDS {
        let result = run_at(bound);
        let arrest = result
            .first_arrest
            .expect("the first descent must arrest")
            .as_hertz();
        assert!(
            (49.00..=49.20).contains(&arrest),
            "T2 FAIL at {bound} GVA·s: first arrest {arrest} Hz outside 49.10 ± 0.10"
        );
    }
}

#[test]
fn t3_initial_rocof_within_25pct_of_measured_at_both_inertia_bounds() {
    for bound in INERTIA_BOUNDS {
        let result = run_at(bound);
        let rocof = result
            .rocof_window_mean
            .expect("the spec pins a RoCoF window")
            .as_hertz_per_second()
            .abs();
        assert!(
            (0.108..=0.180).contains(&rocof),
            "T3 FAIL at {bound} GVA·s: 2-s window RoCoF {rocof} Hz/s outside \
             0.144 ± 25 % [0.108, 0.180]"
        );
    }
}

#[test]
fn t4_counterfactual_1000mw_loss_stays_at_or_above_49_5_hz() {
    for bound in INERTIA_BOUNDS {
        let mut spec = spec_at(bound);
        // Identical conditions, the secured loss only: replace the
        // staged trips with a single 1,000 MW loss at the fault.
        spec.losses = vec![grid_stability::InfeedLoss {
            name: "secured_infeed".to_owned(),
            power: Power::megawatts(1000.0),
            at: Duration::from_seconds(0.0),
        }];
        let result = simulate(&spec).unwrap();
        let min = result.nadir.as_hertz();
        assert!(
            min >= 49.5,
            "T4 FAIL at {bound} GVA·s: 1,000 MW loss reached {min} Hz (< 49.5)"
        );
        assert!(
            result.lfdd_actions.is_empty(),
            "T4 at {bound} GVA·s: LFDD must not operate for the secured loss"
        );
    }
}

/// Diagnostic reporter (run with `--nocapture`): the measured anchor
/// values at both inertia bounds, verbatim, plus the diagnostic-only
/// quantities (fault-to-LFDD time, recovery) that are reported but
/// never gated.
#[test]
fn report_measured_anchor_values() {
    for bound in INERTIA_BOUNDS {
        let result = run_at(bound);
        println!("--- inertia {bound} GVA·s ---");
        println!(
            "T1 nadir            {:.4} Hz at t = {:.2} s",
            result.nadir.as_hertz(),
            result.nadir_at.as_seconds()
        );
        println!(
            "T2 first arrest     {:.4} Hz at t = {:.2} s",
            result.first_arrest.unwrap().as_hertz(),
            result.first_arrest_at.unwrap().as_seconds()
        );
        println!(
            "T3 RoCoF (2 s win)  {:.4} Hz/s (measured event: -0.144)",
            result.rocof_window_mean.unwrap().as_hertz_per_second()
        );
        println!(
            "   steepest 1-s     {:.4} Hz/s",
            result.steepest_1s_rocof.as_hertz_per_second()
        );
        for action in &result.lfdd_actions {
            println!(
                "   LFDD stage {} ({} Hz): triggered {:.2} s, actioned {:.2} s, {} MW \
                 (measured trigger: 75.9 s — diagnostic only)",
                action.stage,
                action.trigger.as_hertz(),
                action.triggered_at.as_seconds(),
                action.actioned_at.as_seconds(),
                action.block.as_gigawatts() * 1000.0
            );
        }
        let mut spec_t4 = spec_at(bound);
        spec_t4.losses = vec![grid_stability::InfeedLoss {
            name: "secured_infeed".to_owned(),
            power: Power::megawatts(1000.0),
            at: Duration::from_seconds(0.0),
        }];
        let t4 = simulate(&spec_t4).unwrap();
        println!(
            "T4 1000 MW loss min {:.4} Hz at t = {:.2} s",
            t4.nadir.as_hertz(),
            t4.nadir_at.as_seconds()
        );
    }
}

/// The MEASURED 2019 nadir is the stability engine's validation
/// credential and, until now, lived only in a doc-comment (T1 above)
/// and as an ungated datum in the reference TOML — the drift-guard
/// tests bound the MODEL's nadir into the protection band but never
/// asserted the measured value it is validated against. Pin both, and
/// pin that the model sits inside the T1 band centred on the measured
/// value (the reconstruction's whole point: it lands within a few mHz
/// of the recorded 48.787 Hz at both published inertia bounds). The
/// measured value is a fact of the NESO 1-s record — a move here is a
/// knowing correction of the credential, not a retune.
#[test]
fn measured_2019_nadir_is_pinned_and_the_model_lands_in_its_t1_band() {
    let reference: toml::Value = toml::from_str(
        &std::fs::read_to_string(repo_root().join("data/reference/stability-2019-event.toml"))
            .unwrap(),
    )
    .unwrap();
    // The record holds the measured nadir 48.787 Hz (NESO 1-s data).
    let measured = reference["frequency"]["nadir_hz_measured"]
        .as_float()
        .unwrap();
    assert!(
        (measured - 48.787).abs() < 1e-9,
        "the committed record's measured 2019 nadir moved: {measured} Hz (pinned 48.787)"
    );
    // The measured nadir itself sits in the LFDD stage-1 protection band
    // (T1), the band the model is validated against.
    assert!(
        48.75 < measured && measured <= 48.80,
        "measured nadir {measured} Hz outside the T1 protection band (48.75, 48.80]"
    );
    // The MODEL lands inside the T1 band AND within a few mHz of the
    // measured value at both published inertia bounds — first measured
    // 2026-07-03 as 48.7928 (210 GVA·s) / 48.7931 (219.632 GVA·s),
    // ~6 mHz above the record. The ±0.02 Hz tolerance is the credential:
    // a reconstruction that drifted out of it would no longer reproduce
    // the event.
    for bound in INERTIA_BOUNDS {
        let modelled = run_at(bound).nadir.as_hertz();
        assert!(
            48.75 < modelled && modelled <= 48.80,
            "at {bound} GVA·s: modelled nadir {modelled} Hz outside the T1 band"
        );
        assert!(
            (modelled - measured).abs() < 0.02,
            "at {bound} GVA·s: modelled nadir {modelled} Hz is more than 20 mHz from the \
             measured 48.787 Hz — the reconstruction no longer lands on the record"
        );
    }
}

/// The event spec must be built from the committed reference record —
/// cross-check the load-bearing numbers against
/// `data/reference/stability-2019-event.toml` so the two files cannot
/// drift apart silently.
#[test]
fn event_spec_matches_the_committed_reference_record() {
    let spec = EventSpec::load(&repo_root().join("scenarios/events/gb-2019-08-09.toml")).unwrap();
    let reference: toml::Value = toml::from_str(
        &std::fs::read_to_string(repo_root().join("data/reference/stability-2019-event.toml"))
            .unwrap(),
    )
    .unwrap();

    // Inertia: the spec's default is the Table 4 headline bound.
    assert_eq!(
        spec.inertia.as_gigavolt_ampere_seconds(),
        reference["pre_event"]["inertia_gva_s"].as_float().unwrap()
    );
    // Demand.
    assert_eq!(
        spec.demand.as_gigawatts(),
        reference["pre_event"]["demand_gw"].as_float().unwrap()
    );
    // The staged infeed losses sum to the reference cumulative total
    // (ESO Table 2's 1,878 MW infeed trips + the 200 MW net loss at
    // 49 Hz = Ofgem's "at least 1,990 MW" cumulative record).
    let spec_total_mw: f64 = spec
        .losses
        .iter()
        .map(|l| l.power.as_gigawatts() * 1000.0)
        .sum();
    let reference_total = reference["losses_totals"]["ofgem_cumulative_min_mw"]
        .as_integer()
        .unwrap() as f64
        + 88.0; // spec carries GT1B (187 MW, post-LFDD) which Ofgem's
    // 1,990 "at least" partially subsumes: 1,891 pre-GT1B + 187 = 2,078.
    assert_eq!(spec_total_mw, reference_total);
    // Response: holdings × measured delivery factors.
    let delivered_mw: f64 = spec
        .responses
        .iter()
        .map(|r| r.power.as_gigawatts() * 1000.0 * r.delivery_factor.value())
        .sum();
    let reference_delivered = reference["pre_event"]["response_primary_validated_delivered_mw"]
        .as_integer()
        .unwrap()
        + reference["pre_event"]["response_secondary_validated_delivered_mw"]
            .as_integer()
            .unwrap();
    assert!(
        (delivered_mw - reference_delivered as f64).abs() < 0.5,
        "spec delivered response {delivered_mw} MW != reference {reference_delivered} MW"
    );
    // LFDD stage 1: the observed 931 MW block at 48.8 Hz, with the
    // published 0.2–0.5 s action-delay range.
    let lfdd = spec.lfdd.as_ref().unwrap();
    assert!((0.2..=0.5).contains(&lfdd.action_delay.as_seconds()));
    assert_eq!(lfdd.stages[0].frequency.as_hertz(), 48.8);
    assert_eq!(lfdd.stages[0].block.as_gigawatts() * 1000.0, 931.0);
    // Stage 2 must be present (T1's "stage 2 did not operate" needs it
    // in the model) at the E3C table's 48.75 Hz.
    assert_eq!(lfdd.stages[1].frequency.as_hertz(), 48.75);
}
