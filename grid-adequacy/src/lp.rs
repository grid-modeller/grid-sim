//! The D12 perfect-foresight LP dispatch CORE (package 2a;
//! `docs/notes/d12-perfect-foresight-lp.md` rules 2, 3, 5; ADR-6 policy
//! set, ADR-10 `good_lp` + HiGHS).
//!
//! # What this is
//!
//! A perfect-foresight linear program over the WHOLE horizon, run in
//! parallel to the rule-based [`crate::run_multi`]. It reads the same
//! fixed inputs the rule-based engine reads and returns the same
//! [`MultiZoneRunResult`] shape, and (package 2b) it IS the inner
//! FEASIBILITY ORACLE of the bisection sizing loop
//! ([`crate::solve::min_storage_for_zero_unserved_lp`]): a feasible
//! zero-unserved dispatch exists iff the minimised total unserved is ~0
//! (`≤ 1e-9` GWh).
//!
//! It is deliberately NOT a [`crate::DispatchPolicy`]: that trait is
//! per-period with a no-lookahead `SystemState` (D4), and perfect
//! foresight needs the whole horizon. So this is a new whole-horizon
//! function, not a policy.
//!
//! # The formulation (all linear — this is an LP, not a MILP)
//!
//! Per zone `z`, period `t`, timestep `dt = 0.5 h`. FIXED coefficients
//! (not variables): demand; renewable/exogenous must-take; per-thermal
//! ceiling `capacity × availability(t)`; link directional capabilities.
//!
//! DECISION variables (all `≥ 0` unless noted):
//! - per-(zone, thermal, t) generation `gen`, bounded above by the
//!   thermal ceiling;
//! - per-(zone, store, t) grid-side `charge` and `discharge` (bounded by
//!   the power rating) and `soc` (bounded `[0, energy]`);
//! - per-(live link, t) directional sending-end flows `fwd` (`from→to`)
//!   and `rev` (`to→from`), bounded by the directional capabilities;
//! - per-(zone, t) `unserved` and `curtailment`.
//!
//! CONSTRAINTS:
//! - **Zone energy balance**, each `(z, t)`:
//!   `must_take + Σ gen + Σ discharge + Σ import·(1−loss) + unserved
//!      == demand + Σ charge + Σ export_sending + curtailment`.
//!   The RECEIVING end gets `sent × (1 − loss)` — the engine's link-loss
//!   convention (`multizone.rs`; `LinkSpec.loss`). Links are explicit
//!   flow variables here (NOT pre-folded into must-take), which is what
//!   enables multi-hop wheeling (N→S→E in one period).
//! - **Storage dynamics** matching [`crate::StoreState::apply`] EXACTLY
//!   (D12 rule 4, like-for-like with the rule-based engine):
//!   `soc[t] == soc_prev + charge·dt·√η − discharge·dt/√η`, with
//!   `soc_prev` the initial SoC for `t = 0`; `0 ≤ soc ≤ energy` every
//!   period. `√η = sqrt(round_trip_efficiency)`; initial SoC =
//!   `energy × initial_soc.unwrap_or(1.0)` (full by default, D4). No
//!   annual reset — SoC carries across the whole horizon; no final-SoC
//!   constraint (perfect foresight may run the store down by horizon
//!   end, the same freedom the start-full rule-based policy has).
//!
//! # Simultaneous charge and discharge
//!
//! With `√η < 1` a round trip strictly loses energy, so simultaneous
//! charge+discharge is dominated under a min-unserved objective and does
//! not appear at the optimum — but an LP CAN return a degenerate vertex
//! with both nonzero. Netting them out post-hoc is impossible without
//! breaking either SoC or the zone balance (the round-trip loss is real),
//! so instead the objective carries a tiny throughput penalty
//! [`CYCLING_PENALTY`] that makes any redundant simultaneous cycling
//! strictly worse and drives it to zero. It cannot trade away storage
//! that reduces unserved: delivering energy `E` to cut unserved costs
//! throughput `E·(1 + 1/η)` (grid-side charge `E/η` plus discharge `E`),
//! so removing `δ` of unserved carries a penalty of only
//! `1e-6·(1 + 1/η)·δ` against a unit benefit `δ` — strictly dominated for
//! every real round-trip efficiency (η ∈ [0.3, 0.95] ⇒ penalty
//! ≈ 2–4×10⁻⁶ per unit, a ~10⁵–10⁶ margin), so the minimised unserved is
//! unchanged and the oracle stays exact. The emitted result is asserted
//! to carry no simultaneous charge/discharge (physical tier).
//!
//! # Objective (2a — the feasibility oracle, D12 rule 2)
//!
//! Minimise total unserved (Σ over `z, t`, in energy) plus the tie-break
//! penalty. The priced/min-cost objective for thermal scenarios is a
//! later, separately-labelled use — NOT part of 2a.
//!
//! # Determinism (ADR-5)
//!
//! Variables and constraints are built in a fixed, documented order
//! (zone, then period, then tech/store/link index). HiGHS runs
//! single-threaded with parallelism off — its serial simplex is
//! deterministic, so `run_multi_lp` twice on the same input yields
//! identical results (pinned by test).
//!
//! # Scope
//!
//! The 2a core (the formulation above) plus the 2b/D13 additions that
//! now live in this file:
//!
//! - the tractability guards [`LP_RTE_FLOOR`] and [`LP_VARIABLE_CAP`]
//!   (`docs/notes/d12-lp-tractability.md`);
//! - the rolling-horizon window ([`run_multi_lp_rolling`]): overlapping
//!   window solves stitched over horizons a single LP cannot carry;
//! - the bisection wiring: [`run_multi_lp`] is the feasibility oracle
//!   of [`crate::solve::min_storage_for_zero_unserved_lp`];
//! - a second labelled objective, [`LpObjective::MinCurtailment`]
//!   ([`run_multi_lp_min_curtailment`]): the least-waste economic
//!   dispatch, whose waste channels — storage round-trip loss and the
//!   D13 loss-as-waste term on lossy links — are counted at weight 1 so
//!   they cannot masquerade as reduced curtailment.
//!
//! Still true: energy-budgeted fleet entries (the NO2 seasonal
//! reservoir) are rejected with a structured error — their
//! perfect-foresight treatment is a cumulative-window constraint that
//! has not been built — and no priced/min-COST objective exists
//! ([`LpObjective`] is `MinUnserved` and `MinCurtailment` only).

use good_lp::solvers::highs::highs;
use good_lp::{
    Expression, ProblemVariables, Solution, SolverModel, Variable, constraint, variable,
};

use std::collections::BTreeMap;

use grid_core::GridError;
use grid_core::scenario::{ExogenousReliability, LinkSpec, Reliability, Scenario, TechId, ZoneId};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, PerUnit, Power};

use crate::availability::AvailabilityModel;
use crate::flow::FLOW_MERIT_ORDER;
use crate::inputs::{ExogenousSupply, MultiZoneInputs, RunInputs, ZoneInputs};
use crate::multizone::{LinkFlowSeries, MultiZoneRunResult, ZoneRunResult};
use crate::policy::{StoreState, build_stores};
use crate::result::{LabelledSeries, RunResult, StoreSeries, TechSeries};

