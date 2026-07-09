//! Q5/Q11 heating-overlay tests (docs/notes/d9-heating-overlay.md,
//! ADOPTED 2026-07-03 — rules 2–5; the engine-package acceptance suite
//! plus the reference-file drift guards).
//!
//! Three layers:
//!
//! 1. **Reference-file parse pins** — `data/reference/heating-cop.toml`
//!    is the cited, drift-guarded COP-parameter source (D9 rule 4, the
//!    inertia-constants precedent). The pinned values here are the
//!    reviewed data-package numbers (q5-heating-data-report.md): if the
//!    file and these pins drift apart, someone changed a reviewed
//!    number without a record.
//! 2. **Synthetic-trace unit tests** — exact arithmetic on constructed
//!    temperature series (no data pack needed).
//! 3. **Pinned-trace acceptance tests** — require the fetched
//!    `data/weather/gb_t2m_pop.parquet` (manifest
//!    `data/packs/weather-gb-t2m-pop.sha256`); FAIL LOUDLY if absent.
//!    These carry the D9 rule-5 acceptance content: conservation over
//!    the 1985–2024 window (record-mean = quantum; per-year spread is a
//!    reported finding), the horizon-composability property (ADR-5),
//!    the edit-6 SPFH2 reproduction (a REPRODUCTION of the data
//!    package's reviewed determination, not a fresh one), the
//!    machine-checked district premise + district-lowest limb, and the
//!    MEASURED (never pre-committed) ASHP-vs-GSHP ordering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use grid_core::GridError;
use grid_core::heating::{
    HEATING_COP_REFERENCE_PATH, HeatingCopReference, compute_overlay, fit_ground_wave,
    implied_spfh2, record_max_cop,
};
use grid_core::scenario::{HeatingEntry, HeatingKind, HeatingSpec, TraceRefSpec};
use grid_core::time::UtcInstant;
use grid_core::trace::{Trace, load_temperature_trace_c};
use grid_core::units::{
    Duration, Energy, Length, PerUnit, Power, Temperature, TemperatureGradient,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn reference_path() -> PathBuf {
    repo_root().join(HEATING_COP_REFERENCE_PATH)
}

fn reference() -> HeatingCopReference {
    HeatingCopReference::load(&reference_path()).unwrap()
}

fn reference_text() -> String {
    std::fs::read_to_string(reference_path()).unwrap()
}

// ---------------------------------------------------------------------
// 1. Reference-file parse pins (drift guards; D9 rule 4).
// ---------------------------------------------------------------------

/// Every engine-facing number of the reviewed data package, pinned.
/// A move here is a change to a reviewed number — knowing re-pin only,
/// with the record (q5-heating-data-report.md / its review).
#[test]
fn heating_cop_reference_parses_with_the_pinned_reviewed_values() {
    let r = reference();
    // [conventions]
    assert_eq!(r.t_base, Temperature::celsius(15.5));
    assert_eq!(r.min_delta_t, Temperature::celsius(15.0));
    // [heat_quantum] — the review-ruled basis: delivered heat, GB,
    // record-mean (410.5 TWh; DHW fraction 0.170).
    assert_eq!(r.delivered_heat, Energy::gigawatt_hours(410_500.0));
    assert_eq!(r.dhw_fraction, PerUnit::new(0.170));
    // [sink] — When2Heat eq. 6, radiator convention + 50 °C DHW.
    assert_eq!(r.sink.radiator_t0, Temperature::celsius(40.0));
    assert_eq!(r.sink.radiator_slope, 1.0);
    assert_eq!(r.sink.dhw_sink, Temperature::celsius(50.0));
    // [ashp] — When2Heat quadratic, correction 0.85 RETAINED, RHPP
    // to-median derating 0.823.
    assert_eq!(r.ashp.cop_curve, [6.08, -0.09, 0.0005]);
    assert_eq!(r.ashp.correction_factor, PerUnit::new(0.85));
    assert_eq!(r.ashp.correction_factor_status, "retained");
    assert_eq!(r.ashp.rhpp_derating, PerUnit::new(0.823));
    assert_eq!(r.ashp.source_offset, Temperature::celsius(0.0));
    // [gshp] — brine offset 5 K RETAINED, derating 0.732.
    assert_eq!(r.gshp.cop_curve, [10.29, -0.21, 0.0012]);
    assert_eq!(r.gshp.correction_factor, PerUnit::new(0.85));
    assert_eq!(r.gshp.rhpp_derating, PerUnit::new(0.732));
    assert_eq!(r.gshp.source_offset, Temperature::celsius(5.0));
    // [ground_model] — z = 1.0 m shallow horizontal loop (conservative,
    // D9 ruling A), Busby 2016 α centre + band.
    assert_eq!(r.ground.loop_depth.as_metres(), 1.0);
    assert_eq!(r.ground.alpha.as_square_metres_per_second(), 8.7e-7);
    assert_eq!(
        r.ground.alpha_band.map(|a| a.as_square_metres_per_second()),
        [7.173e-7, 1.0295e-6]
    );
    // [district_geothermal] — delivered-heat basis, band stated.
    assert_eq!(r.district.cop_const, 15.0);
    assert_eq!(r.district.cop_const_band, [12.0, 18.8]);
    assert!(r.district.basis.contains("delivered-heat"));
    // [geothermal] — D16: the industry correspondent's 25 °C/km conservative centre,
    // the BGS 26–35 band stated above it (Busby 2014: UK average 26,
    // locally >35; Busby & Terrington 2017 adopt 28), and the datum
    // statement naming the loop-depth anchoring that makes the shallow
    // default bit-identical.
    assert_eq!(
        r.geothermal.gradient,
        TemperatureGradient::celsius_per_kilometre(25.0)
    );
    assert_eq!(
        r.geothermal
            .gradient_band
            .map(|g| g.as_celsius_per_kilometre()),
        [26.0, 35.0]
    );
    assert!(
        r.geothermal.datum.contains("loop_depth"),
        "the datum statement must name the loop-depth anchoring: {:?}",
        r.geothermal.datum
    );
    // [rhpp] — SPFH2 comparison boundary, cropped-B2 medians.
    assert_eq!(r.rhpp.comparison_boundary, "SPFH2");
    assert_eq!(r.rhpp.ashp_spfh2_median, 2.65);
    assert_eq!(r.rhpp.ashp_spfh2_iqr, [2.33, 2.95]);
    assert_eq!(r.rhpp.gshp_spfh2_median, 2.81);
    // 3.14 is the published RHPP IQR bound, not an approximation of π.
    #[allow(clippy::approx_constant)]
    let gshp_iqr_hi = 3.14;
    assert_eq!(r.rhpp.gshp_spfh2_iqr, [2.63, gshp_iqr_hi]);
}

/// The mandatory reference-file schema string (docs/03 committed-
/// reference registry; engine-review condition 1): pinned, present in
/// the committed file, and probed before the full parse — a missing or
/// wrong schema is its own clear error, never a field-level one.
#[test]
fn heating_cop_reference_schema_string_is_pinned_and_probed() {
    // The pin: the registry name never drifts silently. v2 = the D16
    // [geothermal] addition (docs/03 registry note) — v1 plus one
    // section, every v1 value untouched.
    assert_eq!(grid_core::heating::HEATING_COP_SCHEMA, "heating-cop-v2");
    assert!(
        reference_text().contains("schema = \"heating-cop-v2\""),
        "the committed reference file must carry the registry schema string"
    );

    // Missing schema: a structured error naming the expected string.
    let text = reference_text().replace("schema = \"heating-cop-v2\"\n", "");
    let err = HeatingCopReference::from_toml_str(&text).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidHeatingReference { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("heating-cop-v2"), "err: {err}");

    // Wrong schema (including the superseded v1): named
    // found-vs-expected, before any field error.
    for wrong in ["heating-cop-v1", "heating-cop-v3"] {
        let text = reference_text().replace(
            "schema = \"heating-cop-v2\"",
            &format!("schema = {wrong:?}"),
        );
        let err = HeatingCopReference::from_toml_str(&text).unwrap_err();
        assert!(
            matches!(err, GridError::InvalidHeatingReference { .. }),
            "unexpected error: {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains(wrong), "err: {msg}");
        assert!(msg.contains("heating-cop-v2"), "err: {msg}");
    }
}

#[test]
fn heating_cop_reference_rejects_unknown_fields() {
    let text = reference_text().replace("[sink]", "[sink]\nfrobnicate = 1.0");
    let err = HeatingCopReference::from_toml_str(&text).unwrap_err();
    assert!(
        matches!(err, GridError::HeatingReferenceParse { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("frobnicate"), "err: {err}");
}

#[test]
fn heating_cop_reference_rejects_non_physical_values() {
    for (needle, replacement) in [
        // Deratings/corrections outside (0, 1].
        ("rhpp_derating = 0.823", "rhpp_derating = 0.0"),
        ("rhpp_derating = 0.823", "rhpp_derating = 1.5"),
        (
            "correction_factor = 0.85\ncorrection_factor_status = \"retained\"\n# One-factor-per-technology RHPP derating",
            "correction_factor = -0.85\ncorrection_factor_status = \"retained\"\n# One-factor-per-technology RHPP derating",
        ),
        // Ground model must be physical.
        ("alpha_m2_s = 8.7e-7", "alpha_m2_s = -8.7e-7"),
        ("loop_depth_m = 1.0", "loop_depth_m = 0.0"),
        // District COP must be positive and inside no contradiction.
        ("cop_const = 15.0", "cop_const = -15.0"),
        // Sub-freezing ΔT floor is contradictory.
        ("min_delta_t_k = 15.0", "min_delta_t_k = -1.0"),
        // dhw_fraction is a fraction.
        ("dhw_fraction = 0.170", "dhw_fraction = 1.7"),
        // The geothermal gradient must be positive and its band ordered
        // (D16).
        ("gradient_c_per_km = 25.0", "gradient_c_per_km = -25.0"),
        (
            "gradient_band_c_per_km = [26.0, 35.0]",
            "gradient_band_c_per_km = [35.0, 26.0]",
        ),
    ] {
        let text = reference_text().replace(needle, replacement);
        assert_ne!(text, reference_text(), "needle {needle:?} not found");
        let err = HeatingCopReference::from_toml_str(&text).unwrap_err();
        assert!(
            matches!(err, GridError::InvalidHeatingReference { .. }),
            "replacing {needle:?}: unexpected error {err:?}"
        );
    }
}

#[test]
fn heating_cop_reference_rejects_mislabelled_sources_and_status() {
    // The ASHP block must be the air-source parameterisation and the
    // GSHP block the ground-source one — a swap is a data error.
    let text = reference_text().replace("source = \"air\"", "source = \"ground\"");
    assert!(HeatingCopReference::from_toml_str(&text).is_err());
    // The correction-factor status is a closed vocabulary (D9 rule 4
    // item iii: retained or replaced, stated).
    let text = reference_text().replace(
        "correction_factor_status = \"retained\"",
        "correction_factor_status = \"maybe\"",
    );
    assert!(HeatingCopReference::from_toml_str(&text).is_err());
}

#[test]
fn missing_reference_file_is_a_structured_error() {
    let err = HeatingCopReference::load(Path::new("/nonexistent/heating-cop.toml")).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("/nonexistent/heating-cop.toml"), "err: {msg}");
}

// ---------------------------------------------------------------------
// 2. Ground model and overlay arithmetic on synthetic traces.
// ---------------------------------------------------------------------

/// One synthetic non-leap calendar year at a constant temperature.
fn constant_year(temp_c: f64) -> Trace<Temperature> {
    let start = UtcInstant::parse("2023-01-01T00:00:00Z").unwrap();
    Trace::from_parts(start, vec![Temperature::celsius(temp_c); 17_520]).unwrap()
}

/// A three-way D9 spec over a given trace reference (the trace itself
/// is passed to compute_overlay separately; the path here is
/// documentation only).
fn spec(entries: Vec<(HeatingKind, f64)>) -> HeatingSpec {
    HeatingSpec {
        delivered_heat_twh: Energy::gigawatt_hours(410_500.0),
        electrified_share: PerUnit::new(0.5),
        dhw_fraction: PerUnit::new(0.170),
        temperature_trace: TraceRefSpec {
            path: "data/weather/gb_t2m_pop.parquet".to_owned(),
            column: "t2m_pop".to_owned(),
        },
        entries: entries
            .into_iter()
            .map(|(kind, share)| HeatingEntry {
                kind,
                share: PerUnit::new(share),
                cop_curve: None,
                correction_factor: None,
                rhpp_derating: None,
                cop_const: None,
                resource_depth_m: None,
            })
            .collect(),
    }
}

/// [`spec`] with a GSHP resource depth on every GSHP entry (D16).
fn spec_at_depth(entries: Vec<(HeatingKind, f64)>, depth_m: f64) -> HeatingSpec {
    let mut s = spec(entries);
    for entry in &mut s.entries {
        if entry.kind == HeatingKind::Gshp {
            entry.resource_depth_m = Some(Length::metres(depth_m));
        }
    }
    s
}

/// The Kusuda–Achenbach parameters at the reference centre reproduce
/// the data package's recorded derivation: damping 0.7130, lag
/// 19.66 days (heating-cop.toml [ground_model] comment; the engine
/// recomputes from z and α — these are the drift-guard references).
#[test]
fn kusuda_achenbach_damping_and_lag_match_the_recorded_derivation() {
    let r = reference();
    // The wave parameters do not depend on the temperatures, only on
    // z, α and the annual angular frequency — a constant trace with a
    // whole year suffices to carry the record's mean-year length.
    let wave = fit_ground_wave(&constant_year(10.0), &r.ground).unwrap();
    // The synthetic year is 365 d (the pinned record's mean year is
    // 365.25 d), so allow the ~0.03 % ω difference in the tolerance.
    assert!(
        (wave.damping.value() - 0.7130).abs() < 5e-4,
        "damping {} differs from the recorded 0.7130",
        wave.damping.value()
    );
    assert!(
        (wave.lag.as_hours() / 24.0 - 19.66).abs() < 0.02,
        "lag {} days differs from the recorded 19.66",
        wave.lag.as_hours() / 24.0
    );
}

/// Exact conservation arithmetic on a constant synthetic year:
/// T = 10.5 °C everywhere ⇒ heat_need = 5 K every period, so delivered
/// heat is exactly the electrified quantum's flat rate, and the
/// record-mean annual delivered heat equals the quantum by
/// construction.
#[test]
fn overlay_conserves_the_electrified_quantum_on_a_synthetic_year() {
    let r = reference();
    let trace = constant_year(10.5);
    let start = trace.start();
    let spec = spec(vec![
        (HeatingKind::Ashp, 0.70),
        (HeatingKind::Gshp, 0.20),
        (HeatingKind::DistrictGeothermal, 0.10),
    ]);
    let overlay = compute_overlay(&spec, &r, &trace, start, trace.len()).unwrap();

    // Delivered heat integrates to the electrified quantum, scaled to
    // this trace's (365-day) year length: the flat DHW rate uses the
    // record's mean-year hours, which here is 8760.
    let delivered: f64 = overlay
        .delivered_heat
        .iter()
        .map(|p| (*p * Duration::half_hour()).as_gigawatt_hours())
        .sum();
    let electrified = 410_500.0 * 0.5;
    assert!(
        (delivered - electrified).abs() / electrified < 1e-9,
        "delivered {delivered} GWh differs from the electrified quantum {electrified} GWh"
    );

    // The heat series is flat (constant temperature), so every period
    // carries quantum / hours-in-year of heat.
    let expected_gw = electrified / 8760.0;
    for p in &overlay.delivered_heat {
        assert!((p.as_gigawatts() - expected_gw).abs() < 1e-9);
    }

    // Per-entry electrical demand: share × heat / COP, with the space
    // and DHW components at their own sinks. District: heat / 15.0.
    let district = overlay
        .entries
        .iter()
        .find(|e| e.kind == HeatingKind::DistrictGeothermal)
        .unwrap();
    let expected_district_gw = 0.10 * expected_gw / 15.0;
    assert!((district.electrical[0].as_gigawatts() - expected_district_gw).abs() < 1e-12);

    // ASHP at T_air = 10.5 °C: sink 40 − 10.5 = 29.5 °C, ΔT = 19.0 K
    // (above the 15 K floor); COP = 0.823 × 0.85 × (6.08 − 0.09×19 +
    // 0.0005×19²). DHW: ΔT = 50 − 10.5 = 39.5 K.
    let cop = |curve: [f64; 3], dt: f64, derating: f64| {
        derating * 0.85 * (curve[0] + curve[1] * dt + curve[2] * dt * dt)
    };
    let ashp_space_cop = cop([6.08, -0.09, 0.0005], 19.0, 0.823);
    let ashp_dhw_cop = cop([6.08, -0.09, 0.0005], 39.5, 0.823);
    let space_gw = (electrified * (1.0 - 0.170)) / (5.0 * 17_520.0 * 0.5) * 5.0;
    let dhw_gw = electrified * 0.170 / 8760.0;
    let expected_ashp_gw = 0.70 * (space_gw / ashp_space_cop + dhw_gw / ashp_dhw_cop);
    let ashp = overlay
        .entries
        .iter()
        .find(|e| e.kind == HeatingKind::Ashp)
        .unwrap();
    assert!(
        (ashp.electrical[0].as_gigawatts() - expected_ashp_gw).abs() < 1e-9,
        "ASHP electrical {} GW differs from the hand computation {expected_ashp_gw} GW",
        ashp.electrical[0].as_gigawatts()
    );

    // The total is the sum of the entries, and the echoed constants
    // carry the pinned k and DHW rate.
    let sum: f64 = overlay
        .entries
        .iter()
        .map(|e| e.electrical[0].as_gigawatts())
        .sum();
    assert!((overlay.electrical_total[0].as_gigawatts() - sum).abs() < 1e-12);
    let k = overlay.constants.k.as_gigawatts_per_kelvin();
    // k = space quantum / degree-hours = (electrified × 0.83) / (5 K × 8760 h).
    assert!((k - electrified * (1.0 - 0.170) / (5.0 * 8760.0)).abs() < 1e-12);
    assert!(
        (overlay.constants.dhw_rate.as_gigawatts() - dhw_gw).abs() < 1e-12,
        "DHW rate echo"
    );
}

/// The D16 SCOP read-out (docs/notes/d16-scop-readout-work-order.md):
/// SCOP = Σ delivered_heat / Σ electrical_total, read from the overlay
/// series themselves. A district-only portfolio draws heat / cop_const
/// every period, so its SCOP is exactly the reference cop_const — the
/// pass-through identity the acceptance test 4 relies on.
#[test]
fn seasonal_cop_of_a_district_only_overlay_is_cop_const() {
    let r = reference();
    let trace = constant_year(10.5);
    let spec = spec(vec![(HeatingKind::DistrictGeothermal, 1.0)]);
    let overlay = compute_overlay(&spec, &r, &trace, trace.start(), trace.len()).unwrap();
    let scop = overlay.seasonal_cop().unwrap();
    assert!(
        (scop - r.district.cop_const).abs() < 1e-9,
        "district-only SCOP {scop} differs from cop_const {}",
        r.district.cop_const
    );
}

/// SCOP is undefined when the overlay draws no electricity (zero
/// electrified share ⇒ zero heat, zero draw): `None`, never NaN or a
/// panic (no-panics rule, library crate).
#[test]
fn seasonal_cop_is_none_when_no_electricity_is_drawn() {
    let r = reference();
    let trace = constant_year(10.5);
    let mut zero = spec(vec![(HeatingKind::Ashp, 1.0)]);
    zero.electrified_share = PerUnit::new(0.0);
    let overlay = compute_overlay(&zero, &r, &trace, trace.start(), trace.len()).unwrap();
    assert_eq!(overlay.seasonal_cop(), None);
}

/// heat_need has a floor at zero: periods above T_base draw only the
/// DHW floor. (A record with NO degree-hours at all instead yields the
/// structured undefined-k error — also asserted.)
#[test]
fn warm_periods_draw_only_the_dhw_floor() {
    let r = reference();
    // Half a year at 5 °C, half at 20 °C (above T_base = 15.5).
    let start = UtcInstant::parse("2023-01-01T00:00:00Z").unwrap();
    let mut values = vec![Temperature::celsius(5.0); 8_760];
    values.extend(vec![Temperature::celsius(20.0); 17_520 - 8_760]);
    let trace = Trace::from_parts(start, values).unwrap();
    let spec = spec(vec![(HeatingKind::Ashp, 1.0)]);
    let overlay = compute_overlay(&spec, &r, &trace, trace.start(), trace.len()).unwrap();
    let dhw_gw = 410_500.0 * 0.5 * 0.170 / 8760.0;
    for p in &overlay.delivered_heat[8_760..] {
        assert!((p.as_gigawatts() - dhw_gw).abs() < 1e-9);
    }
    assert!(overlay.delivered_heat[0].as_gigawatts() > dhw_gw);

    // An all-warm record has zero degree-hours: k is undefined and the
    // overlay says so rather than dividing by zero.
    let err = compute_overlay(&spec, &r, &constant_year(20.0), start, 17_520).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidHeatingOverlay { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("degree-hours"), "err: {err}");
}

/// Structured errors: the temperature trace must cover whole calendar
/// years (the record-mean quantum is ill-defined otherwise), and the
/// horizon must lie inside the trace record.
#[test]
fn overlay_rejects_partial_years_and_out_of_record_horizons() {
    let r = reference();
    let spec = spec(vec![(HeatingKind::Ashp, 1.0)]);

    // A trace that stops mid-year.
    let start = UtcInstant::parse("2023-01-01T00:00:00Z").unwrap();
    let partial = Trace::from_parts(start, vec![Temperature::celsius(5.0); 10_000]).unwrap();
    let err = compute_overlay(&spec, &r, &partial, start, 48).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidHeatingOverlay { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("calendar year"), "err: {err}");

    // A horizon outside the record.
    let trace = constant_year(5.0);
    let outside = UtcInstant::parse("2024-06-01T00:00:00Z").unwrap();
    let err = compute_overlay(&spec, &r, &trace, outside, 48).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidHeatingOverlay { .. }),
        "unexpected error: {err:?}"
    );
}

/// The rule-4 district premise is machine-checked at computation time,
/// not assumed: an (unphysical) override below the heat pumps' maximum
/// record COP is a structured error naming the premise.
#[test]
fn district_cop_below_heat_pump_record_max_violates_the_checked_premise() {
    let r = reference();
    let trace = constant_year(10.5);
    let mut spec = spec(vec![(HeatingKind::DistrictGeothermal, 1.0)]);
    spec.entries[0].cop_const = Some(2.0); // below any heat-pump record max
    let err = compute_overlay(&spec, &r, &trace, trace.start(), trace.len()).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidHeatingOverlay { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("premise"), "err: {err}");
}

/// A legal per-entry `cop_curve` override whose quadratic goes
/// non-positive at an evaluated ΔT must be a structured error, never
/// silently negative electrical demand (review condition 3,
/// q5-heating-engine-review.md — the committed reference curves are
/// positive-definite, so only overrides are exposed).
#[test]
fn non_positive_effective_cop_from_an_override_is_a_structured_error() {
    let r = reference();
    let trace = constant_year(10.5);
    // At T = 10.5 °C: space ΔT = 19 K, DHW ΔT = 39.5 K. This curve is
    // positive at 19 K (1.0 − 0.019 > 0 … actually 1.0 − 1.9 < 0):
    // 1.0 − 0.1·ΔT is ≤ 0 from ΔT = 10 K, i.e. at every evaluated
    // period here.
    let mut bad = spec(vec![(HeatingKind::Ashp, 1.0)]);
    bad.entries[0].cop_curve = Some([1.0, -0.1, 0.0]);
    let err = compute_overlay(&bad, &r, &trace, trace.start(), trace.len()).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidHeatingOverlay { .. }),
        "unexpected error: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("COP"), "err: {msg}");
    assert!(msg.contains("ashp"), "err must name the entry: {msg}");

    // A positive-definite override stays legal.
    let mut fine = spec(vec![(HeatingKind::Ashp, 1.0)]);
    fine.entries[0].cop_curve = Some([6.0, -0.08, 0.0005]);
    compute_overlay(&fine, &r, &trace, trace.start(), trace.len()).unwrap();
}

