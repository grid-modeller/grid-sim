//! Stage 1 acceptance tests — the honesty gate (docs/04 Stage 1).
//!
//! The 2024 reference scenario + the 2024 ERA5-derived CF traces + 2024
//! underlying demand must reproduce the observed 2024 system:
//!
//! 1. annual gas generation within ±5 % of 72.79 TWh (conditional on the
//!    harness handling the three correctable wedges — station load, PS
//!    round-trip loss, coal-closure windowing — which the self-contained
//!    schema-v2 `scenarios/gb-2024-reference.toml` does);
//! 2. net annual imports within ±1 % of the observed exogenous trace
//!    total, 33.30 TWh (a trace-plumbing check);
//! 3. monthly generation-mix correlation ≥ 0.99 (flattened 12×fuel
//!    absolute matrix, both sides in the D3 total-generation convention;
//!    fuel set documented at [`FUELS`]; threshold tightened from the
//!    original ≥ 0.95 after the first run —
//!    docs/notes/stage-1-2024-run-report.md §2/§4);
//! 4. zero unserved energy;
//! 5. determinism: two runs produce identical results and output hashes.
//!
//! Plus a characterisation test guarding the calibration numbers in the
//! scenario against the pack they were derived from, and the Stage 3
//! measurement that the (now active) storage portfolio does not act on
//! 2024 data.
//!
//! These tests need the locally built 2024 data pack (git-ignored;
//! fetched, not committed) and fail loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use grid_adequacy::{RunResult, load_run_inputs, run};
use grid_core::scenario::{AvailabilitySpec, Scenario};
use grid_core::units::Energy;

/// Observed 2024 annual gas generation, TWh (Elexon FUELHH: CCGT 72.62 +
/// OCGT 0.17; validation pack report §2).
const GAS_ACTUAL_TWH: f64 = 72.79;

/// Observed 2024 net imports, TWh (validation pack report §2).
const IMPORTS_ACTUAL_TWH: f64 = 33.30;

/// The monthly generation-mix fuel set, in the D3 total-generation
/// convention on both sides. Model side → actual side
/// (`monthly_generation_2024.csv` column):
///
/// - `ccgt` → `ccgt`, `ocgt` → `ocgt` (gas kept split as dispatched)
/// - `coal` → `coal`, `nuclear` → `nuclear`, `biomass` → `biomass`
/// - `hydro` → `npshyd`
/// - `offshore_wind` + `onshore_wind` → `wind_incl_embedded` (total wind,
///   transmission + embedded — the D3 convention)
/// - `solar` → `solar_embedded` (all GB solar is embedded)
///
/// Exogenous pass-through series (net imports, pumped-storage net,
/// FUELHH "other") are excluded: they are observed inputs on both sides,
/// so including them would inflate the correlation without testing the
/// model.
const FUELS: [(&str, &str); 8] = [
    ("ccgt", "ccgt"),
    ("ocgt", "ocgt"),
    ("coal", "coal"),
    ("nuclear", "nuclear"),
    ("biomass", "biomass"),
    ("hydro", "npshyd"),
    ("wind", "wind_incl_embedded"),
    ("solar", "solar_embedded"),
];

/// Workspace root (scenario and run-input paths are repo-relative).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Fail loudly if the 2024 data pack has not been built locally.
fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first: run \
         scripts/fetch-2024 (fetch.py, build.py) and scripts/era5-cf \
         (fetch_era5.py, derive_cf.py)",
        probe.display()
    );
}

/// Load the (self-contained, schema v2) scenario and run the 2024
/// reference dispatch.
fn run_2024() -> RunResult {
    require_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    run(&scenario, &inputs).unwrap()
}

fn twh(energy: Energy) -> f64 {
    energy.as_gigawatt_hours() / 1000.0
}

// ---------------------------------------------------------------------
// Acceptance test 1: annual gas within ±5 % of 72.79 TWh.
// ---------------------------------------------------------------------

