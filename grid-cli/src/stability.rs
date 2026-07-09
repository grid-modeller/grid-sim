//! `grid-cli stability` — the Stage 6 stability engine's CLI (docs/04
//! Stage 6; ADR-2):
//!
//! - `stability event`: simulate a loss-of-infeed event from an event
//!   spec TOML (optionally overriding the spec's inertia — the
//!   published 2019 record carries two official bounds), writing the
//!   frequency trace as CSV + Parquet (docs/06: both, always), a
//!   `report.toml` (nadir, first arrest, RoCoF, LFDD actions,
//!   era-limit checks) and a PNG chart with the docs/06 hash footer.
//!   `--measured` overlays a measured frequency trace (e.g. the
//!   committed NESO 1-s fixture for 9 Aug 2019) aligned via
//!   `--measured-t0` (the UTC instant of sim t = 0).
//! - `stability inertia`: the minimum-inertia hour finder — run a
//!   scenario's dispatch, aggregate Σ(H × MVA) per period
//!   (`grid_stability::inertia`; MVA = GW / 0.9), write the series
//!   (CSV + Parquet) and a report with the minimum-inertia period and
//!   the hours below each `--floor` (defaults: the FRCR 120 GVA·s
//!   requirement and 102 GVA·s ambition). If the scenario has no
//!   synchronous provision at all, the run output states the finding
//!   in words (the Royal-Society-scenario result). With
//!   `--renewable-scale` (comma-separated factors) it runs the
//!   **Module 6 sweep** instead: wind + solar capacities scaled by
//!   each factor (everything else held at the scenario's values, the
//!   Stage 2 sweep convention), hours below each floor per point, and
//!   the hours-below vs renewable-share chart. "Renewable share" is
//!   the **potential (as-produced, pre-curtailment) wind + solar
//!   energy as a share of underlying demand energy** — the D3
//!   denominator convention and the Stage 1 pooled-curtailment
//!   numerator convention; the delivered share
//!   ((potential − pooled curtailment) / demand) is also recorded in
//!   the CSV. The part-1 UNCONSTRAINED caveat carries into the report,
//!   the chart footer and the console: this is market-only dispatch
//!   with no must-run and no NESO stability actions.
//! - `stability pathway`: the Q8 pathway runner (Stage 6 part 2) —
//!   largest survivable loss-of-infeed vs year under a fleet pathway
//!   (`schema = "fes-pathway-v1"`), one line per dispatch condition
//!   (the band), with the secured-loss standards as reference lines so
//!   "the year the grid can no longer ride through" reads as a date.
//!   All modelling conventions live in `grid_stability::pathway`.
//!
//! Every output carries the docs/06 metadata block (engine git hash,
//! input-file SHA-256s, schema/spec version, timestamp — the CLI
//! layer's one permitted wall-clock read).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{Float64Builder, TimestampMicrosecondBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use clap::{Args, Subcommand};
use grid_adequacy::{RunResult, load_run_inputs, run};
use grid_core::scenario::{SCHEMA_VERSION, Scenario};
use grid_core::time::UtcInstant;
use grid_core::units::Inertia;
use grid_stability::{
    EventResult, EventSpec, InertiaTable, PathwaySpec, inertia_series, run_pathway, simulate,
};
use plotters::prelude::*;
use sha2::{Digest, Sha256};

use crate::run::{now_utc, scenario_data_files, sha256_file};

/// Arguments to `grid-cli stability`.
#[derive(Args)]
pub struct StabilityArgs {
    #[command(subcommand)]
    command: StabilityCommand,
}

#[derive(Subcommand)]
enum StabilityCommand {
    /// Simulate a loss-of-infeed event from an event spec TOML.
    Event(EventArgs),
    /// Find the minimum-inertia hours of a scenario's dispatch; with
    /// --renewable-scale, run the Module 6 sweep (hours below the
    /// inertia floors vs renewable share).
    Inertia(InertiaArgs),
    /// Q8 pathway runner: largest survivable loss-of-infeed vs year
    /// under a fleet pathway (fes-pathway-v1 spec).
    Pathway(PathwayArgs),
    /// Correlate the bottom-up (`grid_stability::inertia_from_generation`)
    /// inertia estimate against the NESO System Inertia outturn series,
    /// over the 2024 data pack.
    ValidateInertia(ValidateInertiaArgs),
}

#[derive(Args)]
struct EventArgs {
    /// Event spec TOML file (`schema = "stability-event-v1"`).
    #[arg(long)]
    event: PathBuf,

    /// Output directory (created if absent).
    #[arg(long)]
    out: PathBuf,

    /// Override the spec's system inertia, GVA·s (e.g. to run both
    /// published bounds of a historical event).
    #[arg(long)]
    inertia_gva_s: Option<f64>,

    /// Measured frequency trace to overlay on the chart: a CSV with
    /// `#` comment lines and `utc_time,frequency_hz` columns.
    #[arg(long, requires = "measured_t0")]
    measured: Option<PathBuf>,

    /// The UTC instant of simulation t = 0 (RFC 3339; fractional
    /// seconds allowed), aligning the measured trace.
    #[arg(long)]
    measured_t0: Option<String>,
}

#[derive(Args)]
struct InertiaArgs {
    /// Scenario TOML file (schema v3).
    #[arg(long)]
    scenario: PathBuf,

    /// Output directory (created if absent).
    #[arg(long)]
    out: PathBuf,

    /// Base directory against which relative trace paths in the
    /// scenario are resolved.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Inertia floor(s) to count hours below, GVA·s (repeatable).
    /// Defaults: 120 (NESO FRCR minimum requirement, 2024-06-19) and
    /// 102 (the 2025 ambition) — data/reference/inertia-constants.toml.
    #[arg(long = "floor")]
    floors: Vec<f64>,

    /// Module 6 sweep: scale wind + solar capacities by each factor
    /// (comma-separated) and chart hours below the floors vs renewable
    /// share. Everything else is held at the scenario's values (the
    /// Stage 2 sweep convention). Empty = the single-run finder.
    #[arg(long = "renewable-scale", value_delimiter = ',')]
    renewable_scales: Vec<f64>,

    /// NESO System Inertia outturn parquet (`inertia_outturn_2024.parquet`,
    /// Stage 6 NESO enrichment): when given, inner-joins this run's own
    /// dispatch-derived `inertia_series` against the NESO outturn column
    /// by UTC instant and reports the correlation (the engine-level
    /// counterpart to `stability validate-inertia`'s method-level check).
    #[arg(long)]
    reference: Option<PathBuf>,
}

#[derive(Args)]
struct PathwayArgs {
    /// Pathway spec TOML file (`schema = "fes-pathway-v1"`).
    #[arg(long)]
    pathway: PathBuf,

    /// Output directory (created if absent).
    #[arg(long)]
    out: PathBuf,
}

#[derive(Args)]
struct ValidateInertiaArgs {
    /// Pack root: reads
    /// `<base-dir>/data/packs/2024/processed/generation_by_fuel_2024.parquet`
    /// and `inertia_outturn_2024.parquet`.
    #[arg(long, default_value = ".")]
    base_dir: PathBuf,

    /// Report output file (TOML; not a directory).
    #[arg(long)]
    out: PathBuf,
}

/// Execute `grid-cli stability`.
pub fn execute(args: &StabilityArgs) -> Result<(), String> {
    match &args.command {
        StabilityCommand::Event(event_args) => event(event_args),
        StabilityCommand::Inertia(inertia_args) => inertia(inertia_args),
        StabilityCommand::Pathway(pathway_args) => pathway(pathway_args),
        StabilityCommand::ValidateInertia(validate_args) => validate_inertia(validate_args),
    }
}

fn toml_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

// ---------------------------------------------------------------------
// stability event
// ---------------------------------------------------------------------