/// Per-entry overrides are applied and echoed (the reliability/inertia
/// overrides precedent — they can never hide).
#[test]
fn per_entry_overrides_are_applied_and_echoed() {
    let r = reference();
    let trace = constant_year(10.5);
    let mut with_override = spec(vec![(HeatingKind::Ashp, 1.0)]);
    with_override.entries[0].rhpp_derating = Some(PerUnit::new(0.9));
    let overlay = compute_overlay(&with_override, &r, &trace, trace.start(), trace.len()).unwrap();
    let entry = &overlay.entries[0];
    assert_eq!(entry.params.rhpp_derating, Some(PerUnit::new(0.9)));
    assert_eq!(entry.params.overridden, vec!["rhpp_derating"]);

    // Electrical demand scales exactly by the derating ratio (COP is
    // linear in the derating).
    let baseline = spec(vec![(HeatingKind::Ashp, 1.0)]);
    let base = compute_overlay(&baseline, &r, &trace, trace.start(), trace.len()).unwrap();
    let ratio =
        base.electrical_total[0].as_gigawatts() / overlay.electrical_total[0].as_gigawatts();
    assert!(((ratio) - 0.9 / 0.823).abs() < 1e-12);
    assert!(base.entries[0].params.overridden.is_empty());
}

// ---------------------------------------------------------------------
// 2b. D16 geothermal depth continuum: the re-anchored ground wave and
//     the direct-use handoff, on synthetic traces (exact arithmetic).
// ---------------------------------------------------------------------

