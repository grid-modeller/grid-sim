//! Pinned regression + enforcement tests for the committed
//! published-pathway reference file
//! (`data/reference/pathways-published.toml`, schema
//! `pathways-published-v1`; evidence
//! `docs/notes/stage7-pathways-data-report.md`, adjudicated
//! ACCEPT-WITH-CONDITIONS in `docs/notes/stage7-pathways-data-review.md`
//! — review conditions 5–8 bind the parser and the scenario package).
//!
//! Two jobs, per the costs-reference-v1 precedent:
//! 1. **Pins** — a silent edit to the committed file fails these tests
//!    (every value below is transcribed from the cited primary and was
//!    independently re-derived by the data reviewer).
//! 2. **Machine enforcement (review condition 5)** — `mappable = false`
//!    aggregates are typed as [`ExcludedAggregate`], never as fleet
//!    capacity; a `mappable = true` aggregate, an aggregate colliding
//!    with a fleet technology, and an out-of-step surplus-electrolysis
//!    pair are structured errors, tested on inline fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_core::GridError;
use grid_core::pathways_published::{
    ExclusionRecord, PATHWAYS_PUBLISHED_SCHEMA, PathwaysPublished,
};
use grid_core::scenario::StorageKind;
use grid_core::units::{Energy, Power};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// TWh → the canonical GWh carrier, with the parser's own conversion
/// arithmetic (bit-identical comparisons).
fn twh(value: f64) -> Energy {
    Energy::gigawatt_hours(value * 1000.0)
}

fn reference() -> PathwaysPublished {
    PathwaysPublished::load(&repo_root().join("data/reference/pathways-published.toml")).unwrap()
}

// ---------------------------------------------------------------------
// Pins on the committed file.
// ---------------------------------------------------------------------

#[test]
fn schema_and_snapshot_years_are_pinned() {
    assert_eq!(PATHWAYS_PUBLISHED_SCHEMA, "pathways-published-v1");
    let reference = reference();
    assert_eq!(reference.snapshot_years, vec![2035, 2050]);
    assert_eq!(reference.pathways.len(), 2);
    assert!(
        reference
            .pathways
            .contains_key("fes2025_electric_engagement")
    );
    assert!(reference.pathways.contains_key("ccc_cb7_balanced"));
}

