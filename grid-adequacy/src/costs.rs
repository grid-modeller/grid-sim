//! Stage 7 package 1: the D8 rule-1 cost stack over a completed
//! adequacy run (`docs/notes/d8-lcoe-methods.md`; the docs/04 Stage 7
//! pin binds every rule referenced below). Like pricing, costing
//! purely *reads* a [`RunResult`] — it can never perturb dispatch, so
//! the pinned Stage 1/2 digests are untouched by construction.
//!
//! # The component lines (D8 rule 1 — the pinned list)
//!
//! 1. **Generation capex annualised + fixed O&M** — per costed asset,
//!    the central overnight capex (incl. site infrastructure) × the
//!    rule-4 CRF at each pinned WACC, plus the source's three fixed
//!    annual lines (fixed O&M + insurance + connection), accrued over
//!    the run horizon.
//! 2. **Variable O&M + fuel + carbon** — gas fuel and carbon come from
//!    the EXISTING Stage 2 SRMC chain on `prices-2024.toml` (D8 rule
//!    1.2: the 2024 actuals are not re-sourced), i.e.
//!    `Σ_t dispatch × Δt × SRMC(t)` per SRMC-bearing technology; VOM —
//!    and, for nuclear, the cost row's fuel and decommissioning/waste
//!    per-MWh figures (no SRMC model exists for nuclear) — are the
//!    cost row's per-MWh rates × that technology's generated energy.
//! 3. **Storage capex + O&M** — power (£/kW) and energy (£/kWh) legs
//!    priced separately (rule 1.3), from the battery row of the costs
//!    reference. The battery row's condition-3 quarantine was LIFTED
//!    2026-07-06 (reviewed act; condition 3.i discharged against the
//!    NREL primary), so consuming it no longer stamps the result
//!    non-quotable — but its 2018-vintage staleness stamp (caveat
//!    3.iii) still travels on every battery-containing artefact.
//!    Hydrogen-store composition (electrolyser + cavern + OCHT legs)
//!    is a later package.
//! 4. **Interconnection** — modelled links' project capex annualised
//!    over a **caller-stated** life (the reference cites no
//!    interconnector asset life; the stated life is a labelled
//!    convention). Import/export *settlement* (rule 8) is not costed
//!    this package — carried as a named limitation on the metadata.
//! 5. **Stability services** — held response volumes priced at the
//!    reference holding costs (£/MW/h, FY2025 EAC volume-weighted
//!    means) over the horizon hours (rule 1.5 / Q8 linkage).
//! 6. **Constraint costs** — a separately labelled ZERO line with a
//!    `pending_d6` flag until D6 resolves the function form; never
//!    silently pooled (rule 1.6).
//!
//! **Unserved energy is NOT priced** into any line (rule 1.7): no VoLL
//! monetising. Reliability is handled by construction — every stack
//! carries the rule-3 stamp (unserved energy + the adequacy standard
//! the scenario was solved to). Curtailment and overbuild carry no
//! separate £ line: their cost IS the capex/O&M already in lines 1/3.
//!
//! # The headline and its denominator (D8 rule 1, edit-4 pin)
//!
//! Headline = total annualised cost ÷ **delivered-to-demand energy**,
//! where delivered-to-demand = GB demand − GB unserved energy over the
//! horizon (the D3 underlying-demand convention). This is a DIFFERENT
//! object from the Package A per-technology delivered series
//! (`TechPricing::energy_delivered` / `delivered_renewable_power`):
//! the two differ by storage losses, boundary flows and unattributed
//! spill, and they are deliberately carried under distinct names.
//! Exports are not GB-demand service and do not enter the denominator.
//!
//! # Accrual convention
//!
//! Annualised (£/yr) lines are accrued over the horizon pro-rata by
//! **calendar-year coverage**: a run spanning exactly calendar 2024
//! accrues exactly one year of annuities (17,568 half-hours of a leap
//! year), a 1985–2024 run exactly 40 — no 8,760-hour approximation.
//! Per-period operating costs (line 2) and holding costs (line 5)
//! accrue directly over the horizon's periods/hours.
//!
//! # WACC banding and framing
//!
//! Every money output is a [`WaccBand`] over the pinned uniform set
//! (4.5/7.5/10.0 % real) — a single-WACC output does not exist as a
//! type. Lines with no capex content are flat across the band, but
//! still carry it. The rule-8 sunk-vs-greenfield framing is a
//! mandatory label ([`CostFraming`]) stamped on the metadata: the
//! stack annuitises exactly the assets the spec lists, and the caller
//! names whether that asset list is a greenfield rebuild or a
//! forward (sunk-excluded) view.
//!
//! # Quarantine propagation (docs/04 Stage 7 pin, corrected form)
//!
//! Consuming a `quotable = false` reference row is LEGAL: the result
//! is computed, stamped non-quotable, and the consumed rows are listed
//! in the metadata. The publish path
//! ([`CostStack::ensure_publishable`]) then refuses a flagged result
//! with a structured [`GridError::NonQuotableResult`]. Publication
//! gates (OCHT/Baringa) and the nuclear bracket rule surface in the
//! metadata the same way; an unmet gate also refuses publication,
//! while a bracket rule is a rendering obligation (quote BOTH nuclear
//! variants) that travels without blocking.

use grid_core::GridError;
use grid_core::costs::{WaccBand, annuity_per_kwh, annuity_per_mw, capital_recovery_factor};
use grid_core::costs_reference::{CostsReference, TechnologyCosts};
use grid_core::units::{Duration, Energy, Money, MoneyRate, PerUnit, Power, Price};

use crate::pricing::PricingInputs;
use crate::result::RunResult;

/// The rule-8 existing-fleet framing — a mandatory label on every cost
/// artefact. The stack always annuitises exactly the assets in the
/// spec; the framing names what that asset list means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostFraming {
    /// "System rebuild cost": every asset alive in the run carries its
    /// rule-4 annuity, as if built today.
    Greenfield,
    /// "Forward cost": sunk capex of already-built assets is excluded
    /// (the caller has left those assets out of the capex spec, or the
    /// scenario explicitly models rebuild/life-extension — the
    /// scenario must say which).
    Forward,
}

