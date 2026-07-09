//! Stage 1 engine unit and property tests on synthetic scenarios:
//! per-period energy balance (proptest), merit-order invariants,
//! availability clamps, and the engine's input-validation errors.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{AvailabilityModel, ExogenousSupply, MERIT_ORDER, RunInputs, RunResult, run};
use grid_core::GridError;
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, Scenario, TechId, WeatherYears,
    ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{PerUnit, Power};
use proptest::prelude::*;

const START: &str = "2024-01-01T00:00:00Z";

/// A single-zone scenario with the given fleet, one demand period per
/// entry of the caller's traces.
fn scenario(fleet: Vec<FleetEntry>, periods: usize) -> Scenario {
    let start = UtcInstant::parse(START).unwrap();
    let end = start.plus_periods(periods as i64 - 1);
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
        links: vec![],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
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

fn power_trace(values: &[f64]) -> Trace<Power> {
    Trace::from_parts(
        UtcInstant::parse(START).unwrap(),
        values.iter().map(|&v| Power::gigawatts(v)).collect(),
    )
    .unwrap()
}

fn cf_trace(values: &[f64]) -> Trace<PerUnit> {
    Trace::from_parts(
        UtcInstant::parse(START).unwrap(),
        values.iter().map(|&v| PerUnit::new(v)).collect(),
    )
    .unwrap()
}

/// Run inputs with the given demand (GW per period), CF traces per
/// renewable tech, availability per thermal tech, and exogenous supplies.
fn inputs(
    demand_gw: &[f64],
    cf: &[(&str, &[f64])],
    availability: &[(&str, AvailabilityModel)],
    exogenous: Vec<ExogenousSupply>,
) -> RunInputs {
    RunInputs {
        demand: power_trace(demand_gw),
        capacity_factors: cf
            .iter()
            .map(|(tech, values)| (TechId::new(*tech), cf_trace(values)))
            .collect::<BTreeMap<_, _>>(),
        exogenous,
        availability: availability
            .iter()
            .map(|(tech, model)| (TechId::new(*tech), model.clone()))
            .collect(),
        heating: None,
    }
}

fn exo(label: &str, values_gw: &[f64], imports: bool) -> ExogenousSupply {
    ExogenousSupply {
        label: label.to_owned(),
        imports,
        // Classification is irrelevant to dispatch behaviour (pure
        // accounting); imports are variable per the methodology.
        reliability: if imports {
            grid_core::scenario::ExogenousReliability::Variable
        } else {
            grid_core::scenario::ExogenousReliability::Firm
        },
        trace: power_trace(values_gw),
    }
}

fn thermal_series<'r>(result: &'r RunResult, tech: &str) -> &'r [Power] {
    &result
        .thermal
        .iter()
        .find(|s| s.tech.as_str() == tech)
        .unwrap()
        .power
}

// ---------------------------------------------------------------------
// Hand-checkable dispatch cases.
// ---------------------------------------------------------------------

#[test]
fn cheapest_technology_serves_first_and_gas_takes_the_residual() {
    // Demand 10 GW; nuclear 4 GW available, ccgt 30 GW: nuclear runs at
    // its cap, ccgt takes the remaining 6 GW.
    let s = scenario(vec![thermal("nuclear", 4.0), thermal("ccgt", 30.0)], 2);
    let result = run(&s, &inputs(&[10.0, 10.0], &[], &[], vec![])).unwrap();
    assert_eq!(
        thermal_series(&result, "nuclear"),
        &[Power::gigawatts(4.0); 2]
    );
    assert_eq!(thermal_series(&result, "ccgt"), &[Power::gigawatts(6.0); 2]);
    assert_eq!(
        result.total_unserved(),
        grid_core::units::Energy::gigawatt_hours(0.0)
    );
    assert_eq!(
        result.total_curtailment(),
        grid_core::units::Energy::gigawatt_hours(0.0)
    );
}

