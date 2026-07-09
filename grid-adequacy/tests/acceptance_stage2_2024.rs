//! Stage 2 acceptance tests (docs/04 Stage 2, tolerances pinned
//! 2026-07-02 from `docs/notes/2024-price-pack-report.md`).
//!
//! The 2024 reference run (Stage 1 dispatch, frozen — its pinned digest
//! must not move) plus the Stage 2 pricing layer must reproduce:
//!
//! 1. **Gas price-setting share**: % of periods with gas (CCGT/OCGT)
//!    flagged price-setting by the model within the observable's own
//!    definition band **[89.4 %, 99.8 %]** — docs/04 Stage 2 as
//!    re-pinned 2026-07-02 after the first Stage 2 run (re-pin record in
//!    docs/04: the original ±3-points-of-99.4 % gate was mis-pinned and
//!    unsatisfiable by dispatch arithmetic — the frozen Stage 1 engine
//!    dispatches zero gas in 6.11 % of 2024 periods, capping the model's
//!    share at 93.89 % — and jointly unsatisfiable with the
//!    capture-ratio gate; `docs/notes/stage-2-2024-run-report.md`).
//!    docs/04 also corrects the "~97 %" claim: on this model gas sets
//!    the price in ≈94 % of periods and is price-consistent in only
//!    ≈64 % — both framings are reported by `grid-cli run`.
//! 2. **Wind capture ratio** (model SMP, model wind) within **±0.05 of
//!    the observed 0.899** (MID price, D3 total wind). Context for the
//!    width: the modelled-wind weighting wedge alone is −0.023 (the
//!    ERA5-weighted observed benchmark is 0.875), price-series choice
//!    +0.005, monthly spread ±0.07 — pack report §5.
//! 3. **Model-price realism, gated** (promoted from reported-only,
//!    2026-07-02 reviewer ruling — docs/04 Stage 2; both statistics
//!    carry genuine model content): median model-SMP/observed-MID
//!    within **[0.90, 1.10]**; monthly model-vs-observed price
//!    correlation ≥ **0.85**. Their presence in the run summary is
//!    asserted by the grid-cli tests.
//!
//! Plus recipe pins: the model CCGT/OCGT SRMC series reproduce the pack
//! report §3 annual means; the emissions totals respect the CO₂-vs-CO₂e
//! factor split; and the model's own 5–95 flexing statistic is pinned as
//! a documented-boundary metric (not gated).
//!
//! These tests need the locally built 2024 data pack (git-ignored;
//! fetched, not committed) and fail loudly if it is absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{
    PricingInputs, PricingResult, RunResult, load_pricing_inputs, load_run_inputs, price_run, run,
};
use grid_core::pricing::{capture_ratio, price_setting_share};
use grid_core::scenario::Scenario;
use grid_core::units::Power;

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
         scripts/fetch-2024 (fetch.py, build.py), scripts/era5-cf and \
         scripts/fetch-prices",
        probe.display()
    );
}

/// Run the 2024 reference dispatch and price it.
fn priced_2024() -> (RunResult, PricingInputs, PricingResult) {
    require_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let pricing_spec = scenario
        .pricing
        .as_ref()
        .expect("the 2024 reference scenario declares a [pricing] block (schema v2)");
    let pricing_inputs = load_pricing_inputs(&scenario, pricing_spec, &root).unwrap();
    let result = run(&scenario, &inputs).unwrap();
    let priced = price_run(&result, &pricing_inputs).unwrap();
    (result, pricing_inputs, priced)
}

// ---------------------------------------------------------------------
// Acceptance test 1 (re-pinned 2026-07-02): gas price-setting share
// within the observable's own definition band [89.4 %, 99.8 %] — the
// 10–90 and 3–97 flexing-proxy endpoints of the pack report §4. The
// exact model value (93.89 %) is regression-pinned separately in
// grid-cli/tests/regression_stage2_2024.rs.
// ---------------------------------------------------------------------

#[test]
fn gas_price_setting_share_within_the_observable_definition_band() {
    let (_, _, priced) = priced_2024();
    let share = 100.0 * price_setting_share(&priced.setter, &["ccgt", "ocgt"]);
    assert!(
        (89.4..=99.8).contains(&share),
        "model gas price-setting share {share:.2} % outside [89.4 %, 99.8 %] (docs/04 \
         Stage 2, re-pinned 2026-07-02 — re-pin record there and in \
         docs/notes/stage-2-2024-run-report.md)"
    );
}

