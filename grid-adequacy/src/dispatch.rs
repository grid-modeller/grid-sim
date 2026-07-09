//! Chronological half-hourly dispatch: merit-order thermal stack
//! (docs/04 Stage 1) plus the Stage 3 storage portfolio under a
//! pluggable dispatch policy (ADR-6).
//!
//! # The dispatch rules, in prose
//!
//! Each half-hourly settlement period, in order (the storage rules are
//! D4's — the full normative prose lives in [`crate::policy`], quoting
//! `docs/notes/d4-rule-based-dispatch.md`):
//!
//! 1. **Must-take supply** is taken in full, never dispatched down:
//!    every weather-driven technology (a fleet entry with a
//!    `capacity_factor_trace`) contributes `capacity × cf(t)`, and every
//!    exogenous supply series contributes its trace value, which may be
//!    negative (exports, pumping load). `net = must_take − demand`;
//!    `net = 0` → no storage action.
//! 2. If `net > 0` the system is in **surplus**: no thermal plant runs,
//!    and the policy charges stores from the surplus (ascending
//!    `dispatch_order`, each up to its power rating and headroom).
//!    Surplus no store absorbs is **curtailment** — reported pooled,
//!    without attribution to individual sources (the renewable series
//!    report *potential* output; per-increment attribution is Q2).
//! 3. If `net < 0` the deficit goes to the **non-storage dispatchable
//!    stack first**, in the fixed merit order [`MERIT_ORDER`], each
//!    technology up to `capacity × availability(t)` — no unit
//!    commitment, no ramp limits, no minimum stable generation.
//! 4. Deficit remaining after the full stack is offered to storage
//!    **discharge** (ascending `dispatch_order`, up to power and SoC).
//!    Storage never runs before the stack (D4 rule 3: the reliability
//!    backstop, feasibility-optimal for adequacy) and never charges
//!    from it (D4 rule 2: surplus-only charging).
//! 5. Whatever remains is **unserved energy**.
//!
//! The engine *validates* every policy decision rather than trusting it
//! — a buggy or adversarial policy yields
//! [`GridError::InvalidDispatchDecision`], never silent corruption — in
//! two tiers (D12 rule 1; see [`crate::policy::PolicyContract`]):
//!
//! - **Physical invariants**, enforced for EVERY policy: per-store
//!   non-negativity, no simultaneous charge/discharge, `charge ≤
//!   max_charge` / `discharge ≤ max_discharge`, SoC bounds, non-negative
//!   derived curtailment/unserved (no conjured energy, no masked
//!   unserved), and per-period energy conservation. These are laws.
//! - **Rule-based policy CHOICES**, enforced only when the policy's
//!   [`crate::policy::PolicyContract`] declares them: surplus-only
//!   charging (`total_charge ≤ surplus`) and discharge-after-stack
//!   (no discharge in surplus; `discharge ≤` post-stack deficit).
//!   [`RuleBased`] declares both; a foresight policy may relax them and
//!   pre-charge from the stack, so the stack then serves `deficit +
//!   charge` and discharge is a post-stack backstop supply.
//!
//! Store SoC accounting uses the symmetric √η split (a D4 *convention*,
//! not a physical law; shared mechanics — see
//! [`crate::policy::StoreState`]).
//!
//! # Why a fixed merit order and not SRMC (Stage 1 decision, still in force)
//!
//! docs/03's `Technology.srmc` exists in the domain model, and Stage 2
//! added the SRMC *pricing* layer — but dispatch ordering deliberately
//! remains this fixed, documented table: the Stage 1 honesty gate was
//! passed under it and Stage 2 froze dispatch (its pinned gate numbers
//! are downstream of this order). 2024 GB outturn is insensitive to
//! fine ordering *within* the thermal stack, because gas is the
//! marginal fuel in ≈ 99 % of periods (the outturn proxy, 99.4 % on the
//! 5–95 band — validation pack report §5; the model-side analogue, the
//! gas price-setting share, is acceptance-banded in
//! `tests/acceptance_stage2_2024.rs` and regression-pinned at 93.89 %
//! in `grid-cli/tests/regression_stage2_2024.rs`), so
//! the only orderings that matter are "everything cheap before gas"
//! and "CCGT before OCGT":
//!
//! 1. `nuclear` — first merit rung (lowest SRMC, least flexible); NOT
//!    must-take: it backs down when residual demand is below its
//!    ceiling (D4 erratum 2026-07-06, policy.rs rule 1);
//! 2. `biomass` — CfD/ROC-supported, low effective marginal cost;
//! 3. `hydro` — run-of-river/reservoir, near-zero marginal cost;
//! 4. `coal` — placed before gas as a **calibration expedient**, not an
//!    SRMC claim: with its calibrated 2024 availability window it then
//!    reproduces the observed residual-baseload running of Ratcliffe's
//!    final months. (At 2024 carbon prices coal's true SRMC often
//!    exceeded CCGT's; ordering it after gas would instead dispatch it
//!    only when CCGT is exhausted, i.e. almost never, understating coal
//!    by its full 1.57 TWh — validation pack report §2.)
//! 5. `ccgt` — the marginal technology nearly everywhere;
//! 6. `ocgt` — peaking, highest SRMC, last resort before storage
//!    discharge and unserved energy.
//!
//! # Non-goals, enforced here
//!
//! - **Links are inert**: imports are exogenous; the `[[links]]` matrix
//!   is not consulted until Stage 5.
//! - **Multi-zone scenarios are rejected** with a clear error (ADR-7).
//! - Only the `rule_based` dispatch policy is accepted by [`run`]
//!   (`perfect_foresight` is not routed through `dispatch.policy`; the
//!   perfect-foresight LP runs via [`crate::run_multi_lp`] —
//!   D12/Stage 7); [`run_with_policy`] takes any [`DispatchPolicy`]
//!   implementation.
//! - **DSR stores are rejected** (schema shape only; Q6 work — see
//!   [`crate::policy`]).

