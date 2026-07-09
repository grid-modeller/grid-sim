//! Raw files → processed tables: the exact transformations of the
//! provisional Python builders (`scripts/fetch-2024/build.py` and the
//! gas-SAP part of `scripts/fetch-prices/build.py`), ported operation
//! for operation so the outputs are cell-exact against the Python-built
//! pack.
//!
//! - `demand_<year>`: the NESO CSV columns (integer MW), settlement
//!   date/period converted to a UTC index at the I/O edge (ADR-3), plus
//!   the built `underlying_demand` = `nd` + `embedded_wind_generation` +
//!   `embedded_solar_generation` (D3 total-generation convention).
//! - `generation_by_fuel_<year>`: Elexon FUELHH pivoted wide (float MW),
//!   deduplicated per (period, fuel) keeping the latest `publishTime`
//!   (Elexon revises), trimmed to the UTC year; INT* columns are net
//!   interconnector flows (+ = import); absent INTGRNL periods before its
//!   2024 go-live are genuine zero flow and are zero-filled — any other
//!   hole is an error naming the cell.
//! - `wind_cf_<year>`: PROVISIONAL observed fleet-wide wind capacity
//!   factor, (FUELHH wind + NESO embedded wind) / 29,100 MW constant
//!   end-2024 capacity, clamped to [0, 1] with any clamping reported.
//! - `gas_sap_daily_<year>`: ONS/National Gas daily System Average Price
//!   of gas, p/kWh × 10 → £/MWh (HHV), rounded half-to-even to 4 d.p.
//!   (NumPy `round` semantics, reproduced bit-for-bit), each half-hour
//!   carrying its UTC day's SAP.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use calamine::{Data, Reader};
use grid_core::time::UtcInstant;

use super::error::FetchDataError;
use super::gbtime::london_midnight_utc;
use super::table::{Column, Table};

/// Constant end-2024 GB wind capacity (MW): 14.7 GW offshore + 14.4 GW
/// onshore (UKWED). The wind-CF trace is an outturn series for Stage 0
/// loading tests, not the ERA5 weather trace (D1); the constant
/// denominator biases early-2024 values low.
const WIND_CAPACITY_MW: f64 = 29_100.0;

/// The full half-hourly UTC index of a calendar year.
pub fn year_index(year: u16) -> Result<Vec<UtcInstant>, FetchDataError> {
    let start = UtcInstant::parse(&format!("{year}-01-01T00:00:00Z"))?;
    let end = UtcInstant::parse(&format!("{}-01-01T00:00:00Z", year + 1))?;
    let periods = start.periods_until_inclusive(end)? - 1;
    Ok((0..periods).map(|i| start.plus_periods(i as i64)).collect())
}

/// A built demand table plus the raw settlement-day period counts the
/// validator cross-checks on the two clock-change days.
pub struct DemandBuild {
    /// The processed demand table.
    pub table: Table,
    /// Periods per settlement `(month, day)` — the raw local-day counts.
    pub settlement_day_periods: HashMap<(u8, u8), usize>,
}

