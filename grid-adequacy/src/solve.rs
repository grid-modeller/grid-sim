//! `min_storage_for_zero_unserved` — bisection over one store's energy
//! capacity (ADR-10: bisection for monotone 1-D problems).
//!
//! # Parameterisation (documented decision)
//!
//! The solver scales the **energy capacity (GWh) of one designated
//! store** — chosen by index into the zone's storage portfolio — and
//! holds everything else fixed: the store's power rating, its
//! round-trip efficiency, its `initial_soc` *fraction* (so the initial
//! energy scales with the capacity), every other store, the fleet and
//! the traces. This matches the headline research question ("how much
//! storage energy does this fleet need?", docs/07 M4/Q1) and keeps the
//! search one-dimensional per ADR-10; power-versus-energy trade-offs
//! are Stage 4 sweep territory.
//!
//! **Monotonicity assumption (stated):** total unserved energy is
//! treated as non-increasing in the designated store's energy capacity
//! under the rule-based policy. For a single store this holds (more
//! headroom never absorbs less surplus and never holds less SoC); with
//! multiple stores the greedy cascade makes pathological
//! counter-examples conceivable but they have not been observed — the
//! bisection trace is reported in full so a non-monotone response would
//! be visible in the artefact.
//!
//! # Feasibility and tolerance
//!
//! A candidate capacity is *feasible* when the run's total unserved
//! energy is ≤ 10⁻⁹ GWh (exact zero up to f64 dust). Bisection starts
//! from a doubling search for a feasible upper bound (capped —
//! infeasible caps are a structured [`GridError::SolveInfeasible`],
//! CLI exit 1) and narrows until `hi − lo` is within the configured
//! tolerance; the reported requirement is the smallest *known-feasible*
//! capacity (`hi`), never an untested interpolation.
//!
//! # The D4 initial-SoC guard
//!
//! Stores start full by default, which can quietly subsidise the first
//! winter: if the found requirement's minimum SoC (for the designated
//! store) occurs within the **first weather year** of the horizon, the
//! result is flagged initial-condition-sensitive and the solve is
//! re-run with a one-year burn-in — unserved energy in year 1 is
//! excluded from the feasibility test (the SoC still evolves through
//! it) — and **both figures are reported**. For single-year horizons
//! the burn-in re-run is meaningless (it would exclude the whole
//! horizon) and is skipped with the flag still reported.

use grid_core::GridError;
use grid_core::scenario::Scenario;
use grid_core::time::UtcInstant;
use grid_core::units::{Duration, Energy, Power};

use crate::dispatch::run;
use crate::inputs::{MultiZoneInputs, RunInputs, single_zone};
use crate::lp::run_multi_lp;
use crate::multizone::{MultiZoneRunResult, run_multi};

/// Solver options; [`SolveOptions::default`] gives the documented
/// defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolveOptions {
    /// Upper cap for the doubling search (default 10⁶ GWh = 1000 TWh —
    /// an order of magnitude above any plausible GB requirement).
    pub max_energy: Energy,
    /// Absolute convergence tolerance on the capacity (default 0.1 GWh).
    pub absolute_tolerance: Energy,
    /// Relative convergence tolerance (default 10⁻³ of the upper
    /// bracket); the effective tolerance is the larger of the two.
    pub relative_tolerance: f64,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            max_energy: Energy::gigawatt_hours(1e6),
            absolute_tolerance: Energy::gigawatt_hours(0.1),
            relative_tolerance: 1e-3,
        }
    }
}

/// One bisection evaluation, reported in full (ADR-10: keep the whole
/// response, not just the optimum).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BisectionIterate {
    /// Candidate energy capacity of the designated store.
    pub energy: Energy,
    /// Unserved energy at that capacity (post-burn-in total in the
    /// burn-in phase).
    pub unserved: Energy,
    /// Whether the candidate met the zero-unserved criterion.
    pub feasible: bool,
}

/// One bisection's outcome: the requirement and its full trace.
#[derive(Debug, Clone, PartialEq)]
pub struct BisectionOutcome {
    /// The smallest known-feasible energy capacity.
    pub requirement: Energy,
    /// Every evaluation, in execution order.
    pub iterations: Vec<BisectionIterate>,
}

