//! The Stage 5 multi-zone chronological dispatch engine (docs/04
//! Stage 5; ADR-7: the `Vec<Zone>` + link matrix goes live here).
//!
//! # Semantics, in prose
//!
//! Each half-hourly period, in order:
//!
//! 1. **Per zone: must-take supply and stack ceilings.** Weather-driven
//!    renewables (CF × capacity) and declared exogenous traces are
//!    taken in full; every dispatchable's ceiling this period is
//!    `capacity × availability(t)`, further capped for budgeted entries
//!    (below). The zone's pre-link residual is `demand − must-take`.
//! 2. **Link flows** (the signal-equalising flow rule — the full
//!    normative prose lives in [`crate::flow`]): borders dispatch in
//!    scenario order, each moving energy from the lower-signal zone to
//!    the higher-signal zone until signals equalise or a bound binds.
//!    The signal is selected per scenario (`dispatch.flow_signal`,
//!    schema v7 — D11): the scarcity score (default; byte-identical to
//!    pre-v7 runs) or the priced ladder (lexicographic per-zone-SRMC
//!    signal; requires `[zones.pricing]` inputs on every zone).
//!    Flows adjust each zone's residual as they are decided. The
//!    capability bound handed to the flow rule is **directional and
//!    per-period** (schema v6): forward (`from → to`) =
//!    `capability_trace` value under its stated sentinel handling when
//!    declared, else `capacity_gw`; reverse = `reverse_capacity_gw`,
//!    defaulting to `capacity_gw` (the pre-v6 symmetric semantics,
//!    byte-for-byte); `availability` multiplies both. The flow rule
//!    itself is unchanged — capability was always its parameter.
//! 3. **Per zone: the single-zone dispatch rules, unchanged** (Stage 1
//!    merit order; D4 storage rules via the pluggable policy) — with
//!    the zone's net link position folded into its must-take supply
//!    (imports positive, exports negative). Links therefore clear
//!    BEFORE storage: a zone's stores see the post-trade position.
//!    Consequences, stated: exports are always served by the stack
//!    (the flow rule never exports past the stack ceiling), never by
//!    storage discharge; storage charges only from post-export
//!    surplus.
//! 4. **Budget drawdown.** A budgeted entry (schema v4 `energy_budget`;
//!    the NO2 seasonal-budget reservoir model, D5) accrues its window's
//!    energy as allowance at the window's first period; its ceiling is
//!    `min(capacity × availability, allowance / Δt)`; its dispatched
//!    energy draws the allowance down; unused allowance carries forward
//!    (water stays in the reservoir). Greedy, zero-foresight — the D4
//!    stance: the model may exhaust a window early and lean on imports
//!    (or go unserved) at the window's tail, and that is a *finding*,
//!    not a bug. NOTE (D5 ruling b): when the budgets are derived from
//!    observed generation, the "hydro generation matches observed
//!    seasonality" limb of any validation is near-tautological — the
//!    gate content is the flow structure, not the budget shape.
//!
//! Zone results are reported per zone as ordinary [`RunResult`]s, with
//! each link's net position appended as an exogenous-style series
//! (label = the link's name; `imports = true`; reliability `variable`
//! per the gb-grid-margin methodology — interconnectors fail with the
//! weather). GB-side validation totals (`net_imports_energy`) therefore
//! work unchanged.
//!
//! **Single-zone inertness (work-order hard constraint):** with one
//! zone there are no dispatchable borders, every link series is zero,
//! and step 3 reduces to exactly the single-zone rules —
//! [`run_multi`] on a one-zone scenario is bit-identical to
//! [`crate::run`] (pinned by test; the 2024 reference digest is the
//! data-backed half of the proof).

use grid_core::GridError;
use grid_core::scenario::{
    DispatchPolicyKind, ExogenousReliability, FlowSignal, LinkSpec, Scenario, TechId, ZoneId,
    ZoneSpec,
};
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, PerUnit, Power, Price};

use crate::availability::AvailabilityModel;
use crate::flow::FLOW_MERIT_ORDER;
use crate::flow::{PricedZoneCurve, ZoneCurve, equalising_flow, equalising_flow_priced};
use crate::inputs::{MultiZoneInputs, ZoneInputs};
use crate::policy::{DispatchPolicy, RuleBased, StoreState, SystemState, build_stores};
use crate::result::{LabelledSeries, RunResult, StoreSeries, TechSeries};

/// One zone's result within a multi-zone run.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneRunResult {
    /// The zone.
    pub id: ZoneId,
    /// The zone's dispatch result (single-zone shape; link net
    /// positions appear among `exogenous`, labelled by link name).
    pub result: RunResult,
}

