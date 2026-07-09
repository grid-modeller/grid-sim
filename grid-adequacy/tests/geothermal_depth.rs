//! D16 geothermal depth-continuum acceptance tests
//! (docs/notes/d16-geothermal-source-temperature.md, rule 4 — the five
//! red-green engine-package tests). Supersedes the exploratory
//! `geothermal_depth_probe.rs` spike (2026-07-06, uncommitted).
//!
//! The load-bearing safety property (D16 rule 4 test 1, the D9 rule-5
//! invariance discipline): with `resource_depth_m` ABSENT or at the
//! shallow reference default (1.0 m, the committed `loop_depth_m`
//! datum), results are BIT-IDENTICAL to the committed D9 behaviour —
//! the three committed pins in `tests/heating.rs` stay unmoved. The
//! gradient is anchored at the committed shallow datum
//! (`T_mean(z) = T_surface_mean + G·(z − loop_depth)`), which is what
//! makes the explicit-1 m case exactly, not approximately, invariant.
//!
//! These tests need the fetched data packs (per-year 1985–2024 pack;
//! the pinned GB t2m trace). They FAIL LOUDLY if any is missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{RunInputs, SolveOptions, load_run_inputs, min_storage_for_zero_unserved, run};
use grid_core::analysis::residual_load;
use grid_core::heating::{HEATING_COP_REFERENCE_PATH, HeatingCopReference, implied_spfh2};
use grid_core::scenario::{HeatingEntry, HeatingKind, HeatingSpec, Scenario, TraceRefSpec};
use grid_core::trace::load_temperature_trace_c;
use grid_core::units::{Energy, Length, PerUnit, Power};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn require(rel: &str) {
    let path = repo_root().join(rel);
    assert!(
        path.exists(),
        "required data is missing ({}) — build the pack first (scripts/fetch-2024, \
         scripts/era5-cf; the t2m trace: derive_t2m_gb.py)",
        path.display()
    );
}

fn require_packs() {
    require("data/weather/gb_t2m_pop.parquet");
    for year in 1985..=2024 {
        require(&format!("data/packs/demand-tiled/demand_{year}.parquet"));
        require(&format!("data/packs/cf/gb_offshore_cf_{year}.parquet"));
    }
}

/// The D9 reference heating block (quantum 410.5 TWh record-mean,
/// electrified share 0.5, DHW 0.170) as a single-technology portfolio,
/// with an optional GSHP resource depth.
fn block(kind: HeatingKind, resource_depth_m: Option<f64>) -> HeatingSpec {
    HeatingSpec {
        delivered_heat_twh: Energy::gigawatt_hours(410_500.0),
        electrified_share: PerUnit::new(0.5),
        dhw_fraction: PerUnit::new(0.170),
        temperature_trace: TraceRefSpec {
            path: "data/weather/gb_t2m_pop.parquet".to_owned(),
            column: "t2m_pop".to_owned(),
        },
        entries: vec![HeatingEntry {
            kind,
            share: PerUnit::new(1.0),
            cop_curve: None,
            correction_factor: None,
            rhpp_derating: None,
            cop_const: None,
            resource_depth_m: resource_depth_m.map(Length::metres),
        }],
    }
}

/// RS-37y inputs with an all-`kind` heating block at an optional
/// resource depth.
fn heated_inputs(kind: HeatingKind, resource_depth_m: Option<f64>) -> (Scenario, RunInputs) {
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join("scenarios/royal-society-37y.toml")).unwrap();
    scenario.zones[0].demand.heating = Some(block(kind, resource_depth_m));
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    (scenario, inputs)
}

/// Peak residual demand of a run, GW (max over periods of
/// `demand − Σ must-take`; this fleet has no exogenous supply).
fn peak_residual(result: &grid_adequacy::RunResult) -> f64 {
    let must_take: Vec<&[Power]> = result
        .renewables
        .iter()
        .map(|s| s.power.as_slice())
        .chain(result.exogenous.iter().map(|s| s.power.as_slice()))
        .collect();
    residual_load(&result.demand, &must_take)
        .unwrap()
        .iter()
        .map(|p| p.as_gigawatts())
        .fold(f64::NEG_INFINITY, f64::max)
}

// ---------------------------------------------------------------------
// Test 1 — invariance at the default (THE SAFETY PIN).
// ---------------------------------------------------------------------

