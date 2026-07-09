//! Stage 6 part 2 acceptance: the Q8 fleet-pathway runner
//! (docs/04 Stage 6 scope: "pathway runner (fleet as function of year)
//! for Q8"; demo artefact "largest-survivable-loss vs. year").
//!
//! Covers, per the work order:
//! - pathway-spec parse round-trip and structured errors (strict
//!   parsing: schema probe, deny_unknown_fields, semantic validation);
//! - the single-machine closed-form gate: with zero damping and no
//!   response services the survival boundary has the closed form
//!   `L* = E · (1 − (f_floor/f₀)²) / T` (from f(t) = f₀·√(1 − L·t/E),
//!   the same closed form the part-1 analytic gate pins), and the
//!   bisection must land on it within its documented tolerance;
//! - monotonicity: more inertia, same response ⇒ the largest survivable
//!   loss does not decrease (up to the bisection tolerance);
//! - determinism: bit-identical results on rerun (ADR-5);
//! - the zero-inertia year handled honestly: largest survivable loss
//!   0 MW with a `zero_inertia` flag — a FINDING, not an error (the
//!   RS-lean precedent);
//! - the 2019-default era assumptions drift-guarded against the
//!   committed 9 Aug 2019 event spec (they are documented as "the 2019
//!   values, cited" — this test makes that claim checkable).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use grid_core::GridError;
use grid_core::units::{Damping, Duration, Frequency, Inertia, PerUnit, Power};
use grid_stability::{
    EventSpec, PathwaySpec, ResponseService, ResponseShape, largest_survivable_loss,
    pathway_year_inertia, run_pathway,
};

/// The hand fixture: two years, one dispatch condition, explicit
/// era assumptions with damping and responses OFF so the closed form
/// applies. Hand-checkable numbers:
///
/// - 2030: ccgt 18 GW at fraction 1.0 ⇒ E = 5.0 s × 18/0.9 GVA =
///   100 GVA·s; demand 262.8 TWh / 8760 h = 30 GW.
/// - 2035: offshore wind + battery only ⇒ E = 0 (zero-inertia year).
const HAND_FIXTURE: &str = r#"
schema = "fes-pathway-v1"
name = "hand fixture"
fes_edition = "test-2026"

[assumptions]
f0_hz = 50.0
demand_gw = 25.0
load_damping_percent_per_hz = 0.0
duration_s = 60.0
timestep_ms = 10.0
survival_floor_hz = 48.8
search_max_loss_mw = 4000.0
search_tolerance_mw = 0.5
responses = []

[[assumptions.dispatch_conditions]]
name = "full"
synchronous_dispatch_fraction = 1.0

[[years]]
year = 2030
demand_twh = 262.8

[[years.fleet]]
technology = "ccgt"
capacity_gw = 18.0

[[years]]
year = 2035

[[years.fleet]]
technology = "offshore_wind"
capacity_gw = 80.0

[[years.storage]]
kind = "battery"
power_gw = 20.0
energy_gwh = 40.0
"#;

/// The data engineer's pinned shape, verbatim — no assumptions block at
/// all. Every era assumption must default (2019 values, flagged).
const PINNED_SHAPE: &str = r#"
schema = "fes-pathway-v1"
name = "pinned shape"
fes_edition = "FES 2024"

[[years]]
year = 2030
demand_twh = 300.0

[[years.fleet]]
technology = "offshore_wind"
capacity_gw = 50.0

[[years.fleet]]
technology = "ccgt"
capacity_gw = 20.0

[[years.storage]]
kind = "battery"
power_gw = 10.0
energy_gwh = 20.0
"#;

// ---------------------------------------------------------------------
// Parsing: round-trip and structured errors.
// ---------------------------------------------------------------------

