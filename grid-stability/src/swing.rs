//! Single-bus aggregate swing-equation event simulation (ADR-2:
//! explicitly NOT EMT or multi-bus — one frequency, one inertia, timed
//! exogenous losses, response-service envelopes, LFDD demand blocks).
//!
//! ## The model, in prose
//!
//! The system's stored kinetic energy at frequency f is
//! `E_kin = E · (f/f₀)²`, where E is the spec's synchronous inertia
//! (GVA·s ≡ GW·s). Power imbalance drains it:
//!
//! `d/dt [E · (f/f₀)²] = −deficit(t, f)`  ⇒
//! `df/dt = −deficit · f₀² / (2 · E · f)`
//!
//! This is the **kinetic-energy-exact** form; the familiar linearised
//! `df/dt = −f₀·ΔP/(2E)` is its value at f = f₀. The frequency-
//! dependent correction factor f₀/f (≈ +2.4 % at 48.8 Hz) is kept
//! because the Stage 6 events reach the LFDD band where it matters at
//! the tens-of-mHz level. E is held constant through the event: the
//! machines lost in the modelled trips carried ~1–3 % of system
//! inertia (ESO appendices Q42 show 219.6 → 212.4 GVA·s through 9 Aug
//! 2019), second-order against the ±5 % official-inertia spread the
//! acceptance tests span.
//!
//! `deficit(t, f) = Σ losses(at ≤ t) − Σ response(t, f) − LFDD(t)
//!                  − damping(f)` (GW), where
//!
//! - **losses** are timed steps (published trip sequence — inputs, not
//!   predictions; ADR-2);
//! - **response services** deliver
//!   `held × delivery_factor × envelope(t) × shape(f)` — envelope:
//!   linear ramp after a delay, optional sustain limit and rundown;
//!   shape: droop-proportional for dynamic services (full at the
//!   spec's droop deviation, zero at/above nominal), latched threshold
//!   activation for static services (see `spec`);
//! - **LFDD** stages trip (latched) when f crosses their threshold,
//!   acting one relay/breaker delay later — the disconnected block adds
//!   to supply-side balance and leaves the damping base;
//! - **damping** is `damping %/Hz × (f₀ − f) × remaining demand` —
//!   under-frequency load relief, negative above nominal.
//!
//! ## Integrator: Heun (RK2), fixed step, default 10 ms
//!
//! - **Accuracy**: second order; against the constant-ΔP closed form
//!   `f(t) = f₀√(1 − P·t/E)` the measured global error at 10 ms is
//!   ~1e-9 Hz over 60 s (acceptance test pins ≤ 1 µHz). Event
//!   discontinuities (trips, LFDD actions) are evaluated on the step
//!   grid, so step-time error is bounded by one step (10 ms ≪ the
//!   0.2–0.5 s LFDD action delay it feeds).
//! - **Stability**: the stiffest feedback is damping,
//!   `τ = 2E/(f₀·D) ≈ 17 s` for the 2019 conditions — four orders
//!   above the step, nowhere near explicit-method limits.
//! - **Determinism**: fixed step count from the spec, no adaptivity,
//!   no randomness; identical spec ⇒ bit-identical trace (ADR-5).
//!
//! Threshold crossings (LFDD triggers, static-service activation) are
//! detected on committed states at step boundaries — trigger times are
//! quantised to the step, which under-resolves nothing at 10 ms vs the
//! 200–500 ms action delays.

use grid_core::GridError;
use grid_core::units::{Duration, Frequency, Power, Rocof};

use crate::spec::{EventSpec, ResponseShape};

/// One LFDD stage operation in a simulated event.
#[derive(Debug, Clone, PartialEq)]
pub struct LfddAction {
    /// 1-based stage number in the spec's table.
    pub stage: u32,
    /// The stage's trigger frequency.
    pub trigger: Frequency,
    /// When frequency crossed the trigger.
    pub triggered_at: Duration,
    /// When the block actually disconnected (trigger + action delay).
    pub actioned_at: Duration,
    /// The demand block disconnected.
    pub block: Power,
}

/// One response service's delivery timeline, aligned with the trace.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceTimeline {
    /// Service label from the spec.
    pub name: String,
    /// Delivered power at each trace point.
    pub delivered: Vec<Power>,
}