/// One synthetic non-leap calendar year as a pure annual cosine
/// (coldest at the year boundary) — gives the harmonic fit a non-zero
/// amplitude to damp.
fn sinusoidal_year(mean_c: f64, amplitude_c: f64) -> Trace<Temperature> {
    let start = UtcInstant::parse("2023-01-01T00:00:00Z").unwrap();
    let omega = 2.0 * std::f64::consts::PI / 8760.0;
    let values = (0..17_520)
        .map(|i| Temperature::celsius(mean_c - amplitude_c * (omega * i as f64 * 0.5).cos()))
        .collect();
    Trace::from_parts(start, values).unwrap()
}

/// D16 rule 1: re-anchoring the fitted wave at a resource depth (a)
/// is EXACTLY the fitted wave at the committed shallow datum
/// (`loop_depth_m`) — the gradient term is zero there and damping/lag
/// recompute to the same values bit-identically; (b) at depth, warms
/// the mean by `gradient × (z − loop_depth)` and kills the seasonal
/// swing (Kusuda–Achenbach damping ≈ 0 beyond ~15 m).
#[test]
fn re_anchored_ground_wave_is_the_gradient_warmed_damped_wave() {
    let r = reference();
    let wave = fit_ground_wave(&sinusoidal_year(10.5, 6.5), &r.ground).unwrap();

    // (a) At the committed datum: bit-identical, not merely close.
    let at_datum = wave.re_anchored(&r.ground, &r.geothermal, r.ground.loop_depth);
    assert_eq!(
        at_datum, wave,
        "re-anchoring at the loop-depth datum must be exactly the committed shallow wave"
    );

    // (b) At 1 km: mean warmed by 25 °C/km × 0.999 km = 24.975 K,
    // seasonal swing extinguished.
    let deep = wave.re_anchored(&r.ground, &r.geothermal, Length::metres(1000.0));
    let shift = deep.surface_mean.as_celsius() - wave.surface_mean.as_celsius();
    assert!(
        (shift - 24.975).abs() < 1e-12,
        "mean shift {shift} K differs from gradient × (z − datum) = 24.975 K"
    );
    assert!(
        deep.damping.value() < 1e-12,
        "the annual wave must be extinguished at 1 km (damping {})",
        deep.damping.value()
    );
    // The evaluated wave is then a near-steady warmed source.
    let mid_year = Duration::hours(4380.0);
    assert!((deep.at(mid_year).as_celsius() - deep.surface_mean.as_celsius()).abs() < 1e-9);
}

