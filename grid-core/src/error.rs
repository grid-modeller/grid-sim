//! Structured error type for all grid-sim library crates (docs/06).

/// Errors returned by grid-sim library APIs.
///
/// Structured via `thiserror`; scenario-file errors carry file and
/// line/column context for user-facing messages (docs/06).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GridError {
    /// A timestamp string is not strict RFC 3339 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
    #[error(
        "invalid UTC timestamp {value:?}: {reason} (ADR-3: UTC only, form YYYY-MM-DDTHH:MM:SSZ)"
    )]
    InvalidTimestamp {
        /// The offending text.
        value: String,
        /// Why it was rejected.
        reason: String,
    },

    /// A horizon's start/end pair does not describe a whole, forward run
    /// of half-hourly settlement periods.
    #[error("invalid horizon: {reason}")]
    InvalidHorizon {
        /// Why it was rejected.
        reason: String,
    },

    /// The scenario has no `schema_version` field, which is mandatory
    /// (ADR-5): without it the file cannot be interpreted safely.
    #[error(
        "missing mandatory `schema_version` field (this engine reads schema_version = {supported})",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    MissingSchemaVersion,

    /// The scenario declares `schema_version = 1`, which schema v2
    /// superseded (Stage 3, 2026-07-02): the Stage 1 run-inputs companion
    /// file was folded into the scenario. The message is the migration
    /// note (docs/05 rule 4: old files fail with a clear migration
    /// message naming what moved, never a field-level error).
    #[error(
        "schema_version 1 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v2 folded the Stage 1 run-inputs companion file into the scenario \
         itself. To migrate, move from the run-inputs file into the scenario: the [demand] \
         table's column and extra_demand_gw into [zones.demand]; each [[exogenous_supply]] \
         table into [[zones.exogenous_supply]]; each [availability.<tech>] table onto its \
         [[zones.fleet]] entry as `availability = {{ flat = … }}` or `{{ monthly = [ … ] }}`; \
         and the [pricing] section into a top-level [pricing] block. Then set schema_version = \
         {supported} and drop the run-inputs file (`grid-cli run --inputs` no longer exists). \
         Full migration note: docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion1Superseded,

    /// The scenario declares `schema_version = 2`, which schema v3
    /// superseded (Stage 6, 2026-07-03): `[[zones.fleet]]` entries
    /// gained the optional stability-metadata fields. The message is
    /// the migration note (docs/05 rule 4).
    #[error(
        "schema_version 2 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v3 added the Stage 6 stability metadata to [[zones.fleet]] \
         entries — optional `inertia_h` (the inertia constant H in seconds, i.e. GVA·s per \
         GVA of machine rating) and optional `synchronous` (bool), with defaults derived per \
         technology from data/reference/inertia-constants.toml (MVA = GW / 0.9 convention). \
         Both fields are optional, so to migrate a v2 file set schema_version = {supported} \
         and change nothing else; add explicit inertia_h/synchronous only to override the \
         derived defaults (overrides are surfaced in run outputs). Full migration note: \
         docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion2Superseded,

    /// The scenario declares `schema_version = 3`, which schema v4
    /// superseded (Stage 5, 2026-07-03): the multi-zone activation
    /// fields. The message is the migration note (docs/05 rule 4).
    #[error(
        "schema_version 3 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v4 added the Stage 5 multi-zone activation fields — on \
         [[links]] an optional `name` (per-link identity for outputs) and a `loss` fraction \
         (default 0; the receiving end gets sent × (1 − loss)); on [[zones.fleet]] an \
         optional `energy_budget` table (`trace`, `columns`, `window_periods`, default 336 — \
         the seasonal-budget reservoir hydro model); and on [zones.demand] an optional \
         `extra_profiles` list of {{ path, column }} MW traces summed onto the base profile \
         (aggregate-zone demand). All are optional or defaulted, so to migrate a v3 file set \
         schema_version = {supported} and change nothing else. Full migration note: \
         docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion3Superseded,

    /// The scenario declares `schema_version = 4`, which schema v5
    /// superseded (Q5/Q11 heating overlay, 2026-07-03,
    /// docs/notes/d9-heating-overlay.md rule 2): the v1–v4
    /// `[zones.demand.heating]` sketch block was REPLACED by the
    /// heating technology portfolio, so v5 is not purely additive. The
    /// message is the migration note (docs/05 rule 4).
    #[error(
        "schema_version 4 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v5 replaced the v1-v4 [zones.demand.heating] sketch block \
         (`enabled` / `heat_demand_per_degree` / `cop_curve` — opaque placeholders no engine \
         code ever read) with the heating technology portfolio (D9 rule 2): \
         `delivered_heat_twh` (record-mean annual buildings-heat quantum, TWh), \
         `electrified_share`, `dhw_fraction`, `temperature_trace = {{ path, column }}`, and \
         [[zones.demand.heating.entries]] tables (kind = \"ashp\" | \"gshp\" | \
         \"district_geothermal\", share, optional per-entry COP overrides; shares sum to 1). \
         To migrate a v4 file WITHOUT a [zones.demand.heating] block, change only the \
         version line to schema_version = {supported}. A v4 file CARRYING the old block must \
         also delete it (it was engine-inert) or rewrite it in the portfolio form. Default \
         COP parameters live in data/reference/heating-cop.toml. Full migration note: \
         docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion4Superseded,

    /// The scenario declares `schema_version = 5`, which schema v6
    /// superseded (the B6 two-zone package, 2026-07-04): per-direction
    /// and time-series link capability plus the exogenous split
    /// multiplier. The message is the migration note (docs/05 rule 4).
    #[error(
        "schema_version 5 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v6 added the B6 two-zone link-capability fields \
         (docs/notes/b6-two-zone-data-review.md §6 ruling) — on [[links]] an optional \
         `reverse_capacity_gw` (capability of the to → from direction; absent = symmetric at \
         capacity_gw) and an optional [links.capability_trace] table (`path`, `column`, \
         `sentinel_high_mw`, `upper_bound_gw`, `masked_fill_gw`: a sparse per-period forward \
         capability series in MW, with values >= the sentinel threshold replaced by the pinned \
         upper bound and zero/NaN/missing rows masked out of gate arithmetic and filled with \
         the pinned central value); and on [[zones.exogenous_supply]] an optional `scale` \
         (default 1.0) splitting a national series across zones. All are optional or \
         defaulted, so to migrate a v5 file change only the version line to schema_version = \
         {supported}. Full migration note: docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion5Superseded,

    /// The scenario declares `schema_version = 6`, which schema v7
    /// superseded (the D11 priced multi-zone dispatch package,
    /// 2026-07-05): the per-zone pricing block and the flow-signal
    /// selector. The message is the migration note (docs/05 rule 4).
    #[error(
        "schema_version 6 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v7 added the D11 priced-dispatch fields \
         (docs/notes/d11-priced-dispatch.md) — an optional [zones.pricing] block per zone \
         (`reference`, optional `carbon_flat_gbp_per_tco2` — absent = the reference file's \
         UKA+CPS step series, present = a flat per-zone carbon level in £/tCO2 — plus \
         `fuel_price` traces and `srmc` recipes, the Stage 2 recipe applied per zone) and an \
         optional [dispatch] `flow_signal` selector (\"scarcity\", the default, or \
         \"priced_ladder\", which requires [zones.pricing] on every zone). All are optional \
         or defaulted, so to migrate a v6 file change only the version line to \
         schema_version = {supported}. Full migration note: docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion6Superseded,

    /// The scenario declares `schema_version = 7`, which schema v8
    /// superseded (the D16 geothermal depth continuum, 2026-07-06):
    /// the optional GSHP resource depth. The message is the migration
    /// note (docs/05 rule 4).
    #[error(
        "schema_version 7 scenarios are no longer read (this engine reads schema_version = \
         {supported}): schema v8 added the D16 geothermal depth-continuum field \
         (docs/notes/d16-geothermal-source-temperature.md) — on gshp \
         [[zones.demand.heating.entries]] an optional `resource_depth_m` (geothermal resource \
         depth in metres; absent = the committed shallow-loop behaviour, byte-identical; \
         present = the source mean warms with depth via the BGS-cited gradient in \
         data/reference/heating-cop.toml [geothermal], with the direct-use handoff to the \
         district effective COP). The field is optional, so to migrate a v7 file change only \
         the version line to schema_version = {supported}. Full migration note: \
         docs/03-domain-model.md",
        supported = crate::scenario::SCHEMA_VERSION
    )]
    SchemaVersion7Superseded,

    /// The scenario declares a `schema_version` this engine does not read.
    #[error("unsupported schema_version {found}: this engine reads schema_version = {supported}")]
    UnsupportedSchemaVersion {
        /// The version the file declares.
        found: i64,
        /// The version this engine supports.
        supported: u32,
    },

    /// The scenario TOML failed to parse; the source error carries
    /// line/column context from the TOML parser.
    #[error("scenario parse error: {source}")]
    ScenarioParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A scenario failed to serialise to TOML.
    #[error("scenario serialise error: {source}")]
    ScenarioSerialise {
        /// The underlying TOML error.
        #[source]
        source: Box<toml::ser::Error>,
    },

    /// Wraps any error with the scenario file it arose in, for
    /// user-facing messages (docs/06).
    #[error("in scenario file {path}: {source}", path = path.display())]
    InScenarioFile {
        /// The scenario file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// An I/O failure.
    #[error("I/O error: {source}")]
    Io {
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A trace file does not exist on disk.
    #[error(
        "trace file not found: {path} (data packs are fetched and built locally, \
         not committed — build the data pack first; see docs/03-domain-model.md)",
        path = path.display()
    )]
    TraceFileMissing {
        /// The missing file.
        path: std::path::PathBuf,
    },

    /// A trace file could not be read as Parquet.
    #[error("failed to read trace file {path}: {source}", path = path.display())]
    TraceRead {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// The underlying Parquet/Arrow error.
        #[source]
        source: Box<parquet::errors::ParquetError>,
    },

    /// A trace file lacks a required column.
    #[error("trace file {path} has no column named {column:?}", path = path.display())]
    TraceColumnMissing {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// The requested column.
        column: String,
    },

    /// A trace column has an unsupported type.
    #[error(
        "trace column {column:?} in {path} has type {found}; expected {expected}",
        path = path.display()
    )]
    TraceColumnType {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// The offending column.
        column: String,
        /// The type found in the file.
        found: String,
        /// The type(s) the loader accepts.
        expected: String,
    },

    /// A trace file holds no periods at all.
    #[error("trace file {path} is empty", path = path.display())]
    TraceEmpty {
        /// The trace file being read.
        path: std::path::PathBuf,
    },

    /// A trace does not hold exactly the expected number of half-hourly
    /// periods.
    #[error(
        "trace file {path} has {found} periods; expected exactly {expected}",
        path = path.display()
    )]
    TracePeriodCount {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// The period count required by the caller (e.g. from the horizon).
        expected: usize,
        /// The period count found in the file.
        found: usize,
    },

    /// In a multi-file trace, a file does not start exactly one
    /// half-hour after its predecessor ends (multi-file traces assemble
    /// consecutive per-year files into one horizon — docs/04 Stage 3).
    #[error(
        "trace file {path} does not continue {previous}: it starts at {found}, expected \
         {expected} (multi-file traces must be consecutive, in list order)",
        path = path.display(),
        previous = previous.display()
    )]
    TraceNotConsecutive {
        /// The out-of-place file.
        path: std::path::PathBuf,
        /// The file it should continue.
        previous: std::path::PathBuf,
        /// Where the file should have started.
        expected: crate::time::UtcInstant,
        /// Where it actually starts.
        found: crate::time::UtcInstant,
    },

    /// A multi-file trace does not hold exactly the expected number of
    /// half-hourly periods in total.
    #[error("trace files [{files}] hold {found} periods in total; expected exactly {expected}")]
    TraceSetPeriodCount {
        /// The files, in list order.
        files: String,
        /// The period count required by the caller (e.g. from the horizon).
        expected: usize,
        /// The total period count found.
        found: usize,
    },

    /// A trace index is not strictly uniform 30-minute UTC spacing (gaps,
    /// duplicates, or nulls — the shapes a local-time index acquires at
    /// clock changes; ADR-3).
    #[error(
        "trace file {path} is not uniformly half-hourly UTC: {reason}",
        path = path.display()
    )]
    TraceIndexNotUniform {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// Which periods are misspaced and how.
        reason: String,
    },

    /// A trace value is NaN or null.
    #[error(
        "trace column {column:?} in {path} has a NaN or null value at period {index}",
        path = path.display()
    )]
    TraceNan {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// The offending column.
        column: String,
        /// The offending period index (0-based).
        index: usize,
    },

    /// Wraps any error with the trace file it arose in.
    #[error("in trace file {path}: {source}", path = path.display())]
    InTraceFile {
        /// The trace file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// A trace constructed in memory would have no periods.
    #[error("cannot construct an empty trace")]
    EmptyTraceConstruction,

    /// A loaded trace starts at a different instant than the run horizon
    /// requires (every input trace must cover exactly the horizon).
    #[error("{context}: trace starts at {found}; the horizon starts at {expected}")]
    TraceStartMismatch {
        /// Which trace is misaligned (path or label).
        context: String,
        /// The horizon's first period start.
        expected: crate::time::UtcInstant,
        /// The trace's first period start.
        found: crate::time::UtcInstant,
    },

    /// The scenario has more than one zone. The schema is multi-zone from
    /// day one (ADR-7) but the v1 engine is single-zone until Stage 5.
    #[error(
        "scenario has {found} zones; the engine is single-zone until Stage 5 \
         (ADR-7 — the schema supports multiple zones, the v1 engine rejects them)"
    )]
    MultiZoneUnsupported {
        /// Number of zones in the scenario.
        found: usize,
    },

    /// A fleet technology has no capacity-factor trace (so it is not
    /// weather-driven must-take) and no rung on the merit ladder the
    /// engine consulted. Two ladders exist (Stage 7 split, documented
    /// at `grid_adequacy::MERIT_ORDER` and
    /// `grid_adequacy::flow::FLOW_MERIT_ORDER`): the single-zone
    /// dispatch path accepts the extended 13-rung Stage 7 set; the
    /// multi-zone engines (run_multi, the LP) accept only the frozen
    /// six-rung flow ladder, pending a signal-convention re-pin.
    #[error(
        "technology {tech:?} has no capacity_factor_trace and no rung on the merit \
         ladder consulted: single-zone dispatch accepts the extended Stage 7 ladder \
         (grid_adequacy MERIT_ORDER — nuclear, biomass, beccs, waste, \
         other_generation, hydro, coal, ccgt_ccs, low_carbon_dispatchable, ccgt, \
         ocgt, oil, hydrogen_turbine); the multi-zone engines accept only the frozen \
         six-rung flow ladder (nuclear, biomass, hydro, coal, ccgt, ocgt) pending a \
         signal-convention re-pin (grid_adequacy flow.rs)"
    )]
    UnknownThermalTechnology {
        /// The unrecognised technology id.
        tech: String,
    },

    /// A scenario/run feature the current stage does not implement.
    #[error("{feature} is not supported yet (see docs/04-implementation-plan.md)")]
    UnsupportedFeature {
        /// The unsupported feature, with the stage that will implement it.
        feature: String,
    },

    /// A scenario is semantically invalid in a way strict parsing cannot
    /// express (out-of-range storage parameters, DSR-only fields on a
    /// non-DSR store, availability on a weather-driven technology, …).
    #[error("invalid scenario: {reason}")]
    InvalidScenario {
        /// Why the scenario was rejected.
        reason: String,
    },

    /// Two stores in one zone share a `dispatch_order` (D4 rule 2:
    /// dispatch_order values must be unique within a zone — the
    /// rule-based policy charges and discharges in this order and a tie
    /// would be an undefined preference).
    #[error(
        "zone {zone}: storage dispatch_order {order} is used by more than one store \
         (dispatch_order must be unique within a zone; D4 rule 2, \
         docs/notes/d4-rule-based-dispatch.md)"
    )]
    DuplicateDispatchOrder {
        /// The zone with the clash.
        zone: String,
        /// The duplicated order value.
        order: u8,
    },

    /// A zone's fleet declares the same technology more than once
    /// (b6 engine-review note 6, characterised 2026-07-06). The engines
    /// key per-technology inputs — CF traces, availability models,
    /// energy budgets, SRMC recipes — by `TechId` in maps (LAST entry
    /// wins), while dispatch builds one unit PER fleet entry (both
    /// dispatch) and result readouts take the FIRST series of a given
    /// id: a duplicate silently corrupts the run, so it is rejected.
    #[error(
        "zone {zone}: fleet technology {technology} is declared more than once — fleet \
         TechIds must be unique within a zone (per-technology inputs are keyed by \
         TechId, so a duplicate entry silently mixes last-wins inputs with \
         both-entries dispatch; merge the entries' capacity into one instead)"
    )]
    DuplicateFleetTechnology {
        /// The zone with the clash.
        zone: String,
        /// The duplicated technology id.
        technology: String,
    },

    /// Run inputs are semantically invalid (bad availability range,
    /// wrong month count, references outside the fleet, misaligned
    /// traces, …).
    #[error("invalid run inputs: {reason}")]
    InvalidRunInputs {
        /// Why the inputs were rejected.
        reason: String,
    },

    /// A storage dispatch policy returned a physically infeasible
    /// decision (charging beyond the surplus or a rating, discharging
    /// beyond the post-stack deficit, SoC out of bounds, wrong action
    /// count). The engine validates every decision rather than trusting
    /// the policy (ADR-6: policies are pluggable).
    #[error("invalid dispatch decision: {reason}")]
    InvalidDispatchDecision {
        /// Why the decision was rejected.
        reason: String,
    },

    /// An analysis utility (residual load, timescale decomposition —
    /// `grid_core::analysis`) received invalid inputs: misaligned
    /// series, an empty series, or unusable decomposition windows.
    #[error("invalid analysis input: {reason}")]
    InvalidAnalysisInput {
        /// Why the input was rejected.
        reason: String,
    },

    /// A solver could not reach its target within its search bounds
    /// (e.g. `min_storage_for_zero_unserved` still has unserved energy
    /// at the maximum store size). Maps to CLI exit code 1 (docs/06).
    #[error("solver infeasible: {reason}")]
    SolveInfeasible {
        /// Why the solve failed.
        reason: String,
    },

    /// A sanity invariant that must hold by construction was measured
    /// violated (e.g. the LP storage requirement exceeding the
    /// rule-based requirement beyond the bisection slack — docs/04
    /// Stage 7). An invariant violation is an engine defect surfaced
    /// loudly, never a reportable finding.
    #[error("sanity invariant violated: {reason}")]
    SanityInvariantViolated {
        /// Which invariant, the measured values and the slack.
        reason: String,
    },

    /// A store's round-trip efficiency sits strictly below the safe
    /// floor the perfect-foresight LP requires (η ≥ 1e-3 — the floor
    /// value included — is the accepted region, far below any real
    /// store). The LP's cycling-penalty tie-break stays sound only for
    /// η well above 1e-6, so a below-floor pack is rejected before the
    /// LP is built (D12 package 2b robustness guard).
    #[error(
        "zone {zone}, store {store}: round_trip_efficiency {efficiency} is below the \
         perfect-foresight LP floor {floor} — the LP's cycling-penalty tie-break needs a \
         round-trip efficiency well above 1e-6 to stay sound"
    )]
    StorageEfficiencyBelowFloor {
        /// The zone holding the store.
        zone: String,
        /// The store's output label.
        store: String,
        /// The offending round-trip efficiency.
        efficiency: f64,
        /// The safe floor.
        floor: f64,
    },

    /// The perfect-foresight LP would build a problem larger than the
    /// size guard permits. HiGHS can abort the whole process (an
    /// uncaught C++ `std::length_error`) on an oversized LP, which a
    /// library crate must never do, so the LP is rejected BEFORE it is
    /// built (D12 package 2b robustness guard; the cap is chosen from
    /// the tractability benchmark to allow binding-window slices of a
    /// few years while rejecting the danger zone).
    #[error(
        "the perfect-foresight LP for this scenario ({periods} periods × {zones} zones) would \
         have about {estimated_variables} decision variables, above the {cap}-variable size \
         guard — run a shorter binding-window horizon (HiGHS can abort the process on an \
         oversized LP; the guard keeps each solve inside the tractable range measured in \
         docs/notes/d12-lp-tractability.md)"
    )]
    LpProblemTooLarge {
        /// The horizon length in half-hourly periods.
        periods: usize,
        /// The number of zones.
        zones: usize,
        /// The estimated decision-variable count.
        estimated_variables: u64,
        /// The size-guard cap.
        cap: u64,
    },

    /// A stability event-spec TOML failed to parse; the source error
    /// carries line/column context from the TOML parser.
    #[error("event spec parse error: {source}")]
    EventSpecParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A stability event spec is semantically invalid (non-positive
    /// inertia or timestep, an unordered LFDD stage table, a response
    /// shape missing its parameter, …).
    #[error("invalid event spec: {reason}")]
    InvalidEventSpec {
        /// Why the spec was rejected.
        reason: String,
    },

    /// Wraps any error with the event-spec file it arose in.
    #[error("in event spec file {path}: {source}", path = path.display())]
    InEventSpecFile {
        /// The event-spec file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// A fleet-pathway spec TOML (Stage 6 Q8) failed to parse; the
    /// source error carries line/column context from the TOML parser.
    #[error("pathway spec parse error: {source}")]
    PathwaySpecParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A fleet-pathway spec is semantically invalid (wrong schema
    /// string, no years, years out of order, out-of-range dispatch
    /// fraction, a static response service where the bisection needs
    /// monotone dynamics, …).
    #[error("invalid pathway spec: {reason}")]
    InvalidPathwaySpec {
        /// Why the spec was rejected.
        reason: String,
    },

    /// Wraps any error with the pathway-spec file it arose in.
    #[error("in pathway spec file {path}: {source}", path = path.display())]
    InPathwaySpecFile {
        /// The pathway-spec file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// An inertia aggregation received a dispatch result that does not
    /// match the scenario it was asked to interpret (unknown technology
    /// series, out-of-range period index).
    #[error("invalid stability input: {reason}")]
    InvalidStabilityInput {
        /// Why the input was rejected.
        reason: String,
    },

    /// A pricing computation received invalid inputs (misaligned series,
    /// out-of-range efficiency, an empty auction table, or an unserved
    /// period with no SRMC-bearing technology to bound its price).
    #[error("invalid pricing inputs: {reason}")]
    InvalidPricing {
        /// Why the pricing inputs were rejected.
        reason: String,
    },

    /// A prices-reference TOML file failed to parse; the source error
    /// carries line/column context from the TOML parser.
    #[error("prices-reference parse error: {source}")]
    PricesReferenceParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A prices-reference file is semantically invalid (wrong schema
    /// string, out-of-range efficiency, malformed auction date, …).
    #[error("invalid prices reference: {reason}")]
    InvalidPricesReference {
        /// Why the reference file was rejected.
        reason: String,
    },

    /// Wraps any error with the prices-reference file it arose in.
    #[error("in prices-reference file {path}: {source}", path = path.display())]
    InPricesReferenceFile {
        /// The prices-reference file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// A costs-reference TOML file (Stage 7, `costs-reference-v1`)
    /// failed to parse; the source error carries line/column context
    /// from the TOML parser (unknown fields are rejected here too).
    #[error("costs-reference parse error: {source}")]
    CostsReferenceParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A costs-reference file is semantically invalid (wrong schema
    /// string, unordered capex bracket or WACC set, a phasing array not
    /// summing to ~1, a negative cost, …).
    #[error("invalid costs reference: {reason}")]
    InvalidCostsReference {
        /// Why the reference file was rejected.
        reason: String,
    },

    /// Wraps any error with the costs-reference file it arose in.
    #[error("in costs-reference file {path}: {source}", path = path.display())]
    InCostsReferenceFile {
        /// The costs-reference file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// A cost computation (rule-4 annualisation, the rule-1 cost stack)
    /// received invalid inputs: a degenerate WACC or asset life, an
    /// unknown cost row or holding service, a quarantined row with no
    /// consumable figure, a zero delivered-energy denominator, …
    #[error("invalid cost inputs: {reason}")]
    InvalidCostInputs {
        /// Why the inputs were rejected.
        reason: String,
    },

    /// A published-pathway reference TOML file (Stage 7,
    /// `pathways-published-v1`) failed to parse; the source error
    /// carries line/column context from the TOML parser (unknown
    /// fields are rejected here too).
    #[error("pathways-published parse error: {source}")]
    PathwaysReferenceParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// A published-pathway reference file is semantically invalid
    /// (wrong schema string, a `mappable = true` aggregate, an
    /// aggregate colliding with a fleet technology, an out-of-step
    /// surplus-electrolysis pair, a year set not matching the snapshot
    /// years, …).
    #[error("invalid pathways-published reference: {reason}")]
    InvalidPathwaysReference {
        /// Why the reference file was rejected.
        reason: String,
    },

    /// Wraps any error with the pathways-published file it arose in.
    #[error("in pathways-published file {path}: {source}", path = path.display())]
    InPathwaysReferenceFile {
        /// The pathways-published file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// A heating portfolio's technology shares do not sum to 1 within
    /// the D9 rule-2 tolerance
    /// (`grid_core::scenario::HEATING_SHARE_SUM_TOLERANCE` = 1e-9). The
    /// structured error names the sum and the entries (D9 rule 2).
    #[error(
        "zone {zone}: heating portfolio shares sum to {sum}, not 1 (tolerance 1e-9; \
         D9 rule 2) — entries: {entries}"
    )]
    HeatingShareSum {
        /// The zone with the bad portfolio.
        zone: String,
        /// The measured share sum.
        sum: f64,
        /// The entries, `kind = share` comma-separated.
        entries: String,
    },

    /// The heating-COP reference TOML (`data/reference/heating-cop.toml`,
    /// D9 rule 4) failed to parse; the source error carries line/column
    /// context from the TOML parser (unknown fields are rejected here
    /// too — the drift-guarded reference-file discipline).
    #[error("heating-COP reference parse error: {source}")]
    HeatingReferenceParse {
        /// The underlying TOML error (boxed: it is large).
        #[source]
        source: Box<toml::de::Error>,
    },

    /// The heating-COP reference file is semantically invalid
    /// (out-of-range factor, non-physical ground-model parameter, a
    /// mislabelled source, an unordered band, …).
    #[error("invalid heating-COP reference: {reason}")]
    InvalidHeatingReference {
        /// Why the reference file was rejected.
        reason: String,
    },

    /// Wraps any error with the heating-COP reference file it arose in.
    #[error("in heating-COP reference file {path}: {source}", path = path.display())]
    InHeatingReferenceFile {
        /// The reference file being read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: Box<GridError>,
    },

    /// The heating overlay computation received invalid inputs (a
    /// temperature trace not covering whole calendar years, a horizon
    /// outside the trace record, a degenerate degree-hour record, …).
    #[error("invalid heating overlay inputs: {reason}")]
    InvalidHeatingOverlay {
        /// Why the overlay inputs were rejected.
        reason: String,
    },

    /// A cost result is stamped non-quotable — it consumed quarantined
    /// reference rows or carries an unmet publication gate — and the
    /// publish path refuses to emit it as a publishable artefact
    /// (docs/04 Stage 7 quarantine rule: flags propagate, the artefact
    /// layer refuses).
    #[error(
        "result is not publishable: {reason} (Stage 7 quarantine rule: quotable = false \
         reference rows and unmet publication gates propagate to result metadata, and the \
         artefact layer refuses to publish a flagged result)"
    )]
    NonQuotableResult {
        /// The quarantined rows consumed and/or gates unmet.
        reason: String,
    },
}