#[test]
fn annual_gas_generation_within_5_percent_of_actual() {
    let result = run_2024();
    let gas =
        twh(result.thermal_energy("ccgt").unwrap()) + twh(result.thermal_energy("ocgt").unwrap());
    let error_percent = 100.0 * (gas - GAS_ACTUAL_TWH) / GAS_ACTUAL_TWH;
    assert!(
        error_percent.abs() <= 5.0,
        "modelled gas {gas:.2} TWh vs actual {GAS_ACTUAL_TWH} TWh: {error_percent:+.2} % (tolerance ±5 %)"
    );
}

// ---------------------------------------------------------------------
// Acceptance test 2: net annual imports within ±1 % of 33.30 TWh.
// ---------------------------------------------------------------------

#[test]
fn net_annual_imports_within_1_percent_of_observed() {
    let result = run_2024();
    let imports = twh(result.net_imports_energy());
    let error_percent = 100.0 * (imports - IMPORTS_ACTUAL_TWH) / IMPORTS_ACTUAL_TWH;
    assert!(
        error_percent.abs() <= 1.0,
        "modelled net imports {imports:.2} TWh vs observed {IMPORTS_ACTUAL_TWH} TWh: \
         {error_percent:+.2} % (tolerance ±1 %)"
    );
}

// ---------------------------------------------------------------------
// Acceptance test 3: monthly generation-mix correlation ≥ 0.99.
//
// docs/04 originally set ≥ 0.95 and anticipated tightening after the
// first Stage 1 run quantified model-side losses; the supervisor
// tightened it to ≥ 0.99 with the reviewer's evidence
// (docs/notes/stage-1-2024-run-report.md §2/§4): a zero-skill baseline
// holding every fuel flat at its observed annual mean scores r = 0.934
// on this flattened metric — so ≥ 0.95 barely excluded zero skill —
// while the achieved r = 0.997 rests on genuinely predicted content
// (the gas fleet's monthly shape, per-fuel r = 0.995; the
// nuclear/wind/solar monthly shapes are calibrated inputs, see the run
// report's circularity inventory).
// ---------------------------------------------------------------------

/// Read the pack's monthly actuals (GWh) for one fuel column.
fn actual_monthly_gwh(column: &str) -> Vec<f64> {
    let path = repo_root().join("data/packs/2024/processed/monthly_generation_2024.csv");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let idx = header
        .iter()
        .position(|c| *c == column)
        .unwrap_or_else(|| panic!("no column {column:?} in {}", path.display()));
    let values: Vec<f64> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(',').nth(idx).unwrap().parse().unwrap())
        .collect();
    assert_eq!(values.len(), 12, "column {column}");
    values
}

/// Modelled monthly energy (GWh) per comparison fuel, wind combined.
fn modelled_monthly_gwh(result: &RunResult) -> BTreeMap<&'static str, Vec<f64>> {
    let monthly = |series: &[grid_core::units::Power]| -> Vec<f64> {
        let by_month = result.monthly_energy(series);
        assert_eq!(by_month.len(), 12);
        by_month.values().map(|e| e.as_gigawatt_hours()).collect()
    };
    let mut out = BTreeMap::new();
    for tech in ["ccgt", "ocgt", "coal", "nuclear", "biomass", "hydro"] {
        let series = result
            .thermal
            .iter()
            .find(|s| s.tech.as_str() == tech)
            .unwrap_or_else(|| panic!("no thermal series {tech}"));
        out.insert(tech, monthly(&series.power));
    }
    let renewable = |tech: &str| -> Vec<f64> {
        let series = result
            .renewables
            .iter()
            .find(|s| s.tech.as_str() == tech)
            .unwrap_or_else(|| panic!("no renewable series {tech}"));
        monthly(&series.power)
    };
    let offshore = renewable("offshore_wind");
    let onshore = renewable("onshore_wind");
    out.insert(
        "wind",
        offshore.iter().zip(&onshore).map(|(a, b)| a + b).collect(),
    );
    out.insert("solar", renewable("solar"));
    out
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len());
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
    let sy: f64 = y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt();
    sxy / (sx * sy)
}

