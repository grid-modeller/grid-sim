//! Scenario schema, version 8 (docs/03-domain-model.md — field names there
//! are normative; any change requires a `schema_version` bump and a
//! migration note in that document).
//!
//! What schema v8 added (the D16 geothermal depth continuum,
//! 2026-07-06; docs/notes/d16-geothermal-source-temperature.md) — one
//! optional field, so a v7 file migrates by setting
//! `schema_version = 8` and changing nothing else:
//!
//! - `[[zones.demand.heating.entries]]` GSHP entries gain an optional
//!   `resource_depth_m` ([`HeatingEntry::resource_depth_m`]): the
//!   geothermal resource depth in metres. Absent ⇒ the committed
//!   shallow-loop behaviour, byte-identical; present ⇒ the source mean
//!   warms with depth via the BGS-cited gradient
//!   (`data/reference/heating-cop.toml` `[geothermal]`,
//!   heating-cop-v2) with the direct-use handoff to the district
//!   effective COP ([`crate::heating`] module docs). GSHP-only —
//!   validation rejects it on `ashp`/`district_geothermal` entries.
//!   (The D10 EV overlay, adopted but not yet built, re-targets its
//!   schema bump to v9.)
//!
//! What schema v7 added (the D11 priced multi-zone dispatch package,
//! 2026-07-05; docs/notes/d11-priced-dispatch.md, ADR-9 touch-point) —
//! all optional or defaulted, so a v6 file migrates by setting
//! `schema_version = 7` and changing nothing else:
//!
//! - `[zones.pricing]` ([`ZonePricingSpec`]): the zone's own SRMC
//!   chain for the priced flow signal — a prices-reference file, one
//!   fuel-price trace per fuel, one SRMC recipe per dispatchable
//!   technology (the Stage 2 recipe reused unchanged, applied per
//!   zone), and the carbon basis: `carbon_flat_gbp_per_tco2` ABSENT
//!   means the reference file's UKA+CPS step series (the GB basis);
//!   PRESENT means a flat per-zone carbon level (the external-zone EUA
//!   basis — no daily EUA series is licence-clean, so the committed
//!   convention is a flat annual mean; prices-eu-2024.toml).
//! - `[dispatch] flow_signal` ([`FlowSignal`]): which flow-rule signal
//!   a multi-zone run equalises — `scarcity` (default: the Stage 5
//!   scarcity score, every pre-v7 scenario byte-identical) or
//!   `priced_ladder` (the D11 lexicographic signal: per-zone marginal
//!   SRMC primary, the Stage 5 scarcity score secondary; requires
//!   `[zones.pricing]` on every zone).
//!
//! What schema v6 added (the B6 two-zone package, 2026-07-04; the
//! link-convention ruling of docs/notes/b6-two-zone-data-review.md §6,
//! transcribed in docs/notes/b6-two-zone-data-report.md §8) — all
//! optional, so a v5 file migrates by setting `schema_version = 6` and
//! changing nothing else:
//!
//! - `[[links]]` entries gain an optional `reverse_capacity_gw`
//!   (capability of the reverse, `to → from`, direction; absent ⇒ the
//!   link is symmetric at `capacity_gw`, the pre-v6 behaviour) and an
//!   optional `[links.capability_trace]` table
//!   ([`LinkCapabilityTraceSpec`]): a per-period **forward**
//!   (`from → to`) capability series in MW that supersedes
//!   `capacity_gw` on horizon periods, with the ruling's sentinel
//!   handling stated in the scenario itself — values ≥
//!   `sentinel_high_mw` are "no constraint recorded" sentinels replaced
//!   by `upper_bound_gw` (the pinned planning upper bound); zero
//!   values, NaN rows and missing rows are MASKED (excluded from
//!   validation-gate arithmetic) and filled with `masked_fill_gw` (the
//!   pinned central value) for dispatch, which must run every period.
//!   `availability` still multiplies both directions.
//! - `[[zones.exogenous_supply]]` entries gain an optional `scale`
//!   (default 1.0): a flat multiplier on the summed MW columns — how a
//!   national exogenous series is split across zones (the 2-zone
//!   scenario's pumped-storage-net and FUELHH-"other" splits).
//!
//! What schema v5 changed (Q5/Q11 heating overlay, 2026-07-03; the
//! adopted design note docs/notes/d9-heating-overlay.md, rule 2):
//! `[zones.demand.heating]` is now the **heating technology portfolio**
//! ([`HeatingSpec`]) — `delivered_heat_twh` (the record-mean annual
//! buildings-heat quantum), `electrified_share`, `dhw_fraction`, the
//! pinned population-weighted temperature trace reference, and
//! `[[zones.demand.heating.entries]]` (kind ∈ ashp | gshp |
//! district_geothermal, share, optional per-entry COP-parameter
//! overrides). v5 **replaces** the v1–v4 sketch block
//! (`enabled` / `heat_demand_per_degree` / `cop_curve` — opaque
//! placeholders no engine code ever read): those fields are removed,
//! so v5 is NOT purely additive — a v4 file carrying the old block
//! fails with the structured migration message
//! ([`GridError::SchemaVersion4Superseded`]); a v4 file without it
//! migrates by changing only the version line. Default COP parameters
//! live in the cited, drift-guarded reference file
//! `data/reference/heating-cop.toml` ([`crate::heating`]), never in
//! free scenario text; per-entry overrides are legal and are always
//! echoed into run outputs.
//!
//! What schema v4 added (Stage 5, 2026-07-03) — the multi-zone activation
//! fields; all optional or defaulted, so a v3 file migrates by setting
//! `schema_version = 4` and changing nothing else:
//!
//! - `[[links]]` entries gain an optional `name` (per-link identity for
//!   outputs and per-border validation — two links may join the same
//!   zone pair, e.g. Nemo and BritNed into CONT-NW) and a `loss`
//!   fraction (default 0.0): the receiving end gets
//!   `sent × (1 − loss)` — the HVDC loss wedge between sending-end and
//!   receiving-end metering (docs/notes/entsoe-stage5-pack-report.md §3).
//! - `[[zones.fleet]]` entries gain an optional `energy_budget`
//!   ([`EnergyBudgetSpec`]): a windowed energy-release constraint for
//!   budget-limited dispatchables (the NO2 seasonal-budget reservoir
//!   hydro model — D5). The named MW columns of the trace are summed
//!   per window of `window_periods` half-hours; the engine releases
//!   each window's energy as dispatchable allowance (unused allowance
//!   carries forward — water stays in the reservoir).
//! - `[zones.demand]` gains `extra_profiles`, a list of
//!   `{ path, column }` MW traces summed onto the base profile before
//!   `annual_scale` — how an aggregate zone's demand (CONT-NW =
//!   BE + NL + DE-LU) is assembled from per-country load traces.
//!
//! What schema v3 added (Stage 6, 2026-07-03): the stability metadata
//! on `[[zones.fleet]]` entries — optional `inertia_h` (the inertia
//! constant H in seconds, machine-MVA base) and optional `synchronous`,
//! with per-technology derived defaults in [`crate::inertia`]
//! (transcribed from `data/reference/inertia-constants.toml`) and
//! overrides surfaced, exactly like the `reliability` field. Both
//! fields are optional, so a v2 file migrates by setting
//! `schema_version = 3` and changing nothing else; v2 files are
//! refused with that migration message (strictness rule: any schema
//! addition bumps the version).
//!
//! A scenario TOML file is **the complete, self-contained description of a
//! run**: shareable, diffable, hashable (ADR-5). Schema v2 (Stage 3,
//! 2026-07-02) restores that sentence to full strength by folding the
//! Stage 1 run-inputs companion file into the scenario itself — the
//! determinism formula returns to
//! `results = f(scenario, data pack, engine)`. The schema is multi-zone
//! from day one (ADR-7) even though the engine rejects >1 zone until
//! Stage 5.
//!
//! Parsing is strict:
//!
//! - `schema_version` is mandatory and must match [`SCHEMA_VERSION`];
//!   a missing or unsupported version is a structured error, never a
//!   silent reinterpretation. Version 1 files get a dedicated migration
//!   message naming exactly what moved (docs/05 rule 4).
//! - Unknown fields are rejected (`deny_unknown_fields`): a field the
//!   engine does not understand would make the file an *incomplete*
//!   description of the run the author intended.
//!
//! What schema v2 added (formerly the run-inputs file, plus Stage 3):
//!
//! - `zones.demand.column` and `zones.demand.extra_demand_gw` — demand
//!   column selection (D3 convention default) and the constant
//!   demand-side adjustment (station transformer load wedge).
//! - `[[zones.exogenous_supply]]` — exogenous must-take supply traces
//!   (net imports until Stage 5, FUELHH "other", …).
//! - `zones.fleet.availability` — per-technology availability models
//!   (`{ flat = 0.61 }` or `{ monthly = [ … ] }`).
//! - The top-level `[pricing]` block (Stage 2 pricing inputs).
//! - Storage: `initial_soc` (fraction of capacity, default full per D4)
//!   and the DSR-only fields `shift_duration` / `daily_volume_limit`
//!   (schema shape now; DSR engine semantics are provisional until Q6 —
//!   docs/notes/d4-rule-based-dispatch.md).
//! - Trace references ([`TraceFiles`]) accept a single Parquet path or a
//!   list of paths concatenated in order — multi-year horizons assembled
//!   from per-year trace files (docs/04 Stage 3).
//!
//! Carried Stage 0 notes:
//!
//! - `horizon.start`/`end` are stored as the RFC 3339 strings written in
//!   the file (lossless round-tripping); [`Horizon::start_instant`] etc.
//!   parse them on demand.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::GridError;
use crate::time::UtcInstant;
use crate::units::{CarbonPrice, Duration, Energy, InertiaConstant, Length, PerUnit, Power};