/// One link's per-period flows, recorded at BOTH ends (module docs;
/// the loss is the wedge between them).
#[derive(Debug, Clone, PartialEq)]
pub struct LinkFlowSeries {
    /// Link identity: the scenario's `name`, or `<from>-<to>-<index>`.
    pub name: String,
    /// Home zone (the scenario's `from`).
    pub from: ZoneId,
    /// Counterparty zone (the scenario's `to`).
    pub to: ZoneId,
    /// Signed power at the `from` end: positive = into `from`
    /// (receiving-end power when importing; −sending-end when
    /// exporting). For GB-home links this is the NESO metering
    /// convention.
    pub home_end: Vec<Power>,
    /// Signed power at the `to` end: positive = into `to`.
    pub away_end: Vec<Power>,
    /// The directional capabilities the flow rule dispatched against
    /// (schema v6): `Some` only for links declaring per-direction or
    /// per-period capability (`reverse_capacity_gw` /
    /// `capability_trace`), so pre-v6 links keep their exact output
    /// shape. Feeds the B6 capability/binding output columns.
    pub capability: Option<LinkCapabilitySeries>,
}

/// The applied directional capabilities of a schema-v6 link, per
/// period, availability included (what the flow rule actually saw).
#[derive(Debug, Clone, PartialEq)]
pub struct LinkCapabilitySeries {
    /// Forward (`from → to`) sending-end capability.
    pub forward: Vec<Power>,
    /// Reverse (`to → from`) sending-end capability.
    pub reverse: Vec<Power>,
    /// Whether the forward capability is OBSERVED (the capability-trace
    /// gate mask; all `true` for links without a trace). Masked periods
    /// dispatch against the pinned fill but are excluded from
    /// validation-gate arithmetic (the B6 ruling).
    pub forward_observed: Vec<bool>,
}

impl LinkFlowSeries {
    /// Net annual energy at the home end (positive = net import into
    /// the home zone) — the per-border validation quantity.
    #[must_use]
    pub fn net_home_energy(&self) -> Energy {
        self.home_end
            .iter()
            .map(|&p| p * Duration::half_hour())
            .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
    }
}

/// The complete result of a multi-zone dispatch run. Two runs of
/// identical inputs are bit-identical (ADR-5).
#[derive(Debug, Clone, PartialEq)]
pub struct MultiZoneRunResult {
    /// Per-zone results, in scenario `[[zones]]` order.
    pub zones: Vec<ZoneRunResult>,
    /// Per-link flow series, in scenario `[[links]]` order.
    pub links: Vec<LinkFlowSeries>,
}

impl MultiZoneRunResult {
    /// The result of one zone, by id.
    #[must_use]
    pub fn zone(&self, id: &str) -> Option<&RunResult> {
        self.zones
            .iter()
            .find(|z| z.id.as_str() == id)
            .map(|z| &z.result)
    }
}

/// The output label of a link (the scenario's `name`, or a derived
/// `<from>-<to>-<index>`).
fn link_label(link: &LinkSpec, index: usize) -> String {
    link.name
        .clone()
        .unwrap_or_else(|| format!("{}-{}-{index}", link.from, link.to))
}

/// Per-zone engine state for the multi-zone loop.
struct ZoneEngine<'a> {
    spec: &'a ZoneSpec,
    demand: &'a Trace<Power>,
    exogenous: &'a [crate::inputs::ExogenousSupply],
    renewables: Vec<RenewableUnit<'a>>,
    thermal: Vec<ThermalUnit<'a>>,
    stores: Vec<StoreState>,
    recorders: Vec<StoreRecorder>,
    curtailment: Vec<Power>,
    unserved: Vec<Power>,
    /// Net link position this period (set during the link pass).
    link_net: Power,
    /// Scratch: this period's must-take total (pre-links).
    must_take: Power,
    /// Scratch: this period's per-unit ceilings, thermal order.
    ceilings: Vec<Power>,
}

struct RenewableUnit<'a> {
    tech: &'a TechId,
    capacity: Power,
    cf: &'a Trace<PerUnit>,
    reliability: grid_core::scenario::Reliability,
    reliability_overridden: bool,
    output: Vec<Power>,
}

struct ThermalUnit<'a> {
    tech: &'a TechId,
    /// Position in the shared multi-zone merit ladder
    /// ([`FLOW_MERIT_ORDER`] — the frozen Stage 1 six-rung stack; the
    /// scarcity signal is numerically index-based, see flow.rs).
    ladder: usize,
    capacity: Power,
    availability: AvailabilityModel,
    /// Remaining budget allowance (budgeted entries only).
    allowance: Option<BudgetState>,
    reliability: grid_core::scenario::Reliability,
    reliability_overridden: bool,
    output: Vec<Power>,
}

struct BudgetState {
    window_periods: usize,
    windows: Vec<Energy>,
    remaining: Energy,
}

struct StoreRecorder {
    charge: Vec<Power>,
    discharge: Vec<Power>,
    soc: Vec<Energy>,
}