#[test]
fn hand_fixture_parses_with_every_field_where_expected() {
    let spec = PathwaySpec::from_toml_str(HAND_FIXTURE).unwrap();
    assert_eq!(spec.name, "hand fixture");
    assert_eq!(spec.fes_edition, "test-2026");
    assert_eq!(spec.years.len(), 2);
    assert_eq!(spec.years[0].year, 2030);
    // demand_twh 262.8 → mean 30 GW under the documented 8,760 h year.
    let demand = spec.years[0].demand.unwrap();
    assert!((demand.as_gigawatts() - 30.0).abs() < 1e-12, "{demand:?}");
    assert!(spec.years[1].demand.is_none());
    assert_eq!(spec.years[0].fleet.len(), 1);
    assert_eq!(spec.years[0].fleet[0].technology.as_str(), "ccgt");
    assert_eq!(spec.years[1].storage.len(), 1);

    let a = &spec.assumptions;
    assert_eq!(a.f0, Frequency::hertz(50.0));
    assert_eq!(a.demand_fallback, Power::gigawatts(25.0));
    assert_eq!(a.load_damping, Damping::percent_of_demand_per_hertz(0.0));
    assert_eq!(a.duration, Duration::from_seconds(60.0));
    assert_eq!(a.survival_floor, Frequency::hertz(48.8));
    assert_eq!(a.search_max_loss, Power::megawatts(4000.0));
    assert_eq!(a.search_tolerance, Power::megawatts(0.5));
    // Explicit values are not flagged as defaulted.
    assert!(!a.load_damping_defaulted);
    assert!(!a.responses_defaulted_to_2019);
    assert!(!a.demand_fallback_defaulted);
    assert!(!a.dispatch_conditions_defaulted);
    assert!(a.responses.is_empty());
    assert_eq!(a.dispatch_conditions.len(), 1);
    assert_eq!(a.dispatch_conditions[0].name, "full");
    assert_eq!(
        a.dispatch_conditions[0].synchronous_dispatch_fraction,
        PerUnit::new(1.0)
    );
    // The secured-loss reference lines default with citations
    // (data/reference/stability-2019-event.toml standards sections).
    let reference: Vec<f64> = a
        .reference_losses
        .iter()
        .map(|r| r.loss.as_gigawatts() * 1000.0)
        .collect();
    assert_eq!(reference, vec![1800.0, 1320.0]);
}

#[test]
fn pinned_shape_parses_with_cited_2019_defaults_flagged() {
    let spec = PathwaySpec::from_toml_str(PINNED_SHAPE).unwrap();
    let a = &spec.assumptions;
    assert!(a.load_damping_defaulted);
    assert!(a.responses_defaulted_to_2019);
    assert!(a.demand_fallback_defaulted);
    assert!(a.dispatch_conditions_defaulted);
    // The 2019 era values (drift-guarded against the committed event
    // spec in `default_era_assumptions_match_the_committed_2019_spec`).
    assert_eq!(a.load_damping, Damping::percent_of_demand_per_hertz(1.836));
    assert_eq!(a.demand_fallback, Power::gigawatts(29.0));
    assert_eq!(a.responses.len(), 3);
    // The documented default band: "min" and "mean" dispatch fractions,
    // measurement-anchored (pathway module docs §1): the 2024 reference
    // run's H-weighted synchronised share has mean 0.344 and p5/p10
    // 0.147/0.169 (measured 2026-07-03), so the pinned defaults are
    // 0.15 and 0.35.
    assert_eq!(a.dispatch_conditions.len(), 2);
    assert_eq!(a.dispatch_conditions[0].name, "min");
    assert_eq!(
        a.dispatch_conditions[0].synchronous_dispatch_fraction,
        PerUnit::new(0.15)
    );
    assert_eq!(a.dispatch_conditions[1].name, "mean");
    assert_eq!(
        a.dispatch_conditions[1].synchronous_dispatch_fraction,
        PerUnit::new(0.35)
    );
    // Survival floor defaults to LFDD stage 1 (48.8 Hz).
    assert_eq!(a.survival_floor, Frequency::hertz(48.8));
}

