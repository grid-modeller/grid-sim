//! The Q8 fleet-pathway runner (Stage 6 part 2; docs/04 Stage 6 scope:
//! "pathway runner (fleet as function of year) for Q8"; demo artefact:
//! largest-survivable-loss vs year — "the year the grid can no longer
//! ride through" as a date).
//!
//! A pathway spec (`schema = "fes-pathway-v1"`) names a fleet per year.
//! For each year and each *dispatch condition* the runner derives the
//! synchronous inertia of the year's fleet and bisects for the largest
//! loss-of-infeed the swing simulation survives. Every modelling
//! convention is stated here, at the definition site:
//!
//! ## 1. Fleet → inertia (the dispatch-condition convention)
//!
//! A pathway year carries installed capacities, not a dispatch, so the
//! part-1 dispatch-keyed sum (`Σ H × dispatched-GW/0.9` per settlement
//! period) has no period to key on. The adopted convention is a
//! **stated synchronous dispatch fraction** φ per named condition:
//!
//! `E(year, φ) = Σ Hᵢ × (capacityᵢ × φ) / 0.9` over synchronous plant,
//!
//! with H and the synchronous flag from the `grid_core::inertia`
//! defaults (cited literature values; the pathway schema carries no
//! per-entry overrides — a pathway asserting e.g. synchronous hydrogen
//! turbines must use a full scenario instead) and MVA = GW / 0.9 at the
//! single part-1 conversion point (`Power::apparent`). Synchronous
//! storage (pumped hydro) counts at its power rating under the same
//! fraction — it is synchronised while running in either direction
//! (part-1 convention); `energy_gwh` is parsed for schema completeness
//! but plays no role in an event-timescale simulation.
//!
//! **The honest answer is a band, not a line**: results are reported
//! per condition, one line each. The default conditions (used when the
//! spec carries none, and flagged in outputs) are `min` φ = 0.15 and
//! `mean` φ = 0.35. Basis, measured from the part-1 2024 reference run
//! (market-only dispatch, `grid-cli stability inertia`, measured
//! 2026-07-03): the H-weighted synchronised share of the installed
//! synchronous fleet — period inertia ÷ 231.4 GVA·s, the fully
//! committed 2024 synchronous fleet's Σ H × cap/0.9 — has mean 0.344,
//! 5th/10th percentiles 0.147/0.169 and 90th percentile 0.574 over the
//! 17,568 periods. So `mean` 0.35 tracks the measured average and
//! `min` 0.15 the measured low decile. The *market-only minimum is
//! zero* (the part-1 UNCONSTRAINED finding: 2 periods of literally
//! zero inertia), so the true lower edge of the band is zero
//! survivable loss — that caveat is carried into every artefact. The
//! defaults are stated, measurement-anchored conventions, not
//! predictions; real NESO operation holds more inertia than
//! market-only dispatch because it pays for synchronous provision.
//!
//! ## 2. Era assumptions are spec inputs (2019 values as the cited
//! default, flagged)
//!
//! Response services, load damping, the demand base, the survival
//! floor and the search parameters are all **inputs of the pathway
//! spec** (docs/04: era-dependent limits and holdings are scenario
//! inputs, never constants). When absent they default to the committed
//! 9 Aug 2019 event reconstruction's values (holdings × published
//! delivery factors × Grid-Code envelope timings; damping 1.836 %/Hz;
//! demand base 29 GW — `scenarios/events/gb-2019-08-09.toml`, per-
//! number citations there), drift-guarded by test
//! (`default_era_assumptions_match_the_committed_2019_spec`) and
//! flagged `responses_defaulted_to_2019` / `load_damping_defaulted` /
//! `demand_fallback_defaulted` in every output. The part-1 no-retuning
//! rule (stage-6-stability-run-report.md §2) binds these values: this
//! module only *reads* them.
//!
//! Response services here are **dynamic (droop-proportional) only**.
//! Static latched services make delivery history-dependent (activation
//! time feeds the sustain/rundown clock), which can break the
//! monotonicity of survival in the loss size that the bisection
//! requires; the spec shape therefore has no `kind`/`trigger_hz`
//! fields, and an era needing static services is out of this runner's
//! scope by construction.
//!
//! ## 3. The survival criterion and the simulated window
//!
//! A loss L **survives** if the simulated frequency never falls below
//! the survival floor — default **48.8 Hz, LFDD stage 1** (E3C stage
//! table; NESO FRCR 2024 risk appetite is a 1-in-30-year 48.8 Hz
//! event) — using the same strict `f < trigger` relay semantics as the
//! part-1 LFDD model: `nadir ≥ floor` passes. The survival simulation
//! carries **no LFDD scheme**: reaching any stage *is* the failure
//! being searched for, so disconnection blocks must not arrest the
//! trajectory. The loss is a single step at t = 0 (the secured-loss
//! standards are defined for instantaneous single losses, not the
//! staged 2019 sequence).
//!
//! The simulated window defaults to **120 s**: the frequency-
//! containment window. The 2019 event took ~76 s from fault to LFDD
//! under a *staged* loss; a single-step loss at pathway-scale inertia
//! reaches its nadir well inside 120 s. Beyond the window,
//! survivability is governed by sustained reserves and control-room
//! dispatch actions that this model excludes by the same documented
//! convention as the 2019 event spec (post-nadir actions out of
//! scope). The window is a spec input (`duration_s`); longer windows
//! give smaller (more conservative) survivable losses.
//!
//! ## 4. The search
//!
//! Bisection on [0, `search_max_loss`] (default 5,000 MW — above every
//! GB secured-loss standard and the 2019 total loss) to the documented
//! tolerance `search_tolerance` (default 1 MW), returning the largest
//! bracket point that survives. Zero loss survives trivially (no
//! imbalance). If even `search_max_loss` survives, the result is the
//! bracket top with `bracket_saturated = true` — reported, not hidden.
//! Survival is monotone non-increasing in L (at fixed (t, f) the
//! deficit grows with L and the dynamics are state-free — see §2), so
//! bisection is sound and deterministic: fixed arithmetic, no
//! randomness, bit-identical reruns (ADR-5).
//!
//! ## 5. The zero-inertia year — a FINDING, not an error
//!
//! With E = 0 the swing equation divides by zero: RoCoF is unbounded
//! and the model is undefined. Following the RS-lean precedent (an
//! all-variable fleet's zero inertia is an *output*), the runner
//! reports **largest survivable loss = 0 MW** — no loss of any size is
//! survivable without synchronous (or explicitly modelled synthetic)
//! provision — with `zero_inertia = true` so callers state the finding
//! in words rather than plotting a silent zero.
//!
//! ## 6. Demand
//!
//! Demand enters the event model only as the load-damping base. A
//! year's optional `demand_twh` is converted to a mean power over a
//! stated 8,760 h year (leap days ignored — a documented convention;
//! the damping term is first-order and the 0.07 % leap-year effect is
//! noise against the damping constant's own uncertainty). Years
//! without `demand_twh` use the spec-level `demand_gw` fallback
//! (default: the 2019 event's 29 GW base, flagged).

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use grid_core::GridError;
use grid_core::inertia::{DEFAULT_POWER_FACTOR, storage_kind_default, technology_default};
use grid_core::scenario::{StorageKind, TechId};
use grid_core::units::{Damping, Duration, Energy, Frequency, Inertia, PerUnit, Power};

