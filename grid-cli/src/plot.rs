//! `grid-cli plot` — chart rendering (docs/06: PNG via plotters, engine
//! and scenario hashes embedded in a footer caption).
//!
//! Stage 1 implements the one demo artefact of docs/04 Stage 1:
//! `plot monthly-mix`, the modelled-vs-actual 2024 monthly generation
//! stack. For each month, two stacked bars: modelled (left, solid) and
//! actual (right, translucent), stacked over the Stage 1 comparison fuel
//! set in the D3 total-generation convention (wind = offshore + onshore
//! model-side vs `wind_incl_embedded`; solar vs `solar_embedded`; hydro
//! vs `npshyd`).
//!
//! Stage 3 part 2 adds `plot soc-trace`, the docs/04 Stage 3 demo
//! artefact: store state of charge over the full run horizon (the
//! multi-week-drawdown / multi-year-recharge picture on the 40-year
//! hydrogen run). It reads a run's `dispatch.csv` (`*_soc_gwh`
//! columns — the same columns the run viewer renders), draws the full
//! per-period series (no decimation: the deep minima are the point of
//! the chart), and also writes `soc_trace.csv` — the SoC series alone
//! with the run's metadata header — next to the run outputs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use plotters::prelude::*;

/// Arguments to `grid-cli plot`.
#[derive(Args)]
pub struct PlotArgs {
    #[command(subcommand)]
    chart: Chart,
}

#[derive(Subcommand)]
enum Chart {
    /// Modelled vs. actual monthly generation stack (Stage 1 demo
    /// artefact).
    MonthlyMix {
        /// A `grid-cli run` output directory (reads monthly_mix.csv and
        /// summary.toml).
        #[arg(long)]
        run: PathBuf,
        /// The observed monthly generation matrix
        /// (data/packs/2024/processed/monthly_generation_2024.csv).
        #[arg(long)]
        actual: PathBuf,
        /// Output PNG path.
        #[arg(long)]
        out: PathBuf,
    },
    /// Module 3(c) (Stage 4 demo artefact): the timescale
    /// decomposition — the scenario's storage requirement attributed to
    /// the diurnal / synoptic / seasonal / inter-annual bands of its
    /// residual load, as a stacked chart plus decomposition.csv and
    /// decomposition.parquet. Runs the attribution's bisection solves
    /// (`grid_adequacy::attribution`), so it takes a minute or two on
    /// the 40-year record.
    Decomposition {
        /// Scenario TOML file (all-must-take fleet + one store — the
        /// RS-style scenarios; see the attribution module docs).
        #[arg(long)]
        scenario: PathBuf,
        /// Base directory against which relative trace paths are
        /// resolved.
        #[arg(long, default_value = ".")]
        base_dir: PathBuf,
        /// Output directory (created if absent).
        #[arg(long)]
        out: PathBuf,
    },
    /// Module 5 (Stage 5 demo artefact): interconnector capacity
    /// credit vs GB residual-demand percentile — per-link mean net
    /// import binned by the percentile of GB residual demand, chart +
    /// capacity_credit.csv. Runs the multi-zone scenario.
    CapacityCredit {
        /// Multi-zone scenario TOML file (e.g.
        /// scenarios/gb-2024-5zone.toml).
        #[arg(long)]
        scenario: PathBuf,
        /// Base directory against which relative trace paths are
        /// resolved.
        #[arg(long, default_value = ".")]
        base_dir: PathBuf,
        /// Output directory (created if absent).
        #[arg(long)]
        out: PathBuf,
    },
    /// Store state of charge over the run horizon (Stage 3 demo
    /// artefact: the multi-week-drawdown / multi-year-recharge
    /// picture). Also writes soc_trace.csv next to the run outputs.
    SocTrace {
        /// A `grid-cli run` output directory (reads dispatch.csv and
        /// summary.toml).
        #[arg(long)]
        run: PathBuf,
        /// Plot only this store (by output label, e.g. `hydrogen`);
        /// default: every store in the run.
        #[arg(long)]
        store: Option<String>,
        /// Output PNG path.
        #[arg(long)]
        out: PathBuf,
    },
}