/// D16 rule 4 test 1: `resource_depth_m` absent and `resource_depth_m
/// = 1.0` (the committed shallow `loop_depth_m` datum) produce
/// BIT-IDENTICAL demand — every half-hour, exact equality, no
/// tolerance. Everything downstream (dispatch, storage solves, the
/// committed pins) is a pure function of this demand (ADR-5), so this
/// is the strongest form of the invariance property. The committed
/// numeric pins themselves stay guarded in `tests/heating.rs`.
#[test]
fn resource_depth_at_the_shallow_default_is_bit_identical_to_absent() {
    require_packs();
    let (_, absent) = heated_inputs(HeatingKind::Gshp, None);
    let (_, at_datum) = heated_inputs(HeatingKind::Gshp, Some(1.0));

    assert_eq!(absent.demand.len(), at_datum.demand.len());
    for t in 0..absent.demand.len() {
        assert_eq!(
            absent.demand.values()[t],
            at_datum.demand.values()[t],
            "period {t}: the shallow-default depth path must be bit-identical to the \
             committed no-depth path (D16 rule 4 test 1 — the safety pin)"
        );
    }

    // The overlay series themselves are bit-identical too (the demand
    // equality above could in principle hide compensating errors).
    let absent_overlay = absent.heating.as_ref().unwrap();
    let datum_overlay = at_datum.heating.as_ref().unwrap();
    assert_eq!(
        absent_overlay.electrical_total, datum_overlay.electrical_total,
        "overlay electrical series must be bit-identical at the shallow default"
    );
    assert_eq!(absent_overlay.delivered_heat, datum_overlay.delivered_heat);
}

// ---------------------------------------------------------------------
// Test 2 — monotonicity (property).
// ---------------------------------------------------------------------

/// D16 rule 4 test 2: a deeper resource is a warmer winter source, so
/// the all-GSHP peak residual falls WEAKLY with depth (flat stretches
/// are legal — the ΔT floor and the direct-use regime both produce
/// them; a RISE is red). Depths span shallow loop → warm aquifer →
/// direct use.
#[test]
fn deeper_resource_weakly_lowers_the_all_gshp_peak_residual() {
    require_packs();
    let depths = [1.0, 15.0, 50.0, 100.0, 200.0, 400.0, 800.0, 1600.0, 3200.0];
    let mut previous: Option<(f64, f64)> = None;
    for &depth in &depths {
        let (scenario, inputs) = heated_inputs(HeatingKind::Gshp, Some(depth));
        let peak = peak_residual(&run(&scenario, &inputs).unwrap());
        if let Some((prev_depth, prev_peak)) = previous {
            assert!(
                peak <= prev_peak + 1e-9,
                "peak residual ROSE with depth: {prev_peak} GW at {prev_depth} m → \
                 {peak} GW at {depth} m (D16 rule 4 test 2)"
            );
        }
        previous = Some((depth, peak));
    }
}

// ---------------------------------------------------------------------
// Test 3 — the direct-use limit (the two endpoints meet).
// ---------------------------------------------------------------------

/// D16 rule 4 test 3: at a depth where the (brine-offset) source
/// temperature exceeds every sink across the record — 3200 m puts the
/// source mean ≈ 90 °C, above the 50 °C DHW sink and every
/// weather-compensated space sink — the all-GSHP entry passes heat
/// through at the district `cop_const`, and its electrical series
/// matches the all-district result within 1e-9 GW per period (the
/// stated tolerance: the two paths sum the space and DHW components in
/// a different order, so exact bitwise equality is not claimed).
#[test]
fn deep_enough_all_gshp_matches_the_district_geothermal_endpoint() {
    require_packs();
    let (_, gshp) = heated_inputs(HeatingKind::Gshp, Some(3200.0));
    let (_, district) = heated_inputs(HeatingKind::DistrictGeothermal, None);

    let gshp_overlay = gshp.heating.as_ref().unwrap();
    let district_overlay = district.heating.as_ref().unwrap();
    for t in 0..gshp_overlay.electrical_total.len() {
        let g = gshp_overlay.electrical_total[t].as_gigawatts();
        let d = district_overlay.electrical_total[t].as_gigawatts();
        assert!(
            (g - d).abs() < 1e-9,
            "period {t}: deep all-GSHP {g} GW differs from district {d} GW by more than \
             the stated 1e-9 GW tolerance (D16 rule 4 test 3 — the endpoints must meet)"
        );
    }
}

