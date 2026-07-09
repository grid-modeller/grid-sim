//! Pinned characterisation tests for the DELIVERED-basis revenue and
//! capture accounting on the 2024 reference run (Package A, ratified
//! 2026-07-03: delivered basis added ALONGSIDE the potential basis —
//! old keys, values and digests unchanged; the potential-basis pins
//! live untouched in `regression_stage2_2024.rs`).
//!
//! In 2024 curtailment is 0.137 GWh over 2 periods, both priced £0
//! (must-take-only), so the two bases are nearly identical here: the
//! delivered capture ratio sits ~1e-6 above the potential one. The pin
//! exists so the delivered-basis machinery is regression-locked before
//! any delivered number is quoted (CLAUDE.md publication rule); the
//! bases only diverge visibly in high-wind sweeps.
//!
//! Requires the locally built 2024 data pack (fetched, not committed);
//! fails loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// The pinned potential-basis wind capture ratio (the Stage 2 pin,
/// repeated here only to anchor the cross-basis assertions; the
/// authoritative pin is in `regression_stage2_2024.rs`).
const PINNED_WIND_CAPTURE_RATIO: f64 = 0.9413;

/// The pinned DELIVERED-basis wind capture ratio (measured 2026-07-03,
/// first delivered-basis run), sitting 1.2e-6 above the potential-basis
/// 0.9413407336 — the 0.137 GWh of £0-priced 2024 curtailment removed
/// from the energy denominator. Pinned at ±1e-7 (the engine is
/// bit-deterministic, ADR-5): tighter than the 1.2e-6 cross-basis gap,
/// so wiring the potential series into the delivered key fails this
/// pin (Package A review condition 1b).
const PINNED_WIND_CAPTURE_RATIO_DELIVERED: f64 = 0.9413419206;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// Fail loudly if the 2024 data pack has not been built locally.
fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first: run \
         scripts/fetch-2024, scripts/era5-cf and scripts/fetch-prices",
        probe.display()
    );
}

/// Run the pinned 2024 reference dispatch + pricing once per test
/// process and return its summary.toml text.
fn pinned_run_summary() -> &'static str {
    static SUMMARY: OnceLock<String> = OnceLock::new();
    SUMMARY.get_or_init(|| {
        require_pack();
        let out_dir = std::env::temp_dir()
            .join("grid-cli-delivered-tests")
            .join("pinned-regression");
        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
            .args([
                "run",
                "--scenario",
                "scenarios/gb-2024-reference.toml",
                "--out",
                out_dir.to_str().unwrap(),
            ])
            .current_dir(repo_root())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "pinned run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(out_dir.join("summary.toml")).unwrap()
    })
}

/// Read a numeric or quoted value from our own summary.toml format.
fn summary_value(summary: &str, key: &str) -> String {
    summary
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| panic!("summary.toml has no key {key:?}"))
}

fn summary_f64(summary: &str, key: &str) -> f64 {
    summary_value(summary, key).parse().unwrap()
}

/// The delivered-basis headline pin: wind capture ratio, delivered
/// basis, on the 2024 reference run.
#[test]
fn pinned_2024_wind_capture_ratio_delivered() {
    let summary = pinned_run_summary();
    let ratio = summary_f64(summary, "wind_capture_ratio_delivered");
    assert!(
        (ratio - PINNED_WIND_CAPTURE_RATIO_DELIVERED).abs() <= 1e-7,
        "delivered-basis wind capture ratio {ratio:.10} differs from the pinned \
         {PINNED_WIND_CAPTURE_RATIO_DELIVERED} (±1e-7; the potential-basis value \
         0.9413407336 sits 1.2e-6 away and MUST fail this pin) — if the change is \
         intentional, update this pin and the Package A record together"
    );
}

/// Cross-basis invariants on the real run: the potential-basis value is
/// untouched (it keeps its Stage 2 pin), and the delivered ratio sits
/// at-or-above it (curtailment is priced £0 under SMP convention 2, so
/// removing curtailed energy from the denominator can only raise the
/// capture price — the direction worked out in
/// `grid-adequacy/tests/pricing_delivered.rs`).
#[test]
fn delivered_capture_ratio_sits_at_or_above_the_unchanged_potential_pin() {
    let summary = pinned_run_summary();
    let potential = summary_f64(summary, "wind_capture_ratio");
    let delivered = summary_f64(summary, "wind_capture_ratio_delivered");
    assert!(
        (potential - PINNED_WIND_CAPTURE_RATIO).abs() <= 0.0005,
        "potential-basis wind capture ratio {potential:.6} moved off its Stage 2 pin \
         {PINNED_WIND_CAPTURE_RATIO} — the delivered basis must be additive (defect, \
         not a re-pin opportunity)"
    );
    assert!(
        delivered >= potential,
        "delivered capture ratio {delivered:.8} below potential {potential:.8} — \
         impossible while curtailment prices at £0"
    );
    // 2024 curtailment is negligible: the bases must be near-identical.
    assert!(
        (delivered - potential).abs() < 1e-4,
        "bases diverge on the 2024 run ({delivered:.8} vs {potential:.8}) — 2024 \
         curtailment is 0.137 GWh, the difference should be ~1e-6"
    );
}

/// Per-technology delivered fields exist for the wind technologies with
/// delivered energy ≤ potential energy and identical revenue (2024
/// curtailment lives in £0 periods only).
#[test]
fn per_technology_delivered_energy_and_revenue_are_consistent() {
    let summary = pinned_run_summary();
    for tech in ["offshore_wind", "onshore_wind"] {
        let section = summary
            .split(&format!("[results.pricing.technologies.{tech}]"))
            .nth(1)
            .unwrap_or_else(|| panic!("no pricing section for {tech}"))
            .split("\n[")
            .next()
            .unwrap()
            .to_owned();
        let value = |key: &str| -> f64 {
            section
                .lines()
                .find_map(|line| {
                    let (k, v) = line.split_once('=')?;
                    (k.trim() == key).then(|| v.trim().parse::<f64>().unwrap())
                })
                .unwrap_or_else(|| panic!("{tech} section has no key {key:?}"))
        };
        let energy = value("energy_twh");
        let energy_delivered = value("energy_delivered_twh");
        assert!(
            energy_delivered <= energy,
            "{tech}: delivered energy {energy_delivered} TWh exceeds potential {energy} TWh"
        );
        let revenue = value("revenue_m_gbp");
        let revenue_delivered = value("revenue_delivered_m_gbp");
        assert_eq!(
            revenue_delivered, revenue,
            "{tech}: delivered revenue differs from potential on the 2024 run, but all \
             2024 curtailment is priced £0 — the bases' revenues must be identical"
        );
    }
}
