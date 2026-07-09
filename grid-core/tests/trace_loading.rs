//! Stage 0 acceptance test 3: loads the real 2024 half-hourly demand and
//! wind capacity-factor traces — 17,568 periods each, uniform 30-minute
//! UTC spacing straight through the 2024-03-31 and 2024-10-27 local
//! clock-change dates, no NaNs (docs/04 Stage 0).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use grid_core::GridError;
use grid_core::time::UtcInstant;
use grid_core::trace::{load_per_unit_trace, load_power_trace_mw, load_price_trace};
use grid_core::units::{PerUnit, Power};

/// Leap-year 2024 half-hourly period count.
const PERIODS_2024: usize = 17_568;

/// Locate a 2024 data-pack fixture, failing loudly if the pack has not
/// been built (data/ is git-ignored; packs are fetched, not committed).
fn fixture(name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../data/packs/2024/processed")
        .join(name);
    assert!(
        path.exists(),
        "2024 data-pack fixture {} is missing — run scripts/fetch-2024 first \
         (fetch.py, build.py) to build the local data pack",
        path.display()
    );
    path
}

#[test]
fn loads_2024_demand_trace_with_correct_period_count() {
    let path = fixture("demand_2024.parquet");
    for column in ["nd", "underlying_demand"] {
        let trace = load_power_trace_mw(&path, column, PERIODS_2024).unwrap();
        assert_eq!(trace.len(), PERIODS_2024, "column {column}");

        // Sanity on the values: GB demand in 2024 sat well inside
        // 10–60 GW every half hour, and the loader converts MW → GW.
        let min = trace.min().unwrap();
        let max = trace.max().unwrap();
        let mean = trace.mean().unwrap();
        assert!(min > Power::gigawatts(10.0), "{column} min {min:?}");
        assert!(max < Power::gigawatts(60.0), "{column} max {max:?}");
        assert!(mean > min && mean < max, "{column} mean {mean:?}");
    }
}

#[test]
fn loads_2024_wind_cf_trace_as_per_unit() {
    let path = fixture("wind_cf_2024.parquet");
    let trace = load_per_unit_trace(&path, "wind_cf", PERIODS_2024).unwrap();
    assert_eq!(trace.len(), PERIODS_2024);
    assert!(trace.min().unwrap() >= PerUnit::new(0.0));
    assert!(trace.max().unwrap() <= PerUnit::new(1.0));
    // A capacity factor trace that never moves would be a build error.
    assert!(trace.max().unwrap() > trace.min().unwrap());
}

// Stage 2: the price traces of the 2024 price-pack extension load as
// £/MWh (no unit conversion — the pack stores prices in £/MWh directly),
// reproducing the price-pack report §3 headline means.
#[test]
fn loads_2024_price_traces_in_pounds_per_megawatt_hour() {
    let gas = load_price_trace(
        &fixture("gas_sap_daily_2024.parquet"),
        "sap_gbp_per_mwh_hhv",
        PERIODS_2024,
    )
    .unwrap();
    // Daily SAP annual mean £28.67/MWh_th HHV.
    let gas_mean = gas.mean().unwrap().as_pounds_per_megawatt_hour();
    assert!((gas_mean - 28.67).abs() < 0.005, "gas mean {gas_mean}");

    let mid = load_price_trace(
        &fixture("market_index_2024.parquet"),
        "mid_price",
        PERIODS_2024,
    )
    .unwrap();
    // Time-weighted mean £71.38/MWh; negative prices are real and kept
    // (min −£61.09/MWh) — the loader must not clamp them.
    let mid_mean = mid.mean().unwrap().as_pounds_per_megawatt_hour();
    assert!((mid_mean - 71.38).abs() < 0.005, "MID mean {mid_mean}");
    let mid_min = mid.min().unwrap().as_pounds_per_megawatt_hour();
    assert!((mid_min + 61.09).abs() < 0.005, "MID min {mid_min}");
}