use crate::spec::{EventSpec, InfeedLoss, ResponseService, ResponseShape};
use crate::swing::simulate;

/// The pathway-spec schema identifier this runner reads.
pub const PATHWAY_SCHEMA: &str = "fes-pathway-v1";

/// A fully validated fleet pathway (see the module docs for every
/// convention).
#[derive(Debug, Clone, PartialEq)]
pub struct PathwaySpec {
    /// Human-readable pathway name.
    pub name: String,
    /// The FES edition (or other source) the pathway transcribes.
    pub fes_edition: String,
    /// Era assumptions: response services, damping, demand base,
    /// survival floor, search parameters, dispatch conditions.
    pub assumptions: PathwayAssumptions,
    /// The pathway years, strictly increasing.
    pub years: Vec<PathwayYear>,
}

/// One pathway year's fleet.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayYear {
    /// Calendar year.
    pub year: i32,
    /// Mean demand derived from the year's `demand_twh` over a stated
    /// 8,760 h year (module docs §6); `None` = use the spec fallback.
    pub demand: Option<Power>,
    /// Generation fleet (installed capacities).
    pub fleet: Vec<PathwayFleetEntry>,
    /// Storage fleet (installed power ratings).
    pub storage: Vec<PathwayStorageEntry>,
}

/// One generation technology's installed capacity in a pathway year.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayFleetEntry {
    /// Scenario-schema technology id (`ccgt`, `offshore_wind`, …);
    /// stability metadata comes from the `grid_core::inertia` defaults
    /// — the pathway schema carries no overrides (module docs §1).
    pub technology: TechId,
    /// Installed capacity.
    pub capacity: Power,
}