/// The spec's era limits checked against the simulated event
/// (reporting only; docs/04: era-dependent limits are inputs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimitReport {
    /// Whether the steepest 1-s mean RoCoF exceeded the era's
    /// loss-of-mains relay limit (`None` if the spec carries no limit).
    pub rocof_relay_exceeded: Option<bool>,
    /// Whether frequency fell below the statutory floor.
    pub statutory_floor_breached: Option<bool>,
}

/// The complete result of one simulated event. Deterministic: identical
/// specs produce bit-identical results (ADR-5).
#[derive(Debug, Clone, PartialEq)]
pub struct EventResult {
    /// Minimum frequency over the event.
    pub nadir: Frequency,
    /// When the nadir occurred.
    pub nadir_at: Duration,
    /// The first arrest: frequency at the first local minimum of the
    /// descent (deviation > 0.05 Hz), `None` if the trajectory never
    /// turns within the window — the swing-physics discriminator (T2).
    pub first_arrest: Option<Frequency>,
    /// When the first arrest occurred.
    pub first_arrest_at: Option<Duration>,
    /// Mean RoCoF over the spec's pinned window (signed; negative =
    /// falling), `None` without a window.
    pub rocof_window_mean: Option<Rocof>,
    /// The steepest (most negative) 1-second mean RoCoF of the event —
    /// what a 1-s loss-of-mains relay window would see.
    pub steepest_1s_rocof: Rocof,
    /// LFDD stages that operated, in operation order.
    pub lfdd_actions: Vec<LfddAction>,
    /// Per-service delivery timelines (docs/03 stability outputs).
    pub response_timelines: Vec<ServiceTimeline>,
    /// Era-limit report, when the spec carries limits.
    pub limit_report: Option<LimitReport>,
    /// The frequency trace, one point per integrator step (t, f).
    trace: Vec<(Duration, Frequency)>,
}

impl EventResult {
    /// The frequency trace, one `(time, frequency)` point per
    /// integrator step, t = 0 first.
    #[must_use]
    pub fn trace(&self) -> &[(Duration, Frequency)] {
        &self.trace
    }

    /// Frequency at an arbitrary time, linearly interpolated between
    /// trace points (clamped to the trace ends).
    #[must_use]
    pub fn frequency_at(&self, at: Duration) -> Frequency {
        interpolate(&self.trace, at)
    }
}

/// Linear interpolation over a trace (clamped to the ends).
fn interpolate(trace: &[(Duration, Frequency)], at: Duration) -> Frequency {
    let t = at.as_seconds();
    match trace.binary_search_by(|(time, _)| time.as_seconds().total_cmp(&t)) {
        Ok(index) => trace[index].1,
        Err(0) => trace[0].1,
        Err(index) if index >= trace.len() => trace[trace.len() - 1].1,
        Err(index) => {
            let (t0, f0) = trace[index - 1];
            let (t1, f1) = trace[index];
            let fraction = (t - t0.as_seconds()) / (t1.as_seconds() - t0.as_seconds());
            Frequency::hertz(f0.as_hertz() + fraction * (f1.as_hertz() - f0.as_hertz()))
        }
    }
}

/// Mutable per-run bookkeeping for threshold-triggered elements.
struct EventState {
    /// Per LFDD stage: `Some(action time)` once triggered.
    lfdd_pending: Vec<Option<f64>>,
    /// Per LFDD stage: `Some(action record)` once actioned.
    lfdd_done: Vec<Option<LfddAction>>,
    /// Per response service: activation time (0 for dynamic services,
    /// `Some(t)` for static ones once triggered).
    activations: Vec<Option<f64>>,
    /// Total demand disconnected by LFDD so far (GW).
    disconnected_gw: f64,
}

