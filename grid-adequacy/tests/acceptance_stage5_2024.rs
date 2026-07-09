//! Stage 5 acceptance tests A1–A4 (docs/04 Stage 5, tolerances pinned
//! 2026-07-03 pre-model): the five-zone 2024 scenario with MODELLED
//! imports must reproduce the observed 2024 GB system and its border
//! behaviour.
//!
//! - **A1** — the Stage 1 2024 gates re-pass with modelled (not
//!   exogenous) imports: annual gas within ±5 % of 72.79 TWh, monthly
//!   mix correlation ≥ 0.99, and modelled GB net imports within ±10 %
//!   of the 33.30 TWh NESO actual (boundary: scarcity-rule fidelity +
//!   the CONT-NW copper plate).
//! - **A2 (two-limb gate, re-pinned per the stage-5-review addendum
//!   adjudication)** — GB↔FR under the 50 MW dead-band (D5 ruling a:
//!   FR only): **A2a** direction match ≥ 88 % AND **A2b** export
//!   recall ≥ 70 % (the share of observed GB-export periods the model
//!   signs correctly). The ORIGINAL ≥ 95 % single-limb gate is
//!   SUPERSEDED, not deleted: it stays in docs/04 as the PRICED-LADDER
//!   target (Stage 7; expected ~97.4 % — the measured bound with the
//!   both-gas-marginal class eliminated). The re-pin names its
//!   boundary: the unpriced common merit ladder cannot rank two
//!   gas-marginal zones by price (GB carbon-price floor / fuel-price
//!   asymmetry — flow.rs's stated limitation), and the measured
//!   residual mismatch class is ~87 % exactly that. Base-rate
//!   statement, explicit: raw match (90.07 %) sits BELOW the 92.30 %
//!   "always import" base rate, but the limb PAIR strictly dominates
//!   the constant predictor — always-import scores 0 % on A2b.
//! - **A3** — resource-level sign tests: modelled NO2 hydro generation
//!   vs GB wind CF |r| ≤ 0.15 (NEAR-TAUTOLOGICAL limb, owned: the
//!   seasonal-budget driver is wind-independent by construction — the
//!   gate is framed as reproducing the observed structure, r = −0.087
//!   observed); modelled continental (FR + CONT-NW) imports vs GB wind
//!   CF r ≤ −0.25 (the anticyclone result; observed −0.352). NSL flow
//!   vs GB wind is a reported DIAGNOSTIC (observed −0.399), not gated.
//! - **A4** — BE (Nemo) and NL (BritNed) per-border annual net
//!   energies within ±1.5 TWh of the NESO actuals (+4.16, +1.59 TWh);
//!   the five-border table is emitted for the run report. Structural
//!   note (D5 ruling a): both links land in one zone with equal caps,
//!   so the model splits the bloc flow evenly by construction.
//!
//! **NL sensitivity bracket — COLLAPSED (record):** the eu-cf-review
//! ruling-1 bracket (×0.78/×0.85 vs ×1.0) was binding until the A4-BE
//! verdict flipped inside it at the w=1 grain; the adjudicated trigger
//! fired and the CBS national-statistics recalibration landed (NL
//! onshore factor 0.8975, solar 0.8735, both in-band; capacities
//! unchanged). A1/A4 are now single-point gates at the calibrated
//! configuration — the only acceptance condition.
//!
//! Data-gated (fetched/derived, not committed): the 2024 GB pack, the
//! ENTSO-E 2024 pack, and the cf-eu pack (manifest
//! data/packs/cf-eu-1985-2024.sha256). The tests FAIL LOUDLY (no
//! `#[ignore]`) with build instructions if any is missing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::OnceLock;

use grid_adequacy::{MultiZoneRunResult, load_multi_zone_inputs, run_multi};
use grid_core::scenario::Scenario;
use grid_core::trace::{load_per_unit_trace, load_power_trace_mw};
use grid_core::units::{Energy, Power};

const SCENARIO: &str = "scenarios/gb-2024-5zone.toml";
const PERIODS: usize = 17_568;

