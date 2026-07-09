//! Storage dispatch policies (ADR-6: pluggable) and the rule-based
//! default.
//!
//! # The rule-based storage dispatch policy, in prose
//!
//! This is the normative statement of the rules the code below
//! implements, condensed from the adopted decision record
//! `docs/notes/d4-rule-based-dispatch.md` (D4, ADOPTED 2026-07-02) —
//! the most contestable modelling choice in the tool (risk R2): the
//! book's storage-requirement numbers are downstream of these rules.
//! The defences are ADR-6's (the policy is pluggable; headline results
//! are reported under both this policy and the Stage 7
//! perfect-foresight LP, with the gap a published finding) plus the D4
//! note itself: every rule stated, every consequence owned.
//!
//! ## Design stance
//!
//! **Greedy, chronological, zero foresight.** The policy sees only the
//! current period and the current store state — enforced structurally:
//! [`RuleBased`] reads only the [`SystemState`] it is handed, which
//! carries the current period's inputs and the store state vector and
//! **nothing from any later period** (a documented invariant of what
//! `SystemState` contains, asserted by the policy-boundary test — not a
//! consequence of the trait shape, which must also serve the Stage 7
//! `PerfectForesight` LP). The `Horizon` argument carries calendar
//! metadata only, never trace data. Rationale: (a) real operators do
//! not know next month's weather, so no-foresight storage requirements
//! are the honest upper envelope; (b) any smarter heuristic imports
//! assumptions a critic can attack — those belong in the LP policy,
//! where they are explicit in the objective; (c) ADR-6 chose this
//! default precisely because it yields *higher and more defensible*
//! storage numbers.
//!
//! ## The rules (each half-hour, in order)
//!
//! 1. **Must-take supply dispatches first**: weather-driven renewables
//!    (CF × capacity) and exogenous traces. Compute
//!    `net = must_take − demand`. If `net = 0`, no storage action.
//!    (ERRATUM 2026-07-06 — comment-consistency sweep M2, corrected in
//!    the D4 note: the adopted prose also listed "must-run plant at
//!    availability", which has NO engine representation. There is no
//!    must-run category: nuclear — and any low-rung thermal — is the
//!    bottom rung of the merit-order stack (rule 3) and is backed
//!    down whenever residual demand is below its available ceiling.
//!    Versus a must-run treatment the engine therefore supplies LESS
//!    in deep-surplus periods, so it slightly UNDERSTATES curtailment
//!    and storage charging — anti-conservative for curtailment
//!    findings. Bounded on the 2024 reference: 116/17,568 back-down
//!    periods, 0.1367 GWh total curtailment, pinned by
//!    `nuclear_backdown_periods_on_the_2024_reference` in
//!    `tests/acceptance_2024.rs`.)
//! 2. **Surplus (`net > 0`) → charge.** Stores charge in ascending
//!    `dispatch_order`, each absorbing
//!    `min(remaining surplus, power rating, headroom-limited intake)`.
//!    Surplus no store can absorb is **curtailment**. `dispatch_order`
//!    values must be unique within a zone; scenario validation rejects
//!    duplicates. *Charging draws from surplus only — never from the
//!    thermal stack.* (One narrow, stated exception when Q6 lands: DSR
//!    load repayment.) Charging from gas to store for later is an
//!    economic bet on future scarcity; a policy without prices or
//!    foresight cannot justify it, and allowing it would contaminate
//!    the storage-requirement question with an arbitrage model. It also
//!    matches the Royal Society's own methodology (electrolysis from
//!    surplus only). Owned consequence: the rule-based policy cannot
//!    pre-charge ahead of a forecast drought; the LP can; that gap is a
//!    designed finding.
//! 3. **Deficit (`net < 0`) → the non-storage dispatchable stack runs
//!    first** (merit order, at availability). Any deficit remaining
//!    after the full stack → **discharge** stores in ascending
//!    `dispatch_order`, each supplying
//!    `min(remaining deficit, power rating, SoC-limited output)`.
//!    Deficit no store can cover is **unserved energy**. *Storage is
//!    the reliability backstop, not a price arbitrageur.* For adequacy
//!    this is the feasibility-optimal discharge order — SoC is
//!    preserved for deficits that exceed the stack. Owned consequence
//!    (Q3, docs/07): under this policy added storage can never displace
//!    gas, so storage's marginal effect on emissions is structurally
//!    zero in mixed fleets — carbon-constraint sweeps must vary fleet
//!    composition or use the LP policy.
//! 4. **No reserve holding.** A store discharges for today's small
//!    deficit even if tomorrow's is fatal, and charges toward full even
//!    if the store will overflow tomorrow. Greedy means greedy. A
//!    reserve-floor variant is the designated kill-criterion-3 fallback
//!    sensitivity (D4), never a silent default change.
//!
//! ## Mechanics (normative)
//!
//! - **Efficiency split**: round-trip efficiency η splits symmetrically,
//!   `η_charge = η_discharge = √η`. SoC accounting: charging adds
//!   `power × Δt × √η`; discharging removes `power × Δt / √η` (ADR-8
//!   v1 carries only round-trip η; the Royal Society's asymmetric
//!   per-leg convention can shift *store-side* headline figures by up
//!   to ~15–20 % — owned in D4, absorbed by the published-order
//!   acceptance band).
//! - **Power ratings symmetric** for charge/discharge (ADR-8 v1).
//! - **`dispatch_order` is scenario data; the policy obeys it.** The
//!   same ascending order is used for charge and discharge (one knob,
//!   explainable). Preset guidance (not enforced): shortest duration
//!   first.
//! - **Initial SoC: full by default**, scenario-overridable
//!   (`initial_soc`), with the bisection solver's year-1 guard
//!   ([`crate::solve`]).
//! - **SoC carries across year boundaries** — no annual reset (resets
//!   are exactly how "a few days of storage" errors happen).
//! - **Determinism**: the decision is a pure function of
//!   (period inputs, store state vector); no randomness, no wall-clock,
//!   no hidden memory (ADR-5).
//!
//! DSR pseudo-storage is schema shape only in this stage: its v1
//! semantics are provisional until Q6 work begins (D4), and the engine
//! rejects DSR stores with a clear error, so no DSR store ever reaches
//! a policy here.