/// Simulate one loss-of-infeed event.
pub fn simulate(spec: &EventSpec) -> Result<EventResult, GridError> {
    let f0 = spec.f0.as_hertz();
    let e_gva_s = spec.inertia.as_gigavolt_ampere_seconds();
    let demand_gw = spec.demand.as_gigawatts();
    let damping_pct = spec.load_damping.as_percent_of_demand_per_hertz();
    let dt = spec.timestep.as_seconds();
    let steps = (spec.duration.as_seconds() / dt).round() as usize;

    let mut state = EventState {
        lfdd_pending: vec![None; spec.lfdd.as_ref().map_or(0, |l| l.stages.len())],
        lfdd_done: vec![None; spec.lfdd.as_ref().map_or(0, |l| l.stages.len())],
        activations: spec
            .responses
            .iter()
            .map(|service| match service.shape {
                // Dynamic services are active from the event start;
                // their delay/ramp envelope models delivery lag.
                ResponseShape::Dynamic { .. } => Some(0.0),
                ResponseShape::Static { .. } => None,
            })
            .collect(),
        disconnected_gw: 0.0,
    };

    // deficit(t, f) in GW given the committed trigger state.
    let deficit = |t: f64, f: f64, state: &EventState| -> f64 {
        let mut deficit = 0.0;
        for loss in &spec.losses {
            if loss.at.as_seconds() <= t {
                deficit += loss.power.as_gigawatts();
            }
        }
        for (service, activation) in spec.responses.iter().zip(&state.activations) {
            deficit -= delivered_gw(service, *activation, t, f0 - f);
        }
        deficit -= state.disconnected_gw;
        // Load damping on the remaining (post-LFDD) demand;
        // under-frequency relieves load, over-frequency adds.
        deficit -= damping_pct / 100.0 * (f0 - f) * (demand_gw - state.disconnected_gw);
        deficit
    };

    let mut trace: Vec<(Duration, Frequency)> = Vec::with_capacity(steps + 1);
    let mut timelines: Vec<Vec<Power>> = vec![Vec::with_capacity(steps + 1); spec.responses.len()];
    let mut f = f0;

    let record = |trace: &mut Vec<(Duration, Frequency)>,
                  timelines: &mut [Vec<Power>],
                  state: &EventState,
                  t: f64,
                  f: f64| {
        trace.push((Duration::from_seconds(t), Frequency::hertz(f)));
        for ((service, activation), timeline) in spec
            .responses
            .iter()
            .zip(&state.activations)
            .zip(timelines.iter_mut())
        {
            timeline.push(Power::gigawatts(delivered_gw(
                service,
                *activation,
                t,
                f0 - f,
            )));
        }
    };

    record(&mut trace, &mut timelines, &state, 0.0, f);
    for step in 0..steps {
        let t = step as f64 * dt;
        // Heun (RK2): predictor with the committed trigger state, then
        // trapezoidal corrector.
        let k1 = -deficit(t, f, &state) * f0 * f0 / (2.0 * e_gva_s * f);
        let f_predicted = f + dt * k1;
        let k2 = -deficit(t + dt, f_predicted, &state) * f0 * f0 / (2.0 * e_gva_s * f_predicted);
        f += dt / 2.0 * (k1 + k2);
        let t_next = (step + 1) as f64 * dt;

        // Threshold detection on the committed state, then delayed
        // actions, both latched.
        update_triggers(spec, &mut state, t_next, f);

        record(&mut trace, &mut timelines, &state, t_next, f);
    }

    // ---------------- derived measurements ----------------
    let (nadir_index, nadir) =
        trace
            .iter()
            .enumerate()
            .fold((0usize, f64::INFINITY), |(best_i, best_f), (i, (_, f))| {
                if f.as_hertz() < best_f {
                    (i, f.as_hertz())
                } else {
                    (best_i, best_f)
                }
            });

    // First arrest: the first trace point that is a local minimum of a
    // genuine descent (deviation > 0.05 Hz — skips numerical wiggle
    // before anything has happened).
    let mut first_arrest = None;
    let mut first_arrest_at = None;
    for i in 1..trace.len().saturating_sub(1) {
        let (prev, here, next) = (
            trace[i - 1].1.as_hertz(),
            trace[i].1.as_hertz(),
            trace[i + 1].1.as_hertz(),
        );
        if here < prev && next > here && (f0 - here) > 0.05 {
            first_arrest = Some(trace[i].1);
            first_arrest_at = Some(trace[i].0);
            break;
        }
    }

    let rocof_window_mean = spec.rocof_window.as_ref().map(|window| {
        let start = window.start;
        let end = Duration::from_seconds(start.as_seconds() + window.duration.as_seconds());
        (interpolate(&trace, end) - interpolate(&trace, start)) / window.duration
    });

    // Steepest 1-s mean RoCoF (what a 1-s relay window sees).
    let steps_per_second = (1.0 / dt).round() as usize;
    let mut steepest = 0.0f64;
    if steps_per_second >= 1 {
        for i in 0..trace.len().saturating_sub(steps_per_second) {
            let slope = trace[i + steps_per_second].1.as_hertz() - trace[i].1.as_hertz();
            if slope < steepest {
                steepest = slope;
            }
        }
    }
    let steepest_1s_rocof = Rocof::hertz_per_second(steepest);

    let lfdd_actions: Vec<LfddAction> = {
        let mut actions: Vec<LfddAction> = state.lfdd_done.iter().flatten().cloned().collect();
        actions.sort_by(|a, b| {
            a.actioned_at
                .as_seconds()
                .total_cmp(&b.actioned_at.as_seconds())
        });
        actions
    };

    let limit_report = spec.limits.as_ref().map(|limits| LimitReport {
        rocof_relay_exceeded: limits
            .rocof_relay
            .map(|relay| steepest.abs() > relay.as_hertz_per_second().abs()),
        statutory_floor_breached: limits.statutory_floor.map(|floor| nadir < floor.as_hertz()),
    });

    Ok(EventResult {
        nadir: Frequency::hertz(nadir),
        nadir_at: trace[nadir_index].0,
        first_arrest,
        first_arrest_at,
        rocof_window_mean,
        steepest_1s_rocof,
        lfdd_actions,
        response_timelines: spec
            .responses
            .iter()
            .zip(timelines)
            .map(|(service, delivered)| ServiceTimeline {
                name: service.name.clone(),
                delivered,
            })
            .collect(),
        limit_report,
        trace,
    })
}