fn event(args: &EventArgs) -> Result<(), String> {
    let mut spec = EventSpec::load(&args.event).map_err(|e| e.to_string())?;
    if let Some(gva_s) = args.inertia_gva_s {
        if gva_s.is_nan() || gva_s <= 0.0 || !gva_s.is_finite() {
            return Err(format!(
                "--inertia-gva-s {gva_s} must be positive and finite"
            ));
        }
        spec.inertia = Inertia::gigavolt_ampere_seconds(gva_s);
    }
    let result = simulate(&spec).map_err(|e| e.to_string())?;

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // docs/06 metadata block for every output.
    let spec_sha256 = sha256_file(&args.event)?;
    let created = now_utc();
    let metadata_pairs: Vec<(&str, String)> = vec![
        ("engine_git_hash", env!("GRID_ENGINE_GIT_HASH").to_owned()),
        ("event_spec_schema", grid_stability::EVENT_SCHEMA.to_owned()),
        ("event_spec_path", args.event.display().to_string()),
        ("event_spec_sha256", spec_sha256.clone()),
        (
            "inertia_gva_s",
            format!("{}", spec.inertia.as_gigavolt_ampere_seconds()),
        ),
        ("created_utc", created.clone()),
    ];
    let mut metadata_block = String::from("# grid-sim output (docs/06 metadata header)\n");
    for (key, value) in &metadata_pairs {
        metadata_block.push_str(&format!("# {key} = {value}\n"));
    }

    // Frequency trace CSV: data section is the deterministic digest
    // input, same discipline as `run`.
    let mut data_section = String::from("t_s,frequency_hz");
    for timeline in &result.response_timelines {
        data_section.push_str(&format!(",{}_delivered_gw", timeline.name));
    }
    data_section.push('\n');
    for (index, (t, f)) in result.trace().iter().enumerate() {
        data_section.push_str(&format!("{},{}", t.as_seconds(), f.as_hertz()));
        for timeline in &result.response_timelines {
            data_section.push_str(&format!(",{}", timeline.delivered[index].as_gigawatts()));
        }
        data_section.push('\n');
    }
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
    let csv_path = args.out.join("frequency_trace.csv");
    std::fs::write(&csv_path, format!("{metadata_block}{data_section}"))
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;
    write_trace_parquet(
        &args.out.join("frequency_trace.parquet"),
        &result,
        &metadata_pairs,
    )?;

    // report.toml.
    let mut report = String::from("# grid-cli stability event report (docs/06)\n[metadata]\n");
    for (key, value) in &metadata_pairs {
        report.push_str(&format!("{key} = {}\n", toml_quote(value)));
    }
    report.push_str(&format!(
        "\n[results]\nresult_digest_sha256 = {}\n",
        toml_quote(&digest)
    ));
    report.push_str(&format!("nadir_hz = {}\n", result.nadir.as_hertz()));
    report.push_str(&format!("nadir_at_s = {}\n", result.nadir_at.as_seconds()));
    if let (Some(arrest), Some(at)) = (result.first_arrest, result.first_arrest_at) {
        report.push_str(&format!("first_arrest_hz = {}\n", arrest.as_hertz()));
        report.push_str(&format!("first_arrest_at_s = {}\n", at.as_seconds()));
    }
    if let Some(rocof) = result.rocof_window_mean {
        report.push_str(&format!(
            "rocof_window_mean_hz_per_s = {}\n",
            rocof.as_hertz_per_second()
        ));
    }
    report.push_str(&format!(
        "steepest_1s_rocof_hz_per_s = {}\n",
        result.steepest_1s_rocof.as_hertz_per_second()
    ));
    if let Some(limits) = &result.limit_report {
        report.push_str("\n[results.limits]\n");
        if let Some(exceeded) = limits.rocof_relay_exceeded {
            report.push_str(&format!("rocof_relay_exceeded = {exceeded}\n"));
        }
        if let Some(breached) = limits.statutory_floor_breached {
            report.push_str(&format!("statutory_floor_breached = {breached}\n"));
        }
    }
    for action in &result.lfdd_actions {
        report.push_str(&format!(
            "\n[[results.lfdd_actions]]\nstage = {}\n",
            action.stage
        ));
        report.push_str(&format!("trigger_hz = {}\n", action.trigger.as_hertz()));
        report.push_str(&format!(
            "triggered_at_s = {}\n",
            action.triggered_at.as_seconds()
        ));
        report.push_str(&format!(
            "actioned_at_s = {}\n",
            action.actioned_at.as_seconds()
        ));
        report.push_str(&format!(
            "block_mw = {}\n",
            action.block.as_gigawatts() * 1000.0
        ));
    }
    let report_path = args.out.join("report.toml");
    std::fs::write(&report_path, report)
        .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;

    // Chart (PNG, docs/06 hash footer), with the measured overlay when
    // given.
    let measured = match (&args.measured, &args.measured_t0) {
        (Some(path), Some(t0)) => Some(read_measured(path, t0, &result)?),
        _ => None,
    };
    render_event_chart(
        &args.out.join("frequency.png"),
        &spec,
        &result,
        measured.as_deref(),
        &metadata_pairs,
    )?;

    // Console summary.
    println!("event: {}", spec.name);
    println!(
        "  inertia {} GVA·s, demand {} GW",
        spec.inertia.as_gigavolt_ampere_seconds(),
        spec.demand.as_gigawatts()
    );
    println!(
        "  nadir {:.4} Hz at t = {:.2} s",
        result.nadir.as_hertz(),
        result.nadir_at.as_seconds()
    );
    if let (Some(arrest), Some(at)) = (result.first_arrest, result.first_arrest_at) {
        println!(
            "  first arrest {:.4} Hz at t = {:.2} s",
            arrest.as_hertz(),
            at.as_seconds()
        );
    }
    if let Some(rocof) = result.rocof_window_mean {
        println!(
            "  RoCoF over the pinned window: {:.4} Hz/s",
            rocof.as_hertz_per_second()
        );
    }
    for action in &result.lfdd_actions {
        println!(
            "  LFDD stage {} ({} Hz): {} MW disconnected at t = {:.2} s",
            action.stage,
            action.trigger.as_hertz(),
            action.block.as_gigawatts() * 1000.0,
            action.actioned_at.as_seconds()
        );
    }
    println!("  result digest sha256 = {digest}");
    println!("  outputs in {}", args.out.display());
    Ok(())
}

fn write_trace_parquet(
    path: &Path,
    result: &EventResult,
    metadata_pairs: &[(&str, String)],
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let mut fields = vec![
        Field::new("t_s", DataType::Float64, false),
        Field::new("frequency_hz", DataType::Float64, false),
    ];
    for timeline in &result.response_timelines {
        fields.push(Field::new(
            format!("{}_delivered_gw", timeline.name),
            DataType::Float64,
            false,
        ));
    }
    let schema = Arc::new(Schema::new(fields));

    let mut times = Float64Builder::new();
    let mut hz = Float64Builder::new();
    for (t, f) in result.trace() {
        times.append_value(t.as_seconds());
        hz.append_value(f.as_hertz());
    }
    let mut arrays: Vec<ArrayRef> = vec![Arc::new(times.finish()), Arc::new(hz.finish())];
    for timeline in &result.response_timelines {
        let mut builder = Float64Builder::new();
        for &delivered in &timeline.delivered {
            builder.append_value(delivered.as_gigawatts());
        }
        arrays.push(Arc::new(builder.finish()));
    }
    let batch = RecordBatch::try_new(schema.clone(), arrays).map_err(|e| err(&e))?;

    let kv: Vec<parquet::file::metadata::KeyValue> = metadata_pairs
        .iter()
        .map(|(key, value)| {
            parquet::file::metadata::KeyValue::new((*key).to_owned(), value.clone())
        })
        .collect();
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

/// Parse a measured `utc_time,frequency_hz` CSV (with `#` comments)
/// into (t seconds since `t0`, Hz) points within the simulated span.
fn read_measured(path: &Path, t0: &str, result: &EventResult) -> Result<Vec<(f64, f64)>, String> {
    // t0 may carry fractional seconds (the 2019 fault is at
    // 15:52:33.490Z); UtcInstant parses whole seconds, so split the
    // fraction off here.
    let (whole, fraction) = match t0.split_once('.') {
        Some((whole, rest)) => {
            let digits = rest.trim_end_matches('Z');
            let fraction = format!("0.{digits}")
                .parse::<f64>()
                .map_err(|e| format!("--measured-t0 {t0:?}: bad fractional seconds: {e}"))?;
            (format!("{whole}Z"), fraction)
        }
        None => (t0.to_owned(), 0.0),
    };
    let t0_instant = UtcInstant::parse(&whole).map_err(|e| e.to_string())?;
    let span_s = result.trace().last().map_or(0.0, |(t, _)| t.as_seconds());

    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut points = Vec::new();
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with("utc_time") || line.trim().is_empty() {
            continue;
        }
        let (stamp, value) = line
            .split_once(',')
            .ok_or_else(|| format!("{}: malformed line {line:?}", path.display()))?;
        let instant = UtcInstant::parse(stamp).map_err(|e| e.to_string())?;
        let t = (instant.unix_micros() - t0_instant.unix_micros()) as f64 / 1e6 - fraction;
        if t < 0.0 || t > span_s {
            continue;
        }
        let hz: f64 = value
            .parse()
            .map_err(|e| format!("{}: bad frequency {value:?}: {e}", path.display()))?;
        points.push((t, hz));
    }
    if points.is_empty() {
        return Err(format!(
            "{}: no measured samples fall inside the simulated span",
            path.display()
        ));
    }
    Ok(points)
}

