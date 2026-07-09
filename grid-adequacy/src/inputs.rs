//! Loading a scenario's run inputs: demand, capacity-factor traces,
//! exogenous supply and availability models, all horizon-aligned.
//!
//! Schema v2 (Stage 3) made the scenario self-contained again — the
//! Stage 1 run-inputs companion file is gone and everything this module
//! loads is named by scenario fields (`grid_core::scenario`). The ADR-5
//! determinism formula is back to
//! `results = f(scenario, data pack, engine)`.
//!
//! ## Conventions pinned here
//!
//! - **CF-trace column**: every `capacity_factor_trace` parquet is read
//!   from its [`CF_COLUMN`] (`"cf"`) column — the ERA5 pipeline output
//!   convention (docs/notes/era5-cf-2024-report.md). A schema-level
//!   column selector was weighed and rejected in Stage 1: the data
//!   pipeline already guarantees the convention, and the loader's
//!   missing-column error names both file and column.
//! - **Demand column**: the demand `base_profile` parquet is
//!   multi-column; the column is chosen by `zones.demand.column`,
//!   defaulting to `underlying_demand` (the D3 total-generation
//!   convention, docs/notes/d3-embedded-convention.md).
//! - **Adjusted demand**: `demand(t) = (base(t) + Σ extras(t)) ×
//!   annual_scale + extra_demand_gw + heating(t)` (extras = the
//!   schema-v4 `extra_profiles` aggregate-zone sum). The constant adder
//!   carries supply-side load that underlying demand excludes (station
//!   transformer load ≈ 0.667 GW in 2024 —
//!   `scenarios/gb-2024-reference.toml` `extra_demand_gw`, guarded by
//!   the acceptance_2024 calibration characterisation test); it is a
//!   validation-harness wedge correction, not consumer demand, which is
//!   why it is not subject to `annual_scale`.
//! - **Heating overlay** (schema v5, Q5/D9): when the zone carries a
//!   `[zones.demand.heating]` portfolio, its electrical demand is
//!   computed by `grid_core::heating` (COP defaults from the
//!   drift-guarded `data/reference/heating-cop.toml`, resolved against
//!   the base directory; the temperature trace from the scenario's
//!   pinned reference) and ADDED to demand before dispatch — nothing
//!   else changes (D9 rule 1). Heating is not subject to
//!   `annual_scale`: it carries its own quantum. The computed overlay
//!   (per-entry series, delivered heat, echoed constants) rides on
//!   [`RunInputs::heating`] for the output layer. Block absent ⇒
//!   `heating = None` and the demand arithmetic is byte-identical to
//!   pre-v5.
//! - **Exogenous must-take supply**: named MW-column sums from pack
//!   parquet file(s), treated by the engine as must-take supply
//!   (negative = export / pumping load). Used for net imports (modelled
//!   in Stage 5) and the FUELHH "other" category (no fleet entry).
//!   Entries flagged `imports = true` feed the run's imports accounting.
//! - **Multi-file traces**: every trace reference may list consecutive
//!   per-year files, concatenated in order (docs/04 Stage 3 multi-year
//!   horizons; `grid_core::trace` validates continuity).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use grid_core::GridError;
use grid_core::heating::{HEATING_COP_REFERENCE_PATH, HeatingCopReference, HeatingOverlay};
use grid_core::prices_reference::PricesReference;
use grid_core::pricing::{carbon_price_step_series, srmc_series};
use grid_core::scenario::{
    AvailabilitySpec, ExogenousReliability, LinkCapabilityTraceSpec, Scenario, TechId, TraceFiles,
    ZoneId, ZonePricingSpec, ZoneSpec,
};
use grid_core::time::{HALF_HOUR_MICROS, UtcInstant};
use grid_core::trace::{
    Trace, load_per_unit_trace_concat, load_power_trace_mw, load_power_trace_mw_concat,
    load_price_trace, load_sparse_power_trace_mw, load_temperature_trace_c,
};
use grid_core::units::{Duration, Energy, PerUnit, Power, Price};

use crate::availability::AvailabilityModel;

/// The pinned value column of every capacity-factor trace parquet.
pub const CF_COLUMN: &str = "cf";