#[test]
fn structured_errors_name_what_is_wrong() {
    // Wrong schema string.
    let toml = HAND_FIXTURE.replace("fes-pathway-v1", "fes-pathway-v9");
    let err = PathwaySpec::from_toml_str(&toml).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidPathwaySpec { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("fes-pathway-v1"));

    // Unknown field (deny_unknown_fields).
    let toml = HAND_FIXTURE.replace("year = 2030", "year = 2030\nfrobnicate = 1");
    let err = PathwaySpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("frobnicate"), "err: {err}");

    // No years at all.
    let mut no_years = String::new();
    for line in HAND_FIXTURE.lines() {
        if line.starts_with("[[years") {
            break;
        }
        no_years.push_str(line);
        no_years.push('\n');
    }
    let err = PathwaySpec::from_toml_str(&no_years).unwrap_err();
    assert!(err.to_string().contains("year"), "err: {err}");

    // Years out of order (strictly increasing required).
    let toml = HAND_FIXTURE.replace("year = 2035", "year = 2030");
    let err = PathwaySpec::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("increasing"), "err: {err}");

    // Dispatch fraction above 1.
    let toml = HAND_FIXTURE.replace(
        "synchronous_dispatch_fraction = 1.0",
        "synchronous_dispatch_fraction = 1.5",
    );
    let err = PathwaySpec::from_toml_str(&toml).unwrap_err();
    assert!(
        err.to_string().contains("synchronous_dispatch_fraction"),
        "err: {err}"
    );

    // Negative capacity.
    let toml = HAND_FIXTURE.replace("capacity_gw = 18.0", "capacity_gw = -1.0");
    assert!(PathwaySpec::from_toml_str(&toml).is_err());

    // A static (latched) response service is rejected: the survival
    // search bisects on a predicate that is only guaranteed monotone in
    // the loss for state-free (dynamic droop) services.
    let toml = HAND_FIXTURE.replace(
        "responses = []",
        r#"[[assumptions.responses]]
name = "static_reserve"
mw = 500
droop_full_deviation_hz = 0.5
trigger_hz = 49.7
delay_s = 1.0
ramp_s = 5.0"#,
    );
    assert!(PathwaySpec::from_toml_str(&toml).is_err());

    // Search tolerance must be positive and below the bracket.
    let toml = HAND_FIXTURE.replace("search_tolerance_mw = 0.5", "search_tolerance_mw = 0.0");
    assert!(PathwaySpec::from_toml_str(&toml).is_err());
}

#[test]
fn load_of_missing_file_names_the_path() {
    let err = PathwaySpec::load(Path::new("/nonexistent/pathway.toml")).unwrap_err();
    assert!(err.to_string().contains("/nonexistent/pathway.toml"));
}

// ---------------------------------------------------------------------
// The fleet→inertia convention: Σ H × (capacity × fraction) / 0.9 over
// synchronous plant (grid_core::inertia defaults; storage counts at its
// power rating under the same fraction).
// ---------------------------------------------------------------------

#[test]
fn pathway_year_inertia_matches_the_hand_sum() {
    let toml = r#"
schema = "fes-pathway-v1"
name = "inertia hand sum"
fes_edition = "test"

[[years]]
year = 2030

[[years.fleet]]
technology = "ccgt"
capacity_gw = 10.0

[[years.fleet]]
technology = "nuclear"
capacity_gw = 5.0

[[years.fleet]]
technology = "offshore_wind"
capacity_gw = 30.0

[[years.storage]]
kind = "pumped_hydro"
power_gw = 2.0
energy_gwh = 10.0

[[years.storage]]
kind = "battery"
power_gw = 5.0
"#;
    let spec = PathwaySpec::from_toml_str(toml).unwrap();
    let inertia = pathway_year_inertia(&spec.years[0], PerUnit::new(0.5));
    // Hand: (5.0×10 + 4.5×5 + 4.5×2) × 0.5 / 0.9 = 81.5 × 0.5/0.9
    // = 45.2777… GVA·s; wind and battery contribute nothing.
    let hand = (5.0 * 10.0 + 4.5 * 5.0 + 4.5 * 2.0) * 0.5 / 0.9;
    assert!(
        (inertia.as_gigavolt_ampere_seconds() - hand).abs() < 1e-9,
        "engine {} vs hand {hand}",
        inertia.as_gigavolt_ampere_seconds()
    );
}