use grid_core::GridError;
use grid_core::scenario::{DispatchPolicyKind, Scenario};
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, PerUnit, Power};

use crate::availability::AvailabilityModel;
use crate::inputs::{RunInputs, single_zone};
use crate::policy::{DispatchPolicy, RuleBased, StoreState, SystemState, build_stores};
use crate::result::{LabelledSeries, RunResult, StoreSeries, TechSeries};

/// The fixed thermal merit order, cheapest first (see the module docs
/// for the full justification of each position).
///
/// # Stage 7 extension (published-pathway scenarios, 2026-07-06)
///
/// The Stage 1 six-rung 2024 stack (`nuclear`, `biomass`, `hydro`,
/// `coal`, `ccgt`, `ocgt`) is extended with the technologies the FES
/// 2025 Electric Engagement and CCC CB7 Balanced Pathway fleets carry
/// and the 2024 fleet does not. This extended ladder orders the
/// SINGLE-ZONE dispatch path only, where only relative order matters —
/// the single-zone digest 779d7444… is unmoved because the six Stage 1
/// rungs keep their relative order and no committed scenario names a
/// new rung. The MULTI-ZONE engines (run_multi, the LP) deliberately
/// stay on the frozen six-rung [`crate::flow::FLOW_MERIT_ORDER`]: the
/// scarcity signal there is NUMERICALLY index-based, so extending it
/// would move the committed 2/3/5/8-zone digests (measured during this
/// build) — a multi-zone scenario naming an extended-only rung is
/// rejected until that signal convention is knowingly re-pinned
/// (multi-zone pathway variants are post-beta).
/// Placements, in prose (the docs/06 obligation; note that ADEQUACY
/// outcomes — unserved energy, curtailment — are ordering-invariant:
/// only the per-technology energy split depends on these choices):
///
/// - `beccs` — directly after unabated `biomass`: BECCS is
///   negative-emissions-credited steam plant, run as baseload where it
///   exists; placing it adjacent to biomass keeps the two steam classes
///   together (the CCC carries BECCS as its own bucket; FES folds it —
///   pathway scenarios that split the fold use this rung).
/// - `waste` — after the biomass pair: energy-from-waste is
///   gate-fee-funded near-must-run plant with effectively negative fuel
///   cost; in 2024 it lived inside the exogenous FUELHH "other" trace,
///   for pathway fleets it is an explicit rung.
/// - `other_generation` — the CCC Figure 7.5.3 residual bucket
///   ("unabated biomass, energy from waste, hydro, and CHP" — note 3):
///   a mixed low-marginal-cost class, placed with its components
///   (after waste, before hydro). Open-set id, never costed or priced.
/// - `hydro`, `coal` — unchanged Stage 1 rungs (see above).
/// - `ccgt_ccs` — abated gas, after coal and BEFORE unabated `ccgt`:
///   a decarbonisation pathway builds CCS gas to displace unabated gas
///   burn, and its effective marginal cost sits below unabated CCGT's
///   once carbon is priced (the fuel-efficiency penalty is smaller than
///   the avoided carbon cost at pathway-era carbon prices). No SRMC
///   recipe exists for it (prices-reference-v1 has no CCS chain), so
///   its fuel cost is a named cost-stack exclusion, not a priced line.
/// - `low_carbon_dispatchable` — the CCC's published aggregate ("gas
///   CCS and hydrogen", split not published): same slot logic as
///   `ccgt_ccs`, immediately after it. Open-set id carried so the CCC
///   bucket dispatches WITHOUT an invented component split.
/// - `ccgt`, `ocgt` — unchanged Stage 1 rungs.
/// - `oil` — after `ocgt`: distillate peakers price above gas peakers;
///   reserve plant of last resort in both pathway fleets (≤ 0.25 GW).
/// - `hydrogen_turbine` — LAST rung before storage discharge and
///   unserved energy: electrolytic hydrogen is the most expensive fuel
///   chain in either pathway, and FES runs the H₂ turbine fleet as the
///   deep-backup strategic reserve. Engine v1 has no hydrogen
///   fuel-chain cost or supply constraint (fes-pathway exclusion e3):
///   the rung dispatches as unlimited-fuel firm plant, a named,
///   pathway-favourable limitation on every consuming artefact.
pub const MERIT_ORDER: [&str; 13] = [
    "nuclear",
    "biomass",
    "beccs",
    "waste",
    "other_generation",
    "hydro",
    "coal",
    "ccgt_ccs",
    "low_carbon_dispatchable",
    "ccgt",
    "ocgt",
    "oil",
    "hydrogen_turbine",
];

