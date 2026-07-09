//! `sweep wind-capacity-zonal` — CLI exposure of the priced multi-zone
//! wind sweep (docs/notes/wind-capacity-zonal-work-order.md): sweep a
//! zone's installed wind on the priced 3-zone engine (NSCO -> B4 ->
//! SSCO -> B6 -> RGB) and record that zone's Module 1 metrics with
//! imports ENDOGENOUS — the locational counterpart of the copper-plate
//! `sweep wind-capacity` (cli.rs precedent:
//! `sweep_wind_capacity_writes_module1_table_and_chart`).
//!
//! Requires the locally built 2024 + cf-gb2/cf-gb3 + b4/b6 data packs;
//! fails loudly with build instructions if absent (regression_3zone.rs
//! precedent).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Output};

use grid_core::scenario::{FlowSignal, Scenario};

const SCENARIO: &str = "scenarios/gb-2024-3zone.toml";

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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn require_packs() {
    for (rel, hint) in [
        (
            "data/packs/2024/processed/demand_2024.parquet",
            "scripts/fetch-2024",
        ),
        (
            "data/packs/cf-gb2/nsco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb3zone.py; verify data/packs/cf-gb3-1985-2024.sha256",
        ),
        (
            "data/packs/b6/processed/b4_da_flows_limits.parquet",
            "scripts/fetch-b6 (build.py --three-zone); verify data/packs/b4.sha256",
        ),
    ] {
        let path = repo_root().join(rel);
        assert!(
            path.exists(),
            "data pack file missing: {} — build it first: {hint}",
            path.display()
        );
    }
}

fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("grid-cli-zonal-sweep-tests")
        .join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}

// ---------------------------------------------------------------------
// Step 0 gate (work order): the 3-zone scenario must PRICE — every zone
// carries a [zones.pricing] block, so the schema-v7 priced-ladder
// validation gate (grid-core scenario.rs) passes and the zonal sweep
// has an SRMC chain per swept zone. SSCO carries the block with NO SRMC
// recipe (no gas plant south of B4 and north of B6): it prices at the
// GBP 0 must-take floor by design — that IS the cannibalisation signal.
// ---------------------------------------------------------------------

#[test]
fn three_zone_scenario_carries_zone_pricing_for_the_priced_gate() {
    let mut scenario = Scenario::load(&repo_root().join(SCENARIO)).unwrap();
    scenario.dispatch.flow_signal = FlowSignal::PricedLadder;
    scenario.validate().expect(
        "the 3-zone scenario must carry [zones.pricing] on every zone (the schema-v7 \
         priced gate) so the zonal wind sweep can price the swept zone",
    );

    let pricing = |id: &str| {
        scenario
            .zones
            .iter()
            .find(|z| z.id.as_str() == id)
            .unwrap_or_else(|| panic!("no zone {id}"))
            .pricing
            .as_ref()
            .unwrap_or_else(|| panic!("zone {id} has no [zones.pricing] block"))
    };
    // Gas SRMC only where there is gas plant: Peterhead CCGT in NSCO,
    // the E+W CCGT+OCGT fleet in RGB.
    assert!(pricing("NSCO").srmc.contains_key("ccgt"));
    assert!(pricing("RGB").srmc.contains_key("ccgt"));
    assert!(pricing("RGB").srmc.contains_key("ocgt"));
    // SSCO has no gas: the block is present (the gate) but names no
    // SRMC recipe — every technology prices at the GBP 0 must-take
    // floor by design.
    assert!(
        pricing("SSCO").srmc.is_empty(),
        "SSCO carries no gas plant; an SRMC recipe here would be an authoring error"
    );
}

// ---------------------------------------------------------------------
// The subcommand (work order Steps 1–3), mirroring
// `sweep_wind_capacity_writes_module1_table_and_chart` on the 3-zone
// scenario: CSV + PNG written, exact column header, provenance +
// assumption blocks, and the swept zone's capture ratio declining
// across the sweep (the locational cannibalisation signal). Coarse
// steps keep the multi-zone re-dispatches within CI time.
// ---------------------------------------------------------------------

/// The exact CSV column list of the work order — order and names.
const HEADER: &str = "wind_capacity_gw,zone,gas_price_setting_share,curtailment_twh,\
                      gas_twh,net_imports_twh,unserved_twh,mean_smp_gbp_per_mwh,\
                      wind_capture_ratio,wind_capture_ratio_delivered";

