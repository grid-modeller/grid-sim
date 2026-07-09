//! Stage 5 CLI tests: the multi-zone `run` output set and the Module 5
//! `plot capacity-credit` artefact on the 5-zone 2024 scenario.
//! Data-gated (fetched/derived packs); fails loudly if they are absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn require_stage5_packs() {
    for rel in [
        "data/packs/2024/processed/demand_2024.parquet",
        "data/packs/entsoe-2024/processed/load_fr_2024.parquet",
        "data/packs/cf-eu/fr/fr_onshore_cf_2024.parquet",
        "data/packs/cf-eu-1985-2024.sha256",
    ] {
        assert!(
            repo_root().join(rel).exists(),
            "Stage 5 data packs incomplete ({rel} missing) — build them first \
             (scripts/fetch-2024 + scripts/era5-cf + scripts/fetch-entsoe + \
             scripts/era5-cf/derive_cf_eu.py)"
        );
    }
}

#[test]
fn multi_zone_run_writes_per_zone_and_link_outputs() {
    require_stage5_packs();
    let out_dir = std::env::temp_dir()
        .join("grid-cli-stage5-tests")
        .join("run-5zone");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "run",
            "--scenario",
            "scenarios/gb-2024-5zone.toml",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "multi-zone run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // docs/06: CSV and Parquet, both, always — per zone plus links.
    for zone in ["GB", "FR", "CONT-NW", "NO2", "DK1", "IE-SEM"] {
        assert!(out_dir.join(format!("dispatch_{zone}.csv")).exists());
        assert!(out_dir.join(format!("dispatch_{zone}.parquet")).exists());
    }
    assert!(out_dir.join("links.csv").exists());
    assert!(out_dir.join("links.parquet").exists());
    let summary = std::fs::read_to_string(out_dir.join("summary.toml")).unwrap();
    for needle in [
        "[results.zones.\"GB\"]",
        "[results.link_flows.\"NSL\"]",
        "net_imports_twh",
        "links_digest_sha256",
    ] {
        assert!(summary.contains(needle), "summary lacks {needle}");
    }
}

#[test]
fn capacity_credit_artefact_is_produced() {
    require_stage5_packs();
    let out_dir = std::env::temp_dir()
        .join("grid-cli-stage5-tests")
        .join("module5");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "plot",
            "capacity-credit",
            "--scenario",
            "scenarios/gb-2024-5zone.toml",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "plot capacity-credit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.join("capacity_credit.png").exists());
    let csv = std::fs::read_to_string(out_dir.join("capacity_credit.csv")).unwrap();
    // 20 bins + header (+ metadata comments).
    let rows = csv.lines().filter(|l| !l.starts_with('#')).count();
    assert_eq!(rows, 21, "expected header + 20 percentile bins");
    assert!(csv.contains("NSL_mean_net_import_gw"));
}
