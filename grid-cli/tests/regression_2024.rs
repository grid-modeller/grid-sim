//! Pinned regression tests for the published Stage 1 numbers
//! (CLAUDE.md rule: every published number gets a pinned regression test
//! — scenario file + expected output — before it is quoted anywhere).
//!
//! The pinned run is the Stage 1 honesty-gate run of the 2024 reference
//! scenario, recorded in `docs/notes/stage-1-2024-run-report.md`
//! (implementer-produced, reviewer re-measured, 2026-07-02). Two
//! independent pins, so a change to the digest algorithm or CSV
//! formatting cannot silently orphan the physical number:
//!
//! - the result digest (SHA-256 over the dispatch.csv data section as
//!   `grid-cli run` writes it) — engine-behaviour-stable, sensitive to
//!   any change in dispatch arithmetic, column set/order, or number
//!   formatting;
//! - the annual gas total, 73.45 TWh (CCGT + OCGT; +0.91 % vs the 72.79
//!   TWh actual), pinned to ±0.01 TWh.
//!
//! Any intentional engine change that moves these must update both this
//! test and the run report — that is the point.
//!
//! Requires the locally built 2024 data pack (fetched, not committed);
//! fails loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// The pinned result digest of the 2024 reference run.
///
/// Re-pin record (the ONE permitted update per the Stage 3 work order;
/// the schema-v2 package landed uncommitted, so this is a single
/// visible re-pin):
/// - old (Stage 1, 2026-07-02, docs/notes/stage-1-2024-run-report.md):
///   `6f82c7b00b0ef592e3f20149c6c6cfd3254709c45445857a8e1678836c088c5d`
/// - new (Stage 3 schema v2 + reliability accounting, 2026-07-02):
///   `779d7444577b0ef1d2201835fd36616c4eed8bfab0d58c3c82c19d0ac2541abd`
/// - cause: dispatch.csv gains six per-store columns (charge/discharge
///   GW, SoC GWh for pumped_hydro and battery; schema v2 active
///   storage) and four reliability-accounting columns (firm_supply_gw,
///   variable_supply_gw, storage_discharge_gw, firm_share —
///   gb-grid-margin methodology, pure accounting). The PHYSICAL series
///   are bit-identical to the Stage 1 run — under D4 the stores never
///   act on 2024 data (initially full, no post-stack deficit all
///   year), so gas, imports, monthly mix, curtailment (0.137 GWh — NOT
///   absorbed: full stores have no headroom) and unserved are
///   unchanged, and the physical pin below passes UNMOVED. The Stage 2
///   prices digest also did not move (regression_stage2_2024.rs).
const PINNED_DIGEST: &str = "779d7444577b0ef1d2201835fd36616c4eed8bfab0d58c3c82c19d0ac2541abd";

/// The pinned Stage 1 annual gas generation, TWh (CCGT + OCGT;
/// docs/notes/stage-1-2024-run-report.md §1).
const PINNED_GAS_TWH: f64 = 73.45;

/// The pinned 2024 reference-run mean firm share of demand (unclamped;
/// gb-grid-margin methodology, first measured 2026-07-02). This is the
/// MODEL's dispatched firm share — gas is dispatched down in windy
/// periods, so it reads lower than an installed-capacity margin.
const PINNED_FIRM_SHARE_MEAN: f64 = 0.5149753201865261;

// --- Load-bearing exogenous wedges and headline energy balance
// (docs/notes/stage-1-2024-run-report.md §1/§3). These are published
// Stage 1 numbers; the CLAUDE.md rule requires each to carry an
// exact-value pin, not merely transitive digest coverage. Measured from
// the pinned run's summary.toml (2026-07-04); the engine is
// bit-deterministic (ADR-5), so the tolerances catch drift, not noise.

/// FUELHH "other" wedge, TWh — the exogenous must-take trace of real but
/// unrepresentable 2024 generation (waste, small CHP). THE MOST
/// LOAD-BEARING unpinned number in the project: with this wedge removed
/// the whole category lands on modelled gas and the ±5 % gas gate FAILS
/// at 76.65 TWh / +5.30 % (pinned as a counterfactual in
/// `grid-adequacy/tests/acceptance_2024.rs`). Report §3.
const PINNED_OTHER_TWH: f64 = 3.34968;

/// Pumped-storage net exogenous supply, TWh (negative = net pumping
/// load). Its magnitude is the 0.60 TWh annual round-trip loss the
/// scenario keeps OFF modelled gas by carrying PS as an observed
/// must-take trace (report §3, wedge 2).
const PINNED_PUMPED_STORAGE_NET_TWH: f64 = -0.600611;

/// Net annual imports, TWh — the exogenous trace-plumbing check
/// (−0.003 % vs the 33.30 TWh observed; report §1).
const PINNED_NET_IMPORTS_TWH: f64 = 33.298844;