/// Fully loaded, horizon-aligned run inputs, ready for
/// [`crate::dispatch::run`]. All traces cover exactly the scenario
/// horizon (same start, same period count).
#[derive(Debug, Clone, PartialEq)]
pub struct RunInputs {
    /// Adjusted demand per period:
    /// `base × annual_scale + extra + heating` (heating already summed
    /// in — the dispatch engine sees heating inside demand, no
    /// special-casing; D9 rule 6b).
    pub demand: Trace<Power>,
    /// Capacity-factor trace per weather-driven technology.
    pub capacity_factors: BTreeMap<TechId, Trace<PerUnit>>,
    /// Exogenous must-take supply series.
    pub exogenous: Vec<ExogenousSupply>,
    /// Availability per technology; technologies absent here default to
    /// flat 1.0 in the engine.
    pub availability: BTreeMap<TechId, AvailabilityModel>,
    /// The computed heating overlay when the zone declares a heating
    /// portfolio (schema v5): per-entry electrical series, delivered
    /// heat, and the echoed constants for the output layer. Its
    /// `electrical_total` is ALREADY included in `demand`. `None` for
    /// scenarios without a heating block (the untouched byte-path).
    pub heating: Option<HeatingOverlay>,
}

/// One loaded exogenous must-take supply series.
#[derive(Debug, Clone, PartialEq)]
pub struct ExogenousSupply {
    /// Output-series label.
    pub label: String,
    /// Whether this series feeds the run's imports accounting.
    pub imports: bool,
    /// Reliability classification (always explicit on exogenous
    /// entries — see `grid_core::scenario::ExogenousSupplySpec`).
    pub reliability: ExogenousReliability,
    /// Net supply per period (negative = export / pumping load).
    pub trace: Trace<Power>,
}

/// Check a loaded trace starts exactly at the horizon start.
fn check_start(context: &str, expected: UtcInstant, found: UtcInstant) -> Result<(), GridError> {
    if found != expected {
        return Err(GridError::TraceStartMismatch {
            context: context.to_owned(),
            expected,
            found,
        });
    }
    Ok(())
}

/// Resolve a [`TraceFiles`] reference against the run's base directory.
fn resolve(files: &TraceFiles, base_dir: &Path) -> Vec<PathBuf> {
    files.paths().iter().map(|p| base_dir.join(p)).collect()
}

/// Convert a scenario availability model into the engine's validated
/// form.
fn to_model(spec: &AvailabilitySpec, tech: &TechId) -> Result<AvailabilityModel, GridError> {
    match spec {
        AvailabilitySpec::Flat { flat } => AvailabilityModel::flat(*flat),
        AvailabilitySpec::Monthly { monthly } => {
            let factors: [PerUnit; 12] =
                monthly
                    .clone()
                    .try_into()
                    .map_err(|_| GridError::InvalidScenario {
                        reason: format!(
                            "availability for {tech}: a monthly profile needs exactly 12 \
                             factors, got {}",
                            monthly.len()
                        ),
                    })?;
            AvailabilityModel::monthly(factors)
        }
    }
}

/// Load and align every input a dispatch run needs: the zone's demand
/// (adjusted per the scenario), one CF trace per weather-driven fleet
/// entry (column [`CF_COLUMN`]), the exogenous supply series, and
/// validated availability models. Relative trace paths are resolved
/// against `base_dir`. Runs [`Scenario::validate`] first.
pub fn load_run_inputs(scenario: &Scenario, base_dir: &Path) -> Result<RunInputs, GridError> {
    scenario.validate()?;
    let zone = single_zone(scenario)?;
    let periods = scenario.horizon.period_count()?;
    let start = scenario.horizon.start_instant()?;
    load_zone_inputs(zone, periods, start, base_dir)
}