/// The scenario schema version this engine reads and writes.
pub const SCHEMA_VERSION: u32 = 8;

/// The default `energy_budget.window_periods`: one week of half-hourly
/// settlement periods (7 × 48 = 336) — the grain of the ENTSO-E
/// reservoir/inflow evidence behind the seasonal-budget hydro model
/// (docs/notes/entsoe-stage5-pack-report.md §6).
pub const DEFAULT_BUDGET_WINDOW_PERIODS: usize = 336;

/// Declares a transparent string identifier newtype.
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps a raw identifier string.
            #[must_use]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                // `pad`, not `write_str`, so callers' width/alignment
                // format specifiers are honoured.
                f.pad(&self.0)
            }
        }
    };
}

string_id!(
    /// A zone identifier (`GB`, `FR`, …). Link counterparties may name
    /// zones without `[[zones]]` entries while imports are exogenous.
    ZoneId
);

string_id!(
    /// A technology identifier (`ccgt`, `offshore_wind`, …). An open set:
    /// docs/03 lists the expected ids but scenarios may introduce more;
    /// the dispatch stages decide what they accept.
    TechId
);

/// A complete scenario: the root of the TOML file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Mandatory schema version (ADR-5); must equal [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Human-readable scenario name.
    pub name: String,
    /// Optional longer description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Simulation horizon.
    pub horizon: Horizon,
    /// Zones (ADR-7: `Vec<Zone>` from day one; v1 engine takes one).
    pub zones: Vec<ZoneSpec>,
    /// Interconnector matrix between zones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkSpec>,
    /// Dispatch policy selection (ADR-6).
    pub dispatch: Dispatch,
    /// Transmission-constraint cost approximation (ADR-12); optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Constraints>,
    /// Optional solver mode (e.g. bisection targets, ADR-10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver: Option<Solver>,
    /// Stage 2 pricing inputs (schema v2; formerly the run-inputs
    /// `[pricing]` section); absent for unpriced runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<PricingSpec>,
}

/// Version-only probe used to check `schema_version` before the strict
/// full parse, so version problems get their own clear errors.
#[derive(Deserialize)]
struct VersionProbe {
    schema_version: Option<i64>,
}

impl Scenario {
    /// Parse a scenario from TOML text.
    ///
    /// Errors: [`GridError::MissingSchemaVersion`] /
    /// [`GridError::UnsupportedSchemaVersion`] for version problems,
    /// [`GridError::ScenarioParse`] (with line/column context from the
    /// TOML parser) for everything else.
    pub fn from_toml_str(toml_text: &str) -> Result<Self, GridError> {
        // Version first, leniently, so a missing/wrong version is reported
        // as such rather than as an arbitrary field error.
        let probe: VersionProbe =
            toml::from_str(toml_text).map_err(|source| GridError::ScenarioParse {
                source: Box::new(source),
            })?;
        match probe.schema_version {
            None => return Err(GridError::MissingSchemaVersion),
            // Superseded versions get their own migration messages
            // naming what moved (docs/05 rule 4), not a generic
            // version error.
            Some(1) => return Err(GridError::SchemaVersion1Superseded),
            Some(2) => return Err(GridError::SchemaVersion2Superseded),
            Some(3) => return Err(GridError::SchemaVersion3Superseded),
            Some(4) => return Err(GridError::SchemaVersion4Superseded),
            Some(5) => return Err(GridError::SchemaVersion5Superseded),
            Some(6) => return Err(GridError::SchemaVersion6Superseded),
            Some(7) => return Err(GridError::SchemaVersion7Superseded),
            Some(v) if v != i64::from(SCHEMA_VERSION) => {
                return Err(GridError::UnsupportedSchemaVersion {
                    found: v,
                    supported: SCHEMA_VERSION,
                });
            }
            Some(_) => {}
        }
        toml::from_str(toml_text).map_err(|source| GridError::ScenarioParse {
            source: Box::new(source),
        })
    }

    /// Read and parse a scenario file, attaching the path to any error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InScenarioFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    /// Serialise to TOML text (the round-trip counterpart of
    /// [`Scenario::from_toml_str`]).
    pub fn to_toml_string(&self) -> Result<String, GridError> {
        toml::to_string(self).map_err(|source| GridError::ScenarioSerialise {
            source: Box::new(source),
        })
    }

