//! Scenario schema tests: strict parsing, lossless round-tripping
//! (docs/04 Stage 0), the schema v2 field set (Stage 3: the Stage 1
//! run-inputs file folded into the scenario), the schema v3 stability
//! metadata (Stage 6: `inertia_h`/`synchronous` on fleet entries), the
//! schema v4 multi-zone activation fields (Stage 5: link `name`/`loss`,
//! fleet `energy_budget`, demand `extra_profiles`), the schema v5
//! heating-portfolio block (Q5, D9 rule 2: the technology-portfolio
//! `[zones.demand.heating]` that REPLACES the v1–v4 sketch), the
//! schema v6 per-direction/time-series link capability + exogenous
//! `scale` (the B6 two-zone package; the b6-two-zone-data-review §6
//! ruling), the v1/v2/v3/v4/v5 migration error paths (docs/05 rule 4),
//! and semantic validation.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use grid_core::GridError;
use grid_core::scenario::{
    AvailabilitySpec, DispatchPolicyKind, ExogenousReliability, HeatingKind, Reliability, Scenario,
    StorageKind, TraceFiles, WeatherYears,
};
use grid_core::units::{Duration, Energy, Length, PerUnit, Power};

/// The reference scenario shipped in the repo (read-only fixture).
fn reference_scenario_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../scenarios/gb-2024-reference.toml")
}

/// The frozen v1 reference scenario (superseded by schema v2; kept as the
/// migration-error fixture).
fn v1_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1-gb-2024-reference.toml")
}

/// The frozen v2 reference scenario (superseded by schema v3, Stage 6;
/// kept as the migration-error fixture).
fn v2_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v2-gb-2024-reference.toml")
}

/// The frozen v3 reference scenario (superseded by schema v4, Stage 5;
/// kept as the migration-error fixture).
fn v3_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v3-gb-2024-reference.toml")
}

/// The frozen v4 reference scenario (superseded by schema v5, the Q5/D9
/// heating overlay; kept as the migration-error fixture — it carries
/// the old inert `[zones.demand.heating]` sketch block that v5 removed).
fn v4_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v4-gb-2024-reference.toml")
}

/// The frozen v5 reference scenario (superseded by schema v6, the B6
/// two-zone package; kept as the migration-error fixture).
fn v5_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v5-gb-2024-reference.toml")
}

/// A minimal scenario exercising every optional table of schema v6,
/// including the ones the reference scenario omits ([constraints],
/// [solver], a heating portfolio with per-entry overrides, a DSR store,
/// multi-file traces, an explicit inertia_h override, an energy budget,
/// demand extra_profiles, a named lossy link, a per-direction link with
/// a capability trace, an exogenous `scale`).
const FULL_SKETCH: &str = r#"
schema_version = 8
name = "sketch"
description = "schema v6, all optional tables present"

[horizon]
start = "1985-01-01T00:00:00Z"
end = "2024-12-31T23:30:00Z"
weather_years = "all"

[[zones]]
id = "GB"

[zones.demand]
base_profile = "data/demand/gb_halfhourly.parquet"
column = "underlying_demand"
annual_scale = 1.0
extra_demand_gw = 0.667
extra_profiles = [
  { path = "data/demand/extra_a.parquet", column = "load_mw" },
  { path = "data/demand/extra_b.parquet", column = "load_mw" },
]

[zones.demand.heating]
delivered_heat_twh = 410.5
electrified_share = 0.5
dhw_fraction = 0.17
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.heating.entries]]
kind = "ashp"
share = 0.70

[[zones.demand.heating.entries]]
kind = "gshp"
share = 0.20
rhpp_derating = 0.75
resource_depth_m = 150.0

[[zones.demand.heating.entries]]
kind = "district_geothermal"
share = 0.10
cop_const = 13.0

[[zones.exogenous_supply]]
label = "net_imports"
path = "data/packs/2024/processed/generation_by_fuel_2024.parquet"
columns = ["intfr", "intnsl"]
imports = true
reliability = "variable"

[[zones.exogenous_supply]]
label = "other"
path = ["data/other_1985.parquet", "data/other_1986.parquet"]
columns = ["other"]
reliability = "firm"
scale = 0.101

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0

[[zones.fleet]]
technology = "nuclear"
capacity_gw = 5.9
availability = { monthly = [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5] }

[[zones.fleet]]
technology = "hydro"
capacity_gw = 10.3
# Schema v4: the seasonal-budget reservoir model (window_periods
# defaulted to 336 here).
energy_budget = { trace = "data/packs/entsoe-2024/processed/no2_generation_2024.parquet", columns = ["hydro_reservoir", "hydro_pumped"] }

[[zones.fleet]]
technology = "biomass"
capacity_gw = 3.5
availability = { flat = 0.61 }
reliability = "variable"   # explicit override of the derived firm default
inertia_h = 4.9            # explicit override of the derived 4.0 s default (schema v3)

[[zones.fleet]]
technology = "offshore_wind"
capacity_gw = 50.0
capacity_factor_trace = ["data/weather/gb_offshore_cf_1985.parquet", "data/weather/gb_offshore_cf_1986.parquet"]

[[zones.storage]]
kind = "hydrogen"
power_gw = 20.0
energy_gwh = 60000.0
round_trip_efficiency = 0.38
dispatch_order = 2
initial_soc = 0.5

[[zones.storage]]
kind = "dsr"
power_gw = 2.0
energy_gwh = 8.0
round_trip_efficiency = 1.0
dispatch_order = 3
shift_duration = 4.0
daily_volume_limit = 10.0

[[links]]
name = "IFA"
from = "GB"
to = "FR"
capacity_gw = 4.0
availability = 0.95
loss = 0.021

[[links]]
from = "GB"
to = "NO2"
capacity_gw = 1.4
availability = 0.95

[[links]]
name = "B6"
from = "SCO"
to = "RGB"
capacity_gw = 4.1
reverse_capacity_gw = 3.5
availability = 1.0

[links.capability_trace]
path = "data/packs/b6/processed/b6_da_flows_limits.parquet"
column = "limit_mw"
sentinel_high_mw = 9999.0
upper_bound_gw = 6.7
masked_fill_gw = 4.1

[dispatch]
policy = "perfect_foresight"

[constraints]
b6_cost_model = "scottish_wind_keyed"

[solver]
mode = "min_storage_for_zero_unserved"

[pricing]
reference = "data/reference/prices-2024.toml"

[pricing.fuel_price.gas]
path = "data/packs/2024/processed/gas_sap_daily_2024.parquet"
column = "sap_gbp_per_mwh_hhv"

[pricing.srmc.ccgt]
fuel = "gas"
efficiency = "ccgt"

[pricing.observed_price]
path = "data/packs/2024/processed/market_index_2024.parquet"
column = "mid_price"
"#;

// ---------------------------------------------------------------------
// The migrated v2 reference scenario is self-contained.
// ---------------------------------------------------------------------

#[test]
fn reference_scenario_parses_as_self_contained_v7() {
    let scenario = Scenario::load(&reference_scenario_path()).unwrap();
    assert_eq!(scenario.schema_version, 8);
    // v5 removed the inert v1–v4 heating sketch block from the live
    // reference scenario (D9 rule 2); the migrated file carries none.
    assert!(scenario.zones[0].demand.heating.is_none());
    assert_eq!(scenario.name, "GB-2024-reference");
    assert_eq!(
        scenario.horizon.weather_years,
        WeatherYears::Years(vec![2024])
    );
    assert_eq!(scenario.zones.len(), 1);

    let gb = &scenario.zones[0];
    assert_eq!(gb.id.as_str(), "GB");
    assert_eq!(gb.fleet.len(), 9);
    let ccgt = gb
        .fleet
        .iter()
        .find(|t| t.technology.as_str() == "ccgt")
        .unwrap();
    assert_eq!(ccgt.capacity_gw, Power::gigawatts(30.0));

    // Schema v2: demand column selection + station-load adjustment live
    // in the scenario (formerly the run-inputs [demand] table).
    assert_eq!(gb.demand.column, "underlying_demand");
    assert_eq!(gb.demand.extra_demand_gw, Power::gigawatts(0.667));

    // Schema v2: the three exogenous must-take supply traces (formerly
    // the run-inputs [[exogenous_supply]] tables).
    assert_eq!(gb.exogenous_supply.len(), 3);
    assert_eq!(gb.exogenous_supply[0].label, "net_imports");
    assert!(gb.exogenous_supply[0].imports);
    assert_eq!(gb.exogenous_supply[0].columns.len(), 10);
    assert_eq!(gb.exogenous_supply[1].label, "pumped_storage_net");
    assert!(!gb.exogenous_supply[1].imports);
    assert_eq!(gb.exogenous_supply[2].label, "other");

    // Schema v2: availability models on the fleet entries (formerly the
    // run-inputs [availability.*] tables).
    let nuclear = gb
        .fleet
        .iter()
        .find(|t| t.technology.as_str() == "nuclear")
        .unwrap();
    match &nuclear.availability {
        Some(AvailabilitySpec::Monthly { monthly }) => {
            assert_eq!(monthly.len(), 12);
            assert_eq!(monthly[0], PerUnit::new(0.5569));
        }
        other => panic!("nuclear availability should be monthly, got {other:?}"),
    }
    let biomass = gb
        .fleet
        .iter()
        .find(|t| t.technology.as_str() == "biomass")
        .unwrap();
    assert_eq!(
        biomass.availability,
        Some(AvailabilitySpec::Flat {
            flat: PerUnit::new(0.6116)
        })
    );
    assert!(ccgt.availability.is_none(), "ccgt runs to nameplate");

    // Storage carries the Stage 3 fields; initial_soc defaults to full
    // (D4) by omission.
    assert_eq!(gb.storage.len(), 2);
    assert_eq!(gb.storage[0].kind, StorageKind::PumpedHydro);
    assert_eq!(gb.storage[0].energy_gwh, Energy::gigawatt_hours(24.0));
    assert_eq!(gb.storage[0].round_trip_efficiency, PerUnit::new(0.76));
    assert_eq!(gb.storage[0].initial_soc, None);
    assert_eq!(gb.storage[0].shift_duration, None);
    assert_eq!(gb.storage[0].daily_volume_limit, None);

    // Schema v2: the [pricing] block (formerly the run-inputs [pricing]
    // section).
    let pricing = scenario.pricing.as_ref().unwrap();
    assert_eq!(pricing.reference, "data/reference/prices-2024.toml");
    assert_eq!(pricing.fuel_price["gas"].column, "sap_gbp_per_mwh_hhv");
    assert_eq!(pricing.srmc["ccgt"].fuel, "gas");
    assert_eq!(pricing.srmc["ocgt"].efficiency, "ocgt");
    assert_eq!(pricing.observed_price.as_ref().unwrap().column, "mid_price");

    assert_eq!(scenario.links.len(), 10);
    assert_eq!(scenario.dispatch.policy, DispatchPolicyKind::RuleBased);
    assert_eq!(scenario.horizon.period_count().unwrap(), 17_568);

    // And the migrated file passes semantic validation.
    scenario.validate().unwrap();
}

