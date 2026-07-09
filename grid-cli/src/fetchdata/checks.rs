//! Pack validation and the cell-exact comparison harness.
//!
//! Validation ports `scripts/fetch-2024/validate.py` (plus the gas-SAP
//! checks of `scripts/fetch-prices/validate.py`):
//!
//! 1. exactly one half-hourly period per settlement period of the year
//!    (17,568 for leap-year 2024) in every trace;
//! 2. no gaps, no duplicates: strictly consecutive 30-minute UTC index;
//! 3. UTC-clean through both GB clock changes — the two clock-change UTC
//!    days still hold exactly 48 periods, and the *raw* NESO settlement
//!    days hold 46 (short) and 50 (long);
//! 4. no NaNs (the documented INTGRNL pre-go-live absence is zero-filled
//!    at build time); negative values are *reported* where anomalous
//!    (small negative station-load artefacts occur) but only INT*
//!    columns are legitimately negative (net flows, + = import);
//!    `wind_cf` bounded to [0, 1]; gas SAP inside a 10–60 £/MWh
//!    plausibility band;
//! 5. `underlying_demand` identity: nd + embedded wind + embedded solar;
//! 6. cross-check (reported, not gated): Elexon transmission generation
//!    (FUELHH `ps` is already net of pumping — NESO pumping must NOT be
//!    subtracted again; report §6.4) + net imports − NESO ND ≈ station
//!    transformer load (~0.67 GW mean in 2024, with 20 documented
//!    publication-glitch periods left as-is in the pack).
//!
//! Comparison: the acceptance harness for the port. Byte-identical
//! Parquet across writers is not achievable, so the standard is
//! cell-exact numerical identity: same row count, same UTC index, and
//! exact equality (bit-level `f64`, exact `i64`) of every cell against a
//! reference processed directory, with every mismatch named (column,
//! timestamp, both values).

use std::collections::HashMap;
use std::path::Path;

use grid_core::time::HALF_HOUR_MICROS;

use super::error::FetchDataError;
use super::gbtime;
use super::table::{Column, Table};

/// Validate one built pack (see module docs); returns the list of
/// failures (empty = pass) and prints informational notes.
pub fn validate(
    year: u16,
    demand: &Table,
    generation: &Table,
    wind_cf: &Table,
    gas_sap: &Table,
    inertia_outturn: &Table,
    settlement_day_periods: &HashMap<(u8, u8), usize>,
) -> Result<Vec<String>, FetchDataError> {
    let mut failures: Vec<String> = Vec::new();
    let expected_periods = super::build::year_index(year)?.len();

    let short_day = (3u8, gbtime::last_sunday_of(i64::from(year), 3)?);
    let long_day = (10u8, gbtime::last_sunday_of(i64::from(year), 10)?);

    for (name, table) in [
        ("demand", demand),
        ("generation", generation),
        ("wind_cf", wind_cf),
        ("gas_sap_daily", gas_sap),
    ] {
        check_index(
            name,
            table,
            expected_periods,
            [short_day, long_day],
            &mut failures,
        );
        check_values(name, table, &mut failures);
    }

    // Raw NESO settlement-day period counts across the clock changes.
    for ((month, day), expected) in [(short_day, 46usize), (long_day, 50usize)] {
        let found = settlement_day_periods.get(&(month, day)).copied();
        if found != Some(expected) {
            failures.push(format!(
                "raw NESO settlement day {year}-{month:02}-{day:02}: {found:?} periods, expected {expected}"
            ));
        }
    }

    // underlying_demand identity (D3).
    match (
        demand.column("underlying_demand"),
        demand.column("nd"),
        demand.column("embedded_wind_generation"),
        demand.column("embedded_solar_generation"),
    ) {
        (
            Some(Column::Int64(underlying)),
            Some(Column::Int64(nd)),
            Some(Column::Int64(wind)),
            Some(Column::Int64(solar)),
        ) => {
            if (0..underlying.len()).any(|i| underlying[i] != nd[i] + wind[i] + solar[i]) {
                failures.push(
                    "demand: underlying_demand != nd + embedded wind + embedded solar".into(),
                );
            }
        }
        _ => failures.push("demand: missing columns for the underlying_demand identity".into()),
    }

    // wind_cf bounds.
    if let Some(Column::Float64(cf)) = wind_cf.column("wind_cf") {
        if cf.iter().any(|v| !(0.0..=1.0).contains(v)) {
            failures.push("wind_cf: values outside [0, 1]".into());
        }
    } else {
        failures.push("wind_cf: missing wind_cf column".into());
    }

    // Gas SAP plausibility band and stats.
    if let Some(Column::Float64(sap)) = gas_sap.column("sap_gbp_per_mwh_hhv") {
        if sap.iter().any(|v| !(10.0..=60.0).contains(v)) {
            failures.push("gas_sap_daily: outside 10-60 GBP/MWh plausibility band".into());
        }
        let mean = sap.iter().sum::<f64>() / sap.len() as f64;
        let min = sap.iter().copied().fold(f64::INFINITY, f64::min);
        let max = sap.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        println!("gas SAP daily: mean {mean:.2}, min {min:.2}, max {max:.2} GBP/MWh (HHV)");
    } else {
        failures.push("gas_sap_daily: missing sap_gbp_per_mwh_hhv column".into());
    }

    cross_check(demand, generation, &mut failures);
    check_inertia(inertia_outturn, &mut failures);
    Ok(failures)
}