#[test]
fn trace_index_is_utc_clean_through_the_clock_changes() {
    let path = fixture("demand_2024.parquet");
    let trace = load_power_trace_mw(&path, "nd", PERIODS_2024).unwrap();

    assert_eq!(
        trace.start(),
        UtcInstant::parse("2024-01-01T00:00:00Z").unwrap()
    );

    // The loader has verified strictly uniform 30-minute UTC spacing over
    // the whole year, so `timestamp_at` reflects the file's actual index.
    // Spot-check straight through both Europe/London clock changes —
    // 2024-03-31 (spring forward, 01:00 UTC) and 2024-10-27 (fall back,
    // 01:00 UTC) — where a local-time index would gap or duplicate.
    // 2024-03-31 00:00 UTC starts at period 90 × 48 = 4320.
    for (index, expected) in [
        (4321, "2024-03-31T00:30:00Z"),
        (4322, "2024-03-31T01:00:00Z"),
        (4323, "2024-03-31T01:30:00Z"),
        // 2024-10-27 00:00 UTC starts at period 300 × 48 = 14400.
        (14401, "2024-10-27T00:30:00Z"),
        (14402, "2024-10-27T01:00:00Z"),
        (14403, "2024-10-27T01:30:00Z"),
        // Last period of the year.
        (17567, "2024-12-31T23:30:00Z"),
    ] {
        assert_eq!(
            trace.timestamp_at(index).unwrap(),
            UtcInstant::parse(expected).unwrap(),
            "period {index}"
        );
    }
    assert!(trace.timestamp_at(PERIODS_2024).is_none());
}

#[test]
fn wrong_expected_period_count_is_an_error() {
    let path = fixture("wind_cf_2024.parquet");
    let err = load_per_unit_trace(&path, "wind_cf", 17_520).unwrap_err();
    match err {
        GridError::TracePeriodCount {
            expected, found, ..
        } => {
            assert_eq!(expected, 17_520);
            assert_eq!(found, PERIODS_2024);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn missing_column_is_an_error_naming_the_column() {
    let path = fixture("wind_cf_2024.parquet");
    let err = load_per_unit_trace(&path, "no_such_column", PERIODS_2024).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no_such_column"), "message was: {msg}");
}

#[test]
fn missing_file_is_a_clear_error_naming_the_path() {
    let err =
        load_per_unit_trace("/nonexistent/trace.parquet".as_ref(), "x", PERIODS_2024).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("/nonexistent/trace.parquet"),
        "message was: {msg}"
    );
    assert!(
        msg.contains("fetched"),
        "message should explain that data packs are fetched, not committed: {msg}"
    );
}

// ---------------------------------------------------------------------
// Synthetic malformed traces, written to a temp dir with the same arrow
// writer stack the loader reads with.
// ---------------------------------------------------------------------

use arrow_array::builder::{Float64Builder, TimestampMicrosecondBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, TimeUnit};

/// Write a single-column trace parquet with the given (timestamp, value)
/// rows into a fresh temp file, and return its path.
fn write_trace(name: &str, rows: &[(i64, f64)], utc: bool) -> PathBuf {
    let tz = utc.then(|| Arc::from("UTC"));
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, tz.clone());
    let schema = Arc::new(Schema::new(vec![
        Field::new("value", DataType::Float64, false),
        Field::new("utc_start", ts_type.clone(), false),
    ]));

    let mut values = Float64Builder::new();
    let mut stamps = TimestampMicrosecondBuilder::new();
    for &(t, v) in rows {
        stamps.append_value(t);
        values.append_value(v);
    }
    let stamps = if let Some(tz) = tz {
        stamps.finish().with_timezone(tz)
    } else {
        stamps.finish()
    };
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(values.finish()) as ArrayRef,
            Arc::new(stamps) as ArrayRef,
        ],
    )
    .unwrap();

    let dir = std::env::temp_dir().join("grid-sim-trace-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = parquet::arrow::ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    path
}

