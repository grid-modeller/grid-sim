//! The Q5/Q11 heating-mix analysis runner (D9 rules 6 and 6b,
//! fixed-fleet leg): hold heat decarbonisation identical, sweep the
//! ASHP/GSHP/district portfolio mix over a simplex grid, and measure
//! what the mix changes — peak residual demand, the 40-year storage
//! requirement (bisection, Stage 3 machinery), per-technology dispatch
//! quantities, and the timescale decomposition of the added requirement
//! (Stage 4 machinery).
//!
//! # What a sweep point computes
//!
//! For each [`MixShares`] point the runner builds the heated scenario
//! (the template heating block with only the entry shares replaced) and
//! evaluates:
//!
//! - **peak residual demand** — max over periods of
//!   `demand − Σ must-take` (`grid_core::analysis::residual_load`, the
//!   engine's own summation order). Storage-independent.
//! - **the storage requirement** — `min_storage_for_zero_unserved`
//!   (bisection) for the designated store, at the caller's STATED
//!   power rating. The rating is applied to BOTH the baseline and every
//!   heated point (the D8-like both-endpoints convention ratified in
//!   docs/notes/q5-heating-engine-review.md), and it is stamped on the
//!   result so no storage number can travel without it.
//!   [`GridError::SolveInfeasible`] at the stated rating is captured as
//!   [`MixOutcome::Infeasible`] — **a reportable result, never a bumped
//!   rating** (the review's binding record item; the committed RS
//!   scenario's 100 GW rating is power-bound infeasible under
//!   electrified heat — the pinned first finding of
//!   `grid-adequacy/tests/heating.rs`).
//! - **dispatch metrics** (per-tech potential energy, pooled
//!   curtailment, store charge/discharge, unserved, heating overlay
//!   totals) — from ONE dispatch run at the scenario's committed store
//!   energy with the stated rating: fixed fleet AND fixed store on both
//!   endpoints of every paired delta (D9 rule 6b, generation-relieved
//!   leg). On an all-must-take fleet the per-tech *potential* is
//!   mix-invariant by construction; the mix moves curtailment, store
//!   cycling and unserved — which is exactly the fixed-fleet
//!   "generation relieved" accounting. The capacity-relieved leg
//!   (equal-reliability avoided build) needs the 1-D capacity solver
//!   (ELCC runner) and is NOT computed here — a named dependency.
//!
//! # Bit-identity with the engine's own input path (load once, prove
//! # equal)
//!
//! Loading the 40-year trace set per point would dominate the sweep, so
//! the baseline inputs are loaded ONCE (`load_run_inputs` on the
//! scenario with the heating block removed) and each point's demand is
//! `baseline_demand + overlay.electrical_total`, with the overlay from
//! `grid_core::heating::compute_overlay` on the once-loaded t2m trace.
//! This is bit-identical to `load_run_inputs` on the heated scenario:
//! the loader computes `(base × scale + extra) + heating` in exactly
//! that association (`crate::inputs`), and the baseline demand IS
//! `(base × scale + extra)`. The identity is not assumed: the
//! acceptance suite (`tests/heating_mix.rs`) requires the D9
//! 0.70/0.20/0.10 point to reproduce the `tests/heating.rs`
//! characterisation pins exactly, and those pins were produced through
//! `load_run_inputs`.
//!
//! # Determinism
//!
//! Each point is a pure function of its scenario variant (ADR-5);
//! rayon execution uses order-preserving `collect` and is bit-identical
//! to serial (the Stage 4 acceptance precedent, re-asserted for this
//! runner in the acceptance suite).
//!
//! # Quoting duties (carried by every artefact built from this module)
//!
//! Any requirement or delta is quoted "at N GW store power, both
//! endpoints", with the 100 GW infeasibility finding travelling with
//! ×1.69-class headlines; the rule-3 no-behavioural-profile caveat
//! makes every portfolio delta a LOWER BOUND; 2024 non-heat demand
//! tiling and the climate-stationary intensity `k` are standing
//! caveats (D9 rule 6).

use std::path::Path;

use rayon::prelude::*;

use grid_core::GridError;
use grid_core::analysis::{DecompositionWindows, residual_load};
use grid_core::heating::{HEATING_COP_REFERENCE_PATH, HeatingCopReference, compute_overlay};
use grid_core::scenario::{HeatingKind, HeatingSpec, Scenario, TechId};
use grid_core::time::UtcInstant;
use grid_core::trace::{Trace, load_temperature_trace_c};
use grid_core::units::{Energy, PerUnit, Power, Temperature};