/// Build the demand table from the NESO Historic Demand CSV.
pub fn build_demand(raw_csv: &Path, year: u16) -> Result<DemandBuild, FetchDataError> {
    if !raw_csv.exists() {
        return Err(FetchDataError::RawFileMissing {
            path: raw_csv.to_path_buf(),
        });
    }
    let parse_err = |reason: String| FetchDataError::RawParse {
        path: raw_csv.to_path_buf(),
        reason,
    };
    let mut reader =
        csv::Reader::from_path(raw_csv).map_err(|e| parse_err(format!("not readable CSV: {e}")))?;
    let headers = reader
        .headers()
        .map_err(|e| parse_err(format!("no header row: {e}")))?
        .clone();
    let position = |name: &str| {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| parse_err(format!("missing column {name}")))
    };
    let date_at = position("SETTLEMENT_DATE")?;
    let period_at = position("SETTLEMENT_PERIOD")?;
    let value_columns: Vec<(usize, String)> = headers
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != date_at && *i != period_at)
        .map(|(i, h)| (i, h.to_lowercase()))
        .collect();

    let mut rows: Vec<(UtcInstant, Vec<i64>)> = Vec::new();
    let mut settlement_day_periods: HashMap<(u8, u8), usize> = HashMap::new();
    for (line, record) in reader.records().enumerate() {
        let record = record.map_err(|e| parse_err(format!("data row {}: {e}", line + 2)))?;
        let field = |at: usize| {
            record
                .get(at)
                .ok_or_else(|| parse_err(format!("data row {}: short record", line + 2)))
        };
        let (y, m, d) = parse_neso_date(field(date_at)?)
            .map_err(|reason| parse_err(format!("data row {}: {reason}", line + 2)))?;
        if y != i64::from(year) {
            return Err(parse_err(format!(
                "data row {}: settlement date year {y} in the {year} file",
                line + 2
            )));
        }
        let period: i64 = field(period_at)?
            .parse()
            .map_err(|e| parse_err(format!("data row {}: SETTLEMENT_PERIOD: {e}", line + 2)))?;
        let utc_start = london_midnight_utc(y, m, d)?.plus_periods(period - 1);
        *settlement_day_periods.entry((m, d)).or_insert(0) += 1;

        let mut values = Vec::with_capacity(value_columns.len());
        for (at, name) in &value_columns {
            let text = field(*at)?;
            values.push(text.parse::<i64>().map_err(|_| {
                parse_err(format!(
                    "data row {}: column {name}: {text:?} is not an integer \
                     (the NESO demand columns are integer MW)",
                    line + 2
                ))
            })?);
        }
        rows.push((utc_start, values));
    }
    rows.sort_by_key(|(utc, _)| *utc);

    let column_at = |name: &str| {
        value_columns
            .iter()
            .position(|(_, n)| n == name)
            .ok_or_else(|| parse_err(format!("missing column {}", name.to_uppercase())))
    };
    let nd_at = column_at("nd")?;
    let wind_at = column_at("embedded_wind_generation")?;
    let solar_at = column_at("embedded_solar_generation")?;

    let index: Vec<UtcInstant> = rows.iter().map(|(utc, _)| *utc).collect();
    let mut columns: Vec<(String, Column)> = value_columns
        .iter()
        .enumerate()
        .map(|(i, (_, name))| {
            (
                name.clone(),
                Column::Int64(rows.iter().map(|(_, v)| v[i]).collect()),
            )
        })
        .collect();
    // D3 (total-generation convention): underlying demand = ND grossed up
    // by the NESO embedded-generation estimates.
    columns.push((
        "underlying_demand".to_owned(),
        Column::Int64(
            rows.iter()
                .map(|(_, v)| v[nd_at] + v[wind_at] + v[solar_at])
                .collect(),
        ),
    ));
    Ok(DemandBuild {
        table: Table::new("demand", index, columns)?,
        settlement_day_periods,
    })
}

/// Build the inertia-outturn table from the two NESO System Inertia CSVs
/// (April–March editions; together the 2023-24 and 2024-25 files cover
/// calendar 2024). Rows from both files are mapped to a UTC half-hour the
/// same way as `build_demand` (`london_midnight_utc` + period offset),
/// deduplicated on that instant (the files may overlap; any overlap must
/// agree), then trimmed to the UTC `year` window.
pub fn build_inertia_outturn(raw_dir: &Path, year: u16) -> Result<Table, FetchDataError> {
    let prev = parse_inertia_csv(&raw_dir.join("neso_inertia_2023_2024.csv"))?;
    let curr = parse_inertia_csv(&raw_dir.join("neso_inertia_2024_2025.csv"))?;

    // BTreeMap dedups on UtcInstant and keeps rows in UTC order.
    let mut by_utc: BTreeMap<UtcInstant, (i64, f64, f64)> = BTreeMap::new();
    for (utc, period, outturn, market) in prev.into_iter().chain(curr) {
        match by_utc.entry(utc) {
            std::collections::btree_map::Entry::Occupied(existing) => {
                if *existing.get() != (period, outturn, market) {
                    return Err(FetchDataError::Build {
                        output: "inertia_outturn",
                        reason: format!(
                            "{utc}: the 2023-24 and 2024-25 NESO inertia files disagree: \
                             {:?} vs {:?}",
                            existing.get(),
                            (period, outturn, market)
                        ),
                    });
                }
            }
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert((period, outturn, market));
            }
        }
    }

    let year_start = UtcInstant::parse(&format!("{year}-01-01T00:00:00Z"))?;
    let year_end = UtcInstant::parse(&format!("{}-01-01T00:00:00Z", year + 1))?;
    let rows: Vec<(UtcInstant, i64, f64, f64)> = by_utc
        .into_iter()
        .filter(|(utc, _)| (year_start..year_end).contains(utc))
        .map(|(utc, (period, outturn, market))| (utc, period, outturn, market))
        .collect();

    let index: Vec<UtcInstant> = rows.iter().map(|(utc, ..)| *utc).collect();
    let period: Vec<i64> = rows.iter().map(|(_, p, ..)| *p).collect();
    let outturn: Vec<f64> = rows.iter().map(|(_, _, o, _)| *o).collect();
    let market: Vec<f64> = rows.iter().map(|(_, _, _, m)| *m).collect();
    Table::new(
        "inertia_outturn",
        index,
        vec![
            ("outturn_inertia_gva_s".to_owned(), Column::Float64(outturn)),
            (
                "market_provided_inertia_gva_s".to_owned(),
                Column::Float64(market),
            ),
            ("settlement_period".to_owned(), Column::Int64(period)),
        ],
    )
}