/// The comparison fuel set, stack order bottom-up. Model columns are
/// `monthly_mix.csv` labels (GWh); actual columns are
/// `monthly_generation_2024.csv` fuels (GWh). Wind is combined
/// model-side to match the D3 total-wind convention.
const FUELS: [(&str, &[&str], &str, RGBColor); 8] = [
    (
        "nuclear",
        &["nuclear_gwh"],
        "nuclear",
        RGBColor(106, 76, 147),
    ),
    (
        "biomass",
        &["biomass_gwh"],
        "biomass",
        RGBColor(146, 102, 57),
    ),
    ("hydro", &["hydro_gwh"], "npshyd", RGBColor(0, 150, 170)),
    ("coal", &["coal_gwh"], "coal", RGBColor(80, 80, 80)),
    ("ccgt", &["ccgt_gwh"], "ccgt", RGBColor(230, 120, 30)),
    ("ocgt", &["ocgt_gwh"], "ocgt", RGBColor(200, 60, 40)),
    (
        "wind",
        &["offshore_wind_gwh", "onshore_wind_gwh"],
        "wind_incl_embedded",
        RGBColor(60, 120, 216),
    ),
    (
        "solar",
        &["solar_gwh"],
        "solar_embedded",
        RGBColor(240, 200, 50),
    ),
];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Parse a simple headerful CSV (no quoting — both inputs are plain
/// numeric matrices), skipping `#` comment lines. Returns column → 12
/// monthly values.
fn read_monthly_csv(path: &Path) -> Result<BTreeMap<String, Vec<f64>>, String> {
    let err = |m: String| format!("{}: {m}", path.display());
    let text = std::fs::read_to_string(path).map_err(|e| err(e.to_string()))?;
    let mut lines = text.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<String> = lines
        .next()
        .ok_or_else(|| err("empty file".to_owned()))?
        .split(',')
        .map(str::to_owned)
        .collect();
    let mut columns: BTreeMap<String, Vec<f64>> =
        header.iter().map(|h| (h.clone(), Vec::new())).collect();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        for (name, field) in header.iter().zip(line.split(',')) {
            // The first column is a month label; store NaN placeholders
            // for non-numeric fields and never read them back.
            let value = field.parse::<f64>().unwrap_or(f64::NAN);
            if let Some(column) = columns.get_mut(name) {
                column.push(value);
            }
        }
    }
    for (name, values) in &columns {
        if values.len() != 12 {
            return Err(err(format!(
                "column {name}: {} rows, expected 12",
                values.len()
            )));
        }
    }
    Ok(columns)
}