/// Tie-break penalty (per unit of grid-side storage throughput energy) on
/// the objective. Tiny relative to the unit coefficient on unserved, so
/// it removes degenerate simultaneous charge/discharge without perturbing
/// the minimised unserved (see the module docs).
const CYCLING_PENALTY: f64 = 1e-6;

/// Weight on unserved energy relative to curtailment in the
/// [`LpObjective::MinCurtailment`] objective. Large enough that the
/// least-waste dispatch never trades feasibility (unserved) for reduced
/// curtailment on any realistic fleet — for an adequate fleet the two are
/// jointly achievable (max delivery serves both), so this simply pins
/// unserved at its feasible minimum while curtailment is minimised.
const MIN_CURTAILMENT_UNSERVED_WEIGHT: f64 = 1.0e6;

/// The safe round-trip-efficiency floor for the perfect-foresight LP
/// (package 2b robustness guard, `docs/notes/d12-lp-tractability.md`).
/// A store with `round_trip_efficiency < LP_RTE_FLOOR` is rejected
/// before the LP is built (η ≥ 1e-3 is the accepted region). The floor
/// (1e-3) sits far below any real
/// store (η ∈ [0.3, 0.95]); the [`CYCLING_PENALTY`] soundness argument
/// (module docs) needs η well above 1e-6, and √η appears as a divisor in
/// the discharge coefficient (`dt/√η`), so a vanishing η would both blow
/// up that coefficient and dissolve the penalty's dominance margin.
pub const LP_RTE_FLOOR: f64 = 1e-3;

/// The perfect-foresight LP decision-variable cap (package 2b
/// robustness guard, `docs/notes/d12-lp-tractability.md`). HiGHS can
/// **abort the whole process** (an uncaught C++ `std::length_error`) on
/// an oversized LP — measured at the 10-year/3-zone point (~3.5 M
/// variables). A library crate must never abort, so the LP is rejected
/// before it is built when the estimated variable count exceeds this
/// cap.
///
/// The benchmark bracketed the abort: 5-year/3-zone (~1.75 M variables,
/// 3.25 GB) SOLVED; 10-year/3-zone (~3.5 M) ABORTED. The cap 2.5 M sits
/// safely between them — it admits the recommended binding-window slice
/// (3-year/3-zone ≈ 1.05 M, up to ~7-year/3-zone) while rejecting the
/// 10-year danger zone. The estimate counts the variables
/// [`run_multi_lp`] actually builds (see [`estimate_lp_variables`]).
pub const LP_VARIABLE_CAP: u64 = 2_500_000;

/// Estimate the number of decision variables the perfect-foresight LP
/// would build for `scenario` over `periods` half-hourly periods, using
/// the same accounting as [`run_multi_lp`]:
///
/// - one `gen` per dispatchable (non-weather-driven) fleet entry;
/// - three per store (`charge`, `discharge`, `soc`);
/// - two per live link (`fwd`, `rev`) — links go live only with more
///   than one zone;
/// - two per zone (`unserved`, `curtailment`);
///
/// all multiplied by `periods`. Energy-budgeted entries are still
/// dispatchable placeholders here (the LP rejects them later), so they
/// count as one `gen` — a conservative over-count for the guard. Pure
/// and deterministic; makes no allocation proportional to `periods`, so
/// it is safe to call as a pre-build size guard.
#[must_use]
pub fn estimate_lp_variables(scenario: &Scenario, periods: usize) -> u64 {
    let links_live = scenario.zones.len() > 1;
    let mut per_period: u64 = 0;
    for zone in &scenario.zones {
        for entry in &zone.fleet {
            // Weather-driven must-take carries no dispatch variable; every
            // other fleet entry is a dispatchable `gen`.
            if entry.capacity_factor_trace.is_none() {
                per_period += 1;
            }
        }
        per_period += 3 * zone.storage.len() as u64;
        per_period += 2; // unserved + curtailment
    }
    if links_live {
        per_period += 2 * scenario.links.len() as u64;
    }
    per_period.saturating_mul(periods as u64)
}

/// Reject a store whose round-trip efficiency is strictly below
/// [`LP_RTE_FLOOR`] (the RTE floor guard; η ≥ 1e-3 — the floor value
/// included — is the accepted region). Runs over the raw scenario
/// specs (before `build_stores`) so the error names the zone and the
/// store's output label.
fn check_rte_floor(scenario: &Scenario) -> Result<(), GridError> {
    for zone in &scenario.zones {
        for spec in &zone.storage {
            let eta = spec.round_trip_efficiency.value();
            if eta < LP_RTE_FLOOR {
                let repeated = zone.storage.iter().filter(|s| s.kind == spec.kind).count() > 1;
                let store = if repeated {
                    format!("{}_{}", spec.kind, spec.dispatch_order)
                } else {
                    spec.kind.as_str().to_owned()
                };
                return Err(GridError::StorageEfficiencyBelowFloor {
                    zone: zone.id.as_str().to_owned(),
                    store,
                    efficiency: eta,
                    floor: LP_RTE_FLOOR,
                });
            }
        }
    }
    Ok(())
}

/// A weather-driven must-take unit's fixed data (recorded pre-curtailment
/// in the result, exactly as the rule-based engine records renewables).
struct RenewableUnit {
    tech: TechId,
    reliability: Reliability,
    reliability_overridden: bool,
    /// Potential output `capacity × cf(t)` per period.
    output: Vec<Power>,
}

/// A dispatchable unit's fixed data: its merit position and per-period
/// ceiling `capacity × availability(t)`.
struct ThermalUnit {
    tech: TechId,
    ladder: usize,
    reliability: Reliability,
    reliability_overridden: bool,
    /// `capacity × availability(t)`, the upper bound on `gen[t]`.
    ceiling: Vec<Power>,
}

/// One zone's fixed LP data, extracted from the scenario and inputs.
struct ZoneData {
    id: ZoneId,
    demand: Vec<Power>,
    /// Total must-take per period (renewables + exogenous), the LP
    /// coefficient. Renewables and exogenous are also carried separately
    /// for the result series.
    must_take: Vec<Power>,
    renewables: Vec<RenewableUnit>,
    thermal: Vec<ThermalUnit>,
    exogenous: Vec<LabelledSeries>,
    stores: Vec<StoreState>,
}

/// One link's fixed LP data (live links only; inert single-zone links are
/// emitted as zero-flow series without variables).
struct LinkData {
    home: usize,
    away: usize,
    loss: f64,
    /// Forward (`home → away`) sending-end capability per period.
    fwd_cap: Vec<f64>,
    /// Reverse (`away → home`) sending-end capability per period.
    rev_cap: Vec<f64>,
}

/// The output label of a link (the scenario's `name`, or a derived
/// `<from>-<to>-<index>`) — mirrors the rule-based engine.
fn link_label(link: &LinkSpec, index: usize) -> String {
    link.name
        .clone()
        .unwrap_or_else(|| format!("{}-{}-{index}", link.from, link.to))
}

