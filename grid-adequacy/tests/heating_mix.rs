//! Q5/Q11 analysis-runs acceptance (D9 rules 6 and 6b, fixed-fleet
//! leg): the heating-mix simplex sweep and the timescale decomposition
//! of the electrified-heat addition, on the Royal-Society 37+-year
//! fleet. Extends the conventions of `heating.rs` (the characterisation
//! pin lives there and MUST keep passing unchanged — this suite ties
//! the sweep machinery to it, never replaces it).
//!
//! Red-first contract:
//! - the 0.70/0.20/0.10 CROSS-CHECK values are the existing
//!   characterisation pins (`heating.rs`), known before the sweep
//!   machinery existed — the sweep must reproduce them EXACTLY (bit
//!   identity, not tolerance);
//! - the district-lowest-peak limb is a theorem given the rule-4
//!   `COP_const` premise check (D9 rule 5 test 3) — asserted red-first;
//! - corner-point values and decomposition bands are MEASURED THEN
//!   PINNED (the D9 ruling-C pattern): first run printed them, the pins
//!   below record that measurement.
//!
//! STORE-POWER CONVENTION (binding record, q5-heating-engine-review):
//! every storage number in this suite is at the STATED 200 GW rating,
//! applied to BOTH endpoints of every delta; the committed 100 GW
//! rating is power-bound infeasible under the heated peak residual
//! (pinned in `heating.rs`) and that finding travels with any
//! requirement delta quoted from here.
//!
//! These tests need the fetched per-year 1985–2024 pack and the pinned
//! GB t2m trace; they FAIL LOUDLY if missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::heating_mix::{HeatingMixContext, MixOutcome, MixShares};
use grid_adequacy::{Execution, SolveOptions};
use grid_core::analysis::DecompositionWindows;
use grid_core::scenario::Scenario;
use grid_core::units::Power;

const HEATED_SCENARIO: &str = "scenarios/royal-society-37y-heated.toml";

/// The stated rating for every storage number here (both endpoints).
const STATED_RATING_GW: f64 = 200.0;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn require_data() {
    let root = repo_root();
    let mut missing: Vec<String> = Vec::new();
    for rel in ["data/weather/gb_t2m_pop.parquet".to_owned()]
        .into_iter()
        .chain((1985..=2024).flat_map(|year| {
            [
                format!("data/packs/demand-tiled/demand_{year}.parquet"),
                format!("data/packs/cf/gb_onshore_cf_{year}.parquet"),
                format!("data/packs/cf/gb_offshore_cf_{year}.parquet"),
                format!("data/packs/cf/gb_solar_cf_{year}.parquet"),
            ]
        }))
    {
        if !root.join(&rel).exists() {
            missing.push(rel);
        }
    }
    assert!(
        missing.is_empty(),
        "required data missing ({} file(s), first {}) — build the packs first \
         (scripts/fetch-2024, scripts/era5-cf, derive_t2m_gb.py)",
        missing.len(),
        missing[0]
    );
}

fn context(root: &std::path::Path) -> HeatingMixContext {
    let scenario = Scenario::load(&root.join(HEATED_SCENARIO)).unwrap();
    HeatingMixContext::load(&scenario, root, Power::gigawatts(STATED_RATING_GW), 0).unwrap()
}

// ---------------------------------------------------------------------
// The sweep: cross-check to the characterisation pin, corner pins,
// determinism.
// ---------------------------------------------------------------------