/// Pull a metadata value out of a run's summary.toml (our own writer's
/// format: `key = "value"`).
fn summary_value(summary: &str, key: &str) -> String {
    summary
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn short(hash: &str) -> &str {
    &hash[..hash.len().min(12)]
}

/// Execute `grid-cli plot`.
pub fn execute(args: &PlotArgs) -> Result<(), String> {
    match &args.chart {
        Chart::MonthlyMix { run, actual, out } => monthly_mix(run, actual, out),
        Chart::Decomposition {
            scenario,
            base_dir,
            out,
        } => decomposition(scenario, base_dir, out),
        Chart::CapacityCredit {
            scenario,
            base_dir,
            out,
        } => capacity_credit(scenario, base_dir, out),
        Chart::SocTrace { run, store, out } => soc_trace(run, store.as_deref(), out),
    }
}

// ---------------------------------------------------------------------
// capacity-credit (Stage 5 demo artefact — Module 5).
// ---------------------------------------------------------------------

/// Module 5: per-link import availability vs GB residual-demand
/// percentile.
///
/// Definitions (pinned here):
/// - **GB residual demand** = GB adjusted demand − GB weather-driven
///   potential output (the must-take renewables; exogenous series and
///   link flows excluded — residual demand is the *stress variable*,
///   what the non-weather system must cover). The top percentiles are
///   the anticyclone hours: cold, becalmed, dark.
/// - Periods are ranked by residual demand and split into 20
///   equal-count bins (5 % each).
/// - Per link and bin: the mean net import at the GB end (GW, NESO
///   convention) and its ratio to the link's derated capacity
///   (capacity × availability) — the *import availability*, a
///   capacity-credit proxy.
///
/// The story this chart exists to show (docs/07 Module 5): every
/// counterparty wind fleet is positively correlated with GB's, so
/// continental import availability FALLS toward the top percentiles,
/// while the hydro-backed NO2 border holds up.
fn capacity_credit(scenario_path: &Path, base_dir: &Path, out: &Path) -> Result<(), String> {
    let scenario = grid_core::scenario::Scenario::load(scenario_path).map_err(|e| e.to_string())?;
    if scenario.zones.len() < 2 {
        return Err("plot capacity-credit needs a multi-zone scenario (Stage 5)".to_owned());
    }
    let inputs =
        grid_adequacy::load_multi_zone_inputs(&scenario, base_dir).map_err(|e| e.to_string())?;
    println!("running the multi-zone dispatch ...");
    let result = grid_adequacy::run_multi(&scenario, &inputs).map_err(|e| e.to_string())?;
    let gb = result
        .zone("GB")
        .ok_or_else(|| "scenario has no GB zone".to_owned())?;
    let periods = gb.periods();

    // GB residual demand per period (definition above).
    let mut residual: Vec<f64> = gb.demand.iter().map(|p| p.as_gigawatts()).collect();
    for series in &gb.renewables {
        for (r, p) in residual.iter_mut().zip(&series.power) {
            *r -= p.as_gigawatts();
        }
    }
    let mut order: Vec<usize> = (0..periods).collect();
    order.sort_by(|&a, &b| {
        residual[a]
            .partial_cmp(&residual[b])
            .unwrap_or(core::cmp::Ordering::Equal)
    });

    const BINS: usize = 20;
    let derated: Vec<f64> = scenario
        .links
        .iter()
        .map(|l| l.capacity_gw.as_gigawatts() * l.availability.value())
        .collect();
    // mean net import (GW) per link per bin, plus mean residual per bin.
    let mut mean_import = vec![vec![0.0f64; BINS]; result.links.len()];
    let mut mean_residual = [0.0f64; BINS];
    let mut counts = [0usize; BINS];
    for (rank, &t) in order.iter().enumerate() {
        let bin = (rank * BINS / periods).min(BINS - 1);
        counts[bin] += 1;
        mean_residual[bin] += residual[t];
        for (link_index, link) in result.links.iter().enumerate() {
            mean_import[link_index][bin] += link.home_end[t].as_gigawatts();
        }
    }
    for bin in 0..BINS {
        let n = counts[bin].max(1) as f64;
        mean_residual[bin] /= n;
        for link in &mut mean_import {
            link[bin] /= n;
        }
    }

    let engine = env!("GRID_ENGINE_GIT_HASH");
    let scenario_sha = crate::run::sha256_file(scenario_path)?;
    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;

    // CSV (docs/06: CSV and Parquet both for tabular outputs — this is
    // a 20-row chart table; CSV alone matches the Module 1/3 artefact
    // precedent for chart-side tables).
    let mut csv = format!(
        "# grid-sim Module 5 capacity-credit table\n# engine_git_hash = {engine}\n\
         # scenario_path = {}\n# scenario_sha256 = {scenario_sha}\n\
         # bins: equal-count percentiles of GB residual demand (demand − weather-driven \
         potential)\nbin,percentile_lo,percentile_hi,mean_residual_gw",
        scenario_path.display()
    );
    for link in &result.links {
        csv.push_str(&format!(
            ",{}_mean_net_import_gw,{}_import_availability",
            link.name, link.name
        ));
    }
    csv.push('\n');
    for bin in 0..BINS {
        csv.push_str(&format!(
            "{bin},{},{},{}",
            100 * bin / BINS,
            100 * (bin + 1) / BINS,
            mean_residual[bin]
        ));
        for (link_index, _) in result.links.iter().enumerate() {
            let import = mean_import[link_index][bin];
            let availability = if derated[link_index] > 0.0 {
                import / derated[link_index]
            } else {
                0.0
            };
            csv.push_str(&format!(",{import},{availability}"));
        }
        csv.push('\n');
    }
    let csv_path = out.join("capacity_credit.csv");
    std::fs::write(&csv_path, &csv)
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // Chart: per-BORDER import availability vs percentile (links to the
    // same counterparty are summed — capacity credit is a border
    // property; per-link series live in the CSV).
    let mut borders: Vec<(String, Vec<f64>, f64)> = Vec::new(); // (name, sum import per bin, derated cap)
    for (link_index, link) in result.links.iter().enumerate() {
        let key = link.to.as_str().to_owned();
        if let Some((_, sums, cap)) = borders.iter_mut().find(|(name, _, _)| *name == key) {
            for (acc, v) in sums.iter_mut().zip(&mean_import[link_index]) {
                *acc += *v;
            }
            *cap += derated[link_index];
        } else {
            borders.push((key, mean_import[link_index].clone(), derated[link_index]));
        }
    }

    let png_path = out.join("capacity_credit.png");
    let root_area = BitMapBackend::new(&png_path, (1400, 900)).into_drawing_area();
    root_area.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root_area.split_vertically(840);
    let y_min = borders
        .iter()
        .flat_map(|(_, sums, cap)| sums.iter().map(move |s| s / cap.max(1e-9)))
        .fold(0.0f64, f64::min)
        - 0.05;
    let mut chart = ChartBuilder::on(&chart_area)
        .margin(20)
        .caption(
            "Module 5 — interconnector import availability vs GB residual-demand percentile",
            ("sans-serif", 28),
        )
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..100.0f64, y_min.min(-0.05)..1.05f64)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc("GB residual-demand percentile (100 = tightest hours)")
        .y_desc("mean net import / derated capacity")
        .draw()
        .map_err(|e| e.to_string())?;
    let palette = [
        RGBColor(60, 120, 216),
        RGBColor(230, 120, 30),
        RGBColor(0, 150, 70),
        RGBColor(150, 60, 160),
        RGBColor(200, 40, 40),
        RGBColor(90, 90, 90),
    ];
    for (which, (name, sums, cap)) in borders.iter().enumerate() {
        let color = palette[which % palette.len()];
        let points: Vec<(f64, f64)> = sums
            .iter()
            .enumerate()
            .map(|(bin, s)| (100.0 * (bin as f64 + 0.5) / BINS as f64, s / cap.max(1e-9)))
            .collect();
        chart
            .draw_series(LineSeries::new(points.clone(), color.stroke_width(3)))
            .map_err(|e| e.to_string())?
            .label(name.clone())
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 18, y)], color.stroke_width(3))
            });
        chart
            .draw_series(
                points
                    .iter()
                    .map(|&(x, y)| Circle::new((x, y), 3, color.filled())),
            )
            .map_err(|e| e.to_string())?;
    }
    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK)
        .position(SeriesLabelPosition::LowerLeft)
        .draw()
        .map_err(|e| e.to_string())?;
    footer
        .draw_text(
            &format!(
                "engine {} | scenario {} ({}) | GB residual demand = demand − weather-driven \
                 potential; 20 equal-count bins",
                short(engine),
                scenario_path.display(),
                short(&scenario_sha),
            ),
            &("sans-serif", 16).into_text_style(&footer),
            (20, 20),
        )
        .map_err(|e| e.to_string())?;
    root_area.present().map_err(|e| e.to_string())?;
    println!("wrote {} and {}", png_path.display(), csv_path.display());
    Ok(())
}