/// Index checks 1–3 for one trace.
fn check_index(
    name: &str,
    table: &Table,
    expected_periods: usize,
    clock_change_days: [(u8, u8); 2],
    failures: &mut Vec<String>,
) {
    if table.len() != expected_periods {
        failures.push(format!(
            "{name}: {} periods, expected {expected_periods}",
            table.len()
        ));
    }
    for pair in table.index.windows(2) {
        if pair[1].unix_micros() - pair[0].unix_micros() != HALF_HOUR_MICROS {
            failures.push(format!(
                "{name}: index not uniform 30-min at {} -> {}",
                pair[0], pair[1]
            ));
            break;
        }
    }
    for (month, day) in clock_change_days {
        let periods = table
            .index
            .iter()
            .filter(|t| {
                let (_, m, d) = t.civil_date();
                (m, d) == (month, day)
            })
            .count();
        if periods != 48 {
            failures.push(format!(
                "{name}: UTC day {month:02}-{day:02} has {periods} periods, expected 48"
            ));
        }
    }
}

/// Value checks: NaN (fail) and anomalous negatives (reported note).
fn check_values(name: &str, table: &Table, failures: &mut Vec<String>) {
    for (column, values) in &table.columns {
        match values {
            Column::Float64(values) => {
                if values.iter().any(|v| v.is_nan()) {
                    failures.push(format!("{name}: NaNs in {column}"));
                }
                // INT* columns are net flows: + import, - export.
                if !column.starts_with("int") {
                    let negatives = values.iter().filter(|v| **v < 0.0).count();
                    if negatives > 0 {
                        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
                        println!(
                            "  note: {name}.{column} has {negatives} negative periods (min {min:.0} MW)"
                        );
                    }
                }
            }
            Column::Int64(values) => {
                if !column.starts_with("int") {
                    let negatives = values.iter().filter(|v| **v < 0).count();
                    if let Some(min) = values.iter().min()
                        && negatives > 0
                    {
                        println!(
                            "  note: {name}.{column} has {negatives} negative periods (min {min} MW)"
                        );
                    }
                }
            }
        }
    }
}