/// The per-zone body of [`load_run_inputs`] (shared with the Stage 5
/// multi-zone loader — a pure refactor of the single-zone path).
fn load_zone_inputs(
    zone: &ZoneSpec,
    periods: usize,
    start: UtcInstant,
    base_dir: &Path,
) -> Result<RunInputs, GridError> {
    // The heating overlay (schema v5, D9): computed from the zone's
    // pinned temperature trace and the drift-guarded COP reference
    // file, then ADDED to demand below. Block absent ⇒ None and the
    // demand arithmetic below is byte-identical to pre-v5.
    let heating = match &zone.demand.heating {
        None => None,
        Some(spec) => {
            let reference = HeatingCopReference::load(&base_dir.join(HEATING_COP_REFERENCE_PATH))?;
            let t_pop = load_temperature_trace_c(
                &base_dir.join(&spec.temperature_trace.path),
                &spec.temperature_trace.column,
            )?;
            Some(grid_core::heating::compute_overlay(
                spec, &reference, &t_pop, start, periods,
            )?)
        }
    };

    // Demand: (base + Σ extra_profiles) × annual_scale + extra_demand_gw
    // + heating (module docs; extra_profiles is the schema-v4
    // aggregate-zone sum; heating is not subject to annual_scale).
    let base = load_power_trace_mw_concat(
        &resolve(&zone.demand.base_profile, base_dir),
        &zone.demand.column,
        periods,
    )?;
    check_start(&zone.demand.base_profile.to_string(), start, base.start())?;
    let mut summed: Vec<Power> = base.values().to_vec();
    for extra_ref in &zone.demand.extra_profiles {
        let trace =
            load_power_trace_mw(&base_dir.join(&extra_ref.path), &extra_ref.column, periods)?;
        check_start(
            &format!("{} [{}]", extra_ref.path, extra_ref.column),
            start,
            trace.start(),
        )?;
        for (acc, &p) in summed.iter_mut().zip(trace.values()) {
            *acc = *acc + p;
        }
    }
    let scale = zone.demand.annual_scale;
    let extra = zone.demand.extra_demand_gw;
    let demand = match &heating {
        // The pre-v5 arithmetic, byte-identical when no heating block
        // exists (D9 rule 1: old pins never move).
        None => Trace::from_parts(
            base.start(),
            summed.iter().map(|&p| p * scale + extra).collect(),
        )?,
        Some(overlay) => Trace::from_parts(
            base.start(),
            summed
                .iter()
                .zip(&overlay.electrical_total)
                .map(|(&p, &h)| p * scale + extra + h)
                .collect(),
        )?,
    };

    // Capacity-factor traces, one per weather-driven fleet entry.
    let mut capacity_factors = BTreeMap::new();
    for entry in &zone.fleet {
        let Some(trace_files) = &entry.capacity_factor_trace else {
            continue;
        };
        let trace =
            load_per_unit_trace_concat(&resolve(trace_files, base_dir), CF_COLUMN, periods)?;
        check_start(&trace_files.to_string(), start, trace.start())?;
        if let Some(bad) = trace
            .values()
            .iter()
            .find(|cf| !(0.0..=1.0).contains(&cf.value()))
        {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "capacity-factor trace {trace_files} has value {} outside 0.0..=1.0",
                    bad.value()
                ),
            });
        }
        capacity_factors.insert(entry.technology.clone(), trace);
    }

    // Exogenous supply: sum the named MW columns of each file set.
    let mut exogenous = Vec::with_capacity(zone.exogenous_supply.len());
    for supply in &zone.exogenous_supply {
        if supply.columns.is_empty() {
            return Err(GridError::InvalidRunInputs {
                reason: format!("exogenous supply {:?} lists no columns", supply.label),
            });
        }
        let paths = resolve(&supply.path, base_dir);
        let mut sum: Vec<Power> = vec![Power::gigawatts(0.0); periods];
        for column in &supply.columns {
            let trace = load_power_trace_mw_concat(&paths, column, periods)?;
            check_start(&format!("{} [{column}]", supply.path), start, trace.start())?;
            for (acc, &p) in sum.iter_mut().zip(trace.values()) {
                *acc = *acc + p;
            }
        }
        // Schema v6 `scale`: the flat split multiplier. Skipped at the
        // default so pre-v6 arithmetic stays byte-identical.
        if supply.scale != 1.0 {
            for p in &mut sum {
                *p = *p * supply.scale;
            }
        }
        exogenous.push(ExogenousSupply {
            label: supply.label.clone(),
            imports: supply.imports,
            reliability: supply.reliability,
            trace: Trace::from_parts(start, sum)?,
        });
    }

    // Availability models from the fleet entries (schema v2).
    let mut availability = BTreeMap::new();
    for entry in &zone.fleet {
        if let Some(spec) = &entry.availability {
            availability.insert(entry.technology.clone(), to_model(spec, &entry.technology)?);
        }
    }

    Ok(RunInputs {
        demand,
        capacity_factors,
        exogenous,
        availability,
        heating,
    })
}