// ---------------------------------------------------------------------
// decomposition (Stage 4 demo artefact — Module 3(c)).
// ---------------------------------------------------------------------

/// Compute the storage attribution by timescale band (canonical
/// windows 24 h / 14 d / 365 d) and render the Module 3(c) chart:
/// one stacked column of the total requirement split into the four
/// bands, with the window-invariance caveats in the footer.
fn decomposition(scenario_path: &Path, base_dir: &Path, out: &Path) -> Result<(), String> {
    let scenario = grid_core::scenario::Scenario::load(scenario_path).map_err(|e| e.to_string())?;
    let inputs = grid_adequacy::load_run_inputs(&scenario, base_dir).map_err(|e| e.to_string())?;
    let windows = grid_core::analysis::DecompositionWindows::standard();
    println!("solving band attribution (four bisections over the horizon) ...");
    let attribution = grid_adequacy::attribute_storage_by_band(
        &scenario,
        &inputs,
        0,
        &windows,
        &grid_adequacy::SolveOptions::default(),
    )
    .map_err(|e| e.to_string())?;

    let engine = env!("GRID_ENGINE_GIT_HASH");
    let scenario_sha = crate::run::sha256_file(scenario_path)?;
    let data_files = crate::run::scenario_data_files(&scenario, base_dir)?;
    let windows_note = format!(
        "{} h / {} d / {} d",
        windows.diurnal.as_hours(),
        windows.synoptic.as_hours() / 24.0,
        windows.seasonal.as_hours() / 24.0
    );
    let total = attribution.total.as_gigawatt_hours();
    let meta: Vec<(String, String)> = vec![
        ("engine_git_hash".to_owned(), engine.to_owned()),
        (
            "scenario_path".to_owned(),
            scenario_path.display().to_string(),
        ),
        ("scenario_sha256".to_owned(), scenario_sha.clone()),
        ("windows".to_owned(), windows_note.clone()),
        ("total_requirement_gwh".to_owned(), total.to_string()),
        (
            "level_requirements_gwh".to_owned(),
            attribution
                .level_requirements
                .iter()
                .map(|e| e.as_gigawatt_hours().to_string())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        ("created_utc".to_owned(), crate::run::now_utc()),
    ];

    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;

    // --- CSV. ---
    let mut csv = String::from("# grid-sim Module 3(c) decomposition (docs/06 metadata header)\n");
    for (key, value) in &meta {
        csv.push_str(&format!("# {key} = {value}\n"));
    }
    for (path, hash) in &data_files {
        csv.push_str(&format!("# data_file {path} sha256={hash}\n"));
    }
    csv.push_str(
        "# method: band attribution = telescoping bisection requirements across the \
         moving-average smoothing cascade (grid_adequacy::attribution module docs); bands \
         sum to the total exactly by construction\n\
         # caveat: the synoptic/seasonal split moves with the synoptic window choice \
         (~4-7% of total across 10-21 d); the total, the diurnal band, the inter-annual \
         band and the synoptic+seasonal aggregate are window-invariant (Stage 4 \
         acceptance suite)\n",
    );
    csv.push_str("band,requirement_gwh,share_of_total\n");
    for band in &attribution.bands {
        csv.push_str(&format!(
            "{},{},{}\n",
            band.band.as_str(),
            band.requirement.as_gigawatt_hours(),
            band.requirement.as_gigawatt_hours() / total,
        ));
    }
    let csv_path = out.join("decomposition.csv");
    std::fs::write(&csv_path, &csv)
        .map_err(|e| format!("cannot write {}: {e}", csv_path.display()))?;

    // --- Parquet (docs/06: both, always). ---
    let fields = vec![
        arrow_schema::Field::new("band", arrow_schema::DataType::Utf8, false),
        arrow_schema::Field::new("requirement_gwh", arrow_schema::DataType::Float64, false),
        arrow_schema::Field::new("share_of_total", arrow_schema::DataType::Float64, false),
    ];
    let arrays: Vec<arrow_array::ArrayRef> = vec![
        std::sync::Arc::new(arrow_array::StringArray::from(
            attribution
                .bands
                .iter()
                .map(|b| b.band.as_str())
                .collect::<Vec<_>>(),
        )),
        std::sync::Arc::new(arrow_array::Float64Array::from(
            attribution
                .bands
                .iter()
                .map(|b| b.requirement.as_gigawatt_hours())
                .collect::<Vec<_>>(),
        )),
        std::sync::Arc::new(arrow_array::Float64Array::from(
            attribution
                .bands
                .iter()
                .map(|b| b.requirement.as_gigawatt_hours() / total)
                .collect::<Vec<_>>(),
        )),
    ];
    let parquet_path = out.join("decomposition.parquet");
    crate::sweep::write_table_parquet(&parquet_path, fields, arrays, &meta, &data_files)?;

    // --- The chart. ---
    let png_path = out.join("decomposition.png");
    render_decomposition_chart(
        &png_path,
        &attribution,
        engine,
        &scenario_sha,
        &windows_note,
    )?;

    for band in &attribution.bands {
        println!(
            "  band {:<12} {:>10.1} GWh ({:>5.2} % of total)",
            band.band.as_str(),
            band.requirement.as_gigawatt_hours(),
            100.0 * band.requirement.as_gigawatt_hours() / total,
        );
    }
    println!("decomposition complete: total {total:.0} GWh");
    println!("  table {}", csv_path.display());
    println!("  table {}", parquet_path.display());
    println!("  chart {}", png_path.display());
    Ok(())
}

/// One stacked column: the total requirement split into the four
/// bands, each labelled with its size and share.
fn render_decomposition_chart(
    path: &Path,
    attribution: &grid_adequacy::StorageAttribution,
    engine: &str,
    scenario_sha: &str,
    windows_note: &str,
) -> Result<(), String> {
    let total = attribution.total.as_gigawatt_hours();
    let total_twh = total / 1000.0;

    let root = BitMapBackend::new(path, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(810);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            "Module 3(c) — where the storage requirement lives: attribution by timescale band",
            ("sans-serif", 26),
        )
        .margin(20)
        .x_label_area_size(10)
        .y_label_area_size(70)
        .build_cartesian_2d(0.0..3.2f64, 0.0..(total_twh * 1.06))
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_x_axis()
        .y_desc("storage requirement, TWh (store-side, D4 convention)")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    // Stack order bottom-up: inter-annual (slowest) at the base.
    let colours = [
        RGBColor(240, 190, 60),  // diurnal — solar gold
        RGBColor(110, 170, 220), // synoptic — sky blue
        RGBColor(40, 90, 160),   // seasonal — deep blue
        RGBColor(90, 90, 100),   // inter-annual — slate
    ];
    let mut cumulative = 0.0f64;
    for index in (0..4).rev() {
        let band = &attribution.bands[index];
        let height = band.requirement.as_gigawatt_hours() / 1000.0;
        let base = cumulative;
        cumulative += height;
        chart
            .draw_series(std::iter::once(Rectangle::new(
                [(0.4, base), (1.4, base + height)],
                colours[index].filled(),
            )))
            .map_err(|e| e.to_string())?;
        // Label to the right of the column, at the segment's centre
        // (stacked separately for zero-height segments).
        let label_y = if height > total_twh * 0.04 {
            base + height / 2.0
        } else {
            base + total_twh * 0.02
        };
        chart
            .draw_series(std::iter::once(Text::new(
                format!(
                    "{}: {:.1} TWh ({:.1} %)",
                    band.band.as_str().replace('_', "-"),
                    height,
                    100.0 * band.requirement.as_gigawatt_hours() / total,
                ),
                (1.5, label_y),
                ("sans-serif", 20).into_font().color(&BLACK),
            )))
            .map_err(|e| e.to_string())?;
    }
    chart
        .draw_series(std::iter::once(Text::new(
            format!("total: {total_twh:.1} TWh"),
            (0.4, total_twh * 1.02),
            ("sans-serif", 22).into_font().color(&BLACK),
        )))
        .map_err(|e| e.to_string())?;

    let caption = format!(
        "grid-sim | engine {} | scenario sha256 {} | MA windows {windows_note} | bands sum \
         exactly; synoptic/seasonal split is window-sensitive (~4-7% over 10-21 d) — \
         diurnal, inter-annual and the total are invariant",
        short(engine),
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
// soc-trace (Stage 3 demo artefact).
// ---------------------------------------------------------------------

/// One store's SoC series pulled out of dispatch.csv.
struct SocSeries {
    /// Store output label (the `_soc_gwh` column minus its suffix).
    label: String,
    /// End-of-period SoC, GWh, one value per settlement period.
    soc_gwh: Vec<f64>,
}

/// The parts of a run's dispatch.csv the soc-trace chart needs: the
/// `#` metadata header (docs/06 — carried into soc_trace.csv verbatim),
/// the per-period timestamps and every `*_soc_gwh` column.
struct DispatchSoc {
    metadata: Vec<String>,
    timestamps: Vec<String>,
    stores: Vec<SocSeries>,
}

fn read_dispatch_soc(path: &Path) -> Result<DispatchSoc, String> {
    let err = |m: String| format!("{}: {m}", path.display());
    let text = std::fs::read_to_string(path).map_err(|e| err(e.to_string()))?;
    let metadata: Vec<String> = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    let mut lines = text.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| err("empty file".to_owned()))?
        .split(',')
        .collect();
    if header.first() != Some(&"utc_start") {
        return Err(err("first column is not utc_start".to_owned()));
    }
    // The engine-emitted SoC columns (the same ones the viewer renders).
    let soc_columns: Vec<(usize, String)> = header
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            name.strip_suffix("_soc_gwh")
                .map(|label| (index, label.to_owned()))
        })
        .collect();
    if soc_columns.is_empty() {
        return Err(err(
            "no *_soc_gwh columns — the run has no storage, so there is no SoC to plot".to_owned(),
        ));
    }

    let mut timestamps = Vec::new();
    let mut stores: Vec<SocSeries> = soc_columns
        .iter()
        .map(|(_, label)| SocSeries {
            label: label.clone(),
            soc_gwh: Vec::new(),
        })
        .collect();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let fields: Vec<&str> = line.split(',').collect();
        timestamps.push(
            fields
                .first()
                .ok_or_else(|| err("empty data row".to_owned()))?
                .to_string(),
        );
        for ((index, _), series) in soc_columns.iter().zip(&mut stores) {
            let field = fields
                .get(*index)
                .ok_or_else(|| err(format!("row with {} fields", fields.len())))?;
            let value = field
                .parse::<f64>()
                .map_err(|e| err(format!("bad SoC value {field:?}: {e}")))?;
            series.soc_gwh.push(value);
        }
    }
    if timestamps.is_empty() {
        return Err(err("no data rows".to_owned()));
    }
    Ok(DispatchSoc {
        metadata,
        timestamps,
        stores,
    })
}