    /// Semantic validation beyond what strict parsing can express.
    /// Called by the engines before a run and by `grid-cli validate`.
    ///
    /// Checks, per zone:
    ///
    /// - fleet technologies are unique within the zone
    ///   ([`GridError::DuplicateFleetTechnology`] — per-technology
    ///   inputs are keyed by `TechId`, so a duplicate entry silently
    ///   corrupts input assembly; b6 engine-review note 6);
    /// - storage `dispatch_order` values are unique (D4 rule 2 —
    ///   [`GridError::DuplicateDispatchOrder`]);
    /// - storage parameters are physical: `round_trip_efficiency` in
    ///   `(0, 1]`, `initial_soc` in `[0, 1]`, non-negative power and
    ///   energy ratings;
    /// - the DSR-only fields (`shift_duration`, `daily_volume_limit`)
    ///   appear only on `kind = "dsr"` stores;
    /// - `availability` models carry factors in `[0, 1]` (monthly:
    ///   exactly 12) and are not attached to weather-driven (CF-trace)
    ///   technologies, which are must-take and never consult one;
    /// - stability metadata (schema v3) is coherent: `inertia_h` is
    ///   positive and finite, never set on an effectively
    ///   non-synchronous entry, and every effectively synchronous entry
    ///   has an effective H (explicit or derived);
    /// - schema v4 multi-zone coherence: zone ids are unique; every
    ///   link's endpoints differ, its `availability` is in [0, 1], its
    ///   `loss` is in [0, 1); and in a multi-zone scenario both link
    ///   endpoints name declared zones (a single-zone scenario may keep
    ///   external counterparty ids while imports are exogenous — the
    ///   links are inert there);
    /// - `energy_budget` (schema v4) is coherent: dispatchable entries
    ///   only (never on a CF-trace entry), at least one column, a
    ///   positive window length;
    /// - schema v6 link capability is physical: `reverse_capacity_gw`
    ///   finite and non-negative; `capability_trace` sentinel threshold
    ///   finite and positive, its `upper_bound_gw`/`masked_fill_gw`
    ///   finite and non-negative; and each exogenous `scale` is a
    ///   finite, non-negative multiplier;
    /// - the heating portfolio (schema v5, D9 rule 2) is coherent:
    ///   entries present, `|Σ share − 1| ≤ 1e-9`
    ///   ([`GridError::HeatingShareSum`], naming the sum and the
    ///   entries), every share and fraction in [0, 1], the quantum
    ///   non-negative and finite, kinds unique (per-entry output series
    ///   are keyed by kind), and per-entry overrides on the right kind:
    ///   `cop_const` on district entries only, curve/correction/derating
    ///   on heat-pump entries only, all physical.
    pub fn validate(&self) -> Result<(), GridError> {
        let invalid = |reason: String| GridError::InvalidScenario { reason };

        // Zone ids must be unique: links address zones by id.
        for (index, zone) in self.zones.iter().enumerate() {
            if self.zones[..index].iter().any(|z| z.id == zone.id) {
                return Err(invalid(format!(
                    "zone id {} is declared more than once",
                    zone.id
                )));
            }
        }

        // Fleet TechIds must be unique WITHIN a zone (the same
        // technology in different zones is the normal multi-zone
        // pattern). The engines key per-technology inputs (CF traces,
        // availability models, energy budgets, SRMC recipes) by TechId
        // in maps — last entry wins — while dispatch builds one unit
        // per fleet entry (both dispatch) and result readouts take the
        // first series of a given id, so a duplicate silently corrupts
        // the run (b6 engine-review note 6, characterised 2026-07-06).
        for zone in &self.zones {
            for (index, entry) in zone.fleet.iter().enumerate() {
                if zone.fleet[..index]
                    .iter()
                    .any(|e| e.technology == entry.technology)
                {
                    return Err(GridError::DuplicateFleetTechnology {
                        zone: zone.id.as_str().to_owned(),
                        technology: entry.technology.as_str().to_owned(),
                    });
                }
            }
        }

        for (index, link) in self.links.iter().enumerate() {
            let label = link.name.clone().unwrap_or_else(|| format!("#{index}"));
            let context = format!("link {label} ({} -> {})", link.from, link.to);
            if link.from == link.to {
                return Err(invalid(format!(
                    "{context}: both endpoints name the same zone"
                )));
            }
            let availability = link.availability.value();
            if !(0.0..=1.0).contains(&availability) || availability.is_nan() {
                return Err(invalid(format!(
                    "{context}: availability {availability} is outside [0, 1]"
                )));
            }
            let loss = link.loss.value();
            if !(0.0..1.0).contains(&loss) || loss.is_nan() {
                return Err(invalid(format!(
                    "{context}: loss {loss} is outside [0, 1) — the receiving end gets \
                     sent × (1 − loss), so a loss of 1 or more delivers nothing"
                )));
            }
            // Schema v6: per-direction / time-series capability must be
            // physical (the sentinel handling is load-bearing gate
            // arithmetic — a NaN threshold would corrupt it silently).
            if let Some(reverse) = link.reverse_capacity_gw {
                let v = reverse.as_gigawatts();
                if !v.is_finite() || v < 0.0 {
                    return Err(invalid(format!(
                        "{context}: reverse_capacity_gw {v} must be a finite, non-negative \
                         capability (GW)"
                    )));
                }
            }
            if let Some(trace) = &link.capability_trace {
                let sentinel_mw = trace.sentinel_high_mw.as_gigawatts() * 1000.0;
                if !sentinel_mw.is_finite() || sentinel_mw <= 0.0 {
                    return Err(invalid(format!(
                        "{context}: capability_trace sentinel_high_mw {sentinel_mw} must be a \
                         finite, positive MW threshold"
                    )));
                }
                for (field, value) in [
                    ("upper_bound_gw", trace.upper_bound_gw.as_gigawatts()),
                    ("masked_fill_gw", trace.masked_fill_gw.as_gigawatts()),
                ] {
                    if !value.is_finite() || value < 0.0 {
                        return Err(invalid(format!(
                            "{context}: capability_trace {field} {value} must be a finite, \
                             non-negative capability (GW)"
                        )));
                    }
                }
            }
            // Multi-zone scenarios dispatch the links (Stage 5), so both
            // endpoints must exist; single-zone scenarios keep the links
            // inert (imports exogenous) and may name external ids.
            if self.zones.len() > 1 {
                for endpoint in [&link.from, &link.to] {
                    if !self.zones.iter().any(|z| &z.id == endpoint) {
                        return Err(invalid(format!(
                            "{context}: endpoint {endpoint} is not a declared zone — a \
                             multi-zone scenario dispatches its links, so every endpoint \
                             needs a [[zones]] entry (ADR-7)"
                        )));
                    }
                }
            }
        }

        // Schema v7 (D11): the priced ladder needs a marginal price for
        // every zone, so every zone must carry a [zones.pricing] block
        // (ADR-7 touch-point: external zones require pricing inputs to
        // be dispatchable under the ladder).
        if self.dispatch.flow_signal == FlowSignal::PricedLadder {
            for zone in &self.zones {
                if zone.pricing.is_none() {
                    return Err(invalid(format!(
                        "zone {}: dispatch.flow_signal = \"priced_ladder\" requires a \
                         [zones.pricing] block on every zone — the flow rule prices each \
                         zone's marginal SRMC (schema v7, D11)",
                        zone.id
                    )));
                }
            }
        }

        for zone in &self.zones {
            let invalid = |reason: String| GridError::InvalidScenario { reason };

            if let Some(heating) = &zone.demand.heating {
                validate_heating(&zone.id, heating)?;
            }

            // Schema v7 (D11): zone pricing coherence — a physical flat
            // carbon level; SRMC recipes naming dispatchable entries of
            // THIS zone's fleet (the load_pricing_inputs rules, checked
            // at validation so authoring errors fail early).
            if let Some(pricing) = &zone.pricing {
                if let Some(flat) = pricing.carbon_flat_gbp_per_tco2 {
                    let v = flat.as_pounds_per_tonne_co2();
                    if !v.is_finite() || v < 0.0 {
                        return Err(invalid(format!(
                            "zone {}, pricing: carbon_flat_gbp_per_tco2 {v} must be a \
                             finite, non-negative carbon price (£/tCO2)",
                            zone.id
                        )));
                    }
                }
                for (tech, recipe) in &pricing.srmc {
                    let entry = zone
                        .fleet
                        .iter()
                        .find(|e| e.technology.as_str() == tech)
                        .ok_or_else(|| {
                            invalid(format!(
                                "zone {}, pricing: srmc names technology {tech:?}, which is \
                                 not in this zone's fleet",
                                zone.id
                            ))
                        })?;
                    if entry.capacity_factor_trace.is_some() {
                        return Err(invalid(format!(
                            "zone {}, pricing: srmc names weather-driven technology \
                             {tech:?}; must-take technologies carry no SRMC model \
                             (grid-core pricing convention 1)",
                            zone.id
                        )));
                    }
                    if !pricing.fuel_price.contains_key(&recipe.fuel) {
                        return Err(invalid(format!(
                            "zone {}, pricing: srmc.{tech} names fuel {:?}, which has no \
                             fuel_price entry in this zone's pricing block",
                            zone.id, recipe.fuel
                        )));
                    }
                }
            }

            // Schema v6: the exogenous split multiplier must be a
            // physical share/factor (signs live in the trace).
            for supply in &zone.exogenous_supply {
                if !supply.scale.is_finite() || supply.scale < 0.0 {
                    return Err(invalid(format!(
                        "zone {}, exogenous supply {:?}: scale {} must be finite and \
                         non-negative (a split share or flat factor; the trace carries any \
                         sign convention)",
                        zone.id, supply.label, supply.scale
                    )));
                }
            }

            let mut orders_seen: Vec<u8> = Vec::with_capacity(zone.storage.len());
            for store in &zone.storage {
                let context = format!("zone {}, {} store", zone.id, store.kind);
                if orders_seen.contains(&store.dispatch_order) {
                    return Err(GridError::DuplicateDispatchOrder {
                        zone: zone.id.as_str().to_owned(),
                        order: store.dispatch_order,
                    });
                }
                orders_seen.push(store.dispatch_order);

                let rte = store.round_trip_efficiency.value();
                if rte <= 0.0 || rte > 1.0 || rte.is_nan() {
                    return Err(invalid(format!(
                        "{context}: round_trip_efficiency {rte} is outside (0, 1]"
                    )));
                }
                if let Some(soc) = store.initial_soc {
                    let v = soc.value();
                    if !(0.0..=1.0).contains(&v) || v.is_nan() {
                        return Err(invalid(format!(
                            "{context}: initial_soc {v} is outside [0, 1]"
                        )));
                    }
                }
                let power = store.power_gw.as_gigawatts();
                if power < 0.0 || power.is_nan() {
                    return Err(invalid(format!(
                        "{context}: power_gw {power} is negative or NaN"
                    )));
                }
                let energy = store.energy_gwh.as_gigawatt_hours();
                if energy < 0.0 || energy.is_nan() {
                    return Err(invalid(format!(
                        "{context}: energy_gwh {energy} is negative or NaN"
                    )));
                }
                if store.kind != StorageKind::Dsr {
                    if store.shift_duration.is_some() {
                        return Err(invalid(format!(
                            "{context}: shift_duration is a DSR-only field (ADR-8)"
                        )));
                    }
                    if store.daily_volume_limit.is_some() {
                        return Err(invalid(format!(
                            "{context}: daily_volume_limit is a DSR-only field (ADR-8)"
                        )));
                    }
                }
            }

            for entry in &zone.fleet {
                // Stability metadata (schema v3): H must be a physical
                // constant; an explicit H on an effectively
                // non-synchronous entry is contradictory (decoupled
                // rotor energy contributes nothing — set
                // `synchronous = true` to claim otherwise); an
                // effectively synchronous entry must have an H from
                // somewhere (explicit, or the derived default).
                if let Some(h) = entry.inertia_h {
                    let v = h.as_seconds();
                    if v.is_nan() || v <= 0.0 || v.is_infinite() {
                        return Err(invalid(format!(
                            "zone {}, technology {}: inertia_h {v} s must be a positive, \
                             finite inertia constant",
                            zone.id, entry.technology
                        )));
                    }
                    if !entry.effective_synchronous() {
                        return Err(invalid(format!(
                            "zone {}, technology {}: inertia_h is set but the entry is not \
                             synchronous — decoupled plant contributes no inertia; set \
                             synchronous = true to model a synchronous coupling",
                            zone.id, entry.technology
                        )));
                    }
                }
                if entry.effective_synchronous() && entry.effective_inertia_h().is_none() {
                    return Err(invalid(format!(
                        "zone {}, technology {}: synchronous but no inertia_h and no derived \
                         default exists for this technology — set inertia_h explicitly \
                         (H in seconds; see data/reference/inertia-constants.toml)",
                        zone.id, entry.technology
                    )));
                }

                if let Some(budget) = &entry.energy_budget {
                    let budget_context =
                        format!("zone {}, technology {}", zone.id, entry.technology);
                    if entry.capacity_factor_trace.is_some() {
                        return Err(invalid(format!(
                            "{budget_context}: energy_budget on a weather-driven \
                             (capacity_factor_trace) technology — a must-take entry is not \
                             dispatched, so a budget cannot constrain it (schema v4)"
                        )));
                    }
                    if budget.columns.is_empty() {
                        return Err(invalid(format!(
                            "{budget_context}: energy_budget lists no columns"
                        )));
                    }
                    if budget.window_periods == 0 {
                        return Err(invalid(format!(
                            "{budget_context}: energy_budget window_periods must be at least 1"
                        )));
                    }
                }

                let Some(availability) = &entry.availability else {
                    continue;
                };
                if entry.capacity_factor_trace.is_some() {
                    return Err(invalid(format!(
                        "zone {}, technology {}: weather-driven (capacity_factor_trace) \
                         technologies are must-take and carry no availability model",
                        zone.id, entry.technology
                    )));
                }
                let check = |factor: PerUnit, what: &str| -> Result<(), GridError> {
                    let v = factor.value();
                    if !(0.0..=1.0).contains(&v) || v.is_nan() {
                        return Err(GridError::InvalidScenario {
                            reason: format!(
                                "zone {}, technology {}: {what} availability factor {v} is \
                                 outside [0, 1]",
                                zone.id, entry.technology
                            ),
                        });
                    }
                    Ok(())
                };
                match availability {
                    AvailabilitySpec::Flat { flat } => check(*flat, "flat")?,
                    AvailabilitySpec::Monthly { monthly } => {
                        if monthly.len() != 12 {
                            return Err(invalid(format!(
                                "zone {}, technology {}: a monthly availability profile needs \
                                 exactly 12 factors, got {}",
                                zone.id,
                                entry.technology,
                                monthly.len()
                            )));
                        }
                        for factor in monthly {
                            check(*factor, "monthly")?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// The D9 rule-2 share-sum tolerance: `|Σ share − 1| ≤ 1e-9`.
pub const HEATING_SHARE_SUM_TOLERANCE: f64 = 1e-9;

/// Semantic validation of one zone's heating portfolio (schema v5, D9
/// rule 2; see [`Scenario::validate`] for the checklist).
fn validate_heating(zone: &ZoneId, heating: &HeatingSpec) -> Result<(), GridError> {
    let invalid = |reason: String| GridError::InvalidScenario {
        reason: format!("zone {zone}, heating: {reason}"),
    };

    let quantum = heating.delivered_heat_twh.as_gigawatt_hours();
    if !quantum.is_finite() || quantum < 0.0 {
        return Err(invalid(format!(
            "delivered_heat_twh {} TWh must be a non-negative, finite quantum",
            quantum / 1000.0
        )));
    }
    let fraction = |value: PerUnit, what: &str| -> Result<(), GridError> {
        let v = value.value();
        if !(0.0..=1.0).contains(&v) || v.is_nan() {
            return Err(invalid(format!("{what} {v} is outside [0, 1]")));
        }
        Ok(())
    };
    fraction(heating.electrified_share, "electrified_share")?;
    fraction(heating.dhw_fraction, "dhw_fraction")?;

    if heating.entries.is_empty() {
        return Err(invalid(
            "the portfolio has no entries — at least one [[zones.demand.heating.entries]] \
             table is required (D9 rule 2)"
                .to_owned(),
        ));
    }

    let mut share_sum = 0.0;
    for (index, entry) in heating.entries.iter().enumerate() {
        fraction(entry.share, &format!("entry {} share", entry.kind))?;
        share_sum += entry.share.value();

        // Per-kind output series: a duplicate kind would be an
        // ambiguous label (and one technology's share belongs on one
        // entry).
        if heating.entries[..index]
            .iter()
            .any(|e| e.kind == entry.kind)
        {
            return Err(invalid(format!(
                "kind {} appears more than once — per-entry output series are keyed by \
                 kind, so each technology takes exactly one entry",
                entry.kind
            )));
        }

        // Field placement: cop_const is the district parameter; the
        // curve family belongs to the heat pumps (D9 rule 4); the D16
        // resource depth belongs to the ground source alone (air has
        // no depth; district is already the direct-use endpoint).
        if entry.kind != HeatingKind::Gshp
            && let Some(depth) = entry.resource_depth_m
        {
            return Err(invalid(format!(
                "entry {}: resource_depth_m ({} m) is a gshp-only field — the D16 depth \
                 continuum re-anchors the ground-source wave; air has no resource depth and \
                 district_geothermal is already the direct-use endpoint",
                entry.kind,
                depth.as_metres()
            )));
        }
        if let Some(depth) = entry.resource_depth_m {
            let metres = depth.as_metres();
            if !metres.is_finite() || metres <= 0.0 {
                return Err(invalid(format!(
                    "entry {}: resource_depth_m {metres} must be positive and finite",
                    entry.kind
                )));
            }
        }
        if entry.kind.is_heat_pump() {
            if entry.cop_const.is_some() {
                return Err(invalid(format!(
                    "entry {}: cop_const is a district_geothermal-only override — heat-pump \
                     COPs come from the rule-4 curve (cop_curve / correction_factor / \
                     rhpp_derating)",
                    entry.kind
                )));
            }
        } else {
            for (field, present) in [
                ("cop_curve", entry.cop_curve.is_some()),
                ("correction_factor", entry.correction_factor.is_some()),
                ("rhpp_derating", entry.rhpp_derating.is_some()),
            ] {
                if present {
                    return Err(invalid(format!(
                        "entry {}: {field} is a heat-pump-only override — district \
                         geothermal carries only cop_const (a constant effective COP)",
                        entry.kind
                    )));
                }
            }
        }
        let positive = |value: f64, what: &str| -> Result<(), GridError> {
            if !value.is_finite() || value <= 0.0 {
                return Err(invalid(format!(
                    "entry {}: {what} {value} must be positive and finite",
                    entry.kind
                )));
            }
            Ok(())
        };
        if let Some(factor) = entry.correction_factor {
            positive(factor.value(), "correction_factor")?;
        }
        if let Some(factor) = entry.rhpp_derating {
            positive(factor.value(), "rhpp_derating")?;
        }
        if let Some(cop) = entry.cop_const {
            positive(cop, "cop_const")?;
        }
        if let Some(curve) = entry.cop_curve
            && curve.iter().any(|c| !c.is_finite())
        {
            return Err(invalid(format!(
                "entry {}: cop_curve coefficients must be finite, got {curve:?}",
                entry.kind
            )));
        }
    }

    if (share_sum - 1.0).abs() > HEATING_SHARE_SUM_TOLERANCE {
        return Err(GridError::HeatingShareSum {
            zone: zone.as_str().to_owned(),
            sum: share_sum,
            entries: heating
                .entries
                .iter()
                .map(|e| format!("{} = {}", e.kind, e.share.value()))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    Ok(())
}

/// Simulation horizon: UTC half-hourly settlement periods (ADR-3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Horizon {
    /// Start of the first settlement period, RFC 3339 UTC.
    pub start: String,
    /// Start of the last settlement period, RFC 3339 UTC (inclusive).
    pub end: String,
    /// Which weather years drive the run.
    pub weather_years: WeatherYears,
}

impl Horizon {
    /// The parsed start instant.
    pub fn start_instant(&self) -> Result<UtcInstant, GridError> {
        UtcInstant::parse(&self.start)
    }

    /// The parsed end instant (start of the last period).
    pub fn end_instant(&self) -> Result<UtcInstant, GridError> {
        UtcInstant::parse(&self.end)
    }

    /// Number of half-hourly settlement periods in the horizon, both
    /// endpoints included (17,568 for calendar leap-year 2024).
    pub fn period_count(&self) -> Result<usize, GridError> {
        self.start_instant()?
            .periods_until_inclusive(self.end_instant()?)
    }
}

/// Weather-year selection: `"all"`, `"worst_on_record"`, or an explicit
/// year list like `[2010]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "WeatherYearsRepr", into = "WeatherYearsRepr")]
pub enum WeatherYears {
    /// Every year in the weather record (`"all"`).
    All,
    /// The single worst year on record, per the engine's adequacy metric
    /// (`"worst_on_record"`).
    WorstOnRecord,
    /// An explicit list of calendar years (`[2010, 2024]`).
    Years(Vec<i32>),
}

/// TOML-facing representation of [`WeatherYears`]: an untagged
/// keyword-string-or-year-array.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum WeatherYearsRepr {
    Keyword(WeatherYearsKeyword),
    Years(Vec<i32>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WeatherYearsKeyword {
    All,
    WorstOnRecord,
}

impl From<WeatherYearsRepr> for WeatherYears {
    fn from(repr: WeatherYearsRepr) -> Self {
        match repr {
            WeatherYearsRepr::Keyword(WeatherYearsKeyword::All) => Self::All,
            WeatherYearsRepr::Keyword(WeatherYearsKeyword::WorstOnRecord) => Self::WorstOnRecord,
            WeatherYearsRepr::Years(years) => Self::Years(years),
        }
    }
}

impl From<WeatherYears> for WeatherYearsRepr {
    fn from(value: WeatherYears) -> Self {
        match value {
            WeatherYears::All => Self::Keyword(WeatherYearsKeyword::All),
            WeatherYears::WorstOnRecord => Self::Keyword(WeatherYearsKeyword::WorstOnRecord),
            WeatherYears::Years(years) => Self::Years(years),
        }
    }
}

/// One or more half-hourly Parquet trace files. A list is concatenated
/// in file order at load (each file must continue exactly where the
/// previous one ends) — the mechanism by which multi-year horizons are
/// assembled from per-year trace files (docs/04 Stage 3). In TOML: a
/// plain string or an array of strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TraceFiles {
    /// A single trace file covering the whole horizon.
    One(String),
    /// Consecutive trace files, concatenated in order.
    Many(Vec<String>),
}

impl TraceFiles {
    /// Wraps an explicit path list (empty lists are rejected at load).
    #[must_use]
    pub fn from_paths(paths: Vec<String>) -> Self {
        Self::Many(paths)
    }

    /// The referenced paths, in concatenation order.
    #[must_use]
    pub fn paths(&self) -> &[String] {
        match self {
            Self::One(path) => core::slice::from_ref(path),
            Self::Many(paths) => paths,
        }
    }
}

impl core::fmt::Display for TraceFiles {
    /// A display form for messages: the single path, or the list.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::One(path) => f.pad(path),
            Self::Many(paths) => f.pad(&format!("[{}]", paths.join(", "))),
        }
    }
}

impl<S: Into<String>> From<S> for TraceFiles {
    fn from(path: S) -> Self {
        Self::One(path.into())
    }
}

/// One zone: demand model, exogenous supply, generation fleet, storage
/// portfolio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneSpec {
    /// Zone identifier.
    pub id: ZoneId,
    /// Demand model for the zone.
    pub demand: DemandSpec,
    /// Exogenous must-take supply traces (schema v2; formerly the
    /// run-inputs `[[exogenous_supply]]` tables).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exogenous_supply: Vec<ExogenousSupplySpec>,
    /// Generation fleet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fleet: Vec<FleetEntry>,
    /// Storage portfolio, ordered by `dispatch_order` (ADR-8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub storage: Vec<StorageSpec>,
    /// The zone's own SRMC chain for the priced flow signal (schema v7,
    /// D11). Absent for zones that never dispatch under the priced
    /// ladder; required on EVERY zone when
    /// `dispatch.flow_signal = "priced_ladder"` ([`Scenario::validate`];
    /// ADR-7 touch-point: external zones need pricing inputs to be
    /// dispatchable under the ladder).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ZonePricingSpec>,
}

/// A zone's pricing inputs for the priced flow signal (schema v7, D11
/// — the ADR-9 touch-point: per-zone SRMC is new input plumbing over
/// the existing Stage 2 SRMC recipe, which is reused unchanged).
///
/// The carbon basis, in prose: `carbon_flat_gbp_per_tco2` ABSENT means
/// the zone prices carbon at the reference file's UKA auction step
/// series plus the Carbon Price Support (the committed GB basis, so the
/// flow rule and the Stage 2 pricing layer agree — D11 review Edit 4);
/// PRESENT means a flat per-zone carbon level in £/tCO2 replacing that
/// series (the external-zone EUA basis: no licence-clean daily EUA
/// series exists, so the committed convention is a flat 2024 annual
/// mean per zone — data/reference/prices-eu-2024.toml, D11 data
/// package). The granularity asymmetry this creates (a stepped GB
/// series against flat external levels) is a stated property of the
/// committed data, not a modelling claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZonePricingSpec {
    /// Path to the committed prices-reference TOML (schema
    /// `prices-reference-v1`), resolved against the run's base
    /// directory. Supplies efficiencies, emission factors, and (when
    /// the flat carbon field is absent) the UKA+CPS carbon series.
    pub reference: String,
    /// Flat per-zone carbon price, £/tCO2 (see the struct docs for the
    /// two bases). Must be finite and non-negative.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbon_flat_gbp_per_tco2: Option<CarbonPrice>,
    /// Per-fuel fuel-price trace (£/MWh-thermal, HHV), keyed by fuel
    /// name — same shape as the top-level `[pricing]` block.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fuel_price: BTreeMap<String, TraceRefSpec>,
    /// SRMC recipe per technology id of THIS zone's fleet (dispatchable
    /// entries only — must-take technologies carry no SRMC model,
    /// grid-core pricing convention 1). Technologies not listed price
    /// at the £0 must-take floor in the flow signal.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub srmc: BTreeMap<String, SrmcRecipeSpec>,
}

/// Demand model:
/// `demand(t) = (base(t) + Σ extras(t)) × annual_scale + extra_demand_gw
/// + heating(t)` (docs/03; extras = the schema-v4 `extra_profiles` sum).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DemandSpec {
    /// Half-hourly base demand trace(s) (Parquet, MW).
    pub base_profile: TraceFiles,
    /// Column of the base-profile parquet to read (schema v2; the D3
    /// total-generation convention's `underlying_demand` by default —
    /// docs/notes/d3-embedded-convention.md).
    #[serde(default = "default_demand_column")]
    pub column: String,
    /// Additional MW demand traces summed onto the base profile before
    /// `annual_scale` (schema v4): how an aggregate zone's demand
    /// (CONT-NW = BE + NL + DE-LU) is assembled from per-country load
    /// traces. `demand(t) = (base(t) + Σ extras(t)) × annual_scale +
    /// extra_demand_gw + heating(t)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_profiles: Vec<TraceRefSpec>,
    /// Demand growth multiplier applied to the base profile
    /// (dimensionless scale factor, not a physical quantity).
    pub annual_scale: f64,
    /// Constant power added to demand every period, GW (schema v2;
    /// default 0). Carries supply-side load the base profile excludes —
    /// the station-transformer-load wedge correction of the 2024
    /// validation harness. Not subject to `annual_scale`: it is a
    /// harness correction, not consumer demand.
    #[serde(default)]
    pub extra_demand_gw: Power,
    /// The electrified-heating technology portfolio (schema v5, Q5/D9).
    /// Absent ⇒ the engine byte-path is untouched (D9 rule 1: scenarios
    /// without a heating block are bit-identical to pre-v5 runs).
    /// Heating demand ADDS to zone demand before dispatch and is not
    /// subject to `annual_scale` (it carries its own quantum).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heating: Option<HeatingSpec>,
}

fn default_demand_column() -> String {
    "underlying_demand".to_owned()
}

/// The exogenous `scale` default (schema v6): the unsplit series.
fn default_exogenous_scale() -> f64 {
    1.0
}

/// Round-trip helper: omit `scale = 1.0` on serialisation (the field
/// default), keeping pre-v6-shaped files byte-stable.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde's signature
fn is_default_exogenous_scale(scale: &f64) -> bool {
    *scale == 1.0
}

/// One exogenous must-take supply trace (schema v2): the named MW
/// columns of the file(s) are summed per period (negative values =
/// export / pumping load). Used for net imports (modelled from Stage 5),
/// and generation with no fleet representation (FUELHH "other").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExogenousSupplySpec {
    /// Output-series label (`net_imports`, `other`, …).
    pub label: String,
    /// Parquet trace file(s), resolved against the run's base directory.
    pub path: TraceFiles,
    /// MW columns to sum.
    pub columns: Vec<String>,
    /// Whether this series feeds the run's imports accounting.
    #[serde(default)]
    pub imports: bool,
    /// Flat multiplier on the summed MW columns (schema v6; default
    /// 1.0). The mechanism by which a national exogenous series is
    /// split across zones by a cited share (the 2-zone scenario's
    /// pumped-storage-net 0.2617/0.7383 and "other" 0.101/0.899
    /// splits). Dimensionless, like `annual_scale`. Must be finite and
    /// non-negative ([`Scenario::validate`] — sign conventions live in
    /// the trace, never in the multiplier).
    #[serde(
        default = "default_exogenous_scale",
        skip_serializing_if = "is_default_exogenous_scale"
    )]
    pub scale: f64,
    /// Reliability classification (gb-grid-margin methodology; see
    /// [`ExogenousReliability`]). **Required** — a hand-written
    /// exogenous series has no safe default: imports are `variable`
    /// (blocking highs becalm GB and its neighbours together, so
    /// interconnectors fail exactly when needed), pumped-storage net
    /// traces are `excluded` (pumping is demand; PS supply sits in
    /// neither bucket), FUELHH "other" is `firm`.
    pub reliability: ExogenousReliability,
}

/// TOML-facing availability model (schema v2; formerly the run-inputs
/// `[availability.*]` tables): `{ flat = 0.61 }` or
/// `{ monthly = [ … ] }`. Factor ranges and the month count are checked
/// by [`Scenario::validate`] and again on conversion to the engine's
/// availability model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AvailabilitySpec {
    /// One factor for every period.
    Flat {
        /// The flat availability factor.
        flat: PerUnit,
    },
    /// One factor per UTC calendar month, January first.
    Monthly {
        /// The twelve monthly factors.
        monthly: Vec<PerUnit>,
    },
}

