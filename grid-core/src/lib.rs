//! # grid-core — shared domain model
//!
//! Responsibilities per ADR-1 (`docs/02-architecture.md`): the scenario
//! schema, domain types, the newtype unit system (ADR-4), weather/demand
//! trace loading (ADR-3: UTC half-hourly settlement periods), the
//! pricing and emissions layers (ADR-9, Stage 2: [`pricing`] and
//! [`prices_reference`]), the residual-load utilities and timescale
//! decomposition (Stage 4: [`analysis`]), the Q5 electrified-heating
//! overlay — the heating-COP reference parser and the D9 rule-3/4
//! demand computation ([`heating`]) — the stability metadata —
//! per-technology inertia constants and synchronous flags with their
//! MW→MVA convention (Stage 6: [`inertia`]) — and the Stage 7 cost
//! layer: the costs-reference parser ([`costs_reference`]), the D8
//! rule-4 annualisation/WACC-banding arithmetic ([`costs`]) and the
//! published-pathway reference parser ([`pathways_published`]).
//!
//! Library conventions (docs/06): no panics — every fallible public API
//! returns [`Result`]&lt;T, [`GridError`]&gt;; no `unsafe`; no globals,
//! wall-clock reads, or environment-dependent behaviour (ADR-5); raw `f64`
//! for a physical quantity never crosses a public API boundary (ADR-4).

pub mod analysis;
pub mod costs;
pub mod costs_reference;
pub mod error;
pub mod heating;
pub mod inertia;
pub mod pathways_published;
pub mod prices_reference;
pub mod pricing;
pub mod scenario;
pub mod time;
pub mod trace;
pub mod units;

pub use error::GridError;
