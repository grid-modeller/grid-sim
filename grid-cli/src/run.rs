//! `grid-cli run` — chronological dispatch (docs/04 Stages 1 and 3)
//! plus the Stage 2 pricing layer.
//!
//! Runs a single-zone scenario (schema v2: fully self-contained — the
//! Stage 1 `--inputs` companion file no longer exists) and writes, per
//! docs/06:
//!
//! - `dispatch.csv` and `dispatch.parquet` — per-period dispatch, GW,
//!   including per-store charge/discharge (GW) and end-of-period SoC
//!   (GWh) columns (CSV for humans, Parquet for analysis — both,
//!   always);
//! - `monthly_mix.csv` — per-calendar-month energy per power series,
//!   GWh (the demo-artefact table and `plot monthly-mix` input; SoC is
//!   a state, not a flow, and is not aggregated here);
//! - `summary.toml` — run aggregates (including per-store totals and
//!   min/max SoC) plus the deterministic result digest (SHA-256 over
//!   the CSV data section);
//! - when the scenario declares a `[pricing]` block (Stage 2):
//!   `prices.csv` and `prices.parquet` — per-period system marginal
//!   price, price-setting technology, and per-technology SRMC — plus a
//!   `[results.pricing]` summary block (SMP aggregates, both gas-
//!   marginal framings, per-technology revenue/capture, emissions in
//!   both labelled bases, the reported realism statistics, and a
//!   separate prices digest). Pricing runs *after* dispatch and reads
//!   its output only: the dispatch digest is untouched by pricing.
//!
//! Every file carries the docs/06 metadata block: engine git hash,
//! scenario hash, per-data-file SHA-256 (the data-pack checksum,
//! computed over the files actually read), schema version, and a
//! timestamp — the one permitted wall-clock read, at this CLI layer
//! only (ADR-5).
//!
//! Exit codes (docs/06): 0 on a completed run, 2 on usage/scenario
//! errors. A completed run with unserved energy still exits 0: unserved
//! energy is a *result* of a `run` (reported in the summary); exit 1 is
//! reserved for infeasibility in `solve`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::builder::{Float64Builder, StringBuilder, TimestampMicrosecondBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use clap::Args;
use grid_adequacy::{
    MultiZoneRunResult, PricingInputs, PricingResult, RunResult, load_multi_zone_inputs,
    load_pricing_inputs, load_run_inputs, price_run, run_multi,
};
use grid_core::pricing::{capture_price, capture_ratio, price_setting_share};
use grid_core::scenario::{SCHEMA_VERSION, Scenario};
use grid_core::time::UtcInstant;
use grid_core::units::{Energy, Power};
use sha2::{Digest, Sha256};

/// Arguments to `grid-cli run`.
#[derive(Args)]
pub struct RunArgs {
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
}

/// The docs/06 output-metadata block.
pub(crate) struct Metadata {
    pub(crate) engine_git_hash: &'static str,
    pub(crate) schema_version: u32,
    pub(crate) scenario_path: String,
    pub(crate) scenario_sha256: String,
    pub(crate) created_utc: String,
    /// SHA-256 of every data file the run read (path → hash, sorted).
    pub(crate) data_files: BTreeMap<String, String>,
}

impl Metadata {
    /// Key/value pairs in stable output order (data files separate).
    pub(crate) fn pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("engine_git_hash", self.engine_git_hash.to_owned()),
            ("schema_version", self.schema_version.to_string()),
            ("scenario_path", self.scenario_path.clone()),
            ("scenario_sha256", self.scenario_sha256.clone()),
            ("created_utc", self.created_utc.clone()),
        ]
    }
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot hash {}: {e}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// SHA-256 of every data file a scenario references (relative to
/// `base_dir`): demand (base + extra profiles), CF traces, exogenous
/// supply, energy-budget traces, pricing inputs — across every zone.
pub(crate) fn scenario_data_files(
    scenario: &Scenario,
    base_dir: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let mut data_files = BTreeMap::new();
    let mut add = |rel: &str| -> Result<(), String> {
        let path = base_dir.join(rel);
        data_files.insert(rel.to_owned(), sha256_file(&path)?);
        Ok(())
    };
    for zone in &scenario.zones {
        for path in zone.demand.base_profile.paths() {
            add(path)?;
        }
        for extra in &zone.demand.extra_profiles {
            add(&extra.path)?;
        }
        if let Some(heating) = &zone.demand.heating {
            // The pinned temperature trace and the drift-guarded COP
            // reference file are run inputs like any other (ADR-5).
            add(&heating.temperature_trace.path)?;
            add(grid_core::heating::HEATING_COP_REFERENCE_PATH)?;
        }
        for entry in &zone.fleet {
            if let Some(trace) = &entry.capacity_factor_trace {
                for path in trace.paths() {
                    add(path)?;
                }
            }
            if let Some(budget) = &entry.energy_budget {
                for path in budget.trace.paths() {
                    add(path)?;
                }
            }
        }
        for supply in &zone.exogenous_supply {
            for path in supply.path.paths() {
                add(path)?;
            }
        }
        // Schema v7 (D11): per-zone pricing inputs are read by
        // `load_multi_zone_inputs` whenever declared, so they are run
        // inputs like any other (ADR-5).
        if let Some(pricing) = &zone.pricing {
            add(&pricing.reference)?;
            for trace_ref in pricing.fuel_price.values() {
                add(&trace_ref.path)?;
            }
        }
    }
    // Schema v6: per-link observed capability traces (the B4/B6 DA
    // series) are read by `load_multi_zone_inputs` — run inputs too.
    for link in &scenario.links {
        if let Some(trace) = &link.capability_trace {
            add(&trace.path)?;
        }
    }
    if let Some(pricing) = &scenario.pricing {
        add(&pricing.reference)?;
        for trace_ref in pricing.fuel_price.values() {
            add(&trace_ref.path)?;
        }
        if let Some(observed) = &pricing.observed_price {
            add(&observed.path)?;
        }
    }
    Ok(data_files)
}

/// The one permitted wall-clock read (ADR-5): output timestamps at the
/// CLI layer.
pub(crate) fn now_utc() -> String {
    let micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0);
    // Truncate to whole seconds for the strict RFC 3339 form.
    UtcInstant::from_unix_micros(micros - micros.rem_euclid(1_000_000)).to_string()
}

/// One named per-period output column, GW.
struct Column<'a> {
    label: String,
    power: &'a [Power],
}

/// All power output columns in stable order: demand, renewables
/// (scenario order), exogenous (scenario order), thermal (merit order),
/// per-store charge and discharge (dispatch order), curtailment,
/// unserved. Store SoC (GWh, a state not a flow) is carried separately
/// ([`soc_columns`]).
fn columns(result: &RunResult) -> Vec<Column<'_>> {
    let mut cols = vec![Column {
        label: "demand".to_owned(),
        power: &result.demand,
    }];
    for series in &result.renewables {
        cols.push(Column {
            label: series.tech.as_str().to_owned(),
            power: &series.power,
        });
    }
    for series in &result.exogenous {
        cols.push(Column {
            label: series.label.clone(),
            power: &series.power,
        });
    }
    for series in &result.thermal {
        cols.push(Column {
            label: series.tech.as_str().to_owned(),
            power: &series.power,
        });
    }
    for store in &result.stores {
        cols.push(Column {
            label: format!("{}_charge", store.label),
            power: &store.charge,
        });
        cols.push(Column {
            label: format!("{}_discharge", store.label),
            power: &store.discharge,
        });
    }
    cols.push(Column {
        label: "curtailment".to_owned(),
        power: &result.curtailment,
    });
    cols.push(Column {
        label: "unserved".to_owned(),
        power: &result.unserved,
    });
    cols
}

