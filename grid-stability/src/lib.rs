//! # grid-stability — system stability engine
//!
//! Responsibilities per ADR-1 and ADR-2 (`docs/02-architecture.md`):
//! aggregate inertia from dispatched synchronous plant at any adequacy
//! timestep ([`inertia`]: `Σ(H × MVA)` under a documented MW→MVA
//! convention), single-bus swing-equation loss-of-infeed event
//! simulation ([`swing`]: RoCoF, frequency nadir, first arrest, LFDD
//! staged demand blocks, era-limit reporting), and frequency-response
//! services with volumes, delays and ramp shapes ([`spec`]).
//! Explicitly *not* EMT or multi-bus. Consumes `grid-adequacy` dispatch
//! output via the shared domain model in `grid-core` (ADR-9: H and
//! `synchronous` per technology live there, schema v3).
//!
//! Era-dependent operating limits (RoCoF relay settings, LFDD stage
//! table) are **event-spec inputs**, never constants (docs/04 Stage 6).
//!
//! Stage 6 part 2 adds the Q8 fleet-pathway runner ([`pathway`]):
//! largest survivable loss-of-infeed vs year under a fleet pathway,
//! with era response/damping assumptions as spec inputs (2019 values
//! as the cited, flagged default) and dispatch conditions reported as
//! a band, not a line.
//!
//! Library conventions (docs/06): no panics — every fallible public API
//! returns [`Result`]&lt;T, `GridError`&gt;; no `unsafe`; no globals,
//! wall-clock reads, or unseeded randomness (ADR-5); raw `f64` for a
//! physical quantity never crosses a public API boundary (ADR-4).

pub mod inertia;
pub mod pathway;
pub mod reference;
pub mod spec;
pub mod swing;

pub use inertia::{
    InertiaTable, has_synchronous_provision, inertia_series, min_inertia, periods_below,
    system_inertia,
};
pub use pathway::{
    DispatchCondition, PATHWAY_SCHEMA, PathwayAssumptions, PathwayFleetEntry, PathwayPoint,
    PathwaySpec, PathwayStorageEntry, PathwayYear, ReferenceLoss, SurvivablePoint,
    largest_survivable_loss, pathway_year_inertia, run_pathway,
};
pub use reference::{Fit, correlate, inertia_from_generation};
pub use spec::{
    EVENT_SCHEMA, EventSpec, InfeedLoss, LfddScheme, LfddStage, OperatingLimits, ResponseService,
    ResponseShape, RocofWindow,
};
pub use swing::{EventResult, LfddAction, LimitReport, ServiceTimeline, simulate};