#[test]
fn renewables_are_must_take_and_surplus_is_curtailed() {
    // Wind potential 12 GW against 10 GW demand: 2 GW curtailed, no
    // thermal runs.
    let s = scenario(
        vec![renewable("onshore_wind", 20.0), thermal("ccgt", 30.0)],
        1,
    );
    let result = run(
        &s,
        &inputs(&[10.0], &[("onshore_wind", &[0.6])], &[], vec![]),
    )
    .unwrap();
    assert_eq!(result.curtailment, vec![Power::gigawatts(2.0)]);
    assert_eq!(thermal_series(&result, "ccgt"), &[Power::gigawatts(0.0)]);
    // The renewable series reports potential (pre-curtailment) output.
    assert_eq!(result.renewables[0].power, vec![Power::gigawatts(12.0)]);
}

#[test]
fn unserved_energy_appears_when_the_stack_is_exhausted() {
    let s = scenario(vec![thermal("ccgt", 5.0)], 1);
    let result = run(&s, &inputs(&[8.0], &[], &[], vec![])).unwrap();
    assert_eq!(result.unserved, vec![Power::gigawatts(3.0)]);
    assert_eq!(
        result.total_unserved(),
        grid_core::units::Energy::gigawatt_hours(1.5)
    );
}

#[test]
fn negative_exogenous_supply_adds_to_the_residual() {
    // Net exports of 2 GW on 10 GW demand: thermal must serve 12 GW.
    let s = scenario(vec![thermal("ccgt", 30.0)], 1);
    let result = run(
        &s,
        &inputs(&[10.0], &[], &[], vec![exo("net_imports", &[-2.0], true)]),
    )
    .unwrap();
    assert_eq!(thermal_series(&result, "ccgt"), &[Power::gigawatts(12.0)]);
    assert_eq!(
        result.net_imports_energy(),
        grid_core::units::Energy::gigawatt_hours(-1.0)
    );
}

#[test]
fn imports_accounting_only_counts_flagged_series() {
    let s = scenario(vec![thermal("ccgt", 30.0)], 1);
    let result = run(
        &s,
        &inputs(
            &[10.0],
            &[],
            &[],
            vec![
                exo("net_imports", &[3.0], true),
                exo("other", &[1.0], false),
            ],
        ),
    )
    .unwrap();
    assert_eq!(
        result.net_imports_energy(),
        grid_core::units::Energy::gigawatt_hours(1.5)
    );
}

// ---------------------------------------------------------------------
// Availability models.
// ---------------------------------------------------------------------

#[test]
fn flat_availability_caps_output() {
    let s = scenario(vec![thermal("biomass", 4.0), thermal("ccgt", 30.0)], 1);
    let result = run(
        &s,
        &inputs(
            &[10.0],
            &[],
            &[(
                "biomass",
                AvailabilityModel::flat(PerUnit::new(0.5)).unwrap(),
            )],
            vec![],
        ),
    )
    .unwrap();
    assert_eq!(thermal_series(&result, "biomass"), &[Power::gigawatts(2.0)]);
    assert_eq!(thermal_series(&result, "ccgt"), &[Power::gigawatts(8.0)]);
}

#[test]
fn monthly_availability_follows_the_calendar_month() {
    // Two periods a month apart via a 31-day horizon: January factor 1.0,
    // February 0.0 (a coal-closure-style window).
    let periods = 31 * 48 + 1; // Jan 1 00:00 .. Feb 1 00:00 inclusive
    let mut monthly = [PerUnit::new(0.0); 12];
    monthly[0] = PerUnit::new(1.0);
    let s = scenario(vec![thermal("coal", 2.0), thermal("ccgt", 30.0)], periods);
    let demand = vec![10.0; periods];
    let result = run(
        &s,
        &inputs(
            &demand,
            &[],
            &[("coal", AvailabilityModel::monthly(monthly).unwrap())],
            vec![],
        ),
    )
    .unwrap();
    let coal = thermal_series(&result, "coal");
    assert_eq!(
        coal[0],
        Power::gigawatts(2.0),
        "January: full window factor"
    );
    assert_eq!(coal[periods - 1], Power::gigawatts(0.0), "February: closed");
}

