//! Stage 4 acceptance tests (docs/04 Stage 4; docs/07 Module 3(c),
//! Module 4, Q4; docs/08 kill criterion 2). Written failing at stage
//! start, per the red-green rule.
//!
//! - (a) Sweep determinism: a rayon-parallel sweep is bit-identical to
//!   the same sweep executed serially (ADR-5 under ADR-10 parallelism).
//! - (b) Decomposition exact-sum: the four timescale bands sum back to
//!   the original residual-load series, period by period, to f64
//!   rounding dust — they are constructed as successive differences of
//!   a smoothing cascade, so the sum telescopes by construction
//!   (`grid_core::analysis::decompose`). Checked on the real 40-year
//!   RS-lean residual.
//! - (c) Storage-attribution sum: the four band attributions sum to the
//!   total storage requirement. **Definition of the sum (normative):**
//!   band attributions are successive differences of the bisection
//!   requirement across the smoothing cascade,
//!   `A(band k) = M(level k−1) − M(level k)` with
//!   `A(inter-annual) = M(seasonal level)`, so
//!   `Σ A = M(unfiltered residual) = total` **exactly by telescoping**;
//!   the tolerance below (1e-6 GWh) covers only f64 re-association in
//!   the summation. Separately, the total itself must reproduce the
//!   Stage 3 pinned requirement for the same scenario within the
//!   bisection's own convergence tolerance (`max(0.1 GWh, 1e-3 × hi)`
//!   ≈ 58.4 GWh here) — that is the tolerance the method justifies:
//!   the attribution's synthetic bare-residual runs replay the same
//!   store mechanics the pinned solve used.
//! - (d) Stability under filter perturbation — THE KILL-CRITERION-2
//!   DETECTOR (docs/08: "if ... attribution is unstable to filter
//!   choice, do not publish the 'few days vs 100 TWh' argument"):
//!   moving the synoptic window across 10 d / 14 d / 21 d must move
//!   every band attribution by less than the stated tolerance (below).
//!   If this test fails, the failure is REPORTED, not tuned away.
//! - (e) Q4 per-year batch: all 40 weather years 1985–2024 solved as
//!   independent single-year scenarios, deterministically; the design
//!   (max) and easiest (min) years identified and pinned.
//!
//! Determinism basis for the pins:
//! `results = f(scenario, data-pack checksums, engine)` (ADR-5); pinned
//! values were first measured against the real 1985–2024 pack
//! (snapshot 39TK56WX185WZ1HP9WNG) on 2026-07-03. A deliberate change
//! to any input is a knowing re-pin, recorded in the stage run report.
//!
//! These tests need the per-year 1985–2024 data pack (fetched, not
//! committed) and FAIL LOUDLY if it is missing (no `#[ignore]`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::sweep::{Dimension, Execution, YearOutcome, per_year_requirements, run_sweep};
use grid_adequacy::{
    Band, SolveOptions, attribute_storage_by_band, load_run_inputs, min_storage_for_zero_unserved,
};
use grid_core::analysis::{DecompositionWindows, decompose, residual_load};
use grid_core::scenario::{Scenario, TechId};
use grid_core::units::{Duration, Energy, Power};

const LEAN_SCENARIO: &str = "scenarios/royal-society-37y-lean.toml";
const BENIGN_SCENARIO: &str = "scenarios/gb-2024-benign-battery.toml";

/// The Stage 3 pinned RS-lean requirement (store-side GWh) — the total
/// the attribution must reproduce.
const LEAN_PINNED_REQUIREMENT_GWH: f64 = 58_432.0;

