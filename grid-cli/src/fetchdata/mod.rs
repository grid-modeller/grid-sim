//! `grid-cli fetch-data` — fetch and build the local data pack
//! (docs/05: the tool fetches and builds its data pack rather than
//! shipping it; docs/06 subcommand list).
//!
//! The Rust port of the provisional Python builders:
//!
//! - `scripts/fetch-2024/{fetch,build,validate}.py` — NESO Historic
//!   Demand + Elexon Insights FUELHH → `demand_<year>`,
//!   `generation_by_fuel_<year>`, `wind_cf_<year>`;
//! - the ONS gas-SAP part of `scripts/fetch-prices/` →
//!   `gas_sap_daily_<year>` (the rest of the price pack — Elexon MID and
//!   imbalance prices — is a later increment).
//!
//! The ERA5 capacity-factor pipeline (`scripts/era5-cf/`) is *not*
//! ported: it remains Python by design (Zarr-over-GCS plus the pinned
//! scientific stack are disproportionate to reimplement; see the
//! documented exception proposed for docs/05).
//!
//! Layout: raw responses land in `<out>/raw/` (existing files are never
//! re-fetched — Elexon revises, so a fetched file is authoritative);
//! processed CSV + Parquet land in `<out>/processed/`. Sources are
//! pinned per year in [`sources`]; only 2024 is pinned so far.
//!
//! Exit codes (docs/06 applied to a data tool): 0 success; 1 the built
//! pack failed validation or the `--compare-with` harness found a value
//! difference (the pack is unusable as a reference — analogous to model
//! infeasibility); 2 usage errors and I/O/network failures.

mod build;
mod checks;
mod error;
mod gbtime;
mod net;
pub(crate) mod table;

use std::path::{Path, PathBuf};

use clap::Args;

pub use error::FetchDataError;

use crate::solve::Failure;

/// Arguments for `grid-cli fetch-data`.
#[derive(Args)]
pub struct FetchDataArgs {
    /// Pack year (sources are pinned per year; only 2024 so far).
    #[arg(long, default_value_t = 2024)]
    year: u16,

    /// Pack directory: raw responses in `<out>/raw`, processed traces in
    /// `<out>/processed`. Required (no default) so a rebuild into the
    /// canonical `data/packs/<year>` — whose file checksums are pinned in
    /// the committed manifest — is always an explicit decision.
    #[arg(long)]
    out: PathBuf,

    /// Skip the network phase and build from already-fetched raw files.
    #[arg(long)]
    skip_fetch: bool,

    /// Compare every built output cell-by-cell against a reference
    /// processed directory (bit-level f64 / exact int identity); any
    /// difference is reported and exits 1.
    #[arg(long, value_name = "PROCESSED_DIR")]
    compare_with: Option<PathBuf>,
}

/// Pinned per-year sources (fixed URLs and date ranges — ADR-5: the pack
/// is a deterministic function of these pins).
struct Sources {
    /// NESO Data Portal "Historic Demand Data" CSV (NESO Open Data
    /// Licence).
    neso_demand_url: &'static str,
    /// Raw filename for the NESO CSV.
    neso_demand_file: String,
    /// ONS "System Average Price (SAP) of gas" xlsx (OGL v3.0).
    ons_sap_url: &'static str,
    /// Raw filename for the ONS xlsx.
    ons_sap_file: &'static str,
    /// Elexon Insights FUELHH settlement-date ranges (BMRS open-data
    /// licence), monthly-chunked with one padding day each side.
    fuelhh_ranges: Vec<(String, String)>,
    /// NESO Data Portal "System Inertia" CSV, 2023-24 edition (NESO Open
    /// Data Licence). Together with `neso_inertia_curr_url` covers
    /// calendar 2024.
    neso_inertia_prev_url: &'static str,
    /// Raw filename for the 2023-24 NESO System Inertia CSV.
    neso_inertia_prev_file: &'static str,
    /// NESO Data Portal "System Inertia" CSV, 2024-25 edition (NESO Open
    /// Data Licence). Together with `neso_inertia_prev_url` covers
    /// calendar 2024.
    neso_inertia_curr_url: &'static str,
    /// Raw filename for the 2024-25 NESO System Inertia CSV.
    neso_inertia_curr_file: &'static str,
}