#[test]
fn monthly_generation_mix_correlation_at_least_0_99() {
    let result = run_2024();
    let model = modelled_monthly_gwh(&result);
    let mut xs = Vec::with_capacity(96);
    let mut ys = Vec::with_capacity(96);
    for (model_fuel, actual_column) in FUELS {
        xs.extend_from_slice(&model[model_fuel]);
        ys.extend_from_slice(&actual_monthly_gwh(actual_column));
    }
    assert_eq!(xs.len(), 96, "12 months × 8 fuels");
    let r = pearson(&xs, &ys);
    assert!(
        r >= 0.99,
        "flattened 12×{}-fuel monthly-mix correlation r = {r:.4} < 0.99 \
         (docs/04 threshold, tightened post-run; \
         docs/notes/stage-1-2024-run-report.md §4)",
        FUELS.len()
    );
}

/// The published monthly-mix correlation (r = 0.9970, report §1/§4) was
/// only GATED at ≥ 0.99 above; the CLAUDE.md rule wants the exact value
/// pinned so it cannot drift down to 0.991 with the gate still green.
/// This ADDS an exact pin (the ≥ 0.99 acceptance gate above is
/// untouched). Measured 2026-07-04; the engine is bit-deterministic
/// (ADR-5).
const PINNED_MONTHLY_MIX_CORRELATION: f64 = 0.9969534895747232;

#[test]
fn monthly_generation_mix_correlation_is_pinned_exactly() {
    let result = run_2024();
    let model = modelled_monthly_gwh(&result);
    let mut xs = Vec::with_capacity(96);
    let mut ys = Vec::with_capacity(96);
    for (model_fuel, actual_column) in FUELS {
        xs.extend_from_slice(&model[model_fuel]);
        ys.extend_from_slice(&actual_monthly_gwh(actual_column));
    }
    let r = pearson(&xs, &ys);
    assert!(
        (r - PINNED_MONTHLY_MIX_CORRELATION).abs() <= 1e-6,
        "monthly-mix correlation r = {r:.10} differs from the pinned \
         {PINNED_MONTHLY_MIX_CORRELATION} (±1e-6) — update this pin AND \
         docs/notes/stage-1-2024-run-report.md together"
    );
}

/// The load-bearing counterfactual (report §3): with the FUELHH 'other'
/// must-take wedge removed, the whole 3.35 TWh category lands on modelled
/// gas and the ±5 % gate FAILS. This pins both the DIRECTION (gas rises)
/// and the magnitude (76.65 TWh, +5.30 %) so the "gate fails without
/// 'other'" claim carries a live regression, not just a run-report note.
#[test]
fn removing_the_other_wedge_pushes_gas_above_the_five_percent_gate() {
    require_pack();
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let before = scenario.zones[0].exogenous_supply.len();
    scenario.zones[0]
        .exogenous_supply
        .retain(|e| e.label != "other");
    assert_eq!(
        scenario.zones[0].exogenous_supply.len(),
        before - 1,
        "the reference scenario must carry exactly one 'other' exogenous entry"
    );
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let result = run(&scenario, &inputs).unwrap();

    let gas =
        twh(result.thermal_energy("ccgt").unwrap()) + twh(result.thermal_energy("ocgt").unwrap());
    let error_percent = 100.0 * (gas - GAS_ACTUAL_TWH) / GAS_ACTUAL_TWH;

    // Magnitude: 76.65 TWh, +5.30 % (reviewer counterfactual, report §3).
    assert!(
        (gas - 76.65).abs() <= 0.1,
        "no-'other' gas {gas:.2} TWh differs from the pinned counterfactual \
         76.65 TWh (±0.1)"
    );
    // Direction: the gate genuinely FAILS (> +5 %) — this is the claim.
    assert!(
        error_percent > 5.0,
        "without the 'other' wedge the gas error {error_percent:+.2} % must \
         exceed the ±5 % gate (the wedge is load-bearing)"
    );
}