// ---------------------------------------------------------------------
// Acceptance test 2 (pinned): wind capture ratio within ±0.05 of the
// observed 0.899.
// ---------------------------------------------------------------------

/// Combined (offshore + onshore) model wind output per period — the D3
/// total-wind convention on the model side.
fn total_wind(result: &RunResult) -> Vec<Power> {
    let series = |tech: &str| -> &[Power] {
        &result
            .renewables
            .iter()
            .find(|s| s.tech.as_str() == tech)
            .unwrap_or_else(|| panic!("no renewable series {tech}"))
            .power
    };
    series("offshore_wind")
        .iter()
        .zip(series("onshore_wind"))
        .map(|(&a, &b)| a + b)
        .collect()
}

#[test]
fn wind_capture_ratio_within_0_05_of_observed() {
    let (result, _, priced) = priced_2024();
    let wind = total_wind(&result);
    let ratio = capture_ratio(&wind, &priced.smp)
        .unwrap()
        .expect("2024 wind output is nonzero");
    println!("model wind capture ratio (D3 total wind, model SMP): {ratio:.4}");
    assert!(
        (ratio - 0.899).abs() <= 0.05,
        "model wind capture ratio {ratio:.4} vs observed 0.899 (tolerance ±0.05; docs/04 \
         Stage 2 pin — context: the modelled-wind weighting wedge alone moves the observed \
         ratio to 0.875, pack report §5)"
    );
}

// ---------------------------------------------------------------------
// Acceptance test 3 (gated; promoted from reported-only 2026-07-02,
// reviewer ruling — docs/04 Stage 2): median model-SMP/observed-MID
// within [0.90, 1.10]; monthly correlation ≥ 0.85.
// ---------------------------------------------------------------------

#[test]
fn median_model_smp_over_observed_mid_within_gate() {
    let (_, _, priced) = priced_2024();
    let realism = priced
        .realism
        .as_ref()
        .expect("the reference [pricing] section declares the observed MID benchmark trace");
    let median = realism.median_model_over_observed;
    println!("median model-SMP/observed-MID = {median:.4}");
    assert!(
        (0.90..=1.10).contains(&median),
        "median model-SMP/observed-MID {median:.4} outside [0.90, 1.10] (docs/04 Stage 2, \
         gated 2026-07-02)"
    );
}

#[test]
fn monthly_model_vs_observed_price_correlation_at_least_0_85() {
    let (_, _, priced) = priced_2024();
    let realism = priced
        .realism
        .as_ref()
        .expect("the reference [pricing] section declares the observed MID benchmark trace");
    let r = realism.monthly_correlation;
    println!("monthly model-vs-observed price correlation = {r:.4}");
    assert!(
        r >= 0.85,
        "monthly model-vs-observed price correlation {r:.4} < 0.85 (docs/04 Stage 2, gated \
         2026-07-02)"
    );
}

// ---------------------------------------------------------------------
// Recipe pins: the per-period SRMC series reproduce the pack report §3
// annual means (CCGT £79.16/MWh, OCGT £110.98/MWh).
// ---------------------------------------------------------------------

#[test]
fn model_srmc_series_reproduce_the_pack_report_annual_means() {
    let (_, inputs, _) = priced_2024();
    let annual_mean = |tech: &str| -> f64 {
        let trace = inputs
            .srmc
            .get(&grid_core::scenario::TechId::new(tech))
            .unwrap_or_else(|| panic!("no SRMC series for {tech}"));
        trace
            .values()
            .iter()
            .map(|p| p.as_pounds_per_megawatt_hour())
            .sum::<f64>()
            / trace.len() as f64
    };
    let ccgt = annual_mean("ccgt");
    assert!(
        (ccgt - 79.16).abs() < 0.02,
        "CCGT SRMC annual mean {ccgt:.3} vs pack report 79.16"
    );
    let ocgt = annual_mean("ocgt");
    assert!(
        (ocgt - 110.98).abs() < 0.02,
        "OCGT SRMC annual mean {ocgt:.3} vs pack report 110.98"
    );
}

