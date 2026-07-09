//! `grid-cli stability` integration tests (Stage 6): the event runner
//! produces the docs/06 artefact set with the pinned anchor values in
//! its report; the inertia finder reproduces the pinned Module 6
//! first-cut numbers; the Q8 pathway runner (Stage 6 part 2) produces
//! the largest-survivable-loss artefact set from a pathway spec; the
//! Module 6 sweep (`inertia --renewable-scale`) produces hours-below-
//! floor vs renewable share with the scale-1.0 endpoint pinned
//! bit-identically to the part-1 first cut.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn grid_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .unwrap()
}

fn out_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("grid-cli-stage6-tests")
        .join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}

/// Read a numeric or quoted value from our own report.toml format.
fn report_value(report: &str, key: &str) -> String {
    report
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| panic!("report has no key {key:?}"))
}

#[test]
fn event_runner_writes_the_docs06_artefact_set_with_anchor_values() {
    let out = out_dir("event");
    let output = grid_cli(&[
        "stability",
        "event",
        "--event",
        "scenarios/events/gb-2019-08-09.toml",
        "--out",
        out.to_str().unwrap(),
        "--measured",
        "data/reference/neso-frequency-2019-08-09-event-window.csv",
        "--measured-t0",
        "2019-08-09T15:52:33.490Z",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // CSV + Parquet, both, always (docs/06), plus report and chart.
    for name in [
        "frequency_trace.csv",
        "frequency_trace.parquet",
        "report.toml",
        "frequency.png",
    ] {
        assert!(out.join(name).exists(), "{name} missing");
    }
    // Metadata header on the CSV.
    let csv = std::fs::read_to_string(out.join("frequency_trace.csv")).unwrap();
    assert!(csv.starts_with("# grid-sim output"));
    assert!(csv.contains("# engine_git_hash = "));
    assert!(csv.contains("# event_spec_sha256 = "));
    // The report carries the T1 anchor inside the protection band and
    // exactly one LFDD action (stage 1).
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();
    let nadir: f64 = report_value(&report, "nadir_hz").parse().unwrap();
    assert!(48.75 < nadir && nadir <= 48.80, "nadir {nadir}");
    assert_eq!(report.matches("[[results.lfdd_actions]]").count(), 1);
    assert!(report.contains("stage = 1"));
    // Era limits (2019 spec): the event's steepest RoCoF exceeded the
    // 0.125 Hz/s relay limit; the statutory floor was breached.
    assert_eq!(report_value(&report, "rocof_relay_exceeded"), "true");
    assert_eq!(report_value(&report, "statutory_floor_breached"), "true");
}

#[test]
fn event_runner_is_deterministic_across_runs() {
    let out_a = out_dir("event-det-a");
    let out_b = out_dir("event-det-b");
    for out in [&out_a, &out_b] {
        let output = grid_cli(&[
            "stability",
            "event",
            "--event",
            "scenarios/events/gb-2019-08-09.toml",
            "--inertia-gva-s",
            "219.632",
            "--out",
            out.to_str().unwrap(),
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let digest = |dir: &Path| {
        let report = std::fs::read_to_string(dir.join("report.toml")).unwrap();
        report_value(&report, "result_digest_sha256")
    };
    assert_eq!(digest(&out_a), digest(&out_b));
}

// ---------------------------------------------------------------------
// Q8 pathway runner (Stage 6 part 2).
// ---------------------------------------------------------------------

#[test]
fn pathway_runner_writes_the_docs06_artefact_set() {
    let out = out_dir("pathway");
    let output = grid_cli(&[
        "stability",
        "pathway",
        "--pathway",
        "grid-cli/tests/fixtures/fes-pathway-hand.toml",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // CSV + Parquet, both, always (docs/06), plus report and chart.
    for name in [
        "pathway.csv",
        "pathway.parquet",
        "report.toml",
        "pathway.png",
    ] {
        assert!(out.join(name).exists(), "{name} missing");
    }
    let csv = std::fs::read_to_string(out.join("pathway.csv")).unwrap();
    assert!(csv.starts_with("# grid-sim output"));
    assert!(csv.contains("# engine_git_hash = "));
    assert!(csv.contains("# pathway_spec_sha256 = "));
    // 3 years × 2 default dispatch conditions (min/mean) = 6 rows.
    let rows: Vec<&str> = csv
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("year") && !l.trim().is_empty())
        .collect();
    assert_eq!(rows.len(), 6, "rows: {rows:?}");

    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();
    // The 2019-era defaults were used and must be flagged, not silent.
    assert_eq!(report_value(&report, "responses_defaulted_to_2019"), "true");
    assert_eq!(report_value(&report, "load_damping_defaulted"), "true");
    assert_eq!(
        report_value(&report, "dispatch_conditions_defaulted"),
        "true"
    );
    // The band caveat (market-only lower edge is zero) must be carried
    // in the artefact itself.
    assert!(report.contains("UNCONSTRAINED"), "band caveat missing");
    // The secured-loss reference lines with their cited defaults.
    assert!(report.contains("sqss_infrequent_infeed_loss"));
    assert!(report.contains("1800"));
    assert!(report.contains("sqss_normal_infeed_loss"));
    assert!(report.contains("1320"));
    // The zero-inertia year (2036: wind + solar + battery + hydrogen)
    // reads survivable loss 0 with the zero_inertia finding flag.
    assert!(
        report.contains("zero_inertia = true"),
        "zero-inertia finding missing from report"
    );
    // Crossing years are stated per condition and reference loss, in
    // BOTH directions (the FES 2025 result rises over the pathway, so
    // the readable date can be the recovery year, not the failure
    // year).
    assert!(
        report.contains("first_year_below"),
        "crossing-year keys missing"
    );
    assert!(
        report.contains("first_year_at_or_above"),
        "recovery-year keys missing"
    );
    // Console: the FINDING line fires for the zero-inertia year.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FINDING"), "stdout: {stdout}");
    assert!(stdout.contains("2036"), "stdout: {stdout}");
}

#[test]
fn pathway_runner_is_deterministic_across_runs() {
    let out_a = out_dir("pathway-det-a");
    let out_b = out_dir("pathway-det-b");
    for out in [&out_a, &out_b] {
        let output = grid_cli(&[
            "stability",
            "pathway",
            "--pathway",
            "grid-cli/tests/fixtures/fes-pathway-hand.toml",
            "--out",
            out.to_str().unwrap(),
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let digest = |dir: &Path| {
        let report = std::fs::read_to_string(dir.join("report.toml")).unwrap();
        report_value(&report, "result_digest_sha256")
    };
    assert_eq!(digest(&out_a), digest(&out_b));
}

/// Pinned regression on the REAL FES 2025 pathway file (pin before
/// quote, docs/05 rule 3; `regression_2024` precedent): the Q8
/// headline numbers quoted from `data/reference/fes-pathway.toml`
/// under the flagged 2019-default era assumptions, first measured
/// 2026-07-03. These are Stage 6 part 2 published numbers and must
/// not move: 2024 largest survivable loss 1,372.68 MW (min, φ=0.15) /
/// 1,573.49 MW (mean, φ=0.35); the pathway is below the 1,800 MW
/// standard from 2024 and first meets it in 2037 (min) / 2035 (mean);
/// it never falls below the 1,320 MW standard. Digest pins the whole
/// CSV data section bit-identically.
#[test]
fn q8_fes2025_pathway_reproduces_the_pinned_headline_numbers() {
    let out = out_dir("pathway-fes2025-pin");
    let output = grid_cli(&[
        "stability",
        "pathway",
        "--pathway",
        "data/reference/fes-pathway.toml",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();

    // The pinned result digest (whole CSV data section, bit-identical).
    assert_eq!(
        report_value(&report, "result_digest_sha256"),
        "fd410616862f386047137615ce99108fa2dc830d26d93e1bc5e3f27fec84e11a"
    );

    // 2024 survivable losses, exact values as written to the artefact.
    for block in [
        "[[results.points]]\nyear = 2024\ncondition = \"min\"\n\
         inertia_gva_s = 42.08029166666667\n\
         largest_survivable_loss_mw = 1372.6806640625\n",
        "[[results.points]]\nyear = 2024\ncondition = \"mean\"\n\
         inertia_gva_s = 98.1873472222222\n\
         largest_survivable_loss_mw = 1573.486328125\n",
    ] {
        assert!(
            report.contains(block),
            "pinned 2024 point missing:\n{block}"
        );
    }

    // Crossing years: below 1,800 MW from the 2024 start, first at or
    // above it in 2037 (min) / 2035 (mean); never below 1,320 MW.
    for block in [
        "condition = \"min\"\nreference = \"sqss_infrequent_infeed_loss\"\n\
         reference_mw = 1800\nfirst_year_below = 2024\nfirst_year_at_or_above = 2037\n",
        "condition = \"mean\"\nreference = \"sqss_infrequent_infeed_loss\"\n\
         reference_mw = 1800\nfirst_year_below = 2024\nfirst_year_at_or_above = 2035\n",
        "condition = \"min\"\nreference = \"sqss_normal_infeed_loss\"\n\
         reference_mw = 1320\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
        "condition = \"mean\"\nreference = \"sqss_normal_infeed_loss\"\n\
         reference_mw = 1320\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
    ] {
        assert!(report.contains(block), "pinned crossing missing:\n{block}");
    }

    // The pins above are only quotable with the 2019-default flags
    // stated (docs/05 rule 3 discipline carried by the artefact).
    assert_eq!(report_value(&report, "responses_defaulted_to_2019"), "true");
    assert_eq!(report_value(&report, "load_damping_defaulted"), "true");
}

/// Pinned regression on the Q8 CURRENT-HOLDINGS variant (the missing
/// counterpart named by stage-6-part2-run-report.md §6 publication
/// rule 2: "2019-era response holdings could not secure 1,800 MW" —
/// never "GB today cannot" — holdings are a spec input awaiting this
/// run). Spec: `data/reference/fes-pathway-current-holdings.toml` —
/// the committed FES 2025 Holistic Transition table verbatim with only
/// the response holdings replaced by the three reviewed FY2025 NESO
/// dynamic LF services (data/reference/response-holdings-2025.toml,
/// contract delivery factor 1.0). Pin-before-quote (docs/05 rule 3);
/// values first measured 2026-07-03: 2024 survivable loss 2,432.86 MW
/// (min, φ=0.15) / 2,700.81 MW (mean, φ=0.35) — the 1,800 MW standard
/// is met from 2024 under both conditions and never lost (no crossing
/// years exist; the 2019-default run's "recovery in 2035/2037" framing
/// is entirely a holdings artefact). Digest pins the whole CSV data
/// section bit-identically.
#[test]
fn q8_current_holdings_pathway_reproduces_the_pinned_headline_numbers() {
    let out = out_dir("pathway-current-holdings-pin");
    let output = grid_cli(&[
        "stability",
        "pathway",
        "--pathway",
        "data/reference/fes-pathway-current-holdings.toml",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();

    // The pinned result digest (whole CSV data section, bit-identical).
    assert_eq!(
        report_value(&report, "result_digest_sha256"),
        "2c8d6997534ccd6c6e6d591cc2ede676d8e2207370c6be263536c65c4252f206"
    );

    // 2024 survivable-loss band and a mid-pathway year (2035, the
    // 2019-default mean crossing year), exact artefact values. The
    // inertia values are response-independent and must equal the
    // 2019-default run's (same fleet, same φ band).
    for block in [
        "[[results.points]]\nyear = 2024\ncondition = \"min\"\n\
         inertia_gva_s = 42.08029166666667\n\
         largest_survivable_loss_mw = 2432.861328125\n",
        "[[results.points]]\nyear = 2024\ncondition = \"mean\"\n\
         inertia_gva_s = 98.1873472222222\n\
         largest_survivable_loss_mw = 2700.8056640625\n",
        "[[results.points]]\nyear = 2035\ncondition = \"min\"\n\
         inertia_gva_s = 33.590608333333336\n\
         largest_survivable_loss_mw = 2686.1572265625\n",
        "[[results.points]]\nyear = 2035\ncondition = \"mean\"\n\
         inertia_gva_s = 78.3780861111111\n\
         largest_survivable_loss_mw = 2940.0634765625\n",
    ] {
        assert!(report.contains(block), "pinned point missing:\n{block}");
    }

    // The §2-comparison-table published cells for 2030 / 2040 / 2050
    // (q8-current-holdings-run-report.md §2, min/mean band): quoted as
    // headlines (2,485.4/2,742.3, 3,082.3/3,322.8, 3,419.2/3,648.1 MW)
    // and therefore value-pinned exactly, not left to the whole-CSV
    // digest alone. First measured 2026-07-04 (digest 2c8d6997 unmoved).
    for block in [
        "[[results.points]]\nyear = 2030\ncondition = \"min\"\n\
         inertia_gva_s = 34.815275\n\
         largest_survivable_loss_mw = 2485.3515625\n",
        "[[results.points]]\nyear = 2030\ncondition = \"mean\"\n\
         inertia_gva_s = 81.23564166666668\n\
         largest_survivable_loss_mw = 2742.3095703125\n",
        "[[results.points]]\nyear = 2040\ncondition = \"min\"\n\
         inertia_gva_s = 44.360325\n\
         largest_survivable_loss_mw = 3082.275390625\n",
        "[[results.points]]\nyear = 2040\ncondition = \"mean\"\n\
         inertia_gva_s = 103.507425\n\
         largest_survivable_loss_mw = 3322.75390625\n",
        "[[results.points]]\nyear = 2050\ncondition = \"min\"\n\
         inertia_gva_s = 44.84684166666666\n\
         largest_survivable_loss_mw = 3419.189453125\n",
        "[[results.points]]\nyear = 2050\ncondition = \"mean\"\n\
         inertia_gva_s = 104.64263055555554\n\
         largest_survivable_loss_mw = 3648.0712890625\n",
    ] {
        assert!(
            report.contains(block),
            "pinned §2 comparison cell missing:\n{block}"
        );
    }

    // Crossing years against both secured-loss standards: there are
    // none — the pathway is at or above BOTH standards from the 2024
    // start under both conditions. The 2019-default run's crossing
    // years (2035 mean / 2037 min against 1,800 MW) do not exist under
    // current holdings.
    for block in [
        "condition = \"min\"\nreference = \"sqss_infrequent_infeed_loss\"\n\
         reference_mw = 1800\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
        "condition = \"mean\"\nreference = \"sqss_infrequent_infeed_loss\"\n\
         reference_mw = 1800\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
        "condition = \"min\"\nreference = \"sqss_normal_infeed_loss\"\n\
         reference_mw = 1320\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
        "condition = \"mean\"\nreference = \"sqss_normal_infeed_loss\"\n\
         reference_mw = 1320\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
    ] {
        assert!(report.contains(block), "pinned crossing missing:\n{block}");
    }

    // The holdings are explicit spec inputs here — the artefact must
    // say so (the damping/demand defaults are still the flagged 2019
    // values, as in the base run).
    assert_eq!(
        report_value(&report, "responses_defaulted_to_2019"),
        "false"
    );
    assert_eq!(report_value(&report, "load_damping_defaulted"), "true");
}

/// Pinned regression on the delivery-factor sensitivity (0.9 uniform
/// on the three FY2025 dynamic services) — the evidence note's
/// prescribed quantification of the contract-vs-measured asymmetry
/// (2019 holdings use measured delivery factors 0.67–1.0; NESO
/// publishes no EAC performance factors, so the central variant
/// carries the contractual 1.0). Pin-before-quote; values first
/// measured 2026-07-03: 2024 survivable loss 2,282.71 MW (min) /
/// 2,527.47 MW (mean) — ~150/173 MW below the contract-factor central
/// variant, still above the 1,800 MW standard from 2024 with no
/// crossing years, so the asymmetry does not change the headline.
#[test]
fn q8_current_holdings_df090_sensitivity_reproduces_the_pinned_numbers() {
    let out = out_dir("pathway-current-holdings-df090-pin");
    let output = grid_cli(&[
        "stability",
        "pathway",
        "--pathway",
        "data/reference/fes-pathway-current-holdings-df090.toml",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();

    assert_eq!(
        report_value(&report, "result_digest_sha256"),
        "2da63eddf5e195c916349e293319a6f0390564bd9a6896c71df2b07f4c3f234c"
    );

    for block in [
        "[[results.points]]\nyear = 2024\ncondition = \"min\"\n\
         inertia_gva_s = 42.08029166666667\n\
         largest_survivable_loss_mw = 2282.71484375\n",
        "[[results.points]]\nyear = 2024\ncondition = \"mean\"\n\
         inertia_gva_s = 98.1873472222222\n\
         largest_survivable_loss_mw = 2527.4658203125\n",
        "[[results.points]]\nyear = 2035\ncondition = \"min\"\n\
         inertia_gva_s = 33.590608333333336\n\
         largest_survivable_loss_mw = 2554.3212890625\n",
        "[[results.points]]\nyear = 2035\ncondition = \"mean\"\n\
         inertia_gva_s = 78.3780861111111\n\
         largest_survivable_loss_mw = 2772.8271484375\n",
    ] {
        assert!(report.contains(block), "pinned point missing:\n{block}");
    }

    // No crossing years under the 0.9 sensitivity either.
    for block in [
        "condition = \"min\"\nreference = \"sqss_infrequent_infeed_loss\"\n\
         reference_mw = 1800\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
        "condition = \"mean\"\nreference = \"sqss_infrequent_infeed_loss\"\n\
         reference_mw = 1800\nfirst_year_below = \"never\"\nfirst_year_at_or_above = 2024\n",
    ] {
        assert!(report.contains(block), "pinned crossing missing:\n{block}");
    }

    assert_eq!(
        report_value(&report, "responses_defaulted_to_2019"),
        "false"
    );
}

/// Pinned regression on the Q8 speed-vs-volume DIAGNOSTIC run
/// (`data/reference/fes-pathway-current-holdings-2019-speed.toml`:
/// current FY2025 volumes and droops at the 2019 tranches' delay/ramp
/// envelopes, mapped by speed rank). The diagnostic is a DECOMPOSITION
/// INSTRUMENT ONLY — NOT QUOTABLE as a holdings scenario (no such
/// service mix was ever procured); its numbers exist so that the
/// difference (central current run − this run) isolates the
/// initiation/ramp timing contribution, and (this run − 2019-default
/// run) the remainder. Pin-before-quote (docs/05 rule 3): the
/// decomposition numbers derived from this run are quotable in the
/// run report only because these pins exist. Values first measured
/// 2026-07-03; digest pins the whole CSV data section bit-identically.
///
/// NOT QUOTABLE — decomposition diagnostic, not a published headline.
/// The following numbers derived from or around this run are reviewer
/// PROBE / DECOMPOSITION intermediates, NOT paper headlines, and are
/// deliberately NOT value-pinned as headlines (they are path-dependent
/// per the beta-readiness audit — the leg split in particular is stated
/// only as a range and only via the §4 sequential decomposition):
///
/// - the leg-B intermediates +38 / +89 MW (2024 min/mean delivery-factor
///   accounting) and +646 MW of held capacity relocating into the
///   sub-second rank (fast-rank effective 472 → 1,178 MW; slow rank
///   1,055 → 461 MW);
/// - the ~34–37 % : ~63–66 % leg-A/leg-B split;
/// - the droop-saturation boundary shift ≤ 0.61 MW (linear-droop
///   optimism probe);
/// - the φ-anchor stats 0.344 / 0.147 / 0.169 and 231.4 / 245.4 GVA·s.
///
/// The whole-CSV digest above still guards this run bit-identically;
/// what is refused here is elevating these intermediates to pinned,
/// independently-quotable headlines.
#[test]
fn q8_speed_diagnostic_run_reproduces_the_pinned_decomposition_numbers() {
    let out = out_dir("pathway-current-holdings-diag-pin");
    let output = grid_cli(&[
        "stability",
        "pathway",
        "--pathway",
        "data/reference/fes-pathway-current-holdings-2019-speed.toml",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();

    // The pinned result digest (whole CSV data section, bit-identical).
    assert_eq!(
        report_value(&report, "result_digest_sha256"),
        "bd8f46842cf74ca799172b19368e839deb82877284d1d0dd8fa72a5028b4f261"
    );

    // Representative points: the 2024 band and the 2035 mid-pathway
    // year, matching the central/df090 pin sets so the decomposition
    // legs are computable from pinned values alone.
    for block in [
        "[[results.points]]\nyear = 2024\ncondition = \"min\"\n\
         inertia_gva_s = 42.08029166666667\n\
         largest_survivable_loss_mw = 2056.884765625\n",
        "[[results.points]]\nyear = 2024\ncondition = \"mean\"\n\
         inertia_gva_s = 98.1873472222222\n\
         largest_survivable_loss_mw = 2283.3251953125\n",
        "[[results.points]]\nyear = 2035\ncondition = \"min\"\n\
         inertia_gva_s = 33.590608333333336\n\
         largest_survivable_loss_mw = 2340.6982421875\n",
        "[[results.points]]\nyear = 2035\ncondition = \"mean\"\n\
         inertia_gva_s = 78.3780861111111\n\
         largest_survivable_loss_mw = 2531.1279296875\n",
    ] {
        assert!(report.contains(block), "pinned point missing:\n{block}");
    }

    assert_eq!(
        report_value(&report, "responses_defaulted_to_2019"),
        "false"
    );
}

/// Pinned regression on the Q8 §3 HELD-VOLUME headlines
/// (q8-current-holdings-run-report.md §3): the FY2025 dynamic
/// low-frequency suite is **2,055 MW held** (DC-L 1,178 + DM-L 416 +
/// DR-L 461), of which **1,594 MW** (DC-L + DM-L) reaches full output
/// within 1 s; the 2019 baseline held **2,336 MW**, **1,896 MW
/// effective** after measured delivery factors. These are the
/// "speed-not-volume" comparison's load-bearing numbers and lived only
/// as committed constants in `data/reference/response-holdings-2025.toml`
/// (the three service volumes are drift-guarded in grid-stability's
/// `pathway.rs`; the comparison/total scalars were unpinned). Pinned
/// here against the committed record so they cannot drift silently.
///
/// PUBLICATION RULE (beta-audit requalification + §6.8): the held-volume
/// comparison is NOMINAL — on the effective (delivery-factor) basis the
/// engine consumes, dynamic LF volume ROSE (1,896 → 2,055 MW). Quote
/// "2,336 → 2,055 MW held" only with the effective-basis caveat; the
/// held-MW fall is not a capability fall.
#[test]
fn q8_held_volume_headlines_match_the_committed_holdings_record() {
    let text =
        std::fs::read_to_string(repo_root().join("data/reference/response-holdings-2025.toml"))
            .unwrap();
    // A single unique integer scalar (value taken before any comment).
    let scalar = |key: &str| -> i64 {
        text.lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == key)
                    .then(|| v.split('#').next().unwrap().trim().parse::<i64>().unwrap())
            })
            .unwrap_or_else(|| panic!("holdings record has no scalar {key}"))
    };
    // Per-service FY2025 mean cleared volume: the first `mw =` line
    // inside each [[services]] block, keyed by that block's name.
    let service_mw = |name: &str| -> i64 {
        text.split("[[services]]")
            .find(|block| block.contains(&format!("name = \"{name}\"")))
            .unwrap_or_else(|| panic!("no [[services]] block named {name}"))
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == "mw")
                    .then(|| v.split('#').next().unwrap().trim().parse::<i64>().unwrap())
            })
            .unwrap_or_else(|| panic!("service {name} has no mw line"))
    };

    let dc = service_mw("dynamic_containment_lf");
    let dm = service_mw("dynamic_moderation_lf");
    let dr = service_mw("dynamic_regulation_lf");
    assert_eq!(
        (dc, dm, dr),
        (1178, 416, 461),
        "FY2025 dynamic LF service volumes moved"
    );
    // 1,594 MW full-at-1s = DC-L + DM-L (the sub-second envelope pair).
    assert_eq!(dc + dm, 1594, "full-at-1s (DC-L + DM-L) headline moved");
    // 2,055 MW held total = all three; must equal the record's [totals].
    assert_eq!(dc + dm + dr, 2055, "held-total arithmetic moved");
    assert_eq!(
        scalar("lf_dynamic_mean_mw"),
        2055,
        "lf_dynamic held-total headline moved"
    );
    // The 2019 baseline comparison figures.
    assert_eq!(
        scalar("comparison_2019_held_mw"),
        2336,
        "2019 held-volume headline moved"
    );
    assert_eq!(
        scalar("comparison_2019_effective_mw"),
        1896,
        "2019 effective-volume headline moved"
    );
}

#[test]
fn pathway_runner_rejects_a_bad_spec_with_exit_2() {
    let dir = out_dir("pathway-bad");
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.toml");
    std::fs::write(
        &bad,
        "schema = \"fes-pathway-v9\"\nname = \"x\"\nfes_edition = \"y\"\n",
    )
    .unwrap();
    let output = grid_cli(&[
        "stability",
        "pathway",
        "--pathway",
        bad.to_str().unwrap(),
        "--out",
        dir.join("out").to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("fes-pathway-v1"), "stderr: {stderr}");
}

/// The Module 6 first cut through the CLI: pinned values must match the
/// grid-stability acceptance pins (inertia_sum.rs). Needs the locally
/// built 2024 data pack.
#[test]
fn inertia_finder_reproduces_the_pinned_2024_first_cut() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first",
        probe.display()
    );
    let out = out_dir("inertia");
    let output = grid_cli(&[
        "stability",
        "inertia",
        "--scenario",
        "scenarios/gb-2024-reference.toml",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["inertia.csv", "inertia.parquet", "report.toml"] {
        assert!(out.join(name).exists(), "{name} missing");
    }
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();
    // The pinned Module 6 numbers (grid-stability/tests/inertia_sum.rs).
    assert_eq!(report_value(&report, "min_inertia_gva_s"), "0");
    assert_eq!(
        report_value(&report, "min_inertia_at"),
        "2024-04-06T11:30:00Z"
    );
    assert_eq!(report_value(&report, "zero_inertia_periods"), "2");
    assert!(report.contains("floor_gva_s = 120"));
    assert!(report.contains("periods_below = 15020"));
    assert!(report.contains("hours_below = 7510"));
    assert!(report.contains("floor_gva_s = 102"));
    assert!(report.contains("periods_below = 13335"));
    // The unconstrained-dispatch caveat must be carried in the artefact
    // itself, not just the docs.
    assert!(report.contains("UNCONSTRAINED"), "caveat missing");
    // 2024 has synchronous provision; the RS finding line must NOT fire.
    assert_eq!(report_value(&report, "has_synchronous_provision"), "true");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("FINDING"), "stdout: {stdout}");
}

// ---------------------------------------------------------------------
// Module 6 (Stage 6 part 2): hours/year below the inertia floors vs
// renewable share (`stability inertia --renewable-scale`).
// ---------------------------------------------------------------------

/// One Module 6 CSV row, keyed by column name.
fn module6_rows(csv: &str) -> Vec<std::collections::BTreeMap<String, f64>> {
    let mut lines = csv.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    lines
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            header
                .iter()
                .zip(line.split(','))
                .map(|(k, v)| ((*k).to_owned(), v.parse::<f64>().unwrap()))
                .collect()
        })
        .collect()
}

/// The Module 6 sweep over renewable scaling. Endpoint pins:
/// - scale 1.0 must reproduce the part-1 first-cut counts
///   bit-identically (15,020 below 120 GVA·s; 13,335 below 102);
/// - scale 2.0 pinned after measurement (pin-after-measure, Stage 4
///   precedent — value first measured 2026-07-03 on the 2024 data
///   pack, engine at Stage 6 part 2).
///
/// Plus the monotonicity sanity: more renewables ⇒ hours below the
/// floor do not decrease. Needs the locally built 2024 data pack.
#[test]
fn module6_sweep_pins_endpoints_and_is_monotone() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first",
        probe.display()
    );
    let out = out_dir("module6");
    let output = grid_cli(&[
        "stability",
        "inertia",
        "--scenario",
        "scenarios/gb-2024-reference.toml",
        "--out",
        out.to_str().unwrap(),
        "--renewable-scale",
        "0.5,1.0,2.0",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "module6_hours_below_vs_share.csv",
        "module6_hours_below_vs_share.parquet",
        "module6_hours_below_vs_share.png",
        "report.toml",
    ] {
        assert!(out.join(name).exists(), "{name} missing");
    }
    let csv = std::fs::read_to_string(out.join("module6_hours_below_vs_share.csv")).unwrap();
    let rows = module6_rows(&csv);
    assert_eq!(rows.len(), 3);

    // Endpoint pin: the scale-1.0 point IS the part-1 first cut.
    let at = |scale: f64| {
        rows.iter()
            .find(|r| r["renewable_scale"] == scale)
            .unwrap_or_else(|| panic!("no row at scale {scale}"))
    };
    let base = at(1.0);
    assert_eq!(base["periods_below_120"], 15020.0);
    assert_eq!(base["hours_below_120"], 7510.0);
    assert_eq!(base["periods_below_102"], 13335.0);

    // Pin-after-measure at scale 2.0 (Stage 4 precedent): first
    // measured 2026-07-03 via `grid-cli stability inertia
    // --renewable-scale 0.5,1.0,2.0` on the 2024 data pack — doubling
    // wind + solar puts 16,601 periods (8,300.5 h) below 120 GVA·s and
    // 15,968 (7,984 h) below 102, at a 72.1 % potential renewable
    // share. A Stage 6 part 2 published number; it must not move.
    let doubled = at(2.0);
    assert_eq!(doubled["periods_below_120"], 16601.0);
    assert_eq!(doubled["periods_below_102"], 15968.0);

    // Monotonicity sanity: more renewables ⇒ hours below the floors do
    // not decrease (merit order: renewable output only displaces
    // synchronous plant).
    for floor in ["periods_below_120", "periods_below_102"] {
        let half = at(0.5)[floor];
        let one = at(1.0)[floor];
        let two = at(2.0)[floor];
        assert!(
            half <= one && one <= two,
            "{floor} not monotone: {half} / {one} / {two}"
        );
    }

    // The renewable-share axis (D3 convention: energy share of
    // underlying demand) must be present and increasing with scale.
    assert!(at(0.5)["renewable_share_potential"] < at(2.0)["renewable_share_potential"]);

    // The unconstrained-dispatch caveat is pinned in the artefact.
    let report = std::fs::read_to_string(out.join("report.toml")).unwrap();
    assert!(report.contains("UNCONSTRAINED"), "caveat missing");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("unconstrained"),
        "console caveat missing: {stdout}"
    );
}