/// The temperature-driven heating demand overlay as a **technology
/// portfolio** (schema v5; docs/notes/d9-heating-overlay.md rule 2 —
/// field names normative). Replaces the v1–v4 sketch block.
///
/// The block models the **buildings heat class**: space heating + DHW,
/// domestic + services — the ECUK quantum scope
/// (`data/reference/heating-cop.toml [heat_quantum]`). Industrial /
/// process heat is a named follow-on as a sibling block, never a
/// reinterpretation of these fields (D9 rule 2).
///
/// Two doc-pinned clarifications (D9 adjudication, note of record 2):
/// the engine consumes only the **product**
/// `delivered_heat_twh × electrified_share` (the electrified quantum —
/// two scenarios with equal products are physically identical); and
/// `dhw_fraction` applies uniformly WITHIN the electrified quantum
/// (electrification is assumed proportional across space heat and DHW —
/// a stated assumption).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatingSpec {
    /// The **record-mean** annual delivered-heat quantum (D9 rule 3:
    /// cold years draw more heat than mild years; per-year totals are a
    /// reported output, never renormalised). Written in the file in
    /// **TWh** (`delivered_heat_twh = 410.5`), carried as [`Energy`]
    /// (ADR-4).
    #[serde(with = "energy_twh")]
    pub delivered_heat_twh: Energy,
    /// Fraction of the quantum that is electrified (0–1).
    pub electrified_share: PerUnit,
    /// Temperature-independent hot-water fraction of the (electrified)
    /// quantum, spread flat (0–1; the ECUK-derived default basis lives
    /// in the reference file).
    pub dhw_fraction: PerUnit,
    /// The pinned population-weighted air-temperature trace
    /// (`{ path, column }`, Parquet, °C, whole calendar years — the
    /// rule-3 intensity `k` is computed over this trace's full record).
    pub temperature_trace: TraceRefSpec,
    /// The technology portfolio; shares must sum to 1 within 1e-9
    /// ([`Scenario::validate`]).
    pub entries: Vec<HeatingEntry>,
}

