//! # grid-cli — research CLI
//!
//! Responsibilities per ADR-1 and ADR-11 (`docs/02-architecture.md`): the
//! phase-one research instrument. Subcommands per docs/06: `run`, `sweep`,
//! `solve`, `validate`, `fetch-data`, `plot`, `stability`.
//!
//! Stage 0 implemented `validate` (scenario parse check, plus `--summary`
//! for the demo artefact). Stage 1 implements `run` (chronological
//! dispatch; see the [`run`] module) and `plot` (the monthly-mix demo
//! chart; see the [`plot`] module). Stage 2 extends `run` with the
//! pricing outputs and implements `sweep wind-capacity` (the Module 1
//! demo artefact). Stage 4 implements the generic sweep runner
//! (`sweep grid`), the Q4 per-year batch (`sweep per-year`) and the
//! Module 3(c) decomposition chart (`plot decomposition`) — see the
//! [`sweep`] and [`plot`] modules. `fetch-data` builds the local data
//! pack (the Rust port of the provisional Python builders — see the
//! [`fetchdata`] module). Stage 6 adds `stability` (swing-equation
//! event runner + minimum-inertia hour finder — see the [`stability`]
//! module; docs/06 subcommand list updated and ratified 2026-07-03).
//!
//! Exit codes (docs/06): 0 success, 1 model infeasibility (`solve` and
//! `fetch-data`, via `solve::Failure::exit_code`),
//! 2 usage/scenario error. This is the process top level: `unwrap`/
//! `expect` and direct exits are permitted here (docs/06), and it is the
//! one layer allowed to read the wall clock for output timestamps (ADR-5).

mod fetchdata;
mod plot;
mod run;
mod solve;
mod stability;
mod sweep;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use grid_core::GridError;
use grid_core::scenario::Scenario;
use grid_core::trace::{load_per_unit_trace, load_power_trace_mw_concat};

/// grid-sim: GB electricity system simulator (energy adequacy + system
/// stability).
#[derive(Parser)]
#[command(name = "grid-cli", version, about)]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Run a scenario: chronological half-hourly dispatch, writing CSV +
    /// Parquet dispatch, a monthly-mix table, and a run summary.
    Run(run::RunArgs),
    /// Run a parameter sweep: `grid` (generic 1-D/2-D sweep from a TOML
    /// spec, full response surface persisted), `heating-mix` (the
    /// Q5/Q11 D9 rule-6/6b simplex sweep + decomposition), `per-year`
    /// (the Q4 batch), or `wind-capacity` (the Stage 2 Module 1
    /// artefact).
    Sweep(sweep::SweepArgs),
    /// Solve for a target: `min_storage_for_zero_unserved` bisection
    /// over one store's energy capacity (Stage 3, ADR-10).
    Solve(solve::SolveArgs),
    /// Parse and check a scenario file; with --summary, print the Stage 0
    /// demo artefact (scenario summary and trace statistics).
    Validate(ValidateArgs),
    /// Fetch and build the local data pack (NESO demand, Elexon FUELHH
    /// generation, provisional wind CF, ONS daily gas SAP), then validate
    /// it; optionally compare cell-exact against a reference pack. The
    /// ERA5 CF pipeline stays in Python (scripts/era5-cf) by design.
    FetchData(fetchdata::FetchDataArgs),
    /// Stage 6 stability engine: `event` (swing-equation loss-of-infeed
    /// simulation from an event spec), `inertia` (minimum-inertia hour
    /// finder over a scenario's dispatch), `pathway` (Q8; Stage 6
    /// part 2 stub).
    Stability(stability::StabilityArgs),
    /// Render charts from run outputs (PNG with metadata footer).
    Plot(plot::PlotArgs),
}

#[derive(Args)]
struct ValidateArgs {
    /// Scenario TOML file to check.
    #[arg(long)]
    scenario: PathBuf,

    /// Print the scenario summary and trace statistics (Stage 0 demo
    /// artefact). Scenario-referenced trace paths are resolved against
    /// the working directory; placeholders that do not exist are noted,
    /// not fatal.
    #[arg(long)]
    summary: bool,

    /// Demand trace column to summarise (the data-pack demand Parquet is
    /// multi-column; docs/03 D3 convention).
    #[arg(long, default_value = "underlying_demand")]
    demand_column: String,