#[test]
fn fes_electric_engagement_pins() {
    let reference = reference();
    let fes = &reference.pathways["fes2025_electric_engagement"];
    assert_eq!(fes.geography, "GB");
    assert_eq!(fes.attribution, "Supported by National Energy SO Open Data");
    assert!(fes.aggregates.is_empty(), "FES carries no aggregates");

    let y2035 = fes.year(2035).unwrap();
    assert_eq!(y2035.demand, twh(450.076));
    assert_eq!(y2035.peak_demand, Some(Power::gigawatts(82.042)));
    assert_eq!(y2035.surplus_electrolysis_excluded, None);
    let cap = |tech: &str| {
        y2035
            .fleet
            .iter()
            .find(|f| f.technology.as_str() == tech)
            .unwrap_or_else(|| panic!("no {tech} in FES 2035"))
            .capacity
    };
    assert_eq!(cap("ccgt"), Power::gigawatts(20.0968));
    assert_eq!(cap("ocgt"), Power::gigawatts(7.0555));
    assert_eq!(cap("offshore_wind"), Power::gigawatts(67.5195));
    assert_eq!(cap("hydrogen_turbine"), Power::gigawatts(0.9958));
    assert_eq!(y2035.fleet.len(), 14);

    let y2050 = fes.year(2050).unwrap();
    assert_eq!(y2050.demand, twh(784.736));
    assert_eq!(y2050.peak_demand, Some(Power::gigawatts(143.625)));
    let cap = |tech: &str| {
        y2050
            .fleet
            .iter()
            .find(|f| f.technology.as_str() == tech)
            .unwrap()
            .capacity
    };
    assert_eq!(cap("ccgt"), Power::gigawatts(31.035));
    assert_eq!(cap("nuclear"), Power::gigawatts(21.56));
    assert_eq!(cap("offshore_wind"), Power::gigawatts(96.3654));
    assert_eq!(cap("solar"), Power::gigawatts(100.9131));
    assert_eq!(cap("interconnector"), Power::gigawatts(24.4));
    assert_eq!(cap("hydrogen_turbine"), Power::gigawatts(27.5201));

    let battery = y2050
        .storage
        .iter()
        .find(|s| s.kind == StorageKind::Battery)
        .unwrap();
    assert_eq!(battery.power, Power::gigawatts(40.4449));
    assert_eq!(battery.energy, Energy::gigawatt_hours(60.257));
    assert_eq!(
        battery.energy_precision, None,
        "FES energies are 4 dp CSV sums"
    );
    let ldes = y2050
        .storage
        .iter()
        .find(|s| s.kind == StorageKind::PumpedHydro)
        .unwrap();
    assert_eq!(ldes.power, Power::gigawatts(16.5781));
    assert_eq!(ldes.energy, Energy::gigawatt_hours(223.3982));

    // Exclusions with magnitude (e1/e4) and the notes (e5/e6).
    match &fes.exclusions["ccs_gas_in_ccgt_gw"] {
        ExclusionRecord::CapacityByYear(map) => {
            assert_eq!(map[&2035], Power::gigawatts(7.183));
            assert_eq!(map[&2050], Power::gigawatts(26.645));
        }
        other => panic!("ccs_gas_in_ccgt_gw: expected a capacity year-map, got {other:?}"),
    }
    match &fes.exclusions["non_networked_solar_gw"] {
        ExclusionRecord::CapacityByYear(map) => {
            assert_eq!(map[&2050], Power::gigawatts(0.0556));
        }
        other => panic!("non_networked_solar_gw: {other:?}"),
    }
    assert!(matches!(
        &fes.exclusions["dsr_and_v2g"],
        ExclusionRecord::Note(_)
    ));
    assert!(matches!(
        &fes.exclusions["electrolysis_demand_note"],
        ExclusionRecord::Note(_)
    ));
}