use grid_core::scenario::{Horizon, StorageKind, StorageSpec, ZoneSpec};
use grid_core::units::{Duration, Energy, PerUnit, Power};
use grid_core::{GridError, time::UtcInstant};

/// The state of one store as a policy sees it: static ratings plus the
/// current state of charge. Constructed by the engine from the
/// scenario's storage portfolio ([`build_stores`]).
#[derive(Debug, Clone, PartialEq)]
pub struct StoreState {
    /// Output-series label (the kind, disambiguated with
    /// `_<dispatch_order>` when a zone repeats a kind).
    pub label: String,
    /// Storage kind.
    pub kind: StorageKind,
    /// Symmetric charge/discharge power rating (ADR-8 v1).
    pub power: Power,
    /// Usable energy capacity.
    pub energy: Energy,
    /// √(round-trip efficiency): the per-leg efficiency of the
    /// symmetric split (module docs).
    pub sqrt_efficiency: PerUnit,
    /// Charge/discharge priority (ascending; unique within the zone).
    pub dispatch_order: u8,
    /// Current state of charge.
    pub soc: Energy,
}

impl StoreState {
    /// The largest grid-side charging power this period can accept:
    /// `min(power rating, headroom-limited intake)` where intake is
    /// limited so that `soc + p×Δt×√η ≤ energy`.
    #[must_use]
    pub fn max_charge(&self, dt: Duration) -> Power {
        let headroom = self.energy - self.soc;
        let intake_limit = (headroom / self.sqrt_efficiency.value()) / dt;
        min_power(self.power, intake_limit)
    }

    /// The largest grid-side discharge power this period can sustain:
    /// `min(power rating, SoC-limited output)` where output is limited
    /// so that `soc − p×Δt/√η ≥ 0`.
    #[must_use]
    pub fn max_discharge(&self, dt: Duration) -> Power {
        let output_limit = (self.soc * self.sqrt_efficiency) / dt;
        min_power(self.power, output_limit)
    }