/// Which quantity the LP minimises (D12).
///
/// The two objectives answer DIFFERENT questions and must not be
/// conflated (the review's rule-2 correction):
/// - [`LpObjective::MinUnserved`] is the **feasibility oracle** (rule 2):
///   the bisection uses it to SIZE storage ("does a zero-unserved
///   dispatch exist?"). On an already-adequate fleet it is *indifferent*
///   to curtailment and flow, so its boundary flows are underdetermined —
///   do NOT read congestion off it.
/// - [`LpObjective::MinCurtailment`] is the **least-waste economic
///   dispatch**: it wheels surplus as far as the links physically allow,
///   so the curtailment that REMAINS is exactly what the transmission
///   restriction (e.g. the Scotland–England B4/B6 boundary) forces to be
///   spilled. Unserved is weighted to strongly dominate, so feasibility is
///   never traded away. It also counts waste channels at weight 1 so they
///   cannot masquerade as reduced curtailment: storage round-trip loss
///   (the D12 term) and, since D13, LINK LOSS on lossy links (`loss ×
///   sent`, both directions — skipped structurally at `loss == 0.0`).
///   This is the objective to read boundary flows/curtailment from — with
///   one caveat: beyond those waste weights it has NO link-flow tie-break
///   term, so in periods where it is indifferent (shifting spill, or
///   costless thermal backing, across a lossless link changes the
///   objective by nothing; shipping spill into a curtailing neighbour
///   over a lossy link nets exactly zero under the loss-as-waste term)
///   the flows are HiGHS-vertex-determined — deterministic per ADR-5, but
///   NOT model-determined. Binding statistics read off this objective
///   must be quoted as a band `[floor, point]`, the floor computed by
///   excluding binding periods in which a downstream zone is itself
///   curtailing (docs/notes/d12-mincurtailment-decision.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpObjective {
    /// Minimise total unserved energy (+ the storage cycling tie-break).
    MinUnserved,
    /// Minimise total curtailment, with unserved strongly dominating.
    MinCurtailment,
}

/// Run the perfect-foresight LP over the whole horizon as the
/// **feasibility oracle** (min unserved; package 2a). Pure function of
/// `(scenario, inputs)` — no wall-clock, no globals, no randomness
/// (ADR-5); HiGHS runs single-threaded and deterministic. The one-zone
/// case is handled as a special case of multi-zone (its links are inert).
/// See the module docs for the formulation and scope.
pub fn run_multi_lp(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
) -> Result<MultiZoneRunResult, GridError> {
    run_multi_lp_core(scenario, inputs, LpObjective::MinUnserved)
}

/// Run the perfect-foresight LP as a **least-waste economic dispatch**
/// (min curtailment, unserved strongly dominating). This is the objective
/// that MEASURES a transmission restriction: the LP wheels surplus as far
/// as the links allow, so the residual curtailment is the boundary's true
/// binding effect. Use this — not the min-unserved oracle — to read
/// boundary flows and curtailment on an already-adequate fleet (e.g. the
/// Scotland–England B4/B6 congestion re-measurement, D12 rule 4).
///
/// CAVEAT (flow degeneracy): the objective carries the waste terms
/// (storage round-trip loss; the D13 loss-as-waste term on lossy links)
/// but no link-flow tie-break, so in periods where it is indifferent the
/// returned flows are solver-vertex artifacts — deterministic per ADR-5,
/// but not model-determined. Quote binding statistics as a band
/// `[floor, point]`; the floor excludes binding periods with downstream
/// curtailment (see [`LpObjective`] and
/// docs/notes/d12-mincurtailment-decision.md).
pub fn run_multi_lp_min_curtailment(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
) -> Result<MultiZoneRunResult, GridError> {
    run_multi_lp_core(scenario, inputs, LpObjective::MinCurtailment)
}

