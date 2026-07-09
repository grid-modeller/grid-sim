//! `grid-cli solve` — `min_storage_for_zero_unserved` (docs/04 Stage 3,
//! ADR-10), replacing the Stage 0 stub.
//!
//! Bisects the energy capacity of one designated store (selected with
//! `--store`, by output label; optional when the scenario has exactly
//! one store) for zero unserved energy over the horizon, holding the
//! store's power rating, its `initial_soc` fraction and everything else
//! fixed — the full parameterisation is documented in
//! `grid_adequacy::solve`. The scenario's optional `[solver]` block, if
//! present, must name this mode.
//!
//! Outputs (docs/06: CSV + Parquet, both, always, plus a TOML summary,
//! all carrying the metadata block):
//!
//! - `solve.csv` / `solve.parquet` — the full bisection trace, one row
//!   per evaluation: phase (`naive` / `burn_in`), candidate energy,
//!   unserved energy, feasibility;
//! - `solve_summary.toml` — the requirement, the designated store, the
//!   D4 initial-SoC guard outputs (min SoC + instant, the
//!   initial-condition-sensitive flag, and the burn-in requirement when
//!   re-run — both figures reported).
//!
//! Exit codes (docs/06): 0 solved, **1 model infeasibility** (no store
//! size reaches zero unserved), 2 usage/scenario errors.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{BooleanBuilder, Float64Builder, StringBuilder, UInt64Builder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema};
use clap::Args;
use grid_adequacy::{BisectionOutcome, SolveOptions, SolveResult, load_run_inputs};
use grid_core::GridError;
use grid_core::scenario::{SCHEMA_VERSION, Scenario};
use sha2::{Digest, Sha256};

use crate::run::{Metadata, now_utc, scenario_data_files, sha256_file};

/// The one solver mode implemented (ADR-10; docs/04 Stage 3).
const MODE: &str = "min_storage_for_zero_unserved";

/// Arguments to `grid-cli solve`.
#[derive(Args)]
pub struct SolveArgs {
    /// Scenario TOML file (schema v2: self-contained).
    #[arg(long)]
    scenario: PathBuf,

    /// Output directory (created if absent).
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which relative trace paths in the scenario
    /// are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Designated store to size, by output label (the kind, e.g.
    /// `hydrogen`, or `kind_order` when a kind repeats). Optional when
    /// the scenario has exactly one store.
    #[arg(long)]
    store: Option<String>,
}

/// A solve failure with its docs/06 exit code: 1 for model
/// infeasibility, 2 for usage/scenario errors.
pub struct Failure {
    /// Human-readable message.
    pub message: String,
    /// Process exit code.
    pub exit_code: u8,
}

impl Failure {
    /// A usage/scenario error (exit 2) — shared with `fetch-data`, which
    /// uses the same failure-with-exit-code shape.
    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            exit_code: 2,
        }
    }
}

impl From<GridError> for Failure {
    fn from(error: GridError) -> Self {
        Self {
            message: error.to_string(),
            exit_code: if matches!(error, GridError::SolveInfeasible { .. }) {
                1
            } else {
                2
            },
        }
    }
}

/// The store output label the engine will use (kind, or `kind_order`
/// for repeated kinds) — mirrors the engine's disambiguation rule.
fn store_labels(scenario: &Scenario) -> Vec<String> {
    let storage = &scenario.zones[0].storage;
    storage
        .iter()
        .map(|store| {
            if storage.iter().filter(|s| s.kind == store.kind).count() > 1 {
                format!("{}_{}", store.kind, store.dispatch_order)
            } else {
                store.kind.as_str().to_owned()
            }
        })
        .collect()
}

