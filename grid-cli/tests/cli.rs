//! Stage 0 CLI behaviour: subcommand stubs, the `validate` demo artefact,
//! and the docs/06 exit codes (0 success, 2 usage/scenario error).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Output};

/// Repo root — the CLI resolves scenario-referenced trace paths against
/// the working directory, and scenario files reference `data/...`
/// relative to the repo root.
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const REFERENCE: &str = "scenarios/gb-2024-reference.toml";

#[test]
fn validate_parses_the_reference_scenario_and_exits_zero() {
    let out = grid_cli(&["validate", "--scenario", REFERENCE]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("GB-2024-reference"), "stdout: {text}");
    assert!(text.contains("schema"), "stdout: {text}");
}

// Stage 0 demo artefact: scenario summary plus trace statistics.
#[test]
fn validate_summary_prints_scenario_and_trace_statistics() {
    let out = grid_cli(&[
        "validate",
        "--scenario",
        REFERENCE,
        "--summary",
        "--trace",
        "data/packs/2024/processed/wind_cf_2024.parquet:wind_cf",
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let text = stdout(&out);

    // Scenario summary: name, zones, fleet capacities, storage, links.
    assert!(text.contains("GB-2024-reference"), "stdout: {text}");
    assert!(text.contains("GB"), "stdout: {text}");
    assert!(text.contains("ccgt"), "stdout: {text}");
    assert!(text.contains("30.0"), "stdout: {text}"); // ccgt GW
    assert!(text.contains("pumped_hydro"), "stdout: {text}");
    assert!(text.contains("rule_based"), "stdout: {text}");
    assert!(text.contains("GB -> FR"), "stdout: {text}");
    assert!(text.contains("17568"), "stdout: {text}"); // horizon periods

    // Trace statistics for the demand base profile and the extra trace:
    // period count, mean/min/max.
    assert!(text.contains("underlying_demand"), "stdout: {text}");
    assert!(text.contains("wind_cf"), "stdout: {text}");
    assert!(text.contains("mean"), "stdout: {text}");
    assert!(text.contains("min"), "stdout: {text}");
    assert!(text.contains("max"), "stdout: {text}");

    // The scenario's per-technology CF traces exist (ERA5-derived, Phase A);
    // the summary reports them present without loading them (column
    // convention is pinned in Stage 1).
    assert!(
        text.contains("gb_offshore_cf_2024.parquet"),
        "stdout: {text}"
    );
    assert!(text.contains("present"), "stdout: {text}");
    assert!(!text.contains("not found"), "stdout: {text}");
}

#[test]
fn validate_with_missing_scenario_file_exits_two() {
    let out = grid_cli(&["validate", "--scenario", "scenarios/no-such.toml"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("no-such.toml"));
}

#[test]
fn validate_with_broken_scenario_exits_two_with_context() {
    let dir = std::env::temp_dir().join("grid-cli-broken.toml");
    std::fs::write(&dir, "name = \"broken\"\n").unwrap();
    let out = grid_cli(&["validate", "--scenario", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("schema_version"),
        "stderr: {}",
        stderr(&out)
    );
}

// The generic sweep runner exists since Stage 4: bare `sweep` is a
// usage error (exit 2) listing the available sweep subcommands.
#[test]
fn bare_sweep_lists_its_subcommands() {
    let out = grid_cli(&["sweep"]);
    assert_eq!(out.status.code(), Some(2));
    let text = stderr(&out);
    for subcommand in ["grid", "per-year", "wind-capacity"] {
        assert!(text.contains(subcommand), "stderr: {text}");
    }
}

// `fetch-data` is implemented (the data-pack builder port); `--out` is
// deliberately required so a rebuild into the canonical, manifest-pinned
// pack directory is always explicit.
#[test]
fn fetch_data_requires_an_explicit_out_directory() {
    let out = grid_cli(&["fetch-data"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("--out"), "stderr: {}", stderr(&out));
}

// Unpinned years are refused with a structured message, not guessed URLs.
#[test]
fn fetch_data_refuses_years_without_pinned_sources() {
    let out = grid_cli(&["fetch-data", "--year", "2023", "--out", "target/tmp/nope"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("no pinned data sources for year 2023"),
        "stderr: {}",
        stderr(&out)
    );
}

#[test]
fn unknown_subcommand_exits_two() {
    let out = grid_cli(&["frobnicate"]);
    assert_eq!(out.status.code(), Some(2));
}

// ---------------------------------------------------------------------
// Stage 1: `run` — chronological dispatch with CSV + Parquet + summary
// outputs carrying the docs/06 metadata header.
// ---------------------------------------------------------------------

/// Fail loudly if the 2024 pack has not been built (fetched, not
/// committed).
fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing — build the pack first (scripts/fetch-2024, scripts/era5-cf)"
    );
}

fn run_to(out_dir: &Path) -> Output {
    grid_cli(&[
        "run",
        "--scenario",
        REFERENCE,
        "--out",
        out_dir.to_str().unwrap(),
    ])
}

fn fresh_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("grid-cli-stage1-tests")
        .join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    dir
}

#[test]
fn run_writes_dispatch_outputs_with_metadata_header() {
    require_pack();
    let dir = fresh_dir("run-outputs");
    let out = run_to(&dir);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    // Stage 3: storage is active; the run reports per-store series.
    let text = stdout(&out);
    assert!(
        text.contains("pumped_hydro_charge") && text.contains("battery_discharge"),
        "stdout should report per-store series: {text}"
    );

    // CSV and Parquet, both, always (docs/06).
    let csv = std::fs::read_to_string(dir.join("dispatch.csv")).unwrap();
    assert!(dir.join("dispatch.parquet").exists());
    assert!(dir.join("monthly_mix.csv").exists());

    // Metadata header block (docs/06): engine git hash, scenario hash,
    // data-pack checksums, schema version, timestamp.
    for needle in [
        "engine_git_hash",
        "scenario_sha256",
        "schema_version",
        "created_utc",
    ] {
        assert!(csv.contains(needle), "dispatch.csv lacks {needle}");
    }
    assert!(
        !csv.contains("run_inputs"),
        "schema v2 runs have no run-inputs file; dispatch.csv still mentions one"
    );

    // Data rows: one per 2024 half-hourly period plus one header row.
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 17_568 + 1);
    let header = data_lines[0];
    for column in [
        "utc_start",
        "demand_gw",
        "offshore_wind_gw",
        "onshore_wind_gw",
        "solar_gw",
        "net_imports_gw",
        "pumped_storage_net_gw",
        "other_gw",
        "nuclear_gw",
        "biomass_gw",
        "hydro_gw",
        "coal_gw",
        "ccgt_gw",
        "ocgt_gw",
        "pumped_hydro_charge_gw",
        "pumped_hydro_discharge_gw",
        "battery_charge_gw",
        "battery_discharge_gw",
        "curtailment_gw",
        "unserved_gw",
        "pumped_hydro_soc_gwh",
        "battery_soc_gwh",
        "firm_supply_gw",
        "variable_supply_gw",
        "storage_discharge_gw",
        "firm_share",
    ] {
        assert!(header.contains(column), "header lacks {column}: {header}");
    }

    // The run summary repeats the metadata and carries the headline
    // aggregates and the deterministic result digest.
    let summary = std::fs::read_to_string(dir.join("summary.toml")).unwrap();
    for needle in [
        "engine_git_hash",
        "scenario_sha256",
        "data_files",
        "result_digest_sha256",
        "unserved_twh",
        "curtailment_twh",
        "net_imports_twh",
        "[results.storage.pumped_hydro]",
        "[results.storage.battery]",
        "min_soc_gwh",
        "[results.reliability]",
        "firm_share_mean",
        "periods_firm_share_below_0_5",
        "[results.reliability.classification]",
    ] {
        assert!(summary.contains(needle), "summary.toml lacks {needle}");
    }
}

#[test]
fn run_is_deterministic_across_invocations() {
    require_pack();
    let dir_a = fresh_dir("determinism-a");
    let dir_b = fresh_dir("determinism-b");
    assert_eq!(run_to(&dir_a).status.code(), Some(0));
    assert_eq!(run_to(&dir_b).status.code(), Some(0));

    let digest = |dir: &Path| -> String {
        let summary = std::fs::read_to_string(dir.join("summary.toml")).unwrap();
        summary
            .lines()
            .find(|l| l.starts_with("result_digest_sha256"))
            .unwrap()
            .to_owned()
    };
    assert_eq!(digest(&dir_a), digest(&dir_b), "output hashes differ");

    // And the CSV data itself is byte-identical (the '#' metadata header
    // may differ by timestamp only).
    let data = |dir: &Path| -> String {
        std::fs::read_to_string(dir.join("dispatch.csv"))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(data(&dir_a), data(&dir_b));
}

#[test]
fn run_with_a_v1_scenario_exits_two_with_the_migration_message() {
    let dir = fresh_dir("v1-scenario");
    let out = grid_cli(&[
        "run",
        "--scenario",
        "grid-core/tests/fixtures/v1-gb-2024-reference.toml",
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(2));
    let text = stderr(&out);
    assert!(text.contains("schema_version 1"), "stderr: {text}");
    assert!(text.contains("run-inputs"), "stderr: {text}");
    assert!(text.contains("docs/03-domain-model.md"), "stderr: {text}");
}

// ---------------------------------------------------------------------
// Stage 2: pricing outputs — prices.csv + prices.parquet with the
// docs/06 metadata header, and the [results.pricing] summary block
// carrying the acceptance-relevant aggregates including the reported
// (not gated) realism statistics (docs/04 Stage 2 test 3).
// ---------------------------------------------------------------------

#[test]
fn run_with_pricing_writes_price_outputs_and_summary_block() {
    require_pack();
    let dir = fresh_dir("run-pricing");
    let out = run_to(&dir);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    // CSV and Parquet, both, always (docs/06).
    let csv = std::fs::read_to_string(dir.join("prices.csv")).unwrap();
    assert!(dir.join("prices.parquet").exists());
    for needle in ["engine_git_hash", "scenario_sha256"] {
        assert!(csv.contains(needle), "prices.csv lacks {needle}");
    }
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 17_568 + 1);
    let header = data_lines[0];
    for column in [
        "utc_start",
        "smp_gbp_per_mwh",
        "price_setter",
        "srmc_ccgt_gbp_per_mwh",
        "srmc_ocgt_gbp_per_mwh",
    ] {
        assert!(header.contains(column), "header lacks {column}: {header}");
    }

    // The pricing hashes cover the price inputs too.
    for needle in [
        "gas_sap_daily_2024.parquet",
        "market_index_2024.parquet",
        "prices-2024.toml",
    ] {
        assert!(csv.contains(needle), "prices.csv metadata lacks {needle}");
    }

    // Summary block: SMP aggregates, both gas-marginal framings
    // (behavioural flag + price-consistency vs observed), wind capture,
    // emissions in both labelled bases, realism stats, digest.
    let summary = std::fs::read_to_string(dir.join("summary.toml")).unwrap();
    for needle in [
        "[results.pricing]",
        "smp_time_weighted_mean_gbp_per_mwh",
        "pct_periods_gas_price_setting",
        "pct_periods_must_take_only",
        "pct_observed_price_within_20pct_of_ccgt_srmc",
        "wind_capture_price_gbp_per_mwh",
        "wind_capture_ratio",
        "total_co2_mt",
        "total_co2e_mt",
        "median_model_smp_over_observed_mid",
        "monthly_corr_model_smp_vs_observed_mid",
        "prices_digest_sha256",
        "[results.pricing.technologies.offshore_wind]",
        "[results.pricing.technologies.ccgt]",
        "[results.pricing.emissions.ccgt]",
    ] {
        assert!(summary.contains(needle), "summary.toml lacks {needle}");
    }
}

// ---------------------------------------------------------------------
// Stage 2 demo artefact (Module 1): % of periods gas-marginal vs
// installed wind capacity — `sweep wind-capacity` writes a CSV table and
// a PNG chart with the docs/06 hash metadata. (The generic sweep runner
// remains Stage 4; bare `sweep` still says so — tested above.)
// ---------------------------------------------------------------------

#[test]
fn sweep_wind_capacity_writes_module1_table_and_chart() {
    require_pack();
    let dir = fresh_dir("sweep-module1");
    let out = grid_cli(&[
        "sweep",
        "wind-capacity",
        "--scenario",
        REFERENCE,
        "--out",
        dir.to_str().unwrap(),
        "--min-gw",
        "20",
        "--max-gw",
        "40",
        "--step-gw",
        "10",
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let csv = std::fs::read_to_string(dir.join("module1_gas_marginal_vs_wind.csv")).unwrap();
    // Metadata + documented sweep assumptions, with per-data-file hashes
    // (docs/06 metadata parity with the run outputs).
    for needle in [
        "engine_git_hash",
        "scenario_sha256",
        "assumption",
        "data_file",
        "gas_sap_daily_2024.parquet",
        "prices-2024.toml",
        "demand_2024.parquet",
    ] {
        assert!(csv.contains(needle), "sweep CSV lacks {needle}");
    }
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 3 + 1, "3 sweep points + header");
    let header = data_lines[0];
    for column in [
        "wind_capacity_gw",
        "pct_periods_gas_price_setting",
        "curtailment_twh",
        "curtailment_pct_of_renewable_potential",
        "gas_twh",
        "wind_capture_ratio",
        // Package A: the delivered basis is ADDED after the existing
        // columns — old columns keep their names and positions.
        "wind_capture_ratio_delivered",
        // Package B: the import-convention bracket columns, appended
        // after the frozen-convention (unsuffixed) columns.
        "curtailment_twh_imports_zero",
        "wind_capture_ratio_imports_zero",
        "wind_capture_ratio_delivered_imports_zero",
        "curtailment_twh_imports_export",
        "wind_capture_ratio_imports_export",
        "wind_capture_ratio_delivered_imports_export",
    ] {
        assert!(header.contains(column), "header lacks {column}: {header}");
    }
    // More wind → gas sets the price less often (monotone over these
    // points), and gas burn falls.
    let field =
        |line: &str, index: usize| -> f64 { line.split(',').nth(index).unwrap().parse().unwrap() };
    let pct_col = header
        .split(',')
        .position(|c| c == "pct_periods_gas_price_setting")
        .unwrap();
    let pcts: Vec<f64> = data_lines[1..].iter().map(|l| field(l, pct_col)).collect();
    assert!(
        pcts[0] > pcts[1] && pcts[1] > pcts[2],
        "gas price-setting share should fall with wind capacity: {pcts:?}"
    );

    // Both capture bases populated, delivered at-or-above potential
    // (curtailment prices at £0 — direction documented in
    // grid-adequacy/tests/pricing_delivered.rs).
    let col = |name: &str| header.split(',').position(|c| c == name).unwrap();
    let (potential_col, delivered_col) = (
        col("wind_capture_ratio"),
        col("wind_capture_ratio_delivered"),
    );
    for line in &data_lines[1..] {
        let potential = field(line, potential_col);
        let delivered = field(line, delivered_col);
        assert!(
            delivered >= potential,
            "delivered capture ratio {delivered} below potential {potential}: {line}"
        );
    }
    // Cross-basis wiring lock (Package A review condition 1a): at the
    // 40 GW point (the last row) curtailment is ~0.8 TWh, so the bases
    // MUST separate strictly (delivered ≈ 0.7175 vs potential
    // ≈ 0.7129). If the CLI ever wires the potential series into the
    // delivered column, the columns are bit-identical and this fails.
    let last = data_lines.last().unwrap();
    let (potential, delivered) = (field(last, potential_col), field(last, delivered_col));
    assert!(
        delivered > potential,
        "at 40 GW the delivered capture ratio ({delivered}) must sit STRICTLY above the \
         potential one ({potential}) — equal values mean the delivered wiring is a no-op"
    );

    let png = std::fs::read(dir.join("module1_gas_marginal_vs_wind.png")).unwrap();
    assert!(png.len() > 10_000, "PNG suspiciously small: {}", png.len());
    assert_eq!(&png[1..4], b"PNG");
}

/// Pinned regression for the Module 1 60 GW-wind headline row (the
/// high-penetration cannibalisation point quoted for Q10/Q2). The
/// audit found the three cells published from this row unpinned — only
/// the capture-ratio and curtailment columns were guarded. This pins
/// the frozen-imports-convention (default) row exactly (measured
/// 2026-07-04; docs/notes/stage-2-2024-run-report.md §3).
#[test]
fn sweep_wind_capacity_pins_the_60gw_frozen_row() {
    require_pack();
    let dir = fresh_dir("sweep-module1-60gw");
    let out = grid_cli(&[
        "sweep",
        "wind-capacity",
        "--scenario",
        REFERENCE,
        "--out",
        dir.to_str().unwrap(),
        "--min-gw",
        "60",
        "--max-gw",
        "60",
        "--step-gw",
        "10",
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let csv = std::fs::read_to_string(dir.join("module1_gas_marginal_vs_wind.csv")).unwrap();
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 1 + 1, "1 sweep point + header");
    let header = data_lines[0];
    let row = data_lines[1];
    let col = |name: &str| header.split(',').position(|c| c == name).unwrap();
    let cell = |name: &str| -> f64 { row.split(',').nth(col(name)).unwrap().parse().unwrap() };

    // Sanity: this really is the 60 GW row.
    assert!((cell("wind_capacity_gw") - 60.0).abs() < 1e-9);

    // The three unpinned published cells, each pinned exactly.
    for (name, pinned, tol) in [
        ("pct_periods_gas_price_setting", 46.46516393442623, 1e-3),
        ("gas_twh", 33.2128204430285, 1e-3),
        ("mean_smp_gbp_per_mwh", 37.13886422215412, 1e-3),
    ] {
        let value = cell(name);
        assert!(
            (value - pinned).abs() <= tol,
            "60 GW sweep {name} = {value} differs from the pinned {pinned} \
             (±{tol}) — if the change is intentional, update this pin AND \
             docs/notes/stage-2-2024-run-report.md §3 together"
        );
    }
}

/// Package B, no silent default: a scenario without interconnector
/// links must refuse the sweep unless `--export-capacity-gw` is given
/// (the export-in-surplus bracket has no capability to use otherwise).
#[test]
fn sweep_wind_capacity_without_links_requires_explicit_export_capacity() {
    require_pack();
    let dir = fresh_dir("sweep-no-links");
    // The reference scenario with its `[[links]]` block stripped (the
    // block sits between the interconnectors comment and [dispatch]).
    let text = std::fs::read_to_string(repo_root().join(REFERENCE)).unwrap();
    let start = text
        .find("# --- Interconnectors")
        .expect("reference scenario carries the interconnectors block");
    let end = text[start..]
        .find("[dispatch]")
        .expect("[dispatch] follows the links block")
        + start;
    let stripped = format!("{}{}", &text[..start], &text[end..]);
    std::fs::create_dir_all(&dir).unwrap();
    let scenario_path = dir.join("no-links.toml");
    std::fs::write(&scenario_path, stripped).unwrap();

    let out = grid_cli(&[
        "sweep",
        "wind-capacity",
        "--scenario",
        scenario_path.to_str().unwrap(),
        "--out",
        dir.join("out").to_str().unwrap(),
        "--min-gw",
        "30",
        "--max-gw",
        "30",
        "--step-gw",
        "5",
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "a link-less scenario without --export-capacity-gw must fail"
    );
    let err = stderr(&out);
    assert!(
        err.contains("--export-capacity-gw") && err.contains("no silent default"),
        "error must name the missing parameter and the no-silent-default rule: {err}"
    );
}

// ---------------------------------------------------------------------
// Stage 1 demo artefact: modelled vs. actual monthly generation stack.
// ---------------------------------------------------------------------

#[test]
fn plot_renders_the_monthly_mix_chart_with_footer_metadata() {
    require_pack();
    let dir = fresh_dir("plot-run");
    assert_eq!(run_to(&dir).status.code(), Some(0));

    let png = dir.join("monthly_mix_2024.png");
    let out = grid_cli(&[
        "plot",
        "monthly-mix",
        "--run",
        dir.to_str().unwrap(),
        "--actual",
        "data/packs/2024/processed/monthly_generation_2024.csv",
        "--out",
        png.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    let bytes = std::fs::read(&png).unwrap();
    assert!(
        bytes.len() > 10_000,
        "PNG suspiciously small: {} bytes",
        bytes.len()
    );
    assert_eq!(&bytes[1..4], b"PNG");
}

// ---------------------------------------------------------------------
// Stage 3 demo artefact: `plot soc-trace` — store state of charge over
// the run horizon (PNG + CSV, docs/06 hash footers). On the 40-year RS
// run this is the multi-week-drawdown / multi-year-recharge picture;
// the test drives it with the benign-battery run (any run with stores).
// ---------------------------------------------------------------------

#[test]
fn plot_soc_trace_writes_png_and_csv_with_metadata() {
    require_pack();
    let dir = fresh_dir("plot-soc-trace");
    let out = grid_cli(&["run", "--scenario", BENIGN, "--out", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let png = dir.join("soc_trace.png");
    let out = grid_cli(&[
        "plot",
        "soc-trace",
        "--run",
        dir.to_str().unwrap(),
        "--out",
        png.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    // The PNG (docs/06: hashes in a footer caption).
    let bytes = std::fs::read(&png).unwrap();
    assert!(
        bytes.len() > 10_000,
        "PNG suspiciously small: {} bytes",
        bytes.len()
    );
    assert_eq!(&bytes[1..4], b"PNG");

    // The companion CSV: the SoC series alone, with the docs/06
    // metadata header, one row per period, one column per store.
    let csv = std::fs::read_to_string(dir.join("soc_trace.csv")).unwrap();
    for needle in [
        "engine_git_hash",
        "scenario_sha256",
        "schema_version",
        "created_utc",
    ] {
        assert!(csv.contains(needle), "soc_trace.csv lacks {needle}");
    }
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(data_lines.len(), 17_568 + 1);
    assert_eq!(data_lines[0], "utc_start,battery_soc_gwh");
    assert!(data_lines[1].starts_with("2024-01-01T00:00:00Z,"));
}

#[test]
fn plot_soc_trace_on_a_storageless_run_exits_two() {
    require_pack();
    let dir = fresh_dir("plot-soc-trace-storageless");
    // A run of the benign scenario with its storage table stripped (cut
    // from the storage table to the dispatch table).
    let full = std::fs::read_to_string(repo_root().join(BENIGN)).unwrap();
    let start = full.find("[[zones.storage]]").unwrap();
    let end = full.find("[dispatch]").unwrap();
    let scenario_path = dir.join("no-storage.toml");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        &scenario_path,
        format!("{}{}", &full[..start], &full[end..]),
    )
    .unwrap();
    let out = grid_cli(&[
        "run",
        "--scenario",
        scenario_path.to_str().unwrap(),
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    let out = grid_cli(&[
        "plot",
        "soc-trace",
        "--run",
        dir.to_str().unwrap(),
        "--out",
        dir.join("soc.png").to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("soc"),
        "the error should say there are no SoC columns: {}",
        stderr(&out)
    );
}

// ---------------------------------------------------------------------
// Stage 3: `solve` — min_storage_for_zero_unserved bisection (ADR-10),
// replacing the Stage 0 stub. Exit codes: 0 solved, 1 model
// infeasibility, 2 usage errors (docs/06).
// ---------------------------------------------------------------------

const BENIGN: &str = "scenarios/gb-2024-benign-battery.toml";

#[test]
fn solve_writes_the_bisection_trace_and_summary() {
    require_pack();
    let dir = fresh_dir("solve-benign");
    let out = grid_cli(&[
        "solve",
        "--scenario",
        BENIGN,
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));

    // CSV + Parquet, both, always (docs/06), with the metadata block.
    let csv = std::fs::read_to_string(dir.join("solve.csv")).unwrap();
    assert!(dir.join("solve.parquet").exists());
    for needle in ["engine_git_hash", "scenario_sha256", "data_file"] {
        assert!(csv.contains(needle), "solve.csv lacks {needle}");
    }
    let data_lines: Vec<&str> = csv.lines().filter(|l| !l.starts_with('#')).collect();
    assert!(data_lines[0].contains("phase,iteration,energy_gwh,unserved_gwh,feasible"));
    assert!(data_lines.len() > 5, "full bisection trace expected");
    // The first evaluation is zero capacity, infeasible (the benign
    // fleet runs short without its battery).
    assert!(data_lines[1].starts_with("naive,0,0,"), "{}", data_lines[1]);
    assert!(data_lines[1].ends_with("false"), "{}", data_lines[1]);

    let summary = std::fs::read_to_string(dir.join("solve_summary.toml")).unwrap();
    for needle in [
        "mode = \"min_storage_for_zero_unserved\"",
        "store = \"battery\"",
        "requirement_gwh",
        "min_soc_gwh",
        "initial_condition_sensitive",
    ] {
        assert!(
            summary.contains(needle),
            "solve_summary.toml lacks {needle}"
        );
    }

    // The requirement is a sane figure for the measured 2024 shortfall
    // (19.1 GWh delivered, √0.88 discharge leg): well under the
    // scenario's 36 GWh, well over the delivered energy of the deepest
    // spell.
    let requirement: f64 = summary
        .lines()
        .find_map(|l| l.strip_prefix("requirement_gwh = "))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        requirement > 5.0 && requirement < 36.0,
        "requirement {requirement} GWh outside the plausible band"
    );
}

#[test]
fn solve_with_an_undersized_store_power_exits_one() {
    require_pack();
    let dir = fresh_dir("solve-infeasible");
    // The benign fleet's deepest post-stack deficit is ≈ 2.5 GW; a
    // 0.5 GW store can never cover it, whatever its energy capacity.
    let toml = std::fs::read_to_string(repo_root().join(BENIGN))
        .unwrap()
        .replace("power_gw = 3.0", "power_gw = 0.5");
    let scenario_path = dir.join("undersized.toml");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&scenario_path, toml).unwrap();
    let out = grid_cli(&[
        "solve",
        "--scenario",
        scenario_path.to_str().unwrap(),
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "model infeasibility is exit 1 (docs/06); stderr: {}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("infeasible"), "{}", stderr(&out));
}

#[test]
fn solve_with_an_unknown_store_exits_two() {
    let dir = fresh_dir("solve-bad-store");
    let out = grid_cli(&[
        "solve",
        "--scenario",
        BENIGN,
        "--store",
        "flywheel",
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(2));
    let text = stderr(&out);
    assert!(text.contains("flywheel"), "stderr: {text}");
    assert!(
        text.contains("battery"),
        "stderr should list the stores: {text}"
    );
}