// ---------------------------------------------------------------------
// The survival search against the closed form (the part-1 analytic
// gate's event: constant ΔP, no damping, no response).
// ---------------------------------------------------------------------

#[test]
fn largest_survivable_loss_matches_the_closed_form_single_machine_case() {
    let spec = PathwaySpec::from_toml_str(HAND_FIXTURE).unwrap();
    let points = run_pathway(&spec).unwrap();
    assert_eq!(points.len(), 2);

    let p2030 = &points[0];
    assert_eq!(p2030.year, 2030);
    assert_eq!(p2030.condition, "full");
    assert!((p2030.inertia.as_gigavolt_ampere_seconds() - 100.0).abs() < 1e-9);
    // Closed form: L* = E·(1 − (48.8/50)²)/T = 100 × 0.047424 / 60 GW
    // = 79.04 MW. The bisection (tolerance 0.5 MW) must land within
    // tolerance below the boundary.
    let found_mw = p2030.survivable.largest_survivable_loss.as_gigawatts() * 1000.0;
    assert!(
        (found_mw - 79.04).abs() <= 0.6,
        "found {found_mw} MW vs closed form 79.04 MW"
    );
    assert!(!p2030.survivable.bracket_saturated);
    assert!(!p2030.survivable.zero_inertia);
}

#[test]
fn zero_inertia_year_reports_zero_survivable_loss_as_a_finding() {
    let spec = PathwaySpec::from_toml_str(HAND_FIXTURE).unwrap();
    let points = run_pathway(&spec).unwrap();
    let p2035 = &points[1];
    assert_eq!(p2035.year, 2035);
    assert_eq!(p2035.inertia, Inertia::gigavolt_ampere_seconds(0.0));
    assert!(p2035.survivable.zero_inertia);
    assert_eq!(
        p2035.survivable.largest_survivable_loss,
        Power::gigawatts(0.0)
    );
}

#[test]
fn bracket_saturation_is_reported_not_hidden() {
    let mut spec = PathwaySpec::from_toml_str(HAND_FIXTURE).unwrap();
    spec.assumptions.search_max_loss = Power::megawatts(50.0);
    spec.assumptions.search_tolerance = Power::megawatts(0.5);
    // E = 100 GVA·s survives far more than 50 MW over 60 s.
    let point = largest_survivable_loss(
        &spec.assumptions,
        Inertia::gigavolt_ampere_seconds(100.0),
        Power::gigawatts(30.0),
    )
    .unwrap();
    assert!(point.bracket_saturated);
    assert_eq!(point.largest_survivable_loss, Power::megawatts(50.0));
}

#[test]
fn more_inertia_same_response_does_not_decrease_the_survivable_loss() {
    // The monotonicity property, under the full 2019-default era
    // assumptions (damping + three dynamic services). Guaranteed
    // because the survival dynamics are state-free (dynamic droop
    // services only): at fixed (t, f) the deficit is decreasing in E's
    // effect, so trajectories for larger E lie above — up to the
    // bisection tolerance.
    let spec = PathwaySpec::from_toml_str(PINNED_SHAPE).unwrap();
    let a = &spec.assumptions;
    let tol_gw = a.search_tolerance.as_gigawatts();
    let demand = Power::gigawatts(30.0);
    let mut previous: Option<f64> = None;
    for e in [20.0, 50.0, 100.0, 200.0, 400.0] {
        let point =
            largest_survivable_loss(a, Inertia::gigavolt_ampere_seconds(e), demand).unwrap();
        let loss = point.largest_survivable_loss.as_gigawatts();
        if let Some(prev) = previous {
            assert!(
                loss >= prev - tol_gw - 1e-12,
                "survivable loss decreased with inertia: {prev} GW → {loss} GW at E = {e}"
            );
        }
        previous = Some(loss);
    }
}