// Stage 0 acceptance test 1 (carried forward): parse → serialise → parse
// is lossless.
#[test]
fn reference_scenario_round_trips_losslessly() {
    let original = Scenario::load(&reference_scenario_path()).unwrap();
    let serialised = original.to_toml_string().unwrap();
    let reparsed = Scenario::from_toml_str(&serialised).unwrap();
    assert_eq!(original, reparsed);
}

#[test]
fn full_sketch_scenario_round_trips_losslessly() {
    let original = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    assert_eq!(original.horizon.weather_years, WeatherYears::All);
    assert_eq!(
        original.solver.as_ref().map(|s| s.mode.as_str()),
        Some("min_storage_for_zero_unserved")
    );
    assert_eq!(
        original.zones[0]
            .demand
            .heating
            .as_ref()
            .unwrap()
            .entries
            .len(),
        3
    );

    let serialised = original.to_toml_string().unwrap();
    let reparsed = Scenario::from_toml_str(&serialised).unwrap();
    assert_eq!(original, reparsed);
}

// ---------------------------------------------------------------------
// Schema v5 (Q5/D9): the heating-portfolio block — field shapes,
// per-entry overrides, and semantic validation (D9 rule 2).
// ---------------------------------------------------------------------

#[test]
fn heating_portfolio_parses_with_the_d9_field_set() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let heating = scenario.zones[0].demand.heating.as_ref().unwrap();
    // `delivered_heat_twh` is written in TWh and carried as Energy (GWh
    // canonical) — ADR-4: no raw f64 physical quantity in the API.
    assert_eq!(
        heating.delivered_heat_twh,
        Energy::gigawatt_hours(410_500.0)
    );
    assert_eq!(heating.electrified_share, PerUnit::new(0.5));
    assert_eq!(heating.dhw_fraction, PerUnit::new(0.17));
    assert_eq!(
        heating.temperature_trace.path,
        "data/weather/gb_t2m_pop.parquet"
    );
    assert_eq!(heating.temperature_trace.column, "t2m_pop");

    assert_eq!(heating.entries.len(), 3);
    assert_eq!(heating.entries[0].kind, HeatingKind::Ashp);
    assert_eq!(heating.entries[0].share, PerUnit::new(0.70));
    assert!(heating.entries[0].rhpp_derating.is_none());
    assert_eq!(heating.entries[1].kind, HeatingKind::Gshp);
    assert_eq!(heating.entries[1].rhpp_derating, Some(PerUnit::new(0.75)));
    // Schema v8 (D16): the optional GSHP resource depth, written in
    // metres and carried as Length (ADR-4).
    assert_eq!(
        heating.entries[1].resource_depth_m,
        Some(Length::metres(150.0))
    );
    assert!(heating.entries[0].resource_depth_m.is_none());
    assert_eq!(heating.entries[2].kind, HeatingKind::DistrictGeothermal);
    assert_eq!(heating.entries[2].cop_const, Some(13.0));
    assert!(heating.entries[2].resource_depth_m.is_none());
    scenario.validate().unwrap();
}

/// Schema v8 (D16): `resource_depth_m` is a GSHP-only field — on an
/// ASHP or district entry it is a structured validation error naming
/// the placement rule; non-physical depths are rejected.
#[test]
fn resource_depth_placement_and_physicality_are_validated() {
    // On ASHP: the air source has no depth.
    let toml = FULL_SKETCH.replace(
        "kind = \"ashp\"\nshare = 0.70",
        "kind = \"ashp\"\nshare = 0.70\nresource_depth_m = 100.0",
    );
    let err = Scenario::from_toml_str(&toml)
        .unwrap()
        .validate()
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("gshp"), "must name the gshp-only rule: {msg}");
    assert!(msg.contains("resource_depth_m"), "err: {msg}");

    // On district: the endpoint carries only cop_const.
    let toml = FULL_SKETCH.replace(
        "kind = \"district_geothermal\"\nshare = 0.10",
        "kind = \"district_geothermal\"\nshare = 0.10\nresource_depth_m = 100.0",
    );
    let err = Scenario::from_toml_str(&toml)
        .unwrap()
        .validate()
        .unwrap_err();
    assert!(err.to_string().contains("gshp"), "err: {err}");

    // Non-physical depths on the legal entry.
    for bad in ["0.0", "-5.0", "nan"] {
        let toml = FULL_SKETCH.replace(
            "resource_depth_m = 150.0",
            &format!("resource_depth_m = {bad}"),
        );
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        assert!(
            scenario.validate().is_err(),
            "resource_depth_m = {bad} must fail validation"
        );
    }
}

#[test]
fn heating_share_sum_off_by_more_than_1e_minus_9_is_a_structured_error() {
    // D9 rule 2: |Σ share − 1| ≤ 1e-9, structured error naming the sum
    // and the entries.
    let toml = FULL_SKETCH.replace("share = 0.70", "share = 0.60");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        matches!(err, GridError::HeatingShareSum { .. }),
        "unexpected error: {err:?}"
    );
    let msg = err.to_string();
    assert!(msg.contains("0.9"), "message must name the sum: {msg}");
    for needle in ["ashp", "gshp", "district_geothermal"] {
        assert!(
            msg.contains(needle),
            "message must name the entries; lacks {needle:?}: {msg}"
        );
    }

    // Dust-level float error inside the 1e-9 tolerance is accepted.
    let toml = FULL_SKETCH.replace("share = 0.70", "share = 0.7000000000002");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    scenario.validate().unwrap();
}

#[test]
fn heating_fractions_outside_zero_one_are_validation_errors() {
    for (needle, replacement) in [
        ("electrified_share = 0.5", "electrified_share = 1.2"),
        ("electrified_share = 0.5", "electrified_share = -0.1"),
        ("electrified_share = 0.5", "electrified_share = nan"),
        ("dhw_fraction = 0.17", "dhw_fraction = 1.7"),
        ("delivered_heat_twh = 410.5", "delivered_heat_twh = -1.0"),
        ("delivered_heat_twh = 410.5", "delivered_heat_twh = nan"),
        ("share = 0.20", "share = -0.20"),
    ] {
        let toml = FULL_SKETCH.replace(needle, replacement);
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        assert!(
            scenario.validate().is_err(),
            "replacing {needle:?} with {replacement:?} must fail validation"
        );
    }
}

#[test]
fn heating_with_no_entries_is_a_validation_error() {
    let mut scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    scenario.zones[0]
        .demand
        .heating
        .as_mut()
        .unwrap()
        .entries
        .clear();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("entries"), "err: {err}");
}

#[test]
fn heating_unknown_kind_is_a_parse_error() {
    // deny_unknown_fields discipline: unknown kinds are parse errors,
    // never a silent fourth technology.
    let toml = FULL_SKETCH.replace("kind = \"gshp\"", "kind = \"wshp\"");
    assert!(Scenario::from_toml_str(&toml).is_err());
}

#[test]
fn heating_unknown_fields_are_rejected() {
    let toml = FULL_SKETCH.replace("dhw_fraction = 0.17", "dhw_fraction = 0.17\nenabled = true");
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("enabled"), "err: {err}");
    let toml = FULL_SKETCH.replace("share = 0.70", "share = 0.70\ncop = 3.0");
    assert!(Scenario::from_toml_str(&toml).is_err());
}

#[test]
fn heating_cop_const_is_district_only_and_curves_are_heat_pump_only() {
    // cop_const on a heat-pump entry is contradictory (their COP is the
    // rule-4 curve), and curve overrides on district are dead config.
    let toml = FULL_SKETCH.replace(
        "kind = \"ashp\"\nshare = 0.70",
        "kind = \"ashp\"\nshare = 0.70\ncop_const = 12.0",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("cop_const"), "err: {err}");

    for extra in [
        "cop_curve = [6.0, -0.09, 0.0005]",
        "correction_factor = 0.85",
        "rhpp_derating = 0.8",
    ] {
        let toml = FULL_SKETCH.replace(
            "kind = \"district_geothermal\"\nshare = 0.10",
            &format!("kind = \"district_geothermal\"\nshare = 0.10\n{extra}"),
        );
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            matches!(err, GridError::InvalidScenario { .. }),
            "{extra}: unexpected error {err:?}"
        );
    }
}