/// Threshold bookkeeping at a committed step boundary: LFDD triggers
/// and delayed actions, static-service activation. All latched.
fn update_triggers(spec: &EventSpec, state: &mut EventState, t: f64, f: f64) {
    if let Some(lfdd) = &spec.lfdd {
        for (index, stage) in lfdd.stages.iter().enumerate() {
            if state.lfdd_pending[index].is_none() && f < stage.frequency.as_hertz() {
                state.lfdd_pending[index] = Some(t + lfdd.action_delay.as_seconds());
            }
            if state.lfdd_done[index].is_none()
                && let Some(action_at) = state.lfdd_pending[index]
                && t >= action_at
            {
                state.disconnected_gw += stage.block.as_gigawatts();
                state.lfdd_done[index] = Some(LfddAction {
                    stage: index as u32 + 1,
                    trigger: stage.frequency,
                    triggered_at: Duration::from_seconds(
                        action_at - lfdd.action_delay.as_seconds(),
                    ),
                    actioned_at: Duration::from_seconds(t),
                    block: stage.block,
                });
            }
        }
    }
    for (service, activation) in spec.responses.iter().zip(&mut state.activations) {
        if activation.is_none()
            && let ResponseShape::Static { trigger } = service.shape
            && f < trigger.as_hertz()
        {
            *activation = Some(t);
        }
    }
}

/// A response service's delivered power (GW) at time t and deviation
/// Δf, given its activation time (`None` = not activated).
fn delivered_gw(
    service: &crate::spec::ResponseService,
    activation: Option<f64>,
    t: f64,
    delta_f: f64,
) -> f64 {
    let Some(t_active) = activation else {
        return 0.0;
    };
    let since = t - t_active;
    let delay = service.delay.as_seconds();
    if since < delay {
        return 0.0;
    }
    // Ramp envelope: 0 → 1 over [delay, delay + ramp].
    let ramp = service.ramp.as_seconds();
    let mut envelope = if ramp > 0.0 {
        ((since - delay) / ramp).min(1.0)
    } else {
        1.0
    };
    // Sustain limit and rundown.
    if let Some(sustain) = service.sustain {
        let sustain = sustain.as_seconds();
        if since >= sustain {
            let rundown = service.rundown.map_or(0.0, |r| r.as_seconds());
            envelope = if rundown > 0.0 && since < sustain + rundown {
                envelope * (1.0 - (since - sustain) / rundown)
            } else {
                0.0
            };
        }
    }
    // Frequency shape: droop for dynamic services (low-frequency
    // response only — nothing delivered at or above nominal); static
    // services deliver their envelope once activated.
    let shape = match service.shape {
        ResponseShape::Dynamic {
            droop_full_deviation,
        } => (delta_f / droop_full_deviation.as_hertz()).clamp(0.0, 1.0),
        ResponseShape::Static { .. } => 1.0,
    };
    service.power.as_gigawatts() * service.delivery_factor.value() * envelope * shape
}