// ---------------------------------------------------------------------
// Test 4 — determinism (ADR-5).
// ---------------------------------------------------------------------

/// D16 rule 4 test 5 (ADR-5): the depth path is a pure function of
/// (scenario, data pack) — two independent loads and runs at the same
/// depth are bit-identical.
#[test]
fn depth_path_is_deterministic() {
    require_packs();
    let (scenario_a, inputs_a) = heated_inputs(HeatingKind::Gshp, Some(500.0));
    let (scenario_b, inputs_b) = heated_inputs(HeatingKind::Gshp, Some(500.0));
    assert_eq!(inputs_a.demand.values(), inputs_b.demand.values());
    let peak_a = peak_residual(&run(&scenario_a, &inputs_a).unwrap());
    let peak_b = peak_residual(&run(&scenario_b, &inputs_b).unwrap());
    assert_eq!(peak_a, peak_b, "two identical runs must agree bitwise");
}

// ---------------------------------------------------------------------
// The D16 rule-5 deliverable: the continuum curve, pinned.
// ---------------------------------------------------------------------

/// The 40-year storage requirement at the STATED 200 GW store-power
/// convention (the committed D9 pin's convention: the committed 100 GW
/// rating is power-bound infeasible under all-electrified heating, so
/// the ENERGY requirement is measured with the rating raised on both
/// endpoints; the peak residual is power-independent).
fn requirement_at_200gw(scenario: &Scenario, inputs: &RunInputs) -> f64 {
    let mut wide = scenario.clone();
    wide.zones[0].storage[0].power_gw = Power::gigawatts(200.0);
    min_storage_for_zero_unserved(&wide, inputs, 0, &SolveOptions::default())
        .unwrap()
        .naive
        .requirement
        .as_gigawatt_hours()
}

/// D16 rule 5, the deliverable: the CONTINUUM CURVE — all-GSHP peak
/// residual and 40-year storage requirement vs geothermal resource
/// depth on the RS-37y fleet, shallow loop → warm aquifer →
/// direct-use, with the all-ASHP and all-district endpoints for
/// context. Every number quoted from this table is PINNED here first
/// (the publication rule; measured 2026-07-06, this engine + pack).
///
/// Assumptions as the committed D9 pin: quantum 410.5 TWh record-mean,
/// electrified_share 0.5, DHW 0.170, reference COP parameters, the
/// 25 °C/km gradient centre. Standing caveats travel with every quote
/// (D16 rule 6): physical only (no £ — Q11 Stage 7), no cooling
/// credit, idealised steady source (no drawdown), gradient
/// centre-used-band-stated.
#[test]
fn continuum_curve_peak_and_storage_vs_depth_pinned() {
    require_packs();
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/royal-society-37y.toml")).unwrap();
    let baseline_inputs = load_run_inputs(&scenario, &root).unwrap();
    let baseline_peak = peak_residual(&run(&scenario, &baseline_inputs).unwrap());
    let baseline_requirement = requirement_at_200gw(&scenario, &baseline_inputs);

    let measure = |kind: HeatingKind, depth: Option<f64>| -> (f64, f64) {
        let (heated_scenario, inputs) = heated_inputs(kind, depth);
        let peak = peak_residual(&run(&heated_scenario, &inputs).unwrap());
        let requirement = requirement_at_200gw(&heated_scenario, &inputs);
        (peak, requirement)
    };

    // The continuum: shallow loop → warm aquifer → direct use.
    let depths = [
        1.0, 15.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 3000.0,
    ];
    let gshp: Vec<(f64, f64, f64)> = depths
        .iter()
        .map(|&z| {
            let (peak, requirement) = measure(HeatingKind::Gshp, Some(z));
            (z, peak, requirement)
        })
        .collect();
    let (ashp_peak, ashp_requirement) = measure(HeatingKind::Ashp, None);
    let (district_peak, district_requirement) = measure(HeatingKind::DistrictGeothermal, None);

    eprintln!(
        "RS-37y continuum (baseline peak {baseline_peak:.6} GW, requirement \
         {baseline_requirement} GWh at 200 GW):"
    );
    eprintln!(
        "  all-ASHP              peak {ashp_peak} GW (delta {:+.3}), requirement \
         {ashp_requirement} GWh",
        ashp_peak - baseline_peak
    );
    for (z, peak, requirement) in &gshp {
        eprintln!(
            "  all-GSHP z = {z:6.0} m  peak {peak} GW (delta {:+.3}), requirement \
             {requirement} GWh",
            peak - baseline_peak
        );
    }
    eprintln!(
        "  all-district          peak {district_peak} GW (delta {:+.3}), requirement \
         {district_requirement} GWh",
        district_peak - baseline_peak
    );

    // THE PINS (measured; a move is a knowing re-pin with the record).
    let pin = |got: f64, expected: f64, what: &str| {
        assert!(
            (got - expected).abs() < 1e-9,
            "PINNED {what} moved: got {got}, pinned {expected}"
        );
    };
    pin(baseline_peak, 92.238_714_905_744_56, "baseline peak");
    pin(baseline_requirement, 23_872.0, "baseline requirement");
    pin(ashp_peak, 115.688_943_360_872_74, "all-ASHP peak");
    pin(ashp_requirement, 43_488.0, "all-ASHP requirement");
    let pinned_gshp = [
        (1.0, 114.396_903_567_512_37, 41_248.0),
        (15.0, 111.856_189_778_648_43, 38_304.0),
        (100.0, 110.704_452_412_494_49, 36_672.0),
        (250.0, 108.898_391_483_077_25, 34_144.0),
        (500.0, 106.387_963_252_077_53, 31_472.0),
        (750.0, 104.373_827_675_675_16, 30_656.0),
        (1000.0, 104.095_678_322_521_76, 30_528.0),
        (1250.0, 103.989_699_125_007_63, 28_880.0),
        (1500.0, 96.448_556_016_785_69, 26_272.0),
        (1750.0, 96.448_556_016_785_69, 26_272.0),
        (2000.0, 95.850_577_322_069_94, 25_872.0),
        (3000.0, 95.850_577_322_069_94, 25_872.0),
    ];
    for ((z, peak, requirement), (pz, ppeak, prequirement)) in gshp.iter().zip(&pinned_gshp) {
        assert_eq!(z, pz, "depth grid drifted");
        pin(*peak, *ppeak, &format!("all-GSHP peak at {z} m"));
        pin(
            *requirement,
            *prequirement,
            &format!("all-GSHP requirement at {z} m"),
        );
    }
    pin(district_peak, 95.850_577_322_069_94, "all-district peak");
    pin(district_requirement, 25_872.0, "all-district requirement");

    // The endpoints meet: deep all-GSHP (≥ 2000 m) IS the district
    // result on both measures — the trichotomy is a continuum.
    pin(gshp[11].1, district_peak, "3000 m = district peak");
    pin(
        gshp[11].2,
        district_requirement,
        "3000 m = district requirement",
    );
}