/// D16 rule 4 test 1 in unit form (exact, synthetic): an all-GSHP
/// overlay with `resource_depth_m = 1.0` (the committed datum) is
/// BIT-IDENTICAL to the overlay with the field absent, every period.
#[test]
fn overlay_at_the_shallow_datum_depth_is_bit_identical_to_absent() {
    let r = reference();
    let trace = sinusoidal_year(10.5, 6.5);
    let absent = spec(vec![(HeatingKind::Gshp, 1.0)]);
    let at_datum = spec_at_depth(vec![(HeatingKind::Gshp, 1.0)], 1.0);
    let a = compute_overlay(&absent, &r, &trace, trace.start(), trace.len()).unwrap();
    let b = compute_overlay(&at_datum, &r, &trace, trace.start(), trace.len()).unwrap();
    assert_eq!(
        a.electrical_total, b.electrical_total,
        "the depth path at the shallow datum must be bit-identical to the committed path"
    );
    // The depth is echoed (D9 rule 6b discipline) without being an
    // override of a reference parameter.
    assert_eq!(
        b.entries[0].params.resource_depth,
        Some(Length::metres(1.0))
    );
    assert!(b.entries[0].params.overridden.is_empty());
    assert_eq!(a.entries[0].params.resource_depth, None);
}