fn render_event_chart(
    out: &Path,
    spec: &EventSpec,
    result: &EventResult,
    measured: Option<&[(f64, f64)]>,
    metadata_pairs: &[(&str, String)],
) -> Result<(), String> {
    let span_s = result.trace().last().map_or(1.0, |(t, _)| t.as_seconds());
    let modelled_min = result.nadir.as_hertz();
    let measured_min = measured
        .map(|points| points.iter().fold(f64::INFINITY, |acc, &(_, f)| acc.min(f)))
        .unwrap_or(f64::INFINITY);
    let y_min = modelled_min.min(measured_min) - 0.15;
    let y_max = 50.35;

    let root = BitMapBackend::new(out, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(830);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(&spec.name, ("sans-serif", 28))
        .margin(20)
        .x_label_area_size(45)
        .y_label_area_size(80)
        .build_cartesian_2d(0.0..span_s, y_min..y_max)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc("seconds from event start")
        .y_desc("frequency, Hz")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    // LFDD stage-1 trigger level as a guide line, when the spec has one.
    if let Some(lfdd) = &spec.lfdd {
        let level = lfdd.stages[0].frequency.as_hertz();
        chart
            .draw_series(LineSeries::new(
                [(0.0, level), (span_s, level)],
                BLACK.mix(0.4).stroke_width(1),
            ))
            .map_err(|e| e.to_string())?
            .label(format!("LFDD stage 1 ({level} Hz)"))
            .legend(|(x, y)| {
                PathElement::new(vec![(x, y), (x + 18, y)], BLACK.mix(0.4).stroke_width(1))
            });
    }

    let modelled_color = RGBColor(0, 110, 160);
    chart
        .draw_series(LineSeries::new(
            result
                .trace()
                .iter()
                .map(|(t, f)| (t.as_seconds(), f.as_hertz())),
            modelled_color.stroke_width(2),
        ))
        .map_err(|e| e.to_string())?
        .label("modelled (single-bus swing)")
        .legend(move |(x, y)| {
            PathElement::new(vec![(x, y), (x + 18, y)], modelled_color.stroke_width(3))
        });

    if let Some(points) = measured {
        let measured_color = RGBColor(200, 60, 40);
        chart
            .draw_series(LineSeries::new(
                points.iter().copied(),
                measured_color.stroke_width(2),
            ))
            .map_err(|e| e.to_string())?
            .label("measured (NESO 1-s data)")
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 18, y)], measured_color.stroke_width(3))
            });
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::LowerRight)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let lookup = |key: &str| {
        metadata_pairs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or("?")
    };
    let spec_hash = lookup("event_spec_sha256");
    let caption = format!(
        "grid-sim | engine {} | event spec sha256 {} | inertia {} GVA·s | generated {}",
        lookup("engine_git_hash"),
        &spec_hash[..12.min(spec_hash.len())],
        lookup("inertia_gva_s"),
        lookup("created_utc"),
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
// stability inertia
// ---------------------------------------------------------------------

fn inertia(args: &InertiaArgs) -> Result<(), String> {
    if !args.renewable_scales.is_empty() {
        return module6(args);
    }
    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    let inputs = load_run_inputs(&scenario, &args.base_dir).map_err(|e| e.to_string())?;
    let result = run(&scenario, &inputs).map_err(|e| e.to_string())?;
    let table = InertiaTable::from_scenario(&scenario).map_err(|e| e.to_string())?;
    let series = inertia_series(&result, &table).map_err(|e| e.to_string())?;
    let neso_fit = args
        .reference
        .as_ref()
        .map(|path| engine_vs_neso_fit(&result, &series, path).map(|fit| (path.clone(), fit)))
        .transpose()?;

    let floors = if args.floors.is_empty() {
        vec![120.0, 102.0]
    } else {
        args.floors.clone()
    };

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // docs/06 metadata.
    let created = now_utc();
    let metadata_pairs: Vec<(&str, String)> = vec![
        ("engine_git_hash", env!("GRID_ENGINE_GIT_HASH").to_owned()),
        ("schema_version", SCHEMA_VERSION.to_string()),
        ("scenario_path", args.scenario.display().to_string()),
        ("scenario_sha256", sha256_file(&args.scenario)?),
        ("created_utc", created.clone()),
    ];
    let data_files = scenario_data_files(&scenario, &args.base_dir)?;
    let mut metadata_block = String::from("# grid-sim output (docs/06 metadata header)\n");
    for (key, value) in &metadata_pairs {
        metadata_block.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &data_files {
        metadata_block.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }

    // Series CSV (data section = digest input) + Parquet.
    let mut data_section = String::from("utc_start,inertia_gva_s\n");
    for (t, value) in series.iter().enumerate() {
        data_section.push_str(&format!(
            "{},{}\n",
            result.timestamp_at(t),
            value.as_gigavolt_ampere_seconds()
        ));
    }
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
    let csv_path = args.out.join("inertia.csv");
    std::fs::write(&csv_path, format!("{metadata_block}{data_section}"))
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;
    write_inertia_parquet(
        &args.out.join("inertia.parquet"),
        &result,
        &series,
        &metadata_pairs,
    )?;

    // Report.
    let (min_index, min) = grid_stability::min_inertia(&series).ok_or("empty inertia series")?;
    let mean = series
        .iter()
        .map(|v| v.as_gigavolt_ampere_seconds())
        .sum::<f64>()
        / series.len() as f64;
    let max = series
        .iter()
        .map(|v| v.as_gigavolt_ampere_seconds())
        .fold(0.0_f64, f64::max);
    let zero_periods = series
        .iter()
        .filter(|v| v.as_gigavolt_ampere_seconds() == 0.0)
        .count();

    let mut report = String::from("# grid-cli stability inertia report (docs/06)\n[metadata]\n");
    for (key, value) in &metadata_pairs {
        let value = if *key == "schema_version" {
            value.clone()
        } else {
            toml_quote(value)
        };
        report.push_str(&format!("{key} = {value}\n"));
    }
    report.push_str("\n[metadata.data_files]\n");
    for (path, hash) in &data_files {
        report.push_str(&format!("{} = {}\n", toml_quote(path), toml_quote(hash)));
    }
    report.push_str(&format!(
        "\n[results]\nperiods = {}\nresult_digest_sha256 = {}\n",
        series.len(),
        toml_quote(&digest)
    ));
    report.push_str(
        "# MODEL-dispatch inertia, UNCONSTRAINED by any inertia product: the adequacy\n\
         # engine has no must-run floor and no NESO stability actions, so the low tail\n\
         # reads lower than the operational record. The gap below the floors is the\n\
         # Module 6 finding (what the energy market alone would provide), not a claim\n\
         # about operated GB frequency security.\n",
    );
    report.push_str(&format!(
        "min_inertia_gva_s = {}\n",
        min.as_gigavolt_ampere_seconds()
    ));
    report.push_str(&format!(
        "min_inertia_at = {}\n",
        toml_quote(&result.timestamp_at(min_index).to_string())
    ));
    report.push_str(&format!("mean_inertia_gva_s = {mean}\n"));
    report.push_str(&format!("max_inertia_gva_s = {max}\n"));
    report.push_str(&format!("zero_inertia_periods = {zero_periods}\n"));
    let synchronous_provision = grid_stability::has_synchronous_provision(&scenario);
    report.push_str(&format!(
        "has_synchronous_provision = {synchronous_provision}\n"
    ));
    for floor in &floors {
        let below =
            grid_stability::periods_below(&series, Inertia::gigavolt_ampere_seconds(*floor));
        report.push_str(&format!(
            "\n[[results.floors]]\nfloor_gva_s = {floor}\nperiods_below = {below}\n\
             hours_below = {}\n",
            below as f64 * 0.5
        ));
    }
    if let Some((reference, fit)) = &neso_fit {
        report.push_str(&format!(
            "\n[results.neso_correlation]\n# engine inertia_series vs NESO System Inertia \
             outturn ({}), inner-joined by UTC instant (Stage 6 NESO enrichment Task 9).\n\
             # This is the engine-level (dispatch) counterpart to `stability \
             validate-inertia`'s method-level (bottom-up-from-generation) check; a scenario\n\
             # dispatch is cost-optimised, not a reconstruction of actual 2024 unit \
             commitment, so a lower correlation than the method-level check is expected.\n\
             n = {}\npearson_r = {}\nslope = {}\nintercept = {}\nmedian_ratio = {}\n",
            toml_quote(&reference.display().to_string()),
            fit.n,
            fit.pearson_r,
            fit.slope,
            fit.intercept,
            fit.median_ratio
        ));
    }
    // The effective per-technology metadata, overrides surfaced (same
    // cannot-hide rule as the reliability field).
    report.push_str("\n[results.technologies]\n");
    for zone in &scenario.zones {
        for entry in &zone.fleet {
            let h = entry
                .effective_inertia_h()
                .map_or("0 (non-synchronous)".to_owned(), |h| {
                    h.as_seconds().to_string()
                });
            let overridden = entry.inertia_overridden() || entry.synchronous_overridden();
            report.push_str(&format!(
                "{} = {}\n",
                entry.technology,
                toml_quote(&format!(
                    "synchronous={} H={h}{}",
                    entry.effective_synchronous(),
                    if overridden { " OVERRIDDEN" } else { "" }
                ))
            ));
        }
    }
    let report_path = args.out.join("report.toml");
    std::fs::write(&report_path, report)
        .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;

    // Console summary.
    println!(
        "inertia series: {} periods (model dispatch, unconstrained by any inertia product)",
        series.len()
    );
    println!(
        "  min {:.2} GVA·s at {}; mean {mean:.2}; max {max:.2}",
        min.as_gigavolt_ampere_seconds(),
        result.timestamp_at(min_index)
    );
    for floor in &floors {
        let below =
            grid_stability::periods_below(&series, Inertia::gigavolt_ampere_seconds(*floor));
        println!(
            "  below {floor} GVA·s: {below} periods = {} hours over the horizon",
            below as f64 * 0.5
        );
    }
    if !synchronous_provision {
        println!(
            "  FINDING: this fleet has NO synchronous plant or storage — an all-variable \
             fleet has zero system inertia at every hour; without synthetic provision the \
             stability question is not \"how much margin\" but \"undefined\"."
        );
    }
    if let Some((_, fit)) = &neso_fit {
        println!(
            "  vs NESO outturn: n={} pearson_r={:.6} slope={:.6} intercept={:.6} \
             median_ratio={:.6}",
            fit.n, fit.pearson_r, fit.slope, fit.intercept, fit.median_ratio
        );
    }
    println!("  result digest sha256 = {digest}");
    println!("  outputs in {}", args.out.display());
    Ok(())
}

/// Task 9 (Stage 6 NESO enrichment): correlate this run's own
/// dispatch-derived `inertia_series` against the NESO System Inertia
/// outturn parquet (`--reference`), inner-joined by UTC instant — each
/// engine period `t`'s instant is `result.timestamp_at(t)`
/// (`result.start.plus_periods(t)`, the same instant
/// `grid-adequacy::dispatch` uses to build the run), not assumed
/// positional alignment. This is the engine-level counterpart to
/// `stability validate-inertia`'s method-level (bottom-up-from-
/// generation) check: unlike that method, this is the actual scenario
/// dispatch result, which is cost-optimised rather than a
/// reconstruction of actual unit commitment, so a lower correlation
/// than the method-level check is an expected, honest finding.
fn engine_vs_neso_fit(
    result: &RunResult,
    series: &[Inertia],
    reference: &Path,
) -> Result<grid_stability::Fit, String> {
    let neso_table = crate::fetchdata::table::Table::read_parquet(reference)
        .map_err(|e| format!("{}: {e}", reference.display()))?;
    let crate::fetchdata::table::Column::Float64(outturn) = neso_table
        .column("outturn_inertia_gva_s")
        .ok_or_else(|| format!("{}: no outturn_inertia_gva_s column", reference.display()))?
    else {
        return Err(format!(
            "{}: outturn_inertia_gva_s column is not float64",
            reference.display()
        ));
    };
    let neso_positions: std::collections::BTreeMap<UtcInstant, usize> = neso_table
        .index
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, i))
        .collect();

    let mut ours = Vec::new();
    let mut neso = Vec::new();
    for (t, &value) in series.iter().enumerate() {
        let instant = result.timestamp_at(t);
        if let Some(&pos) = neso_positions.get(&instant) {
            ours.push(value);
            neso.push(Inertia::gigavolt_ampere_seconds(outturn[pos]));
        }
    }
    grid_stability::correlate(&ours, &neso).map_err(|e| e.to_string())
}

