//! `grid-cli sweep` — parameter sweeps.
//!
//! Stage 4 implements the generic sweep runner (`sweep grid`: a TOML
//! sweep spec → the full response surface, CSV + Parquet + a heatmap
//! chart for 2-D sweeps, evaluated by `grid_adequacy::sweep` with rayon
//! and bit-identical to serial — ADR-10) and the Q4 per-year batch mode
//! (`sweep per-year`: every weather year as an independent single-year
//! `min_storage_for_zero_unserved` solve).
//!
//! The Q5/Q11 analysis runs add `sweep heating-mix`
//! (`grid_adequacy::heating_mix` — D9 rules 6/6b, fixed-fleet leg): the
//! ASHP/GSHP/district simplex at fixed heat decarbonisation, one
//! bisection solve per point at a STATED store rating (required CLI
//! argument, stamped per output row — the q5-heating-engine-review
//! binding record), the no-heating baseline for the deltas, the
//! timescale decomposition of the added requirement at the three
//! pure-technology corners, and the two-gradient chart. Every artefact
//! carries the D9 rule-6 assumption block
//! ([`heating_mix_assumption_lines`]), including the verbatim
//! store-rating quoting duty and the ELCC-runner dependency note for
//! the (out-of-scope) capacity-relieved leg.
//!
//! Stage 2's hand-rolled `sweep wind-capacity` (the **Module 1 demo
//! artefact**: % of periods gas-marginal vs installed wind capacity,
//! 10→60 GW) is kept as-is — its outputs are pinned by the Stage 2
//! regression tests, and it needs the pricing layer per point, which
//! the generic runner deliberately does not price.
//!
//! # Sweep assumptions (documented here and in the output CSV — they are
//! contestable and will be revisited in Stages 4/5)
//!
//! 1. **Onshore and offshore wind scale proportionally** from the
//!    reference split (14.4 GW onshore / 14.7 GW offshore of the 29.1 GW
//!    end-2024 GB fleet), keeping the ERA5-derived 2024 CF *shapes* —
//!    no locational shift of new capacity, no wake/technology change.
//! 2. **Everything else is held at 2024 observed values**: demand,
//!    the thermal fleet and its availability calibration, solar, and the
//!    exogenous must-take traces (net imports, pumped-storage net,
//!    FUELHH "other"). In reality imports respond to prices (Stage 5),
//!    storage would charge on surplus (Stage 3), and demand grows —
//!    high-wind points therefore *overstate* curtailment and understate
//!    absorbed wind. That bias is the point of revisiting in Stage 4/5.
//! 3. **Curtailment is pooled** (Stage 1 convention): the % reported is
//!    pooled curtailed energy over total *renewable potential* energy
//!    (wind + solar potential), with no attribution to individual
//!    sources (per-increment attribution is Q2).
//!
//! Outputs (both under `--out`, typically `runs/…`, git-ignored):
//! `module1_gas_marginal_vs_wind.csv` (docs/06 metadata header +
//! assumptions block) and `module1_gas_marginal_vs_wind.png` (hash
//! footer).

use std::path::PathBuf;

use clap::{Args, Subcommand};
use grid_adequacy::{
    Execution, MultiZoneWindPoint, RunResult, load_multi_zone_inputs, load_pricing_inputs,
    load_run_inputs, price_run, wind_capacity_sweep_multi, wind_capacity_sweep_multi_group,
};
use grid_core::pricing::price_setting_share;
use grid_core::scenario::Scenario;
use grid_core::units::{Energy, Power};
use plotters::prelude::*;
use sha2::{Digest, Sha256};

/// Arguments to `grid-cli sweep`.
#[derive(Args)]
pub struct SweepArgs {
    #[command(subcommand)]
    sweep: SweepCommand,
}

#[derive(Subcommand)]
enum SweepCommand {
    /// Generic 1-D/2-D parameter sweep from a TOML spec (Stage 4):
    /// full response surface persisted as CSV + Parquet, heatmap chart
    /// for 2-D sweeps.
    Grid(GridArgs),
    /// Q5/Q11 heating-mix simplex sweep (D9 rules 6/6b): sweep the
    /// ASHP/GSHP/district shares at fixed heat decarbonisation on a
    /// heated scenario, solving the storage requirement per point at a
    /// STATED store rating, plus the timescale decomposition of the
    /// added requirement at the three pure-technology corners.
    HeatingMix(HeatingMixArgs),
    /// Q4 per-year batch (Stage 4): solve the minimum store size for
    /// zero unserved for every weather year as an independent
    /// single-year scenario.
    PerYear(PerYearArgs),
    /// Module 1 (docs/04 Stage 2): sweep installed wind capacity,
    /// scaling onshore + offshore proportionally, and record the gas
    /// price-setting share and curtailment per point.
    WindCapacity(WindCapacityArgs),
    /// Locational counterpart of `wind-capacity` (D11 rule 2 / D13
    /// group; CLI exposure per docs/notes/wind-capacity-zonal-work-
    /// order.md): sweep one zone's — or, with repeated `--zone`, a zone
    /// group's — installed wind on the priced multi-zone engine, with
    /// imports ENDOGENOUS through the scenario's links, and record the
    /// swept zone's capture ratio, mean SMP, curtailment, gas and net
    /// imports per step.
    WindCapacityZonal(WindCapacityZonalArgs),
}

#[derive(Args)]
struct WindCapacityArgs {
    /// Scenario TOML file (the reference fleet to scale; must declare a
    /// [pricing] block).
    #[arg(long)]
    scenario: PathBuf,

    /// Output directory (created if absent).
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which relative trace paths are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Smallest total wind capacity, GW.
    #[arg(long, default_value_t = 10.0)]
    min_gw: f64,

    /// Largest total wind capacity, GW.
    #[arg(long, default_value_t = 60.0)]
    max_gw: f64,

    /// Capacity step, GW.
    #[arg(long, default_value_t = 5.0)]
    step_gw: f64,

    /// Export capability (GW) capping the export-in-surplus bracket
    /// convention (Package B). Overrides the scenario-derived value;
    /// REQUIRED when the scenario declares no interconnector links —
    /// there is no silent default.
    #[arg(long)]
    export_capacity_gw: Option<f64>,
}

/// One sweep point's results. The capture ratio carries BOTH bases
/// (Package A): `wind_capture_ratio` is the unchanged potential-basis
/// convention; `wind_capture_ratio_delivered` removes each period's
/// pro-rata share of the pooled curtailment from the wind series first
/// (definitions in prose at `grid_adequacy::pricing`).
///
/// Package B adds the import-convention bracket (definitions in prose
/// at `grid_adequacy::import_convention`): the unsuffixed fields are
/// the FROZEN convention (the pre-Package-B default, bit-identical);
/// `*_imports_zero` is ZERO-IN-SURPLUS and `*_imports_export` is
/// EXPORT-IN-SURPLUS, each re-dispatched and re-priced from the
/// transformed exogenous import trace.
struct SweepPoint {
    wind_capacity_gw: f64,
    pct_gas_price_setting: f64,
    curtailment_twh: f64,
    curtailment_pct_of_renewable_potential: f64,
    gas_twh: f64,
    mean_smp_gbp_per_mwh: f64,
    wind_capture_ratio: Option<f64>,
    wind_capture_ratio_delivered: Option<f64>,
    curtailment_twh_imports_zero: f64,
    wind_capture_ratio_imports_zero: Option<f64>,
    wind_capture_ratio_delivered_imports_zero: Option<f64>,
    curtailment_twh_imports_export: f64,
    wind_capture_ratio_imports_export: Option<f64>,
    wind_capture_ratio_delivered_imports_export: Option<f64>,
}

/// Execute `grid-cli sweep`.
pub fn execute(args: &SweepArgs) -> Result<(), String> {
    match &args.sweep {
        SweepCommand::Grid(args) => grid(args),
        SweepCommand::HeatingMix(args) => heating_mix(args),
        SweepCommand::PerYear(args) => per_year(args),
        SweepCommand::WindCapacity(args) => wind_capacity(args),
        SweepCommand::WindCapacityZonal(args) => wind_capacity_zonal(args),
    }
}

fn twh(e: Energy) -> f64 {
    e.as_gigawatt_hours() / 1000.0
}

// ---------------------------------------------------------------------
// `sweep grid` — the generic Stage 4 sweep runner.
// ---------------------------------------------------------------------

#[derive(Args)]
struct GridArgs {
    /// Sweep spec TOML (names the scenario and one or two dimensions —
    /// see `grid_adequacy::sweep::SweepSpec`).
    #[arg(long)]
    spec: PathBuf,

    /// Output directory (created if absent): sweep.csv, sweep.parquet,
    /// and surface.png for 2-D sweeps.
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which the spec's scenario path and the
    /// scenario's trace paths are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Force the serial execution path (the default is rayon; results
    /// are bit-identical either way — asserted by the acceptance
    /// suite).
    #[arg(long)]
    serial: bool,
}