#[test]
fn availability_factors_outside_zero_one_are_rejected() {
    assert!(matches!(
        AvailabilityModel::flat(PerUnit::new(1.2)),
        Err(GridError::InvalidRunInputs { .. })
    ));
    assert!(matches!(
        AvailabilityModel::flat(PerUnit::new(-0.1)),
        Err(GridError::InvalidRunInputs { .. })
    ));
    let mut monthly = [PerUnit::new(0.5); 12];
    monthly[3] = PerUnit::new(1.01);
    assert!(AvailabilityModel::monthly(monthly).is_err());
}

// ---------------------------------------------------------------------
// Engine input validation.
// ---------------------------------------------------------------------

#[test]
fn more_than_one_zone_is_rejected_with_a_clear_error() {
    let mut s = scenario(vec![thermal("ccgt", 30.0)], 1);
    let mut second = s.zones[0].clone();
    second.id = ZoneId::new("FR");
    s.zones.push(second);
    let err = run(&s, &inputs(&[10.0], &[], &[], vec![])).unwrap_err();
    match err {
        GridError::MultiZoneUnsupported { found } => assert_eq!(found, 2),
        other => panic!("unexpected error: {other:?}"),
    }
    // The message must say what to do about it (ADR-7 / Stage 5).
    let msg = GridError::MultiZoneUnsupported { found: 2 }.to_string();
    assert!(msg.contains("single-zone"), "message was: {msg}");
    assert!(msg.contains("Stage 5"), "message was: {msg}");
}