/// Run the multi-zone chronological dispatch under the scenario's
/// declared policy (`rule_based` only — `perfect_foresight` is not
/// routed through `dispatch.policy`; the perfect-foresight LP runs via
/// [`crate::run_multi_lp`], D12/Stage 7).
///
/// Pure function of `(scenario, inputs)` — no wall-clock, no globals,
/// no randomness (ADR-5). See the module docs for the semantics and
/// [`crate::flow`] for the flow rule.
pub fn run_multi(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
) -> Result<MultiZoneRunResult, GridError> {
    if scenario.dispatch.policy != DispatchPolicyKind::RuleBased {
        return Err(GridError::UnsupportedFeature {
            feature: format!(
                "the {} dispatch policy (not routed through dispatch.policy — this engine \
                 implements rule_based; the perfect-foresight LP runs via run_multi_lp, D12)",
                scenario.dispatch.policy
            ),
        });
    }
    run_multi_with_policy(scenario, inputs, &RuleBased)
}

/// [`run_multi`] with an explicit storage dispatch policy (ADR-6).
pub fn run_multi_with_policy(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    policy: &dyn DispatchPolicy,
) -> Result<MultiZoneRunResult, GridError> {
    scenario.validate()?;

    // Zone inputs must align with the scenario's zones, in order.
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
            reason: "a multi-zone run needs at least one zone and one period".to_owned(),
        });
    }
    let start = inputs.zones[0].inputs.demand.start();
    let dt = Duration::half_hour();
    let zero = Power::gigawatts(0.0);

    // Build per-zone engines.
    let mut engines: Vec<ZoneEngine> = Vec::with_capacity(scenario.zones.len());
    for (spec, zin) in scenario.zones.iter().zip(&inputs.zones) {
        engines.push(build_zone_engine(spec, zin, periods, start)?);
    }

    // Single-zone scenarios keep their links INERT (the GB reference
    // pattern: links may name external counterparties while imports are
    // exogenous) — the multi-zone path must be provably inert on one
    // zone (work-order hard constraint).
    let links_live = scenario.zones.len() > 1;

    // Link identities, zone indices, and border grouping (flow-rule
    // prose rule 5: same-pair links dispatch jointly).
    let zone_index = |id: &ZoneId| -> Result<usize, GridError> {
        scenario
            .zones
            .iter()
            .position(|z| &z.id == id)
            .ok_or_else(|| GridError::InvalidScenario {
                reason: format!("link endpoint {id} is not a declared zone"),
            })
    };
    struct LinkState<'a> {
        home: usize,
        away: usize,
        /// Forward (`from → to`) flat capability × availability, GW.
        fwd_flat: f64,
        /// Reverse (`to → from`) capability × availability, GW (schema
        /// v6 `reverse_capacity_gw`; = `fwd_flat` when absent — the
        /// pre-v6 symmetric semantics).
        rev_flat: f64,
        /// Per-period forward capability (schema v6 `capability_trace`;
        /// availability NOT yet applied — see `fwd_cap`).
        fwd_trace: Option<&'a crate::inputs::LinkCapability>,
        availability: f64,
        /// Whether the link declares v6 capability detail (records a
        /// [`LinkCapabilitySeries`] in the output).
        detailed: bool,
        loss: f64,
        home_end: Vec<Power>,
        away_end: Vec<Power>,
    }
    impl LinkState<'_> {
        /// The sending-end capability of the forward direction at `t`.
        fn fwd_cap(&self, t: usize) -> f64 {
            match self.fwd_trace {
                Some(capability) => capability.forward[t].as_gigawatts() * self.availability,
                None => self.fwd_flat,
            }
        }

        /// The sending-end capability toward `importer`'s direction at
        /// `t` when `exporter` exports.
        fn cap_toward(&self, exporter: usize, t: usize) -> f64 {
            if exporter == self.home {
                self.fwd_cap(t)
            } else {
                self.rev_flat
            }
        }
    }
    // Schema v6 alignment: a non-empty capability list must match the
    // link list; a trace-declaring link must have its capability loaded.
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
    let mut link_states: Vec<LinkState> = Vec::with_capacity(scenario.links.len());
    for (index, link) in scenario.links.iter().enumerate() {
        let fwd_trace = match (&link.capability_trace, links_live) {
            (Some(_), true) => {
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
                if capability.forward.len() != periods || capability.observed.len() != periods {
                    return Err(GridError::InvalidRunInputs {
                        reason: format!(
                            "link {}: capability covers {} periods; the horizon has {periods}",
                            link_label(link, index),
                            capability.forward.len()
                        ),
                    });
                }
                Some(capability)
            }
            _ => None,
        };
        let availability = link.availability.value();
        link_states.push(if links_live {
            LinkState {
                home: zone_index(&link.from)?,
                away: zone_index(&link.to)?,
                fwd_flat: link.capacity_gw.as_gigawatts() * availability,
                rev_flat: link
                    .reverse_capacity_gw
                    .unwrap_or(link.capacity_gw)
                    .as_gigawatts()
                    * availability,
                fwd_trace,
                availability,
                detailed: link.reverse_capacity_gw.is_some() || link.capability_trace.is_some(),
                loss: link.loss.value(),
                home_end: Vec::with_capacity(periods),
                away_end: Vec::with_capacity(periods),
            }
        } else {
            // Inert: zero flow every period, no endpoint resolution
            // (the counterparty may be external).
            LinkState {
                home: usize::MAX,
                away: usize::MAX,
                fwd_flat: 0.0,
                rev_flat: 0.0,
                fwd_trace: None,
                availability: 0.0,
                detailed: false,
                loss: 0.0,
                home_end: vec![Power::gigawatts(0.0); periods],
                away_end: vec![Power::gigawatts(0.0); periods],
            }
        });
    }
    // Borders: link indices grouped by unordered zone pair, in first
    // appearance order.
    let mut borders: Vec<(usize, usize, Vec<usize>)> = Vec::new();
    if links_live {
        for (index, state) in link_states.iter().enumerate() {
            let key = if state.home <= state.away {
                (state.home, state.away)
            } else {
                (state.away, state.home)
            };
            if let Some((_, _, members)) = borders.iter_mut().find(|(a, b, _)| (*a, *b) == key) {
                members.push(index);
            } else {
                borders.push((key.0, key.1, vec![index]));
            }
        }
    }

    // The priced ladder (schema v7, D11; flow-rule prose 1b). The
    // scarcity path below is byte-untouched when this is false.
    let priced = links_live && scenario.dispatch.flow_signal == FlowSignal::PricedLadder;
    // Per zone, per thermal unit (engine merit order): the unit's SRMC
    // series, `None` for rungs without a recipe (the £0 floor).
    let zone_srmc: Vec<Vec<Option<&[Price]>>> = if priced {
        let mut all = Vec::with_capacity(engines.len());
        for (engine, zin) in engines.iter().zip(&inputs.zones) {
            let pricing = zin
                .pricing
                .as_ref()
                .ok_or_else(|| GridError::InvalidRunInputs {
                    reason: format!(
                        "zone {}: dispatch.flow_signal = \"priced_ladder\" but no pricing \
                         inputs were loaded for the zone ([zones.pricing], schema v7 — \
                         every zone needs pricing inputs to be dispatchable under the \
                         ladder, ADR-7 touch-point)",
                        zin.id
                    ),
                })?;
            for (tech, trace) in &pricing.srmc {
                if trace.len() != periods || trace.start() != start {
                    return Err(GridError::InvalidRunInputs {
                        reason: format!(
                            "zone {}: SRMC series for {tech} does not cover the horizon \
                             ({} periods from {}; expected {periods} from {start})",
                            zin.id,
                            trace.len(),
                            trace.start(),
                        ),
                    });
                }
            }
            all.push(
                engine
                    .thermal
                    .iter()
                    .map(|unit| pricing.srmc.get(unit.tech).map(Trace::values))
                    .collect(),
            );
        }
        all
    } else {
        Vec::new()
    };
    // The run-scope fleet-SRMC ceiling per period: the unserved-region
    // price (Stage 2 convention 3 evaluated across every zone's priced
    // technologies, so unserved outbids every dispatched rung anywhere
    // — flow-rule prose 1b). A priced run with no priced technology at
    // all has no ceiling to price unserved periods: a structured
    // error, never a silent £0.
    let ceiling_price: Vec<f64> = if priced {
        let mut ceiling = vec![f64::NEG_INFINITY; periods];
        let mut any = false;
        for zin in &inputs.zones {
            let Some(pricing) = &zin.pricing else {
                continue;
            };
            for trace in pricing.srmc.values() {
                any = true;
                for (c, p) in ceiling.iter_mut().zip(trace.values()) {
                    *c = c.max(p.as_pounds_per_megawatt_hour());
                }
            }
        }
        if !any {
            return Err(GridError::InvalidRunInputs {
                reason: "priced_ladder: no zone declares any priced technology, so there \
                         is no fleet-SRMC ceiling to price unserved periods (Stage 2 \
                         convention 3; scarcity pricing arrives with the cost-synthesis \
                         stage)"
                    .to_owned(),
            });
        }
        ceiling
    } else {
        Vec::new()
    };

    /// The per-period flow-signal curves of every zone, one variant per
    /// selected signal (flow-rule prose 1a/1b).
    enum PeriodCurves {
        Scarcity(Vec<ZoneCurve>),
        Priced(Vec<PricedZoneCurve>),
    }

    for t in 0..periods {
        let instant = start.plus_periods(t as i64);

        // Step 1: must-take and ceilings per zone; pre-link residuals.
        let mut residuals: Vec<f64> = Vec::with_capacity(engines.len());
        for engine in &mut engines {
            engine.begin_period(t, instant, dt);
            residuals.push((engine.demand.values()[t] - engine.must_take).as_gigawatts());
        }

        // Step 2: the flow rule, border by border, under the selected
        // signal (prose 1a scarcity / 1b priced ladder).
        let curves: PeriodCurves = if priced {
            PeriodCurves::Priced(
                engines
                    .iter()
                    .enumerate()
                    .map(|(z, engine)| {
                        let segments: Vec<(usize, f64, f64)> = engine
                            .thermal
                            .iter()
                            .zip(&engine.ceilings)
                            .enumerate()
                            .map(|(i, (unit, ceiling))| {
                                let price = zone_srmc[z][i]
                                    .map_or(0.0, |srmc| srmc[t].as_pounds_per_megawatt_hour());
                                (unit.ladder, ceiling.as_gigawatts(), price)
                            })
                            .collect();
                        PricedZoneCurve::new(&segments, ceiling_price[t])
                    })
                    .collect::<Result<_, _>>()?,
            )
        } else {
            PeriodCurves::Scarcity(
                engines
                    .iter()
                    .map(|engine| {
                        let segments: Vec<(usize, f64)> = engine
                            .thermal
                            .iter()
                            .zip(&engine.ceilings)
                            .map(|(unit, ceiling)| (unit.ladder, ceiling.as_gigawatts()))
                            .collect();
                        ZoneCurve::new(&segments)
                    })
                    .collect::<Result<_, _>>()?,
            )
        };

        for &(zone_a, zone_b, ref members) in &borders {
            // Direction: toward the higher signal (the lexicographic
            // pair under the priced ladder). Ties: no flow.
            let direction = match &curves {
                PeriodCurves::Scarcity(curves) => {
                    let signal_a = curves[zone_a].signal(residuals[zone_a]);
                    let signal_b = curves[zone_b].signal(residuals[zone_b]);
                    if signal_a < signal_b {
                        Some((zone_a, zone_b))
                    } else if signal_b < signal_a {
                        Some((zone_b, zone_a))
                    } else {
                        None
                    }
                }
                PeriodCurves::Priced(curves) => {
                    // (f64, f64) tuples order lexicographically; the
                    // components are NaN-free by curve validation.
                    let signal_a = curves[zone_a].signal(residuals[zone_a]);
                    let signal_b = curves[zone_b].signal(residuals[zone_b]);
                    if signal_a < signal_b {
                        Some((zone_a, zone_b))
                    } else if signal_b < signal_a {
                        Some((zone_b, zone_a))
                    } else {
                        None
                    }
                }
            };
            let Some((exp, imp)) = direction else {
                for &m in members {
                    link_states[m].home_end.push(zero);
                    link_states[m].away_end.push(zero);
                }
                continue;
            };
            // Directional capability (schema v6): each member's cap in
            // the direction being dispatched this period.
            let total_cap: f64 = members
                .iter()
                .map(|&m| link_states[m].cap_toward(exp, t))
                .sum();
            // Capacity-weighted mean loss for the joint equalisation;
            // per-link losses apply exactly at the accounting split.
            let mean_loss = if total_cap > 0.0 {
                members
                    .iter()
                    .map(|&m| link_states[m].loss * link_states[m].cap_toward(exp, t))
                    .sum::<f64>()
                    / total_cap
            } else {
                0.0
            };
            let sent = match &curves {
                PeriodCurves::Scarcity(curves) => equalising_flow(
                    &curves[exp],
                    residuals[exp],
                    &curves[imp],
                    residuals[imp],
                    total_cap,
                    mean_loss,
                ),
                PeriodCurves::Priced(curves) => equalising_flow_priced(
                    &curves[exp],
                    residuals[exp],
                    &curves[imp],
                    residuals[imp],
                    total_cap,
                    mean_loss,
                ),
            };
            let mut delivered_total = 0.0;
            for &m in members {
                let state = &link_states[m];
                let share = if total_cap > 0.0 {
                    state.cap_toward(exp, t) / total_cap
                } else {
                    0.0
                };
                let link_sent = sent * share;
                let link_delivered = link_sent * (1.0 - state.loss);
                delivered_total += link_delivered;
                let (at_exp, at_imp) = (
                    Power::gigawatts(-link_sent),
                    Power::gigawatts(link_delivered),
                );
                let state = &mut link_states[m];
                if state.home == exp {
                    state.home_end.push(at_exp);
                    state.away_end.push(at_imp);
                } else {
                    state.home_end.push(at_imp);
                    state.away_end.push(at_exp);
                }
            }
            residuals[exp] += sent;
            residuals[imp] -= delivered_total;
        }

        // Fold each zone's net link position into its must-take supply.
        for (z, engine) in engines.iter_mut().enumerate() {
            let mut net = zero;
            if links_live {
                for state in &link_states {
                    if state.home == z {
                        net = net + state.home_end[t];
                    } else if state.away == z {
                        net = net + state.away_end[t];
                    }
                }
            }
            engine.link_net = net;
        }

        // Step 3 + 4: the single-zone dispatch rules per zone.
        for engine in &mut engines {
            engine.dispatch_period(t, policy, &scenario.horizon, dt)?;
        }
    }

    // Assemble results. Link flows are folded into each zone's result
    // as exogenous-style series (module docs): imports-flagged,
    // reliability `variable` (interconnectors fail with the weather —
    // the gb-grid-margin classification the GB reference scenario used
    // for its exogenous imports trace).
    let links: Vec<LinkFlowSeries> = scenario
        .links
        .iter()
        .enumerate()
        .zip(link_states)
        .map(|((index, link), state)| {
            let capability = state.detailed.then(|| LinkCapabilitySeries {
                forward: (0..periods)
                    .map(|t| Power::gigawatts(state.fwd_cap(t)))
                    .collect(),
                reverse: vec![Power::gigawatts(state.rev_flat); periods],
                forward_observed: match state.fwd_trace {
                    Some(cap) => cap.observed.clone(),
                    None => vec![true; periods],
                },
            });
            LinkFlowSeries {
                name: link_label(link, index),
                from: link.from.clone(),
                to: link.to.clone(),
                home_end: state.home_end,
                away_end: state.away_end,
                capability,
            }
        })
        .collect();
    let mut zones = Vec::with_capacity(engines.len());
    for ((z, engine), zin) in engines.into_iter().enumerate().zip(&inputs.zones) {
        let mut exogenous: Vec<LabelledSeries> = engine
            .exogenous
            .iter()
            .map(|supply| LabelledSeries {
                label: supply.label.clone(),
                imports: supply.imports,
                reliability: supply.reliability,
                power: supply.trace.values().to_vec(),
            })
            .collect();
        if links_live {
            for (link, series) in scenario.links.iter().zip(&links) {
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
        zones.push(ZoneRunResult {
            id: zin.id.clone(),
            result: RunResult {
                start,
                demand: engine.demand.values().to_vec(),
                renewables: engine
                    .renewables
                    .into_iter()
                    .map(|unit| TechSeries {
                        tech: unit.tech.clone(),
                        reliability: unit.reliability,
                        reliability_overridden: unit.reliability_overridden,
                        power: unit.output,
                    })
                    .collect(),
                exogenous,
                thermal: engine
                    .thermal
                    .into_iter()
                    .map(|unit| TechSeries {
                        tech: unit.tech.clone(),
                        reliability: unit.reliability,
                        reliability_overridden: unit.reliability_overridden,
                        power: unit.output,
                    })
                    .collect(),
                stores: engine
                    .stores
                    .iter()
                    .zip(engine.recorders)
                    .map(|(store, recorder)| StoreSeries {
                        label: store.label.clone(),
                        kind: store.kind,
                        charge: recorder.charge,
                        discharge: recorder.discharge,
                        soc: recorder.soc,
                    })
                    .collect(),
                curtailment: engine.curtailment,
                unserved: engine.unserved,
            },
        });
    }
    Ok(MultiZoneRunResult { zones, links })
}

/// Build one zone's engine state (mirrors the single-zone classifier in
/// [`crate::dispatch`]; kept separate so the frozen Stage 1–4 path is
/// untouched byte-for-byte).
fn build_zone_engine<'a>(
    spec: &'a ZoneSpec,
    zin: &'a ZoneInputs,
    periods: usize,
    start: grid_core::time::UtcInstant,
) -> Result<ZoneEngine<'a>, GridError> {
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

    let mut renewables: Vec<RenewableUnit<'a>> = Vec::new();
    let mut thermal: Vec<ThermalUnit<'a>> = Vec::new();
    for entry in &spec.fleet {
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
            renewables.push(RenewableUnit {
                tech: &entry.technology,
                capacity: entry.capacity_gw,
                cf,
                reliability: entry.effective_reliability(),
                reliability_overridden: entry.reliability_overridden(),
                output: Vec::with_capacity(periods),
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
                .unwrap_or(
                    // Infallible: 1.0 is in range.
                    AvailabilityModel::flat(PerUnit::new(1.0))?,
                );
            let allowance = match &entry.energy_budget {
                None => None,
                Some(_) => {
                    let schedule = zin.budgets.get(&entry.technology).ok_or_else(|| {
                        GridError::InvalidRunInputs {
                            reason: format!(
                                "zone {}: no budget schedule loaded for {}",
                                spec.id, entry.technology
                            ),
                        }
                    })?;
                    let needed = periods.div_ceil(schedule.window_periods);
                    if schedule.windows.len() != needed {
                        return Err(GridError::InvalidRunInputs {
                            reason: format!(
                                "zone {}, technology {}: {} budget windows for a horizon \
                                 needing {needed} (window_periods = {})",
                                spec.id,
                                entry.technology,
                                schedule.windows.len(),
                                schedule.window_periods
                            ),
                        });
                    }
                    Some(BudgetState {
                        window_periods: schedule.window_periods,
                        windows: schedule.windows.clone(),
                        remaining: Energy::gigawatt_hours(0.0),
                    })
                }
            };
            thermal.push(ThermalUnit {
                tech: &entry.technology,
                ladder,
                capacity: entry.capacity_gw,
                availability,
                allowance,
                reliability: entry.effective_reliability(),
                reliability_overridden: entry.reliability_overridden(),
                output: vec![Power::gigawatts(0.0); periods],
            });
        }
    }
    thermal.sort_by_key(|unit| unit.ladder);
    // Two entries on the same ladder rung would make the zone curve
    // ill-defined (and the scenario ambiguous).
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

    for supply in &zin.inputs.exogenous {
        if supply.trace.len() != periods || supply.trace.start() != start {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "zone {}: exogenous supply {:?} does not cover the horizon",
                    spec.id, supply.label
                ),
            });
        }
    }

    let stores = build_stores(spec)?;
    let recorders = stores
        .iter()
        .map(|_| StoreRecorder {
            charge: Vec::with_capacity(periods),
            discharge: Vec::with_capacity(periods),
            soc: Vec::with_capacity(periods),
        })
        .collect();

    Ok(ZoneEngine {
        spec,
        demand: &zin.inputs.demand,
        exogenous: &zin.inputs.exogenous,
        renewables,
        thermal,
        stores,
        recorders,
        curtailment: Vec::with_capacity(periods),
        unserved: Vec::with_capacity(periods),
        link_net: Power::gigawatts(0.0),
        must_take: Power::gigawatts(0.0),
        ceilings: Vec::new(),
    })
}

