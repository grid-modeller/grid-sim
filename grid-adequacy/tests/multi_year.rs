//! Multi-year continuous horizons (docs/04 Stage 3): per-year Parquet
//! trace files assemble into one horizon, and store SoC carries across
//! the year boundary with no annual reset — end-to-end through the
//! loader and engine, against synthetic full-length year files, plus
//! (Stage 3 part 2) mixed leap/non-leap year-length handling, the full
//! synthetic 1985–2024 horizon (~700k periods) and the real per-year
//! data pack (required — a missing pack fails loudly, no silent skip).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::Arc;

use arrow_array::builder::{Float64Builder, TimestampMicrosecondBuilder};
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use grid_adequacy::{load_run_inputs, run};
use grid_core::scenario::Scenario;
use grid_core::time::UtcInstant;
use grid_core::units::{Duration, Energy};

/// Half-hourly periods in 2023 (non-leap) and 2024 (leap).
const PERIODS_2023: usize = 17_520;
const PERIODS_2024: usize = 17_568;

/// Half-hourly periods in calendar `year` (proleptic Gregorian).
fn periods_in_year(year: i32) -> usize {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    if leap { 17_568 } else { 17_520 }
}

/// Write a full synthetic year of half-hourly data: one MW `demand`
/// column and one `cf` column, values chosen per period by closures.
fn write_year(
    dir: &std::path::Path,
    name: &str,
    start: &str,
    periods: usize,
    demand_mw: impl Fn(usize) -> f64,
    cf: impl Fn(usize) -> f64,
) -> PathBuf {
    let start = UtcInstant::parse(start).unwrap();
    let ts_type = DataType::Timestamp(TimeUnit::Microsecond, Some(Arc::from("UTC")));
    let schema = Arc::new(Schema::new(vec![
        Field::new("utc_start", ts_type, false),
        Field::new("demand", DataType::Float64, false),
        Field::new("cf", DataType::Float64, false),
    ]));
    let mut stamps = TimestampMicrosecondBuilder::new();
    let mut demand_col = Float64Builder::new();
    let mut cf_col = Float64Builder::new();
    for t in 0..periods {
        stamps.append_value(start.plus_periods(t as i64).unix_micros());
        demand_col.append_value(demand_mw(t));
        cf_col.append_value(cf(t));
    }
    let arrays: Vec<ArrayRef> = vec![
        Arc::new(stamps.finish().with_timezone("UTC")),
        Arc::new(demand_col.finish()),
        Arc::new(cf_col.finish()),
    ];
    let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();
    let path = dir.join(name);
    let file = std::fs::File::create(&path).unwrap();
    let mut writer = parquet::arrow::ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    path
}

/// Two synthetic years, one file each: every period of year 1 (2023) is
/// a 2 GW surplus; every period of year 2 (2024) is a 1 GW deficit. A
/// store that starts EMPTY can only serve year 2 with energy banked in
/// year 1 and carried across the boundary — an annual SoC reset (the
/// classic "few days of storage" error, D4 mechanics) would leave year
/// 2 unserved from its first period.
#[test]
fn per_year_files_assemble_and_soc_carries_across_the_boundary() {
    let dir = std::env::temp_dir().join("grid-adequacy-multi-year-tests");
    std::fs::create_dir_all(&dir).unwrap();

    // Wind 10 GW nameplate. Year 1: demand 8 GW, cf 1.0 → +2 GW surplus.
    // Year 2: demand 8 GW, cf 0.7 → −1 GW deficit.
    let y2023 = write_year(
        &dir,
        "y2023.parquet",
        "2023-01-01T00:00:00Z",
        PERIODS_2023,
        |_| 8_000.0,
        |_| 1.0,
    );
    let y2024 = write_year(
        &dir,
        "y2024.parquet",
        "2024-01-01T00:00:00Z",
        PERIODS_2024,
        |_| 8_000.0,
        |_| 0.7,
    );

    let scenario = Scenario::from_toml_str(&format!(
        r#"
schema_version = 8
name = "multi-year-synthetic"

[horizon]
start = "2023-01-01T00:00:00Z"
end = "2024-12-31T23:30:00Z"
weather_years = [2023, 2024]

[[zones]]
id = "GB"

[zones.demand]
base_profile = ["{y1}", "{y2}"]
column = "demand"
annual_scale = 1.0

[[zones.fleet]]
technology = "onshore_wind"
capacity_gw = 10.0
capacity_factor_trace = ["{y1}", "{y2}"]

[[zones.storage]]
kind = "hydrogen"
power_gw = 5.0
energy_gwh = 20000.0
round_trip_efficiency = 1.0
dispatch_order = 1
initial_soc = 0.0

[dispatch]
policy = "rule_based"
"#,
        y1 = y2023.to_str().unwrap(),
        y2 = y2024.to_str().unwrap(),
    ))
    .unwrap();

    assert_eq!(
        scenario.horizon.period_count().unwrap(),
        PERIODS_2023 + PERIODS_2024
    );
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    assert_eq!(inputs.demand.len(), PERIODS_2023 + PERIODS_2024);

    let result = run(&scenario, &inputs).unwrap();
    assert_eq!(result.periods(), PERIODS_2023 + PERIODS_2024);

    // Year 1 banks 2 GW × 8760 h = 17,520 GWh (η = 1).
    let store = &result.stores[0];
    let end_of_year_1 = store.soc[PERIODS_2023 - 1];
    assert!(
        (end_of_year_1.as_gigawatt_hours() - 17_520.0).abs() < 1e-6,
        "end-of-2023 SoC {end_of_year_1:?}"
    );
    // First period of year 2 discharges FROM THE CARRIED SoC — no reset.
    let first_2024 = store.soc[PERIODS_2023];
    let expected = end_of_year_1 - Duration::half_hour() * grid_core::units::Power::gigawatts(1.0);
    assert!(
        (first_2024.as_gigawatt_hours() - expected.as_gigawatt_hours()).abs() < 1e-9,
        "first-2024 SoC {first_2024:?}, expected {expected:?}"
    );
    // Year 2's 1 GW × 8784 h = 8,784 GWh deficit is fully served from
    // the bank: zero unserved across the whole two-year horizon.
    assert_eq!(result.total_unserved(), Energy::gigawatt_hours(0.0));
    let final_soc = store.soc.last().unwrap();
    assert!(
        (final_soc.as_gigawatt_hours() - (17_520.0 - 8_784.0)).abs() < 1e-6,
        "final SoC {final_soc:?}"
    );
}

