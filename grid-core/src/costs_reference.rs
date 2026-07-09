//! Parser for the committed GB cost-inputs reference file
//! (`data/reference/costs-gb.toml`, schema `costs-reference-v1`) — the
//! Stage 7 pinned source of capex, O&M, asset lives, the uniform WACC
//! set, fuel/carbon trajectories, stability-service holding costs,
//! interconnector capex and the rule-9 emission factors
//! (evidence: `docs/notes/stage7-cost-inputs-report.md`, adjudicated
//! ACCEPT-WITH-NOTES in `docs/notes/stage7-cost-inputs-review.md`;
//! pinned into docs/04 Stage 7 on 2026-07-03).
//!
//! Parsing is strict, the `prices-reference-v1` pattern: the `schema`
//! string is probed first (so a future revision fails with a clear
//! message), unknown fields are rejected everywhere, and semantic
//! validation returns structured errors naming the offending table and
//! field. The 2024 fuel/carbon **actuals** and the SRMC efficiency
//! chain stay in `prices-2024.toml` (D8 rule 1.2) — this file carries
//! the *forward* trajectories and the cost stack's own inputs.
//!
//! **The governance fields are load-bearing** (docs/04 Stage 7 pin):
//! machine-readable `quotable` / `verified` quarantine flags, the
//! nuclear `bracket_rule`, the OCHT `publication_gate`, the battery
//! `staleness_stamp` and the cavern `binding_convention` are parsed
//! into the validated structs so consumers can propagate them into
//! result metadata; the artefact layer refuses to publish a result
//! that consumed a quarantined row
//! (`GridError::NonQuotableResult`).
//!
//! Build/pre-development phasing arrays (review condition 11) are
//! validated to sum to ~1 within [`PHASING_SUM_TOLERANCE`] — the
//! tolerance exists because two arrays are transcribed as published
//! with rounding shortfalls (onshore pre-development sums to 0.98,
//! biomass to 0.99). They are carried for the rule-4 IDC escalation,
//! which is out of scope this package (see [`crate::costs`] module
//! docs for the basis limitation).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::GridError;
use crate::costs::{WaccBand, annuity_per_mw};
use crate::units::{
    AnnualCapacityCost, CapacityCost, CarbonPrice, CostPerMass, EmissionsRate, EnergyCapacityCost,
    Length, Money, PerUnit, Power, Price,
};

/// The reference-file schema string this parser reads.
pub const COSTS_REFERENCE_SCHEMA: &str = "costs-reference-v1";

/// Tolerance on `Σ(phasing fractions) = 1`: admits the as-published
/// rounding shortfalls (onshore wind pre-development 0.98, biomass
/// 0.99) while rejecting genuinely broken arrays.
pub const PHASING_SUM_TOLERANCE: f64 = 0.025;

/// A published `[low, central, high]` bracket (capex ranges, per-tech
/// hurdle-rate triplets). Distinct from [`WaccBand`]: a bracket is the
/// *source's* published spread; a WACC band is *our* pinned rule-4
/// sensitivity axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bracket<T> {
    /// The published low value.
    pub low: T,
    /// The published central value.
    pub central: T,
    /// The published high value.
    pub high: T,
}

/// A cited source record (provenance; every number in the file names
/// one).
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRecord {
    /// Source title, including edition and price base.
    pub title: String,
    /// Licence statement.
    pub licence: String,
    /// Publication URL, when the source is URL-addressed.
    pub url: Option<String>,
    /// Pinned snapshot checksum, when the source is a committed pack
    /// snapshot instead.
    pub sha256: Option<String>,
}

/// The GDP-deflator table used for every price-base conversion in the
/// file (ONS GDP deflator at market prices, 2024 = 100).
#[derive(Debug, Clone, PartialEq)]
pub struct Deflator {
    /// Series description.
    pub series: String,
    /// Licence statement.
    pub licence: String,
    /// Multiplier to real 2024 GBP, keyed `y<year>` (e.g. `y2014`).
    pub to_2024: BTreeMap<String, PerUnit>,
}

/// The unrounded anchors behind the pinned WACC set (review
/// condition 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaccAnchors {
    /// Ofgem RIIO-3 allowed REAL (CPIH) WACC range for electricity
    /// transmission.
    pub low_range: [PerUnit; 2],
    /// CEPA lead-scenario L-M band.
    pub central: PerUnit,
    /// CEPA M-H band.
    pub high: PerUnit,
}

/// The D8 rule-4 WACC assumptions: the pinned uniform three-rate set,
/// its unrounded anchors, and the labelled per-technology sensitivity
/// (never the headline).
#[derive(Debug, Clone, PartialEq)]
pub struct WaccAssumptions {
    /// The pinned uniform set: 4.5 / 7.5 / 10.0 % real.
    pub set: WaccBand<PerUnit>,
    /// Unrounded anchors from the cited evidence.
    pub anchors_unrounded: WaccAnchors,
    /// Source keys of the anchors.
    pub sources: Vec<String>,
    /// Source key of the per-technology sensitivity table.
    pub per_tech_source: String,
    /// Per-technology rates — a labelled sensitivity only (D8 rule 4
    /// forbids per-tech WACCs in headlines).
    pub per_tech_sensitivity: BTreeMap<String, PerUnit>,
}

/// One generation technology's cost row (overnight component basis).
#[derive(Debug, Clone, PartialEq)]
pub struct TechnologyCosts {
    /// Source key.
    pub source: String,
    /// Overnight capex (pre-development + construction), £/kW,
    /// published `[low, central, high]`.
    pub capex_per_kw: Bracket<CapacityCost>,
    /// Whether the capex was deflator-converted into 2024 GBP (the
    /// `_gbp2024_` field spelling) rather than published in 2024 GBP.
    pub capex_converted_to_gbp2024: bool,
    /// Site infrastructure, £/kW (separate overnight component;
    /// absent where the source folds it into construction).
    pub infrastructure_per_kw: Option<CapacityCost>,
    /// Fixed O&M, £/MW/yr.
    pub fom_per_mw_yr: AnnualCapacityCost,
    /// Insurance, £/MW/yr.
    pub insurance_per_mw_yr: AnnualCapacityCost,
    /// Connection and use-of-system, £/MW/yr.
    pub connection_per_mw_yr: AnnualCapacityCost,
    /// Variable O&M, £/MWh.
    pub vom_per_mwh: Price,
    /// Fuel cost, £/MWh (nuclear only — gas fuel is priced through the
    /// Stage 2 SRMC chain, D8 rule 1.2).
    pub fuel_per_mwh: Option<Price>,
    /// Decommissioning and waste, £/MWh levelised (nuclear only).
    pub decommissioning_waste_per_mwh: Option<Price>,
    /// New-plant thermal efficiency, HHV (thermal technologies).
    pub efficiency_hhv_new_plant: Option<PerUnit>,
    /// Source net load factor for 2030 commissioning (renewables) —
    /// recorded; the engine computes its own capacity factors.
    pub net_load_factor_2030: Option<PerUnit>,
    /// Source load factor (biomass).
    pub load_factor: Option<PerUnit>,
    /// Operating life, years — the rule-4 CRF `n`.
    pub life_years: u32,
    /// Construction duration, years.
    pub build_years: u32,
    /// Pre-development duration, years.
    pub predev_years: Option<u32>,
    /// Pre-development annual spend fractions (review condition 11).
    pub predev_phasing: Option<Vec<PerUnit>>,
    /// Construction annual spend fractions (review condition 11).
    pub build_phasing: Vec<PerUnit>,
    /// The source's own per-tech hurdle-rate triplet (sensitivity
    /// record only).
    pub hurdle_rate: Option<Bracket<PerUnit>>,
    /// CEPA 2024 per-tech hurdle rate (sensitivity record only).
    pub hurdle_rate_cepa_2024: Option<PerUnit>,
    /// EGC 2023 single hurdle value (biomass; sensitivity record only).
    pub hurdle_rate_2023: Option<PerUnit>,
    /// Both-variants-quoted rule (nuclear, review condition 4) —
    /// surfaces in the metadata of every consuming result.
    pub bracket_rule: Option<String>,
    /// Machine-readable quarantine flag; `true` when absent.
    pub quotable: bool,
}

impl TechnologyCosts {
    /// Central overnight capex including site infrastructure — the
    /// cost stack's annualisation basis (the published capex range is
    /// a separate, labelled bracket).
    #[must_use]
    pub fn total_overnight_capex_central(&self) -> CapacityCost {
        match self.infrastructure_per_kw {
            Some(infrastructure) => self.capex_per_kw.central + infrastructure,
            None => self.capex_per_kw.central,
        }
    }

