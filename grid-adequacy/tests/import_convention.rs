//! Package B (ratified 2026-07-03): the import-convention bracketing
//! transformations — tests for `apply_import_convention` and
//! `link_export_capability`.
//!
//! Definitions under test (full prose at the code site,
//! `grid_adequacy::import_convention`):
//!
//! - **Pre-import surplus period**: domestic must-take — weather-driven
//!   potential (capacity × CF, at the swept capacity) plus every
//!   NON-import exogenous must-take series — STRICTLY exceeds demand;
//!   the surplus magnitude is that excess. Computed at trace level,
//!   pre-storage by construction.
//! - **FROZEN**: identity (the current sweep behaviour, bit-identical).
//! - **ZERO-IN-SURPLUS**: every imports-flagged series is 0 in surplus
//!   periods, unchanged elsewhere (including periods where the frozen
//!   trace already exports — the value is set to 0, not clamped).
//! - **EXPORT-IN-SURPLUS**: the aggregate imports-flagged supply is
//!   −min(export_capacity, surplus magnitude) in surplus periods,
//!   unchanged elsewhere. The min() means GB exports its own surplus
//!   only: exports never force thermal dispatch.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::import_convention::{
    ImportConvention, apply_import_convention, link_export_capability,
};
use grid_adequacy::{ExogenousSupply, RunInputs};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, ExogenousReliability, FleetEntry, Horizon, LinkSpec,
    Scenario, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{PerUnit, Power};
use proptest::prelude::*;

const START: &str = "2024-01-01T00:00:00Z";

fn start() -> UtcInstant {
    UtcInstant::parse(START).unwrap()
}

fn power_trace(values_gw: &[f64]) -> Trace<Power> {
    Trace::from_parts(
        start(),
        values_gw.iter().map(|&v| Power::gigawatts(v)).collect(),
    )
    .unwrap()
}

fn cf_trace(values: &[f64]) -> Trace<PerUnit> {
    Trace::from_parts(start(), values.iter().map(|&v| PerUnit::new(v)).collect()).unwrap()
}

fn renewable(tech: &str, capacity_gw: f64) -> FleetEntry {
    FleetEntry {
        technology: TechId::new(tech),
        capacity_gw: Power::gigawatts(capacity_gw),
        capacity_factor_trace: Some(format!("synthetic/{tech}.parquet").into()),
        availability: None,
        reliability: None,
        inertia_h: None,
        synchronous: None,
        energy_budget: None,
    }
}

fn thermal(tech: &str, capacity_gw: f64) -> FleetEntry {
    FleetEntry {
        technology: TechId::new(tech),
        capacity_gw: Power::gigawatts(capacity_gw),
        capacity_factor_trace: None,
        availability: None,
        reliability: None,
        inertia_h: None,
        synchronous: None,
        energy_budget: None,
    }
}