use crate::attribution::{StorageAttribution, attribute_storage_by_band};
use crate::dispatch::run;
use crate::inputs::{RunInputs, load_run_inputs, single_zone};
use crate::result::RunResult;
use crate::solve::{SolveOptions, min_storage_for_zero_unserved};
use crate::sweep::Execution;

/// One point of the ASHP/GSHP/district simplex, held as integer
/// numerators over a common denominator so lattice shares are exact
/// (`7/10` produces the same f64 as the literal `0.70` — the
/// correctly-rounded quotient) and the simplex constraint
/// `ashp + gshp + district = 1` holds by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixShares {
    ashp: u32,
    gshp: u32,
    district: u32,
    denominator: u32,
}

impl MixShares {
    /// Build a mix from numerators; they must sum to the (nonzero)
    /// denominator.
    pub fn new(ashp: u32, gshp: u32, district: u32, denominator: u32) -> Result<Self, GridError> {
        if denominator == 0 {
            return Err(GridError::InvalidScenario {
                reason: "heating-mix shares need a nonzero denominator".to_owned(),
            });
        }
        // Checked sum (docs/06: no panics in library crates — raw u32
        // addition would overflow-panic in debug and wrap in release).
        let sum = ashp
            .checked_add(gshp)
            .and_then(|partial| partial.checked_add(district));
        if sum != Some(denominator) {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "heating-mix shares {ashp}/{gshp}/{district} over {denominator} do not \
                     sum to the denominator — the simplex constraint is exact by construction"
                ),
            });
        }
        Ok(Self {
            ashp,
            gshp,
            district,
            denominator,
        })
    }

    /// ASHP share of the electrified quantum.
    #[must_use]
    pub fn ashp_share(&self) -> PerUnit {
        self.share(self.ashp)
    }

    /// GSHP share of the electrified quantum.
    #[must_use]
    pub fn gshp_share(&self) -> PerUnit {
        self.share(self.gshp)
    }

    /// District-geothermal share of the electrified quantum.
    #[must_use]
    pub fn district_share(&self) -> PerUnit {
        self.share(self.district)
    }

    fn share(&self, numerator: u32) -> PerUnit {
        PerUnit::new(f64::from(numerator) / f64::from(self.denominator))
    }

    /// Human-readable label (`ashp 0.70 / gshp 0.20 / district 0.10`).
    #[must_use]
    pub fn label(&self) -> String {
        format!(
            "ashp {:.2} / gshp {:.2} / district {:.2}",
            self.ashp_share().value(),
            self.gshp_share().value(),
            self.district_share().value()
        )
    }
}

/// Largest legal simplex denominator (a 0.001 share step —
/// 501,501 lattice points). The bound exists so adversarial-but-legal
/// inputs (e.g. a tiny CLI `--step`) become a structured error instead
/// of a u32-overflowing lattice-size computation or a
/// hundreds-of-gigabytes allocation; every point costs a full 40-year
/// bisection solve, so anything beyond this cap is a mistake, not a
/// study.
pub const MAX_SIMPLEX_DENOMINATOR: u32 = 1_000;

/// The full simplex lattice at step `1/denominator`: every
/// `(ashp, gshp, district)` with numerators summing to `denominator` —
/// `(n+1)(n+2)/2` points (66 at n = 10). Ordered ASHP-descending, then
/// GSHP-descending: the first point is all-ASHP, the last all-district.
/// Denominators above [`MAX_SIMPLEX_DENOMINATOR`] are a structured
/// error (no panics in library crates — docs/06).
pub fn simplex_mixes(denominator: u32) -> Result<Vec<MixShares>, GridError> {
    if denominator == 0 {
        return Err(GridError::InvalidScenario {
            reason: "heating-mix simplex needs a nonzero step denominator".to_owned(),
        });
    }
    if denominator > MAX_SIMPLEX_DENOMINATOR {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "heating-mix simplex denominator {denominator} exceeds the cap of \
                 {MAX_SIMPLEX_DENOMINATOR} (share step 1/{MAX_SIMPLEX_DENOMINATOR}): the \
                 lattice would hold {}+ points, each costing a full multi-year bisection \
                 solve — use a coarser --step",
                // Saturating: display-only arithmetic, never a panic.
                (u64::from(denominator) + 1).saturating_mul(u64::from(denominator) + 2) / 2
            ),
        });
    }
    let n = denominator;
    // In-cap arithmetic cannot overflow: (1001 × 1002) / 2 ≪ u32::MAX.
    let mut mixes = Vec::with_capacity(((n + 1) * (n + 2) / 2) as usize);
    for ashp in (0..=n).rev() {
        for gshp in (0..=(n - ashp)).rev() {
            mixes.push(MixShares::new(ashp, gshp, n - ashp - gshp, n)?);
        }
    }
    Ok(mixes)
}