fn sources(year: u16) -> Result<Sources, FetchDataError> {
    if year != 2024 {
        return Err(FetchDataError::UnsupportedYear {
            year,
            supported: "2024",
        });
    }
    // Monthly chunks in settlement dates (Europe/London clock days), plus
    // one day each side so UTC-year trimming has full cover around the
    // settlement-day/UTC-day offset.
    let month_ends = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut fuelhh_ranges = vec![("2023-12-31".to_owned(), "2023-12-31".to_owned())];
    for (month, end) in (1..=12).zip(month_ends) {
        fuelhh_ranges.push((
            format!("2024-{month:02}-01"),
            format!("2024-{month:02}-{end:02}"),
        ));
    }
    fuelhh_ranges.push(("2025-01-01".to_owned(), "2025-01-01".to_owned()));
    Ok(Sources {
        neso_demand_url: "https://api.neso.energy/dataset/8f2fe0af-871c-488d-8bad-960426f24601/\
                          resource/f6d02c0f-957b-48cb-82ee-09003f2ba759/download/demanddata_2024.csv",
        neso_demand_file: format!("demanddata_{year}.csv"),
        ons_sap_url: "https://www.ons.gov.uk/file?uri=/economy/economicoutputandproductivity/\
                      output/datasets/systemaveragepricesapofgas/2024/\
                      systemaveragepriceofgasdataset090125.xlsx",
        ons_sap_file: "ons_sap_of_gas_090125.xlsx",
        fuelhh_ranges,
        neso_inertia_prev_url: "https://api.neso.energy/dataset/8f3cd0ce-6636-469e-b582-55eadfeaa1d9/\
                                resource/5bd6ec4d-a2df-4c94-9b27-fdf8cf04d7dd/download/inertia.csv",
        neso_inertia_prev_file: "neso_inertia_2023_2024.csv",
        neso_inertia_curr_url: "https://api.neso.energy/dataset/8f3cd0ce-6636-469e-b582-55eadfeaa1d9/\
                                resource/7a12d0bd-448d-42a9-b333-4a32761dbad4/download/inertia.csv",
        neso_inertia_curr_file: "neso_inertia_2024_2025.csv",
    })
}

const ELEXON_FUELHH_STREAM: &str = "https://data.elexon.co.uk/bmrs/api/v1/datasets/FUELHH/stream";

/// Run the subcommand: fetch (unless skipped), build, write, validate,
/// and optionally compare.
pub fn execute(args: &FetchDataArgs) -> Result<(), Failure> {
    let usage = |e: FetchDataError| Failure::usage(e.to_string());
    let sources = sources(args.year).map_err(usage)?;

    let raw_dir = args.out.join("raw");
    let processed_dir = args.out.join("processed");
    std::fs::create_dir_all(&raw_dir)
        .map_err(|e| Failure::usage(format!("cannot create {}: {e}", raw_dir.display())))?;
    std::fs::create_dir_all(&processed_dir)
        .map_err(|e| Failure::usage(format!("cannot create {}: {e}", processed_dir.display())))?;

    if !args.skip_fetch {
        fetch_raw(&sources, &raw_dir).map_err(usage)?;
    }

    // Build all five processed tables.
    let demand =
        build::build_demand(&raw_dir.join(&sources.neso_demand_file), args.year).map_err(usage)?;
    let generation = build::build_generation(&raw_dir, args.year).map_err(usage)?;
    let wind_cf = build::build_wind_cf(&demand.table, &generation).map_err(usage)?;
    let gas_sap =
        build::build_gas_sap(&raw_dir.join(sources.ons_sap_file), args.year).map_err(usage)?;
    let inertia_outturn = build::build_inertia_outturn(&raw_dir, args.year).map_err(usage)?;

    if wind_cf.clamped_below > 0 || wind_cf.clamped_above > 0 {
        println!(
            "wind_cf: clamped {} periods <0 and {} periods >1 (raw range {:.4}..{:.4})",
            wind_cf.clamped_below, wind_cf.clamped_above, wind_cf.raw_min, wind_cf.raw_max
        );
    } else {
        println!(
            "wind_cf: no clamping needed (range {:.4}..{:.4})",
            wind_cf.raw_min, wind_cf.raw_max
        );
    }

    let year = args.year;
    let outputs: [(String, &table::Table); 5] = [
        (format!("demand_{year}"), &demand.table),
        (format!("generation_by_fuel_{year}"), &generation),
        (format!("wind_cf_{year}"), &wind_cf.table),
        (format!("gas_sap_daily_{year}"), &gas_sap),
        (format!("inertia_outturn_{year}"), &inertia_outturn),
    ];
    for (stem, built) in &outputs {
        built
            .write_csv(&processed_dir.join(format!("{stem}.csv")))
            .map_err(usage)?;
        built
            .write_parquet(&processed_dir.join(format!("{stem}.parquet")))
            .map_err(usage)?;
        println!("built {stem}: {} periods", built.len());
    }

    // Validate the built pack (port of the Python validators).
    let failures = checks::validate(
        args.year,
        &demand.table,
        &generation,
        &wind_cf.table,
        &gas_sap,
        &inertia_outturn,
        &demand.settlement_day_periods,
    )
    .map_err(usage)?;
    if !failures.is_empty() {
        println!("\nFAILURES:");
        for failure in &failures {
            println!("  - {failure}");
        }
        return Err(Failure {
            message: format!("pack failed validation ({} checks)", failures.len()),
            exit_code: 1,
        });
    }
    println!("\nAll validation checks passed.");

    // Optional acceptance harness: cell-exact identity vs a reference.
    if let Some(reference_dir) = &args.compare_with {
        compare_outputs(&outputs, reference_dir)?;
    }
    Ok(())
}

