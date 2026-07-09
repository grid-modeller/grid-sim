//! Q5 heating-overlay output artefacts (D9 rule 6b): `grid-cli run` on
//! a scenario WITH a heating block writes `heating.{csv,parquet}` and a
//! `[results.heating]` summary section carrying the pinned constants
//! (k, DHW rate, ground damping/lag, deratings, cop_const), per-year
//! delivered-heat totals, and per-entry effective parameters with
//! overrides flagged. A scenario WITHOUT one writes none of it (the
//! byte-path check is regression_2024.rs / regression_5zone.rs).
//!
//! Requires the 2024 pack and the pinned GB t2m trace; fails loudly if
//! absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// The reference scenario with the D9 heating block (a 0.75 GSHP
/// derating override included so the override echo is exercised).
fn heated_scenario_file(dir: &Path) -> PathBuf {
    let text = std::fs::read_to_string(repo_root().join("scenarios/gb-2024-reference.toml"))
        .unwrap()
        .replace(
            "extra_demand_gw = 0.667\n",
            r#"extra_demand_gw = 0.667

[zones.demand.heating]
delivered_heat_twh = 410.5
electrified_share = 0.5
dhw_fraction = 0.170
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.heating.entries]]
kind = "ashp"
share = 0.70

[[zones.demand.heating.entries]]
kind = "gshp"
share = 0.20
rhpp_derating = 0.75