/// One named per-period state-of-charge column, GWh (appended after the
/// power columns in dispatch.csv/parquet).
struct SocColumn<'a> {
    label: String,
    soc: &'a [Energy],
}

fn soc_columns(result: &RunResult) -> Vec<SocColumn<'_>> {
    result
        .stores
        .iter()
        .map(|store| SocColumn {
            label: format!("{}_soc", store.label),
            soc: &store.soc,
        })
        .collect()
}

fn metadata_comment_block(meta: &Metadata) -> String {
    let mut block = String::from("# grid-sim output (docs/06 metadata header)\n");
    for (key, value) in meta.pairs() {
        block.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &meta.data_files {
        block.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    block
}

/// The CSV data section (header + rows, no metadata comments) — also the
/// input to the deterministic result digest.
fn csv_data_section(result: &RunResult) -> String {
    let cols = columns(result);
    let socs = soc_columns(result);
    let reliability = ReliabilityColumns::of(result);
    let mut text = String::from("utc_start");
    for col in &cols {
        text.push_str(&format!(",{}_gw", col.label));
    }
    for soc in &socs {
        text.push_str(&format!(",{}_gwh", soc.label));
    }
    text.push_str(",firm_supply_gw,variable_supply_gw,storage_discharge_gw,firm_share");
    text.push('\n');
    for t in 0..result.periods() {
        text.push_str(&result.timestamp_at(t).to_string());
        for col in &cols {
            text.push_str(&format!(",{}", col.power[t].as_gigawatts()));
        }
        for soc in &socs {
            text.push_str(&format!(",{}", soc.soc[t].as_gigawatt_hours()));
        }
        text.push_str(&format!(
            ",{},{},{},{}",
            reliability.firm[t].as_gigawatts(),
            reliability.variable[t].as_gigawatts(),
            reliability.storage_discharge[t].as_gigawatts(),
            reliability.firm_share[t],
        ));
        text.push('\n');
    }
    text
}

/// The derived reliability-accounting columns (gb-grid-margin
/// methodology; `grid_adequacy::result`): firm and variable supply,
/// total storage discharge (the fourth category, never folded into
/// firm), and the unclamped firm share of demand.
struct ReliabilityColumns {
    firm: Vec<Power>,
    variable: Vec<Power>,
    storage_discharge: Vec<Power>,
    firm_share: Vec<f64>,
}

impl ReliabilityColumns {
    fn of(result: &RunResult) -> Self {
        Self {
            firm: result.firm_supply(),
            variable: result.variable_supply(),
            storage_discharge: result.storage_discharge(),
            firm_share: result.firm_share(),
        }
    }
}

fn write_dispatch_parquet(path: &Path, result: &RunResult, meta: &Metadata) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let cols = columns(result);
    let socs = soc_columns(result);

    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let mut fields = vec![Field::new("utc_start", ts_type, false)];
    for col in &cols {
        fields.push(Field::new(
            format!("{}_gw", col.label),
            DataType::Float64,
            false,
        ));
    }
    for soc in &socs {
        fields.push(Field::new(
            format!("{}_gwh", soc.label),
            DataType::Float64,
            false,
        ));
    }
    for name in [
        "firm_supply_gw",
        "variable_supply_gw",
        "storage_discharge_gw",
        "firm_share",
    ] {
        fields.push(Field::new(name, DataType::Float64, false));
    }
    let schema = Arc::new(Schema::new(fields));

    let mut stamps = TimestampMicrosecondBuilder::new();
    for t in 0..result.periods() {
        stamps.append_value(result.timestamp_at(t).unix_micros());
    }
    let mut arrays: Vec<ArrayRef> = vec![Arc::new(stamps.finish().with_timezone("UTC"))];
    for col in &cols {
        let mut builder = Float64Builder::new();
        for &p in col.power {
            builder.append_value(p.as_gigawatts());
        }
        arrays.push(Arc::new(builder.finish()));
    }
    for soc in &socs {
        let mut builder = Float64Builder::new();
        for &e in soc.soc {
            builder.append_value(e.as_gigawatt_hours());
        }
        arrays.push(Arc::new(builder.finish()));
    }
    let reliability = ReliabilityColumns::of(result);
    for values in [
        &reliability.firm,
        &reliability.variable,
        &reliability.storage_discharge,
    ] {
        let mut builder = Float64Builder::new();
        for &p in values {
            builder.append_value(p.as_gigawatts());
        }
        arrays.push(Arc::new(builder.finish()));
    }
    {
        let mut builder = Float64Builder::new();
        for &share in &reliability.firm_share {
            builder.append_value(share);
        }
        arrays.push(Arc::new(builder.finish()));
    }
    let batch = RecordBatch::try_new(schema.clone(), arrays).map_err(|e| err(&e))?;

    // The docs/06 metadata block goes into the Parquet footer key-value
    // metadata.
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

fn monthly_mix_csv(result: &RunResult, meta: &Metadata) -> String {
    let cols = columns(result);
    let mut text = metadata_comment_block(meta);
    text.push_str("# per-calendar-month energy, GWh\nmonth");
    for col in &cols {
        text.push_str(&format!(",{}_gwh", col.label));
    }
    text.push('\n');
    let keys: Vec<(i64, u8)> = result
        .monthly_energy(&result.demand)
        .keys()
        .copied()
        .collect();
    let monthly: Vec<BTreeMap<(i64, u8), Energy>> = cols
        .iter()
        .map(|col| result.monthly_energy(col.power))
        .collect();
    for key in keys {
        text.push_str(&format!("{:04}-{:02}", key.0, key.1));
        for series in &monthly {
            text.push_str(&format!(",{}", series[&key].as_gigawatt_hours()));
        }
        text.push('\n');
    }
    text
}

// ---------------------------------------------------------------------
// Q5 heating-overlay outputs (D9 rule 6b: per-period heating electrical
// demand, total and per-entry; per-period delivered heat; the pinned
// constants and any per-entry overrides echoed). Written ONLY when the
// scenario declares a heating block — scenarios without one produce
// byte-identical outputs to pre-v5 (the dispatch digest never moves).
// Residual-load/decomposition machinery sees heating inside demand — no
// special-casing anywhere downstream.
// ---------------------------------------------------------------------

/// The heating CSV data section (header + rows, no metadata comments) —
/// also the input to the heating digest. Columns: `utc_start`,
/// delivered heat (thermal GW), total electrical demand, then one
/// electrical column per portfolio entry (scenario order).
fn heating_csv_data_section(overlay: &grid_core::heating::HeatingOverlay) -> String {
    let mut text = String::from("utc_start,delivered_heat_gw,heating_electrical_total_gw");
    for entry in &overlay.entries {
        text.push_str(&format!(",heating_{}_gw", entry.kind));
    }
    text.push('\n');
    for t in 0..overlay.electrical_total.len() {
        text.push_str(&overlay.start.plus_periods(t as i64).to_string());
        text.push_str(&format!(
            ",{},{}",
            overlay.delivered_heat[t].as_gigawatts(),
            overlay.electrical_total[t].as_gigawatts()
        ));
        for entry in &overlay.entries {
            text.push_str(&format!(",{}", entry.electrical[t].as_gigawatts()));
        }
        text.push('\n');
    }
    text
}

fn write_heating_parquet(
    path: &Path,
    overlay: &grid_core::heating::HeatingOverlay,
    meta: &Metadata,
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let mut fields = vec![
        Field::new("utc_start", ts_type, false),
        Field::new("delivered_heat_gw", DataType::Float64, false),
        Field::new("heating_electrical_total_gw", DataType::Float64, false),
    ];
    for entry in &overlay.entries {
        fields.push(Field::new(
            format!("heating_{}_gw", entry.kind),
            DataType::Float64,
            false,
        ));
    }
    let schema = Arc::new(Schema::new(fields));

    let periods = overlay.electrical_total.len();
    let mut stamps = TimestampMicrosecondBuilder::new();
    for t in 0..periods {
        stamps.append_value(overlay.start.plus_periods(t as i64).unix_micros());
    }
    let mut arrays: Vec<ArrayRef> = vec![Arc::new(stamps.finish().with_timezone("UTC"))];
    for series in std::iter::once(&overlay.delivered_heat)
        .chain(std::iter::once(&overlay.electrical_total))
        .chain(overlay.entries.iter().map(|e| &e.electrical))
    {
        let mut builder = Float64Builder::new();
        for &p in series {
            builder.append_value(p.as_gigawatts());
        }
        arrays.push(Arc::new(builder.finish()));
    }
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

/// The `[results.heating]` summary section: the pinned constants
/// (k, DHW rate, damping, lag, deratings, cop_const), per-year
/// delivered-heat totals (the inter-annual spread is a FINDING, never
/// normalised away — D9 rule 3), and per-entry effective parameters
/// with overrides flagged.
fn heating_summary_section(
    overlay: &grid_core::heating::HeatingOverlay,
    heating_digest: &str,
) -> String {
    let dt_h = 0.5;
    let c = &overlay.constants;
    let mut text = String::from("\n[results.heating]\n");
    text.push_str(&format!(
        "heating_digest_sha256 = {}\n",
        toml_quote(heating_digest)
    ));
    text.push_str(&format!(
        "k_gw_per_kelvin = {}\n",
        c.k.as_gigawatts_per_kelvin()
    ));
    text.push_str(&format!("dhw_rate_gw = {}\n", c.dhw_rate.as_gigawatts()));
    text.push_str(&format!(
        "mean_annual_degree_hours_c_h = {}\n",
        c.mean_annual_degree_hours.as_celsius_hours()
    ));
    text.push_str(&format!(
        "record_start = {}\n",
        toml_quote(&c.record_start.to_string())
    ));
    text.push_str(&format!("record_years = {}\n", c.record_years));
    text.push_str(&format!(
        "electrified_quantum_twh = {}\n",
        c.electrified_quantum.as_gigawatt_hours() / 1000.0
    ));
    text.push_str(&format!("t_base_c = {}\n", c.t_base.as_celsius()));
    text.push_str(&format!(
        "ground_surface_mean_c = {}\n",
        c.ground.surface_mean.as_celsius()
    ));
    text.push_str(&format!(
        "ground_surface_amplitude_c = {}\n",
        c.ground.surface_amplitude.as_celsius()
    ));
    text.push_str(&format!("ground_damping = {}\n", c.ground.damping.value()));
    text.push_str(&format!(
        "ground_lag_days = {}\n",
        c.ground.lag.as_hours() / 24.0
    ));
    let horizon_delivered: f64 = overlay
        .delivered_heat
        .iter()
        .map(|p| p.as_gigawatts() * dt_h)
        .sum();
    let horizon_electrical: f64 = overlay
        .electrical_total
        .iter()
        .map(|p| p.as_gigawatts() * dt_h)
        .sum();
    text.push_str(&format!(
        "delivered_heat_twh = {}\n",
        horizon_delivered / 1000.0
    ));
    text.push_str(&format!(
        "electrical_twh = {}\n",
        horizon_electrical / 1000.0
    ));

    // Per-calendar-year delivered heat over the horizon (rule 3: a
    // reported output whose spread is a finding).
    text.push_str("\n[results.heating.delivered_heat_per_year_twh]\n");
    let mut current: Option<(i64, f64)> = None;
    let mut per_year: Vec<(i64, f64)> = Vec::new();
    for (t, p) in overlay.delivered_heat.iter().enumerate() {
        let (year, _, _) = overlay.start.plus_periods(t as i64).civil_date();
        match &mut current {
            Some((y, acc)) if *y == year => *acc += p.as_gigawatts() * dt_h,
            _ => {
                if let Some(done) = current.take() {
                    per_year.push(done);
                }
                current = Some((year, p.as_gigawatts() * dt_h));
            }
        }
    }
    if let Some(done) = current {
        per_year.push(done);
    }
    for (year, gwh) in per_year {
        text.push_str(&format!("\"{year}\" = {}\n", gwh / 1000.0));
    }

    // Per-entry effective parameters and series totals; overrides can
    // never hide (the reliability/inertia precedent).
    for entry in &overlay.entries {
        text.push_str(&format!("\n[results.heating.entries.{}]\n", entry.kind));
        text.push_str(&format!("share = {}\n", entry.share.value()));
        let electrical: f64 = entry
            .electrical
            .iter()
            .map(|p| p.as_gigawatts() * dt_h)
            .sum();
        text.push_str(&format!("electrical_twh = {}\n", electrical / 1000.0));
        if let Some(curve) = entry.params.cop_curve {
            text.push_str(&format!(
                "cop_curve = [{}, {}, {}]\n",
                curve[0], curve[1], curve[2]
            ));
        }
        if let Some(factor) = entry.params.correction_factor {
            text.push_str(&format!("correction_factor = {}\n", factor.value()));
        }
        if let Some(factor) = entry.params.rhpp_derating {
            text.push_str(&format!("rhpp_derating = {}\n", factor.value()));
        }
        if let Some(offset) = entry.params.source_offset {
            text.push_str(&format!("source_offset_k = {}\n", offset.as_celsius()));
        }
        if let Some(cop) = entry.params.cop_const {
            text.push_str(&format!("cop_const = {cop}\n"));
        }
        // The D16 geothermal resource depth (schema v8): a scenario
        // field, not a reference override — echoed on its own line.
        if let Some(depth) = entry.params.resource_depth {
            text.push_str(&format!("resource_depth_m = {}\n", depth.as_metres()));
        }
        text.push_str(&format!(
            "overridden = [{}]\n",
            entry
                .params
                .overridden
                .iter()
                .map(|f| toml_quote(f))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    text
}

/// Write the heating output pair (`heating<suffix>.{csv,parquet}`) and
/// return the summary section. `suffix` is empty on the single-zone
/// path and `_<zone>` per zone on the multi-zone path.
fn write_heating_outputs(
    out_dir: &Path,
    suffix: &str,
    overlay: &grid_core::heating::HeatingOverlay,
    meta: &Metadata,
) -> Result<(String, String), String> {
    let data_section = heating_csv_data_section(overlay);
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
    let csv_path = out_dir.join(format!("heating{suffix}.csv"));
    std::fs::write(
        &csv_path,
        format!("{}{data_section}", metadata_comment_block(meta)),
    )
    .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;
    write_heating_parquet(
        &out_dir.join(format!("heating{suffix}.parquet")),
        overlay,
        meta,
    )?;
    Ok((heating_summary_section(overlay, &digest), digest))
}

// ---------------------------------------------------------------------
// Stage 2 pricing outputs.
// ---------------------------------------------------------------------

/// The prices CSV data section (header + rows, no metadata comments) —
/// also the input to the prices digest. Columns: `utc_start`, the SMP,
/// the price-setting technology (empty in must-take-only periods), then
/// one SRMC column per priced technology (alphabetical).
fn prices_csv_data_section(
    result: &RunResult,
    pricing_inputs: &PricingInputs,
    priced: &PricingResult,
) -> String {
    let mut text = String::from("utc_start,smp_gbp_per_mwh,price_setter");
    for tech in pricing_inputs.srmc.keys() {
        text.push_str(&format!(",srmc_{tech}_gbp_per_mwh"));
    }
    text.push('\n');
    for t in 0..result.periods() {
        text.push_str(&result.timestamp_at(t).to_string());
        text.push_str(&format!(",{}", priced.smp[t].as_pounds_per_megawatt_hour()));
        text.push(',');
        if let Some(tech) = &priced.setter[t] {
            text.push_str(tech.as_str());
        }
        for srmc in pricing_inputs.srmc.values() {
            text.push_str(&format!(
                ",{}",
                srmc.values()[t].as_pounds_per_megawatt_hour()
            ));
        }
        text.push('\n');
    }
    text
}

fn write_prices_parquet(
    path: &Path,
    result: &RunResult,
    pricing_inputs: &PricingInputs,
    priced: &PricingResult,
    meta: &Metadata,
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());

    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let mut fields = vec![
        Field::new("utc_start", ts_type, false),
        Field::new("smp_gbp_per_mwh", DataType::Float64, false),
        Field::new("price_setter", DataType::Utf8, false),
    ];
    for tech in pricing_inputs.srmc.keys() {
        fields.push(Field::new(
            format!("srmc_{tech}_gbp_per_mwh"),
            DataType::Float64,
            false,
        ));
    }
    let schema = Arc::new(Schema::new(fields));

    let mut stamps = TimestampMicrosecondBuilder::new();
    let mut smp = Float64Builder::new();
    let mut setter = StringBuilder::new();
    for t in 0..result.periods() {
        stamps.append_value(result.timestamp_at(t).unix_micros());
        smp.append_value(priced.smp[t].as_pounds_per_megawatt_hour());
        setter.append_value(priced.setter[t].as_ref().map_or("", |tech| tech.as_str()));
    }
    let mut arrays: Vec<ArrayRef> = vec![
        Arc::new(stamps.finish().with_timezone("UTC")),
        Arc::new(smp.finish()),
        Arc::new(setter.finish()),
    ];
    for srmc in pricing_inputs.srmc.values() {
        let mut builder = Float64Builder::new();
        for price in srmc.values() {
            builder.append_value(price.as_pounds_per_megawatt_hour());
        }
        arrays.push(Arc::new(builder.finish()));
    }
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

/// Combined (offshore + onshore) DELIVERED wind output: the D3
/// total-wind convention on the delivered basis (pro-rata share of the
/// pooled curtailment — `grid_adequacy::pricing`); `None` if the fleet
/// has no wind.
fn total_wind_delivered(result: &RunResult) -> Result<Option<Vec<Power>>, String> {
    let delivered = grid_adequacy::delivered_renewable_power(result).map_err(|e| e.to_string())?;
    let mut total = vec![Power::gigawatts(0.0); result.periods()];
    let mut any = false;
    for (series, delivered_power) in result.renewables.iter().zip(&delivered) {
        if !matches!(series.tech.as_str(), "offshore_wind" | "onshore_wind") {
            continue;
        }
        any = true;
        for (acc, &p) in total.iter_mut().zip(delivered_power) {
            *acc = *acc + p;
        }
    }
    Ok(any.then_some(total))
}

/// Combined (offshore + onshore) model wind output — the D3 total-wind
/// convention on the model side; `None` if the fleet has no wind.
fn total_wind(result: &RunResult) -> Option<Vec<Power>> {
    let wind: Vec<&[Power]> = result
        .renewables
        .iter()
        .filter(|s| matches!(s.tech.as_str(), "offshore_wind" | "onshore_wind"))
        .map(|s| s.power.as_slice())
        .collect();
    if wind.is_empty() {
        return None;
    }
    let mut total = vec![Power::gigawatts(0.0); result.periods()];
    for series in wind {
        for (acc, &p) in total.iter_mut().zip(series) {
            *acc = *acc + p;
        }
    }
    Some(total)
}

/// The share (%) of periods where the observed price sits within ±20 %
/// of the CCGT SRMC — the *price-consistency* framing of the gas-
/// marginal statistic (definition A of the price-pack report §4;
/// observed 2024 value ≈ 64 %). docs/04 requires both framings reported;
/// the behavioural framing is the model's price-setter flag.
fn observed_within_20pct_of_ccgt_srmc(pricing_inputs: &PricingInputs) -> Option<f64> {
    let observed = pricing_inputs.observed_price.as_ref()?;
    let srmc = pricing_inputs
        .srmc
        .get(&grid_core::scenario::TechId::new("ccgt"))?;
    let within = observed
        .values()
        .iter()
        .zip(srmc.values())
        .filter(|(p, s)| {
            (p.as_pounds_per_megawatt_hour() - s.as_pounds_per_megawatt_hour()).abs()
                <= 0.2 * s.as_pounds_per_megawatt_hour()
        })
        .count();
    Some(100.0 * within as f64 / observed.len() as f64)
}

/// The `[results.pricing]` summary block.
fn pricing_summary_section(
    result: &RunResult,
    pricing_inputs: &PricingInputs,
    priced: &PricingResult,
    prices_digest: &str,
) -> Result<String, String> {
    let priced_techs: Vec<&str> = pricing_inputs.srmc.keys().map(|t| t.as_str()).collect();
    let gas_share = 100.0 * price_setting_share(&priced.setter, &priced_techs);
    let must_take_only = 100.0 * priced.setter.iter().filter(|s| s.is_none()).count() as f64
        / result.periods() as f64;

    let mut text = String::from("\n[results.pricing]\n");
    text.push_str(&format!(
        "prices_digest_sha256 = {}\n",
        toml_quote(prices_digest)
    ));
    text.push_str(&format!(
        "smp_time_weighted_mean_gbp_per_mwh = {}\n",
        priced.smp_time_weighted_mean.as_pounds_per_megawatt_hour()
    ));
    // Both gas-marginal framings (docs/04 Stage 2): the model's
    // behavioural price-setter flag, and the observed-price consistency
    // with the CCGT SRMC.
    text.push_str(&format!("pct_periods_gas_price_setting = {gas_share}\n"));
    text.push_str(&format!("pct_periods_must_take_only = {must_take_only}\n"));
    if let Some(pct) = observed_within_20pct_of_ccgt_srmc(pricing_inputs) {
        text.push_str(&format!(
            "pct_observed_price_within_20pct_of_ccgt_srmc = {pct}\n"
        ));
    }
    text.push_str(&format!("unserved_periods = {}\n", priced.unserved_periods));

    if let Some(wind) = total_wind(result) {
        let capture = capture_price(&wind, &priced.smp).map_err(|e| e.to_string())?;
        let ratio = capture_ratio(&wind, &priced.smp).map_err(|e| e.to_string())?;
        if let (Some(capture), Some(ratio)) = (capture, ratio) {
            text.push_str(&format!(
                "wind_capture_price_gbp_per_mwh = {}\n",
                capture.as_pounds_per_megawatt_hour()
            ));
            text.push_str(&format!("wind_capture_ratio = {ratio}\n"));
        }
    }
    // Delivered-basis wind capture, ADDED alongside the potential-basis
    // keys above (Package A; both conventions in prose at
    // `grid_adequacy::pricing`).
    if let Some(wind) = total_wind_delivered(result)? {
        let capture = capture_price(&wind, &priced.smp).map_err(|e| e.to_string())?;
        let ratio = capture_ratio(&wind, &priced.smp).map_err(|e| e.to_string())?;
        if let (Some(capture), Some(ratio)) = (capture, ratio) {
            text.push_str(&format!(
                "wind_capture_price_delivered_gbp_per_mwh = {}\n",
                capture.as_pounds_per_megawatt_hour()
            ));
            text.push_str(&format!("wind_capture_ratio_delivered = {ratio}\n"));
        }
    }

    text.push_str(&format!(
        "total_co2_mt = {}\n",
        priced.total_co2.as_tonnes_co2() / 1e6
    ));
    text.push_str(&format!(
        "total_co2e_mt = {}\n",
        priced.total_co2e.as_tonnes_co2() / 1e6
    ));

    if let Some(realism) = &priced.realism {
        // Reported, not gated (docs/04 Stage 2): observed benchmarks are
        // median P/SRMC 0.955 (note the reciprocal direction) and
        // monthly corr 0.85.
        text.push_str(&format!(
            "median_model_smp_over_observed_mid = {}\n",
            realism.median_model_over_observed
        ));
        text.push_str(&format!(
            "monthly_corr_model_smp_vs_observed_mid = {}\n",
            realism.monthly_correlation
        ));
    }

    for tech in &priced.technologies {
        text.push_str(&format!("\n[results.pricing.technologies.{}]\n", tech.tech));
        text.push_str(&format!(
            "energy_twh = {}\n",
            tech.energy.as_gigawatt_hours() / 1000.0
        ));
        text.push_str(&format!(
            "revenue_m_gbp = {}\n",
            tech.revenue.as_pounds() / 1e6
        ));
        if let Some(capture) = tech.capture_price {
            text.push_str(&format!(
                "capture_price_gbp_per_mwh = {}\n",
                capture.as_pounds_per_megawatt_hour()
            ));
        }
        if let Some(ratio) = tech.capture_ratio {
            text.push_str(&format!("capture_ratio = {ratio}\n"));
        }
        // Delivered basis, added alongside (Package A) — coincides with
        // the potential basis for thermal and at zero curtailment.
        text.push_str(&format!(
            "energy_delivered_twh = {}\n",
            tech.energy_delivered.as_gigawatt_hours() / 1000.0
        ));
        text.push_str(&format!(
            "revenue_delivered_m_gbp = {}\n",
            tech.revenue_delivered.as_pounds() / 1e6
        ));
        if let Some(capture) = tech.capture_price_delivered {
            text.push_str(&format!(
                "capture_price_delivered_gbp_per_mwh = {}\n",
                capture.as_pounds_per_megawatt_hour()
            ));
        }
        if let Some(ratio) = tech.capture_ratio_delivered {
            text.push_str(&format!("capture_ratio_delivered = {ratio}\n"));
        }
    }

    for emissions in &priced.emissions {
        text.push_str(&format!(
            "\n[results.pricing.emissions.{}]\n",
            emissions.tech
        ));
        text.push_str(&format!(
            "co2_kt = {}\n",
            emissions.co2.as_tonnes_co2() / 1e3
        ));
        text.push_str(&format!(
            "co2e_kt = {}\n",
            emissions.co2e.as_tonnes_co2() / 1e3
        ));
    }
    Ok(text)
}

fn toml_quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn summary_toml(result: &RunResult, meta: &Metadata, digest: &str) -> String {
    let twh = |e: Energy| e.as_gigawatt_hours() / 1000.0;
    let mut text = String::from("# grid-cli run summary (docs/06)\n[metadata]\n");
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
        "\n[results]\nperiods = {}\nresult_digest_sha256 = {}\n",
        result.periods(),
        toml_quote(digest)
    ));
    text.push_str(&format!(
        "demand_twh = {}\n",
        twh(result.total_demand_energy())
    ));
    text.push_str(&format!(
        "net_imports_twh = {}\n",
        twh(result.net_imports_energy())
    ));
    text.push_str(&format!(
        "curtailment_twh = {}\n",
        twh(result.total_curtailment())
    ));
    text.push_str(&format!(
        "unserved_twh = {}\n",
        twh(result.total_unserved())
    ));
    text.push_str("\n[results.energy_twh]\n");
    for col in columns(result) {
        if col.label == "demand" || col.label == "curtailment" || col.label == "unserved" {
            continue;
        }
        text.push_str(&format!(
            "{} = {}\n",
            col.label,
            twh(RunResult::total_energy(col.power))
        ));
    }
    // Per-store aggregates (docs/03 outputs: storage min/max SoC).
    for store in &result.stores {
        text.push_str(&format!("\n[results.storage.{}]\n", store.label));
        text.push_str(&format!(
            "charge_twh = {}\n",
            twh(RunResult::total_energy(&store.charge))
        ));
        text.push_str(&format!(
            "discharge_twh = {}\n",
            twh(RunResult::total_energy(&store.discharge))
        ));
        if let (Some((min_index, min_soc)), Some(max_soc)) = (store.min_soc(), store.max_soc()) {
            text.push_str(&format!("min_soc_gwh = {}\n", min_soc.as_gigawatt_hours()));
            text.push_str(&format!(
                "min_soc_at = {}\n",
                toml_quote(&result.timestamp_at(min_index).to_string())
            ));
            text.push_str(&format!("max_soc_gwh = {}\n", max_soc.as_gigawatt_hours()));
        }
    }
    // Reliability accounting (gb-grid-margin methodology; see
    // grid_adequacy::result). The full classification is emitted so
    // overrides cannot hide.
    if let Some(stats) = result.firm_share_stats() {
        text.push_str("\n[results.reliability]\n");
        text.push_str(&format!("firm_share_mean = {}\n", stats.mean));
        text.push_str(&format!("firm_share_min = {}\n", stats.min));
        text.push_str(&format!("firm_share_p25 = {}\n", stats.p25));
        text.push_str(&format!(
            "periods_firm_share_below_0_5 = {}\n",
            stats.below_threshold
        ));
        text.push_str(&format!(
            "threshold = {}\n",
            grid_adequacy::FIRM_SHARE_ALARM_THRESHOLD
        ));
        text.push_str("\n[results.reliability.classification]\n");
        for series in result.renewables.iter().chain(&result.thermal) {
            text.push_str(&format!(
                "{} = {}\n",
                series.tech,
                toml_quote(series.reliability.as_str())
            ));
        }
        for series in &result.exogenous {
            text.push_str(&format!(
                "{} = {}\n",
                series.label,
                toml_quote(series.reliability.as_str())
            ));
        }
        for store in &result.stores {
            // The fourth category: storage is never folded into firm.
            text.push_str(&format!("{} = {}\n", store.label, toml_quote("storage")));
        }
        let overrides: Vec<_> = result
            .renewables
            .iter()
            .chain(&result.thermal)
            .filter(|s| s.reliability_overridden)
            .collect();
        if !overrides.is_empty() {
            text.push_str("\n[results.reliability.overrides]\n");
            for series in overrides {
                text.push_str(&format!(
                    "{} = {}\n",
                    series.tech,
                    toml_quote(series.reliability.as_str())
                ));
            }
        }
    }
    text
}

/// Execute `grid-cli run`.
pub fn execute(args: &RunArgs) -> Result<(), String> {
    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    if scenario.zones.len() > 1 {
        // Stage 5: the multi-zone engine path (per-zone outputs +
        // links.csv). The single-zone path below is byte-untouched.
        return execute_multi(args, &scenario);
    }
    let inputs = load_run_inputs(&scenario, &args.base_dir).map_err(|e| e.to_string())?;
    let result = grid_adequacy::run(&scenario, &inputs).map_err(|e| e.to_string())?;

    // Stage 2: price the completed run when a [pricing] block is
    // declared (pricing reads the dispatch output only — the dispatch
    // digest cannot move).
    let pricing = match &scenario.pricing {
        Some(pricing_spec) => {
            let pricing_inputs = load_pricing_inputs(&scenario, pricing_spec, &args.base_dir)
                .map_err(|e| e.to_string())?;
            let priced = price_run(&result, &pricing_inputs).map_err(|e| e.to_string())?;
            Some((pricing_inputs, priced))
        }
        None => None,
    };

    // Remaining non-goals, stated out loud rather than silently skipped.
    if !scenario.links.is_empty() {
        println!(
            "note: {} scenario links are ignored (imports are an exogenous trace until Stage 5)",
            scenario.links.len()
        );
    }

    // Metadata: hashes of everything the run depended on.
    let meta = Metadata {
        engine_git_hash: env!("GRID_ENGINE_GIT_HASH"),
        schema_version: SCHEMA_VERSION,
        scenario_path: args.scenario.display().to_string(),
        scenario_sha256: sha256_file(&args.scenario)?,
        created_utc: now_utc(),
        data_files: scenario_data_files(&scenario, &args.base_dir)?,
    };

    // Outputs: CSV + Parquet, both, always (docs/06).
    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;
    let write = |name: &str, contents: &str| -> Result<(), String> {
        let path = args.out.join(name);
        std::fs::write(&path, contents).map_err(|e| format!("cannot write {}: {e}", path.display()))
    };

    let data_section = csv_data_section(&result);
    let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
    write(
        "dispatch.csv",
        &format!("{}{data_section}", metadata_comment_block(&meta)),
    )?;
    write_dispatch_parquet(&args.out.join("dispatch.parquet"), &result, &meta)?;
    write("monthly_mix.csv", &monthly_mix_csv(&result, &meta))?;

    let mut summary = summary_toml(&result, &meta, &digest);

    // Q5 heating overlay (D9 rule 6b): outputs written ONLY when the
    // scenario declares a heating block.
    let mut heating_digest = None;
    if let Some(overlay) = &inputs.heating {
        let (section, h_digest) = write_heating_outputs(&args.out, "", overlay, &meta)?;
        summary.push_str(&section);
        heating_digest = Some(h_digest);
    }

    let mut prices_digest = None;
    if let Some((pricing_inputs, priced)) = &pricing {
        let prices_section = prices_csv_data_section(&result, pricing_inputs, priced);
        let p_digest = format!("{:x}", Sha256::digest(prices_section.as_bytes()));
        write(
            "prices.csv",
            &format!("{}{prices_section}", metadata_comment_block(&meta)),
        )?;
        write_prices_parquet(
            &args.out.join("prices.parquet"),
            &result,
            pricing_inputs,
            priced,
            &meta,
        )?;
        summary.push_str(&pricing_summary_section(
            &result,
            pricing_inputs,
            priced,
            &p_digest,
        )?);
        prices_digest = Some(p_digest);
    }
    write("summary.toml", &summary)?;

    // Headline aggregates.
    let twh = |e: Energy| e.as_gigawatt_hours() / 1000.0;
    println!(
        "run complete: {} periods from {}",
        result.periods(),
        result.start
    );
    println!(
        "  demand      {:8.2} TWh",
        twh(result.total_demand_energy())
    );
    for col in columns(&result) {
        if col.label == "demand" {
            continue;
        }
        println!(
            "  {:<20} {:8.3} TWh",
            col.label,
            twh(RunResult::total_energy(col.power))
        );
    }
    println!("  result digest sha256 = {digest}");
    if let Some(stats) = result.firm_share_stats() {
        println!(
            "  reliability: firm share of demand mean {:.4}, min {:.4}, P25 {:.4}; {} periods \
             below {} (unclamped; gb-grid-margin methodology)",
            stats.mean,
            stats.min,
            stats.p25,
            stats.below_threshold,
            grid_adequacy::FIRM_SHARE_ALARM_THRESHOLD,
        );
    }
    if let Some((pricing_inputs, priced)) = &pricing {
        let priced_techs: Vec<&str> = pricing_inputs.srmc.keys().map(|t| t.as_str()).collect();
        println!(
            "  pricing: mean SMP {:.2} GBP/MWh, priced techs ({}) set the price in {:.2} % \
             of periods, must-take-only {:.2} %",
            priced.smp_time_weighted_mean.as_pounds_per_megawatt_hour(),
            priced_techs.join("+"),
            100.0 * price_setting_share(&priced.setter, &priced_techs),
            100.0 * priced.setter.iter().filter(|s| s.is_none()).count() as f64
                / result.periods() as f64,
        );
        if let Some(wind) = total_wind(&result)
            && let Some(ratio) = capture_ratio(&wind, &priced.smp).map_err(|e| e.to_string())?
        {
            println!("  wind capture ratio = {ratio:.4}");
        }
        if let Some(wind) = total_wind_delivered(&result)?
            && let Some(ratio) = capture_ratio(&wind, &priced.smp).map_err(|e| e.to_string())?
        {
            println!("  wind capture ratio (delivered basis) = {ratio:.4}");
        }
        println!(
            "  emissions: {:.2} MtCO2 (pricing basis) / {:.2} MtCO2e (accounting basis)",
            priced.total_co2.as_tonnes_co2() / 1e6,
            priced.total_co2e.as_tonnes_co2() / 1e6,
        );
        if let Some(p_digest) = &prices_digest {
            println!("  prices digest sha256 = {p_digest}");
        }
    }
    if let (Some(h_digest), Some(overlay)) = (&heating_digest, &inputs.heating) {
        let twh_of = |series: &[grid_core::units::Power]| -> f64 {
            series.iter().map(|p| p.as_gigawatts() * 0.5).sum::<f64>() / 1000.0
        };
        println!(
            "  heating: {:.2} TWh delivered heat, {:.2} TWh electrical \
             (k = {:.4} GW/K, DHW {:.3} GW); digest sha256 = {h_digest}",
            twh_of(&overlay.delivered_heat),
            twh_of(&overlay.electrical_total),
            overlay.constants.k.as_gigawatts_per_kelvin(),
            overlay.constants.dhw_rate.as_gigawatts(),
        );
    }
    println!("  outputs in {}", args.out.display());
    Ok(())
}

// ---------------------------------------------------------------------
// Stage 5: the multi-zone run path.
// ---------------------------------------------------------------------

/// Sending-end power in a link's FORWARD (`from → to`) direction at
/// period `t` (0 when the link imports into `from`): the quantity the
/// schema-v6 capability/binding columns are measured against.
fn forward_sent_gw(link: &grid_adequacy::LinkFlowSeries, t: usize) -> f64 {
    (-link.home_end[t].as_gigawatts()).max(0.0)
}

/// Whether a capability-detailed link is BINDING in its forward
/// direction at `t`: the sending-end flow reaches 99 % of the applied
/// capability (the B6 gate-(iii) convention) on an OBSERVED-capability
/// period (masked periods never count — the ruling: missing stays
/// missing).
fn forward_binding(
    capability: &grid_adequacy::LinkCapabilitySeries,
    link: &grid_adequacy::LinkFlowSeries,
    t: usize,
) -> bool {
    capability.forward_observed[t]
        && forward_sent_gw(link, t) >= 0.99 * capability.forward[t].as_gigawatts()
}

/// The links CSV data section: one row per period, signed GW at each
/// end of every link (`<name>_home_gw` = into the `from` zone — the
/// NESO convention for GB-home links; `<name>_away_gw` = into the `to`
/// zone). Links with schema-v6 capability detail additionally carry
/// `<name>_fwd_cap_gw` (the applied forward capability) and
/// `<name>_binding` (1 when the forward flow reaches 99 % of it on an
/// observed period — the B6 gate-(iii) convention); pre-v6 links keep
/// their exact column set, so pinned digests never move. Also the
/// input to the links digest.
fn links_csv_data_section(result: &MultiZoneRunResult, start: UtcInstant) -> String {
    let mut text = String::from("utc_start");
    for link in &result.links {
        text.push_str(&format!(",{}_home_gw,{}_away_gw", link.name, link.name));
        if link.capability.is_some() {
            text.push_str(&format!(",{}_fwd_cap_gw,{}_binding", link.name, link.name));
        }
    }
    text.push('\n');
    let periods = result.links.first().map_or(0, |l| l.home_end.len());
    for t in 0..periods {
        text.push_str(&start.plus_periods(t as i64).to_string());
        for link in &result.links {
            text.push_str(&format!(
                ",{},{}",
                link.home_end[t].as_gigawatts(),
                link.away_end[t].as_gigawatts()
            ));
            if let Some(capability) = &link.capability {
                text.push_str(&format!(
                    ",{},{}",
                    capability.forward[t].as_gigawatts(),
                    u8::from(forward_binding(capability, link, t))
                ));
            }
        }
        text.push('\n');
    }
    text
}

/// Assumption comment lines for links carrying schema-v6 capability
/// detail — the B6 ruling's conventions travel on the artefact itself
/// (docs/notes/b6-two-zone-data-review.md §6). Empty for scenarios
/// without such links (pre-v6 artefacts byte-identical).
fn links_assumption_lines(scenario: &Scenario) -> String {
    let mut text = String::new();
    for link in &scenario.links {
        let Some(name) = link
            .name
            .as_deref()
            .filter(|_| link.reverse_capacity_gw.is_some() || link.capability_trace.is_some())
        else {
            continue;
        };
        text.push_str(&format!(
            "# link {name} capability conventions (schema v6; the b6-two-zone-data-review \
             §6 ruling):\n"
        ));
        match &link.capability_trace {
            Some(trace) => text.push_str(&format!(
                "#   forward ({} -> {}) capability = observed trace {} [{}], MW; values >= \
                 {} MW are no-constraint sentinels replaced by the pinned upper bound \
                 {} GW; zero/NaN/missing rows are MASKED out of validation-gate arithmetic \
                 and dispatch against the pinned central fill {} GW; availability {} \
                 multiplies\n",
                link.from,
                link.to,
                trace.path,
                trace.column,
                trace.sentinel_high_mw.as_gigawatts() * 1000.0,
                trace.upper_bound_gw.as_gigawatts(),
                trace.masked_fill_gw.as_gigawatts(),
                link.availability.value(),
            )),
            None => text.push_str(&format!(
                "#   forward ({} -> {}) capability = {} GW flat x availability {}\n",
                link.from,
                link.to,
                link.capacity_gw.as_gigawatts(),
                link.availability.value(),
            )),
        }
        if let Some(reverse) = link.reverse_capacity_gw {
            text.push_str(&format!(
                "#   reverse ({} -> {}) capability = {} GW flat x availability {}\n",
                link.to,
                link.from,
                reverse.as_gigawatts(),
                link.availability.value(),
            ));
        }
        if name == "B6" {
            // The ruling's quote duty travels verbatim on every B6
            // artefact (ruling (c)), plus the engine-review conditions.
            text.push_str(
                "#   B6 QUOTE DUTY: model constraint/curtailment outputs are a LOWER BOUND \
                 on the Scottish constraint phenomenon (B6-only slice; the intra-Scottish \
                 B4/B5 boundaries are structurally invisible to a two-zone model). \
                 Like-for-like cost anchor: B6/SCOTEX GBP 90.5m calendar 2024; the Scottish \
                 boundary group (GBP 525.8m) is CONTEXT ONLY, never a tuning target — the \
                 link capability must never be tuned to reproduce it.\n",
            );
            text.push_str(
                "#   B6-ATTRIBUTABLE SUBTRACTION RULE (engine review condition 7): a \
                 constrained-minus-copper Scottish curtailment delta is NEVER quoted alone \
                 — blocking Scottish surplus at B6 shuffles curtailment between zones, so \
                 the rest-of-GB zone moves the OPPOSITE way in the same comparison; carry \
                 both legs or quote the system net.\n",
            );
            text.push_str(
                "#   ROBUSTNESS/ADEQUACY QUOTE DUTY (engine review §1d): storage-requirement \
                 sensitivities from this geometry carry 'rule-based dispatch, upper-bias' \
                 (flows clear before storage, blind to store headroom; LP/Stage-7 is the \
                 named resolver) alongside the stress convention (2024 boundary capability) \
                 and the store-placement conditioning.\n",
            );
        }
    }
    text
}

fn write_links_parquet(
    path: &Path,
    result: &MultiZoneRunResult,
    start: UtcInstant,
    meta: &Metadata,
) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("cannot write {}: {e}", path.display());
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let mut fields = vec![Field::new("utc_start", ts_type, false)];
    for link in &result.links {
        fields.push(Field::new(
            format!("{}_home_gw", link.name),
            DataType::Float64,
            false,
        ));
        fields.push(Field::new(
            format!("{}_away_gw", link.name),
            DataType::Float64,
            false,
        ));
        if link.capability.is_some() {
            // Schema-v6 capability detail (matches the CSV columns).
            fields.push(Field::new(
                format!("{}_fwd_cap_gw", link.name),
                DataType::Float64,
                false,
            ));
            fields.push(Field::new(
                format!("{}_binding", link.name),
                DataType::Float64,
                false,
            ));
        }
    }
    let schema = Arc::new(Schema::new(fields));
    let periods = result.links.first().map_or(0, |l| l.home_end.len());
    let mut stamps = TimestampMicrosecondBuilder::new();
    for t in 0..periods {
        stamps.append_value(start.plus_periods(t as i64).unix_micros());
    }
    let mut arrays: Vec<ArrayRef> = vec![Arc::new(stamps.finish().with_timezone("UTC"))];
    for link in &result.links {
        for series in [&link.home_end, &link.away_end] {
            let mut builder = Float64Builder::new();
            for &p in series {
                builder.append_value(p.as_gigawatts());
            }
            arrays.push(Arc::new(builder.finish()));
        }
        if let Some(capability) = &link.capability {
            let mut cap = Float64Builder::new();
            let mut binding = Float64Builder::new();
            for t in 0..periods {
                cap.append_value(capability.forward[t].as_gigawatts());
                binding.append_value(f64::from(u8::from(forward_binding(capability, link, t))));
            }
            arrays.push(Arc::new(cap.finish()));
            arrays.push(Arc::new(binding.finish()));
        }
    }
    let batch = RecordBatch::try_new(schema.clone(), arrays).map_err(|e| err(&e))?;
    let mut kv = Vec::new();
    for (key, value) in meta.pairs() {
        kv.push(parquet::file::metadata::KeyValue::new(
            key.to_owned(),
            value,
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

/// `grid-cli run` on a multi-zone scenario (Stage 5): per-zone
/// `dispatch_<zone>.{csv,parquet}` (the single-zone column set per
/// zone, link net positions among the exogenous columns), the link
/// flow series (`links.{csv,parquet}`), and one `summary.toml` with
/// per-zone digests and per-link annual energies.
fn execute_multi(args: &RunArgs, scenario: &Scenario) -> Result<(), String> {
    let inputs = load_multi_zone_inputs(scenario, &args.base_dir).map_err(|e| e.to_string())?;
    let result = run_multi(scenario, &inputs).map_err(|e| e.to_string())?;
    let start = result.zones[0].result.start;

    if scenario.pricing.is_some() {
        println!(
            "note: the [pricing] block is ignored on multi-zone runs (GB-zone price \
             formation is Stage 2 machinery, single-zone only — docs/04 Stage 5 non-goals)"
        );
    }

    let meta = Metadata {
        engine_git_hash: env!("GRID_ENGINE_GIT_HASH"),
        schema_version: SCHEMA_VERSION,
        scenario_path: args.scenario.display().to_string(),
        scenario_sha256: sha256_file(&args.scenario)?,
        created_utc: now_utc(),
        data_files: scenario_data_files(scenario, &args.base_dir)?,
    };
    std::fs::create_dir_all(&args.out)
        .map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;
    let write = |name: &str, contents: &str| -> Result<(), String> {
        let path = args.out.join(name);
        std::fs::write(&path, contents).map_err(|e| format!("cannot write {}: {e}", path.display()))
    };

    let twh = |e: Energy| e.as_gigawatt_hours() / 1000.0;
    let mut summary = String::from("# grid-cli run summary (multi-zone; docs/06)\n[metadata]\n");
    for (key, value) in meta.pairs() {
        let value = if key == "schema_version" {
            value
        } else {
            toml_quote(&value)
        };
        summary.push_str(&format!("{key} = {value}\n"));
    }
    summary.push_str("\n[metadata.data_files]\n");
    for (path, hash) in &meta.data_files {
        summary.push_str(&format!("{} = {}\n", toml_quote(path), toml_quote(hash)));
    }
    summary.push_str(&format!(
        "\n[results]\nperiods = {}\nzones = {}\nlinks = {}\n",
        result.zones[0].result.periods(),
        result.zones.len(),
        result.links.len()
    ));

    println!(
        "multi-zone run complete: {} zones, {} links, {} periods from {start}",
        result.zones.len(),
        result.links.len(),
        result.zones[0].result.periods(),
    );

    for zone in &result.zones {
        let data_section = csv_data_section(&zone.result);
        let digest = format!("{:x}", Sha256::digest(data_section.as_bytes()));
        write(
            &format!("dispatch_{}.csv", zone.id),
            &format!("{}{data_section}", metadata_comment_block(&meta)),
        )?;
        write_dispatch_parquet(
            &args.out.join(format!("dispatch_{}.parquet", zone.id)),
            &zone.result,
            &meta,
        )?;
        // Q5 heating overlay, per zone (only zones declaring a block).
        if let Some(zone_inputs) = inputs.zones.iter().find(|z| z.id == zone.id)
            && let Some(overlay) = &zone_inputs.inputs.heating
        {
            let (section, _) =
                write_heating_outputs(&args.out, &format!("_{}", zone.id), overlay, &meta)?;
            // The section header is [results.heating…]; scope it per
            // zone by rewriting the table names.
            summary.push_str(
                &section.replace("[results.heating", &format!("[results.heating_{}", zone.id)),
            );
        }
        summary.push_str(&format!(
            "\n[results.zones.{}]\n",
            toml_quote(zone.id.as_str())
        ));
        summary.push_str(&format!("result_digest_sha256 = {}\n", toml_quote(&digest)));
        summary.push_str(&format!(
            "demand_twh = {}\n",
            twh(zone.result.total_demand_energy())
        ));
        summary.push_str(&format!(
            "net_imports_twh = {}\n",
            twh(zone.result.net_imports_energy())
        ));
        summary.push_str(&format!(
            "curtailment_twh = {}\n",
            twh(zone.result.total_curtailment())
        ));
        summary.push_str(&format!(
            "unserved_twh = {}\n",
            twh(zone.result.total_unserved())
        ));
        println!(
            "  zone {:<8} demand {:8.2} TWh, net imports {:+7.2} TWh, unserved {:.3} TWh, \
             digest {}",
            zone.id,
            twh(zone.result.total_demand_energy()),
            twh(zone.result.net_imports_energy()),
            twh(zone.result.total_unserved()),
            short(&digest),
        );
    }

    let links_section = links_csv_data_section(&result, start);
    let links_digest = format!("{:x}", Sha256::digest(links_section.as_bytes()));
    // Capability-convention assumption lines (schema v6) ride between
    // the metadata block and the digested data section, so pre-v6
    // pinned links digests never move.
    write(
        "links.csv",
        &format!(
            "{}{}{links_section}",
            metadata_comment_block(&meta),
            links_assumption_lines(scenario)
        ),
    )?;
    write_links_parquet(&args.out.join("links.parquet"), &result, start, &meta)?;
    summary.push_str(&format!(
        "\n[results.link_flows]\nlinks_digest_sha256 = {}\n",
        toml_quote(&links_digest)
    ));
    for link in &result.links {
        summary.push_str(&format!(
            "\n[results.link_flows.{}]\n",
            toml_quote(&link.name)
        ));
        summary.push_str(&format!("from = {}\n", toml_quote(link.from.as_str())));
        summary.push_str(&format!("to = {}\n", toml_quote(link.to.as_str())));
        summary.push_str(&format!("net_home_twh = {}\n", twh(link.net_home_energy())));
        // Schema-v6 capability detail: forward binding statistics.
        // Binding is counted on CAPABILITY-OBSERVED periods only (masked
        // periods never count). LABELLING WARNING (engine-review
        // condition 5): the share below has denominator = the
        // capability's own observed-period count
        // (`forward_capability_observed_periods`, e.g. 17,158 for the
        // B6 2024 run: 17,568 − 53 zero − 354 missing − 3 NaN). It is
        // NOT the gate-(iii) validation statistic, whose denominator is
        // the DA FLOW MASK (rows with both flow and limit observed, e.g.
        // 17,211 for B6 2024 — the zero-limit rows stay in that
        // denominator). The two shares (~0.23301 here vs ~0.23229 at
        // gate iii) are close but MUST NOT be quoted interchangeably;
        // the gate statistic lives only in acceptance_b6_2zone.rs
        // against the observed DA series, never in this artefact.
        if let Some(capability) = &link.capability {
            let periods = link.home_end.len();
            let observed = capability.forward_observed.iter().filter(|&&o| o).count();
            let binding = (0..periods)
                .filter(|&t| forward_binding(capability, link, t))
                .count();
            summary.push_str(&format!(
                "forward_capability_observed_periods = {observed}\n"
            ));
            summary.push_str(&format!("forward_binding_periods = {binding}\n"));
            if observed > 0 {
                summary.push_str(&format!(
                    "forward_binding_share_of_capability_observed = {}  # denominator = \
                     forward_capability_observed_periods; NOT the gate-(iii) DA-flow-mask \
                     statistic (see acceptance_b6_2zone.rs)\n",
                    binding as f64 / observed as f64
                ));
            }
        }
        println!(
            "  link {:<10} {} <-> {:<8} net into {}: {:+7.3} TWh",
            link.name,
            link.from,
            link.to,
            link.from,
            twh(link.net_home_energy()),
        );
    }
    write("summary.toml", &summary)?;
    println!("  links digest sha256 = {links_digest}");
    println!("  outputs in {}", args.out.display());
    Ok(())
}

fn short(hash: &str) -> &str {
    &hash[..hash.len().min(12)]
}