/// Which published battery build-vintage prices a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreVintage {
    /// 2030-build medium (the forward-looking pin).
    Build2030,
    /// 2018-build medium (the historical bracket end).
    Build2018,
}

/// One costed generation asset: which run series it prices its
/// variable costs from, which reference row costs it, and the capacity
/// carrying capex + fixed O&M.
#[derive(Debug, Clone, PartialEq)]
pub struct CostedGeneration {
    /// The fleet technology id, matching a `RunResult` series.
    pub tech: String,
    /// The costs-reference `[technologies.*]` row key.
    pub cost_row: String,
    /// Installed capacity (the scenario's, not derivable from the run).
    pub capacity: Power,
}

/// One costed battery store (power + energy legs, D8 rule 1.3).
#[derive(Debug, Clone, PartialEq)]
pub struct CostedBattery {
    /// Output label (matches the store label where one exists).
    pub label: String,
    /// Power rating (the £/kW leg).
    pub power: Power,
    /// Energy capacity (the £/kWh leg).
    pub energy: Energy,
    /// Which published build vintage prices the store.
    pub vintage: StoreVintage,
}

/// One costed interconnector: a reference row key plus the
/// caller-stated annualisation life (the reference cites none).
#[derive(Debug, Clone, PartialEq)]
pub struct CostedLink {
    /// The costs-reference `[interconnectors.*]` row key.
    pub row: String,
    /// Stated asset life for the rule-4 annuity — a labelled
    /// convention, not a cited figure.
    pub life_years: u32,
}

/// One held stability service, priced at the reference holding costs.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceHolding {
    /// Service name, matching a `[holding_costs.*]` key (which matches
    /// `response-holdings-2025.toml` service names).
    pub service: String,
    /// Held volume (constant over the horizon in this package).
    pub held: Power,
}

/// The cost stack's asset and convention inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct CostStackSpec {
    /// Rule-8 framing label (mandatory).
    pub framing: CostFraming,
    /// The rule-3 stamp: the adequacy standard this scenario was
    /// solved to (e.g. "zero unserved over 1985–2024"), or an explicit
    /// statement that it was not solved to one.
    pub adequacy_standard: String,
    /// Costed generation assets.
    pub generation: Vec<CostedGeneration>,
    /// Costed battery stores.
    pub batteries: Vec<CostedBattery>,
    /// Costed interconnectors.
    pub interconnectors: Vec<CostedLink>,
    /// Held stability services.
    pub holdings: Vec<ServiceHolding>,
}

/// The rule-3 reliability stamp carried on every cost artefact.
#[derive(Debug, Clone, PartialEq)]
pub struct ReliabilityStamp {
    /// Total unserved energy of the run.
    pub unserved_energy: Energy,
    /// The adequacy standard the scenario was solved to, verbatim.
    pub adequacy_standard: String,
}

/// Governance metadata stamped on every cost stack (docs/04 Stage 7
/// pin: quarantine flags propagate; the artefact layer refuses flagged
/// results).
#[derive(Debug, Clone, PartialEq)]
pub struct CostMetadata {
    /// Rule-8 framing label.
    pub framing: CostFraming,
    /// The three pinned WACC values the band was computed at.
    pub wacc: WaccBand<PerUnit>,
    /// Rule-3 reliability stamp.
    pub reliability: ReliabilityStamp,
    /// `false` iff the stack consumed at least one quarantined
    /// (`quotable = false`) reference row.
    pub quotable: bool,
    /// The quarantined rows consumed, by reference table path.
    pub consumed_quarantined_rows: Vec<String>,
    /// Unmet publication gates carried by consumed rows (OCHT/Baringa).
    pub publication_gates: Vec<String>,
    /// Bracket rules carried by consumed rows (nuclear both-variants) —
    /// rendering obligations, not quarantines.
    pub bracket_rules: Vec<String>,
    /// Staleness stamps carried by consumed rows (battery 2018
    /// vintage).
    pub staleness_stamps: Vec<String>,
    /// Named limitations of this package's computation (overnight
    /// capex without IDC escalation; rule-8 settlement not costed).
    pub limitations: Vec<String>,
}

impl CostMetadata {
    /// The publish gate (docs/04 Stage 7 pin): refuse a result that
    /// consumed quarantined rows or carries an unmet publication gate.
    pub fn ensure_publishable(&self) -> Result<(), GridError> {
        let mut problems = Vec::new();
        if !self.consumed_quarantined_rows.is_empty() {
            problems.push(format!(
                "consumed quarantined rows [{}]",
                self.consumed_quarantined_rows.join(", ")
            ));
        }
        for gate in &self.publication_gates {
            problems.push(format!("unmet publication gate: {gate}"));
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(GridError::NonQuotableResult {
                reason: problems.join("; "),
            })
        }
    }
}

/// The constraint-costs line: a named zero with a flag until D6
/// resolves the function form — never silently pooled (D8 rule 1.6).
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintCostsLine {
    /// The line value (zero at every band point while D6 is open).
    pub value: WaccBand<Money>,
    /// `true` while D6 (constraint-cost function form) is unresolved.
    pub pending_d6: bool,
    /// The convention statement.
    pub note: String,
}

