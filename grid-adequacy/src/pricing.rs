//! Stage 2 pricing plumbing: load the scenario's `[pricing]` block
//! (schema v2; formerly the run-inputs `[pricing]` section) into
//! per-period SRMC series and per-technology emission intensities, and
//! price a completed dispatch run.
//!
//! The pricing *model* — the SRMC recipe, the system-marginal-price
//! conventions, revenue/capture/emissions arithmetic — lives in
//! [`grid_core::pricing`] (ADR-9). This module only maps the adequacy
//! engine's [`RunResult`] series through it, purely *reading* the
//! dispatch output: pricing can never perturb dispatch, so the Stage 1
//! pinned result digest is untouched by construction.
//!
//! Scope choices, stated out loud:
//!
//! - **Priced technologies**: revenue, capture price and capture ratio
//!   are computed for every fleet technology (renewables and thermal).
//!   Exogenous must-take series (net imports, pumped-storage net,
//!   FUELHH "other") are *not* technologies and get no revenue
//!   accounting; they also never set the price (grid-core convention 2).
//! - **Renewable revenue and capture carry BOTH bases** (Package A,
//!   ratified 2026-07-03). The two conventions, in prose:
//!
//!   **Potential basis (Stage 2 convention, unchanged):** renewable
//!   output is *potential* (pre-curtailment) output — Stage 1 reports
//!   curtailment as a pooled system quantity with no attribution — so
//!   `energy`, `revenue`, `capture_price` and `capture_ratio` are
//!   computed on the potential series. All previously published fields
//!   keep this basis, their names and their values.
//!
//!   **Delivered basis (additive):** delivered = potential − this
//!   technology's share of the pooled curtailment. Per period, the
//!   pooled curtailment is attributed to the weather-driven renewables
//!   **pro-rata by potential output** (a uniform curtailment fraction
//!   across renewables — see [`delivered_renewable_power`] for the
//!   choice's rationale and its documented ambiguities), and
//!   `energy_delivered`, `revenue_delivered`,
//!   `capture_price_delivered` and `capture_ratio_delivered` are the
//!   same arithmetic on the delivered series. Thermal dispatch *is*
//!   delivered output, so a thermal technology's two bases coincide by
//!   construction.
//!
//!   **Direction of divergence, worked out rather than assumed:**
//!   under the dispatch rules curtailment occurs only in surplus
//!   periods, and surplus periods are must-take-only periods, priced
//!   £0 (grid-core SMP convention 2). Curtailed energy therefore earns
//!   £0 on the potential basis too: the two REVENUE totals are
//!   identical under the current conventions, while delivered ENERGY
//!   is strictly smaller whenever curtailment is nonzero — so the
//!   delivered capture price/ratio is **at or above** the potential
//!   one (the potential basis dilutes the capture price with
//!   zero-revenue energy; it does not overstate revenue in-model).
//!   The bases' revenues would separate only if curtailment ever
//!   coincided with a nonzero price (e.g. future minimum-stable
//!   generation or negative-price features). In the 2024 reference run
//!   curtailment is 0.137 GWh over 2 periods and the bases are
//!   near-identical; they diverge visibly in high-wind sweeps.
//!   Per-increment curtailment attribution is a later-stage analysis
//!   (Q2).
//! - **Emissions** are accounted for technologies with an SRMC recipe
//!   (the gas fleet — the only fleet entries with committed emission
//!   factors). Biomass, coal and the exogenous "other" category carry no
//!   committed factors and are **omitted from the emissions list**
//!   entirely (no entry, not a zero entry) — a documented accounting
//!   gap, not a claim of zero emissions; run totals therefore cover the
//!   gas fleet only.

use std::collections::BTreeMap;
use std::path::Path;

use grid_core::GridError;
use grid_core::prices_reference::PricesReference;
use grid_core::pricing::{
    PricedSeries, capture_price, capture_ratio, carbon_price_step_series, median_ratio,
    monthly_mean_correlation, revenue, srmc_series, system_marginal_price,
    time_weighted_mean_price, total_emissions,
};
use grid_core::scenario::{Scenario, TechId};
use grid_core::time::UtcInstant;
use grid_core::trace::{Trace, load_price_trace};
use grid_core::units::{Duration, Emissions, EmissionsRate, Energy, Money, Power, Price};