/// The full solve result, including the D4 initial-SoC guard outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct SolveResult {
    /// Output label of the designated store.
    pub store_label: String,
    /// The naive (whole-horizon) solve.
    pub naive: BisectionOutcome,
    /// Minimum SoC of the designated store at the naive requirement…
    pub min_soc: Energy,
    /// …and when it (first) occurs.
    pub min_soc_at: UtcInstant,
    /// D4 guard: the minimum SoC falls within the first weather year,
    /// so the naive figure leans on the initial condition.
    pub initial_condition_sensitive: bool,
    /// The one-year burn-in re-run (only when flagged and the horizon
    /// extends beyond year 1).
    pub burn_in: Option<BisectionOutcome>,
    /// Why the burn-in re-run was skipped despite the flag (single-year
    /// horizon), if it was.
    pub burn_in_skipped: Option<String>,
}

/// Zero-unserved criterion: total unserved ≤ 10⁻⁹ GWh (module docs).
fn is_feasible(unserved: Energy) -> bool {
    unserved.as_gigawatt_hours() <= 1e-9
}

/// Total unserved energy of a run, counting periods from `from_index`.
fn unserved_from(unserved: &[Power], from_index: usize) -> Energy {
    unserved
        .iter()
        .skip(from_index)
        .map(|&p| p * Duration::half_hour())
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

/// Find the minimum energy capacity of the store at `store_index` (into
/// the zone's storage list, scenario order) for zero unserved energy
/// over the horizon. See the module docs for the parameterisation, the
/// tolerance semantics and the initial-SoC guard.
pub fn min_storage_for_zero_unserved(
    scenario: &Scenario,
    inputs: &RunInputs,
    store_index: usize,
    options: &SolveOptions,
) -> Result<SolveResult, GridError> {
    scenario.validate()?;
    let zone = single_zone(scenario)?;
    let Some(designated) = zone.storage.get(store_index) else {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "solve designates store index {store_index}, but zone {} has {} stores",
                zone.id,
                zone.storage.len()
            ),
        });
    };
    // Label via the engine's disambiguation rule (duplicate kinds).
    let repeated = zone
        .storage
        .iter()
        .filter(|s| s.kind == designated.kind)
        .count()
        > 1;
    let store_label = if repeated {
        format!("{}_{}", designated.kind, designated.dispatch_order)
    } else {
        designated.kind.as_str().to_owned()
    };

    // One run per candidate: clone the scenario, swap the designated
    // store's energy. Pure and deterministic (ADR-5).
    let evaluate =
        |energy: Energy, from_index: usize| -> Result<(Energy, crate::RunResult), GridError> {
            let mut candidate = scenario.clone();
            candidate.zones[0].storage[store_index].energy_gwh = energy;
            let result = run(&candidate, inputs)?;
            Ok((unserved_from(&result.unserved, from_index), result))
        };

    let naive = bisect(&evaluate, 0, options, &store_label)?;

    // The D4 initial-SoC guard: min SoC of the designated store at the
    // naive requirement, and whether it falls in the first weather year.
    let (_, at_requirement) = evaluate(naive.requirement, 0)?;
    let series = at_requirement
        .stores
        .iter()
        .find(|s| s.label == store_label)
        .ok_or_else(|| GridError::InvalidScenario {
            reason: format!("designated store {store_label} produced no output series"),
        })?;
    let (min_index, min_soc) = series.min_soc().ok_or_else(|| GridError::InvalidScenario {
        reason: "solve over an empty horizon".to_owned(),
    })?;
    let min_soc_at = at_requirement.timestamp_at(min_index);
    let start = scenario.horizon.start_instant()?;
    let (start_year, _, _) = start.civil_date();
    let (min_year, _, _) = min_soc_at.civil_date();
    let initial_condition_sensitive = min_year == start_year;

    let mut burn_in = None;
    let mut burn_in_skipped = None;
    if initial_condition_sensitive {
        // First period of the second weather year.
        let periods = scenario.horizon.period_count()?;
        let year2 = UtcInstant::parse(&format!("{:04}-01-01T00:00:00Z", start_year + 1))?;
        let burn_in_periods = start.periods_until_inclusive(year2)? - 1;
        if burn_in_periods >= periods {
            burn_in_skipped = Some(format!(
                "the horizon does not extend beyond the first weather year ({start_year}); a \
                 one-year burn-in would exclude every period — the naive figure stands, \
                 flagged initial-condition-sensitive"
            ));
        } else {
            burn_in = Some(bisect(&evaluate, burn_in_periods, options, &store_label)?);
        }
    }

    Ok(SolveResult {
        store_label,
        naive,
        min_soc,
        min_soc_at,
        initial_condition_sensitive,
        burn_in,
        burn_in_skipped,
    })
}