fn write_inertia_parquet(
    path: &Path,
    result: &RunResult,
    series: &[Inertia],
    metadata_pairs: &[(&str, String)],
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let schema = Arc::new(Schema::new(vec![
        Field::new("utc_start", ts_type, false),
        Field::new("inertia_gva_s", DataType::Float64, false),
    ]));
    let mut stamps = TimestampMicrosecondBuilder::new();
    let mut values = Float64Builder::new();
    for (t, value) in series.iter().enumerate() {
        stamps.append_value(result.timestamp_at(t).unix_micros());
        values.append_value(value.as_gigavolt_ampere_seconds());
    }
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(stamps.finish().with_timezone("UTC")),
        Arc::new(values.finish()),
    ];
    let batch = RecordBatch::try_new(schema.clone(), arrays).map_err(|e| err(&e))?;
    let kv: Vec<parquet::file::metadata::KeyValue> = metadata_pairs
        .iter()
        .map(|(key, value)| {
            parquet::file::metadata::KeyValue::new((*key).to_owned(), value.clone())
        })
        .collect();
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

// ---------------------------------------------------------------------
// stability pathway (Stage 6 part 2, Q8)
// ---------------------------------------------------------------------

/// The Q8 pathway runner: parse the pathway spec, run every year ×
/// dispatch condition (`grid_stability::pathway` — all modelling
/// conventions documented there), and write the docs/06 artefact set:
/// `pathway.csv` + `pathway.parquet` (one row per year × condition),
/// `report.toml` (assumptions with their defaulted-flags, the band
/// caveat, crossing years per reference standard) and `pathway.png`
/// (largest survivable loss vs year, one line per condition, the
/// secured-loss standards as reference lines).
fn pathway(args: &PathwayArgs) -> Result<(), String> {
    let spec = PathwaySpec::load(&args.pathway).map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    let points = run_pathway(&spec).map_err(|e| e.to_string())?;
    let elapsed = started.elapsed().as_secs_f64();

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // docs/06 metadata block for every output.
    let spec_sha256 = sha256_file(&args.pathway)?;
    let created = now_utc();
    let metadata_pairs: Vec<(&str, String)> = vec![
        ("engine_git_hash", env!("GRID_ENGINE_GIT_HASH").to_owned()),
        (
            "pathway_spec_schema",
            grid_stability::PATHWAY_SCHEMA.to_owned(),
        ),
        ("pathway_spec_path", args.pathway.display().to_string()),
        ("pathway_spec_sha256", spec_sha256),
        ("fes_edition", spec.fes_edition.clone()),
        ("created_utc", created),
    ];
    let mut metadata_block = String::from("# grid-sim output (docs/06 metadata header)\n");
    for (key, value) in &metadata_pairs {
        metadata_block.push_str(&format!("# {key} = {value}\n"));
    }

    // CSV: the data section is the deterministic digest input (same
    // discipline as `run` and `stability event`).
    let mut data_section = String::from(
        "year,condition,synchronous_dispatch_fraction,inertia_gva_s,demand_gw,\
         demand_from_year,largest_survivable_loss_mw,bracket_saturated,zero_inertia\n",
    );
    for point in &points {
        data_section.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            point.year,
            point.condition,
            point.fraction.value(),
            point.inertia.as_gigavolt_ampere_seconds(),
            point.demand.as_gigawatts(),
            point.demand_from_year,
            point.survivable.largest_survivable_loss.as_gigawatts() * 1000.0,
            point.survivable.bracket_saturated,
            point.survivable.zero_inertia,
        ));
    }
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
    let csv_path = args.out.join("pathway.csv");
    std::fs::write(&csv_path, format!("{metadata_block}{data_section}"))
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // Parquet (docs/06: both, always).
    let meta_owned: Vec<(String, String)> = metadata_pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    let f64_column =
        |values: Vec<f64>| -> ArrayRef { Arc::new(arrow_array::Float64Array::from(values)) };
    let fields = vec![
        Field::new("year", DataType::Int64, false),
        Field::new("condition", DataType::Utf8, false),
        Field::new("synchronous_dispatch_fraction", DataType::Float64, false),
        Field::new("inertia_gva_s", DataType::Float64, false),
        Field::new("demand_gw", DataType::Float64, false),
        Field::new("demand_from_year", DataType::Boolean, false),
        Field::new("largest_survivable_loss_mw", DataType::Float64, false),
        Field::new("bracket_saturated", DataType::Boolean, false),
        Field::new("zero_inertia", DataType::Boolean, false),
    ];
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(arrow_array::Int64Array::from(
            points.iter().map(|p| i64::from(p.year)).collect::<Vec<_>>(),
        )),
        Arc::new(arrow_array::StringArray::from(
            points
                .iter()
                .map(|p| p.condition.clone())
                .collect::<Vec<_>>(),
        )),
        f64_column(points.iter().map(|p| p.fraction.value()).collect()),
        f64_column(
            points
                .iter()
                .map(|p| p.inertia.as_gigavolt_ampere_seconds())
                .collect(),
        ),
        f64_column(points.iter().map(|p| p.demand.as_gigawatts()).collect()),
        Arc::new(arrow_array::BooleanArray::from(
            points
                .iter()
                .map(|p| p.demand_from_year)
                .collect::<Vec<_>>(),
        )),
        f64_column(
            points
                .iter()
                .map(|p| p.survivable.largest_survivable_loss.as_gigawatts() * 1000.0)
                .collect(),
        ),
        Arc::new(arrow_array::BooleanArray::from(
            points
                .iter()
                .map(|p| p.survivable.bracket_saturated)
                .collect::<Vec<_>>(),
        )),
        Arc::new(arrow_array::BooleanArray::from(
            points
                .iter()
                .map(|p| p.survivable.zero_inertia)
                .collect::<Vec<_>>(),
        )),
    ];
    crate::sweep::write_table_parquet(
        &args.out.join("pathway.parquet"),
        fields,
        arrays,
        &meta_owned,
        &std::collections::BTreeMap::new(),
    )?;

    // report.toml.
    let a = &spec.assumptions;
    let mut report = String::from("# grid-cli stability pathway report (docs/06)\n[metadata]\n");
    for (key, value) in &metadata_pairs {
        report.push_str(&format!("{key} = {}\n", toml_quote(value)));
    }
    report.push_str(&format!("pathway_name = {}\n", toml_quote(&spec.name)));
    report.push_str(&format!(
        "\n[assumptions]\n\
         # Era assumptions are SPEC INPUTS (grid_stability::pathway docs §2); any\n\
         # *_defaulted = true below means the cited 2019 value was used and the\n\
         # result must be quoted with that stated.\n\
         f0_hz = {}\n\
         demand_fallback_gw = {}\ndemand_fallback_defaulted = {}\n\
         load_damping_percent_per_hz = {}\nload_damping_defaulted = {}\n\
         duration_s = {}\ntimestep_s = {}\nsurvival_floor_hz = {}\n\
         search_max_loss_mw = {}\nsearch_tolerance_mw = {}\n\
         responses_defaulted_to_2019 = {}\ndispatch_conditions_defaulted = {}\n",
        a.f0.as_hertz(),
        a.demand_fallback.as_gigawatts(),
        a.demand_fallback_defaulted,
        a.load_damping.as_percent_of_demand_per_hertz(),
        a.load_damping_defaulted,
        a.duration.as_seconds(),
        a.timestep.as_seconds(),
        a.survival_floor.as_hertz(),
        a.search_max_loss.as_gigawatts() * 1000.0,
        a.search_tolerance.as_gigawatts() * 1000.0,
        a.responses_defaulted_to_2019,
        a.dispatch_conditions_defaulted,
    ));
    for service in &a.responses {
        report.push_str(&format!(
            "\n[[assumptions.responses]]\nname = {}\nheld_mw = {}\ndelivery_factor = {}\n",
            toml_quote(&service.name),
            service.power.as_gigawatts() * 1000.0,
            service.delivery_factor.value(),
        ));
    }
    for condition in &a.dispatch_conditions {
        report.push_str(&format!(
            "\n[[assumptions.dispatch_conditions]]\nname = {}\nsynchronous_dispatch_fraction = {}\n",
            toml_quote(&condition.name),
            condition.synchronous_dispatch_fraction.value(),
        ));
    }
    for reference in &a.reference_losses {
        report.push_str(&format!(
            "\n[[assumptions.reference_losses]]\nname = {}\nmw = {}\n",
            toml_quote(&reference.name),
            reference.loss.as_gigawatts() * 1000.0,
        ));
    }
    report.push_str(&format!(
        "\n[results]\nresult_digest_sha256 = {}\n",
        toml_quote(&digest)
    ));
    report.push_str(
        "# DISPATCH-CONDITION BAND, not a prediction: each line fixes a stated\n\
         # synchronous dispatch fraction (grid_stability::pathway docs §1). The\n\
         # market-only lower edge of the band — part 1's UNCONSTRAINED dispatch,\n\
         # no must-run, no NESO stability actions — reaches ZERO synchronous\n\
         # inertia, at which no loss of any size is survivable. Real operation\n\
         # holds more inertia than the market alone provides because NESO pays\n\
         # for it; what that costs is the Stage 7 question.\n",
    );
    // Crossing years: per condition × reference standard, in BOTH
    // directions. `first_year_below` is "the year the grid can no
    // longer ride through the standard's loss"; `first_year_at_or_above`
    // is the recovery reading — needed because a pathway's survivable
    // loss can RISE (the FES 2025 HT result: demand growth scales the
    // damping base while the synchronous fleet holds roughly level, so
    // the readable date is the year the standard is first met).
    for condition in &a.dispatch_conditions {
        for reference in &a.reference_losses {
            let year_where = |predicate: &dyn Fn(f64) -> bool| {
                points
                    .iter()
                    .filter(|p| p.condition == condition.name)
                    .find(|p| predicate(p.survivable.largest_survivable_loss.as_gigawatts()))
                    .map_or_else(|| "\"never\"".to_owned(), |p| p.year.to_string())
            };
            let reference_gw = reference.loss.as_gigawatts();
            report.push_str(&format!(
                "\n[[results.crossings]]\ncondition = {}\nreference = {}\nreference_mw = {}\n\
                 first_year_below = {}\nfirst_year_at_or_above = {}\n",
                toml_quote(&condition.name),
                toml_quote(&reference.name),
                reference_gw * 1000.0,
                year_where(&|loss| loss < reference_gw),
                year_where(&|loss| loss >= reference_gw),
            ));
        }
    }
    for point in &points {
        report.push_str(&format!(
            "\n[[results.points]]\nyear = {}\ncondition = {}\ninertia_gva_s = {}\n\
             largest_survivable_loss_mw = {}\nbracket_saturated = {}\nzero_inertia = {}\n",
            point.year,
            toml_quote(&point.condition),
            point.inertia.as_gigavolt_ampere_seconds(),
            point.survivable.largest_survivable_loss.as_gigawatts() * 1000.0,
            point.survivable.bracket_saturated,
            point.survivable.zero_inertia,
        ));
    }
    let report_path = args.out.join("report.toml");
    std::fs::write(&report_path, report)
        .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;

    // Chart.
    render_pathway_chart(
        &args.out.join("pathway.png"),
        &spec,
        &points,
        &metadata_pairs,
    )?;

    // Console summary.
    println!(
        "pathway: {} ({} years × {} dispatch conditions, {elapsed:.2} s)",
        spec.name,
        spec.years.len(),
        a.dispatch_conditions.len(),
    );
    if a.responses_defaulted_to_2019 || a.load_damping_defaulted {
        println!(
            "  NOTE: era response/damping assumptions defaulted to the cited 2019 values \
             (flagged in report.toml) — future response holdings are a scenario question"
        );
    }
    for point in &points {
        if point.survivable.zero_inertia {
            println!(
                "  {} [{}]: FINDING — zero synchronous inertia; no loss of any size is \
                 survivable without synchronous or explicitly modelled synthetic provision",
                point.year, point.condition,
            );
        } else {
            println!(
                "  {} [{}]: inertia {:.1} GVA·s, largest survivable loss {:.0} MW{}",
                point.year,
                point.condition,
                point.inertia.as_gigavolt_ampere_seconds(),
                point.survivable.largest_survivable_loss.as_gigawatts() * 1000.0,
                if point.survivable.bracket_saturated {
                    " (>= search bracket top)"
                } else {
                    ""
                },
            );
        }
    }
    println!("  result digest sha256 = {digest}");
    println!("  outputs in {}", args.out.display());
    Ok(())
}

