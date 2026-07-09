//! `grid-cli sweep heating-mix` artefact tests (Q5/Q11, D9 rules
//! 6/6b): the subcommand writes heating_mix_sweep.{csv,parquet},
//! heating_mix_decomposition.{csv,parquet} and the two-gradient chart,
//! with the stated store rating stamped per row and the D9/review
//! assumption block (quote duties verbatim) on every artefact; and a
//! SolveInfeasible at the stated rating is REPORTED in the sweep
//! artefact (infeasible_reason column), never silently bumped.
//!
//! Runs on a generated SINGLE-YEAR (2024) RS-style heated scenario so
//! the suite stays affordable; the 40-year pinned numbers live in
//! grid-adequacy/tests/heating_mix.rs. Needs the 2024 slice of the
//! per-year pack and the pinned t2m trace; fails loudly if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn require_data() {
    for rel in [
        "data/packs/demand-tiled/demand_2024.parquet",
        "data/packs/cf/gb_offshore_cf_2024.parquet",
        "data/packs/cf/gb_onshore_cf_2024.parquet",
        "data/packs/cf/gb_solar_cf_2024.parquet",
        "data/weather/gb_t2m_pop.parquet",
    ] {
        let probe = repo_root().join(rel);
        assert!(
            probe.exists(),
            "required data missing ({}) — build the packs/trace first",
            probe.display()
        );
    }
}

/// A single-year (2024) RS-style scenario with the D9 heating block:
/// the RS fleet and store, 2024 traces only — cheap enough for the CLI
/// suite while exercising the full artefact path.
fn single_year_heated_scenario(dir: &Path) -> PathBuf {
    let text = r#"
schema_version = 8
name = "rs-2024-heated-cli-test"

[horizon]
start = "2024-01-01T00:00:00Z"
end   = "2024-12-31T23:30:00Z"
weather_years = "all"

[[zones]]
id = "GB"

[zones.demand]
annual_scale = 2.177
column = "underlying_demand"
base_profile = ["data/packs/demand-tiled/demand_2024.parquet"]

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

[[zones.fleet]]
technology = "offshore_wind"
capacity_gw = 240.0
capacity_factor_trace = ["data/packs/cf/gb_offshore_cf_2024.parquet"]

[[zones.fleet]]
technology = "onshore_wind"
capacity_gw = 80.0
capacity_factor_trace = ["data/packs/cf/gb_onshore_cf_2024.parquet"]

[[zones.fleet]]
technology = "solar"
capacity_gw = 200.0
capacity_factor_trace = ["data/packs/cf/gb_solar_cf_2024.parquet"]

[[zones.storage]]
kind = "hydrogen"
power_gw = 100.0
energy_gwh = 100000.0
round_trip_efficiency = 0.40
dispatch_order = 1

[dispatch]
policy = "rule_based"
"#;
    let path = dir.join("rs-2024-heated.toml");
    std::fs::write(&path, text).unwrap();
    path
}

fn run_sweep(scenario: &Path, out_dir: &Path, store_power_gw: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "sweep",
            "heating-mix",
            "--scenario",
            scenario.to_str().unwrap(),
            "--base-dir",
            repo_root().to_str().unwrap(),
            "--out",
            out_dir.to_str().unwrap(),
            "--step",
            "1",
            "--store-power-gw",
            store_power_gw,
        ])
        .current_dir(repo_root())
        .output()
        .unwrap()
}

/// Column index of `name` in a CSV header line.
fn column_index(header: &str, name: &str) -> usize {
    header
        .split(',')
        .position(|c| c == name)
        .unwrap_or_else(|| panic!("no column {name} in {header}"))
}

/// Footer key-value metadata of a parquet file as (key, value) pairs.
fn parquet_metadata_keys(path: &Path) -> Vec<(String, String)> {
    use parquet::file::reader::FileReader;
    let file = std::fs::File::open(path).unwrap();
    let reader = parquet::file::reader::SerializedFileReader::new(file).unwrap();
    reader
        .metadata()
        .file_metadata()
        .key_value_metadata()
        .expect("parquet artefacts carry the docs/06 metadata block")
        .iter()
        .map(|kv| (kv.key.clone(), kv.value.clone().unwrap_or_default()))
        .collect()
}