#[test]
fn ccc_balanced_pathway_pins() {
    let reference = reference();
    let ccc = &reference.pathways["ccc_cb7_balanced"];
    assert_eq!(
        ccc.geography, "UK",
        "the UK-not-GB scope flag is load-bearing (review condition 6)"
    );

    let y2035 = ccc.year(2035).unwrap();
    assert_eq!(y2035.demand, twh(443.541));
    assert_eq!(
        y2035.peak_demand, None,
        "no CCC peak exists (quarantine 3) — a constructed peak is a scenario convention"
    );
    assert_eq!(y2035.surplus_electrolysis_excluded, Some(twh(29.0)));
    assert_eq!(y2035.fleet.len(), 5, "only five unambiguous CCC mappings");

    let y2050 = ccc.year(2050).unwrap();
    assert_eq!(y2050.demand, twh(692.025));
    assert_eq!(y2050.surplus_electrolysis_excluded, Some(twh(89.0)));
    let cap = |tech: &str| {
        y2050
            .fleet
            .iter()
            .find(|f| f.technology.as_str() == tech)
            .unwrap()
            .capacity
    };
    assert_eq!(cap("nuclear"), Power::gigawatts(10.98));
    assert_eq!(cap("offshore_wind"), Power::gigawatts(125.0));
    assert_eq!(cap("onshore_wind"), Power::gigawatts(37.41));
    assert_eq!(cap("solar"), Power::gigawatts(106.4));
    assert_eq!(cap("interconnector"), Power::gigawatts(27.938));

    // CB7 storage: GW from sheet 7.5.4, GWh from Table 7.5.1 (rounded
    // integers) — the energy_precision stamp MUST travel (review
    // condition 5).
    let battery = y2035
        .storage
        .iter()
        .find(|s| s.kind == StorageKind::Battery)
        .unwrap();
    assert_eq!(battery.power, Power::gigawatts(21.04));
    assert_eq!(battery.energy, Energy::gigawatt_hours(54.0));
    assert!(
        battery
            .energy_precision
            .as_deref()
            .unwrap()
            .contains("rounded integer"),
        "CB7 battery GWh must carry its rounding stamp"
    );
    let medium = y2050
        .storage
        .iter()
        .find(|s| s.kind == StorageKind::PumpedHydro)
        .unwrap();
    assert_eq!(medium.power, Power::gigawatts(6.92));
    assert_eq!(medium.energy, Energy::gigawatt_hours(433.0));
    assert!(medium.energy_precision.is_some());

    // The five published buckets the engine cannot take stay typed as
    // ExcludedAggregate — named exclusions, never fleet capacity.
    let names: Vec<&str> = ccc.aggregates.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "unabated_gas",
            "low_carbon_dispatchable",
            "other_generation",
            "ccs_biomass",
            "smart_demand_flexibility",
        ]
    );
    let aggregate = |name: &str| {
        ccc.aggregates
            .iter()
            .find(|a| a.name == name)
            .unwrap_or_else(|| panic!("no aggregate {name}"))
    };
    assert_eq!(
        aggregate("unabated_gas").capacity_by_year[&2035],
        Power::gigawatts(29.71)
    );
    assert_eq!(
        aggregate("unabated_gas").capacity_by_year[&2050],
        Power::gigawatts(0.0)
    );
    assert_eq!(
        aggregate("low_carbon_dispatchable").capacity_by_year[&2050],
        Power::gigawatts(38.28)
    );
    assert_eq!(
        aggregate("other_generation").capacity_by_year[&2050],
        Power::gigawatts(5.5)
    );
    assert_eq!(
        aggregate("ccs_biomass").capacity_by_year[&2050],
        Power::gigawatts(1.29)
    );

    // The c1 exclusion (review condition 1) is machine-visible in BOTH
    // sites and the parser has verified they agree.
    match &ccc.exclusions["surplus_electrolysis_demand_twh"] {
        ExclusionRecord::EnergyByYear(map) => {
            assert_eq!(map[&2035], twh(29.0));
            assert_eq!(map[&2050], twh(89.0));
        }
        other => panic!("surplus_electrolysis_demand_twh: {other:?}"),
    }
    assert!(matches!(
        &ccc.exclusions["demand_basis_wedge"],
        ExclusionRecord::Note(_)
    ));
}

// ---------------------------------------------------------------------
// Machine enforcement on inline fixtures (review condition 5).
// ---------------------------------------------------------------------

fn fixture(aggregates: &str, year_extra: &str, exclusions: &str) -> String {
    format!(
        r#"
schema = "pathways-published-v1"
assembled = 2026-07-06
snapshot_years = [2035]

[sources.src]
title = "t"
url = "https://example.org"
sha256 = "00"
retrieved = 2026-07-06
licence = "l"

[pathways.p]
name = "P"
edition = "e"
geography = "GB"
attribution = "a"

[[pathways.p.years]]
year = 2035
demand_twh = 100.0
{year_extra}

[[pathways.p.years.fleet]]
technology = "solar"
capacity_gw = 10.0

[[pathways.p.years.storage]]
kind = "battery"
power_gw = 1.0
energy_gwh = 2.0
{exclusions}
{aggregates}
"#
    )
}

#[test]
fn minimal_fixture_parses() {
    let parsed = PathwaysPublished::from_toml_str(&fixture("", "", "")).unwrap();
    let year = parsed.pathways["p"].year(2035).unwrap();
    assert_eq!(year.fleet[0].capacity, Power::gigawatts(10.0));
}