/// Execute `grid-cli solve`.
pub fn execute(args: &SolveArgs) -> Result<(), Failure> {
    let scenario = Scenario::load(&args.scenario)?;
    if let Some(solver) = &scenario.solver
        && solver.mode != MODE
    {
        return Err(Failure::usage(format!(
            "scenario [solver] mode {:?} is not implemented; the only mode is {MODE:?}",
            solver.mode
        )));
    }
    if scenario.zones.len() != 1 {
        return Err(GridError::MultiZoneUnsupported {
            found: scenario.zones.len(),
        }
        .into());
    }

    // Designate the store.
    let labels = store_labels(&scenario);
    let store_index = match &args.store {
        Some(wanted) => labels.iter().position(|l| l == wanted).ok_or_else(|| {
            Failure::usage(format!(
                "--store {wanted:?} does not name a store; the scenario's stores are [{}]",
                labels.join(", ")
            ))
        })?,
        None => {
            if labels.len() == 1 {
                0
            } else {
                return Err(Failure::usage(format!(
                    "the scenario has {} stores; pick one with --store (one of [{}])",
                    labels.len(),
                    labels.join(", ")
                )));
            }
        }
    };

    let inputs = load_run_inputs(&scenario, &args.base_dir)?;
    let options = SolveOptions::default();
    println!(
        "solving {MODE} for store {} (bisection; tolerance max({} GWh, {} relative))…",
        labels[store_index],
        options.absolute_tolerance.as_gigawatt_hours(),
        options.relative_tolerance
    );
    let result =
        grid_adequacy::min_storage_for_zero_unserved(&scenario, &inputs, store_index, &options)?;

    // Metadata + outputs.
    let meta = Metadata {
        engine_git_hash: env!("GRID_ENGINE_GIT_HASH"),
        schema_version: SCHEMA_VERSION,
        scenario_path: args.scenario.display().to_string(),
        scenario_sha256: sha256_file(&args.scenario).map_err(Failure::usage)?,
        created_utc: now_utc(),
        data_files: scenario_data_files(&scenario, &args.base_dir).map_err(Failure::usage)?,
    };
    std::fs::create_dir_all(&args.out)
        .map_err(|e| Failure::usage(format!("cannot create {}: {e}", args.out.display())))?;

    let csv = solve_csv(&result, &meta);
    let csv_path = args.out.join("solve.csv");
    std::fs::write(&csv_path, &csv)
        .map_err(|e| Failure::usage(format!("cannot write {}: {e}", csv_path.display())))?;
    write_solve_parquet(&args.out.join("solve.parquet"), &result, &meta).map_err(Failure::usage)?;
    let summary = summary_toml(&result, &meta, &csv);
    let summary_path = args.out.join("solve_summary.toml");
    std::fs::write(&summary_path, &summary)
        .map_err(|e| Failure::usage(format!("cannot write {}: {e}", summary_path.display())))?;

    // Headline.
    println!(
        "requirement: {:.3} GWh of {} energy for zero unserved ({} evaluations)",
        result.naive.requirement.as_gigawatt_hours(),
        result.store_label,
        result.naive.iterations.len()
    );
    println!(
        "  min SoC at that size: {:.3} GWh at {}{}",
        result.min_soc.as_gigawatt_hours(),
        result.min_soc_at,
        if result.initial_condition_sensitive {
            " — WITHIN THE FIRST WEATHER YEAR (initial-condition-sensitive, D4 guard)"
        } else {
            ""
        }
    );
    if let Some(burn_in) = &result.burn_in {
        println!(
            "  one-year burn-in requirement: {:.3} GWh ({} evaluations) — both figures \
             reported (D4)",
            burn_in.requirement.as_gigawatt_hours(),
            burn_in.iterations.len()
        );
    }
    if let Some(reason) = &result.burn_in_skipped {
        println!("  burn-in re-run skipped: {reason}");
    }
    println!("  outputs in {}", args.out.display());
    Ok(())
}

/// Iterate both phases' iterations with their phase names.
fn phases(result: &SolveResult) -> Vec<(&'static str, &BisectionOutcome)> {
    let mut out = vec![("naive", &result.naive)];
    if let Some(burn_in) = &result.burn_in {
        out.push(("burn_in", burn_in));
    }
    out
}