    /// Apply one period's decision to the SoC:
    /// `ΔSoC = charge×Δt×√η − discharge×Δt/√η` (module docs), snapping
    /// f64 dust at the [0, energy] bounds (and erroring if the decision
    /// misses them by more than rounding dust — the engine has already
    /// validated the decision against [`Self::max_charge`] /
    /// [`Self::max_discharge`]).
    pub(crate) fn apply(
        &mut self,
        charge: Power,
        discharge: Power,
        dt: Duration,
    ) -> Result<(), GridError> {
        let gained = charge * dt * self.sqrt_efficiency;
        let drawn = (discharge * dt) / self.sqrt_efficiency.value();
        let mut soc = self.soc + gained - drawn;
        let capacity = self.energy.as_gigawatt_hours();
        let tolerance = 1e-9 * capacity.max(1.0);
        let value = soc.as_gigawatt_hours();
        if value > capacity {
            if value - capacity > tolerance {
                return Err(GridError::InvalidDispatchDecision {
                    reason: format!(
                        "store {}: SoC {value} GWh would exceed capacity {capacity} GWh",
                        self.label
                    ),
                });
            }
            soc = self.energy;
        } else if value < 0.0 {
            if -value > tolerance {
                return Err(GridError::InvalidDispatchDecision {
                    reason: format!(
                        "store {}: SoC {value} GWh would fall below zero",
                        self.label
                    ),
                });
            }
            soc = Energy::gigawatt_hours(0.0);
        }
        self.soc = soc;
        Ok(())
    }
}

/// Build the engine's store states from a zone's storage portfolio, in
/// ascending `dispatch_order`. Errors: [`GridError::UnsupportedFeature`]
/// for DSR stores (their v1 semantics are provisional until Q6 — D4);
/// duplicate orders and unphysical parameters are caught earlier by
/// `Scenario::validate`, which the engine runs first.
pub(crate) fn build_stores(zone: &ZoneSpec) -> Result<Vec<StoreState>, GridError> {
    let mut stores = Vec::with_capacity(zone.storage.len());
    for spec in &zone.storage {
        if spec.kind == StorageKind::Dsr {
            return Err(GridError::UnsupportedFeature {
                feature: "DSR pseudo-storage dispatch (kind = \"dsr\" is schema shape only: \
                          its v1 semantics are provisional until the Q6 work begins — \
                          docs/notes/d4-rule-based-dispatch.md §DSR)"
                    .to_owned(),
            });
        }
        stores.push(StoreState {
            label: store_label(zone, spec),
            kind: spec.kind,
            power: spec.power_gw,
            energy: spec.energy_gwh,
            sqrt_efficiency: PerUnit::new(spec.round_trip_efficiency.value().sqrt()),
            dispatch_order: spec.dispatch_order,
            // Initial SoC: full by default (D4), scenario-overridable.
            soc: spec.energy_gwh * spec.initial_soc.unwrap_or(PerUnit::new(1.0)),
        });
    }
    stores.sort_by_key(|s| s.dispatch_order);
    Ok(stores)
}

/// The output label of one store: its kind, disambiguated with the
/// dispatch order when the zone repeats a kind.
fn store_label(zone: &ZoneSpec, spec: &StorageSpec) -> String {
    let repeated = zone.storage.iter().filter(|s| s.kind == spec.kind).count() > 1;
    if repeated {
        format!("{}_{}", spec.kind, spec.dispatch_order)
    } else {
        spec.kind.as_str().to_owned()
    }
}

