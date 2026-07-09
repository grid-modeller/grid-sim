//! Aggregate system inertia from dispatched synchronous plant (docs/04
//! Stage 6; ADR-2: the stability engine's first input is the fleet *as
//! dispatched* at a chosen adequacy timestep).
//!
//! ## The aggregation convention (documented in prose, load-bearing)
//!
//! System inertia at a timestep is **E = Σ Hᵢ × MVAᵢ over synchronised
//! plant**. This model aggregates plant by technology, not by unit, so
//! the synchronised MVA of a technology tranche must be inferred from
//! its dispatch:
//!
//! 1. **Fleet technologies**: a technology contributes while its
//!    dispatched output is strictly positive, with synchronised
//!    apparent power = dispatched GW ÷ power factor
//!    ([`grid_core::inertia::DEFAULT_POWER_FACTOR`] = 0.9, the
//!    documented MVA convention). Rationale: in a merit-order world,
//!    committed units run at high load factor, so dispatched MW tracks
//!    synchronised MVA up to the power factor; NESO itself estimates
//!    system inertia from a dispatch/demand model, not a unit register
//!    (ESO 9-Aug-2019 appendices, Appendix M Q42). Known bias, stated:
//!    part-loaded units keep full rotating mass synchronised, so this
//!    convention *understates* inertia when plant runs part-loaded —
//!    conservative for adequacy-of-inertia questions.
//! 2. **Storage**: a store contributes while it is RUNNING in either
//!    direction — pumping is demand but the machine is synchronised —
//!    with apparent power = (charge GW + discharge GW) ÷ power factor.
//!    Whether a kind is synchronous comes from
//!    [`grid_core::inertia::storage_kind_default`] (pumped hydro yes;
//!    battery/hydrogen/DSR no — the hydrogen v1 choice is documented
//!    there).
//! 3. **Exogenous supply traces carry NO inertia** — they are bare MW
//!    series with no machine metadata. On the 2024 reference scenario
//!    this omits the observed pumped-storage trace's running hours
//!    (~1–3 GVA·s while running): a known, conservative gap, noted in
//!    the scenario header.
//! 4. **Weather-driven output is used as produced** (pre-curtailment
//!    potential, the [`RunResult`] convention). Immaterial today: every
//!    weather-driven technology is non-synchronous unless a scenario
//!    explicitly overrides it, in which case the author owns the
//!    convention.
//!
//! The dispatch is NOT constrained by any inertia floor — the adequacy
//! engine has no inertia product (real GB dispatch does, via NESO
//! stability actions). Module 6's finding is exactly the gap between
//! this unconstrained series and the operational floors.

use std::collections::BTreeMap;

use grid_adequacy::RunResult;
use grid_core::GridError;
use grid_core::inertia::{DEFAULT_POWER_FACTOR, storage_kind_default};
use grid_core::scenario::Scenario;
use grid_core::units::{Inertia, InertiaConstant};

/// The effective per-technology stability metadata of one scenario:
/// the lookup table the aggregation uses. Built once per scenario;
/// technologies map to `Some(H)` (synchronous) or `None`
/// (non-synchronous, contributes nothing).
#[derive(Debug, Clone, PartialEq)]
pub struct InertiaTable {
    /// Technology id → effective H; `None` = effectively
    /// non-synchronous.
    contributions: BTreeMap<String, Option<InertiaConstant>>,
}