/// The published "tightest system moment": thermal margin ≈ 0.23 GW at
/// 2024-01-16 10:00Z (report §1). This was a reviewer diagnostic with no
/// output field. It is reconstructed here from the run's own dispatch +
/// the scenario's capacities/availabilities as the smallest unused
/// dispatchable-thermal headroom (Σ capacity·availability − dispatched)
/// over the year — the must-run plant contribute ~zero headroom, so at
/// the binding period this is essentially the spare CCGT/OCGT capacity.
/// Both the value (0.2317 GW) and the binding period (740 → 2024-01-16
/// 10:00Z) are pinned.
#[test]
fn tightest_thermal_margin_is_pinned_in_value_and_time() {
    use grid_core::time::UtcInstant;

    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let result = run_2024();
    let periods = result.demand.len();

    // Hours per 2024 (leap) month → per-period month index.
    let hours: [f64; 12] = [
        744.0, 696.0, 744.0, 720.0, 744.0, 720.0, 744.0, 744.0, 720.0, 744.0, 720.0, 744.0,
    ];
    let month_of = |t: usize| -> usize {
        let mut acc = 0.0;
        for (m, h) in hours.iter().enumerate() {
            acc += h * 2.0;
            if (t as f64) < acc {
                return m;
            }
        }
        11
    };

    let zone = &scenario.zones[0];
    let cap = |tech: &str| -> f64 {
        zone.fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap()
            .capacity_gw
            .as_gigawatts()
    };
    let avail = |tech: &str, t: usize| -> f64 {
        let e = zone
            .fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap();
        match &e.availability {
            Some(AvailabilitySpec::Monthly { monthly }) => monthly[month_of(t)].value(),
            Some(AvailabilitySpec::Flat { flat }) => flat.value(),
            None => 1.0,
        }
    };
    let dispatched = |tech: &str, t: usize| -> f64 {
        result
            .thermal
            .iter()
            .find(|s| s.tech.as_str() == tech)
            .unwrap()
            .power[t]
            .as_gigawatts()
    };
    let thermals = ["ccgt", "ocgt", "nuclear", "biomass", "hydro", "coal"];

    let mut min_margin = f64::INFINITY;
    let mut min_t = 0usize;
    for t in 0..periods {
        let margin: f64 = thermals
            .iter()
            .map(|tech| cap(tech) * avail(tech, t) - dispatched(tech, t))
            .sum();
        if margin < min_margin {
            min_margin = margin;
            min_t = t;
        }
    }

    // Value: 0.2317 GW (report's "≈ 0.23 GW").
    assert!(
        (min_margin - 0.23171957931638332).abs() <= 1e-4,
        "tightest thermal margin {min_margin:.5} GW differs from the pinned \
         0.23172 GW (±1e-4) — update this pin AND the run report §1 together"
    );
    // Time: the binding period is 740, which is 2024-01-16 10:00Z
    // (half-hourly from the 2024-01-01T00:00:00Z start).
    assert_eq!(min_t, 740, "tightest-margin period moved");
    assert_eq!(
        result.start,
        UtcInstant::parse("2024-01-01T00:00:00Z").unwrap()
    );
    let target = UtcInstant::parse("2024-01-16T10:00:00Z").unwrap();
    assert_eq!(
        result.start.periods_until_inclusive(target).unwrap(),
        741,
        "period 740 must be 2024-01-16 10:00Z (741 half-hours inclusive)"
    );
}

// ---------------------------------------------------------------------
// Acceptance test 4: zero unserved energy for 2024 (as in reality).
// ---------------------------------------------------------------------

#[test]
fn zero_unserved_energy_for_2024() {
    let result = run_2024();
    let unserved = result.total_unserved();
    assert_eq!(
        unserved,
        Energy::gigawatt_hours(0.0),
        "2024 had no demand-control events; modelled unserved = {} GWh",
        unserved.as_gigawatt_hours()
    );
}

// ---------------------------------------------------------------------
// Acceptance test 5: determinism — identical results across runs.
// ---------------------------------------------------------------------

#[test]
fn two_runs_produce_identical_results() {
    let first = run_2024();
    let second = run_2024();
    // Bit-identical, not approximately equal: the engine is a pure
    // function of (scenario, inputs) per ADR-5.
    assert!(first == second, "two runs of the same inputs differ");
}

// ---------------------------------------------------------------------
// Characterisation: the calibration numbers in the scenario (schema v2;
// formerly the run-inputs file) match the pack they were derived from
// (guards silent drift of either).
// ---------------------------------------------------------------------