/// Everything a storage dispatch policy may see for one period: the
/// current period's inputs and the current store state vector.
///
/// **No-future invariant (D4, normative):** every field is a pure
/// function of the current period's inputs and the store SoC evolution
/// up to it. No trace data, no aggregate, no lookahead of any later
/// period is reachable from a `SystemState` — verified behaviourally by
/// the policy-boundary test. Policies needing foresight (the Stage 7
/// LP) receive it elsewhere; the shared trait shape does not provide it.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemState<'a> {
    /// Start of the current settlement period.
    pub instant: UtcInstant,
    /// Demand this period (adjusted per the scenario).
    pub demand: Power,
    /// Total must-take supply this period: weather-driven renewables at
    /// CF × capacity plus exogenous traces (rule 1) — and, in a
    /// multi-zone run, the zone's net link position folded in before the
    /// policy sees it (imports positive; links clear before storage —
    /// `multizone.rs` step 3).
    pub must_take: Power,
    /// Total non-storage dispatchable capacity available this period
    /// (Σ capacity × availability): what the stack can serve before
    /// storage discharge is consulted (rule 3).
    pub stack_available: Power,
    /// Store states in ascending `dispatch_order`.
    pub stores: &'a [StoreState],
}

/// One store's grid-side action for the period. At most one of the two
/// is nonzero (a store cannot charge and discharge simultaneously).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StoreAction {
    /// Power drawn from the grid into the store.
    pub charge: Power,
    /// Power delivered from the store to the grid.
    pub discharge: Power,
}

/// A policy's storage decision for one period: one action per store,
/// aligned with [`SystemState::stores`].
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchDecision {
    /// Per-store actions, in [`SystemState::stores`] order.
    pub actions: Vec<StoreAction>,
}

/// A policy's **dispatch contract**: which of the rule-based
/// *policy-tier* conventions the engine should enforce for this policy
/// (D12 rule 1, `docs/notes/d12-perfect-foresight-lp.md`).
///
/// # Two tiers of per-period validation
///
/// The engine validates every policy's decision each period in two
/// tiers, and this struct selects the second:
///
/// - **Physical invariants — laws, enforced for EVERY policy regardless
///   of contract.** Per-store non-negativity of charge/discharge; no
///   simultaneous charge and discharge; `charge ≤ max_charge` and
///   `discharge ≤ max_discharge` (ratings, headroom, SoC); the SoC
///   bounds inside [`StoreState::apply`]; non-negative derived
///   curtailment and unserved (no conjured energy, no masked unserved);
///   and per-period **energy conservation** (`must_take + stack_output +
///   discharge + unserved == demand + charge + curtailment`). No policy
///   may break these — they are the acceptance-test spine, and a
///   decision that violates one is
///   [`GridError::InvalidDispatchDecision`], never silent corruption.
/// - **Rule-based policy CHOICES — conventions, enforced only when this
///   contract declares them.** The fields below. They are the
///   [`RuleBased`] policy's *own* rules (D4), not physical laws: a
///   foresight policy (the Stage 7 LP) declares them `false` so it may,
///   for example, pre-charge a store from the thermal stack ahead of a
///   forecast drought — a decision the rule-based contract forbids at
///   the surplus-only check but the physical tier permits.
///
/// The symmetric √η per-leg split is a rule-based *convention* too (D4,
/// a ~15–20 % store-side shift), but it lives in shared mechanics
/// ([`StoreState::apply`], [`StoreState::max_charge`],
/// [`StoreState::max_discharge`]) and is not expressed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyContract {
    /// Stores charge **only from a surplus** (`total_charge ≤ surplus`;
    /// D4 rule 2 — never from the thermal stack). `RuleBased`: `true`.
    /// A policy that sets this `false` may draw charging power from the
    /// stack (the stack then serves `deficit + charge`), which is what
    /// lets a foresight policy pre-charge ahead of a drought.
    pub charge_from_surplus_only: bool,
    /// Stores discharge **only to serve the post-stack deficit** — so no
    /// discharge in a surplus period, and `total_discharge ≤` the
    /// deficit left after the full stack has run (D4 rule 3 — storage is
    /// the reliability backstop, never dispatched before the stack).
    /// `RuleBased`: `true`.
    pub discharge_after_stack_only: bool,
}

