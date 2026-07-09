//! Reliable/unreliable generation classification (the owner's
//! gb-grid-margin methodology, implemented exactly as published:
//! binary, correlated-failure criterion, no derating; storage discharge
//! is its own fourth category, never folded into firm).
//!
//! - Derived defaults reproduce the published roster per technology.
//! - Explicit overrides are respected AND surfaced in the result.
//! - Partition property: firm + variable + storage discharge +
//!   excluded == total supply, every period.
//! - `firm_share` is unclamped (net-export periods exceed 1.0).
//! - Classification is pure accounting: it cannot perturb dispatch.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{ExogenousSupply, RunInputs, run};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, ExogenousReliability, FleetEntry, Horizon,
    Reliability, Scenario, StorageKind, StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, PerUnit, Power};

const START: &str = "2024-01-01T00:00:00Z";

fn fleet_entry(
    tech: &str,
    capacity: f64,
    cf: bool,
    reliability: Option<Reliability>,
) -> FleetEntry {
    FleetEntry {
        technology: TechId::new(tech),
        capacity_gw: Power::gigawatts(capacity),
        capacity_factor_trace: cf.then(|| format!("synthetic/{tech}.parquet").into()),
        availability: None,
        reliability,
        inertia_h: None,
        synchronous: None,
        energy_budget: None,
    }
}