/// One storage kind's installed rating in a pathway year.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayStorageEntry {
    /// Scenario-schema storage kind.
    pub kind: StorageKind,
    /// Installed power rating.
    pub power: Power,
    /// Usable energy capacity — parsed for schema completeness, unused
    /// by the event-timescale survival model (module docs §1).
    pub energy: Option<Energy>,
}

/// One named dispatch condition: the stated fraction of the
/// synchronous fleet's capacity taken as synchronised (module docs §1).
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchCondition {
    /// Condition label (one chart line per condition).
    pub name: String,
    /// The synchronous dispatch fraction φ ∈ [0, 1].
    pub synchronous_dispatch_fraction: PerUnit,
}

/// One secured-loss reference line for the Q8 chart.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceLoss {
    /// Standard label.
    pub name: String,
    /// The secured loss.
    pub loss: Power,
}

/// The era assumptions of a pathway run — all spec inputs, with the
/// 2019 values as cited, flagged defaults (module docs §2).
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayAssumptions {
    /// Nominal system frequency (default 50 Hz).
    pub f0: Frequency,
    /// Demand fallback for years without `demand_twh` (default: the
    /// 2019 event's 29 GW base).
    pub demand_fallback: Power,
    /// Whether `demand_fallback` was defaulted rather than given.
    pub demand_fallback_defaulted: bool,
    /// Load damping, %/Hz of remaining demand (default: the part-1
    /// 2019 derivation, 1.836 %/Hz).
    pub load_damping: Damping,
    /// Whether `load_damping` was defaulted rather than given.
    pub load_damping_defaulted: bool,
    /// Simulated window per survival check (default 120 s — module
    /// docs §3).
    pub duration: Duration,
    /// Integrator step (default 10 ms, the part-1 record).
    pub timestep: Duration,
    /// Survival floor (default 48.8 Hz, LFDD stage 1).
    pub survival_floor: Frequency,
    /// Bisection bracket top (default 5,000 MW).
    pub search_max_loss: Power,
    /// Bisection tolerance (default 1 MW).
    pub search_tolerance: Power,
    /// Frequency-response services held in the pathway's era —
    /// dynamic (droop) services only (module docs §2).
    pub responses: Vec<ResponseService>,
    /// Whether `responses` defaulted to the 2019 holdings.
    pub responses_defaulted_to_2019: bool,
    /// The dispatch conditions (default: `min` 0.15 / `mean` 0.35 —
    /// module docs §1).
    pub dispatch_conditions: Vec<DispatchCondition>,
    /// Whether `dispatch_conditions` was defaulted.
    pub dispatch_conditions_defaulted: bool,
    /// Secured-loss reference lines (default: 1,800 MW SQSS
    /// infrequent-infeed-loss / FRCR planning largest loss, and
    /// 1,320 MW SQSS normal infeed loss —
    /// `data/reference/stability-2019-event.toml` [standards_2019] /
    /// [standards_current]).
    pub reference_losses: Vec<ReferenceLoss>,
}