/// Total pooled curtailment, TWh — 0.137 GWh over 2 periods, the
/// tightest-surplus moment on the 2024 run (report §1). Pinned in TWh as
/// the summary reports it.
const PINNED_CURTAILMENT_TWH: f64 = 0.00013670684844255376;

/// Per-fuel annual energies, TWh (report §1). Wind is pinned as the
/// offshore + onshore total (82.61 TWh, the D3 convention headline);
/// solar and nuclear/biomass/hydro/coal are the modelled annual series;
/// OCGT is pinned separately from the CCGT+OCGT gas gate to catch a
/// silent shift within the gas split.
const PINNED_WIND_TWH: f64 = 82.61000460683046; // 45.58454935 + 37.02545526
const PINNED_SOLAR_TWH: f64 = 13.950000267693742;
const PINNED_NUCLEAR_TWH: f64 = 38.241173598707306;
const PINNED_BIOMASS_TWH: f64 = 18.501645897495365;
const PINNED_HYDRO_TWH: f64 = 3.4234944077155025;
const PINNED_COAL_TWH: f64 = 1.460006939611193;
const PINNED_OCGT_TWH: f64 = 0.0010985914313209123;

/// The ERA5 one-factor-per-technology calibration constants baked into
/// the CF parquet traces at pack-build time (`scripts/era5-cf`,
/// docs/notes/era5-cf-2024-report.md). There is NO Rust engine path that
/// applies these — the scenario points `capacity_factor_trace` at the
/// already-calibrated parquet — so they are PACK-BUILD-PINNED ONLY: this
/// characterisation test reads them back from the pack's
/// `era5_cf_report_2024.json` and guards against a silent re-derivation.
/// (offshore, onshore, solar), report `technologies.*.calibration_factor`.
const PINNED_CF_FACTORS: [(&str, f64); 3] =
    [("offshore", 0.8975), ("onshore", 1.0395), ("solar", 0.8837)];

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// Fail loudly if the 2024 data pack has not been built locally.
fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first: run \
         scripts/fetch-2024 (fetch.py, build.py) and scripts/era5-cf \
         (fetch_era5.py, derive_cf.py)",
        probe.display()
    );
}