/// The scenario's single zone, or [`GridError::MultiZoneUnsupported`]
/// (ADR-7: the schema is multi-zone; the single-zone `run` path stays
/// single-zone — multi-zone scenarios run under
/// [`crate::multizone::run_multi`] since Stage 5).
pub(crate) fn single_zone(scenario: &Scenario) -> Result<&ZoneSpec, GridError> {
    match scenario.zones.as_slice() {
        [zone] => Ok(zone),
        zones => Err(GridError::MultiZoneUnsupported { found: zones.len() }),
    }
}

// ---------------------------------------------------------------------
// Stage 5 multi-zone inputs.
// ---------------------------------------------------------------------

/// One fleet entry's loaded energy-budget schedule (schema v4
/// `energy_budget`): per-window release energies, computed by summing
/// the named MW columns of the budget trace over consecutive windows of
/// `window_periods` half-hours from the horizon start (the last window
/// may be short). Engine semantics — greedy release with carry-forward
/// — are documented at [`crate::multizone`].
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetSchedule {
    /// Window length, half-hourly periods.
    pub window_periods: usize,
    /// Release energy per window, in window order; exactly
    /// `ceil(horizon periods / window_periods)` entries.
    pub windows: Vec<Energy>,
}

/// One zone's loaded inputs for a multi-zone run.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneInputs {
    /// The zone this belongs to (must match the scenario's `[[zones]]`
    /// order and ids — checked by `run_multi`).
    pub id: ZoneId,
    /// The zone's demand/CF/exogenous/availability inputs (identical
    /// shape to the single-zone path).
    pub inputs: RunInputs,
    /// Budget schedules for fleet entries carrying `energy_budget`,
    /// keyed by technology.
    pub budgets: BTreeMap<TechId, BudgetSchedule>,
    /// The zone's pricing inputs for the priced flow signal (schema
    /// v7, D11): `Some` when the zone declares `[zones.pricing]`.
    /// Never consulted under the `scarcity` flow signal — the default
    /// path is byte-untouched.
    pub pricing: Option<ZonePricingInputs>,
}

/// A zone's loaded SRMC chain for the priced flow signal (schema v7,
/// D11): one per-period SRMC series per priced technology, computed by
/// the Stage 2 recipe (`grid_core::pricing::srmc_series`, reused
/// unchanged) under the zone's declared carbon basis — the reference
/// file's UKA+CPS step series when `carbon_flat_gbp_per_tco2` is
/// absent, else the flat per-zone level.
#[derive(Debug, Clone, PartialEq)]
pub struct ZonePricingInputs {
    /// Per-period SRMC per priced technology, £/MWh-electric.
    pub srmc: BTreeMap<TechId, Trace<Price>>,
}