// ---------------------------------------------------------------------
// The D16 SCOP read-out (docs/notes/d16-scop-readout-work-order.md):
// the seasonal-average delivered COP latent in the committed overlay
// series, surfaced per depth and pinned. A read-out, not new physics —
// SCOP = Σ delivered_heat / Σ electrical_total over the pinned trace,
// as-delivered (RHPP deratings included), read from the SAME overlay
// that produces the continuum above so it cannot drift from it.
// ---------------------------------------------------------------------

/// The committed continuum depth grid (the parent pinned test's grid,
/// duplicated verbatim — the SCOP tests must walk the same depths).
const SCOP_DEPTHS: [f64; 12] = [
    1.0, 15.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 3000.0,
];

/// The overlay SCOP of a single-technology heated run (the read-out
/// under test), plus the raw GW-sums behind it.
fn scop_and_sums(kind: HeatingKind, depth: Option<f64>) -> (f64, f64, f64) {
    let (_, inputs) = heated_inputs(kind, depth);
    let overlay = inputs.heating.as_ref().unwrap();
    let scop = overlay.seasonal_cop().unwrap();
    let heat: f64 = overlay
        .delivered_heat
        .iter()
        .map(|p| p.as_gigawatts())
        .sum();
    let elec: f64 = overlay
        .electrical_total
        .iter()
        .map(|p| p.as_gigawatts())
        .sum();
    (scop, heat, elec)
}

