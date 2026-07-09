//! Half-hourly trace loading from the Parquet data packs (ADR-3, docs/03
//! weather data model).
//!
//! A trace file is Parquet with one or more value columns and a
//! `utc_start` index column of type `timestamp[us, tz=UTC]` (the data-pack
//! convention, [`TIME_COLUMN`]). Loading verifies, in this order:
//!
//! 1. the file exists and is readable Parquet;
//! 2. the requested column and the `utc_start` index column exist, with
//!    supported types (`float64`/`int64` values; explicitly UTC
//!    microsecond timestamps — a naive index is refused per ADR-3);
//! 3. the trace is non-empty and holds exactly the expected number of
//!    periods (17,568 for leap-year 2024);
//! 4. the index is strictly uniform 30-minute UTC spacing — this is what
//!    makes the trace "UTC-clean" through local clock-change dates, where
//!    a local-time index would gap (spring) or duplicate (autumn);
//! 5. no value is null or NaN.
//!
//! Values enter the unit system at this boundary and leave the loader as
//! newtypes ([`Power`], [`PerUnit`]); raw `f64` does not cross the public
//! API (ADR-4).

use std::fs::File;
use std::path::Path;

use arrow_array::{Array, Float64Array, Int64Array, TimestampMicrosecondArray};
use arrow_schema::{DataType, TimeUnit};
use parquet::arrow::ProjectionMask;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

use crate::GridError;
use crate::time::{HALF_HOUR_MICROS, UtcInstant};
use crate::units::{PerUnit, Power, Price, Temperature, UnitScalar};

/// Name of the UTC index column in every data-pack trace file.
pub const TIME_COLUMN: &str = "utc_start";

/// A validated half-hourly trace: a start instant plus one value per
/// settlement period, at strictly uniform 30-minute UTC spacing (the
/// loader has already verified the file's index, so the start and length
/// fully determine every period's timestamp).
#[derive(Debug, Clone, PartialEq)]
pub struct Trace<U> {
    start: UtcInstant,
    values: Vec<U>,
}

impl<U> Trace<U> {
    /// Construct a trace directly from a start instant and per-period
    /// values (one per half-hourly settlement period) — for synthetic
    /// traces and derived series (e.g. scaled demand, summed
    /// interconnector flows). File loading still goes through the
    /// validating loaders below.
    ///
    /// Errors with [`GridError::EmptyTraceConstruction`] if `values` is
    /// empty (a trace always has at least one period).
    pub fn from_parts(start: UtcInstant, values: Vec<U>) -> Result<Self, GridError> {
        if values.is_empty() {
            return Err(GridError::EmptyTraceConstruction);
        }
        Ok(Self { start, values })
    }

    /// Number of half-hourly periods.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the trace has no periods (never true for a loaded trace).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Start of the first settlement period.
    #[must_use]
    pub fn start(&self) -> UtcInstant {
        self.start
    }

    /// Start of settlement period `index`, or `None` past the end.
    #[must_use]
    pub fn timestamp_at(&self, index: usize) -> Option<UtcInstant> {
        (index < self.values.len()).then(|| self.start.plus_periods(index as i64))
    }

    /// The per-period values.
    #[must_use]
    pub fn values(&self) -> &[U] {
        &self.values
    }
}

// Statistics are implemented generically over the crate-private
// `UnitScalar` conversion (free functions, so no private bound leaks into
// the public `Trace` interface) and exposed per concrete unit type: the
// escape hatch between newtypes and raw f64 stays inside this crate.
fn mean_raw<U: UnitScalar>(values: &[U]) -> Option<U> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().map(|v| v.raw()).sum();
    Some(U::from_raw(sum / values.len() as f64))
}

fn fold_raw<U: UnitScalar>(values: &[U], pick: fn(f64, f64) -> f64) -> Option<U> {
    values.iter().map(|v| v.raw()).reduce(pick).map(U::from_raw)
}

macro_rules! trace_stats {
    ($unit:ty) => {
        impl Trace<$unit> {
            /// Arithmetic mean over all periods (`None` only if empty,
            /// which a loaded trace never is).
            #[must_use]
            pub fn mean(&self) -> Option<$unit> {
                mean_raw(&self.values)
            }

            /// Smallest value (`None` only if empty).
            #[must_use]
            pub fn min(&self) -> Option<$unit> {
                fold_raw(&self.values, f64::min)
            }

            /// Largest value (`None` only if empty).
            #[must_use]
            pub fn max(&self) -> Option<$unit> {
                fold_raw(&self.values, f64::max)
            }
        }
    };
}