/// The survival-search result for one (year, condition) point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurvivablePoint {
    /// The largest loss that survives (bracket point; module docs §4).
    pub largest_survivable_loss: Power,
    /// True when even the bracket top survives — the true value lies
    /// above the search bracket.
    pub bracket_saturated: bool,
    /// True for the zero-inertia FINDING (module docs §5): the swing
    /// model is undefined and the reported loss is 0 MW by convention.
    pub zero_inertia: bool,
}

/// One (year, dispatch-condition) result of a pathway run.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayPoint {
    /// Calendar year.
    pub year: i32,
    /// Dispatch-condition label.
    pub condition: String,
    /// The condition's synchronous dispatch fraction.
    pub fraction: PerUnit,
    /// The year's synchronous inertia under the condition.
    pub inertia: Inertia,
    /// The damping base used (year demand or the spec fallback).
    pub demand: Power,
    /// Whether the demand came from the year's `demand_twh`.
    pub demand_from_year: bool,
    /// The survival-search result.
    pub survivable: SurvivablePoint,
}

/// The synchronous inertia of a pathway year at a dispatch fraction:
/// `Σ H × (capacity × φ) / 0.9` over synchronous plant and storage
/// (module docs §1).
#[must_use]
pub fn pathway_year_inertia(year: &PathwayYear, fraction: PerUnit) -> Inertia {
    let pf = DEFAULT_POWER_FACTOR;
    let mut total = Inertia::gigavolt_ampere_seconds(0.0);
    for entry in &year.fleet {
        let default = technology_default(&entry.technology);
        if let Some(h) = default.h
            && default.synchronous
        {
            total = total + h * (entry.capacity * fraction).apparent(pf);
        }
    }
    for store in &year.storage {
        let default = storage_kind_default(store.kind);
        if let Some(h) = default.h
            && default.synchronous
        {
            total = total + h * (store.power * fraction).apparent(pf);
        }
    }
    total
}

/// Whether a loss survives: simulate a single step loss at t = 0 under
/// the era assumptions and require the nadir to stay at or above the
/// survival floor (module docs §3).
fn survives(event: &mut EventSpec, floor: Frequency, loss: Power) -> Result<bool, GridError> {
    if loss.as_gigawatts() <= 0.0 {
        // No imbalance: frequency holds at nominal.
        return Ok(true);
    }
    event.losses[0].power = loss;
    let result = simulate(event)?;
    Ok(result.nadir >= floor)
}

