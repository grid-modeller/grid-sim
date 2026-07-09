//! Smoke test for `grid-cli stability validate-inertia` (Stage 6 NESO
//! enrichment, Task 7). The real pin (n / pearson_r / slope / intercept /
//! median_ratio) is Task 8's job; this only checks the subcommand parses,
//! runs against the real 2024 pack, and writes a report that carries the
//! fields it promises.
//!
//! Requires the locally built 2024 data pack (fetched, not committed);
//! fails loudly if it is absent (the `require_pack` idiom from
//! `regression_2024.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

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

#[test]
fn validate_inertia_runs_against_the_2024_pack() {
    require_pack();
    let out_path = std::env::temp_dir()
        .join("grid-cli-stability-tests")
        .join("validate-inertia-smoke.toml");
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
    assert!(
        report.contains("pearson_r"),
        "report is missing pearson_r:\n{report}"
    );
}