    /// The rule-4 annuity of the central overnight capex (incl.
    /// infrastructure) at one WACC, £/MW/yr. Overnight basis — no IDC
    /// escalation over the phasing arrays (see [`crate::costs`]).
    pub fn annuity(&self, wacc: PerUnit) -> Result<AnnualCapacityCost, GridError> {
        annuity_per_mw(self.total_overnight_capex_central(), wacc, self.life_years)
    }

    /// The source's three fixed annual lines summed: fixed O&M +
    /// insurance + connection, £/MW/yr.
    #[must_use]
    pub fn fixed_om_per_mw_yr(&self) -> AnnualCapacityCost {
        self.fom_per_mw_yr + self.insurance_per_mw_yr + self.connection_per_mw_yr
    }

    /// The per-MWh operating adder outside the SRMC chain: VOM plus
    /// (where present) fuel and decommissioning/waste. Gas fuel and
    /// carbon are NOT here — they come from the Stage 2 SRMC chain on
    /// `prices-2024.toml` (D8 rule 1.2).
    #[must_use]
    pub fn variable_cost_per_mwh(&self) -> Price {
        let mut total = self.vom_per_mwh;
        if let Some(fuel) = self.fuel_per_mwh {
            total = total + fuel;
        }
        if let Some(decommissioning) = self.decommissioning_waste_per_mwh {
            total = total + decommissioning;
        }
        total
    }
}

/// The observed all-in nuclear project cost (Sizewell C FID) — one half
/// of the mandatory nuclear bracket (review condition 4). A labelled
/// variant, NOT a component cost set: the basis differs from the
/// overnight convention.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedNuclearCost {
    /// Source key.
    pub source: String,
    /// All-in project cost, £/kW.
    pub capex_per_kw_project_total: CapacityCost,
    /// Project capacity.
    pub capacity: Power,
    /// Basis caveat — travels with the bracket wherever it is quoted.
    pub basis: String,
    /// Both-variants-quoted rule.
    pub bracket_rule: String,
}

/// Battery storage costs, power and energy legs priced separately
/// (D8 rule 1.3). The condition-3 quarantine was LIFTED 2026-07-06 as
/// a coordinated reviewed act (condition 3.i discharged against the
/// NREL primary, NREL/TP-6A40-93281); the row is `quotable = true`.
/// Caveats 3.ii (duration-attribution split) and 3.iii (2018
/// projection vintage — the `staleness_stamp` propagates to every
/// battery-quoting artefact) REMAIN in force.
#[derive(Debug, Clone, PartialEq)]
pub struct BatteryCosts {
    /// Source key.
    pub source: String,
    /// Machine-readable quarantine flag (`true` since the 2026-07-06
    /// reviewed lift; byte-pinned in the reference tests).
    pub quotable: bool,
    /// Power leg, 2030-build medium, £/kW (2024 GBP).
    pub power_per_kw_2030_build: CapacityCost,
    /// Energy leg, 2030-build medium, £/kWh (2024 GBP).
    pub energy_per_kwh_2030_build: EnergyCapacityCost,
    /// Power leg, 2018-build medium, £/kW (2024 GBP).
    pub power_per_kw_2018_build: CapacityCost,
    /// Energy leg, 2018-build medium, £/kWh (2024 GBP).
    pub energy_per_kwh_2018_build: EnergyCapacityCost,
    /// Fixed O&M — the source's £/kW/yr converted to the canonical
    /// £/MW/yr at parse (×10³).
    pub fom_per_mw_yr: AnnualCapacityCost,
    /// Operating life, years.
    pub life_years: u32,
    /// Cycle life, full cycles.
    pub cycle_life: u32,
    /// Round-trip efficiency.
    pub round_trip_efficiency: PerUnit,
    /// Usable depth of discharge.
    pub usable_depth_of_discharge: PerUnit,
    /// Construction duration, years.
    pub build_years: u32,
    /// Construction spend fractions.
    pub build_phasing: Vec<PerUnit>,
    /// CEPA 2024 hurdle rate (sensitivity record only).
    pub hurdle_rate_cepa_2024: PerUnit,
    /// Staleness stamp — propagates to any artefact quoting a
    /// battery-containing cost (review condition 3.iii).
    pub staleness_stamp: String,
}

/// Electrolyser (hydrogen charge leg) costs.
#[derive(Debug, Clone, PartialEq)]
pub struct ElectrolyserCosts {
    /// Source key.
    pub source: String,
    /// Electrolyser technology (PEM).
    pub tech: String,
    /// Published capex bracket per kW of H₂ HHV output, **2020 GBP**
    /// (the source's basis; the converted central is
    /// `capex_per_kwe_2030_central`).
    pub capex_per_kw_h2_hhv_2030_gbp2020: Bracket<CapacityCost>,
    /// Central capex per kW of ELECTRICAL input, 2024 GBP — the
    /// engine's charge-leg unit.
    pub capex_per_kwe_2030_central: CapacityCost,
    /// Electrical input per unit of H₂ output, kWhe/kWh_H2 HHV
    /// (dimensionless energy ratio, > 1).
    pub efficiency_kwhe_per_kwh_h2_hhv: PerUnit,
    /// Fixed O&M — the source's £/kW_H2/yr converted to £/MW_H2/yr at
    /// parse (×10³).
    pub fom_per_mw_h2_yr: AnnualCapacityCost,
    /// Variable O&M per MWh of H₂ produced (includes stack
    /// replacement, review note x3).
    pub vom_per_mwh_h2: Price,
    /// Operating life, years.
    pub life_years: u32,
    /// Construction duration, years.
    pub build_years: u32,
    /// Construction spend fractions.
    pub build_phasing: Vec<PerUnit>,
    /// The source's own hurdle rate (sensitivity record only).
    pub hurdle_rate_hpc2021: PerUnit,
    /// CEPA 2024 hurdle rate (sensitivity record only).
    pub hurdle_rate_cepa_2024: PerUnit,
    /// Alkaline-variant central capex per kWe, 2024 GBP (recorded).
    pub alkaline_capex_per_kwe_2030_central: CapacityCost,
}

/// Salt-cavern hydrogen storage: a LEVELISED cost with a BINDING
/// cycling convention (review condition 5) — not a capex. Applying it
/// to a store cycling materially below ~9 cycles/yr re-opens the capex
/// gap as a named blocker.
#[derive(Debug, Clone, PartialEq)]
pub struct HydrogenCavernCosts {
    /// Source key.
    pub source: String,
    /// Levelised cost, £/kg H₂.
    pub levelised_per_kg: CostPerMass,
    /// Levelised cost per MWh of H₂ throughput (HHV).
    pub levelised_per_mwh_h2_hhv: Price,
    /// The cycling rate the levelised figure assumes.
    pub cycles_per_year_assumed: u32,
    /// Basis statement (levelised, NOT capex).
    pub basis: String,
    /// The binding convention — normative, surfaces with any consuming
    /// result.
    pub binding_convention: String,
}

/// OCHT (100 % hydrogen open-cycle turbine) reconversion costs — the
/// D4 discharge leg. Carries a **publication gate** (review
/// condition 6): the Baringa H2P primary must be checked before any
/// OCHT-containing number is published; consuming results carry the
/// gate in metadata and the publish path refuses while it stands.
#[derive(Debug, Clone, PartialEq)]
pub struct OchtCosts {
    /// Source key.
    pub source: String,
    /// Overnight capex, £/kW, `[low, central, high]`.
    pub capex_per_kw: Bracket<CapacityCost>,
    /// Site infrastructure, £/kW.
    pub infrastructure_per_kw: CapacityCost,
    /// Fixed O&M, £/MW/yr.
    pub fom_per_mw_yr: AnnualCapacityCost,
    /// Insurance, £/MW/yr.
    pub insurance_per_mw_yr: AnnualCapacityCost,
    /// Connection, £/MW/yr.
    pub connection_per_mw_yr: AnnualCapacityCost,
    /// Variable O&M, £/MWh.
    pub vom_per_mwh: Price,
    /// Default efficiency, HHV — Annex A as published (the opponent's
    /// default; conservative/higher-cost end).
    pub efficiency_hhv_default: PerUnit,
    /// Mandatory labelled sensitivity (~35 % LHV × 0.846) — must
    /// accompany every OCHT-consuming result.
    pub efficiency_hhv_sensitivity_labelled: PerUnit,
    /// The publication gate (review condition 6).
    pub publication_gate: String,
    /// Operating life, years.
    pub life_years: u32,
    /// Construction duration, years.
    pub build_years: u32,
    /// Pre-development duration, years.
    pub predev_years: u32,
    /// Pre-development spend fractions.
    pub predev_phasing: Vec<PerUnit>,
    /// Construction spend fractions.
    pub build_phasing: Vec<PerUnit>,
    /// The source's hurdle-rate triplet (sensitivity record only).
    pub hurdle_rate: Bracket<PerUnit>,
}