trace_stats!(Power);
trace_stats!(PerUnit);
trace_stats!(Price);
trace_stats!(Temperature);

/// Load a power trace stored in **megawatts** (the data-pack demand
/// convention), converting to the canonical gigawatts.
pub fn load_power_trace_mw(
    path: &Path,
    column: &str,
    expected_periods: usize,
) -> Result<Trace<Power>, GridError> {
    let (start, values) = load_f64_column(path, column, expected_periods)?;
    Ok(Trace {
        start,
        values: values.into_iter().map(Power::megawatts).collect(),
    })
}

/// Load a price trace stored in **£/MWh** (the price-pack convention —
/// gas SAP, market index, imbalance prices carry their unit in the
/// column name). No unit conversion; negative prices are real market
/// outcomes and are kept.
pub fn load_price_trace(
    path: &Path,
    column: &str,
    expected_periods: usize,
) -> Result<Trace<Price>, GridError> {
    let (start, values) = load_f64_column(path, column, expected_periods)?;
    Ok(Trace {
        start,
        values: values
            .into_iter()
            .map(Price::pounds_per_megawatt_hour)
            .collect(),
    })
}

/// Load a temperature trace stored in **degrees Celsius** (the
/// GB t2m derivation convention — `data/weather/gb_t2m_pop.parquet`
/// column `t2m_pop` is float64 °C; docs/notes/q5-heating-data-report.md
/// §1). No unit conversion happens here (°C is the [`Temperature`]
/// canonical unit), unlike the MW→GW power loaders — the column
/// convention decision is documented at this single loading point.
///
/// Unlike the horizon-aligned loaders, this reads the **whole file**
/// (every validation of the module docs except the period count): the
/// heating overlay's pinned intensity `k` and ground-wave fit are
/// computed over the trace's full record regardless of the run horizon
/// (D9 rule 3 — `heat(t)` is a pure function of `T_pop(t)`; horizon
/// subsetting never changes it).
pub fn load_temperature_trace_c(
    path: &Path,
    column: &str,
) -> Result<Trace<Temperature>, GridError> {
    let (start, values) = load_f64_column_inner(path, column)?;
    Ok(Trace {
        start,
        values: values.into_iter().map(Temperature::celsius).collect(),
    })
}

/// Load a dimensionless per-unit trace (capacity factors, availabilities).
pub fn load_per_unit_trace(
    path: &Path,
    column: &str,
    expected_periods: usize,
) -> Result<Trace<PerUnit>, GridError> {
    let (start, values) = load_f64_column(path, column, expected_periods)?;
    Ok(Trace {
        start,
        values: values.into_iter().map(PerUnit::new).collect(),
    })
}

/// Load one `column` from several **consecutive** trace files (e.g.
/// per-year files assembling a multi-year horizon — docs/04 Stage 3),
/// concatenated in list order. Each file is validated exactly like a
/// single-file load; additionally each file must start exactly one
/// half-hour after its predecessor ends
/// ([`GridError::TraceNotConsecutive`]) and the files together must hold
/// exactly `expected_periods` ([`GridError::TraceSetPeriodCount`]). An
/// empty list is [`GridError::EmptyTraceConstruction`].
fn load_f64_concat(
    paths: &[std::path::PathBuf],
    column: &str,
    expected_periods: usize,
) -> Result<(UtcInstant, Vec<f64>), GridError> {
    let (first, rest) = paths
        .split_first()
        .ok_or(GridError::EmptyTraceConstruction)?;
    let (start, mut values) = load_f64_column_inner(first, column)?;
    let mut previous = first;
    for path in rest {
        let expected_start = start.plus_periods(values.len() as i64);
        let (file_start, file_values) = load_f64_column_inner(path, column)?;
        if file_start != expected_start {
            return Err(GridError::TraceNotConsecutive {
                path: path.clone(),
                previous: previous.clone(),
                expected: expected_start,
                found: file_start,
            });
        }
        values.extend(file_values);
        previous = path;
    }
    if values.len() != expected_periods {
        return Err(GridError::TraceSetPeriodCount {
            files: paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            expected: expected_periods,
            found: values.len(),
        });
    }
    Ok((start, values))
}