/// The Q8 chart: largest survivable loss vs year, one line per
/// dispatch condition, the secured-loss standards as reference lines.
fn render_pathway_chart(
    out: &Path,
    spec: &PathwaySpec,
    points: &[grid_stability::PathwayPoint],
    metadata_pairs: &[(&str, String)],
) -> Result<(), String> {
    let a = &spec.assumptions;
    let years: Vec<i32> = spec.years.iter().map(|y| y.year).collect();
    let x_min = f64::from(*years.first().ok_or("empty pathway")?);
    let x_max = f64::from(*years.last().ok_or("empty pathway")?);
    let x_pad = ((x_max - x_min) * 0.05).max(0.5);
    let y_max = points
        .iter()
        .map(|p| p.survivable.largest_survivable_loss.as_gigawatts() * 1000.0)
        .chain(
            a.reference_losses
                .iter()
                .map(|r| r.loss.as_gigawatts() * 1000.0),
        )
        .fold(0.0_f64, f64::max)
        * 1.15
        + 50.0;

    let root = BitMapBackend::new(out, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(810);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            format!(
                "Q8 — largest survivable loss-of-infeed vs year ({}, {})",
                spec.name, spec.fes_edition
            ),
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(45)
        .y_label_area_size(80)
        .build_cartesian_2d((x_min - x_pad)..(x_max + x_pad), 0.0..y_max)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc("pathway year")
        .y_desc("largest survivable loss, MW")
        .x_label_formatter(&|x| format!("{x:.0}"))
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    // Secured-loss reference lines: where a condition's line crosses
    // below, that year the grid can no longer ride through the
    // standard's loss.
    for (index, reference) in a.reference_losses.iter().enumerate() {
        let mw = reference.loss.as_gigawatts() * 1000.0;
        let style = if index == 0 {
            BLACK.stroke_width(2)
        } else {
            BLACK.mix(0.5).stroke_width(1)
        };
        let label = format!("{} ({mw:.0} MW)", reference.name);
        chart
            .draw_series(LineSeries::new(
                [(x_min - x_pad, mw), (x_max + x_pad, mw)],
                style,
            ))
            .map_err(|e| e.to_string())?
            .label(label)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 18, y)], style));
    }

    let palette = [
        RGBColor(0, 110, 160),
        RGBColor(200, 60, 40),
        RGBColor(60, 140, 60),
        RGBColor(150, 90, 180),
    ];
    for (index, condition) in a.dispatch_conditions.iter().enumerate() {
        let colour = palette[index % palette.len()];
        let series: Vec<(f64, f64)> = points
            .iter()
            .filter(|p| p.condition == condition.name)
            .map(|p| {
                (
                    f64::from(p.year),
                    p.survivable.largest_survivable_loss.as_gigawatts() * 1000.0,
                )
            })
            .collect();
        chart
            .draw_series(LineSeries::new(
                series.iter().copied(),
                colour.stroke_width(3),
            ))
            .map_err(|e| e.to_string())?
            .label(format!(
                "condition {} (φ = {})",
                condition.name,
                condition.synchronous_dispatch_fraction.value()
            ))
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 18, y)], colour.stroke_width(3))
            });
        chart
            .draw_series(
                series
                    .iter()
                    .map(|&(x, y)| Circle::new((x, y), 5, colour.filled())),
            )
            .map_err(|e| e.to_string())?;
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let lookup = |key: &str| {
        metadata_pairs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or("?")
    };
    let spec_hash = lookup("pathway_spec_sha256");
    let line1 = format!(
        "grid-sim | engine {} | pathway spec sha256 {} | generated {}",
        lookup("engine_git_hash"),
        &spec_hash[..12.min(spec_hash.len())],
        lookup("created_utc"),
    );
    let line2 = format!(
        "dispatch-condition BAND (stated fractions, not predictions); market-only lower edge \
         is zero inertia{}",
        if spec.assumptions.responses_defaulted_to_2019 {
            "; response holdings defaulted to 2019 values"
        } else {
            ""
        },
    );
    for (line, y) in [(line1, 15), (line2, 40)] {
        footer
            .draw(&Text::new(
                line,
                (20, y),
                ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
            ))
            .map_err(|e| e.to_string())?;
    }
    root.present().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// Module 6 sweep: hours below the inertia floors vs renewable share.