#[test]
fn scenario_calibration_matches_the_pack() {
    require_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let zone = &scenario.zones[0];
    let availability_monthly = |tech: &str| -> Vec<f64> {
        match &zone
            .fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap_or_else(|| panic!("no fleet entry {tech}"))
            .availability
        {
            Some(AvailabilitySpec::Monthly { monthly }) => {
                monthly.iter().map(|f| f.value()).collect()
            }
            other => panic!("{tech} availability should be monthly, got {other:?}"),
        }
    };
    let availability_flat = |tech: &str| -> f64 {
        match &zone
            .fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap_or_else(|| panic!("no fleet entry {tech}"))
            .availability
        {
            Some(AvailabilitySpec::Flat { flat }) => flat.value(),
            other => panic!("{tech} availability should be flat, got {other:?}"),
        }
    };

    // Hours per calendar month, 2024 (leap year).
    let hours: [f64; 12] = [
        744.0, 696.0, 744.0, 720.0, 744.0, 720.0, 744.0, 744.0, 720.0, 744.0, 720.0, 744.0,
    ];

    let monthly = |column: &str| actual_monthly_gwh(column);

    // Nuclear: monthly energy / (5.9 GW × hours), 4 dp.
    let nuclear = monthly("nuclear");
    let pinned = availability_monthly("nuclear");
    for m in 0..12 {
        let derived = nuclear[m] / (5.9 * hours[m]);
        assert!(
            (derived - pinned[m]).abs() < 5e-5,
            "nuclear month {}: derived {derived:.5} vs pinned {:.5}",
            m + 1,
            pinned[m]
        );
    }

    // Biomass and hydro: flat annual factors.
    let year_hours: f64 = hours.iter().sum();
    let biomass = monthly("biomass").iter().sum::<f64>() / (3.5 * year_hours);
    assert!(
        (biomass - availability_flat("biomass")).abs() < 5e-5,
        "biomass flat: derived {biomass:.5}"
    );
    let hydro = monthly("npshyd").iter().sum::<f64>() / (1.9 * year_hours);
    assert!(
        (hydro - availability_flat("hydro")).abs() < 5e-5,
        "hydro flat: derived {hydro:.5}"
    );

    // Coal: flat inside Jan–Sep, zero Oct–Dec (closure 2024-09-30).
    let coal = monthly("coal");
    let window_hours: f64 = hours[..9].iter().sum();
    let coal_factor = coal[..9].iter().sum::<f64>() / (2.0 * window_hours);
    let pinned = availability_monthly("coal");
    for (m, factor) in pinned.iter().enumerate().take(9) {
        assert!(
            (factor - coal_factor).abs() < 5e-5,
            "coal month {}: pinned {factor:.5} vs derived window factor {coal_factor:.5}",
            m + 1
        );
    }
    for (m, factor) in pinned.iter().enumerate().skip(9) {
        assert_eq!(
            *factor,
            0.0,
            "coal is closed from October (month {})",
            m + 1
        );
    }

    // Station transformer load: the pinned constant matches the mean 2024
    // residual of FUELHH total (PS net) + net imports − ND (report §3),
    // recomputed here from the half-hourly pack.
    let periods = 17_568;
    let gen_path = root.join("data/packs/2024/processed/generation_by_fuel_2024.parquet");
    let mut residual_gw = vec![0.0f64; periods];
    let fuel_columns = [
        "biomass", "ccgt", "coal", "npshyd", "nuclear", "ocgt", "oil", "other", "ps", "wind",
        "intelec", "intew", "intfr", "intgrnl", "intifa2", "intirl", "intned", "intnem", "intnsl",
        "intvkl",
    ];
    for column in fuel_columns {
        let trace = grid_core::trace::load_power_trace_mw(&gen_path, column, periods).unwrap();
        for (acc, p) in residual_gw.iter_mut().zip(trace.values()) {
            *acc += p.as_gigawatts();
        }
    }
    let demand_path = root.join("data/packs/2024/processed/demand_2024.parquet");
    let nd = grid_core::trace::load_power_trace_mw(&demand_path, "nd", periods).unwrap();
    for (acc, p) in residual_gw.iter_mut().zip(nd.values()) {
        *acc -= p.as_gigawatts();
    }
    let mean_residual = residual_gw.iter().sum::<f64>() / periods as f64;
    let pinned = zone.demand.extra_demand_gw.as_gigawatts();
    assert!(
        (mean_residual - pinned).abs() < 0.003,
        "station load: derived mean residual {mean_residual:.4} GW vs pinned {pinned} GW"
    );
}