/// The sweep machinery run over the three corners, the D9
/// 0.70/0.20/0.10 lattice point, and the full interior of the
/// ASHP→district edge (all on the step-0.1 simplex).
///
/// CROSS-CHECK (red-first — the values are the `heating.rs`
/// characterisation pins, which predate this machinery): the sweep's
/// 0.70/0.20/0.10 point and its baseline must reproduce those pins
/// EXACTLY — peak residual 92.23871490574456 → 113.4466987983204 GW,
/// requirement at 200 GW 23,872 → 40,224 GWh. This ties the sweep to
/// the pinned baseline; a divergence means the sweep's input
/// construction is NOT the engine's own arithmetic.
///
/// CORNER PINS (measured 2026-07-04, then pinned — D9 ruling C), all at
/// 200 GW store power, both endpoints, quantum 410.5 TWh × 0.5
/// electrified, DHW 0.170, reference COP parameters (none of the three
/// corner solves is year-1-sensitive; no burn-in re-run):
///   all-ASHP:     peak residual 115.68894336087274 GW,
///                 requirement 43,488 GWh
///   all-GSHP:     peak residual 114.39690356751237 GW,
///                 requirement 41,248 GWh
///   all-district: peak residual  95.85057732206994 GW,
///                 requirement 25,872 GWh
/// Expected direction (D9 rule 5 test 3): all-district lowest is a
/// THEOREM given the machine-checked `COP_const` premise — asserted
/// here; all-ASHP ≥ all-GSHP is an EXPECTATION pinned from
/// measurement — if a re-pin ever inverts it, that inversion is a
/// finding at full prominence (Package A/B lesson, kill criterion 4),
/// not a number to tune away.
///
/// ASHP→DISTRICT EDGE PINS (review condition 2 + ruling R1,
/// docs/notes/q5-heating-mix-review.md; measured 2026-07-04, then
/// pinned): the interior edge requirements, GWh at 200 GW both
/// endpoints, shifts 0.1 … 0.9 away from all-ASHP:
///   40,672 / 37,856 / 35,104 / 32,352 / 31,040 / 30,000 / 28,960 /
///   27,936 / 26,896.
/// THE KNEE IS REAL (reviewer-verified at ~100× the 16–32 GWh
/// bisection quantum): steps −2,816/−2,816/−2,752/−2,752 GWh per
/// 10 % on the first four tenths, transition −1,312, then ≈−1,030.
/// QUOTE DUTY (R1): any quoted ASHP→district storage gradient states
/// the two limbs (≈−2,750…−2,816 GWh per 10 % from all-ASHP;
/// ≈−1,030 beyond) or shows the curve; the −1,762 edge average may
/// only appear alongside the limbs — the policy-relevant marginal
/// value from an ASHP-heavy start is the STEEP limb.
///
/// CORNER CURTAILMENT PINS (review condition 2 + ruling R2; horizon
/// totals at the committed 100 TWh dispatch store with the stated
/// 200 GW rating; measured 2026-07-04, then pinned):
///   baseline 18,711,641.18768845 GWh; all-ASHP 14,288,541.744407296;
///   all-GSHP 14,569,496.35542618; all-district 17,962,897.880462743.
/// Directions asserted: electrified heat REDUCES curtailment (heated
/// corners < baseline — absorption of otherwise-curtailed energy,
/// directly and via extra store cycling at η = 0.40); the district
/// share, needing ~5.7× less electricity for the same heat, FORGOES
/// most of that absorption (all-district curtailment > all-ASHP).
/// QUOTE DUTY (R2): the network value of geothermal is peak + storage
/// relief MINUS foregone curtailment absorption; the netting is a
/// Stage 7 (£) statement — until then the two sides are quoted side
/// by side in physical units, never collapsed into one number, and
/// curtailment numbers state the fixed dispatch store they sit at.
#[test]
fn sweep_reproduces_the_characterisation_pin_and_pins_the_corners() {
    require_data();
    let root = repo_root();
    let context = context(&root);

    let mut mixes = vec![
        MixShares::new(10, 0, 0, 10).unwrap(),
        MixShares::new(0, 10, 0, 10).unwrap(),
        MixShares::new(0, 0, 10, 10).unwrap(),
        MixShares::new(7, 2, 1, 10).unwrap(),
    ];
    // The interior of the ASHP→district edge (shifts 0.1 … 0.9),
    // indices 4..13.
    for district in 1..=9 {
        mixes.push(MixShares::new(10 - district, 0, district, 10).unwrap());
    }
    let options = SolveOptions::default();
    let sweep = context
        .sweep(&mixes, &options, Execution::Parallel)
        .unwrap();

    // Determinism under rayon (the Stage 4 acceptance precedent): the
    // serial re-run is bit-identical.
    let serial = context.sweep(&mixes, &options, Execution::Serial).unwrap();
    assert_eq!(sweep, serial, "rayon and serial sweeps differ (ADR-5)");

    let requirement = |outcome: &MixOutcome| -> f64 {
        match outcome {
            MixOutcome::Feasible { requirement, .. } => requirement.as_gigawatt_hours(),
            MixOutcome::Infeasible { reason } => {
                panic!("expected a feasible solve at 200 GW; got: {reason}")
            }
        }
    };

    for point in &sweep.points {
        eprintln!(
            "  {}: peak residual {} GW, requirement {} GWh (year-1-sensitive {:?}), heating \
             electrical {:.2} TWh, curtailment {:.2} TWh",
            point.shares.label(),
            point.metrics.peak_residual.as_gigawatts(),
            requirement(&point.metrics.outcome),
            point.metrics.outcome,
            point.metrics.heating_electrical.as_gigawatt_hours() / 1000.0,
            point.metrics.curtailment.as_gigawatt_hours() / 1000.0,
        );
    }

    // --- The cross-check (the heating.rs characterisation pins). ---
    assert!(
        (sweep.baseline.peak_residual.as_gigawatts() - 92.238_714_905_744_56).abs() < 1e-12,
        "baseline peak residual diverged from the characterisation pin: {}",
        sweep.baseline.peak_residual.as_gigawatts()
    );
    assert_eq!(
        requirement(&sweep.baseline.outcome),
        23_872.0,
        "baseline requirement at 200 GW diverged from the characterisation pin \
         (= the Stage 3 100 GW pin; the rating never bound the baseline)"
    );
    let d9 = &sweep.points[3];
    assert_eq!(d9.shares, MixShares::new(7, 2, 1, 10).unwrap());
    assert!(
        (d9.metrics.peak_residual.as_gigawatts() - 113.446_698_798_320_4).abs() < 1e-12,
        "0.70/0.20/0.10 peak residual diverged from the characterisation pin: {} — the \
         sweep's input construction is not the engine's own arithmetic",
        d9.metrics.peak_residual.as_gigawatts()
    );
    assert_eq!(
        requirement(&d9.metrics.outcome),
        40_224.0,
        "0.70/0.20/0.10 requirement at 200 GW diverged from the characterisation pin"
    );

    // --- Direction theorems/expectations (red-first where theorem). ---
    let (ashp, gshp, district) = (&sweep.points[0], &sweep.points[1], &sweep.points[2]);
    assert!(
        district.metrics.peak_residual < ashp.metrics.peak_residual
            && district.metrics.peak_residual < gshp.metrics.peak_residual,
        "the district-lowest-peak limb failed — a THEOREM given the rule-4 COP_const \
         premise check; investigate before quoting anything from this sweep"
    );
    assert!(
        requirement(&district.metrics.outcome) < requirement(&ashp.metrics.outcome)
            && requirement(&district.metrics.outcome) < requirement(&gshp.metrics.outcome),
        "the district-lowest-requirement limb failed"
    );
    // Measured direction (pinned from measurement, NOT a theorem): an
    // inversion on a future re-pin is a full-prominence finding.
    assert!(
        ashp.metrics.peak_residual > gshp.metrics.peak_residual,
        "FINDING (report at full prominence, do not tune): the all-ASHP peak no longer \
         exceeds the all-GSHP peak — the measured ASHP/GSHP ordering inverted"
    );

    // --- The corner pins (measured 2026-07-04, then pinned). ---
    let pin = |what: &str, measured: f64, pinned: f64| {
        assert!(
            (measured - pinned).abs() < 1e-9,
            "PINNED {what} moved: measured {measured}, pinned {pinned}"
        );
    };
    pin(
        "all-ASHP peak residual (GW)",
        ashp.metrics.peak_residual.as_gigawatts(),
        115.688_943_360_872_74,
    );
    pin(
        "all-ASHP requirement at 200 GW (GWh)",
        requirement(&ashp.metrics.outcome),
        43_488.0,
    );
    pin(
        "all-GSHP peak residual (GW)",
        gshp.metrics.peak_residual.as_gigawatts(),
        114.396_903_567_512_37,
    );
    pin(
        "all-GSHP requirement at 200 GW (GWh)",
        requirement(&gshp.metrics.outcome),
        41_248.0,
    );
    pin(
        "all-district peak residual (GW)",
        district.metrics.peak_residual.as_gigawatts(),
        95.850_577_322_069_94,
    );
    pin(
        "all-district requirement at 200 GW (GWh)",
        requirement(&district.metrics.outcome),
        25_872.0,
    );

    // --- The ASHP→district edge pins (review condition 2 / R1). ---
    let edge_pins = [
        40_672.0, 37_856.0, 35_104.0, 32_352.0, 31_040.0, 30_000.0, 28_960.0, 27_936.0, 26_896.0,
    ];
    for (offset, pinned) in edge_pins.iter().enumerate() {
        let point = &sweep.points[4 + offset];
        assert_eq!(
            point.shares,
            MixShares::new(9 - offset as u32, 0, 1 + offset as u32, 10).unwrap()
        );
        pin(
            &format!(
                "ASHP→district edge requirement at shift 0.{} (GWh at 200 GW)",
                offset + 1
            ),
            requirement(&point.metrics.outcome),
            *pinned,
        );
    }

    // --- The corner curtailment pins + directions (condition 2 / R2;
    // horizon totals at the committed dispatch store, stated rating).
    let curtailment = |m: &grid_adequacy::MixMetrics| m.curtailment.as_gigawatt_hours();
    for (label, m) in [
        ("baseline", &sweep.baseline),
        ("all-ASHP", &ashp.metrics),
        ("all-GSHP", &gshp.metrics),
        ("all-district", &district.metrics),
    ] {
        eprintln!("  curtailment {label}: {} GWh", curtailment(m));
    }
    assert!(
        curtailment(&ashp.metrics) < curtailment(&sweep.baseline)
            && curtailment(&gshp.metrics) < curtailment(&sweep.baseline)
            && curtailment(&district.metrics) < curtailment(&sweep.baseline),
        "DIRECTION (R2): electrified heat must REDUCE curtailment (absorption of \
         otherwise-curtailed energy); an inversion is a full-prominence finding"
    );
    assert!(
        curtailment(&district.metrics) > curtailment(&ashp.metrics),
        "DIRECTION (R2): the district share forgoes most of the curtailment absorption \
         (needs ~5.7x less electricity) — all-district curtailment must exceed all-ASHP's"
    );

    // GEOTHERMAL HEADLINE PIN: the "~5.7x less heating electricity for
    // the same delivered heat" figure (§2 run report; the reason
    // all-district forgoes the curtailment absorption above) was guarded
    // ONLY by the direction inequality — it could drift with the COP
    // model with no test failing. Pin the two horizon heating-electrical
    // totals and their ratio. Values first measured 2026-07-04 (all-ASHP
    // 3,096.55 TWh, all-district 547.33 TWh over the 40-year horizon;
    // ratio 5.658, i.e. the ~5.7x headline). Delivered heat is
    // portfolio-invariant (asserted below), so the ratio is a pure
    // electricity-intensity (COP) ratio.
    let heating_twh =
        |m: &grid_adequacy::MixMetrics| m.heating_electrical.as_gigawatt_hours() / 1000.0;
    let ashp_heat_twh = heating_twh(&ashp.metrics);
    let district_heat_twh = heating_twh(&district.metrics);
    assert!(
        (ashp_heat_twh - 3096.55).abs() < 0.01,
        "PINNED all-ASHP heating-electrical total moved: measured {ashp_heat_twh} TWh"
    );
    assert!(
        (district_heat_twh - 547.33).abs() < 0.01,
        "PINNED all-district heating-electrical total moved: measured {district_heat_twh} TWh"
    );
    let district_vs_ashp_ratio = ashp_heat_twh / district_heat_twh;
    assert!(
        (district_vs_ashp_ratio - 5.6576).abs() < 0.01,
        "PINNED district-vs-ASHP heating-electricity ratio (the ~5.7x headline) moved: \
         measured {district_vs_ashp_ratio}"
    );
    pin(
        "baseline curtailment (GWh, horizon total)",
        curtailment(&sweep.baseline),
        18_711_641.187_688_45,
    );
    pin(
        "all-ASHP curtailment (GWh, horizon total)",
        curtailment(&ashp.metrics),
        14_288_541.744_407_296,
    );
    pin(
        "all-GSHP curtailment (GWh, horizon total)",
        curtailment(&gshp.metrics),
        14_569_496.355_426_18,
    );
    pin(
        "all-district curtailment (GWh, horizon total)",
        curtailment(&district.metrics),
        17_962_897.880_462_743,
    );

    // Every row carries its stated rating (the review's binding item).
    assert_eq!(sweep.store_power, Power::gigawatts(STATED_RATING_GW));

    // Rule-5 invariance spine: delivered heat is identical across
    // mixes by construction (same quantum, same weather).
    for point in &sweep.points {
        assert_eq!(
            point.metrics.delivered_heat, sweep.points[0].metrics.delivered_heat,
            "delivered heat must be portfolio-invariant (D9 rule 5)"
        );
    }
    // Per-tech potential is mix-invariant on an all-must-take fleet:
    // what varies is curtailment/store cycling/unserved — the fixed-
    // fleet generation-relieved accounting (D9 rule 6b).
    for point in &sweep.points {
        assert_eq!(point.metrics.tech_potential, sweep.baseline.tech_potential);
    }
}

