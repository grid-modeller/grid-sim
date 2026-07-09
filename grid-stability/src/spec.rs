//! Stability event specification (Stage 6): the complete, self-contained
//! description of one loss-of-infeed event — system conditions, staged
//! losses, response services, LFDD scheme, era-dependent operating
//! limits, and the pinned RoCoF measurement window.
//!
//! Era-dependent limits are **event-spec inputs, not constants**
//! (docs/04 Stage 6, corrected 2026-07-03): the loss-of-mains RoCoF
//! relay limit was 0.125 Hz/s in 2019, is 1 Hz/s post-ALoMCP, and NESO
//! designs to 0.5 Hz/s; LFDD stage 1 is 48.8 Hz (49.2 Hz is the SQSS
//! infrequent-loss floor, a different standard). The spec carries the
//! limits of *its* era; the engine only reports against them.
//!
//! Parsing is strict, mirroring the scenario schema: a mandatory
//! `schema = "stability-event-v1"` string, `deny_unknown_fields`
//! everywhere, MW/Hz/seconds spelled out in field names and converted
//! to the canonical newtypes at this single boundary.

use std::path::Path;

use serde::Deserialize;

use grid_core::GridError;
use grid_core::units::{Damping, Duration, Frequency, Inertia, PerUnit, Power, Rocof};

/// The event-spec schema identifier this engine reads.
pub const EVENT_SCHEMA: &str = "stability-event-v1";

/// A fully validated stability event (see the module docs).
#[derive(Debug, Clone, PartialEq)]
pub struct EventSpec {
    /// Human-readable event name.
    pub name: String,
    /// Nominal system frequency (50 Hz in GB).
    pub f0: Frequency,
    /// Pre-event system demand (the damping base and the LFDD stage
    /// percentage base).
    pub demand: Power,
    /// System inertia E = Σ(H × MVA) at the event, GVA·s — from the
    /// published record, or from `system_inertia` at an adequacy
    /// timestep.
    pub inertia: Inertia,
    /// Load damping, percent of remaining demand per hertz of
    /// deviation. A first-order free parameter (literature 1–2.5 %/Hz);
    /// every spec documents its own derivation.
    pub load_damping: Damping,
    /// Simulated span from the event start.
    pub duration: Duration,
    /// Fixed integrator step (default 10 ms; see `swing` for the
    /// stability/accuracy discussion).
    pub timestep: Duration,
    /// Staged infeed losses (timed exogenous inputs, per ADR-2).
    pub losses: Vec<InfeedLoss>,
    /// Frequency-response services.
    pub responses: Vec<ResponseService>,
    /// Low-frequency demand disconnection scheme (staged demand blocks).
    pub lfdd: Option<LfddScheme>,
    /// Era-dependent operating limits, for reporting.
    pub limits: Option<OperatingLimits>,
    /// The pinned RoCoF measurement window.
    pub rocof_window: Option<RocofWindow>,
}

/// One timed infeed loss.
#[derive(Debug, Clone, PartialEq)]
pub struct InfeedLoss {
    /// Label for the timeline.
    pub name: String,
    /// Lost infeed (positive).
    pub power: Power,
    /// When the trip occurs, seconds from the event start.
    pub at: Duration,
}

/// One frequency-response service: a held volume with a measured
/// delivery factor and a delivery envelope.
///
/// The delivered power at time t is
/// `held × delivery_factor × envelope(t) × shape(f)`, where the
/// envelope ramps linearly from `delay` after activation to full at
/// `delay + ramp`, holds until `sustain` after activation (if given),
/// then runs down linearly over `rundown` (if given; a missing
/// `rundown` with a `sustain` is a step to zero). Dynamic services
/// activate at the event start; static services activate when
/// frequency first crosses their trigger (latched).
#[derive(Debug, Clone, PartialEq)]
pub struct ResponseService {
    /// Service label.
    pub name: String,
    /// Held (contracted) volume.
    pub power: Power,
    /// Measured delivery factor applied to the held volume.
    pub delivery_factor: PerUnit,
    /// Delivery shape: proportional (dynamic) or triggered (static).
    pub shape: ResponseShape,
    /// Delay from activation to first delivery.
    pub delay: Duration,
    /// Ramp length from first delivery to full delivery.
    pub ramp: Duration,
    /// Time after activation at which delivery stops being sustained.
    pub sustain: Option<Duration>,
    /// Rundown length after `sustain`.
    pub rundown: Option<Duration>,
}

/// The frequency-dependence of a response service's delivery.
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseShape {
    /// Droop-proportional: delivery scales with the under-frequency
    /// deviation, reaching full at `droop_full_deviation` (GB primary
    /// response is sized to be fully delivered at −0.5 Hz). No delivery
    /// at or above nominal.
    Dynamic {
        /// Deviation at which delivery saturates.
        droop_full_deviation: Frequency,
    },
    /// Stepped: activates (latched) when frequency first falls below
    /// the trigger, then delivers per the envelope regardless of
    /// subsequent recovery.
    Static {
        /// Activation threshold.
        trigger: Frequency,
    },
}