#[test]
fn heating_mix_sweep_writes_the_stamped_artefacts() {
    require_data();
    let out_dir = std::env::temp_dir()
        .join("grid-cli-heating-mix-tests")
        .join("feasible");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::create_dir_all(&out_dir).unwrap();
    let scenario = single_year_heated_scenario(&out_dir);

    let output = run_sweep(&scenario, &out_dir, "200");
    assert_eq!(
        output.status.code(),
        Some(0),
        "sweep failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Both tabular formats + the chart (docs/06: both, always).
    for name in [
        "heating_mix_sweep.csv",
        "heating_mix_sweep.parquet",
        "heating_mix_decomposition.csv",
        "heating_mix_decomposition.parquet",
        "heating_mix_gradients.png",
    ] {
        assert!(out_dir.join(name).exists(), "missing artefact {name}");
    }

    // --- The sweep table: baseline + the 3 corners (step 1). ---
    let csv = std::fs::read_to_string(out_dir.join("heating_mix_sweep.csv")).unwrap();
    let mut lines = csv.lines().filter(|l| !l.starts_with('#'));
    let header = lines.next().unwrap().to_owned();
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), 4, "baseline + 3 corner rows");
    assert!(rows[0].starts_with("baseline,"));

    // The stated rating is stamped on EVERY row (the review's binding
    // record item: no storage number travels without its rating).
    let power_col = column_index(&header, "store_power_gw");
    let requirement_col = column_index(&header, "requirement_gwh");
    let reason_col = column_index(&header, "infeasible_reason");
    for row in &rows {
        let cells: Vec<&str> = row.split(',').collect();
        assert_eq!(cells[power_col], "200", "rating not stamped: {row}");
        // Feasible at 200 GW: a requirement, no infeasible reason.
        assert!(!cells[requirement_col].is_empty(), "no requirement: {row}");
        assert_eq!(cells[reason_col], "\"\"", "unexpected reason: {row}");
    }

    // --- The assumption block (quote duties verbatim). ---
    for needle in [
        "at 200 GW store power, both endpoints",
        "POWER-BOUND INFEASIBLE",
        "never a silently bumped rating",
        "lower bound",
        "2024 non-heat demand tiled",
        "climate-stationary heat intensity",
        "ELCC runner, wave-2 paper-4 enabler",
        "NESO Open Data Licence",
        "Copernicus Climate Change Service",
    ] {
        assert!(csv.contains(needle), "sweep CSV lacks {needle:?}");
    }

    // --- The same duties ride in the PARQUET metadata under UNIQUELY
    // NUMBERED keys (review condition 1: duplicate `assumption` keys
    // collapse to one line in dict-based readers — pyarrow's default
    // view — hiding seven quote-duty lines from standard tooling).
    // Checked on BOTH parquet artefacts.
    for name in [
        "heating_mix_sweep.parquet",
        "heating_mix_decomposition.parquet",
    ] {
        let keys = parquet_metadata_keys(&out_dir.join(name));
        let assumption_keys: Vec<&String> = keys
            .iter()
            .map(|(k, _)| k)
            .filter(|k| k.starts_with("assumption"))
            .collect();
        // No duplicate keys among them (the dict-collapse hazard).
        let mut unique = assumption_keys.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            assumption_keys.len(),
            "{name}: duplicate assumption keys would collapse in dict-based readers"
        );
        // Every line of the block is present, individually addressable.
        for index in 1..=8 {
            let key = format!("assumption_{index}");
            assert!(
                keys.iter().any(|(k, _)| *k == key),
                "{name}: parquet metadata lacks {key}"
            );
        }
        // The load-bearing duties are in the metadata VALUES.
        let joined: String = keys
            .iter()
            .filter(|(k, _)| k.starts_with("assumption"))
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "at 200 GW store power, both endpoints",
            "never a silently bumped rating",
            "ELCC runner, wave-2 paper-4 enabler",
            "NESO Open Data Licence",
            "TOTAL OVER THE SCENARIO HORIZON",
        ] {
            assert!(
                joined.contains(needle),
                "{name}: parquet assumption metadata lacks {needle:?}"
            );
        }
    }

    // --- The decomposition table: 4 named points, windows stamped. ---
    let decomposition =
        std::fs::read_to_string(out_dir.join("heating_mix_decomposition.csv")).unwrap();
    assert!(decomposition.contains("window convention"));
    let mut lines = decomposition.lines().filter(|l| !l.starts_with('#'));
    let header = lines.next().unwrap().to_owned();
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), 4);
    let expected_labels = ["baseline", "all_ashp", "all_gshp", "all_district"];
    let windows_cols = [
        (column_index(&header, "window_diurnal_h"), "24"),
        (column_index(&header, "window_synoptic_h"), "336"),
        (column_index(&header, "window_seasonal_h"), "8760"),
    ];
    let total_col = column_index(&header, "total_gwh");
    let band_cols = [
        column_index(&header, "diurnal_gwh"),
        column_index(&header, "synoptic_gwh"),
        column_index(&header, "seasonal_gwh"),
        column_index(&header, "inter_annual_gwh"),
    ];
    for (row, label) in rows.iter().zip(expected_labels) {
        let cells: Vec<&str> = row.split(',').collect();
        assert_eq!(cells[0], label);
        assert_eq!(cells[column_index(&header, "store_power_gw")], "200");
        for (col, expected) in windows_cols {
            assert_eq!(cells[col], expected, "window not stamped: {row}");
        }
        // Telescoping: the bands sum to the total (kill criterion 2),
        // checked on the artefact itself.
        let total: f64 = cells[total_col].parse().unwrap();
        let band_sum: f64 = band_cols
            .iter()
            .map(|&c| cells[c].parse::<f64>().unwrap())
            .sum();
        assert!(
            (band_sum - total).abs() < 1e-6,
            "{label}: bands {band_sum} vs total {total}"
        );
    }
}