/// The DESNZ FFPA 2025 gas price trajectory, converted at parse from
/// the published p/therm (real 2024) into £/MWh-thermal HHV with the
/// file's own UK-statutory-therm factor.
#[derive(Debug, Clone, PartialEq)]
pub struct GasTrajectory {
    /// Source key.
    pub source: String,
    /// The published unit (documentation).
    pub unit: String,
    /// Trajectory years, strictly ascending.
    pub years: Vec<i64>,
    /// Low scenario, £/MWh-thermal HHV.
    pub low: Vec<Price>,
    /// Central scenario, £/MWh-thermal HHV.
    pub central: Vec<Price>,
    /// High scenario, £/MWh-thermal HHV.
    pub high: Vec<Price>,
    /// The p/therm → £/MWh_HHV conversion constant (UK statutory
    /// therm = 29.3071 kWh gross CV; review condition 2). A unit
    /// conversion factor, not a physical quantity.
    pub p_per_therm_to_gbp_per_mwh_hhv: f64,
}

/// The DESNZ traded-carbon-values trajectory (modelling values, "not
/// forecasts" — stamp travels with any consuming artefact).
#[derive(Debug, Clone, PartialEq)]
pub struct CarbonTrajectory {
    /// Source key.
    pub source: String,
    /// The published unit (documentation).
    pub unit: String,
    /// Trajectory years, strictly ascending.
    pub years: Vec<i64>,
    /// Low scenario, £/tCO2e real 2024.
    pub low: Vec<CarbonPrice>,
    /// Central scenario, £/tCO2e real 2024.
    pub central: Vec<CarbonPrice>,
    /// High scenario, £/tCO2e real 2024.
    pub high: Vec<CarbonPrice>,
    /// Carbon Price Support, £/tCO2 NOMINAL frozen (review
    /// condition 8).
    pub cps_nominal: CarbonPrice,
    /// The pinned CPS pathway convention.
    pub cps_convention: String,
}

/// One response product's holding cost, £/MW per hour of holding
/// (dimensionally £/MWh, hence the [`Price`] carrier; the "energy" is
/// MW-held × hours).
#[derive(Debug, Clone, PartialEq)]
pub struct HoldingCost {
    /// FY2025 volume-weighted mean clearing price.
    pub central: Price,
    /// Per-window p5..p95 range.
    pub range_p5_p95: [Price; 2],
    /// Unweighted mean (recorded).
    pub mean_unweighted: Price,
}

/// The recorded (not consumed) high-frequency product prices — the Q8
/// survival model excludes them; negative DRH is genuine.
#[derive(Debug, Clone, PartialEq)]
pub struct HighFrequencyHoldingCosts {
    /// Dynamic containment HF.
    pub dch: Price,
    /// Dynamic moderation HF.
    pub dmh: Price,
    /// Dynamic regulation HF (negative: net payment TO NESO).
    pub drh: Price,
}

/// Stability-service holding costs (D8 rule 1.5), keyed to match
/// `response-holdings-2025.toml` `[[services]]` names.
#[derive(Debug, Clone, PartialEq)]
pub struct HoldingCosts {
    /// Source key (the pinned FY2025 EAC snapshot).
    pub source: String,
    /// The unit statement (`GBP/MW/h`).
    pub unit: String,
    /// The covered period.
    pub period: String,
    /// Low-frequency dynamic products, by holdings-file service name.
    pub services: BTreeMap<String, HoldingCost>,
    /// High-frequency products (recorded, not consumed).
    pub high_frequency: HighFrequencyHoldingCosts,
}

/// One interconnector's cost row. Only rows with `verified = true` /
/// `quotable = true` carry a consumable GBP capex; the quarantined
/// rows (review condition 9) deliberately surface **no** point capex —
/// their unverified figures (a EUR estimate, a conflicting GBP range)
/// are validated at parse but not exposed for computation.
#[derive(Debug, Clone, PartialEq)]
pub struct InterconnectorCosts {
    /// Source key, where a primary exists.
    pub source: Option<String>,
    /// Whether the capex is verified to a primary.
    pub verified: bool,
    /// Machine-readable quarantine flag.
    pub quotable: bool,
    /// Project capex, £ — present only on verified rows.
    pub capex: Option<Money>,
    /// Link capacity.
    pub capacity: Power,
    /// Route length.
    pub length: Length,
    /// Status / provenance statement.
    pub status: String,
}

/// Emission factors closing the D8 rule-9 gap (review condition 7).
#[derive(Debug, Clone, PartialEq)]
pub struct EmissionFactors {
    /// Coal (electricity generation): CO₂-only pricing factor,
    /// tCO2/MWh-thermal HHV.
    pub coal_co2_per_mwh_th_hhv: EmissionsRate,
    /// Coal CO₂e accounting factor.
    pub coal_co2e_per_mwh_th_hhv: EmissionsRate,
    /// Biomass pricing factor — biogenic CO₂ zero-rated (UK ETS).
    pub biomass_co2_for_pricing: EmissionsRate,
    /// Biomass non-CO₂ combustion factor (CH₄+N₂O) — accounting only,
    /// never priced.
    pub biomass_co2e_non_co2: EmissionsRate,
    /// The "other" category pricing factor (zero by convention).
    pub other_co2_per_mwh_th_hhv: EmissionsRate,
    /// The MANDATORY residual-reporting convention for "other".
    pub other_convention: String,
}

/// The validated contents of a costs-reference file. Every number
/// carries its citation in the TOML source.
#[derive(Debug, Clone, PartialEq)]
pub struct CostsReference {
    /// The price base (real 2024 GBP).
    pub price_base: String,
    /// Assembly date, as written.
    pub assembled: String,
    /// The deflator table behind every conversion.
    pub deflator: Deflator,
    /// Cited sources by key.
    pub sources: BTreeMap<String, SourceRecord>,
    /// The rule-4 WACC assumptions.
    pub wacc: WaccAssumptions,
    /// Generation technology rows by key (`ccgt`, `ocgt`, `nuclear`,
    /// `onshore_wind`, `offshore_wind`, `solar_pv`, `biomass`).
    pub technologies: BTreeMap<String, TechnologyCosts>,
    /// The observed nuclear project cost — the other half of the
    /// mandatory bracket.
    pub nuclear_observed: ObservedNuclearCost,
    /// Battery storage split (quarantined).
    pub battery: BatteryCosts,
    /// Electrolyser (hydrogen charge leg).
    pub electrolyser: ElectrolyserCosts,
    /// Salt-cavern hydrogen storage (levelised-with-convention).
    pub hydrogen_cavern: HydrogenCavernCosts,
    /// OCHT reconversion (publication-gated).
    pub hydrogen_reconversion_ocht: OchtCosts,
    /// FFPA 2025 gas trajectory.
    pub gas_trajectory: GasTrajectory,
    /// Traded-carbon-values trajectory.
    pub carbon_trajectory: CarbonTrajectory,
    /// Stability-service holding costs.
    pub holding_costs: HoldingCosts,
    /// Interconnector rows by key.
    pub interconnectors: BTreeMap<String, InterconnectorCosts>,
    /// Rule-9 emission factors.
    pub emission_factors: EmissionFactors,
}

// ---------------------------------------------------------------------
// TOML-facing raw structures (strict: deny_unknown_fields throughout).
// ---------------------------------------------------------------------

