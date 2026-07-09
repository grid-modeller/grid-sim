//! Run-input loading from a schema-v2 scenario: column selection,
//! MW→GW at the boundary, multi-column summing, demand adjustment,
//! alignment checks, availability conversion, and multi-file (per-year)
//! trace assembly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use arrow_array::builder::{Float64Builder, TimestampMicrosecondBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use grid_adequacy::load_run_inputs;
use grid_core::GridError;
use grid_core::scenario::Scenario;
use grid_core::units::Power;

const T0: i64 = 1_704_067_200 * 1_000_000; // 2024-01-01T00:00:00Z
const HALF_HOUR: i64 = 1_800 * 1_000_000;

/// Write a multi-column half-hourly MW trace parquet starting at `t0`.
fn write_trace(name: &str, t0: i64, columns: &[(&str, &[f64])]) -> PathBuf {
    let n = columns[0].1.len();
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let mut fields = vec![Field::new("utc_start", ts_type, false)];
    for (col, _) in columns {
        fields.push(Field::new(*col, DataType::Float64, false));
    }
    let schema = Arc::new(Schema::new(fields));

    let mut stamps = TimestampMicrosecondBuilder::new();
    for i in 0..n {
        stamps.append_value(t0 + i as i64 * HALF_HOUR);
    }
    let mut arrays: Vec<ArrayRef> = vec![Arc::new(stamps.finish().with_timezone("UTC"))];
    for (_, values) in columns {
        let mut b = Float64Builder::new();
        for &v in *values {
            b.append_value(v);
        }
        arrays.push(Arc::new(b.finish()));
    }
    let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();

    let dir = std::env::temp_dir().join("grid-adequacy-inputs-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = parquet::arrow::ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    path
}

/// A minimal v2 scenario whose zone tables are supplied as TOML text.
fn scenario_with(periods: usize, zone_tables: &str) -> Scenario {
    let end_micros = T0 + (periods as i64 - 1) * HALF_HOUR;
    let end = grid_core::time::UtcInstant::from_unix_micros(end_micros);
    Scenario::from_toml_str(&format!(
        r#"
schema_version = 8
name = "inputs-test"

[horizon]
start = "2024-01-01T00:00:00Z"
end = "{end}"
weather_years = [2024]

[[zones]]
id = "GB"
{zone_tables}

[dispatch]
policy = "rule_based"
"#
    ))
    .unwrap()
}

// ---------------------------------------------------------------------
// Demand loading.
// ---------------------------------------------------------------------

#[test]
fn demand_is_scaled_and_adjusted_and_converted_from_mw() {
    let demand = write_trace(
        "demand.parquet",
        T0,
        &[("underlying_demand", &[20_000.0, 30_000.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
annual_scale = 2.0
extra_demand_gw = 0.5

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0
"#,
            demand.to_str().unwrap()
        ),
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    // demand(t) = base(t) × scale + adjustment: (20 GW × 2) + 0.5.
    assert_eq!(
        inputs.demand.values(),
        &[Power::gigawatts(40.5), Power::gigawatts(60.5)]
    );
}

#[test]
fn demand_column_selection_defaults_to_the_d3_convention() {
    let demand = write_trace(
        "demand-columns.parquet",
        T0,
        &[
            ("nd", &[1_000.0, 1_000.0]),
            ("underlying_demand", &[2_000.0, 2_000.0]),
        ],
    );
    // No `column` field → underlying_demand.
    let scenario = scenario_with(
        2,
        &format!(
            "[zones.demand]\nbase_profile = \"{}\"\nannual_scale = 1.0\n",
            demand.to_str().unwrap()
        ),
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    assert_eq!(inputs.demand.values()[0], Power::gigawatts(2.0));
    // Explicit column selection is honoured.
    let scenario = scenario_with(
        2,
        &format!(
            "[zones.demand]\nbase_profile = \"{}\"\ncolumn = \"nd\"\nannual_scale = 1.0\n",
            demand.to_str().unwrap()
        ),
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    assert_eq!(inputs.demand.values()[0], Power::gigawatts(1.0));
}

// ---------------------------------------------------------------------
// Exogenous supply.
// ---------------------------------------------------------------------

#[test]
fn exogenous_columns_are_summed() {
    let demand = write_trace(
        "demand2.parquet",
        T0,
        &[("underlying_demand", &[20_000.0, 20_000.0])],
    );
    let flows = write_trace(
        "flows.parquet",
        T0,
        &[("a", &[1_000.0, -500.0]), ("b", &[2_000.0, 1_500.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
annual_scale = 1.0

[[zones.exogenous_supply]]
label = "net_imports"
path = "{}"
columns = ["a", "b"]
imports = true
reliability = "variable"
"#,
            demand.to_str().unwrap(),
            flows.to_str().unwrap()
        ),
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    assert_eq!(inputs.exogenous.len(), 1);
    assert!(inputs.exogenous[0].imports);
    assert_eq!(
        inputs.exogenous[0].trace.values(),
        &[Power::gigawatts(3.0), Power::gigawatts(1.0)]
    );
}

#[test]
fn an_exogenous_supply_without_columns_is_rejected() {
    let demand = write_trace(
        "demand5.parquet",
        T0,
        &[("underlying_demand", &[20_000.0, 20_000.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
annual_scale = 1.0

[[zones.exogenous_supply]]
label = "empty"
path = "irrelevant.parquet"
columns = []
reliability = "firm"
"#,
            demand.to_str().unwrap()
        ),
    );
    let err = load_run_inputs(&scenario, "/".as_ref()).unwrap_err();
    assert!(err.to_string().contains("empty"), "error: {err}");
}

// ---------------------------------------------------------------------
// Multi-file (per-year) trace assembly through the scenario.
// ---------------------------------------------------------------------

#[test]
fn per_year_demand_files_assemble_into_one_horizon() {
    let y1 = write_trace(
        "demand-y1.parquet",
        T0,
        &[("underlying_demand", &[10_000.0, 20_000.0])],
    );
    let y2 = write_trace(
        "demand-y2.parquet",
        T0 + 2 * HALF_HOUR,
        &[("underlying_demand", &[30_000.0])],
    );
    let scenario = scenario_with(
        3,
        &format!(
            "[zones.demand]\nbase_profile = [\"{}\", \"{}\"]\nannual_scale = 1.0\n",
            y1.to_str().unwrap(),
            y2.to_str().unwrap()
        ),
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    let gw: Vec<f64> = inputs
        .demand
        .values()
        .iter()
        .map(|p| p.as_gigawatts())
        .collect();
    assert_eq!(gw, [10.0, 20.0, 30.0]);
}

#[test]
fn non_consecutive_per_year_files_are_rejected() {
    let y1 = write_trace(
        "demand-gap-y1.parquet",
        T0,
        &[("underlying_demand", &[10_000.0])],
    );
    let y2 = write_trace(
        "demand-gap-y2.parquet",
        T0 + 2 * HALF_HOUR, // one period missing
        &[("underlying_demand", &[30_000.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            "[zones.demand]\nbase_profile = [\"{}\", \"{}\"]\nannual_scale = 1.0\n",
            y1.to_str().unwrap(),
            y2.to_str().unwrap()
        ),
    );
    let err = load_run_inputs(&scenario, "/".as_ref()).unwrap_err();
    assert!(
        matches!(err, GridError::TraceNotConsecutive { .. }),
        "unexpected error: {err:?}"
    );
}

// ---------------------------------------------------------------------
// Alignment and unsupported features.
// ---------------------------------------------------------------------

#[test]
fn a_trace_starting_at_the_wrong_instant_is_rejected() {
    let demand = write_trace(
        "late-demand.parquet",
        T0 + HALF_HOUR, // starts one period after the horizon
        &[("underlying_demand", &[20_000.0, 20_000.0])],
    );
    // Two periods expected, file has two — but starts late, so the
    // period-count check passes and the start check must catch it...
    // except a late 2-period file cannot exist for a 2-period horizon
    // without the count failing first; use a 2-period horizon and a
    // 2-period late file: count ok, start wrong.
    let scenario = scenario_with(
        2,
        &format!(
            "[zones.demand]\nbase_profile = \"{}\"\nannual_scale = 1.0\n",
            demand.to_str().unwrap()
        ),
    );
    let err = load_run_inputs(&scenario, "/".as_ref()).unwrap_err();
    assert!(
        matches!(err, GridError::TraceStartMismatch { .. }),
        "unexpected error: {err:?}"
    );
}

/// Schema v5: a heating block whose temperature trace does not exist
/// fails with the structured missing-trace error naming the path —
/// the overlay is loaded like every other input, never silently
/// skipped. (The overlay computation itself is covered by the
/// grid-core heating suite and tests/heating.rs.)
#[test]
fn a_heating_block_with_a_missing_temperature_trace_fails_loudly() {
    let demand = write_trace(
        "demand3.parquet",
        T0,
        &[("underlying_demand", &[20_000.0, 20_000.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
annual_scale = 1.0

[zones.demand.heating]
delivered_heat_twh = 410.5
electrified_share = 0.5
dhw_fraction = 0.17
temperature_trace = {{ path = "data/weather/nonexistent_t2m.parquet", column = "t2m_pop" }}

[[zones.demand.heating.entries]]
kind = "ashp"
share = 1.0
"#,
            demand.to_str().unwrap()
        ),
    );
    // Base dir = the repo root, so the committed heating-cop.toml
    // resolves and the missing TEMPERATURE trace is what fails.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let err = load_run_inputs(&scenario, &root).unwrap_err();
    assert!(
        matches!(err, GridError::TraceFileMissing { .. }),
        "unexpected error: {err:?}"
    );
    assert!(
        err.to_string().contains("nonexistent_t2m.parquet"),
        "error: {err}"
    );
}

#[test]
fn availability_with_wrong_month_count_is_rejected_at_load() {
    let demand = write_trace(
        "demand4.parquet",
        T0,
        &[("underlying_demand", &[20_000.0, 20_000.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
annual_scale = 1.0

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0
availability = {{ monthly = [0.5, 0.5] }}
"#,
            demand.to_str().unwrap()
        ),
    );
    let err = load_run_inputs(&scenario, "/".as_ref()).unwrap_err();
    assert!(err.to_string().contains("12"), "error: {err}");
}

// ---------------------------------------------------------------------
// Schema v6: exogenous `scale` and the link-capability assembly (the
// B6 ruling's sentinel handling — docs/notes/b6-two-zone-data-review.md
// §6a).
// ---------------------------------------------------------------------

use grid_adequacy::build_link_capability;
use grid_core::scenario::LinkCapabilityTraceSpec;
use grid_core::time::UtcInstant;

#[test]
fn exogenous_scale_splits_the_summed_series() {
    let demand = write_trace(
        "demand-scale.parquet",
        T0,
        &[("underlying_demand", &[20_000.0, 20_000.0])],
    );
    let flows = write_trace("flows-scale.parquet", T0, &[("ps", &[1_000.0, -500.0])]);
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
annual_scale = 1.0

[[zones.exogenous_supply]]
label = "pumped_storage_net"
path = "{}"
columns = ["ps"]
scale = 0.25
reliability = "excluded"
"#,
            demand.to_str().unwrap(),
            flows.to_str().unwrap()
        ),
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    // The split share applies AFTER column summing, preserving signs.
    assert_eq!(
        inputs.exogenous[0].trace.values(),
        &[Power::gigawatts(0.25), Power::gigawatts(-0.125)]
    );
}

fn b6_spec() -> LinkCapabilityTraceSpec {
    LinkCapabilityTraceSpec {
        path: "synthetic/b6.parquet".to_owned(),
        column: "limit_mw".to_owned(),
        sentinel_high_mw: Power::megawatts(9999.0),
        upper_bound_gw: Power::gigawatts(6.7),
        masked_fill_gw: Power::gigawatts(4.1),
    }
}

fn t(index: i64) -> UtcInstant {
    UtcInstant::from_unix_micros(T0 + index * HALF_HOUR)
}

#[test]
fn capability_assembly_applies_the_ruling_sentinel_handling() {
    // Six-period horizon covering every case of the ruling:
    //   0: observed 4,100 MW           → 4.1 GW, observed
    //   1: high sentinel (10,000 MW)   → 6.7 GW upper bound, observed
    //   2: zero sentinel               → 4.1 GW fill, MASKED
    //   3: NaN row (value None)        → 4.1 GW fill, MASKED
    //   4: missing row                 → 4.1 GW fill, MASKED
    //   5: observed 2,700 MW           → 2.7 GW, observed
    let points = vec![
        (t(0), Some(Power::megawatts(4_100.0))),
        (t(1), Some(Power::megawatts(10_000.0))),
        (t(2), Some(Power::megawatts(0.0))),
        (t(3), None),
        // t(4) missing entirely.
        (t(5), Some(Power::megawatts(2_700.0))),
    ];
    let capability = build_link_capability(&points, &b6_spec(), t(0), 6).unwrap();
    let gw: Vec<f64> = capability
        .forward
        .iter()
        .map(|p| p.as_gigawatts())
        .collect();
    assert_eq!(gw, vec![4.1, 6.7, 4.1, 4.1, 4.1, 2.7]);
    assert_eq!(
        capability.observed,
        vec![true, true, false, false, false, true]
    );
}

#[test]
fn capability_assembly_ignores_points_outside_the_horizon() {
    // Rows before the start and past the end are simply not the run's
    // business (the b6 file spans 2023 → 2026; a 2024 run reads 2024).
    let points = vec![
        (t(-1), Some(Power::megawatts(1_000.0))),
        (t(0), Some(Power::megawatts(4_100.0))),
        (t(2), Some(Power::megawatts(5_000.0))),
    ];
    let capability = build_link_capability(&points, &b6_spec(), t(0), 2).unwrap();
    let gw: Vec<f64> = capability
        .forward
        .iter()
        .map(|p| p.as_gigawatts())
        .collect();
    assert_eq!(gw, vec![4.1, 4.1], "period 1 is missing → fill");
    assert_eq!(capability.observed, vec![true, false]);
}

#[test]
fn capability_assembly_rejects_off_grid_and_negative_rows() {
    let off_grid = vec![(
        UtcInstant::from_unix_micros(T0 + HALF_HOUR / 2),
        Some(Power::megawatts(4_000.0)),
    )];
    let err = build_link_capability(&off_grid, &b6_spec(), t(0), 2).unwrap_err();
    assert!(err.to_string().contains("half-hourly"), "err: {err}");

    let negative = vec![(t(0), Some(Power::megawatts(-100.0)))];
    let err = build_link_capability(&negative, &b6_spec(), t(0), 2).unwrap_err();
    assert!(err.to_string().contains("negative"), "err: {err}");
}

// ---------------------------------------------------------------------
// Schema v7 (D11): per-zone pricing inputs for the priced flow signal.
// ---------------------------------------------------------------------

/// Build a two-zone scenario with per-zone pricing: GB on the reference
/// UKA+CPS carbon basis, FR on a flat EUA level, both on the same
/// synthetic gas trace (the committed GB-SAP-fallback shape).
fn priced_two_zone_scenario(gas_path: &str, demand_path: &str, periods: usize) -> Scenario {
    let end_micros = T0 + (periods as i64 - 1) * HALF_HOUR;
    let end = grid_core::time::UtcInstant::from_unix_micros(end_micros);
    Scenario::from_toml_str(&format!(
        r#"
schema_version = 8
name = "priced-inputs-test"

[horizon]
start = "2024-01-01T00:00:00Z"
end = "{end}"
weather_years = [2024]

[[zones]]
id = "GB"

[zones.demand]
base_profile = "{demand_path}"
column = "load_mw"
annual_scale = 1.0

[zones.pricing]
reference = "data/reference/prices-2024.toml"

[zones.pricing.fuel_price.gas]
path = "{gas_path}"
column = "gas_price"

[zones.pricing.srmc.ccgt]
fuel = "gas"
efficiency = "ccgt"

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0

[[zones]]
id = "FR"

[zones.demand]
base_profile = "{demand_path}"
column = "load_mw"
annual_scale = 1.0

[zones.pricing]
reference = "data/reference/prices-2024.toml"
carbon_flat_gbp_per_tco2 = 55.01

[zones.pricing.fuel_price.gas]
path = "{gas_path}"
column = "gas_price"

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
"#
    ))
    .unwrap()
}

/// The per-zone SRMC chain loads: the same Stage 2 recipe per zone,
/// with the zone's declared carbon basis — GB the reference UKA+CPS
/// step series, FR the flat EUA level. The per-period difference
/// between the two zones' ccgt SRMCs must equal
/// (carbon_GB(t) − 55.01) × EF/η exactly.
#[test]
fn per_zone_pricing_inputs_load_with_the_declared_carbon_bases() {
    let periods = 4;
    // Fuel price is a price trace (£/MWh), not MW — but the parquet
    // shape is the same; the loader reads the named column.
    let gas = write_trace(
        "d11_gas.parquet",
        T0,
        &[("gas_price", &[30.0, 30.0, 31.0, 29.0])],
    );
    let demand = write_trace(
        "d11_demand.parquet",
        T0,
        &[("load_mw", &[10_000.0, 10_000.0, 10_000.0, 10_000.0])],
    );
    let scenario =
        priced_two_zone_scenario(gas.to_str().unwrap(), demand.to_str().unwrap(), periods);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let inputs = grid_adequacy::load_multi_zone_inputs(&scenario, &root).unwrap();

    let gb = inputs.zones[0].pricing.as_ref().expect("GB pricing loaded");
    let fr = inputs.zones[1].pricing.as_ref().expect("FR pricing loaded");
    let tech = grid_core::scenario::TechId::new("ccgt");
    let gb_srmc = &gb.srmc[&tech];
    let fr_srmc = &fr.srmc[&tech];
    assert_eq!(gb_srmc.len(), periods);
    assert_eq!(fr_srmc.len(), periods);

    // Recompute the expected wedge from the committed reference file.
    let reference = grid_core::prices_reference::PricesReference::load(
        &root.join("data/reference/prices-2024.toml"),
    )
    .unwrap();
    let eta = reference.efficiency_hhv["ccgt"].value();
    let ef = reference.ef_co2_thermal.as_tonnes_per_megawatt_hour();
    // First periods of 2024 sit before the first 2024 auction: the
    // step series forward-fills the FIRST auction's price (grid-core
    // convention), plus CPS.
    let first_auction = reference.uka_auctions[0]
        .clearing_price
        .as_pounds_per_tonne_co2();
    let cps = reference.cps.as_pounds_per_tonne_co2();
    let expected_wedge = (first_auction + cps - 55.01) * ef / eta;
    for t in 0..periods {
        let wedge = gb_srmc.values()[t].as_pounds_per_megawatt_hour()
            - fr_srmc.values()[t].as_pounds_per_megawatt_hour();
        assert!(
            (wedge - expected_wedge).abs() < 1e-9,
            "period {t}: wedge {wedge} vs expected {expected_wedge}"
        );
    }
}

/// A zone without a pricing block loads `pricing: None`; the scarcity
/// path never consults pricing (byte-untouched default).
#[test]
fn zones_without_pricing_blocks_load_none() {
    let demand = write_trace(
        "d11_demand_none.parquet",
        T0,
        &[("load_mw", &[10_000.0, 10_000.0])],
    );
    let scenario = scenario_with(
        2,
        &format!(
            r#"
[zones.demand]
base_profile = "{}"
column = "load_mw"
annual_scale = 1.0

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0
"#,
            demand.to_str().unwrap()
        ),
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let inputs = grid_adequacy::load_multi_zone_inputs(&scenario, &root).unwrap();
    assert!(inputs.zones[0].pricing.is_none());
}

/// An SRMC recipe naming a fuel outside prices-reference-v1's emission
/// factors ("gas" only) is a structured loading error, exactly as in
/// the single-zone pricing loader.
#[test]
fn zone_pricing_non_gas_fuel_is_a_structured_error() {
    let periods = 2;
    let gas = write_trace("d11_oil.parquet", T0, &[("oil_price", &[60.0, 60.0])]);
    let demand = write_trace(
        "d11_demand_oil.parquet",
        T0,
        &[("load_mw", &[10_000.0, 10_000.0])],
    );
    let toml = priced_two_zone_scenario(gas.to_str().unwrap(), demand.to_str().unwrap(), periods)
        .to_toml_string()
        .unwrap()
        .replace("fuel = \"gas\"", "fuel = \"oil\"")
        .replace(
            "[zones.pricing.fuel_price.gas]",
            "[zones.pricing.fuel_price.oil]",
        )
        .replace("column = \"gas_price\"", "column = \"oil_price\"");
    let scenario = Scenario::from_toml_str(&toml).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let err = grid_adequacy::load_multi_zone_inputs(&scenario, &root).unwrap_err();
    assert!(
        matches!(err, GridError::InvalidRunInputs { .. }),
        "unexpected: {err:?}"
    );
    assert!(err.to_string().contains("gas"), "err: {err}");
}
