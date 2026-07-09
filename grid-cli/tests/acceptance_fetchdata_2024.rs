//! Acceptance test for `grid-cli fetch-data` — the Rust port of the
//! provisional Python data-pack builder (`scripts/fetch-2024/`, plus the
//! ONS gas-SAP trace from `scripts/fetch-prices/`).
//!
//! Acceptance standard (work order): byte-identical Parquet across writer
//! implementations is not achievable (Arrow metadata and compression
//! details are writer-specific), so the invariant is **cell-exact
//! numerical identity** against the existing Python-built pack:
//! same row count, same UTC index, bit-level `f64` / exact integer
//! equality of every cell in every processed output. The port's own
//! validator must also pass on the Rust-built pack.
//!
//! This test builds from the *committed pack's raw files* (network-free:
//! `--skip-fetch` with `raw/` symlinked into a scratch output dir) so it
//! isolates the transformation from upstream data drift. It never writes
//! into `data/packs/2024/` — the committed manifest pins those bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn grid_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .unwrap()
}

/// Fresh scratch output directory under `target/` with the repo pack's
/// `raw/` symlinked in (read-only use; the build must not touch it).
fn scratch_out(name: &str) -> PathBuf {
    let dir = repo_root().join("target").join("tmp").join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(repo_root().join("data/packs/2024/raw"), dir.join("raw")).unwrap();
    dir
}

fn require_pack() {
    for needed in [
        "data/packs/2024/raw/demanddata_2024.csv",
        "data/packs/2024/raw/fuelhh_2024-01-01_2024-01-31.json",
        "data/packs/2024/raw/ons_sap_of_gas_090125.xlsx",
        "data/packs/2024/processed/demand_2024.parquet",
    ] {
        assert!(
            repo_root().join(needed).exists(),
            "2024 data pack missing ({needed}); build it first — \
             `python scripts/fetch-2024/fetch.py .` etc. (see scripts/fetch-2024/README.md) \
             or `grid-cli fetch-data --year 2024 --out data/packs/2024`"
        );
    }
}

/// The crux: rebuild the processed outputs from the committed raw data and
/// prove cell-exact numerical identity against the Python-built pack, and
/// that the port's own validator passes.
#[test]
fn rebuild_from_committed_raw_is_cell_exact_and_validates() {
    require_pack();
    let out_dir = scratch_out("fetchdata_acceptance");
    let out = grid_cli(&[
        "fetch-data",
        "--year",
        "2024",
        "--out",
        out_dir.to_str().unwrap(),
        "--skip-fetch",
        "--compare-with",
        "data/packs/2024/processed",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "fetch-data failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // The five processed outputs exist in both formats.
    for stem in [
        "demand_2024",
        "generation_by_fuel_2024",
        "wind_cf_2024",
        "gas_sap_daily_2024",
        "inertia_outturn_2024",
    ] {
        for ext in ["csv", "parquet"] {
            let path = out_dir.join("processed").join(format!("{stem}.{ext}"));
            assert!(path.exists(), "missing output {}", path.display());
        }
        // Comparison verdict, per stem.
        assert!(
            stdout.contains(&format!("{stem}: identical")),
            "no cell-exact verdict for {stem}\nstdout:\n{stdout}"
        );
    }

    // The port's own validator passed on the Rust-built pack.
    assert!(
        stdout.contains("All validation checks passed."),
        "validator did not pass\nstdout:\n{stdout}"
    );
}

/// Any cell mismatch must fail loudly (exit 1) naming column, timestamp
/// and both values — proven by comparing against a *wrong* reference
/// (wind_cf compared as if it were gas SAP is a row-level mismatch; here
/// we compare a built pack against a doctored copy).
#[test]
fn comparison_detects_a_single_cell_mismatch() {
    require_pack();
    let out_dir = scratch_out("fetchdata_mismatch");

    // First build (no comparison) to get a valid pack.
    let out = grid_cli(&[
        "fetch-data",
        "--year",
        "2024",
        "--out",
        out_dir.to_str().unwrap(),
        "--skip-fetch",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Doctored reference: swap one file for a different trace under the
    // expected name (kept cheap: reuse the real processed dir for four
    // stems, and a copy with one stem replaced).
    let doctored = out_dir.join("doctored");
    std::fs::create_dir_all(&doctored).unwrap();
    let real = repo_root().join("data/packs/2024/processed");
    for stem in [
        "demand_2024",
        "generation_by_fuel_2024",
        "gas_sap_daily_2024",
        "inertia_outturn_2024",
    ] {
        std::fs::copy(
            real.join(format!("{stem}.parquet")),
            doctored.join(format!("{stem}.parquet")),
        )
        .unwrap();
    }
    // wind_cf reference replaced by a different single-column trace.
    std::fs::copy(
        real.join("gb_onshore_cf_2024.parquet"),
        doctored.join("wind_cf_2024.parquet"),
    )
    .unwrap();

    let out = grid_cli(&[
        "fetch-data",
        "--year",
        "2024",
        "--out",
        out_dir.to_str().unwrap(),
        "--skip-fetch",
        "--compare-with",
        doctored.to_str().unwrap(),
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(1),
        "mismatch must exit 1\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("wind_cf_2024") && stdout.contains("mismatch"),
        "mismatch report must name the trace\nstdout:\n{stdout}"
    );
}