fn run_multi_lp_core(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    objective_mode: LpObjective,
) -> Result<MultiZoneRunResult, GridError> {
    scenario.validate()?;

    if inputs.zones.len() != scenario.zones.len() {
        return Err(GridError::InvalidRunInputs {
            reason: format!(
                "{} zone inputs for {} scenario zones",
                inputs.zones.len(),
                scenario.zones.len()
            ),
        });
    }
    for (spec, zin) in scenario.zones.iter().zip(&inputs.zones) {
        if spec.id != zin.id {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "zone inputs out of order: scenario zone {} paired with inputs for {}",
                    spec.id, zin.id
                ),
            });
        }
    }

    let periods = inputs
        .zones
        .first()
        .map(|z| z.inputs.demand.len())
        .unwrap_or(0);
    if periods == 0 {
        return Err(GridError::InvalidRunInputs {
            reason: "an LP run needs at least one zone and one period".to_owned(),
        });
    }
    let start = inputs.zones[0].inputs.demand.start();
    let dt = Duration::half_hour();

    // Robustness guards (package 2b, docs/notes/d12-lp-tractability.md),
    // applied BEFORE any LP or per-period allocation is built:
    //  1. the RTE floor — reject a below-floor store (the cycling-penalty
    //     soundness argument fails as η → 0);
    //  2. the size cap — reject an oversized LP so HiGHS is never handed a
    //     problem large enough to abort the process.
    check_rte_floor(scenario)?;
    let estimated_variables = estimate_lp_variables(scenario, periods);
    if estimated_variables > LP_VARIABLE_CAP {
        return Err(GridError::LpProblemTooLarge {
            periods,
            zones: scenario.zones.len(),
            estimated_variables,
            cap: LP_VARIABLE_CAP,
        });
    }

    // Extract fixed per-zone data (must-take, ceilings, stores, ...).
    let mut zones: Vec<ZoneData> = Vec::with_capacity(scenario.zones.len());
    for (spec, zin) in scenario.zones.iter().zip(&inputs.zones) {
        zones.push(build_zone_data(spec, zin, periods, start)?);
    }

    // Links go live only with more than one zone (the GB reference
    // pattern: a single-zone scenario may name external counterparties
    // while its links stay inert — matches `run_multi`).
    let links_live = scenario.zones.len() > 1;
    let zone_index = |id: &ZoneId| -> Result<usize, GridError> {
        scenario
            .zones
            .iter()
            .position(|z| &z.id == id)
            .ok_or_else(|| GridError::InvalidScenario {
                reason: format!("link endpoint {id} is not a declared zone"),
            })
    };
    // Schema-v6 alignment (mirrors `run_multi`).
    if !inputs.link_capabilities.is_empty()
        && inputs.link_capabilities.len() != scenario.links.len()
    {
        return Err(GridError::InvalidRunInputs {
            reason: format!(
                "{} link capability inputs for {} scenario links",
                inputs.link_capabilities.len(),
                scenario.links.len()
            ),
        });
    }
    let mut links: Vec<Option<LinkData>> = Vec::with_capacity(scenario.links.len());
    for (index, link) in scenario.links.iter().enumerate() {
        if !links_live {
            links.push(None);
            continue;
        }
        links.push(Some(build_link_data(
            link,
            index,
            periods,
            inputs,
            &zone_index,
        )?));
    }

    // ------------------------------------------------------------------
    // Build the LP. Variable creation order is fixed for determinism:
    // for each zone (scenario order): thermal (merit order) × t, then
    // store × t (charge, discharge, soc); then live links × t (fwd, rev);
    // then per-zone unserved/curtailment × t.
    // ------------------------------------------------------------------
    let mut vars = ProblemVariables::new();

    // thermal_gen[z][thermal][t]
    let mut thermal_gen: Vec<Vec<Vec<Variable>>> = Vec::with_capacity(zones.len());
    // charge/discharge/soc[z][store][t]
    let mut charge: Vec<Vec<Vec<Variable>>> = Vec::with_capacity(zones.len());
    let mut discharge: Vec<Vec<Vec<Variable>>> = Vec::with_capacity(zones.len());
    let mut soc: Vec<Vec<Vec<Variable>>> = Vec::with_capacity(zones.len());
    for zd in &zones {
        let mut zgen = Vec::with_capacity(zd.thermal.len());
        for unit in &zd.thermal {
            let mut series = Vec::with_capacity(periods);
            for t in 0..periods {
                series.push(vars.add(variable().min(0.0).max(unit.ceiling[t].as_gigawatts())));
            }
            zgen.push(series);
        }
        thermal_gen.push(zgen);

        let mut zch = Vec::with_capacity(zd.stores.len());
        let mut zdis = Vec::with_capacity(zd.stores.len());
        let mut zsoc = Vec::with_capacity(zd.stores.len());
        for store in &zd.stores {
            let power = store.power.as_gigawatts();
            let energy = store.energy.as_gigawatt_hours();
            let mut ch = Vec::with_capacity(periods);
            let mut di = Vec::with_capacity(periods);
            let mut sc = Vec::with_capacity(periods);
            for _ in 0..periods {
                ch.push(vars.add(variable().min(0.0).max(power)));
                di.push(vars.add(variable().min(0.0).max(power)));
                sc.push(vars.add(variable().min(0.0).max(energy)));
            }
            zch.push(ch);
            zdis.push(di);
            zsoc.push(sc);
        }
        charge.push(zch);
        discharge.push(zdis);
        soc.push(zsoc);
    }

    // fwd/rev[link][t] (live links only; None otherwise).
    let mut fwd: Vec<Option<Vec<Variable>>> = Vec::with_capacity(links.len());
    let mut rev: Vec<Option<Vec<Variable>>> = Vec::with_capacity(links.len());
    for link in &links {
        match link {
            None => {
                fwd.push(None);
                rev.push(None);
            }
            Some(ld) => {
                let mut f = Vec::with_capacity(periods);
                let mut r = Vec::with_capacity(periods);
                for t in 0..periods {
                    f.push(vars.add(variable().min(0.0).max(ld.fwd_cap[t])));
                    r.push(vars.add(variable().min(0.0).max(ld.rev_cap[t])));
                }
                fwd.push(Some(f));
                rev.push(Some(r));
            }
        }
    }

    // unserved/curtailment[z][t]
    let mut unserved: Vec<Vec<Variable>> = Vec::with_capacity(zones.len());
    let mut curtailment: Vec<Vec<Variable>> = Vec::with_capacity(zones.len());
    for _ in &zones {
        let mut u = Vec::with_capacity(periods);
        let mut c = Vec::with_capacity(periods);
        for _ in 0..periods {
            u.push(vars.add(variable().min(0.0)));
            c.push(vars.add(variable().min(0.0)));
        }
        unserved.push(u);
        curtailment.push(c);
    }

    // Objective: minimise total unserved energy + tie-break penalty on
    // total storage throughput energy (both in GWh via ×dt). Under
    // `MinCurtailment` the unserved term is scaled up so it strongly
    // dominates, and a unit curtailment term is added — the least-waste
    // dispatch (the `MinUnserved` terms are byte-for-byte unchanged, so
    // the feasibility oracle and its pinned digests do not move).
    let dt_h = dt.as_hours();
    let mut objective = Expression::from(0.0);
    for (z, zd) in zones.iter().enumerate() {
        for &u in &unserved[z] {
            match objective_mode {
                LpObjective::MinUnserved => objective += dt_h * u,
                LpObjective::MinCurtailment => {
                    objective += (MIN_CURTAILMENT_UNSERVED_WEIGHT * dt_h) * u;
                }
            }
        }
        if objective_mode == LpObjective::MinCurtailment {
            for &c in &curtailment[z] {
                objective += dt_h * c;
            }
        }
        for (s, zstore) in zd.stores.iter().enumerate() {
            // Under MinCurtailment, count each store's round-trip loss as
            // waste (rate 1−η on charged energy). This makes dumping
            // surplus into storage losses cost EXACTLY as much as
            // curtailing it, so the LP never fakes a simultaneous
            // charge+discharge to hide spillage (the physical no-cycling
            // invariant then holds), while genuinely storing surplus for a
            // later deficit still nets a gain. MinUnserved adds nothing
            // here, so its objective stays byte-for-byte unchanged.
            if objective_mode == LpObjective::MinCurtailment {
                let loss_rate = 1.0 - zstore.sqrt_efficiency.value().powi(2);
                for &c in &charge[z][s] {
                    objective += (loss_rate * dt_h) * c;
                }
            }
            for (&c, &d) in charge[z][s].iter().zip(&discharge[z][s]) {
                objective += (CYCLING_PENALTY * dt_h) * c;
                objective += (CYCLING_PENALTY * dt_h) * d;
            }
        }
    }
    // Loss-as-waste (D13 rule 4, adjudication B(i)): under MinCurtailment,
    // energy burned in link losses joins the waste terms at WEIGHT 1
    // (`loss × sent`, both directions) — the exact analogue of the storage
    // round-trip-loss term above. Without it, shipping surplus into a
    // neighbour that is itself curtailing changed the objective by
    // −x·loss, so the LP STRICTLY PREFERRED burning surplus as link loss
    // (it even ran counterflow at cap in both directions purely to burn
    // it — the red fixture in tests/lp_loss_waste.rs). With the term the
    // disposal class becomes exactly INDIFFERENT — it joins the ordinary
    // degeneracy classes under the [floor, point] band discipline, it is
    // not eliminated — while genuine wheeling that serves load or
    // displaces thermal still nets a strict −x·(1−loss) gain. Conditions
    // (all four, per the adopted design): MinCurtailment only (the
    // MinUnserved oracle objective is byte-for-byte unchanged);
    // STRUCTURALLY skipped at `loss == 0.0` so the committed lossless
    // family's objective is byte-identical, not merely numerically equal.
    if objective_mode == LpObjective::MinCurtailment {
        for (li, link) in links.iter().enumerate() {
            let Some(ld) = link else { continue };
            if ld.loss == 0.0 {
                continue;
            }
            if let (Some(f), Some(r)) = (&fwd[li], &rev[li]) {
                for t in 0..periods {
                    objective += (ld.loss * dt_h) * f[t];
                    objective += (ld.loss * dt_h) * r[t];
                }
            }
        }
    }

    let mut model = vars.minimise(objective).using(highs);
    // Deterministic serial solve (ADR-5): single thread, no parallelism,
    // quiet. HiGHS's serial simplex is reproducible.
    model.set_verbose(false);
    model = model.set_threads(1);
    model = model.set_parallel(good_lp::solvers::highs::HighsParallelType::Off);

    // Constraints, in a fixed order (zone, then period).
    for z in 0..zones.len() {
        let zd = &zones[z];
        for t in 0..periods {
            // Zone energy balance.
            let mut supply = Expression::from(zd.must_take[t].as_gigawatts());
            for g in &thermal_gen[z] {
                supply += g[t];
            }
            for d in &discharge[z] {
                supply += d[t];
            }
            supply += unserved[z][t];
            // Link imports arrive at the receiving end as sent × (1−loss).
            // `fwd`/`rev` are `Some` exactly when the link is live.
            for (li, link) in links.iter().enumerate() {
                let Some(ld) = link else { continue };
                if ld.away == z
                    && let Some(f) = &fwd[li]
                {
                    supply += (1.0 - ld.loss) * f[t];
                } else if ld.home == z
                    && let Some(r) = &rev[li]
                {
                    supply += (1.0 - ld.loss) * r[t];
                }
            }

            let mut load = Expression::from(zd.demand[t].as_gigawatts());
            for c in &charge[z] {
                load += c[t];
            }
            load += curtailment[z][t];
            // Exports leave at the sending end (full power drawn locally).
            for (li, link) in links.iter().enumerate() {
                let Some(ld) = link else { continue };
                if ld.home == z
                    && let Some(f) = &fwd[li]
                {
                    load += f[t];
                } else if ld.away == z
                    && let Some(r) = &rev[li]
                {
                    load += r[t];
                }
            }

            model = model.with(constraint!(supply == load));
        }

        // Storage dynamics, matching StoreState::apply exactly.
        for (s, store) in zd.stores.iter().enumerate() {
            let sqrt_eta = store.sqrt_efficiency.value();
            let initial = store.soc.as_gigawatt_hours();
            for t in 0..periods {
                let gained = (dt_h * sqrt_eta) * charge[z][s][t];
                let drawn = (dt_h / sqrt_eta) * discharge[z][s][t];
                let prev: Expression = if t == 0 {
                    Expression::from(initial)
                } else {
                    soc[z][s][t - 1].into()
                };
                model = model.with(constraint!(soc[z][s][t] == prev + gained - drawn));
            }
        }
    }

    let solution = model.solve().map_err(|e| GridError::SolveInfeasible {
        reason: format!(
            "the perfect-foresight LP did not solve ({e}); the min-unserved \
             formulation is always feasible and bounded, so this indicates a \
             solver or formulation fault"
        ),
    })?;

    // ------------------------------------------------------------------
    // Extract the solution into a MultiZoneRunResult.
    // ------------------------------------------------------------------
    let val = |v: Variable| Power::gigawatts(clamp_dust(solution.value(v)));
    let val_energy = |v: Variable| Energy::gigawatt_hours(clamp_dust(solution.value(v)));

    // Link flow series (both ends), for every scenario link.
    let link_series: Vec<LinkFlowSeries> = scenario
        .links
        .iter()
        .enumerate()
        .map(|(li, link)| {
            // `fwd`/`rev` are `Some` exactly when the link is live.
            let (home_end, away_end) = match (&links[li], &fwd[li], &rev[li]) {
                (Some(ld), Some(f), Some(r)) => {
                    let mut home = Vec::with_capacity(periods);
                    let mut away = Vec::with_capacity(periods);
                    for t in 0..periods {
                        let fwd_v = clamp_dust(solution.value(f[t]));
                        let rev_v = clamp_dust(solution.value(r[t]));
                        // home end positive = into home: receives rev×(1−loss),
                        // sends fwd (export drawn locally, negative).
                        home.push(Power::gigawatts(rev_v * (1.0 - ld.loss) - fwd_v));
                        // away end positive = into away: receives fwd×(1−loss),
                        // sends rev.
                        away.push(Power::gigawatts(fwd_v * (1.0 - ld.loss) - rev_v));
                    }
                    (home, away)
                }
                // Inert (single-zone) link: zero flow at both ends.
                _ => (
                    vec![Power::gigawatts(0.0); periods],
                    vec![Power::gigawatts(0.0); periods],
                ),
            };
            LinkFlowSeries {
                name: link_label(link, li),
                from: link.from.clone(),
                to: link.to.clone(),
                home_end,
                away_end,
                // Package 2a does not emit the v6 capability columns (a
                // rule-based-flow output detail); None keeps the shape.
                capability: None,
            }
        })
        .collect();

    let mut zone_results = Vec::with_capacity(zones.len());
    for (z, zd) in zones.iter().enumerate() {
        // Exogenous series carried through, then link net positions
        // appended (imports-flagged, Variable reliability) exactly as the
        // rule-based engine folds them — so the RunResult conservation
        // identity and net-imports accounting work unchanged.
        let mut exogenous = zd.exogenous.clone();
        if links_live {
            for (link, series) in scenario.links.iter().zip(&link_series) {
                let end = if scenario.zones[z].id == link.from {
                    &series.home_end
                } else if scenario.zones[z].id == link.to {
                    &series.away_end
                } else {
                    continue;
                };
                exogenous.push(LabelledSeries {
                    label: series.name.clone(),
                    imports: true,
                    reliability: ExogenousReliability::Variable,
                    power: end.clone(),
                });
            }
        }

        let renewables = zd
            .renewables
            .iter()
            .map(|unit| TechSeries {
                tech: unit.tech.clone(),
                reliability: unit.reliability,
                reliability_overridden: unit.reliability_overridden,
                power: unit.output.clone(),
            })
            .collect();
        let thermal = zd
            .thermal
            .iter()
            .enumerate()
            .map(|(ti, unit)| TechSeries {
                tech: unit.tech.clone(),
                reliability: unit.reliability,
                reliability_overridden: unit.reliability_overridden,
                power: (0..periods).map(|t| val(thermal_gen[z][ti][t])).collect(),
            })
            .collect();
        let stores = zd
            .stores
            .iter()
            .enumerate()
            .map(|(si, store)| StoreSeries {
                label: store.label.clone(),
                kind: store.kind,
                charge: (0..periods).map(|t| val(charge[z][si][t])).collect(),
                discharge: (0..periods).map(|t| val(discharge[z][si][t])).collect(),
                soc: (0..periods).map(|t| val_energy(soc[z][si][t])).collect(),
            })
            .collect();

        let result = RunResult {
            start,
            demand: zd.demand.clone(),
            renewables,
            exogenous,
            thermal,
            stores,
            curtailment: (0..periods).map(|t| val(curtailment[z][t])).collect(),
            unserved: (0..periods).map(|t| val(unserved[z][t])).collect(),
        };
        zone_results.push(ZoneRunResult {
            id: zd.id.clone(),
            result,
        });
    }

    let result = MultiZoneRunResult {
        zones: zone_results,
        links: link_series,
    };

    // Physical-tier guard on the emitted result (D12 rule 1): per-period
    // energy conservation and no simultaneous charge/discharge. A no-op on
    // any correct solve; it turns a solver/formulation fault into a
    // structured error rather than a silently corrupt result.
    check_physical_invariants(&result)?;

    Ok(result)
}