#[test]
fn pathway_results_are_bit_identical_on_rerun() {
    let spec = PathwaySpec::from_toml_str(PINNED_SHAPE).unwrap();
    let a = run_pathway(&spec).unwrap();
    let b = run_pathway(&spec).unwrap();
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.year, y.year);
        assert_eq!(x.condition, y.condition);
        assert_eq!(
            x.inertia.as_gigavolt_ampere_seconds().to_bits(),
            y.inertia.as_gigavolt_ampere_seconds().to_bits()
        );
        assert_eq!(
            x.survivable
                .largest_survivable_loss
                .as_gigawatts()
                .to_bits(),
            y.survivable
                .largest_survivable_loss
                .as_gigawatts()
                .to_bits()
        );
    }
}

// ---------------------------------------------------------------------
// Drift guard: the "2019 values, cited" defaults must actually be the
// committed 9 Aug 2019 event spec's values (response holdings ×
// delivery factors × envelope timings, damping, demand base). The
// no-retuning rule (stage-6-stability-run-report.md §2) binds those
// values; this test makes any drift loud.
// ---------------------------------------------------------------------

#[test]
fn default_era_assumptions_match_the_committed_2019_spec() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let event = EventSpec::load(&root.join("scenarios/events/gb-2019-08-09.toml")).unwrap();
    let pathway = PathwaySpec::from_toml_str(PINNED_SHAPE).unwrap();
    let a = &pathway.assumptions;
    assert_eq!(a.responses, event.responses, "response services drifted");
    assert_eq!(a.load_damping, event.load_damping, "damping drifted");
    assert_eq!(a.demand_fallback, event.demand, "demand base drifted");
    assert_eq!(a.f0, event.f0, "nominal frequency drifted");
}

// ---------------------------------------------------------------------
// Q8 current-holdings variant (stage-6-part2-run-report.md §6
// publication rule 2 counterpart): the variant pathway spec must be the
// committed FES 2025 Holistic Transition table verbatim with ONLY the
// response holdings replaced by the three reviewed FY2025 NESO dynamic
// low-frequency services (data/reference/response-holdings-2025.toml,
// reviewed docs/notes/q8-current-holdings.md — commit 887d5e4). These
// guards make "same everything, only the holdings" checkable, the same
// way the 2019 defaults are drift-guarded above.
// ---------------------------------------------------------------------

/// The three reviewed FY2025 services, transcribed from
/// `data/reference/response-holdings-2025.toml` (FY2025 mean cleared
/// volumes; Service Terms saturation/timing; delivery_factor per the
/// stated convention — contract 1.0 for the central variant).
fn reviewed_2025_responses(delivery_factor: f64) -> Vec<ResponseService> {
    let dynamic = |hz: f64| ResponseShape::Dynamic {
        droop_full_deviation: Frequency::hertz(hz),
    };
    vec![
        ResponseService {
            name: "dynamic_containment_lf".to_owned(),
            power: Power::megawatts(1178.0),
            delivery_factor: PerUnit::new(delivery_factor),
            shape: dynamic(0.5),
            delay: Duration::from_seconds(0.5),
            ramp: Duration::from_seconds(0.5),
            sustain: None,
            rundown: None,
        },
        ResponseService {
            name: "dynamic_moderation_lf".to_owned(),
            power: Power::megawatts(416.0),
            delivery_factor: PerUnit::new(delivery_factor),
            shape: dynamic(0.2),
            delay: Duration::from_seconds(0.5),
            ramp: Duration::from_seconds(0.5),
            sustain: None,
            rundown: None,
        },
        ResponseService {
            name: "dynamic_regulation_lf".to_owned(),
            power: Power::megawatts(461.0),
            delivery_factor: PerUnit::new(delivery_factor),
            shape: dynamic(0.2),
            delay: Duration::from_seconds(2.0),
            ramp: Duration::from_seconds(8.0),
            sustain: None,
            rundown: None,
        },
    ]
}