/// Stage 3 measurement (double-counting tension, documented in the
/// scenario): the reference scenario carries BOTH the observed exogenous
/// pumped-storage trace and active stores. Under D4 the active stores
/// must never act on 2024 data — initially full (D4 default), never a
/// post-stack deficit (no discharge), never headroom (no charge) — so
/// the Stage 1/2 physical numbers stay untouched and the validation
/// stays honest. This test IS the measured store activity.
#[test]
fn active_storage_does_not_act_on_the_2024_reference_run() {
    let result = run_2024();
    assert_eq!(result.stores.len(), 2);
    for store in &result.stores {
        let charged: f64 = store.charge.iter().map(|p| p.as_gigawatts()).sum();
        let discharged: f64 = store.discharge.iter().map(|p| p.as_gigawatts()).sum();
        assert_eq!(charged, 0.0, "{} charged {charged} GW-periods", store.label);
        assert_eq!(
            discharged, 0.0,
            "{} discharged {discharged} GW-periods",
            store.label
        );
        let (_, min_soc) = store.min_soc().unwrap();
        assert_eq!(
            min_soc,
            store.max_soc().unwrap(),
            "{}: SoC moved despite zero flows",
            store.label
        );
    }

    // Consequently the physical series are identical with the stores
    // removed — the Stage 1/2 gate numbers cannot have moved.
    require_pack();
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    scenario.zones[0].storage.clear();
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let without_storage = run(&scenario, &inputs).unwrap();
    let with_storage = run_2024();
    assert!(with_storage.thermal == without_storage.thermal);
    assert!(with_storage.curtailment == without_storage.curtailment);
    assert!(with_storage.unserved == without_storage.unserved);
}

/// Characterisation pin for the D4 rule-1 erratum (2026-07-06;
/// comment-consistency sweep M2, `docs/notes/comment-consistency-sweep.md`;
/// erratum in `docs/notes/d4-rule-based-dispatch.md`): the engine has NO
/// must-run category. Nuclear sits at the bottom of the merit-order stack
/// and is BACKED DOWN whenever the post-must-take residual demand is below
/// its available ceiling (capacity × availability) — including surplus
/// periods, where no thermal plant runs at all. Versus a must-run
/// treatment this supplies LESS in deep-surplus periods, so the engine
/// slightly UNDERSTATES curtailment and storage charging
/// (anti-conservative for curtailment findings; bounded small on the 2024
/// fleet, growing with nuclear share).
///
/// This is a CHARACTERISATION pin of existing, ruled-canonical behaviour
/// (the validated instrument every committed digest was measured with) —
/// not red-green: it was written against the current engine and passes
/// immediately, so the divergence can never silently change. Values are
/// exact under bit-determinism (ADR-5).
#[test]
fn nuclear_backdown_periods_on_the_2024_reference() {
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let result = run_2024();
    let periods = result.periods();
    assert_eq!(periods, 17_568);

    // Nuclear's available ceiling per period: capacity × monthly
    // availability (2024 leap-year month lengths).
    let hours: [f64; 12] = [
        744.0, 696.0, 744.0, 720.0, 744.0, 720.0, 744.0, 744.0, 720.0, 744.0, 720.0, 744.0,
    ];
    let month_of = |t: usize| -> usize {
        let mut acc = 0.0;
        for (m, h) in hours.iter().enumerate() {
            acc += h * 2.0;
            if (t as f64) < acc {
                return m;
            }
        }
        11
    };
    let entry = scenario.zones[0]
        .fleet
        .iter()
        .find(|e| e.technology.as_str() == "nuclear")
        .unwrap();
    let capacity = entry.capacity_gw.as_gigawatts();
    let monthly: Vec<f64> = match &entry.availability {
        Some(AvailabilitySpec::Monthly { monthly }) => monthly.iter().map(|f| f.value()).collect(),
        other => panic!("nuclear availability should be monthly, got {other:?}"),
    };
    let nuclear = &result
        .thermal
        .iter()
        .find(|s| s.tech.as_str() == "nuclear")
        .unwrap()
        .power;

    let mut backdown_periods = 0usize;
    let mut max_gap_gw = 0.0f64;
    for t in 0..periods {
        let ceiling = capacity * monthly[month_of(t)];
        let gap = ceiling - nuclear[t].as_gigawatts();
        if gap > 1e-9 {
            backdown_periods += 1;
            max_gap_gw = max_gap_gw.max(gap);
        }
    }

    // The pinned divergence bound on the 2024 reference (measured
    // 2026-07-06): 116 of 17,568 periods; max back-down 5.14539 GW —
    // the full ceiling (5.9 GW × 0.8721 availability) of a period in
    // which nuclear runs at zero, i.e. a surplus period where no
    // thermal plant runs at all.
    assert_eq!(
        backdown_periods, 116,
        "nuclear back-down period count moved — the D4 rule-1 erratum \
         bound (comment-consistency sweep M2) must be re-measured and the \
         erratum + manual register row updated together"
    );
    assert!(
        (max_gap_gw - 5.14539).abs() <= 1e-9,
        "max nuclear back-down {max_gap_gw:.17} GW differs from the pinned \
         5.14539 GW (= 5.9 GW × 0.8721)"
    );

    // The bias bound: total 2024-reference curtailment is tiny
    // (0.1367 GWh), so the understated-curtailment direction is bounded
    // small at the 2024 fleet.
    let curtailment_gwh = result.total_curtailment().as_gigawatt_hours();
    assert!(
        (curtailment_gwh - 0.1367).abs() <= 1e-4,
        "2024-reference total curtailment {curtailment_gwh:.6} GWh differs \
         from the pinned 0.1367 GWh"
    );
}