/// TOML representation of `delivered_heat_twh`: the file writes TWh,
/// the domain type is [`Energy`] (GWh canonical). The ×1000 factor is
/// applied here, at the single defined conversion point.
mod energy_twh {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::units::Energy;

    pub fn serialize<S: Serializer>(energy: &Energy, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(energy.as_gigawatt_hours() / 1000.0)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Energy, D::Error> {
        f64::deserialize(deserializer).map(|twh| Energy::gigawatt_hours(twh * 1000.0))
    }
}

/// One heating-portfolio entry (D9 rule 2): a technology kind, its
/// share of the electrified quantum, and optional COP-parameter
/// overrides. Defaults live in the cited, drift-guarded reference file
/// `data/reference/heating-cop.toml` ([`crate::heating`]); overrides
/// are legal (the reliability/inertia overrides precedent) and are
/// always echoed into run outputs so they cannot hide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatingEntry {
    /// Which heating technology this entry models.
    pub kind: HeatingKind,
    /// Share of the electrified heat quantum served by this entry
    /// (0–1; shares sum to 1 across the portfolio).
    pub share: PerUnit,
    /// COP-curve coefficient override `[c0, c1, c2]` for
    /// `COP = c0 + c1·ΔT + c2·ΔT²` (heat-pump kinds only; dimensionless
    /// per Kⁿ curve coefficients, the When2Heat quadratic form).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cop_curve: Option<[f64; 3]>,
    /// Override of the When2Heat field-calibration correction factor
    /// (heat-pump kinds only; reference default 0.85, RETAINED).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_factor: Option<PerUnit>,
    /// Override of the one-factor-per-technology RHPP derating
    /// (heat-pump kinds only; reference defaults 0.823 ASHP /
    /// 0.732 GSHP — D9 rule 4 item iv).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rhpp_derating: Option<PerUnit>,
    /// Override of the district constant effective COP (heat delivered
    /// to buildings ÷ total electrical draw, delivered-heat basis — D9
    /// rule 4; district entries only; dimensionless ratio).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cop_const: Option<f64>,
    /// The D16 geothermal resource depth (schema v8; GSHP entries
    /// only). Written in the file in **metres**
    /// (`resource_depth_m = 150.0`), carried as [`Length`] (ADR-4).
    /// Absent ⇒ the committed shallow-loop behaviour, byte-identical;
    /// the committed 1.0 m datum reproduces it bit-identically (the
    /// D16 rule-4 test-1 invariance).
    #[serde(
        default,
        with = "optional_length_m",
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_depth_m: Option<Length>,
}