/// Total unserved energy of a multi-zone run, summed over every zone,
/// counting periods from `from_index` (the LP oracle's feasibility
/// quantity: a feasible zero-unserved dispatch exists iff this is ~0).
fn total_unserved_multi(result: &MultiZoneRunResult, from_index: usize) -> Energy {
    result
        .zones
        .iter()
        .map(|z| unserved_from(&z.result.unserved, from_index))
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

/// Find the minimum energy capacity of the store at `store_index` in
/// zone `zone_index` (both into scenario order) for zero TOTAL unserved
/// energy under the **perfect-foresight LP** — the D12 package-2b
/// bisection FEASIBILITY ORACLE (`docs/notes/d12-perfect-foresight-lp.md`
/// rule 2). It mirrors [`min_storage_for_zero_unserved`] exactly — same
/// doubling-search-then-interval-halving [`bisect`], same
/// [`is_feasible`] criterion (total unserved ≤ 1e-9 GWh), same
/// [`SolveResult`] shape and year-1 burn-in guard — but takes
/// [`MultiZoneInputs`] and uses [`run_multi_lp`] as the inner feasibility
/// test instead of the rule-based dispatch.
///
/// # Store designation (documented convention)
///
/// The scaled store is picked by `(zone_index, store_index)`: the index
/// of the zone in scenario `[[zones]]` order, then the index of the
/// store in that zone's `storage` list. Everything else is held fixed —
/// the store's power rating, its round-trip efficiency, its `initial_soc`
/// *fraction* (so its initial energy scales with the capacity), every
/// other store, the fleet and the traces — exactly as the single-zone
/// rule-based solver does.
///
/// Under the rule-2 oracle framing the LP measures the SAME quantity the
/// rule-based bisection does (minimum store for zero unserved), so `LP
/// requirement ≤ RuleBased requirement` holds on identical fleets and the
/// same √η convention (rule 4) — the sanity invariant, non-strict in
/// general and strict where foresight/wheeling genuinely helps.
pub fn min_storage_for_zero_unserved_lp(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    zone_index: usize,
    store_index: usize,
    options: &SolveOptions,
) -> Result<SolveResult, GridError> {
    min_storage_multi(
        scenario,
        inputs,
        zone_index,
        store_index,
        options,
        &run_multi_lp,
    )
}

/// The shared multi-zone bisection: [`min_storage_for_zero_unserved_lp`]
/// with the inner dispatch function abstracted, so the SAME machinery
/// (same doubling-search-then-halving [`bisect`], same feasibility
/// criterion, same store designation and year-1 burn-in guard) measures
/// the requirement under BOTH the perfect-foresight LP and the
/// scenario's rule-based path — the like-for-like precondition of the
/// docs/04 Stage 7 gap report.
fn min_storage_multi(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    zone_index: usize,
    store_index: usize,
    options: &SolveOptions,
    runner: &dyn Fn(&Scenario, &MultiZoneInputs) -> Result<MultiZoneRunResult, GridError>,
) -> Result<SolveResult, GridError> {
    scenario.validate()?;
    let Some(zone) = scenario.zones.get(zone_index) else {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "LP solve designates zone index {zone_index}, but the scenario has {} zones",
                scenario.zones.len()
            ),
        });
    };
    let Some(designated) = zone.storage.get(store_index) else {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "LP solve designates store index {store_index}, but zone {} has {} stores",
                zone.id,
                zone.storage.len()
            ),
        });
    };
    // Label via the engine's disambiguation rule (duplicate kinds).
    let repeated = zone
        .storage
        .iter()
        .filter(|s| s.kind == designated.kind)
        .count()
        > 1;
    let store_label = if repeated {
        format!("{}_{}", designated.kind, designated.dispatch_order)
    } else {
        designated.kind.as_str().to_owned()
    };

    // One dispatch per candidate: clone the scenario, swap the
    // designated store's energy. Pure and deterministic (ADR-5).
    let evaluate =
        |energy: Energy, from_index: usize| -> Result<(Energy, MultiZoneRunResult), GridError> {
            let mut candidate = scenario.clone();
            candidate.zones[zone_index].storage[store_index].energy_gwh = energy;
            let result = runner(&candidate, inputs)?;
            Ok((total_unserved_multi(&result, from_index), result))
        };

    let naive = bisect(&evaluate, 0, options, &store_label)?;

    // The D4 initial-SoC guard: min SoC of the designated store at the
    // naive requirement, and whether it falls in the first weather year.
    let (_, at_requirement) = evaluate(naive.requirement, 0)?;
    let zone_result = &at_requirement.zones[zone_index].result;
    let series = zone_result
        .stores
        .iter()
        .find(|s| s.label == store_label)
        .ok_or_else(|| GridError::InvalidScenario {
            reason: format!("designated store {store_label} produced no output series"),
        })?;
    let (min_index, min_soc) = series.min_soc().ok_or_else(|| GridError::InvalidScenario {
        reason: "solve over an empty horizon".to_owned(),
    })?;
    let min_soc_at = zone_result.timestamp_at(min_index);
    let start = scenario.horizon.start_instant()?;
    let (start_year, _, _) = start.civil_date();
    let (min_year, _, _) = min_soc_at.civil_date();
    let initial_condition_sensitive = min_year == start_year;

    let mut burn_in = None;
    let mut burn_in_skipped = None;
    if initial_condition_sensitive {
        // First period of the second weather year.
        let periods = scenario.horizon.period_count()?;
        let year2 = UtcInstant::parse(&format!("{:04}-01-01T00:00:00Z", start_year + 1))?;
        let burn_in_periods = start.periods_until_inclusive(year2)? - 1;
        if burn_in_periods >= periods {
            burn_in_skipped = Some(format!(
                "the horizon does not extend beyond the first weather year ({start_year}); a \
                 one-year burn-in would exclude every period — the naive figure stands, \
                 flagged initial-condition-sensitive"
            ));
        } else {
            burn_in = Some(bisect(&evaluate, burn_in_periods, options, &store_label)?);
        }
    }

    Ok(SolveResult {
        store_label,
        naive,
        min_soc,
        min_soc_at,
        initial_condition_sensitive,
        burn_in,
        burn_in_skipped,
    })
}