/// One point's solve outcome at the stated rating.
#[derive(Debug, Clone, PartialEq)]
pub enum MixOutcome {
    /// The bisection found a requirement.
    Feasible {
        /// Minimum store energy for zero unserved (the naive,
        /// whole-horizon figure — the Stage 3 headline convention).
        requirement: Energy,
        /// The D4 initial-SoC guard flag.
        initial_condition_sensitive: bool,
        /// The one-year burn-in requirement when the guard re-ran the
        /// solve (both figures reported, per D4).
        burn_in_requirement: Option<Energy>,
    },
    /// No store size achieves zero unserved at the stated rating — a
    /// REPORTABLE RESULT (the review's binding item), never a silently
    /// bumped rating.
    Infeasible {
        /// The solver's structured reason.
        reason: String,
    },
}

/// Metrics of one evaluated configuration (a mix point or the
/// no-heating baseline).
#[derive(Debug, Clone, PartialEq)]
pub struct MixMetrics {
    /// Peak residual demand (max of `demand − Σ must-take`);
    /// storage-independent.
    pub peak_residual: Power,
    /// The solve at the stated rating.
    pub outcome: MixOutcome,
    /// Total heating electrical energy over the horizon (zero for the
    /// baseline).
    pub heating_electrical: Energy,
    /// Total delivered heat over the horizon (portfolio-invariant by
    /// construction — D9 rule 5; zero for the baseline).
    pub delivered_heat: Energy,
    /// Per-technology potential (pre-curtailment) energy, fleet order.
    /// Mix-invariant on an all-must-take fleet: the mix moves
    /// curtailment/store cycling/unserved, not potential.
    pub tech_potential: Vec<(TechId, Energy)>,
    /// Pooled curtailment energy (Stage 1 convention, no per-source
    /// attribution) at the fixed dispatch store.
    pub curtailment: Energy,
    /// Store grid-side discharge energy at the fixed dispatch store.
    pub store_discharge: Energy,
    /// Store grid-side charge energy at the fixed dispatch store.
    pub store_charge: Energy,
    /// Unserved energy at the fixed dispatch store.
    pub unserved: Energy,
}

/// One evaluated simplex point.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingMixPoint {
    /// The mix.
    pub shares: MixShares,
    /// Its metrics.
    pub metrics: MixMetrics,
}

/// A completed mix sweep: the stated store configuration, the
/// no-heating baseline, and every point (full response surface,
/// ADR-10).
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingMixSweep {
    /// The stated store power rating, applied to BOTH endpoints of
    /// every delta — stamped here so no storage number travels without
    /// it.
    pub store_power: Power,
    /// The fixed store energy the dispatch metrics used (the scenario's
    /// committed value; the solve column sizes the store separately).
    pub dispatch_store_energy: Energy,
    /// The no-heating baseline metrics (same fleet, same store
    /// configuration).
    pub baseline: MixMetrics,
    /// Every mix point, in input order.
    pub points: Vec<HeatingMixPoint>,
}

/// One named attribution of the D9 rule-6(c) decomposition set.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedAttribution {
    /// `baseline`, `all_ashp`, `all_gshp` or `all_district`.
    pub label: &'static str,
    /// The Stage 4 storage attribution by timescale band.
    pub attribution: StorageAttribution,
}

/// Everything the sweep loads once: the baseline scenario (heating
/// removed, stated rating applied), its run inputs, the heating
/// template, the COP reference and the pinned t2m trace.
#[derive(Debug, Clone)]
pub struct HeatingMixContext {
    scenario: Scenario,
    template: HeatingSpec,
    inputs: RunInputs,
    reference: HeatingCopReference,
    t_pop: Trace<Temperature>,
    store_index: usize,
    store_power: Power,
    start: UtcInstant,
    periods: usize,
}

