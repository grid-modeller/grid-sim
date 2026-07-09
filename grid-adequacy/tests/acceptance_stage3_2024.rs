//! Stage 3 benign-year acceptance test (docs/04 Stage 3; D4 acceptance
//! hook): single benign weather year (2024) + a 12 h battery on a
//! renewables-heavy fleet → **zero unserved energy** — the "a few days
//! of storage is enough" claim's home turf, reproduced deliberately
//! before the 37+-year runs dismantle it (Stage 3 part 2).
//!
//! The scenario (`scenarios/gb-2024-benign-battery.toml`) documents the
//! fleet choice; this test also guards against the claim being
//! trivially true (the fleet without its battery must actually run
//! short) and pins the measured shape of the 2024 shortfall.
//!
//! Needs the locally built 2024 data pack (fetched, not committed).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{load_run_inputs, run};
use grid_core::scenario::Scenario;
use grid_core::units::Energy;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

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

#[test]
fn benign_year_with_a_12h_battery_has_zero_unserved() {
    require_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-benign-battery.toml")).unwrap();

    // The battery really is 12 hours (energy = 12 h × power).
    let battery = &scenario.zones[0].storage[0];
    let duration_h = battery.energy_gwh.as_gigawatt_hours() / battery.power_gw.as_gigawatts();
    assert!(
        (duration_h - 12.0).abs() < 1e-9,
        "the benign scenario's battery is {duration_h} h, not 12 h"
    );

    let inputs = load_run_inputs(&scenario, &root).unwrap();

    // Not trivially benign: WITHOUT the battery, 2024 runs short — a
    // handful of GWh across a few winter spells (measured 19.1 GWh over
    // 33 half-hours, deepest 2.5 GW; pinned loosely so ERA5 pipeline
    // refinements don't false-alarm).
    let mut bare = scenario.clone();
    bare.zones[0].storage.clear();
    let without = run(&bare, &inputs).unwrap();
    let unserved_gwh = without.total_unserved().as_gigawatt_hours();
    assert!(
        unserved_gwh > 5.0 && unserved_gwh < 60.0,
        "without the battery, 2024 unserved should be a small-but-real \
         shortfall (measured 19.1 GWh); got {unserved_gwh} GWh — the \
         benign test would be trivial or mis-fleeted"
    );

    // THE acceptance assertion: with the 12 h battery, zero unserved.
    let result = run(&scenario, &inputs).unwrap();
    assert_eq!(
        result.total_unserved(),
        Energy::gigawatt_hours(0.0),
        "single benign year + 12 h battery must give exactly zero unserved \
         (docs/04 Stage 3)"
    );

    // And the battery did the work (it cycled), rather than the fleet
    // never needing it.
    let battery = &result.stores[0];
    let discharged: f64 = battery
        .discharge
        .iter()
        .map(|&p| (p * grid_core::units::Duration::half_hour()).as_gigawatt_hours())
        .sum();
    assert!(
        (discharged - unserved_gwh).abs() < 1.0,
        "the battery's discharge ({discharged} GWh) should cover the bare \
         fleet's shortfall ({unserved_gwh} GWh)"
    );
}