/// Three consecutive years spanning a leap year — 17,520 / 17,568 /
/// 17,520 periods — with exact 30-minute continuity across BOTH file
/// boundaries. Every quantity is binary-exact (0.75, 0.5-hour periods),
/// so the assertions are equalities up to accumulation dust:
///
/// - 2023 (cf 1.0): +2 GW surplus × 8,760 h banks 17,520 GWh (η = 1);
/// - 2024 leap (cf 0.75): −0.5 GW deficit × 8,784 h draws 4,392 GWh;
/// - 2025 (cf 0.75): −0.5 GW deficit × 8,760 h draws 4,380 GWh;
/// - final SoC = 17,520 − 8,772 = 8,748 GWh. A 365-day-year assumption
///   anywhere in the chain would land on 8,760 GWh instead — the leap
///   year is load-bearing.
#[test]
fn three_years_spanning_a_leap_year_keep_exact_continuity() {
    let dir = std::env::temp_dir().join("grid-adequacy-multi-year-tests-leap3");
    std::fs::create_dir_all(&dir).unwrap();

    let years: Vec<PathBuf> = [(2023, 1.0), (2024, 0.75), (2025, 0.75)]
        .into_iter()
        .map(|(year, cf)| {
            write_year(
                &dir,
                &format!("y{year}.parquet"),
                &format!("{year}-01-01T00:00:00Z"),
                periods_in_year(year),
                |_| 8_000.0,
                move |_| cf,
            )
        })
        .collect();
    let files = format!(
        "[\"{}\", \"{}\", \"{}\"]",
        years[0].display(),
        years[1].display(),
        years[2].display()
    );

    let scenario = Scenario::from_toml_str(&format!(
        r#"
schema_version = 8
name = "three-year-leap-synthetic"

[horizon]
start = "2023-01-01T00:00:00Z"
end = "2025-12-31T23:30:00Z"
weather_years = [2023, 2024, 2025]

[[zones]]
id = "GB"

[zones.demand]
base_profile = {files}
column = "demand"
annual_scale = 1.0

[[zones.fleet]]
technology = "onshore_wind"
capacity_gw = 10.0
capacity_factor_trace = {files}

[[zones.storage]]
kind = "hydrogen"
power_gw = 5.0
energy_gwh = 20000.0
round_trip_efficiency = 1.0
dispatch_order = 1
initial_soc = 0.0

[dispatch]
policy = "rule_based"
"#,
    ))
    .unwrap();

    let total = PERIODS_2023 + PERIODS_2024 + 17_520;
    assert_eq!(scenario.horizon.period_count().unwrap(), total);
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    assert_eq!(inputs.demand.len(), total);
    // Continuity: the loader stitched three files into one uniform
    // half-hourly index (spot-check both boundaries).
    assert_eq!(
        inputs.demand.timestamp_at(PERIODS_2023).unwrap(),
        UtcInstant::parse("2024-01-01T00:00:00Z").unwrap()
    );
    assert_eq!(
        inputs
            .demand
            .timestamp_at(PERIODS_2023 + PERIODS_2024)
            .unwrap(),
        UtcInstant::parse("2025-01-01T00:00:00Z").unwrap()
    );

    let result = run(&scenario, &inputs).unwrap();
    assert_eq!(result.periods(), total);
    assert_eq!(result.total_unserved(), Energy::gigawatt_hours(0.0));

    let store = &result.stores[0];
    let gwh = |index: usize| store.soc[index].as_gigawatt_hours();
    // End of 2023: the full 17,520 GWh bank.
    assert!((gwh(PERIODS_2023 - 1) - 17_520.0).abs() < 1e-6);
    // SoC carries across BOTH boundaries: the first period of each
    // deficit year draws exactly 0.5 GW × 0.5 h = 0.25 GWh from the
    // carried SoC (an annual reset would jump instead).
    for boundary in [PERIODS_2023, PERIODS_2023 + PERIODS_2024] {
        let delta = gwh(boundary) - gwh(boundary - 1);
        assert!(
            (delta + 0.25).abs() < 1e-9,
            "SoC step across the year boundary at period {boundary} was {delta} GWh, \
             expected exactly -0.25 GWh (no reset)"
        );
    }
    // Final SoC: 17,520 − (4,392 + 4,380) = 8,748 GWh — leap-exact.
    assert!(
        (gwh(total - 1) - 8_748.0).abs() < 1e-6,
        "final SoC {} GWh; 8,760 GWh here means a 365-day-year assumption",
        gwh(total - 1)
    );
}