fn solve_csv(result: &SolveResult, meta: &Metadata) -> String {
    let mut text = String::from("# grid-cli solve output (docs/06 metadata header)\n");
    for (key, value) in meta.pairs() {
        text.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &meta.data_files {
        text.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    text.push_str("phase,iteration,energy_gwh,unserved_gwh,feasible\n");
    for (phase, outcome) in phases(result) {
        for (index, it) in outcome.iterations.iter().enumerate() {
            text.push_str(&format!(
                "{phase},{index},{},{},{}\n",
                it.energy.as_gigawatt_hours(),
                it.unserved.as_gigawatt_hours(),
                it.feasible
            ));
        }
    }
    text
}

fn write_solve_parquet(path: &Path, result: &SolveResult, meta: &Metadata) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let schema = Arc::new(Schema::new(vec![
        Field::new("phase", DataType::Utf8, false),
        Field::new("iteration", DataType::UInt64, false),
        Field::new("energy_gwh", DataType::Float64, false),
        Field::new("unserved_gwh", DataType::Float64, false),
        Field::new("feasible", DataType::Boolean, false),
    ]));
    let mut phase_col = StringBuilder::new();
    let mut iteration = UInt64Builder::new();
    let mut energy = Float64Builder::new();
    let mut unserved = Float64Builder::new();
    let mut feasible = BooleanBuilder::new();
    for (phase, outcome) in phases(result) {
        for (index, it) in outcome.iterations.iter().enumerate() {
            phase_col.append_value(phase);
            iteration.append_value(index as u64);
            energy.append_value(it.energy.as_gigawatt_hours());
            unserved.append_value(it.unserved.as_gigawatt_hours());
            feasible.append_value(it.feasible);
        }
    }
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(phase_col.finish()),
        Arc::new(iteration.finish()),
        Arc::new(energy.finish()),
        Arc::new(unserved.finish()),
        Arc::new(feasible.finish()),
    ];
    let batch = RecordBatch::try_new(schema.clone(), arrays).map_err(|e| err(&e))?;

    let mut kv = Vec::new();
    for (key, value) in meta.pairs() {
        kv.push(parquet::file::metadata::KeyValue::new(
            key.to_owned(),
            value,
        ));
    }
    for (data_path, hash) in &meta.data_files {
        kv.push(parquet::file::metadata::KeyValue::new(
            format!("data_file_sha256:{data_path}"),
            hash.clone(),
        ));
    }
    let props = parquet::file::properties::WriterProperties::builder()
        .set_key_value_metadata(Some(kv))
        .build();
    let file = std::fs::File::create(path).map_err(|e| err(&e))?;
    let mut writer =
        parquet::arrow::ArrowWriter::try_new(file, schema, Some(props)).map_err(|e| err(&e))?;
    writer.write(&batch).map_err(|e| err(&e))?;
    writer.close().map_err(|e| err(&e))?;
    Ok(())
}

fn toml_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn summary_toml(result: &SolveResult, meta: &Metadata, csv: &str) -> String {
    // Digest over the CSV data section (rows after the metadata
    // comments), mirroring the run outputs' determinism pin.
    let data_section: String = csv
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));

    let mut text = String::from("# grid-cli solve summary (docs/06)\n[metadata]\n");
    for (key, value) in meta.pairs() {
        let value = if key == "schema_version" {
            value
        } else {
            toml_quote(&value)
        };
        text.push_str(&format!("{key} = {value}\n"));
    }
    text.push_str("\n[metadata.data_files]\n");
    for (path, hash) in &meta.data_files {
        text.push_str(&format!("{} = {}\n", toml_quote(path), toml_quote(hash)));
    }
    text.push_str(&format!(
        "\n[results]\nmode = {}\nstore = {}\nsolve_digest_sha256 = {}\n",
        toml_quote(MODE),
        toml_quote(&result.store_label),
        toml_quote(&digest)
    ));
    text.push_str(&format!(
        "requirement_gwh = {}\n",
        result.naive.requirement.as_gigawatt_hours()
    ));
    text.push_str(&format!(
        "evaluations = {}\n",
        result.naive.iterations.len()
    ));
    text.push_str(&format!(
        "min_soc_gwh = {}\nmin_soc_at = {}\n",
        result.min_soc.as_gigawatt_hours(),
        toml_quote(&result.min_soc_at.to_string())
    ));
    text.push_str(&format!(
        "initial_condition_sensitive = {}\n",
        result.initial_condition_sensitive
    ));
    if let Some(burn_in) = &result.burn_in {
        text.push_str(&format!(
            "burn_in_requirement_gwh = {}\nburn_in_evaluations = {}\n",
            burn_in.requirement.as_gigawatt_hours(),
            burn_in.iterations.len()
        ));
    }
    if let Some(reason) = &result.burn_in_skipped {
        text.push_str(&format!("burn_in_skipped = {}\n", toml_quote(reason)));
    }
    text
}
