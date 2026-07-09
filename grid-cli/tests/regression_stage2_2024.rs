//! Pinned regression tests for the Stage 2 pricing numbers (CLAUDE.md
//! rule: every published number gets a pinned regression test before it
//! is quoted anywhere), same pattern as `regression_2024.rs`.
//!
//! The pinned run is the 2024 reference scenario priced with the Stage 2
//! `[pricing]` section (first measured 2026-07-02). Three independent
//! pins:
//!
//! - the **prices digest** (SHA-256 over the prices.csv data section as
//!   `grid-cli run` writes it) — sensitive to any change in the SRMC
//!   recipe, the SMP conventions, column set/order, or number
//!   formatting;
//! - the **gas price-setting share**, 93.89 % of periods — the model's
//!   behavioural gas-marginal flag. The docs/04 Stage 2 gate was
//!   re-pinned 2026-07-02 to the observable's own definition band
//!   [89.4 %, 99.8 %] (the original ±3-points-of-99.4 % pin was
//!   unsatisfiable by dispatch arithmetic — re-pin record in docs/04 and
//!   `docs/notes/stage-2-2024-run-report.md`), so this value PASSES its
//!   gate (`grid-adequacy/tests/acceptance_stage2_2024.rs`); it stays
//!   pinned exactly here so any engine change that moves it is caught;
//! - the **wind capture ratio**, 0.9413 (model SMP, model D3 total
//!   wind; observed benchmark 0.899 ± 0.05 — PASSES its gate).
//!
//! The Stage 1 dispatch digest is pinned separately in
//! `regression_2024.rs` and must not move when pricing is added — that
//! is asserted there, not here.
//!
//! Requires the locally built 2024 data pack (fetched, not committed);
//! fails loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// The pinned Stage 2 prices digest (measured 2026-07-02).
///
/// Deliberately UNCHANGED by the Stage 3 schema-v2 migration
/// (2026-07-02): prices.csv carries only the SMP, price-setter and SRMC
/// columns, none of which the (inactive on 2024 data) storage portfolio
/// touches — the dispatch digest moved (new store columns; re-pin
/// record in regression_2024.rs), this one did not.
const PINNED_PRICES_DIGEST: &str =
    "1d38ed7513340bfc2323e710883a4d67822ac95fc6a436b652329671f809538d";

/// The pinned model gas price-setting share, % of periods.
const PINNED_GAS_PRICE_SETTING_PCT: f64 = 93.89;

/// The pinned model wind capture ratio.
const PINNED_WIND_CAPTURE_RATIO: f64 = 0.9413;

// --- Published Stage 2 headline numbers that were only band-gated (in
// `grid-adequacy/tests/acceptance_stage2_2024.rs`) or comment-only until
// now. The CLAUDE.md rule requires an exact-value pin so they cannot
// drift silently inside a passing band. Measured 2026-07-04 from the
// pinned run's summary.toml.

/// Mean (time-weighted) model system marginal price, £/MWh — the 2024
/// price-level headline (report §2). Had NO pin anywhere before this.
const PINNED_MEAN_SMP_GBP_PER_MWH: f64 = 74.5409068500577;

/// Median model-SMP / observed-MID ratio — the price-level realism gate
/// (report §2), previously band-gated [0.90, 1.10] only.
const PINNED_MEDIAN_SMP_OVER_MID: f64 = 1.0099582902644124;

/// Monthly model-vs-observed price correlation — the price-shape realism
/// gate (report §2), previously gated ≥ 0.85 only.
const PINNED_MONTHLY_PRICE_CORR: f64 = 0.9516563827058433;

/// Annual emissions, MtCO2 (pricing basis) and MtCO2e (accounting
/// basis), gas fleet only (report §2). Previously only the CO2e/CO2
/// factor ratio and a wide 25–35 Mt band were pinned.
const PINNED_TOTAL_CO2_MT: f64 = 27.40044772506372;
const PINNED_TOTAL_CO2E_MT: f64 = 27.455990187444005;

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
            .join("grid-cli-stage2-tests")
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

#[test]
fn pinned_2024_prices_digest() {
    let summary = pinned_run_summary();
    let digest = summary_value(summary, "prices_digest_sha256");
    assert_eq!(
        digest, PINNED_PRICES_DIGEST,
        "the 2024 reference run's prices digest moved — if the pricing \
         change is intentional, update this pin AND the Stage 2 record \
         together"
    );
}

#[test]
fn pinned_2024_gas_price_setting_share() {
    let summary = pinned_run_summary();
    let share: f64 = summary_value(summary, "pct_periods_gas_price_setting")
        .parse()
        .unwrap();
    assert!(
        (share - PINNED_GAS_PRICE_SETTING_PCT).abs() <= 0.01,
        "gas price-setting share {share:.4} % differs from the pinned \
         {PINNED_GAS_PRICE_SETTING_PCT} % (±0.01)"
    );
}

#[test]
fn pinned_2024_wind_capture_ratio() {
    let summary = pinned_run_summary();
    let ratio: f64 = summary_value(summary, "wind_capture_ratio")
        .parse()
        .unwrap();
    assert!(
        (ratio - PINNED_WIND_CAPTURE_RATIO).abs() <= 0.0005,
        "wind capture ratio {ratio:.5} differs from the pinned \
         {PINNED_WIND_CAPTURE_RATIO} (±0.0005)"
    );
}

/// Mean model SMP, median-ratio and monthly-correlation price realism
/// numbers, and the annual emissions totals — each pinned exactly (they
/// were band-gated or comment-only before).
#[test]
fn pinned_2024_price_realism_and_emissions_headlines() {
    let summary = pinned_run_summary();
    let f = |key: &str| -> f64 { summary_value(summary, key).parse().unwrap() };

    let mean_smp = f("smp_time_weighted_mean_gbp_per_mwh");
    assert!(
        (mean_smp - PINNED_MEAN_SMP_GBP_PER_MWH).abs() <= 1e-3,
        "mean SMP £{mean_smp:.4}/MWh differs from the pinned \
         £{PINNED_MEAN_SMP_GBP_PER_MWH}/MWh (±1e-3)"
    );

    let median = f("median_model_smp_over_observed_mid");
    assert!(
        (median - PINNED_MEDIAN_SMP_OVER_MID).abs() <= 1e-4,
        "median model-SMP/observed-MID {median:.6} differs from the pinned \
         {PINNED_MEDIAN_SMP_OVER_MID} (±1e-4; band gate [0.90,1.10] unchanged \
         in acceptance_stage2_2024.rs)"
    );

    let corr = f("monthly_corr_model_smp_vs_observed_mid");
    assert!(
        (corr - PINNED_MONTHLY_PRICE_CORR).abs() <= 1e-4,
        "monthly price correlation {corr:.6} differs from the pinned \
         {PINNED_MONTHLY_PRICE_CORR} (±1e-4; ≥0.85 gate unchanged)"
    );

    let co2 = f("total_co2_mt");
    assert!(
        (co2 - PINNED_TOTAL_CO2_MT).abs() <= 1e-3,
        "total CO2 {co2:.5} Mt differs from the pinned {PINNED_TOTAL_CO2_MT} Mt \
         (±1e-3; gas fleet only)"
    );
    let co2e = f("total_co2e_mt");
    assert!(
        (co2e - PINNED_TOTAL_CO2E_MT).abs() <= 1e-3,
        "total CO2e {co2e:.5} Mt differs from the pinned {PINNED_TOTAL_CO2E_MT} Mt \
         (±1e-3; gas fleet only)"
    );
}