/// The D8 rule-1 cost stack over one run: six component lines, their
/// exact total, the delivered-to-demand denominator and the headline,
/// all WACC-banded, plus the governance metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct CostStack {
    /// Line 1: generation capex annualised + fixed O&M.
    pub generation_capex_fom: WaccBand<Money>,
    /// Line 2: variable O&M + fuel + carbon (Stage 2 SRMC chain).
    pub variable_om_fuel_carbon: WaccBand<Money>,
    /// Line 3: storage capex + O&M (power/energy split).
    pub storage_capex_om: WaccBand<Money>,
    /// Line 4: interconnection capex annualised.
    pub interconnection: WaccBand<Money>,
    /// Line 5: stability services at reference holding costs.
    pub stability_services: WaccBand<Money>,
    /// Line 6: constraint costs (named zero-with-flag pending D6).
    pub constraint_costs: ConstraintCostsLine,
    /// Σ of the six lines, exact (D8 rule 2): summed in line order
    /// 1→6 by one left fold per band point.
    pub total: WaccBand<Money>,
    /// The rule-1 denominator: GB demand − unserved energy (D3
    /// convention). NOT the per-technology delivered series.
    pub delivered_to_demand_energy: Energy,
    /// The headline: total ÷ delivered-to-demand energy, £/MWh.
    pub headline_per_mwh_delivered: WaccBand<Price>,
    /// Governance metadata (quarantine, gates, stamps, framing).
    pub metadata: CostMetadata,
}

impl CostStack {
    /// The publish gate — see [`CostMetadata::ensure_publishable`].
    pub fn ensure_publishable(&self) -> Result<(), GridError> {
        self.metadata.ensure_publishable()
    }
}

fn invalid(reason: String) -> GridError {
    GridError::InvalidCostInputs { reason }
}

/// Whether a calendar year is a leap year (proleptic Gregorian).
fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Half-hourly settlement periods in a calendar year.
fn periods_in_year(year: i64) -> f64 {
    if is_leap_year(year) {
        17_568.0
    } else {
        17_520.0
    }
}

/// The run horizon's calendar-year coverage: each period contributes
/// `1 / periods_in_its_year`, so a run over exactly calendar 2024 is
/// exactly 1.0 and the 1985–2024 record exactly 40.0 (module docs,
/// "Accrual convention").
#[must_use]
pub fn horizon_years(result: &RunResult) -> f64 {
    let mut years = 0.0;
    let mut index = 0usize;
    let total = result.periods();
    while index < total {
        let (year, _, _) = result.timestamp_at(index).civil_date();
        // Count this run's periods within the current calendar year in
        // one stride (periods are chronological half-hours).
        let mut count = 0usize;
        while index < total && result.timestamp_at(index).civil_date().0 == year {
            count += 1;
            index += 1;
        }
        years += count as f64 / periods_in_year(year);
    }
    years
}

/// The D8 rule-1 headline denominator: **delivered-to-demand energy**
/// = GB demand − GB unserved energy over the horizon (the D3
/// underlying-demand convention). A DIFFERENT object from the Package
/// A per-technology delivered series (module docs).
#[must_use]
pub fn delivered_to_demand_energy(result: &RunResult) -> Energy {
    result.total_demand_energy() - result.total_unserved()
}

/// Total energy of a per-period power series (half-hour periods).
fn series_energy(power: &[Power]) -> Energy {
    power
        .iter()
        .map(|&p| p * Duration::half_hour())
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

/// Find one technology's dispatch series (renewable potential or
/// thermal dispatch) by id.
fn find_series<'a>(result: &'a RunResult, tech: &str) -> Option<&'a [Power]> {
    result
        .renewables
        .iter()
        .chain(&result.thermal)
        .find(|series| series.tech.as_str() == tech)
        .map(|series| series.power.as_slice())
}

/// Whether a technology is one of the run's weather-driven series.
fn is_renewable(result: &RunResult, tech: &str) -> bool {
    result
        .renewables
        .iter()
        .any(|series| series.tech.as_str() == tech)
}

/// One costed generation asset's cost components: the accrued fixed
/// cost band (rule-4 annuity incl. site infrastructure + the source's
/// three fixed annual lines, over the horizon years) and the per-MWh
/// operating adder × the technology's generated energy. Shared by the
/// rule-1 stack (line 1 + the line-2 adder) and the rule-6a Q9
/// decomposition, so the two can never drift apart arithmetically.
struct AssetCostLine<'a> {
    /// The costs-reference row that priced the asset.
    row: &'a TechnologyCosts,
    /// Accrued annuity + fixed O&M over the horizon, per WACC.
    fixed: WaccBand<Money>,
    /// Per-MWh adder (VOM; nuclear fuel + decommissioning) × generated
    /// energy — the asset's line-2 contribution outside the SRMC chain.
    variable_adder: Money,
    /// The technology's generated energy over the run: potential
    /// (pre-curtailment) output for weather-driven technologies,
    /// dispatched output for thermal — the run's series convention.
    generated: Energy,
}