fn grid(args: &GridArgs) -> Result<(), String> {
    let spec = grid_adequacy::SweepSpec::load(&args.spec).map_err(|e| e.to_string())?;
    let scenario_path = args.base_dir.join(&spec.scenario);
    let scenario = Scenario::load(&scenario_path).map_err(|e| e.to_string())?;
    let dimensions = spec.resolve(&scenario).map_err(|e| e.to_string())?;
    let execution = if args.serial {
        grid_adequacy::Execution::Serial
    } else {
        grid_adequacy::Execution::Parallel
    };

    let started = std::time::Instant::now();
    let surface = grid_adequacy::run_sweep(&scenario, &args.base_dir, &dimensions, execution)
        .map_err(|e| e.to_string())?;
    let elapsed = started.elapsed().as_secs_f64();

    // Metadata (docs/06): engine + spec + scenario + data-file hashes.
    let engine = env!("GRID_ENGINE_GIT_HASH");
    let spec_sha = crate::run::sha256_file(&args.spec)?;
    let scenario_sha = crate::run::sha256_file(&scenario_path)?;
    let data_files = crate::run::scenario_data_files(&scenario, &args.base_dir)?;
    let created = crate::run::now_utc();
    let mut meta: Vec<(String, String)> = vec![
        ("engine_git_hash".to_owned(), engine.to_owned()),
        (
            "sweep_spec_path".to_owned(),
            args.spec.display().to_string(),
        ),
        ("sweep_spec_sha256".to_owned(), spec_sha),
        (
            "scenario_path".to_owned(),
            scenario_path.display().to_string(),
        ),
        ("scenario_sha256".to_owned(), scenario_sha.clone()),
        ("created_utc".to_owned(), created),
    ];
    if let Some(name) = &spec.name {
        meta.push(("sweep_name".to_owned(), name.clone()));
    }

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // Column layout: one coordinate column per dimension, then the
    // fixed metrics, then one min-SoC column per store.
    let store_labels: Vec<String> = surface
        .points
        .first()
        .map(|p| p.store_min_soc.iter().map(|(l, _)| l.clone()).collect())
        .unwrap_or_default();

    // --- CSV (full response surface — ADR-10). ---
    let mut csv = String::from("# grid-sim sweep grid output (docs/06 metadata header)\n");
    for (key, value) in &meta {
        csv.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &data_files {
        csv.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    for dimension in &surface.dimensions {
        csv.push_str(&format!("{},", dimension.column()));
    }
    csv.push_str(
        "demand_twh,renewable_potential_twh,unserved_gwh,unserved_periods,curtailment_twh",
    );
    for label in &store_labels {
        csv.push_str(&format!(",min_soc_{label}_gwh"));
    }
    csv.push('\n');
    for point in &surface.points {
        for (dimension, &index) in surface.dimensions.iter().zip(&point.indices) {
            csv.push_str(&format!("{},", dimension.coordinate(index)));
        }
        csv.push_str(&format!(
            "{},{},{},{},{}",
            twh(point.demand),
            twh(point.renewable_potential),
            point.unserved.as_gigawatt_hours(),
            point.unserved_periods,
            twh(point.curtailment),
        ));
        for (_, soc) in &point.store_min_soc {
            csv.push_str(&format!(",{}", soc.as_gigawatt_hours()));
        }
        csv.push('\n');
    }
    let csv_path = args.out.join("sweep.csv");
    std::fs::write(&csv_path, &csv)
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // --- Parquet (same table — docs/06: both, always). ---
    let mut fields: Vec<arrow_schema::Field> = Vec::new();
    let mut arrays: Vec<arrow_array::ArrayRef> = Vec::new();
    let f64_column = |values: Vec<f64>| -> arrow_array::ArrayRef {
        std::sync::Arc::new(arrow_array::Float64Array::from(values))
    };
    for (d, dimension) in surface.dimensions.iter().enumerate() {
        fields.push(arrow_schema::Field::new(
            dimension.column(),
            arrow_schema::DataType::Float64,
            false,
        ));
        arrays.push(f64_column(
            surface
                .points
                .iter()
                .map(|p| dimension.coordinate(p.indices[d]))
                .collect(),
        ));
    }
    let metric = |name: &str,
                  values: Vec<f64>,
                  fields: &mut Vec<arrow_schema::Field>,
                  arrays: &mut Vec<arrow_array::ArrayRef>| {
        fields.push(arrow_schema::Field::new(
            name,
            arrow_schema::DataType::Float64,
            false,
        ));
        arrays.push(f64_column(values));
    };
    let points = &surface.points;
    metric(
        "demand_twh",
        points.iter().map(|p| twh(p.demand)).collect(),
        &mut fields,
        &mut arrays,
    );
    metric(
        "renewable_potential_twh",
        points.iter().map(|p| twh(p.renewable_potential)).collect(),
        &mut fields,
        &mut arrays,
    );
    metric(
        "unserved_gwh",
        points
            .iter()
            .map(|p| p.unserved.as_gigawatt_hours())
            .collect(),
        &mut fields,
        &mut arrays,
    );
    metric(
        "unserved_periods",
        points.iter().map(|p| p.unserved_periods as f64).collect(),
        &mut fields,
        &mut arrays,
    );
    metric(
        "curtailment_twh",
        points.iter().map(|p| twh(p.curtailment)).collect(),
        &mut fields,
        &mut arrays,
    );
    for (s, label) in store_labels.iter().enumerate() {
        metric(
            &format!("min_soc_{label}_gwh"),
            points
                .iter()
                .map(|p| p.store_min_soc[s].1.as_gigawatt_hours())
                .collect(),
            &mut fields,
            &mut arrays,
        );
    }
    let parquet_path = args.out.join("sweep.parquet");
    write_table_parquet(&parquet_path, fields, arrays, &meta, &data_files)?;

    // --- Heatmap chart for 2-D sweeps. ---
    let mut chart_path = None;
    if surface.dimensions.len() == 2 {
        let path = args.out.join("surface.png");
        render_surface_chart(&path, &surface, engine, &scenario_sha)?;
        chart_path = Some(path);
    }

    println!(
        "sweep grid complete: {} points in {elapsed:.1} s ({})",
        surface.points.len(),
        if args.serial { "serial" } else { "rayon" },
    );
    println!("  table {}", csv_path.display());
    println!("  table {}", parquet_path.display());
    if let Some(path) = chart_path {
        println!("  chart {}", path.display());
    }
    Ok(())
}

/// Write a small numeric table as Parquet with the docs/06 metadata in
/// the footer key-value block.
pub(crate) fn write_table_parquet(
    path: &PathBuf,
    fields: Vec<arrow_schema::Field>,
    arrays: Vec<arrow_array::ArrayRef>,
    meta: &[(String, String)],
    data_files: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let schema = std::sync::Arc::new(arrow_schema::Schema::new(fields));
    let batch = arrow_array::RecordBatch::try_new(schema.clone(), arrays).map_err(|e| err(&e))?;
    let mut kv = Vec::new();
    for (key, value) in meta {
        kv.push(parquet::file::metadata::KeyValue::new(
            key.clone(),
            value.clone(),
        ));
    }
    for (data_path, hash) in data_files {
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

/// Cell edges for a (possibly non-uniform) coordinate axis: midpoints
/// between neighbours, end cells mirrored.
fn cell_edges(coords: &[f64]) -> Vec<f64> {
    if coords.len() == 1 {
        return vec![coords[0] - 0.5, coords[0] + 0.5];
    }
    let mut edges = Vec::with_capacity(coords.len() + 1);
    edges.push(coords[0] - (coords[1] - coords[0]) / 2.0);
    for pair in coords.windows(2) {
        edges.push((pair[0] + pair[1]) / 2.0);
    }
    let n = coords.len();
    edges.push(coords[n - 1] + (coords[n - 1] - coords[n - 2]) / 2.0);
    edges
}

/// The Module 4 surface chart: unserved energy over the 2-D grid,
/// zero-unserved (feasible) cells green, the rest on a log colour ramp.
fn render_surface_chart(
    path: &PathBuf,
    surface: &grid_adequacy::SweepResult,
    engine: &str,
    scenario_sha: &str,
) -> Result<(), String> {
    let d0 = &surface.dimensions[0];
    let d1 = &surface.dimensions[1];
    let x_coords: Vec<f64> = (0..d0.len()).map(|i| d0.coordinate(i)).collect();
    let y_coords: Vec<f64> = (0..d1.len()).map(|j| d1.coordinate(j)).collect();
    let x_edges = cell_edges(&x_coords);
    let y_edges = cell_edges(&y_coords);

    let unserved: Vec<f64> = surface
        .points
        .iter()
        .map(|p| p.unserved.as_gigawatt_hours())
        .collect();
    let positive_min = unserved
        .iter()
        .copied()
        .filter(|&u| u > 0.0)
        .fold(f64::INFINITY, f64::min);
    let max = unserved.iter().copied().fold(0.0f64, f64::max);

    let feasible = RGBColor(76, 165, 96);
    let colour = |u: f64| -> RGBColor {
        if u <= 0.0 {
            return feasible;
        }
        // Log ramp light-amber → dark red across the positive range.
        let f = if max > positive_min {
            ((u.ln() - positive_min.ln()) / (max.ln() - positive_min.ln())).clamp(0.0, 1.0)
        } else {
            1.0
        };
        RGBColor(
            (250.0 - 120.0 * f) as u8,
            (200.0 - 170.0 * f) as u8,
            (120.0 - 90.0 * f) as u8,
        )
    };

    let root = BitMapBackend::new(path, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(830);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            "Module 4 — the storage × overbuild triangle: unserved energy over 1985–2024 \
             (green = zero unserved)",
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(90)
        .build_cartesian_2d(
            x_edges[0]..x_edges[x_edges.len() - 1],
            y_edges[0]..y_edges[y_edges.len() - 1],
        )
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .disable_mesh()
        .x_desc(d0.column())
        .y_desc(d1.column())
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    chart
        .draw_series(surface.points.iter().map(|point| {
            let (i, j) = (point.indices[0], point.indices[1]);
            Rectangle::new(
                [(x_edges[i], y_edges[j]), (x_edges[i + 1], y_edges[j + 1])],
                colour(point.unserved.as_gigawatt_hours()).filled(),
            )
        }))
        .map_err(|e| e.to_string())?;

    let caption = format!(
        "grid-sim | engine {engine} | scenario sha256 {} | colour: unserved energy, log ramp \
         {positive_min:.1}..{max:.0} GWh; green = feasible (zero unserved)",
        short(scenario_sha),
    );
    footer
        .draw(&Text::new(
            caption,
            (20, 25),
            ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
        ))
        .map_err(|e| e.to_string())?;
    root.present().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// `sweep per-year` — the Q4 batch.
// ---------------------------------------------------------------------

#[derive(Args)]
struct PerYearArgs {
    /// Scenario TOML file (must reference per-year trace files).
    #[arg(long)]
    scenario: PathBuf,

    /// Output directory (created if absent): per_year.csv,
    /// per_year.parquet.
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which relative trace paths are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Index of the store the solver sizes (scenario order).
    #[arg(long, default_value_t = 0)]
    store_index: usize,

    /// First weather year of the batch.
    #[arg(long, default_value_t = 1985)]
    first_year: i32,

    /// Last weather year of the batch (inclusive).
    #[arg(long, default_value_t = 2024)]
    last_year: i32,

    /// Force the serial execution path.
    #[arg(long)]
    serial: bool,
}

fn per_year(args: &PerYearArgs) -> Result<(), String> {
    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    let execution = if args.serial {
        grid_adequacy::Execution::Serial
    } else {
        grid_adequacy::Execution::Parallel
    };
    let batch = grid_adequacy::per_year_requirements(
        &scenario,
        &args.base_dir,
        args.first_year..=args.last_year,
        args.store_index,
        &grid_adequacy::SolveOptions::default(),
        execution,
    )
    .map_err(|e| e.to_string())?;

    let engine = env!("GRID_ENGINE_GIT_HASH");
    let scenario_sha = crate::run::sha256_file(&args.scenario)?;
    let data_files = crate::run::scenario_data_files(&scenario, &args.base_dir)?;
    let meta: Vec<(String, String)> = vec![
        ("engine_git_hash".to_owned(), engine.to_owned()),
        (
            "scenario_path".to_owned(),
            args.scenario.display().to_string(),
        ),
        ("scenario_sha256".to_owned(), scenario_sha),
        ("store_index".to_owned(), args.store_index.to_string()),
        ("created_utc".to_owned(), crate::run::now_utc()),
    ];

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // --- CSV. ---
    let mut csv = String::from("# grid-sim per-year batch output (Q4; docs/06 metadata header)\n");
    for (key, value) in &meta {
        csv.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &data_files {
        csv.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    csv.push_str(
        "# note: single-year solves start with the store full on 1 January (D4 default) and \
         cannot run the burn-in guard; initial_condition_sensitive is reported per year\n",
    );
    csv.push_str(
        "year,feasible,requirement_gwh,min_soc_gwh,min_soc_at,initial_condition_sensitive,\
         infeasible_reason\n",
    );
    for year in &batch {
        match &year.outcome {
            grid_adequacy::YearOutcome::Feasible {
                requirement,
                min_soc,
                min_soc_at,
                initial_condition_sensitive,
            } => {
                csv.push_str(&format!(
                    "{},true,{},{},{},{},\n",
                    year.year,
                    requirement.as_gigawatt_hours(),
                    min_soc.as_gigawatt_hours(),
                    min_soc_at,
                    initial_condition_sensitive,
                ));
            }
            grid_adequacy::YearOutcome::Infeasible { reason } => {
                csv.push_str(&format!(
                    "{},false,,,,,\"{}\"\n",
                    year.year,
                    reason.replace('"', "'"),
                ));
            }
        }
    }
    let csv_path = args.out.join("per_year.csv");
    std::fs::write(&csv_path, &csv)
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // --- Parquet. ---
    let fields = vec![
        arrow_schema::Field::new("year", arrow_schema::DataType::Int64, false),
        arrow_schema::Field::new("feasible", arrow_schema::DataType::Boolean, false),
        arrow_schema::Field::new("requirement_gwh", arrow_schema::DataType::Float64, true),
        arrow_schema::Field::new("min_soc_gwh", arrow_schema::DataType::Float64, true),
        arrow_schema::Field::new("min_soc_at", arrow_schema::DataType::Utf8, true),
        arrow_schema::Field::new(
            "initial_condition_sensitive",
            arrow_schema::DataType::Boolean,
            true,
        ),
        arrow_schema::Field::new("infeasible_reason", arrow_schema::DataType::Utf8, true),
    ];
    let mut years = Vec::new();
    let mut feasible_flags = Vec::new();
    let mut requirements: Vec<Option<f64>> = Vec::new();
    let mut min_socs: Vec<Option<f64>> = Vec::new();
    let mut min_soc_ats: Vec<Option<String>> = Vec::new();
    let mut sensitives: Vec<Option<bool>> = Vec::new();
    let mut reasons: Vec<Option<String>> = Vec::new();
    for year in &batch {
        years.push(i64::from(year.year));
        match &year.outcome {
            grid_adequacy::YearOutcome::Feasible {
                requirement,
                min_soc,
                min_soc_at,
                initial_condition_sensitive,
            } => {
                feasible_flags.push(true);
                requirements.push(Some(requirement.as_gigawatt_hours()));
                min_socs.push(Some(min_soc.as_gigawatt_hours()));
                min_soc_ats.push(Some(min_soc_at.to_string()));
                sensitives.push(Some(*initial_condition_sensitive));
                reasons.push(None);
            }
            grid_adequacy::YearOutcome::Infeasible { reason } => {
                feasible_flags.push(false);
                requirements.push(None);
                min_socs.push(None);
                min_soc_ats.push(None);
                sensitives.push(None);
                reasons.push(Some(reason.clone()));
            }
        }
    }
    let arrays: Vec<arrow_array::ArrayRef> = vec![
        std::sync::Arc::new(arrow_array::Int64Array::from(years)),
        std::sync::Arc::new(arrow_array::BooleanArray::from(feasible_flags)),
        std::sync::Arc::new(arrow_array::Float64Array::from(requirements)),
        std::sync::Arc::new(arrow_array::Float64Array::from(min_socs)),
        std::sync::Arc::new(arrow_array::StringArray::from(min_soc_ats)),
        std::sync::Arc::new(arrow_array::BooleanArray::from(sensitives)),
        std::sync::Arc::new(arrow_array::StringArray::from(reasons)),
    ];
    let parquet_path = args.out.join("per_year.parquet");
    write_table_parquet(&parquet_path, fields, arrays, &meta, &data_files)?;

    // Console summary with the distribution's extremes.
    let mut feasible: Vec<(i32, f64)> = Vec::new();
    for year in &batch {
        match &year.outcome {
            grid_adequacy::YearOutcome::Feasible { requirement, .. } => {
                println!(
                    "  {}: {:>10.0} GWh",
                    year.year,
                    requirement.as_gigawatt_hours()
                );
                feasible.push((year.year, requirement.as_gigawatt_hours()));
            }
            grid_adequacy::YearOutcome::Infeasible { .. } => {
                println!("  {}: INFEASIBLE", year.year);
            }
        }
    }
    if let (Some(&(max_year, max_gwh)), Some(&(min_year, min_gwh))) = (
        feasible
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)),
        feasible
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)),
    ) {
        println!(
            "per-year batch complete: {} years; design year {max_year} ({max_gwh:.0} GWh), \
             easiest {min_year} ({min_gwh:.0} GWh)",
            batch.len()
        );
    }
    println!("  table {}", csv_path.display());
    println!("  table {}", parquet_path.display());
    Ok(())
}

// ---------------------------------------------------------------------
// `sweep heating-mix` — the Q5/Q11 analysis runs (D9 rules 6 and 6b,
// fixed-fleet leg).
// ---------------------------------------------------------------------

#[derive(Args)]
struct HeatingMixArgs {
    /// Heated scenario TOML file: must carry a [zones.demand.heating]
    /// TEMPLATE block (quantum, electrified_share, dhw_fraction,
    /// temperature_trace and one entry per kind); the sweep replaces
    /// only the entry shares over the simplex.
    #[arg(long)]
    scenario: PathBuf,

    /// Output directory (created if absent): heating_mix_sweep.{csv,
    /// parquet}, heating_mix_decomposition.{csv,parquet},
    /// heating_mix_gradients.png.
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which relative trace paths are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Share step of the simplex grid (0.1 → the 66-point simplex).
    /// Must divide 1 evenly.
    #[arg(long, default_value_t = 0.1)]
    step: f64,

    /// The STATED store power rating (GW), applied to BOTH endpoints
    /// (the no-heating baseline and every heated point) for the run
    /// and the solve. REQUIRED — there is no silent default: the
    /// committed RS rating (100 GW) is power-bound infeasible under
    /// electrified heat (the pinned heating.rs finding), and every
    /// storage number must travel with its rating
    /// (docs/notes/q5-heating-engine-review.md, binding record).
    #[arg(long)]
    store_power_gw: f64,

    /// Index of the store the solver sizes (scenario order).
    #[arg(long, default_value_t = 0)]
    store_index: usize,

    /// Force the serial execution path (default rayon; bit-identical
    /// either way — asserted by the acceptance suite).
    #[arg(long)]
    serial: bool,
}

fn heating_mix(args: &HeatingMixArgs) -> Result<(), String> {
    // Step → integer simplex denominator (exact lattice shares).
    if !(args.step > 0.0 && args.step <= 1.0) {
        return Err(format!("--step must be in (0, 1]; got {}", args.step));
    }
    let denominator = (1.0 / args.step).round();
    if ((1.0 / args.step) - denominator).abs() > 1e-9 || denominator < 1.0 {
        return Err(format!(
            "--step {} does not divide 1 evenly; use e.g. 0.1, 0.2, 0.25, 0.5 or 1",
            args.step
        ));
    }
    let mixes = grid_adequacy::simplex_mixes(denominator as u32).map_err(|e| e.to_string())?;

    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    let store_power = grid_core::units::Power::gigawatts(args.store_power_gw);
    let started = std::time::Instant::now();
    let context = grid_adequacy::HeatingMixContext::load(
        &scenario,
        &args.base_dir,
        store_power,
        args.store_index,
    )
    .map_err(|e| e.to_string())?;
    let execution = if args.serial {
        grid_adequacy::Execution::Serial
    } else {
        grid_adequacy::Execution::Parallel
    };
    let options = grid_adequacy::SolveOptions::default();

    let sweep = context
        .sweep(&mixes, &options, execution)
        .map_err(|e| e.to_string())?;
    let sweep_elapsed = started.elapsed().as_secs_f64();

    // Metadata (docs/06).
    let engine = env!("GRID_ENGINE_GIT_HASH");
    let scenario_sha = crate::run::sha256_file(&args.scenario)?;
    let data_files = crate::run::scenario_data_files(&scenario, &args.base_dir)?;
    let meta: Vec<(String, String)> = vec![
        ("engine_git_hash".to_owned(), engine.to_owned()),
        (
            "scenario_path".to_owned(),
            args.scenario.display().to_string(),
        ),
        ("scenario_sha256".to_owned(), scenario_sha.clone()),
        (
            "store_power_gw_stated".to_owned(),
            format!("{}", store_power.as_gigawatts()),
        ),
        ("simplex_step".to_owned(), format!("{}", args.step)),
        ("created_utc".to_owned(), crate::run::now_utc()),
    ];

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // The sweep artefacts land BEFORE the decomposition runs: if the
    // stated rating is infeasible, the decomposition (whose bisections
    // need a feasible rating) fails — but the sweep artefact with its
    // reportable infeasible_reason column is already on disk.
    let sweep_csv_path = args.out.join("heating_mix_sweep.csv");
    let sweep_parquet_path = args.out.join("heating_mix_sweep.parquet");
    write_heating_mix_sweep_csv(&sweep_csv_path, &sweep, &meta, &data_files)?;
    write_heating_mix_sweep_parquet(&sweep_parquet_path, &sweep, &meta, &data_files)?;
    let chart_path = args.out.join("heating_mix_gradients.png");
    render_heating_mix_chart(&chart_path, &sweep, engine, &scenario_sha)?;

    // The D9 rule-6(c) decomposition set, standard Stage 4 windows
    // (24 h / 14 d / 365 d — stamped on the artefact; the
    // synoptic-vs-seasonal ranking must never be quoted without them).
    let windows = grid_core::analysis::DecompositionWindows::standard();
    let attributions = context.attributions(&windows, &options).map_err(|e| {
        format!(
            "decomposition failed after the sweep artefacts were written to {}: {e}",
            args.out.display()
        )
    })?;
    let elapsed = started.elapsed().as_secs_f64();

    let decomposition_csv_path = args.out.join("heating_mix_decomposition.csv");
    let decomposition_parquet_path = args.out.join("heating_mix_decomposition.parquet");
    write_heating_mix_decomposition_csv(
        &decomposition_csv_path,
        &attributions,
        store_power,
        &windows,
        &meta,
        &data_files,
    )?;
    write_heating_mix_decomposition_parquet(
        &decomposition_parquet_path,
        &attributions,
        store_power,
        &windows,
        &meta,
        &data_files,
    )?;

    // Console summary: corners, the two gradients, decomposition.
    print_heating_mix_summary(&sweep, &attributions);
    println!(
        "heating-mix sweep complete: {} simplex points + baseline in {sweep_elapsed:.1} s, \
         + 4 decompositions = {elapsed:.1} s total ({})",
        sweep.points.len(),
        if args.serial { "serial" } else { "rayon" },
    );
    for path in [
        &sweep_csv_path,
        &sweep_parquet_path,
        &decomposition_csv_path,
        &decomposition_parquet_path,
    ] {
        println!("  table {}", path.display());
    }
    println!("  chart {}", chart_path.display());
    Ok(())
}

/// The assumption block every heating-mix artefact carries (D9 rule 6
/// caveats + the review's binding quoting duties), as CSV `#` comment
/// lines. The rating/infeasibility wording is the reviewer's
/// quote-duty, verbatim where ×1.69-class numbers appear.
fn heating_mix_assumption_lines(store_power_gw: f64) -> Vec<String> {
    vec![
        format!(
            "assumption 1 (store rating, BINDING): every storage number in this artefact is \
             at {store_power_gw} GW store power, both endpoints (stamped per row in \
             store_power_gw; the no-heating baseline is solved at the same rating). The \
             committed RS scenario rating is 100 GW, and at 100 GW the heated solve is \
             POWER-BOUND INFEASIBLE (pinned finding, grid-adequacy/tests/heating.rs: the \
             heated peak residual exceeds the rating). Any requirement or delta quoted from \
             here carries 'at {store_power_gw} GW store power, both endpoints' next to the \
             number, and the 100 GW infeasibility finding travels with the x1.69-class \
             storage headline. SolveInfeasible at a stated rating is a reportable result \
             (infeasible_reason column), never a silently bumped rating"
        ),
        "assumption 2 (lower bound): no within-day behavioural heating profile (D9 rule 3) \
         — behavioural morning/evening peaking lands on cold, solar-free hours, so the \
         heating peak AND the portfolio deltas (the ASHP->GSHP and ASHP->district \
         gradients) are UNDERSTATED: every measured network value of geothermal here is a \
         lower bound"
            .to_owned(),
        "assumption 3: 2024 non-heat demand tiled over 1985-2024 (calendar-date rule); the \
         RS-style ~570 TWh/yr non-heat demand is the observed 2024 profile, not an \
         electrified-future profile"
            .to_owned(),
        "assumption 4: climate-stationary heat intensity — one pinned k and DHW fraction \
         across all 40 weather years (fixed building stock, no retrofit/stock/warming \
         trend): the runs answer 'the stated stock in year Y's weather'. The cold-year \
         covariance (more heat AND worse COP in the same years) is captured by \
         construction and is a finding, not a caveat"
            .to_owned(),
        "assumption 5 (generation relieved, D9 rule 6b): FIXED-FLEET leg only. Per-tech \
         columns are POTENTIAL (pre-curtailment) output — mix-invariant on this \
         all-must-take fleet; what the mix moves is curtailment (pooled, Stage 1 \
         convention, no per-source attribution), store cycling and unserved. Dispatch \
         metrics are at the scenario's committed store energy (dispatch_store_energy_gwh \
         column) with the stated rating: fixed fleet AND fixed store on both endpoints of \
         every paired delta. The capacity-relieved leg (equal-reliability avoided build) \
         REQUIRES the 1-D capacity solver (ELCC runner, wave-2 paper-4 enabler) — a named \
         dependency, NOT computed here"
            .to_owned(),
        "assumption 6 (standing programme): ERA5-derived CF traces calibrated \
         one-factor-per-tech to observed 2024 energies (frozen-2024 calibration across all \
         weather years); COP parameters from the drift-guarded \
         data/reference/heating-cop.toml (When2Heat curves, RHPP-derated; sha256 in the \
         data_file lines)"
            .to_owned(),
        "data: demand from NESO data (NESO Open Data Licence, attribution required); CF \
         and t2m traces derived from ERA5 (Copernicus Climate Change Service / ECMWF, \
         CC-BY 4.0)"
            .to_owned(),
        "note: every energy column is a TOTAL OVER THE SCENARIO HORIZON (40 weather years \
         on the RS scenario), not an annual figure; divide by the horizon's year count \
         for annual means"
            .to_owned(),
    ]
}

/// Numeric cell: full-precision f64, or empty for None (CSV
/// convention shared with `per_year`).
fn cell(value: Option<f64>) -> String {
    value.map_or(String::new(), |v| v.to_string())
}

/// One sweep row flattened for the artefact writers: label, shares
/// (None for the baseline row) and metrics.
struct HeatingMixRow<'a> {
    point: &'a str,
    shares: Option<(f64, f64, f64)>,
    metrics: &'a grid_adequacy::MixMetrics,
}

fn heating_mix_rows(sweep: &grid_adequacy::HeatingMixSweep) -> Vec<HeatingMixRow<'_>> {
    let mut rows = vec![HeatingMixRow {
        point: "baseline",
        shares: None,
        metrics: &sweep.baseline,
    }];
    for point in &sweep.points {
        rows.push(HeatingMixRow {
            point: "mix",
            shares: Some((
                point.shares.ashp_share().value(),
                point.shares.gshp_share().value(),
                point.shares.district_share().value(),
            )),
            metrics: &point.metrics,
        });
    }
    rows
}

/// Requirement of an outcome (None when infeasible) plus the guard
/// fields, for the artefact writers.
fn outcome_fields(
    outcome: &grid_adequacy::MixOutcome,
) -> (Option<f64>, Option<bool>, Option<f64>, Option<String>) {
    match outcome {
        grid_adequacy::MixOutcome::Feasible {
            requirement,
            initial_condition_sensitive,
            burn_in_requirement,
        } => (
            Some(requirement.as_gigawatt_hours()),
            Some(*initial_condition_sensitive),
            burn_in_requirement.map(|e| e.as_gigawatt_hours()),
            None,
        ),
        grid_adequacy::MixOutcome::Infeasible { reason } => {
            (None, None, None, Some(reason.clone()))
        }
    }
}

fn baseline_requirement_gwh(sweep: &grid_adequacy::HeatingMixSweep) -> Option<f64> {
    outcome_fields(&sweep.baseline.outcome).0
}

fn write_heating_mix_sweep_csv(
    path: &PathBuf,
    sweep: &grid_adequacy::HeatingMixSweep,
    meta: &[(String, String)],
    data_files: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let mut csv =
        String::from("# grid-sim heating-mix sweep (Q5/Q11, D9 rules 6/6b; docs/06 header)\n");
    for (key, value) in meta {
        csv.push_str(&format!("# {key} = {value}\n"));
    }
    for (data_path, hash) in data_files {
        csv.push_str(&format!("# data_file {data_path} sha256={hash}\n"));
    }
    for line in heating_mix_assumption_lines(sweep.store_power.as_gigawatts()) {
        csv.push_str(&format!("# {line}\n"));
    }
    csv.push_str(
        "point,ashp_share,gshp_share,district_share,store_power_gw,dispatch_store_energy_gwh,\
         requirement_gwh,requirement_delta_gwh,initial_condition_sensitive,\
         burn_in_requirement_gwh,infeasible_reason,peak_residual_gw,peak_residual_delta_gw,\
         heating_electrical_twh,delivered_heat_twh",
    );
    for (tech, _) in &sweep.baseline.tech_potential {
        csv.push_str(&format!(",{tech}_potential_twh"));
    }
    csv.push_str(
        ",curtailment_twh,curtailment_delta_twh,store_discharge_twh,store_charge_twh,\
         unserved_gwh\n",
    );

    let baseline_requirement = baseline_requirement_gwh(sweep);
    let baseline_peak = sweep.baseline.peak_residual.as_gigawatts();
    let baseline_curtailment = twh(sweep.baseline.curtailment);
    for row in heating_mix_rows(sweep) {
        let m = row.metrics;
        let (requirement, sensitive, burn_in, reason) = outcome_fields(&m.outcome);
        let is_baseline = row.shares.is_none();
        let requirement_delta = match (is_baseline, requirement, baseline_requirement) {
            (false, Some(r), Some(b)) => Some(r - b),
            _ => None,
        };
        let (a, g, d) = row
            .shares
            .map_or((None, None, None), |(a, g, d)| (Some(a), Some(g), Some(d)));
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},\"{}\",{},{},{},{}",
            row.point,
            cell(a),
            cell(g),
            cell(d),
            sweep.store_power.as_gigawatts(),
            sweep.dispatch_store_energy.as_gigawatt_hours(),
            cell(requirement),
            cell(requirement_delta),
            sensitive.map_or(String::new(), |s| s.to_string()),
            cell(burn_in),
            reason.unwrap_or_default().replace('"', "'"),
            m.peak_residual.as_gigawatts(),
            if is_baseline {
                String::new()
            } else {
                (m.peak_residual.as_gigawatts() - baseline_peak).to_string()
            },
            twh(m.heating_electrical),
            twh(m.delivered_heat),
        ));
        for (_, energy) in &m.tech_potential {
            csv.push_str(&format!(",{}", twh(*energy)));
        }
        csv.push_str(&format!(
            ",{},{},{},{},{}\n",
            twh(m.curtailment),
            if is_baseline {
                String::new()
            } else {
                (twh(m.curtailment) - baseline_curtailment).to_string()
            },
            twh(m.store_discharge),
            twh(m.store_charge),
            m.unserved.as_gigawatt_hours(),
        ));
    }
    std::fs::write(path, &csv).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn write_heating_mix_sweep_parquet(
    path: &PathBuf,
    sweep: &grid_adequacy::HeatingMixSweep,
    meta: &[(String, String)],
    data_files: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    use arrow_array::{ArrayRef, BooleanArray, Float64Array, StringArray};
    use arrow_schema::{DataType, Field};

    let rows = heating_mix_rows(sweep);
    let baseline_requirement = baseline_requirement_gwh(sweep);
    let baseline_peak = sweep.baseline.peak_residual.as_gigawatts();
    let baseline_curtailment = twh(sweep.baseline.curtailment);

    let mut fields: Vec<Field> = Vec::new();
    let mut arrays: Vec<ArrayRef> = Vec::new();
    let utf8 = |name: &str,
                values: Vec<Option<String>>,
                fields: &mut Vec<Field>,
                arrays: &mut Vec<ArrayRef>| {
        fields.push(Field::new(name, DataType::Utf8, true));
        arrays.push(std::sync::Arc::new(StringArray::from(values)));
    };
    let f64_opt = |name: &str,
                   values: Vec<Option<f64>>,
                   fields: &mut Vec<Field>,
                   arrays: &mut Vec<ArrayRef>| {
        fields.push(Field::new(name, DataType::Float64, true));
        arrays.push(std::sync::Arc::new(Float64Array::from(values)));
    };

    utf8(
        "point",
        rows.iter().map(|r| Some(r.point.to_owned())).collect(),
        &mut fields,
        &mut arrays,
    );
    for (name, pick) in [
        ("ashp_share", 0usize),
        ("gshp_share", 1),
        ("district_share", 2),
    ] {
        f64_opt(
            name,
            rows.iter()
                .map(|r| r.shares.map(|s| [s.0, s.1, s.2][pick]))
                .collect(),
            &mut fields,
            &mut arrays,
        );
    }
    f64_opt(
        "store_power_gw",
        rows.iter()
            .map(|_| Some(sweep.store_power.as_gigawatts()))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "dispatch_store_energy_gwh",
        rows.iter()
            .map(|_| Some(sweep.dispatch_store_energy.as_gigawatt_hours()))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "requirement_gwh",
        rows.iter()
            .map(|r| outcome_fields(&r.metrics.outcome).0)
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "requirement_delta_gwh",
        rows.iter()
            .map(|r| {
                match (
                    r.shares,
                    outcome_fields(&r.metrics.outcome).0,
                    baseline_requirement,
                ) {
                    (Some(_), Some(v), Some(b)) => Some(v - b),
                    _ => None,
                }
            })
            .collect(),
        &mut fields,
        &mut arrays,
    );
    fields.push(Field::new(
        "initial_condition_sensitive",
        DataType::Boolean,
        true,
    ));
    arrays.push(std::sync::Arc::new(BooleanArray::from(
        rows.iter()
            .map(|r| outcome_fields(&r.metrics.outcome).1)
            .collect::<Vec<_>>(),
    )));
    f64_opt(
        "burn_in_requirement_gwh",
        rows.iter()
            .map(|r| outcome_fields(&r.metrics.outcome).2)
            .collect(),
        &mut fields,
        &mut arrays,
    );
    utf8(
        "infeasible_reason",
        rows.iter()
            .map(|r| outcome_fields(&r.metrics.outcome).3)
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "peak_residual_gw",
        rows.iter()
            .map(|r| Some(r.metrics.peak_residual.as_gigawatts()))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "peak_residual_delta_gw",
        rows.iter()
            .map(|r| {
                r.shares
                    .map(|_| r.metrics.peak_residual.as_gigawatts() - baseline_peak)
            })
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "heating_electrical_twh",
        rows.iter()
            .map(|r| Some(twh(r.metrics.heating_electrical)))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "delivered_heat_twh",
        rows.iter()
            .map(|r| Some(twh(r.metrics.delivered_heat)))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    for (index, (tech, _)) in sweep.baseline.tech_potential.iter().enumerate() {
        f64_opt(
            &format!("{tech}_potential_twh"),
            rows.iter()
                .map(|r| Some(twh(r.metrics.tech_potential[index].1)))
                .collect(),
            &mut fields,
            &mut arrays,
        );
    }
    f64_opt(
        "curtailment_twh",
        rows.iter()
            .map(|r| Some(twh(r.metrics.curtailment)))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "curtailment_delta_twh",
        rows.iter()
            .map(|r| {
                r.shares
                    .map(|_| twh(r.metrics.curtailment) - baseline_curtailment)
            })
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "store_discharge_twh",
        rows.iter()
            .map(|r| Some(twh(r.metrics.store_discharge)))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "store_charge_twh",
        rows.iter()
            .map(|r| Some(twh(r.metrics.store_charge)))
            .collect(),
        &mut fields,
        &mut arrays,
    );
    f64_opt(
        "unserved_gwh",
        rows.iter()
            .map(|r| Some(r.metrics.unserved.as_gigawatt_hours()))
            .collect(),
        &mut fields,
        &mut arrays,
    );

    let mut meta = meta.to_vec();
    // Numbered keys (review condition 1): duplicate `assumption` keys
    // collapse to a single visible line in dict-based parquet readers
    // (pyarrow's default view), hiding the quote duties.
    for (index, line) in heating_mix_assumption_lines(sweep.store_power.as_gigawatts())
        .into_iter()
        .enumerate()
    {
        meta.push((format!("assumption_{}", index + 1), line));
    }
    write_table_parquet(path, fields, arrays, &meta, data_files)
}

fn write_heating_mix_decomposition_csv(
    path: &PathBuf,
    attributions: &[grid_adequacy::NamedAttribution],
    store_power: grid_core::units::Power,
    windows: &grid_core::analysis::DecompositionWindows,
    meta: &[(String, String)],
    data_files: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let mut csv = String::from(
        "# grid-sim heating-mix decomposition (D9 rule 6(c): the timescale decomposition \
         of the added storage requirement; docs/06 header)\n",
    );
    for (key, value) in meta {
        csv.push_str(&format!("# {key} = {value}\n"));
    }
    for (data_path, hash) in data_files {
        csv.push_str(&format!("# data_file {data_path} sha256={hash}\n"));
    }
    for line in heating_mix_assumption_lines(store_power.as_gigawatts()) {
        csv.push_str(&format!("# {line}\n"));
    }
    csv.push_str(&format!(
        "# window convention (Stage 4 publication rule — the synoptic-vs-seasonal ranking \
         is window-sensitive and must never be quoted without it): diurnal {} h / synoptic \
         {} h / seasonal {} h; bands attributed by telescoping bisection requirements \
         across the smoothing cascade (grid_adequacy::attribution)\n",
        windows.diurnal.as_hours(),
        windows.synoptic.as_hours(),
        windows.seasonal.as_hours(),
    ));
    csv.push_str(
        "point,store_power_gw,window_diurnal_h,window_synoptic_h,window_seasonal_h,\
         total_gwh,diurnal_gwh,synoptic_gwh,seasonal_gwh,inter_annual_gwh,delta_total_gwh,\
         delta_diurnal_gwh,delta_synoptic_gwh,delta_seasonal_gwh,delta_inter_annual_gwh\n",
    );
    let baseline = &attributions[0].attribution;
    for named in attributions {
        let a = &named.attribution;
        let is_baseline = named.label == "baseline";
        let delta = |value: f64, base: f64| -> String {
            if is_baseline {
                String::new()
            } else {
                (value - base).to_string()
            }
        };
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            named.label,
            store_power.as_gigawatts(),
            windows.diurnal.as_hours(),
            windows.synoptic.as_hours(),
            windows.seasonal.as_hours(),
            a.total.as_gigawatt_hours(),
            a.bands[0].requirement.as_gigawatt_hours(),
            a.bands[1].requirement.as_gigawatt_hours(),
            a.bands[2].requirement.as_gigawatt_hours(),
            a.bands[3].requirement.as_gigawatt_hours(),
            delta(
                a.total.as_gigawatt_hours(),
                baseline.total.as_gigawatt_hours()
            ),
            delta(
                a.bands[0].requirement.as_gigawatt_hours(),
                baseline.bands[0].requirement.as_gigawatt_hours()
            ),
            delta(
                a.bands[1].requirement.as_gigawatt_hours(),
                baseline.bands[1].requirement.as_gigawatt_hours()
            ),
            delta(
                a.bands[2].requirement.as_gigawatt_hours(),
                baseline.bands[2].requirement.as_gigawatt_hours()
            ),
            delta(
                a.bands[3].requirement.as_gigawatt_hours(),
                baseline.bands[3].requirement.as_gigawatt_hours()
            ),
        ));
    }
    std::fs::write(path, &csv).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn write_heating_mix_decomposition_parquet(
    path: &PathBuf,
    attributions: &[grid_adequacy::NamedAttribution],
    store_power: grid_core::units::Power,
    windows: &grid_core::analysis::DecompositionWindows,
    meta: &[(String, String)],
    data_files: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    use arrow_array::{ArrayRef, Float64Array, StringArray};
    use arrow_schema::{DataType, Field};

    let baseline = &attributions[0].attribution;
    let mut fields: Vec<Field> = Vec::new();
    let mut arrays: Vec<ArrayRef> = Vec::new();

    fields.push(Field::new("point", DataType::Utf8, false));
    arrays.push(std::sync::Arc::new(StringArray::from(
        attributions.iter().map(|n| n.label).collect::<Vec<_>>(),
    )));
    let column = |name: &str,
                  values: Vec<Option<f64>>,
                  fields: &mut Vec<Field>,
                  arrays: &mut Vec<ArrayRef>| {
        fields.push(Field::new(name, DataType::Float64, true));
        arrays.push(std::sync::Arc::new(Float64Array::from(values)));
    };
    let all =
        |value: f64| -> Vec<Option<f64>> { attributions.iter().map(|_| Some(value)).collect() };
    column(
        "store_power_gw",
        all(store_power.as_gigawatts()),
        &mut fields,
        &mut arrays,
    );
    column(
        "window_diurnal_h",
        all(windows.diurnal.as_hours()),
        &mut fields,
        &mut arrays,
    );
    column(
        "window_synoptic_h",
        all(windows.synoptic.as_hours()),
        &mut fields,
        &mut arrays,
    );
    column(
        "window_seasonal_h",
        all(windows.seasonal.as_hours()),
        &mut fields,
        &mut arrays,
    );
    let band = |index: Option<usize>| -> Vec<Option<f64>> {
        attributions
            .iter()
            .map(|n| {
                Some(match index {
                    None => n.attribution.total.as_gigawatt_hours(),
                    Some(i) => n.attribution.bands[i].requirement.as_gigawatt_hours(),
                })
            })
            .collect()
    };
    column("total_gwh", band(None), &mut fields, &mut arrays);
    column("diurnal_gwh", band(Some(0)), &mut fields, &mut arrays);
    column("synoptic_gwh", band(Some(1)), &mut fields, &mut arrays);
    column("seasonal_gwh", band(Some(2)), &mut fields, &mut arrays);
    column("inter_annual_gwh", band(Some(3)), &mut fields, &mut arrays);
    let band_delta = |index: Option<usize>| -> Vec<Option<f64>> {
        attributions
            .iter()
            .map(|n| {
                if n.label == "baseline" {
                    return None;
                }
                Some(match index {
                    None => {
                        n.attribution.total.as_gigawatt_hours() - baseline.total.as_gigawatt_hours()
                    }
                    Some(i) => {
                        n.attribution.bands[i].requirement.as_gigawatt_hours()
                            - baseline.bands[i].requirement.as_gigawatt_hours()
                    }
                })
            })
            .collect()
    };
    column(
        "delta_total_gwh",
        band_delta(None),
        &mut fields,
        &mut arrays,
    );
    column(
        "delta_diurnal_gwh",
        band_delta(Some(0)),
        &mut fields,
        &mut arrays,
    );
    column(
        "delta_synoptic_gwh",
        band_delta(Some(1)),
        &mut fields,
        &mut arrays,
    );
    column(
        "delta_seasonal_gwh",
        band_delta(Some(2)),
        &mut fields,
        &mut arrays,
    );
    column(
        "delta_inter_annual_gwh",
        band_delta(Some(3)),
        &mut fields,
        &mut arrays,
    );

    let mut meta = meta.to_vec();
    meta.push((
        "window_convention".to_owned(),
        format!(
            "diurnal {} h / synoptic {} h / seasonal {} h — never quote the \
             synoptic-vs-seasonal ranking without the window (Stage 4 publication rule)",
            windows.diurnal.as_hours(),
            windows.synoptic.as_hours(),
            windows.seasonal.as_hours()
        ),
    ));
    // Numbered keys — same rationale and numbering as the sweep writer.
    for (index, line) in heating_mix_assumption_lines(store_power.as_gigawatts())
        .into_iter()
        .enumerate()
    {
        meta.push((format!("assumption_{}", index + 1), line));
    }
    write_table_parquet(path, fields, arrays, &meta, data_files)
}