/// Check the `inertia_outturn` table: every UTC calendar day must hold
/// exactly 48 periods — the every-other-day gap signature is a real
/// defect seen in a NESO current-year System Inertia feed — except the
/// two UTC-year boundary dates (1 January and 31 December), where the
/// pack's UTC-year trim can legitimately leave a partial day. Also checks
/// `outturn_inertia_gva_s` against a sane plausibility band, and reports
/// (does not fail) how often the market-provided figure exceeds outturn
/// — a known NESO methodology quirk seen in ~6% of real periods.
fn check_inertia(table: &Table, failures: &mut Vec<String>) {
    let mut periods_per_day: HashMap<(i64, u8, u8), usize> = HashMap::new();
    for instant in &table.index {
        *periods_per_day.entry(instant.civil_date()).or_insert(0) += 1;
    }
    let mut days: Vec<_> = periods_per_day.into_iter().collect();
    days.sort();
    for ((year, month, day), periods) in days {
        if (month, day) == (1, 1) || (month, day) == (12, 31) {
            continue; // legitimate partial day at the UTC-year trim.
        }
        if periods != 48 {
            failures.push(format!(
                "inertia_outturn: UTC day {year}-{month:02}-{day:02} has {periods} periods, expected 48"
            ));
        }
    }

    if let Some(Column::Float64(outturn)) = table.column("outturn_inertia_gva_s") {
        if outturn.iter().any(|v| !(50.0..=400.0).contains(v)) {
            failures.push(
                "inertia_outturn: outturn_inertia_gva_s outside 50-400 GVA.s plausibility band"
                    .into(),
            );
        }
    } else {
        failures.push("inertia_outturn: missing outturn_inertia_gva_s column".into());
    }

    if let (Some(Column::Float64(market)), Some(Column::Float64(outturn))) = (
        table.column("market_provided_inertia_gva_s"),
        table.column("outturn_inertia_gva_s"),
    ) {
        let exceeding = market.iter().zip(outturn).filter(|(m, o)| m > o).count();
        println!(
            "inertia_outturn: market_provided_inertia_gva_s > outturn_inertia_gva_s in {exceeding} of {} periods",
            market.len()
        );
    }
}

/// Check 6: the net-PS supply identity, reported not gated.
fn cross_check(demand: &Table, generation: &Table, failures: &mut Vec<String>) {
    let Some(Column::Int64(nd)) = demand.column("nd") else {
        failures.push("cross-check: demand table has no int64 nd column".into());
        return;
    };
    if generation.len() != demand.len() {
        failures.push("cross-check: demand and generation lengths differ".into());
        return;
    }
    let mut residual = vec![0.0f64; generation.len()];
    for (column, values) in &generation.columns {
        let Column::Float64(values) = values else {
            failures.push(format!(
                "cross-check: generation column {column} not float64"
            ));
            return;
        };
        // Elexon `ps` within transmission generation is net (pumping
        // negative), so pumping is already accounted for; subtracting
        // NESO pumping too would mix metering conventions within one
        // identity (report §6.4). INT* columns are net imports and are
        // part of supply.
        for (accumulated, value) in residual.iter_mut().zip(values) {
            *accumulated += value;
        }
    }
    let mean_nd = nd.iter().sum::<i64>() as f64 / nd.len() as f64;
    for (accumulated, demand_mw) in residual.iter_mut().zip(nd) {
        *accumulated -= *demand_mw as f64;
    }

    let n = residual.len() as f64;
    let mean = residual.iter().sum::<f64>() / n;
    let variance = residual.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let mut sorted = residual.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let quantile = |q: f64| {
        // NumPy linear interpolation, for information only.
        let position = q * (sorted.len() as f64 - 1.0);
        let low = position.floor() as usize;
        let high = position.ceil() as usize;
        sorted[low] + (sorted[high] - sorted[low]) * (position - position.floor())
    };
    println!("\nCross-check: (Elexon tx gen incl. net PS + net imports) - NESO ND [MW]");
    println!("  count {}", residual.len());
    println!("  mean {mean:.1}  std {:.1}", variance.sqrt());
    println!(
        "  min {:.1}  25% {:.1}  50% {:.1}  75% {:.1}  max {:.1}",
        sorted[0],
        quantile(0.25),
        quantile(0.5),
        quantile(0.75),
        sorted[sorted.len() - 1]
    );
    let mean_abs = residual.iter().map(|r| r.abs()).sum::<f64>() / n;
    println!(
        "  mean |residual| / mean ND: {:.2}%",
        mean_abs / mean_nd * 100.0
    );
    println!(
        "  annual residual energy: {:.2} TWh",
        residual.iter().sum::<f64>() * 0.5 / 1e6
    );
}