use grid_core::scenario::PricingSpec;

use crate::inputs::single_zone;
use crate::result::RunResult;

/// Fully loaded pricing inputs: one SRMC series and one pair of
/// electric-basis emission intensities per priced technology, plus the
/// optional observed benchmark price.
#[derive(Debug, Clone, PartialEq)]
pub struct PricingInputs {
    /// Per-period SRMC per priced technology, £/MWh-electric.
    pub srmc: BTreeMap<TechId, Trace<Price>>,
    /// CO₂-only electric-basis intensity per priced technology
    /// (tCO2/MWh-electric) — the **pricing-consistent** factor.
    pub ef_electric_co2: BTreeMap<TechId, EmissionsRate>,
    /// CO₂e electric-basis intensity per priced technology
    /// (tCO2e/MWh-electric) — the **accounting** factor.
    pub ef_electric_co2e: BTreeMap<TechId, EmissionsRate>,
    /// Observed market price for the reported realism statistics.
    pub observed_price: Option<Trace<Price>>,
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

/// Load and validate everything the pricing layer needs: the committed
/// prices-reference file, one fuel-price trace per declared fuel, the
/// UKA+CPS carbon step series over the scenario horizon, and one SRMC
/// series + emission-intensity pair per technology with a declared
/// recipe. Relative paths are resolved against `base_dir`.
///
/// Errors with [`GridError::InvalidRunInputs`] when a recipe names a
/// technology outside the fleet, a weather-driven (must-take)
/// technology, an undeclared fuel, a fuel without reference emission
/// factors, or an unknown efficiency key.
pub fn load_pricing_inputs(
    scenario: &Scenario,
    spec: &PricingSpec,
    base_dir: &Path,
) -> Result<PricingInputs, GridError> {
    let zone = single_zone(scenario)?;
    let periods = scenario.horizon.period_count()?;
    let start = scenario.horizon.start_instant()?;

    let reference = PricesReference::load(&base_dir.join(&spec.reference))?;

    // One carbon step series (UKA auctions + CPS) for the whole run.
    let carbon = carbon_price_step_series(&reference.uka_auctions, reference.cps, start, periods)?;

    // Fuel-price traces, £/MWh-thermal HHV (the trace's own unit; the
    // reference file pins the HHV-consistency rule).
    let mut fuel_prices: BTreeMap<&str, Trace<Price>> = BTreeMap::new();
    for (fuel, trace_ref) in &spec.fuel_price {
        let trace = load_price_trace(&base_dir.join(&trace_ref.path), &trace_ref.column, periods)?;
        check_start(&trace_ref.path, start, trace.start())?;
        fuel_prices.insert(fuel, trace);
    }

    let mut srmc = BTreeMap::new();
    let mut ef_electric_co2 = BTreeMap::new();
    let mut ef_electric_co2e = BTreeMap::new();
    for (tech, recipe) in &spec.srmc {
        let entry = zone
            .fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .ok_or_else(|| GridError::InvalidRunInputs {
                reason: format!(
                    "pricing.srmc names technology {tech:?}, which is not in the fleet"
                ),
            })?;
        if entry.capacity_factor_trace.is_some() {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "pricing.srmc names weather-driven technology {tech:?}; must-take \
                     technologies carry no SRMC model (grid-core pricing convention 1)"
                ),
            });
        }
        let fuel_price =
            fuel_prices
                .get(recipe.fuel.as_str())
                .ok_or_else(|| GridError::InvalidRunInputs {
                    reason: format!(
                        "pricing.srmc.{tech} names fuel {:?}, which has no \
                         pricing.fuel_price entry",
                        recipe.fuel
                    ),
                })?;
        // Emission factors per fuel: prices-reference-v1 carries natural
        // gas only.
        if recipe.fuel != "gas" {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "pricing.srmc.{tech} names fuel {:?}; prices-reference-v1 carries \
                     emission factors for \"gas\" (natural gas) only",
                    recipe.fuel
                ),
            });
        }
        let efficiency = *reference
            .efficiency_hhv
            .get(&recipe.efficiency)
            .ok_or_else(|| GridError::InvalidRunInputs {
                reason: format!(
                    "pricing.srmc.{tech} names efficiency key {:?}, which is not in the \
                     prices-reference file",
                    recipe.efficiency
                ),
            })?;
        let tech_id = TechId::new(tech.clone());
        srmc.insert(
            tech_id.clone(),
            srmc_series(fuel_price, &carbon, efficiency, reference.ef_co2_thermal)?,
        );
        ef_electric_co2.insert(tech_id.clone(), reference.ef_co2_thermal / efficiency);
        ef_electric_co2e.insert(tech_id, reference.ef_co2e_thermal / efficiency);
    }

    let observed_price = match &spec.observed_price {
        Some(trace_ref) => {
            let trace =
                load_price_trace(&base_dir.join(&trace_ref.path), &trace_ref.column, periods)?;
            check_start(&trace_ref.path, start, trace.start())?;
            Some(trace)
        }
        None => None,
    };

    Ok(PricingInputs {
        srmc,
        ef_electric_co2,
        ef_electric_co2e,
        observed_price,
    })
}