/// Load one zone's `[zones.pricing]` block into SRMC series (see
/// [`ZonePricingInputs`]). Fleet-membership and fuel-declaration
/// coherence are checked by [`Scenario::validate`]; this loader
/// enforces the data-level rules (prices-reference-v1 carries emission
/// factors for `"gas"` only; the efficiency key must exist; traces
/// must align with the horizon).
fn load_zone_pricing(
    zone: &ZoneSpec,
    spec: &ZonePricingSpec,
    periods: usize,
    start: UtcInstant,
    base_dir: &Path,
) -> Result<ZonePricingInputs, GridError> {
    let reference = PricesReference::load(&base_dir.join(&spec.reference))?;

    // The zone's carbon series (struct docs of `ZonePricingSpec`).
    let carbon = match spec.carbon_flat_gbp_per_tco2 {
        Some(flat) => Trace::from_parts(start, vec![flat; periods])?,
        None => carbon_price_step_series(&reference.uka_auctions, reference.cps, start, periods)?,
    };

    let mut fuel_prices: BTreeMap<&str, Trace<Price>> = BTreeMap::new();
    for (fuel, trace_ref) in &spec.fuel_price {
        let trace = load_price_trace(&base_dir.join(&trace_ref.path), &trace_ref.column, periods)?;
        check_start(&trace_ref.path, start, trace.start())?;
        fuel_prices.insert(fuel, trace);
    }

    let mut srmc = BTreeMap::new();
    for (tech, recipe) in &spec.srmc {
        if recipe.fuel != "gas" {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "zone {}, pricing.srmc.{tech} names fuel {:?}; prices-reference-v1 \
                     carries emission factors for \"gas\" (natural gas) only",
                    zone.id, recipe.fuel
                ),
            });
        }
        let fuel_price =
            fuel_prices
                .get(recipe.fuel.as_str())
                .ok_or_else(|| GridError::InvalidRunInputs {
                    reason: format!(
                        "zone {}, pricing.srmc.{tech} names fuel {:?}, which has no \
                         fuel_price entry",
                        zone.id, recipe.fuel
                    ),
                })?;
        let efficiency = *reference
            .efficiency_hhv
            .get(&recipe.efficiency)
            .ok_or_else(|| GridError::InvalidRunInputs {
                reason: format!(
                    "zone {}, pricing.srmc.{tech} names efficiency key {:?}, which is not \
                     in the prices-reference file",
                    zone.id, recipe.efficiency
                ),
            })?;
        srmc.insert(
            TechId::new(tech.clone()),
            srmc_series(fuel_price, &carbon, efficiency, reference.ef_co2_thermal)?,
        );
    }
    Ok(ZonePricingInputs { srmc })
}

/// One link's loaded per-period forward capability (schema v6
/// `capability_trace`): the observed sparse series aligned to the
/// horizon under the scenario's declared sentinel handling
/// (docs/notes/b6-two-zone-data-review.md §6a, restated at
/// `grid_core::scenario::LinkCapabilityTraceSpec`).
#[derive(Debug, Clone, PartialEq)]
pub struct LinkCapability {
    /// Forward (`from → to`) capability per horizon period, GW —
    /// sentinel-handled and mask-filled: what dispatch runs against
    /// (link `availability` is applied by the engine, not here).
    pub forward: Vec<Power>,
    /// Whether the period's capability is OBSERVED. `false` for missing
    /// rows, NaN rows and zero-limit sentinels — those periods carry
    /// the pinned `masked_fill_gw` for dispatch but are **excluded from
    /// validation-gate arithmetic** (the ruling: missing stays
    /// missing). High sentinels (≥ `sentinel_high_mw`) are observed
    /// "no constraint recorded" states and stay in gates at
    /// `upper_bound_gw`.
    pub observed: Vec<bool>,
}

/// Align a sparse observed capability series to the horizon under the
/// scenario's declared sentinel rules (see [`LinkCapability`]). Pure
/// function of its arguments; points outside the horizon are ignored;
/// points off the half-hourly grid and negative capabilities are
/// structured errors.
pub fn build_link_capability(
    points: &[(UtcInstant, Option<Power>)],
    spec: &LinkCapabilityTraceSpec,
    start: UtcInstant,
    periods: usize,
) -> Result<LinkCapability, GridError> {
    let mut forward = vec![spec.masked_fill_gw; periods];
    let mut observed = vec![false; periods];
    for &(t, value) in points {
        let offset = t.unix_micros() - start.unix_micros();
        if offset < 0 {
            continue;
        }
        if offset % HALF_HOUR_MICROS != 0 {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "link capability trace {}: row at {t} is not on the half-hourly \
                     settlement grid starting {start}",
                    spec.path
                ),
            });
        }
        let index = (offset / HALF_HOUR_MICROS) as usize;
        if index >= periods {
            continue;
        }
        let Some(value) = value else {
            continue; // NaN/null row: stays masked at the fill.
        };
        let mw = value.as_gigawatts() * 1000.0;
        if mw < 0.0 {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "link capability trace {}: negative capability {mw} MW at {t}",
                    spec.path
                ),
            });
        }
        if mw >= spec.sentinel_high_mw.as_gigawatts() * 1000.0 {
            // "No constraint recorded" sentinel → the pinned planning
            // upper bound; the period stays observed (in gates).
            forward[index] = spec.upper_bound_gw;
            observed[index] = true;
        } else if mw == 0.0 {
            // Zero-limit sentinel: unobserved unless corroborated as a
            // real outage (the ruling) — masked, filled.
        } else {
            forward[index] = value;
            observed[index] = true;
        }
    }
    Ok(LinkCapability { forward, observed })
}