/// A low-frequency demand-disconnection scheme: staged demand blocks
/// tripped by under-frequency relays, each after the scheme's action
/// delay (relay + breaker), latched — reconnection is manual and hours
/// away, outside the event window.
#[derive(Debug, Clone, PartialEq)]
pub struct LfddScheme {
    /// Relay-plus-breaker action delay (published range 0.2–0.5 s for
    /// the 2019 event).
    pub action_delay: Duration,
    /// Stages in strictly descending frequency order.
    pub stages: Vec<LfddStage>,
}

/// One LFDD stage.
#[derive(Debug, Clone, PartialEq)]
pub struct LfddStage {
    /// Trigger frequency.
    pub frequency: Frequency,
    /// Demand block disconnected when the stage operates.
    pub block: Power,
}

/// Era-dependent operating limits carried by the spec (reporting only —
/// the physics does not read them).
#[derive(Debug, Clone, PartialEq)]
pub struct OperatingLimits {
    /// Loss-of-mains RoCoF relay limit of the era.
    pub rocof_relay: Option<Rocof>,
    /// Statutory/operational frequency floor (49.5 Hz in GB).
    pub statutory_floor: Option<Frequency>,
}

/// The pinned RoCoF measurement window (docs/04 T3: RoCoF is
/// window-definition dependent, so the window is an input).
#[derive(Debug, Clone, PartialEq)]
pub struct RocofWindow {
    /// Window start, seconds from the event start.
    pub start: Duration,
    /// Window length.
    pub duration: Duration,
}

// ---------------------------------------------------------------------
// TOML-facing raw structs: explicit units in field names, converted to
// the canonical newtypes in one place (ADR-4 boundary).
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpec {
    schema: String,
    name: String,
    f0_hz: f64,
    demand_gw: f64,
    inertia_gva_s: f64,
    load_damping_percent_per_hz: f64,
    duration_s: f64,
    #[serde(default = "default_timestep_ms")]
    timestep_ms: f64,
    #[serde(default)]
    losses: Vec<RawLoss>,
    #[serde(default)]
    responses: Vec<RawResponse>,
    lfdd: Option<RawLfdd>,
    limits: Option<RawLimits>,
    rocof_window: Option<RawWindow>,
}