/// One parsed row of a NESO System Inertia CSV: UTC half-hour, settlement
/// period, outturn inertia, market-provided inertia.
type InertiaRow = (UtcInstant, i64, f64, f64);

/// Parse one NESO System Inertia CSV (`Settlement Date` ISO `YYYY-MM-DD`,
/// `Settlement Period`, `Outturn Inertia`, `Market Provided Inertia`).
fn parse_inertia_csv(raw_csv: &Path) -> Result<Vec<InertiaRow>, FetchDataError> {
    if !raw_csv.exists() {
        return Err(FetchDataError::RawFileMissing {
            path: raw_csv.to_path_buf(),
        });
    }
    let parse_err = |reason: String| FetchDataError::RawParse {
        path: raw_csv.to_path_buf(),
        reason,
    };
    let mut reader =
        csv::Reader::from_path(raw_csv).map_err(|e| parse_err(format!("not readable CSV: {e}")))?;
    let headers = reader
        .headers()
        .map_err(|e| parse_err(format!("no header row: {e}")))?
        .clone();
    let position = |name: &str| {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| parse_err(format!("missing column {name}")))
    };
    let date_at = position("Settlement Date")?;
    let period_at = position("Settlement Period")?;
    let outturn_at = position("Outturn Inertia")?;
    let market_at = position("Market Provided Inertia")?;

    let mut rows = Vec::new();
    for (line, record) in reader.records().enumerate() {
        let record = record.map_err(|e| parse_err(format!("data row {}: {e}", line + 2)))?;
        let field = |at: usize| {
            record
                .get(at)
                .ok_or_else(|| parse_err(format!("data row {}: short record", line + 2)))
        };
        let (y, m, d) = parse_iso_neso_date(field(date_at)?)
            .map_err(|reason| parse_err(format!("data row {}: {reason}", line + 2)))?;
        let period_text = field(period_at)?;
        let period: i64 = period_text
            .parse()
            .map_err(|e| parse_err(format!("data row {}: Settlement Period: {e}", line + 2)))?;
        let outturn_text = field(outturn_at)?;
        let outturn: f64 = outturn_text.parse().map_err(|_| {
            parse_err(format!(
                "data row {}: Outturn Inertia: {outturn_text:?} is not a number",
                line + 2
            ))
        })?;
        let market_text = field(market_at)?;
        let market: f64 = market_text.parse().map_err(|_| {
            parse_err(format!(
                "data row {}: Market Provided Inertia: {market_text:?} is not a number",
                line + 2
            ))
        })?;
        let utc_start = london_midnight_utc(y, m, d)?.plus_periods(period - 1);
        rows.push((utc_start, period, outturn, market));
    }
    Ok(rows)
}

/// Parse an ISO NESO settlement date (`2024-01-01`, `%Y-%m-%d`).
fn parse_iso_neso_date(text: &str) -> Result<(i64, u8, u8), String> {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("Settlement Date {text:?}: expected YYYY-MM-DD"));
    };
    let year: i64 = year
        .parse()
        .map_err(|_| format!("Settlement Date {text:?}: bad year"))?;
    let month: u8 = month
        .parse()
        .map_err(|_| format!("Settlement Date {text:?}: bad month"))?;
    let day: u8 = day
        .parse()
        .map_err(|_| format!("Settlement Date {text:?}: bad day"))?;
    Ok((year, month, day))
}