// ---------------------------------------------------------------------

/// One Module 6 sweep point.
struct Module6Point {
    scale: f64,
    renewable_potential: grid_core::units::Energy,
    demand: grid_core::units::Energy,
    share_potential: f64,
    share_delivered: f64,
    min_inertia: Inertia,
    below: Vec<usize>, // one count per floor
}

/// The Module 6 sweep (docs/04 Stage 6 demo artefact): sweep wind +
/// solar capacity over the given scale factors, count the periods
/// below each inertia floor per point, and chart hours-below vs
/// renewable share. Conventions (documented in the module header and
/// in the artefact itself): wind + solar scale together from the
/// scenario's split; everything else held; "renewable share" is
/// potential (pre-curtailment) wind + solar energy over underlying
/// demand energy (D3 denominator); delivered share is
/// (potential − pooled curtailment) / demand. The dispatch remains
/// UNCONSTRAINED by any inertia product — the caveat carries into
/// report, chart footer and console.
fn module6(args: &InertiaArgs) -> Result<(), String> {
    for &scale in &args.renewable_scales {
        if !(scale.is_finite() && scale > 0.0) {
            return Err(format!(
                "--renewable-scale {scale} must be positive and finite"
            ));
        }
    }
    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    // Traces are loaded once against the reference scenario; only fleet
    // capacities change per point (the Stage 2 sweep convention — the
    // dispatch engine reads capacities from the scaled scenario).
    let inputs = load_run_inputs(&scenario, &args.base_dir).map_err(|e| e.to_string())?;
    let table = InertiaTable::from_scenario(&scenario).map_err(|e| e.to_string())?;
    let floors = if args.floors.is_empty() {
        vec![120.0, 102.0]
    } else {
        args.floors.clone()
    };
    let scaled_techs = ["onshore_wind", "offshore_wind", "solar"];

    let started = std::time::Instant::now();
    let mut points: Vec<Module6Point> = Vec::with_capacity(args.renewable_scales.len());
    for &scale in &args.renewable_scales {
        let mut scaled = scenario.clone();
        for entry in &mut scaled.zones[0].fleet {
            if scaled_techs.contains(&entry.technology.as_str()) {
                entry.capacity_gw = entry.capacity_gw * scale;
            }
        }
        let result = run(&scaled, &inputs).map_err(|e| e.to_string())?;
        let series = inertia_series(&result, &table).map_err(|e| e.to_string())?;
        let renewable_potential = result
            .renewables
            .iter()
            .filter(|s| scaled_techs.contains(&s.tech.as_str()))
            .map(|s| RunResult::total_energy(&s.power))
            .fold(grid_core::units::Energy::gigawatt_hours(0.0), |acc, e| {
                acc + e
            });
        let demand = RunResult::total_energy(&result.demand);
        let curtailment = result.total_curtailment();
        let (_, min) = grid_stability::min_inertia(&series).ok_or("empty inertia series")?;
        let below: Vec<usize> = floors
            .iter()
            .map(|&floor| {
                grid_stability::periods_below(&series, Inertia::gigavolt_ampere_seconds(floor))
            })
            .collect();
        points.push(Module6Point {
            scale,
            renewable_potential,
            demand,
            share_potential: renewable_potential.as_gigawatt_hours() / demand.as_gigawatt_hours(),
            share_delivered: (renewable_potential.as_gigawatt_hours()
                - curtailment.as_gigawatt_hours())
                / demand.as_gigawatt_hours(),
            min_inertia: min,
            below,
        });
    }
    let elapsed = started.elapsed().as_secs_f64();

    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;

    // docs/06 metadata.
    let created = now_utc();
    let metadata_pairs: Vec<(&str, String)> = vec![
        ("engine_git_hash", env!("GRID_ENGINE_GIT_HASH").to_owned()),
        ("schema_version", SCHEMA_VERSION.to_string()),
        ("scenario_path", args.scenario.display().to_string()),
        ("scenario_sha256", sha256_file(&args.scenario)?),
        ("created_utc", created),
    ];
    let data_files = scenario_data_files(&scenario, &args.base_dir)?;
    let mut metadata_block = String::from("# grid-sim output (docs/06 metadata header)\n");
    for (key, value) in &metadata_pairs {
        metadata_block.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &data_files {
        metadata_block.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    metadata_block.push_str(
        "# convention: wind + solar capacities scaled together from the scenario split;\n\
         # everything else held at scenario values. renewable_share_potential =\n\
         # potential (pre-curtailment) wind+solar energy / underlying demand energy\n\
         # (D3 denominator); renewable_share_delivered subtracts POOLED curtailment\n\
         # (Stage 1 convention). Dispatch is UNCONSTRAINED by any inertia product.\n",
    );

    // CSV (data section = digest input).
    let mut data_section = String::from(
        "renewable_scale,renewable_potential_twh,demand_twh,renewable_share_potential,renewable_share_delivered,min_inertia_gva_s",
    );
    for floor in &floors {
        data_section.push_str(&format!(",periods_below_{floor},hours_below_{floor}"));
    }
    data_section.push('\n');
    for point in &points {
        data_section.push_str(&format!(
            "{},{},{},{},{},{}",
            point.scale,
            point.renewable_potential.as_gigawatt_hours() / 1000.0,
            point.demand.as_gigawatt_hours() / 1000.0,
            point.share_potential,
            point.share_delivered,
            point.min_inertia.as_gigavolt_ampere_seconds(),
        ));
        for &below in &point.below {
            data_section.push_str(&format!(",{below},{}", below as f64 * 0.5));
        }
        data_section.push('\n');
    }
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
    let csv_path = args.out.join("module6_hours_below_vs_share.csv");
    std::fs::write(&csv_path, format!("{metadata_block}{data_section}"))
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // Parquet.
    let meta_owned: Vec<(String, String)> = metadata_pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    let f64_column =
        |values: Vec<f64>| -> ArrayRef { Arc::new(arrow_array::Float64Array::from(values)) };
    let mut fields = vec![
        Field::new("renewable_scale", DataType::Float64, false),
        Field::new("renewable_potential_twh", DataType::Float64, false),
        Field::new("demand_twh", DataType::Float64, false),
        Field::new("renewable_share_potential", DataType::Float64, false),
        Field::new("renewable_share_delivered", DataType::Float64, false),
        Field::new("min_inertia_gva_s", DataType::Float64, false),
    ];
    let mut arrays: Vec<ArrayRef> = vec![
        f64_column(points.iter().map(|p| p.scale).collect()),
        f64_column(
            points
                .iter()
                .map(|p| p.renewable_potential.as_gigawatt_hours() / 1000.0)
                .collect(),
        ),
        f64_column(
            points
                .iter()
                .map(|p| p.demand.as_gigawatt_hours() / 1000.0)
                .collect(),
        ),
        f64_column(points.iter().map(|p| p.share_potential).collect()),
        f64_column(points.iter().map(|p| p.share_delivered).collect()),
        f64_column(
            points
                .iter()
                .map(|p| p.min_inertia.as_gigavolt_ampere_seconds())
                .collect(),
        ),
    ];
    for (index, floor) in floors.iter().enumerate() {
        fields.push(Field::new(
            format!("periods_below_{floor}"),
            DataType::Float64,
            false,
        ));
        arrays.push(f64_column(
            points.iter().map(|p| p.below[index] as f64).collect(),
        ));
        fields.push(Field::new(
            format!("hours_below_{floor}"),
            DataType::Float64,
            false,
        ));
        arrays.push(f64_column(
            points.iter().map(|p| p.below[index] as f64 * 0.5).collect(),
        ));
    }
    crate::sweep::write_table_parquet(
        &args.out.join("module6_hours_below_vs_share.parquet"),
        fields,
        arrays,
        &meta_owned,
        &data_files,
    )?;

    // report.toml, with the part-1 caveat block (three-way pinning:
    // report + chart footer + console; the CLI test asserts it).
    let mut report = String::from(
        "# grid-cli stability inertia --renewable-scale report (Module 6, docs/06)\n[metadata]\n",
    );
    for (key, value) in &metadata_pairs {
        let value = if *key == "schema_version" {
            value.clone()
        } else {
            toml_quote(value)
        };
        report.push_str(&format!("{key} = {value}\n"));
    }
    report.push_str("\n[metadata.data_files]\n");
    for (path, hash) in &data_files {
        report.push_str(&format!("{} = {}\n", toml_quote(path), toml_quote(hash)));
    }
    report.push_str(&format!(
        "\n[results]\npoints = {}\nresult_digest_sha256 = {}\nsweep_seconds = {elapsed}\n",
        points.len(),
        toml_quote(&digest)
    ));
    report.push_str(
        "# MODEL-dispatch inertia, UNCONSTRAINED by any inertia product: the adequacy\n\
         # engine has no must-run floor and no NESO stability actions, so the low tail\n\
         # reads lower than the operational record. The gap below the floors is the\n\
         # Module 6 finding (what the energy market alone would provide), not a claim\n\
         # about operated GB. Do not quote hours-below counts without this caveat.\n",
    );
    for point in &points {
        report.push_str(&format!(
            "\n[[results.points]]\nrenewable_scale = {}\nrenewable_share_potential = {}\n\
             renewable_share_delivered = {}\nmin_inertia_gva_s = {}\n",
            point.scale,
            point.share_potential,
            point.share_delivered,
            point.min_inertia.as_gigavolt_ampere_seconds(),
        ));
        for (floor, &below) in floors.iter().zip(&point.below) {
            report.push_str(&format!(
                "\n[[results.points.floors]]\nfloor_gva_s = {floor}\nperiods_below = {below}\n\
                 hours_below = {}\n",
                below as f64 * 0.5
            ));
        }
    }
    let report_path = args.out.join("report.toml");
    std::fs::write(&report_path, report)
        .map_err(|e| format!("cannot write {}: {e}", report_path.display()))?;

    // Chart.
    render_module6_chart(
        &args.out.join("module6_hours_below_vs_share.png"),
        &points,
        &floors,
        &metadata_pairs,
    )?;

    // Console summary (the caveat is the headline, not a footnote).
    println!(
        "module 6 sweep: {} points in {elapsed:.1} s (model dispatch, unconstrained by any \
         inertia product — do not quote without the caveat)",
        points.len()
    );
    for point in &points {
        let below: Vec<String> = floors
            .iter()
            .zip(&point.below)
            .map(|(floor, &b)| format!("below {floor}: {b} periods = {} h", b as f64 * 0.5))
            .collect();
        println!(
            "  scale {:>4}: share (potential) {:5.1} %, {}",
            point.scale,
            point.share_potential * 100.0,
            below.join(", "),
        );
    }
    println!("  result digest sha256 = {digest}");
    println!("  outputs in {}", args.out.display());
    Ok(())
}

/// The Module 6 chart: hours/year below each inertia floor vs
/// renewable share (potential wind + solar energy over underlying
/// demand — the D3 convention), with the UNCONSTRAINED caveat in the
/// footer.
fn render_module6_chart(
    out: &Path,
    points: &[Module6Point],
    floors: &[f64],
    metadata_pairs: &[(&str, String)],
) -> Result<(), String> {
    // Sort by share for a clean line (CSV keeps the given order).
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&i, &j| {
        points[i]
            .share_potential
            .total_cmp(&points[j].share_potential)
    });
    let x_min = points
        .iter()
        .map(|p| p.share_potential)
        .fold(f64::INFINITY, f64::min)
        * 100.0;
    let x_max = points
        .iter()
        .map(|p| p.share_potential)
        .fold(0.0_f64, f64::max)
        * 100.0;
    let x_pad = ((x_max - x_min) * 0.05).max(1.0);
    let y_max = points
        .iter()
        .flat_map(|p| p.below.iter())
        .map(|&b| b as f64 * 0.5)
        .fold(0.0_f64, f64::max)
        * 1.1
        + 10.0;

    let root = BitMapBackend::new(out, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(810);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            "Module 6 — hours/year below the inertia floors vs renewable share \
             (market-only dispatch, UNCONSTRAINED by any inertia product)",
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(80)
        .build_cartesian_2d((x_min - x_pad)..(x_max + x_pad), 0.0..y_max)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc("renewable share, % (potential wind + solar energy / underlying demand)")
        .y_desc("hours below floor (one-year horizon)")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let palette = [RGBColor(0, 110, 160), RGBColor(200, 60, 40)];
    for (index, floor) in floors.iter().enumerate() {
        let colour = palette[index % palette.len()];
        let series: Vec<(f64, f64)> = order
            .iter()
            .map(|&i| {
                (
                    points[i].share_potential * 100.0,
                    points[i].below[index] as f64 * 0.5,
                )
            })
            .collect();
        chart
            .draw_series(LineSeries::new(
                series.iter().copied(),
                colour.stroke_width(3),
            ))
            .map_err(|e| e.to_string())?
            .label(format!("hours below {floor} GVA·s"))
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 18, y)], colour.stroke_width(3))
            });
        chart
            .draw_series(
                series
                    .iter()
                    .map(|&(x, y)| Circle::new((x, y), 5, colour.filled())),
            )
            .map_err(|e| e.to_string())?;
    }
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::LowerRight)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    let lookup = |key: &str| {
        metadata_pairs
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or("?")
    };
    let scenario_hash = lookup("scenario_sha256");
    let line1 = format!(
        "grid-sim | engine {} | scenario sha256 {} | generated {}",
        lookup("engine_git_hash"),
        &scenario_hash[..12.min(scenario_hash.len())],
        lookup("created_utc"),
    );
    let line2 = "UNCONSTRAINED market-only dispatch: no must-run, no NESO stability actions — \
                 the gap below the floors is the finding, not operated GB"
        .to_owned();
    for (line, y) in [(line1, 15), (line2, 40)] {
        footer
            .draw(&Text::new(
                line,
                (20, y),
                ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
            ))
            .map_err(|e| e.to_string())?;
    }
    root.present().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------