#[test]
fn a_thermal_technology_outside_the_merit_order_is_rejected() {
    let s = scenario(vec![thermal("fusion", 10.0)], 1);
    let err = run(&s, &inputs(&[10.0], &[], &[], vec![])).unwrap_err();
    match err {
        GridError::UnknownThermalTechnology { ref tech } => assert_eq!(tech, "fusion"),
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(err.to_string().contains("fusion"));
}

#[test]
fn a_renewable_without_a_loaded_cf_trace_is_rejected() {
    let s = scenario(vec![renewable("solar", 10.0)], 1);
    // No CF trace supplied in the inputs.
    let err = run(&s, &inputs(&[10.0], &[], &[], vec![])).unwrap_err();
    assert!(err.to_string().contains("solar"), "error: {err}");
}

#[test]
fn mismatched_trace_lengths_are_rejected() {
    let s = scenario(vec![renewable("solar", 10.0), thermal("ccgt", 30.0)], 2);
    let err = run(
        &s,
        &inputs(&[10.0, 10.0], &[("solar", &[0.5])], &[], vec![]),
    )
    .unwrap_err();
    assert!(
        matches!(err, GridError::InvalidRunInputs { .. }),
        "unexpected error: {err:?}"
    );
}

#[test]
fn perfect_foresight_policy_is_rejected_in_stage_1() {
    let mut s = scenario(vec![thermal("ccgt", 30.0)], 1);
    s.dispatch.policy = DispatchPolicyKind::PerfectForesight;
    let err = run(&s, &inputs(&[10.0], &[], &[], vec![])).unwrap_err();
    assert!(
        err.to_string().contains("perfect_foresight"),
        "error: {err}"
    );
}

// ---------------------------------------------------------------------
// Property tests.
// ---------------------------------------------------------------------

/// Strategy: a small random system — demand, wind CF, exogenous flows,
/// availabilities — over a two-day horizon.
fn arb_system() -> impl Strategy<
    Value = (
        Vec<f64>, // demand GW
        Vec<f64>, // wind cf
        Vec<f64>, // exogenous GW (may be negative)
        f64,      // wind capacity
        f64,      // nuclear capacity
        f64,      // ccgt capacity
        f64,      // nuclear flat availability
    ),
> {
    let n = 96usize;
    (
        prop::collection::vec(0.0f64..80.0, n),
        prop::collection::vec(0.0f64..=1.0, n),
        prop::collection::vec(-10.0f64..10.0, n),
        0.0f64..60.0,
        0.0f64..12.0,
        0.0f64..40.0,
        0.0f64..=1.0,
    )
}

proptest! {
    /// Per-period energy balance: must-take + thermal + unserved =
    /// demand + curtailment (docs/04 Stage 1; exact up to f64 rounding).
    #[test]
    fn energy_balance_holds_every_period(
        (demand, wind_cf, exo_gw, wind_cap, nuc_cap, ccgt_cap, nuc_avail) in arb_system()
    ) {
        let n = demand.len();
        let s = scenario(
            vec![
                renewable("onshore_wind", wind_cap),
                thermal("nuclear", nuc_cap),
                thermal("ccgt", ccgt_cap),
            ],
            n,
        );
        let result = run(
            &s,
            &inputs(
                &demand,
                &[("onshore_wind", &wind_cf)],
                &[("nuclear", AvailabilityModel::flat(PerUnit::new(nuc_avail)).unwrap())],
                vec![exo("net_imports", &exo_gw, true)],
            ),
        )
        .unwrap();

        for t in 0..n {
            let must_take: f64 = result.renewables.iter().map(|r| r.power[t].as_gigawatts()).sum::<f64>()
                + result.exogenous.iter().map(|e| e.power[t].as_gigawatts()).sum::<f64>();
            let thermal: f64 = result.thermal.iter().map(|s| s.power[t].as_gigawatts()).sum();
            let lhs = must_take + thermal + result.unserved[t].as_gigawatts();
            let rhs = result.demand[t].as_gigawatts() + result.curtailment[t].as_gigawatts();
            prop_assert!(
                (lhs - rhs).abs() <= 1e-9 * rhs.abs().max(1.0),
                "period {t}: supply+unserved {lhs} != demand+curtailment {rhs}"
            );
        }
    }

    /// Merit-order invariant: a more expensive technology never runs
    /// while a cheaper one has headroom.
    #[test]
    fn expensive_plant_never_runs_while_cheaper_has_headroom(
        (demand, wind_cf, exo_gw, wind_cap, nuc_cap, ccgt_cap, nuc_avail) in arb_system()
    ) {
        let n = demand.len();
        let s = scenario(
            vec![
                renewable("onshore_wind", wind_cap),
                thermal("nuclear", nuc_cap),
                thermal("ccgt", ccgt_cap),
            ],
            n,
        );
        let availability = [
            ("nuclear", AvailabilityModel::flat(PerUnit::new(nuc_avail)).unwrap()),
        ];
        let result = run(
            &s,
            &inputs(&demand, &[("onshore_wind", &wind_cf)], &availability, vec![exo("x", &exo_gw, false)]),
        )
        .unwrap();

        let nuclear = thermal_series(&result, "nuclear");
        let ccgt = thermal_series(&result, "ccgt");
        for t in 0..n {
            let nuclear_cap_t = nuc_cap * nuc_avail;
            if ccgt[t].as_gigawatts() > 1e-12 {
                prop_assert!(
                    nuclear[t].as_gigawatts() >= nuclear_cap_t - 1e-9,
                    "period {t}: ccgt runs at {} GW while nuclear sits at {} of {} GW",
                    ccgt[t].as_gigawatts(), nuclear[t].as_gigawatts(), nuclear_cap_t
                );
            }
        }
    }

    /// Availability clamp: every thermal output is within
    /// [0, capacity × availability(t)]; curtailment and unserved are
    /// non-negative and never coincide.
    #[test]
    fn outputs_respect_availability_clamps(
        (demand, wind_cf, exo_gw, wind_cap, nuc_cap, ccgt_cap, nuc_avail) in arb_system()
    ) {
        let n = demand.len();
        let s = scenario(
            vec![
                renewable("onshore_wind", wind_cap),
                thermal("nuclear", nuc_cap),
                thermal("ccgt", ccgt_cap),
            ],
            n,
        );
        let availability = [
            ("nuclear", AvailabilityModel::flat(PerUnit::new(nuc_avail)).unwrap()),
        ];
        let result = run(
            &s,
            &inputs(&demand, &[("onshore_wind", &wind_cf)], &availability, vec![exo("x", &exo_gw, false)]),
        )
        .unwrap();

        let nuclear = thermal_series(&result, "nuclear");
        let ccgt = thermal_series(&result, "ccgt");
        for t in 0..n {
            prop_assert!(nuclear[t].as_gigawatts() >= 0.0);
            prop_assert!(nuclear[t].as_gigawatts() <= nuc_cap * nuc_avail + 1e-12);
            prop_assert!(ccgt[t].as_gigawatts() >= 0.0);
            prop_assert!(ccgt[t].as_gigawatts() <= ccgt_cap + 1e-12);
            prop_assert!(result.curtailment[t].as_gigawatts() >= 0.0);
            prop_assert!(result.unserved[t].as_gigawatts() >= 0.0);
            prop_assert!(
                result.curtailment[t].as_gigawatts() == 0.0
                    || result.unserved[t].as_gigawatts() == 0.0,
                "period {t}: simultaneous curtailment and unserved energy"
            );
        }
    }
}

// ---------------------------------------------------------------------
// Merit order is what the module documents.
// ---------------------------------------------------------------------

#[test]
fn merit_order_is_the_documented_stack() {
    // Stage 1 pinned the six-rung 2024 stack; Stage 7 (published-pathway
    // scenarios) extends the ladder with the pathway technologies. The
    // RELATIVE order of the original six rungs is unchanged — the
    // committed 2024/2/3/5/8-zone digests depend only on that relative
    // order and are re-verified unmoved by the regression suite.
    assert_eq!(
        MERIT_ORDER,
        [
            "nuclear",
            "biomass",
            "beccs",
            "waste",
            "other_generation",
            "hydro",
            "coal",
            "ccgt_ccs",
            "low_carbon_dispatchable",
            "ccgt",
            "ocgt",
            "oil",
            "hydrogen_turbine",
        ]
    );
    // The Stage 1 relative order, asserted directly.
    let position = |tech: &str| MERIT_ORDER.iter().position(|t| *t == tech).unwrap();
    let stage1 = ["nuclear", "biomass", "hydro", "coal", "ccgt", "ocgt"];
    for pair in stage1.windows(2) {
        assert!(
            position(pair[0]) < position(pair[1]),
            "Stage 1 relative order broken: {} must precede {}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn stage7_pathway_thermal_rungs_are_dispatchable() {
    // Before Stage 7 these ids were UnknownThermalTechnology; the
    // published-pathway scenarios (FES EE, CCC BP) need them.
    for tech in [
        "beccs",
        "waste",
        "other_generation",
        "ccgt_ccs",
        "low_carbon_dispatchable",
        "oil",
        "hydrogen_turbine",
    ] {
        let s = scenario(vec![thermal(tech, 5.0)], 1);
        let result = run(&s, &inputs(&[3.0], &[], &[], vec![]))
            .unwrap_or_else(|e| panic!("{tech} should dispatch: {e}"));
        assert_eq!(thermal_series(&result, tech), &[Power::gigawatts(3.0)]);
    }
}

#[test]
fn stage7_pathway_rungs_dispatch_in_documented_order() {
    // 2 GW each of ccgt_ccs → ccgt → ocgt → oil → hydrogen_turbine.
    // Demand 5 GW: CCS gas and CCGT run full, OCGT takes 1 GW, oil and
    // hydrogen stay cold. Demand 9 GW: oil runs full, hydrogen takes
    // the final 1 GW (the last rung before unserved).
    let fleet = vec![
        thermal("ccgt_ccs", 2.0),
        thermal("ccgt", 2.0),
        thermal("ocgt", 2.0),
        thermal("oil", 2.0),
        thermal("hydrogen_turbine", 2.0),
    ];
    let s = scenario(fleet, 2);
    let result = run(&s, &inputs(&[5.0, 9.0], &[], &[], vec![])).unwrap();
    let gw = |v: f64| Power::gigawatts(v);
    assert_eq!(thermal_series(&result, "ccgt_ccs"), &[gw(2.0), gw(2.0)]);
    assert_eq!(thermal_series(&result, "ccgt"), &[gw(2.0), gw(2.0)]);
    assert_eq!(thermal_series(&result, "ocgt"), &[gw(1.0), gw(2.0)]);
    assert_eq!(thermal_series(&result, "oil"), &[gw(0.0), gw(2.0)]);
    assert_eq!(
        thermal_series(&result, "hydrogen_turbine"),
        &[gw(0.0), gw(1.0)]
    );
    assert_eq!(
        result.total_unserved(),
        grid_core::units::Energy::gigawatt_hours(0.0)
    );
}