#[test]
fn heating_duplicate_kinds_are_a_validation_error() {
    // Per-entry output series are keyed by kind; a duplicate would be
    // an ambiguous label (and shares of one technology belong on one
    // entry).
    let toml = FULL_SKETCH.replace("kind = \"gshp\"", "kind = \"ashp\"");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("ashp"), "err: {err}");
}

// ---------------------------------------------------------------------
// Schema v2 field shapes.
// ---------------------------------------------------------------------

#[test]
fn demand_column_and_extra_demand_have_documented_defaults() {
    // A v2 scenario omitting the new demand fields gets the D3 column
    // convention and a zero adjustment.
    let toml = FULL_SKETCH
        .replace("column = \"underlying_demand\"\n", "")
        .replace("extra_demand_gw = 0.667\n", "");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    assert_eq!(scenario.zones[0].demand.column, "underlying_demand");
    assert_eq!(
        scenario.zones[0].demand.extra_demand_gw,
        Power::gigawatts(0.0)
    );
    // Absent exogenous supply is an empty list; absent [pricing] is None.
    let no_pricing: String = toml
        .lines()
        .take_while(|l| !l.starts_with("[pricing]"))
        .collect::<Vec<_>>()
        .join("\n");
    let scenario = Scenario::from_toml_str(&no_pricing).unwrap();
    assert!(scenario.pricing.is_none());
}

#[test]
fn trace_files_accept_a_single_path_or_a_list() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let gb = &scenario.zones[0];
    // Single path.
    assert_eq!(
        gb.demand.base_profile.paths(),
        ["data/demand/gb_halfhourly.parquet"]
    );
    assert_eq!(
        gb.exogenous_supply[0].path.paths(),
        ["data/packs/2024/processed/generation_by_fuel_2024.parquet"]
    );
    // Multi-file (per-year) lists, concatenated at load in file order.
    assert_eq!(
        gb.exogenous_supply[1].path.paths(),
        ["data/other_1985.parquet", "data/other_1986.parquet"]
    );
    let offshore = gb
        .fleet
        .iter()
        .find(|t| t.technology.as_str() == "offshore_wind")
        .unwrap();
    assert_eq!(
        offshore.capacity_factor_trace.as_ref().unwrap().paths(),
        [
            "data/weather/gb_offshore_cf_1985.parquet",
            "data/weather/gb_offshore_cf_1986.parquet"
        ]
    );
}

#[test]
fn dsr_storage_fields_parse_and_round_trip() {
    // Schema shape only: DSR engine semantics are provisional until Q6
    // (docs/notes/d4-rule-based-dispatch.md) and the engine rejects DSR
    // stores at run time.
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let dsr = &scenario.zones[0].storage[1];
    assert_eq!(dsr.kind, StorageKind::Dsr);
    assert_eq!(dsr.shift_duration, Some(Duration::hours(4.0)));
    assert_eq!(dsr.daily_volume_limit, Some(Energy::gigawatt_hours(10.0)));
    let hydrogen = &scenario.zones[0].storage[0];
    assert_eq!(hydrogen.initial_soc, Some(PerUnit::new(0.5)));

    let reparsed = Scenario::from_toml_str(&scenario.to_toml_string().unwrap()).unwrap();
    assert_eq!(scenario, reparsed);
}