/// Per-technology dispatch state for the thermal stack.
struct ThermalUnit<'a> {
    tech: &'a grid_core::scenario::TechId,
    capacity: Power,
    availability: AvailabilityModel,
    reliability: grid_core::scenario::Reliability,
    reliability_overridden: bool,
    output: Vec<Power>,
}

/// A weather-driven must-take technology.
struct RenewableUnit<'a> {
    tech: &'a grid_core::scenario::TechId,
    capacity: Power,
    cf: &'a Trace<PerUnit>,
    reliability: grid_core::scenario::Reliability,
    reliability_overridden: bool,
    output: Vec<Power>,
}

/// Per-store output accumulators.
struct StoreRecorder {
    charge: Vec<Power>,
    discharge: Vec<Power>,
    soc: Vec<Energy>,
}

/// Run the chronological dispatch over the scenario horizon under the
/// scenario's declared policy (`rule_based` only — `perfect_foresight`
/// is not routed through `dispatch.policy`; the perfect-foresight LP
/// runs via [`crate::run_multi_lp`], D12/Stage 7).
///
/// The engine is a pure function of `(scenario, inputs)` (ADR-5): no
/// wall-clock, no globals, no randomness. See the module docs for the
/// dispatch rules.
///
/// Errors: [`GridError::MultiZoneUnsupported`],
/// [`GridError::UnsupportedFeature`] (non-rule-based policy; DSR
/// stores), [`GridError::UnknownThermalTechnology`],
/// [`GridError::DuplicateDispatchOrder`] and
/// [`GridError::InvalidScenario`] from validation, and
/// [`GridError::InvalidRunInputs`] for missing or misaligned traces.
pub fn run(scenario: &Scenario, inputs: &RunInputs) -> Result<RunResult, GridError> {
    if scenario.dispatch.policy != DispatchPolicyKind::RuleBased {
        return Err(GridError::UnsupportedFeature {
            feature: format!(
                "the {} dispatch policy (not routed through dispatch.policy — this engine \
                 implements rule_based; the perfect-foresight LP runs via run_multi_lp, D12)",
                scenario.dispatch.policy
            ),
        });
    }
    run_with_policy(scenario, inputs, &RuleBased)
}

