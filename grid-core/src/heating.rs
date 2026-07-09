//! The Q5/Q11 electrified-heating overlay (docs/notes/d9-heating-overlay.md,
//! ADOPTED 2026-07-03 — rules 1–4 implemented here exactly; the schema
//! block is [`crate::scenario::HeatingSpec`]).
//!
//! # What the overlay is (D9 rule 1)
//!
//! A demand-side transformation, not a supply technology: it ADDS
//! electrified-heating demand to the electricity demand trace,
//! half-hourly, as a deterministic function of (a) a delivered-heat
//! quantum, (b) a portfolio of heating technologies, and (c) the
//! population-weighted GB air temperature trace. It changes nothing
//! else: no dispatch rules, no pricing conventions, no storage
//! mechanics. Scenarios without a heating block are bit-identical to
//! pre-v5 runs.
//!
//! # Heat demand shape (D9 rule 3 — the conventions, in prose)
//!
//! - `heat_need(t) = max(T_base − T_pop(t), 0)` with `T_base = 15.5 °C`
//!   (the UK degree-day convention, cited in the reference file).
//! - Space-heat energy is scaled by a **single pinned intensity
//!   coefficient**, never per-year renormalisation:
//!   `k = electrified space-heat quantum ÷ mean annual degree-hours`
//!   over the temperature trace's full record (the pinned 1985–2024
//!   window in the reference scenarios), computed once and echoed into
//!   run outputs. `delivered_heat_twh` is therefore the RECORD-MEAN
//!   annual quantum: cold years draw more heat than mild years — the
//!   inter-annual physics the 40-year storage question measures, and
//!   half of the cold-year covariance (more heat AND worse COP in the
//!   same hours).
//! - A **hot-water floor**: `dhw_fraction` of the electrified quantum
//!   is temperature-independent, spread flat at
//!   `DHW quantum ÷ mean-year hours`.
//! - `heat(t) = k · heat_need(t) + DHW rate` is a pure function of
//!   `T_pop(t)`: horizon subsetting never changes it (ADR-5
//!   composability — the constants come from the trace's full record,
//!   not from the run horizon).
//! - **No within-day behavioural profile in v1** (understates the
//!   heating peak AND the portfolio deltas in the same direction — the
//!   measured network value of geothermal is a lower bound; stated in
//!   every artefact quoting the rule-6 gradients).
//!
//! # COP models (D9 rule 4 — the source temperature is the point)
//!
//! Electrical demand per entry:
//! `P_elec(t) = share × [space(t)/COP_space(t) + DHW rate/COP_dhw(t)]`
//! (the space sink is weather-compensated, the DHW sink constant, so
//! the two components carry their own COPs — exactly the split the
//! reviewed SPFH2 determination uses).
//!
//! - **ASHP**: `T_source = T_pop(t)` (air). COP =
//!   `rhpp_derating × correction_factor × (c0 + c1·ΔT + c2·ΔT²)` with
//!   `ΔT = max(T_sink − T_source, 15 K)` — the When2Heat quadratic
//!   (correction 0.85 RETAINED) with the ONE per-technology RHPP
//!   to-median derating (0.823), from the drift-guarded reference file.
//!   `T_sink = 40 − 1.0·T_pop(t)` (weather-compensated radiator
//!   convention); DHW sink 50 °C.
//! - **GSHP**: same curve family (10.29/−0.21/0.0012, derating 0.732),
//!   but `T_source = T_ground(t) − 5 K` (brine offset), with
//!   `T_ground(t)` the **damped, phase-lagged annual wave** of the
//!   fitted single harmonic of `T_pop`:
//!   `damping = exp(−z√(ω/2α))`, `lag = z/√(2αω)` (Kusuda–Achenbach) at
//!   the shallow-horizontal loop depth z = 1.0 m (the conservative
//!   case) with cited GB soil diffusivity α. The single-harmonic fit is
//!   the physics, not an approximation of convenience: damping depth
//!   scales with √period, so the ground extinguishes the diurnal and
//!   synoptic harmonics the fit discards. The point this model carries:
//!   the GSHP source barely feels the cold snap that crushes ASHP COP.
//! - **GSHP depth continuum (D16, schema v8;
//!   docs/notes/d16-geothermal-source-temperature.md)**: an optional
//!   per-entry `resource_depth_m` re-anchors the SAME fitted wave at a
//!   geothermal resource depth — damping/lag recomputed at z, source
//!   mean warmed by `gradient × (z − loop_depth)` (the loop-depth
//!   datum makes the shallow default bit-identical: the rule-4 test-1
//!   safety pin; gradient from `[geothermal]`, heating-cop-v2, BGS-
//!   cited, 25 °C/km centre / 26–35 band). The **direct-use handoff**:
//!   when the brine-offset source meets a component's sink, that
//!   component passes through at the district `cop_const`; below it
//!   the curve COP is capped AT `cop_const` ("capped at, then handed
//!   to" — no COP → ∞, district-lowest can tie but never invert). Each
//!   half-hour and component crosses at its own depth, so the depth
//!   continuum is smooth in aggregate; the per-period floor→pass-through
//!   step is the stated consequence of keeping the calibrated curve
//!   and its ΔT floor frozen. Field absent ⇒ the committed pre-v8
//!   path, byte for byte.
//! - **District/deep geothermal**: pump load only —
//!   `P_elec = share × heat(t) / cop_const`, temperature-independent,
//!   `cop_const` on the delivered-heat basis. Its premise (`cop_const`
//!   exceeds the heat pumps' maximum record COP — the district-lowest
//!   ordering limb) is machine-checked at computation time, not
//!   assumed.
//!
//! Default parameters live in the cited, drift-guarded reference file
//! [`HEATING_COP_REFERENCE_PATH`] (`data/reference/heating-cop.toml`,
//! the inertia-constants precedent) — never hard-coded, never free
//! scenario text. Per-entry scenario overrides are legal and are
//! echoed into run outputs together with the pinned constants
//! (k, DHW rate, damping, lag, deratings, cop_const — D9 rule 6b).

use std::path::Path;

use serde::Deserialize;

use crate::GridError;
use crate::scenario::{HeatingEntry, HeatingKind, HeatingSpec};
use crate::time::UtcInstant;
use crate::trace::Trace;
use crate::units::{
    DegreeHours, Diffusivity, Duration, Energy, HeatIntensity, Length, PerUnit, Power, Temperature,
    TemperatureGradient,
};

/// Repo-relative path of the drift-guarded COP-parameter reference
/// file (D9 rule 4). Resolved against the run's base directory by the
/// input loader — the scenario schema deliberately carries no path for
/// it. Path-as-constant rationale (accepted with the precedent record
/// corrected, q5-heating-engine-review.md ruling 3): D9 rule 2's
/// normative field list carries no COP path and rule 4 orders
/// "reference file, NOT hard-coded, NOT free scenario text"; the file
/// is a COMMITTED engine input inside the engine git hash, so the
/// ADR-5 determinism formula holds, and its sha256 rides in every run
/// output's data-file metadata. This is deliberately NOT the
/// prices-2024.toml pattern (a scenario field,
/// `PricingSpec.reference`) nor the inertia-constants pattern (values
/// transcribed into code, file as evidence): per-entry scenario
/// overrides are the scenario-side control here, always echoed.
pub const HEATING_COP_REFERENCE_PATH: &str = "data/reference/heating-cop.toml";