/// Multi-file counterpart of [`load_power_trace_mw`] (see
/// [`load_f64_concat`] for the assembly rules).
pub fn load_power_trace_mw_concat(
    paths: &[std::path::PathBuf],
    column: &str,
    expected_periods: usize,
) -> Result<Trace<Power>, GridError> {
    let (start, values) = load_f64_concat(paths, column, expected_periods)?;
    Ok(Trace {
        start,
        values: values.into_iter().map(Power::megawatts).collect(),
    })
}

/// Multi-file counterpart of [`load_per_unit_trace`] (see
/// [`load_f64_concat`] for the assembly rules).
pub fn load_per_unit_trace_concat(
    paths: &[std::path::PathBuf],
    column: &str,
    expected_periods: usize,
) -> Result<Trace<PerUnit>, GridError> {
    let (start, values) = load_f64_concat(paths, column, expected_periods)?;
    Ok(Trace {
        start,
        values: values.into_iter().map(PerUnit::new).collect(),
    })
}

/// Load a **sparse** power series stored in megawatts: `(utc_start,
/// value)` rows exactly as they exist in the file — rows may be missing
/// from the half-hourly grid and values may be null or NaN (returned as
/// `None`), unlike every dense loader above. This is the shape of
/// observed operational series (the B6 day-ahead boundary limit/flow
/// series, schema v6 `capability_trace`): NESO's file has missing days
/// and NaN rows, and the review ruling requires absences to STAY absent
/// ("missing periods stay missing") rather than be filled at load.
///
/// Checks: the file exists and is readable Parquet; both columns exist
/// with supported types (the index explicitly UTC, ADR-3); timestamps
/// are strictly increasing (duplicates and disorder rejected — sparse,
/// but still a time series); the file is non-empty. Zero is a VALUE at
/// this layer: sentinel semantics belong to the scenario's declared
/// capability-trace spec, never to a silent loader convention.
pub fn load_sparse_power_trace_mw(
    path: &Path,
    column: &str,
) -> Result<Vec<(UtcInstant, Option<Power>)>, GridError> {
    let (stamps, values) = load_sparse_f64_column(path, column)?;
    Ok(stamps
        .into_iter()
        .zip(values)
        .map(|(t, v)| (UtcInstant::from_unix_micros(t), v.map(Power::megawatts)))
        .collect())
}

/// The sparse counterpart of [`load_f64_column_inner`]: strictly
/// increasing (not necessarily uniform) timestamps; null/NaN values
/// preserved as `None`.
fn load_sparse_f64_column(
    path: &Path,
    column: &str,
) -> Result<(Vec<i64>, Vec<Option<f64>>), GridError> {
    if !path.exists() {
        return Err(GridError::TraceFileMissing {
            path: path.to_path_buf(),
        });
    }
    let read_err = |source: parquet::errors::ParquetError| GridError::TraceRead {
        path: path.to_path_buf(),
        source: Box::new(source),
    };

    let file = File::open(path).map_err(|source| GridError::InTraceFile {
        path: path.to_path_buf(),
        source: Box::new(GridError::Io { source }),
    })?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(read_err)?;

    let schema = builder.schema().clone();
    let column_index = |name: &str| -> Result<usize, GridError> {
        schema
            .index_of(name)
            .map_err(|_| GridError::TraceColumnMissing {
                path: path.to_path_buf(),
                column: name.to_owned(),
            })
    };
    let value_index = column_index(column)?;
    let time_index = column_index(TIME_COLUMN)?;

    let time_field = schema.field(time_index);
    match time_field.data_type() {
        DataType::Timestamp(TimeUnit::Microsecond, Some(tz))
            if matches!(tz.as_ref(), "UTC" | "utc" | "+00:00") => {}
        other => {
            return Err(GridError::TraceColumnType {
                path: path.to_path_buf(),
                column: TIME_COLUMN.to_owned(),
                found: format!("{other}"),
                expected: "timestamp[us, tz=UTC] (naive or local indices are refused; ADR-3)"
                    .to_owned(),
            });
        }
    }

    let mask = ProjectionMask::roots(builder.parquet_schema(), [value_index, time_index]);
    let reader = builder.with_projection(mask).build().map_err(read_err)?;

    let mut stamps: Vec<i64> = Vec::new();
    let mut values: Vec<Option<f64>> = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|e| read_err(e.into()))?;
        let by_name = |name: &str| {
            batch
                .column_by_name(name)
                .ok_or_else(|| GridError::TraceColumnMissing {
                    path: path.to_path_buf(),
                    column: name.to_owned(),
                })
        };

        let time_col = by_name(TIME_COLUMN)?;
        let time_col = time_col
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .ok_or_else(|| GridError::TraceColumnType {
                path: path.to_path_buf(),
                column: TIME_COLUMN.to_owned(),
                found: format!("{}", time_col.data_type()),
                expected: "timestamp[us, tz=UTC]".to_owned(),
            })?;
        if time_col.null_count() > 0 {
            return Err(GridError::TraceIndexNotUniform {
                path: path.to_path_buf(),
                reason: "index contains nulls".to_owned(),
            });
        }
        stamps.extend(time_col.values().iter().copied());

        let value_col = by_name(column)?;
        let arr = value_col
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| GridError::TraceColumnType {
                path: path.to_path_buf(),
                column: column.to_owned(),
                found: format!("{}", value_col.data_type()),
                expected: "float64 (sparse loader)".to_owned(),
            })?;
        for i in 0..arr.len() {
            if arr.is_null(i) {
                values.push(None);
            } else {
                let v = arr.value(i);
                values.push(if v.is_nan() { None } else { Some(v) });
            }
        }
    }

    if stamps.is_empty() {
        return Err(GridError::TraceEmpty {
            path: path.to_path_buf(),
        });
    }
    // Strictly increasing: a sparse series may gap, never duplicate or
    // run backwards.
    for (i, pair) in stamps.windows(2).enumerate() {
        if pair[1] <= pair[0] {
            return Err(GridError::TraceIndexNotUniform {
                path: path.to_path_buf(),
                reason: format!(
                    "sparse index not strictly increasing: row {} at {} follows row {} at {}",
                    i + 1,
                    UtcInstant::from_unix_micros(pair[1]),
                    i,
                    UtcInstant::from_unix_micros(pair[0]),
                ),
            });
        }
    }
    Ok((stamps, values))
}