/// One technology's revenue accounting, on both bases (module docs):
/// the potential-basis fields keep their Stage 2 names and values; the
/// `*_delivered` fields are the same arithmetic on the delivered
/// (post-curtailment) series.
#[derive(Debug, Clone, PartialEq)]
pub struct TechPricing {
    /// The technology.
    pub tech: TechId,
    /// Total dispatched energy (potential output for renewables — see
    /// module docs).
    pub energy: Energy,
    /// Revenue at the system marginal price: `Σ dispatch × Δt × SMP`.
    pub revenue: Money,
    /// Capture price (revenue / energy); `None` for zero output.
    pub capture_price: Option<Price>,
    /// Capture price / time-weighted mean SMP; `None` for zero output.
    pub capture_ratio: Option<f64>,
    /// Delivered (post-curtailment) energy; equals `energy` for
    /// thermal technologies and whenever curtailment is zero.
    pub energy_delivered: Energy,
    /// Revenue on the delivered series. Identical to `revenue` under
    /// the current SMP conventions (curtailment prices at £0 — module
    /// docs); carried separately so the identity is checkable and
    /// survives any future pricing change.
    pub revenue_delivered: Money,
    /// Delivered-basis capture price; `None` for zero delivered
    /// output. At or above `capture_price` in-model (module docs).
    pub capture_price_delivered: Option<Price>,
    /// Delivered-basis capture ratio; `None` for zero delivered
    /// output.
    pub capture_ratio_delivered: Option<f64>,
}

/// One technology's emissions accounting, both bases carried and
/// labelled (CO₂-only is the pricing-consistent basis; CO₂e is the
/// accounting basis — reference-file rule).
#[derive(Debug, Clone, PartialEq)]
pub struct TechEmissions {
    /// The technology.
    pub tech: TechId,
    /// Combustion CO₂ (pricing basis), tonnes.
    pub co2: Emissions,
    /// Total CO₂e incl. CH₄ and N₂O (accounting basis), tonnes.
    pub co2e: Emissions,
}

/// The reported (not gated) model-price realism statistics of docs/04
/// Stage 2, computed against the observed benchmark trace.
#[derive(Debug, Clone, PartialEq)]
pub struct RealismStats {
    /// Median of per-period model-SMP / observed price ratios.
    pub median_model_over_observed: f64,
    /// Pearson correlation of calendar-month mean prices.
    pub monthly_correlation: f64,
}