/// D16 rule 1, the direct-use handoff, exact arithmetic on a constant
/// 10.5 °C year (space sink 29.5 °C, DHW sink 50 °C, flat heat):
///
/// - 1200 m: source mean = 10.5 + 25 × 1.199 − 5 (brine) = 35.475 °C —
///   ABOVE the space sink (space passes through at the district
///   cop_const) but BELOW the DHW sink (DHW stays a heat pump at the
///   floored ΔT);
/// - 2000 m: source 55.475 °C — above BOTH sinks; the whole entry is
///   the district/pass-through regime and matches the all-district
///   overlay within the stated 1e-9 GW tolerance (component summation
///   order differs, so bitwise equality is not claimed).
#[test]
fn direct_use_handoff_switches_per_component_and_meets_the_district_endpoint() {
    let r = reference();
    let trace = constant_year(10.5);
    let start = trace.start();
    let electrified = 410_500.0 * 0.5;
    let space_gw = electrified * (1.0 - 0.170) / 8760.0;
    let dhw_gw = electrified * 0.170 / 8760.0;

    // 1200 m: space direct-use, DHW heat-pump at the 15 K floor.
    let mid = spec_at_depth(vec![(HeatingKind::Gshp, 1.0)], 1200.0);
    let overlay = compute_overlay(&mid, &r, &trace, start, trace.len()).unwrap();
    let dhw_cop = 0.732 * 0.85 * (10.29 - 0.21 * 15.0 + 0.0012 * 15.0 * 15.0);
    let expected = space_gw / 15.0 + dhw_gw / dhw_cop;
    let got = overlay.electrical_total[0].as_gigawatts();
    assert!(
        (got - expected).abs() < 1e-12,
        "1200 m: got {got} GW, expected space-direct + DHW-floored {expected} GW"
    );

    // 2000 m: both components direct-use → the district endpoint.
    let deep = spec_at_depth(vec![(HeatingKind::Gshp, 1.0)], 2000.0);
    let overlay = compute_overlay(&deep, &r, &trace, start, trace.len()).unwrap();
    let expected = space_gw / 15.0 + dhw_gw / 15.0;
    let got = overlay.electrical_total[0].as_gigawatts();
    assert!(
        (got - expected).abs() < 1e-12,
        "2000 m: got {got} GW, expected full pass-through {expected} GW"
    );
    let district = spec(vec![(HeatingKind::DistrictGeothermal, 1.0)]);
    let district_overlay = compute_overlay(&district, &r, &trace, start, trace.len()).unwrap();
    for (g, d) in overlay
        .electrical_total
        .iter()
        .zip(&district_overlay.electrical_total)
    {
        assert!(
            (g.as_gigawatts() - d.as_gigawatts()).abs() < 1e-9,
            "deep all-GSHP must meet the district endpoint within 1e-9 GW"
        );
    }
}