fn scenario(fleet: Vec<FleetEntry>, storage: Vec<StorageSpec>, periods: usize) -> Scenario {
    let start = UtcInstant::parse(START).unwrap();
    Scenario {
        schema_version: 5,
        name: "reliability-synthetic".to_owned(),
        description: None,
        horizon: Horizon {
            start: START.to_owned(),
            end: start.plus_periods(periods as i64 - 1).to_string(),
            weather_years: WeatherYears::Years(vec![2024]),
        },
        zones: vec![ZoneSpec {
            pricing: None,
            id: ZoneId::new("GB"),
            demand: DemandSpec {
                base_profile: "unused".into(),
                column: "underlying_demand".to_owned(),
                extra_profiles: vec![],
                annual_scale: 1.0,
                extra_demand_gw: Power::gigawatts(0.0),
                heating: None,
            },
            exogenous_supply: vec![],
            fleet,
            storage,
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

fn trace_gw(values: &[f64]) -> Trace<Power> {
    Trace::from_parts(
        UtcInstant::parse(START).unwrap(),
        values.iter().map(|&v| Power::gigawatts(v)).collect(),
    )
    .unwrap()
}

fn inputs(demand_gw: &[f64], cf: &[(&str, &[f64])], exogenous: Vec<ExogenousSupply>) -> RunInputs {
    RunInputs {
        demand: trace_gw(demand_gw),
        capacity_factors: cf
            .iter()
            .map(|(tech, values)| {
                (
                    TechId::new(*tech),
                    Trace::from_parts(
                        UtcInstant::parse(START).unwrap(),
                        values.iter().map(|&v| PerUnit::new(v)).collect(),
                    )
                    .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
        exogenous,
        availability: BTreeMap::new(),
        heating: None,
    }
}

fn exo(label: &str, values_gw: &[f64], reliability: ExogenousReliability) -> ExogenousSupply {
    ExogenousSupply {
        label: label.to_owned(),
        imports: reliability == ExogenousReliability::Variable,
        reliability,
        trace: trace_gw(values_gw),
    }
}

/// The published roster, reproduced by the derived defaults per
/// technology: firm = ccgt, ocgt, nuclear, biomass, hydro, coal, oil,
/// other; variable = wind (on+offshore), solar.
#[test]
fn derived_defaults_match_the_published_roster() {
    for tech in [
        "ccgt", "ocgt", "nuclear", "biomass", "hydro", "coal", "oil", "other",
    ] {
        let entry = fleet_entry(tech, 1.0, false, None);
        assert_eq!(
            entry.effective_reliability(),
            Reliability::Firm,
            "{tech} should derive firm"
        );
        assert!(!entry.reliability_overridden());
    }
    for tech in ["offshore_wind", "onshore_wind", "solar"] {
        let entry = fleet_entry(tech, 1.0, true, None);
        assert_eq!(
            entry.effective_reliability(),
            Reliability::Variable,
            "{tech} should derive variable"
        );
        assert!(!entry.reliability_overridden());
    }
}

/// An explicit override is respected in the accounting AND surfaced in
/// the result series, so it cannot hide.
#[test]
fn overrides_are_respected_and_surfaced() {
    // Biomass forced variable; wind forced firm (both overrides).
    let s = scenario(
        vec![
            fleet_entry("biomass", 4.0, false, Some(Reliability::Variable)),
            fleet_entry("onshore_wind", 10.0, true, Some(Reliability::Firm)),
            fleet_entry("ccgt", 20.0, false, None),
        ],
        vec![],
        1,
    );
    let result = run(&s, &inputs(&[10.0], &[("onshore_wind", &[0.5])], vec![])).unwrap();

    let wind = &result.renewables[0];
    assert_eq!(wind.reliability, Reliability::Firm);
    assert!(wind.reliability_overridden);
    let biomass = result
        .thermal
        .iter()
        .find(|t| t.tech.as_str() == "biomass")
        .unwrap();
    assert_eq!(biomass.reliability, Reliability::Variable);
    assert!(biomass.reliability_overridden);
    let ccgt = result
        .thermal
        .iter()
        .find(|t| t.tech.as_str() == "ccgt")
        .unwrap();
    assert!(!ccgt.reliability_overridden);

    // And the buckets follow the overrides: wind (5 GW potential) is
    // firm; biomass output is variable.
    let firm = result.firm_supply();
    let variable = result.variable_supply();
    // Demand 10: wind 5 (firm), then biomass 4 + ccgt 1 dispatch.
    assert!((firm[0].as_gigawatts() - (5.0 + 1.0)).abs() < 1e-12);
    assert!((variable[0].as_gigawatts() - 4.0).abs() < 1e-12);
}

/// Partition property: every supply series lands in exactly one of
/// firm / variable / storage-discharge / excluded, so the four sum to
/// total supply in every period.
#[test]
fn firm_variable_storage_excluded_partition_total_supply() {
    let store = StorageSpec {
        kind: StorageKind::Battery,
        power_gw: Power::gigawatts(3.0),
        energy_gwh: Energy::gigawatt_hours(10.0),
        round_trip_efficiency: PerUnit::new(1.0),
        dispatch_order: 1,
        initial_soc: Some(PerUnit::new(1.0)),
        shift_duration: None,
        daily_volume_limit: None,
    };
    let s = scenario(
        vec![
            fleet_entry("onshore_wind", 20.0, true, None),
            fleet_entry("nuclear", 5.0, false, None),
            fleet_entry("ccgt", 6.0, false, None),
        ],
        vec![store],
        4,
    );
    // Periods: surplus, deficit-with-discharge, balanced, deep deficit.
    let demand = [8.0, 16.0, 10.0, 40.0];
    let wind_cf = [0.6, 0.1, 0.2, 0.0];
    let exogenous = vec![
        exo(
            "net_imports",
            &[2.0, -1.0, 1.5, 2.0],
            ExogenousReliability::Variable,
        ),
        exo("other", &[0.5, 0.5, 0.5, 0.5], ExogenousReliability::Firm),
        exo(
            "pumped_storage_net",
            &[-0.5, 1.0, 0.0, 0.7],
            ExogenousReliability::Excluded,
        ),
    ];
    let result = run(
        &s,
        &inputs(&demand, &[("onshore_wind", &wind_cf)], exogenous),
    )
    .unwrap();

    let firm = result.firm_supply();
    let variable = result.variable_supply();
    let storage = result.storage_discharge();
    for t in 0..demand.len() {
        let excluded: f64 = result
            .exogenous
            .iter()
            .filter(|s| s.reliability == ExogenousReliability::Excluded)
            .map(|s| s.power[t].as_gigawatts())
            .sum();
        let total: f64 = result
            .renewables
            .iter()
            .chain(&result.thermal)
            .map(|s| s.power[t].as_gigawatts())
            .sum::<f64>()
            + result
                .exogenous
                .iter()
                .map(|s| s.power[t].as_gigawatts())
                .sum::<f64>()
            + storage[t].as_gigawatts();
        let parts = firm[t].as_gigawatts()
            + variable[t].as_gigawatts()
            + storage[t].as_gigawatts()
            + excluded;
        assert!(
            (parts - total).abs() <= 1e-12 * total.abs().max(1.0),
            "period {t}: partition {parts} != total supply {total}"
        );
    }
    // Sanity: the store really discharged somewhere (the fourth
    // category is exercised, not vacuously zero).
    assert!(storage.iter().any(|&p| p > Power::gigawatts(0.0)));
}

/// The headline metric is UNCLAMPED: with firm supply above demand
/// (net-export conditions) the share exceeds 1.0 and is reported as is.
#[test]
fn firm_share_is_unclamped_and_stats_are_consistent() {
    let s = scenario(
        vec![
            fleet_entry("nuclear", 5.0, false, None),
            fleet_entry("ccgt", 30.0, false, None),
        ],
        vec![],
        2,
    );
    // Exports of 5 GW in period 0: the stack serves demand + export, so
    // firm supply = 10 + 5 = 15 GW against 10 GW demand → share 1.5.
    let exogenous = vec![exo(
        "net_imports",
        &[-5.0, 5.0],
        ExogenousReliability::Variable,
    )];
    let result = run(&s, &inputs(&[10.0, 10.0], &[], exogenous)).unwrap();
    let shares = result.firm_share();
    assert!((shares[0] - 1.5).abs() < 1e-12, "share {shares:?}");
    assert!((shares[1] - 0.5).abs() < 1e-12, "share {shares:?}");
    let stats = result.firm_share_stats().unwrap();
    assert!((stats.mean - 1.0).abs() < 1e-12);
    assert!((stats.min - 0.5).abs() < 1e-12);
    assert!((stats.p25 - 0.75).abs() < 1e-12, "p25 {}", stats.p25);
    // share == 0.5 is NOT below the strict 0.5 threshold.
    assert_eq!(stats.below_threshold, 0);
}