impl HeatingMixContext {
    /// Load the context from a scenario carrying a
    /// `[zones.demand.heating]` TEMPLATE block (quantum, electrified
    /// share, DHW fraction, temperature trace, and one entry per kind —
    /// all three kinds required; the sweep replaces only the shares).
    /// `store_power` is the STATED rating applied to the designated
    /// store for both the baseline and every heated point.
    pub fn load(
        scenario: &Scenario,
        base_dir: &Path,
        store_power: Power,
        store_index: usize,
    ) -> Result<Self, GridError> {
        scenario.validate()?;
        let zone = single_zone(scenario)?;
        let Some(template) = zone.demand.heating.clone() else {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "zone {}: the heating-mix sweep needs a [zones.demand.heating] template \
                     block (quantum, electrified_share, dhw_fraction, temperature_trace and \
                     the three entries); the sweep replaces only the entry shares",
                    zone.id
                ),
            });
        };
        for kind in [
            HeatingKind::Ashp,
            HeatingKind::Gshp,
            HeatingKind::DistrictGeothermal,
        ] {
            if !template.entries.iter().any(|e| e.kind == kind) {
                return Err(GridError::InvalidScenario {
                    reason: format!(
                        "the heating template must carry one entry per kind; {} is missing \
                         (the simplex sweeps all three shares)",
                        kind.as_str()
                    ),
                });
            }
        }
        if store_index >= zone.storage.len() {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "heating-mix sweep designates store index {store_index}, but zone {} has \
                     {} stores",
                    zone.id,
                    zone.storage.len()
                ),
            });
        }
        if !(store_power.as_gigawatts().is_finite() && store_power.as_gigawatts() > 0.0) {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "the stated store rating must be positive and finite (got {} GW)",
                    store_power.as_gigawatts()
                ),
            });
        }

        // The baseline scenario: heating removed, stated rating applied.
        let mut baseline = scenario.clone();
        baseline.zones[0].demand.heating = None;
        baseline.zones[0].storage[store_index].power_gw = store_power;
        let inputs = load_run_inputs(&baseline, base_dir)?;

        let reference = HeatingCopReference::load(&base_dir.join(HEATING_COP_REFERENCE_PATH))?;
        let t_pop = load_temperature_trace_c(
            &base_dir.join(&template.temperature_trace.path),
            &template.temperature_trace.column,
        )?;
        let start = scenario.horizon.start_instant()?;
        let periods = scenario.horizon.period_count()?;

        Ok(Self {
            scenario: baseline,
            template,
            inputs,
            reference,
            t_pop,
            store_index,
            store_power,
            start,
            periods,
        })
    }

    /// The heated scenario + inputs for one mix (module docs: overlay
    /// added to the once-loaded baseline demand, bit-identical to the
    /// loader's own arithmetic).
    fn heated_variant(&self, mix: &MixShares) -> Result<(Scenario, RunInputs), GridError> {
        let mut spec = self.template.clone();
        for entry in &mut spec.entries {
            entry.share = match entry.kind {
                HeatingKind::Ashp => mix.ashp_share(),
                HeatingKind::Gshp => mix.gshp_share(),
                HeatingKind::DistrictGeothermal => mix.district_share(),
            };
        }
        let overlay = compute_overlay(
            &spec,
            &self.reference,
            &self.t_pop,
            self.start,
            self.periods,
        )?;
        let demand = Trace::from_parts(
            self.inputs.demand.start(),
            self.inputs
                .demand
                .values()
                .iter()
                .zip(&overlay.electrical_total)
                .map(|(&d, &h)| d + h)
                .collect(),
        )?;
        let mut scenario = self.scenario.clone();
        scenario.zones[0].demand.heating = Some(spec);
        let inputs = RunInputs {
            demand,
            capacity_factors: self.inputs.capacity_factors.clone(),
            exogenous: self.inputs.exogenous.clone(),
            availability: self.inputs.availability.clone(),
            heating: Some(overlay),
        };
        Ok((scenario, inputs))
    }

    /// Evaluate one configuration: dispatch run at the fixed store,
    /// bisection solve at the stated rating (module docs).
    fn metrics(
        &self,
        scenario: &Scenario,
        inputs: &RunInputs,
        options: &SolveOptions,
    ) -> Result<MixMetrics, GridError> {
        let result = run(scenario, inputs)?;
        let peak_residual = peak_residual(&result)?;

        let outcome =
            match min_storage_for_zero_unserved(scenario, inputs, self.store_index, options) {
                Ok(solved) => MixOutcome::Feasible {
                    requirement: solved.naive.requirement,
                    initial_condition_sensitive: solved.initial_condition_sensitive,
                    burn_in_requirement: solved.burn_in.map(|b| b.requirement),
                },
                Err(GridError::SolveInfeasible { reason }) => MixOutcome::Infeasible { reason },
                Err(other) => return Err(other),
            };

        let zero = Energy::gigawatt_hours(0.0);
        let (heating_electrical, delivered_heat) = match &inputs.heating {
            None => (zero, zero),
            Some(overlay) => (
                RunResult::total_energy(&overlay.electrical_total),
                RunResult::total_energy(&overlay.delivered_heat),
            ),
        };
        let tech_potential = result
            .renewables
            .iter()
            .map(|s| (s.tech.clone(), RunResult::total_energy(&s.power)))
            .collect();
        let sum_stores = |select: fn(&crate::result::StoreSeries) -> &[Power]| -> Energy {
            result
                .stores
                .iter()
                .map(|s| RunResult::total_energy(select(s)))
                .fold(zero, |acc, e| acc + e)
        };

        Ok(MixMetrics {
            peak_residual,
            outcome,
            heating_electrical,
            delivered_heat,
            tech_potential,
            curtailment: result.total_curtailment(),
            store_discharge: sum_stores(|s| &s.discharge),
            store_charge: sum_stores(|s| &s.charge),
            unserved: result.total_unserved(),
        })
    }

    /// Run the sweep over `mixes` (module docs). Parallel and serial
    /// execution are bit-identical.
    pub fn sweep(
        &self,
        mixes: &[MixShares],
        options: &SolveOptions,
        execution: Execution,
    ) -> Result<HeatingMixSweep, GridError> {
        if mixes.is_empty() {
            return Err(GridError::InvalidScenario {
                reason: "heating-mix sweep: no mix points given".to_owned(),
            });
        }
        let baseline = self.metrics(&self.scenario, &self.inputs, options)?;

        let evaluate = |mix: &MixShares| -> Result<HeatingMixPoint, GridError> {
            let (scenario, inputs) = self.heated_variant(mix)?;
            Ok(HeatingMixPoint {
                shares: *mix,
                metrics: self.metrics(&scenario, &inputs, options)?,
            })
        };
        let points = match execution {
            Execution::Parallel => mixes
                .par_iter()
                .map(evaluate)
                .collect::<Result<Vec<_>, _>>()?,
            Execution::Serial => mixes.iter().map(evaluate).collect::<Result<Vec<_>, _>>()?,
        };

        Ok(HeatingMixSweep {
            store_power: self.store_power,
            dispatch_store_energy: self.scenario.zones[0].storage[self.store_index].energy_gwh,
            baseline,
            points,
        })
    }

    /// The D9 rule-6(c) decomposition set: the Stage 4 storage
    /// attribution by timescale band for the no-heating baseline and
    /// the three pure-technology corners, in that order, all at the
    /// stated rating. The window convention is the caller's and must be
    /// quoted with any band ranking (the Stage 4 publication rule).
    pub fn attributions(
        &self,
        windows: &DecompositionWindows,
        options: &SolveOptions,
    ) -> Result<Vec<NamedAttribution>, GridError> {
        let mut out = vec![NamedAttribution {
            label: "baseline",
            attribution: attribute_storage_by_band(
                &self.scenario,
                &self.inputs,
                self.store_index,
                windows,
                options,
            )?,
        }];
        for (label, mix) in [
            ("all_ashp", MixShares::new(1, 0, 0, 1)?),
            ("all_gshp", MixShares::new(0, 1, 0, 1)?),
            ("all_district", MixShares::new(0, 0, 1, 1)?),
        ] {
            let (scenario, inputs) = self.heated_variant(&mix)?;
            out.push(NamedAttribution {
                label,
                attribution: attribute_storage_by_band(
                    &scenario,
                    &inputs,
                    self.store_index,
                    windows,
                    options,
                )?,
            });
        }
        Ok(out)
    }
}