/// Loaded, horizon-aligned inputs for every zone of a multi-zone run.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiZoneInputs {
    /// Per-zone inputs, in the scenario's `[[zones]]` order.
    pub zones: Vec<ZoneInputs>,
    /// Per-link capability inputs, in the scenario's `[[links]]` order:
    /// `Some` for links declaring a schema-v6 `capability_trace`. May
    /// be empty when no link declares one (synthetic-input ergonomics);
    /// the engine errors on a trace-declaring link without its entry.
    pub link_capabilities: Vec<Option<LinkCapability>>,
}

/// Load run inputs for every zone of a (possibly multi-zone) scenario,
/// including the schema-v4 energy-budget schedules. Relative trace
/// paths are resolved against `base_dir`. Runs [`Scenario::validate`]
/// first.
pub fn load_multi_zone_inputs(
    scenario: &Scenario,
    base_dir: &Path,
) -> Result<MultiZoneInputs, GridError> {
    scenario.validate()?;
    let periods = scenario.horizon.period_count()?;
    let start = scenario.horizon.start_instant()?;
    let mut zones = Vec::with_capacity(scenario.zones.len());
    for zone in &scenario.zones {
        let inputs = load_zone_inputs(zone, periods, start, base_dir)?;
        let mut budgets = BTreeMap::new();
        for entry in &zone.fleet {
            let Some(budget) = &entry.energy_budget else {
                continue;
            };
            // Sum the named MW columns of the budget trace (same rule
            // as exogenous supply), then integrate per window.
            let paths = resolve(&budget.trace, base_dir);
            let mut sum: Vec<Power> = vec![Power::gigawatts(0.0); periods];
            for column in &budget.columns {
                let trace = load_power_trace_mw_concat(&paths, column, periods)?;
                check_start(
                    &format!("{} [{column}]", budget.trace),
                    start,
                    trace.start(),
                )?;
                for (acc, &p) in sum.iter_mut().zip(trace.values()) {
                    *acc = *acc + p;
                }
            }
            let dt = Duration::half_hour();
            let mut windows: Vec<Energy> = Vec::new();
            for (index, &power) in sum.iter().enumerate() {
                if index % budget.window_periods == 0 {
                    windows.push(Energy::gigawatt_hours(0.0));
                }
                // Infallible: a window was just pushed for index 0.
                if let Some(last) = windows.last_mut() {
                    *last = *last + power * dt;
                }
            }
            budgets.insert(
                entry.technology.clone(),
                BudgetSchedule {
                    window_periods: budget.window_periods,
                    windows,
                },
            );
        }
        // Schema v7 (D11): the zone's pricing inputs for the priced
        // flow signal. Loaded whenever declared; consulted only under
        // `flow_signal = "priced_ladder"`.
        let pricing = match &zone.pricing {
            None => None,
            Some(spec) => Some(load_zone_pricing(zone, spec, periods, start, base_dir)?),
        };
        zones.push(ZoneInputs {
            id: zone.id.clone(),
            inputs,
            budgets,
            pricing,
        });
    }

    // Schema v6: per-link observed capability traces (sparse, sentinel-
    // handled per the scenario's declared spec). A missing pack file is
    // a loud TraceFileMissing naming the path.
    let mut link_capabilities = Vec::with_capacity(scenario.links.len());
    for link in &scenario.links {
        link_capabilities.push(match &link.capability_trace {
            None => None,
            Some(spec) => {
                let points = load_sparse_power_trace_mw(&base_dir.join(&spec.path), &spec.column)?;
                Some(build_link_capability(&points, spec, start, periods)?)
            }
        });
    }

    Ok(MultiZoneInputs {
        zones,
        link_capabilities,
    })
}
