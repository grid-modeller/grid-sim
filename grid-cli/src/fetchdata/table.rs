//! In-memory processed tables and their CSV/Parquet forms.
//!
//! A processed output is a `utc_start` index (half-hourly UTC instants)
//! plus named columns of `int64` or `float64` — exactly the value types
//! the Python-built pack carries (NESO demand columns are integers;
//! everything derived is float64). Writers reproduce the pack
//! conventions:
//!
//! - CSV: `utc_start` first, `YYYY-MM-DDTHH:MM:SSZ` timestamps, floats in
//!   shortest round-trip form with a pandas-style trailing `.0` on
//!   integral values (so a value-identical build is also byte-identical
//!   CSV in practice);
//! - Parquet: snappy-compressed, `timestamp[us, tz=UTC]` index — the
//!   grid-core trace-loader contract. Parquet *bytes* are writer-specific
//!   and are not expected to match the pyarrow-built files; value
//!   identity is checked cell-by-cell by [`super::checks`].

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, Float64Array, Int64Array, RecordBatch, TimestampMicrosecondArray,
};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use grid_core::time::UtcInstant;

use super::error::FetchDataError;

/// One named column of values.
#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    /// 64-bit integers (the NESO demand columns).
    Int64(Vec<i64>),
    /// 64-bit floats (everything derived or pivoted).
    Float64(Vec<f64>),
}

impl Column {
    /// Number of values.
    pub fn len(&self) -> usize {
        match self {
            Column::Int64(v) => v.len(),
            Column::Float64(v) => v.len(),
        }
    }
}

/// A processed table: UTC half-hourly index plus named columns, all the
/// same length.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// The `utc_start` index.
    pub index: Vec<UtcInstant>,
    /// `(name, values)` in output order.
    pub columns: Vec<(String, Column)>,
}

impl Table {
    /// Construct, checking every column matches the index length.
    pub fn new(
        output: &'static str,
        index: Vec<UtcInstant>,
        columns: Vec<(String, Column)>,
    ) -> Result<Self, FetchDataError> {
        for (name, column) in &columns {
            if column.len() != index.len() {
                return Err(FetchDataError::Build {
                    output,
                    reason: format!(
                        "column {name} has {} values for {} index rows",
                        column.len(),
                        index.len()
                    ),
                });
            }
        }
        Ok(Self { index, columns })
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// The named column, if present.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }

    /// Write the pack CSV form (see module docs).
    pub fn write_csv(&self, path: &Path) -> Result<(), FetchDataError> {
        let io = |source: std::io::Error| FetchDataError::io(path, source);
        let mut out = BufWriter::new(File::create(path).map_err(io)?);
        let mut header = String::from("utc_start");
        for (name, _) in &self.columns {
            header.push(',');
            header.push_str(name);
        }
        writeln!(out, "{header}").map_err(io)?;
        for (row, instant) in self.index.iter().enumerate() {
            let mut line = instant.to_string();
            for (_, column) in &self.columns {
                line.push(',');
                match column {
                    Column::Int64(v) => line.push_str(&v[row].to_string()),
                    Column::Float64(v) => line.push_str(&format_f64_pandas(v[row])),
                }
            }
            writeln!(out, "{line}").map_err(io)?;
        }
        out.flush().map_err(io)
    }

    /// Write the pack Parquet form (see module docs).
    pub fn write_parquet(&self, path: &Path) -> Result<(), FetchDataError> {
        let table_err = |reason: String| FetchDataError::Table {
            path: path.to_path_buf(),
            reason,
        };
        let mut fields = vec![Field::new(
            "utc_start",
            DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
            false,
        )];
        let mut arrays: Vec<ArrayRef> = vec![Arc::new(
            TimestampMicrosecondArray::from(
                self.index
                    .iter()
                    .map(|t| t.unix_micros())
                    .collect::<Vec<_>>(),
            )
            .with_timezone("UTC"),
        )];
        for (name, column) in &self.columns {
            match column {
                Column::Int64(v) => {
                    fields.push(Field::new(name, DataType::Int64, false));
                    arrays.push(Arc::new(Int64Array::from(v.clone())));
                }
                Column::Float64(v) => {
                    fields.push(Field::new(name, DataType::Float64, false));
                    arrays.push(Arc::new(Float64Array::from(v.clone())));
                }
            }
        }
        let schema = Arc::new(Schema::new(fields));
        let batch =
            RecordBatch::try_new(schema.clone(), arrays).map_err(|e| table_err(e.to_string()))?;
        let file = File::create(path).map_err(|source| FetchDataError::io(path, source))?;
        let props = WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build();
        let mut writer = ArrowWriter::try_new(file, schema, Some(props))
            .map_err(|e| table_err(e.to_string()))?;
        writer.write(&batch).map_err(|e| table_err(e.to_string()))?;
        writer.close().map_err(|e| table_err(e.to_string()))?;
        Ok(())
    }

    /// Read a processed Parquet file back into a [`Table`] (used by the
    /// comparison harness on the reference pack). Accepts exactly the
    /// pack schema: a `timestamp[us, tz=UTC]` `utc_start` column plus
    /// `int64`/`float64` value columns, no nulls.
    pub fn read_parquet(path: &Path) -> Result<Self, FetchDataError> {
        let table_err = |reason: String| FetchDataError::Table {
            path: path.to_path_buf(),
            reason,
        };
        let file = File::open(path).map_err(|source| FetchDataError::io(path, source))?;
        let builder =
            ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| table_err(e.to_string()))?;
        let schema = builder.schema().clone();
        let reader = builder.build().map_err(|e| table_err(e.to_string()))?;

        let mut index: Vec<UtcInstant> = Vec::new();
        let mut columns: Vec<(String, Column)> = schema
            .fields()
            .iter()
            .filter(|f| f.name() != "utc_start")
            .map(|f| match f.data_type() {
                DataType::Int64 => Ok((f.name().clone(), Column::Int64(Vec::new()))),
                DataType::Float64 => Ok((f.name().clone(), Column::Float64(Vec::new()))),
                other => Err(table_err(format!(
                    "column {} has unsupported type {other} (pack columns are int64/float64)",
                    f.name()
                ))),
            })
            .collect::<Result<_, _>>()?;

        for batch in reader {
            let batch = batch.map_err(|e| table_err(e.to_string()))?;
            let time = batch
                .column_by_name("utc_start")
                .and_then(|c| c.as_any().downcast_ref::<TimestampMicrosecondArray>())
                .ok_or_else(|| {
                    table_err("utc_start column missing or not timestamp[us]".to_owned())
                })?;
            if time.null_count() > 0 {
                return Err(table_err("nulls in utc_start".to_owned()));
            }
            index.extend(
                time.values()
                    .iter()
                    .map(|micros| UtcInstant::from_unix_micros(*micros)),
            );
            for (name, column) in &mut columns {
                let array = batch
                    .column_by_name(name)
                    .ok_or_else(|| table_err(format!("column {name} missing from a batch")))?;
                if array.null_count() > 0 {
                    return Err(table_err(format!("nulls in column {name}")));
                }
                match column {
                    Column::Int64(values) => {
                        let array = array
                            .as_any()
                            .downcast_ref::<Int64Array>()
                            .ok_or_else(|| table_err(format!("column {name}: type changed")))?;
                        values.extend(array.values().iter().copied());
                    }
                    Column::Float64(values) => {
                        let array = array
                            .as_any()
                            .downcast_ref::<Float64Array>()
                            .ok_or_else(|| table_err(format!("column {name}: type changed")))?;
                        values.extend(array.values().iter().copied());
                    }
                }
            }
        }
        Table::new("read_parquet", index, columns)
    }
}

