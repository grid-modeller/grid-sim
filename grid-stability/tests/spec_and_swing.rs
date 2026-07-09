//! Unit tests: strict event-spec parsing/validation and the isolated
//! swing-model behaviours the Aug-2019 acceptance run exercises only in
//! combination (static services, LFDD latching, envelope rundown,
//! damping equilibrium).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use grid_core::GridError;
use grid_core::units::{Damping, Duration, Frequency, Inertia, PerUnit, Power};
use grid_stability::{
    EventSpec, InfeedLoss, LfddScheme, LfddStage, ResponseService, ResponseShape, simulate,
};

const MINIMAL: &str = r#"
schema = "stability-event-v1"
name = "minimal"
f0_hz = 50.0
demand_gw = 29.0
inertia_gva_s = 210.0
load_damping_percent_per_hz = 1.0
duration_s = 30.0

[[losses]]
name = "trip"
mw = 1000
at_s = 0.0
"#;

// ---------------------------------------------------------------------
// Spec parsing.
// ---------------------------------------------------------------------

#[test]
fn minimal_spec_parses_with_documented_defaults() {
    let spec = EventSpec::from_toml_str(MINIMAL).unwrap();
    assert_eq!(spec.f0, Frequency::hertz(50.0));
    assert_eq!(spec.timestep, Duration::from_seconds(0.010));
    assert_eq!(spec.losses.len(), 1);
    assert_eq!(spec.losses[0].power, Power::megawatts(1000.0));
    assert!(spec.responses.is_empty());
    assert!(spec.lfdd.is_none());
}

#[test]
fn wrong_schema_string_is_rejected() {
    let toml = MINIMAL.replace("stability-event-v1", "stability-event-v9");
    let err = EventSpec::from_toml_str(&toml).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidEventSpec { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("stability-event-v1"));
}

#[test]
fn unknown_fields_are_rejected() {
    let toml = MINIMAL.replace("demand_gw = 29.0", "demand_gw = 29.0\nfrobnicate = 1");
    let err = EventSpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("frobnicate"), "err: {err}");
}

#[test]
fn semantic_validation_rejects_incoherent_specs() {
    for (needle, replacement, what) in [
        ("inertia_gva_s = 210.0", "inertia_gva_s = 0.0", "inertia"),
        ("duration_s = 30.0", "duration_s = -1.0", "duration"),
        ("mw = 1000", "mw = -100", "negative loss"),
    ] {
        let toml = MINIMAL.replace(needle, replacement);
        assert!(
            EventSpec::from_toml_str(&toml).is_err(),
            "{what} should be rejected"
        );
    }
    // Dynamic service without a droop parameter.
    let toml = format!(
        "{MINIMAL}\n[[responses]]\nname = \"p\"\nkind = \"dynamic\"\nmw = 100\ndelay_s = 1.0\nramp_s = 5.0\n"
    );
    let err = EventSpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("droop"), "err: {err}");
    // Static service without a trigger.
    let toml = format!(
        "{MINIMAL}\n[[responses]]\nname = \"s\"\nkind = \"static\"\nmw = 100\ndelay_s = 1.0\nramp_s = 5.0\n"
    );
    let err = EventSpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("trigger"), "err: {err}");
    // LFDD stages out of order.
    let toml = format!(
        "{MINIMAL}\n[lfdd]\naction_delay_s = 0.3\nstages = [{{ hz = 48.75, mw = 100 }}, {{ hz = 48.8, mw = 100 }}]\n"
    );
    let err = EventSpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("descending"), "err: {err}");
    // Delivery factor above 1.
    let toml = format!(
        "{MINIMAL}\n[[responses]]\nname = \"p\"\nkind = \"dynamic\"\nmw = 100\ndelivery_factor = 1.2\ndroop_full_deviation_hz = 0.5\ndelay_s = 1.0\nramp_s = 5.0\n"
    );
    let err = EventSpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("delivery_factor"), "err: {err}");
}

#[test]
fn load_of_missing_file_names_the_path() {
    let err = EventSpec::load(std::path::Path::new("/nonexistent/event.toml")).unwrap_err();
    assert!(err.to_string().contains("/nonexistent/event.toml"));
}

// ---------------------------------------------------------------------
// Swing behaviours in isolation.
// ---------------------------------------------------------------------