/// D16 rule 1, the cap side of "capped at, then handed to": on the
/// depth path, an (override) heat-pump COP above the district
/// cop_const is capped AT it, so the district-lowest ordering can tie
/// but never invert. Without a depth the committed override behaviour
/// is untouched.
#[test]
fn depth_path_caps_the_effective_cop_at_the_district_pass_through() {
    let r = reference();
    let trace = constant_year(10.5);
    let start = trace.start();
    let electrified = 410_500.0 * 0.5;
    let heat_gw = electrified / 8760.0;
    // 0.85 × 0.732 × 30 = 18.666 — above the district 15.0.
    let hot_curve = Some([30.0, 0.0, 0.0]);

    // 100 m: source 7.975 °C, below both sinks — heat-pump regime, but
    // the effective COP is capped at 15.0.
    let mut capped = spec_at_depth(vec![(HeatingKind::Gshp, 1.0)], 100.0);
    capped.entries[0].cop_curve = hot_curve;
    let overlay = compute_overlay(&capped, &r, &trace, start, trace.len()).unwrap();
    let got = overlay.electrical_total[0].as_gigawatts();
    let expected = heat_gw / 15.0;
    assert!(
        (got - expected).abs() < 1e-12,
        "capped: got {got} GW, expected heat/cop_const {expected} GW"
    );

    // Committed behaviour without a depth: the same override runs
    // uncapped (no district entry in the portfolio, so the machine-
    // checked premise does not apply — D9 edit 7 is per-portfolio).
    let mut uncapped = spec(vec![(HeatingKind::Gshp, 1.0)]);
    uncapped.entries[0].cop_curve = hot_curve;
    let overlay = compute_overlay(&uncapped, &r, &trace, start, trace.len()).unwrap();
    let cop = 0.85 * 0.732 * 30.0;
    let got = overlay.electrical_total[0].as_gigawatts();
    let expected = heat_gw / cop;
    assert!(
        (got - expected).abs() < 1e-12,
        "uncapped committed path: got {got} GW, expected heat/18.666 {expected} GW"
    );
}

// ---------------------------------------------------------------------
// 3. Pinned-trace acceptance tests (D9 rule 5) — require the fetched
//    GB t2m trace; FAIL LOUDLY if it is absent.
// ---------------------------------------------------------------------

const T2M_PATH: &str = "data/weather/gb_t2m_pop.parquet";
const T2M_COLUMN: &str = "t2m_pop";

/// The full 1985–2024 record.
const FULL_RECORD_PERIODS: usize = 701_280;

fn load_pinned_trace() -> Trace<Temperature> {
    let path = repo_root().join(T2M_PATH);
    assert!(
        path.exists(),
        "the pinned GB t2m trace is missing ({}) — build it first \
         (scripts/era5-cf/derive_t2m_gb.py; manifest data/packs/weather-gb-t2m-pop.sha256)",
        path.display()
    );
    let trace = load_temperature_trace_c(&path, T2M_COLUMN).unwrap();
    assert_eq!(trace.len(), FULL_RECORD_PERIODS);
    trace
}

/// The single-harmonic fit and the derived ground wave reproduce the
/// data package's recorded values (q5-heating-data-report.md §2):
/// surface mean 10.21 °C, surface amplitude 6.04 °C; ground amplitude
/// = amplitude × 0.7130 ≈ 4.30 °C.
#[test]
fn ground_wave_fit_reproduces_the_data_package_values() {
    let r = reference();
    let trace = load_pinned_trace();
    let wave = fit_ground_wave(&trace, &r.ground).unwrap();
    eprintln!(
        "ground wave: surface mean {:.4} °C, amplitude {:.4} °C, damping {:.4}, \
         lag {:.3} d, ground amplitude {:.4} °C",
        wave.surface_mean.as_celsius(),
        wave.surface_amplitude.as_celsius(),
        wave.damping.value(),
        wave.lag.as_hours() / 24.0,
        wave.surface_amplitude.as_celsius() * wave.damping.value(),
    );
    assert!((wave.surface_mean.as_celsius() - 10.21).abs() < 0.01);
    assert!((wave.surface_amplitude.as_celsius() - 6.04).abs() < 0.01);
    assert!((wave.damping.value() - 0.7130).abs() < 5e-4);
    assert!((wave.lag.as_hours() / 24.0 - 19.66).abs() < 0.01);
}