#[test]
fn a_mappable_true_aggregate_is_refused() {
    let toml = fixture(
        r#"
[[pathways.p.aggregates]]
name = "some_bucket"
mappable = true
capacity_gw = { y2035 = 5.0 }
definition = "d"
suggested_treatment = "s"
"#,
        "",
        "",
    );
    match PathwaysPublished::from_toml_str(&toml) {
        Err(GridError::InvalidPathwaysReference { reason }) => {
            assert!(
                reason.contains("mappable"),
                "the refusal must name the mappable flag: {reason}"
            );
        }
        other => panic!("expected InvalidPathwaysReference, got {other:?}"),
    }
}

#[test]
fn an_aggregate_colliding_with_a_fleet_technology_is_refused() {
    let toml = fixture(
        r#"
[[pathways.p.aggregates]]
name = "solar"
mappable = false
capacity_gw = { y2035 = 5.0 }
definition = "d"
suggested_treatment = "s"
"#,
        "",
        "",
    );
    match PathwaysPublished::from_toml_str(&toml) {
        Err(GridError::InvalidPathwaysReference { reason }) => {
            assert!(reason.contains("solar"), "reason: {reason}");
        }
        other => panic!("expected InvalidPathwaysReference, got {other:?}"),
    }
}

#[test]
fn an_aggregate_missing_a_snapshot_year_is_refused() {
    let toml = fixture(
        r#"
[[pathways.p.aggregates]]
name = "bucket"
mappable = false
capacity_gw = { y2036 = 5.0 }
definition = "d"
suggested_treatment = "s"
"#,
        "",
        "",
    );
    assert!(matches!(
        PathwaysPublished::from_toml_str(&toml),
        Err(GridError::InvalidPathwaysReference { .. })
    ));
}

#[test]
fn an_out_of_step_surplus_electrolysis_pair_is_refused() {
    // Year block says 29, the exclusions register says 28: the two
    // sites must stay in step (the committed file's own rule).
    let toml = fixture(
        "",
        "surplus_electrolysis_excluded_twh = 29.0",
        r#"
[pathways.p.exclusions]
surplus_electrolysis_demand_twh = { y2035 = 28.0 }
"#,
    );
    match PathwaysPublished::from_toml_str(&toml) {
        Err(GridError::InvalidPathwaysReference { reason }) => {
            assert!(
                reason.contains("surplus"),
                "the refusal must name the out-of-step pair: {reason}"
            );
        }
        other => panic!("expected InvalidPathwaysReference, got {other:?}"),
    }
}

#[test]
fn a_year_magnitude_exclusion_without_a_unit_suffix_is_refused() {
    let toml = fixture(
        "",
        "",
        r#"
[pathways.p.exclusions]
mystery_quantity = { y2035 = 1.0 }
"#,
    );
    assert!(matches!(
        PathwaysPublished::from_toml_str(&toml),
        Err(GridError::InvalidPathwaysReference { .. })
    ));
}

#[test]
fn unknown_fields_and_wrong_schemas_are_refused() {
    let toml =
        fixture("", "", "").replace("demand_twh = 100.0", "demand_twh = 100.0\nsurprise = 1");
    assert!(matches!(
        PathwaysPublished::from_toml_str(&toml),
        Err(GridError::PathwaysReferenceParse { .. })
    ));
    let toml = fixture("", "", "").replace("pathways-published-v1", "pathways-published-v2");
    match PathwaysPublished::from_toml_str(&toml) {
        Err(GridError::InvalidPathwaysReference { reason }) => {
            assert!(reason.contains("pathways-published-v1"), "reason: {reason}");
        }
        other => panic!("expected InvalidPathwaysReference, got {other:?}"),
    }
}

#[test]
fn duplicate_fleet_technologies_within_a_year_are_refused() {
    let toml = fixture("", "", "").replace(
        "[[pathways.p.years.storage]]",
        "[[pathways.p.years.fleet]]\ntechnology = \"solar\"\ncapacity_gw = 1.0\n\n[[pathways.p.years.storage]]",
    );
    assert!(matches!(
        PathwaysPublished::from_toml_str(&toml),
        Err(GridError::InvalidPathwaysReference { .. })
    ));
}

#[test]
fn loading_is_deterministic() {
    assert!(reference() == reference());
}