impl PolicyContract {
    /// The rule-based contract: every policy-tier obligation guaranteed
    /// — exactly today's [`RuleBased`] behaviour. This is also the
    /// conservative default (see [`DispatchPolicy::contract`]): a policy
    /// that declares nothing is held to the full rule-based discipline,
    /// on top of the always-enforced physical tier.
    #[must_use]
    pub const fn rule_based() -> Self {
        Self {
            charge_from_surplus_only: true,
            discharge_after_stack_only: true,
        }
    }
}

/// The pluggable storage dispatch policy interface (ADR-6, verbatim
/// shape). Implementations: [`RuleBased`] (the default). The
/// perfect-foresight LP is deliberately NOT a policy — the trait is
/// per-period with a no-lookahead [`SystemState`], and perfect foresight
/// needs the whole horizon, so the LP is the whole-horizon function
/// [`crate::run_multi_lp`] (D12, `crate::lp` module docs). Results for
/// headline claims are reported under both; the gap is a documented
/// finding, not a bug.
pub trait DispatchPolicy {
    /// Decide every store's action for the current period. `horizon`
    /// carries calendar metadata only, never trace data (D4).
    fn dispatch(&self, state: &SystemState<'_>, horizon: &Horizon) -> DispatchDecision;

    /// This policy's [`PolicyContract`]: the rule-based policy-tier
    /// conventions the engine should enforce for it (D12 rule 1). The
    /// physical-invariant tier binds regardless of what this returns.
    ///
    /// The default is the full rule-based contract — the conservative
    /// choice, so a policy that does not override is held to today's
    /// rules. A policy that dispatches differently (the Stage 7 LP)
    /// overrides this to relax the tier it needs.
    fn contract(&self) -> PolicyContract {
        PolicyContract::rule_based()
    }
}

/// The greedy, chronological, zero-foresight rule-based policy — the
/// default (ADR-6) and the module-docs prose made executable. See the
/// module docs for the full rules and their owned consequences.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuleBased;

impl DispatchPolicy for RuleBased {
    fn dispatch(&self, state: &SystemState<'_>, _horizon: &Horizon) -> DispatchDecision {
        let dt = Duration::half_hour();
        let mut actions = vec![StoreAction::default(); state.stores.len()];

        // Stores are consulted in ascending dispatch_order. The engine
        // hands them pre-sorted, but the policy re-derives the order:
        // obeying the scenario's dispatch_order is a *policy* rule
        // (D4 rule 2), not an input convention it trusts.
        let mut order: Vec<usize> = (0..state.stores.len()).collect();
        order.sort_by_key(|&i| state.stores[i].dispatch_order);

        // Rule 1: net position after must-take supply. net = 0 → no
        // storage action (actions stay zero).
        let net = state.must_take - state.demand;
        let zero = Power::gigawatts(0.0);

        if net > zero {
            // Rule 2: surplus → charge in ascending dispatch_order,
            // each store absorbing min(remaining surplus, power rating,
            // headroom-limited intake). What remains is curtailment
            // (the engine accounts for it).
            let mut surplus = net;
            for index in order {
                let intake = min_power(surplus, state.stores[index].max_charge(dt));
                actions[index].charge = intake;
                surplus = surplus - intake;
            }
        } else if net < zero {
            // Rule 3: deficit → the non-storage stack runs first; only
            // the post-stack deficit reaches storage, discharged in
            // ascending dispatch_order up to min(remaining deficit,
            // power rating, SoC-limited output). What remains is
            // unserved energy (engine accounting). Rule 4: no reserve
            // holding — today's deficit is served greedily.
            let deficit = -net;
            let mut remaining = deficit - min_power(deficit, state.stack_available);
            for index in order {
                let output = min_power(remaining, state.stores[index].max_discharge(dt));
                actions[index].discharge = output;
                remaining = remaining - output;
            }
        }

        DispatchDecision { actions }
    }