/// Trace-alignment guard: the demand trace really is the D3 underlying-
/// demand column (annual 261.83 TWh, report), not ND — a mixed-convention
/// run would silently understate gas by the embedded wedge.
#[test]
fn demand_input_is_underlying_demand_in_the_d3_convention() {
    let result = run_2024();
    let demand_twh = twh(result.total_demand_energy());
    // 261.83 (underlying demand) + 5.86 (station load adjustment).
    let expected = 261.83 + 5.86;
    assert!(
        (demand_twh - expected).abs() < 0.5,
        "adjusted demand {demand_twh:.2} TWh; expected ≈ {expected:.2} TWh"
    );
}

/// Reliability accounting on the reference run (gb-grid-margin
/// methodology): the classification matches the published roster —
/// derived, no overrides — the pumped-storage trace is excluded, and
/// the four categories partition total supply exactly, every period.
#[test]
fn reference_run_reliability_partition_and_roster() {
    use grid_core::scenario::{ExogenousReliability, Reliability};

    let result = run_2024();
    // Roster: every thermal series firm, every weather series variable,
    // none overridden (the reference fleet carries no explicit fields).
    for series in &result.thermal {
        assert_eq!(series.reliability, Reliability::Firm, "{}", series.tech);
        assert!(!series.reliability_overridden, "{}", series.tech);
    }
    for series in &result.renewables {
        assert_eq!(series.reliability, Reliability::Variable, "{}", series.tech);
        assert!(!series.reliability_overridden, "{}", series.tech);
    }
    // Exogenous: imports variable, "other" firm, PS excluded.
    let exo = |label: &str| {
        result
            .exogenous
            .iter()
            .find(|s| s.label == label)
            .unwrap()
            .reliability
    };
    assert_eq!(exo("net_imports"), ExogenousReliability::Variable);
    assert_eq!(exo("other"), ExogenousReliability::Firm);
    assert_eq!(exo("pumped_storage_net"), ExogenousReliability::Excluded);

    // Partition: firm + variable + storage discharge + excluded ==
    // total supply, every one of the 17,568 periods.
    let firm = result.firm_supply();
    let variable = result.variable_supply();
    let storage = result.storage_discharge();
    for t in 0..result.periods() {
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
            (parts - total).abs() <= 1e-9 * total.abs().max(1.0),
            "period {t}: partition {parts} != total supply {total}"
        );
    }
}