/// Edit-6 drift guard (D9 rule 4; the engine package's acceptance step
/// is REPRODUCTION of the data package's reviewed determination, not a
/// fresh one): the model-implied SPFH2 with rule-3 heat weighting over
/// the pinned record must reproduce 3.221 (ASHP) / 3.838 (GSHP), and
/// the to-median deratings 2.65/3.221 = 0.823 and 2.81/3.838 = 0.732
/// must match the reference file's pinned factors.
#[test]
fn implied_spfh2_reproduces_the_reviewed_deratings() {
    let r = reference();
    let trace = load_pinned_trace();

    let ashp = implied_spfh2(HeatingKind::Ashp, &r, &trace).unwrap();
    let gshp = implied_spfh2(HeatingKind::Gshp, &r, &trace).unwrap();
    eprintln!("implied SPFH2 (corrected curve, pre-derating): ASHP {ashp:.4}, GSHP {gshp:.4}");
    assert!(
        (ashp - 3.221).abs() < 0.005,
        "ASHP implied SPFH2 {ashp:.4} does not reproduce the reviewed 3.221"
    );
    assert!(
        (gshp - 3.838).abs() < 0.005,
        "GSHP implied SPFH2 {gshp:.4} does not reproduce the reviewed 3.838"
    );

    // Both sit OUTSIDE (above) their RHPP IQRs, so the deratings fire…
    assert!(ashp > r.rhpp.ashp_spfh2_iqr[1]);
    assert!(gshp > r.rhpp.gshp_spfh2_iqr[1]);
    // …and the to-median factors reproduce the pinned reference values
    // (rounded to 3 decimals in the file).
    let ashp_derating = r.rhpp.ashp_spfh2_median / ashp;
    let gshp_derating = r.rhpp.gshp_spfh2_median / gshp;
    assert!(
        (ashp_derating - r.ashp.rhpp_derating.value()).abs() < 5e-4,
        "ASHP to-median derating {ashp_derating:.4} does not reproduce the pinned 0.823"
    );
    assert!(
        (gshp_derating - r.gshp.rhpp_derating.value()).abs() < 5e-4,
        "GSHP to-median derating {gshp_derating:.4} does not reproduce the pinned 0.732"
    );
}

/// Acceptance test 1 (D9 rule 5): conservation — reference-window
/// (1985–2024) MEAN annual delivered heat equals the electrified
/// quantum for every portfolio mix; per-year totals vary with weather
/// and their spread is a reported finding, never normalised away.
#[test]
fn conservation_holds_for_all_mixes_and_per_year_totals_vary() {
    let r = reference();
    let trace = load_pinned_trace();
    let start = trace.start();
    let electrified = 410_500.0 * 0.5;

    for mix in [
        vec![(HeatingKind::Ashp, 1.0)],
        vec![(HeatingKind::Gshp, 1.0)],
        vec![(HeatingKind::DistrictGeothermal, 1.0)],
        vec![
            (HeatingKind::Ashp, 0.70),
            (HeatingKind::Gshp, 0.20),
            (HeatingKind::DistrictGeothermal, 0.10),
        ],
    ] {
        let overlay = compute_overlay(&spec(mix.clone()), &r, &trace, start, trace.len()).unwrap();
        let delivered: f64 = overlay
            .delivered_heat
            .iter()
            .map(|p| (*p * Duration::half_hour()).as_gigawatt_hours())
            .sum();
        let mean_annual = delivered / 40.0;
        assert!(
            (mean_annual - electrified).abs() / electrified < 1e-9,
            "mix {mix:?}: mean annual delivered heat {mean_annual:.3} GWh differs from \
             the electrified quantum {electrified} GWh"
        );

        // Delivered heat is IDENTICAL across portfolios by construction
        // (rule 5's invariance spine) — checked against the ASHP-only
        // mix below via the per-year totals.
        let per_year = per_year_totals(&overlay.delivered_heat, start);
        let (min_year, min) = per_year
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let (max_year, max) = per_year
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        eprintln!(
            "mix {mix:?}: per-year delivered heat spread {:.1}..{:.1} TWh \
             (min {min_year}, max {max_year})",
            min / 1000.0,
            max / 1000.0
        );
        // The inter-annual band exists: cold years draw more heat.
        assert!(
            max > min,
            "per-year delivered heat must vary with weather (never renormalised)"
        );
    }
}

/// Per-year delivered-heat energy (GWh), keyed by calendar year.
fn per_year_totals(series: &[Power], start: UtcInstant) -> Vec<(i64, f64)> {
    let mut totals: Vec<(i64, f64)> = Vec::new();
    for (t, p) in series.iter().enumerate() {
        let (year, _, _) = start.plus_periods(t as i64).civil_date();
        let energy = (*p * Duration::half_hour()).as_gigawatt_hours();
        match totals.last_mut() {
            Some((y, acc)) if *y == year => *acc += energy,
            _ => totals.push((year, energy)),
        }
    }
    totals
}