/// The two D9 rule-6 gradient edges of a sweep: ASHP→GSHP
/// (district = 0) and ASHP→district (GSHP = 0), each ordered by the
/// share shifted away from ASHP.
fn heating_mix_edges(
    sweep: &grid_adequacy::HeatingMixSweep,
) -> (
    Vec<&grid_adequacy::HeatingMixPoint>,
    Vec<&grid_adequacy::HeatingMixPoint>,
) {
    let mut to_gshp: Vec<_> = sweep
        .points
        .iter()
        .filter(|p| p.shares.district_share().value() == 0.0)
        .collect();
    to_gshp.sort_by(|a, b| {
        a.shares
            .gshp_share()
            .value()
            .partial_cmp(&b.shares.gshp_share().value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut to_district: Vec<_> = sweep
        .points
        .iter()
        .filter(|p| p.shares.gshp_share().value() == 0.0)
        .collect();
    to_district.sort_by(|a, b| {
        a.shares
            .district_share()
            .value()
            .partial_cmp(&b.shares.district_share().value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    (to_gshp, to_district)
}

/// Console summary: the corner table, the two gradients (average per
/// 10 % of the electrified quantum shifted away from ASHP), and the
/// decomposition table.
fn print_heating_mix_summary(
    sweep: &grid_adequacy::HeatingMixSweep,
    attributions: &[grid_adequacy::NamedAttribution],
) {
    let requirement = |m: &grid_adequacy::MixMetrics| outcome_fields(&m.outcome).0;
    let describe = |label: &str, m: &grid_adequacy::MixMetrics| {
        println!(
            "  {label:<26} peak residual {:>8.3} GW; requirement {} GWh at {} GW; \
             curtailment {:>9.2} TWh; heating electrical {:>8.2} TWh",
            m.peak_residual.as_gigawatts(),
            requirement(m).map_or_else(|| "INFEASIBLE".to_owned(), |r| format!("{r:>8.0}")),
            sweep.store_power.as_gigawatts(),
            twh(m.curtailment),
            twh(m.heating_electrical),
        );
    };
    println!("corner points (store rating stated per line — the quoting duty):");
    describe("no-heating baseline", &sweep.baseline);
    for point in &sweep.points {
        let s = point.shares;
        let is_corner = [s.ashp_share(), s.gshp_share(), s.district_share()]
            .iter()
            .any(|share| share.value() == 1.0);
        let is_d9 = s.ashp_share().value() == 0.70
            && s.gshp_share().value() == 0.20
            && s.district_share().value() == 0.10;
        if is_corner || is_d9 {
            describe(&s.label(), &point.metrics);
        }
    }

    let (to_gshp, to_district) = heating_mix_edges(sweep);
    let gradient = |edge: &[&grid_adequacy::HeatingMixPoint],
                    name: &str,
                    shifted: fn(&grid_adequacy::MixShares) -> f64| {
        let (Some(first), Some(last)) = (edge.first(), edge.last()) else {
            return;
        };
        let span = shifted(&last.shares) - shifted(&first.shares);
        if span <= 0.0 {
            return;
        }
        let per_tenth = |a: f64, b: f64| (b - a) / span * 0.1;
        let peak = per_tenth(
            first.metrics.peak_residual.as_gigawatts(),
            last.metrics.peak_residual.as_gigawatts(),
        );
        let req = match (requirement(&first.metrics), requirement(&last.metrics)) {
            (Some(a), Some(b)) => format!("{:+.0} GWh", per_tenth(a, b)),
            _ => "n/a (infeasible endpoint)".to_owned(),
        };
        let curtailment = per_tenth(
            twh(first.metrics.curtailment),
            twh(last.metrics.curtailment),
        );
        println!(
            "  {name}: {peak:+.3} GW peak, {req} storage, {curtailment:+.2} TWh curtailment \
             per 10 % of the electrified quantum shifted (edge average; per-point rows in \
             the CSV; LOWER BOUNDS — no behavioural profile)"
        );
    };
    println!("gradients (the D9 rule-6 network value of geothermal):");
    gradient(&to_gshp, "ASHP -> GSHP    ", |s| s.gshp_share().value());
    gradient(&to_district, "ASHP -> district", |s| {
        s.district_share().value()
    });

    println!(
        "decomposition of the added requirement (GWh at {} GW; windows 24 h / 14 d / 365 d \
         — never quote the synoptic-vs-seasonal ranking without the window):",
        sweep.store_power.as_gigawatts()
    );
    for named in attributions {
        let a = &named.attribution;
        println!(
            "  {:<13} total {:>6.0} = diurnal {:>5.0} + synoptic {:>6.0} + seasonal {:>6.0} \
             + inter-annual {:>3.0}",
            named.label,
            a.total.as_gigawatt_hours(),
            a.bands[0].requirement.as_gigawatt_hours(),
            a.bands[1].requirement.as_gigawatt_hours(),
            a.bands[2].requirement.as_gigawatt_hours(),
            a.bands[3].requirement.as_gigawatt_hours(),
        );
    }
}

/// The two-panel gradient chart: peak residual demand (left) and the
/// storage requirement (right) along the ASHP→GSHP and ASHP→district
/// edges. The full simplex lives in the CSV/Parquet — the chart is the
/// readable presentation of the two rule-6 gradients.
fn render_heating_mix_chart(
    path: &PathBuf,
    sweep: &grid_adequacy::HeatingMixSweep,
    engine: &str,
    scenario_sha: &str,
) -> Result<(), String> {
    let (to_gshp, to_district) = heating_mix_edges(sweep);
    let requirement_twh =
        |m: &grid_adequacy::MixMetrics| outcome_fields(&m.outcome).0.map(|gwh| gwh / 1000.0);

    let root = BitMapBackend::new(path, (1600, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(810);
    let (left, right) = chart_area.split_horizontally(800);

    let gshp_colour = RGBColor(60, 120, 216);
    let district_colour = RGBColor(40, 140, 70);
    let baseline_colour = RGBColor(120, 120, 120);

    let draw_panel = |area: &DrawingArea<BitMapBackend, plotters::coord::Shift>,
                      title: &str,
                      y_desc: &str,
                      values: &dyn Fn(&grid_adequacy::MixMetrics) -> Option<f64>,
                      baseline: Option<f64>|
     -> Result<(), String> {
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for point in to_gshp.iter().chain(&to_district) {
            if let Some(v) = values(&point.metrics) {
                y_min = y_min.min(v);
                y_max = y_max.max(v);
            }
        }
        if let Some(b) = baseline {
            y_min = y_min.min(b);
            y_max = y_max.max(b);
        }
        if !(y_min.is_finite() && y_max.is_finite()) {
            return Ok(()); // every point infeasible: nothing to draw
        }
        let pad = (0.08 * (y_max - y_min)).max(1e-6);
        let mut chart = ChartBuilder::on(area)
            .caption(title, ("sans-serif", 24))
            .margin(20)
            .x_label_area_size(50)
            .y_label_area_size(80)
            .build_cartesian_2d(0.0..1.0f64, (y_min - pad)..(y_max + pad))
            .map_err(|e| e.to_string())?;
        chart
            .configure_mesh()
            .x_desc("share of the electrified quantum shifted away from ASHP")
            .y_desc(y_desc)
            .axis_desc_style(("sans-serif", 17))
            .label_style(("sans-serif", 15))
            .draw()
            .map_err(|e| e.to_string())?;

        let mut series = |points: &[&grid_adequacy::HeatingMixPoint],
                          shifted: fn(&grid_adequacy::MixShares) -> f64,
                          colour: RGBColor,
                          label: &str|
         -> Result<(), String> {
            let line: Vec<(f64, f64)> = points
                .iter()
                .filter_map(|p| values(&p.metrics).map(|v| (shifted(&p.shares), v)))
                .collect();
            chart
                .draw_series(LineSeries::new(
                    line.iter().copied(),
                    colour.stroke_width(3),
                ))
                .map_err(|e| e.to_string())?
                .label(label)
                .legend(move |(x, y)| {
                    PathElement::new(vec![(x, y), (x + 20, y)], colour.stroke_width(3))
                });
            chart
                .draw_series(
                    line.iter()
                        .map(|&(x, y)| Circle::new((x, y), 4, colour.filled())),
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        };
        series(
            &to_gshp,
            |s| s.gshp_share().value(),
            gshp_colour,
            "ASHP -> GSHP",
        )?;
        series(
            &to_district,
            |s| s.district_share().value(),
            district_colour,
            "ASHP -> district geothermal",
        )?;
        if let Some(b) = baseline {
            chart
                .draw_series(LineSeries::new(
                    [(0.0, b), (1.0, b)],
                    baseline_colour.stroke_width(2),
                ))
                .map_err(|e| e.to_string())?
                .label("no-heating baseline")
                .legend(move |(x, y)| {
                    PathElement::new(vec![(x, y), (x + 20, y)], baseline_colour.stroke_width(2))
                });
        }
        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .border_style(BLACK)
            .background_style(WHITE.mix(0.9))
            .label_font(("sans-serif", 15))
            .draw()
            .map_err(|e| e.to_string())?;
        Ok(())
    };

    draw_panel(
        &left,
        "Peak residual demand vs heating mix (lower bounds: no behavioural profile)",
        "peak residual demand, GW",
        &|m| Some(m.peak_residual.as_gigawatts()),
        Some(sweep.baseline.peak_residual.as_gigawatts()),
    )?;
    draw_panel(
        &right,
        &format!(
            "40-year storage requirement vs heating mix (at {} GW store power, both endpoints)",
            sweep.store_power.as_gigawatts()
        ),
        "storage requirement, TWh (store-side)",
        &requirement_twh,
        baseline_requirement_gwh(sweep).map(|gwh| gwh / 1000.0),
    )?;

    let caption_top = format!(
        "grid-sim | engine {engine} | scenario sha256 {} | at {} GW store power, both \
         endpoints; the committed 100 GW rating is power-bound INFEASIBLE under \
         electrified heat (pinned finding)",
        short(scenario_sha),
        sweep.store_power.as_gigawatts(),
    );
    let caption_bottom = "deltas are LOWER BOUNDS (no behavioural heating profile) | 2024 \
                          non-heat tiling; climate-stationary k | full simplex in \
                          heating_mix_sweep.csv";
    for (line, y) in [(caption_top.as_str(), 20), (caption_bottom, 45)] {
        footer
            .draw(&Text::new(
                line,
                (20, y),
                ("sans-serif", 14).into_font().color(&BLACK.mix(0.7)),
            ))
            .map_err(|e| e.to_string())?;
    }
    root.present().map_err(|e| e.to_string())?;
    Ok(())
}

fn wind_capacity(args: &WindCapacityArgs) -> Result<(), String> {
    if !(args.step_gw > 0.0 && args.max_gw >= args.min_gw && args.min_gw > 0.0) {
        return Err(format!(
            "invalid sweep range: min {} GW, max {} GW, step {} GW",
            args.min_gw, args.max_gw, args.step_gw
        ));
    }

    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    let pricing_spec = scenario.pricing.as_ref().ok_or(
        "the scenario has no [pricing] block; the Module 1 sweep needs the Stage 2 \
         pricing inputs to flag the price-setting technology",
    )?;
    // Traces are loaded once against the reference scenario; only fleet
    // capacities change per point, and the dispatch engine reads those
    // from the (scaled) scenario.
    let inputs = load_run_inputs(&scenario, &args.base_dir).map_err(|e| e.to_string())?;
    let pricing_inputs =
        load_pricing_inputs(&scenario, pricing_spec, &args.base_dir).map_err(|e| e.to_string())?;

    let is_wind = |tech: &str| matches!(tech, "offshore_wind" | "onshore_wind");
    let reference_wind_gw: f64 = scenario.zones[0]
        .fleet
        .iter()
        .filter(|e| is_wind(e.technology.as_str()))
        .map(|e| e.capacity_gw.as_gigawatts())
        .sum();
    if reference_wind_gw <= 0.0 {
        return Err("the scenario has no onshore/offshore wind capacity to scale".to_owned());
    }

    // Export capability for the export-in-surplus bracket convention
    // (Package B): an explicit CLI value wins; otherwise the scenario's
    // own links (Σ capacity_gw × availability); otherwise a hard error —
    // no silent default.
    let (export_capacity, export_capacity_source) = match args.export_capacity_gw {
        Some(gw) => {
            if !(gw > 0.0 && gw.is_finite()) {
                return Err(format!(
                    "--export-capacity-gw must be a positive, finite GW value (got {gw})"
                ));
            }
            (
                grid_core::units::Power::gigawatts(gw),
                "cli --export-capacity-gw".to_owned(),
            )
        }
        None => {
            match grid_adequacy::link_export_capability(&scenario).map_err(|e| e.to_string())? {
                Some(capability) => (
                    capability,
                    "scenario links (sum of capacity_gw x availability)".to_owned(),
                ),
                None => {
                    return Err(
                        "the scenario declares no interconnector links touching the zone, so the \
                     export-in-surplus bracket convention has no export capability; pass \
                     --export-capacity-gw explicitly (there is no silent default)"
                            .to_owned(),
                    );
                }
            }
        }
    };
    let priced_techs: Vec<String> = pricing_inputs
        .srmc
        .keys()
        .map(|t| t.as_str().to_owned())
        .collect();
    let priced_techs: Vec<&str> = priced_techs.iter().map(String::as_str).collect();

    // The capacity points: min, min+step, …, max (max included when the
    // range is a whole number of steps — 10→60 by 5 gives 11 points).
    let mut points = Vec::new();
    let steps = ((args.max_gw - args.min_gw) / args.step_gw).round() as usize;
    for i in 0..=steps {
        let target = args.min_gw + i as f64 * args.step_gw;
        if target <= args.max_gw + 1e-9 {
            points.push(target);
        }
    }

    let mut results: Vec<SweepPoint> = Vec::with_capacity(points.len());
    for &target_gw in &points {
        let factor = target_gw / reference_wind_gw;
        let mut scaled = scenario.clone();
        for entry in &mut scaled.zones[0].fleet {
            if is_wind(entry.technology.as_str()) {
                entry.capacity_gw = entry.capacity_gw * factor;
            }
        }
        let result = grid_adequacy::run(&scaled, &inputs).map_err(|e| e.to_string())?;
        let priced = price_run(&result, &pricing_inputs).map_err(|e| e.to_string())?;

        let renewable_potential: Energy = result
            .renewables
            .iter()
            .map(|s| RunResult::total_energy(&s.power))
            .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e);
        let curtailment = result.total_curtailment();
        let gas = result
            .thermal_energy("ccgt")
            .unwrap_or(Energy::gigawatt_hours(0.0))
            + result
                .thermal_energy("ocgt")
                .unwrap_or(Energy::gigawatt_hours(0.0));

        let (wind_capture_ratio, wind_capture_ratio_delivered) =
            wind_capture_both_bases(&result, &priced.smp)?;

        // The Package B import-convention bracket: re-dispatch and
        // re-price the SAME swept point under the zero-in-surplus and
        // export-in-surplus transformations of the exogenous import
        // trace (grid_adequacy::import_convention prose). The frozen
        // numbers above come from the untransformed inputs — the
        // pre-Package-B path, untouched.
        let bracket = |convention: &grid_adequacy::ImportConvention|
         -> Result<(f64, Option<f64>, Option<f64>), String> {
            let variant_inputs =
                grid_adequacy::apply_import_convention(&scaled, &inputs, convention)
                    .map_err(|e| e.to_string())?;
            let variant = grid_adequacy::run(&scaled, &variant_inputs).map_err(|e| e.to_string())?;
            let variant_priced = price_run(&variant, &pricing_inputs).map_err(|e| e.to_string())?;
            let (potential, delivered) = wind_capture_both_bases(&variant, &variant_priced.smp)?;
            Ok((twh(variant.total_curtailment()), potential, delivered))
        };
        let (
            curtailment_twh_imports_zero,
            wind_capture_ratio_imports_zero,
            wind_capture_ratio_delivered_imports_zero,
        ) = bracket(&grid_adequacy::ImportConvention::ZeroInSurplus)?;
        let (
            curtailment_twh_imports_export,
            wind_capture_ratio_imports_export,
            wind_capture_ratio_delivered_imports_export,
        ) = bracket(&grid_adequacy::ImportConvention::ExportInSurplus { export_capacity })?;

        results.push(SweepPoint {
            wind_capacity_gw: target_gw,
            pct_gas_price_setting: 100.0 * price_setting_share(&priced.setter, &priced_techs),
            curtailment_twh: twh(curtailment),
            curtailment_pct_of_renewable_potential: 100.0 * curtailment.as_gigawatt_hours()
                / renewable_potential.as_gigawatt_hours(),
            gas_twh: twh(gas),
            mean_smp_gbp_per_mwh: priced.smp_time_weighted_mean.as_pounds_per_megawatt_hour(),
            wind_capture_ratio,
            wind_capture_ratio_delivered,
            curtailment_twh_imports_zero,
            wind_capture_ratio_imports_zero,
            wind_capture_ratio_delivered_imports_zero,
            curtailment_twh_imports_export,
            wind_capture_ratio_imports_export,
            wind_capture_ratio_delivered_imports_export,
        });
        println!(
            "  {target_gw:5.1} GW wind: gas price-setting {:6.2} %, curtailment {:7.3} TWh \
             ({:5.2} % of renewable potential), gas {:6.2} TWh",
            results.last().unwrap().pct_gas_price_setting,
            results.last().unwrap().curtailment_twh,
            results
                .last()
                .unwrap()
                .curtailment_pct_of_renewable_potential,
            results.last().unwrap().gas_twh,
        );
    }

    // Metadata (docs/06): engine + input hashes.
    let sha256_file = |path: &PathBuf| -> Result<String, String> {
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot hash {}: {e}", path.display()))?;
        Ok(format!("{:x}", Sha256::digest(&bytes)))
    };
    let engine = env!("GRID_ENGINE_GIT_HASH");
    let scenario_sha = sha256_file(&args.scenario)?;

    // Per-data-file hashes over everything the sweep read (docs/06
    // metadata parity with the run outputs).
    let data_files = crate::run::scenario_data_files(&scenario, &args.base_dir)?;

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // --- CSV table ---
    let mut csv = String::from("# grid-sim Module 1 sweep (docs/04 Stage 2 demo artefact)\n");
    csv.push_str(&format!("# engine_git_hash = {engine}\n"));
    csv.push_str(&format!(
        "# scenario_path = {}\n# scenario_sha256 = {scenario_sha}\n",
        args.scenario.display()
    ));
    for (path, hash) in &data_files {
        csv.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    csv.push_str(
        "# assumption 1: onshore+offshore wind scaled proportionally from the reference \
         split; 2024 ERA5 CF shapes kept at all capacities\n\
         # assumption 2: demand, thermal fleet + availability calibration, solar and the \
         exogenous must-take traces (imports, pumped storage, other) held at 2024 observed \
         values — imports/storage/demand responses arrive in Stages 3-5, so high-wind \
         points overstate curtailment\n\
         # assumption 3: curtailment is pooled (Stage 1 convention); % is over total \
         renewable (wind+solar) potential energy\n\
         # assumption 4: wind_capture_ratio is on POTENTIAL output (pooled-curtailment \
         convention, unchanged); wind_capture_ratio_delivered removes each period's \
         pro-rata share of the pooled curtailment first. Curtailment periods price at \
         GBP 0, so revenue is identical on both bases and the delivered ratio sits at or \
         above the potential one (definitions: grid_adequacy::pricing)\n",
    );
    csv.push_str(&format!(
        "# assumption 5 (Package B import-convention bracket): unsuffixed columns are the \
         FROZEN convention (imports held at 2024 observed values — the pre-Package-B \
         default, bit-identical); *_imports_zero sets the imports-flagged trace to 0 in \
         pre-import surplus periods (domestic must-take at the swept capacity strictly \
         above demand); *_imports_export sets it to -min(export_capacity, surplus \
         magnitude) there — the min() exports GB's own surplus only, never forcing \
         thermal dispatch. Mask is pre-storage by construction. export_capacity = \
         {export_capacity_value:.3} GW from {export_capacity_source} (definitions: \
         grid_adequacy::import_convention)\n",
        export_capacity_value = export_capacity.as_gigawatts(),
    ));
    csv.push_str(
        "# assumption 6 (interpretation guard): the import conventions act ONLY in \
         pre-import surplus periods, and the zero/export transformations keep every such \
         period must-take-only (GBP 0-priced) by construction — they move CURTAILMENT \
         and delivered energy, NOT price formation. Potential-basis capture is invariant \
         across the conventions wherever frozen masked periods are also GBP 0 (true \
         unless observed exports exceed the swept surplus in a masked period — none in \
         2024 at 40-60 GW; equality of the three wind_capture_ratio* columns is the \
         built-in check), and the gas / mean-SMP columns are frozen-run quantities by \
         definition, unchanged by construction. The delivered-capture spread across \
         conventions is therefore a convention WIDTH under a missing export-price \
         channel (tier 2: multi-zone pricing), not a correction direction — do NOT read \
         the lower delivered capture under export-in-surplus as import flexibility \
         worsening capture\n",
    );
    csv.push_str(
        "wind_capacity_gw,pct_periods_gas_price_setting,curtailment_twh,\
         curtailment_pct_of_renewable_potential,gas_twh,mean_smp_gbp_per_mwh,\
         wind_capture_ratio,wind_capture_ratio_delivered,\
         curtailment_twh_imports_zero,wind_capture_ratio_imports_zero,\
         wind_capture_ratio_delivered_imports_zero,\
         curtailment_twh_imports_export,wind_capture_ratio_imports_export,\
         wind_capture_ratio_delivered_imports_export\n",
    );
    let opt = |r: Option<f64>| r.map_or(String::new(), |r| r.to_string());
    for p in &results {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            p.wind_capacity_gw,
            p.pct_gas_price_setting,
            p.curtailment_twh,
            p.curtailment_pct_of_renewable_potential,
            p.gas_twh,
            p.mean_smp_gbp_per_mwh,
            opt(p.wind_capture_ratio),
            opt(p.wind_capture_ratio_delivered),
            p.curtailment_twh_imports_zero,
            opt(p.wind_capture_ratio_imports_zero),
            opt(p.wind_capture_ratio_delivered_imports_zero),
            p.curtailment_twh_imports_export,
            opt(p.wind_capture_ratio_imports_export),
            opt(p.wind_capture_ratio_delivered_imports_export),
        ));
    }
    let csv_path = args.out.join("module1_gas_marginal_vs_wind.csv");
    std::fs::write(&csv_path, &csv)
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // --- PNG chart ---
    let png_path = args.out.join("module1_gas_marginal_vs_wind.png");
    // Compact form for the chart footer (the CSV assumption 5 line
    // carries the full-precision value and its source).
    let export_note = format!(
        "export-in-surplus cap {:.2} GW ({})",
        export_capacity.as_gigawatts(),
        if args.export_capacity_gw.is_some() {
            "cli"
        } else {
            "scenario links"
        }
    );
    render_chart(&png_path, &results, engine, &scenario_sha, &export_note)?;

    println!("sweep complete: {} points", results.len());
    println!("  table {}", csv_path.display());
    println!("  chart {}", png_path.display());
    Ok(())
}

/// Total (offshore + onshore) wind capture ratio on BOTH bases for one
/// completed, priced run: potential output (pooled-curtailment
/// convention, Package A unchanged) and delivered output (potential
/// minus the pro-rata share of the pooled curtailment —
/// `grid_adequacy::pricing` prose).
fn wind_capture_both_bases(
    result: &RunResult,
    smp: &[grid_core::units::Price],
) -> Result<(Option<f64>, Option<f64>), String> {
    let is_wind = |tech: &str| matches!(tech, "offshore_wind" | "onshore_wind");
    let mut potential = vec![grid_core::units::Power::gigawatts(0.0); result.periods()];
    for series in result
        .renewables
        .iter()
        .filter(|s| is_wind(s.tech.as_str()))
    {
        for (acc, &p) in potential.iter_mut().zip(&series.power) {
            *acc = *acc + p;
        }
    }
    let delivered_all =
        grid_adequacy::delivered_renewable_power(result).map_err(|e| e.to_string())?;
    let mut delivered = vec![grid_core::units::Power::gigawatts(0.0); result.periods()];
    for (series, delivered_power) in result.renewables.iter().zip(&delivered_all) {
        if !is_wind(series.tech.as_str()) {
            continue;
        }
        for (acc, &p) in delivered.iter_mut().zip(delivered_power) {
            *acc = *acc + p;
        }
    }
    Ok((
        grid_core::pricing::capture_ratio(&potential, smp).map_err(|e| e.to_string())?,
        grid_core::pricing::capture_ratio(&delivered, smp).map_err(|e| e.to_string())?,
    ))
}

fn short(hash: &str) -> &str {
    &hash[..hash.len().min(12)]
}

fn render_chart(
    path: &PathBuf,
    results: &[SweepPoint],
    engine: &str,
    scenario_sha: &str,
    export_note: &str,
) -> Result<(), String> {
    let x_min = results.first().map_or(0.0, |p| p.wind_capacity_gw);
    let x_max = results.last().map_or(1.0, |p| p.wind_capacity_gw);

    let root = BitMapBackend::new(path, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(830);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            "Module 1 — how often does gas set the GB price as wind grows? (2024 system \
             otherwise held fixed)",
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_max, 0.0..100.0f64)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc("installed wind capacity, GW (onshore + offshore, scaled proportionally)")
        .y_desc("% of half-hourly periods (gas price-setting, curtailment) / capture ratio × 100")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let gas = RGBColor(230, 120, 30);
    let curt = RGBColor(60, 120, 216);
    chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|p| (p.wind_capacity_gw, p.pct_gas_price_setting)),
            gas.stroke_width(3),
        ))
        .map_err(|e| e.to_string())?
        .label("% periods gas price-setting (model flag)")
        .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], gas.stroke_width(3)));
    chart
        .draw_series(results.iter().map(|p| {
            Circle::new(
                (p.wind_capacity_gw, p.pct_gas_price_setting),
                5,
                gas.filled(),
            )
        }))
        .map_err(|e| e.to_string())?;
    chart
        .draw_series(LineSeries::new(
            results
                .iter()
                .map(|p| (p.wind_capacity_gw, p.curtailment_pct_of_renewable_potential)),
            curt.stroke_width(3),
        ))
        .map_err(|e| e.to_string())?
        .label("curtailment, % of renewable potential")
        .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], curt.stroke_width(3)));
    chart
        .draw_series(results.iter().map(|p| {
            Circle::new(
                (p.wind_capacity_gw, p.curtailment_pct_of_renewable_potential),
                5,
                curt.filled(),
            )
        }))
        .map_err(|e| e.to_string())?;

    // Both wind-capture bases (Package A), scaled ×100 onto the percent
    // axis: potential (pooled-curtailment convention, the Stage 2
    // published curve) and delivered (post-curtailment). The delivered
    // curve sits at or above the potential one — curtailment prices at
    // £0, so the bases differ in energy, not revenue.
    let capture_potential = RGBColor(40, 140, 70);
    let capture_delivered = RGBColor(140, 60, 170);
    let mut capture_series =
        |values: Vec<(f64, f64)>, colour: RGBColor, label: &str| -> Result<(), String> {
            chart
                .draw_series(LineSeries::new(
                    values.iter().copied(),
                    colour.stroke_width(3),
                ))
                .map_err(|e| e.to_string())?
                .label(label)
                .legend(move |(x, y)| {
                    PathElement::new(vec![(x, y), (x + 20, y)], colour.stroke_width(3))
                });
            chart
                .draw_series(
                    values
                        .iter()
                        .map(|&(x, y)| Circle::new((x, y), 5, colour.filled())),
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        };
    capture_series(
        results
            .iter()
            .filter_map(|p| {
                p.wind_capture_ratio
                    .map(|r| (p.wind_capacity_gw, 100.0 * r))
            })
            .collect(),
        capture_potential,
        "wind capture ratio × 100 (potential basis, frozen imports)",
    )?;
    capture_series(
        results
            .iter()
            .filter_map(|p| {
                p.wind_capture_ratio_delivered
                    .map(|r| (p.wind_capacity_gw, 100.0 * r))
            })
            .collect(),
        capture_delivered,
        "wind capture ratio × 100 (delivered basis, frozen imports)",
    )?;

    // The Package B import-convention bracket on the HEADLINE basis
    // (delivered — the Package A ruling): zero-in-surplus and
    // export-in-surplus around the frozen delivered curve above. The
    // full conventions × bases grid lives in the CSV; the chart carries
    // the delivered bracket plus the frozen potential curve for
    // continuity with the published figure (a six-curve chart of every
    // combination is unreadable — documented presentation choice).
    let capture_delivered_zero = RGBColor(90, 40, 130);
    let capture_delivered_export = RGBColor(200, 120, 220);
    capture_series(
        results
            .iter()
            .filter_map(|p| {
                p.wind_capture_ratio_delivered_imports_zero
                    .map(|r| (p.wind_capacity_gw, 100.0 * r))
            })
            .collect(),
        capture_delivered_zero,
        "wind capture ratio × 100 (delivered basis, zero-in-surplus imports)",
    )?;
    capture_series(
        results
            .iter()
            .filter_map(|p| {
                p.wind_capture_ratio_delivered_imports_export
                    .map(|r| (p.wind_capacity_gw, 100.0 * r))
            })
            .collect(),
        capture_delivered_export,
        "wind capture ratio × 100 (delivered basis, export-in-surplus imports)",
    )?;

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::MiddleLeft)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let caption = format!(
        "grid-sim | engine {engine} | scenario sha256 {} | demand, thermal fleet, solar and \
         exogenous traces held at 2024 values (see CSV assumptions) | {export_note}",
        short(scenario_sha),
    );
    footer
        .draw(&Text::new(
            caption,
            (20, 25),
            ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
        ))
        .map_err(|e| e.to_string())?;
    root.present().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// `sweep wind-capacity-zonal` — CLI exposure of the priced multi-zone