// =====================================================================
// The per-scenario LP-vs-rule-based storage GAP REPORT (docs/04 Stage 7
// acceptance: "LP storage requirement ≤ rule-based on every scenario
// (sanity invariant); gap reported per scenario").
//
// Both requirements are measured by the SAME bisection machinery on the
// SAME store designation and options — the like-for-like precondition
// of the D12 rule-4 invariant (identical fleet, same √η convention).
// The non-LP leg runs `run_multi`, i.e. the scenario's declared
// per-period policy path (RuleBased for every committed scenario); the
// LP leg runs `run_multi_lp` (deliberately NOT a DispatchPolicy — the
// adopted D12 design supersedes docs/04's older "PerfectForesight
// policy" wording). The gap is a REPORTED FINDING (ADR-6), never tuned
// away; a measured LP > rule-based beyond the bisection slack is an
// engine defect and errors loudly instead of being reported.
// =====================================================================

/// The per-scenario gap artefact: both solves in full (ADR-10: keep the
/// whole response), the headline requirements, the gap, the invariant
/// slack it was checked under, and the publication stamps (quarantine →
/// refuse; the HiGHS cross-machine floating-point caveat travels with
/// every published LP number — the D12 2b review obligation).
#[derive(Debug, Clone, PartialEq)]
pub struct StorageGapReport {
    /// The scenario's name (per-scenario reporting).
    pub scenario_name: String,
    /// Output label of the designated store.
    pub store_label: String,
    /// The rule-based-path solve, in full.
    pub rule_based: SolveResult,
    /// The perfect-foresight LP solve, in full.
    pub lp: SolveResult,
    /// The rule-based (naive, whole-horizon) requirement. Burn-in
    /// figures, where the year-1 guard fired, travel inside the
    /// embedded [`SolveResult`]s.
    pub rule_based_requirement: Energy,
    /// The LP (naive, whole-horizon) requirement.
    pub lp_requirement: Energy,
    /// The reported gap: rule-based − LP (non-negative up to the
    /// bisection slack, by the invariant).
    pub gap: Energy,
    /// The slack the invariant was checked under: the sum of the two
    /// solves' effective bisection tolerances.
    pub invariant_slack: Energy,
    /// `false` iff the caller declared quarantined inputs.
    pub quotable: bool,
    /// Quarantined inputs the scenario consumed, as declared by the
    /// caller (the artefact layer knows what data fed the scenario).
    pub consumed_quarantined_inputs: Vec<String>,
    /// Publication caveats that travel with every quote of this report.
    pub caveats: Vec<String>,
}