    /// The rule-based contract: every policy-tier convention guaranteed
    /// (surplus-only charging, discharge-after-stack). Stated explicitly
    /// so the reference behaviour is greppable and cannot drift with the
    /// trait default. This is what pins the digest `779d7444…`.
    fn contract(&self) -> PolicyContract {
        PolicyContract::rule_based()
    }
}

/// Partial-order minimum for `Power` (total on the non-NaN inputs the
/// engine produces).
fn min_power(a: Power, b: Power) -> Power {
    if a < b { a } else { b }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn state_store(power: f64, energy: f64, rte: f64, order: u8, soc: f64) -> StoreState {
        StoreState {
            label: "battery".to_owned(),
            kind: StorageKind::Battery,
            power: Power::gigawatts(power),
            energy: Energy::gigawatt_hours(energy),
            sqrt_efficiency: PerUnit::new(rte.sqrt()),
            dispatch_order: order,
            soc: Energy::gigawatt_hours(soc),
        }
    }

    #[test]
    fn max_charge_respects_power_and_headroom() {
        let dt = Duration::half_hour();
        // Plenty of headroom: power-limited.
        let store = state_store(5.0, 100.0, 1.0, 1, 0.0);
        assert_eq!(store.max_charge(dt), Power::gigawatts(5.0));
        // 1 GWh headroom at η = 0.81 (√η = 0.9): intake limit is
        // 1 / (0.5 × 0.9) = 2.2222… GW.
        let store = state_store(5.0, 10.0, 0.81, 1, 9.0);
        let max = store.max_charge(dt).as_gigawatts();
        assert!((max - 1.0 / (0.5 * 0.9)).abs() < 1e-12, "max {max}");
        // Full: nothing.
        let store = state_store(5.0, 10.0, 0.81, 1, 10.0);
        assert_eq!(store.max_charge(dt), Power::gigawatts(0.0));
    }

    #[test]
    fn max_discharge_respects_power_and_soc() {
        let dt = Duration::half_hour();
        // Plenty of SoC: power-limited.
        let store = state_store(5.0, 100.0, 1.0, 1, 100.0);
        assert_eq!(store.max_discharge(dt), Power::gigawatts(5.0));
        // 1 GWh SoC at η = 0.81: output limit is 1 × 0.9 / 0.5 = 1.8 GW.
        let store = state_store(5.0, 10.0, 0.81, 1, 1.0);
        let max = store.max_discharge(dt).as_gigawatts();
        assert!((max - 1.8).abs() < 1e-12, "max {max}");
        // Empty: nothing.
        let store = state_store(5.0, 10.0, 0.81, 1, 0.0);
        assert_eq!(store.max_discharge(dt), Power::gigawatts(0.0));
    }

    #[test]
    fn apply_updates_soc_with_the_sqrt_eta_split() {
        let dt = Duration::half_hour();
        let mut store = state_store(5.0, 10.0, 0.81, 1, 5.0);
        // Charge 2 GW: +2 × 0.5 × 0.9 = +0.9 GWh.
        store
            .apply(Power::gigawatts(2.0), Power::gigawatts(0.0), dt)
            .unwrap();
        assert!((store.soc.as_gigawatt_hours() - 5.9).abs() < 1e-12);
        // Discharge 2 GW: −2 × 0.5 / 0.9 = −1.1111… GWh.
        store
            .apply(Power::gigawatts(0.0), Power::gigawatts(2.0), dt)
            .unwrap();
        assert!((store.soc.as_gigawatt_hours() - (5.9 - 1.0 / 0.9)).abs() < 1e-12);
    }

    #[test]
    fn apply_rejects_decisions_beyond_the_bounds() {
        let dt = Duration::half_hour();
        let mut store = state_store(5.0, 1.0, 1.0, 1, 1.0);
        let err = store
            .apply(Power::gigawatts(5.0), Power::gigawatts(0.0), dt)
            .unwrap_err();
        assert!(matches!(err, GridError::InvalidDispatchDecision { .. }));
        let mut store = state_store(5.0, 1.0, 1.0, 1, 0.0);
        let err = store
            .apply(Power::gigawatts(0.0), Power::gigawatts(5.0), dt)
            .unwrap_err();
        assert!(matches!(err, GridError::InvalidDispatchDecision { .. }));
    }
}