/// A bare spec builder for behaviour tests.
fn bare(loss_mw: f64, damping: f64, duration_s: f64) -> EventSpec {
    EventSpec {
        name: "behaviour".to_owned(),
        f0: Frequency::hertz(50.0),
        demand: Power::gigawatts(29.0),
        inertia: Inertia::gigavolt_ampere_seconds(210.0),
        load_damping: Damping::percent_of_demand_per_hertz(damping),
        duration: Duration::from_seconds(duration_s),
        timestep: Duration::from_seconds(0.010),
        losses: vec![InfeedLoss {
            name: "trip".to_owned(),
            power: Power::megawatts(loss_mw),
            at: Duration::from_seconds(0.0),
        }],
        responses: vec![],
        lfdd: None,
        limits: None,
        rocof_window: None,
    }
}

#[test]
fn damping_alone_settles_at_the_analytic_equilibrium() {
    // 1,000 MW loss against 2 %/Hz of 29 GW = 0.58 GW/Hz: equilibrium
    // deviation = 1.0 / 0.58 = 1.7241 Hz. τ ≈ 2E/(f₀·D) ≈ 7.2 s, so
    // 120 s ≥ 16τ: settled to well under 1 mHz.
    let spec = bare(1000.0, 2.0, 120.0);
    let result = simulate(&spec).unwrap();
    let settled = result.trace().last().unwrap().1.as_hertz();
    let expected = 50.0 - 1.0 / (0.02 * 29.0);
    assert!(
        (settled - expected).abs() < 1e-3,
        "settled {settled} Hz vs analytic equilibrium {expected} Hz"
    );
}

#[test]
fn static_service_triggers_latched_and_arrests_the_fall() {
    // A static 1,200 MW service triggered at 49.7 Hz against a
    // 1,000 MW loss with no damping: frequency falls to the trigger,
    // the service steps in (after its delay+ramp) and, delivering more
    // than the loss, drives frequency back UP through the trigger —
    // where it must stay latched (delivery continues) rather than
    // chattering off.
    let mut spec = bare(1000.0, 0.0, 60.0);
    spec.responses = vec![ResponseService {
        name: "static_reserve".to_owned(),
        power: Power::megawatts(1200.0),
        delivery_factor: PerUnit::new(1.0),
        shape: ResponseShape::Static {
            trigger: Frequency::hertz(49.7),
        },
        delay: Duration::from_seconds(0.5),
        ramp: Duration::from_seconds(1.0),
        sustain: None,
        rundown: None,
    }];
    let result = simulate(&spec).unwrap();
    // Nadir just below the trigger (delay + ramp deep), then recovery.
    let nadir = result.nadir.as_hertz();
    assert!(nadir < 49.7 && nadir > 49.4, "nadir {nadir}");
    let end = result.trace().last().unwrap().1.as_hertz();
    assert!(end > 49.9, "latched service should keep recovering: {end}");
    // The delivery timeline shows full delivery at the end (latched,
    // even though frequency has recovered above the trigger).
    let timeline = &result.response_timelines[0];
    assert_eq!(timeline.name, "static_reserve");
    assert!((timeline.delivered.last().unwrap().as_gigawatts() - 1.2).abs() < 1e-12);
}

#[test]
fn sustain_limit_runs_delivery_down_and_frequency_falls_again() {
    // A service that fully covers the loss but expires at 10 s (2 s
    // rundown): frequency arrests, then resumes falling once delivery
    // runs down — the double-dip shape of an expiring service.
    let mut spec = bare(1000.0, 0.0, 30.0);
    spec.responses = vec![ResponseService {
        name: "expiring".to_owned(),
        power: Power::megawatts(1000.0),
        delivery_factor: PerUnit::new(1.0),
        shape: ResponseShape::Dynamic {
            droop_full_deviation: Frequency::hertz(0.1),
        },
        delay: Duration::from_seconds(0.0),
        ramp: Duration::from_seconds(1.0),
        sustain: Some(Duration::from_seconds(10.0)),
        rundown: Some(Duration::from_seconds(2.0)),
    }];
    let result = simulate(&spec).unwrap();
    let f_at = |t: f64| result.frequency_at(Duration::from_seconds(t)).as_hertz();
    // Near-arrested by t = 8 s…
    let before_expiry = f_at(8.0);
    // …falling again well after the rundown completes.
    let after_expiry = f_at(20.0);
    assert!(
        before_expiry - after_expiry > 0.3,
        "delivery rundown must resume the fall: f(8s) = {before_expiry}, f(20s) = {after_expiry}"
    );
    // Delivery at 30 s is exactly zero.
    assert_eq!(
        result.response_timelines[0]
            .delivered
            .last()
            .unwrap()
            .as_gigawatts(),
        0.0
    );
}

