//! # grid-adequacy — energy adequacy engine
//!
//! Responsibilities per ADR-1 and ADR-2 (`docs/02-architecture.md`):
//! chronological half-hourly merit-order dispatch over arbitrary horizons
//! (up to the full 40-year weather record), storage dispatch policies
//! (pluggable per ADR-6: `RuleBased` is the default; the
//! perfect-foresight LP is deliberately NOT a policy but the
//! whole-horizon function [`run_multi_lp`] — D12, [`lp`] module docs),
//! solvers (bisection for monotone 1-D problems per ADR-10), and the
//! parameter sweep runner.
//!
//! Stage 1 implemented single-zone merit-order dispatch
//! (`docs/04-implementation-plan.md` Stage 1): weather-driven renewables
//! as must-take, a fixed thermal merit order (see [`dispatch`] for the
//! rules in prose), flat and monthly availability models, exogenous
//! imports, and unserved-energy / curtailment accounting. Links stay
//! inert until Stage 5; multi-zone scenarios are rejected (ADR-7).
//!
//! Stage 2 added the pricing plumbing ([`pricing`]): loading the
//! scenario's `[pricing]` block and pricing a completed dispatch run
//! through the `grid_core::pricing` layer (ADR-9). Pricing reads the
//! dispatch output and can never perturb it.
//!
//! Stage 3 activates the storage portfolio ([`policy`]: the D4
//! rule-based dispatch rules in full prose, per docs/06) with √η SoC
//! accounting, multi-year continuous horizons (SoC carries across
//! years), and the `min_storage_for_zero_unserved` bisection solver
//! ([`solve`]). DSR stores are schema shape only until Q6 and are
//! rejected at run time.
//!
//! Stage 4 adds the parameter sweep runner ([`sweep`]: rayon-parallel,
//! bit-identical to serial, full response surfaces kept per ADR-10),
//! the Q4 per-year batch mode, and the storage attribution by
//! timescale band ([`attribution`] — the Module 3(c) machinery, with
//! its kill-criterion-2 posture documented in the module).
//!
//! The Q5/Q11 analysis runs (D9 rules 6/6b) add the heating-mix
//! simplex runner ([`heating_mix`]): the ASHP/GSHP/district portfolio
//! sweep with per-point bisection solves at a stated store rating, and
//! the timescale decomposition of the electrified-heat addition on the
//! Stage 4 attribution machinery.
//!
//! Stage 7 (package 1) adds the D8 rule-1 cost stack ([`costs`]): the
//! six pinned component lines over a completed run, WACC-banded,
//! reconciling exactly (rule 2), with the quarantine
//! propagate-then-refuse metadata of the docs/04 Stage 7 pin. Like
//! pricing, costing reads the dispatch output and can never perturb
//! it.
//!
//! The Stage 7 Q9/bridges package adds the D8 rule-6a three-wedge
//! decomposition ([`q9_decomposition`]) of the gap between plant-gate
//! LCOE and the delivered system cost — an exact accounting identity,
//! never an attribution — and the per-scenario rule-based-vs-LP
//! storage gap report ([`storage_gap_report`]): both requirements
//! measured by the same bisection, the D12 rule-4 sanity invariant
//! (LP ≤ rule-based) asserted structurally, the gap a reported finding
//! (ADR-6).
//!
//! Library conventions (docs/06): no panics — every fallible public API
//! returns [`Result`]&lt;T, `GridError`&gt;; no `unsafe`; no globals,
//! wall-clock reads, or environment-dependent behaviour (ADR-5); raw
//! `f64` for a physical quantity never crosses a public API boundary
//! (ADR-4).

pub mod attribution;
pub mod availability;
pub mod costs;
pub mod dispatch;
pub mod flow;
pub mod heating_mix;
pub mod import_convention;
pub mod inputs;
pub mod lp;
pub mod multizone;
pub mod policy;
pub mod pricing;
pub mod result;
pub mod solve;
pub mod sweep;

pub use attribution::{Band, BandAttribution, StorageAttribution, attribute_storage_by_band};
pub use availability::AvailabilityModel;
pub use costs::{
    ConstraintCostsLine, CostFraming, CostMetadata, CostStack, CostStackSpec, CostedBattery,
    CostedCoverage, CostedGeneration, CostedLink, Q9Decomposition, ReliabilityStamp,
    ServiceHolding, StoreVintage, TechPlantGate, cost_stack, delivered_to_demand_energy,
    horizon_years, q9_decomposition,
};
pub use dispatch::{MERIT_ORDER, run, run_with_policy};
pub use heating_mix::{
    HeatingMixContext, HeatingMixPoint, HeatingMixSweep, MixMetrics, MixOutcome, MixShares,
    NamedAttribution, simplex_mixes,
};
pub use import_convention::{ImportConvention, apply_import_convention, link_export_capability};
pub use inputs::{
    BudgetSchedule, CF_COLUMN, ExogenousSupply, LinkCapability, MultiZoneInputs, RunInputs,
    ZoneInputs, ZonePricingInputs, build_link_capability, load_multi_zone_inputs, load_run_inputs,
};
pub use lp::{
    LP_RTE_FLOOR, LP_VARIABLE_CAP, LpObjective, estimate_lp_variables, run_multi_lp,
    run_multi_lp_min_curtailment, run_multi_lp_rolling,
};
pub use multizone::{
    LinkCapabilitySeries, LinkFlowSeries, MultiZoneRunResult, ZoneRunResult, run_multi,
    run_multi_with_policy,
};
pub use policy::{
    DispatchDecision, DispatchPolicy, PolicyContract, RuleBased, StoreAction, StoreState,
    SystemState,
};
pub use pricing::{
    PricingInputs, PricingResult, RealismStats, TechEmissions, TechPricing,
    delivered_renewable_power, load_pricing_inputs, price_run,
};
pub use result::{
    FIRM_SHARE_ALARM_THRESHOLD, FirmShareStats, LabelledSeries, RunResult, StoreSeries, TechSeries,
};
pub use solve::{
    BisectionIterate, BisectionOutcome, SolveOptions, SolveResult, StorageGapReport,
    check_storage_gap_invariant, min_storage_for_zero_unserved, min_storage_for_zero_unserved_lp,
    storage_gap_report,
};
pub use sweep::{
    Dimension, DimensionSpec, Execution, MultiZoneGroupWindSweep, MultiZoneWindPoint,
    MultiZoneWindSweep, SweepPoint, SweepResult, SweepSpec, YearOutcome, YearRequirement,
    per_year_requirements, run_sweep, wind_capacity_sweep_multi, wind_capacity_sweep_multi_group,
};