/// Run the perfect-foresight LP over the horizon in **overlapping
/// rolling windows** (package 2b, `docs/notes/d12-perfect-foresight-lp.md`
/// rule 3). A single full-40-year multi-zone LP is not viable — HiGHS
/// crashes past ~5 years (`docs/notes/d12-lp-tractability.md`) — so the
/// horizon is solved in windows of `window_periods`, each carrying
/// storage state forward from the previous one.
///
/// Each window is one [`run_multi_lp`] solve over its own slice. Of the
/// window's `window_periods` decisions, only the first `commit_periods`
/// are committed to the output; the rest are the **overlap** — foresight
/// lookahead that lets the committed decisions plan against what is
/// coming (rule 3: the window must be long enough to see the binding
/// recharge). The next window starts `commit_periods` later, seeded with
/// the state of charge each store reached at the commit boundary. The
/// final window commits everything it sees.
///
/// Determinism (ADR-5) is preserved: every window is a pure LP solve of a
/// pure function of the sliced inputs, HiGHS is deterministic, and the
/// stitching is fixed-order.
///
/// A window that spans the whole horizon is a single solve and is
/// returned **bit-identical** to [`run_multi_lp`] (no windowing occurs).
///
/// Errors: [`GridError::InvalidRunInputs`] for `commit_periods == 0` or
/// `window_periods < commit_periods` or an empty horizon;
/// [`GridError::UnsupportedFeature`] for schema-v4 energy budgets or
/// schema-v6 per-period link-capability traces, which the multi-window
/// path does not yet re-window (a follow-on slice of package 2b); plus
/// any error [`run_multi_lp`] raises on a window.
pub fn run_multi_lp_rolling(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    window_periods: usize,
    commit_periods: usize,
) -> Result<MultiZoneRunResult, GridError> {
    if commit_periods == 0 || window_periods < commit_periods {
        return Err(GridError::InvalidRunInputs {
            reason: format!(
                "rolling-horizon LP needs 1 <= commit_periods <= window_periods \
                 (got window {window_periods}, commit {commit_periods})"
            ),
        });
    }
    let periods = inputs
        .zones
        .first()
        .map(|z| z.inputs.demand.len())
        .unwrap_or(0);
    if periods == 0 {
        return Err(GridError::InvalidRunInputs {
            reason: "a rolling LP run needs at least one zone and one period".to_owned(),
        });
    }

    // One window covers the whole horizon → exactly the whole-horizon LP,
    // with no slicing or scenario reconstruction, so it is bit-identical.
    if window_periods >= periods {
        return run_multi_lp(scenario, inputs);
    }

    // The multi-window path re-slices traces and reconstructs a per-window
    // scenario. Energy budgets (schema v4) and per-period link-capability
    // traces (schema v6) are not yet re-windowed — refuse them loudly
    // rather than silently mis-slicing (a follow-on slice of package 2b).
    if inputs.zones.iter().any(|z| !z.budgets.is_empty()) {
        return Err(GridError::UnsupportedFeature {
            feature: "energy_budget under the rolling-horizon LP (not yet re-windowed \
                      — package 2b follow-on)"
                .to_owned(),
        });
    }
    if inputs.link_capabilities.iter().any(Option::is_some) {
        return Err(GridError::UnsupportedFeature {
            feature: "per-period link-capability traces under the rolling-horizon LP \
                      (not yet re-windowed — package 2b follow-on)"
                .to_owned(),
        });
    }

    // Cross-zone trace alignment, BEFORE any slicing (mirrors the
    // per-zone horizon checks `run_multi_lp` makes in `build_zone_data`):
    // `periods` is computed from zone 0 alone, and the window loop below
    // slices every zone's demand / capacity-factor / exogenous traces, so
    // a shorter non-first-zone trace would index out of bounds. A library
    // crate must return a structured error, never panic.
    let start = inputs.zones[0].inputs.demand.start();
    for z in &inputs.zones {
        if z.inputs.demand.len() != periods || z.inputs.demand.start() != start {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "zone {}: demand trace does not cover the horizon \
                     ({} periods from {}; expected {periods} from {start})",
                    z.id,
                    z.inputs.demand.len(),
                    z.inputs.demand.start(),
                ),
            });
        }
        for (tech, trace) in &z.inputs.capacity_factors {
            if trace.len() != periods || trace.start() != start {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "zone {}: capacity-factor trace for {tech} does not cover \
                         the horizon ({} periods from {}; expected {periods} from {start})",
                        z.id,
                        trace.len(),
                        trace.start(),
                    ),
                });
            }
        }
        for supply in &z.inputs.exogenous {
            if supply.trace.len() != periods || supply.trace.start() != start {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "zone {}: exogenous supply {:?} does not cover the horizon \
                         ({} periods from {}; expected {periods} from {start})",
                        z.id,
                        supply.label,
                        supply.trace.len(),
                        supply.trace.start(),
                    ),
                });
            }
        }
    }
    let mut acc: Option<MultiZoneRunResult> = None;
    // Carried end-of-commit SoC, per zone, per result-store (the stores
    // are in `dispatch_order`-sorted order — the order `build_stores`
    // emits, mirrored in the result).
    let mut carried: Option<Vec<Vec<Energy>>> = None;
    let mut cursor = 0usize;

    while cursor < periods {
        let w = window_periods.min(periods - cursor);
        // Interior windows commit `commit_periods` and keep the rest as
        // overlap; the final window commits all it sees.
        let commit = if cursor + w >= periods {
            w
        } else {
            commit_periods.min(w)
        };

        let win_scenario = window_scenario(scenario, start, cursor, w, carried.as_ref())?;
        let win_inputs = slice_multi_inputs(inputs, cursor, w)?;
        let win_result = run_multi_lp(&win_scenario, &win_inputs)?;

        // Seed the next window with the SoC each store holds at the end of
        // the committed segment (period `commit - 1`).
        carried = Some(
            win_result
                .zones
                .iter()
                .map(|z| z.result.stores.iter().map(|s| s.soc[commit - 1]).collect())
                .collect(),
        );

        match &mut acc {
            None => {
                let mut first = win_result;
                truncate_result(&mut first, commit);
                acc = Some(first);
            }
            Some(a) => append_committed(a, &win_result, commit),
        }
        cursor += commit;
    }

    acc.ok_or_else(|| GridError::InvalidRunInputs {
        reason: "rolling LP produced no windows".to_owned(),
    })
}