// wind sweep (docs/notes/wind-capacity-zonal-work-order.md). Wraps the
// tested library functions unchanged:
// `grid_adequacy::wind_capacity_sweep_multi` (one `--zone`) and
// `wind_capacity_sweep_multi_group` (several `--zone` flags: one shared
// scaling factor across the group's wind fleets, aggregate metrics —
// the D13 basis-(A) recipe). Imports are ENDOGENOUS in the multi-zone
// engine (the swept zone's `net_imports` is the modelled link position
// per capacity), so there is deliberately NO `--export-capacity-gw`
// analogue: the copper-plate sweep's Package B bracket flag has no
// meaning here.
// ---------------------------------------------------------------------

#[derive(Args)]
struct WindCapacityZonalArgs {
    /// Scenario TOML file (multi-zone; the swept zone(s) must carry
    /// wind, and pricing the metrics needs [zones.pricing] blocks).
    #[arg(long)]
    scenario: PathBuf,

    /// Output directory (created if absent).
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which relative trace paths are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Swept zone id. Repeat the flag for a zone GROUP: the group's
    /// wind fleets scale with ONE shared factor and the recorded
    /// metrics are the group aggregate.
    #[arg(long = "zone", required = true)]
    zones: Vec<String>,

    /// Smallest total wind capacity, GW.
    #[arg(long, default_value_t = 10.0)]
    min_gw: f64,