/// Peak residual demand of a run: max over periods of
/// `demand − Σ must-take`, in the engine's own summation order.
fn peak_residual(result: &RunResult) -> Result<Power, GridError> {
    let must_take: Vec<&[Power]> = result
        .renewables
        .iter()
        .map(|s| s.power.as_slice())
        .chain(result.exogenous.iter().map(|s| s.power.as_slice()))
        .collect();
    let residual = residual_load(&result.demand, &must_take)?;
    residual
        .into_iter()
        .reduce(|a, b| if b > a { b } else { a })
        .ok_or(GridError::InvalidAnalysisInput {
            reason: "peak residual of an empty run".to_owned(),
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn simplex_lattice_has_the_triangular_count_and_exact_sums() {
        let mixes = simplex_mixes(10).unwrap();
        assert_eq!(mixes.len(), 66, "step 0.1 gives the 66-point simplex");
        // First all-ASHP, last all-district (documented order).
        assert_eq!(mixes[0], MixShares::new(10, 0, 0, 10).unwrap());
        assert_eq!(mixes[65], MixShares::new(0, 0, 10, 10).unwrap());
        // Every point satisfies the simplex constraint within the
        // schema's 1e-9 share-sum tolerance, and no duplicates.
        for mix in &mixes {
            let sum =
                mix.ashp_share().value() + mix.gshp_share().value() + mix.district_share().value();
            assert!((sum - 1.0).abs() <= 1e-9, "{}: sum {sum}", mix.label());
        }
        let mut seen = mixes.clone();
        seen.dedup();
        assert_eq!(seen.len(), 66);
    }

    #[test]
    fn lattice_shares_reproduce_the_literal_decimals_bit_for_bit() {
        // 7/10 must be the same f64 as the literal 0.70 (both are the
        // correctly-rounded value), so the D9 point matches the
        // characterisation pin's `PerUnit::new(0.70)` exactly.
        let mix = MixShares::new(7, 2, 1, 10).unwrap();
        assert_eq!(mix.ashp_share(), PerUnit::new(0.70));
        assert_eq!(mix.gshp_share(), PerUnit::new(0.20));
        assert_eq!(mix.district_share(), PerUnit::new(0.10));
    }

    #[test]
    fn mix_shares_reject_bad_numerators_and_zero_denominator() {
        assert!(matches!(
            MixShares::new(5, 5, 5, 10).unwrap_err(),
            GridError::InvalidScenario { .. }
        ));
        assert!(matches!(
            MixShares::new(0, 0, 0, 0).unwrap_err(),
            GridError::InvalidScenario { .. }
        ));
        assert!(matches!(
            simplex_mixes(0).unwrap_err(),
            GridError::InvalidScenario { .. }
        ));
    }

    /// No panics in library crates (docs/06): adversarial-but-legal
    /// inputs that would overflow u32 arithmetic (huge numerators; a
    /// tiny CLI `--step` reaching an astronomical denominator) must be
    /// structured errors, never overflow panics (debug) or silent
    /// wraps (release).
    #[test]
    fn overflow_scale_inputs_are_structured_errors_not_panics() {
        // Numerator sum overflows u32.
        assert!(matches!(
            MixShares::new(u32::MAX, u32::MAX, u32::MAX, 1).unwrap_err(),
            GridError::InvalidScenario { .. }
        ));
        // A wrapping sum must NOT satisfy the simplex constraint:
        // u32::MAX + 1 + 0 wraps to 0 in release arithmetic.
        assert!(matches!(
            MixShares::new(u32::MAX, 1, 0, 0).unwrap_err(),
            GridError::InvalidScenario { .. }
        ));
        // Denominators whose lattice size overflows or is absurd
        // (u32::MAX would be ~9e18 points; 1e9 from `--step 1e-9`
        // would be ~5e17 points): refused with the stated cap.
        for denominator in [u32::MAX, 1_000_000_000, MAX_SIMPLEX_DENOMINATOR + 1] {
            let err = simplex_mixes(denominator).unwrap_err();
            assert!(
                matches!(&err, GridError::InvalidScenario { .. })
                    && err
                        .to_string()
                        .contains(&MAX_SIMPLEX_DENOMINATOR.to_string()),
                "denominator {denominator}: expected the cap in the error, got: {err}"
            );
        }
        // The cap itself is legal.
        assert!(simplex_mixes(MAX_SIMPLEX_DENOMINATOR).is_ok());
    }

    #[test]
    fn simplex_step_one_is_the_three_corners() {
        let mixes = simplex_mixes(1).unwrap();
        assert_eq!(
            mixes,
            vec![
                MixShares::new(1, 0, 0, 1).unwrap(),
                MixShares::new(0, 1, 0, 1).unwrap(),
                MixShares::new(0, 0, 1, 1).unwrap(),
            ]
        );
    }
}