/// The mandatory reference-file schema string (the docs/03
/// committed-reference registry discipline; probed before the full
/// parse so shape changes fail with a clear message, never
/// field-level errors). v2 = v1 plus the D16 `[geothermal]` section
/// (docs/notes/d16-geothermal-source-temperature.md); every v1 value
/// is untouched.
pub const HEATING_COP_SCHEMA: &str = "heating-cop-v2";

// ---------------------------------------------------------------------
// The reference file: raw TOML mirror → validated domain form.
// ---------------------------------------------------------------------

/// The validated contents of `data/reference/heating-cop.toml`.
/// Field-for-field a transcription of the reviewed data package
/// (docs/notes/q5-heating-data-report.md); the parse pins in
/// `grid-core/tests/heating.rs` guard it against drift.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingCopReference {
    /// UK degree-day base temperature (15.5 °C, DESNZ ET 7.1).
    pub t_base: Temperature,
    /// The When2Heat ΔT floor (15 K, "in line with the manufacturer
    /// data") — a temperature DIFFERENCE.
    pub min_delta_t: Temperature,
    /// Default record-mean GB delivered-heat quantum (410.5 TWh).
    pub delivered_heat: Energy,
    /// Default DHW fraction on the same basis (0.170).
    pub dhw_fraction: PerUnit,
    /// Weather-compensated sink parameters.
    pub sink: SinkParams,
    /// ASHP curve set (air source).
    pub ashp: HeatPumpParams,
    /// GSHP curve set (ground source, brine offset).
    pub gshp: HeatPumpParams,
    /// Kusuda–Achenbach ground-model parameters.
    pub ground: GroundModelParams,
    /// District/deep-geothermal constant effective COP.
    pub district: DistrictParams,
    /// The D16 geothermal gradient (depth-continuum source model).
    pub geothermal: GeothermalParams,
    /// The RHPP field-trial SPF bands (the edit-6 cross-check
    /// reference; SPFs are dimensionless).
    pub rhpp: RhppBands,
}

/// Weather-compensated heating-curve parameters (When2Heat eq. 6).
#[derive(Debug, Clone, PartialEq)]
pub struct SinkParams {
    /// Radiator flow temperature at 0 °C ambient (40 °C).
    pub radiator_t0: Temperature,
    /// Radiator compensation slope (dimensionless °C per °C; 1.0).
    pub radiator_slope: f64,
    /// Floor-heating intercept (transcribed; unused in v1).
    pub floor_t0: Temperature,
    /// Floor-heating slope (transcribed; unused in v1).
    pub floor_slope: f64,
    /// Constant DHW sink (50 °C).
    pub dhw_sink: Temperature,
}

/// One heat-pump technology's COP parameterisation.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatPumpParams {
    /// When2Heat quadratic `[c0, c1, c2]` in ΔT (dimensionless per Kⁿ).
    pub cop_curve: [f64; 3],
    /// When2Heat field-calibration correction factor (0.85).
    pub correction_factor: PerUnit,
    /// Its D9 edit-6(iii) status (`retained` — the RHPP derating never
    /// stacks on a replaced factor).
    pub correction_factor_status: String,
    /// The ONE per-technology RHPP to-median derating (edit 6(iv)).
    pub rhpp_derating: PerUnit,
    /// Source-side offset subtracted from the source temperature
    /// (GSHP brine heat-exchanger, 5 K; 0 for ASHP) — a DIFFERENCE.
    pub source_offset: Temperature,
}

/// Kusuda–Achenbach undisturbed-ground-wave parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundModelParams {
    /// Nominal loop depth z (1.0 m, shallow horizontal — conservative).
    pub loop_depth: Length,
    /// The stated loop-depth band (1.0–1.2 m).
    pub loop_depth_band: [Length; 2],
    /// GB soil thermal diffusivity α, cited centre.
    pub alpha: Diffusivity,
    /// The cited α band (texture-class medians).
    pub alpha_band: [Diffusivity; 2],
    /// The full site range, recorded for context.
    pub alpha_site_range: [Diffusivity; 2],
}

/// District/deep-geothermal effective-COP parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct DistrictParams {
    /// Constant effective COP, delivered-heat basis (15.0).
    pub cop_const: f64,
    /// The stated band ([12.0, 18.8]).
    pub cop_const_band: [f64; 2],
    /// The basis statement (must name delivered-heat — D9 edit 7).
    pub basis: String,
}

/// D16 geothermal-gradient parameters (`[geothermal]`, heating-cop-v2):
/// the depth-continuum source model
/// `T_source_mean(z) = T_surface_mean + gradient × (z − loop_depth)`,
/// anchored at the committed shallow datum so `resource_depth_m` at the
/// datum is bit-identical to the committed D9 behaviour (the rule-4
/// test-1 invariance).
#[derive(Debug, Clone, PartialEq)]
pub struct GeothermalParams {
    /// The gradient centre (25 °C/km — the industry correspondent's conservative case,
    /// below the BGS band; margin direction stated in the file).
    pub gradient: TemperatureGradient,
    /// The stated BGS band ([26, 35] °C/km — Busby 2014 / Busby &
    /// Terrington 2017).
    pub gradient_band: [TemperatureGradient; 2],
    /// The datum statement (must name the loop-depth anchoring — the
    /// district `basis` precedent: conventions are machine-checked
    /// statements, not comments).
    pub datum: String,
}

/// RHPP field-trial SPF bands (Lowe et al. 2017, Table 3-2, cropped
/// B2), boundary named per number (SEPEMO).
#[derive(Debug, Clone, PartialEq)]
pub struct RhppBands {
    /// The cross-check boundary (`SPFH2`).
    pub comparison_boundary: String,
    /// ASHP SPFH2 median (2.65).
    pub ashp_spfh2_median: f64,
    /// ASHP SPFH2 IQR.
    pub ashp_spfh2_iqr: [f64; 2],
    /// GSHP SPFH2 median (2.81).
    pub gshp_spfh2_median: f64,
    /// GSHP SPFH2 IQR.
    pub gshp_spfh2_iqr: [f64; 2],
    /// ASHP SPFH4 median (transcribed for the record).
    pub ashp_spfh4_median: f64,
    /// ASHP SPFH4 IQR.
    pub ashp_spfh4_iqr: [f64; 2],
    /// GSHP SPFH4 median.
    pub gshp_spfh4_median: f64,
    /// GSHP SPFH4 IQR.
    pub gshp_spfh4_iqr: [f64; 2],
}

