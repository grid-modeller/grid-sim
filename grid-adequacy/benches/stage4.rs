//! docs/06 benchmark smoke tests (`cargo bench -p grid-adequacy`),
//! Stage 4. Plain-timing harness (`harness = false`) — no benchmark
//! framework dependency; wall-clock reads are fine here (this is not a
//! library crate).
//!
//! Targets (docs/06 "Performance targets") and CI thresholds (2× the
//! target, for smoke-not-flake):
//!
//! 1. Full 40-year half-hourly single-zone run **with storage**:
//!    target < 1 s single-threaded → threshold 2 s.
//! 2. 10⁴-point sweep of single-year scenarios: target < 1 min with
//!    rayon on 8 cores → threshold 2 min. (docs/06 says "10⁴-scenario
//!    sweep" without fixing the horizon; this is the single-year
//!    reading.)
//! 3. The same 10⁴-point sweep of FULL 40-YEAR scenarios (the hardest
//!    reading: 100 × 100 fleet_scale × store_energy on the RS-lean
//!    scenario, ~7 × 10⁹ dispatched periods). First measurement
//!    2026-07-03 on an 8P+4E-core machine: **77 s — the 60 s target is
//!    MISSED on this machine under this reading; the 120 s CI
//!    threshold passes.** Cause (measured): the 40-year run is
//!    memory-system-bound under parallelism — 30 ms single-threaded
//!    inflates to ~92 ms of CPU per point at 12 threads (~3×), and 8
//!    threads is *slower* (~100 s wall), so it is not scheduler or
//!    efficiency-core drag. Remediation would need engine-level work
//!    (a leaner sweep-mode result that does not materialise ~60 MB of
//!    per-run output series) — recorded for the supervisor, not
//!    attempted here: the dispatch loop is the validated Stage 1–3
//!    engine and stays untouched.
//!
//! Needs the local data packs (fetched, not committed); fails loudly
//! if absent, like the acceptance tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::time::Instant;

use grid_adequacy::sweep::{Dimension, Execution, run_sweep};
use grid_adequacy::{load_run_inputs, run};
use grid_core::scenario::{Scenario, TechId};
use grid_core::units::Energy;

const LEAN_SCENARIO: &str = "scenarios/royal-society-37y-lean.toml";
const BENIGN_SCENARIO: &str = "scenarios/gb-2024-benign-battery.toml";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn main() {
    let root = repo_root();
    let scenario_path = root.join(LEAN_SCENARIO);
    assert!(
        root.join("data/packs/cf/gb_offshore_cf_1985.parquet")
            .exists(),
        "benchmarks need the local 1985–2024 data pack (fetched, not committed)"
    );
    let scenario = Scenario::load(&scenario_path).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());

    // --- Benchmark 1: the 40-year single run (median of 5). ---
    let mut runs_ms: Vec<f64> = (0..5)
        .map(|_| {
            let started = Instant::now();
            let result = run(&scenario, &inputs).unwrap();
            let elapsed = started.elapsed().as_secs_f64() * 1e3;
            assert_eq!(result.periods(), 701_280);
            elapsed
        })
        .collect();
    runs_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let single_run_ms = runs_ms[runs_ms.len() / 2];
    println!(
        "bench: 40-year single-zone run with storage: {single_run_ms:.1} ms (median of 5; \
         target < 1000 ms, CI threshold 2000 ms)"
    );
    assert!(
        single_run_ms < 2_000.0,
        "40-year run took {single_run_ms:.1} ms — over the 2 s CI threshold (docs/06)"
    );

    let linspace = |start: f64, stop: f64, count: usize| -> Vec<f64> {
        (0..count)
            .map(|i| start + (stop - start) * i as f64 / (count - 1) as f64)
            .collect()
    };
    let dimensions = |store_gwh_hi: f64| {
        vec![
            Dimension::FleetScale {
                technologies: vec![
                    TechId::new("offshore_wind"),
                    TechId::new("onshore_wind"),
                    TechId::new("solar"),
                ],
                values: linspace(0.85, 1.45, 100),
            },
            Dimension::StoreEnergy {
                store_index: 0,
                values: linspace(1_000.0, store_gwh_hi, 100)
                    .into_iter()
                    .map(Energy::gigawatt_hours)
                    .collect(),
            },
        ]
    };

    // --- Benchmark 2: 10⁴-point sweep, single-year scenarios. ---
    let benign = Scenario::load(&root.join(BENIGN_SCENARIO)).unwrap();
    let started = Instant::now();
    let surface = run_sweep(&benign, &root, &dimensions(100.0), Execution::Parallel).unwrap();
    let single_year_sweep_s = started.elapsed().as_secs_f64();
    assert_eq!(surface.points.len(), 10_000);
    println!(
        "bench: 10^4-point sweep (100×100, single-year runs) on {threads} threads: \
         {single_year_sweep_s:.1} s (target < 60 s on 8 cores, CI threshold 120 s)"
    );
    assert!(
        single_year_sweep_s < 120.0,
        "10^4-point single-year sweep took {single_year_sweep_s:.1} s — over the 120 s \
         CI threshold (docs/06)"
    );

    // --- Benchmark 3: 10⁴-point sweep, full 40-year scenarios (the
    // hardest reading; measured target miss documented in the module
    // docs — the CI threshold is what is asserted). ---
    let started = Instant::now();
    let surface = run_sweep(&scenario, &root, &dimensions(80_000.0), Execution::Parallel).unwrap();
    let sweep_s = started.elapsed().as_secs_f64();
    assert_eq!(surface.points.len(), 10_000);
    println!(
        "bench: 10^4-point sweep (100×100, full 40-year runs) on {threads} threads: \
         {sweep_s:.1} s (target < 60 s on 8 cores — see module docs; CI threshold 120 s)"
    );
    assert!(
        sweep_s < 120.0,
        "10^4-point 40-year sweep took {sweep_s:.1} s — over the 120 s CI threshold (docs/06)"
    );
}