/// Outcome of comparing one built table against its reference file.
pub struct CompareReport {
    /// Human-readable mismatch descriptions (empty = cell-exact).
    pub mismatches: Vec<String>,
    /// Cells compared (for the verdict line).
    pub cells: usize,
}

/// How many mismatches to describe in full before summarising.
const MISMATCH_DETAIL_CAP: usize = 20;

/// Compare a built table against a reference Parquet file, cell by cell.
///
/// Column matching is by name (order is writer-layout, not data);
/// equality is exact `i64` and bit-level `f64` (`to_bits`), the strictest
/// well-defined float identity (the pack holds no NaNs, so bit equality
/// is not confounded by NaN payloads).
pub fn compare(built: &Table, reference_path: &Path) -> Result<CompareReport, FetchDataError> {
    let reference = Table::read_parquet(reference_path)?;
    let mut mismatches: Vec<String> = Vec::new();
    let mut cells = 0usize;

    if built.len() != reference.len() {
        mismatches.push(format!(
            "row count: built {} vs reference {}",
            built.len(),
            reference.len()
        ));
        return Ok(CompareReport { mismatches, cells });
    }
    for (row, (b, r)) in built.index.iter().zip(&reference.index).enumerate() {
        if b != r {
            mismatches.push(format!("utc_start[{row}]: built {b} vs reference {r}"));
        }
        if mismatches.len() > MISMATCH_DETAIL_CAP {
            return Ok(CompareReport { mismatches, cells });
        }
    }

    let built_names: Vec<&str> = built.columns.iter().map(|(n, _)| n.as_str()).collect();
    let reference_names: Vec<&str> = reference.columns.iter().map(|(n, _)| n.as_str()).collect();
    for name in &reference_names {
        if !built_names.contains(name) {
            mismatches.push(format!("column {name}: in reference but not built"));
        }
    }
    for name in &built_names {
        if !reference_names.contains(name) {
            mismatches.push(format!("column {name}: built but not in reference"));
        }
    }

    for (name, built_column) in &built.columns {
        let Some(reference_column) = reference.column(name) else {
            continue;
        };
        let mut column_mismatches = 0usize;
        match (built_column, reference_column) {
            (Column::Int64(b), Column::Int64(r)) => {
                for (row, (bv, rv)) in b.iter().zip(r).enumerate() {
                    cells += 1;
                    if bv != rv {
                        column_mismatches += 1;
                        if mismatches.len() <= MISMATCH_DETAIL_CAP {
                            mismatches.push(format!(
                                "{name} @ {}: built {bv} vs reference {rv}",
                                built.index[row]
                            ));
                        }
                    }
                }
            }
            (Column::Float64(b), Column::Float64(r)) => {
                for (row, (bv, rv)) in b.iter().zip(r).enumerate() {
                    cells += 1;
                    if bv.to_bits() != rv.to_bits() {
                        column_mismatches += 1;
                        if mismatches.len() <= MISMATCH_DETAIL_CAP {
                            mismatches.push(format!(
                                "{name} @ {}: built {bv:?} ({:#018x}) vs reference {rv:?} ({:#018x})",
                                built.index[row],
                                bv.to_bits(),
                                rv.to_bits()
                            ));
                        }
                    }
                }
            }
            (b, r) => {
                mismatches.push(format!(
                    "column {name}: type differs (built {}, reference {})",
                    type_name(b),
                    type_name(r)
                ));
            }
        }
        if column_mismatches > 0 {
            mismatches.push(format!(
                "{name}: {column_mismatches} mismatching cells in total"
            ));
        }
    }
    Ok(CompareReport { mismatches, cells })
}