/// Fetch every pinned raw source into `raw_dir` (skipping files already
/// present), verifying the FUELHH responses are non-empty JSON arrays
/// before accepting them.
fn fetch_raw(sources: &Sources, raw_dir: &Path) -> Result<(), FetchDataError> {
    net::get_to_file(
        sources.neso_demand_url,
        &raw_dir.join(&sources.neso_demand_file),
    )?;
    net::get_to_file(
        sources.neso_inertia_prev_url,
        &raw_dir.join(sources.neso_inertia_prev_file),
    )?;
    net::get_to_file(
        sources.neso_inertia_curr_url,
        &raw_dir.join(sources.neso_inertia_curr_file),
    )?;
    for (from, to) in &sources.fuelhh_ranges {
        let path = raw_dir.join(format!("fuelhh_{from}_{to}.json"));
        if path.exists() {
            println!("skip (exists): {}", path.display());
            continue;
        }
        let url = format!("{ELEXON_FUELHH_STREAM}?settlementDateFrom={from}&settlementDateTo={to}");
        let body = net::get(&url)?;
        // Shape check before accepting (fetch.py raises on non-list/empty).
        let parsed: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| FetchDataError::BadResponse {
                url: url.clone(),
                reason: format!("not JSON: {e}"),
            })?;
        let records = parsed
            .as_array()
            .ok_or_else(|| FetchDataError::BadResponse {
                url: url.clone(),
                reason: "expected a JSON array".to_owned(),
            })?;
        if records.is_empty() {
            return Err(FetchDataError::BadResponse {
                url,
                reason: "empty FUELHH response".to_owned(),
            });
        }
        let tmp = path.with_extension("part");
        std::fs::write(&tmp, &body).map_err(|source| FetchDataError::io(&tmp, source))?;
        std::fs::rename(&tmp, &path).map_err(|source| FetchDataError::io(&path, source))?;
        println!("fetched {} ({} records)", path.display(), records.len());
    }
    net::get_to_file(sources.ons_sap_url, &raw_dir.join(sources.ons_sap_file))?;
    Ok(())
}

/// Compare the built outputs against a reference processed directory,
/// printing a per-stem verdict; any mismatch exits 1.
fn compare_outputs(
    outputs: &[(String, &table::Table)],
    reference_dir: &Path,
) -> Result<(), Failure> {
    println!(
        "\nCell-exact comparison against {}:",
        reference_dir.display()
    );
    let mut total_mismatches = 0usize;
    for (stem, built) in outputs {
        let reference_path = reference_dir.join(format!("{stem}.parquet"));
        let report =
            checks::compare(built, &reference_path).map_err(|e| Failure::usage(e.to_string()))?;
        if report.mismatches.is_empty() {
            println!(
                "  {stem}: identical ({} rows, {} columns, {} cells bit-exact)",
                built.len(),
                built.columns.len(),
                report.cells
            );
        } else {
            println!("  {stem}: {} mismatch line(s):", report.mismatches.len());
            for line in &report.mismatches {
                println!("    - {line}");
            }
            total_mismatches += report.mismatches.len();
        }
    }
    if total_mismatches > 0 {
        return Err(Failure {
            message: format!(
                "comparison failed: {total_mismatches} mismatch line(s) vs {}",
                reference_dir.display()
            ),
            exit_code: 1,
        });
    }
    println!("  verdict: cell-exact numerical identity for all outputs");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn only_pinned_years_are_accepted() {
        assert!(sources(2024).is_ok());
        let Err(err) = sources(2023) else {
            panic!("2023 has no pinned sources and must be refused")
        };
        assert!(
            matches!(err, FetchDataError::UnsupportedYear { .. }),
            "{err}"
        );
    }

    #[test]
    fn fuelhh_ranges_cover_the_padded_year_in_monthly_chunks() {
        let sources = sources(2024).unwrap();
        assert_eq!(sources.fuelhh_ranges.len(), 14);
        assert_eq!(
            sources.fuelhh_ranges.first().unwrap(),
            &("2023-12-31".to_owned(), "2023-12-31".to_owned())
        );
        assert_eq!(
            sources.fuelhh_ranges[2],
            ("2024-02-01".to_owned(), "2024-02-29".to_owned())
        );
        assert_eq!(
            sources.fuelhh_ranges.last().unwrap(),
            &("2025-01-01".to_owned(), "2025-01-01".to_owned())
        );
    }

    #[test]
    fn inertia_sources_pin_both_files_for_calendar_2024() {
        let s = sources(2024).unwrap();
        assert!(
            s.neso_inertia_prev_url
                .contains("5bd6ec4d-a2df-4c94-9b27-fdf8cf04d7dd")
        );
        assert!(
            s.neso_inertia_curr_url
                .contains("7a12d0bd-448d-42a9-b333-4a32761dbad4")
        );
        assert_eq!(s.neso_inertia_prev_file, "neso_inertia_2023_2024.csv");
        assert_eq!(s.neso_inertia_curr_file, "neso_inertia_2024_2025.csv");
    }
}