// Raw serde mirror of the file layout (strict: unknown fields are
// rejected — the drift-guarded reference-file discipline). Values are
// raw f64 HERE ONLY; the validated form above is newtyped (ADR-4).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReference {
    /// The mandatory schema string ([`HEATING_COP_SCHEMA`]); probed
    /// leniently before this strict parse.
    #[allow(dead_code)]
    schema: String,
    #[allow(dead_code)]
    meta: RawMeta,
    conventions: RawConventions,
    heat_quantum: RawHeatQuantum,
    sink: RawSink,
    ashp: RawHeatPump,
    gshp: RawHeatPump,
    ground_model: RawGroundModel,
    district_geothermal: RawDistrict,
    geothermal: RawGeothermal,
    rhpp: RawRhpp,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMeta {
    #[allow(dead_code)]
    status: String,
    #[allow(dead_code)]
    assembled: String,
    #[allow(dead_code)]
    evidence_note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConventions {
    t_base_c: f64,
    min_delta_t_k: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHeatQuantum {
    delivered_heat_twh: f64,
    #[allow(dead_code)]
    basis: String,
    dhw_fraction: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSink {
    radiator_t0_c: f64,
    radiator_slope: f64,
    floor_t0_c: f64,
    floor_slope: f64,
    dhw_sink_c: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHeatPump {
    cop_curve: [f64; 3],
    source: String,
    #[serde(default)]
    source_offset_k: f64,
    correction_factor: f64,
    correction_factor_status: String,
    rhpp_derating: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGroundModel {
    loop_depth_m: f64,
    loop_depth_band_m: [f64; 2],
    alpha_m2_s: f64,
    alpha_band_m2_s: [f64; 2],
    alpha_site_range_m2_s: [f64; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDistrict {
    cop_const: f64,
    cop_const_band: [f64; 2],
    basis: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeothermal {
    gradient_c_per_km: f64,
    gradient_band_c_per_km: [f64; 2],
    datum: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRhpp {
    comparison_boundary: String,
    ashp_spfh2_median: f64,
    ashp_spfh2_iqr: [f64; 2],
    gshp_spfh2_median: f64,
    gshp_spfh2_iqr: [f64; 2],
    ashp_spfh4_median: f64,
    ashp_spfh4_iqr: [f64; 2],
    gshp_spfh4_median: f64,
    gshp_spfh4_iqr: [f64; 2],
}

impl HeatingCopReference {
    /// Parse and validate the reference file's TOML text.
    ///
    /// Errors: [`GridError::HeatingReferenceParse`] (with line/column
    /// context; unknown fields rejected) and
    /// [`GridError::InvalidHeatingReference`] for semantic problems.
    pub fn from_toml_str(text: &str) -> Result<Self, GridError> {
        // Schema string first, leniently (the scenario-version-probe
        // pattern): a missing or wrong schema is reported as such —
        // a clear migration/wrong-file message, never a field error.
        #[derive(Deserialize)]
        struct SchemaProbe {
            schema: Option<String>,
        }
        let probe: SchemaProbe =
            toml::from_str(text).map_err(|source| GridError::HeatingReferenceParse {
                source: Box::new(source),
            })?;
        match probe.schema.as_deref() {
            None => {
                return Err(GridError::InvalidHeatingReference {
                    reason: format!(
                        "missing mandatory `schema` string (this engine reads schema =                          {HEATING_COP_SCHEMA:?}; the docs/03 committed-reference registry)"
                    ),
                });
            }
            Some(found) if found != HEATING_COP_SCHEMA => {
                return Err(GridError::InvalidHeatingReference {
                    reason: format!(
                        "schema {found:?} is not the {HEATING_COP_SCHEMA:?} this engine                          reads — a shape change to the reference file requires a schema                          bump and a docs/03 registry note"
                    ),
                });
            }
            Some(_) => {}
        }
        let raw: RawReference =
            toml::from_str(text).map_err(|source| GridError::HeatingReferenceParse {
                source: Box::new(source),
            })?;
        Self::validate(raw)
    }

    /// Read, parse and validate the reference file, attaching the path
    /// to any error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InHeatingReferenceFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    fn validate(raw: RawReference) -> Result<Self, GridError> {
        let invalid = |reason: String| GridError::InvalidHeatingReference { reason };
        let fraction = |value: f64, what: &str| -> Result<(), GridError> {
            if !(0.0..=1.0).contains(&value) || value.is_nan() {
                return Err(invalid(format!("{what} {value} is outside [0, 1]")));
            }
            Ok(())
        };
        let positive = |value: f64, what: &str| -> Result<(), GridError> {
            if !value.is_finite() || value <= 0.0 {
                return Err(invalid(format!(
                    "{what} {value} must be positive and finite"
                )));
            }
            Ok(())
        };
        let band = |values: [f64; 2], what: &str| -> Result<(), GridError> {
            if !(values[0].is_finite() && values[1].is_finite()) || values[0] > values[1] {
                return Err(invalid(format!(
                    "{what} band {values:?} is not ordered lo ≤ hi"
                )));
            }
            Ok(())
        };

        if raw.conventions.min_delta_t_k < 0.0 || !raw.conventions.min_delta_t_k.is_finite() {
            return Err(invalid(format!(
                "min_delta_t_k {} must be a non-negative ΔT floor",
                raw.conventions.min_delta_t_k
            )));
        }
        if !raw.conventions.t_base_c.is_finite() {
            return Err(invalid("t_base_c must be finite".to_owned()));
        }
        positive(raw.heat_quantum.delivered_heat_twh, "delivered_heat_twh")?;
        fraction(raw.heat_quantum.dhw_fraction, "dhw_fraction")?;

        let heat_pump = |raw_hp: &RawHeatPump,
                         label: &str,
                         expected_source: &str|
         -> Result<HeatPumpParams, GridError> {
            if raw_hp.source != expected_source {
                return Err(invalid(format!(
                    "[{label}] source is {:?}; the {label} block must be the \
                     {expected_source}-source parameterisation",
                    raw_hp.source
                )));
            }
            if !matches!(
                raw_hp.correction_factor_status.as_str(),
                "retained" | "replaced"
            ) {
                return Err(invalid(format!(
                    "[{label}] correction_factor_status {:?} must be \"retained\" or \
                     \"replaced\" (D9 rule 4 item iii)",
                    raw_hp.correction_factor_status
                )));
            }
            for (value, what) in [
                (raw_hp.correction_factor, "correction_factor"),
                (raw_hp.rhpp_derating, "rhpp_derating"),
            ] {
                positive(value, &format!("[{label}] {what}"))?;
                if value > 1.0 {
                    return Err(invalid(format!(
                        "[{label}] {what} {value} exceeds 1 — a derating never uprates"
                    )));
                }
            }
            if raw_hp.cop_curve.iter().any(|c| !c.is_finite()) {
                return Err(invalid(format!(
                    "[{label}] cop_curve {:?} must be finite",
                    raw_hp.cop_curve
                )));
            }
            if raw_hp.source_offset_k < 0.0 || !raw_hp.source_offset_k.is_finite() {
                return Err(invalid(format!(
                    "[{label}] source_offset_k {} must be non-negative",
                    raw_hp.source_offset_k
                )));
            }
            Ok(HeatPumpParams {
                cop_curve: raw_hp.cop_curve,
                correction_factor: PerUnit::new(raw_hp.correction_factor),
                correction_factor_status: raw_hp.correction_factor_status.clone(),
                rhpp_derating: PerUnit::new(raw_hp.rhpp_derating),
                source_offset: Temperature::celsius(raw_hp.source_offset_k),
            })
        };
        let ashp = heat_pump(&raw.ashp, "ashp", "air")?;
        let gshp = heat_pump(&raw.gshp, "gshp", "ground")?;

        positive(raw.ground_model.loop_depth_m, "loop_depth_m")?;
        positive(raw.ground_model.alpha_m2_s, "alpha_m2_s")?;
        band(raw.ground_model.loop_depth_band_m, "loop_depth")?;
        band(raw.ground_model.alpha_band_m2_s, "alpha")?;
        band(raw.ground_model.alpha_site_range_m2_s, "alpha site range")?;

        positive(raw.district_geothermal.cop_const, "cop_const")?;
        band(raw.district_geothermal.cop_const_band, "cop_const")?;
        if !raw.district_geothermal.basis.contains("delivered") {
            return Err(invalid(format!(
                "district basis {:?} must state the delivered-heat basis (D9 edit 7)",
                raw.district_geothermal.basis
            )));
        }

        positive(raw.geothermal.gradient_c_per_km, "gradient_c_per_km")?;
        band(raw.geothermal.gradient_band_c_per_km, "geothermal gradient")?;
        if !raw.geothermal.datum.contains("loop_depth") {
            return Err(invalid(format!(
                "geothermal datum {:?} must state the loop-depth anchoring (D16 rule-4 \
                 test-1 invariance: the gradient is measured from the committed shallow \
                 datum, not from z = 0)",
                raw.geothermal.datum
            )));
        }
        for (value, what) in [
            (raw.rhpp.ashp_spfh2_median, "ashp_spfh2_median"),
            (raw.rhpp.gshp_spfh2_median, "gshp_spfh2_median"),
            (raw.rhpp.ashp_spfh4_median, "ashp_spfh4_median"),
            (raw.rhpp.gshp_spfh4_median, "gshp_spfh4_median"),
        ] {
            positive(value, what)?;
        }
        for (values, what) in [
            (raw.rhpp.ashp_spfh2_iqr, "ashp_spfh2_iqr"),
            (raw.rhpp.gshp_spfh2_iqr, "gshp_spfh2_iqr"),
            (raw.rhpp.ashp_spfh4_iqr, "ashp_spfh4_iqr"),
            (raw.rhpp.gshp_spfh4_iqr, "gshp_spfh4_iqr"),
        ] {
            band(values, what)?;
        }

        Ok(Self {
            t_base: Temperature::celsius(raw.conventions.t_base_c),
            min_delta_t: Temperature::celsius(raw.conventions.min_delta_t_k),
            delivered_heat: Energy::gigawatt_hours(raw.heat_quantum.delivered_heat_twh * 1000.0),
            dhw_fraction: PerUnit::new(raw.heat_quantum.dhw_fraction),
            sink: SinkParams {
                radiator_t0: Temperature::celsius(raw.sink.radiator_t0_c),
                radiator_slope: raw.sink.radiator_slope,
                floor_t0: Temperature::celsius(raw.sink.floor_t0_c),
                floor_slope: raw.sink.floor_slope,
                dhw_sink: Temperature::celsius(raw.sink.dhw_sink_c),
            },
            ashp,
            gshp,
            ground: GroundModelParams {
                loop_depth: Length::metres(raw.ground_model.loop_depth_m),
                loop_depth_band: raw.ground_model.loop_depth_band_m.map(Length::metres),
                alpha: Diffusivity::square_metres_per_second(raw.ground_model.alpha_m2_s),
                alpha_band: raw
                    .ground_model
                    .alpha_band_m2_s
                    .map(Diffusivity::square_metres_per_second),
                alpha_site_range: raw
                    .ground_model
                    .alpha_site_range_m2_s
                    .map(Diffusivity::square_metres_per_second),
            },
            district: DistrictParams {
                cop_const: raw.district_geothermal.cop_const,
                cop_const_band: raw.district_geothermal.cop_const_band,
                basis: raw.district_geothermal.basis,
            },
            geothermal: GeothermalParams {
                gradient: TemperatureGradient::celsius_per_kilometre(
                    raw.geothermal.gradient_c_per_km,
                ),
                gradient_band: raw
                    .geothermal
                    .gradient_band_c_per_km
                    .map(TemperatureGradient::celsius_per_kilometre),
                datum: raw.geothermal.datum,
            },
            rhpp: RhppBands {
                comparison_boundary: raw.rhpp.comparison_boundary,
                ashp_spfh2_median: raw.rhpp.ashp_spfh2_median,
                ashp_spfh2_iqr: raw.rhpp.ashp_spfh2_iqr,
                gshp_spfh2_median: raw.rhpp.gshp_spfh2_median,
                gshp_spfh2_iqr: raw.rhpp.gshp_spfh2_iqr,
                ashp_spfh4_median: raw.rhpp.ashp_spfh4_median,
                ashp_spfh4_iqr: raw.rhpp.ashp_spfh4_iqr,
                gshp_spfh4_median: raw.rhpp.gshp_spfh4_median,
                gshp_spfh4_iqr: raw.rhpp.gshp_spfh4_iqr,
            },
        })
    }
}

// ---------------------------------------------------------------------
// The record: whole-calendar-year validation over the pinned trace.
// ---------------------------------------------------------------------

/// The temperature trace's validated record shape: whole calendar
/// years, so record-mean annual quantities are well defined.
struct Record {
    /// Number of whole calendar years.
    years: u32,
    /// Total hours in the record.
    total_hours: f64,
}

fn validate_record(trace: &Trace<Temperature>) -> Result<Record, GridError> {
    let invalid = |reason: String| GridError::InvalidHeatingOverlay { reason };
    let start = trace.start();
    let (start_year, start_month, start_day) = start.civil_date();
    let day_micros: i64 = 24 * 60 * 60 * 1_000_000;
    let is_new_year = |instant: UtcInstant| -> bool {
        let (_, month, day) = instant.civil_date();
        month == 1 && day == 1 && instant.unix_micros().rem_euclid(day_micros) == 0
    };
    if !(start_month == 1 && start_day == 1 && start.unix_micros().rem_euclid(day_micros) == 0) {
        return Err(invalid(format!(
            "temperature trace must start at a calendar year boundary (00:00 UTC, 1 Jan); \
             it starts at {start} — the rule-3 record-mean quantum is defined over whole \
             calendar years"
        )));
    }
    let after_end = start.plus_periods(trace.len() as i64);
    if !is_new_year(after_end) {
        return Err(invalid(format!(
            "temperature trace must end at a calendar year boundary (last period \
             23:30–00:00 UTC, 31 Dec); it ends before {after_end} — the rule-3 record-mean \
             quantum is defined over whole calendar years"
        )));
    }
    let (end_year_exclusive, _, _) = after_end.civil_date();
    let years = u32::try_from(end_year_exclusive - start_year).map_err(|_| {
        invalid("temperature trace record spans a non-positive year count".to_owned())
    })?;
    Ok(Record {
        years,
        total_hours: trace.len() as f64 * 0.5,
    })
}

// ---------------------------------------------------------------------
// The ground wave (D9 rule 4, GSHP; Kusuda–Achenbach).
// ---------------------------------------------------------------------

/// The fitted surface harmonic and its damped, lagged ground form:
/// `T_ground(t) = mean + damping · [a·cos(ω(t − lag)) + b·sin(ω(t − lag))]`
/// where `mean + a·cos(ωt) + b·sin(ωt)` is the least-squares single
/// annual harmonic of the surface trace and damping/lag are the
/// analytic conduction solution at the reference loop depth and soil
/// diffusivity — pinned physics, no free parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundWave {
    /// Fitted surface (air-trace) annual mean.
    pub surface_mean: Temperature,
    /// Fitted surface annual amplitude (√(a² + b²)).
    pub surface_amplitude: Temperature,
    /// Kusuda–Achenbach damping `exp(−z√(ω/2α))` at the reference z, α.
    pub damping: PerUnit,
    /// Kusuda–Achenbach phase lag `z/√(2αω)`.
    pub lag: Duration,
    // Fit internals for evaluation (kept private: `at` is the API).
    a: f64,
    b: f64,
    omega_per_hour: f64,
}

impl GroundWave {
    /// Undisturbed ground temperature a given span after the trace
    /// (record) start.
    #[must_use]
    pub fn at(&self, since_record_start: Duration) -> Temperature {
        let t = since_record_start.as_hours() - self.lag.as_hours();
        let phase = self.omega_per_hour * t;
        Temperature::celsius(
            self.surface_mean.as_celsius()
                + self.damping.value() * (self.a * phase.cos() + self.b * phase.sin()),
        )
    }

    /// The D16 depth-continuum wave: the SAME fitted annual harmonic,
    /// re-anchored at a geothermal resource depth — damping and lag
    /// recomputed at `resource_depth` (the identical Kusuda–Achenbach
    /// expressions, so the committed datum reproduces the committed
    /// wave bit-identically) and the mean warmed by
    /// `gradient × (resource_depth − loop_depth)` (D16 rule 1; the
    /// loop-depth datum is what makes the shallow default invariant —
    /// the rule-4 test-1 safety pin). The `surface_mean` field of the
    /// returned wave carries the WARMED source mean.
    #[must_use]
    pub fn re_anchored(
        &self,
        ground: &GroundModelParams,
        geothermal: &GeothermalParams,
        resource_depth: Length,
    ) -> GroundWave {
        let (damping, lag) = kusuda_achenbach(resource_depth, ground.alpha, self.omega_per_hour);
        let warmed = self.surface_mean.as_celsius()
            + geothermal.gradient.as_celsius_per_kilometre()
                * (resource_depth.as_kilometres() - ground.loop_depth.as_kilometres());
        GroundWave {
            surface_mean: Temperature::celsius(warmed),
            surface_amplitude: self.surface_amplitude,
            damping,
            lag,
            a: self.a,
            b: self.b,
            omega_per_hour: self.omega_per_hour,
        }
    }
}

/// The Kusuda–Achenbach damping and phase lag at a depth:
/// `damping = exp(−z√(ω/2α))`, `lag = z/√(2αω)`, ω in rad/s (KA65).
/// The single shared implementation — `fit_ground_wave` (the committed
/// shallow datum) and `GroundWave::re_anchored` (the D16 depth path)
/// must agree bit-for-bit at the same z.
fn kusuda_achenbach(depth: Length, alpha: Diffusivity, omega_per_hour: f64) -> (PerUnit, Duration) {
    let z = depth.as_metres();
    let alpha = alpha.as_square_metres_per_second();
    let omega_per_second = omega_per_hour / 3600.0;
    let damping = (-z * (omega_per_second / (2.0 * alpha)).sqrt()).exp();
    let lag_seconds = z / (2.0 * alpha * omega_per_second).sqrt();
    (PerUnit::new(damping), Duration::from_seconds(lag_seconds))
}

/// Fit the single annual harmonic of the temperature trace (closed-form
/// least squares on `[1, cos ωt, sin ωt]`, ω = 2π / the record's mean
/// year) and derive the Kusuda–Achenbach damping and lag from the
/// reference ground-model parameters.
pub fn fit_ground_wave(
    trace: &Trace<Temperature>,
    ground: &GroundModelParams,
) -> Result<GroundWave, GridError> {
    let record = validate_record(trace)?;
    let mean_year_hours = record.total_hours / f64::from(record.years);
    let omega_per_hour = 2.0 * std::f64::consts::PI / mean_year_hours;

    // Normal equations for T ≈ m + a·cos(ωt) + b·sin(ωt), t in hours.
    let mut s = [[0.0f64; 3]; 3];
    let mut rhs = [0.0f64; 3];
    for (index, value) in trace.values().iter().enumerate() {
        let phase = omega_per_hour * (index as f64 * 0.5);
        let basis = [1.0, phase.cos(), phase.sin()];
        for i in 0..3 {
            for j in 0..3 {
                s[i][j] += basis[i] * basis[j];
            }
            rhs[i] += basis[i] * value.as_celsius();
        }
    }
    let [m, a, b] = solve3(s, rhs).ok_or_else(|| GridError::InvalidHeatingOverlay {
        reason: "the annual-harmonic fit is degenerate (singular normal equations) — the \
                 temperature record is too short or constant in the fit basis"
            .to_owned(),
    })?;

    // damping = exp(−z√(ω/2α)), lag = z/√(2αω), ω in rad/s (KA65) —
    // the shared helper, so the D16 depth path reproduces this exactly
    // at the datum.
    let (damping, lag) = kusuda_achenbach(ground.loop_depth, ground.alpha, omega_per_hour);

    Ok(GroundWave {
        surface_mean: Temperature::celsius(m),
        surface_amplitude: Temperature::celsius(a.hypot(b)),
        damping,
        lag,
        a,
        b,
        omega_per_hour,
    })
}

/// Solve a 3×3 linear system by Gaussian elimination with partial
/// pivoting; `None` if singular.
fn solve3(mut m: [[f64; 3]; 3], mut rhs: [f64; 3]) -> Option<[f64; 3]> {
    for col in 0..3 {
        let pivot = (col..3).max_by(|&i, &j| {
            m[i][col]
                .abs()
                .partial_cmp(&m[j][col].abs())
                .unwrap_or(core::cmp::Ordering::Equal)
        })?;
        if m[pivot][col].abs() < 1e-12 {
            return None;
        }
        m.swap(col, pivot);
        rhs.swap(col, pivot);
        let pivot_row = m[col];
        for row in (col + 1)..3 {
            let factor = m[row][col] / pivot_row[col];
            for (k, &pivot_value) in pivot_row.iter().enumerate().skip(col) {
                m[row][k] -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[col];
        }
    }
    let mut x = [0.0f64; 3];
    for row in (0..3).rev() {
        let mut sum = rhs[row];
        for col in (row + 1)..3 {
            sum -= m[row][col] * x[col];
        }
        x[row] = sum / m[row][row];
    }
    Some(x)
}

// ---------------------------------------------------------------------
// COP evaluation.
// ---------------------------------------------------------------------

/// One entry's effective (post-override) parameters, echoed into run
/// outputs so overrides can never hide (D9 rules 2/6b).
#[derive(Debug, Clone, PartialEq)]
pub struct EntryParams {
    /// Effective COP curve (heat-pump kinds; `None` for district).
    pub cop_curve: Option<[f64; 3]>,
    /// Effective correction factor (heat-pump kinds).
    pub correction_factor: Option<PerUnit>,
    /// Effective RHPP derating (heat-pump kinds).
    pub rhpp_derating: Option<PerUnit>,
    /// Source offset (GSHP brine, 5 K; 0 for ASHP; `None` district).
    pub source_offset: Option<Temperature>,
    /// Effective constant COP (district; `None` for heat pumps).
    pub cop_const: Option<f64>,
    /// The D16 geothermal resource depth (GSHP entries that carry the
    /// scenario field; `None` = the committed shallow behaviour).
    /// Echoed like every convention (D9 rule 6b) but NOT an override
    /// of a reference parameter, so it never enters `overridden`.
    pub resource_depth: Option<Length>,
    /// Which fields the scenario overrode (empty = reference defaults).
    pub overridden: Vec<&'static str>,
}

fn entry_params(entry: &HeatingEntry, reference: &HeatingCopReference) -> EntryParams {
    let mut overridden = Vec::new();
    match entry.kind {
        HeatingKind::Ashp | HeatingKind::Gshp => {
            let defaults = if entry.kind == HeatingKind::Ashp {
                &reference.ashp
            } else {
                &reference.gshp
            };
            let curve = entry.cop_curve.unwrap_or(defaults.cop_curve);
            if entry.cop_curve.is_some() {
                overridden.push("cop_curve");
            }
            let correction = entry
                .correction_factor
                .unwrap_or(defaults.correction_factor);
            if entry.correction_factor.is_some() {
                overridden.push("correction_factor");
            }
            let derating = entry.rhpp_derating.unwrap_or(defaults.rhpp_derating);
            if entry.rhpp_derating.is_some() {
                overridden.push("rhpp_derating");
            }
            EntryParams {
                cop_curve: Some(curve),
                correction_factor: Some(correction),
                rhpp_derating: Some(derating),
                source_offset: Some(defaults.source_offset),
                cop_const: None,
                resource_depth: entry.resource_depth_m,
                overridden,
            }
        }
        HeatingKind::DistrictGeothermal => {
            let cop = entry.cop_const.unwrap_or(reference.district.cop_const);
            if entry.cop_const.is_some() {
                overridden.push("cop_const");
            }
            EntryParams {
                cop_curve: None,
                correction_factor: None,
                rhpp_derating: None,
                source_offset: None,
                cop_const: Some(cop),
                resource_depth: None,
                overridden,
            }
        }
    }
}

/// The corrected (and optionally derated) quadratic COP at a sink/source
/// pair, with the ΔT floor. COPs are dimensionless ratios.
fn heat_pump_cop(
    curve: [f64; 3],
    factor: f64,
    sink: Temperature,
    source: Temperature,
    min_delta_t: Temperature,
) -> f64 {
    let mut dt = (sink - source).as_celsius();
    if dt < min_delta_t.as_celsius() {
        dt = min_delta_t.as_celsius();
    }
    factor * (curve[0] + curve[1] * dt + curve[2] * dt * dt)
}

/// The weather-compensated radiator sink at an ambient temperature.
fn space_sink(sink: &SinkParams, ambient: Temperature) -> Temperature {
    sink.radiator_t0 - sink.radiator_slope * ambient
}

// ---------------------------------------------------------------------
// The overlay itself (D9 rules 3–4) and its echoed constants (rule 6b).
// ---------------------------------------------------------------------

/// The echoed overlay constants (D9 rule 6b: every convention visible
/// in outputs).
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingConstants {
    /// The pinned intensity coefficient k (GW of heat per K of heat
    /// need).
    pub k: HeatIntensity,
    /// The flat DHW heat rate.
    pub dhw_rate: Power,
    /// Mean annual degree-hours of the record — the denominator of k
    /// (D9 rule 3), on the [`DegreeHours`] newtype (ADR-4; review
    /// condition 2).
    pub mean_annual_degree_hours: DegreeHours,
    /// The record the constants were computed over: first period…
    pub record_start: UtcInstant,
    /// …and whole calendar years spanned (the pinned window; 40 for
    /// the reference 1985–2024 trace).
    pub record_years: u32,
    /// The electrified annual quantum (`delivered_heat_twh ×
    /// electrified_share` — the product the engine consumes).
    pub electrified_quantum: Energy,
    /// The degree-day base temperature (15.5 °C).
    pub t_base: Temperature,
    /// The fitted, damped, lagged ground wave (carries damping and lag).
    pub ground: GroundWave,
}

/// One portfolio entry's computed electrical-demand series plus its
/// effective parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingEntrySeries {
    /// The technology kind (also the output-series label).
    pub kind: HeatingKind,
    /// Its share of the electrified quantum.
    pub share: PerUnit,
    /// Per-period electrical demand, horizon-aligned.
    pub electrical: Vec<Power>,
    /// Effective parameters and which were overridden.
    pub params: EntryParams,
}

/// The computed heating overlay for one zone: horizon-aligned series
/// plus the echoed constants.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingOverlay {
    /// First settlement period of the (horizon-aligned) series.
    pub start: UtcInstant,
    /// Per-period delivered heat (thermal GW; identical across
    /// portfolios by construction — D9 rule 5).
    pub delivered_heat: Vec<Power>,
    /// Per-period total heating electrical demand (the series added to
    /// zone demand before dispatch).
    pub electrical_total: Vec<Power>,
    /// Per-entry electrical demand and effective parameters.
    pub entries: Vec<HeatingEntrySeries>,
    /// The pinned constants (k, DHW rate, damping, lag, …).
    pub constants: HeatingConstants,
}

impl HeatingOverlay {
    /// The seasonal-average delivered COP (SCOP) over the overlay's
    /// horizon: Σ delivered_heat / Σ electrical_total — the D16 SCOP
    /// read-out (docs/notes/d16-scop-readout-work-order.md). Read from
    /// the committed series themselves so it cannot drift from the
    /// electricity already in the pinned runs; AS-DELIVERED (the RHPP
    /// deratings are inside the electrical series), not nameplate.
    /// `None` when the overlay draws no electricity (zero electrified
    /// quantum or an empty horizon) — the ratio is undefined.
    pub fn seasonal_cop(&self) -> Option<f64> {
        let heat: f64 = self.delivered_heat.iter().map(|p| p.as_gigawatts()).sum();
        let electricity: f64 = self.electrical_total.iter().map(|p| p.as_gigawatts()).sum();
        (electricity > 0.0).then(|| heat / electricity)
    }
}

/// Compute the heating overlay: constants over the temperature trace's
/// FULL record (whole calendar years enforced), series over the
/// requested horizon window. See the module docs for every convention.
///
/// Errors: [`GridError::InvalidHeatingOverlay`] for a partial-year
/// record, a horizon outside the record, a degenerate degree-hour
/// record, or a violated district-COP premise.
pub fn compute_overlay(
    spec: &HeatingSpec,
    reference: &HeatingCopReference,
    t_pop: &Trace<Temperature>,
    horizon_start: UtcInstant,
    periods: usize,
) -> Result<HeatingOverlay, GridError> {
    let invalid = |reason: String| GridError::InvalidHeatingOverlay { reason };
    let record = validate_record(t_pop)?;

    // Horizon-window alignment inside the record.
    let record_start = t_pop.start();
    if horizon_start.unix_micros() < record_start.unix_micros() {
        return Err(invalid(format!(
            "run horizon starts at {horizon_start}, before the temperature record ({record_start})"
        )));
    }
    let offset = record_start
        .periods_until_inclusive(horizon_start)
        .map_err(|_| {
            invalid(format!(
                "run horizon start {horizon_start} is not aligned to the temperature \
                 record's half-hourly periods (record start {record_start})"
            ))
        })?
        - 1;
    if offset + periods > t_pop.len() {
        return Err(invalid(format!(
            "run horizon ({periods} periods from {horizon_start}) extends beyond the \
             temperature record ({} periods from {record_start})",
            t_pop.len()
        )));
    }

    // Rule-3 constants over the FULL record: never per-year, never
    // per-horizon.
    let dt = Duration::half_hour();
    let t_base = reference.t_base;
    let mut total_degree_hours = DegreeHours::celsius_hours(0.0);
    for &temp in t_pop.values() {
        let need = t_base - temp;
        if need.as_celsius() > 0.0 {
            total_degree_hours = total_degree_hours + need * dt;
        }
    }
    let mean_annual_degree_hours = total_degree_hours / f64::from(record.years);
    let electrified_quantum = spec.delivered_heat_twh * spec.electrified_share;
    let space_quantum = electrified_quantum * (PerUnit::new(1.0) - spec.dhw_fraction);
    let dhw_quantum = electrified_quantum * spec.dhw_fraction;
    if mean_annual_degree_hours.as_celsius_hours() <= 0.0 && space_quantum.as_gigawatt_hours() > 0.0
    {
        return Err(invalid(
            "the record has zero degree-hours (never below T_base) but a positive \
             space-heat quantum — the intensity k is undefined"
                .to_owned(),
        ));
    }
    let k = if space_quantum.as_gigawatt_hours() > 0.0 {
        space_quantum / mean_annual_degree_hours
    } else {
        HeatIntensity::gigawatts_per_kelvin(0.0)
    };
    let mean_year_hours = record.total_hours / f64::from(record.years);
    let dhw_rate = dhw_quantum / Duration::hours(mean_year_hours);

    // The ground wave, fitted on the full record (used by GSHP only,
    // but echoed always — it is a pinned constant of the run).
    let ground = fit_ground_wave(t_pop, &reference.ground)?;

    // Effective per-entry parameters, and the machine-checked district
    // premise (D9 edit 7): cop_const must exceed the heat pumps'
    // maximum record COP on the engine-facing (derated) curves. The
    // premise is checked on the CURVE (heat-pump-regime) COPs at the
    // committed shallow wave: a D16 depth entry's effective COP is
    // capped at cop_const by construction (it can tie the district
    // endpoint — that tie IS the direct-use unification, not a premise
    // violation), so depth never loosens the check; an override curve
    // above cop_const still fails it here, conservatively, even though
    // the depth path would cap it.
    let params: Vec<EntryParams> = spec
        .entries
        .iter()
        .map(|entry| entry_params(entry, reference))
        .collect();
    if let Some(district_cop) = params.iter().find_map(|p| p.cop_const) {
        let mut hp_max = record_max_cop(HeatingKind::Ashp, reference, t_pop)?.max(record_max_cop(
            HeatingKind::Gshp,
            reference,
            t_pop,
        )?);
        // Overridden heat-pump curves in this portfolio move the
        // premise's right-hand side; include them.
        for (entry, entry_params) in spec.entries.iter().zip(&params) {
            if entry.kind.is_heat_pump() && !entry_params.overridden.is_empty() {
                hp_max = hp_max.max(record_max_cop_with(
                    entry.kind,
                    entry_params,
                    reference,
                    t_pop,
                    &ground,
                )?);
            }
        }
        if district_cop <= hp_max {
            return Err(invalid(format!(
                "district cop_const {district_cop} does not exceed the heat pumps' maximum \
                 record COP {hp_max:.3} — the district-lowest ordering premise (D9 rule 4, \
                 edit 7) is checked, not assumed"
            )));
        }
    }

    // The D16 depth path: a GSHP entry carrying `resource_depth_m`
    // sources from the SAME fitted harmonic re-anchored at its resource
    // depth (damping/lag at z, mean warmed by gradient × (z − datum)).
    // Entries without the field keep the committed wave — the absent
    // path is byte-identical to pre-v8, and the datum depth reproduces
    // it bit-identically (rule-4 test 1).
    let entry_waves: Vec<Option<GroundWave>> = spec
        .entries
        .iter()
        .map(|entry| {
            entry
                .resource_depth_m
                .map(|depth| ground.re_anchored(&reference.ground, &reference.geothermal, depth))
        })
        .collect();

    // The horizon-aligned series.
    let zero = Power::gigawatts(0.0);
    let mut delivered_heat = Vec::with_capacity(periods);
    let mut electrical_total = vec![zero; periods];
    let mut per_entry: Vec<Vec<Power>> = spec
        .entries
        .iter()
        .map(|_| Vec::with_capacity(periods))
        .collect();

    for (t, total_slot) in electrical_total.iter_mut().enumerate() {
        let record_index = offset + t;
        let air = t_pop.values()[record_index];
        let need = t_base - air;
        let space_heat = if need.as_celsius() > 0.0 {
            k * need
        } else {
            zero
        };
        let heat = space_heat + dhw_rate;
        delivered_heat.push(heat);

        let sink = space_sink(&reference.sink, air);
        let since_start = Duration::hours(record_index as f64 * 0.5);
        for (((entry, entry_params), entry_wave), series) in spec
            .entries
            .iter()
            .zip(&params)
            .zip(&entry_waves)
            .zip(per_entry.iter_mut())
        {
            let electrical = match entry.kind {
                HeatingKind::Ashp | HeatingKind::Gshp => {
                    let source = match entry_wave {
                        // The D16 depth path (GSHP with resource_depth_m).
                        Some(wave) => wave.at(since_start),
                        None if entry.kind == HeatingKind::Ashp => air,
                        None => ground.at(since_start),
                    };
                    // Infallible unwraps by construction of
                    // `entry_params` for heat-pump kinds.
                    let curve = entry_params.cop_curve.unwrap_or([0.0; 3]);
                    let factor = entry_params.correction_factor.map_or(1.0, PerUnit::value)
                        * entry_params.rhpp_derating.map_or(1.0, PerUnit::value);
                    let offset_k = entry_params
                        .source_offset
                        .unwrap_or(Temperature::celsius(0.0));
                    let source = source - offset_k;
                    // COPs per component. On the D16 depth path the
                    // direct-use handoff applies (rule 1): when the
                    // brine-offset source meets the sink, no heat pump
                    // is needed — the component passes through at the
                    // district effective COP; below that, the curve COP
                    // is CAPPED at the pass-through value ("capped at,
                    // then handed to"), so the district-lowest ordering
                    // (D9 edit 7) can tie but never invert. The absent
                    // path is the committed pre-v8 expression, bit for
                    // bit.
                    let component_cop = |component_sink: Temperature| -> f64 {
                        if entry_wave.is_some() {
                            let pass = reference.district.cop_const;
                            if source.as_celsius() >= component_sink.as_celsius() {
                                pass
                            } else {
                                heat_pump_cop(
                                    curve,
                                    factor,
                                    component_sink,
                                    source,
                                    reference.min_delta_t,
                                )
                                .min(pass)
                            }
                        } else {
                            heat_pump_cop(
                                curve,
                                factor,
                                component_sink,
                                source,
                                reference.min_delta_t,
                            )
                        }
                    };
                    let space_cop = component_cop(sink);
                    let dhw_cop = component_cop(reference.sink.dhw_sink);
                    // Positivity guard (review condition 3,
                    // q5-heating-engine-review.md): the committed
                    // reference curves are positive-definite over the
                    // record, but a legal per-entry cop_curve override
                    // can go non-positive at large ΔT, which would
                    // silently yield NEGATIVE electrical demand — a
                    // structured error instead, naming entry and period.
                    for (cop, component) in [(space_cop, "space"), (dhw_cop, "DHW")] {
                        if cop <= 0.0 || cop.is_nan() {
                            return Err(invalid(format!(
                                "entry {}: effective {component} COP {cop} is not positive at \
                                 {} (T_pop {} °C) — a COP-curve override must stay positive \
                                 over the evaluated ΔT range, or electrical demand would go \
                                 negative",
                                entry.kind,
                                horizon_start.plus_periods(t as i64),
                                air.as_celsius()
                            )));
                        }
                    }
                    entry.share.value() * (space_heat / space_cop + dhw_rate / dhw_cop)
                }
                HeatingKind::DistrictGeothermal => {
                    let cop = entry_params.cop_const.unwrap_or(1.0);
                    entry.share.value() * (heat / cop)
                }
            };
            series.push(electrical);
            *total_slot = *total_slot + electrical;
        }
    }

    Ok(HeatingOverlay {
        start: horizon_start,
        delivered_heat,
        electrical_total,
        entries: spec
            .entries
            .iter()
            .zip(params)
            .zip(per_entry)
            .map(|((entry, entry_params), electrical)| HeatingEntrySeries {
                kind: entry.kind,
                share: entry.share,
                electrical,
                params: entry_params,
            })
            .collect(),
        constants: HeatingConstants {
            k,
            dhw_rate,
            mean_annual_degree_hours,
            record_start,
            record_years: record.years,
            electrified_quantum,
            t_base,
            ground,
        },
    })
}

// ---------------------------------------------------------------------
// Record COP statistics and the edit-6 SPFH2 reproduction.
// ---------------------------------------------------------------------

/// Maximum COP over the record for a heat-pump kind on the ENGINE-FACING
/// curve (correction × RHPP derating applied) at the reference
/// parameters — the machine-checked side of the district-lowest premise.
///
/// Errors on a district kind (it has no curve) or an invalid record.
pub fn record_max_cop(
    kind: HeatingKind,
    reference: &HeatingCopReference,
    t_pop: &Trace<Temperature>,
) -> Result<f64, GridError> {
    let defaults = match kind {
        HeatingKind::Ashp => &reference.ashp,
        HeatingKind::Gshp => &reference.gshp,
        HeatingKind::DistrictGeothermal => {
            return Err(GridError::InvalidHeatingOverlay {
                reason: "district_geothermal has no COP curve — its effective COP is the \
                         constant cop_const"
                    .to_owned(),
            });
        }
    };
    let params = EntryParams {
        cop_curve: Some(defaults.cop_curve),
        correction_factor: Some(defaults.correction_factor),
        rhpp_derating: Some(defaults.rhpp_derating),
        source_offset: Some(defaults.source_offset),
        cop_const: None,
        resource_depth: None,
        overridden: Vec::new(),
    };
    let ground = fit_ground_wave(t_pop, &reference.ground)?;
    record_max_cop_with(kind, &params, reference, t_pop, &ground)
}

/// [`record_max_cop`] at explicit effective parameters (override-aware).
fn record_max_cop_with(
    kind: HeatingKind,
    params: &EntryParams,
    reference: &HeatingCopReference,
    t_pop: &Trace<Temperature>,
    ground: &GroundWave,
) -> Result<f64, GridError> {
    let curve = params.cop_curve.unwrap_or([0.0; 3]);
    let factor = params.correction_factor.map_or(1.0, PerUnit::value)
        * params.rhpp_derating.map_or(1.0, PerUnit::value);
    let offset = params.source_offset.unwrap_or(Temperature::celsius(0.0));
    // The caller guarantees a heat-pump kind (district is rejected in
    // `record_max_cop` and never routed here by `compute_overlay`).
    let uses_ground = kind == HeatingKind::Gshp;
    let mut max = f64::NEG_INFINITY;
    for (index, &air) in t_pop.values().iter().enumerate() {
        let source = if uses_ground {
            ground.at(Duration::hours(index as f64 * 0.5))
        } else {
            air
        } - offset;
        let sink = space_sink(&reference.sink, air);
        let space = heat_pump_cop(curve, factor, sink, source, reference.min_delta_t);
        let dhw = heat_pump_cop(
            curve,
            factor,
            reference.sink.dhw_sink,
            source,
            reference.min_delta_t,
        );
        max = max.max(space.max(dhw));
    }
    Ok(max)
}

/// The model-implied SPFH2 for a heat-pump kind with the rule-3 heat
/// weighting over the pinned record, on the CORRECTED (0.85) but
/// PRE-DERATING curve — the D9 edit-6 item (i) quantity. The engine
/// package's acceptance step is REPRODUCTION of the data package's
/// reviewed determination (ASHP 3.221, GSHP 3.838; to-median deratings
/// 0.823 / 0.732), not a fresh one:
///
/// `1/SPF = (1−f)·Σ[hn(t)/COP_sp(t)]/Σhn(t) + f·mean[1/COP_dhw(t)]`,
/// f = the reference DHW fraction (0.170).
pub fn implied_spfh2(
    kind: HeatingKind,
    reference: &HeatingCopReference,
    t_pop: &Trace<Temperature>,
) -> Result<f64, GridError> {
    let defaults = match kind {
        HeatingKind::Ashp => &reference.ashp,
        HeatingKind::Gshp => &reference.gshp,
        HeatingKind::DistrictGeothermal => {
            return Err(GridError::InvalidHeatingOverlay {
                reason: "SPFH2 is a heat-pump quantity; district_geothermal carries cop_const"
                    .to_owned(),
            });
        }
    };
    let ground = fit_ground_wave(t_pop, &reference.ground)?;
    let factor = defaults.correction_factor.value(); // pre-derating
    let f = reference.dhw_fraction.value();
    let uses_ground = kind == HeatingKind::Gshp;

    let mut weighted_inverse_space = 0.0f64;
    let mut total_need = 0.0f64;
    let mut inverse_dhw_sum = 0.0f64;
    for (index, &air) in t_pop.values().iter().enumerate() {
        let source = if uses_ground {
            ground.at(Duration::hours(index as f64 * 0.5))
        } else {
            air
        } - defaults.source_offset;
        let need = (reference.t_base - air).as_celsius().max(0.0);
        if need > 0.0 {
            let sink = space_sink(&reference.sink, air);
            let cop = heat_pump_cop(
                defaults.cop_curve,
                factor,
                sink,
                source,
                reference.min_delta_t,
            );
            weighted_inverse_space += need / cop;
            total_need += need;
        }
        let dhw_cop = heat_pump_cop(
            defaults.cop_curve,
            factor,
            reference.sink.dhw_sink,
            source,
            reference.min_delta_t,
        );
        inverse_dhw_sum += 1.0 / dhw_cop;
    }
    if total_need <= 0.0 {
        return Err(GridError::InvalidHeatingOverlay {
            reason: "the record has zero degree-hours — the space-heat SPF weighting is \
                     undefined"
                .to_owned(),
        });
    }
    let inverse_spf = (1.0 - f) * (weighted_inverse_space / total_need)
        + f * (inverse_dhw_sum / t_pop.len() as f64);
    Ok(1.0 / inverse_spf)
}