#[test]
fn sweep_wind_capacity_zonal_writes_table_and_chart_for_nsco() {
    require_packs();
    let dir = fresh_dir("zonal-nsco");
    let out = grid_cli(&[
        "sweep",
        "wind-capacity-zonal",
        "--scenario",
        SCENARIO,
        "--out",
        dir.to_str().unwrap(),
        "--zone",
        "NSCO",
        "--min-gw",
        "10",
        "--max-gw",
        "40",
        "--step-gw",
        "15",
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let csv = std::fs::read_to_string(dir.join("module1_zonal_capture_vs_wind_NSCO.csv")).unwrap();

    // Provenance header (docs/06 parity with the sibling sweep) plus
    // the three work-order assumption blocks: the 3-zone honesty
    // conventions verbatim, the supervisor's GBP-0-floor guard, and the
    // capture-ratio definitions.
    for needle in [
        "engine_git_hash",
        "scenario_sha256",
        "data_file",
        "demand_2024.parquet",
        "nsco_onshore_cf_2024.parquet",
        "b4_da_flows_limits.parquet",
        "gas_sap_daily_2024.parquet",
        "prices-2024.toml",
        // Block 1 — verbatim scenario-header honesty conventions.
        "DIRECTION + PINNED TOTALS",
        "NO B4-vs-B6",
        "DA-only, no outturn anchor",
        "ANTI-CONSERVATIVE",
        // Block 2 — the supervisor's hostile-reader guard, verbatim.
        "SMP floors at 0 GBP/MWh; no negative pricing, no CfD-floor bidding \
         — understates cannibalisation (anti-cannibalisation-conservative)",
        // Block 3 — capture-ratio definitions (Stage 2 / P-Q10).
        "POTENTIAL",
        "pro-rata",
        "never NaN",
    ] {
        assert!(csv.contains(needle), "zonal sweep CSV lacks {needle:?}");
    }

    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 3 + 1, "3 sweep points + header");
    assert_eq!(data_lines[0], HEADER, "column header must match exactly");

    let col = |name: &str| HEADER.split(',').position(|c| c == name.trim()).unwrap();
    let field = |line: &str, name: &str| -> f64 {
        line.split(',').nth(col(name)).unwrap().parse().unwrap()
    };
    for (line, gw) in data_lines[1..].iter().zip([10.0, 25.0, 40.0]) {
        assert!((field(line, "wind_capacity_gw") - gw).abs() < 1e-9);
        assert_eq!(line.split(',').nth(col("zone")).unwrap(), "NSCO");
    }

    // The locational cannibalisation signal: the swept zone's capture
    // ratio (POTENTIAL basis — the Stage 2 published convention)
    // declines monotonically-or-nearly across the sweep. The DELIVERED
    // basis is deliberately NOT asserted monotone: at extreme
    // curtailment behind the B4 wall (80.8 of ~119 TWh potential
    // curtailed at 40 GW) the pro-rata removal concentrates the
    // surviving delivered energy in priced periods, so the delivered
    // ratio turns UP again (measured 0.393 → 0.383 → 0.429) while the
    // potential ratio collapses — a characterised property, not a bug.
    let ratios: Vec<f64> = data_lines[1..]
        .iter()
        .map(|l| field(l, "wind_capture_ratio"))
        .collect();
    for pair in ratios.windows(2) {
        assert!(
            pair[1] < pair[0] + 1e-3,
            "wind_capture_ratio must decline (nearly) monotonically: {ratios:?}"
        );
    }
    assert!(
        ratios.last().unwrap() < ratios.first().unwrap(),
        "wind_capture_ratio must decline across the sweep: {ratios:?}"
    );
    // Cross-basis direction lock (the Package A convention): curtailed
    // periods price at GBP 0, so the delivered ratio sits at or above
    // the potential one in every row.
    for line in &data_lines[1..] {
        let potential = field(line, "wind_capture_ratio");
        let delivered = field(line, "wind_capture_ratio_delivered");
        assert!(
            delivered >= potential,
            "delivered capture ratio {delivered} below potential {potential}: {line}"
        );
    }

    // Pin the 40 GW high-wind row exactly (measured 2026-07-07, the
    // work-order acceptance row): the locational collapse quoted from
    // this artefact is guarded here before anything is published.
    let last = *data_lines.last().unwrap();
    for (name, pinned) in [
        ("wind_capacity_gw", 40.0),
        ("gas_price_setting_share", 0.08487021857923498),
        ("curtailment_twh", 80.77926834271888),
        ("gas_twh", 0.5064059395110525),
        ("net_imports_twh", -16.77018297149849),
        ("unserved_twh", 0.0),
        ("mean_smp_gbp_per_mwh", 6.7574286266052495),
        ("wind_capture_ratio", 0.10095620948764589),
        ("wind_capture_ratio_delivered", 0.4289250410437181),
    ] {
        let cell = field(last, name);
        assert!(
            (cell - pinned).abs() <= 1e-9 * pinned.abs().max(1.0),
            "40 GW row {name} moved: {cell} vs pinned {pinned} — a deliberate \
             engine/pack/scenario change requires a knowing re-pin with the record"
        );
    }

    let png = std::fs::read(dir.join("module1_zonal_capture_vs_wind_NSCO.png")).unwrap();
    assert!(png.len() > 10_000, "PNG suspiciously small: {}", png.len());
    assert_eq!(&png[1..4], b"PNG");
}

/// Several `--zone` flags select the zone-GROUP sweep (one shared
/// scaling factor, aggregate metrics — the D13 library function): one
/// CSV/PNG pair for the group, named and labelled by the joined ids.
#[test]
fn sweep_wind_capacity_zonal_group_aggregates_the_scottish_zones() {
    require_packs();
    let dir = fresh_dir("zonal-group");
    let out = grid_cli(&[
        "sweep",
        "wind-capacity-zonal",
        "--scenario",
        SCENARIO,
        "--out",
        dir.to_str().unwrap(),
        "--zone",
        "NSCO",
        "--zone",
        "SSCO",
        "--min-gw",
        "15",
        "--max-gw",
        "30",
        "--step-gw",
        "15",
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let csv =
        std::fs::read_to_string(dir.join("module1_zonal_capture_vs_wind_NSCO+SSCO.csv")).unwrap();
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 2 + 1, "2 sweep points + header");
    assert_eq!(data_lines[0], HEADER, "column header must match exactly");
    for line in &data_lines[1..] {
        assert_eq!(line.split(',').nth(1).unwrap(), "NSCO+SSCO");
    }

    let png = std::fs::read(dir.join("module1_zonal_capture_vs_wind_NSCO+SSCO.png")).unwrap();
    assert_eq!(&png[1..4], b"PNG");
}