fn default_timestep_ms() -> f64 {
    10.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLoss {
    name: String,
    mw: f64,
    at_s: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResponse {
    name: String,
    kind: RawResponseKind,
    mw: f64,
    #[serde(default = "default_delivery_factor")]
    delivery_factor: f64,
    /// Dynamic services: deviation (Hz) at which delivery saturates.
    droop_full_deviation_hz: Option<f64>,
    /// Static services: activation threshold (Hz).
    trigger_hz: Option<f64>,
    delay_s: f64,
    ramp_s: f64,
    sustain_s: Option<f64>,
    rundown_s: Option<f64>,
}

fn default_delivery_factor() -> f64 {
    1.0
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RawResponseKind {
    Dynamic,
    Static,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLfdd {
    action_delay_s: f64,
    stages: Vec<RawStage>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStage {
    hz: f64,
    mw: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimits {
    rocof_relay_hz_per_s: Option<f64>,
    statutory_floor_hz: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWindow {
    start_s: f64,
    duration_s: f64,
}

impl EventSpec {
    /// Parse an event spec from TOML text (strict; see module docs).
    pub fn from_toml_str(toml_text: &str) -> Result<Self, GridError> {
        let raw: RawSpec =
            toml::from_str(toml_text).map_err(|source| GridError::EventSpecParse {
                source: Box::new(source),
            })?;
        Self::from_raw(raw)
    }

    /// Read and parse an event-spec file, attaching the path to any
    /// error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InEventSpecFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    fn from_raw(raw: RawSpec) -> Result<Self, GridError> {
        let invalid = |reason: String| GridError::InvalidEventSpec { reason };
        if raw.schema != EVENT_SCHEMA {
            return Err(invalid(format!(
                "schema {:?} is not the supported {EVENT_SCHEMA:?}",
                raw.schema
            )));
        }
        let finite_positive = |value: f64, what: &str| -> Result<f64, GridError> {
            if value.is_nan() || value <= 0.0 || !value.is_finite() {
                return Err(GridError::InvalidEventSpec {
                    reason: format!("{what} must be positive and finite, got {value}"),
                });
            }
            Ok(value)
        };
        let finite_non_negative = |value: f64, what: &str| -> Result<f64, GridError> {
            if value < 0.0 || !value.is_finite() {
                return Err(GridError::InvalidEventSpec {
                    reason: format!("{what} must be non-negative and finite, got {value}"),
                });
            }
            Ok(value)
        };

        let f0 = Frequency::hertz(finite_positive(raw.f0_hz, "f0_hz")?);
        let demand = Power::gigawatts(finite_positive(raw.demand_gw, "demand_gw")?);
        let inertia =
            Inertia::gigavolt_ampere_seconds(finite_positive(raw.inertia_gva_s, "inertia_gva_s")?);
        let load_damping = Damping::percent_of_demand_per_hertz(finite_non_negative(
            raw.load_damping_percent_per_hz,
            "load_damping_percent_per_hz",
        )?);
        let duration = Duration::from_seconds(finite_positive(raw.duration_s, "duration_s")?);
        let timestep =
            Duration::from_seconds(finite_positive(raw.timestep_ms, "timestep_ms")? / 1000.0);
        if timestep.as_seconds() > duration.as_seconds() {
            return Err(invalid("timestep exceeds the event duration".to_owned()));
        }

        let mut losses = Vec::with_capacity(raw.losses.len());
        for loss in raw.losses {
            losses.push(InfeedLoss {
                power: Power::megawatts(finite_positive(
                    loss.mw,
                    &format!("losses.{}.mw", loss.name),
                )?),
                at: Duration::from_seconds(finite_non_negative(
                    loss.at_s,
                    &format!("losses.{}.at_s", loss.name),
                )?),
                name: loss.name,
            });
        }

        let mut responses = Vec::with_capacity(raw.responses.len());
        for service in raw.responses {
            let context = format!("responses.{}", service.name);
            let shape = match service.kind {
                RawResponseKind::Dynamic => {
                    if service.trigger_hz.is_some() {
                        return Err(invalid(format!(
                            "{context}: trigger_hz is a static-service field"
                        )));
                    }
                    let droop = service.droop_full_deviation_hz.ok_or_else(|| {
                        GridError::InvalidEventSpec {
                            reason: format!(
                                "{context}: dynamic services need droop_full_deviation_hz"
                            ),
                        }
                    })?;
                    ResponseShape::Dynamic {
                        droop_full_deviation: Frequency::hertz(finite_positive(
                            droop,
                            &format!("{context}.droop_full_deviation_hz"),
                        )?),
                    }
                }
                RawResponseKind::Static => {
                    if service.droop_full_deviation_hz.is_some() {
                        return Err(invalid(format!(
                            "{context}: droop_full_deviation_hz is a dynamic-service field"
                        )));
                    }
                    let trigger =
                        service
                            .trigger_hz
                            .ok_or_else(|| GridError::InvalidEventSpec {
                                reason: format!("{context}: static services need trigger_hz"),
                            })?;
                    ResponseShape::Static {
                        trigger: Frequency::hertz(finite_positive(
                            trigger,
                            &format!("{context}.trigger_hz"),
                        )?),
                    }
                }
            };
            let factor = finite_non_negative(
                service.delivery_factor,
                &format!("{context}.delivery_factor"),
            )?;
            if factor > 1.0 {
                return Err(invalid(format!(
                    "{context}: delivery_factor {factor} exceeds 1 (a measured delivery \
                     factor is a fraction of the held volume)"
                )));
            }
            if service.sustain_s.is_none() && service.rundown_s.is_some() {
                return Err(invalid(format!(
                    "{context}: rundown_s without sustain_s (an indefinitely sustained \
                     service never runs down)"
                )));
            }
            responses.push(ResponseService {
                power: Power::megawatts(finite_positive(service.mw, &format!("{context}.mw"))?),
                delivery_factor: PerUnit::new(factor),
                shape,
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

        let lfdd = match raw.lfdd {
            None => None,
            Some(raw_lfdd) => {
                let mut stages = Vec::with_capacity(raw_lfdd.stages.len());
                let mut previous: Option<f64> = None;
                for stage in &raw_lfdd.stages {
                    let hz = finite_positive(stage.hz, "lfdd stage hz")?;
                    if let Some(prev) = previous
                        && hz >= prev
                    {
                        return Err(invalid(format!(
                            "lfdd stages must be in strictly descending frequency order \
                             ({hz} Hz follows {prev} Hz)"
                        )));
                    }
                    previous = Some(hz);
                    stages.push(LfddStage {
                        frequency: Frequency::hertz(hz),
                        block: Power::megawatts(finite_positive(stage.mw, "lfdd stage mw")?),
                    });
                }
                if stages.is_empty() {
                    return Err(invalid("lfdd scheme with no stages".to_owned()));
                }
                Some(LfddScheme {
                    action_delay: Duration::from_seconds(finite_non_negative(
                        raw_lfdd.action_delay_s,
                        "lfdd.action_delay_s",
                    )?),
                    stages,
                })
            }
        };

        let limits = raw.limits.map(|raw_limits| OperatingLimits {
            rocof_relay: raw_limits.rocof_relay_hz_per_s.map(Rocof::hertz_per_second),
            statutory_floor: raw_limits.statutory_floor_hz.map(Frequency::hertz),
        });

        let rocof_window = match raw.rocof_window {
            None => None,
            Some(window) => Some(RocofWindow {
                start: Duration::from_seconds(finite_non_negative(
                    window.start_s,
                    "rocof_window.start_s",
                )?),
                duration: Duration::from_seconds(finite_positive(
                    window.duration_s,
                    "rocof_window.duration_s",
                )?),
            }),
        };

        Ok(Self {
            name: raw.name,
            f0,
            demand,
            inertia,
            load_damping,
            duration,
            timestep,
            losses,
            responses,
            lfdd,
            limits,
            rocof_window,
        })
    }
}
