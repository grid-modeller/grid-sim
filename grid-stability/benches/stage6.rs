//! docs/06 benchmark smoke test (`cargo bench -p grid-stability`),
//! Stage 6. Plain-timing harness (`harness = false`), same pattern as
//! the Stage 4 benches — no framework dependency; wall-clock reads are
//! fine here (not a library crate).
//!
//! Target (docs/06 "Performance targets"): stability event simulation
//! **< 10 ms per event**; CI smoke threshold 2× = 20 ms.
//!
//! The measured event is the full 9 August 2019 reproduction: 300 s
//! simulated at a 10 ms step (30,000 Heun steps), seven staged losses,
//! three response services with delivery timelines recorded, nine LFDD
//! stages. First measurement 2026-07-03 (Apple Silicon desktop core,
//! release profile): 1.7 ms per event — ~6× under target. (The debug
//! profile runs ~25 ms; the docs/06 target is a release-profile
//! number, hence the bench harness rather than a #[test].)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::time::Instant;

use grid_stability::{EventSpec, simulate};

fn main() {
    let spec = EventSpec::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("scenarios/events/gb-2019-08-09.toml"),
    )
    .unwrap();

    // Warm-up (page-in, allocator), then the mean of 100.
    let warmup = simulate(&spec).unwrap();
    assert!(warmup.trace().len() > 10_000);

    let runs = 100u32;
    let started = Instant::now();
    for _ in 0..runs {
        let result = simulate(&spec).unwrap();
        std::hint::black_box(&result);
    }
    let mean = started.elapsed() / runs;
    println!("stage6 bench: mean event simulation {mean:?} (target < 10 ms, smoke < 20 ms)");
    assert!(
        mean.as_millis() < 20,
        "event simulation took {mean:?} per event — docs/06 smoke threshold is 20 ms (2× the \
         10 ms target)"
    );
    println!("stage6 bench: OK");
}