/// Compute one costed generation asset's [`AssetCostLine`]. Errors
/// mirror the cost stack's: unknown cost row, a costed technology
/// absent from the run, or a renewable with nonzero VOM (its
/// potential-vs-delivered basis would be smuggled — unsupported until a
/// basis is pinned).
fn generation_asset_line<'a>(
    reference: &'a CostsReference,
    result: &RunResult,
    asset: &CostedGeneration,
    wacc: WaccBand<PerUnit>,
    years: f64,
) -> Result<AssetCostLine<'a>, GridError> {
    let row = reference.technologies.get(&asset.cost_row).ok_or_else(|| {
        invalid(format!(
            "generation asset {}: cost row {:?} is not in the costs reference \
             (known rows: {})",
            asset.tech,
            asset.cost_row,
            reference
                .technologies
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })?;

    let annuities = wacc.try_map(|&rate| row.annuity(rate))?;
    let fixed = row.fixed_om_per_mw_yr();
    let fixed_band =
        annuities.map(|&annuity| ((annuity + fixed) * asset.capacity).over_years(years));

    let series = find_series(result, &asset.tech).ok_or_else(|| {
        invalid(format!(
            "generation asset {}: no dispatch series of that technology in the run \
             (the spec and the run result disagree)",
            asset.tech
        ))
    })?;
    let per_mwh = row.variable_cost_per_mwh();
    if is_renewable(result, &asset.tech) && per_mwh != Price::pounds_per_megawatt_hour(0.0) {
        // A nonzero per-MWh rate on a renewable would need a
        // potential-vs-delivered basis choice this package has not
        // pinned; every reference renewable VOM is zero, so refuse
        // rather than smuggle a convention.
        return Err(invalid(format!(
            "generation asset {}: nonzero per-MWh cost on a weather-driven technology is \
             unsupported (potential-vs-delivered basis not pinned for VOM)",
            asset.tech
        )));
    }
    let generated = series_energy(series);

    Ok(AssetCostLine {
        row,
        fixed: fixed_band,
        variable_adder: generated * per_mwh,
        generated,
    })
}

/// Compute the D8 rule-1 cost stack for a completed run. Pure function
/// of `(result, pricing, reference, spec)` (ADR-5); reads the dispatch
/// output, never modifies it. See the module docs for every convention
/// in prose.
///
/// Errors with [`GridError::InvalidCostInputs`] on an unknown cost row
/// or holding service, a costed technology absent from the run, a
/// quarantined interconnector row (no consumable figure exists), a
/// renewable with nonzero VOM (its potential/delivered basis would be
/// smuggled — unsupported until a basis is pinned), or a non-positive
/// delivered-energy denominator.
#[allow(
    clippy::too_many_lines,
    reason = "one linear pass over the six pinned lines"
)]
pub fn cost_stack(
    result: &RunResult,
    pricing: &PricingInputs,
    reference: &CostsReference,
    spec: &CostStackSpec,
) -> Result<CostStack, GridError> {
    let wacc = reference.wacc.set;
    let years = horizon_years(result);
    let horizon = Duration::hours(result.periods() as f64 * 0.5);

    let mut consumed_quarantined_rows = Vec::new();
    // No consumable reference row carries a publication gate yet (the
    // gated OCHT row becomes consumable with hydrogen-store costing);
    // the metadata field and the refuse path are live regardless.
    let publication_gates = Vec::new();
    let mut bracket_rules = Vec::new();
    let mut staleness_stamps = Vec::new();

    let zero_band = || WaccBand {
        low: Money::pounds(0.0),
        central: Money::pounds(0.0),
        high: Money::pounds(0.0),
    };
    let add = |acc: &mut WaccBand<Money>, rhs: WaccBand<Money>| {
        acc.low = acc.low + rhs.low;
        acc.central = acc.central + rhs.central;
        acc.high = acc.high + rhs.high;
    };

    // ------------------------------------------------------------------
    // Line 1 — generation capex annualised + fixed O&M; and the cost
    // rows' contribution to line 2 (VOM + non-SRMC fuel/decomm). The
    // per-asset arithmetic lives in [`generation_asset_line`], shared
    // with the Q9 decomposition.
    // ------------------------------------------------------------------
    let mut generation_capex_fom = zero_band();
    let mut variable_adders = Money::pounds(0.0);
    for asset in &spec.generation {
        let line = generation_asset_line(reference, result, asset, wacc, years)?;
        if !line.row.quotable {
            consumed_quarantined_rows.push(format!("technologies.{}", asset.cost_row));
        }
        if let Some(rule) = &line.row.bracket_rule {
            bracket_rules.push(format!("technologies.{}: {rule}", asset.cost_row));
        }
        add(&mut generation_capex_fom, line.fixed);
        variable_adders = variable_adders + line.variable_adder;
    }

    // ------------------------------------------------------------------
    // Line 2 — variable O&M + fuel + carbon. Gas fuel+carbon via the
    // EXISTING Stage 2 SRMC chain (D8 rule 1.2): Σ dispatch × Δt ×
    // SRMC(t) over the run's SRMC-bearing technologies.
    // ------------------------------------------------------------------
    let mut fuel_carbon = Money::pounds(0.0);
    for thermal in &result.thermal {
        let Some(srmc) = pricing.srmc.get(&thermal.tech) else {
            continue;
        };
        if srmc.len() != result.periods() {
            return Err(invalid(format!(
                "SRMC series for {} has {} periods; the run has {}",
                thermal.tech,
                srmc.len(),
                result.periods()
            )));
        }
        for (&power, &price) in thermal.power.iter().zip(srmc.values()) {
            fuel_carbon = fuel_carbon + (power * Duration::half_hour()) * price;
        }
    }
    let variable_total = fuel_carbon + variable_adders;
    let variable_om_fuel_carbon = WaccBand {
        low: variable_total,
        central: variable_total,
        high: variable_total,
    };

    // ------------------------------------------------------------------
    // Line 3 — storage capex + O&M, power and energy legs separate.
    // ------------------------------------------------------------------
    let mut storage_capex_om = zero_band();
    if !spec.batteries.is_empty() {
        let battery = &reference.battery;
        if !battery.quotable {
            consumed_quarantined_rows.push("storage.battery_li_ion".to_owned());
        }
        // Review condition 3.iii: the staleness stamp is a property of
        // the ROW (a 2018 projection vintage), not of the quarantine —
        // it travels on every battery-containing artefact regardless of
        // the quotable flag. The 2026-07-06 condition-3.i lift did not
        // lift 3.ii/3.iii.
        staleness_stamps.push(format!(
            "storage.battery_li_ion: {}",
            battery.staleness_stamp
        ));
        for store in &spec.batteries {
            let (power_capex, energy_capex) = match store.vintage {
                StoreVintage::Build2030 => (
                    battery.power_per_kw_2030_build,
                    battery.energy_per_kwh_2030_build,
                ),
                StoreVintage::Build2018 => (
                    battery.power_per_kw_2018_build,
                    battery.energy_per_kwh_2018_build,
                ),
            };
            let line = wacc.try_map(|&rate| {
                let power_leg = annuity_per_mw(power_capex, rate, battery.life_years)?;
                let energy_leg = annuity_per_kwh(energy_capex, rate, battery.life_years)?;
                let annual: MoneyRate = power_leg * store.power
                    + energy_leg * store.energy
                    + battery.fom_per_mw_yr * store.power;
                Ok(annual.over_years(years))
            })?;
            add(&mut storage_capex_om, line);
        }
    }

    // ------------------------------------------------------------------
    // Line 4 — interconnection capex annualised (caller-stated life).
    // ------------------------------------------------------------------
    let mut interconnection = zero_band();
    for link in &spec.interconnectors {
        let row = reference.interconnectors.get(&link.row).ok_or_else(|| {
            invalid(format!(
                "interconnector row {:?} is not in the costs reference",
                link.row
            ))
        })?;
        let Some(capex) = row.capex else {
            return Err(invalid(format!(
                "interconnector row {:?} carries no consumable GBP capex (status: {}) — \
                 quarantined pending a primary (review condition 9)",
                link.row, row.status
            )));
        };
        if !row.quotable {
            consumed_quarantined_rows.push(format!("interconnectors.{}", link.row));
        }
        let line = wacc.try_map(|&rate| {
            let crf = capital_recovery_factor(rate, link.life_years)?;
            Ok(MoneyRate::pounds_per_year(capex.as_pounds() * crf.value()).over_years(years))
        })?;
        add(&mut interconnection, line);
    }

    // ------------------------------------------------------------------
    // Line 5 — stability services at reference holding costs:
    // held MW × horizon hours × £/MW/h.
    // ------------------------------------------------------------------
    let mut stability_total = Money::pounds(0.0);
    for holding in &spec.holdings {
        let cost = reference
            .holding_costs
            .services
            .get(&holding.service)
            .ok_or_else(|| {
                invalid(format!(
                    "holding service {:?} is not in the costs reference (known services: {})",
                    holding.service,
                    reference
                        .holding_costs
                        .services
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;
        stability_total = stability_total + (holding.held * horizon) * cost.central;
    }
    let stability_services = WaccBand {
        low: stability_total,
        central: stability_total,
        high: stability_total,
    };

    // ------------------------------------------------------------------
    // Line 6 — constraint costs: named zero-with-flag pending D6.
    // ------------------------------------------------------------------
    let constraint_costs = ConstraintCostsLine {
        value: zero_band(),
        pending_d6: true,
        note: "constraint costs pending D6 (function form unresolved): reported as a \
               separately labelled zero line from the B6 approximation, never silently \
               pooled (D8 rule 1.6 / ADR-12)"
            .to_owned(),
    };

    // ------------------------------------------------------------------
    // Total (rule 2: exact left-fold in line order 1→6) and headline.
    // ------------------------------------------------------------------
    let lines = [
        &generation_capex_fom,
        &variable_om_fuel_carbon,
        &storage_capex_om,
        &interconnection,
        &stability_services,
        &constraint_costs.value,
    ];
    let mut total = zero_band();
    for line in lines {
        add(&mut total, *line);
    }

    let delivered = delivered_to_demand_energy(result);
    // NaN-safe positivity check (a NaN denominator must also refuse).
    let delivered_gwh = delivered.as_gigawatt_hours();
    if delivered_gwh.is_nan() || delivered_gwh <= 0.0 {
        return Err(invalid(format!(
            "delivered-to-demand energy is {} GWh; the rule-1 headline needs a positive \
             denominator (GB demand − unserved, D3 convention)",
            delivered.as_gigawatt_hours()
        )));
    }
    let headline_per_mwh_delivered = total.map(|&money| money / delivered);

    let metadata = CostMetadata {
        framing: spec.framing,
        wacc,
        reliability: ReliabilityStamp {
            unserved_energy: result.total_unserved(),
            adequacy_standard: spec.adequacy_standard.clone(),
        },
        quotable: consumed_quarantined_rows.is_empty(),
        consumed_quarantined_rows,
        publication_gates,
        bracket_rules,
        staleness_stamps,
        limitations: vec![
            "capex basis: overnight component costs, NOT escalated over the source's build \
             phasing at the WACC (no IDC) — under-costs long-build technologies relative to \
             the source's own method; phasing arrays are parsed and carried in \
             costs-reference-v1 for the escalation (evidence note §1, review condition 11)"
                .to_owned(),
            "import/export settlement (D8 rule 8) is not costed this package: multi-zone \
             SRMC is pending the priced ladder and no cited reference import price exists; \
             the interconnection line is link capex only"
                .to_owned(),
        ],
    };

    Ok(CostStack {
        generation_capex_fom,
        variable_om_fuel_carbon,
        storage_capex_om,
        interconnection,
        stability_services,
        constraint_costs,
        total,
        delivered_to_demand_energy: delivered,
        headline_per_mwh_delivered,
        metadata,
    })
}

// =====================================================================
// Q9 — the D8 rule-6a three-wedge gap decomposition
// (`docs/notes/d8-lcoe-methods.md` rule 6a; docs/04 Stage 7 acceptance
// line "LCOE vs. delivered £/MWh gap fully decomposed (Q9)").
//
// The decomposition is a SYSTEM-LEVEL ACCOUNTING IDENTITY, exact under
// rule 2 — never an attribution of shared costs to a single technology
// (rules 5 and 7 forbid that). Write, per WACC band point:
//
//   L̄  = generation-weighted mean of per-tech plant-gate LCOEs
//        (weighting basis stated below),
//   G  = Σ costed generation energy (the weighting basis),
//   E  = delivered-to-demand energy (the rule-1 denominator),
//   Cg = the generation cost lines (rule-1 lines 1 + 2),
//   Cm = the rule-1 lines absent from plant-gate LCOE (lines 3–6:
//        storage, interconnection, stability services, constraints),
//   H  = the rule-1 headline = (Cg + Cm) / E.
//
// The three wedges, each computed from its OWN definition (never by
// subtracting from the total — the D8 adjudication's anti-tautology
// rule):
//
//   1. UTILISATION wedge = Σᵢ C_fixᵢ × (1 − rᵢ) / G, where rᵢ =
//      realised generation / the generation the source's CF assumption
//      implies. Realised capacity factors below the LCOE figure's
//      assumed CF spread the same fixed cost over less energy.
//   2. DENOMINATOR wedge = Cg/E − Cg/G: the same generation cost per
//      delivered-to-demand MWh instead of per generated MWh
//      (curtailment, storage round-trip losses, boundary flows, and —
//      where the costed set does not cover every supply series — the
//      uncosted supply named in the artefact's coverage note).
//   3. MISSING-LINE wedge = Cm/E: the whole-system cost lines a
//      plant-gate LCOE never contains.
//
// Then L̄ + wedge₁ + wedge₂ + wedge₃ = H with NO residual term: the
// construction telescopes exactly in real arithmetic (L̄ + wedge₁ =
// Cg/G by definition of rᵢ). Numerically the anchors are independent
// f64 folds, so the reconstruction differs from H only by f64 dust —
// the acceptance test (`tests/q9_decomposition.rs`) asserts closure at
// ≤ 1e-9 relative (the committed reconciliation precedent's bound) and
// bit-reproducibility, and recomputes every wedge independently.
//
// CF-assumption convention (stated): a technology whose reference row
// publishes a load-factor assumption (`net_load_factor_2030`,
// `load_factor`) is priced at that CF in its plant-gate LCOE and
// contributes to the utilisation wedge. A row publishing NO assumption
// (ccgt — the source prices capex identically across its LF columns;
// ocgt; nuclear — the 2016 set carries none into the reference) is
// priced at its REALISED CF, contributing zero to the utilisation
// wedge by construction; its plant-gate contribution is exactly its
// accrued fixed + variable cost.
// =====================================================================

/// One technology's plant-gate LCOE bridge term (D8 rule 6: reported
/// because every reader expects it, always adjacent to the system-cost
/// number that supersedes it).
#[derive(Debug, Clone, PartialEq)]
pub struct TechPlantGate {
    /// The fleet technology id.
    pub tech: String,
    /// The costs-reference row that priced it.
    pub cost_row: String,
    /// Generated energy over the run (the weighting basis: potential
    /// output for weather-driven technologies, dispatched for thermal).
    pub generated_energy: Energy,
    /// Realised capacity factor over the run.
    pub realised_cf: PerUnit,
    /// The source's CF assumption, where the row publishes one; `None`
    /// selects the realised-CF convention (module docs).
    pub assumed_cf: Option<PerUnit>,
    /// Plant-gate LCOE (fixed cost at the assumed — else realised — CF
    /// plus realised variable cost per MWh), per WACC. `None` only when
    /// no CF basis exists (an idle technology with no published
    /// assumption): its £ contribution is still exact, its £/MWh figure
    /// is undefined.
    pub plant_gate_lcoe: Option<WaccBand<Price>>,
}

/// The rule-6a costed-coverage statement — a mandatory dedicated field
/// (stage-7 Q9 review, condition 2). On partially-costed scenarios,
/// uncosted supply serves demand inside the delivered denominator E
/// while its energy sits outside the weighting basis G, so the
/// denominator wedge — labelled 'curtailment'/'balancing' in the
/// docs/07 mapping — can be NEGATIVE (measured on the 2024 reference:
/// −16.63 £/MWh central, dragging the whole gap to −3.04). That sign
/// must never appear unexplained: this statement sits adjacent to any
/// rendered denominator wedge, and the wedge's only accessor
/// ([`Q9Decomposition::denominator_wedge`]) co-emits it.
#[derive(Debug, Clone, PartialEq)]
pub struct CostedCoverage {
    /// `true` iff every supply series of the run is a costed
    /// generation asset (G spans all supply).
    pub complete: bool,
    /// Uncosted supply series, by name, in run order (weather-driven,
    /// then thermal in merit order, then `exogenous:`-prefixed
    /// must-take traces).
    pub uncosted: Vec<String>,
    /// The prose statement that must accompany any rendered
    /// denominator wedge.
    pub statement: String,
}

/// The Q9 artefact: the rule-1 cost stack it decomposes, the rule-6
/// plant-gate bridge terms, the four anchors and the three wedges —
/// all WACC-banded, with the rule-6a mandatory statements (weighting
/// basis, term-to-label mapping, costed coverage) carried as fields so
/// no chart can omit them. See the module-level Q9 section for the
/// construction.
#[derive(Debug, Clone, PartialEq)]
pub struct Q9Decomposition {
    /// The rule-1 cost stack this decomposition explains (its metadata
    /// carries the rule-3 reliability stamp, the rule-4 WACC set and
    /// the quarantine flags — they gate this artefact too).
    pub stack: CostStack,
    /// Per-technology plant-gate bridge terms, in spec order.
    pub plant_gate: Vec<TechPlantGate>,
    /// L̄: the generation-weighted mean plant-gate LCOE.
    pub plant_gate_lcoe_mean: WaccBand<Price>,
    /// G: the weighting basis (Σ costed generation energy).
    pub generation_energy: Energy,
    /// Cg/G: generation cost (lines 1+2) per generated MWh.
    pub generation_cost_per_generated: WaccBand<Price>,
    /// Cg/E: the same cost per delivered-to-demand MWh.
    pub generation_cost_per_delivered: WaccBand<Price>,
    /// Wedge 3 of rule 6a (module docs order: utilisation).
    pub utilisation_wedge: WaccBand<Price>,
    /// Wedge 1 of rule 6a (denominator). PRIVATE by design (review
    /// condition 2): read it through
    /// [`Q9Decomposition::denominator_wedge`], which co-emits the
    /// costed-coverage statement that explains its sign.
    denominator_wedge: WaccBand<Price>,
    /// Wedge 2 of rule 6a (missing lines).
    pub missing_line_wedge: WaccBand<Price>,
    /// The decomposed object: headline − mean plant-gate LCOE.
    pub gap: WaccBand<Price>,
    /// Rule-6a mandatory statement: the weighting basis of L̄.
    pub weighting_basis: String,
    /// Rule-6a mandatory statement: the exact term-to-label mapping for
    /// the docs/07 Q9 presentational labels, including what
    /// "transmission" means in this model (ADR-12).
    pub label_mapping: String,
    /// Rule-6a mandatory statement: the costed-coverage boundary
    /// (review condition 2 — the denominator wedge's sign depends on
    /// it).
    pub coverage: CostedCoverage,
}

impl Q9Decomposition {
    /// The publish gate: a Q9 artefact is publishable exactly when its
    /// underlying cost stack is (quarantine flags and publication gates
    /// propagate — docs/04 Stage 7 pin).
    pub fn ensure_publishable(&self) -> Result<(), GridError> {
        self.stack.ensure_publishable()
    }

    /// Wedge 1 of rule 6a (denominator), co-emitted with the
    /// costed-coverage statement — the quarantine-refusal precedent
    /// ([`CostMetadata::ensure_publishable`]) applied to rendering
    /// (review condition 2): there is no coverage-free accessor, so an
    /// artefact/render path cannot obtain the wedge without receiving
    /// the statement that explains its sign (negative on
    /// partially-costed scenarios such as the 2024 reference).
    #[must_use]
    pub fn denominator_wedge(&self) -> (WaccBand<Price>, &CostedCoverage) {
        (self.denominator_wedge, &self.coverage)
    }
}

/// Compute the Q9 rule-6a decomposition over a completed run. Pure
/// function of `(result, pricing, reference, spec)` (ADR-5); builds the
/// rule-1 stack internally so the two artefacts can never disagree.
///
/// Preconditions (each a structured [`GridError::InvalidCostInputs`],
/// because the identity's attribution only holds when every generation
/// cost line is attributable to a costed technology):
/// - no duplicate technology among the costed generation assets;
/// - every SRMC-bearing thermal series in the run is a costed asset
///   (otherwise line 2 carries cost no plant-gate term owns);
/// - every costed asset has positive capacity (the CF bases divide by
///   it);
/// - positive total generation energy.
#[allow(
    clippy::too_many_lines,
    reason = "one linear pass over the rule-6a anchors and wedges"
)]
pub fn q9_decomposition(
    result: &RunResult,
    pricing: &PricingInputs,
    reference: &CostsReference,
    spec: &CostStackSpec,
) -> Result<Q9Decomposition, GridError> {
    let stack = cost_stack(result, pricing, reference, spec)?;
    let wacc = reference.wacc.set;
    let years = horizon_years(result);
    let horizon = Duration::hours(result.periods() as f64 * 0.5);

    // Precondition: no duplicate costed technologies (a duplicate would
    // double-count its generation in the weighting basis).
    for (index, asset) in spec.generation.iter().enumerate() {
        if spec.generation[..index]
            .iter()
            .any(|a| a.tech == asset.tech)
        {
            return Err(invalid(format!(
                "Q9: technology {} appears more than once in the costed generation \
                 assets — the weighting basis would double-count it",
                asset.tech
            )));
        }
    }
    // Precondition: every SRMC-bearing thermal series is costed.
    for thermal in &result.thermal {
        if pricing.srmc.contains_key(&thermal.tech)
            && !spec
                .generation
                .iter()
                .any(|a| a.tech == thermal.tech.as_str())
        {
            return Err(invalid(format!(
                "Q9: thermal technology {} carries an SRMC series but is not a costed \
                 generation asset — its line-2 cost would have no plant-gate term to \
                 land in",
                thermal.tech
            )));
        }
    }

    let zero_band = || WaccBand {
        low: Money::pounds(0.0),
        central: Money::pounds(0.0),
        high: Money::pounds(0.0),
    };
    let add = |acc: &mut WaccBand<Money>, rhs: WaccBand<Money>| {
        acc.low = acc.low + rhs.low;
        acc.central = acc.central + rhs.central;
        acc.high = acc.high + rhs.high;
    };

    let mut generation_energy = Energy::gigawatt_hours(0.0);
    let mut mean_numerator = zero_band(); // Σᵢ (C_fixᵢ·rᵢ + C_varᵢ), £
    let mut utilisation_numerator = zero_band(); // Σᵢ C_fixᵢ·(1 − rᵢ), £
    let mut plant_gate = Vec::with_capacity(spec.generation.len());

    for asset in &spec.generation {
        // NaN-safe positivity check (a NaN capacity must also refuse).
        let capacity_gw = asset.capacity.as_gigawatts();
        if capacity_gw.is_nan() || capacity_gw <= 0.0 {
            return Err(invalid(format!(
                "Q9: costed asset {} has non-positive capacity {capacity_gw} GW — the \
                 capacity-factor bases divide by it",
                asset.tech
            )));
        }
        let line = generation_asset_line(reference, result, asset, wacc, years)?;

        // The asset's full variable cost: the per-MWh adder plus its
        // SRMC-chain fuel + carbon where an SRMC series exists (D8 rule
        // 1.2 — the same arithmetic per technology as the stack's line
        // 2). The dispatch series exists: `generation_asset_line` found
        // it above (empty-slice fallback for the type only).
        let mut variable = line.variable_adder;
        let tech_id = grid_core::scenario::TechId::new(&asset.tech);
        if let Some(srmc) = pricing.srmc.get(&tech_id) {
            let series = find_series(result, &asset.tech).unwrap_or(&[]);
            for (&power, &price) in series.iter().zip(srmc.values()) {
                variable = variable + (power * Duration::half_hour()) * price;
            }
        }

        // rᵢ: realised generation over the generation the source's CF
        // assumption implies; the realised-CF convention (rᵢ = 1) where
        // the row publishes none (module docs).
        let assumed_cf = line.row.net_load_factor_2030.or(line.row.load_factor);
        let realised_cf = PerUnit::new(
            line.generated.as_gigawatt_hours() / (asset.capacity * horizon).as_gigawatt_hours(),
        );
        let (ratio, lcoe_basis) = match assumed_cf {
            Some(cf) => {
                let assumed_generation = asset.capacity * horizon * cf;
                (
                    line.generated.as_gigawatt_hours() / assumed_generation.as_gigawatt_hours(),
                    assumed_generation,
                )
            }
            None => (1.0, line.generated),
        };

        add(&mut mean_numerator, {
            let variable_all = variable;
            line.fixed.map(|&fix| fix * ratio + variable_all)
        });
        add(
            &mut utilisation_numerator,
            line.fixed.map(|&fix| fix * (1.0 - ratio)),
        );

        // The per-tech bridge LCOE: fixed over the CF basis + realised
        // variable per MWh. Undefined (None) only for an idle
        // technology with no published assumption.
        let plant_gate_lcoe = if lcoe_basis.as_gigawatt_hours() > 0.0 {
            let variable_per_mwh = if line.generated.as_gigawatt_hours() > 0.0 {
                variable / line.generated
            } else {
                Price::pounds_per_megawatt_hour(0.0)
            };
            Some(
                line.fixed
                    .map(|&fix| fix / lcoe_basis)
                    .map(|&fixed_per_mwh| fixed_per_mwh + variable_per_mwh),
            )
        } else {
            None
        };

        generation_energy = generation_energy + line.generated;
        plant_gate.push(TechPlantGate {
            tech: asset.tech.clone(),
            cost_row: asset.cost_row.clone(),
            generated_energy: line.generated,
            realised_cf,
            assumed_cf,
            plant_gate_lcoe,
        });
    }

    let generation_gwh = generation_energy.as_gigawatt_hours();
    if generation_gwh.is_nan() || generation_gwh <= 0.0 {
        return Err(invalid(format!(
            "Q9: total costed generation energy is {generation_gwh} GWh; the weighting \
             basis needs a positive denominator"
        )));
    }
    let delivered = stack.delivered_to_demand_energy;

    // The anchors, each from its own construction (module docs).
    let generation_cost = {
        let mut cg = zero_band();
        add(&mut cg, stack.generation_capex_fom);
        add(&mut cg, stack.variable_om_fuel_carbon);
        cg
    };
    let missing_lines = {
        let mut cm = zero_band();
        add(&mut cm, stack.storage_capex_om);
        add(&mut cm, stack.interconnection);
        add(&mut cm, stack.stability_services);
        add(&mut cm, stack.constraint_costs.value);
        cm
    };

    let plant_gate_lcoe_mean = mean_numerator.map(|&money| money / generation_energy);
    let generation_cost_per_generated = generation_cost.map(|&money| money / generation_energy);
    let generation_cost_per_delivered = generation_cost.map(|&money| money / delivered);
    let utilisation_wedge = utilisation_numerator.map(|&money| money / generation_energy);
    let denominator_wedge = WaccBand {
        low: generation_cost_per_delivered.low - generation_cost_per_generated.low,
        central: generation_cost_per_delivered.central - generation_cost_per_generated.central,
        high: generation_cost_per_delivered.high - generation_cost_per_generated.high,
    };
    let missing_line_wedge = missing_lines.map(|&money| money / delivered);
    let gap = WaccBand {
        low: stack.headline_per_mwh_delivered.low - plant_gate_lcoe_mean.low,
        central: stack.headline_per_mwh_delivered.central - plant_gate_lcoe_mean.central,
        high: stack.headline_per_mwh_delivered.high - plant_gate_lcoe_mean.high,
    };

    // The rule-6a costed-coverage statement (review condition 2): any
    // run supply series outside the costed set (uncosted technologies,
    // exogenous traces) carries cost the identity cannot see and energy
    // the weighting basis excludes — it moves the denominator wedge
    // (negative where uncosted supply makes E exceed G) and is named
    // here, by series, in run order.
    let uncosted: Vec<String> = result
        .renewables
        .iter()
        .chain(&result.thermal)
        .filter(|series| {
            !spec
                .generation
                .iter()
                .any(|a| a.tech == series.tech.as_str())
        })
        .map(|series| series.tech.as_str().to_owned())
        .chain(
            result
                .exogenous
                .iter()
                .map(|series| format!("exogenous:{}", series.label)),
        )
        .collect();
    let statement = if uncosted.is_empty() {
        "costed coverage: COMPLETE — every supply series of the run is a costed \
         generation asset (G spans all supply)"
            .to_owned()
    } else {
        format!(
            "costed coverage: INCOMPLETE — the weighting basis G covers the costed \
             technologies only; uncosted supply series present ({}) serve demand inside \
             the delivered denominator E but outside G, and their costs are outside the \
             stack entirely, so the denominator wedge ('curtailment'/'balancing' in the \
             docs/07 mapping) can be NEGATIVE; never render that wedge without this \
             statement adjacent",
            uncosted.join(", ")
        )
    };
    let coverage = CostedCoverage {
        complete: uncosted.is_empty(),
        uncosted,
        statement,
    };

    Ok(Q9Decomposition {
        stack,
        plant_gate,
        plant_gate_lcoe_mean,
        generation_energy,
        generation_cost_per_generated,
        generation_cost_per_delivered,
        utilisation_wedge,
        denominator_wedge,
        missing_line_wedge,
        gap,
        weighting_basis: "generation-weighted by each costed technology's run energy: \
                          potential (pre-curtailment) output for weather-driven \
                          technologies, dispatched output for thermal — the run's series \
                          convention, so curtailment lands in the denominator wedge"
            .to_owned(),
        label_mapping: "docs/07 Q9 label mapping (rule 6a): 'curtailment' and 'balancing' \
                        = the denominator wedge (generation vs delivered-to-demand energy: \
                        curtailment, storage round-trip losses, boundary flows); 'backup' \
                        and 'stability' = the missing-line wedge's storage and \
                        stability-service lines; 'transmission' = constraint costs + \
                        interconnection ONLY (no network model, ADR-12); realised-vs-assumed \
                        capacity factors = the utilisation wedge"
            .to_owned(),
        coverage,
    })
}