/// TOML representation of `resource_depth_m`: the file writes metres,
/// the domain type is [`Length`] (km canonical) — the `energy_twh`
/// single-conversion-point pattern, on an optional field.
mod optional_length_m {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::units::Length;

    pub fn serialize<S: Serializer>(
        length: &Option<Length>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match length {
            // Unreachable under `skip_serializing_if`, kept total.
            None => serializer.serialize_none(),
            Some(length) => serializer.serialize_f64(length.as_metres()),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Length>, D::Error> {
        Option::<f64>::deserialize(deserializer).map(|m| m.map(Length::metres))
    }
}

/// The heating technology kinds of D9 rule 2. A closed set: each kind
/// selects a COP source-temperature model, so unknown kinds are parse
/// errors (`deny_unknown_fields` discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeatingKind {
    /// Air-source heat pump: COP on the air temperature `T_pop(t)`.
    Ashp,
    /// Ground-source heat pump: COP on the damped, phase-lagged annual
    /// ground wave `T_ground(t)` (Kusuda–Achenbach).
    Gshp,
    /// District/deep geothermal: pump load only — a constant effective
    /// COP, temperature-independent by construction.
    DistrictGeothermal,
}

impl HeatingKind {
    /// The TOML spelling (`ashp`, `gshp`, `district_geothermal`) — also
    /// the per-entry output-series label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ashp => "ashp",
            Self::Gshp => "gshp",
            Self::DistrictGeothermal => "district_geothermal",
        }
    }

    /// Whether the kind is a heat pump (carries the curve/correction/
    /// derating parameters) as opposed to district geothermal (carries
    /// only `cop_const`).
    #[must_use]
    pub const fn is_heat_pump(self) -> bool {
        matches!(self, Self::Ashp | Self::Gshp)
    }
}

impl core::fmt::Display for HeatingKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad(self.as_str())
    }
}

/// Binary reliability classification of generation under a correlated
/// synoptic weather event — the owner's gb-grid-margin methodology,
/// implemented exactly as published: **binary, no derating anywhere**;
/// the criterion is correlated failure (a blocking high becalms GB's
/// wind and its neighbours' together).
///
/// Storage is deliberately NOT a value of this enum: the published
/// analysis has no storage representation, so the simulator reports
/// storage discharge as its own fourth output category, never folded
/// into firm — the question "does storage-backed supply count as
/// reliable?" stays visibly open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reliability {
    /// Supply that does not fail with the weather event: gas CCGT/OCGT,
    /// nuclear, biomass, hydro, coal, oil, "other".
    Firm,
    /// The methodology's "weather & imports": wind (on+offshore) and
    /// solar — and, on exogenous entries, interconnector imports, which
    /// fail exactly when a GB-wide lull needs them.
    Variable,
}

impl Reliability {
    /// The TOML spelling (`firm`, `variable`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Firm => "firm",
            Self::Variable => "variable",
        }
    }
}

impl core::fmt::Display for Reliability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad(self.as_str())
    }
}

/// Reliability classification of an exogenous supply series:
/// [`Reliability`] plus the third state the methodology assigns pumped
/// storage — **excluded from both buckets** (its pumping is demand; its
/// generation is neither firm nor weather-driven).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExogenousReliability {
    /// Counted in the firm bucket (e.g. FUELHH "other").
    Firm,
    /// Counted in the variable bucket (e.g. net imports).
    Variable,
    /// Counted in neither bucket (pumped-storage net traces).
    Excluded,
}