/// Build a window's scenario: the same fleet/links/storage as the parent,
/// its horizon narrowed to `[from, from+len)`, and each store's
/// `initial_soc` overridden with the SoC carried from the previous
/// window's commit boundary (absent for the first window). SoC is stored
/// as a fraction of usable energy, clamped to `[0, 1]` against solver
/// dust (the SoC can never physically exceed the store's capacity).
fn window_scenario(
    scenario: &Scenario,
    start: UtcInstant,
    from: usize,
    len: usize,
    carried: Option<&Vec<Vec<Energy>>>,
) -> Result<Scenario, GridError> {
    let mut s = scenario.clone();
    let win_start = start.plus_periods(from as i64);
    let win_end = win_start.plus_periods(len as i64 - 1);
    s.horizon.start = win_start.to_string();
    s.horizon.end = win_end.to_string();

    if let Some(carried) = carried {
        for (zi, zone) in s.zones.iter_mut().enumerate() {
            // Result-store order = spec indices sorted by dispatch_order.
            let mut order: Vec<usize> = (0..zone.storage.len()).collect();
            order.sort_by_key(|&i| zone.storage[i].dispatch_order);
            for (k, &si) in order.iter().enumerate() {
                let energy = zone.storage[si].energy_gwh.as_gigawatt_hours();
                let frac = if energy > 0.0 {
                    (carried[zi][k].as_gigawatt_hours() / energy).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                zone.storage[si].initial_soc = Some(PerUnit::new(frac));
            }
        }
    }
    Ok(s)
}

/// Slice every zone's inputs to the window `[from, from+len)`. Link
/// capabilities are all absent on this path (rejected above), so they
/// pass through unchanged.
fn slice_multi_inputs(
    inputs: &MultiZoneInputs,
    from: usize,
    len: usize,
) -> Result<MultiZoneInputs, GridError> {
    let mut zones = Vec::with_capacity(inputs.zones.len());
    for z in &inputs.zones {
        zones.push(slice_zone_inputs(z, from, len)?);
    }
    Ok(MultiZoneInputs {
        zones,
        link_capabilities: inputs.link_capabilities.clone(),
    })
}

/// Slice one zone's inputs to `[from, from+len)`. Demand, capacity-factor
/// and exogenous traces are re-anchored to the window's start instant;
/// availability models are functions of the absolute instant, so they
/// carry unchanged. Heating is already folded into demand at load time,
/// so the LP dispatch never reads it (output-only) — dropped here.
fn slice_zone_inputs(zin: &ZoneInputs, from: usize, len: usize) -> Result<ZoneInputs, GridError> {
    let win_start = zin.inputs.demand.start().plus_periods(from as i64);
    let mut capacity_factors = BTreeMap::new();
    for (tech, trace) in &zin.inputs.capacity_factors {
        capacity_factors.insert(tech.clone(), slice_trace(trace, win_start, from, len)?);
    }
    let mut exogenous = Vec::with_capacity(zin.inputs.exogenous.len());
    for supply in &zin.inputs.exogenous {
        exogenous.push(ExogenousSupply {
            label: supply.label.clone(),
            imports: supply.imports,
            reliability: supply.reliability,
            trace: slice_trace(&supply.trace, win_start, from, len)?,
        });
    }
    Ok(ZoneInputs {
        // Window slices carry no pricing inputs: the LP never consults
        // the flow-signal SRMC chain (the priced ladder is a rule-based
        // flow-rule signal, not an LP term — D11 rule 3 / ADR-10).
        pricing: None,
        id: zin.id.clone(),
        inputs: RunInputs {
            demand: slice_trace(&zin.inputs.demand, win_start, from, len)?,
            capacity_factors,
            exogenous,
            availability: zin.inputs.availability.clone(),
            heating: None,
        },
        budgets: BTreeMap::new(),
    })
}

/// Re-anchor a trace's `[from, from+len)` slice to `start`.
fn slice_trace<U: Clone>(
    trace: &Trace<U>,
    start: UtcInstant,
    from: usize,
    len: usize,
) -> Result<Trace<U>, GridError> {
    Trace::from_parts(start, trace.values()[from..from + len].to_vec())
}

/// Truncate every per-period series of a window result to its first
/// `commit` periods (the committed segment of the first window).
fn truncate_result(result: &mut MultiZoneRunResult, commit: usize) {
    for z in &mut result.zones {
        let rr = &mut z.result;
        rr.demand.truncate(commit);
        for ts in &mut rr.renewables {
            ts.power.truncate(commit);
        }
        for ls in &mut rr.exogenous {
            ls.power.truncate(commit);
        }
        for ts in &mut rr.thermal {
            ts.power.truncate(commit);
        }
        for ss in &mut rr.stores {
            ss.charge.truncate(commit);
            ss.discharge.truncate(commit);
            ss.soc.truncate(commit);
        }
        rr.curtailment.truncate(commit);
        rr.unserved.truncate(commit);
    }
    for l in &mut result.links {
        l.home_end.truncate(commit);
        l.away_end.truncate(commit);
        if let Some(cap) = &mut l.capability {
            cap.forward.truncate(commit);
            cap.reverse.truncate(commit);
            cap.forward_observed.truncate(commit);
        }
    }
}

/// Append the first `commit` periods of `win` onto the accumulator. The
/// two results share structure (same zones, series, links, in the same
/// order), so the append is positional.
fn append_committed(acc: &mut MultiZoneRunResult, win: &MultiZoneRunResult, commit: usize) {
    for (za, zw) in acc.zones.iter_mut().zip(&win.zones) {
        let a = &mut za.result;
        let w = &zw.result;
        a.demand.extend_from_slice(&w.demand[..commit]);
        for (ta, tw) in a.renewables.iter_mut().zip(&w.renewables) {
            ta.power.extend_from_slice(&tw.power[..commit]);
        }
        for (la, lw) in a.exogenous.iter_mut().zip(&w.exogenous) {
            la.power.extend_from_slice(&lw.power[..commit]);
        }
        for (ta, tw) in a.thermal.iter_mut().zip(&w.thermal) {
            ta.power.extend_from_slice(&tw.power[..commit]);
        }
        for (sa, sw) in a.stores.iter_mut().zip(&w.stores) {
            sa.charge.extend_from_slice(&sw.charge[..commit]);
            sa.discharge.extend_from_slice(&sw.discharge[..commit]);
            sa.soc.extend_from_slice(&sw.soc[..commit]);
        }
        a.curtailment.extend_from_slice(&w.curtailment[..commit]);
        a.unserved.extend_from_slice(&w.unserved[..commit]);
    }
    for (la, lw) in acc.links.iter_mut().zip(&win.links) {
        la.home_end.extend_from_slice(&lw.home_end[..commit]);
        la.away_end.extend_from_slice(&lw.away_end[..commit]);
        if let (Some(ca), Some(cw)) = (&mut la.capability, &lw.capability) {
            ca.forward.extend_from_slice(&cw.forward[..commit]);
            ca.reverse.extend_from_slice(&cw.reverse[..commit]);
            ca.forward_observed
                .extend_from_slice(&cw.forward_observed[..commit]);
        }
    }
}

/// Snap HiGHS solution dust to clean zeros/values. The solver returns
/// values a few ULPs off exact bounds (e.g. `−3e-16`); snapping keeps
/// non-negativity and determinism-friendly output. The threshold is far
/// below any physically meaningful power/energy in these scenarios.
fn clamp_dust(v: f64) -> f64 {
    if v.abs() < 1e-9 { 0.0 } else { v }
}

/// Build one zone's fixed LP data. Mirrors the rule-based engine's
/// classifier (`multizone.rs::build_zone_engine`) for the FIXED parts:
/// weather-driven renewables as must-take, per-period thermal ceilings at
/// availability, exogenous must-take, and the storage portfolio.
/// Energy-budgeted entries are rejected (see the module docs — 2b scope).
fn build_zone_data(
    spec: &grid_core::scenario::ZoneSpec,
    zin: &ZoneInputs,
    periods: usize,
    start: grid_core::time::UtcInstant,
) -> Result<ZoneData, GridError> {
    if zin.inputs.demand.len() != periods || zin.inputs.demand.start() != start {
        return Err(GridError::InvalidRunInputs {
            reason: format!(
                "zone {}: demand trace does not cover the horizon \
                 ({} periods from {}; expected {periods} from {start})",
                spec.id,
                zin.inputs.demand.len(),
                zin.inputs.demand.start(),
            ),
        });
    }

    let mut renewables: Vec<RenewableUnit> = Vec::new();
    let mut thermal: Vec<ThermalUnit> = Vec::new();
    for entry in &spec.fleet {
        if entry.energy_budget.is_some() {
            return Err(GridError::UnsupportedFeature {
                feature: format!(
                    "the perfect-foresight LP core (package 2a) does not yet model \
                     energy-budgeted dispatch ({} in zone {}): its perfect-foresight \
                     treatment is a cumulative-window energy constraint scheduled for \
                     the 2b scaling work",
                    entry.technology, spec.id
                ),
            });
        }
        if entry.capacity_factor_trace.is_some() {
            let cf = zin
                .inputs
                .capacity_factors
                .get(&entry.technology)
                .ok_or_else(|| GridError::InvalidRunInputs {
                    reason: format!(
                        "zone {}: no capacity-factor trace loaded for weather-driven \
                         technology {}",
                        spec.id, entry.technology
                    ),
                })?;
            if cf.len() != periods || cf.start() != start {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "zone {}: capacity-factor trace for {} does not cover the horizon",
                        spec.id, entry.technology
                    ),
                });
            }
            let output = cf.values().iter().map(|&f| entry.capacity_gw * f).collect();
            renewables.push(RenewableUnit {
                tech: entry.technology.clone(),
                reliability: entry.effective_reliability(),
                reliability_overridden: entry.reliability_overridden(),
                output,
            });
        } else {
            let ladder = FLOW_MERIT_ORDER
                .iter()
                .position(|t| *t == entry.technology.as_str())
                .ok_or_else(|| GridError::UnknownThermalTechnology {
                    tech: entry.technology.as_str().to_owned(),
                })?;
            let availability = zin
                .inputs
                .availability
                .get(&entry.technology)
                .cloned()
                .unwrap_or(AvailabilityModel::flat(PerUnit::new(1.0))?);
            let ceiling = (0..periods)
                .map(|t| {
                    let instant = start.plus_periods(t as i64);
                    entry.capacity_gw * availability.factor_at(instant)
                })
                .collect();
            thermal.push(ThermalUnit {
                tech: entry.technology.clone(),
                ladder,
                reliability: entry.effective_reliability(),
                reliability_overridden: entry.reliability_overridden(),
                ceiling,
            });
        }
    }
    thermal.sort_by_key(|unit| unit.ladder);
    for pair in thermal.windows(2) {
        if pair[0].ladder == pair[1].ladder {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "zone {}: two dispatchable entries share merit position ({})",
                    spec.id, pair[0].tech
                ),
            });
        }
    }

    // Exogenous must-take, aligned to the horizon.
    let mut exogenous: Vec<LabelledSeries> = Vec::with_capacity(zin.inputs.exogenous.len());
    for supply in &zin.inputs.exogenous {
        if supply.trace.len() != periods || supply.trace.start() != start {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "zone {}: exogenous supply {:?} does not cover the horizon",
                    spec.id, supply.label
                ),
            });
        }
        exogenous.push(LabelledSeries {
            label: supply.label.clone(),
            imports: supply.imports,
            reliability: supply.reliability,
            power: supply.trace.values().to_vec(),
        });
    }

    // Must-take total per period: renewables (potential) + exogenous.
    let mut must_take = vec![Power::gigawatts(0.0); periods];
    for unit in &renewables {
        for (acc, &p) in must_take.iter_mut().zip(&unit.output) {
            *acc = *acc + p;
        }
    }
    for supply in &exogenous {
        for (acc, &p) in must_take.iter_mut().zip(&supply.power) {
            *acc = *acc + p;
        }
    }

    let stores = build_stores(spec)?;

    Ok(ZoneData {
        id: spec.id.clone(),
        demand: zin.inputs.demand.values().to_vec(),
        must_take,
        renewables,
        thermal,
        exogenous,
        stores,
    })
}