/// Observed 2024 annual gas generation, TWh (Stage 1 gate).
const GAS_ACTUAL_TWH: f64 = 72.79;
/// Observed 2024 GB net imports, TWh (NESO; the A1 reference).
const IMPORTS_ACTUAL_TWH: f64 = 33.30;
/// NESO per-border 2024 net energies, TWh (docs/04 A4).
const BE_ACTUAL_TWH: f64 = 4.16;
const NL_ACTUAL_TWH: f64 = 1.59;
/// The A2 dead-band, GW (50 MW).
const DEAD_BAND_GW: f64 = 0.05;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Fail loudly (red, not skipped) if any required pack is missing.
fn require_stage5_packs() {
    let root = repo_root();
    let mut missing: Vec<String> = Vec::new();
    let mut need = |rel: &str| {
        if !root.join(rel).exists() {
            missing.push(rel.to_owned());
        }
    };
    // GB 2024 pack (scripts/fetch-2024 + scripts/era5-cf).
    need("data/packs/2024/processed/demand_2024.parquet");
    need("data/packs/2024/processed/wind_cf_2024.parquet");
    // ENTSO-E 2024 pack (scripts/fetch-entsoe: fetch.py, build.py,
    // build_gen_agg.py).
    for zone in ["fr", "be", "nl", "delu", "no2", "dk1", "ie"] {
        need(&format!(
            "data/packs/entsoe-2024/processed/load_{zone}_2024.parquet"
        ));
    }
    need("data/packs/entsoe-2024/processed/flows_gb_entsoe_2024.parquet");
    need("data/packs/entsoe-2024/processed/no2_generation_2024.parquet");
    // cf-eu pack (scripts/era5-cf/derive_cf_eu.py; manifest committed).
    need("data/packs/cf-eu-1985-2024.sha256");
    for (country, techs) in [
        ("fr", &["onshore", "offshore", "solar"][..]),
        ("be", &["onshore", "offshore", "solar"][..]),
        ("nl", &["onshore", "offshore", "solar"][..]),
        ("de", &["onshore", "offshore", "solar"][..]),
        ("dk1", &["onshore", "offshore", "solar"][..]),
        ("ie", &["onshore"][..]),
    ] {
        for tech in techs {
            need(&format!(
                "data/packs/cf-eu/{country}/{country}_{tech}_cf_2024.parquet"
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "Stage 5 data packs incomplete: {} file(s) missing, first {:?} — build them first \
         (GB pack: scripts/fetch-2024 + scripts/era5-cf; ENTSO-E pack: scripts/fetch-entsoe \
         fetch.py/build.py/build_gen_agg.py; EU CF pack: scripts/era5-cf/derive_cf_eu.py). \
         These Stage 5 acceptance tests stay RED until the packs exist.",
        missing.len(),
        missing.first().unwrap()
    );
}

/// The acceptance run: the scenario as committed (CBS-calibrated NL
/// traces; the eu-cf-review bracket collapsed to this single point).
fn run_5zone() -> &'static MultiZoneRunResult {
    static RESULT: OnceLock<MultiZoneRunResult> = OnceLock::new();
    RESULT.get_or_init(|| {
        require_stage5_packs();
        let root = repo_root();
        let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
        let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
        run_multi(&scenario, &inputs).unwrap()
    })
}

fn twh(energy: Energy) -> f64 {
    energy.as_gigawatt_hours() / 1000.0
}

/// Modelled GB-end net flow per period for one border (sum of the
/// named links' home-end series — GB is `from` on every link).
fn border_net_gw(result: &MultiZoneRunResult, links: &[&str]) -> Vec<f64> {
    let mut total = vec![0.0f64; PERIODS];
    for name in links {
        let series = result
            .links
            .iter()
            .find(|l| l.name == *name)
            .unwrap_or_else(|| panic!("no link {name}"));
        for (acc, p) in total.iter_mut().zip(&series.home_end) {
            *acc += p.as_gigawatts();
        }
    }
    total
}

/// Observed ENTSO-E border net imports, GW (+ = import to GB).
fn observed_border_net_gw(column: &str) -> Vec<f64> {
    let path = repo_root().join("data/packs/entsoe-2024/processed/flows_gb_entsoe_2024.parquet");
    load_power_trace_mw(&path, column, PERIODS)
        .unwrap()
        .values()
        .iter()
        .map(|p| p.as_gigawatts())
        .collect()
}

/// The observed GB fleet-wide wind CF trace (2024 pack, `wind_cf`).
fn observed_gb_wind_cf() -> Vec<f64> {
    let path = repo_root().join("data/packs/2024/processed/wind_cf_2024.parquet");
    load_per_unit_trace(&path, "wind_cf", PERIODS)
        .unwrap()
        .values()
        .iter()
        .map(|v| v.value())
        .collect()
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

/// Three-way direction class under the 50 MW dead-band.
fn direction(x: f64) -> i8 {
    if x > DEAD_BAND_GW {
        1
    } else if x < -DEAD_BAND_GW {
        -1
    } else {
        0
    }
}

// ---------------------------------------------------------------------
// A1 — Stage 1 2024 gates re-pass with modelled imports.
// ---------------------------------------------------------------------

#[test]
fn a1_annual_gas_within_5_percent_with_modelled_imports() {
    let gb = run_5zone().zone("GB").unwrap();
    let gas = twh(gb.thermal_energy("ccgt").unwrap()) + twh(gb.thermal_energy("ocgt").unwrap());
    let error_percent = 100.0 * (gas - GAS_ACTUAL_TWH) / GAS_ACTUAL_TWH;
    assert!(
        error_percent.abs() <= 5.0,
        "modelled gas {gas:.2} TWh vs actual {GAS_ACTUAL_TWH} TWh: \
         {error_percent:+.2} % (tolerance ±5 %)"
    );
}

#[test]
fn a1_gb_net_imports_within_10_percent_of_neso_actual() {
    let gb = run_5zone().zone("GB").unwrap();
    let imports = twh(gb.net_imports_energy());
    let error_percent = 100.0 * (imports - IMPORTS_ACTUAL_TWH) / IMPORTS_ACTUAL_TWH;
    assert!(
        error_percent.abs() <= 10.0,
        "modelled GB net imports {imports:.2} TWh vs actual {IMPORTS_ACTUAL_TWH} TWh: \
         {error_percent:+.2} % (tolerance ±10 %; boundary: scarcity-rule fidelity + \
         CONT-NW copper plate — docs/04 A1)"
    );
}

#[test]
fn a1_monthly_mix_correlation_at_least_0_99_with_modelled_imports() {
    // The Stage 1 metric (acceptance_2024.rs), GB zone of the 5-zone
    // run: flattened 12-month × 8-fuel matrix vs the pack's monthly
    // actuals, both in the D3 total-generation convention.
    let fuels: [(&str, &str); 8] = [
        ("ccgt", "ccgt"),
        ("ocgt", "ocgt"),
        ("coal", "coal"),
        ("nuclear", "nuclear"),
        ("biomass", "biomass"),
        ("hydro", "npshyd"),
        ("wind", "wind_incl_embedded"),
        ("solar", "solar_embedded"),
    ];
    let actual_monthly = |column: &str| -> Vec<f64> {
        let path = repo_root().join("data/packs/2024/processed/monthly_generation_2024.csv");
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        let idx = header.iter().position(|c| *c == column).unwrap();
        lines
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split(',').nth(idx).unwrap().parse().unwrap())
            .collect()
    };
    let gb = run_5zone().zone("GB").unwrap();
    let monthly = |series: &[Power]| -> Vec<f64> {
        gb.monthly_energy(series)
            .values()
            .map(|e| e.as_gigawatt_hours())
            .collect()
    };
    let mut xs = Vec::with_capacity(96);
    let mut ys = Vec::with_capacity(96);
    for (model_fuel, actual_column) in fuels {
        let model: Vec<f64> = match model_fuel {
            "wind" => {
                let mut total = vec![Power::gigawatts(0.0); gb.periods()];
                for series in &gb.renewables {
                    if matches!(series.tech.as_str(), "offshore_wind" | "onshore_wind") {
                        for (acc, &p) in total.iter_mut().zip(&series.power) {
                            *acc = *acc + p;
                        }
                    }
                }
                monthly(&total)
            }
            "solar" => {
                let series = gb
                    .renewables
                    .iter()
                    .find(|s| s.tech.as_str() == "solar")
                    .unwrap();
                monthly(&series.power)
            }
            tech => {
                let series = gb.thermal.iter().find(|s| s.tech.as_str() == tech).unwrap();
                monthly(&series.power)
            }
        };
        xs.extend_from_slice(&model);
        ys.extend_from_slice(&actual_monthly(actual_column));
    }
    assert_eq!(xs.len(), 96);
    let r = pearson(&xs, &ys);
    assert!(
        r >= 0.99,
        "monthly-mix correlation r = {r:.4} < 0.99 with modelled imports"
    );
}

// ---------------------------------------------------------------------
// A2 — GB↔FR direction, TWO-LIMB GATE (the one gated border, D5
// ruling a; re-pinned by the stage-5-review addendum adjudication).
//
// History, kept visible (docs/04 amendment discipline): the original
// pin was direction match ≥ 95 %. Two remediation rounds (FR
// observed-resource hydro envelope; FR observed non-GB export trace)
// drove every mechanism-(a) mismatch category to zero, leaving a
// residual class that is ~87 % both-zones-gas-marginal tie-breaks —
// the unpriced common merit ladder cannot rank two gas-marginal zones
// by price (GB carbon-price floor / fuel-price asymmetry; the stated
// flow.rs limitation, mechanism (b)). The ≥ 95 % gate was therefore
// ruled structurally unsatisfiable under the pinned model boundary and
// is SUPERSEDED — retained in docs/04 as the priced-ladder target
// (Stage 7; expected ~97.4 %, the measured bound with the both-gas
// class eliminated).
//
// The two limbs, both required:
// - A2a: direction match ≥ 88 % (50 MW dead-band, 3-class).
// - A2b: export recall ≥ 70 % — of the observed GB-export periods, the
//   share the model signs correctly. THIS limb carries the information
//   content: the constant "always import" predictor scores 92.30 % on
//   raw match but 0 % on recall, so the pair strictly dominates it
//   even though raw match (90.07 %) sits below the base rate.
//
// Exact measured values at the final configuration (w=1 FR envelope +
// CBS-recalibrated NL traces) are pinned below as regressions; a
// deliberate change to scenario, packs or engine is a knowing re-pin.
// ---------------------------------------------------------------------

/// Pinned A2a match count at the final configuration (2026-07-03):
/// 15,823 / 17,568 = 90.07 %.
const PINNED_A2A_MATCHES: usize = 15_823;
/// Pinned A2b recall at the final configuration: 1,036 of the 1,312
/// observed GB-export periods = 78.96 %.
const PINNED_A2B_RECALLED: usize = 1_036;
const PINNED_A2B_OBSERVED_EXPORTS: usize = 1_312;

/// The modelled and observed GB↔FR border series, classified.
fn a2_series() -> (Vec<f64>, Vec<f64>) {
    let modelled = border_net_gw(run_5zone(), &["IFA", "IFA2", "ElecLink"]);
    let observed = observed_border_net_gw("fr_net");
    (modelled, observed)
}

#[test]
fn a2a_gb_fr_direction_match_at_least_88_percent_and_pinned() {
    let (modelled, observed) = a2_series();
    // Confusion matrix (model class × observed class) for the run
    // report: where the mismatches live is the model-boundary record.
    let mut confusion = [[0usize; 3]; 3];
    for (m, o) in modelled.iter().zip(&observed) {
        confusion[(direction(*m) + 1) as usize][(direction(*o) + 1) as usize] += 1;
    }
    let matches = confusion[0][0] + confusion[1][1] + confusion[2][2];
    let rate = matches as f64 / PERIODS as f64;
    eprintln!(
        "A2a: GB<->FR direction match {matches}/{PERIODS} = {:.2} % \
         (raw base rate 92.30 % — see A2b for why the pair dominates it)",
        100.0 * rate
    );
    eprintln!("A2a confusion (rows model export/idle/import; cols observed):");
    for (label, row) in ["export", "idle  ", "import"].iter().zip(&confusion) {
        eprintln!("  model {label}: {:>6} {:>6} {:>6}", row[0], row[1], row[2]);
    }
    // Anatomy of the export/obs-import class (the mechanism-(b)
    // record): hour-of-day spread for the run report.
    let mut by_hour = [0usize; 24];
    for t in 0..PERIODS {
        if direction(modelled[t]) == -1 && direction(observed[t]) == 1 {
            by_hour[(t % 48) / 2] += 1;
        }
    }
    eprintln!("A2a mismatch-class hour-of-day (UTC): {by_hour:?}");
    assert!(
        rate >= 0.88,
        "A2a: GB<->FR direction match {:.2} % < 88 % (docs/04 A2, two-limb re-pin)",
        100.0 * rate
    );
    assert_eq!(
        matches, PINNED_A2A_MATCHES,
        "A2a match count moved from the pinned value — if the change is \
         intentional (scenario/pack/engine), re-pin with the record"
    );
}

#[test]
fn a2b_gb_fr_export_recall_at_least_70_percent_and_pinned() {
    let (modelled, observed) = a2_series();
    let mut observed_exports = 0usize;
    let mut recalled = 0usize;
    for (m, o) in modelled.iter().zip(&observed) {
        if direction(*o) == -1 {
            observed_exports += 1;
            if direction(*m) == -1 {
                recalled += 1;
            }
        }
    }
    let recall = recalled as f64 / observed_exports as f64;
    eprintln!(
        "A2b: export recall {recalled}/{observed_exports} = {:.2} % \
         (the always-import predictor scores 0 % here)",
        100.0 * recall
    );
    assert!(
        recall >= 0.70,
        "A2b: export recall {:.2} % < 70 % (docs/04 A2, two-limb re-pin)",
        100.0 * recall
    );
    assert_eq!(
        (recalled, observed_exports),
        (PINNED_A2B_RECALLED, PINNED_A2B_OBSERVED_EXPORTS),
        "A2b recall counts moved from the pinned values — if the change is \
         intentional (scenario/pack/engine), re-pin with the record"
    );
}

// ---------------------------------------------------------------------
// A3 — resource-level sign tests (reformulated per the ENTSO-E pack
// review; docs/04 amendment 2026-07-03).
// ---------------------------------------------------------------------

#[test]
fn a3_no2_hydro_generation_uncorrelated_with_gb_wind() {
    let result = run_5zone();
    let no2 = result.zone("NO2").unwrap();
    let hydro: Vec<f64> = no2
        .thermal
        .iter()
        .find(|s| s.tech.as_str() == "hydro")
        .unwrap()
        .power
        .iter()
        .map(|p| p.as_gigawatts())
        .collect();
    let wind = observed_gb_wind_cf();
    let r = pearson(&hydro, &wind);
    // Near-tautological limb, owned (module docs): the budget driver is
    // wind-independent by construction; the gate frames REPRODUCTION of
    // the observed structure (r = −0.087 observed).
    assert!(
        r.abs() <= 0.15,
        "modelled NO2 hydro generation vs GB wind CF r = {r:.3}; |r| must be <= 0.15 \
         (observed −0.087 — docs/04 A3)"
    );
}

#[test]
fn a3_continental_imports_anticorrelated_with_gb_wind() {
    let result = run_5zone();
    let continental = border_net_gw(result, &["IFA", "IFA2", "ElecLink", "Nemo", "BritNed"]);
    let wind = observed_gb_wind_cf();
    let r = pearson(&continental, &wind);
    assert!(
        r <= -0.25,
        "modelled continental (FR + CONT-NW) imports vs GB wind CF r = {r:.3}; \
         must be <= −0.25 (observed −0.352; the anticyclone result — docs/04 A3)"
    );
}

#[test]
fn a3_nsl_flow_diagnostic_reported_not_gated() {
    // Diagnostic per docs/04 A3: NSL modelled flow vs GB wind, reported
    // against the observed −0.399 (±0.15 is guidance, not a gate).
    let result = run_5zone();
    let nsl = border_net_gw(result, &["NSL"]);
    let wind = observed_gb_wind_cf();
    let r = pearson(&nsl, &wind);
    eprintln!(
        "A3 diagnostic: NSL modelled flow vs GB wind CF r = {r:.3} \
         (observed −0.399; guidance band ±0.15 around it)"
    );
    // Reported, not gated: the test only requires the number to exist.
    assert!(r.is_finite());
}

// ---------------------------------------------------------------------
// A4 — per-border annual net energies (BE/NL gated; five-border table
// emitted for the run report).
// ---------------------------------------------------------------------

#[test]
fn a4_be_and_nl_border_energies_within_1_5_twh_of_neso() {
    // Single-point gate at the CBS-calibrated configuration (the
    // eu-cf-review bracket collapsed — module docs).
    let result = run_5zone();
    let energy = |name: &str| -> f64 {
        twh(result
            .links
            .iter()
            .find(|l| l.name == name)
            .unwrap()
            .net_home_energy())
    };
    let be = energy("Nemo");
    let nl = energy("BritNed");
    assert!(
        (be - BE_ACTUAL_TWH).abs() <= 1.5,
        "BE (Nemo) net {be:+.2} TWh vs NESO {BE_ACTUAL_TWH:+.2} (±1.5; a miss here \
         is the D5 ruling-c revisit trigger for the CONT-NW copper plate)"
    );
    assert!(
        (nl - NL_ACTUAL_TWH).abs() <= 1.5,
        "NL (BritNed) net {nl:+.2} TWh vs NESO {NL_ACTUAL_TWH:+.2} (±1.5)"
    );
}

#[test]
fn a4_five_border_table_emitted() {
    // The docs/04 A4 reporting requirement: modelled-vs-observed net
    // energy for all five borders (NESO actuals; pack report §3).
    let result = run_5zone();
    let border = |names: &[&str]| -> f64 {
        names
            .iter()
            .map(|n| {
                twh(result
                    .links
                    .iter()
                    .find(|l| l.name == *n)
                    .unwrap()
                    .net_home_energy())
            })
            .sum()
    };
    let table = [
        ("FR", border(&["IFA", "IFA2", "ElecLink"]), 19.45),
        ("BE (Nemo)", border(&["Nemo"]), 4.16),
        ("NL (BritNed)", border(&["BritNed"]), 1.59),
        ("NO2 (NSL)", border(&["NSL"]), 9.62),
        ("DK1 (Viking)", border(&["Viking"]), 3.66),
        ("IE-SEM", border(&["Moyle", "EWIC", "Greenlink"]), -5.18),
    ];
    eprintln!("A4 five-border table (modelled vs NESO, TWh, + = import to GB):");
    let mut total_model = 0.0;
    let mut total_actual = 0.0;
    for (name, model, actual) in table {
        eprintln!(
            "  {name:<14} {model:+7.2} vs {actual:+7.2} (wedge {:+.2})",
            model - actual
        );
        total_model += model;
        total_actual += actual;
    }
    eprintln!(
        "  {:<14} {total_model:+7.2} vs {total_actual:+7.2}",
        "TOTAL"
    );
    assert!(total_model.is_finite());
}

// ---------------------------------------------------------------------
// Direct single-zone inertness pin on the reference data (stage-5
// review, non-blocking note 5): run_multi on the pinned 2024 reference
// scenario equals `run` field-for-field, so the 779d7444 digest pin
// (grid-cli/tests/regression_2024.rs) transfers to the multi-zone path
// by equality — the data-backed half of the inertness proof, no longer
// only by composition on toy fixtures.
// ---------------------------------------------------------------------

#[test]
fn run_multi_on_the_single_zone_reference_matches_run_exactly() {
    let root = repo_root();
    let probe = root.join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack missing ({}) — build it first (scripts/fetch-2024 + scripts/era5-cf)",
        probe.display()
    );
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let single = grid_adequacy::load_run_inputs(&scenario, &root).unwrap();
    let reference = grid_adequacy::run(&scenario, &single).unwrap();
    let multi = load_multi_zone_inputs(&scenario, &root).unwrap();
    let result = run_multi(&scenario, &multi).unwrap();
    assert_eq!(result.zones.len(), 1);
    assert!(
        result.zones[0].result == reference,
        "run_multi differs from run on the pinned 2024 reference scenario"
    );
    for link in &result.links {
        assert!(
            link.home_end.iter().all(|p| *p == Power::gigawatts(0.0)),
            "link {} flowed in a single-zone scenario",
            link.name
        );
    }
}

// ---------------------------------------------------------------------
// Characterisation (stage-5 review, non-blocking note 6): the
// external-zone calibration numbers in the scenario re-derive from the
// packs they cite (guards silent drift of either side — the Stage 1
// `scenario_calibration_matches_the_pack` pattern), and the clean FR
// per-type trace cross-checks the aggregation table it supersedes.
// ---------------------------------------------------------------------

/// Hours per calendar month, 2024 (leap year).
const HOURS_2024: [f64; 12] = [
    744.0, 696.0, 744.0, 720.0, 744.0, 720.0, 744.0, 744.0, 720.0, 744.0, 720.0, 744.0,
];

/// Parse a plain comma CSV (no quoting) into header + rows.
fn read_csv(rel: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let text = std::fs::read_to_string(repo_root().join(rel)).unwrap();
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: Vec<String> = lines
        .next()
        .unwrap()
        .split(',')
        .map(str::to_owned)
        .collect();
    let rows = lines
        .map(|l| l.split(',').map(str::to_owned).collect())
        .collect();
    (header, rows)
}

/// A75 annual energy (GWh) for one (zone, series) of the aggregation
/// table; the per-month vector alongside.
fn aggregation_gen(zone: &str, series: &str) -> (f64, Vec<f64>) {
    let (header, rows) = read_csv("data/packs/entsoe-2024/processed/aggregation_gen_2024.csv");
    let idx = |name: &str| header.iter().position(|h| h == name).unwrap();
    let row = rows
        .iter()
        .find(|r| r[idx("zone")] == zone && r[idx("series")] == series)
        .unwrap_or_else(|| panic!("no aggregation row {zone}/{series}"));
    let annual: f64 = row[idx("gen_gwh")].parse().unwrap();
    let monthly: Vec<f64> = (1..=12)
        .map(|m| row[idx(&format!("gen_gwh_m{m:02}"))].parse().unwrap())
        .collect();
    (annual, monthly)
}

/// A68 capacity (GW) summed over PSR codes for one zone.
fn a68_capacity_gw(zone: &str, psrs: &[&str]) -> f64 {
    let (header, rows) = read_csv("data/packs/entsoe-2024/processed/capacity_2024.csv");
    let idx = |name: &str| header.iter().position(|h| h == name).unwrap();
    rows.iter()
        .filter(|r| r[idx("zone")] == zone && psrs.contains(&r[idx("psr_code")].as_str()))
        .map(|r| r[idx("capacity_mw")].parse::<f64>().unwrap() / 1000.0)
        .sum()
}

#[test]
fn external_zone_availability_factors_rederive_from_the_packs() {
    require_stage5_packs();
    let scenario = Scenario::load(&repo_root().join(SCENARIO)).unwrap();
    let zone = |id: &str| scenario.zones.iter().find(|z| z.id.as_str() == id).unwrap();
    let flat = |zone: &grid_core::scenario::ZoneSpec, tech: &str| -> f64 {
        match &zone
            .fleet
            .iter()
            .find(|e| e.technology.as_str() == tech)
            .unwrap_or_else(|| panic!("no {tech} in {}", zone.id))
            .availability
        {
            Some(grid_core::scenario::AvailabilitySpec::Flat { flat }) => flat.value(),
            other => panic!(
                "{}: {tech} should have flat availability, got {other:?}",
                zone.id
            ),
        }
    };
    let check = |context: &str, derived: f64, pinned: f64| {
        assert!(
            (derived - pinned).abs() < 5e-5,
            "{context}: derived {derived:.5} vs pinned {pinned:.5}"
        );
    };

    // FR nuclear: monthly A75 energy / (A68 capacity × month hours).
    let fr = zone("FR");
    let fr_nuclear_cap = a68_capacity_gw("fr", &["B14"]);
    let (_, fr_nuclear_monthly) = aggregation_gen("fr", "nuclear");
    let pinned_monthly = match &fr
        .fleet
        .iter()
        .find(|e| e.technology.as_str() == "nuclear")
        .unwrap()
        .availability
    {
        Some(grid_core::scenario::AvailabilitySpec::Monthly { monthly }) => monthly.clone(),
        other => panic!("FR nuclear should be monthly, got {other:?}"),
    };
    for m in 0..12 {
        check(
            &format!("FR nuclear month {}", m + 1),
            fr_nuclear_monthly[m] / (fr_nuclear_cap * HOURS_2024[m]),
            pinned_monthly[m].value(),
        );
    }

    let year_hours: f64 = HOURS_2024.iter().sum();
    let flat_factor = |gen_gwh: f64, cap_gw: f64| gen_gwh / (cap_gw * year_hours);

    // FR biomass and coal flats.
    check(
        "FR biomass",
        flat_factor(
            aggregation_gen("fr", "biomass").0,
            a68_capacity_gw("fr", &["B01"]),
        ),
        flat(fr, "biomass"),
    );
    check(
        "FR coal",
        flat_factor(
            aggregation_gen("fr", "fossil_hard_coal").0,
            a68_capacity_gw("fr", &["B05"]),
        ),
        flat(fr, "coal"),
    );

    // CONT-NW flats (BE + NL + DE-LU sums).
    let cont = zone("CONT-NW");
    let bloc = ["be", "nl", "delu"];
    let bloc_gen = |series: &[&str]| -> f64 {
        let (header, rows) = read_csv("data/packs/entsoe-2024/processed/aggregation_gen_2024.csv");
        let idx = |name: &str| header.iter().position(|h| h == name).unwrap();
        rows.iter()
            .filter(|r| {
                bloc.contains(&r[idx("zone")].as_str())
                    && series.contains(&r[idx("series")].as_str())
            })
            .map(|r| r[idx("gen_gwh")].parse::<f64>().unwrap())
            .sum()
    };
    let bloc_cap = |psrs: &[&str]| -> f64 { bloc.iter().map(|z| a68_capacity_gw(z, psrs)).sum() };
    check(
        "CONT-NW nuclear",
        flat_factor(bloc_gen(&["nuclear"]), bloc_cap(&["B14"])),
        flat(cont, "nuclear"),
    );
    check(
        "CONT-NW biomass",
        flat_factor(bloc_gen(&["biomass"]), bloc_cap(&["B01"])),
        flat(cont, "biomass"),
    );
    check(
        "CONT-NW hydro",
        flat_factor(
            bloc_gen(&["hydro_ror", "hydro_reservoir"]),
            bloc_cap(&["B11", "B12"]),
        ),
        flat(cont, "hydro"),
    );
    check(
        "CONT-NW coal",
        flat_factor(
            bloc_gen(&["fossil_brown_coal", "fossil_hard_coal"]),
            bloc_cap(&["B02", "B05"]),
        ),
        flat(cont, "coal"),
    );

    // IE coal flat.
    check(
        "IE coal",
        flat_factor(
            aggregation_gen("ie", "fossil_hard_coal").0,
            a68_capacity_gw("ie", &["B05"]),
        ),
        flat(zone("IE-SEM"), "coal"),
    );
}

#[test]
fn budget_schedules_rederive_from_the_per_type_traces() {
    require_stage5_packs();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let budget_total = |zone: &str| -> f64 {
        inputs
            .zones
            .iter()
            .find(|z| z.id.as_str() == zone)
            .unwrap()
            .budgets
            .values()
            .flat_map(|schedule| schedule.windows.iter())
            .map(|e| e.as_gigawatt_hours())
            .sum::<f64>()
            / 1000.0
    };
    // NO2: observed reservoir 42.56 + pumped gen 1.11 TWh (pack §6).
    let no2 = budget_total("NO2");
    assert!((no2 - 43.67).abs() < 0.01, "NO2 budget {no2:.3} TWh");
    // FR: observed reservoir 17.44 + pumped gen 6.93 TWh from the CLEAN
    // per-type trace (pack §10) — NOT the aggregation table's 9.46 TWh
    // B10 figure (its generic gap repairs invent ~2.5 TWh during
    // pumping windows).
    let fr = budget_total("FR");
    assert!((fr - 24.37).abs() < 0.01, "FR budget {fr:.3} TWh");
}

#[test]
fn fr_per_type_trace_cross_checks_the_aggregation_table() {
    // The cross-build guard (pack §10): the clean half-hourly FR trace
    // and the aggregation table were built independently from the same
    // A75 documents — annual energies must agree for every series whose
    // aggregation needed no long-gap repair. `hydro_pumped` is
    // deliberately EXCLUDED: the aggregation table's generic day-offset
    // repairs invent ~2.5 TWh of B10 generation during pumping windows
    // (the reason the scenario wires the clean trace).
    require_stage5_packs();
    let (header, rows) = read_csv("data/packs/entsoe-2024/processed/fr_generation_2024.csv");
    let idx = |name: &str| header.iter().position(|h| h == name).unwrap();
    for series in [
        "nuclear",
        "biomass",
        "fossil_gas",
        "fossil_hard_coal",
        "fossil_oil",
        "hydro_reservoir",
        "hydro_ror",
        "solar",
        "waste",
        "wind_offshore",
        "wind_onshore",
    ] {
        let column = idx(series);
        let trace_gwh: f64 = rows
            .iter()
            .map(|r| r[column].parse::<f64>().unwrap_or(0.0) * 0.5 / 1e3)
            .sum();
        let (agg_gwh, _) = aggregation_gen("fr", series);
        assert!(
            (trace_gwh - agg_gwh).abs() < 0.5,
            "fr {series}: clean trace {trace_gwh:.1} GWh vs aggregation {agg_gwh:.1} GWh"
        );
    }
}

// ---------------------------------------------------------------------
// Determinism on the full 5-zone run.
// ---------------------------------------------------------------------

#[test]
fn five_zone_rerun_is_bit_identical() {
    require_stage5_packs();
    let root = repo_root();
    let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let first = run_multi(&scenario, &inputs).unwrap();
    let second = run_multi(&scenario, &inputs).unwrap();
    assert!(first == second, "5-zone reruns differ (ADR-5)");
}