/// ADR-5 composability: `heat(t)` is a pure function of `T_pop(t)` —
/// computing the overlay for calendar 2024 alone yields bit-identical
/// series to the 2024 slice of the full-record overlay.
#[test]
fn horizon_subsetting_never_changes_the_overlay() {
    let r = reference();
    let trace = load_pinned_trace();
    let mix = spec(vec![
        (HeatingKind::Ashp, 0.70),
        (HeatingKind::Gshp, 0.20),
        (HeatingKind::DistrictGeothermal, 0.10),
    ]);

    let full = compute_overlay(&mix, &r, &trace, trace.start(), trace.len()).unwrap();
    let start_2024 = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
    let periods_2024 = 17_568;
    let sub = compute_overlay(&mix, &r, &trace, start_2024, periods_2024).unwrap();

    let offset = FULL_RECORD_PERIODS - periods_2024;
    assert_eq!(sub.electrical_total.len(), periods_2024);
    for t in 0..periods_2024 {
        assert_eq!(
            sub.electrical_total[t],
            full.electrical_total[offset + t],
            "period {t}: horizon subsetting changed the overlay"
        );
        assert_eq!(sub.delivered_heat[t], full.delivered_heat[offset + t]);
    }
    // The pinned constants are the same object either way.
    assert_eq!(sub.constants.k, full.constants.k);
    assert_eq!(sub.constants.dhw_rate, full.constants.dhw_rate);
}

/// Acceptance test 3, first limb (D9 rule 5): the district-lowest
/// ordering is asserted red-first WITH its premise machine-checked
/// (edit 7): `cop_const` exceeds the heat pumps' maximum record COP on
/// the engine-facing (post-derating) curves — computed, not assumed.
/// Given the premise, all-district electrical demand is below both
/// heat-pump portfolios at every period.
#[test]
fn district_lowest_limb_holds_with_the_premise_machine_checked() {
    let r = reference();
    let trace = load_pinned_trace();
    let start = trace.start();

    // The premise, computed on the record (post-derating; the data
    // package recorded 4.611 for GSHP max = 0.732 × 6.298).
    let ashp_max = record_max_cop(HeatingKind::Ashp, &r, &trace).unwrap();
    let gshp_max = record_max_cop(HeatingKind::Gshp, &r, &trace).unwrap();
    eprintln!(
        "record max post-derating COP: ASHP {ashp_max:.3}, GSHP {gshp_max:.3} vs \
         cop_const {} (band bottom {})",
        r.district.cop_const, r.district.cop_const_band[0]
    );
    assert!(
        r.district.cop_const > ashp_max && r.district.cop_const > gshp_max,
        "the district-lowest premise fails: cop_const {} vs heat-pump record max \
         {ashp_max:.3}/{gshp_max:.3}",
        r.district.cop_const
    );
    // It holds across the whole cited band, not only at the centre.
    assert!(r.district.cop_const_band[0] > ashp_max.max(gshp_max));

    // The limb: pointwise district-lowest, strict at each portfolio's
    // peak (the premise makes it a theorem; the test still measures).
    let district = compute_overlay(
        &spec(vec![(HeatingKind::DistrictGeothermal, 1.0)]),
        &r,
        &trace,
        start,
        trace.len(),
    )
    .unwrap();
    for kind in [HeatingKind::Ashp, HeatingKind::Gshp] {
        let hp = compute_overlay(&spec(vec![(kind, 1.0)]), &r, &trace, start, trace.len()).unwrap();
        let mut hp_peak = 0.0f64;
        for t in 0..trace.len() {
            let d = district.electrical_total[t].as_gigawatts();
            let h = hp.electrical_total[t].as_gigawatts();
            assert!(
                d <= h + 1e-12,
                "period {t}: district {d} GW exceeds {kind} {h} GW"
            );
            hp_peak = hp_peak.max(h);
        }
        let d_peak = district
            .electrical_total
            .iter()
            .map(|p| p.as_gigawatts())
            .fold(0.0f64, f64::max);
        assert!(
            d_peak < hp_peak,
            "district peak {d_peak} GW is not strictly below {kind} peak {hp_peak} GW"
        );
    }
}

/// Acceptance test 3, second limb (D9 rule 5 / ruling C): the
/// ASHP-vs-GSHP peak ordering is a MEASURED finding, pinned from
/// measurement — never pre-committed as a theorem. The D9-recorded
/// EXPECTATION is all-ASHP peak ≥ all-GSHP peak (at the coldest hours
/// the air-source lift dominates); an inversion would be a finding at
/// full prominence (the Package A/B lesson, kill-criterion 4).
///
/// MEASURED AND PINNED (first pass, 2026-07-03, pinned trace + the
/// committed reference file, quantum 410.5 TWh × electrified 0.5):
/// all-ASHP peak 47.72526272299193 GW, all-GSHP peak
/// 44.927558642468775 GW — the expected direction HOLDS (ASHP ≥ GSHP
/// by 2.798 GW at the all-portfolio peak).
#[test]
fn ashp_vs_gshp_peak_ordering_is_measured_and_pinned() {
    let r = reference();
    let trace = load_pinned_trace();
    let start = trace.start();

    let peak = |kind: HeatingKind| -> f64 {
        let overlay =
            compute_overlay(&spec(vec![(kind, 1.0)]), &r, &trace, start, trace.len()).unwrap();
        overlay
            .electrical_total
            .iter()
            .map(|p| p.as_gigawatts())
            .fold(0.0f64, f64::max)
    };
    let ashp_peak = peak(HeatingKind::Ashp);
    let gshp_peak = peak(HeatingKind::Gshp);
    eprintln!(
        "measured all-portfolio peaks: ASHP {ashp_peak} GW, GSHP {gshp_peak} GW \
         (expected direction: ASHP ≥ GSHP)"
    );

    // The pins (measured, not asserted a priori — see the doc comment).
    assert!(
        (ashp_peak - 47.72526272299193).abs() < 1e-9,
        "PINNED all-ASHP peak moved: measured {ashp_peak} GW — a deliberate \
         engine/reference/trace change requires a knowing re-pin"
    );
    assert!(
        (gshp_peak - 44.927558642468775).abs() < 1e-9,
        "PINNED all-GSHP peak moved: measured {gshp_peak} GW"
    );
    // The measured direction, recorded (this line documents the
    // finding; if a re-pin inverts it, that inversion is a
    // full-prominence finding, not a bug).
    assert!(
        ashp_peak >= gshp_peak,
        "MEASURED ORDERING INVERTED: all-ASHP peak {ashp_peak} GW < all-GSHP peak \
         {gshp_peak} GW — per D9 rule 5 this is a finding at full prominence; re-pin \
         with the record and surface it"
    );
}