/// Build one live link's directional per-period capabilities, replicating
/// the rule-based engine's `LinkState` derivation (`multizone.rs`):
/// forward = `capability_trace × availability` when declared, else
/// `capacity_gw × availability`; reverse = `reverse_capacity_gw`
/// (default `capacity_gw`) `× availability`.
fn build_link_data(
    link: &LinkSpec,
    index: usize,
    periods: usize,
    inputs: &MultiZoneInputs,
    zone_index: &impl Fn(&ZoneId) -> Result<usize, GridError>,
) -> Result<LinkData, GridError> {
    let availability = link.availability.value();
    let fwd_flat = link.capacity_gw.as_gigawatts() * availability;
    let rev_flat = link
        .reverse_capacity_gw
        .unwrap_or(link.capacity_gw)
        .as_gigawatts()
        * availability;

    let fwd_cap: Vec<f64> = match &link.capability_trace {
        None => vec![fwd_flat; periods],
        Some(_) => {
            let capability = inputs
                .link_capabilities
                .get(index)
                .and_then(|c| c.as_ref())
                .ok_or_else(|| GridError::InvalidRunInputs {
                    reason: format!(
                        "link {} declares a capability_trace but no capability inputs \
                         were loaded for it",
                        link_label(link, index)
                    ),
                })?;
            if capability.forward.len() != periods {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "link {}: capability covers {} periods; the horizon has {periods}",
                        link_label(link, index),
                        capability.forward.len()
                    ),
                });
            }
            capability
                .forward
                .iter()
                .map(|p| p.as_gigawatts() * availability)
                .collect()
        }
    };

    Ok(LinkData {
        home: zone_index(&link.from)?,
        away: zone_index(&link.to)?,
        loss: link.loss.value(),
        fwd_cap,
        rev_cap: vec![rev_flat; periods],
    })
}