impl ExogenousReliability {
    /// The TOML spelling (`firm`, `variable`, `excluded`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Firm => "firm",
            Self::Variable => "variable",
            Self::Excluded => "excluded",
        }
    }
}

impl core::fmt::Display for ExogenousReliability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad(self.as_str())
    }
}

/// One technology block in a zone's fleet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetEntry {
    /// Technology identifier (`ccgt`, `offshore_wind`, …).
    pub technology: TechId,
    /// Installed capacity, GW.
    pub capacity_gw: Power,
    /// Half-hourly capacity-factor trace file(s) (weather-driven
    /// technologies only; column `cf`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_factor_trace: Option<TraceFiles>,
    /// Availability model (schema v2; dispatchable technologies only).
    /// Absent means flat 1.0 — the plant runs to nameplate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<AvailabilitySpec>,
    /// Reliability classification override. Absent means the derived
    /// default ([`FleetEntry::derived_reliability`]), which matches the
    /// published gb-grid-margin roster for the standard technology set.
    /// Overrides are legal (the classification is a contestable
    /// modelling assertion, made visible) but are always emitted into
    /// run outputs so they cannot hide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reliability: Option<Reliability>,
    /// Inertia constant H override, seconds (= GVA·s per GVA of machine
    /// rating, quoted on the machine **MVA** base — schema v3, Stage 6,
    /// ADR-9). Absent means the derived per-technology default of
    /// `grid_core::inertia` (transcribed from
    /// `data/reference/inertia-constants.toml`); `None` effective for
    /// non-synchronous plant. Same derived-default-plus-surfaced-
    /// override pattern as `reliability`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inertia_h: Option<InertiaConstant>,
    /// Synchronous-coupling override (schema v3, Stage 6). Absent means
    /// the derived per-technology default. An entry that is effectively
    /// synchronous must have an effective `inertia_h`
    /// ([`Scenario::validate`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synchronous: Option<bool>,
    /// Windowed energy-release constraint (schema v4, Stage 5): the
    /// seasonal-budget model for budget-limited dispatchables (NO2
    /// reservoir hydro, D5). Dispatchable entries only — a
    /// weather-driven (CF-trace) entry is must-take and cannot carry
    /// one ([`Scenario::validate`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_budget: Option<EnergyBudgetSpec>,
}

/// A windowed energy-release constraint on a dispatchable fleet entry
/// (schema v4, Stage 5 — the seasonal-budget reservoir hydro model).
///
/// The named MW columns of the trace file(s) are summed per consecutive
/// window of `window_periods` half-hourly periods, counted from the
/// horizon start; each window's energy is released as dispatch allowance
/// at the window's first period. Unused allowance carries forward across
/// windows (water stays in the reservoir); the entry's per-period output
/// is capped at `min(capacity × availability, remaining allowance / Δt)`.
///
/// This is a budget-grade constraint, **not** a reservoir optimisation:
/// the evidence behind it (ENTSO-E A72 weekly reservoir filling and the
/// stated inflow proxy) supports a weekly energy grain and nothing finer
/// (docs/notes/entsoe-stage5-pack-report.md §6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnergyBudgetSpec {
    /// Parquet trace file(s) with the budget-defining MW columns,
    /// resolved against the run's base directory. Must cover the
    /// horizon exactly, like every other trace.
    pub trace: TraceFiles,
    /// MW columns summed to form the budget series.
    pub columns: Vec<String>,
    /// Window length in half-hourly periods (default
    /// [`DEFAULT_BUDGET_WINDOW_PERIODS`] = one week).
    #[serde(default = "default_budget_window_periods")]
    pub window_periods: usize,
}

fn default_budget_window_periods() -> usize {
    DEFAULT_BUDGET_WINDOW_PERIODS
}

impl FleetEntry {
    /// The derived reliability default: weather-driven (has a
    /// capacity-factor trace) ⇒ variable; dispatchable ⇒ firm. This
    /// reproduces the published gb-grid-margin roster exactly for the
    /// standard technology set (firm: ccgt, ocgt, nuclear, biomass,
    /// hydro, coal, oil, other; variable: onshore/offshore wind, solar).
    #[must_use]
    pub fn derived_reliability(&self) -> Reliability {
        if self.capacity_factor_trace.is_some() {
            Reliability::Variable
        } else {
            Reliability::Firm
        }
    }

    /// The effective classification: the explicit override when
    /// present, else the derived default.
    #[must_use]
    pub fn effective_reliability(&self) -> Reliability {
        self.reliability
            .unwrap_or_else(|| self.derived_reliability())
    }

    /// Whether the explicit field overrides the derived default (an
    /// explicit field equal to the default is a restatement, not an
    /// override of the methodology).
    #[must_use]
    pub fn reliability_overridden(&self) -> bool {
        self.reliability
            .is_some_and(|explicit| explicit != self.derived_reliability())
    }

    // -----------------------------------------------------------------
    // Stability metadata (schema v3, Stage 6, ADR-9). Derived defaults
    // live in `grid_core::inertia` (transcribed from the committed
    // evidence file); explicit fields override and are surfaced.
    // -----------------------------------------------------------------

    /// The effective synchronous flag: the explicit override when
    /// present, else the derived per-technology default.
    #[must_use]
    pub fn effective_synchronous(&self) -> bool {
        self.synchronous
            .unwrap_or_else(|| crate::inertia::technology_default(&self.technology).synchronous)
    }

    /// The effective inertia constant: the explicit override when
    /// present, else the derived per-technology default. `None` for an
    /// effectively non-synchronous entry (which stores no rotating
    /// kinetic energy behind a coupling this model sees).
    #[must_use]
    pub fn effective_inertia_h(&self) -> Option<InertiaConstant> {
        if !self.effective_synchronous() {
            return None;
        }
        self.inertia_h
            .or_else(|| crate::inertia::technology_default(&self.technology).h)
    }

    /// Whether the explicit `synchronous` field overrides the derived
    /// default (a restatement is not an override).
    #[must_use]
    pub fn synchronous_overridden(&self) -> bool {
        self.synchronous.is_some_and(|explicit| {
            explicit != crate::inertia::technology_default(&self.technology).synchronous
        })
    }

    /// Whether the explicit `inertia_h` field overrides the derived
    /// default (a restatement is not an override).
    #[must_use]
    pub fn inertia_overridden(&self) -> bool {
        self.inertia_h.is_some_and(|explicit| {
            crate::inertia::technology_default(&self.technology).h != Some(explicit)
        })
    }
}

/// One store in a zone's storage portfolio (ADR-8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageSpec {
    /// Storage kind.
    pub kind: StorageKind,
    /// Symmetric charge/discharge power limit, GW (v1).
    pub power_gw: Power,
    /// Usable energy capacity, GWh.
    pub energy_gwh: Energy,
    /// Round-trip efficiency (0–1); split symmetrically as
    /// `η_charge = η_discharge = √η` by the engine (D4, ADR-8 v1).
    pub round_trip_efficiency: PerUnit,
    /// Position in the rule-based charge/discharge order (1 = first);
    /// unique within a zone ([`Scenario::validate`], D4 rule 2).
    pub dispatch_order: u8,
    /// Initial state of charge as a fraction of `energy_gwh` (schema
    /// v2). Default when absent: **full** (1.0), per D4 — bisection
    /// results apply D4's initial-SoC guard against the transient this
    /// choice creates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_soc: Option<PerUnit>,
    /// DSR only (schema v2, shape only until Q6): how long deferred load
    /// may be shifted, hours.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift_duration: Option<Duration>,
    /// DSR only (schema v2, shape only until Q6): maximum deferred
    /// energy per UTC calendar day, GWh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_volume_limit: Option<Energy>,
}

/// The storage kinds of ADR-8. A closed set: each kind carries modelling
/// semantics (DSR is pseudo-storage), so unknown kinds are parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    /// Grid-scale battery.
    Battery,
    /// Pumped-storage hydro.
    PumpedHydro,
    /// Hydrogen (electrolysis → storage → reconversion; η ≈ 0.35–0.40).
    Hydrogen,
    /// Demand-side response modelled as pseudo-storage.
    Dsr,
}

impl StorageKind {
    /// The TOML spelling of the kind (`pumped_hydro`, …).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::PumpedHydro => "pumped_hydro",
            Self::Hydrogen => "hydrogen",
            Self::Dsr => "dsr",
        }
    }
}

impl core::fmt::Display for StorageKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad(self.as_str())
    }
}