/// The pricing layer's output for one dispatch run.
#[derive(Debug, Clone, PartialEq)]
pub struct PricingResult {
    /// System marginal price per period.
    pub smp: Vec<Price>,
    /// Price-setting technology per period (`None` in must-take-only
    /// periods).
    pub setter: Vec<Option<TechId>>,
    /// Time-weighted mean SMP over the run.
    pub smp_time_weighted_mean: Price,
    /// Number of periods with unserved energy (0 in 2024; such periods
    /// price at the fleet SRMC ceiling — grid-core convention 3).
    pub unserved_periods: usize,
    /// Revenue accounting per fleet technology (renewables in scenario
    /// order, then thermal in merit order).
    pub technologies: Vec<TechPricing>,
    /// Emissions per priced technology.
    pub emissions: Vec<TechEmissions>,
    /// Run total, CO₂-only (pricing basis).
    pub total_co2: Emissions,
    /// Run total, CO₂e (accounting basis).
    pub total_co2e: Emissions,
    /// Realism statistics, when an observed benchmark was declared.
    pub realism: Option<RealismStats>,
}

/// Delivered (post-curtailment) output per weather-driven renewable:
/// one series per entry of `result.renewables`, in the same order.
///
/// The pooled-curtailment convention (Stage 1) reports curtailment as
/// one system series with no attribution; the delivered basis needs an
/// allocation, so the rule is stated here in full prose:
///
/// 1. **Pro-rata sharing.** In each period, the attributed curtailment
///    is shared across the renewables in proportion to their potential
///    output — every renewable is curtailed by the same fraction of
///    its potential. This is the only allocation consistent with the
///    pool carrying no ordering information; any priority ordering
///    (curtail solar first, wind first, …) would be an invented
///    dispatch rule the engine does not have. Per-increment / marginal
///    attribution is a separate analysis (Q2), not this function.
/// 2. **Pool membership (documented ambiguity).** The pooled surplus
///    can, in principle, exceed the renewable pool: exogenous
///    must-take series (imports, pumped-storage net, "other") also
///    feed the surplus, and they carry no revenue accounting on either
///    basis. Only `min(curtailment, renewable potential)` is
///    attributed to the renewables — delivered output clamps at zero
///    and the residual spill stays unattributed at the pool level.
///    (Under the current dispatch rules the surplus also contains the
///    exogenous contribution, so this attribution is an upper bound on
///    renewable curtailment — the conservative direction for the
///    delivered basis: it can only lower delivered energy, never raise
///    it above potential.)
/// 3. **Zero-potential periods.** A period with curtailment but zero
///    renewable potential (pure exogenous spill) attributes nothing;
///    delivered = potential = 0, no NaN.
/// 4. **£0-floor periods.** No choice is needed: curtailment periods
///    are must-take-only periods and price at £0 (module docs), which
///    is why the delivered basis changes energy but not revenue under
///    the current SMP conventions.
///
/// Negative curtailment values (never produced by the engine) are
/// treated as zero rather than inflating delivered output above
/// potential.
///
/// Errors with [`GridError::InvalidPricing`] when a renewable series
/// and the curtailment series disagree on the period count.
pub fn delivered_renewable_power(result: &RunResult) -> Result<Vec<Vec<Power>>, GridError> {
    let periods = result.curtailment.len();
    for series in &result.renewables {
        if series.power.len() != periods {
            return Err(GridError::InvalidPricing {
                reason: format!(
                    "delivered output: renewable series {} has {} periods; the curtailment \
                     series has {periods}",
                    series.tech,
                    series.power.len()
                ),
            });
        }
    }
    let zero = Power::gigawatts(0.0);
    let mut delivered: Vec<Vec<Power>> = result
        .renewables
        .iter()
        .map(|_| Vec::with_capacity(periods))
        .collect();
    for t in 0..periods {
        let pool = result
            .renewables
            .iter()
            .fold(zero, |acc, s| acc + s.power[t]);
        // Rule 2: attribute at most the renewable pool; treat negative
        // curtailment as zero (prose above).
        let curtailed = if result.curtailment[t] < zero {
            zero
        } else if result.curtailment[t] < pool {
            result.curtailment[t]
        } else {
            pool
        };
        // Rule 1 (pro-rata) via the kept fraction, exact at both ends:
        // 1 − 0/pool = 1 and 1 − pool/pool = 0 in IEEE arithmetic.
        let kept = if pool > zero {
            1.0 - curtailed.as_gigawatts() / pool.as_gigawatts()
        } else {
            1.0 // rule 3: nothing to attribute, delivered = potential
        };
        for (out, series) in delivered.iter_mut().zip(&result.renewables) {
            out.push(series.power[t] * kept);
        }
    }
    Ok(delivered)
}