/// SCOP acceptance test 1 — the calibration anchor: the all-ASHP
/// delivered SCOP over the pinned trace reproduces the committed
/// RHPP-anchored calibration point EXACTLY (derived, not re-measured:
/// COP is linear in the derating factor and the RS-37y horizon is the
/// full 701,280-period record, so the overlay SCOP equals
/// rhpp_derating × implied_spfh2 — the D9 edit-6 machinery). It lands
/// near the RHPP field median 2.65: the model is honest about
/// air-source, as-delivered, not nameplate.
#[test]
fn all_ashp_delivered_scop_reproduces_the_rhpp_calibration_point() {
    require_packs();
    let root = repo_root();
    let (scop, _, _) = scop_and_sums(HeatingKind::Ashp, None);

    let reference = HeatingCopReference::load(&root.join(HEATING_COP_REFERENCE_PATH)).unwrap();
    let t_pop =
        load_temperature_trace_c(&root.join("data/weather/gb_t2m_pop.parquet"), "t2m_pop").unwrap();
    let expected = reference.ashp.rhpp_derating.value()
        * implied_spfh2(HeatingKind::Ashp, &reference, &t_pop).unwrap();
    assert!(
        (scop - expected).abs() < 1e-9,
        "all-ASHP delivered SCOP {scop} differs from the derated implied SPFH2 {expected} \
         (SCOP acceptance test 1 — the calibration anchor)"
    );
    assert!(
        (scop - reference.rhpp.ashp_spfh2_median).abs() < 0.05,
        "all-ASHP delivered SCOP {scop} does not land near the RHPP field median {}",
        reference.rhpp.ashp_spfh2_median
    );
}

/// SCOP acceptance test 2 — the anti-drift pin: at every depth in the
/// committed grid, Σ delivered_heat / SCOP reproduces the overlay's own
/// Σ electrical_total — the SCOP is the exact electricity behind the
/// pinned peak/storage numbers, one run, two lenses. Tolerance 1e-9
/// RELATIVE: the GW-sums are ~10⁷-magnitude over 701,280 periods, so an
/// absolute 1e-9 would be below f64 resolution of the sums themselves.
#[test]
fn scop_reconciles_with_the_overlay_electricity_at_every_depth() {
    require_packs();
    for &z in &SCOP_DEPTHS {
        let (scop, heat, elec) = scop_and_sums(HeatingKind::Gshp, Some(z));
        assert!(
            (heat / scop - elec).abs() <= 1e-9 * elec,
            "depth {z} m: Σheat/SCOP = {} GW-sum differs from Σelectrical_total = {elec} \
             GW-sum (SCOP acceptance test 2 — the anti-drift pin)",
            heat / scop
        );
    }
}

/// SCOP acceptance test 3 — monotonicity: a deeper resource is a warmer
/// source, so all-GSHP SCOP rises WEAKLY with depth (flat stretches are
/// legal — the ΔT floor and the direct-use cap both produce them; a
/// FALL is red). The mirror of the parent test's peak falling weakly.
#[test]
fn all_gshp_scop_rises_weakly_with_depth() {
    require_packs();
    let mut previous: Option<(f64, f64)> = None;
    for &z in &SCOP_DEPTHS {
        let (scop, _, _) = scop_and_sums(HeatingKind::Gshp, Some(z));
        if let Some((prev_z, prev_scop)) = previous {
            assert!(
                scop >= prev_scop - 1e-9,
                "SCOP FELL with depth: {prev_scop} at {prev_z} m → {scop} at {z} m \
                 (SCOP acceptance test 3)"
            );
        }
        previous = Some((z, scop));
    }
}

/// SCOP acceptance test 4 — the direct-use cap: at/above the crossover
/// the all-GSHP entry passes heat through at the district cop_const, so
/// its SCOP converges to the all-district SCOP — which is cop_const
/// (= 15, the delivered-heat pass-through) — and never diverges
/// (no COP → ∞: the electricity never reaches zero).
#[test]
fn deep_all_gshp_scop_is_the_district_pass_through() {
    require_packs();
    let root = repo_root();
    let reference = HeatingCopReference::load(&root.join(HEATING_COP_REFERENCE_PATH)).unwrap();
    let (district_scop, _, _) = scop_and_sums(HeatingKind::DistrictGeothermal, None);
    assert!(
        (district_scop - reference.district.cop_const).abs() < 1e-9,
        "all-district SCOP {district_scop} differs from cop_const {}",
        reference.district.cop_const
    );
    let (deep_scop, _, _) = scop_and_sums(HeatingKind::Gshp, Some(3000.0));
    assert!(
        (deep_scop - district_scop).abs() < 1e-9,
        "deep (3000 m) all-GSHP SCOP {deep_scop} differs from the all-district SCOP \
         {district_scop} (SCOP acceptance test 4 — the endpoints meet in the correspondent's units)"
    );
}

