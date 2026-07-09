//! Import-convention bracketing for capacity sweeps (Package B,
//! ratified 2026-07-03) — tier 1 of the frozen-imports-under-sweep
//! deviation (memory/project-state.md): three EXOGENOUS-TRACE
//! transformations that bracket what interconnector behaviour could do
//! at swept capacities far from the 2024 anchor, without any change to
//! the dispatch engine. The endogenous alternative (pricing on the
//! multi-zone engine) is tier 2, at Q10/Q2 revision time.
//!
//! # The three conventions, in prose (D4 precedent: the contestable
//! # rules live in the code)
//!
//! - **FROZEN** — the identity. The imports-flagged exogenous series
//!   stay at their observed values at every swept capacity (the
//!   pre-Package-B behaviour, and the sweep default). Physically wrong
//!   away from the anchor in both directions; kept as the continuity
//!   convention.
//! - **ZERO-IN-SURPLUS** — every imports-flagged series is set to 0 in
//!   *pre-import surplus periods* (defined below), unchanged
//!   elsewhere. This models a counterparty that neither supplies GB
//!   nor absorbs GB surplus when GB is already oversupplied. Note the
//!   documented edge: in surplus periods where the frozen trace
//!   already *exports* (negative values — observed 2024 behaviour in
//!   windy periods), the value is SET TO 0, not clamped — the
//!   convention is "no exchange in surplus", not "no imports in
//!   surplus".
//! - **EXPORT-IN-SURPLUS** — the aggregate imports-flagged supply is
//!   set to `−min(export_capacity, surplus magnitude)` in pre-import
//!   surplus periods, unchanged elsewhere. The `min()` is deliberate
//!   and load-bearing: GB exports **its own pre-import surplus** up to
//!   interconnector capability; exports must never force thermal
//!   dispatch to serve foreign demand (which lies outside this
//!   single-zone model). With the cap, the post-import balance in a
//!   surplus period stays ≥ 0, so the transformation can never create
//!   a deficit the engine would meet with gas.
//!
//! # Pre-import surplus: the pinned definition
//!
//! Derived from the dispatch engine's own must-take convention
//! (`crate::dispatch`, rule 1: `must_take(t) = Σ renewables
//! capacity × cf(t) + Σ exogenous(t)`, surplus iff strictly positive
//! excess over demand):
//!
//! ```text
//! domestic(t) = Σ_fleet-entries-with-CF-trace  capacity_gw × cf(t)
//!             + Σ_exogenous with imports=false  trace(t)
//! s(t)        = domestic(t) − demand(t)
//! surplus period ⇔ s(t) > 0 (strict, matching dispatch's `net > 0`)
//! ```
//!
//! `domestic` is the must-take sum MINUS the imports-flagged series —
//! i.e. what GB supplies before any interconnector exchange, at the
//! swept capacity (the caller passes the swept scenario, so the mask
//! moves with the swept wind capacity, as it must). Boundary periods
//! (`s(t) = 0` exactly) are NOT surplus periods: the export target
//! would be −min(cap, 0) = 0 anyway, but ZERO-IN-SURPLUS would zero a
//! nonzero import there, so the strict inequality is the pinned
//! choice.
//!
//! **Storage subtlety, stated out loud:** the mask is **pre-storage by
//! construction** — it is a trace-level computation made before
//! dispatch runs, so storage charging (which absorbs post-import
//! surplus inside the engine, D4 rule 2) does not and cannot shrink
//! the mask. A store that would have absorbed the surplus still sees
//! it; only the import side changes.
//!
//! # Aggregation over several imports-flagged series
//!
//! The conventions are defined on the AGGREGATE net-import supply.
//! When a scenario flags several series (none in the repo today — the
//! reference scenario carries exactly one), the aggregate target lands
//! on the first flagged series and the others are zeroed in surplus
//! periods; all are unchanged elsewhere. Documented choice, not
//! physics: the aggregate is what dispatch sees.
//!
//! # Export capability from the scenario's links
//!
//! [`link_export_capability`] sums `capacity_gw × availability` over
//! every `[[links]]` entry touching the scenario's single zone (either
//! endpoint) — the same derating the Stage 5 flow rule applies every
//! period. For the 2024 reference scenario: 9.8 GW nameplate ex-
//! Greenlink (availability 0 for the 2024 validation year) × 0.95 =
//! **9.31 GW**. `None` when the scenario declares no links touching
//! the zone: the caller (CLI) must then require an explicit parameter
//! — no silent default.

use grid_core::GridError;
use grid_core::scenario::Scenario;
use grid_core::trace::Trace;
use grid_core::units::Power;

use crate::inputs::{ExogenousSupply, RunInputs, single_zone};

/// One of the three bracketing import conventions (module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportConvention {
    /// Identity: imports stay at their observed trace values.
    Frozen,
    /// Imports set to zero in pre-import surplus periods.
    ZeroInSurplus,
    /// Imports set to `−min(export_capacity, surplus magnitude)` in
    /// pre-import surplus periods.
    ExportInSurplus {
        /// Total export capability (GW) capping the surplus export.
        export_capacity: Power,
    },
}

impl ImportConvention {
    /// Output/label token for CSV columns and chart legends.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Frozen => "frozen",
            Self::ZeroInSurplus => "zero-in-surplus",
            Self::ExportInSurplus { .. } => "export-in-surplus",
        }
    }
}