    /// Extra trace to summarise as a dimensionless series, as
    /// PATH:COLUMN (repeatable). Example:
    /// data/packs/2024/processed/wind_cf_2024.parquet:wind_cf
    #[arg(long, value_name = "PATH:COLUMN")]
    trace: Vec<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // `solve` and `fetch-data` distinguish failed-model/failed-pack
    // outcomes (exit 1, docs/06) from usage/scenario errors (exit 2);
    // everything else exits 0 or 2.
    if let CliCommand::Solve(args) = &cli.command {
        return exit_with(solve::execute(args));
    }
    if let CliCommand::FetchData(args) = &cli.command {
        return exit_with(fetchdata::execute(args));
    }
    let result = match cli.command {
        CliCommand::Run(args) => run::execute(&args),
        CliCommand::Sweep(args) => sweep::execute(&args),
        CliCommand::Solve(_) | CliCommand::FetchData(_) => unreachable!("handled above"),
        CliCommand::Stability(args) => stability::execute(&args),
        CliCommand::Plot(args) => plot::execute(&args),
        CliCommand::Validate(args) => validate(&args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

/// Exit-code mapping for subcommands using the `Failure` shape (solve,
/// fetch-data).
fn exit_with(result: Result<(), solve::Failure>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("error: {}", failure.message);
            ExitCode::from(failure.exit_code)
        }
    }
}

fn validate(args: &ValidateArgs) -> Result<(), String> {
    let scenario = Scenario::load(&args.scenario).map_err(|e| e.to_string())?;
    // Semantic checks beyond strict parsing (duplicate dispatch_order,
    // out-of-range storage parameters, …).
    scenario.validate().map_err(|e| e.to_string())?;
    println!(
        "scenario OK: {} (schema_version {})",
        scenario.name, scenario.schema_version
    );

    if !args.summary {
        return Ok(());
    }
    print_summary(&scenario)?;
    print_trace_statistics(&scenario, &args.demand_column, &args.trace)
}

fn print_summary(scenario: &Scenario) -> Result<(), String> {
    if let Some(description) = &scenario.description {
        println!("  {description}");
    }
    let periods = scenario.horizon.period_count().map_err(|e| e.to_string())?;
    println!();
    println!(
        "Horizon: {} .. {} ({periods} half-hourly periods, weather years {:?})",
        scenario.horizon.start, scenario.horizon.end, scenario.horizon.weather_years
    );

    for zone in &scenario.zones {
        println!("Zone {}:", zone.id);
        println!(
            "  demand: {} x {} (heating overlay: {})",
            zone.demand.base_profile,
            zone.demand.annual_scale,
            match &zone.demand.heating {
                Some(h) => format!(
                    "{} TWh x {} electrified, {} entries",
                    h.delivered_heat_twh.as_gigawatt_hours() / 1000.0,
                    h.electrified_share.value(),
                    h.entries.len()
                ),
                None => "absent".to_owned(),
            }
        );
        let total: f64 = zone
            .fleet
            .iter()
            .map(|t| t.capacity_gw.as_gigawatts())
            .sum();
        println!(
            "  fleet ({} technologies, {total:.1} GW):",
            zone.fleet.len()
        );
        for tech in &zone.fleet {
            println!(
                "    {:<15} {:>6.1} GW{}",
                tech.technology,
                tech.capacity_gw.as_gigawatts(),
                match &tech.capacity_factor_trace {
                    Some(trace) => format!("  cf trace: {trace}"),
                    None => String::new(),
                }
            );
        }
        println!("  storage ({} stores):", zone.storage.len());
        for store in &zone.storage {
            println!(
                "    {:<15} {:>6.1} GW  {:>8.1} GWh  rte {:.2}  order {}",
                store.kind,
                store.power_gw.as_gigawatts(),
                store.energy_gwh.as_gigawatt_hours(),
                store.round_trip_efficiency.value(),
                store.dispatch_order
            );
        }
    }

    println!("Links ({}):", scenario.links.len());
    for link in &scenario.links {
        println!(
            "  {} -> {:<4} {:>5.1} GW  availability {:.2}",
            link.from,
            link.to,
            link.capacity_gw.as_gigawatts(),
            link.availability.value()
        );
    }
    println!("Dispatch policy: {}", scenario.dispatch.policy);
    Ok(())
}

fn print_trace_statistics(
    scenario: &Scenario,
    demand_column: &str,
    extra: &[String],
) -> Result<(), String> {
    let periods = scenario.horizon.period_count().map_err(|e| e.to_string())?;
    println!();
    println!("Trace statistics ({periods} periods expected from horizon):");

    for zone in &scenario.zones {
        let paths: Vec<PathBuf> = zone
            .demand
            .base_profile
            .paths()
            .iter()
            .map(PathBuf::from)
            .collect();
        match load_power_trace_mw_concat(&paths, demand_column, periods) {
            Ok(trace) => println!(
                "  {}[{demand_column}]: {} periods from {}, mean {:.2} GW, min {:.2} GW, max {:.2} GW",
                zone.demand.base_profile,
                trace.len(),
                trace.start(),
                trace.mean().map_or(f64::NAN, |v| v.as_gigawatts()),
                trace.min().map_or(f64::NAN, |v| v.as_gigawatts()),
                trace.max().map_or(f64::NAN, |v| v.as_gigawatts()),
            ),
            Err(GridError::TraceFileMissing { .. }) => println!(
                "  {}: not found (placeholder path? data packs are fetched, not committed)",
                zone.demand.base_profile
            ),
            Err(other) => return Err(other.to_string()),
        }

        for tech in &zone.fleet {
            let Some(cf_trace) = &tech.capacity_factor_trace else {
                continue;
            };
            if cf_trace.paths().iter().any(|p| !PathBuf::from(p).exists()) {
                println!(
                    "  {cf_trace}: not found (placeholder path? data packs are fetched, not committed)"
                );
                continue;
            }
            // CF traces use the pinned `cf` column (Stage 1 convention);
            // the summary reports presence without loading them.
            println!("  {cf_trace}: present (column convention `cf`, pinned in Stage 1)");
        }
    }

    for spec in extra {
        let Some((path, column)) = spec.rsplit_once(':') else {
            return Err(format!("--trace {spec}: expected PATH:COLUMN"));
        };
        let trace =
            load_per_unit_trace(path.as_ref(), column, periods).map_err(|e| e.to_string())?;
        println!(
            "  {path}[{column}]: {} periods from {}, mean {:.3}, min {:.3}, max {:.3}",
            trace.len(),
            trace.start(),
            trace.mean().map_or(f64::NAN, |v| v.value()),
            trace.min().map_or(f64::NAN, |v| v.value()),
            trace.max().map_or(f64::NAN, |v| v.value()),
        );
    }
    Ok(())
}