impl StorageGapReport {
    /// The publish gate (docs/04 Stage 7 pin): refuse a report whose
    /// inputs were quarantined.
    pub fn ensure_publishable(&self) -> Result<(), GridError> {
        if self.consumed_quarantined_inputs.is_empty() {
            Ok(())
        } else {
            Err(GridError::NonQuotableResult {
                reason: format!(
                    "consumed quarantined inputs [{}]",
                    self.consumed_quarantined_inputs.join(", ")
                ),
            })
        }
    }
}

/// The effective bisection tolerance at a converged requirement: the
/// larger of the absolute and relative tolerances (mirrors `bisect`'s
/// stopping rule).
fn effective_tolerance(requirement: Energy, options: &SolveOptions) -> Energy {
    let relative = requirement * options.relative_tolerance;
    if relative > options.absolute_tolerance {
        relative
    } else {
        options.absolute_tolerance
    }
}

/// The docs/04 Stage 7 sanity invariant, checked structurally: the LP
/// requirement may not exceed the rule-based requirement by more than
/// `slack` (both requirements are smallest-known-feasible bisection
/// outputs, each true only to its own tolerance). A violation is a
/// [`GridError::SanityInvariantViolated`] — an engine defect surfaced
/// loudly, never a reportable finding.
pub fn check_storage_gap_invariant(
    rule_based: Energy,
    lp: Energy,
    slack: Energy,
) -> Result<(), GridError> {
    if lp > rule_based + slack {
        return Err(GridError::SanityInvariantViolated {
            reason: format!(
                "LP storage requirement {} GWh exceeds the rule-based requirement {} GWh \
                 beyond the bisection slack {} GWh — perfect foresight can never need MORE \
                 storage than the greedy policy on an identical fleet (D12 rule 4)",
                lp.as_gigawatt_hours(),
                rule_based.as_gigawatt_hours(),
                slack.as_gigawatt_hours()
            ),
        });
    }
    Ok(())
}