// ---------------------------------------------------------------------
// The timescale decomposition of the added requirement (D9 rule 6(c)).
// ---------------------------------------------------------------------

/// Stage 4 attribution machinery on the three named corners vs the
/// no-heating baseline, standard windows 24 h / 14 d / 365 d (the
/// Stage 4 publication rule: the synoptic-vs-seasonal ranking is
/// window-sensitive — never quoted without the window), all at the
/// stated 200 GW rating.
///
/// PINNED (measured 2026-07-04, then pinned — D9 ruling C; GWh at
/// 200 GW store power, both endpoints, windows 24 h / 14 d / 365 d):
///   baseline:     total 23,872 = diurnal 1,088 + synoptic 9,512
///                 + seasonal 13,272 + inter-annual 0
///   all-ASHP:     total 43,488 = 2,848 + 16,096 + 24,544 + 0
///   all-GSHP:     total 41,248 = 2,880 + 15,152 + 23,216 + 0
///   all-district: total 25,872 = 1,040 +  9,816 + 15,016 + 0
///
/// FINDINGS (carried to the run report; window convention stated above
/// per the Stage 4 publication rule):
/// 1. The electrified-heat addition loads the SEASONAL band hardest at
///    every heat-pump corner: all-ASHP adds +19,616 GWh of which
///    seasonal +11,272, synoptic +6,584, diurnal +1,760 (all-GSHP:
///    +17,376 = +9,944 / +5,640 / +1,792). The technology gradient is
///    concentrated there too: the ASHP−district seasonal delta
///    (9,528 GWh) exceeds the synoptic (6,280) and diurnal (1,808)
///    deltas.
/// 2. All-district adds only +2,000 GWh total, and its DIURNAL
///    attribution falls slightly below the baseline's (1,040 vs
///    1,088 GWh — a legitimate outcome of the telescoping attribution:
///    the flat pump load shifts which smoothing level binds).
/// 3. Inter-annual attribution stays 0 at every corner (the
///    365 d-smoothed residual never needs the store on this overbuilt
///    fleet — the Stage 4 posture restated under heating).
#[test]
fn decomposition_of_the_added_requirement_pins_the_three_corners() {
    require_data();
    let root = repo_root();
    let context = context(&root);

    let options = SolveOptions::default();
    let windows = DecompositionWindows::standard();
    let attributions = context.attributions(&windows, &options).unwrap();
    assert_eq!(attributions.len(), 4);

    let expected: [(&str, [f64; 5]); 4] = [
        ("baseline", [23_872.0, 1_088.0, 9_512.0, 13_272.0, 0.0]),
        ("all_ashp", [43_488.0, 2_848.0, 16_096.0, 24_544.0, 0.0]),
        ("all_gshp", [41_248.0, 2_880.0, 15_152.0, 23_216.0, 0.0]),
        ("all_district", [25_872.0, 1_040.0, 9_816.0, 15_016.0, 0.0]),
    ];

    for named in &attributions {
        let attribution = &named.attribution;
        eprintln!(
            "  {:<13} total {} GWh; bands {} / {} / {} / {}",
            named.label,
            attribution.total.as_gigawatt_hours(),
            attribution.bands[0].requirement.as_gigawatt_hours(),
            attribution.bands[1].requirement.as_gigawatt_hours(),
            attribution.bands[2].requirement.as_gigawatt_hours(),
            attribution.bands[3].requirement.as_gigawatt_hours(),
        );
    }

    for (named, (label, values)) in attributions.iter().zip(&expected) {
        let attribution = &named.attribution;
        let total = attribution.total.as_gigawatt_hours();
        assert_eq!(named.label, *label);

        // Telescoping: bands sum to the total (kill criterion 2).
        let band_sum: f64 = attribution
            .bands
            .iter()
            .map(|b| b.requirement.as_gigawatt_hours())
            .sum();
        assert!(
            (band_sum - total).abs() < 1e-6,
            "{label}: bands sum to {band_sum}, total {total}"
        );

        // The pins (measured then pinned).
        assert!(
            (total - values[0]).abs() < 1e-9,
            "PINNED {label} attribution total moved: measured {total}, pinned {}",
            values[0]
        );
        for (band, pinned) in attribution.bands.iter().zip(&values[1..]) {
            assert!(
                (band.requirement.as_gigawatt_hours() - pinned).abs() < 1e-9,
                "PINNED {label} {} band moved: measured {}, pinned {pinned}",
                band.band.as_str(),
                band.requirement.as_gigawatt_hours()
            );
        }
    }

    // The attribution totals must reproduce the sweep's requirements
    // (same solver, same rating): baseline 23,872, corners per the
    // sweep pins — asserted through the expected table above; here the
    // cross-machinery identity is the baseline vs the Stage 3 pin.
    assert_eq!(
        attributions[0].attribution.total.as_gigawatt_hours(),
        23_872.0
    );
}