[[zones.demand.heating.entries]]
kind = "district_geothermal"
share = 0.10
"#,
        );
    let path = dir.join("gb-2024-heated.toml");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn heated_run_writes_the_d9_output_series_and_echoed_constants() {
    for rel in [
        "data/packs/2024/processed/demand_2024.parquet",
        "data/weather/gb_t2m_pop.parquet",
    ] {
        let probe = repo_root().join(rel);
        assert!(
            probe.exists(),
            "required data missing ({}) — build the packs/trace first",
            probe.display()
        );
    }

    let out_dir = std::env::temp_dir()
        .join("grid-cli-heating-tests")
        .join("heated-outputs");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::create_dir_all(&out_dir).unwrap();
    let scenario = heated_scenario_file(&out_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "run",
            "--scenario",
            scenario.to_str().unwrap(),
            "--base-dir",
            repo_root().to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "heated run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The rule-6b series files exist, CSV and Parquet both (docs/06).
    assert!(out_dir.join("heating.csv").exists());
    assert!(out_dir.join("heating.parquet").exists());
    let heating_csv = std::fs::read_to_string(out_dir.join("heating.csv")).unwrap();
    let header = heating_csv.lines().find(|l| !l.starts_with('#')).unwrap();
    assert_eq!(
        header,
        "utc_start,delivered_heat_gw,heating_electrical_total_gw,heating_ashp_gw,\
         heating_gshp_gw,heating_district_geothermal_gw"
    );

    // The echoed constants (D9 rule 6b) — checked against the reviewed
    // data-package values where they are pinned there.
    let summary = std::fs::read_to_string(out_dir.join("summary.toml")).unwrap();
    let section = summary.split("[results.heating]").nth(1).unwrap();
    let value = |key: &str| -> String {
        section
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| panic!("[results.heating] has no {key}"))
    };
    // k = electrified space quantum / mean annual degree-hours; the
    // degree-hours reproduce the data package's 50,454 °C·h.
    let dh: f64 = value("mean_annual_degree_hours_c_h").parse().unwrap();
    assert!((dh - 50_454.0).abs() < 1.0, "degree-hours {dh}");
    let k: f64 = value("k_gw_per_kelvin").parse().unwrap();
    assert!((k - 205_250.0 * 0.83 / dh).abs() < 1e-9);
    let damping: f64 = value("ground_damping").parse().unwrap();
    assert!((damping - 0.7130).abs() < 5e-4);
    let lag: f64 = value("ground_lag_days").parse().unwrap();
    assert!((lag - 19.66).abs() < 0.01);
    assert_eq!(value("record_years"), "40");
    assert_eq!(value("t_base_c"), "15.5");

    // Per-entry parameter echo: the reference deratings on ASHP, the
    // 0.75 override on GSHP (flagged), cop_const on district.
    let entry_section = |kind: &str| -> String {
        summary
            .split(&format!("[results.heating.entries.{kind}]"))
            .nth(1)
            .unwrap_or_else(|| panic!("no entries section for {kind}"))
            .split("\n[")
            .next()
            .unwrap()
            .to_owned()
    };
    let ashp = entry_section("ashp");
    assert!(ashp.contains("rhpp_derating = 0.823"), "{ashp}");
    assert!(ashp.contains("overridden = []"), "{ashp}");
    let gshp = entry_section("gshp");
    assert!(gshp.contains("rhpp_derating = 0.75"), "{gshp}");
    assert!(gshp.contains("overridden = [\"rhpp_derating\"]"), "{gshp}");
    let district = entry_section("district_geothermal");
    assert!(district.contains("cop_const = 15"), "{district}");

    // Per-year delivered heat is reported (2024-only horizon: one
    // year, and 2024 was a warm year — below the 205.25 TWh record
    // mean, exactly the never-renormalised physics of rule 3).
    let per_year = summary
        .split("[results.heating.delivered_heat_per_year_twh]")
        .nth(1)
        .unwrap();
    let year_2024: f64 = per_year
        .lines()
        .find_map(|l| l.strip_prefix("\"2024\" = "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (150.0..205.25).contains(&year_2024),
        "2024 delivered heat {year_2024} TWh should sit below the record mean"
    );
}

/// The MULTI-ZONE heating output branch (engine-review condition 4 —
/// a characterisation test of the per-zone path): the 5-zone scenario
/// with a heating block on the GB zone writes `heating_GB.{csv,
/// parquet}` and a `[results.heating_GB]` summary section (the
/// per-zone section rename), writes NO heating files for the unheated
/// zones, and moves the GB dispatch digest off its unheated pin
/// (heating is inside GB demand).
#[test]
fn multi_zone_heated_run_writes_per_zone_heating_outputs() {
    for rel in [
        "data/packs/entsoe-2024/processed/load_fr_2024.parquet",
        "data/weather/gb_t2m_pop.parquet",
    ] {
        let probe = repo_root().join(rel);
        assert!(
            probe.exists(),
            "required data missing ({}) — build the packs/trace first",
            probe.display()
        );
    }

    let out_dir = std::env::temp_dir()
        .join("grid-cli-heating-tests")
        .join("heated-5zone-outputs");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::create_dir_all(&out_dir).unwrap();

    // The 5-zone scenario with the D9 heating block on GB only (the
    // GB zone's demand table is the one carrying the station-load
    // wedge; the external zones stay unheated).
    let text = std::fs::read_to_string(repo_root().join("scenarios/gb-2024-5zone.toml"))
        .unwrap()
        .replace(
            "extra_demand_gw = 0.667\n",
            r#"extra_demand_gw = 0.667

[zones.demand.heating]
delivered_heat_twh = 410.5
electrified_share = 0.5
dhw_fraction = 0.170
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.heating.entries]]
kind = "ashp"
share = 0.70

[[zones.demand.heating.entries]]
kind = "gshp"
share = 0.20

[[zones.demand.heating.entries]]
kind = "district_geothermal"
share = 0.10
"#,
        );
    let scenario = out_dir.join("gb-2024-5zone-heated.toml");
    std::fs::write(&scenario, text).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "run",
            "--scenario",
            scenario.to_str().unwrap(),
            "--base-dir",
            repo_root().to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "heated 5-zone run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Per-zone heating files: GB only.
    assert!(out_dir.join("heating_GB.csv").exists());
    assert!(out_dir.join("heating_GB.parquet").exists());
    for zone in ["FR", "CONT-NW", "NO2", "DK1", "IE-SEM"] {
        assert!(
            !out_dir.join(format!("heating_{zone}.csv")).exists(),
            "unheated zone {zone} must write no heating file"
        );
    }

    // The per-zone summary section rename, constants included.
    let summary = std::fs::read_to_string(out_dir.join("summary.toml")).unwrap();
    assert!(!summary.contains("[results.heating]"), "unscoped section");
    let section = summary
        .split("[results.heating_GB]")
        .nth(1)
        .expect("no [results.heating_GB] section");
    for needle in [
        "k_gw_per_kelvin = ",
        "ground_damping = ",
        "record_years = 40",
    ] {
        assert!(section.contains(needle), "section lacks {needle:?}");
    }
    assert!(
        summary.contains("[results.heating_GB.entries.ashp]"),
        "per-entry sections must be zone-scoped"
    );
    assert!(
        summary.contains("[results.heating_GB.delivered_heat_per_year_twh]"),
        "per-year totals must be zone-scoped"
    );

    // Heating is inside GB demand: the GB dispatch digest must differ
    // from the unheated pin (regression_5zone.rs), while an unheated
    // external zone's dispatch may only move through the flow coupling
    // — not asserted here; the unheated-scenario pins live in
    // regression_5zone.rs.
    let gb_digest = summary
        .split("[results.zones.\"GB\"]")
        .nth(1)
        .unwrap()
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == "result_digest_sha256").then(|| v.trim().trim_matches('"').to_owned())
        })
        .unwrap();
    assert_ne!(
        gb_digest, "c783b306737eb4854b951d023c106578a9bfc5d428a6588c8e73e85ed1b03e5a",
        "the heated GB dispatch must differ from the unheated pin"
    );
}