impl<'a> ZoneEngine<'a> {
    /// Step 1 for this zone: record renewable output, total must-take
    /// (pre-links), and per-unit ceilings (availability and budget).
    fn begin_period(&mut self, t: usize, instant: grid_core::time::UtcInstant, dt: Duration) {
        let mut must_take = Power::gigawatts(0.0);
        for unit in &mut self.renewables {
            let output = unit.capacity * unit.cf.values()[t];
            unit.output.push(output);
            must_take = must_take + output;
        }
        for supply in self.exogenous {
            must_take = must_take + supply.trace.values()[t];
        }
        self.must_take = must_take;

        self.ceilings.clear();
        for unit in &mut self.thermal {
            // Budget accrual at window starts (module docs, step 4).
            if let Some(budget) = &mut unit.allowance
                && t.is_multiple_of(budget.window_periods)
            {
                let window = t / budget.window_periods;
                budget.remaining = budget.remaining + budget.windows[window];
            }
            let mut ceiling = unit.capacity * unit.availability.factor_at(instant);
            if let Some(budget) = &unit.allowance {
                let release_limit = budget.remaining / dt;
                if release_limit < ceiling {
                    ceiling = release_limit;
                }
            }
            self.ceilings.push(ceiling);
        }
    }

    /// Steps 3–4 for this zone: the single-zone dispatch rules with the
    /// link net position folded into must-take.
    fn dispatch_period(
        &mut self,
        t: usize,
        policy: &dyn DispatchPolicy,
        horizon: &grid_core::scenario::Horizon,
        dt: Duration,
    ) -> Result<(), GridError> {
        let zero = Power::gigawatts(0.0);
        let demand = self.demand.values()[t];
        let must_take = self.must_take + self.link_net;

        let stack_available = self
            .ceilings
            .iter()
            .fold(zero, |acc, &ceiling| acc + ceiling);

        let decision = policy.dispatch(
            &SystemState {
                instant: self.demand.start().plus_periods(t as i64),
                demand,
                must_take,
                stack_available,
                stores: &self.stores,
            },
            horizon,
        );
        if decision.actions.len() != self.stores.len() {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "zone {}, period {t}: {} actions for {} stores",
                    self.spec.id,
                    decision.actions.len(),
                    self.stores.len()
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
        // non-negativity / no-simultaneous-charge-discharge.
        let tolerance = |scale: Power| 1e-9 * scale.as_gigawatts().abs().max(1.0);
        let mut total_charge = zero;
        let mut total_discharge = zero;
        for (store, action) in self.stores.iter().zip(&decision.actions) {
            let infeasible = |reason: String| GridError::InvalidDispatchDecision {
                reason: format!(
                    "zone {}, period {t}, store {}: {reason}",
                    self.spec.id, store.label
                ),
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
        // for a policy whose contract declares it (D12 rule 1).
        if contract.charge_from_surplus_only
            && (total_charge - surplus).as_gigawatts() > tolerance(surplus)
        {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "zone {}, period {t}: total charge {} GW exceeds the surplus {} GW \
                     (charging draws from surplus only — D4 rule 2)",
                    self.spec.id,
                    total_charge.as_gigawatts(),
                    surplus.as_gigawatts()
                ),
            });
        }

        let (period_curtailment, period_unserved) = if net > zero {
            // POLICY TIER — no discharge in a surplus period (D4 rule 3).
            if contract.discharge_after_stack_only
                && (total_discharge - zero).as_gigawatts() > tolerance(zero)
            {
                return Err(GridError::InvalidDispatchDecision {
                    reason: format!(
                        "zone {}, period {t}: discharge during a surplus period \
                         (storage discharges after the full stack — D4 rule 3)",
                        self.spec.id
                    ),
                });
            }
            // For RuleBased total_discharge == 0 here, so
            // `(surplus − total_charge) + 0.0` is bit-identical.
            ((surplus - total_charge) + total_discharge, zero)
        } else {
            // The stack serves the deficit PLUS any charging load a
            // relaxed policy imposes. For RuleBased total_charge == 0 in a
            // non-surplus period, so `deficit + 0.0` is bit-identical.
            let mut remaining = deficit + total_charge;
            for (unit, &ceiling) in self.thermal.iter_mut().zip(&self.ceilings) {
                let output = if remaining < ceiling {
                    remaining
                } else {
                    ceiling
                };
                unit.output[t] = output;
                remaining = remaining - output;
                // Step 4: budget drawdown by dispatched energy.
                if let Some(budget) = &mut unit.allowance {
                    let drawn = output * dt;
                    let left = budget.remaining - drawn;
                    budget.remaining = if left < Energy::gigawatt_hours(0.0) {
                        // f64 dust only: the ceiling already enforced
                        // the allowance.
                        Energy::gigawatt_hours(0.0)
                    } else {
                        left
                    };
                }
            }
            // POLICY TIER — discharge ≤ post-stack deficit (D4 rule 3).
            if contract.discharge_after_stack_only
                && (total_discharge - remaining).as_gigawatts() > tolerance(remaining)
            {
                return Err(GridError::InvalidDispatchDecision {
                    reason: format!(
                        "zone {}, period {t}: total discharge {} GW exceeds the post-stack \
                         deficit {} GW (D4 rule 3)",
                        self.spec.id,
                        total_discharge.as_gigawatts(),
                        remaining.as_gigawatts()
                    ),
                });
            }
            (zero, remaining - total_discharge)
        };
        // PHYSICAL TIER (every policy): the derived series are
        // non-negative — no negative curtailment (energy conjured) and no
        // masked (negative) unserved. A no-op under RuleBased (the
        // policy-tier checks above already guaranteed it; digest unmoved);
        // once those are relaxed by contract it is the guard that stops
        // `curtailment`/`unserved` — the plug variables of the
        // conservation identity below — from absorbing a law violation.
        if period_curtailment.as_gigawatts() < -tolerance(period_curtailment) {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "zone {}, period {t}: negative curtailment {} GW — charge or discharge \
                     exceeds what the period can physically supply",
                    self.spec.id,
                    period_curtailment.as_gigawatts()
                ),
            });
        }
        if period_unserved.as_gigawatts() < -tolerance(period_unserved) {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "zone {}, period {t}: negative unserved {} GW — discharge exceeds the \
                     post-stack deficit (unserved energy must not be masked)",
                    self.spec.id,
                    period_unserved.as_gigawatts()
                ),
            });
        }
        // PHYSICAL TIER (every policy): charging and unserved energy are
        // mutually exclusive within a zone-period. Unserved means load was
        // shed for lack of supply (imports already fold into `must_take`);
        // any energy simultaneously routed into storage could have served
        // that shed load, so recording both conjures energy — the store's
        // SoC rises on supply that never existed while the phantom charge
        // inflates the unbounded positive `unserved` plug. A no-op under
        // RuleBased (never charges in a deficit; digest unmoved); binds
        // only once a policy relaxes `charge_from_surplus_only`.
        let mutual_tol = tolerance(demand);
        if total_charge.as_gigawatts() > mutual_tol && period_unserved.as_gigawatts() > mutual_tol {
            return Err(GridError::InvalidDispatchDecision {
                reason: format!(
                    "zone {}, period {t}: {} GW charged into storage while {} GW of demand \
                     is unserved — charging cannot draw on energy that was not supplied",
                    self.spec.id,
                    total_charge.as_gigawatts(),
                    period_unserved.as_gigawatts()
                ),
            });
        }
        self.curtailment.push(period_curtailment);
        self.unserved.push(period_unserved);

        // PHYSICAL TIER (every policy): energy conservation this period
        // (must_take already folds in link_net). Because curtailment and
        // unserved are the residual plug variables of this identity, the
        // check is a plug-balanced guard on the accounting arithmetic, NOT
        // a backstop against negative outputs — the non-negativity guard
        // above is that. A no-op on any correct dispatch — produces no
        // value, so the digest is unmoved.
        let stack_output = self
            .thermal
            .iter()
            .fold(zero, |acc, unit| acc + unit.output[t]);
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
                    "zone {}, period {t}: energy not conserved — supply {} GW ≠ load {} GW",
                    self.spec.id,
                    supply.as_gigawatts(),
                    load.as_gigawatts()
                ),
            });
        }

        for ((store, action), recorder) in self
            .stores
            .iter_mut()
            .zip(&decision.actions)
            .zip(&mut self.recorders)
        {
            store.apply(action.charge, action.discharge, dt)?;
            recorder.charge.push(action.charge);
            recorder.discharge.push(action.discharge);
            recorder.soc.push(store.soc);
        }
        Ok(())
    }
}