/// Distinct series colours for up to a handful of stores.
const SOC_COLORS: [RGBColor; 4] = [
    RGBColor(0, 110, 160),  // hydrogen-ish blue
    RGBColor(220, 120, 30), // orange
    RGBColor(60, 150, 80),  // green
    RGBColor(150, 80, 160), // purple
];

fn soc_trace(run: &Path, store: Option<&str>, out: &Path) -> Result<(), String> {
    let dispatch = read_dispatch_soc(&run.join("dispatch.csv"))?;
    let summary = std::fs::read_to_string(run.join("summary.toml"))
        .map_err(|e| format!("{}: {e}", run.join("summary.toml").display()))?;

    let stores: Vec<&SocSeries> = match store {
        Some(wanted) => {
            let found: Vec<&SocSeries> = dispatch
                .stores
                .iter()
                .filter(|s| s.label == wanted)
                .collect();
            if found.is_empty() {
                return Err(format!(
                    "--store {wanted:?} does not name a store in this run; available: [{}]",
                    dispatch
                        .stores
                        .iter()
                        .map(|s| s.label.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            found
        }
        None => dispatch.stores.iter().collect(),
    };

    // The companion CSV: the run's own metadata header (verbatim — it
    // already carries the engine/scenario/data-file hashes, docs/06)
    // plus the SoC series alone.
    let mut csv = String::new();
    for line in &dispatch.metadata {
        csv.push_str(line);
        csv.push('\n');
    }
    csv.push_str("utc_start");
    for series in &stores {
        csv.push_str(&format!(",{}_soc_gwh", series.label));
    }
    csv.push('\n');
    for (t, stamp) in dispatch.timestamps.iter().enumerate() {
        csv.push_str(stamp);
        for series in &stores {
            csv.push_str(&format!(",{}", series.soc_gwh[t]));
        }
        csv.push('\n');
    }
    let csv_path = run.join("soc_trace.csv");
    std::fs::write(&csv_path, &csv).map_err(|e| format!("{}: {e}", csv_path.display()))?;

    // The chart: full per-period series, x in days since the run start
    // labelled with calendar dates (no decimation — the deep drawdown
    // minima are the point of the artefact).
    let periods = dispatch.timestamps.len();
    let days_total = periods as f64 * 0.5 / 24.0;
    let start = grid_core::time::UtcInstant::parse(&dispatch.timestamps[0])
        .map_err(|e| format!("dispatch.csv utc_start: {e}"))?;
    let y_max = stores
        .iter()
        .flat_map(|s| s.soc_gwh.iter())
        .fold(0.0_f64, |acc, &v| acc.max(v))
        .max(1.0)
        * 1.05;

    let root = BitMapBackend::new(out, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(830);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            format!(
                "Storage state of charge, {} .. {}",
                &dispatch.timestamps[0][..10],
                &dispatch.timestamps[periods - 1][..10]
            ),
            ("sans-serif", 28),
        )
        .margin(20)
        .x_label_area_size(45)
        .y_label_area_size(80)
        .build_cartesian_2d(0.0..days_total, 0.0..y_max)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .x_desc("")
        .y_desc("state of charge, GWh")
        .x_label_formatter(&|days| {
            // Calendar label at this day offset: bare years once the
            // horizon is long enough for the default ~10 ticks to land
            // in distinct years (> ~8 years), year-month below that.
            let instant = start.plus_periods((days * 48.0) as i64);
            let (year, month, _) = instant.civil_date();
            if days_total > 3_000.0 {
                format!("{year}")
            } else {
                format!("{year}-{month:02}")
            }
        })
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    for (which, series) in stores.iter().enumerate() {
        let color = SOC_COLORS[which % SOC_COLORS.len()];
        chart
            .draw_series(LineSeries::new(
                series
                    .soc_gwh
                    .iter()
                    .enumerate()
                    .map(|(t, &soc)| (t as f64 * 0.5 / 24.0, soc)),
                &color,
            ))
            .map_err(|e| e.to_string())?
            .label(&series.label)
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + 18, y)], color.stroke_width(3))
            });
    }
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    // Footer caption: engine + scenario hashes (docs/06).
    let caption = format!(
        "grid-sim | engine {} | scenario sha256 {} | {} periods | generated {}",
        summary_value(&summary, "engine_git_hash"),
        short(&summary_value(&summary, "scenario_sha256")),
        periods,
        summary_value(&summary, "created_utc"),
    );
    footer
        .draw(&Text::new(
            caption,
            (20, 25),
            ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
        ))
        .map_err(|e| e.to_string())?;

    root.present().map_err(|e| e.to_string())?;
    println!(
        "chart written to {} (SoC series in {})",
        out.display(),
        csv_path.display()
    );
    Ok(())
}