// stability validate-inertia (Stage 6 NESO enrichment, Task 7)
// ---------------------------------------------------------------------

/// Translate a `generation_by_fuel` pack column name to the grid_core tech
/// id `grid_stability::inertia_from_generation` should see, or `None` if
/// the column is deliberately omitted from the bottom-up estimate.
///
/// - `npshyd` (FUELHH's non-pumped hydro) MUST become `hydro`, or it
///   silently contributes 0 under `grid_core::inertia::technology_default`.
/// - `ps` (pumped storage) is OMITTED: it is synchronous only via
///   `grid_core::inertia::storage_kind_default`, which
///   `inertia_from_generation` does not reach. Passing it through
///   untranslated would contribute the same 0 while hiding that gap, so
///   it is dropped and called out explicitly in the run report as a
///   known parity gap against the engine's own `system_inertia` (which
///   DOES count PS).
/// - Everything else (`ccgt`, `ocgt`, `nuclear`, `coal`, `biomass` already
///   match; `oil` has no grid_core arm; `wind`, `other` and the
///   interconnector columns are non-synchronous) passes through
///   unchanged and contributes 0 honestly via the catch-all default.
fn translate_fuel_column(name: &str) -> Option<String> {
    match name {
        "ps" => None,
        "npshyd" => Some("hydro".to_owned()),
        other => Some(other.to_owned()),
    }
}

