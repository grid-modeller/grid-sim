//! Stage 6 acceptance: the single-machine constant-ΔP case matches the
//! closed-form swing solution (docs/04 Stage 6, first acceptance test).
//!
//! With a constant power deficit `P`, no damping, no response and no
//! LFDD, the kinetic-energy-exact swing equation
//! `d/dt [E·(f/f₀)²] = −P` integrates in closed form to
//!
//! `f(t) = f₀ · sqrt(1 − P·t/E)`.
//!
//! The engine integrates with Heun's method (RK2) at a fixed 10 ms step
//! (see `grid_stability::swing` for the stability/accuracy discussion).
//! The pinned integrator tolerance is **1 µHz (1e-6 Hz)** over 60 s —
//! measured global error is ~1e-9 Hz (second-order method, smooth RHS),
//! so the pin holds three orders of magnitude of slack without being
//! loose enough to hide a method regression to first order (~1e-4 Hz).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use grid_core::units::{Damping, Duration, Frequency, Inertia, Power};
use grid_stability::{EventSpec, InfeedLoss, simulate};

/// The analytic no-feedback event: 1,000 MW lost at t = 0 against
/// 210 GVA·s, nothing else acting.
fn analytic_spec() -> EventSpec {
    EventSpec {
        name: "analytic-constant-dp".to_owned(),
        f0: Frequency::hertz(50.0),
        demand: Power::gigawatts(29.0),
        inertia: Inertia::gigavolt_ampere_seconds(210.0),
        load_damping: Damping::percent_of_demand_per_hertz(0.0),
        duration: Duration::from_seconds(60.0),
        timestep: Duration::from_seconds(0.010),
        losses: vec![InfeedLoss {
            name: "constant".to_owned(),
            power: Power::megawatts(1000.0),
            at: Duration::from_seconds(0.0),
        }],
        responses: vec![],
        lfdd: None,
        limits: None,
        rocof_window: None,
    }
}

#[test]
fn constant_dp_matches_closed_form_within_integrator_tolerance() {
    let spec = analytic_spec();
    let result = simulate(&spec).unwrap();

    let f0 = 50.0;
    let p_gw = 1.0;
    let e_gva_s = 210.0;
    let mut worst = 0.0_f64;
    for (t, f) in result.trace() {
        let expected = f0 * (1.0 - p_gw * t.as_seconds() / e_gva_s).sqrt();
        let error = (f.as_hertz() - expected).abs();
        worst = worst.max(error);
    }
    assert!(
        worst <= 1e-6,
        "integrator error vs closed form: worst {worst:.3e} Hz > pinned 1e-6 Hz"
    );
    // The trace must actually span the event (guard against a vacuous
    // pass over an empty trace).
    assert_eq!(result.trace().len(), 6_001);
}

#[test]
fn initial_rocof_matches_f0_dp_over_2e() {
    // The linearised initial RoCoF f₀·ΔP/(2E) = 50 × 1.0 / 420 =
    // 0.11905 Hz/s. At t = 0 (f = f₀) the exact form coincides with the
    // linearisation, so the engine's measured RoCoF over a short window
    // from t = 0 must sit just above it in magnitude (f falls, so
    // |df/dt| grows slightly under the exact form — within 0.5 % over
    // 1 s).
    let mut spec = analytic_spec();
    spec.rocof_window = Some(grid_stability::RocofWindow {
        start: Duration::from_seconds(0.0),
        duration: Duration::from_seconds(1.0),
    });
    let result = simulate(&spec).unwrap();
    let rocof = result.rocof_window_mean.unwrap();
    let linearised = 50.0 * 1.0 / (2.0 * 210.0);
    let ratio = rocof.as_hertz_per_second().abs() / linearised;
    assert!(
        (0.995..=1.005).contains(&ratio),
        "windowed RoCoF {} Hz/s vs linearised {linearised} Hz/s (ratio {ratio})",
        rocof.as_hertz_per_second()
    );
    // Signed: falling frequency reads negative.
    assert!(rocof.as_hertz_per_second() < 0.0);
}