    /// Largest total wind capacity, GW.
    #[arg(long, default_value_t = 60.0)]
    max_gw: f64,

    /// Capacity step, GW.
    #[arg(long, default_value_t = 5.0)]
    step_gw: f64,
}

fn wind_capacity_zonal(args: &WindCapacityZonalArgs) -> Result<(), String> {
    if !(args.step_gw > 0.0 && args.max_gw >= args.min_gw && args.min_gw > 0.0) {
        return Err(format!(
            "invalid sweep range: min {} GW, max {} GW, step {} GW",
            args.min_gw, args.max_gw, args.step_gw
        ));
    }

    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    if scenario.zones.len() < 2 {
        return Err(
            "the zonal sweep re-dispatches a multi-zone scenario; for a single-zone \
             scenario use `sweep wind-capacity` (the copper-plate Module 1 sweep)"
                .to_owned(),
        );
    }
    // Traces and per-zone SRMC chains are loaded once and shared across
    // points (capacity scaling enters at dispatch, not trace loading —
    // the library's stated convention 4).
    let inputs = load_multi_zone_inputs(&scenario, &args.base_dir).map_err(|e| e.to_string())?;

    // The capacity points: min, min+step, …, max (max included when the
    // range is a whole number of steps — the sibling sweep's rule).
    let mut capacities: Vec<Power> = Vec::new();
    let steps = ((args.max_gw - args.min_gw) / args.step_gw).round() as usize;
    for i in 0..=steps {
        let target = args.min_gw + i as f64 * args.step_gw;
        if target <= args.max_gw + 1e-9 {
            capacities.push(Power::gigawatts(target));
        }
    }

    // One `--zone` → the single-zone sweep; several → the zone-group
    // sweep (one shared factor, aggregate metrics). The joined ids
    // label the group's rows and artefacts.
    let zone_ids: Vec<&str> = args.zones.iter().map(String::as_str).collect();
    let (zone_label, points) = if let [zone] = zone_ids.as_slice() {
        let sweep =
            wind_capacity_sweep_multi(&scenario, &inputs, zone, &capacities, Execution::Parallel)
                .map_err(|e| e.to_string())?;
        (sweep.zone.as_str().to_owned(), sweep.points)
    } else {
        let sweep = wind_capacity_sweep_multi_group(
            &scenario,
            &inputs,
            &zone_ids,
            &capacities,
            Execution::Parallel,
        )
        .map_err(|e| e.to_string())?;
        (args.zones.join("+"), sweep.points)
    };

    let ratio_cell = |r: Option<f64>| r.map_or(String::new(), |r| r.to_string());
    for p in &points {
        println!(
            "  {:5.1} GW wind in {zone_label}: capture {} / {} (potential/delivered), \
             mean SMP {:6.2} GBP/MWh, curtailment {:7.3} TWh, gas {:6.2} TWh, \
             net imports {:7.3} TWh",
            p.wind_capacity.as_gigawatts(),
            ratio_cell(p.wind_capture_ratio),
            ratio_cell(p.wind_capture_ratio_delivered),
            p.mean_smp.as_pounds_per_megawatt_hour(),
            twh(p.curtailment),
            twh(p.gas),
            twh(p.net_imports),
        );
    }

    // Metadata (docs/06): engine + scenario + per-data-file hashes —
    // parity with the sibling sweep's provenance header.
    let engine = env!("GRID_ENGINE_GIT_HASH");
    let scenario_sha = crate::run::sha256_file(&args.scenario)?;
    let data_files = crate::run::scenario_data_files(&scenario, &args.base_dir)?;

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // --- CSV table ---
    let mut csv = String::from(
        "# grid-sim Module 1 ZONAL sweep (docs/notes/wind-capacity-zonal-work-order.md): \
         the swept zone's wind capacity on the priced multi-zone engine, imports \
         ENDOGENOUS through the scenario's links\n",
    );
    csv.push_str(&format!("# engine_git_hash = {engine}\n"));
    csv.push_str(&format!(
        "# scenario_path = {}\n# scenario_sha256 = {scenario_sha}\n",
        args.scenario.display()
    ));
    csv.push_str(&format!("# swept_zone = {zone_label}\n"));
    for (path, hash) in &data_files {
        csv.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    // Assumption block 1 — the 3-zone honesty conventions, VERBATIM
    // from the scenario file's own header (its binding-obligation
    // banner and per-gate direction signs).
    csv.push_str(
        "# assumption 1 (3-zone honesty conventions, verbatim from the scenario header):\n\
         #   \"(2) The model may quote DIRECTION + PINNED TOTALS under stated\n\
         #   conventions ONLY. NO \"B4 effect proper\" %, NO B4-vs-B6\n\
         #   decomposition (the single-pass rule across two hub-sharing borders\n\
         #   compounds the equal-depth artefact that inverted the B6 magnitude;\n\
         #   the Stage-7 LP is the resolver). B4 is quotable for DIRECTION +\n\
         #   BINDING FREQUENCY; its net magnitude carries \"DA-only, no outturn anchor\".\"\n\
         #   \"- For the B4 gate it is ANTI-CONSERVATIVE: concentrating onshore\n\
         #   capacity north (0.4077 vs cluster 0.311) raises northern generation\n\
         #   per unit Scottish capacity, so the split OVERSTATES B4 binding. This\n\
         #   does not breach the lower-bound duty because B4 magnitude is quoted\n\
         #   only as a DA-anchored wedge; the headline is direction + pinned\n\
         #   totals, never a \"B4 effect proper\".\"\n",
    );
    // Assumption block 2 — the price-floor guard (supervisor addition,
    // the hostile-reader line: the missing negative-price channel can
    // only make cannibalisation LOOK milder, never worse).
    csv.push_str(
        "# assumption 2: SMP floors at 0 GBP/MWh; no negative pricing, no CfD-floor \
         bidding — understates cannibalisation (anti-cannibalisation-conservative)\n",
    );
    // Assumption block 3 — which capture ratio is which (the Stage 2 /
    // P-Q10 conventions, the Package A pair).
    csv.push_str(
        "# assumption 3 (capture-ratio definitions, Stage 2 / P-Q10 conventions): \
         wind_capture_ratio is the swept zone's total (onshore+offshore) wind capture \
         price over its time-weighted mean SMP on the POTENTIAL output basis \
         (pooled-curtailment convention, unchanged); wind_capture_ratio_delivered \
         removes each period's pro-rata share of the zone's pooled curtailment from \
         the wind series first (delivered basis). Curtailment periods price at GBP 0, \
         so revenue is identical on both bases and the delivered ratio sits at or \
         above the potential one (definitions: grid_adequacy::pricing and \
         grid_adequacy::sweep::MultiZoneWindPoint). A blank ratio cell is a \
         well-defined None (a 0/0 quotient: zero wind output or an all-GBP-0 SMP \
         series), never NaN\n",
    );
    csv.push_str(
        "wind_capacity_gw,zone,gas_price_setting_share,curtailment_twh,gas_twh,\
         net_imports_twh,unserved_twh,mean_smp_gbp_per_mwh,wind_capture_ratio,\
         wind_capture_ratio_delivered\n",
    );
    for p in &points {
        csv.push_str(&format!(
            "{},{zone_label},{},{},{},{},{},{},{},{}\n",
            p.wind_capacity.as_gigawatts(),
            p.gas_price_setting_share,
            twh(p.curtailment),
            twh(p.gas),
            twh(p.net_imports),
            twh(p.unserved),
            p.mean_smp.as_pounds_per_megawatt_hour(),
            ratio_cell(p.wind_capture_ratio),
            ratio_cell(p.wind_capture_ratio_delivered),
        ));
    }
    let csv_path = args
        .out
        .join(format!("module1_zonal_capture_vs_wind_{zone_label}.csv"));
    std::fs::write(&csv_path, &csv)
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // --- PNG chart ---
    let png_path = args
        .out
        .join(format!("module1_zonal_capture_vs_wind_{zone_label}.png"));
    render_zonal_chart(&png_path, &points, &zone_label, engine, &scenario_sha)?;

    println!("sweep complete: {} points", points.len());
    println!("  table {}", csv_path.display());
    println!("  chart {}", png_path.display());
    Ok(())
}

/// The analogous chart to [`render_chart`], on the zonal sweep's
/// percent axis: the swept zone's gas price-setting share and both
/// wind-capture bases (×100) against installed wind capacity, with the
/// docs/06 hash footer.
fn render_zonal_chart(
    path: &PathBuf,
    points: &[MultiZoneWindPoint],
    zone_label: &str,
    engine: &str,
    scenario_sha: &str,
) -> Result<(), String> {
    let gw = |p: &MultiZoneWindPoint| p.wind_capacity.as_gigawatts();
    let x_min = points.first().map_or(0.0, gw);
    let x_max = points.last().map_or(1.0, gw);

    let root = BitMapBackend::new(path, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(830);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            format!(
                "Module 1 zonal — {zone_label} wind capture vs installed wind on the \
                 priced multi-zone engine (imports endogenous)"
            ),
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_max, 0.0..100.0f64)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc(format!(
            "installed wind capacity in {zone_label}, GW (onshore + offshore, scaled \
             proportionally)"
        ))
        .y_desc("% of half-hourly periods (gas price-setting) / capture ratio × 100")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let mut series =
        |values: Vec<(f64, f64)>, colour: RGBColor, label: &str| -> Result<(), String> {
            chart
                .draw_series(LineSeries::new(
                    values.iter().copied(),
                    colour.stroke_width(3),
                ))
                .map_err(|e| e.to_string())?
                .label(label)
                .legend(move |(x, y)| {
                    PathElement::new(vec![(x, y), (x + 20, y)], colour.stroke_width(3))
                });
            chart
                .draw_series(
                    values
                        .iter()
                        .map(|&(x, y)| Circle::new((x, y), 5, colour.filled())),
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        };
    series(
        points
            .iter()
            .map(|p| (gw(p), 100.0 * p.gas_price_setting_share))
            .collect(),
        RGBColor(230, 120, 30),
        "% periods gas price-setting (model flag)",
    )?;
    series(
        points
            .iter()
            .filter_map(|p| p.wind_capture_ratio.map(|r| (gw(p), 100.0 * r)))
            .collect(),
        RGBColor(40, 140, 70),
        "wind capture ratio × 100 (potential basis)",
    )?;
    series(
        points
            .iter()
            .filter_map(|p| p.wind_capture_ratio_delivered.map(|r| (gw(p), 100.0 * r)))
            .collect(),
        RGBColor(140, 60, 170),
        "wind capture ratio × 100 (delivered basis)",
    )?;

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::MiddleLeft)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let caption = format!(
        "grid-sim | engine {engine} | scenario sha256 {} | zone {zone_label} | SMP floors \
         at 0 GBP/MWh (anti-cannibalisation-conservative; see CSV assumptions) | imports \
         endogenous (multi-zone links)",
        short(scenario_sha),
    );
    footer
        .draw(&Text::new(
            caption,
            (20, 25),
            ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
        ))
        .map_err(|e| e.to_string())?;
    root.present().map_err(|e| e.to_string())?;
    Ok(())
}