/// [`run`] with an explicit storage dispatch policy (ADR-6: pluggable).
/// The scenario's `dispatch.policy` field is not consulted — the caller
/// chooses the implementation.
pub fn run_with_policy(
    scenario: &Scenario,
    inputs: &RunInputs,
    policy: &dyn DispatchPolicy,
) -> Result<RunResult, GridError> {
    scenario.validate()?;
    let zone = single_zone(scenario)?;

    // The frozen single-zone path does not implement the schema-v4
    // energy budget; refuse it loudly rather than silently ignoring the
    // constraint (budgeted dispatch runs under the Stage 5 multi-zone
    // engine, which handles one-zone scenarios identically otherwise).
    if let Some(entry) = zone.fleet.iter().find(|e| e.energy_budget.is_some()) {
        return Err(GridError::UnsupportedFeature {
            feature: format!(
                "energy_budget on {} in the single-zone run path (budgeted dispatch is \
                 implemented by grid_adequacy::run_multi — Stage 5)",
                entry.technology
            ),
        });
    }

    let periods = inputs.demand.len();
    let start = inputs.demand.start();
    let dt = Duration::half_hour();
    let zero = Power::gigawatts(0.0);

    // Classify the fleet. Scenario order is preserved for renewables;
    // thermal units are dispatched in MERIT_ORDER position order.
    let mut renewables: Vec<RenewableUnit> = Vec::new();
    let mut thermal: Vec<(usize, ThermalUnit)> = Vec::new();
    for entry in &zone.fleet {
        if entry.capacity_factor_trace.is_some() {
            let cf = inputs
                .capacity_factors
                .get(&entry.technology)
                .ok_or_else(|| GridError::InvalidRunInputs {
                    reason: format!(
                        "no capacity-factor trace loaded for weather-driven technology {}",
                        entry.technology
                    ),
                })?;
            if cf.len() != periods || cf.start() != start {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "capacity-factor trace for {} does not cover the horizon \
                         ({} periods from {}; expected {periods} from {start})",
                        entry.technology,
                        cf.len(),
                        cf.start(),
                    ),
                });
            }
            renewables.push(RenewableUnit {
                tech: &entry.technology,
                capacity: entry.capacity_gw,
                cf,
                reliability: entry.effective_reliability(),
                reliability_overridden: entry.reliability_overridden(),
                output: Vec::with_capacity(periods),
            });
        } else {
            let position = MERIT_ORDER
                .iter()
                .position(|t| *t == entry.technology.as_str())
                .ok_or_else(|| GridError::UnknownThermalTechnology {
                    tech: entry.technology.as_str().to_owned(),
                })?;
            let availability = inputs
                .availability
                .get(&entry.technology)
                .cloned()
                .unwrap_or(
                    // Infallible: 1.0 is in range.
                    AvailabilityModel::flat(PerUnit::new(1.0))?,
                );
            thermal.push((
                position,
                ThermalUnit {
                    tech: &entry.technology,
                    capacity: entry.capacity_gw,
                    availability,
                    reliability: entry.effective_reliability(),
                    reliability_overridden: entry.reliability_overridden(),
                    output: vec![Power::gigawatts(0.0); periods],
                },
            ));
        }
    }
    thermal.sort_by_key(|(position, _)| *position);
    let mut thermal: Vec<ThermalUnit> = thermal.into_iter().map(|(_, unit)| unit).collect();

    for supply in &inputs.exogenous {
        if supply.trace.len() != periods || supply.trace.start() != start {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "exogenous supply {:?} does not cover the horizon \
                     ({} periods from {}; expected {periods} from {start})",
                    supply.label,
                    supply.trace.len(),
                    supply.trace.start(),
                ),
            });
        }
    }

    // Storage portfolio, ascending dispatch_order (D4; DSR rejected).
    let mut stores: Vec<StoreState> = build_stores(zone)?;
    let mut recorders: Vec<StoreRecorder> = stores
        .iter()
        .map(|_| StoreRecorder {
            charge: Vec::with_capacity(periods),
            discharge: Vec::with_capacity(periods),
            soc: Vec::with_capacity(periods),
        })
        .collect();

    let mut curtailment = Vec::with_capacity(periods);
    let mut unserved = Vec::with_capacity(periods);

    for t in 0..periods {
        let instant = start.plus_periods(t as i64);
        let demand = inputs.demand.values()[t];

        // Rule 1: must-take supply.
        let mut must_take = Power::gigawatts(0.0);
        for unit in &mut renewables {
            let output = unit.capacity * unit.cf.values()[t];
            unit.output.push(output);
            must_take = must_take + output;
        }
        for supply in &inputs.exogenous {
            must_take = must_take + supply.trace.values()[t];
        }

        // What the stack could serve this period (for the policy's
        // rule-3 view; the actual stack dispatch below).
        let mut stack_available = Power::gigawatts(0.0);
        let mut ceilings = Vec::with_capacity(thermal.len());
        for unit in &thermal {
            let ceiling = unit.capacity * unit.availability.factor_at(instant);
            stack_available = stack_available + ceiling;
            ceilings.push(ceiling);
        }

        // The policy decides storage actions from the current period
        // only (the SystemState no-future invariant, D4).
        let decision = policy.dispatch(
            &SystemState {
                instant,
                demand,
                must_take,
                stack_available,
                stores: &stores,
            },
            &scenario.horizon,
        );
        if decision.actions.len() != stores.len() {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "period {t}: {} actions for {} stores",
                    decision.actions.len(),
                    stores.len()
                ),
            });
        }

        let net = must_take - demand;
        let surplus = if net > zero { net } else { zero };
        let deficit = if net < zero { -net } else { zero };

        // The policy's contract selects the policy-tier checks below
        // (D12 rule 1). The physical tier runs unconditionally.
        let contract = policy.contract();

        // PHYSICAL TIER (every policy): per-store ratings/SoC limits and
        // non-negativity / no-simultaneous-charge-discharge (module docs).
        let tolerance = |scale: Power| 1e-9 * scale.as_gigawatts().abs().max(1.0);
        let mut total_charge = zero;
        let mut total_discharge = zero;
        for (store, action) in stores.iter().zip(&decision.actions) {
            let infeasible = |reason: String| GridError::InvalidDispatchDecision {
                reason: format!("period {t}, store {}: {reason}", store.label),
            };
            if action.charge < zero || action.discharge < zero {
                return Err(infeasible("negative charge or discharge".to_owned()));
            }
            if action.charge > zero && action.discharge > zero {
                return Err(infeasible("simultaneous charge and discharge".to_owned()));
            }
            let max_charge = store.max_charge(dt);
            if (action.charge - max_charge).as_gigawatts() > tolerance(max_charge) {
                return Err(infeasible(format!(
                    "charge {} GW exceeds the feasible {} GW",
                    action.charge.as_gigawatts(),
                    max_charge.as_gigawatts()
                )));
            }
            let max_discharge = store.max_discharge(dt);
            if (action.discharge - max_discharge).as_gigawatts() > tolerance(max_discharge) {
                return Err(infeasible(format!(
                    "discharge {} GW exceeds the feasible {} GW",
                    action.discharge.as_gigawatts(),
                    max_discharge.as_gigawatts()
                )));
            }
            total_charge = total_charge + action.charge;
            total_discharge = total_discharge + action.discharge;
        }
        // POLICY TIER — surplus-only charging (D4 rule 2). Enforced only
        // for a policy whose contract declares it; a foresight policy may
        // charge from the stack instead.
        if contract.charge_from_surplus_only
            && (total_charge - surplus).as_gigawatts() > tolerance(surplus)
        {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "period {t}: total charge {} GW exceeds the surplus {} GW \
                     (charging draws from surplus only — D4 rule 2)",
                    total_charge.as_gigawatts(),
                    surplus.as_gigawatts()
                ),
            });
        }

        let (period_curtailment, period_unserved) = if net > zero {
            // POLICY TIER — no discharge in a surplus period (D4 rule 3:
            // discharge serves the post-stack deficit only, which is zero
            // here). Enforced only under the contract; without it a policy
            // may discharge in surplus (accounted as extra curtailment).
            if contract.discharge_after_stack_only
                && (total_discharge - zero).as_gigawatts() > tolerance(zero)
            {
                return Err(GridError::InvalidDispatchDecision {
                    reason: format!(
                        "period {t}: discharge {} GW during a surplus period — no post-stack \
                         deficit exists (storage discharges after the full stack — D4 rule 3)",
                        total_discharge.as_gigawatts()
                    ),
                });
            }

            // Rule 2: surplus → storage charge; anything a store did not
            // absorb (plus any discharge, which a relaxed policy may add)
            // is curtailment. For RuleBased total_discharge == 0 here, so
            // `(surplus − total_charge) + 0.0` is bit-identical to the old
            // `surplus − total_charge`.
            ((surplus - total_charge) + total_discharge, zero)
        } else {
            // Rule 3: the thermal stack runs first, in merit order,
            // clamped to capacity × availability(t). It serves the deficit
            // PLUS any charging load a relaxed policy imposes (charge is
            // extra load). For RuleBased total_charge == 0 in a non-surplus
            // period, so `deficit + 0.0` is bit-identical to `deficit`.
            let mut remaining = deficit + total_charge;
            for (unit, &ceiling) in thermal.iter_mut().zip(&ceilings) {
                let output = if remaining < ceiling {
                    remaining
                } else {
                    ceiling
                };
                unit.output[t] = output;
                remaining = remaining - output;
            }

            // POLICY TIER — discharge ≤ post-stack deficit (D4 rule 3).
            // Enforced only under the contract.
            if contract.discharge_after_stack_only
                && (total_discharge - remaining).as_gigawatts() > tolerance(remaining)
            {
                return Err(GridError::InvalidDispatchDecision {
                    reason: format!(
                        "period {t}: total discharge {} GW exceeds the post-stack deficit \
                         {} GW (storage discharges after the full stack — D4 rule 3)",
                        total_discharge.as_gigawatts(),
                        remaining.as_gigawatts()
                    ),
                });
            }

            // Rule 5: whatever the stack and discharge leave is unserved.
            (zero, remaining - total_discharge)
        };
        // PHYSICAL TIER (every policy): the derived series are
        // non-negative — no negative curtailment (energy conjured) and no
        // masked (negative) unserved. Under RuleBased the policy-tier
        // checks above already guaranteed this, so it is a no-op there
        // (digest unmoved); once those checks are relaxed by contract it
        // is the guard that keeps `curtailment`/`unserved` — which are
        // the plug variables of the conservation identity below — from
        // absorbing a physical-law violation. (A relaxed policy charging
        // beyond the surplus would otherwise push a negative curtailment.)
        if period_curtailment.as_gigawatts() < -tolerance(period_curtailment) {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "period {t}: negative curtailment {} GW — charge or discharge \
                     exceeds what the period can physically supply",
                    period_curtailment.as_gigawatts()
                ),
            });
        }
        if period_unserved.as_gigawatts() < -tolerance(period_unserved) {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "period {t}: negative unserved {} GW — discharge exceeds the \
                     post-stack deficit (unserved energy must not be masked)",
                    period_unserved.as_gigawatts()
                ),
            });
        }
        // PHYSICAL TIER (every policy): charging and unserved energy are
        // mutually exclusive within a period. Unserved energy means load
        // was shed for lack of supply; any energy simultaneously routed
        // INTO storage could have served that shed load instead, so
        // recording both is conjured energy (the store's SoC rises on
        // supply that never existed, the phantom charge folded into
        // `unserved`). The non-negativity guards above do NOT catch this
        // in a deficit: the charge inflates the positive `unserved` plug,
        // which has no upper bound, and the conservation identity still
        // balances. RuleBased never charges in a deficit, so this is a
        // no-op for it (digest unmoved); it binds only once a policy
        // relaxes `charge_from_surplus_only` and pre-charges from supply
        // that is not there.
        let mutual_tol = tolerance(demand);
        if total_charge.as_gigawatts() > mutual_tol && period_unserved.as_gigawatts() > mutual_tol {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "period {t}: {} GW charged into storage while {} GW of demand is \
                     unserved — charging cannot draw on energy that was not supplied",
                    total_charge.as_gigawatts(),
                    period_unserved.as_gigawatts()
                ),
            });
        }
        curtailment.push(period_curtailment);
        unserved.push(period_unserved);

        // PHYSICAL TIER (every policy): energy conservation this period.
        // supply == load, i.e. must_take + stack + discharge + unserved ==
        // demand + charge + curtailment. Because curtailment and unserved
        // are the residual plug variables of this identity, the check is a
        // plug-balanced guard on the accounting arithmetic (it catches a
        // gross float blow-up or a mis-wired term), NOT a backstop against
        // negative outputs — the non-negativity guard above is that. A
        // no-op on any correct dispatch (RuleBased included — it produces
        // no value, so the digest is unmoved).
        let stack_output = thermal.iter().fold(zero, |acc, unit| acc + unit.output[t]);
        let supply = must_take + stack_output + total_discharge + period_unserved;
        let load = demand + total_charge + period_curtailment;
        let scale = must_take
            .as_gigawatts()
            .abs()
            .max(demand.as_gigawatts().abs())
            .max(1.0);
        if (supply - load).as_gigawatts().abs() > 1e-6 * scale {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "period {t}: energy not conserved — supply {} GW ≠ load {} GW",
                    supply.as_gigawatts(),
                    load.as_gigawatts()
                ),
            });
        }

        // Apply the validated actions to the store SoCs and record.
        for ((store, action), recorder) in
            stores.iter_mut().zip(&decision.actions).zip(&mut recorders)
        {
            store.apply(action.charge, action.discharge, dt)?;
            recorder.charge.push(action.charge);
            recorder.discharge.push(action.discharge);
            recorder.soc.push(store.soc);
        }
    }

    Ok(RunResult {
        start,
        demand: inputs.demand.values().to_vec(),
        renewables: renewables
            .into_iter()
            .map(|unit| TechSeries {
                tech: unit.tech.clone(),
                reliability: unit.reliability,
                reliability_overridden: unit.reliability_overridden,
                power: unit.output,
            })
            .collect(),
        exogenous: inputs
            .exogenous
            .iter()
            .map(|supply| LabelledSeries {
                label: supply.label.clone(),
                imports: supply.imports,
                reliability: supply.reliability,
                power: supply.trace.values().to_vec(),
            })
            .collect(),
        thermal: thermal
            .into_iter()
            .map(|unit| TechSeries {
                tech: unit.tech.clone(),
                reliability: unit.reliability,
                reliability_overridden: unit.reliability_overridden,
                power: unit.output,
            })
            .collect(),
        stores: stores
            .iter()
            .zip(recorders)
            .map(|(store, recorder)| StoreSeries {
                label: store.label.clone(),
                kind: store.kind,
                charge: recorder.charge,
                discharge: recorder.discharge,
                soc: recorder.soc,
            })
            .collect(),
        curtailment,
        unserved,
    })
}