/// Parse a NESO settlement date (`01-JAN-2024`, `%d-%b-%Y`).
fn parse_neso_date(text: &str) -> Result<(i64, u8, u8), String> {
    let mut parts = text.split('-');
    let (Some(day), Some(month), Some(year), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("SETTLEMENT_DATE {text:?}: expected DD-MON-YYYY"));
    };
    let day: u8 = day
        .parse()
        .map_err(|_| format!("SETTLEMENT_DATE {text:?}: bad day"))?;
    let month = match month.to_ascii_uppercase().as_str() {
        "JAN" => 1,
        "FEB" => 2,
        "MAR" => 3,
        "APR" => 4,
        "MAY" => 5,
        "JUN" => 6,
        "JUL" => 7,
        "AUG" => 8,
        "SEP" => 9,
        "OCT" => 10,
        "NOV" => 11,
        "DEC" => 12,
        other => return Err(format!("SETTLEMENT_DATE {text:?}: bad month {other:?}")),
    };
    let year: i64 = year
        .parse()
        .map_err(|_| format!("SETTLEMENT_DATE {text:?}: bad year"))?;
    Ok((year, month, day))
}

/// One FUELHH record (unknown fields ignored — the raw JSON also carries
/// `dataset`, `settlementDate`, `settlementPeriod`).
#[derive(serde::Deserialize)]
struct FuelhhRecord {
    #[serde(rename = "startTime")]
    start_time: String,
    #[serde(rename = "publishTime")]
    publish_time: String,
    #[serde(rename = "fuelType")]
    fuel_type: String,
    /// Integer MW in the raw JSON; carried as `f64` because the pivoted
    /// table is float64 (the pandas pivot upcasts).
    generation: f64,
}

/// Build the generation-by-fuel table from the FUELHH monthly chunks.
pub fn build_generation(raw_dir: &Path, year: u16) -> Result<Table, FetchDataError> {
    let mut chunk_paths: Vec<PathBuf> = std::fs::read_dir(raw_dir)
        .map_err(|source| FetchDataError::io(raw_dir, source))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("fuelhh_") && n.ends_with(".json"))
        })
        .collect();
    chunk_paths.sort();
    if chunk_paths.is_empty() {
        return Err(FetchDataError::RawFileMissing {
            path: raw_dir.join("fuelhh_*.json"),
        });
    }

    // Latest publication per (period, fuel) wins — Elexon revises. Ties on
    // publishTime keep the later record in file order (chunk date ranges
    // are disjoint, so ties can only occur within one chunk).
    let mut latest: HashMap<(UtcInstant, String), (UtcInstant, f64)> = HashMap::new();
    let mut fuels: BTreeSet<String> = BTreeSet::new();
    for path in &chunk_paths {
        let parse_err = |reason: String| FetchDataError::RawParse {
            path: path.clone(),
            reason,
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| FetchDataError::io(path, source))?;
        let records: Vec<FuelhhRecord> =
            serde_json::from_str(&text).map_err(|e| parse_err(e.to_string()))?;
        if records.is_empty() {
            return Err(parse_err("empty FUELHH chunk".to_owned()));
        }
        for record in records {
            let utc_start = UtcInstant::parse(&record.start_time)?;
            let publish = UtcInstant::parse(&record.publish_time)?;
            fuels.insert(record.fuel_type.clone());
            let entry = latest.entry((utc_start, record.fuel_type));
            match entry {
                std::collections::hash_map::Entry::Occupied(mut kept) => {
                    if publish >= kept.get().0 {
                        kept.insert((publish, record.generation));
                    }
                }
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert((publish, record.generation));
                }
            }
        }
    }

    // Trim to the UTC year and pivot wide.
    let year_start = UtcInstant::parse(&format!("{year}-01-01T00:00:00Z"))?;
    let year_end = UtcInstant::parse(&format!("{}-01-01T00:00:00Z", year + 1))?;
    let index: Vec<UtcInstant> = latest
        .keys()
        .map(|(utc, _)| *utc)
        .filter(|utc| (year_start..year_end).contains(utc))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut columns: Vec<(String, Column)> = Vec::with_capacity(fuels.len());
    for fuel in &fuels {
        let mut values = Vec::with_capacity(index.len());
        for utc in &index {
            match latest.get(&(*utc, fuel.clone())) {
                Some((_, generation)) => values.push(*generation),
                // INTGRNL (Greenlink) only reports from its late-2024
                // go-live; absent periods are genuinely zero flow, not
                // gaps.
                None if fuel == "INTGRNL" => values.push(0.0),
                None => {
                    return Err(FetchDataError::GenerationGap {
                        fuel: fuel.to_lowercase(),
                        utc_start: *utc,
                    });
                }
            }
        }
        columns.push((fuel.to_lowercase(), Column::Float64(values)));
    }
    Table::new("generation_by_fuel", index, columns)
}