impl InertiaTable {
    /// Build the lookup table from a scenario's effective fleet
    /// metadata (explicit overrides where given, derived defaults
    /// otherwise — `grid_core::inertia`). Runs the scenario's semantic
    /// validation first, so incoherent metadata cannot reach the sum.
    ///
    /// All zones are read (the table is keyed by technology id);
    /// duplicate technology ids with *different* effective metadata are
    /// rejected — the dispatch result labels series by technology id
    /// only, so the mapping must be unambiguous.
    pub fn from_scenario(scenario: &Scenario) -> Result<Self, GridError> {
        scenario.validate()?;
        let mut contributions: BTreeMap<String, Option<InertiaConstant>> = BTreeMap::new();
        for zone in &scenario.zones {
            for entry in &zone.fleet {
                let effective = if entry.effective_synchronous() {
                    // Scenario::validate guarantees synchronous entries
                    // an effective H; re-checked here rather than
                    // defaulted, so a broken guarantee is loud.
                    match entry.effective_inertia_h() {
                        Some(h) => Some(h),
                        None => {
                            return Err(GridError::InvalidStabilityInput {
                                reason: format!(
                                    "technology {}: synchronous with no effective inertia_h \
                                     (scenario validation should have rejected this)",
                                    entry.technology
                                ),
                            });
                        }
                    }
                } else {
                    None
                };
                match contributions.get(entry.technology.as_str()) {
                    Some(existing) if *existing != effective => {
                        return Err(GridError::InvalidStabilityInput {
                            reason: format!(
                                "technology {} appears more than once with different effective \
                                 inertia metadata — the dispatch result labels series by \
                                 technology id, so the mapping must be unambiguous",
                                entry.technology
                            ),
                        });
                    }
                    _ => {
                        contributions.insert(entry.technology.as_str().to_owned(), effective);
                    }
                }
            }
        }
        Ok(Self { contributions })
    }

    /// The effective H of a technology id, or an error if the scenario
    /// never declared it (a scenario/result mismatch, not a quiet zero).
    fn h_of(&self, tech: &str) -> Result<Option<InertiaConstant>, GridError> {
        self.contributions
            .get(tech)
            .copied()
            .ok_or_else(|| GridError::InvalidStabilityInput {
                reason: format!(
                    "dispatch result carries technology {tech:?} which the scenario's fleet \
                     does not declare — the result and scenario do not belong together"
                ),
            })
    }
}

/// System inertia at one settlement period of a dispatch run:
/// Σ(H × MVA) over dispatched synchronous plant, under the module-level
/// convention.
pub fn system_inertia(
    result: &RunResult,
    table: &InertiaTable,
    period: usize,
) -> Result<Inertia, GridError> {
    if period >= result.periods() {
        return Err(GridError::InvalidStabilityInput {
            reason: format!(
                "period index {period} is out of range (run has {} periods)",
                result.periods()
            ),
        });
    }
    let pf = DEFAULT_POWER_FACTOR;
    let mut total = Inertia::gigavolt_ampere_seconds(0.0);
    for series in result.renewables.iter().chain(&result.thermal) {
        let Some(h) = table.h_of(series.tech.as_str())? else {
            continue;
        };
        let power = series.power[period];
        if power.as_gigawatts() > 0.0 {
            total = total + h * power.apparent(pf);
        }
    }
    for store in &result.stores {
        let default = storage_kind_default(store.kind);
        let Some(h) = default.h else { continue };
        // Running in either direction synchronises the machine set —
        // pumping hours count (grid_core::inertia PS note).
        let active = store.charge[period] + store.discharge[period];
        if active.as_gigawatts() > 0.0 {
            total = total + h * active.apparent(pf);
        }
    }
    Ok(total)
}

/// The per-period system-inertia series of a whole run.
pub fn inertia_series(result: &RunResult, table: &InertiaTable) -> Result<Vec<Inertia>, GridError> {
    (0..result.periods())
        .map(|t| system_inertia(result, table, t))
        .collect()
}

/// The minimum-inertia period of a series and its index (`None` for an
/// empty series). Ties resolve to the first occurrence — deterministic.
#[must_use]
pub fn min_inertia(series: &[Inertia]) -> Option<(usize, Inertia)> {
    let mut best: Option<(usize, Inertia)> = None;
    for (index, &value) in series.iter().enumerate() {
        if best.is_none_or(|(_, current)| value < current) {
            best = Some((index, value));
        }
    }
    best
}

/// Number of periods strictly below an inertia floor.
#[must_use]
pub fn periods_below(series: &[Inertia], floor: Inertia) -> usize {
    series.iter().filter(|&&value| value < floor).count()
}

/// Whether a scenario has any synchronous provision at all (fleet or
/// storage). `false` is the Royal-Society-scenario finding: an
/// all-variable fleet has zero system inertia at every hour without
/// synthetic provision — the CLI states it in the run output.
#[must_use]
pub fn has_synchronous_provision(scenario: &Scenario) -> bool {
    scenario.zones.iter().any(|zone| {
        zone.fleet.iter().any(|e| e.effective_synchronous())
            || zone
                .storage
                .iter()
                .any(|s| storage_kind_default(s.kind).synchronous)
    })
}