/// Run the pinned 2024 reference dispatch once per test process and
/// return its summary.toml text.
fn pinned_run_summary() -> &'static str {
    static SUMMARY: OnceLock<String> = OnceLock::new();
    SUMMARY.get_or_init(|| {
        require_pack();
        let out_dir = std::env::temp_dir()
            .join("grid-cli-stage1-tests")
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

#[test]
fn pinned_2024_reference_result_digest() {
    let summary = pinned_run_summary();
    let digest = summary_value(summary, "result_digest_sha256");
    assert_eq!(
        digest, PINNED_DIGEST,
        "the 2024 reference run's result digest moved — if the engine \
         change is intentional, update this pin AND \
         docs/notes/stage-1-2024-run-report.md together"
    );
}

#[test]
fn pinned_2024_reference_firm_share_mean() {
    let summary = pinned_run_summary();
    let mean: f64 = summary_value(summary, "firm_share_mean").parse().unwrap();
    assert!(
        (mean - PINNED_FIRM_SHARE_MEAN).abs() <= 1e-9,
        "2024 reference mean firm share {mean} differs from the pinned \
         {PINNED_FIRM_SHARE_MEAN} — classification is pure accounting over a \
         frozen dispatch, so any move means either the dispatch or the \
         classification changed; update this pin only with the record"
    );
}

#[test]
fn pinned_2024_reference_annual_gas_twh() {
    let summary = pinned_run_summary();
    let ccgt: f64 = summary_value(summary, "ccgt").parse().unwrap();
    let ocgt: f64 = summary_value(summary, "ocgt").parse().unwrap();
    let gas = ccgt + ocgt;
    assert!(
        (gas - PINNED_GAS_TWH).abs() <= 0.01,
        "annual gas {gas:.4} TWh differs from the pinned {PINNED_GAS_TWH} TWh \
         (±0.01; docs/notes/stage-1-2024-run-report.md §1) — if the engine \
         change is intentional, update this pin AND the run report together"
    );
}

/// The three load-bearing exogenous wedges (§3) and the imports/curtailment
/// balance (§1), each pinned to its exact 2024-run value.
#[test]
fn pinned_2024_reference_exogenous_wedges_and_balance() {
    let summary = pinned_run_summary();

    let other = summary_f64(summary, "other");
    assert!(
        (other - PINNED_OTHER_TWH).abs() <= 1e-3,
        "FUELHH 'other' wedge {other:.5} TWh differs from the pinned \
         {PINNED_OTHER_TWH} TWh (±1e-3) — this wedge is load-bearing (the gas \
         gate fails without it); update the pin AND the run report together"
    );

    let ps = summary_f64(summary, "pumped_storage_net");
    assert!(
        (ps - PINNED_PUMPED_STORAGE_NET_TWH).abs() <= 1e-3,
        "pumped-storage net {ps:.5} TWh differs from the pinned \
         {PINNED_PUMPED_STORAGE_NET_TWH} TWh (±1e-3; |value| = 0.60 TWh \
         round-trip loss)"
    );

    let imports = summary_f64(summary, "net_imports");
    assert!(
        (imports - PINNED_NET_IMPORTS_TWH).abs() <= 1e-3,
        "net imports {imports:.5} TWh differ from the pinned \
         {PINNED_NET_IMPORTS_TWH} TWh (±1e-3)"
    );

    let curtailment = summary_f64(summary, "curtailment_twh");
    assert!(
        (curtailment - PINNED_CURTAILMENT_TWH).abs() <= 1e-6,
        "curtailment {curtailment:.9} TWh differs from the pinned \
         {PINNED_CURTAILMENT_TWH} TWh (±1e-6; 0.137 GWh over 2 periods)"
    );
}

/// Per-fuel annual energies (§1), pinned exactly. Wind is the offshore +
/// onshore total (the D3 headline); OCGT is pinned separately from the
/// CCGT+OCGT gas gate.
#[test]
fn pinned_2024_reference_per_fuel_annual_energies() {
    let summary = pinned_run_summary();
    let wind = summary_f64(summary, "offshore_wind") + summary_f64(summary, "onshore_wind");
    for (label, actual, pinned, tol) in [
        ("wind (offshore+onshore)", wind, PINNED_WIND_TWH, 1e-3),
        (
            "solar",
            summary_f64(summary, "solar"),
            PINNED_SOLAR_TWH,
            1e-3,
        ),
        (
            "nuclear",
            summary_f64(summary, "nuclear"),
            PINNED_NUCLEAR_TWH,
            1e-3,
        ),
        (
            "biomass",
            summary_f64(summary, "biomass"),
            PINNED_BIOMASS_TWH,
            1e-3,
        ),
        (
            "hydro",
            summary_f64(summary, "hydro"),
            PINNED_HYDRO_TWH,
            1e-3,
        ),
        ("coal", summary_f64(summary, "coal"), PINNED_COAL_TWH, 1e-3),
        ("ocgt", summary_f64(summary, "ocgt"), PINNED_OCGT_TWH, 1e-5),
    ] {
        assert!(
            (actual - pinned).abs() <= tol,
            "{label} annual energy {actual} TWh differs from the pinned \
             {pinned} TWh (±{tol})"
        );
    }
}

/// Characterisation: the ERA5 one-factor-per-technology calibration
/// constants baked into the CF traces at pack-build time. No Rust engine
/// path applies these (the scenario consumes the already-calibrated
/// parquet), so this reads them back from the pack's committed derivation
/// report and pins them — pack-build-pinned only.
#[test]
fn pinned_2024_cf_calibration_factors_are_pack_build_pinned() {
    require_pack();
    let report_path = repo_root().join("data/packs/2024/processed/era5_cf_report_2024.json");
    let json = std::fs::read_to_string(&report_path).unwrap();
    // Minimal targeted extraction from the machine-generated report: for
    // each technology block, read its `calibration_factor` number. Kept
    // dependency-free deliberately (this is a pack-provenance guard). The
    // tech names (offshore/onshore/solar) also appear as keys in earlier
    // sections (monthly_r/monthly_twh), so anchor the search inside the
    // `technologies` block first.
    let tech_block_start = json
        .find("\"technologies\"")
        .expect("era5 report has a technologies block");
    let tech_block = &json[tech_block_start..];
    let factor = |tech: &str| -> f64 {
        let anchor = format!("\"{tech}\":");
        let from = tech_block
            .find(&anchor)
            .unwrap_or_else(|| panic!("no \"{tech}\" block in {}", report_path.display()));
        let needle = "\"calibration_factor\":";
        let at = tech_block[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("no calibration_factor for {tech}"))
            + from
            + needle.len();
        let end = tech_block[at..].find(',').unwrap() + at;
        tech_block[at..end].trim().parse().unwrap()
    };
    for (tech, pinned) in PINNED_CF_FACTORS {
        let f = factor(tech);
        assert!(
            (f - pinned).abs() <= 1e-6,
            "{tech} ERA5 calibration factor {f} differs from the pinned \
             {pinned} (±1e-6) — a re-derivation of the pack moved it; update \
             this pin AND docs/notes/era5-cf-2024-report.md together"
        );
    }
}