/// SolveInfeasible at the stated rating is a REPORTABLE RESULT: at a
/// deliberately undersized 5 GW rating every solve is power-bound, the
/// sweep artefact still lands with the reasons in the
/// infeasible_reason column and empty requirement cells, and the
/// command then fails loudly on the decomposition (which needs a
/// feasible rating) — nothing is silently bumped.
#[test]
fn infeasible_rating_is_reported_in_the_artefact_not_bumped() {
    require_data();
    let out_dir = std::env::temp_dir()
        .join("grid-cli-heating-mix-tests")
        .join("infeasible");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    std::fs::create_dir_all(&out_dir).unwrap();
    let scenario = single_year_heated_scenario(&out_dir);

    let output = run_sweep(&scenario, &out_dir, "5");
    assert_ne!(
        output.status.code(),
        Some(0),
        "the decomposition cannot succeed at an infeasible rating"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("after the sweep artefacts were written"),
        "the error must say the sweep artefacts landed: {stderr}"
    );

    let csv = std::fs::read_to_string(out_dir.join("heating_mix_sweep.csv")).unwrap();
    let mut lines = csv.lines().filter(|l| !l.starts_with('#'));
    let header = lines.next().unwrap().to_owned();
    let requirement_col = column_index(&header, "requirement_gwh");
    let power_col = column_index(&header, "store_power_gw");
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), 4);
    for row in rows {
        let cells: Vec<&str> = row.split(',').collect();
        assert_eq!(cells[power_col], "5", "the 5 GW rating must be stamped");
        assert!(
            cells[requirement_col].is_empty(),
            "no requirement may be reported at an infeasible rating: {row}"
        );
        assert!(
            row.contains("power rating too small") || row.contains("search cap"),
            "the solver's structured reason must ride in the row: {row}"
        );
    }
}