/// Read one value column plus the UTC index, applying every check listed
/// in the module docs. Returns the validated start instant and raw values
/// in the file's own unit (unit conversion is the callers' job).
fn load_f64_column(
    path: &Path,
    column: &str,
    expected_periods: usize,
) -> Result<(UtcInstant, Vec<f64>), GridError> {
    let (start, values) = load_f64_column_inner(path, column)?;
    if values.len() != expected_periods {
        return Err(GridError::TracePeriodCount {
            path: path.to_path_buf(),
            expected: expected_periods,
            found: values.len(),
        });
    }
    Ok((start, values))
}

/// Shared single-file loader: every module-docs check except the
/// caller's expected period count.
fn load_f64_column_inner(path: &Path, column: &str) -> Result<(UtcInstant, Vec<f64>), GridError> {
    if !path.exists() {
        return Err(GridError::TraceFileMissing {
            path: path.to_path_buf(),
        });
    }
    let read_err = |source: parquet::errors::ParquetError| GridError::TraceRead {
        path: path.to_path_buf(),
        source: Box::new(source),
    };

    let file = File::open(path).map_err(|source| GridError::InTraceFile {
        path: path.to_path_buf(),
        source: Box::new(GridError::Io { source }),
    })?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(read_err)?;

    // Project down to the two columns of interest.
    let schema = builder.schema().clone();
    let column_index = |name: &str| -> Result<usize, GridError> {
        schema
            .index_of(name)
            .map_err(|_| GridError::TraceColumnMissing {
                path: path.to_path_buf(),
                column: name.to_owned(),
            })
    };
    let value_index = column_index(column)?;
    let time_index = column_index(TIME_COLUMN)?;

    // Refuse non-UTC or naive indices up front (ADR-3).
    let time_field = schema.field(time_index);
    match time_field.data_type() {
        DataType::Timestamp(TimeUnit::Microsecond, Some(tz))
            if matches!(tz.as_ref(), "UTC" | "utc" | "+00:00") => {}
        other => {
            return Err(GridError::TraceColumnType {
                path: path.to_path_buf(),
                column: TIME_COLUMN.to_owned(),
                found: format!("{other}"),
                expected: "timestamp[us, tz=UTC] (naive or local indices are refused; ADR-3)"
                    .to_owned(),
            });
        }
    }

    let mask = ProjectionMask::roots(builder.parquet_schema(), [value_index, time_index]);
    let reader = builder.with_projection(mask).build().map_err(read_err)?;

    let mut stamps: Vec<i64> = Vec::new();
    let mut values: Vec<f64> = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|e| read_err(e.into()))?;
        // Look columns up by name: projection preserves names, not the
        // original indices.
        let by_name = |name: &str| {
            batch
                .column_by_name(name)
                .ok_or_else(|| GridError::TraceColumnMissing {
                    path: path.to_path_buf(),
                    column: name.to_owned(),
                })
        };

        let time_col = by_name(TIME_COLUMN)?;
        let time_col = time_col
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .ok_or_else(|| GridError::TraceColumnType {
                path: path.to_path_buf(),
                column: TIME_COLUMN.to_owned(),
                found: format!("{}", time_col.data_type()),
                expected: "timestamp[us, tz=UTC]".to_owned(),
            })?;
        if time_col.null_count() > 0 {
            return Err(GridError::TraceIndexNotUniform {
                path: path.to_path_buf(),
                reason: "index contains nulls".to_owned(),
            });
        }
        stamps.extend(time_col.values().iter().copied());

        let value_col = by_name(column)?;
        let start_row = values.len();
        match value_col.data_type() {
            DataType::Float64 => {
                let arr = value_col
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .ok_or_else(|| GridError::TraceColumnType {
                        path: path.to_path_buf(),
                        column: column.to_owned(),
                        found: format!("{}", value_col.data_type()),
                        expected: "float64 or int64".to_owned(),
                    })?;
                push_values(path, column, start_row, arr, &mut values, |a, i| a.value(i))?;
            }
            DataType::Int64 => {
                let arr = value_col
                    .as_any()
                    .downcast_ref::<Int64Array>()
                    .ok_or_else(|| GridError::TraceColumnType {
                        path: path.to_path_buf(),
                        column: column.to_owned(),
                        found: format!("{}", value_col.data_type()),
                        expected: "float64 or int64".to_owned(),
                    })?;
                push_values(path, column, start_row, arr, &mut values, |a, i| {
                    a.value(i) as f64
                })?;
            }
            other => {
                return Err(GridError::TraceColumnType {
                    path: path.to_path_buf(),
                    column: column.to_owned(),
                    found: format!("{other}"),
                    expected: "float64 or int64".to_owned(),
                });
            }
        }
    }

    // Non-empty (the period count is the callers' check: exact for a
    // single file, exact-in-total for multi-file assembly).
    if stamps.is_empty() {
        return Err(GridError::TraceEmpty {
            path: path.to_path_buf(),
        });
    }

    // Strictly uniform half-hourly UTC spacing, no gaps, no duplicates.
    for (i, pair) in stamps.windows(2).enumerate() {
        let step = pair[1] - pair[0];
        if step != HALF_HOUR_MICROS {
            return Err(GridError::TraceIndexNotUniform {
                path: path.to_path_buf(),
                reason: format!(
                    "period {} at {} is {} seconds after period {} at {}; expected exactly 1800",
                    i + 1,
                    UtcInstant::from_unix_micros(pair[1]),
                    step / 1_000_000,
                    i,
                    UtcInstant::from_unix_micros(pair[0]),
                ),
            });
        }
    }

    Ok((UtcInstant::from_unix_micros(stamps[0]), values))
}

/// Append one batch's values, rejecting nulls and NaNs with the offending
/// whole-trace row index.
fn push_values<A: Array>(
    path: &Path,
    column: &str,
    start_row: usize,
    arr: &A,
    out: &mut Vec<f64>,
    get: impl Fn(&A, usize) -> f64,
) -> Result<(), GridError> {
    let nan_at = |index: usize| GridError::TraceNan {
        path: path.to_path_buf(),
        column: column.to_owned(),
        index,
    };
    for i in 0..arr.len() {
        if arr.is_null(i) {
            return Err(nan_at(start_row + i));
        }
        let v = get(arr, i);
        if v.is_nan() {
            return Err(nan_at(start_row + i));
        }
        out.push(v);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn from_parts_builds_a_trace_and_rejects_empty() {
        let start = UtcInstant::from_unix_micros(0);
        let trace = Trace::from_parts(start, vec![Power::gigawatts(1.0); 3]).unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.start(), start);
        assert_eq!(trace.timestamp_at(2), Some(start.plus_periods(2)));
        assert!(matches!(
            Trace::<Power>::from_parts(start, vec![]),
            Err(GridError::EmptyTraceConstruction)
        ));
    }
}