/// Price a completed dispatch run under the grid-core pricing
/// conventions. Pure function of `(result, inputs)` (ADR-5); reads the
/// dispatch output, never modifies it.
pub fn price_run(result: &RunResult, inputs: &PricingInputs) -> Result<PricingResult, GridError> {
    // The SRMC-bearing (thermal) series drive the SMP; renewables and
    // exogenous must-take never set the price (module docs).
    let mut series = Vec::with_capacity(result.thermal.len());
    for thermal in &result.thermal {
        series.push(PricedSeries {
            tech: thermal.tech.clone(),
            power: &thermal.power,
            srmc: inputs.srmc.get(&thermal.tech).map(|t| t.values()),
        });
    }
    let prices = system_marginal_price(&series, &result.unserved)?;

    let smp_time_weighted_mean =
        time_weighted_mean_price(&prices.smp).ok_or_else(|| GridError::InvalidPricing {
            reason: "cannot price a run with no periods".to_owned(),
        })?;
    let unserved_periods = result
        .unserved
        .iter()
        .filter(|&&u| u > Power::gigawatts(0.0))
        .count();

    // Both revenue bases per technology (module docs): the potential
    // basis on the recorded series, the delivered basis on the
    // pro-rata post-curtailment series for renewables; thermal
    // dispatch IS delivered output, so its bases share one series.
    let delivered = delivered_renewable_power(result)?;
    let series_energy = |power: &[Power]| {
        power
            .iter()
            .map(|&p| p * Duration::half_hour())
            .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
    };
    let mut technologies = Vec::new();
    for (index, tech_series) in result.renewables.iter().chain(&result.thermal).enumerate() {
        let delivered_power: &[Power] = delivered
            .get(index)
            .map_or(&tech_series.power, Vec::as_slice);
        technologies.push(TechPricing {
            tech: tech_series.tech.clone(),
            energy: series_energy(&tech_series.power),
            revenue: revenue(&tech_series.power, &prices.smp)?,
            capture_price: capture_price(&tech_series.power, &prices.smp)?,
            capture_ratio: capture_ratio(&tech_series.power, &prices.smp)?,
            energy_delivered: series_energy(delivered_power),
            revenue_delivered: revenue(delivered_power, &prices.smp)?,
            capture_price_delivered: capture_price(delivered_power, &prices.smp)?,
            capture_ratio_delivered: capture_ratio(delivered_power, &prices.smp)?,
        });
    }

    let mut emissions = Vec::new();
    let mut total_co2 = Emissions::tonnes_co2(0.0);
    let mut total_co2e = Emissions::tonnes_co2(0.0);
    for thermal in &result.thermal {
        let Some(&co2_rate) = inputs.ef_electric_co2.get(&thermal.tech) else {
            continue;
        };
        let Some(&co2e_rate) = inputs.ef_electric_co2e.get(&thermal.tech) else {
            continue;
        };
        let co2 = total_emissions(&thermal.power, co2_rate);
        let co2e = total_emissions(&thermal.power, co2e_rate);
        total_co2 = total_co2 + co2;
        total_co2e = total_co2e + co2e;
        emissions.push(TechEmissions {
            tech: thermal.tech.clone(),
            co2,
            co2e,
        });
    }

    let realism = match &inputs.observed_price {
        Some(observed) => Some(RealismStats {
            median_model_over_observed: median_ratio(&prices.smp, observed.values())?,
            monthly_correlation: monthly_mean_correlation(
                result.start,
                &prices.smp,
                observed.values(),
            )?,
        }),
        None => None,
    };

    Ok(PricingResult {
        smp: prices.smp,
        setter: prices.setter,
        smp_time_weighted_mean,
        unserved_periods,
        technologies,
        emissions,
        total_co2,
        total_co2e,
        realism,
    })
}