const T0: i64 = 1_704_067_200 * 1_000_000; // 2024-01-01T00:00:00Z
const HALF_HOUR: i64 = 1_800 * 1_000_000;

#[test]
fn gap_in_the_index_is_rejected() {
    // Third period missing: 00:00, 00:30, 01:30 — the shape a local-time
    // spring-forward index would have.
    let path = write_trace(
        "gap.parquet",
        &[(T0, 1.0), (T0 + HALF_HOUR, 1.0), (T0 + 3 * HALF_HOUR, 1.0)],
        true,
    );
    let err = load_per_unit_trace(&path, "value", 3).unwrap_err();
    assert!(
        matches!(err, GridError::TraceIndexNotUniform { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn duplicated_timestamp_is_rejected() {
    // Duplicate period — the shape a local-time fall-back index would have.
    let path = write_trace(
        "dup.parquet",
        &[(T0, 1.0), (T0 + HALF_HOUR, 1.0), (T0 + HALF_HOUR, 1.0)],
        true,
    );
    let err = load_per_unit_trace(&path, "value", 3).unwrap_err();
    assert!(
        matches!(err, GridError::TraceIndexNotUniform { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn nan_values_are_rejected() {
    let path = write_trace(
        "nan.parquet",
        &[
            (T0, 1.0),
            (T0 + HALF_HOUR, f64::NAN),
            (T0 + 2 * HALF_HOUR, 1.0),
        ],
        true,
    );
    let err = load_per_unit_trace(&path, "value", 3).unwrap_err();
    match err {
        GridError::TraceNan { index, .. } => assert_eq!(index, 1),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn naive_timestamps_are_rejected() {
    // ADR-3: no naive datetimes anywhere — a trace index without an
    // explicit UTC timezone is refused at the I/O edge.
    let path = write_trace("naive.parquet", &[(T0, 1.0), (T0 + HALF_HOUR, 1.0)], false);
    let err = load_per_unit_trace(&path, "value", 2).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("UTC"), "message was: {msg}");
}

#[test]
fn empty_trace_is_rejected() {
    let path = write_trace("empty.parquet", &[], true);
    assert!(load_per_unit_trace(&path, "value", 0).is_err());
}

// ---------------------------------------------------------------------
// Multi-file trace assembly (schema v2 / Stage 3): consecutive per-year
// files concatenate into one horizon-spanning trace; gaps, overlaps and
// wrong totals are rejected. Synthetic stand-ins for the per-year
// 1985–2023 CF files (docs/04 Stage 3 multi-year runs).
// ---------------------------------------------------------------------

use grid_core::trace::{load_per_unit_trace_concat, load_power_trace_mw_concat};

#[test]
fn consecutive_files_concatenate_into_one_trace() {
    // "Year one" is 3 periods, "year two" starts exactly one period after
    // year one ends.
    let year1 = write_trace(
        "concat-y1.parquet",
        &[
            (T0, 1000.0),
            (T0 + HALF_HOUR, 2000.0),
            (T0 + 2 * HALF_HOUR, 3000.0),
        ],
        true,
    );
    let year2 = write_trace(
        "concat-y2.parquet",
        &[(T0 + 3 * HALF_HOUR, 4000.0), (T0 + 4 * HALF_HOUR, 5000.0)],
        true,
    );
    let trace = load_power_trace_mw_concat(&[year1, year2], "value", 5).unwrap();
    assert_eq!(trace.len(), 5);
    assert_eq!(trace.start(), UtcInstant::from_unix_micros(T0));
    let gw: Vec<f64> = trace.values().iter().map(|p| p.as_gigawatts()).collect();
    assert_eq!(gw, [1.0, 2.0, 3.0, 4.0, 5.0]);
    // A single-file list behaves exactly like the single-file loader.
    let single = write_trace(
        "concat-single.parquet",
        &[(T0, 0.5), (T0 + HALF_HOUR, 0.7)],
        true,
    );
    let trace = load_per_unit_trace_concat(&[single], "value", 2).unwrap();
    assert_eq!(trace.values(), &[PerUnit::new(0.5), PerUnit::new(0.7)]);
}

#[test]
fn a_gap_between_files_is_rejected() {
    let year1 = write_trace("gap-y1.parquet", &[(T0, 1.0), (T0 + HALF_HOUR, 1.0)], true);
    // Starts one period late: a whole missing half-hour at the boundary.
    let year2 = write_trace("gap-y2.parquet", &[(T0 + 3 * HALF_HOUR, 1.0)], true);
    let err = load_power_trace_mw_concat(&[year1, year2], "value", 3).unwrap_err();
    assert!(
        matches!(err, GridError::TraceNotConsecutive { .. }),
        "unexpected error: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("gap-y2.parquet"), "message was: {msg}");
    assert!(msg.contains("gap-y1.parquet"), "message was: {msg}");
}

#[test]
fn an_overlap_between_files_is_rejected() {
    let year1 = write_trace("ovl-y1.parquet", &[(T0, 1.0), (T0 + HALF_HOUR, 1.0)], true);
    // Re-starts at the first file's last period: a duplicated half-hour.
    let year2 = write_trace("ovl-y2.parquet", &[(T0 + HALF_HOUR, 1.0)], true);
    let err = load_power_trace_mw_concat(&[year1, year2], "value", 3).unwrap_err();
    assert!(
        matches!(err, GridError::TraceNotConsecutive { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn wrong_total_period_count_across_files_is_rejected() {
    let year1 = write_trace("tot-y1.parquet", &[(T0, 1.0), (T0 + HALF_HOUR, 1.0)], true);
    let year2 = write_trace("tot-y2.parquet", &[(T0 + 2 * HALF_HOUR, 1.0)], true);
    let err = load_power_trace_mw_concat(&[year1, year2], "value", 4).unwrap_err();
    match err {
        GridError::TraceSetPeriodCount {
            expected, found, ..
        } => {
            assert_eq!(expected, 4);
            assert_eq!(found, 3);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn an_empty_file_list_is_rejected() {
    let err = load_power_trace_mw_concat(&[], "value", 1).unwrap_err();
    assert!(
        matches!(err, GridError::EmptyTraceConstruction),
        "unexpected error: {err:?}"
    );
}

// ---------------------------------------------------------------------
// Schema v6: the SPARSE loader for observed boundary-capability series
// (the B6 day-ahead limit series — rows may be missing, values may be
// null/NaN; docs/notes/b6-two-zone-data-review.md §6a).
// ---------------------------------------------------------------------

use grid_core::trace::load_sparse_power_trace_mw;

/// Write a sparse trace parquet: nullable value column, arbitrary
/// (possibly gapped) UTC timestamps.
fn write_sparse_trace(name: &str, rows: &[(i64, Option<f64>)]) -> PathBuf {
    let tz: Arc<str> = Arc::from("UTC");
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(tz.clone()));
    let schema = Arc::new(Schema::new(vec![
        Field::new("limit_mw", DataType::Float64, true),
        Field::new("utc_start", ts_type, false),
    ]));
    let mut values = Float64Builder::new();
    let mut stamps = TimestampMicrosecondBuilder::new();
    for &(t, v) in rows {
        stamps.append_value(t);
        match v {
            Some(v) => values.append_value(v),
            None => values.append_null(),
        }
    }
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(values.finish()) as ArrayRef,
            Arc::new(stamps.finish().with_timezone(tz)) as ArrayRef,
        ],
    )
    .unwrap();
    let dir = std::env::temp_dir().join("grid-sim-trace-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = parquet::arrow::ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    path
}

#[test]
fn sparse_loader_preserves_gaps_and_absent_values() {
    // A gap (period 2 missing), a null and a NaN — all preserved as
    // absences, never invented or rejected.
    let path = write_sparse_trace(
        "sparse.parquet",
        &[
            (T0, Some(4100.0)),
            (T0 + HALF_HOUR, None),
            (T0 + 3 * HALF_HOUR, Some(f64::NAN)),
            (T0 + 4 * HALF_HOUR, Some(0.0)),
        ],
    );
    let points = load_sparse_power_trace_mw(&path, "limit_mw").unwrap();
    assert_eq!(points.len(), 4);
    assert_eq!(points[0].0, UtcInstant::from_unix_micros(T0));
    assert_eq!(points[0].1, Some(Power::megawatts(4100.0)));
    assert_eq!(points[1].1, None, "null value must load as None");
    assert_eq!(
        points[2].0,
        UtcInstant::from_unix_micros(T0 + 3 * HALF_HOUR),
        "the row gap must be preserved, not filled"
    );
    assert_eq!(points[2].1, None, "NaN value must load as None");
    // A zero is a VALUE at this layer (the sentinel semantics live in
    // the scenario's capability_trace spec, not in the loader).
    assert_eq!(points[3].1, Some(Power::megawatts(0.0)));
}

#[test]
fn sparse_loader_rejects_non_increasing_timestamps() {
    let path = write_sparse_trace("sparse-dup.parquet", &[(T0, Some(1.0)), (T0, Some(2.0))]);
    let err = load_sparse_power_trace_mw(&path, "limit_mw").unwrap_err();
    assert!(
        matches!(err, GridError::TraceIndexNotUniform { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn sparse_loader_missing_file_and_column_are_clear_errors() {
    let err =
        load_sparse_power_trace_mw("/nonexistent/b6.parquet".as_ref(), "limit_mw").unwrap_err();
    assert!(err.to_string().contains("/nonexistent/b6.parquet"));

    let path = write_sparse_trace("sparse-col.parquet", &[(T0, Some(1.0))]);
    let err = load_sparse_power_trace_mw(&path, "no_such_column").unwrap_err();
    assert!(err.to_string().contains("no_such_column"));
}

/// The real b6 pack file loads (fetched-not-committed; loud failure
/// with build instructions if absent — the trace-test precedent).
#[test]
fn sparse_loader_reads_the_b6_day_ahead_limit_series() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../data/packs/b6/processed/b6_da_flows_limits.parquet");
    assert!(
        path.exists(),
        "b6 data pack is missing ({}) — build it first: scripts/fetch-b6 (fetch.py, then \
         build.py), then verify data/packs/b6.sha256",
        path.display()
    );
    let limits = load_sparse_power_trace_mw(&path, "limit_mw").unwrap();
    let flows = load_sparse_power_trace_mw(&path, "flow_mw").unwrap();
    // The pinned pack retrieval (b6.sha256, 2026-07-04): 60,006 rows
    // spanning 2023-01-01 → 2026-07-04, 2024 coverage 17,214 rows with
    // 3 NaN rows (b6_report.json, reviewer-verified).
    assert_eq!(limits.len(), 60_006);
    assert_eq!(flows.len(), 60_006);
    let start_2024 = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
    let end_2024 = UtcInstant::parse("2025-01-01T00:00:00Z").unwrap();
    let in_2024 = |points: &[(UtcInstant, Option<Power>)]| {
        points
            .iter()
            .filter(|(t, _)| *t >= start_2024 && *t < end_2024)
            .count()
    };
    let nan_2024 = |points: &[(UtcInstant, Option<Power>)]| {
        points
            .iter()
            .filter(|(t, v)| *t >= start_2024 && *t < end_2024 && v.is_none())
            .count()
    };
    assert_eq!(in_2024(&limits), 17_214);
    assert_eq!(nan_2024(&limits), 3);
    assert_eq!(nan_2024(&flows), 3);
}