fn scenario_with(fleet: Vec<FleetEntry>, links: Vec<LinkSpec>, periods: usize) -> Scenario {
    let end = start().plus_periods(periods as i64 - 1);
    Scenario {
        schema_version: 5,
        name: "synthetic".to_owned(),
        description: None,
        horizon: Horizon {
            start: START.to_owned(),
            end: end.to_string(),
            weather_years: WeatherYears::Years(vec![2024]),
        },
        zones: vec![ZoneSpec {
            pricing: None,
            id: ZoneId::new("GB"),
            demand: DemandSpec {
                base_profile: "unused-in-synthetic-runs".into(),
                column: "underlying_demand".to_owned(),
                extra_profiles: vec![],
                annual_scale: 1.0,
                extra_demand_gw: Power::gigawatts(0.0),
                heating: None,
            },
            exogenous_supply: vec![],
            fleet,
            storage: vec![],
        }],
        links,
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

fn exo(label: &str, imports: bool, values_gw: &[f64]) -> ExogenousSupply {
    ExogenousSupply {
        label: label.to_owned(),
        imports,
        reliability: ExogenousReliability::Excluded,
        trace: power_trace(values_gw),
    }
}

fn run_inputs(
    demand_gw: &[f64],
    cf: &[(&str, &[f64])],
    exogenous: Vec<ExogenousSupply>,
) -> RunInputs {
    RunInputs {
        demand: power_trace(demand_gw),
        capacity_factors: cf
            .iter()
            .map(|(tech, values)| (TechId::new(*tech), cf_trace(values)))
            .collect::<BTreeMap<_, _>>(),
        exogenous,
        availability: BTreeMap::new(),
        heating: None,
    }
}

/// The reference fixture: 10 GW wind, one thermal unit, one non-import
/// exogenous series ("other", 1 GW flat) and one imports-flagged series
/// with mixed signs. Domestic must-take D(t) = 10 × cf(t) + 1.
///
/// | t | demand | cf   | D(t) | s(t) = D − demand | class            |
/// |---|--------|------|------|--------------------|------------------|
/// | 0 | 10     | 0.20 | 3.0  | −7.0               | deficit          |
/// | 1 | 5      | 0.80 | 9.0  | +4.0               | surplus, s > cap |
/// | 2 | 5      | 0.55 | 6.5  | +1.5               | surplus, s < cap |
/// | 3 | 6      | 0.50 | 6.0  |  0.0               | boundary (NOT surplus — strict) |
/// | 4 | 4      | 0.60 | 7.0  | +3.0               | surplus, frozen already exports |
fn fixture() -> (Scenario, RunInputs) {
    let scenario = scenario_with(
        vec![renewable("onshore_wind", 10.0), thermal("ccgt", 20.0)],
        vec![],
        5,
    );
    let inputs = run_inputs(
        &[10.0, 5.0, 5.0, 6.0, 4.0],
        &[("onshore_wind", &[0.2, 0.8, 0.55, 0.5, 0.6])],
        vec![
            exo("net_imports", true, &[3.0, 2.0, 2.0, 2.0, -1.0]),
            exo("other", false, &[1.0; 5]),
        ],
    );
    (scenario, inputs)
}

fn import_values(inputs: &RunInputs, label: &str) -> Vec<f64> {
    inputs
        .exogenous
        .iter()
        .find(|s| s.label == label)
        .unwrap()
        .trace
        .values()
        .iter()
        .map(|p| p.as_gigawatts())
        .collect()
}

const CAP: f64 = 2.5;

fn export_convention() -> ImportConvention {
    ImportConvention::ExportInSurplus {
        export_capacity: Power::gigawatts(CAP),
    }
}

// ---------------------------------------------------------------------
// Requirement (1), library level: FROZEN is the identity.
// ---------------------------------------------------------------------

#[test]
fn frozen_returns_bit_identical_inputs() {
    let (scenario, inputs) = fixture();
    let frozen = apply_import_convention(&scenario, &inputs, &ImportConvention::Frozen).unwrap();
    assert_eq!(frozen, inputs);
}

// ---------------------------------------------------------------------
// Requirement (2): the variants alter ONLY strict-surplus periods.
// ---------------------------------------------------------------------

#[test]
fn zero_in_surplus_zeroes_imports_only_in_strict_surplus_periods() {
    let (scenario, inputs) = fixture();
    let out =
        apply_import_convention(&scenario, &inputs, &ImportConvention::ZeroInSurplus).unwrap();
    // t0 deficit and t3 exact balance (strict mask) keep the frozen
    // values; surplus periods are zeroed — INCLUDING t4, where the
    // frozen trace already exports (−1 → 0, documented choice).
    assert_eq!(
        import_values(&out, "net_imports"),
        [3.0, 0.0, 0.0, 2.0, 0.0]
    );
    // Non-import exogenous series untouched.
    assert_eq!(import_values(&out, "other"), [1.0; 5]);
    // Demand / CF inputs untouched.
    assert_eq!(out.demand, inputs.demand);
    assert_eq!(out.capacity_factors, inputs.capacity_factors);
}

#[test]
fn export_in_surplus_caps_at_capacity_and_at_surplus_magnitude() {
    let (scenario, inputs) = fixture();
    let out = apply_import_convention(&scenario, &inputs, &export_convention()).unwrap();
    // t1: s = 4.0 > cap 2.5 → −2.5 (capacity binds).
    // t2: s = 1.5 < cap → −1.5 (surplus binds: exports never force
    //     thermal dispatch).
    // t4: s = 3.0 → −2.5, replacing the frozen −1.
    // t0 (deficit) and t3 (exact balance) unchanged.
    assert_eq!(
        import_values(&out, "net_imports"),
        [3.0, -2.5, -1.5, 2.0, -2.5]
    );
    assert_eq!(import_values(&out, "other"), [1.0; 5]);
}

// ---------------------------------------------------------------------
// Requirement (5): a zero-surplus system makes the three conventions
// bit-identical.
// ---------------------------------------------------------------------

#[test]
fn no_surplus_makes_all_three_conventions_bit_identical() {
    let scenario = scenario_with(
        vec![renewable("onshore_wind", 10.0), thermal("ccgt", 20.0)],
        vec![],
        4,
    );
    // Demand above domestic must-take everywhere (D = 10 cf + 1 ≤ 9).
    let inputs = run_inputs(
        &[20.0, 15.0, 12.0, 9.1],
        &[("onshore_wind", &[0.1, 0.5, 0.8, 0.8])],
        vec![
            exo("net_imports", true, &[3.0, -2.0, 1.0, 0.5]),
            exo("other", false, &[1.0; 4]),
        ],
    );
    let frozen = apply_import_convention(&scenario, &inputs, &ImportConvention::Frozen).unwrap();
    let zero =
        apply_import_convention(&scenario, &inputs, &ImportConvention::ZeroInSurplus).unwrap();
    let export = apply_import_convention(&scenario, &inputs, &export_convention()).unwrap();
    assert_eq!(frozen, inputs);
    assert_eq!(zero, inputs);
    assert_eq!(export, inputs);
}

// ---------------------------------------------------------------------
// Aggregate rule for multiple imports-flagged series (documented
// choice: the aggregate target lands on the first series, the rest are
// zeroed in surplus periods; unchanged elsewhere).
// ---------------------------------------------------------------------

#[test]
fn multiple_import_series_carry_the_aggregate_on_the_first() {
    let scenario = scenario_with(vec![renewable("onshore_wind", 10.0)], vec![], 2);
    // t0: D = 8 + 0 (no non-import exo) vs demand 4 → s = 4 (surplus).
    // t1: D = 2 vs demand 4 → deficit.
    let inputs = run_inputs(
        &[4.0, 4.0],
        &[("onshore_wind", &[0.8, 0.2])],
        vec![
            exo("imports_a", true, &[1.0, 1.5]),
            exo("imports_b", true, &[0.5, 0.25]),
        ],
    );
    let zero =
        apply_import_convention(&scenario, &inputs, &ImportConvention::ZeroInSurplus).unwrap();
    assert_eq!(import_values(&zero, "imports_a"), [0.0, 1.5]);
    assert_eq!(import_values(&zero, "imports_b"), [0.0, 0.25]);
    let export = apply_import_convention(&scenario, &inputs, &export_convention()).unwrap();
    assert_eq!(import_values(&export, "imports_a"), [-2.5, 1.5]);
    assert_eq!(import_values(&export, "imports_b"), [0.0, 0.25]);
}

// ---------------------------------------------------------------------
// Property test (requirement 2, generalised): variants alter only
// strict-surplus periods; the export value equals −min(cap, s) exactly.
// ---------------------------------------------------------------------

proptest! {
    #[test]
    fn variants_alter_only_surplus_periods_and_respect_both_caps(
        demand in prop::collection::vec(0.0f64..50.0, 1..96),
        cf_seed in prop::collection::vec(0.0f64..1.0, 96),
        imports_seed in prop::collection::vec(-5.0f64..5.0, 96),
        other_seed in prop::collection::vec(0.0f64..3.0, 96),
        cap in 0.0f64..10.0,
    ) {
        let n = demand.len();
        let cf = &cf_seed[..n];
        let imports = &imports_seed[..n];
        let other = &other_seed[..n];
        let scenario = scenario_with(
            vec![renewable("onshore_wind", 25.0), thermal("ccgt", 60.0)],
            vec![],
            n,
        );
        let inputs = run_inputs(
            &demand,
            &[("onshore_wind", cf)],
            vec![
                exo("net_imports", true, imports),
                exo("other", false, other),
            ],
        );
        let zero =
            apply_import_convention(&scenario, &inputs, &ImportConvention::ZeroInSurplus).unwrap();
        let export = apply_import_convention(
            &scenario,
            &inputs,
            &ImportConvention::ExportInSurplus {
                export_capacity: Power::gigawatts(cap),
            },
        )
        .unwrap();
        let zero_values = import_values(&zero, "net_imports");
        let export_values = import_values(&export, "net_imports");
        for t in 0..n {
            let s = 25.0 * cf[t] + other[t] - demand[t];
            if s > 0.0 {
                prop_assert_eq!(zero_values[t], 0.0);
                prop_assert_eq!(export_values[t], -s.min(cap));
                // Both caps respected: never beyond capacity, never
                // beyond the surplus magnitude.
                prop_assert!(export_values[t] >= -cap);
                prop_assert!(export_values[t] >= -s);
            } else {
                // Non-surplus periods bit-identical to frozen.
                prop_assert_eq!(zero_values[t], imports[t]);
                prop_assert_eq!(export_values[t], imports[t]);
            }
        }
        // The non-import series is untouched by both variants.
        prop_assert_eq!(import_values(&zero, "other"), other.to_vec());
        prop_assert_eq!(import_values(&export, "other"), other.to_vec());
    }
}

// ---------------------------------------------------------------------
// Export capability from the scenario's links.
// ---------------------------------------------------------------------

#[test]
fn link_export_capability_sums_derated_capacity_of_links_touching_the_zone() {
    let link = |from: &str, to: &str, cap: f64, avail: f64| LinkSpec {
        name: None,
        from: ZoneId::new(from),
        to: ZoneId::new(to),
        capacity_gw: Power::gigawatts(cap),
        reverse_capacity_gw: None,
        capability_trace: None,
        availability: PerUnit::new(avail),
        loss: PerUnit::new(0.0),
    };
    // 2.0 × 0.95 + 0.5 × 0.0 (commissioning link contributes nothing)
    // + 1.0 × 1.0 (reverse-declared link still touches the zone).
    let scenario = scenario_with(
        vec![renewable("onshore_wind", 10.0)],
        vec![
            link("GB", "FR", 2.0, 0.95),
            link("GB", "SEM", 0.5, 0.0),
            link("FR", "GB", 1.0, 1.0),
        ],
        2,
    );
    let capability = link_export_capability(&scenario).unwrap();
    assert!((capability.unwrap().as_gigawatts() - 2.9).abs() < 1e-12);

    // No links → None (the CLI must then demand an explicit parameter —
    // no silent default).
    let bare = scenario_with(vec![renewable("onshore_wind", 10.0)], vec![], 2);
    assert_eq!(link_export_capability(&bare).unwrap(), None);
}