// ---------------------------------------------------------------------
// Emissions accounting: CO₂-only (pricing basis) vs CO₂e (accounting
// basis) both carried, labelled, and in the documented ratio.
// ---------------------------------------------------------------------

#[test]
fn emissions_totals_respect_the_co2_vs_co2e_factor_split() {
    let (_, _, priced) = priced_2024();
    let co2 = priced.total_co2.as_tonnes_co2();
    let co2e = priced.total_co2e.as_tonnes_co2();
    assert!(co2 > 0.0, "2024 gas fleet emitted CO2");
    // CO2e/CO2 = 0.18290/0.18253 for every gas technology, hence for the
    // total as well.
    let expected = 0.18290 / 0.18253;
    let observed = co2e / co2;
    assert!(
        (observed - expected).abs() < 1e-9,
        "CO2e/CO2 ratio {observed} vs factor ratio {expected}"
    );
    // Order-of-magnitude sanity: ~73.5 TWh of gas at ≈0.37–0.52
    // tCO2/MWh_e is 25–35 MtCO2.
    assert!(
        (25.0e6..35.0e6).contains(&co2),
        "total gas CO2 {co2} tonnes outside 25–35 Mt"
    );
}

// ---------------------------------------------------------------------
// Revenue conservation on the real run: Σ per-technology revenue equals
// the SMP-weighted total of the same dispatch series (integration-level
// check of the unit-tested property).
// ---------------------------------------------------------------------

#[test]
fn per_technology_revenues_sum_to_smp_weighted_total_dispatch() {
    let (result, _, priced) = priced_2024();
    let sum: f64 = priced
        .technologies
        .iter()
        .map(|t| t.revenue.as_pounds())
        .sum();
    let mut total = vec![Power::gigawatts(0.0); result.periods()];
    for series in result.renewables.iter().chain(&result.thermal) {
        for (acc, &p) in total.iter_mut().zip(&series.power) {
            *acc = *acc + p;
        }
    }
    let direct = grid_core::pricing::revenue(&total, &priced.smp)
        .unwrap()
        .as_pounds();
    assert!(
        ((sum - direct) / direct).abs() < 1e-9,
        "Σ per-tech revenue £{sum:.0} != SMP-weighted total £{direct:.0}"
    );
}

// ---------------------------------------------------------------------
// Documented-boundary pin (not gated; docs/04 Stage 2 re-pin record,
// `docs/notes/stage-2-2024-run-report.md`): the model's own 5–95
// flexing statistic — % of periods with CCGT output strictly between
// 5 % and 95 % of its 2024 model maximum, the like-for-like comparison
// against the observed 99.4 % proxy. The ~14-point gap is the model
// boundary: CCGT minimum-stable generation / part-loading is not
// modelled, so model CCGT parks at zero where real CCGT stays on
// (observed CCGT reached zero in only 9 of 17,568 periods in 2024).
// ---------------------------------------------------------------------

#[test]
fn pinned_model_5_95_flexing_statistic() {
    let (result, _, _) = priced_2024();
    let ccgt = &result
        .thermal
        .iter()
        .find(|s| s.tech.as_str() == "ccgt")
        .expect("no ccgt series")
        .power;
    let max = ccgt.iter().fold(
        Power::gigawatts(0.0),
        |acc, &p| if p > acc { p } else { acc },
    );
    let flexing = ccgt
        .iter()
        .filter(|&&p| p > max * 0.05 && p < max * 0.95)
        .count();
    let share = 100.0 * flexing as f64 / ccgt.len() as f64;
    println!("model 5–95 flexing statistic: {share:.4} % (observed proxy 99.4 %)");
    assert!(
        (share - 85.7013).abs() <= 0.01,
        "model 5–95 flexing statistic {share:.4} % differs from the pinned 85.7013 % \
         (±0.01; boundary metric, docs/notes/stage-2-2024-run-report.md — if the engine \
         change is intentional, update this pin and the run report together)"
    );
}

// ---------------------------------------------------------------------
// Determinism: pricing the same run twice is bit-identical (ADR-5).
// ---------------------------------------------------------------------

#[test]
fn pricing_is_deterministic() {
    let (result, inputs, first) = priced_2024();
    let second = price_run(&result, &inputs).unwrap();
    assert!(first == second, "two pricings of the same run differ");
}