/// The physical-tier guard on an emitted LP result (D12 rule 1): the
/// per-period energy-conservation identity (over the RunResult's folded
/// series) and no simultaneous charge/discharge. A no-op on any correct
/// solve; a structured error otherwise.
fn check_physical_invariants(result: &MultiZoneRunResult) -> Result<(), GridError> {
    for zr in &result.zones {
        let r = &zr.result;
        for t in 0..r.periods() {
            let zero = Power::gigawatts(0.0);
            let supply = r
                .renewables
                .iter()
                .chain(&r.thermal)
                .fold(zero, |acc, s| acc + s.power[t])
                + r.exogenous.iter().fold(zero, |acc, s| acc + s.power[t])
                + r.stores.iter().fold(zero, |acc, s| acc + s.discharge[t]);
            let load = r.demand[t]
                + r.stores.iter().fold(zero, |acc, s| acc + s.charge[t])
                + r.curtailment[t];
            let supply = supply + r.unserved[t];
            let scale = r.demand[t].as_gigawatts().abs().max(1.0);
            if (supply - load).as_gigawatts().abs() > 1e-6 * scale {
                return Err(GridError::SolveInfeasible {
                    reason: format!(
                        "zone {} period {t}: LP result violates energy conservation — \
                         supply {} GW != load {} GW",
                        zr.id,
                        supply.as_gigawatts(),
                        load.as_gigawatts()
                    ),
                });
            }
            for s in &r.stores {
                if s.charge[t].as_gigawatts() > 1e-9 && s.discharge[t].as_gigawatts() > 1e-9 {
                    return Err(GridError::SolveInfeasible {
                        reason: format!(
                            "zone {} period {t}: LP result has store {} charging and \
                             discharging simultaneously",
                            zr.id, s.label
                        ),
                    });
                }
            }
        }
    }
    Ok(())
}