/// Clamping report for the wind-CF build (any clamping is printed, as in
/// the Python builder).
pub struct WindCfBuild {
    /// The single-column `wind_cf` table.
    pub table: Table,
    /// Periods clamped below 0 / above 1, and the raw range.
    pub clamped_below: usize,
    /// Periods clamped above 1.
    pub clamped_above: usize,
    /// Raw (pre-clamp) minimum.
    pub raw_min: f64,
    /// Raw (pre-clamp) maximum.
    pub raw_max: f64,
}

/// Build the provisional observed wind capacity-factor trace.
///
/// See the module docs and `WIND_CAPACITY_MW`: this is the outturn CF
/// (curtailment and outages included, constant capacity denominator),
/// kept for Stage 0 trace-loading tests.
pub fn build_wind_cf(demand: &Table, generation: &Table) -> Result<WindCfBuild, FetchDataError> {
    for (row, (d, g)) in demand.index.iter().zip(&generation.index).enumerate() {
        if d != g {
            return Err(FetchDataError::Misaligned {
                row,
                demand: *d,
                generation: *g,
            });
        }
    }
    let build_err = |reason: &str| FetchDataError::Build {
        output: "wind_cf",
        reason: reason.to_owned(),
    };
    if demand.len() != generation.len() {
        return Err(build_err("demand and generation lengths differ"));
    }
    let Some(Column::Float64(wind)) = generation.column("wind") else {
        return Err(build_err("generation table has no float64 wind column"));
    };
    let Some(Column::Int64(embedded)) = demand.column("embedded_wind_generation") else {
        return Err(build_err(
            "demand table has no int64 embedded_wind_generation column",
        ));
    };

    let mut values = Vec::with_capacity(wind.len());
    let (mut below, mut above) = (0usize, 0usize);
    let (mut raw_min, mut raw_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for (w, e) in wind.iter().zip(embedded) {
        // Same operation order as the Python: (tx wind + embedded wind)
        // / capacity — f64 arithmetic is bit-reproducible across the two.
        let cf = (w + *e as f64) / WIND_CAPACITY_MW;
        raw_min = raw_min.min(cf);
        raw_max = raw_max.max(cf);
        if cf < 0.0 {
            below += 1;
            values.push(0.0);
        } else if cf > 1.0 {
            above += 1;
            values.push(1.0);
        } else {
            values.push(cf);
        }
    }
    Ok(WindCfBuild {
        table: Table::new(
            "wind_cf",
            demand.index.clone(),
            vec![("wind_cf".to_owned(), Column::Float64(values))],
        )?,
        clamped_below: below,
        clamped_above: above,
        raw_min,
        raw_max,
    })
}

/// Build the daily gas SAP trace from the ONS xlsx.
pub fn build_gas_sap(raw_xlsx: &Path, year: u16) -> Result<Table, FetchDataError> {
    if !raw_xlsx.exists() {
        return Err(FetchDataError::RawFileMissing {
            path: raw_xlsx.to_path_buf(),
        });
    }
    let parse_err = |reason: String| FetchDataError::RawParse {
        path: raw_xlsx.to_path_buf(),
        reason,
    };
    let mut workbook: calamine::Xlsx<_> =
        calamine::open_workbook(raw_xlsx).map_err(|e| parse_err(format!("not readable: {e}")))?;
    let sheet = "Table 1 Daily SAP of Gas";
    let range = workbook
        .worksheet_range(sheet)
        .map_err(|e| parse_err(format!("sheet {sheet:?}: {e}")))?;

    // Data rows carry an Excel date (serial or typed datetime) in column
    // A and the actual-day SAP (p/kWh) in column B; header and note rows
    // don't and are skipped, like the Python's fixed skiprows + column
    // rename.
    let mut daily: BTreeMap<(i64, u8, u8), f64> = BTreeMap::new();
    for row in range.rows() {
        let Some(serial) = row.first().and_then(excel_date_serial) else {
            continue;
        };
        // Excel serial day 25569 is 1970-01-01 (proleptic-Gregorian, and
        // the pre-1900-03-01 leap bug is 60+ years out of range here).
        let days_since_epoch = serial - 25_569;
        let date = UtcInstant::from_unix_micros(days_since_epoch * 24 * 3_600 * 1_000_000);
        let (y, m, d) = date.civil_date();
        if y != i64::from(year) {
            continue;
        }
        let Some(Data::Float(p_per_kwh)) = row.get(1) else {
            return Err(parse_err(format!(
                "no numeric SAP value for {y:04}-{m:02}-{d:02}"
            )));
        };
        // p/kWh -> £/MWh (×10); SAP is on a gross-CV (HHV) basis. Rounded
        // to 4 d.p. exactly as NumPy does (scale, round half-to-even,
        // unscale) to keep the CSV free of binary float noise and the
        // cells bit-identical to the Python build.
        let gbp_per_mwh = round_half_even_4dp(p_per_kwh * 10.0);
        if daily.insert((y, m, d), gbp_per_mwh).is_some() {
            return Err(parse_err(format!(
                "duplicate SAP row for {y:04}-{m:02}-{d:02}"
            )));
        }
    }

    let index = year_index(year)?;
    let expected_days = index.len() / 48;
    if daily.len() != expected_days {
        return Err(FetchDataError::Build {
            output: "gas_sap_daily",
            reason: format!(
                "expected {expected_days} daily SAP rows for {year}, got {}",
                daily.len()
            ),
        });
    }
    let mut values = Vec::with_capacity(index.len());
    for instant in &index {
        let day = instant.civil_date();
        match daily.get(&day) {
            Some(sap) => values.push(*sap),
            None => {
                return Err(FetchDataError::Build {
                    output: "gas_sap_daily",
                    reason: format!("no SAP for {:04}-{:02}-{:02}", day.0, day.1, day.2),
                });
            }
        }
    }
    Table::new(
        "gas_sap_daily",
        index,
        vec![("sap_gbp_per_mwh_hhv".to_owned(), Column::Float64(values))],
    )
}

/// The Excel date serial of a cell, if it holds one.
fn excel_date_serial(cell: &Data) -> Option<i64> {
    let serial = match cell {
        Data::Float(f) => *f,
        Data::Int(i) => *i as f64,
        Data::DateTime(dt) => dt.as_f64(),
        _ => return None,
    };
    (serial > 0.0 && serial.fract() == 0.0).then_some(serial as i64)
}

/// NumPy `round(x, 4)`: scale by 10⁴, round half to even, unscale.
fn round_half_even_4dp(value: f64) -> f64 {
    (value * 10_000.0).round_ties_even() / 10_000.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn neso_dates_parse() {
        assert_eq!(parse_neso_date("01-JAN-2024").unwrap(), (2024, 1, 1));
        assert_eq!(parse_neso_date("27-OCT-2024").unwrap(), (2024, 10, 27));
        assert!(parse_neso_date("2024-01-01").is_err());
        assert!(parse_neso_date("01-VOR-2024").is_err());
    }

    #[test]
    fn year_index_covers_the_leap_year() {
        let index = year_index(2024).unwrap();
        assert_eq!(index.len(), 17_568);
        assert_eq!(index[0].to_string(), "2024-01-01T00:00:00Z");
        assert_eq!(index[17_567].to_string(), "2024-12-31T23:30:00Z");
    }

    #[test]
    fn rounding_matches_numpy_half_even_semantics() {
        // Expectations pinned by running NumPy itself (np.round(v, 4)) —
        // including its documented float-representation quirks: an exact
        // tie rounds to even (0.00005 -> 0.0), while 0.00015 scales to
        // 1.4999999999999998 in binary and so rounds *down*.
        assert_eq!(round_half_even_4dp(0.000_05), 0.0);
        assert_eq!(round_half_even_4dp(0.000_15), 0.000_1);
        // A real SAP row: 1.7083 p/kWh -> 17.083 £/MWh (the xlsx stores
        // the long form 1.7082999999999999, the same f64).
        assert_eq!(round_half_even_4dp(1.7083 * 10.0), 17.083);
        assert_eq!(round_half_even_4dp(17.083_449_99), 17.083_4);
    }

    #[test]
    fn demand_build_from_a_synthetic_neso_csv() {
        let dir = std::env::temp_dir().join(format!("gridsim-neso-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("demanddata.csv");
        // Two winter periods plus one summer period (BST: 01-JUL period 1
        // starts 2024-06-30T23:00Z), deliberately out of order.
        std::fs::write(
            &path,
            "SETTLEMENT_DATE,SETTLEMENT_PERIOD,ND,EMBEDDED_WIND_GENERATION,EMBEDDED_SOLAR_GENERATION\n\
             01-JUL-2024,1,100,10,1\n\
             01-JAN-2024,2,200,20,2\n\
             01-JAN-2024,1,300,30,3\n",
        )
        .unwrap();
        let build = build_demand(&path, 2024).unwrap();
        let table = build.table;
        let stamps: Vec<String> = table.index.iter().map(ToString::to_string).collect();
        assert_eq!(
            stamps,
            [
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:30:00Z",
                "2024-06-30T23:00:00Z"
            ]
        );
        // Lowercased source columns in order, then the built column.
        let names: Vec<&str> = table.columns.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "nd",
                "embedded_wind_generation",
                "embedded_solar_generation",
                "underlying_demand"
            ]
        );
        assert_eq!(
            table.column("underlying_demand"),
            Some(&Column::Int64(vec![333, 222, 111]))
        );
        assert_eq!(build.settlement_day_periods[&(1, 1)], 2);
    }

    #[test]
    fn demand_build_rejects_non_integer_values() {
        let dir = std::env::temp_dir().join(format!("gridsim-neso-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.csv");
        std::fs::write(
            &path,
            "SETTLEMENT_DATE,SETTLEMENT_PERIOD,ND,EMBEDDED_WIND_GENERATION,EMBEDDED_SOLAR_GENERATION\n\
             01-JAN-2024,1,1.5,10,1\n",
        )
        .unwrap();
        assert!(matches!(
            build_demand(&path, 2024),
            Err(FetchDataError::RawParse { .. })
        ));
    }

    fn write_fuelhh(dir: &Path, name: &str, records: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, records).unwrap();
        path
    }

    #[test]
    fn generation_build_dedupes_on_latest_publish_time_and_fills_intgrnl() {
        let dir = std::env::temp_dir().join(format!("gridsim-fuelhh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Period 1 has a revision (later publishTime must win); period 2
        // has no INTGRNL record (zero-filled); a 2023 record is trimmed.
        write_fuelhh(
            &dir,
            "fuelhh_2024-01-01_2024-01-31.json",
            r#"[
              {"publishTime":"2024-01-01T00:30:00Z","startTime":"2024-01-01T00:00:00Z","fuelType":"CCGT","generation":100},
              {"publishTime":"2024-01-02T00:00:00Z","startTime":"2024-01-01T00:00:00Z","fuelType":"CCGT","generation":150},
              {"publishTime":"2024-01-01T00:30:00Z","startTime":"2024-01-01T00:00:00Z","fuelType":"INTGRNL","generation":7},
              {"publishTime":"2024-01-01T01:00:00Z","startTime":"2024-01-01T00:30:00Z","fuelType":"CCGT","generation":110},
              {"publishTime":"2024-01-01T01:00:00Z","startTime":"2023-12-31T23:30:00Z","fuelType":"CCGT","generation":999}
            ]"#,
        );
        let table = build_generation(&dir, 2024).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(
            table.column("ccgt"),
            Some(&Column::Float64(vec![150.0, 110.0]))
        );
        assert_eq!(
            table.column("intgrnl"),
            Some(&Column::Float64(vec![7.0, 0.0]))
        );
    }

    #[test]
    fn generation_build_names_any_non_intgrnl_hole() {
        let dir = std::env::temp_dir().join(format!("gridsim-fuelhh-gap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_fuelhh(
            &dir,
            "fuelhh_2024-01-01_2024-01-31.json",
            r#"[
              {"publishTime":"2024-01-01T00:30:00Z","startTime":"2024-01-01T00:00:00Z","fuelType":"CCGT","generation":100},
              {"publishTime":"2024-01-01T01:00:00Z","startTime":"2024-01-01T00:30:00Z","fuelType":"WIND","generation":5}
            ]"#,
        );
        let err = build_generation(&dir, 2024).unwrap_err();
        assert!(matches!(err, FetchDataError::GenerationGap { .. }), "{err}");
    }

    #[test]
    fn wind_cf_is_the_documented_ratio_with_clamp_accounting() {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        let index: Vec<UtcInstant> = (0..2).map(|i| start.plus_periods(i)).collect();
        let demand = Table::new(
            "d",
            index.clone(),
            vec![(
                "embedded_wind_generation".to_owned(),
                Column::Int64(vec![2_804, 30_000]),
            )],
        )
        .unwrap();
        let generation = Table::new(
            "g",
            index,
            vec![("wind".to_owned(), Column::Float64(vec![6_246.0, 2_000.0]))],
        )
        .unwrap();
        let build = build_wind_cf(&demand, &generation).unwrap();
        let Some(Column::Float64(cf)) = build.table.column("wind_cf") else {
            panic!("missing wind_cf column")
        };
        assert_eq!(cf[0], (6_246.0 + 2_804.0) / 29_100.0);
        assert_eq!(cf[1], 1.0); // clamped
        assert_eq!(build.clamped_above, 1);
        assert_eq!(build.clamped_below, 0);
    }

    #[test]
    fn inertia_outturn_covers_the_trimmed_utc_year_from_fixture() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/inertia");
        let t = build_inertia_outturn(&dir, 2024).unwrap();
        // The fixture's 2023-12-31 and 2025-01-01 rows fall outside
        // calendar 2024 and trim out, leaving the UTC-year boundary
        // periods: 2024-01-01 P1/P2 (from the 2023-24 file) and
        // 2024-12-31 P47/P48 (from the 2024-25 file).
        let stamps: Vec<String> = t.index.iter().map(ToString::to_string).collect();
        assert_eq!(
            stamps,
            [
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:30:00Z",
                "2024-12-31T23:00:00Z",
                "2024-12-31T23:30:00Z",
            ]
        );
        assert_eq!(
            t.column("outturn_inertia_gva_s"),
            Some(&Column::Float64(vec![151.0, 152.0, 162.0, 163.0]))
        );
        assert_eq!(
            t.column("market_provided_inertia_gva_s"),
            Some(&Column::Float64(vec![141.0, 142.0, 152.0, 153.0]))
        );
        assert_eq!(
            t.column("settlement_period"),
            Some(&Column::Int64(vec![1, 2, 47, 48]))
        );
    }

    #[test]
    fn inertia_outturn_dedupes_an_agreeing_collision_across_files() {
        let dir =
            std::env::temp_dir().join(format!("gridsim-inertia-agree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let header = "Settlement Date,Settlement Period,Outturn Inertia,Market Provided Inertia\n";
        std::fs::write(
            dir.join("neso_inertia_2023_2024.csv"),
            format!("{header}2024-01-01,1,151.0,141.0\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("neso_inertia_2024_2025.csv"),
            format!("{header}2024-01-01,1,151.0,141.0\n"),
        )
        .unwrap();
        let t = build_inertia_outturn(&dir, 2024).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(
            t.column("outturn_inertia_gva_s"),
            Some(&Column::Float64(vec![151.0]))
        );
    }

    #[test]
    fn inertia_outturn_rejects_a_disagreeing_collision_across_files() {
        let dir =
            std::env::temp_dir().join(format!("gridsim-inertia-disagree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let header = "Settlement Date,Settlement Period,Outturn Inertia,Market Provided Inertia\n";
        std::fs::write(
            dir.join("neso_inertia_2023_2024.csv"),
            format!("{header}2024-01-01,1,151.0,141.0\n"),
        )
        .unwrap();
        std::fs::write(
            dir.join("neso_inertia_2024_2025.csv"),
            format!("{header}2024-01-01,1,999.0,141.0\n"),
        )
        .unwrap();
        let err = build_inertia_outturn(&dir, 2024).unwrap_err();
        assert!(matches!(err, FetchDataError::Build { .. }), "{err}");
    }
}