/// One interconnector in the link matrix (ADR-7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSpec {
    /// Optional per-link identity (schema v4) for outputs and per-border
    /// validation — two links may join the same zone pair (Nemo and
    /// BritNed both land in CONT-NW). Absent: the engine derives
    /// `<from>-<to>-<index>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Home zone.
    pub from: ZoneId,
    /// Counterparty zone (may be external, without a `[[zones]]` entry,
    /// while a single-zone scenario carries imports as an exogenous
    /// trace; in a multi-zone scenario both endpoints must be declared
    /// zones — [`Scenario::validate`]).
    pub to: ZoneId,
    /// Nameplate capacity, GW (a cap on sending-end power). Without the
    /// v6 fields below this bounds BOTH directions (the pre-v6
    /// symmetric semantics); with them it is the **forward**
    /// (`from → to`) capability, superseded per period by
    /// `capability_trace` when present.
    pub capacity_gw: Power,
    /// Capability of the **reverse** (`to → from`) direction, GW
    /// (schema v6). Absent ⇒ symmetric at `capacity_gw`. The B6 ruling
    /// needs export 4.1 / import 3.5 GW asymmetry
    /// (docs/notes/b6-two-zone-data-review.md §6a).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse_capacity_gw: Option<Power>,
    /// Per-period **forward** (`from → to`) capability trace (schema
    /// v6; [`LinkCapabilityTraceSpec`]) — the observed half-hourly DA
    /// limit series of the B6 2024 validation configuration. Supersedes
    /// `capacity_gw` on horizon periods; `availability` still
    /// multiplies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_trace: Option<LinkCapabilityTraceSpec>,
    /// Availability derating (0–1), applied to capacity every period —
    /// a deterministic derate, not a stochastic outage model (ADR-5).
    pub availability: PerUnit,
    /// Transmission loss fraction (schema v4; default 0): the receiving
    /// end gets `sent × (1 − loss)`. This is the HVDC loss wedge between
    /// sending-end (ENTSO-E) and receiving-end (NESO) metering —
    /// docs/notes/entsoe-stage5-pack-report.md §3. Must be in [0, 1).
    #[serde(default)]
    pub loss: PerUnit,
}

/// TOML representation of an MW power field carried as [`Power`] (GW
/// canonical): the file writes MW, the ×1000 factor is applied here, at
/// the single defined conversion point (the `energy_twh` precedent).
mod power_mw {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::units::Power;

    pub fn serialize<S: Serializer>(power: &Power, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(power.as_gigawatts() * 1000.0)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Power, D::Error> {
        f64::deserialize(deserializer).map(Power::megawatts)
    }
}

/// A per-period forward link-capability trace (schema v6): the observed
/// half-hourly boundary limit series, with the sentinel handling of the
/// B6 link-convention ruling (docs/notes/b6-two-zone-data-review.md
/// §6a) stated IN the scenario — never a silent loader default.
///
/// Semantics (implemented by the grid-adequacy input loader):
///
/// - the referenced file is a **sparse** utc-indexed MW series (rows
///   may be missing; values may be NaN) — unlike every other trace it
///   is not required to cover the horizon;
/// - a value ≥ `sentinel_high_mw` is a "no constraint recorded"
///   sentinel → the period's capability is `upper_bound_gw` (the pinned
///   planning upper bound; ETYS 6.7 GW for B6) and the period stays IN
///   gate arithmetic;
/// - a value of exactly 0, a NaN value, or a missing row → the period's
///   capability is treated as UNOBSERVED: it is **masked out of
///   validation-gate arithmetic** and filled with `masked_fill_gw` (the
///   pinned central value; the 2024 median 4.1 GW for B6) so dispatch,
///   which must run every period, has a stated capability;
/// - negative values are structured errors (a capability cannot be
///   negative).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkCapabilityTraceSpec {
    /// Parquet trace file (sparse; `utc_start` index), resolved against
    /// the run's base directory.
    pub path: String,
    /// MW value column to read.
    pub column: String,
    /// Values ≥ this are "no constraint recorded" sentinels. Written
    /// in the file in **MW** (`sentinel_high_mw = 9999.0`), carried as
    /// [`Power`] (ADR-4: no raw `f64` physical quantity crosses this
    /// public API — the ×1000 lives at the single [`power_mw`]
    /// conversion point, the `energy_twh` precedent).
    #[serde(with = "power_mw")]
    pub sentinel_high_mw: Power,
    /// Replacement capability for high sentinels, GW (the pinned
    /// planning upper bound — cite it in the scenario).
    pub upper_bound_gw: Power,
    /// Fill capability for masked periods (zero sentinels, NaN rows,
    /// missing rows), GW — masked periods are excluded from
    /// validation-gate arithmetic (the ruling: missing stays missing).
    pub masked_fill_gw: Power,
}

/// Dispatch policy selection (ADR-6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    /// Which storage dispatch policy the run uses.
    pub policy: DispatchPolicyKind,
    /// Which flow-rule signal a multi-zone run equalises (schema v7,
    /// D11). Default `scarcity` — every pre-v7 scenario keeps its
    /// exact behaviour; omitted from serialisation at the default.
    #[serde(default, skip_serializing_if = "FlowSignal::is_default")]
    pub flow_signal: FlowSignal,
}

/// The flow-rule signal of a multi-zone run (schema v7, D11 — the
/// ADR-6 policy set generalised to
/// {scarcity-rule, priced-ladder, perfect-foresight-LP}).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowSignal {
    /// The Stage 5 scarcity score (price-blind; the committed
    /// validated behaviour and the default).
    #[default]
    Scarcity,
    /// The D11 lexicographic signal: (per-zone marginal SRMC primary,
    /// the full Stage 5 scarcity score secondary — ladder index +
    /// fractional utilisation, with the −surplus and 6+unserved
    /// regions). Requires `[zones.pricing]` on every zone
    /// ([`Scenario::validate`]).
    PricedLadder,
}

impl FlowSignal {
    /// The TOML spelling (`scarcity`, `priced_ladder`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scarcity => "scarcity",
            Self::PricedLadder => "priced_ladder",
        }
    }

    /// Whether this is the default (serialisation-skip helper).
    #[allow(clippy::trivially_copy_pass_by_ref, reason = "serde's signature")]
    fn is_default(&self) -> bool {
        *self == Self::Scarcity
    }
}

impl core::fmt::Display for FlowSignal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad(self.as_str())
    }
}

/// The pluggable storage dispatch policies (ADR-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchPolicyKind {
    /// Greedy/heuristic, no foresight — the default and the more
    /// defensible storage-requirement estimate.
    RuleBased,
    /// LP over the horizon (HiGHS via `good_lp`, Stage 7). Declared but
    /// not routed: the engines reject this enum value; the
    /// perfect-foresight LP runs via `grid_adequacy::run_multi_lp`
    /// (D12 — deliberately a whole-horizon function, not a policy).
    PerfectForesight,
}

impl DispatchPolicyKind {
    /// The TOML spelling of the policy (`rule_based`, `perfect_foresight`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuleBased => "rule_based",
            Self::PerfectForesight => "perfect_foresight",
        }
    }
}

impl core::fmt::Display for DispatchPolicyKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.pad(self.as_str())
    }
}

/// Transmission-constraint cost approximation (ADR-12). Fields may be
/// null in early stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraints {
    /// Constraint-cost model keyed to Scottish wind output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b6_cost_model: Option<String>,
}

/// Optional solver mode (ADR-10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Solver {
    /// Solver mode name, e.g. `min_storage_for_zero_unserved`.
    pub mode: String,
}

/// The `[pricing]` block (schema v2; formerly the run-inputs `[pricing]`
/// section): everything the Stage 2 pricing layer needs beyond the
/// dispatch inputs. Numbers themselves (efficiencies, emission factors,
/// UKA auctions, CPS) live in the committed prices-reference file (every
/// value cited) — this block only *names* them, so the single committed
/// source cannot drift from a second transcription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PricingSpec {
    /// Path to the committed prices-reference TOML (schema
    /// `prices-reference-v1`), resolved against the run's base directory.
    pub reference: String,
    /// Per-fuel per-period fuel-price trace (£/MWh-thermal, HHV), keyed
    /// by fuel name (`gas` is the only fuel prices-reference-v1 carries
    /// factors for).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fuel_price: BTreeMap<String, TraceRefSpec>,
    /// SRMC recipe per technology id: which fuel trace and which
    /// reference-file efficiency key price it. Technologies not listed
    /// carry no SRMC model and never set the price (must-run/calibrated
    /// plant — the grid-core pricing conventions).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub srmc: BTreeMap<String, SrmcRecipeSpec>,
    /// Observed market price trace (£/MWh) for the realism statistics of
    /// docs/04 Stage 2; optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_price: Option<TraceRefSpec>,
}

/// A `{ path, column }` reference to one column of a half-hourly trace
/// parquet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceRefSpec {
    /// Parquet trace file, resolved against the run's base directory.
    pub path: String,
    /// Value column to read.
    pub column: String,
}

/// One technology's SRMC recipe (see [`PricingSpec::srmc`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SrmcRecipeSpec {
    /// Fuel name, keying both [`PricingSpec::fuel_price`] and the
    /// reference file's emission factors.
    pub fuel: String,
    /// Efficiency key into the reference file's `[efficiency.*]` tables
    /// (HHV basis).
    pub efficiency: String,
}