/// The full 40-year horizon shape (1985–2024, 701,280 periods) through
/// the loader and engine on synthetic per-year files — every year: cf
/// 1.0 for the first half (surplus +2 GW), 0.75 for the second half
/// (deficit −0.5 GW) against flat 8 GW demand and 10 GW wind, with a
/// 10,000 GWh store (5 GW, η = 1) starting empty. Hand-checkable
/// steady state:
///
/// - each first half banks 8,760 GWh (8,784 leap), each second half
///   draws 2,190 GWh (2,196 leap) — so unserved is zero everywhere and
///   the store overflows (curtailment) from year 2 on;
/// - year-end SoC: 6,570 GWh after 1985 (empty start), then
///   10,000 − 2,190 = 7,810 GWh every non-leap year and
///   10,000 − 2,196 = 7,804 GWh every leap year;
/// - every year boundary continues the SoC trajectory: the first period
///   of each year charges +2 GW × 0.5 h = +1 GWh, never resets.
///
/// Also the Stage 4 performance reference point: the test prints load
/// and dispatch wall times (docs/06 target: 40-year run < 1 s
/// single-threaded — measured and reported, not optimised, in Stage 3).
#[test]
fn synthetic_forty_year_horizon_dispatches_continuously() {
    let dir = std::env::temp_dir().join("grid-adequacy-multi-year-tests-40y");
    std::fs::create_dir_all(&dir).unwrap();

    let years: Vec<i32> = (1985..=2024).collect();
    let mut paths: Vec<String> = Vec::new();
    for &year in &years {
        let periods = periods_in_year(year);
        let half = periods / 2;
        let path = write_year(
            &dir,
            &format!("y{year}.parquet"),
            &format!("{year}-01-01T00:00:00Z"),
            periods,
            |_| 8_000.0,
            move |t| if t < half { 1.0 } else { 0.75 },
        );
        paths.push(path.display().to_string());
    }
    let files = format!(
        "[{}]",
        paths
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let scenario = Scenario::from_toml_str(&format!(
        r#"
schema_version = 8
name = "forty-year-synthetic"

[horizon]
start = "1985-01-01T00:00:00Z"
end = "2024-12-31T23:30:00Z"
weather_years = "all"

[[zones]]
id = "GB"

[zones.demand]
base_profile = {files}
column = "demand"
annual_scale = 1.0

[[zones.fleet]]
technology = "onshore_wind"
capacity_gw = 10.0
capacity_factor_trace = {files}

[[zones.storage]]
kind = "hydrogen"
power_gw = 5.0
energy_gwh = 10000.0
round_trip_efficiency = 1.0
dispatch_order = 1
initial_soc = 0.0

[dispatch]
policy = "rule_based"
"#,
    ))
    .unwrap();

    const FULL_RECORD_PERIODS: usize = 701_280;
    assert_eq!(
        scenario.horizon.period_count().unwrap(),
        FULL_RECORD_PERIODS
    );

    let load_started = std::time::Instant::now();
    let inputs = load_run_inputs(&scenario, "/".as_ref()).unwrap();
    let load_elapsed = load_started.elapsed();
    assert_eq!(inputs.demand.len(), FULL_RECORD_PERIODS);

    let run_started = std::time::Instant::now();
    let result = run(&scenario, &inputs).unwrap();
    let run_elapsed = run_started.elapsed();
    eprintln!(
        "40-year synthetic horizon: load {load_elapsed:?}, dispatch {run_elapsed:?} \
         (docs/06 target < 1 s; Stage 4 owns optimisation)"
    );

    assert_eq!(result.periods(), FULL_RECORD_PERIODS);
    assert_eq!(result.total_unserved(), Energy::gigawatt_hours(0.0));
    assert!(
        result.total_curtailment().as_gigawatt_hours() > 0.0,
        "the 10,000 GWh store must overflow from year 2 on"
    );

    let store = &result.stores[0];
    let gwh = |index: usize| store.soc[index].as_gigawatt_hours();
    let mut boundary = 0usize;
    for (offset, &year) in years.iter().enumerate() {
        let periods = periods_in_year(year);
        // Year-end SoC (hand-computed above).
        let expected_year_end = if offset == 0 {
            6_570.0
        } else if periods == 17_568 {
            7_804.0
        } else {
            7_810.0
        };
        let year_end = boundary + periods - 1;
        assert!(
            (gwh(year_end) - expected_year_end).abs() < 1e-6,
            "end-of-{year} SoC {} GWh, expected {expected_year_end}",
            gwh(year_end)
        );
        // Boundary continuity: the first period of every year after the
        // first charges +1 GWh from the CARRIED SoC.
        if offset > 0 {
            let delta = gwh(boundary) - gwh(boundary - 1);
            assert!(
                (delta - 1.0).abs() < 1e-9,
                "SoC step into {year} was {delta} GWh, expected exactly +1 GWh (no reset)"
            );
        }
        boundary += periods;
    }
}

// ---------------------------------------------------------------------
// The real 1985–2024 record (Stage 3 part 2 data pack), through the
// Royal-Society-style scenario's trace lists. The per-year files are
// fetched/derived, not committed; a missing pack FAILS LOUDLY (the
// require-packs posture, same as the Stage 3 part 2 acceptance tests,
// acceptance_stage3_rs37y.rs — no silent skip).
// ---------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn real_forty_year_record_loads_continuously() {
    let root = repo_root();
    let scenario_path = root.join("scenarios/royal-society-37y.toml");
    let scenario = Scenario::load(&scenario_path).unwrap();

    // The scenario is a committed artefact; its DATA is fetched. Fail
    // loudly when any referenced trace file is absent (a cargo-captured
    // eprintln-skip is invisible in a normal run — the require-packs
    // pattern instead, matching acceptance_stage3_rs37y.rs).
    let zone = &scenario.zones[0];
    let mut referenced: Vec<String> = zone.demand.base_profile.paths().to_vec();
    for entry in &zone.fleet {
        if let Some(trace) = &entry.capacity_factor_trace {
            referenced.extend(trace.paths().iter().cloned());
        }
    }
    if let Some(missing) = referenced.iter().find(|p| !root.join(p).exists()) {
        panic!(
            "{missing} is missing — build the per-year 1985–2024 pack (data/packs/cf, \
             data/packs/demand-tiled) before running the multi-year real-record test"
        );
    }

    const FULL_RECORD_PERIODS: usize = 701_280;
    assert_eq!(
        scenario.horizon.period_count().unwrap(),
        FULL_RECORD_PERIODS
    );
    // load_run_inputs validates, per trace list: per-file UTC half-hourly
    // uniformity, exact 30-minute continuity at every file boundary
    // (TraceNotConsecutive otherwise), the total period count and the
    // horizon-start alignment — this is the whole-record continuity test.
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    assert_eq!(inputs.demand.len(), FULL_RECORD_PERIODS);
    assert_eq!(
        inputs.demand.start(),
        UtcInstant::parse("1985-01-01T00:00:00Z").unwrap()
    );
    assert_eq!(
        inputs.capacity_factors.len(),
        3,
        "onshore + offshore + solar"
    );
    for (tech, trace) in &inputs.capacity_factors {
        assert_eq!(trace.len(), FULL_RECORD_PERIODS, "{tech} CF trace");
    }
}
