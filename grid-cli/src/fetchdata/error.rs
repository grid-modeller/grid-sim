//! Structured errors for the data-pack builder.
//!
//! `fetch-data` lives in `grid-cli` (network and filesystem edges are CLI
//! territory), but is held to the library standard: no panics, every
//! failure a structured `Result` (docs/06). Only `main` converts these to
//! process exit codes.

use std::path::PathBuf;

use grid_core::time::UtcInstant;

/// Everything that can go wrong fetching, building, validating or
/// comparing a data pack.
#[derive(Debug, thiserror::Error)]
pub enum FetchDataError {
    /// The requested pack year has no pinned source URLs yet.
    ///
    /// Sources are pinned per year (fixed URLs and date ranges, ADR-5
    /// determinism); supporting a new year is a deliberate change, not a
    /// URL template guess.
    #[error(
        "no pinned data sources for year {year}: only {supported} is supported \
         (adding a year means pinning its NESO resource URL and Elexon date \
         ranges in fetchdata::sources)"
    )]
    UnsupportedYear {
        /// The year asked for.
        year: u16,
        /// The years with pinned sources.
        supported: &'static str,
    },

    /// An HTTP fetch failed after all retries.
    #[error("fetching {url} failed after {attempts} attempts: {reason}")]
    Fetch {
        /// Source URL.
        url: String,
        /// Retry attempts made.
        attempts: u32,
        /// Final error.
        reason: String,
    },

    /// A fetched response failed the basic shape check (e.g. an Elexon
    /// stream response that is not a non-empty JSON array).
    #[error("unexpected response from {url}: {reason}")]
    BadResponse {
        /// Source URL.
        url: String,
        /// What was wrong.
        reason: String,
    },

    /// Filesystem I/O, with the path that failed.
    #[error("{path}: {source}")]
    Io {
        /// The file or directory involved.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// A raw input file is missing (e.g. `--skip-fetch` without a
    /// previously fetched `raw/` directory).
    #[error(
        "raw input {path} is missing — run without --skip-fetch to fetch it \
         (raw files land in <out>/raw and existing files are never re-fetched)"
    )]
    RawFileMissing {
        /// The expected raw file.
        path: PathBuf,
    },

    /// A raw file failed to parse.
    #[error("parsing {path}: {reason}")]
    RawParse {
        /// The raw file.
        path: PathBuf,
        /// What failed, with row/cell context where available.
        reason: String,
    },

    /// The generation pivot has a hole that is not the documented
    /// INTGRNL pre-go-live case (build.py would emit NaN and fail
    /// validation later; the port fails at build time, naming the cell).
    #[error(
        "generation_by_fuel: no record for fuel {fuel} at {utc_start} \
         (only INTGRNL absences are documented as genuine zero flow)"
    )]
    GenerationGap {
        /// The fuel-type column (lowercased).
        fuel: String,
        /// The half-hour with no record.
        utc_start: UtcInstant,
    },

    /// The demand and generation traces do not line up period-for-period
    /// (the wind-CF build needs aligned indices).
    #[error("wind_cf: demand and generation indices differ at row {row}: {demand} vs {generation}")]
    Misaligned {
        /// Row number (0-based).
        row: usize,
        /// Demand-side timestamp.
        demand: UtcInstant,
        /// Generation-side timestamp.
        generation: UtcInstant,
    },

    /// A build-time invariant failed (counts, duplicates, calendar holes).
    #[error("building {output}: {reason}")]
    Build {
        /// Which processed output.
        output: &'static str,
        /// The violated invariant.
        reason: String,
    },

    /// Writing or reading a processed table failed.
    #[error("{path}: {reason}")]
    Table {
        /// The processed file.
        path: PathBuf,
        /// What failed.
        reason: String,
    },

    /// A grid-core error (timestamp parsing and the like).
    #[error(transparent)]
    Core(#[from] grid_core::GridError),
}

impl FetchDataError {
    /// Shorthand for [`FetchDataError::Io`].
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