/// Measure the rule-based and perfect-foresight-LP storage requirements
/// on one scenario with the same designation and options, assert the
/// sanity invariant, and return the gap artefact. See the section
/// comment above for the conventions; `quarantined_inputs` is the
/// caller's declaration of any quarantined data the scenario consumed
/// (the report is then stamped non-quotable and the publish path
/// refuses it).
pub fn storage_gap_report(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    zone_index: usize,
    store_index: usize,
    options: &SolveOptions,
    quarantined_inputs: &[String],
) -> Result<StorageGapReport, GridError> {
    let rule_based = min_storage_multi(
        scenario,
        inputs,
        zone_index,
        store_index,
        options,
        &run_multi,
    )?;
    let lp = min_storage_multi(
        scenario,
        inputs,
        zone_index,
        store_index,
        options,
        &run_multi_lp,
    )?;

    let rule_based_requirement = rule_based.naive.requirement;
    let lp_requirement = lp.naive.requirement;
    let invariant_slack = effective_tolerance(rule_based_requirement, options)
        + effective_tolerance(lp_requirement, options);
    check_storage_gap_invariant(rule_based_requirement, lp_requirement, invariant_slack)?;

    let store_label = rule_based.store_label.clone();
    Ok(StorageGapReport {
        scenario_name: scenario.name.clone(),
        store_label,
        rule_based_requirement,
        lp_requirement,
        gap: rule_based_requirement - lp_requirement,
        invariant_slack,
        rule_based,
        lp,
        quotable: quarantined_inputs.is_empty(),
        consumed_quarantined_inputs: quarantined_inputs.to_vec(),
        caveats: vec![
            "HiGHS floating-point caveat: LP solutions are deterministic on one machine \
             but may differ across machines/architectures at floating-point level — state \
             this with any published LP number (D12 2b review obligation)"
                .to_owned(),
            "the rule-based-vs-LP gap is a reported finding (ADR-6 dual-policy discipline), \
             not an error to be tuned away; the LP is the perfect-foresight central-planner \
             floor and the rule-based figure the no-foresight envelope — reality sits \
             between"
                .to_owned(),
        ],
    })
}

/// The bisection itself: doubling search for a feasible upper bound,
/// then interval halving; requirement = smallest known-feasible
/// capacity.
///
/// Generic over the run-result type `R` the `evaluate` closure returns
/// (discarded here — only the unserved energy drives the search), so the
/// SAME machinery serves both the rule-based bisection (`R =
/// RunResult`) and the LP bisection (`R = MultiZoneRunResult`). For the
/// rule-based caller `R` is inferred as `RunResult`, so its generated
/// code and behaviour are byte-for-byte unchanged.
fn bisect<R>(
    evaluate: &dyn Fn(Energy, usize) -> Result<(Energy, R), GridError>,
    from_index: usize,
    options: &SolveOptions,
    store_label: &str,
) -> Result<BisectionOutcome, GridError> {
    let mut iterations = Vec::new();
    let mut check = |energy: Energy| -> Result<bool, GridError> {
        let (unserved, _) = evaluate(energy, from_index)?;
        let feasible = is_feasible(unserved);
        iterations.push(BisectionIterate {
            energy,
            unserved,
            feasible,
        });
        Ok(feasible)
    };

    // Zero capacity feasible → the fleet needs no storage at all.
    if check(Energy::gigawatt_hours(0.0))? {
        return Ok(BisectionOutcome {
            requirement: Energy::gigawatt_hours(0.0),
            iterations,
        });
    }

    // Doubling search for a feasible upper bracket.
    let mut lo = Energy::gigawatt_hours(0.0);
    let mut hi = Energy::gigawatt_hours(1.0);
    loop {
        if hi > options.max_energy {
            return Err(GridError::SolveInfeasible {
                reason: format!(
                    "store {store_label}: unserved energy persists at the {} GWh search cap — \
                     no storage size achieves zero unserved (is the fleet's firm capacity \
                     below peak residual demand, or the store's power rating too small?)",
                    options.max_energy.as_gigawatt_hours()
                ),
            });
        }
        if check(hi)? {
            break;
        }
        lo = hi;
        hi = hi * 2.0;
    }

    // Interval halving to tolerance.
    let tolerance = |hi: Energy| -> Energy {
        let relative = hi * options.relative_tolerance;
        if relative > options.absolute_tolerance {
            relative
        } else {
            options.absolute_tolerance
        }
    };
    while (hi - lo) > tolerance(hi) {
        let mid = (lo + hi) / 2.0;
        if check(mid)? {
            hi = mid;
        } else {
            lo = mid;
        }
    }

    Ok(BisectionOutcome {
        requirement: hi,
        iterations,
    })
}
