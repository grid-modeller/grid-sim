//! Regression for the multi-zone twin of the deficit-charge physical
//! leak (`ZoneEngine::dispatch_period`): a contract-relaxed policy that
//! charges a store in a deficit period the stack cannot back must be
//! rejected, not silently recorded as risen SoC + phantom `unserved`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use grid_adequacy::{
    DispatchDecision, DispatchPolicy, MultiZoneInputs, PolicyContract, RunInputs, StoreAction,
    SystemState, ZoneInputs, run_multi_with_policy,
};
use grid_core::GridError;
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, Scenario, StorageKind,
    StorageSpec, TechId, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Duration, Energy, PerUnit, Power};

const START: &str = "2024-01-01T00:00:00Z";

/// Charge every store at max feasible power each period, never discharge,
/// with BOTH policy-tier obligations relaxed.
struct ChargeAlways;

impl DispatchPolicy for ChargeAlways {
    fn dispatch(&self, state: &SystemState<'_>, _horizon: &Horizon) -> DispatchDecision {
        let dt = Duration::half_hour();
        DispatchDecision {
            actions: state
                .stores
                .iter()
                .map(|s| StoreAction {
                    charge: s.max_charge(dt),
                    discharge: Power::gigawatts(0.0),
                })
                .collect(),
        }
    }

    fn contract(&self) -> PolicyContract {
        PolicyContract {
            charge_from_surplus_only: false,
            discharge_after_stack_only: false,
        }
    }
}

#[test]
fn multizone_deficit_charge_with_no_supply_is_rejected() {
    // One zone, ccgt 0 GW (stack supplies nothing), demand 10 GW → a pure
    // deficit, an empty 5 GW / 100 GWh battery. ChargeAlways tries to
    // charge 5 GW with no supply to back it.
    let scenario = Scenario {
        schema_version: 6,
        name: "mz-deficit-charge".to_owned(),
        description: None,
        horizon: Horizon {
            start: START.to_owned(),
            end: START.to_owned(),
            weather_years: WeatherYears::Years(vec![2024]),
        },
        zones: vec![ZoneSpec {
            pricing: None,
            id: ZoneId::new("Z"),
            demand: DemandSpec {
                base_profile: "unused".into(),
                column: "underlying_demand".to_owned(),
                extra_profiles: vec![],
                annual_scale: 1.0,
                extra_demand_gw: Power::gigawatts(0.0),
                heating: None,
            },
            exogenous_supply: vec![],
            fleet: vec![FleetEntry {
                technology: TechId::new("ccgt"),
                capacity_gw: Power::gigawatts(0.0),
                capacity_factor_trace: None,
                availability: None,
                reliability: None,
                inertia_h: None,
                synchronous: None,
                energy_budget: None,
            }],
            storage: vec![StorageSpec {
                kind: StorageKind::Battery,
                power_gw: Power::gigawatts(5.0),
                energy_gwh: Energy::gigawatt_hours(100.0),
                round_trip_efficiency: PerUnit::new(1.0),
                dispatch_order: 1,
                initial_soc: Some(PerUnit::new(0.0)),
                shift_duration: None,
                daily_volume_limit: None,
            }],
        }],
        links: vec![],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    };
    let inputs = MultiZoneInputs {
        zones: vec![ZoneInputs {
            pricing: None,
            id: ZoneId::new("Z"),
            inputs: RunInputs {
                demand: Trace::from_parts(
                    UtcInstant::parse(START).unwrap(),
                    vec![Power::gigawatts(10.0)],
                )
                .unwrap(),
                capacity_factors: BTreeMap::new(),
                exogenous: vec![],
                availability: BTreeMap::new(),
                heating: None,
            },
            budgets: BTreeMap::new(),
        }],
        link_capabilities: vec![],
    };

    let err = run_multi_with_policy(&scenario, &inputs, &ChargeAlways).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidDispatchDecision { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("unserved"), "message: {err}");
}