/// The largest survivable loss at one inertia/demand point, by
/// bisection to the spec's tolerance (module docs §4–§5).
pub fn largest_survivable_loss(
    assumptions: &PathwayAssumptions,
    inertia: Inertia,
    demand: Power,
) -> Result<SurvivablePoint, GridError> {
    if inertia.as_gigavolt_ampere_seconds() <= 0.0 {
        // The zero-inertia FINDING (module docs §5).
        return Ok(SurvivablePoint {
            largest_survivable_loss: Power::gigawatts(0.0),
            bracket_saturated: false,
            zero_inertia: true,
        });
    }
    // One event spec, its loss mutated per bisection candidate.
    let mut event = EventSpec {
        name: "pathway-survival".to_owned(),
        f0: assumptions.f0,
        demand,
        inertia,
        load_damping: assumptions.load_damping,
        duration: assumptions.duration,
        timestep: assumptions.timestep,
        losses: vec![InfeedLoss {
            name: "step-loss".to_owned(),
            power: Power::gigawatts(0.0),
            at: Duration::from_seconds(0.0),
        }],
        responses: assumptions.responses.clone(),
        // No LFDD in the survival simulation: reaching any stage IS
        // the failure being searched for (module docs §3).
        lfdd: None,
        limits: None,
        rocof_window: None,
    };
    let floor = assumptions.survival_floor;
    let mut lo = 0.0_f64; // survives (trivially)
    let mut hi = assumptions.search_max_loss.as_gigawatts();
    if survives(&mut event, floor, Power::gigawatts(hi))? {
        return Ok(SurvivablePoint {
            largest_survivable_loss: assumptions.search_max_loss,
            bracket_saturated: true,
            zero_inertia: false,
        });
    }
    let tolerance = assumptions.search_tolerance.as_gigawatts();
    while hi - lo > tolerance {
        let mid = 0.5 * (lo + hi);
        if survives(&mut event, floor, Power::gigawatts(mid))? {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(SurvivablePoint {
        largest_survivable_loss: Power::gigawatts(lo),
        bracket_saturated: false,
        zero_inertia: false,
    })
}

/// Run a whole pathway: every year × every dispatch condition, in spec
/// order (years validated strictly increasing; deterministic).
pub fn run_pathway(spec: &PathwaySpec) -> Result<Vec<PathwayPoint>, GridError> {
    let mut points = Vec::new();
    for year in &spec.years {
        let (demand, demand_from_year) = match year.demand {
            Some(demand) => (demand, true),
            None => (spec.assumptions.demand_fallback, false),
        };
        for condition in &spec.assumptions.dispatch_conditions {
            let inertia = pathway_year_inertia(year, condition.synchronous_dispatch_fraction);
            let survivable = largest_survivable_loss(&spec.assumptions, inertia, demand)?;
            points.push(PathwayPoint {
                year: year.year,
                condition: condition.name.clone(),
                fraction: condition.synchronous_dispatch_fraction,
                inertia,
                demand,
                demand_from_year,
                survivable,
            });
        }
    }
    Ok(points)
}

// ---------------------------------------------------------------------
// TOML-facing raw structs: explicit units in field names, strict
// parsing (schema probe + deny_unknown_fields), converted to the
// canonical newtypes in one place (ADR-4 boundary) — the same pattern
// as the event spec and the scenario.
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPathway {
    schema: String,
    name: String,
    fes_edition: String,
    assumptions: Option<RawAssumptions>,
    #[serde(default)]
    years: Vec<RawYear>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawAssumptions {
    f0_hz: Option<f64>,
    demand_gw: Option<f64>,
    load_damping_percent_per_hz: Option<f64>,
    duration_s: Option<f64>,
    timestep_ms: Option<f64>,
    survival_floor_hz: Option<f64>,
    search_max_loss_mw: Option<f64>,
    search_tolerance_mw: Option<f64>,
    responses: Option<Vec<RawPathwayResponse>>,
    dispatch_conditions: Option<Vec<RawCondition>>,
    reference_losses: Option<Vec<RawReferenceLoss>>,
}

/// Dynamic (droop) services only — no `kind`/`trigger_hz` fields by
/// design (module docs §2: bisection needs state-free dynamics).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPathwayResponse {
    name: String,
    mw: f64,
    #[serde(default = "default_delivery_factor")]
    delivery_factor: f64,
    droop_full_deviation_hz: f64,
    delay_s: f64,
    ramp_s: f64,
    sustain_s: Option<f64>,
    rundown_s: Option<f64>,
}

fn default_delivery_factor() -> f64 {
    1.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCondition {
    name: String,
    synchronous_dispatch_fraction: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReferenceLoss {
    name: String,
    mw: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawYear {
    year: i32,
    demand_twh: Option<f64>,
    #[serde(default)]
    fleet: Vec<RawFleetEntry>,
    #[serde(default)]
    storage: Vec<RawStorageEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFleetEntry {
    technology: TechId,
    capacity_gw: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStorageEntry {
    kind: StorageKind,
    power_gw: f64,
    energy_gwh: Option<f64>,
}

/// The 2019-era default response services — the committed 9 Aug 2019
/// event spec's holdings × delivery factors × envelope timings,
/// verbatim (`scenarios/events/gb-2019-08-09.toml`; per-number
/// citations there). Drift-guarded by
/// `default_era_assumptions_match_the_committed_2019_spec`.
fn default_2019_responses() -> Vec<ResponseService> {
    let droop = Frequency::hertz(0.5);
    vec![
        ResponseService {
            name: "battery_ffr".to_owned(),
            power: Power::megawatts(472.0),
            delivery_factor: PerUnit::new(1.0),
            shape: ResponseShape::Dynamic {
                droop_full_deviation: droop,
            },
            delay: Duration::from_seconds(0.3),
            ramp: Duration::from_seconds(0.7),
            sustain: Some(Duration::from_seconds(30.0)),
            rundown: Some(Duration::from_seconds(10.0)),
        },
        ResponseService {
            name: "conventional_primary".to_owned(),
            power: Power::megawatts(550.0),
            delivery_factor: PerUnit::new(0.670909),
            shape: ResponseShape::Dynamic {
                droop_full_deviation: droop,
            },
            delay: Duration::from_seconds(2.0),
            ramp: Duration::from_seconds(8.0),
            sustain: Some(Duration::from_seconds(30.0)),
            rundown: Some(Duration::from_seconds(10.0)),
        },
        ResponseService {
            name: "secondary".to_owned(),
            power: Power::megawatts(1314.0),
            delivery_factor: PerUnit::new(0.802892),
            shape: ResponseShape::Dynamic {
                droop_full_deviation: droop,
            },
            delay: Duration::from_seconds(10.0),
            ramp: Duration::from_seconds(20.0),
            sustain: None,
            rundown: None,
        },
    ]
}

impl PathwaySpec {
    /// Parse a pathway spec from TOML text (strict; see module docs).
    pub fn from_toml_str(toml_text: &str) -> Result<Self, GridError> {
        let raw: RawPathway =
            toml::from_str(toml_text).map_err(|source| GridError::PathwaySpecParse {
                source: Box::new(source),
            })?;
        Self::from_raw(raw)
    }

    /// Read and parse a pathway-spec file, attaching the path to any
    /// error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InPathwaySpecFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    #[allow(clippy::too_many_lines)] // one validation pass, linear and flat
    fn from_raw(raw: RawPathway) -> Result<Self, GridError> {
        let invalid = |reason: String| GridError::InvalidPathwaySpec { reason };
        if raw.schema != PATHWAY_SCHEMA {
            return Err(invalid(format!(
                "schema {:?} is not the supported {PATHWAY_SCHEMA:?}",
                raw.schema
            )));
        }
        let finite_positive = |value: f64, what: &str| -> Result<f64, GridError> {
            if value.is_nan() || value <= 0.0 || !value.is_finite() {
                return Err(GridError::InvalidPathwaySpec {
                    reason: format!("{what} must be positive and finite, got {value}"),
                });
            }
            Ok(value)
        };
        let finite_non_negative = |value: f64, what: &str| -> Result<f64, GridError> {
            if value < 0.0 || !value.is_finite() {
                return Err(GridError::InvalidPathwaySpec {
                    reason: format!("{what} must be non-negative and finite, got {value}"),
                });
            }
            Ok(value)
        };

        let a = raw.assumptions.unwrap_or_default();
        let f0 = Frequency::hertz(finite_positive(a.f0_hz.unwrap_or(50.0), "f0_hz")?);
        // 2019 defaults, flagged (module docs §2/§6).
        let demand_fallback_defaulted = a.demand_gw.is_none();
        let demand_fallback =
            Power::gigawatts(finite_positive(a.demand_gw.unwrap_or(29.0), "demand_gw")?);
        let load_damping_defaulted = a.load_damping_percent_per_hz.is_none();
        let load_damping = Damping::percent_of_demand_per_hertz(finite_non_negative(
            a.load_damping_percent_per_hz.unwrap_or(1.836),
            "load_damping_percent_per_hz",
        )?);
        let duration = Duration::from_seconds(finite_positive(
            a.duration_s.unwrap_or(120.0),
            "duration_s",
        )?);
        let timestep = Duration::from_seconds(
            finite_positive(a.timestep_ms.unwrap_or(10.0), "timestep_ms")? / 1000.0,
        );
        if timestep.as_seconds() > duration.as_seconds() {
            return Err(invalid("timestep exceeds the survival window".to_owned()));
        }
        let survival_floor = Frequency::hertz(finite_positive(
            a.survival_floor_hz.unwrap_or(48.8),
            "survival_floor_hz",
        )?);
        if survival_floor.as_hertz() >= f0.as_hertz() {
            return Err(invalid(format!(
                "survival_floor_hz {} must lie below f0_hz {}",
                survival_floor.as_hertz(),
                f0.as_hertz()
            )));
        }
        let search_max_loss = Power::megawatts(finite_positive(
            a.search_max_loss_mw.unwrap_or(5000.0),
            "search_max_loss_mw",
        )?);
        let search_tolerance = Power::megawatts(finite_positive(
            a.search_tolerance_mw.unwrap_or(1.0),
            "search_tolerance_mw",
        )?);
        if search_tolerance.as_gigawatts() >= search_max_loss.as_gigawatts() {
            return Err(invalid(
                "search_tolerance_mw must be smaller than search_max_loss_mw".to_owned(),
            ));
        }

        let responses_defaulted_to_2019 = a.responses.is_none();
        let responses = match a.responses {
            None => default_2019_responses(),
            Some(raw_responses) => {
                let mut responses = Vec::with_capacity(raw_responses.len());
                for service in raw_responses {
                    let context = format!("responses.{}", service.name);
                    let factor = finite_non_negative(
                        service.delivery_factor,
                        &format!("{context}.delivery_factor"),
                    )?;
                    if factor > 1.0 {
                        return Err(invalid(format!(
                            "{context}: delivery_factor {factor} exceeds 1"
                        )));
                    }
                    if service.sustain_s.is_none() && service.rundown_s.is_some() {
                        return Err(invalid(format!("{context}: rundown_s without sustain_s")));
                    }
                    responses.push(ResponseService {
                        power: Power::megawatts(finite_positive(
                            service.mw,
                            &format!("{context}.mw"),
                        )?),
                        delivery_factor: PerUnit::new(factor),
                        shape: ResponseShape::Dynamic {
                            droop_full_deviation: Frequency::hertz(finite_positive(
                                service.droop_full_deviation_hz,
                                &format!("{context}.droop_full_deviation_hz"),
                            )?),
                        },
                        delay: Duration::from_seconds(finite_non_negative(
                            service.delay_s,
                            &format!("{context}.delay_s"),
                        )?),
                        ramp: Duration::from_seconds(finite_non_negative(
                            service.ramp_s,
                            &format!("{context}.ramp_s"),
                        )?),
                        sustain: match service.sustain_s {
                            Some(s) => Some(Duration::from_seconds(finite_positive(
                                s,
                                &format!("{context}.sustain_s"),
                            )?)),
                            None => None,
                        },
                        rundown: match service.rundown_s {
                            Some(s) => Some(Duration::from_seconds(finite_positive(
                                s,
                                &format!("{context}.rundown_s"),
                            )?)),
                            None => None,
                        },
                        name: service.name,
                    });
                }
                responses
            }
        };

        let dispatch_conditions_defaulted = a.dispatch_conditions.is_none();
        let dispatch_conditions = match a.dispatch_conditions {
            // The documented default band (module docs §1):
            // measurement-anchored to the 2024 reference run's
            // H-weighted synchronised share (min ≈ its low decile,
            // mean ≈ its mean 0.344).
            None => vec![
                DispatchCondition {
                    name: "min".to_owned(),
                    synchronous_dispatch_fraction: PerUnit::new(0.15),
                },
                DispatchCondition {
                    name: "mean".to_owned(),
                    synchronous_dispatch_fraction: PerUnit::new(0.35),
                },
            ],
            Some(raw_conditions) => {
                if raw_conditions.is_empty() {
                    return Err(invalid(
                        "dispatch_conditions is empty — at least one condition is needed \
                         (omit the table entirely for the documented defaults)"
                            .to_owned(),
                    ));
                }
                let mut seen = BTreeSet::new();
                let mut conditions = Vec::with_capacity(raw_conditions.len());
                for condition in raw_conditions {
                    if condition.name.is_empty() {
                        return Err(invalid("dispatch condition with an empty name".to_owned()));
                    }
                    if !seen.insert(condition.name.clone()) {
                        return Err(invalid(format!(
                            "duplicate dispatch condition name {:?}",
                            condition.name
                        )));
                    }
                    let fraction = finite_non_negative(
                        condition.synchronous_dispatch_fraction,
                        &format!("{}.synchronous_dispatch_fraction", condition.name),
                    )?;
                    if fraction > 1.0 {
                        return Err(invalid(format!(
                            "{}.synchronous_dispatch_fraction {fraction} exceeds 1",
                            condition.name
                        )));
                    }
                    conditions.push(DispatchCondition {
                        name: condition.name,
                        synchronous_dispatch_fraction: PerUnit::new(fraction),
                    });
                }
                conditions
            }
        };

        let reference_losses = match a.reference_losses {
            // Cited defaults: data/reference/stability-2019-event.toml
            // [standards_current] largest_loss_planning_mw = 1800 (SQSS
            // infrequent infeed loss since 2014; FRCR 2024 planning
            // largest loss) and [standards_2019]
            // sqss_normal_infeed_loss_mw = 1320 (GSR015).
            None => vec![
                ReferenceLoss {
                    name: "sqss_infrequent_infeed_loss".to_owned(),
                    loss: Power::megawatts(1800.0),
                },
                ReferenceLoss {
                    name: "sqss_normal_infeed_loss".to_owned(),
                    loss: Power::megawatts(1320.0),
                },
            ],
            Some(raw_losses) => {
                let mut losses = Vec::with_capacity(raw_losses.len());
                for loss in raw_losses {
                    losses.push(ReferenceLoss {
                        loss: Power::megawatts(finite_positive(
                            loss.mw,
                            &format!("reference_losses.{}.mw", loss.name),
                        )?),
                        name: loss.name,
                    });
                }
                losses
            }
        };

        if raw.years.is_empty() {
            return Err(invalid(
                "a pathway needs at least one [[years]] entry".to_owned(),
            ));
        }
        let mut years = Vec::with_capacity(raw.years.len());
        let mut previous_year: Option<i32> = None;
        for raw_year in raw.years {
            if let Some(previous) = previous_year
                && raw_year.year <= previous
            {
                return Err(invalid(format!(
                    "years must be strictly increasing ({} follows {previous})",
                    raw_year.year
                )));
            }
            previous_year = Some(raw_year.year);
            let demand = match raw_year.demand_twh {
                None => None,
                // Mean power over a stated 8,760 h year (module docs §6).
                Some(twh) => Some(Power::gigawatts(
                    finite_positive(twh, &format!("years.{}.demand_twh", raw_year.year))? * 1000.0
                        / 8760.0,
                )),
            };
            let mut fleet = Vec::with_capacity(raw_year.fleet.len());
            for entry in raw_year.fleet {
                fleet.push(PathwayFleetEntry {
                    capacity: Power::gigawatts(finite_non_negative(
                        entry.capacity_gw,
                        &format!(
                            "years.{}.fleet.{}.capacity_gw",
                            raw_year.year, entry.technology
                        ),
                    )?),
                    technology: entry.technology,
                });
            }
            let mut storage = Vec::with_capacity(raw_year.storage.len());
            for entry in raw_year.storage {
                storage.push(PathwayStorageEntry {
                    power: Power::gigawatts(finite_non_negative(
                        entry.power_gw,
                        &format!("years.{}.storage.{}.power_gw", raw_year.year, entry.kind),
                    )?),
                    energy: match entry.energy_gwh {
                        None => None,
                        Some(gwh) => Some(Energy::gigawatt_hours(finite_non_negative(
                            gwh,
                            &format!("years.{}.storage.{}.energy_gwh", raw_year.year, entry.kind),
                        )?)),
                    },
                    kind: entry.kind,
                });
            }
            years.push(PathwayYear {
                year: raw_year.year,
                demand,
                fleet,
                storage,
            });
        }

        Ok(Self {
            name: raw.name,
            fes_edition: raw.fes_edition,
            assumptions: PathwayAssumptions {
                f0,
                demand_fallback,
                demand_fallback_defaulted,
                load_damping,
                load_damping_defaulted,
                duration,
                timestep,
                survival_floor,
                search_max_loss,
                search_tolerance,
                responses,
                responses_defaulted_to_2019,
                dispatch_conditions,
                dispatch_conditions_defaulted,
                reference_losses,
            },
            years,
        })
    }
}