fn type_name(column: &Column) -> &'static str {
    match column {
        Column::Int64(_) => "int64",
        Column::Float64(_) => "float64",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use grid_core::time::UtcInstant;

    fn small_table(values: Vec<f64>) -> Table {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        let index: Vec<UtcInstant> = (0..values.len() as i64)
            .map(|i| start.plus_periods(i))
            .collect();
        Table::new("t", index, vec![("x".to_owned(), Column::Float64(values))]).unwrap()
    }

    /// An `inertia_outturn`-shaped fixture: `days` calendar days starting
    /// 2024-06-15 (well clear of the UTC-year boundary dates), each with
    /// the given number of half-hourly periods starting from settlement
    /// period 1 (constant, in-band `outturn`/`market` values).
    fn inertia_fixture(days: &[usize]) -> Table {
        let mut index = Vec::new();
        for (day_offset, &periods) in days.iter().enumerate() {
            let day_start = UtcInstant::parse("2024-06-15T00:00:00Z")
                .unwrap()
                .plus_periods(48 * day_offset as i64);
            for period in 0..periods {
                index.push(day_start.plus_periods(period as i64));
            }
        }
        let n = index.len();
        Table::new(
            "inertia_outturn",
            index,
            vec![
                (
                    "outturn_inertia_gva_s".to_owned(),
                    Column::Float64(vec![150.0; n]),
                ),
                (
                    "market_provided_inertia_gva_s".to_owned(),
                    Column::Float64(vec![140.0; n]),
                ),
                ("settlement_period".to_owned(), Column::Int64(vec![1; n])),
            ],
        )
        .unwrap()
    }

    #[test]
    fn check_inertia_passes_a_clean_two_day_fixture_and_flags_a_short_day() {
        let good = inertia_fixture(&[48, 48]);
        let mut f = Vec::new();
        check_inertia(&good, &mut f);
        assert!(f.is_empty(), "clean fixture must pass: {f:?}");

        // Same shape, but the second day only has settlement periods 1
        // and 2 (the every-other-day gap signature): 50 periods total.
        let short = inertia_fixture(&[48, 2]);
        assert_eq!(short.len(), 50);
        let mut f2 = Vec::new();
        check_inertia(&short, &mut f2);
        assert!(
            f2.iter().any(|m| m.contains("periods")),
            "must flag the short day: {f2:?}"
        );
    }

    #[test]
    fn check_inertia_flags_an_out_of_band_outturn_value() {
        let mut table = inertia_fixture(&[48, 48]);
        for (name, column) in &mut table.columns {
            if name == "outturn_inertia_gva_s"
                && let Column::Float64(values) = column
            {
                values[0] = 10.0; // below the 50-400 GVA.s band
            }
        }
        let mut f = Vec::new();
        check_inertia(&table, &mut f);
        assert!(
            f.iter().any(|m| m.contains("outturn_inertia_gva_s")),
            "must flag the out-of-band value: {f:?}"
        );
    }

    #[test]
    fn compare_is_bit_exact_and_names_the_cell() {
        let dir = std::env::temp_dir().join(format!("gridsim-compare-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("reference.parquet");
        small_table(vec![1.0, 0.5, -0.0])
            .write_parquet(&path)
            .unwrap();

        // Identical values: cell-exact.
        let report = compare(&small_table(vec![1.0, 0.5, -0.0]), &path).unwrap();
        assert!(report.mismatches.is_empty(), "{:?}", report.mismatches);
        assert_eq!(report.cells, 3);

        // One ULP off: named with column, timestamp and both values.
        let report = compare(&small_table(vec![1.0, 0.5f64.next_up(), -0.0]), &path).unwrap();
        assert_eq!(report.mismatches.len(), 2, "{:?}", report.mismatches);
        assert!(report.mismatches[0].contains("x @ 2024-01-01T00:30:00Z"));

        // +0.0 vs -0.0 differ at bit level — the comparison must see it.
        let report = compare(&small_table(vec![1.0, 0.5, 0.0]), &path).unwrap();
        assert_eq!(report.mismatches.len(), 2, "{:?}", report.mismatches);
    }

    #[test]
    fn validation_passes_a_synthetic_clean_pack_and_flags_a_gap() {
        // A full synthetic 2024: constant everything.
        let index = super::super::build::year_index(2024).unwrap();
        let n = index.len();
        let demand = Table::new(
            "d",
            index.clone(),
            vec![
                ("nd".to_owned(), Column::Int64(vec![1000; n])),
                (
                    "embedded_wind_generation".to_owned(),
                    Column::Int64(vec![10; n]),
                ),
                (
                    "embedded_solar_generation".to_owned(),
                    Column::Int64(vec![5; n]),
                ),
                ("underlying_demand".to_owned(), Column::Int64(vec![1015; n])),
            ],
        )
        .unwrap();
        let generation = Table::new(
            "g",
            index.clone(),
            vec![
                ("ccgt".to_owned(), Column::Float64(vec![900.0; n])),
                ("intfr".to_owned(), Column::Float64(vec![100.0; n])),
            ],
        )
        .unwrap();
        let wind_cf = Table::new(
            "w",
            index.clone(),
            vec![("wind_cf".to_owned(), Column::Float64(vec![0.5; n]))],
        )
        .unwrap();
        let gas_sap = Table::new(
            "s",
            index.clone(),
            vec![(
                "sap_gbp_per_mwh_hhv".to_owned(),
                Column::Float64(vec![25.0; n]),
            )],
        )
        .unwrap();
        let inertia_outturn = Table::new(
            "i",
            index.clone(),
            vec![
                (
                    "outturn_inertia_gva_s".to_owned(),
                    Column::Float64(vec![150.0; n]),
                ),
                (
                    "market_provided_inertia_gva_s".to_owned(),
                    Column::Float64(vec![140.0; n]),
                ),
                ("settlement_period".to_owned(), Column::Int64(vec![1; n])),
            ],
        )
        .unwrap();
        let mut settlement_days = HashMap::new();
        settlement_days.insert((3u8, 31u8), 46usize);
        settlement_days.insert((10u8, 27u8), 50usize);

        let failures = validate(
            2024,
            &demand,
            &generation,
            &wind_cf,
            &gas_sap,
            &inertia_outturn,
            &settlement_days,
        )
        .unwrap();
        assert!(failures.is_empty(), "{failures:?}");

        // Drop one period from wind_cf: both the count and the uniformity
        // checks must fire.
        let mut broken_index = index.clone();
        broken_index.remove(100);
        let broken = Table::new(
            "w",
            broken_index,
            vec![("wind_cf".to_owned(), Column::Float64(vec![0.5; n - 1]))],
        )
        .unwrap();
        let failures = validate(
            2024,
            &demand,
            &generation,
            &broken,
            &gas_sap,
            &inertia_outturn,
            &settlement_days,
        )
        .unwrap();
        assert!(
            failures
                .iter()
                .any(|f| f.contains("wind_cf: 17567 periods")),
            "{failures:?}"
        );
        assert!(
            failures.iter().any(|f| f.contains("not uniform")),
            "{failures:?}"
        );

        // Broken identity and a wrong raw settlement-day count.
        let mut wrong_days = settlement_days.clone();
        wrong_days.insert((3u8, 31u8), 48usize);
        let failures = validate(
            2024,
            &demand,
            &generation,
            &wind_cf,
            &gas_sap,
            &inertia_outturn,
            &wrong_days,
        )
        .unwrap();
        assert!(
            failures
                .iter()
                .any(|f| f.contains("raw NESO settlement day")),
            "{failures:?}"
        );
    }
}
