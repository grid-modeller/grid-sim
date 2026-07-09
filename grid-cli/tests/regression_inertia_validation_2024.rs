//! Pinned regression tests for the Stage 6 NESO enrichment inertia
//! characterisation (CLAUDE.md rule: every published number gets a pinned
//! regression test before it is quoted anywhere).
//!
//! `inertia_method_tracks_neso_2024` pins the Task 7 full-year 2024 run of
//! `grid-cli stability validate-inertia`, correlating the bottom-up
//! `grid_stability::inertia_from_generation` estimate against the NESO
//! System Inertia outturn series, recorded in
//! `docs/notes/stage-6-inertia-validation-run-report.md` §1.
//!
//! `engine_inertia_tracks_neso_2024` pins the Task 9 full-year 2024 run of
//! `grid-cli stability inertia --reference <neso parquet>`, correlating the
//! engine's own dispatch-derived `inertia_series` (the canonical
//! `scenarios/gb-2024-reference.toml` scenario) against the same NESO
//! outturn series, recorded in
//! `docs/notes/stage-6-inertia-validation-run-report.md` §5.
//!
//! Any intentional engine change that moves these must update both this
//! test and the run report — that is the point.
//!
//! Requires the locally built 2024 data pack (fetched, not committed);
//! fails loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

/// The pinned matched-period count (docs/notes/stage-6-inertia-validation-run-report.md §1).
const PINNED_N: usize = 17568;

/// The pinned Pearson r between the bottom-up and NESO outturn series.
const PINNED_PEARSON_R: f64 = 0.9575443124106625;

/// The pinned OLS slope (`neso ≈ slope·ours + intercept`).
const PINNED_SLOPE: f64 = 1.5427278883728945;

/// The pinned OLS intercept, GVA·s.
const PINNED_INTERCEPT: f64 = 53.333651518948145;

/// The pinned median ratio (`neso[i] / ours[i]`, nonzero `ours` only).
const PINNED_MEDIAN_RATIO: f64 = 2.2537730575740635;

/// The pinned matched-period count for the Task 9 engine-vs-NESO run
/// (docs/notes/stage-6-inertia-validation-run-report.md §5): both series
/// cover the same full 2024 year, so every period matched.
const PINNED_ENGINE_N: usize = 17568;

/// The pinned Pearson r between the engine's own dispatch-derived
/// `inertia_series` (canonical `scenarios/gb-2024-reference.toml` run)
/// and the NESO outturn series
/// (docs/notes/stage-6-inertia-validation-run-report.md §5). Lower than
/// `PINNED_PEARSON_R` (the method-level check) is expected: the scenario
/// dispatch is cost-optimised, not a reconstruction of actual 2024 unit
/// commitment.
const PINNED_ENGINE_PEARSON_R: f64 = 0.9372226149035376;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// Fail loudly if the 2024 data pack has not been built locally.
fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/generation_by_fuel_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first: run \
         scripts/fetch-2024 (fetch.py, build.py)",
        probe.display()
    );
}

/// Read a numeric or quoted value from our own report.toml format.
fn report_value(report: &str, key: &str) -> String {
    report
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| panic!("report.toml has no key {key:?}"))
}

#[test]
fn inertia_method_tracks_neso_2024() {
    require_pack();
    let out_path = std::env::temp_dir()
        .join("grid-cli-stability-tests")
        .join("regression-validate-inertia-2024.toml");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    if out_path.exists() {
        std::fs::remove_file(&out_path).unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "stability",
            "validate-inertia",
            "--base-dir",
            ".",
            "--out",
            out_path.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "validate-inertia failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = std::fs::read_to_string(&out_path).unwrap();

    let n: usize = report_value(&report, "n").parse().unwrap();
    assert_eq!(
        n, PINNED_N,
        "matched-period count {n} differs from the pinned {PINNED_N} \
         (docs/notes/stage-6-inertia-validation-run-report.md §1) — update \
         the pin AND the run report together"
    );

    let pearson_r: f64 = report_value(&report, "pearson_r").parse().unwrap();
    assert!(
        (pearson_r - PINNED_PEARSON_R).abs() <= 0.005,
        "Pearson r {pearson_r} differs from the pinned {PINNED_PEARSON_R} \
         (±0.005; docs/notes/stage-6-inertia-validation-run-report.md §1) — \
         update the pin AND the run report together"
    );

    let slope: f64 = report_value(&report, "slope").parse().unwrap();
    assert!(
        (slope - PINNED_SLOPE).abs() <= 0.02,
        "OLS slope {slope} differs from the pinned {PINNED_SLOPE} (±0.02; \
         docs/notes/stage-6-inertia-validation-run-report.md §1) — update \
         the pin AND the run report together"
    );

    let intercept: f64 = report_value(&report, "intercept").parse().unwrap();
    assert!(
        (intercept - PINNED_INTERCEPT).abs() <= 1.0,
        "OLS intercept {intercept} GVA·s differs from the pinned \
         {PINNED_INTERCEPT} GVA·s (±1.0; \
         docs/notes/stage-6-inertia-validation-run-report.md §1) — update \
         the pin AND the run report together"
    );

    let median_ratio: f64 = report_value(&report, "median_ratio").parse().unwrap();
    assert!(
        (median_ratio - PINNED_MEDIAN_RATIO).abs() <= 0.02,
        "median ratio {median_ratio} differs from the pinned \
         {PINNED_MEDIAN_RATIO} (±0.02; \
         docs/notes/stage-6-inertia-validation-run-report.md §1) — update \
         the pin AND the run report together"
    );
}

/// Task 9: the engine's own dispatch-derived `inertia_series` (the
/// canonical 2024 reference scenario) vs the NESO System Inertia
/// outturn series, via `grid-cli stability inertia --reference`.
#[test]
fn engine_inertia_tracks_neso_2024() {
    require_pack();
    let out_dir = std::env::temp_dir()
        .join("grid-cli-stability-tests")
        .join("regression-engine-inertia-2024");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }

    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "stability",
            "inertia",
            "--scenario",
            "scenarios/gb-2024-reference.toml",
            "--base-dir",
            ".",
            "--out",
            out_dir.to_str().unwrap(),
            "--reference",
            "data/packs/2024/processed/inertia_outturn_2024.parquet",
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stability inertia --reference failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report = std::fs::read_to_string(out_dir.join("report.toml")).unwrap();

    let n: usize = report_value(&report, "n").parse().unwrap();
    assert_eq!(
        n, PINNED_ENGINE_N,
        "engine-vs-NESO matched-period count {n} differs from the pinned \
         {PINNED_ENGINE_N} \
         (docs/notes/stage-6-inertia-validation-run-report.md §5) — update \
         the pin AND the run report together"
    );

    let pearson_r: f64 = report_value(&report, "pearson_r").parse().unwrap();
    assert!(
        (pearson_r - PINNED_ENGINE_PEARSON_R).abs() <= 0.01,
        "engine-vs-NESO Pearson r {pearson_r} differs from the pinned \
         {PINNED_ENGINE_PEARSON_R} (±0.01; \
         docs/notes/stage-6-inertia-validation-run-report.md §5) — update \
         the pin AND the run report together"
    );
}