fn monthly_mix(run: &Path, actual: &Path, out: &Path) -> Result<(), String> {
    let model = read_monthly_csv(&run.join("monthly_mix.csv"))?;
    let observed = read_monthly_csv(actual)?;
    let summary = std::fs::read_to_string(run.join("summary.toml"))
        .map_err(|e| format!("{}: {e}", run.join("summary.toml").display()))?;

    let model_fuel = |columns: &[&str], month: usize| -> Result<f64, String> {
        columns.iter().try_fold(0.0, |acc, col| {
            Ok(acc
                + model
                    .get(*col)
                    .ok_or_else(|| format!("monthly_mix.csv has no column {col}"))?[month])
        })
    };
    let actual_fuel = |column: &str, month: usize| -> Result<f64, String> {
        Ok(observed
            .get(column)
            .ok_or_else(|| format!("{} has no column {column}", actual.display()))?[month])
    };

    // Stack totals set the y range (TWh).
    let mut y_max: f64 = 0.0;
    for month in 0..12 {
        let mut model_total = 0.0;
        let mut actual_total = 0.0;
        for (_, model_cols, actual_col, _) in FUELS {
            model_total += model_fuel(model_cols, month)?;
            actual_total += actual_fuel(actual_col, month)?;
        }
        y_max = y_max.max(model_total).max(actual_total);
    }
    let y_max = (y_max / 1000.0) * 1.12;

    let root = BitMapBackend::new(out, (1400, 900)).into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    let (chart_area, footer) = root.split_vertically(830);

    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            "GB 2024 monthly generation stack — modelled (left) vs actual (right)",
            ("sans-serif", 28),
        )
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..12.0, 0.0..y_max)
        .map_err(|e| e.to_string())?;
    chart
        .configure_mesh()
        .disable_x_mesh()
        .x_labels(0)
        .y_desc("TWh")
        .axis_desc_style(("sans-serif", 18))
        .label_style(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    // Month names centred under each bar group (plotters ticks land on
    // group boundaries, so they are drawn manually).
    for (month, name) in MONTHS.iter().enumerate() {
        let (x, y) = chart.backend_coord(&(month as f64 + 0.5, 0.0));
        chart_area
            .draw(&Text::new(
                *name,
                (x - 12, y + 8),
                ("sans-serif", 16).into_font(),
            ))
            .map_err(|e| e.to_string())?;
    }

    // Bars. For month m (0-based): modelled bar spans x [m+0.08, m+0.48],
    // actual bar [m+0.52, m+0.92]; both stack the fuels bottom-up.
    for (name, model_cols, actual_col, color) in FUELS {
        let mut rects = Vec::with_capacity(24);
        for month in 0..12 {
            let x = month as f64;
            let mut base_model = 0.0;
            let mut base_actual = 0.0;
            for (_, prior_cols, prior_actual, _) in FUELS.iter().take_while(|f| f.0 != name) {
                base_model += model_fuel(prior_cols, month)?;
                base_actual += actual_fuel(prior_actual, month)?;
            }
            let model_twh = model_fuel(model_cols, month)? / 1000.0;
            let actual_twh = actual_fuel(actual_col, month)? / 1000.0;
            let base_model = base_model / 1000.0;
            let base_actual = base_actual / 1000.0;
            rects.push(Rectangle::new(
                [(x + 0.08, base_model), (x + 0.48, base_model + model_twh)],
                color.filled(),
            ));
            rects.push(Rectangle::new(
                [
                    (x + 0.52, base_actual),
                    (x + 0.92, base_actual + actual_twh),
                ],
                color.mix(0.55).filled(),
            ));
        }
        chart
            .draw_series(rects)
            .map_err(|e| e.to_string())?
            .label(name)
            .legend(move |(x, y)| Rectangle::new([(x, y - 6), (x + 14, y + 6)], color.filled()));
    }
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .border_style(BLACK)
        .background_style(WHITE.mix(0.9))
        .label_font(("sans-serif", 16))
        .draw()
        .map_err(|e| e.to_string())?;

    // Footer caption: engine + scenario hashes (docs/06; schema v2 —
    // the scenario is self-contained, no run-inputs hash exists).
    let caption = format!(
        "grid-sim | engine {} | scenario sha256 {} | actuals: Elexon FUELHH / NESO estimates (D3 convention) | generated {}",
        summary_value(&summary, "engine_git_hash"),
        short(&summary_value(&summary, "scenario_sha256")),
        summary_value(&summary, "created_utc"),
    );
    footer
        .draw(&Text::new(
            caption,
            (20, 25),
            ("sans-serif", 15).into_font().color(&BLACK.mix(0.7)),
        ))
        .map_err(|e| e.to_string())?;

    root.present().map_err(|e| e.to_string())?;
    println!("chart written to {}", out.display());
    Ok(())
}