/// Shortest round-trip float formatting with a pandas-style trailing
/// `.0` on integral values (Python's `repr(float)` and Rust's `Display`
/// both print the shortest decimal that round-trips; they differ only on
/// integral floats, where Python keeps the `.0`).
///
/// Known divergence (reviewer-noted): Python switches to scientific
/// notation for |v| < 1e-4 and >= 1e16; Rust `Display` never does. A
/// future value in that range would break CSV *byte* identity with a
/// pandas-built pack — cell-exact parquet comparison (the acceptance
/// standard) is unaffected.
fn format_f64_pandas(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e16 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn float_formatting_matches_pandas_conventions() {
        assert_eq!(format_f64_pandas(1194.0), "1194.0");
        assert_eq!(format_f64_pandas(-0.0), "-0.0");
        assert_eq!(format_f64_pandas(0.0), "0.0");
        assert_eq!(format_f64_pandas(17.083), "17.083");
        assert_eq!(
            format_f64_pandas(0.356_725_085_910_652_94),
            "0.35672508591065294"
        );
        assert_eq!(format_f64_pandas(-179.0), "-179.0");
    }

    #[test]
    fn rejects_ragged_columns() {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        let index = vec![start, start.plus_periods(1)];
        let bad = Table::new(
            "t",
            index,
            vec![("x".to_owned(), Column::Int64(vec![1, 2, 3]))],
        );
        assert!(bad.is_err());
    }

    #[test]
    fn parquet_round_trips_values_and_index() {
        let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
        let index: Vec<UtcInstant> = (0..4).map(|i| start.plus_periods(i)).collect();
        let table = Table::new(
            "t",
            index,
            vec![
                ("a".to_owned(), Column::Int64(vec![1, -2, 3, 4])),
                (
                    "b".to_owned(),
                    Column::Float64(vec![0.5, -0.0, 1194.0, 0.35672508591065294]),
                ),
            ],
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("gridsim-table-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("round_trip.parquet");
        table.write_parquet(&path).unwrap();
        let back = Table::read_parquet(&path).unwrap();
        assert_eq!(table, back);
    }

    #[test]
    fn csv_form_is_the_pack_convention() {
        let start = UtcInstant::parse("2024-12-31T23:00:00Z").unwrap();
        let index: Vec<UtcInstant> = (0..2).map(|i| start.plus_periods(i)).collect();
        let table = Table::new(
            "t",
            index,
            vec![
                ("nd".to_owned(), Column::Int64(vec![21783, -71])),
                ("cf".to_owned(), Column::Float64(vec![0.5, 1194.0])),
            ],
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("gridsim-table-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("form.csv");
        table.write_csv(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            "utc_start,nd,cf\n\
             2024-12-31T23:00:00Z,21783,0.5\n\
             2024-12-31T23:30:00Z,-71,1194.0\n"
        );
    }
}