/// The full 1985–2024 record: 30 × 365 + 10 × 366 days, half-hourly.
const FULL_RECORD_PERIODS: usize = 701_280;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Fail loudly if the per-year 1985–2024 pack is missing (repo
/// convention since Stage 3: acceptance tests are red, never skipped).
fn require_full_record_pack() {
    let root = repo_root();
    let mut missing: Vec<String> = Vec::new();
    for year in 1985..=2024 {
        for rel in [
            format!("data/packs/demand-tiled/demand_{year}.parquet"),
            format!("data/packs/cf/gb_onshore_cf_{year}.parquet"),
            format!("data/packs/cf/gb_offshore_cf_{year}.parquet"),
            format!("data/packs/cf/gb_solar_cf_{year}.parquet"),
        ] {
            if !root.join(&rel).exists() {
                missing.push(rel);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the per-year 1985–2024 data pack is incomplete: {} file(s) missing, first {} — \
         build it per scripts/era5-cf and scripts/fetch-2024 (fetched, not committed)",
        missing.len(),
        missing[0]
    );
}

/// The RS-lean residual-load series (demand − must-take), computed in
/// the dispatch engine's own summation order so the attribution's
/// synthetic runs replay the pinned solve's arithmetic.
fn lean_residual(root: &std::path::Path) -> (Scenario, grid_adequacy::RunInputs, Vec<Power>) {
    let scenario = Scenario::load(&root.join(LEAN_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, root).unwrap();
    let result = grid_adequacy::run(&scenario, &inputs).unwrap();
    let must_take: Vec<&[Power]> = result
        .renewables
        .iter()
        .map(|s| s.power.as_slice())
        .chain(result.exogenous.iter().map(|s| s.power.as_slice()))
        .collect();
    let residual = residual_load(&result.demand, &must_take).unwrap();
    (scenario, inputs, residual)
}

// ---------------------------------------------------------------------
// (a) Sweep determinism under rayon.
// ---------------------------------------------------------------------

/// A rayon-parallel sweep must be bit-identical to serial execution:
/// every point is a pure function of its scenario variant, and ordered
/// collection preserves point order, so the two `SweepResult`s must
/// compare exactly equal (f64 bit equality via `PartialEq`).
#[test]
fn sweep_is_bit_identical_between_rayon_and_serial() {
    let root = repo_root();
    let scenario = Scenario::load(&root.join(BENIGN_SCENARIO)).unwrap();
    let dimensions = vec![
        Dimension::FleetScale {
            technologies: vec![
                TechId::new("offshore_wind"),
                TechId::new("onshore_wind"),
                TechId::new("solar"),
            ],
            values: vec![0.8, 1.0, 1.2],
        },
        Dimension::StoreEnergy {
            store_index: 0,
            values: vec![
                Energy::gigawatt_hours(9.0),
                Energy::gigawatt_hours(36.0),
                Energy::gigawatt_hours(72.0),
            ],
        },
    ];
    let parallel = run_sweep(&scenario, &root, &dimensions, Execution::Parallel).unwrap();
    let serial = run_sweep(&scenario, &root, &dimensions, Execution::Serial).unwrap();
    assert_eq!(
        parallel, serial,
        "rayon and serial sweep results differ — determinism under parallelism is broken \
         (ADR-5 / docs/04 Stage 4 acceptance)"
    );
    assert_eq!(parallel.points.len(), 9, "3×3 grid must yield 9 points");
    // The sweep is not vacuous: the 12 h battery point (36 GWh at 1.0×)
    // is the pinned benign zero-unserved case; the 9 GWh / 0.8× corner
    // must be short.
    let full = parallel
        .points
        .iter()
        .find(|p| p.indices == vec![1, 1])
        .unwrap();
    assert_eq!(full.unserved, Energy::gigawatt_hours(0.0));
    let corner = parallel
        .points
        .iter()
        .find(|p| p.indices == vec![0, 0])
        .unwrap();
    assert!(corner.unserved > Energy::gigawatt_hours(0.0));
}

// ---------------------------------------------------------------------
// (b) Decomposition exact-sum on the real 40-year residual.
// ---------------------------------------------------------------------

/// The four bands sum back to the original residual series period by
/// period. The bands are successive differences of a smoothing cascade
/// (diurnal = r − s₁, synoptic = s₁ − s₂, seasonal = s₂ − s₃,
/// inter-annual = s₃), so the sum telescopes to r *by construction*;
/// the 1e-6 GW tolerance covers only f64 re-association (measured dust
/// is ≲ 1e-11 GW on ~50 GW values).
#[test]
fn decomposition_bands_sum_to_the_residual_on_the_real_record() {
    require_full_record_pack();
    let root = repo_root();
    let (_, _, residual) = lean_residual(&root);
    assert_eq!(residual.len(), FULL_RECORD_PERIODS);

    let decomposition = decompose(&residual, &DecompositionWindows::standard()).unwrap();
    let mut worst = 0.0f64;
    for (t, &value) in residual.iter().enumerate() {
        let sum = decomposition.diurnal[t]
            + decomposition.synoptic[t]
            + decomposition.seasonal[t]
            + decomposition.inter_annual[t];
        let error = (sum - value).as_gigawatts().abs();
        worst = worst.max(error);
    }
    eprintln!("decomposition exact-sum: worst per-period error {worst:.3e} GW");
    assert!(
        worst < 1e-6,
        "bands do not sum to the residual: worst error {worst} GW — the telescoping \
         construction is broken (kill criterion 2: bands must approximately sum)"
    );
}

// ---------------------------------------------------------------------
// (c) Storage-attribution sum on the RS-lean pinned scenario.
// ---------------------------------------------------------------------

/// Band attributions sum to the total storage requirement (telescoping,
/// module docs above), and the total reproduces the Stage 3 pinned
/// 58,432 GWh within the bisection's own convergence tolerance.
///
/// PINNED (first measurement, 2026-07-03, standard windows 24 h /
/// 14 d / 365 d): total 58,432 GWh — **exactly** the Stage 3 pin (the
/// bare-residual replay reproduced the pinned solve bit-for-bit, so
/// the tolerance assertion below passed with zero error); bands
/// diurnal 14,816 GWh (25.4 %), synoptic 18,224 GWh (31.2 %),
/// seasonal 25,392 GWh (43.5 %), inter-annual 0 GWh (0.0 %).
///
/// TWO FINDINGS THE NARRATIVE MUST CARRY (kill-criterion-4 posture —
/// contrary-looking results reported with full prominence):
///
/// 1. **The diurnal band is a quarter, not a sliver.** Under this
///    attribution (requirement drop when sub-window variation is
///    smoothed away), removing intra-day variation saves 14.8 TWh —
///    because at η = 0.40 every within-day swing cycled through the
///    hydrogen store pays the ~60 % round-trip toll during the binding
///    drawdown (smoothing ≈ toll-free intra-day balancing; 0.95 GWh of
///    store per GWh of daily two-way traffic). The "few days of
///    storage" claim addresses at most this quarter; ~75 % of the
///    requirement (43.6 TWh) is variation at synoptic-and-slower
///    timescales that no daily-cycling battery fleet answers.
/// 2. **The inter-annual band attributes 0 GWh** — at 1.35× supply the
///    365 d-smoothed residual is in surplus at every window position,
///    so the year-scale *mean* never needs the store. The multi-year
///    below-full episodes (Stage 3's 720-day drawdown) are driven by
///    seasonal-band variation plus slow post-drought recharge; "the
///    requirement is inter-annual" is NOT supported by this
///    attribution and must not be claimed — the supported claim is
///    "the requirement is seasonal-and-synoptic, and recovery is
///    multi-year".
#[test]
fn band_attributions_sum_to_the_pinned_lean_requirement() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(LEAN_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let attribution = attribute_storage_by_band(
        &scenario,
        &inputs,
        0,
        &DecompositionWindows::standard(),
        &SolveOptions::default(),
    )
    .unwrap();

    let total = attribution.total.as_gigawatt_hours();
    let band_sum: f64 = attribution
        .bands
        .iter()
        .map(|b| b.requirement.as_gigawatt_hours())
        .sum();
    for band in &attribution.bands {
        eprintln!(
            "  band {:<12} {:>10.1} GWh ({:>5.2} % of total)",
            band.band.as_str(),
            band.requirement.as_gigawatt_hours(),
            100.0 * band.requirement.as_gigawatt_hours() / total,
        );
    }
    eprintln!("  total {total:.1} GWh; band sum {band_sum:.1} GWh");

    // The sum is telescoping — exact up to f64 re-association.
    assert!(
        (band_sum - total).abs() < 1e-6,
        "band attributions sum to {band_sum} GWh but the total is {total} GWh \
         (must telescope exactly; kill criterion 2)"
    );
    // The total reproduces the Stage 3 pin within the bisection's own
    // convergence tolerance (the tolerance the method justifies).
    let bisection_tolerance = (1e-3 * total).max(0.1);
    assert!(
        (total - LEAN_PINNED_REQUIREMENT_GWH).abs() <= bisection_tolerance,
        "attribution total {total} GWh does not reproduce the Stage 3 pinned \
         {LEAN_PINNED_REQUIREMENT_GWH} GWh within the bisection tolerance \
         {bisection_tolerance} GWh"
    );

    // The regression pins (doc comment above; deterministic, ADR-5).
    assert!(
        (total - 58_432.0).abs() < 1e-9,
        "PINNED attribution total moved: measured {total} GWh"
    );
    let pinned = [
        (Band::Diurnal, 14_816.0),
        (Band::Synoptic, 18_224.0),
        (Band::Seasonal, 25_392.0),
        (Band::InterAnnual, 0.0),
    ];
    for ((band, expected), measured) in pinned.iter().zip(&attribution.bands) {
        assert_eq!(*band, measured.band);
        assert!(
            (measured.requirement.as_gigawatt_hours() - expected).abs() < 1e-9,
            "PINNED {} attribution moved: measured {} GWh, pinned {expected} GWh",
            band.as_str(),
            measured.requirement.as_gigawatt_hours()
        );
    }
}

// ---------------------------------------------------------------------
// (d) Stability under filter perturbation — the kill-criterion-2
// detector.
// ---------------------------------------------------------------------

/// Moving the synoptic window across 10 d / 14 d / 21 d (docs/04 work
/// order). The gate has three parts, each protecting a specific
/// publication claim; the tolerances were set from the FIRST
/// measurement (2026-07-03, recorded below) and the reasoning is
/// stated so a future failure is a kill-criterion-2 event, not a knob:
///
/// 1. **Invariants, gated at f64 dust (< 1e-9 GWh):** the total, the
///    diurnal band, the inter-annual band, and the synoptic+seasonal
///    aggregate. Every smoothing level is an MA of the ORIGINAL
///    residual (not a re-smoothed cascade), so none of these depends
///    on the synoptic window — measured shifts were exactly 0.0. These
///    carry the two headline claims: "the diurnal quarter is the
///    cycling toll" and "~75 % of the requirement lives at
///    synoptic-and-slower timescales".
/// 2. **The adjacent synoptic↔seasonal trade, gated at 10 % of
///    total:** moving the boundary reclassifies genuine 10–21-day
///    content between the two bands it separates — measured 2,496 GWh
///    (4.27 %) for 14 d → 10 d and 3,840 GWh (6.57 %) for 14 d → 21 d.
///    This is the definition moving, not noise (nothing leaks to
///    non-adjacent bands — part 1), but it must stay small enough that
///    the invariant aggregates dominate the story; 10 % ≈ 1.5× the
///    measured worst.
/// 3. **No leakage:** part 1 already implies it; asserted per band.
///
/// REPORTED LIMITATION (not gated away): at a 21 d synoptic window the
/// synoptic band (22,064 GWh, 37.8 %) overtakes the seasonal band
/// (21,552 GWh, 36.9 %) — the synoptic-vs-seasonal *ranking* is
/// boundary-sensitive and must never be published without stating the
/// window convention. The publishable, window-invariant statements are
/// the diurnal share, the inter-annual share, and the
/// synoptic+seasonal aggregate (43,616 GWh, 74.6 %).
///
/// If this test fails, kill criterion 2 has fired: REPORT it — do not
/// widen the tolerances.
#[test]
fn band_attribution_is_stable_under_synoptic_window_perturbation() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(LEAN_SCENARIO)).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let windows_with_synoptic = |days: f64| DecompositionWindows {
        diurnal: Duration::hours(24.0),
        synoptic: Duration::hours(days * 24.0),
        seasonal: Duration::hours(365.0 * 24.0),
    };

    let attribution_at = |days: f64| {
        attribute_storage_by_band(
            &scenario,
            &inputs,
            0,
            &windows_with_synoptic(days),
            &SolveOptions::default(),
        )
        .unwrap()
    };

    let gwh = |e: Energy| e.as_gigawatt_hours();
    let baseline = attribution_at(14.0);
    let total = gwh(baseline.total);
    const ADJACENT_TRADE_TOLERANCE: f64 = 0.10; // of total; doc comment
    const INVARIANT_TOLERANCE_GWH: f64 = 1e-9;

    for days in [10.0, 21.0] {
        let perturbed = attribution_at(days);

        // Part 1: the window-invariant quantities, at f64 dust.
        let invariants = [
            ("total", gwh(perturbed.total), total),
            (
                "diurnal band",
                gwh(perturbed.bands[0].requirement),
                gwh(baseline.bands[0].requirement),
            ),
            (
                "inter-annual band",
                gwh(perturbed.bands[3].requirement),
                gwh(baseline.bands[3].requirement),
            ),
            (
                "synoptic+seasonal aggregate",
                gwh(perturbed.bands[1].requirement) + gwh(perturbed.bands[2].requirement),
                gwh(baseline.bands[1].requirement) + gwh(baseline.bands[2].requirement),
            ),
        ];
        for (what, at_perturbed, at_baseline) in invariants {
            assert!(
                (at_perturbed - at_baseline).abs() < INVARIANT_TOLERANCE_GWH,
                "KILL CRITERION 2: the {what} must be invariant to the synoptic window \
                 but moved {at_baseline} → {at_perturbed} GWh at {days} d — report, do \
                 not tune",
            );
        }

        // Part 2/3: per-band shifts — the adjacent pair within
        // tolerance, everything else already pinned invariant above.
        for (base, pert) in baseline.bands.iter().zip(&perturbed.bands) {
            let shift = (gwh(pert.requirement) - gwh(base.requirement)).abs();
            eprintln!(
                "  synoptic {days:>4} d: band {:<12} shift {shift:>8.1} GWh \
                 ({:.2} % of total)",
                base.band.as_str(),
                100.0 * shift / total,
            );
            assert!(
                shift < ADJACENT_TRADE_TOLERANCE * total,
                "KILL CRITERION 2: {} attribution moved {shift:.1} GWh \
                 ({:.1} % of the {total:.0} GWh total) when the synoptic window moved \
                 14 d → {days} d — attribution is unstable to filter choice; do NOT \
                 publish the decomposition (docs/08), report instead",
                base.band.as_str(),
                100.0 * shift / total,
            );
        }
    }
}

// ---------------------------------------------------------------------
// (e) Q4 per-year batch: one year or forty?
// ---------------------------------------------------------------------

/// Every weather year 1985–2024 solved as an independent single-year
/// scenario on the RS-lean fleet (Q4, docs/07): 40 deterministic
/// requirements, the distribution's extremes identified.
///
/// PINNED (first measurement, 2026-07-03): all 40 years feasible;
/// design (max) year **2021** at 44,640 GWh — NOT the a-priori
/// expectation of 2010 (docs/07 Q4 anticipated 2010, the record's
/// worst fleet-weighted wind CF year; measured, 2010 comes THIRD at
/// 36,608 GWh behind 1989 at 36,736 GWh — the requirement depends on
/// deficit *timing* against the demand shape, not on annual CF alone;
/// reported as found, kill-criterion-4 posture). Easiest year 1999 at
/// 7,020 GWh — a 6.4× spread across years, which is Q4's teaching
/// point: "modelled on year X" can mean anything within that spread.
///
/// Every single year, started FULL on 1 January (the D4 convention;
/// all 40 flagged initial-condition-sensitive, burn-in impossible on a
/// one-year horizon), sits BELOW the 40-year continuous requirement of
/// 58,432 GWh — even the 2021 design year underestimates it by 24 %:
/// no single-year study, however unlucky its year, reproduces the
/// multi-year record's number (Module 3(b) restated for Q4).
/// Determinism: the batch is re-run serially and must be bit-identical
/// to the rayon run.
#[test]
fn per_year_batch_solves_all_forty_years_and_identifies_the_design_year() {
    require_full_record_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(LEAN_SCENARIO)).unwrap();
    let options = SolveOptions::default();

    let batch = per_year_requirements(
        &scenario,
        &root,
        1985..=2024,
        0,
        &options,
        Execution::Parallel,
    )
    .unwrap();
    assert_eq!(batch.len(), 40);

    // Determinism: an independent serial re-run is bit-identical.
    let rerun = per_year_requirements(
        &scenario,
        &root,
        1985..=2024,
        0,
        &options,
        Execution::Serial,
    )
    .unwrap();
    assert_eq!(
        batch, rerun,
        "per-year batch is not deterministic across executions"
    );

    let mut feasible: Vec<(i32, f64)> = Vec::new();
    for year in &batch {
        match &year.outcome {
            YearOutcome::Feasible { requirement, .. } => {
                eprintln!(
                    "  {}: {:>8.0} GWh",
                    year.year,
                    requirement.as_gigawatt_hours()
                );
                feasible.push((year.year, requirement.as_gigawatt_hours()));
            }
            YearOutcome::Infeasible { reason } => {
                eprintln!("  {}: INFEASIBLE ({reason})", year.year);
            }
        }
    }
    assert_eq!(
        feasible.len(),
        40,
        "some years were infeasible — a finding to report, and a pin to revisit"
    );

    let (max_year, max_gwh) = feasible
        .iter()
        .copied()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    let (min_year, min_gwh) = feasible
        .iter()
        .copied()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    eprintln!(
        "per-year distribution: design year {max_year} ({max_gwh:.0} GWh), easiest \
         {min_year} ({min_gwh:.0} GWh); 40-year continuous requirement \
         {LEAN_PINNED_REQUIREMENT_GWH:.0} GWh"
    );

    // The regression pins (doc comment above; deterministic, ADR-5).
    assert_eq!(
        max_year, 2021,
        "PINNED design year moved (measured 2026-07-03: 2021, ahead of 1989 and 2010)"
    );
    assert!(
        (max_gwh - 44_640.0).abs() < 1e-9,
        "PINNED design-year requirement moved: measured {max_gwh} GWh"
    );
    assert_eq!(min_year, 1999, "PINNED easiest year moved");
    assert!(
        (min_gwh - 7_020.0).abs() < 1e-9,
        "PINNED easiest-year requirement moved: measured {min_gwh} GWh"
    );
    // The Q4 lesson, asserted: every single year — started full, even
    // the design year — underestimates the 40-year continuous
    // requirement.
    assert!(
        max_gwh < LEAN_PINNED_REQUIREMENT_GWH,
        "the worst single year ({max_gwh} GWh) now MEETS/EXCEEDS the 40-year \
         requirement ({LEAN_PINNED_REQUIREMENT_GWH} GWh) — the Module 3(b) contrast \
         inverted; investigate before quoting"
    );
}

// ---------------------------------------------------------------------
// Cross-check: the attribution's synthetic bare-residual replay
// reproduces the engine's arithmetic (documented assumption of (c)).
// ---------------------------------------------------------------------

/// The bare-residual solve (no fleet, residual fed as demand) must
/// reproduce the real-scenario solve exactly: same bisection lattice,
/// same feasibility flips. Run on a single-year variant of the benign
/// scenario (cheap) rather than the 40-year record: thermal stripped
/// (attribution is defined for fleets where storage faces the raw
/// residual — the module docs), weather capacity scaled ×1.5 so the
/// year is feasible on annual energy, and the battery's power rating
/// raised to 100 GW so the solve is energy-limited, not power-limited.
#[test]
fn bare_residual_replay_reproduces_the_real_scenario_solve() {
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join(BENIGN_SCENARIO)).unwrap();
    let zone = &mut scenario.zones[0];
    zone.fleet
        .retain(|entry| entry.capacity_factor_trace.is_some());
    for entry in &mut zone.fleet {
        entry.capacity_gw = entry.capacity_gw * 1.5;
    }
    zone.storage[0].power_gw = Power::gigawatts(100.0);
    let inputs = load_run_inputs(&scenario, &root).unwrap();

    let real = min_storage_for_zero_unserved(&scenario, &inputs, 0, &SolveOptions::default())
        .unwrap()
        .naive
        .requirement;

    // The attribution total is M(unfiltered residual) — the replay.
    let attribution = attribute_storage_by_band(
        &scenario,
        &inputs,
        0,
        &DecompositionWindows::standard(),
        &SolveOptions::default(),
    )
    .unwrap();

    assert_eq!(
        attribution.total,
        real,
        "bare-residual replay diverged from the real-scenario solve: {} vs {} GWh",
        attribution.total.as_gigawatt_hours(),
        real.as_gigawatt_hours()
    );
}