#[test]
fn lfdd_stages_trip_in_order_with_the_action_delay_and_latch() {
    // No response at all: a 2,000 MW loss walks down through several
    // stages; each acts one delay after its trigger and stays off.
    let mut spec = bare(2000.0, 1.0, 120.0);
    spec.lfdd = Some(LfddScheme {
        action_delay: Duration::from_seconds(0.4),
        stages: vec![
            LfddStage {
                frequency: Frequency::hertz(48.8),
                block: Power::megawatts(931.0),
            },
            LfddStage {
                frequency: Frequency::hertz(48.75),
                block: Power::megawatts(1450.0),
            },
        ],
    });
    let result = simulate(&spec).unwrap();
    assert_eq!(result.lfdd_actions.len(), 2);
    let stage1 = &result.lfdd_actions[0];
    assert_eq!(stage1.stage, 1);
    // Action exactly one delay after the trigger (quantised to the
    // 10 ms step).
    let gap = stage1.actioned_at.as_seconds() - stage1.triggered_at.as_seconds();
    assert!((gap - 0.4).abs() < 0.011, "action delay gap {gap}");
    // 931 + 1450 MW disconnected + damping (~0.29 GW/Hz × deviation)
    // against 2,000 MW lost: recovery — and no un-latching wobble: the
    // frequency at the end exceeds both triggers with blocks still off
    // (equilibrium ≈ 50 − (2.0 − 2.381)/0.29 above nominal, clamped by
    // over-frequency damping sign).
    let end = result.trace().last().unwrap().1.as_hertz();
    assert!(end > 48.8, "end {end}");
}

#[test]
fn losses_sum_and_stage_in_time() {
    // Two staged trips over a 7 s window: the RoCoF window over the
    // first second sees only the first 500 MW (linearised
    // 0.0595 Hz/s); the steepest 1-s window sits right after the
    // second trip, seeing the combined 1,500 MW (linearised
    // 0.1786 Hz/s, ×f₀/f ≈ ×1.007 at the ~49.65 Hz it happens at —
    // the kinetic-exact form steepens slightly as f falls).
    let mut spec = bare(500.0, 0.0, 7.0);
    spec.losses.push(InfeedLoss {
        name: "second".to_owned(),
        power: Power::megawatts(1000.0),
        at: Duration::from_seconds(5.0),
    });
    spec.rocof_window = Some(grid_stability::RocofWindow {
        start: Duration::from_seconds(0.0),
        duration: Duration::from_seconds(1.0),
    });
    let result = simulate(&spec).unwrap();
    let window = result.rocof_window_mean.unwrap().as_hertz_per_second();
    assert!(
        (window.abs() - 0.0595).abs() < 0.001,
        "first-second RoCoF {window}"
    );
    let steepest = result.steepest_1s_rocof.as_hertz_per_second();
    assert!(
        (steepest.abs() - 0.180).abs() < 0.003,
        "steepest 1-s RoCoF {steepest}"
    );
}

#[test]
fn era_limits_are_reported_not_enforced() {
    let mut spec = bare(1000.0, 0.0, 10.0);
    spec.limits = Some(grid_stability::OperatingLimits {
        rocof_relay: Some(grid_core::units::Rocof::hertz_per_second(0.125)),
        statutory_floor: Some(Frequency::hertz(49.5)),
    });
    let result = simulate(&spec).unwrap();
    let report = result.limit_report.unwrap();
    // 1,000 MW on 210 GVA·s ≈ 0.119 Hz/s < 0.125 relay limit; the
    // un-arrested fall breaches 49.5 within the window.
    assert_eq!(report.rocof_relay_exceeded, Some(false));
    assert_eq!(report.statutory_floor_breached, Some(true));
}