/// Everything except the responses must be identical between the base
/// FES pathway spec and a variant of it.
fn assert_same_but_for_responses(base: &PathwaySpec, variant: &PathwaySpec) {
    assert_eq!(variant.name, base.name, "pathway name drifted");
    assert_eq!(variant.fes_edition, base.fes_edition, "edition drifted");
    assert_eq!(
        variant.years, base.years,
        "fleet table drifted from the base FES 2025 Holistic Transition"
    );
    let (a, b) = (&variant.assumptions, &base.assumptions);
    assert_eq!(a.f0, b.f0);
    assert_eq!(a.demand_fallback, b.demand_fallback);
    assert_eq!(a.demand_fallback_defaulted, b.demand_fallback_defaulted);
    assert_eq!(a.load_damping, b.load_damping);
    assert_eq!(a.load_damping_defaulted, b.load_damping_defaulted);
    assert_eq!(a.duration, b.duration);
    assert_eq!(a.timestep, b.timestep);
    assert_eq!(a.survival_floor, b.survival_floor);
    assert_eq!(a.search_max_loss, b.search_max_loss);
    assert_eq!(a.search_tolerance, b.search_tolerance);
    assert_eq!(a.dispatch_conditions, b.dispatch_conditions);
    assert_eq!(
        a.dispatch_conditions_defaulted,
        b.dispatch_conditions_defaulted
    );
    assert_eq!(a.reference_losses, b.reference_losses);
}

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn current_holdings_variant_is_the_base_fes_table_with_only_the_reviewed_responses() {
    let root = repo_root();
    let base = PathwaySpec::load(&root.join("data/reference/fes-pathway.toml")).unwrap();
    let variant =
        PathwaySpec::load(&root.join("data/reference/fes-pathway-current-holdings.toml")).unwrap();
    assert_same_but_for_responses(&base, &variant);
    // The holdings are explicit spec inputs, not 2019 defaults.
    assert!(!variant.assumptions.responses_defaulted_to_2019);
    // Mechanical transcription of the reviewed record, contract
    // delivery factor 1.0.
    assert_eq!(variant.assumptions.responses, reviewed_2025_responses(1.0));
}

#[test]
fn df090_sensitivity_variant_differs_from_central_only_in_delivery_factor() {
    let root = repo_root();
    let base = PathwaySpec::load(&root.join("data/reference/fes-pathway.toml")).unwrap();
    let variant =
        PathwaySpec::load(&root.join("data/reference/fes-pathway-current-holdings-df090.toml"))
            .unwrap();
    assert_same_but_for_responses(&base, &variant);
    assert!(!variant.assumptions.responses_defaulted_to_2019);
    // 0.9 uniform on the three dynamic services — the evidence note's
    // prescribed quantification of the contract-vs-measured asymmetry
    // (2019 uses measured factors 0.67–1.0; EAC publishes none).
    assert_eq!(variant.assumptions.responses, reviewed_2025_responses(0.9));
}

#[test]
fn speed_diagnostic_variant_carries_current_volumes_at_2019_envelope_timings() {
    // The speed-vs-volume isolation diagnostic: current FY2025 volumes
    // and droops, but the three 2019 tranches' delay/ramp envelopes
    // mapped by speed rank (0.3/0.7, 2/8, 10/20). Diagnostic only —
    // never quotable as a holdings scenario.
    let root = repo_root();
    let base = PathwaySpec::load(&root.join("data/reference/fes-pathway.toml")).unwrap();
    let variant = PathwaySpec::load(
        &root.join("data/reference/fes-pathway-current-holdings-2019-speed.toml"),
    )
    .unwrap();
    assert_same_but_for_responses(&base, &variant);
    let mut expected = reviewed_2025_responses(1.0);
    expected[0].delay = Duration::from_seconds(0.3);
    expected[0].ramp = Duration::from_seconds(0.7);
    expected[1].delay = Duration::from_seconds(2.0);
    expected[1].ramp = Duration::from_seconds(8.0);
    expected[2].delay = Duration::from_seconds(10.0);
    expected[2].ramp = Duration::from_seconds(20.0);
    assert_eq!(variant.assumptions.responses, expected);
}