/// Apply an import convention to loaded run inputs, returning the
/// transformed inputs (the originals are untouched; FROZEN returns a
/// bit-identical clone). The scenario provides the fleet capacities the
/// mask needs — pass the SWEPT scenario variant so the mask moves with
/// the swept capacity. Full definitions in the module docs.
///
/// Errors with [`GridError::InvalidRunInputs`] when a weather-driven
/// fleet entry has no loaded CF trace or a trace disagrees with the
/// demand trace on period count (the same alignment dispatch enforces).
pub fn apply_import_convention(
    scenario: &Scenario,
    inputs: &RunInputs,
    convention: &ImportConvention,
) -> Result<RunInputs, GridError> {
    if *convention == ImportConvention::Frozen {
        return Ok(inputs.clone());
    }
    let zone = single_zone(scenario)?;
    let periods = inputs.demand.len();
    let zero = Power::gigawatts(0.0);

    // domestic(t) = weather-driven potential at the swept capacities
    // + non-import exogenous must-take (module docs).
    let mut domestic = vec![zero; periods];
    for entry in &zone.fleet {
        if entry.capacity_factor_trace.is_none() {
            continue;
        }
        let cf = inputs
            .capacity_factors
            .get(&entry.technology)
            .ok_or_else(|| GridError::InvalidRunInputs {
                reason: format!(
                    "import convention: no capacity-factor trace loaded for weather-driven \
                     technology {}",
                    entry.technology
                ),
            })?;
        if cf.len() != periods {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "import convention: CF trace for {} has {} periods; demand has {periods}",
                    entry.technology,
                    cf.len()
                ),
            });
        }
        for (acc, &factor) in domestic.iter_mut().zip(cf.values()) {
            *acc = *acc + entry.capacity_gw * factor;
        }
    }
    for supply in &inputs.exogenous {
        if supply.imports {
            continue;
        }
        if supply.trace.len() != periods {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "import convention: exogenous series {:?} has {} periods; demand has \
                     {periods}",
                    supply.label,
                    supply.trace.len()
                ),
            });
        }
        for (acc, &p) in domestic.iter_mut().zip(supply.trace.values()) {
            *acc = *acc + p;
        }
    }

    // Frozen returned early above, so the variant is one of the two
    // transformations: `None` = zero-in-surplus, `Some(cap)` =
    // export-in-surplus with that capability.
    let export_cap = match convention {
        ImportConvention::Frozen => None, // unreachable: early return above
        ImportConvention::ZeroInSurplus => None,
        ImportConvention::ExportInSurplus { export_capacity } => Some(*export_capacity),
    };

    // Per-period aggregate import target in surplus periods (None =
    // period not in the mask, keep the frozen value). Strict mask:
    // s(t) > 0 (module docs).
    let target = |t: usize| -> Option<Power> {
        let surplus = domestic[t] - inputs.demand.values()[t];
        if surplus <= zero {
            return None;
        }
        Some(match export_cap {
            None => zero,
            Some(capability) => {
                // The deliberate min(): export GB's own surplus only,
                // up to capability — never force thermal dispatch.
                if surplus < capability {
                    -surplus
                } else {
                    -capability
                }
            }
        })
    };

    // Rebuild the imports-flagged series: the aggregate target lands on
    // the FIRST flagged series, later flagged series are zeroed in mask
    // periods (module docs); non-import series are untouched.
    let mut first_import_seen = false;
    let mut exogenous = Vec::with_capacity(inputs.exogenous.len());
    for supply in &inputs.exogenous {
        if !supply.imports {
            exogenous.push(supply.clone());
            continue;
        }
        if supply.trace.len() != periods {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "import convention: imports series {:?} has {} periods; demand has \
                     {periods}",
                    supply.label,
                    supply.trace.len()
                ),
            });
        }
        let carries_aggregate = !first_import_seen;
        first_import_seen = true;
        let values: Vec<Power> = supply
            .trace
            .values()
            .iter()
            .enumerate()
            .map(|(t, &frozen)| match target(t) {
                None => frozen,
                Some(aggregate) => {
                    if carries_aggregate {
                        aggregate
                    } else {
                        zero
                    }
                }
            })
            .collect();
        exogenous.push(ExogenousSupply {
            label: supply.label.clone(),
            imports: supply.imports,
            reliability: supply.reliability,
            trace: Trace::from_parts(supply.trace.start(), values)?,
        });
    }

    Ok(RunInputs {
        demand: inputs.demand.clone(),
        capacity_factors: inputs.capacity_factors.clone(),
        exogenous,
        availability: inputs.availability.clone(),
        // The overlay rides along unchanged: the convention rewrites
        // exogenous imports only, never demand (heating included).
        heating: inputs.heating.clone(),
    })
}

/// Total link export capability of the scenario's single zone:
/// `Σ capacity_gw × availability` over links with either endpoint in
/// the zone (module docs). `Ok(None)` when no link touches the zone —
/// the caller must then require an explicit capability (no silent
/// default).
pub fn link_export_capability(scenario: &Scenario) -> Result<Option<Power>, GridError> {
    let zone = single_zone(scenario)?;
    let mut total = Power::gigawatts(0.0);
    let mut any = false;
    for link in &scenario.links {
        if link.from != zone.id && link.to != zone.id {
            continue;
        }
        any = true;
        total = total + link.capacity_gw * link.availability;
    }
    Ok(any.then_some(total))
}