/// `stability validate-inertia`: correlate the bottom-up
/// `grid_stability::inertia_from_generation` estimate (computed from the
/// 2024 pack's `generation_by_fuel` table) against the NESO System
/// Inertia outturn series (`inertia_outturn_2024`), inner-joined on their
/// `UtcInstant` index so only periods present in both tables are
/// compared. Reads pack tables directly (no scenario, no dispatch run).
fn validate_inertia(args: &ValidateInertiaArgs) -> Result<(), String> {
    let generation_path = args
        .base_dir
        .join("data/packs/2024/processed/generation_by_fuel_2024.parquet");
    let inertia_path = args
        .base_dir
        .join("data/packs/2024/processed/inertia_outturn_2024.parquet");
    let generation = crate::fetchdata::table::Table::read_parquet(&generation_path)
        .map_err(|e| format!("{}: {e}", generation_path.display()))?;
    let neso_table = crate::fetchdata::table::Table::read_parquet(&inertia_path)
        .map_err(|e| format!("{}: {e}", inertia_path.display()))?;

    // Inner-join on the UtcInstant index: keep only periods present in
    // BOTH tables, in ascending time order (defends against the gap
    // issue rather than assuming the two tables line up row-for-row).
    let gen_positions: std::collections::BTreeMap<UtcInstant, usize> = generation
        .index
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, i))
        .collect();
    let neso_positions: std::collections::BTreeMap<UtcInstant, usize> = neso_table
        .index
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, i))
        .collect();
    let common: Vec<UtcInstant> = gen_positions
        .keys()
        .filter(|t| neso_positions.contains_key(t))
        .copied()
        .collect();

    // Translate + align the generation fuel columns to the joined
    // periods, tracking which fuels contribute (a nonzero H), which are
    // included but contribute zero (non-synchronous), and which are
    // omitted outright (documented gaps).
    let mut contributing: Vec<String> = Vec::new();
    let mut zero_contribution: Vec<String> = Vec::new();
    let mut omitted: Vec<String> = Vec::new();
    let mut fuels: Vec<(String, Vec<f64>)> = Vec::new();
    for (name, column) in &generation.columns {
        let Some(translated) = translate_fuel_column(name) else {
            omitted.push(name.clone());
            continue;
        };
        let crate::fetchdata::table::Column::Float64(values) = column else {
            return Err(format!(
                "generation column {name:?} is not float64 (pack columns are expected float64)"
            ));
        };
        let tech = grid_core::scenario::TechId::new(translated.as_str());
        if grid_core::inertia::technology_default(&tech).h.is_some() {
            contributing.push(format!("{name} -> {translated}"));
        } else {
            zero_contribution.push(format!("{name} -> {translated}"));
        }
        let aligned: Vec<f64> = common.iter().map(|t| values[gen_positions[t]]).collect();
        fuels.push((translated, aligned));
    }

    let crate::fetchdata::table::Column::Float64(outturn) = neso_table
        .column("outturn_inertia_gva_s")
        .ok_or("inertia_outturn table has no outturn_inertia_gva_s column")?
    else {
        return Err("inertia_outturn outturn_inertia_gva_s column is not float64".to_owned());
    };
    let neso: Vec<Inertia> = common
        .iter()
        .map(|t| Inertia::gigavolt_ampere_seconds(outturn[neso_positions[t]]))
        .collect();

    let ours = grid_stability::inertia_from_generation(&fuels).map_err(|e| e.to_string())?;
    let fit = grid_stability::correlate(&ours, &neso).map_err(|e| e.to_string())?;

    if let Some(parent) = args.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }

    let created = now_utc();
    let mut report = String::from(
        "# grid-cli stability validate-inertia report (docs/06)\n\
         # Bottom-up inertia (grid_stability::inertia_from_generation, computed\n\
         # directly from the pack's per-fuel generation MW) vs the NESO System\n\
         # Inertia outturn series, over the 2024 pack, inner-joined on the shared\n\
         # UtcInstant periods. NESO's outturn is NESO's OWN MODEL ESTIMATE (not a\n\
         # measurement), used here as an external reference, not ground truth.\n\
         # Supported by National Energy SO Open Data.\n\
         [metadata]\n",
    );
    report.push_str(&format!(
        "engine_git_hash = {}\n",
        toml_quote(env!("GRID_ENGINE_GIT_HASH"))
    ));
    report.push_str(&format!(
        "generation_path = {}\n",
        toml_quote(&generation_path.display().to_string())
    ));
    report.push_str(&format!(
        "inertia_outturn_path = {}\n",
        toml_quote(&inertia_path.display().to_string())
    ));
    report.push_str(&format!("created_utc = {}\n", toml_quote(&created)));
    report.push_str(&format!(
        "\n[results]\nn = {}\npearson_r = {}\nslope = {}\nintercept = {}\nmedian_ratio = {}\n",
        fit.n, fit.pearson_r, fit.slope, fit.intercept, fit.median_ratio
    ));
    report.push_str("\n[results.fuels]\n");
    report.push_str(&format!(
        "contributing = {}\n",
        toml_quote(&contributing.join(", "))
    ));
    report.push_str(&format!(
        "zero_contribution = {}\n",
        toml_quote(&zero_contribution.join(", "))
    ));
    report.push_str(&format!("omitted = {}\n", toml_quote(&omitted.join(", "))));
    report.push_str(
        "# omitted: pumped storage (ps) is synchronous only via\n\
         # grid_core::inertia::storage_kind_default, which\n\
         # inertia_from_generation does not reach; the engine's own\n\
         # system_inertia DOES count PS, so this bottom-up estimate is\n\
         # asymmetrically low relative to it by that much. oil (in\n\
         # zero_contribution) has no grid_core arm at all. Both are small,\n\
         # documented parity gaps, not silent omissions.\n",
    );

    std::fs::write(&args.out, &report)
        .map_err(|e| format!("cannot write {}: {e}", args.out.display()))?;

    println!(
        "validate-inertia: {} matched periods (of {} generation / {} outturn rows)",
        fit.n,
        generation.len(),
        neso_table.len()
    );
    println!(
        "  pearson_r {:.6}  slope {:.6}  intercept {:.4}  median_ratio {:.6}",
        fit.pearson_r, fit.slope, fit.intercept, fit.median_ratio
    );
    println!("  report written to {}", args.out.display());
    Ok(())
}