/// SCOP acceptance test 5 — determinism (ADR-5): two independent loads
/// and computations at the same depth give bit-identical SCOP.
#[test]
fn scop_is_deterministic() {
    require_packs();
    let (scop_a, _, _) = scop_and_sums(HeatingKind::Gshp, Some(500.0));
    let (scop_b, _, _) = scop_and_sums(HeatingKind::Gshp, Some(500.0));
    assert_eq!(scop_a, scop_b, "two identical runs must agree bitwise");
}

/// The D16 SCOP deliverable: the SCOP-vs-depth curve, PINNED alongside
/// the committed continuum — all-ASHP, all-GSHP across the committed
/// depth grid, all-district. Every number quoted in the run report and
/// the reply to the correspondent is pinned here first (the publication rule;
/// measured 2026-07-06, this engine + pack). Same standing caveats as
/// the parent pin: physical only (no £), no cooling credit, gradient
/// centre 25 °C/km with the BGS 26–35 band stated, idealised steady
/// source.
#[test]
fn scop_vs_depth_pinned() {
    require_packs();
    let (ashp_scop, _, _) = scop_and_sums(HeatingKind::Ashp, None);
    let gshp: Vec<(f64, f64)> = SCOP_DEPTHS
        .iter()
        .map(|&z| (z, scop_and_sums(HeatingKind::Gshp, Some(z)).0))
        .collect();
    let (district_scop, _, _) = scop_and_sums(HeatingKind::DistrictGeothermal, None);

    eprintln!("RS-37y seasonal COP (delivered, RHPP-derated) vs depth:");
    eprintln!("  all-ASHP              SCOP {ashp_scop}");
    for (z, scop) in &gshp {
        eprintln!("  all-GSHP z = {z:6.0} m  SCOP {scop}");
    }
    eprintln!("  all-district          SCOP {district_scop}");

    let pin = |got: f64, expected: f64, what: &str| {
        assert!(
            (got - expected).abs() < 1e-9,
            "PINNED {what} moved: got {got}, pinned {expected}"
        );
    };
    pin(ashp_scop, 2.651_334_338_108_820_4, "all-ASHP SCOP");
    let pinned_gshp = [
        (1.0, 2.810_383_303_689_396),
        (15.0, 2.970_297_802_811_172),
        (100.0, 3.150_592_884_138_049),
        (250.0, 3.484_335_075_625_818),
        (500.0, 4.023_964_252_128_673),
        (750.0, 4.358_817_505_648_68),
        (1000.0, 5.017_814_877_708_33),
        (1250.0, 7.824_393_095_253_202),
        (1500.0, 10.667_094_514_576_947),
        (1750.0, 10.845_319_572_283_344),
        (2000.0, 14.999_999_999_967_464),
        (3000.0, 14.999_999_999_967_464),
    ];
    for ((z, scop), (pz, pscop)) in gshp.iter().zip(&pinned_gshp) {
        assert_eq!(z, pz, "depth grid drifted");
        pin(*scop, *pscop, &format!("all-GSHP SCOP at {z} m"));
    }
    pin(district_scop, 14.999_999_999_967_464, "all-district SCOP");

    // The endpoints meet in the correspondent's units too: deep all-GSHP IS the
    // district pass-through.
    pin(gshp[11].1, district_scop, "3000 m SCOP = district SCOP");
}

// ---------------------------------------------------------------------
// Test 5 — calibration anchor (OWED: awaiting validation data).
// ---------------------------------------------------------------------

/// D16 rule 4 test 4: reproduce a real installation's measured
/// seasonal efficiency at its known depth/source temperature within a
/// stated tolerance. The validation data (the company (on file) operating projects
/// — Richard/the correspondent to supply — or published United Downs/Southampton/
/// Eastgate figures) has NOT yet been delivered, so this anchor is
/// OWED: the test is ignored, not silently green.
#[test]
#[ignore = "OWED (D16 rule 3 item 4): no real-installation validation data supplied yet — \
            the company (on file) anchor (Richard/the correspondent) or United Downs/Southampton/Eastgate published \
            figures; un-ignore and pin when the data package delivers them"]
fn calibration_anchor_against_a_real_installation() {
    panic!(
        "the D16 calibration anchor is owed: supply a real installation's measured seasonal \
         efficiency at known depth (the company (on file) / United Downs / Southampton / Eastgate), then \
         pin the model's reproduction here within a stated tolerance"
    );
}