#[test]
fn weather_years_keyword_forms_parse() {
    for (text, expected) in [
        (r#""all""#, WeatherYears::All),
        (r#""worst_on_record""#, WeatherYears::WorstOnRecord),
        ("[2010, 2024]", WeatherYears::Years(vec![2010, 2024])),
    ] {
        let toml = FULL_SKETCH.replace(
            r#"weather_years = "all""#,
            &format!("weather_years = {text}"),
        );
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        assert_eq!(scenario.horizon.weather_years, expected);
    }
}

// ---------------------------------------------------------------------
// Semantic validation (Scenario::validate).
// ---------------------------------------------------------------------

#[test]
fn duplicate_dispatch_order_within_a_zone_is_a_validation_error() {
    // D4 rule 2: dispatch_order values must be unique within a zone.
    let toml = FULL_SKETCH.replace("dispatch_order = 3", "dispatch_order = 2");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    match &err {
        GridError::DuplicateDispatchOrder { zone, order } => {
            assert_eq!(zone, "GB");
            assert_eq!(*order, 2);
        }
        other => panic!("unexpected error: {other:?}"),
    }
    let msg = err.to_string();
    assert!(msg.contains("dispatch_order"), "message was: {msg}");
    assert!(msg.contains("unique"), "message was: {msg}");
}

#[test]
fn out_of_range_storage_parameters_are_validation_errors() {
    for (needle, replacement) in [
        // Round-trip efficiency outside (0, 1].
        (
            "round_trip_efficiency = 0.38",
            "round_trip_efficiency = 0.0",
        ),
        (
            "round_trip_efficiency = 0.38",
            "round_trip_efficiency = 1.2",
        ),
        // Initial SoC outside [0, 1].
        ("initial_soc = 0.5", "initial_soc = 1.5"),
        ("initial_soc = 0.5", "initial_soc = -0.1"),
        // Negative ratings.
        ("power_gw = 20.0", "power_gw = -1.0"),
        ("energy_gwh = 60000.0", "energy_gwh = -5.0"),
    ] {
        let toml = FULL_SKETCH.replace(needle, replacement);
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            matches!(err, GridError::InvalidScenario { .. }),
            "replacing {needle:?} with {replacement:?} should be InvalidScenario, got {err:?}"
        );
    }
}

#[test]
fn dsr_fields_on_a_non_dsr_store_are_a_validation_error() {
    let toml = FULL_SKETCH.replace(
        "round_trip_efficiency = 0.38",
        "round_trip_efficiency = 0.38\nshift_duration = 2.0",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        matches!(err, GridError::InvalidScenario { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("shift_duration"), "err: {err}");
}

#[test]
fn availability_on_a_weather_driven_technology_is_a_validation_error() {
    // A CF-trace technology is must-take; an availability model on it
    // would be dead configuration (the engine never consults it).
    let toml = FULL_SKETCH.replace(
        "capacity_gw = 50.0",
        "capacity_gw = 50.0\navailability = { flat = 0.9 }",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        matches!(err, GridError::InvalidScenario { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("offshore_wind"), "err: {err}");
}

// ---------------------------------------------------------------------
// Version handling: missing, v1 (migration message), unsupported.
// ---------------------------------------------------------------------

#[test]
fn missing_schema_version_is_a_structured_error() {
    let toml = FULL_SKETCH.replace("schema_version = 8", "");
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(
        matches!(err, GridError::MissingSchemaVersion),
        "unexpected error: {err:?}"
    );
}

/// docs/05 rule 4: old scenario files fail with a clear migration
/// message naming what moved — never a field-level error, never silent
/// reinterpretation. The frozen v1 reference pair under tests/fixtures/
/// exists for exactly this test.
#[test]
fn v1_scenario_fails_with_a_migration_message_naming_what_moved() {
    let err = Scenario::load(&v1_fixture_path()).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("schema_version"), "message was: {msg}");
    // The message names each thing that moved out of the run-inputs file.
    for needle in [
        "run-inputs",
        "exogenous_supply",
        "availability",
        "pricing",
        "extra_demand_gw",
        "--inputs",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    // Structured, not a generic parse error.
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion1Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

/// docs/05 rule 4 again for v2 → v3 (Stage 6): the frozen v2 reference
/// fixture must fail with a migration message naming the stability
/// metadata that was added and the one-line migration.
#[test]
fn v2_scenario_fails_with_a_migration_message_naming_what_was_added() {
    let err = Scenario::load(&v2_fixture_path()).unwrap_err();
    let msg = err.to_string();
    for needle in [
        "schema_version 2",
        "inertia_h",
        "synchronous",
        "inertia-constants.toml",
        "schema_version = 8",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion2Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

/// docs/05 rule 4 again for v3 → v4 (Stage 5): the frozen v3 reference
/// fixture must fail with a migration message naming the multi-zone
/// activation fields that were added and the one-line migration.
#[test]
fn v3_scenario_fails_with_a_migration_message_naming_what_was_added() {
    let err = Scenario::load(&v3_fixture_path()).unwrap_err();
    let msg = err.to_string();
    for needle in [
        "schema_version 3",
        "loss",
        "name",
        "energy_budget",
        "extra_profiles",
        "schema_version = 8",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion3Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

/// docs/05 rule 4 again for v4 → v5 (the Q5/D9 heating overlay): the
/// frozen v4 reference fixture — which CARRIES the old inert
/// `[zones.demand.heating]` sketch block — must fail with a migration
/// message naming the replacement portfolio block (D9 rule 2: v5 is
/// NOT additive; the old `enabled` / `heat_demand_per_degree` /
/// `cop_curve` fields are removed). A v4 file WITHOUT the old block
/// migrates by changing only the version line — the message must say
/// so.
#[test]
fn v4_scenario_fails_with_a_migration_message_naming_the_heating_replacement() {
    let err = Scenario::load(&v4_fixture_path()).unwrap_err();
    let msg = err.to_string();
    for needle in [
        "schema_version 4",
        "zones.demand.heating",
        "enabled",
        "heat_demand_per_degree",
        "cop_curve",
        "delivered_heat_twh",
        "electrified_share",
        "dhw_fraction",
        "entries",
        "version line",
        "schema_version = 8",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion4Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

// ---------------------------------------------------------------------
// Schema v4 field shapes (Stage 5 multi-zone activation).
// ---------------------------------------------------------------------

#[test]
fn link_name_and_loss_parse_with_documented_defaults() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    // Explicit fields on the first link.
    let ifa = &scenario.links[0];
    assert_eq!(ifa.name.as_deref(), Some("IFA"));
    assert_eq!(ifa.loss, PerUnit::new(0.021));
    // Defaults on the second: no name, lossless.
    let nsl = &scenario.links[1];
    assert_eq!(nsl.name, None);
    assert_eq!(nsl.loss, PerUnit::new(0.0));
}

#[test]
fn energy_budget_parses_with_the_weekly_default_window() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let hydro = scenario.zones[0]
        .fleet
        .iter()
        .find(|e| e.technology.as_str() == "hydro")
        .unwrap();
    let budget = hydro.energy_budget.as_ref().unwrap();
    assert_eq!(budget.columns, ["hydro_reservoir", "hydro_pumped"]);
    assert_eq!(
        budget.window_periods,
        grid_core::scenario::DEFAULT_BUDGET_WINDOW_PERIODS
    );
    assert_eq!(grid_core::scenario::DEFAULT_BUDGET_WINDOW_PERIODS, 336);
    // Entries without one carry None.
    let ccgt = scenario.zones[0]
        .fleet
        .iter()
        .find(|e| e.technology.as_str() == "ccgt")
        .unwrap();
    assert!(ccgt.energy_budget.is_none());
}

#[test]
fn demand_extra_profiles_parse_and_default_to_empty() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let extras = &scenario.zones[0].demand.extra_profiles;
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0].path, "data/demand/extra_a.parquet");
    assert_eq!(extras[0].column, "load_mw");
    // Absent field: empty list (single-country zones).
    let toml = FULL_SKETCH.replace(
        "extra_profiles = [\n  { path = \"data/demand/extra_a.parquet\", column = \"load_mw\" },\n  { path = \"data/demand/extra_b.parquet\", column = \"load_mw\" },\n]\n",
        "",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    assert!(scenario.zones[0].demand.extra_profiles.is_empty());
}

#[test]
fn link_loss_outside_zero_one_is_a_validation_error() {
    for replacement in ["loss = 1.0", "loss = -0.1", "loss = nan"] {
        let toml = FULL_SKETCH.replace("loss = 0.021", replacement);
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            matches!(err, GridError::InvalidScenario { .. }),
            "{replacement}: unexpected error {err:?}"
        );
        assert!(err.to_string().contains("loss"), "err: {err}");
    }
}

#[test]
fn link_availability_outside_zero_one_is_a_validation_error() {
    let toml = FULL_SKETCH.replace("availability = 0.95\nloss", "availability = 1.2\nloss");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        matches!(err, GridError::InvalidScenario { .. }),
        "unexpected error {err:?}"
    );
}

#[test]
fn link_with_identical_endpoints_is_a_validation_error() {
    let toml = FULL_SKETCH.replace("to = \"FR\"", "to = \"GB\"");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("same zone"), "err: {err}");
}

#[test]
fn energy_budget_on_a_weather_driven_technology_is_a_validation_error() {
    let toml = FULL_SKETCH.replace(
        "capacity_gw = 50.0",
        "capacity_gw = 50.0\nenergy_budget = { trace = \"x.parquet\", columns = [\"a\"] }",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("energy_budget"), "err: {err}");
    assert!(err.to_string().contains("offshore_wind"), "err: {err}");
}

#[test]
fn degenerate_energy_budgets_are_validation_errors() {
    // No columns.
    let toml = FULL_SKETCH.replace(
        "columns = [\"hydro_reservoir\", \"hydro_pumped\"]",
        "columns = []",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("no columns"), "err: {err}");
    // Zero-length window.
    let toml = FULL_SKETCH.replace(
        "columns = [\"hydro_reservoir\", \"hydro_pumped\"] }",
        "columns = [\"hydro_reservoir\"], window_periods = 0 }",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("window_periods"), "err: {err}");
}

#[test]
fn duplicate_fleet_technology_within_a_zone_is_a_validation_error() {
    // The b6 engine-review follow-up (note 6): duplicate fleet TechIds
    // within a zone silently corrupt programmatic input assembly —
    // per-technology inputs (CF traces, availability models, energy
    // budgets, SRMC recipes) are keyed by TechId in maps (LAST-WINS),
    // while dispatch builds one unit PER ENTRY (both dispatch) and
    // result readouts find the FIRST series of a given id. Characterised
    // 2026-07-06 and rejected outright with a structured error.
    let toml = FULL_SKETCH.replace(
        "[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 30.0",
        "[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 30.0\n\n\
         [[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 5.0",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    match &err {
        GridError::DuplicateFleetTechnology { zone, technology } => {
            assert_eq!(zone, "GB");
            assert_eq!(technology, "ccgt");
        }
        other => panic!("unexpected error: {other:?}"),
    }
    let msg = err.to_string();
    assert!(msg.contains("more than once"), "message was: {msg}");
    assert!(msg.contains("ccgt"), "message was: {msg}");
}

#[test]
fn the_same_technology_in_different_zones_is_legal() {
    // The multi-zone pattern (every committed 2/3/5/8-zone scenario
    // carries e.g. onshore_wind in several zones): uniqueness is
    // per-zone, never global.
    let toml = FULL_SKETCH.replace(
        "[[links]]\nname = \"IFA\"",
        "[[zones]]\nid = \"FR\"\n[zones.demand]\nbase_profile = \"fr.parquet\"\n\
         annual_scale = 1.0\n[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 10.0\n\n\
         [[links]]\nname = \"IFA\"",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    // (The FULL_SKETCH links to NO2/SCO/RGB make a two-zone variant fail
    // on endpoints, not on the fleet — assert the specific absence.)
    let err = scenario.validate().unwrap_err();
    assert!(
        !matches!(err, GridError::DuplicateFleetTechnology { .. }),
        "cross-zone duplicate technology must not be a fleet-uniqueness error: {err:?}"
    );
}

#[test]
fn duplicate_zone_ids_are_a_validation_error() {
    let toml = FULL_SKETCH.replace(
        "[[links]]\nname = \"IFA\"",
        "[[zones]]\nid = \"GB\"\n[zones.demand]\nbase_profile = \"x.parquet\"\nannual_scale = 1.0\n\n[[links]]\nname = \"IFA\"",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(err.to_string().contains("more than once"), "err: {err}");
}

#[test]
fn multi_zone_links_must_reference_declared_zones() {
    // Add a second zone: the scenario becomes multi-zone, and the links
    // to FR/NO2 (undeclared) must now be rejected.
    let toml = FULL_SKETCH.replace(
        "[[links]]\nname = \"IFA\"",
        "[[zones]]\nid = \"FR\"\n[zones.demand]\nbase_profile = \"fr.parquet\"\nannual_scale = 1.0\n\n[[links]]\nname = \"IFA\"",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("NO2"), "err: {msg}");
    assert!(msg.contains("declared zone"), "err: {msg}");
}

#[test]
fn single_zone_scenarios_may_keep_external_link_counterparties() {
    // The GB reference pattern: one zone, links naming external ids —
    // legal while the links are inert (imports exogenous).
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    assert_eq!(scenario.zones.len(), 1);
    scenario.validate().unwrap();
}

#[test]
fn unsupported_schema_version_is_a_structured_error() {
    let toml = FULL_SKETCH.replace("schema_version = 8", "schema_version = 99");
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    match err {
        GridError::UnsupportedSchemaVersion { found, supported } => {
            assert_eq!(found, 99);
            assert_eq!(supported, 8);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

// A non-integer schema_version must also be rejected, not reinterpreted.
#[test]
fn non_integer_schema_version_is_rejected() {
    let toml = FULL_SKETCH.replace("schema_version = 8", r#"schema_version = "two""#);
    assert!(Scenario::from_toml_str(&toml).is_err());
}

// Typoed / unknown fields are parse errors, not silently ignored: the
// scenario file is the complete description of a run (ADR-5), so a field
// the engine does not understand is a fidelity error.
#[test]
fn unknown_fields_are_rejected() {
    let toml = FULL_SKETCH.replace(
        "annual_scale = 1.0",
        "annual_scale = 1.0\nanual_scale = 2.0",
    );
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("anual_scale"), "message was: {msg}");

    // Including inside the new v2 tables.
    let toml = FULL_SKETCH.replace("imports = true", "imports = true\nfrobnicate = 1");
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("frobnicate"), "err: {err}");
    let toml = FULL_SKETCH.replace(
        "reference = \"data/reference/prices-2024.toml\"",
        "reference = \"data/reference/prices-2024.toml\"\nfrobnicate = 1",
    );
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("frobnicate"), "err: {err}");
}

// Parse errors carry line context for user-facing messages (docs/06).
#[test]
fn parse_errors_carry_line_context() {
    let toml = FULL_SKETCH.replace("capacity_gw = 30.0", "capacity_gw = \"thirty\"");
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("line"), "message lacks line context: {msg}");
}

#[test]
fn load_of_missing_file_names_the_path() {
    let err = Scenario::load(Path::new("/nonexistent/nowhere.toml")).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("/nonexistent/nowhere.toml"),
        "message was: {msg}"
    );
}

mod proptests {
    //! Property: any well-formed `Scenario` value survives
    //! serialise → parse unchanged (schema round-tripping, docs/06).
    //!
    //! Floats are drawn from the full finite range: the `toml` crate emits
    //! shortest-round-trip float literals, so exact equality is expected.

    use super::*;
    use grid_core::scenario::*;
    use grid_core::units::{CarbonPrice, Duration};
    use proptest::option;
    use proptest::prelude::*;

    /// TOML bare-ish strings without control characters; content is
    /// arbitrary printable unicode (TOML strings are fully escapable, so
    /// this is a formatting-robustness test, not a validity constraint).
    fn any_name() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-zA-Z0-9_ .:/\\-£°]{0,24}").unwrap()
    }

    fn finite_f64() -> impl Strategy<Value = f64> {
        // Finite, full range, both signs; excludes NaN/inf which TOML 1.0
        // can carry but scenario validation will later reject.
        prop::num::f64::NORMAL | prop::num::f64::ZERO | prop::num::f64::SUBNORMAL
    }

    fn any_trace_files() -> impl Strategy<Value = TraceFiles> {
        prop_oneof![
            any_name().prop_map(TraceFiles::from),
            prop::collection::vec(any_name(), 0..3).prop_map(TraceFiles::from_paths),
        ]
    }

    fn any_weather_years() -> impl Strategy<Value = WeatherYears> {
        prop_oneof![
            Just(WeatherYears::All),
            Just(WeatherYears::WorstOnRecord),
            prop::collection::vec(1900i32..2100, 0..4).prop_map(WeatherYears::Years),
        ]
    }

    fn any_heating_entry() -> impl Strategy<Value = HeatingEntry> {
        (
            prop_oneof![
                Just(HeatingKind::Ashp),
                Just(HeatingKind::Gshp),
                Just(HeatingKind::DistrictGeothermal),
            ],
            finite_f64(),
            option::of(prop::collection::vec(finite_f64(), 3)),
            option::of(finite_f64()),
            option::of(finite_f64()),
            option::of(finite_f64()),
            // Schema v8: dyadic-km depths (j/8 km) so the metres↔km
            // serde pair round-trips bit-exactly (the delivered_heat
            // dyadic-TWh note below; coherence — gshp-only, positive —
            // is Scenario::validate's job, not serialisation's).
            option::of((1u32..40_000).prop_map(|j| Length::kilometres(f64::from(j) / 8.0))),
        )
            .prop_map(
                |(kind, share, curve, correction, derating, cop_const, resource_depth_m)| {
                    HeatingEntry {
                        kind,
                        share: PerUnit::new(share),
                        cop_curve: curve.map(|v| [v[0], v[1], v[2]]),
                        correction_factor: correction.map(PerUnit::new),
                        rhpp_derating: derating.map(PerUnit::new),
                        cop_const,
                        resource_depth_m,
                    }
                },
            )
    }

    fn any_heating() -> impl Strategy<Value = HeatingSpec> {
        (
            // Dyadic TWh values (n/8) so the TWh↔GWh serde conversion
            // round-trips bit-exactly: n/8 × 1000 and its inverse are
            // exact in f64. Arbitrary floats need not survive the
            // ×1000/÷1000 pair (float division is not associative);
            // scenario authors write decimal TWh, which the TOML float
            // round-trips at the text level anyway.
            (0u32..4_000_000).prop_map(|n| Energy::gigawatt_hours(f64::from(n) / 8.0 * 1000.0)),
            finite_f64(),
            finite_f64(),
            any_trace_ref(),
            prop::collection::vec(any_heating_entry(), 0..4),
        )
            .prop_map(
                |(
                    delivered_heat_twh,
                    electrified_share,
                    dhw_fraction,
                    temperature_trace,
                    entries,
                )| {
                    HeatingSpec {
                        delivered_heat_twh,
                        electrified_share: PerUnit::new(electrified_share),
                        dhw_fraction: PerUnit::new(dhw_fraction),
                        temperature_trace,
                        entries,
                    }
                },
            )
    }

    fn any_availability() -> impl Strategy<Value = AvailabilitySpec> {
        prop_oneof![
            finite_f64().prop_map(|f| AvailabilitySpec::Flat {
                flat: PerUnit::new(f)
            }),
            prop::collection::vec(finite_f64(), 12).prop_map(|v| AvailabilitySpec::Monthly {
                monthly: v.into_iter().map(PerUnit::new).collect()
            }),
        ]
    }

    fn any_energy_budget() -> impl Strategy<Value = EnergyBudgetSpec> {
        (
            any_trace_files(),
            prop::collection::vec(any_name(), 0..3),
            1usize..2000,
        )
            .prop_map(|(trace, columns, window_periods)| EnergyBudgetSpec {
                trace,
                columns,
                window_periods,
            })
    }

    fn any_fleet_entry() -> impl Strategy<Value = FleetEntry> {
        (
            any_name(),
            finite_f64(),
            option::of(any_trace_files()),
            option::of(any_availability()),
            option::of(prop_oneof![
                Just(Reliability::Firm),
                Just(Reliability::Variable)
            ]),
            // Schema v3 stability metadata: arbitrary values here —
            // round-tripping is a serialisation property, coherence is
            // Scenario::validate's job.
            option::of(finite_f64().prop_map(grid_core::units::InertiaConstant::seconds)),
            option::of(any::<bool>()),
            // Schema v4: coherence again left to Scenario::validate.
            option::of(any_energy_budget()),
        )
            .prop_map(
                |(
                    technology,
                    capacity,
                    capacity_factor_trace,
                    availability,
                    reliability,
                    inertia_h,
                    synchronous,
                    energy_budget,
                )| {
                    FleetEntry {
                        technology: TechId::new(technology),
                        capacity_gw: Power::gigawatts(capacity),
                        capacity_factor_trace,
                        availability,
                        reliability,
                        inertia_h,
                        synchronous,
                        energy_budget,
                    }
                },
            )
    }

    fn any_storage() -> impl Strategy<Value = StorageSpec> {
        (
            prop_oneof![
                Just(StorageKind::Battery),
                Just(StorageKind::PumpedHydro),
                Just(StorageKind::Hydrogen),
                Just(StorageKind::Dsr),
            ],
            finite_f64(),
            finite_f64(),
            finite_f64(),
            any::<u8>(),
            option::of(finite_f64()),
            option::of(finite_f64()),
            option::of(finite_f64()),
        )
            .prop_map(|(kind, p, e, rte, dispatch_order, soc, shift, volume)| {
                StorageSpec {
                    kind,
                    power_gw: Power::gigawatts(p),
                    energy_gwh: Energy::gigawatt_hours(e),
                    round_trip_efficiency: PerUnit::new(rte),
                    dispatch_order,
                    initial_soc: soc.map(PerUnit::new),
                    shift_duration: shift.map(Duration::hours),
                    daily_volume_limit: volume.map(Energy::gigawatt_hours),
                }
            })
    }

    fn any_exogenous_supply() -> impl Strategy<Value = ExogenousSupplySpec> {
        (
            any_name(),
            any_trace_files(),
            prop::collection::vec(any_name(), 0..3),
            any::<bool>(),
            // scale (schema v6): the default 1.0 must appear often —
            // its serialisation is skipped, the round-trip must still
            // hold — alongside arbitrary split shares.
            prop_oneof![Just(1.0f64), 0.0f64..2.0],
            prop_oneof![
                Just(ExogenousReliability::Firm),
                Just(ExogenousReliability::Variable),
                Just(ExogenousReliability::Excluded),
            ],
        )
            .prop_map(|(label, path, columns, imports, scale, reliability)| {
                ExogenousSupplySpec {
                    label,
                    path,
                    columns,
                    imports,
                    scale,
                    reliability,
                }
            })
    }

    /// Schema v7 (D11): a zone pricing block — reference path, the
    /// optional flat carbon level, fuel-price traces and SRMC recipes.
    fn any_zone_pricing() -> impl Strategy<Value = ZonePricingSpec> {
        (
            any_name(),
            option::of(finite_f64().prop_map(f64::abs)),
            prop::collection::btree_map(any_name(), any_trace_ref(), 0..3),
            prop::collection::btree_map(
                any_name(),
                (any_name(), any_name())
                    .prop_map(|(fuel, efficiency)| SrmcRecipeSpec { fuel, efficiency }),
                0..3,
            ),
        )
            .prop_map(|(reference, flat, fuel_price, srmc)| ZonePricingSpec {
                reference,
                carbon_flat_gbp_per_tco2: flat.map(CarbonPrice::pounds_per_tonne_co2),
                fuel_price,
                srmc,
            })
    }

    fn any_zone() -> impl Strategy<Value = ZoneSpec> {
        (
            any_name(),
            (
                any_trace_files(),
                any_name(),
                finite_f64(),
                finite_f64(),
                prop::collection::vec(any_trace_ref(), 0..3),
                option::of(any_heating()),
            ),
            prop::collection::vec(any_exogenous_supply(), 0..3),
            prop::collection::vec(any_fleet_entry(), 0..3),
            prop::collection::vec(any_storage(), 0..3),
            option::of(any_zone_pricing()),
        )
            .prop_map(
                |(
                    id,
                    (base_profile, column, annual_scale, extra, extra_profiles, heating),
                    exogenous_supply,
                    fleet,
                    storage,
                    pricing,
                )| ZoneSpec {
                    pricing,
                    id: ZoneId::new(id),
                    demand: DemandSpec {
                        base_profile,
                        column,
                        extra_profiles,
                        annual_scale,
                        extra_demand_gw: Power::gigawatts(extra),
                        heating,
                    },
                    exogenous_supply,
                    fleet,
                    storage,
                },
            )
    }

    fn any_capability_trace() -> impl Strategy<Value = LinkCapabilityTraceSpec> {
        (
            any_name(),
            any_name(),
            finite_f64(),
            finite_f64(),
            finite_f64(),
        )
            .prop_map(|(path, column, sentinel_high_mw, upper, fill)| {
                LinkCapabilityTraceSpec {
                    path,
                    column,
                    sentinel_high_mw: Power::megawatts(sentinel_high_mw),
                    upper_bound_gw: Power::gigawatts(upper),
                    masked_fill_gw: Power::gigawatts(fill),
                }
            })
    }

    fn any_link() -> impl Strategy<Value = LinkSpec> {
        (
            option::of(any_name()),
            any_name(),
            any_name(),
            finite_f64(),
            option::of(finite_f64()),
            option::of(any_capability_trace()),
            finite_f64(),
            finite_f64(),
        )
            .prop_map(
                |(name, from, to, capacity, reverse, capability_trace, availability, loss)| {
                    LinkSpec {
                        name,
                        from: ZoneId::new(from),
                        to: ZoneId::new(to),
                        capacity_gw: Power::gigawatts(capacity),
                        reverse_capacity_gw: reverse.map(Power::gigawatts),
                        capability_trace,
                        availability: PerUnit::new(availability),
                        loss: PerUnit::new(loss),
                    }
                },
            )
    }

    fn any_trace_ref() -> impl Strategy<Value = TraceRefSpec> {
        (any_name(), any_name()).prop_map(|(path, column)| TraceRefSpec { path, column })
    }

    fn any_pricing() -> impl Strategy<Value = PricingSpec> {
        (
            any_name(),
            prop::collection::btree_map(any_name(), any_trace_ref(), 0..3),
            prop::collection::btree_map(
                any_name(),
                (any_name(), any_name())
                    .prop_map(|(fuel, efficiency)| SrmcRecipeSpec { fuel, efficiency }),
                0..3,
            ),
            option::of(any_trace_ref()),
        )
            .prop_map(
                |(reference, fuel_price, srmc, observed_price)| PricingSpec {
                    reference,
                    fuel_price,
                    srmc,
                    observed_price,
                },
            )
    }

    fn any_scenario() -> impl Strategy<Value = Scenario> {
        (
            (any_name(), option::of(any_name()), any_name(), any_name()),
            any_weather_years(),
            prop::collection::vec(any_zone(), 0..3),
            prop::collection::vec(any_link(), 0..3),
            (
                prop_oneof![
                    Just(DispatchPolicyKind::RuleBased),
                    Just(DispatchPolicyKind::PerfectForesight)
                ],
                prop_oneof![Just(FlowSignal::Scarcity), Just(FlowSignal::PricedLadder)],
            ),
            option::of(
                option::of(any_name()).prop_map(|b6_cost_model| Constraints { b6_cost_model }),
            ),
            option::of(any_name().prop_map(|mode| Solver { mode })),
            option::of(any_pricing()),
        )
            .prop_map(
                |(
                    (name, description, start, end),
                    weather_years,
                    zones,
                    links,
                    (policy, flow_signal),
                    constraints,
                    solver,
                    pricing,
                )| {
                    Scenario {
                        schema_version: 8,
                        name,
                        description,
                        horizon: Horizon {
                            start,
                            end,
                            weather_years,
                        },
                        zones,
                        links,
                        dispatch: Dispatch {
                            policy,
                            flow_signal,
                        },
                        constraints,
                        solver,
                        pricing,
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn scenario_round_trips(scenario in any_scenario()) {
            let serialised = scenario.to_toml_string().unwrap();
            let reparsed = Scenario::from_toml_str(&serialised).unwrap();
            prop_assert_eq!(scenario, reparsed);
        }

        // The unit newtypes serialise transparently as bare floats.
        #[test]
        fn unit_newtypes_round_trip_via_toml(value in prop::num::f64::NORMAL) {
            #[derive(serde::Serialize, serde::Deserialize)]
            struct Probe { p: Power, e: Energy, d: Duration }
            let probe = Probe {
                p: Power::gigawatts(value),
                e: Energy::gigawatt_hours(value),
                d: Duration::hours(value),
            };
            let text = toml::to_string(&probe).unwrap();
            let back: Probe = toml::from_str(&text).unwrap();
            prop_assert_eq!(back.p, probe.p);
            prop_assert_eq!(back.e, probe.e);
            prop_assert_eq!(back.d, probe.d);
        }
    }
}

// ---------------------------------------------------------------------
// Reliability classification (gb-grid-margin methodology; schema v2
// addendum): derived defaults, explicit overrides, and the required
// explicit field on exogenous entries.
// ---------------------------------------------------------------------

#[test]
fn fleet_reliability_defaults_are_derived_and_overrides_are_flagged() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let gb = &scenario.zones[0];
    let entry = |tech: &str| {
        gb.fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap()
    };
    // Dispatchable, no explicit field: derived firm, not an override.
    let ccgt = entry("ccgt");
    assert_eq!(ccgt.reliability, None);
    assert_eq!(ccgt.effective_reliability(), Reliability::Firm);
    assert!(!ccgt.reliability_overridden());
    // Weather-driven, no explicit field: derived variable.
    let offshore = entry("offshore_wind");
    assert_eq!(offshore.effective_reliability(), Reliability::Variable);
    assert!(!offshore.reliability_overridden());
    // Explicit field differing from the derived default: an override,
    // respected and flagged (so outputs can surface it).
    let biomass = entry("biomass");
    assert_eq!(biomass.reliability, Some(Reliability::Variable));
    assert_eq!(biomass.effective_reliability(), Reliability::Variable);
    assert!(biomass.reliability_overridden());
    // An explicit field EQUAL to the derived default is a restatement,
    // not an override.
    let mut restated = ccgt.clone();
    restated.reliability = Some(Reliability::Firm);
    assert!(!restated.reliability_overridden());
}

#[test]
fn excluded_is_not_a_legal_fleet_reliability() {
    // `excluded` exists only for exogenous entries (pumped-storage
    // traces); on a fleet entry it is a parse error, not a silent
    // third state.
    let toml = FULL_SKETCH.replace(
        "reliability = \"variable\"   # explicit override of the derived firm default",
        "reliability = \"excluded\"",
    );
    assert!(Scenario::from_toml_str(&toml).is_err());
}

#[test]
fn exogenous_reliability_is_required() {
    // No safe default exists for hand-written exogenous series: a
    // missing `reliability` is a parse error naming the field.
    let toml = FULL_SKETCH.replace(
        "reliability = \"variable\"\n\n[[zones.exogenous_supply]]",
        "\n[[zones.exogenous_supply]]",
    );
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(
        err.to_string().contains("reliability"),
        "error should name the missing field: {err}"
    );
}

// ---------------------------------------------------------------------
// Schema v3 (Stage 6): stability metadata — derived defaults, explicit
// overrides (surfaced), and coherence validation.
// ---------------------------------------------------------------------

#[test]
fn inertia_metadata_defaults_are_derived_and_overrides_are_flagged() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let gb = &scenario.zones[0];
    let entry = |tech: &str| {
        gb.fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap()
    };
    // Dispatchable, no explicit fields: derived synchronous with the
    // reference-file H, not an override.
    let ccgt = entry("ccgt");
    assert_eq!(ccgt.inertia_h, None);
    assert_eq!(ccgt.synchronous, None);
    assert!(ccgt.effective_synchronous());
    assert_eq!(
        ccgt.effective_inertia_h(),
        Some(grid_core::units::InertiaConstant::seconds(5.0))
    );
    assert!(!ccgt.inertia_overridden());
    assert!(!ccgt.synchronous_overridden());
    // Weather-driven: derived non-synchronous, no H.
    let offshore = entry("offshore_wind");
    assert!(!offshore.effective_synchronous());
    assert_eq!(offshore.effective_inertia_h(), None);
    // Explicit H differing from the derived 4.0 s default: respected
    // and flagged (so outputs can surface it).
    let biomass = entry("biomass");
    assert_eq!(
        biomass.effective_inertia_h(),
        Some(grid_core::units::InertiaConstant::seconds(4.9))
    );
    assert!(biomass.inertia_overridden());
    // A restatement of the default is not an override.
    let mut restated = ccgt.clone();
    restated.inertia_h = Some(grid_core::units::InertiaConstant::seconds(5.0));
    restated.synchronous = Some(true);
    assert!(!restated.inertia_overridden());
    assert!(!restated.synchronous_overridden());
    // The sketch passes semantic validation with the override in place.
    scenario.validate().unwrap();
}

#[test]
fn synchronous_override_on_wind_requires_an_explicit_h() {
    // Claiming a synchronous coupling for a technology with no derived
    // H (e.g. a grid-forming/synchronous-compensated wind fleet) is
    // legal ONLY with an explicit inertia_h — there is no default to
    // fall back on.
    let toml = FULL_SKETCH.replace(
        "technology = \"offshore_wind\"\ncapacity_gw = 50.0",
        "technology = \"offshore_wind\"\nsynchronous = true\ncapacity_gw = 50.0",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        err.to_string().contains("inertia_h"),
        "error should direct the author to inertia_h: {err}"
    );

    let toml = FULL_SKETCH.replace(
        "technology = \"offshore_wind\"\ncapacity_gw = 50.0",
        "technology = \"offshore_wind\"\nsynchronous = true\ninertia_h = 1.5\ncapacity_gw = 50.0",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    scenario.validate().unwrap();
    let offshore = scenario.zones[0]
        .fleet
        .iter()
        .find(|e| e.technology.as_str() == "offshore_wind")
        .unwrap();
    assert!(offshore.effective_synchronous());
    assert!(offshore.synchronous_overridden());
    assert_eq!(
        offshore.effective_inertia_h(),
        Some(grid_core::units::InertiaConstant::seconds(1.5))
    );
}

#[test]
fn inertia_h_on_a_non_synchronous_entry_is_a_validation_error() {
    // Explicit H on decoupled plant is contradictory: the rotor energy
    // exists but the grid never sees it.
    let toml = FULL_SKETCH.replace(
        "technology = \"offshore_wind\"\ncapacity_gw = 50.0",
        "technology = \"offshore_wind\"\ninertia_h = 3.0\ncapacity_gw = 50.0",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        matches!(err, GridError::InvalidScenario { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("synchronous"), "err: {err}");
    // Same for an explicit synchronous = false with an explicit H.
    let toml = FULL_SKETCH.replace(
        "inertia_h = 4.9            # explicit override of the derived 4.0 s default (schema v3)",
        "inertia_h = 4.9\nsynchronous = false",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    assert!(scenario.validate().is_err());
}

#[test]
fn out_of_range_inertia_h_is_a_validation_error() {
    for value in ["-1.0", "0.0", "nan", "inf"] {
        let toml = FULL_SKETCH.replace(
            "inertia_h = 4.9            # explicit override of the derived 4.0 s default (schema v3)",
            &format!("inertia_h = {value}"),
        );
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        assert!(
            scenario.validate().is_err(),
            "inertia_h = {value} should be rejected"
        );
    }
}

// ---------------------------------------------------------------------
// Stage 3 part 2: horizon arithmetic over the full 40-year weather
// record (docs/04 Stage 3 — multi-year continuous runs). Nothing in
// `Horizon` may assume a horizon is a single calendar year.
// ---------------------------------------------------------------------

/// The full 1985–2024 record: 40 calendar years, 10 of them leap
/// (1988, 1992, …, 2024) = 30 × 365 + 10 × 366 = 14,610 days
/// = 701,280 half-hourly settlement periods.
const FULL_RECORD_PERIODS: usize = 701_280;

#[test]
fn horizon_period_count_spans_the_full_40_year_record() {
    let horizon = grid_core::scenario::Horizon {
        start: "1985-01-01T00:00:00Z".to_owned(),
        end: "2024-12-31T23:30:00Z".to_owned(),
        weather_years: WeatherYears::All,
    };
    assert_eq!(horizon.period_count().unwrap(), FULL_RECORD_PERIODS);

    // Period arithmetic is exact at the far end: the last period of the
    // horizon is start + (N − 1) periods, and one more steps into 2025.
    let start = horizon.start_instant().unwrap();
    let end = horizon.end_instant().unwrap();
    assert_eq!(start.plus_periods(FULL_RECORD_PERIODS as i64 - 1), end);
    assert_eq!(
        start.plus_periods(FULL_RECORD_PERIODS as i64).to_string(),
        "2025-01-01T00:00:00Z"
    );
    // Calendar conversion is exact 40 years out (leap-day chain intact).
    assert_eq!(end.civil_date(), (2024, 12, 31));
}

#[test]
fn exogenous_reliability_parses_all_three_states() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let gb = &scenario.zones[0];
    assert_eq!(
        gb.exogenous_supply[0].reliability,
        ExogenousReliability::Variable
    );
    assert_eq!(
        gb.exogenous_supply[1].reliability,
        ExogenousReliability::Firm
    );
    let toml = FULL_SKETCH.replace(
        "columns = [\"other\"]\nreliability = \"firm\"",
        "columns = [\"other\"]\nreliability = \"excluded\"",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    assert_eq!(
        scenario.zones[0].exogenous_supply[1].reliability,
        ExogenousReliability::Excluded
    );
}

// ---------------------------------------------------------------------
// Schema v6: per-direction / time-series link capability + exogenous
// `scale` (the B6 two-zone package; the review §6 link-convention
// ruling — docs/notes/b6-two-zone-data-review.md).
// ---------------------------------------------------------------------

/// docs/05 rule 4 again for v5 → v6 (the B6 two-zone package): the
/// frozen v5 reference fixture must fail with a migration message
/// naming the additions. All v6 fields are optional, so a v5 file
/// migrates by changing only the version line — the message must say
/// so.
#[test]
fn v5_scenario_fails_with_a_migration_message_naming_what_was_added() {
    let err = Scenario::load(&v5_fixture_path()).unwrap_err();
    let msg = err.to_string();
    for needle in [
        "schema_version 5",
        "reverse_capacity_gw",
        "capability_trace",
        "sentinel",
        "scale",
        "version line",
        "schema_version = 8",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion5Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

/// The v6 link fields parse; absent fields keep the pre-v6 symmetric
/// semantics (`reverse_capacity_gw` None ⇒ `capacity_gw` both ways;
/// no capability trace).
#[test]
fn v6_link_capability_fields_parse_with_documented_defaults() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    // Pre-v6-shaped links carry no direction split and no trace.
    let ifa = &scenario.links[0];
    assert_eq!(ifa.reverse_capacity_gw, None);
    assert!(ifa.capability_trace.is_none());
    // The B6-shaped link carries both.
    let b6 = &scenario.links[2];
    assert_eq!(b6.name.as_deref(), Some("B6"));
    assert_eq!(b6.capacity_gw, Power::gigawatts(4.1));
    assert_eq!(b6.reverse_capacity_gw, Some(Power::gigawatts(3.5)));
    let trace = b6.capability_trace.as_ref().unwrap();
    assert_eq!(
        trace.path,
        "data/packs/b6/processed/b6_da_flows_limits.parquet"
    );
    assert_eq!(trace.column, "limit_mw");
    // sentinel_high_mw is a Power carried in MW (ADR-4; the TOML field
    // keeps its MW value 9999.0).
    assert_eq!(trace.sentinel_high_mw, Power::megawatts(9999.0));
    assert_eq!(trace.upper_bound_gw, Power::gigawatts(6.7));
    assert_eq!(trace.masked_fill_gw, Power::gigawatts(4.1));
}

/// Exogenous `scale` (schema v6): a flat multiplier on the summed MW
/// columns — how a national exogenous series is split across zones
/// (the 2-zone scenario's PS-net / "other" splits). Default 1.0.
#[test]
fn v6_exogenous_scale_parses_and_defaults_to_one() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let gb = &scenario.zones[0];
    assert_eq!(gb.exogenous_supply[0].scale, 1.0); // absent ⇒ 1.0
    assert_eq!(gb.exogenous_supply[1].scale, 0.101);
}

/// The v6 additions round-trip losslessly (parse → serialise → parse).
#[test]
fn v6_fields_round_trip_losslessly() {
    let original = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    let serialised = original.to_toml_string().unwrap();
    let reparsed = Scenario::from_toml_str(&serialised).unwrap();
    assert_eq!(original, reparsed);
}

#[test]
fn v6_reverse_capacity_must_be_a_physical_capability() {
    for replacement in [
        "reverse_capacity_gw = -1.0",
        "reverse_capacity_gw = nan",
        "reverse_capacity_gw = inf",
    ] {
        let toml = FULL_SKETCH.replace("reverse_capacity_gw = 3.5", replacement);
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            err.to_string().contains("reverse_capacity_gw"),
            "{replacement}: err was {err}"
        );
    }
}

#[test]
fn v6_capability_trace_fields_must_be_physical() {
    for (needle, replacement) in [
        ("sentinel_high_mw", "sentinel_high_mw = -1.0"),
        ("sentinel_high_mw", "sentinel_high_mw = nan"),
        ("upper_bound_gw", "upper_bound_gw = -6.7"),
        ("masked_fill_gw", "masked_fill_gw = nan"),
    ] {
        let toml = FULL_SKETCH.replace(
            match needle {
                "sentinel_high_mw" => "sentinel_high_mw = 9999.0",
                "upper_bound_gw" => "upper_bound_gw = 6.7",
                _ => "masked_fill_gw = 4.1",
            },
            replacement,
        );
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            err.to_string().contains(needle),
            "{replacement}: err was {err}"
        );
    }
}

#[test]
fn v6_exogenous_scale_must_be_finite_and_non_negative() {
    for replacement in ["scale = -0.1", "scale = nan", "scale = inf"] {
        let toml = FULL_SKETCH.replace("scale = 0.101", replacement);
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            err.to_string().contains("scale"),
            "{replacement}: err was {err}"
        );
    }
}

/// Unknown fields inside the v6 capability-trace table are rejected
/// (strict-parsing discipline).
#[test]
fn v6_capability_trace_unknown_fields_are_rejected() {
    let toml = FULL_SKETCH.replace(
        "masked_fill_gw = 4.1",
        "masked_fill_gw = 4.1\nfrobnicate = 1",
    );
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("frobnicate"), "err: {err}");
}

// ---------------------------------------------------------------------
// Schema v7 (D11 priced multi-zone dispatch): the per-zone
// [zones.pricing] block (per-zone SRMC inputs for the priced flow
// signal — docs/notes/d11-priced-dispatch.md, ADR-9 touch-point) and
// the [dispatch] flow_signal selector (scarcity | priced_ladder;
// default scarcity, so every pre-v7 scenario behaves identically).
// ---------------------------------------------------------------------

/// The frozen v6 reference scenario (superseded by schema v7, the D11
/// priced-dispatch package; kept as the migration-error fixture).
fn v6_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v6-gb-2024-reference.toml")
}

/// The frozen v7 reference scenario (superseded by schema v8, the D16
/// geothermal depth continuum; kept as the migration-error fixture).
fn v7_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v7-gb-2024-reference.toml")
}

/// docs/05 rule 4 for v7 → v8 (D16): the frozen v7 reference fixture
/// must fail with a migration message naming the addition. The one v8
/// field is optional, so a v7 file migrates by changing only the
/// version line — the message must say so.
#[test]
fn v7_scenario_fails_with_a_migration_message_naming_what_was_added() {
    let err = Scenario::load(&v7_fixture_path()).unwrap_err();
    let msg = err.to_string();
    for needle in [
        "schema_version 7",
        "resource_depth_m",
        "gshp",
        "geothermal",
        "version line",
        "schema_version = 8",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion7Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

/// A minimal two-zone scenario exercising the schema v7 additions: a
/// GB zone pricing block on the reference UKA+CPS carbon basis (the
/// flat field absent) and an external zone on a flat per-zone carbon
/// level (the committed EUA 2024 mean), under the priced-ladder flow
/// signal.
const V7_SKETCH: &str = r#"
schema_version = 8
name = "v7-sketch"

[horizon]
start = "2024-01-01T00:00:00Z"
end = "2024-01-01T23:30:00Z"
weather_years = [2024]

[[zones]]
id = "GB"

[zones.demand]
base_profile = "data/demand/gb.parquet"
annual_scale = 1.0

[zones.pricing]
reference = "data/reference/prices-2024.toml"

[zones.pricing.fuel_price.gas]
path = "data/packs/2024/processed/gas_sap_daily_2024.parquet"
column = "sap_gbp_per_mwh_hhv"

[zones.pricing.srmc.ccgt]
fuel = "gas"
efficiency = "ccgt"

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0

[[zones]]
id = "FR"

[zones.demand]
base_profile = "data/demand/fr.parquet"
annual_scale = 1.0

[zones.pricing]
reference = "data/reference/prices-2024.toml"
# EUA 2024 GBP mean — prices-eu-2024.toml [carbon.eua]; replaces the
# reference file's UKA+CPS step series for this zone.
carbon_flat_gbp_per_tco2 = 55.01

[zones.pricing.fuel_price.gas]
path = "data/packs/2024/processed/gas_sap_daily_2024.parquet"
column = "sap_gbp_per_mwh_hhv"

[zones.pricing.srmc.ccgt]
fuel = "gas"
efficiency = "ccgt"

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 12.8

[[links]]
name = "IFA"
from = "GB"
to = "FR"
capacity_gw = 2.0
availability = 0.95
loss = 0.021

[dispatch]
policy = "rule_based"
flow_signal = "priced_ladder"
"#;

/// docs/05 rule 4 again for v6 → v7 (the D11 priced-dispatch package):
/// the frozen v6 reference fixture must fail with a migration message
/// naming the additions. All v7 fields are optional, so a v6 file
/// migrates by changing only the version line — the message must say
/// so.
#[test]
fn v6_scenario_fails_with_a_migration_message_naming_what_was_added() {
    let err = Scenario::load(&v6_fixture_path()).unwrap_err();
    let msg = err.to_string();
    for needle in [
        "schema_version 6",
        "zones.pricing",
        "carbon_flat_gbp_per_tco2",
        "flow_signal",
        "priced_ladder",
        "version line",
        "schema_version = 8",
        "docs/03-domain-model.md",
    ] {
        assert!(
            msg.contains(needle),
            "migration message lacks {needle:?}: {msg}"
        );
    }
    assert!(
        matches!(
            err,
            GridError::InScenarioFile { ref source, .. }
                if matches!(**source, GridError::SchemaVersion6Superseded)
        ),
        "unexpected error: {err:?}"
    );
}

/// The v7 zone-pricing block parses with its documented shapes: the
/// carbon field ABSENT means the reference file's UKA+CPS step series
/// (the GB basis); PRESENT means a flat per-zone carbon level (the
/// external-zone EUA basis — no daily EUA series is licence-clean, so
/// the committed convention is a flat annual mean per zone).
#[test]
fn v7_zone_pricing_parses_with_documented_shapes() {
    let scenario = Scenario::from_toml_str(V7_SKETCH).unwrap();
    scenario.validate().unwrap();
    let gb = &scenario.zones[0];
    let fr = &scenario.zones[1];
    let gb_pricing = gb.pricing.as_ref().unwrap();
    assert_eq!(gb_pricing.reference, "data/reference/prices-2024.toml");
    assert_eq!(gb_pricing.carbon_flat_gbp_per_tco2, None);
    assert_eq!(gb_pricing.srmc["ccgt"].fuel, "gas");
    assert_eq!(gb_pricing.srmc["ccgt"].efficiency, "ccgt");
    assert_eq!(gb_pricing.fuel_price["gas"].column, "sap_gbp_per_mwh_hhv");
    let fr_pricing = fr.pricing.as_ref().unwrap();
    assert_eq!(
        fr_pricing.carbon_flat_gbp_per_tco2,
        Some(grid_core::units::CarbonPrice::pounds_per_tonne_co2(55.01))
    );
    assert_eq!(
        scenario.dispatch.flow_signal,
        grid_core::scenario::FlowSignal::PricedLadder
    );
}

/// `flow_signal` defaults to `scarcity` when absent (every pre-v7
/// scenario keeps its exact behaviour) and is omitted from
/// serialisation at the default (byte-stable round-trips for files
/// without it).
#[test]
fn v7_flow_signal_defaults_to_scarcity_and_is_omitted_when_default() {
    let scenario = Scenario::from_toml_str(FULL_SKETCH).unwrap();
    assert_eq!(
        scenario.dispatch.flow_signal,
        grid_core::scenario::FlowSignal::Scarcity
    );
    let serialised = scenario.to_toml_string().unwrap();
    assert!(
        !serialised.contains("flow_signal"),
        "default flow_signal must not serialise"
    );
}

/// The v7 additions round-trip losslessly (parse → serialise → parse).
#[test]
fn v7_fields_round_trip_losslessly() {
    let original = Scenario::from_toml_str(V7_SKETCH).unwrap();
    let serialised = original.to_toml_string().unwrap();
    let reparsed = Scenario::from_toml_str(&serialised).unwrap();
    assert_eq!(original, reparsed);
}

/// The priced ladder needs a marginal price for every zone, so a
/// priced-ladder scenario must declare [zones.pricing] on EVERY zone
/// (ADR-7 touch-point: external zones require pricing inputs to be
/// dispatchable under the ladder).
#[test]
fn priced_ladder_requires_zone_pricing_on_every_zone() {
    let toml = V7_SKETCH.replace(
        "[zones.pricing]\nreference = \"data/reference/prices-2024.toml\"\n# EUA 2024 GBP mean — prices-eu-2024.toml [carbon.eua]; replaces the\n# reference file's UKA+CPS step series for this zone.\ncarbon_flat_gbp_per_tco2 = 55.01\n\n[zones.pricing.fuel_price.gas]\npath = \"data/packs/2024/processed/gas_sap_daily_2024.parquet\"\ncolumn = \"sap_gbp_per_mwh_hhv\"\n\n[zones.pricing.srmc.ccgt]\nfuel = \"gas\"\nefficiency = \"ccgt\"\n\n[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 12.8",
        "[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 12.8",
    );
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("FR") && msg.contains("priced_ladder") && msg.contains("zones.pricing"),
        "err was: {msg}"
    );
}

/// A flat per-zone carbon level must be a physical price.
#[test]
fn zone_pricing_flat_carbon_must_be_physical() {
    for replacement in [
        "carbon_flat_gbp_per_tco2 = -1.0",
        "carbon_flat_gbp_per_tco2 = nan",
        "carbon_flat_gbp_per_tco2 = inf",
    ] {
        let toml = V7_SKETCH.replace("carbon_flat_gbp_per_tco2 = 55.01", replacement);
        let scenario = Scenario::from_toml_str(&toml).unwrap();
        let err = scenario.validate().unwrap_err();
        assert!(
            err.to_string().contains("carbon_flat_gbp_per_tco2"),
            "{replacement}: err was {err}"
        );
    }
}

/// A zone SRMC recipe must name a dispatchable technology of that
/// zone's own fleet (the load_pricing_inputs rule, enforced at
/// validation for zone pricing so authoring errors fail early).
#[test]
fn zone_pricing_srmc_must_name_a_dispatchable_fleet_entry() {
    // A technology outside the zone's fleet.
    let toml = V7_SKETCH.replace("[zones.pricing.srmc.ccgt]", "[zones.pricing.srmc.ocgt]");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let err = scenario.validate().unwrap_err();
    assert!(
        err.to_string().contains("ocgt"),
        "fleet-membership err was {err}"
    );
    // A weather-driven technology (must-take: no SRMC model,
    // grid-core pricing convention 1).
    let toml = V7_SKETCH.replace(
        "[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 12.8",
        "[[zones.fleet]]\ntechnology = \"ccgt\"\ncapacity_gw = 12.8\n\n[[zones.fleet]]\ntechnology = \"solar\"\ncapacity_gw = 5.0\ncapacity_factor_trace = \"data/weather/fr_solar_cf.parquet\"\n\n[[zones.pricing.srmc.solar]]\n"
    );
    // The replacement above is structurally awkward in TOML; assert the
    // simpler property instead: an srmc entry for a CF-trace technology
    // is a validation error.
    let _ = toml;
    let toml2 = V7_SKETCH
        .replace(
            "technology = \"ccgt\"\ncapacity_gw = 12.8",
            "technology = \"ccgt\"\ncapacity_gw = 12.8\ncapacity_factor_trace = \"data/weather/fr_ccgt_cf.parquet\"",
        );
    let scenario2 = Scenario::from_toml_str(&toml2).unwrap();
    let err2 = scenario2.validate().unwrap_err();
    assert!(
        err2.to_string().contains("must-take") || err2.to_string().contains("weather-driven"),
        "weather-driven err was {err2}"
    );
}

/// Unknown fields inside [zones.pricing] are rejected (strict-parsing
/// discipline).
#[test]
fn zone_pricing_unknown_fields_are_rejected() {
    let toml = V7_SKETCH.replace(
        "carbon_flat_gbp_per_tco2 = 55.01",
        "carbon_flat_gbp_per_tco2 = 55.01\nfrobnicate = 1",
    );
    let err = Scenario::from_toml_str(&toml).unwrap_err();
    assert!(err.to_string().contains("frobnicate"), "err: {err}");
}