#[derive(Deserialize)]
struct SchemaProbe {
    schema: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReference {
    #[allow(dead_code, reason = "consumed by the schema probe")]
    schema: String,
    price_base: String,
    assembled: toml::value::Datetime,
    deflator: RawDeflator,
    sources: BTreeMap<String, RawSource>,
    wacc: RawWacc,
    technologies: BTreeMap<String, RawTechnology>,
    storage: RawStorage,
    trajectories: RawTrajectories,
    holding_costs: RawHoldingCosts,
    interconnectors: BTreeMap<String, RawInterconnector>,
    emission_factor: RawEmissionFactors,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeflator {
    series: String,
    licence: String,
    to_2024: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    title: String,
    licence: String,
    url: Option<String>,
    #[allow(dead_code, reason = "provenance URL, documentation only")]
    annex_a: Option<String>,
    #[allow(dead_code, reason = "provenance URL, documentation only")]
    annex: Option<String>,
    #[allow(dead_code, reason = "provenance URL, documentation only")]
    data: Option<String>,
    #[allow(dead_code, reason = "provenance URL, documentation only")]
    dataset_page: Option<String>,
    sha256: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWacc {
    real_low: f64,
    real_central: f64,
    real_high: f64,
    anchors_unrounded: RawWaccAnchors,
    sources: Vec<String>,
    per_tech_sensitivity: RawPerTechSensitivity,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWaccAnchors {
    low_range: Vec<f64>,
    central: f64,
    high: f64,
}

/// The 14 CEPA per-tech assignments, named explicitly so an added or
/// renamed technology fails strict parsing (a schema change requires a
/// version bump).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPerTechSensitivity {
    source: String,
    solar_pv: f64,
    onshore_wind: f64,
    offshore_wind_fixed: f64,
    offshore_wind_floating: f64,
    biomass_unabated: f64,
    nuclear_large_rab: f64,
    gas_unabated: f64,
    battery_li_ion_cm: f64,
    pumped_hydro_cap_floor: f64,
    ldes_novel: f64,
    hydrogen_ccht_ocht_mature: f64,
    hydrogen_ccht_ocht_emerging: f64,
    hydrogen_electrolyser: f64,
    interconnector_cap_floor: f64,
}

/// Union of the generation-technology row shapes (2024-GBP-published
/// vs deflator-converted field spellings; nuclear extras; the observed
/// nuclear variant). Validation demands the right combination per row.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTechnology {
    source: String,
    capex_gbp_per_kw: Option<Vec<f64>>,
    capex_gbp2024_per_kw: Option<Vec<f64>>,
    infrastructure_gbp_per_kw: Option<f64>,
    infrastructure_gbp2024_per_kw: Option<f64>,
    fom_gbp_per_mw_yr: Option<f64>,
    fom_gbp2024_per_mw_yr: Option<f64>,
    insurance_gbp_per_mw_yr: Option<f64>,
    insurance_gbp2024_per_mw_yr: Option<f64>,
    connection_gbp_per_mw_yr: Option<f64>,
    connection_gbp2024_per_mw_yr: Option<f64>,
    vom_gbp_per_mwh: Option<f64>,
    vom_gbp2024_per_mwh: Option<f64>,
    fuel_gbp2024_per_mwh: Option<f64>,
    decommissioning_waste_gbp2024_per_mwh: Option<f64>,
    efficiency_hhv_new_plant: Option<f64>,
    net_load_factor_2030: Option<f64>,
    load_factor: Option<f64>,
    life_years: Option<i64>,
    build_years: Option<i64>,
    predev_years: Option<i64>,
    predev_phasing: Option<Vec<f64>>,
    build_phasing: Option<Vec<f64>>,
    hurdle_rate: Option<Vec<f64>>,
    hurdle_rate_cepa_2024: Option<f64>,
    hurdle_rate_2023: Option<f64>,
    bracket_rule: Option<String>,
    quotable: Option<bool>,
    // Observed-variant fields (nuclear_observed only).
    capex_gbp_per_kw_project_total: Option<f64>,
    capacity_gw: Option<f64>,
    basis: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStorage {
    battery_li_ion: RawBattery,
    hydrogen: RawHydrogenStorage,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBattery {
    source: String,
    quotable: bool,
    power_gbp2024_per_kw_2030build: f64,
    energy_gbp2024_per_kwh_2030build: f64,
    power_gbp2024_per_kw_2018build: f64,
    energy_gbp2024_per_kwh_2018build: f64,
    fom_gbp2024_per_kw_yr: f64,
    life_years: i64,
    cycle_life: i64,
    round_trip_efficiency: f64,
    usable_depth_of_discharge: f64,
    build_years: i64,
    build_phasing: Vec<f64>,
    hurdle_rate_cepa_2024: f64,
    staleness_stamp: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHydrogenStorage {
    electrolyser: RawElectrolyser,
    cavern: RawCavern,
    reconversion_ocht: RawOcht,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawElectrolyser {
    source: String,
    tech: String,
    capex_gbp2020_per_kw_h2_hhv_2030: Vec<f64>,
    capex_gbp2024_per_kwe_2030_central: f64,
    efficiency_kwhe_per_kwh_h2_hhv: f64,
    fom_gbp2024_per_kw_h2_yr: f64,
    vom_gbp2024_per_mwh_h2: f64,
    life_years: i64,
    build_years: i64,
    build_phasing: Vec<f64>,
    hurdle_rate_hpc2021: f64,
    hurdle_rate_cepa_2024: f64,
    alkaline_capex_gbp2024_per_kwe_2030_central: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCavern {
    source: String,
    levelised_gbp2024_per_kg: f64,
    levelised_gbp2024_per_mwh_h2_hhv: f64,
    cycles_per_year_assumed: i64,
    basis: String,
    binding_convention: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOcht {
    source: String,
    capex_gbp_per_kw: Vec<f64>,
    infrastructure_gbp_per_kw: f64,
    fom_gbp_per_mw_yr: f64,
    insurance_gbp_per_mw_yr: f64,
    connection_gbp_per_mw_yr: f64,
    vom_gbp_per_mwh: f64,
    efficiency_hhv_default: f64,
    efficiency_hhv_sensitivity_labelled: f64,
    publication_gate: String,
    life_years: i64,
    build_years: i64,
    predev_years: i64,
    predev_phasing: Vec<f64>,
    build_phasing: Vec<f64>,
    hurdle_rate: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTrajectories {
    gas: RawGasTrajectory,
    carbon: RawCarbonTrajectory,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGasTrajectory {
    source: String,
    unit: String,
    years: Vec<i64>,
    low: Vec<f64>,
    central: Vec<f64>,
    high: Vec<f64>,
    p_per_therm_to_gbp_per_mwh_hhv: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCarbonTrajectory {
    source: String,
    unit: String,
    years: Vec<i64>,
    low: Vec<f64>,
    central: Vec<f64>,
    high: Vec<f64>,
    cps_gbp_per_tco2_nominal: f64,
    cps_convention: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHoldingCosts {
    source: String,
    unit: String,
    period: String,
    dynamic_containment_lf: RawHoldingCost,
    dynamic_moderation_lf: RawHoldingCost,
    dynamic_regulation_lf: RawHoldingCost,
    high_frequency_products: RawHighFrequency,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHoldingCost {
    central: f64,
    range_p5_p95: Vec<f64>,
    mean_unweighted: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHighFrequency {
    dch: f64,
    dmh: f64,
    drh: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInterconnector {
    source: Option<String>,
    verified: bool,
    quotable: bool,
    capex_gbp_bn: Option<f64>,
    capacity_gw: f64,
    length_km: f64,
    status: String,
    // Validated but deliberately NOT surfaced (quarantined rows must
    // carry no consumable figure): a derived diagnostic, a
    // foreign-currency estimate and a conflicting range.
    #[allow(dead_code, reason = "recomputable diagnostic, not consumed")]
    derived_gbp_m_per_gw_km: Option<f64>,
    #[allow(dead_code, reason = "quarantined EUR estimate, not consumed")]
    capex_eur_bn_estimated: Option<f64>,
    #[allow(dead_code, reason = "quarantined conflicting range, not consumed")]
    capex_gbp_bn_range: Option<Vec<f64>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEmissionFactors {
    coal_electricity_generation: RawCoalFactor,
    biomass: RawBiomassFactor,
    other_category: RawOtherFactor,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCoalFactor {
    #[allow(dead_code, reason = "provenance, documentation only")]
    source: String,
    co2_tonnes_per_mwh_th_hhv: f64,
    co2e_tonnes_per_mwh_th_hhv: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBiomassFactor {
    #[allow(dead_code, reason = "provenance, documentation only")]
    source: String,
    co2_tonnes_per_mwh_th_for_pricing: f64,
    co2e_tonnes_per_mwh_th_non_co2: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOtherFactor {
    co2_tonnes_per_mwh_th_hhv: f64,
    convention: String,
}

// ---------------------------------------------------------------------
// Validation.
// ---------------------------------------------------------------------

fn invalid(reason: String) -> GridError {
    GridError::InvalidCostsReference { reason }
}

/// A fraction strictly inside (0, 1]; `what` names the field.
fn fraction(what: &str, value: f64) -> Result<PerUnit, GridError> {
    if !(value > 0.0 && value <= 1.0) {
        return Err(invalid(format!("{what} = {value} is outside (0, 1]")));
    }
    Ok(PerUnit::new(value))
}

/// A non-negative money-like figure; `what` names the field.
fn non_negative(what: &str, value: f64) -> Result<f64, GridError> {
    if !value.is_finite() || value < 0.0 {
        return Err(invalid(format!("{what} = {value} must be non-negative")));
    }
    Ok(value)
}

/// A strictly positive figure; `what` names the field.
fn positive(what: &str, value: f64) -> Result<f64, GridError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid(format!("{what} = {value} must be positive")));
    }
    Ok(value)
}

/// A positive year count.
fn years(what: &str, value: i64) -> Result<u32, GridError> {
    u32::try_from(value)
        .ok()
        .filter(|&v| v > 0)
        .ok_or_else(|| invalid(format!("{what} = {value} must be a positive year count")))
}

/// A `[low, central, high]` bracket of non-negative values.
fn bracket(what: &str, values: &[f64]) -> Result<Bracket<f64>, GridError> {
    let [low, central, high] = values else {
        return Err(invalid(format!(
            "{what} must be a [low, central, high] triple, got {} entries",
            values.len()
        )));
    };
    for value in [low, central, high] {
        non_negative(what, *value)?;
    }
    if !(low <= central && central <= high) {
        return Err(invalid(format!(
            "{what} = [{low}, {central}, {high}] is not ordered low ≤ central ≤ high (capex \
             brackets are published [low, central, high])"
        )));
    }
    Ok(Bracket {
        low: *low,
        central: *central,
        high: *high,
    })
}

/// A build/pre-development phasing array: every fraction in (0, 1],
/// summing to 1 within [`PHASING_SUM_TOLERANCE`] (review condition 11;
/// the tolerance admits the as-published rounding — module docs).
fn phasing(what: &str, values: &[f64]) -> Result<Vec<PerUnit>, GridError> {
    if values.is_empty() {
        return Err(invalid(format!("{what} is empty")));
    }
    let mut out = Vec::with_capacity(values.len());
    for &value in values {
        out.push(fraction(what, value)?);
    }
    let sum: f64 = values.iter().sum();
    if (sum - 1.0).abs() > PHASING_SUM_TOLERANCE {
        return Err(invalid(format!(
            "{what} sums to {sum}; spend fractions must sum to 1.0 ± {PHASING_SUM_TOLERANCE} \
             (the tolerance covers as-published rounding only)"
        )));
    }
    Ok(out)
}

/// One of the two field spellings (`x_gbp_…` published 2024 GBP vs
/// `x_gbp2024_…` deflator-converted), exactly one required. Returns
/// the value and whether the converted spelling was used.
fn one_spelling(
    what: &str,
    published: Option<f64>,
    converted: Option<f64>,
) -> Result<(f64, bool), GridError> {
    match (published, converted) {
        (Some(value), None) => Ok((value, false)),
        (None, Some(value)) => Ok((value, true)),
        (None, None) => Err(invalid(format!("{what} is missing"))),
        (Some(_), Some(_)) => Err(invalid(format!(
            "{what} appears in both its published-2024-GBP and converted spellings"
        ))),
    }
}

fn validate_technology(key: &str, raw: &RawTechnology) -> Result<TechnologyCosts, GridError> {
    let ctx = |field: &str| format!("technologies.{key}: {field}");

    let (capex_values, capex_converted) = match (&raw.capex_gbp_per_kw, &raw.capex_gbp2024_per_kw) {
        (Some(values), None) => (values, false),
        (None, Some(values)) => (values, true),
        (None, None) => return Err(invalid(ctx("capex bracket is missing"))),
        (Some(_), Some(_)) => {
            return Err(invalid(ctx(
                "capex appears in both its published and converted spellings",
            )));
        }
    };
    let capex = bracket(&ctx("capex"), capex_values)?;

    let (infrastructure, infrastructure_converted) = match (
        raw.infrastructure_gbp_per_kw,
        raw.infrastructure_gbp2024_per_kw,
    ) {
        (Some(value), None) => (Some(value), false),
        (None, Some(value)) => (Some(value), true),
        (None, None) => (None, false),
        (Some(_), Some(_)) => {
            return Err(invalid(ctx(
                "infrastructure appears in both its published and converted spellings",
            )));
        }
    };
    if infrastructure_converted != capex_converted && infrastructure.is_some() {
        return Err(invalid(ctx(
            "capex and infrastructure use different price-base spellings",
        )));
    }
    let infrastructure_per_kw = infrastructure
        .map(|value| non_negative(&ctx("infrastructure"), value))
        .transpose()?
        .map(CapacityCost::pounds_per_kilowatt);

    let (fom, _) = one_spelling(
        &ctx("fom"),
        raw.fom_gbp_per_mw_yr,
        raw.fom_gbp2024_per_mw_yr,
    )?;
    let (insurance, _) = one_spelling(
        &ctx("insurance"),
        raw.insurance_gbp_per_mw_yr,
        raw.insurance_gbp2024_per_mw_yr,
    )?;
    let (connection, _) = one_spelling(
        &ctx("connection"),
        raw.connection_gbp_per_mw_yr,
        raw.connection_gbp2024_per_mw_yr,
    )?;
    let (vom, _) = one_spelling(&ctx("vom"), raw.vom_gbp_per_mwh, raw.vom_gbp2024_per_mwh)?;
    non_negative(&ctx("fom"), fom)?;
    non_negative(&ctx("insurance"), insurance)?;
    non_negative(&ctx("connection"), connection)?;
    non_negative(&ctx("vom"), vom)?;

    let life_years = years(
        &ctx("life_years"),
        raw.life_years
            .ok_or_else(|| invalid(ctx("life_years is missing")))?,
    )?;
    let build_years = years(
        &ctx("build_years"),
        raw.build_years
            .ok_or_else(|| invalid(ctx("build_years is missing")))?,
    )?;
    let predev_years = raw
        .predev_years
        .map(|value| years(&ctx("predev_years"), value))
        .transpose()?;

    let build_phasing = phasing(
        &ctx("build_phasing"),
        raw.build_phasing
            .as_deref()
            .ok_or_else(|| invalid(ctx("build_phasing is missing (review condition 11)")))?,
    )?;
    let predev_phasing = raw
        .predev_phasing
        .as_deref()
        .map(|values| phasing(&ctx("predev_phasing"), values))
        .transpose()?;

    let hurdle_rate = raw
        .hurdle_rate
        .as_deref()
        .map(|values| -> Result<Bracket<PerUnit>, GridError> {
            let b = bracket(&ctx("hurdle_rate"), values)?;
            Ok(Bracket {
                low: fraction(&ctx("hurdle_rate.low"), b.low)?,
                central: fraction(&ctx("hurdle_rate.central"), b.central)?,
                high: fraction(&ctx("hurdle_rate.high"), b.high)?,
            })
        })
        .transpose()?;

    Ok(TechnologyCosts {
        source: raw.source.clone(),
        capex_per_kw: Bracket {
            low: CapacityCost::pounds_per_kilowatt(capex.low),
            central: CapacityCost::pounds_per_kilowatt(capex.central),
            high: CapacityCost::pounds_per_kilowatt(capex.high),
        },
        capex_converted_to_gbp2024: capex_converted,
        infrastructure_per_kw,
        fom_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(fom),
        insurance_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(insurance),
        connection_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(connection),
        vom_per_mwh: Price::pounds_per_megawatt_hour(vom),
        fuel_per_mwh: raw
            .fuel_gbp2024_per_mwh
            .map(|value| non_negative(&ctx("fuel"), value))
            .transpose()?
            .map(Price::pounds_per_megawatt_hour),
        decommissioning_waste_per_mwh: raw
            .decommissioning_waste_gbp2024_per_mwh
            .map(|value| non_negative(&ctx("decommissioning_waste"), value))
            .transpose()?
            .map(Price::pounds_per_megawatt_hour),
        efficiency_hhv_new_plant: raw
            .efficiency_hhv_new_plant
            .map(|value| fraction(&ctx("efficiency_hhv_new_plant"), value))
            .transpose()?,
        net_load_factor_2030: raw
            .net_load_factor_2030
            .map(|value| fraction(&ctx("net_load_factor_2030"), value))
            .transpose()?,
        load_factor: raw
            .load_factor
            .map(|value| fraction(&ctx("load_factor"), value))
            .transpose()?,
        life_years,
        build_years,
        predev_years,
        predev_phasing,
        build_phasing,
        hurdle_rate,
        hurdle_rate_cepa_2024: raw
            .hurdle_rate_cepa_2024
            .map(|value| fraction(&ctx("hurdle_rate_cepa_2024"), value))
            .transpose()?,
        hurdle_rate_2023: raw
            .hurdle_rate_2023
            .map(|value| fraction(&ctx("hurdle_rate_2023"), value))
            .transpose()?,
        bracket_rule: raw.bracket_rule.clone(),
        quotable: raw.quotable.unwrap_or(true),
    })
}

fn validate_observed_nuclear(raw: &RawTechnology) -> Result<ObservedNuclearCost, GridError> {
    let ctx = |field: &str| format!("technologies.nuclear_observed: {field}");
    // The observed variant is a labelled project total, not a component
    // set — component fields on it would be a category error.
    if raw.capex_gbp_per_kw.is_some() || raw.capex_gbp2024_per_kw.is_some() {
        return Err(invalid(ctx(
            "carries component capex fields; it is a project-total variant",
        )));
    }
    let capex = positive(
        &ctx("capex_gbp_per_kw_project_total"),
        raw.capex_gbp_per_kw_project_total
            .ok_or_else(|| invalid(ctx("capex_gbp_per_kw_project_total is missing")))?,
    )?;
    let capacity = positive(
        &ctx("capacity_gw"),
        raw.capacity_gw
            .ok_or_else(|| invalid(ctx("capacity_gw is missing")))?,
    )?;
    Ok(ObservedNuclearCost {
        source: raw.source.clone(),
        capex_per_kw_project_total: CapacityCost::pounds_per_kilowatt(capex),
        capacity: Power::gigawatts(capacity),
        basis: raw
            .basis
            .clone()
            .ok_or_else(|| invalid(ctx("basis is missing")))?,
        bracket_rule: raw
            .bracket_rule
            .clone()
            .ok_or_else(|| invalid(ctx("bracket_rule is missing (review condition 4)")))?,
    })
}

fn validate_trajectory_axes(
    what: &str,
    years_axis: &[i64],
    low: &[f64],
    central: &[f64],
    high: &[f64],
) -> Result<(), GridError> {
    if years_axis.is_empty() {
        return Err(invalid(format!("{what}: empty year axis")));
    }
    if !years_axis.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(invalid(format!("{what}: years must be strictly ascending")));
    }
    for (name, series) in [("low", low), ("central", central), ("high", high)] {
        if series.len() != years_axis.len() {
            return Err(invalid(format!(
                "{what}: {name} has {} entries but years has {}",
                series.len(),
                years_axis.len()
            )));
        }
        for &value in series {
            positive(&format!("{what}.{name}"), value)?;
        }
    }
    for i in 0..years_axis.len() {
        if !(low[i] <= central[i] && central[i] <= high[i]) {
            return Err(invalid(format!(
                "{what}: year {}: scenarios are not ordered low ≤ central ≤ high",
                years_axis[i]
            )));
        }
    }
    Ok(())
}

fn validate_holding(what: &str, raw: &RawHoldingCost) -> Result<HoldingCost, GridError> {
    let [p5, p95] = raw.range_p5_p95[..] else {
        return Err(invalid(format!(
            "{what}: range_p5_p95 must have exactly two entries"
        )));
    };
    if p5 > p95 {
        return Err(invalid(format!("{what}: p5 {p5} exceeds p95 {p95}")));
    }
    Ok(HoldingCost {
        central: Price::pounds_per_megawatt_hour(raw.central),
        range_p5_p95: [
            Price::pounds_per_megawatt_hour(p5),
            Price::pounds_per_megawatt_hour(p95),
        ],
        mean_unweighted: Price::pounds_per_megawatt_hour(raw.mean_unweighted),
    })
}

impl CostsReference {
    /// Parse a costs-reference file from TOML text (strict; see module
    /// docs).
    pub fn from_toml_str(toml_text: &str) -> Result<Self, GridError> {
        let parse_err = |source: toml::de::Error| GridError::CostsReferenceParse {
            source: Box::new(source),
        };
        // Schema first, leniently, so a revision mismatch is reported
        // as such rather than as an arbitrary field error.
        let probe: SchemaProbe = toml::from_str(toml_text).map_err(parse_err)?;
        match probe.schema.as_deref() {
            None => {
                return Err(invalid(format!(
                    "missing mandatory `schema` field (this engine reads \
                     {COSTS_REFERENCE_SCHEMA:?})"
                )));
            }
            Some(found) if found != COSTS_REFERENCE_SCHEMA => {
                return Err(invalid(format!(
                    "unsupported schema {found:?}: this engine reads \
                     {COSTS_REFERENCE_SCHEMA:?}"
                )));
            }
            Some(_) => {}
        }
        let raw: RawReference = toml::from_str(toml_text).map_err(parse_err)?;
        Self::validate(raw)
    }

    /// Read and parse a costs-reference file, attaching the path to any
    /// error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InCostsReferenceFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one linear pass over the file's tables"
    )]
    fn validate(raw: RawReference) -> Result<Self, GridError> {
        // Deflator.
        let mut to_2024 = BTreeMap::new();
        for (key, value) in &raw.deflator.to_2024 {
            positive(&format!("deflator.to_2024.{key}"), *value)?;
            to_2024.insert(key.clone(), PerUnit::new(*value));
        }

        // Sources.
        let sources = raw
            .sources
            .iter()
            .map(|(key, source)| {
                (
                    key.clone(),
                    SourceRecord {
                        title: source.title.clone(),
                        licence: source.licence.clone(),
                        url: source.url.clone(),
                        sha256: source.sha256.clone(),
                    },
                )
            })
            .collect();

        // WACC set: three real rates strictly inside (0, 1), ordered.
        let (low, central, high) = (raw.wacc.real_low, raw.wacc.real_central, raw.wacc.real_high);
        for (name, value) in [
            ("real_low", low),
            ("real_central", central),
            ("real_high", high),
        ] {
            fraction(&format!("wacc.{name}"), value)?;
        }
        if !(low < central && central < high) {
            return Err(invalid(format!(
                "wacc set [{low}, {central}, {high}] is not ordered low < central < high \
                 (D8 rule 4)"
            )));
        }
        let [anchor_low_a, anchor_low_b] = raw.wacc.anchors_unrounded.low_range[..] else {
            return Err(invalid(
                "wacc.anchors_unrounded.low_range must have exactly two entries".to_owned(),
            ));
        };
        let anchors_unrounded = WaccAnchors {
            low_range: [
                fraction("wacc.anchors_unrounded.low_range[0]", anchor_low_a)?,
                fraction("wacc.anchors_unrounded.low_range[1]", anchor_low_b)?,
            ],
            central: fraction(
                "wacc.anchors_unrounded.central",
                raw.wacc.anchors_unrounded.central,
            )?,
            high: fraction(
                "wacc.anchors_unrounded.high",
                raw.wacc.anchors_unrounded.high,
            )?,
        };
        let sensitivity = &raw.wacc.per_tech_sensitivity;
        let mut per_tech_sensitivity = BTreeMap::new();
        for (name, value) in [
            ("solar_pv", sensitivity.solar_pv),
            ("onshore_wind", sensitivity.onshore_wind),
            ("offshore_wind_fixed", sensitivity.offshore_wind_fixed),
            ("offshore_wind_floating", sensitivity.offshore_wind_floating),
            ("biomass_unabated", sensitivity.biomass_unabated),
            ("nuclear_large_rab", sensitivity.nuclear_large_rab),
            ("gas_unabated", sensitivity.gas_unabated),
            ("battery_li_ion_cm", sensitivity.battery_li_ion_cm),
            ("pumped_hydro_cap_floor", sensitivity.pumped_hydro_cap_floor),
            ("ldes_novel", sensitivity.ldes_novel),
            (
                "hydrogen_ccht_ocht_mature",
                sensitivity.hydrogen_ccht_ocht_mature,
            ),
            (
                "hydrogen_ccht_ocht_emerging",
                sensitivity.hydrogen_ccht_ocht_emerging,
            ),
            ("hydrogen_electrolyser", sensitivity.hydrogen_electrolyser),
            (
                "interconnector_cap_floor",
                sensitivity.interconnector_cap_floor,
            ),
        ] {
            per_tech_sensitivity.insert(
                name.to_owned(),
                fraction(&format!("wacc.per_tech_sensitivity.{name}"), value)?,
            );
        }
        let wacc = WaccAssumptions {
            set: WaccBand {
                low: PerUnit::new(low),
                central: PerUnit::new(central),
                high: PerUnit::new(high),
            },
            anchors_unrounded,
            sources: raw.wacc.sources.clone(),
            per_tech_source: sensitivity.source.clone(),
            per_tech_sensitivity,
        };

        // Technologies: the observed nuclear variant is validated to
        // its own shape; every other row is a component-cost row.
        let mut technologies = BTreeMap::new();
        let mut nuclear_observed = None;
        for (key, tech) in &raw.technologies {
            if key == "nuclear_observed" {
                nuclear_observed = Some(validate_observed_nuclear(tech)?);
            } else {
                if tech.capex_gbp_per_kw_project_total.is_some() {
                    return Err(invalid(format!(
                        "technologies.{key}: project-total capex on a component row \
                         (only nuclear_observed is a project-total variant)"
                    )));
                }
                technologies.insert(key.clone(), validate_technology(key, tech)?);
            }
        }
        let nuclear_observed = nuclear_observed.ok_or_else(|| {
            invalid(
                "technologies.nuclear_observed is missing (the mandatory nuclear bracket, \
                 review condition 4)"
                    .to_owned(),
            )
        })?;
        if let Some(nuclear) = technologies.get("nuclear")
            && nuclear.bracket_rule.is_none()
        {
            return Err(invalid(
                "technologies.nuclear: bracket_rule is missing (review condition 4)".to_owned(),
            ));
        }

        // Battery (quarantined row).
        let battery_raw = &raw.storage.battery_li_ion;
        let battery = BatteryCosts {
            source: battery_raw.source.clone(),
            quotable: battery_raw.quotable,
            power_per_kw_2030_build: CapacityCost::pounds_per_kilowatt(positive(
                "storage.battery_li_ion: power 2030-build",
                battery_raw.power_gbp2024_per_kw_2030build,
            )?),
            energy_per_kwh_2030_build: EnergyCapacityCost::pounds_per_kilowatt_hour(positive(
                "storage.battery_li_ion: energy 2030-build",
                battery_raw.energy_gbp2024_per_kwh_2030build,
            )?),
            power_per_kw_2018_build: CapacityCost::pounds_per_kilowatt(positive(
                "storage.battery_li_ion: power 2018-build",
                battery_raw.power_gbp2024_per_kw_2018build,
            )?),
            energy_per_kwh_2018_build: EnergyCapacityCost::pounds_per_kilowatt_hour(positive(
                "storage.battery_li_ion: energy 2018-build",
                battery_raw.energy_gbp2024_per_kwh_2018build,
            )?),
            // £/kW/yr → the canonical £/MW/yr.
            fom_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(
                non_negative(
                    "storage.battery_li_ion: fom",
                    battery_raw.fom_gbp2024_per_kw_yr,
                )? * 1.0e3,
            ),
            life_years: years("storage.battery_li_ion: life_years", battery_raw.life_years)?,
            cycle_life: years("storage.battery_li_ion: cycle_life", battery_raw.cycle_life)?,
            round_trip_efficiency: fraction(
                "storage.battery_li_ion: round_trip_efficiency",
                battery_raw.round_trip_efficiency,
            )?,
            usable_depth_of_discharge: fraction(
                "storage.battery_li_ion: usable_depth_of_discharge",
                battery_raw.usable_depth_of_discharge,
            )?,
            build_years: years(
                "storage.battery_li_ion: build_years",
                battery_raw.build_years,
            )?,
            build_phasing: phasing(
                "storage.battery_li_ion: build_phasing",
                &battery_raw.build_phasing,
            )?,
            hurdle_rate_cepa_2024: fraction(
                "storage.battery_li_ion: hurdle_rate_cepa_2024",
                battery_raw.hurdle_rate_cepa_2024,
            )?,
            staleness_stamp: battery_raw.staleness_stamp.clone(),
        };
        // The condition-3 battery quarantine guard that rejected
        // `quotable = true` here was REMOVED 2026-07-06 as part of the
        // coordinated reviewed lift the guard's own error message
        // demanded: condition 3.i (NREL 2025 bracket re-verification
        // against the primary) was discharged 2026-07-06
        // (NREL/TP-6A40-93281 via OSTI, sha256-pinned in
        // data/packs/costs-evidence.sha256; evidence committed 23676f1),
        // and this revision is the reference+engine change citing it.
        // The flag stays load-bearing and byte-pinned in
        // grid-core/tests/costs_reference.rs (now to the LIFTED state);
        // caveats 3.ii/3.iii still travel on the row.

        // Electrolyser.
        let electrolyser_raw = &raw.storage.hydrogen.electrolyser;
        let electrolyser_capex_2020 = bracket(
            "storage.hydrogen.electrolyser: capex_gbp2020_per_kw_h2_hhv_2030",
            &electrolyser_raw.capex_gbp2020_per_kw_h2_hhv_2030,
        )?;
        let electrolyser = ElectrolyserCosts {
            source: electrolyser_raw.source.clone(),
            tech: electrolyser_raw.tech.clone(),
            capex_per_kw_h2_hhv_2030_gbp2020: Bracket {
                low: CapacityCost::pounds_per_kilowatt(electrolyser_capex_2020.low),
                central: CapacityCost::pounds_per_kilowatt(electrolyser_capex_2020.central),
                high: CapacityCost::pounds_per_kilowatt(electrolyser_capex_2020.high),
            },
            capex_per_kwe_2030_central: CapacityCost::pounds_per_kilowatt(positive(
                "storage.hydrogen.electrolyser: capex per kWe",
                electrolyser_raw.capex_gbp2024_per_kwe_2030_central,
            )?),
            efficiency_kwhe_per_kwh_h2_hhv: PerUnit::new(positive(
                "storage.hydrogen.electrolyser: efficiency_kwhe_per_kwh_h2_hhv",
                electrolyser_raw.efficiency_kwhe_per_kwh_h2_hhv,
            )?),
            fom_per_mw_h2_yr: AnnualCapacityCost::pounds_per_megawatt_year(
                non_negative(
                    "storage.hydrogen.electrolyser: fom",
                    electrolyser_raw.fom_gbp2024_per_kw_h2_yr,
                )? * 1.0e3,
            ),
            vom_per_mwh_h2: Price::pounds_per_megawatt_hour(non_negative(
                "storage.hydrogen.electrolyser: vom",
                electrolyser_raw.vom_gbp2024_per_mwh_h2,
            )?),
            life_years: years(
                "storage.hydrogen.electrolyser: life_years",
                electrolyser_raw.life_years,
            )?,
            build_years: years(
                "storage.hydrogen.electrolyser: build_years",
                electrolyser_raw.build_years,
            )?,
            build_phasing: phasing(
                "storage.hydrogen.electrolyser: build_phasing",
                &electrolyser_raw.build_phasing,
            )?,
            hurdle_rate_hpc2021: fraction(
                "storage.hydrogen.electrolyser: hurdle_rate_hpc2021",
                electrolyser_raw.hurdle_rate_hpc2021,
            )?,
            hurdle_rate_cepa_2024: fraction(
                "storage.hydrogen.electrolyser: hurdle_rate_cepa_2024",
                electrolyser_raw.hurdle_rate_cepa_2024,
            )?,
            alkaline_capex_per_kwe_2030_central: CapacityCost::pounds_per_kilowatt(positive(
                "storage.hydrogen.electrolyser: alkaline capex per kWe",
                electrolyser_raw.alkaline_capex_gbp2024_per_kwe_2030_central,
            )?),
        };

        // Cavern (levelised-with-convention).
        let cavern_raw = &raw.storage.hydrogen.cavern;
        let hydrogen_cavern = HydrogenCavernCosts {
            source: cavern_raw.source.clone(),
            levelised_per_kg: CostPerMass::pounds_per_kilogram(positive(
                "storage.hydrogen.cavern: levelised per kg",
                cavern_raw.levelised_gbp2024_per_kg,
            )?),
            levelised_per_mwh_h2_hhv: Price::pounds_per_megawatt_hour(positive(
                "storage.hydrogen.cavern: levelised per MWh",
                cavern_raw.levelised_gbp2024_per_mwh_h2_hhv,
            )?),
            cycles_per_year_assumed: years(
                "storage.hydrogen.cavern: cycles_per_year_assumed",
                cavern_raw.cycles_per_year_assumed,
            )?,
            basis: cavern_raw.basis.clone(),
            binding_convention: cavern_raw.binding_convention.clone(),
        };

        // OCHT (publication-gated).
        let ocht_raw = &raw.storage.hydrogen.reconversion_ocht;
        let ocht_capex = bracket(
            "storage.hydrogen.reconversion_ocht: capex",
            &ocht_raw.capex_gbp_per_kw,
        )?;
        let ocht_hurdle = bracket(
            "storage.hydrogen.reconversion_ocht: hurdle_rate",
            &ocht_raw.hurdle_rate,
        )?;
        let hydrogen_reconversion_ocht = OchtCosts {
            source: ocht_raw.source.clone(),
            capex_per_kw: Bracket {
                low: CapacityCost::pounds_per_kilowatt(ocht_capex.low),
                central: CapacityCost::pounds_per_kilowatt(ocht_capex.central),
                high: CapacityCost::pounds_per_kilowatt(ocht_capex.high),
            },
            infrastructure_per_kw: CapacityCost::pounds_per_kilowatt(non_negative(
                "storage.hydrogen.reconversion_ocht: infrastructure",
                ocht_raw.infrastructure_gbp_per_kw,
            )?),
            fom_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(non_negative(
                "storage.hydrogen.reconversion_ocht: fom",
                ocht_raw.fom_gbp_per_mw_yr,
            )?),
            insurance_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(non_negative(
                "storage.hydrogen.reconversion_ocht: insurance",
                ocht_raw.insurance_gbp_per_mw_yr,
            )?),
            connection_per_mw_yr: AnnualCapacityCost::pounds_per_megawatt_year(non_negative(
                "storage.hydrogen.reconversion_ocht: connection",
                ocht_raw.connection_gbp_per_mw_yr,
            )?),
            vom_per_mwh: Price::pounds_per_megawatt_hour(non_negative(
                "storage.hydrogen.reconversion_ocht: vom",
                ocht_raw.vom_gbp_per_mwh,
            )?),
            efficiency_hhv_default: fraction(
                "storage.hydrogen.reconversion_ocht: efficiency_hhv_default",
                ocht_raw.efficiency_hhv_default,
            )?,
            efficiency_hhv_sensitivity_labelled: fraction(
                "storage.hydrogen.reconversion_ocht: efficiency_hhv_sensitivity_labelled",
                ocht_raw.efficiency_hhv_sensitivity_labelled,
            )?,
            publication_gate: ocht_raw.publication_gate.clone(),
            life_years: years(
                "storage.hydrogen.reconversion_ocht: life_years",
                ocht_raw.life_years,
            )?,
            build_years: years(
                "storage.hydrogen.reconversion_ocht: build_years",
                ocht_raw.build_years,
            )?,
            predev_years: years(
                "storage.hydrogen.reconversion_ocht: predev_years",
                ocht_raw.predev_years,
            )?,
            predev_phasing: phasing(
                "storage.hydrogen.reconversion_ocht: predev_phasing",
                &ocht_raw.predev_phasing,
            )?,
            build_phasing: phasing(
                "storage.hydrogen.reconversion_ocht: build_phasing",
                &ocht_raw.build_phasing,
            )?,
            hurdle_rate: Bracket {
                low: fraction(
                    "storage.hydrogen.reconversion_ocht: hurdle_rate.low",
                    ocht_hurdle.low,
                )?,
                central: fraction(
                    "storage.hydrogen.reconversion_ocht: hurdle_rate.central",
                    ocht_hurdle.central,
                )?,
                high: fraction(
                    "storage.hydrogen.reconversion_ocht: hurdle_rate.high",
                    ocht_hurdle.high,
                )?,
            },
        };

        // Trajectories.
        let gas_raw = &raw.trajectories.gas;
        validate_trajectory_axes(
            "trajectories.gas",
            &gas_raw.years,
            &gas_raw.low,
            &gas_raw.central,
            &gas_raw.high,
        )?;
        let factor = positive(
            "trajectories.gas: p_per_therm_to_gbp_per_mwh_hhv",
            gas_raw.p_per_therm_to_gbp_per_mwh_hhv,
        )?;
        let convert = |series: &[f64]| {
            series
                .iter()
                .map(|&p_per_therm| Price::pounds_per_megawatt_hour(p_per_therm * factor))
                .collect()
        };
        let gas_trajectory = GasTrajectory {
            source: gas_raw.source.clone(),
            unit: gas_raw.unit.clone(),
            years: gas_raw.years.clone(),
            low: convert(&gas_raw.low),
            central: convert(&gas_raw.central),
            high: convert(&gas_raw.high),
            p_per_therm_to_gbp_per_mwh_hhv: factor,
        };

        let carbon_raw = &raw.trajectories.carbon;
        validate_trajectory_axes(
            "trajectories.carbon",
            &carbon_raw.years,
            &carbon_raw.low,
            &carbon_raw.central,
            &carbon_raw.high,
        )?;
        let to_carbon = |series: &[f64]| {
            series
                .iter()
                .map(|&value| CarbonPrice::pounds_per_tonne_co2(value))
                .collect()
        };
        let carbon_trajectory = CarbonTrajectory {
            source: carbon_raw.source.clone(),
            unit: carbon_raw.unit.clone(),
            years: carbon_raw.years.clone(),
            low: to_carbon(&carbon_raw.low),
            central: to_carbon(&carbon_raw.central),
            high: to_carbon(&carbon_raw.high),
            cps_nominal: CarbonPrice::pounds_per_tonne_co2(non_negative(
                "trajectories.carbon: cps_gbp_per_tco2_nominal",
                carbon_raw.cps_gbp_per_tco2_nominal,
            )?),
            cps_convention: carbon_raw.cps_convention.clone(),
        };

        // Holding costs, keyed to the response-holdings service names.
        let holdings_raw = &raw.holding_costs;
        let mut services = BTreeMap::new();
        for (name, entry) in [
            (
                "dynamic_containment_lf",
                &holdings_raw.dynamic_containment_lf,
            ),
            ("dynamic_moderation_lf", &holdings_raw.dynamic_moderation_lf),
            ("dynamic_regulation_lf", &holdings_raw.dynamic_regulation_lf),
        ] {
            services.insert(
                name.to_owned(),
                validate_holding(&format!("holding_costs.{name}"), entry)?,
            );
        }
        let holding_costs = HoldingCosts {
            source: holdings_raw.source.clone(),
            unit: holdings_raw.unit.clone(),
            period: holdings_raw.period.clone(),
            services,
            high_frequency: HighFrequencyHoldingCosts {
                dch: Price::pounds_per_megawatt_hour(holdings_raw.high_frequency_products.dch),
                dmh: Price::pounds_per_megawatt_hour(holdings_raw.high_frequency_products.dmh),
                drh: Price::pounds_per_megawatt_hour(holdings_raw.high_frequency_products.drh),
            },
        };

        // Interconnectors: a consumable capex may exist only on
        // verified rows (the quarantined figures stay unexposed).
        let mut interconnectors = BTreeMap::new();
        for (key, row) in &raw.interconnectors {
            let ctx = |field: &str| format!("interconnectors.{key}: {field}");
            if row.capex_gbp_bn.is_some() && !(row.verified && row.quotable) {
                return Err(invalid(ctx(
                    "carries a point GBP capex but is not verified+quotable — quarantined rows \
                     must not expose a consumable figure (review condition 9)",
                )));
            }
            let capex = row
                .capex_gbp_bn
                .map(|value| positive(&ctx("capex_gbp_bn"), value))
                .transpose()?
                .map(|billions| Money::pounds(billions * 1.0e9));
            interconnectors.insert(
                key.clone(),
                InterconnectorCosts {
                    source: row.source.clone(),
                    verified: row.verified,
                    quotable: row.quotable,
                    capex,
                    capacity: Power::gigawatts(positive(&ctx("capacity_gw"), row.capacity_gw)?),
                    length: Length::kilometres(positive(&ctx("length_km"), row.length_km)?),
                    status: row.status.clone(),
                },
            );
        }

        // Emission factors (rule 9).
        let coal = &raw.emission_factor.coal_electricity_generation;
        if coal.co2e_tonnes_per_mwh_th_hhv < coal.co2_tonnes_per_mwh_th_hhv {
            return Err(invalid(
                "emission_factor.coal_electricity_generation: CO2e factor is below the CO2-only \
                 factor (CO2e includes CO2)"
                    .to_owned(),
            ));
        }
        let emission_factors = EmissionFactors {
            coal_co2_per_mwh_th_hhv: EmissionsRate::tonnes_per_megawatt_hour(non_negative(
                "emission_factor.coal_electricity_generation: co2",
                coal.co2_tonnes_per_mwh_th_hhv,
            )?),
            coal_co2e_per_mwh_th_hhv: EmissionsRate::tonnes_per_megawatt_hour(non_negative(
                "emission_factor.coal_electricity_generation: co2e",
                coal.co2e_tonnes_per_mwh_th_hhv,
            )?),
            biomass_co2_for_pricing: EmissionsRate::tonnes_per_megawatt_hour(non_negative(
                "emission_factor.biomass: co2 for pricing",
                raw.emission_factor
                    .biomass
                    .co2_tonnes_per_mwh_th_for_pricing,
            )?),
            biomass_co2e_non_co2: EmissionsRate::tonnes_per_megawatt_hour(non_negative(
                "emission_factor.biomass: co2e non-CO2",
                raw.emission_factor.biomass.co2e_tonnes_per_mwh_th_non_co2,
            )?),
            other_co2_per_mwh_th_hhv: EmissionsRate::tonnes_per_megawatt_hour(non_negative(
                "emission_factor.other_category: co2",
                raw.emission_factor.other_category.co2_tonnes_per_mwh_th_hhv,
            )?),
            other_convention: raw.emission_factor.other_category.convention.clone(),
        };

        Ok(Self {
            price_base: raw.price_base,
            assembled: raw.assembled.to_string(),
            deflator: Deflator {
                series: raw.deflator.series,
                licence: raw.deflator.licence,
                to_2024,
            },
            sources,
            wacc,
            technologies,
            nuclear_observed,
            battery,
            electrolyser,
            hydrogen_cavern,
            hydrogen_reconversion_ocht,
            gas_trajectory,
            carbon_trajectory,
            holding_costs,
            interconnectors,
            emission_factors,
        })
    }
}
