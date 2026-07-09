//! Stage 7 Q9 acceptance tests: the D8 rule-6a three-wedge identity
//! decomposing the gap between the generation-weighted mean plant-gate
//! LCOE and the rule-1 delivered system cost
//! (`docs/notes/d8-lcoe-methods.md` rule 6a; docs/04 Stage 7 acceptance
//! line "LCOE vs. delivered £/MWh gap fully decomposed (Q9)").
//!
//! Independence structure (the D8 adjudication anti-tautology rule,
//! following the committed `cost_stack.rs` precedent): every wedge and
//! every anchor is recomputed here from the parsed reference numbers
//! and the raw run series using test-local arithmetic (including a
//! test-local CRF) — never by calling the library's wedge, line or
//! annuity functions, and never by subtracting from the total. The
//! library's wedges must (a) match the independent recomputation and
//! (b) close the identity mean-LCOE + Σwedges = headline with no
//! residual term.
//!
//! Exactness convention (stated for the reviewer): the rule-6a identity
//! has no residual term BY CONSTRUCTION — the three wedges telescope to
//! the gap in real arithmetic. Numerically, the four anchors (mean
//! plant-gate LCOE, per-generated cost, per-delivered cost, headline)
//! are independent f64 folds, so the reconstruction is asserted at
//! ≤ 1e-9 relative (the committed reconciliation precedent's bound; the
//! observed residual is f64 dust, orders of magnitude below it), while
//! `gap = headline − mean` is asserted BITWISE against the library's
//! own anchors, and determinism is asserted bitwise.
//!
//! Coverage: a hand-checkable synthetic storage-bearing fixture (every
//! number recomputed in this file) and the 2024 reference scenario
//! (identity closure and quotability/staleness stamps on the real
//! instrument — quarantine propagation is exercised on a re-quarantined
//! in-memory reference since the 2026-07-06 battery lift; needs the
//! locally built packs and fails loudly without them — the audit's
//! standing rule).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use grid_adequacy::costs::{
    CostFraming, CostStackSpec, CostedBattery, CostedGeneration, CostedLink, ServiceHolding,
    StoreVintage, cost_stack, q9_decomposition,
};
use grid_adequacy::pricing::PricingInputs;
use grid_adequacy::result::{RunResult, StoreSeries, TechSeries};
use grid_adequacy::{load_pricing_inputs, load_run_inputs, run};
use grid_core::GridError;
use grid_core::costs::WaccBand;
use grid_core::costs_reference::CostsReference;
use grid_core::scenario::{Reliability, Scenario, StorageKind, TechId};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, PerUnit, Power, Price};

/// Workspace root (reference and scenario paths are repo-relative).
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn reference() -> CostsReference {
    CostsReference::load(&repo_root().join("data/reference/costs-gb.toml")).unwrap()
}

/// The committed reference with the battery row RE-QUARANTINED in
/// memory. The real row's condition-3 quarantine was lifted 2026-07-06
/// as a reviewed act (condition 3.i discharged, NREL/TP-6A40-93281), so
/// the quarantine-propagation machinery is exercised on this
/// synthetically quarantined variant.
fn requarantined_battery_reference() -> CostsReference {
    let text = std::fs::read_to_string(repo_root().join("data/reference/costs-gb.toml")).unwrap();
    let mutated = text.replace(
        "quotable = true                           # condition 3.i",
        "quotable = false # (test mutation) condition 3.i",
    );
    assert_ne!(mutated, text, "battery flag mutation did not apply");
    CostsReference::from_toml_str(&mutated).unwrap()
}

const PERIODS: usize = 48; // one day of 2024 (a 17,568-period leap year)
const YEARS: f64 = 48.0 / 17_568.0; // fraction of calendar 2024 covered

fn start() -> UtcInstant {
    UtcInstant::parse("2024-01-01T00:00:00Z").unwrap()
}

/// A hand-checkable synthetic STORAGE-BEARING day (the cost_stack.rs
/// fixture plus a cycling store): 10 GW flat demand; 3 GW onshore-wind
/// potential; 6 GW CCGT and 1 GW nuclear flat; 0.5 GW unserved in two
/// periods; a battery charging 1 GW for the first 12 periods and
/// discharging 0.9 GW for the next 6. The series are hand-crafted
/// accounting inputs, not a dispatch trace (the committed cost_stack.rs
/// precedent).
fn run_result() -> RunResult {
    let flat = |gw: f64| vec![Power::gigawatts(gw); PERIODS];
    let mut unserved = flat(0.0);
    unserved[10] = Power::gigawatts(0.5);
    unserved[11] = Power::gigawatts(0.5);
    let mut charge = flat(0.0);
    let mut discharge = flat(0.0);
    let mut soc = Vec::with_capacity(PERIODS);
    let mut level = 0.0;
    for (t, (c, d)) in charge.iter_mut().zip(discharge.iter_mut()).enumerate() {
        if t < 12 {
            *c = Power::gigawatts(1.0);
            level += 0.5 * 0.9; // √η charge leg at η ≈ 0.81
        } else if t < 18 {
            *d = Power::gigawatts(0.9);
            level -= 0.9 * 0.5 / 0.9;
        }
        soc.push(Energy::gigawatt_hours(level));
    }
    RunResult {
        start: start(),
        demand: flat(10.0),
        renewables: vec![TechSeries {
            tech: TechId::new("onshore_wind"),
            reliability: Reliability::Variable,
            reliability_overridden: false,
            power: flat(3.0),
        }],
        exogenous: vec![],
        thermal: vec![
            TechSeries {
                tech: TechId::new("ccgt"),
                reliability: Reliability::Firm,
                reliability_overridden: false,
                power: flat(6.0),
            },
            TechSeries {
                tech: TechId::new("nuclear"),
                reliability: Reliability::Firm,
                reliability_overridden: false,
                power: flat(1.0),
            },
        ],
        stores: vec![StoreSeries {
            label: "battery".to_owned(),
            kind: StorageKind::Battery,
            charge,
            discharge,
            soc,
        }],
        curtailment: flat(0.0),
        unserved,
    }
}

/// Pricing inputs carrying one SRMC series (CCGT, flat £50/MWh) — the
/// Stage 2 chain line 2 consumes (D8 rule 1.2).
fn pricing_inputs() -> PricingInputs {
    let srmc_trace = Trace::from_parts(
        start(),
        vec![Price::pounds_per_megawatt_hour(50.0); PERIODS],
    )
    .unwrap();
    let mut srmc = BTreeMap::new();
    srmc.insert(TechId::new("ccgt"), srmc_trace);
    PricingInputs {
        srmc,
        ef_electric_co2: BTreeMap::new(),
        ef_electric_co2e: BTreeMap::new(),
        observed_price: None,
    }
}

/// The costed spec: wind carries 12 GW capacity (realised CF 0.25 vs
/// the source's assumed 0.36 → a positive utilisation wedge); ccgt and
/// nuclear rows publish no load-factor assumption (realised-CF
/// convention, zero utilisation contribution — stated).
fn spec_with_battery() -> CostStackSpec {
    CostStackSpec {
        framing: CostFraming::Greenfield,
        adequacy_standard: "not solved to a standard (synthetic test fixture)".to_owned(),
        generation: vec![
            CostedGeneration {
                tech: "ccgt".to_owned(),
                cost_row: "ccgt".to_owned(),
                capacity: Power::gigawatts(8.0),
            },
            CostedGeneration {
                tech: "onshore_wind".to_owned(),
                cost_row: "onshore_wind".to_owned(),
                capacity: Power::gigawatts(12.0),
            },
            CostedGeneration {
                tech: "nuclear".to_owned(),
                cost_row: "nuclear".to_owned(),
                capacity: Power::gigawatts(1.0),
            },
        ],
        batteries: vec![CostedBattery {
            label: "battery".to_owned(),
            power: Power::gigawatts(1.0),
            energy: Energy::gigawatt_hours(4.0),
            vintage: StoreVintage::Build2030,
        }],
        interconnectors: vec![CostedLink {
            row: "viking_link".to_owned(),
            life_years: 40,
        }],
        holdings: vec![ServiceHolding {
            service: "dynamic_containment_lf".to_owned(),
            held: Power::gigawatts(1.178),
        }],
    }
}

fn spec_without_battery() -> CostStackSpec {
    let mut spec = spec_with_battery();
    spec.batteries.clear();
    spec
}

/// Test-local capital recovery factor — deliberately NOT the library's.
fn crf(rate: f64, life_years: u32) -> f64 {
    let x = (1.0 + rate).powi(life_years as i32);
    rate * x / (x - 1.0)
}

// -----------------------------------------------------------------------
// Test-local independent recomputation, in £ and £/MWh, from the parsed
// reference numbers and the raw series. Never calls a library cost or
// wedge function.
// -----------------------------------------------------------------------

/// Accrued fixed cost (annuity + fom + insurance + connection) of one
/// costed asset over the horizon, £: (capex incl. site infrastructure ×
/// CRF + fixed lines) × MW × YEARS.
fn fixed_ccgt(rate: f64) -> f64 {
    ((1020.0 + 14.6) * 1000.0 * crf(rate, 25) + 22_900.0) * 8_000.0 * YEARS
}
fn fixed_wind(rate: f64) -> f64 {
    ((1380.0 + 315.9) * 1000.0 * crf(rate, 35) + 39_900.0) * 12_000.0 * YEARS
}
fn fixed_nuclear(rate: f64) -> f64 {
    ((5820.0 + 4.7) * 1000.0 * crf(rate, 60) + 111_840.0) * 1_000.0 * YEARS
}

/// Variable costs, £: CCGT 6 GW × 24 h = 144,000 MWh at SRMC £50 + VOM
/// £5; nuclear 24,000 MWh at 6.7 + 6.7 + 2.7 = £16.1/MWh; wind zero.
const VAR_CCGT: f64 = 144_000.0 * 55.0;
const VAR_NUCLEAR: f64 = 24_000.0 * 16.1;

/// Weighting basis G: potential output for the weather-driven wind
/// (3 GW × 24 h), dispatched output for thermal — 72 + 144 + 24 GWh.
const G_MWH: f64 = 240_000.0;
/// Rule-1 denominator E: 240 GWh demand − 0.5 GWh unserved.
const E_MWH: f64 = 239_500.0;
/// Wind utilisation ratio: realised generation over the generation the
/// source's assumed CF implies — 72,000 / (12,000 MW × 0.36 × 24 h).
const R_WIND: f64 = 72_000.0 / 103_680.0;

/// The other rule-1 lines, £ (identical arithmetic to the committed
/// cost_stack.rs fixture): battery power+energy legs, Viking Link, one
/// DC-LF holding, constraint zero.
fn missing_lines(rate: f64, with_battery: bool) -> f64 {
    let storage = if with_battery {
        (262.0 * 1000.0 * crf(rate, 15) * 1_000.0
            + 135.0 * crf(rate, 15) * 4.0e6
            + 12_900.0 * 1_000.0)
            * YEARS
    } else {
        0.0
    };
    let interconnection = 1.7e9 * crf(rate, 40) * YEARS;
    let stability = 1_178.0 * 24.0 * 3.31;
    storage + interconnection + stability + 0.0
}

/// All independent Q9 quantities at one WACC, £/MWh:
/// (mean plant-gate LCOE, utilisation wedge, denominator wedge,
/// missing-line wedge, headline).
fn independent_q9(rate: f64, with_battery: bool) -> (f64, f64, f64, f64, f64) {
    let generation_fixed = fixed_ccgt(rate) + fixed_wind(rate) + fixed_nuclear(rate);
    let generation_cost = generation_fixed + VAR_CCGT + VAR_NUCLEAR;
    // Mean plant-gate LCOE: Σ_i G_i × LCOE_i / G, with the fixed part of
    // each LCOE priced at the SOURCE's assumed CF where the row
    // publishes one (wind 0.36) and at the realised CF otherwise (ccgt,
    // nuclear — contribution C_fix + C_var exactly).
    let l_mean = (fixed_ccgt(rate)
        + VAR_CCGT
        + fixed_nuclear(rate)
        + VAR_NUCLEAR
        + fixed_wind(rate) * R_WIND)
        / G_MWH;
    // Utilisation wedge: Σ_i C_fix_i × (1 − realised/assumed) / G — only
    // wind carries a published CF assumption.
    let w_util = fixed_wind(rate) * (1.0 - R_WIND) / G_MWH;
    // Denominator wedge: the same generation cost per delivered MWh
    // minus per generated MWh.
    let w_denom = generation_cost / E_MWH - generation_cost / G_MWH;
    // Missing-line wedge: the rule-1 lines absent from plant-gate LCOE
    // (storage, interconnection, stability, constraints) per delivered.
    let w_miss = missing_lines(rate, with_battery) / E_MWH;
    let headline = (generation_cost + missing_lines(rate, with_battery)) / E_MWH;
    (l_mean, w_util, w_denom, w_miss, headline)
}

fn assert_close(got: Price, expected: f64, what: &str) {
    let got = got.as_pounds_per_megawatt_hour();
    let scale = expected.abs().max(1.0);
    assert!(
        ((got - expected) / scale).abs() < 1e-9,
        "{what}: got £{got}/MWh, independently recomputed £{expected}/MWh"
    );
}

fn band(b: &WaccBand<Price>) -> [Price; 3] {
    [b.low, b.central, b.high]
}

// ---------------------------------------------------------------------
// The identity, each wedge independently recomputed (synthetic
// storage-bearing fixture).
// ---------------------------------------------------------------------

#[test]
fn q9_wedges_match_the_independent_recomputation() {
    let reference = reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    let rates = [0.045, 0.075, 0.100];
    for (i, rate) in rates.into_iter().enumerate() {
        let (l_mean, w_util, w_denom, w_miss, headline) = independent_q9(rate, true);
        assert_close(
            band(&q9.plant_gate_lcoe_mean)[i],
            l_mean,
            &format!("mean plant-gate LCOE at WACC {rate}"),
        );
        assert_close(
            band(&q9.utilisation_wedge)[i],
            w_util,
            &format!("utilisation wedge at WACC {rate}"),
        );
        assert_close(
            band(&q9.denominator_wedge().0)[i],
            w_denom,
            &format!("denominator wedge at WACC {rate}"),
        );
        assert_close(
            band(&q9.missing_line_wedge)[i],
            w_miss,
            &format!("missing-line wedge at WACC {rate}"),
        );
        assert_close(
            band(&q9.stack.headline_per_mwh_delivered)[i],
            headline,
            &format!("headline at WACC {rate}"),
        );
        assert_close(
            band(&q9.gap)[i],
            headline - l_mean,
            &format!("gap at WACC {rate}"),
        );
    }
}

#[test]
fn q9_identity_closes_with_no_residual_term() {
    let reference = reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    let (denominator_wedge, _) = q9.denominator_wedge();
    for i in 0..3 {
        let l_mean = band(&q9.plant_gate_lcoe_mean)[i].as_pounds_per_megawatt_hour();
        let w_util = band(&q9.utilisation_wedge)[i].as_pounds_per_megawatt_hour();
        let w_denom = band(&denominator_wedge)[i].as_pounds_per_megawatt_hour();
        let w_miss = band(&q9.missing_line_wedge)[i].as_pounds_per_megawatt_hour();
        let headline = band(&q9.stack.headline_per_mwh_delivered)[i].as_pounds_per_megawatt_hour();
        // The identity: mean + the three wedges = headline, no residual
        // term in the construction; reconstruction bound per the module
        // docs (independent f64 folds).
        let reconstructed = l_mean + w_util + w_denom + w_miss;
        assert!(
            ((reconstructed - headline) / headline.abs().max(1.0)).abs() < 1e-9,
            "band point {i}: mean {l_mean} + wedges {w_util}/{w_denom}/{w_miss} \
             reconstructs {reconstructed}, headline {headline}"
        );
        // gap = headline − mean, bitwise against the library's own
        // anchors (the decomposed object is exactly the rule-6 gap).
        assert_eq!(
            band(&q9.gap)[i],
            band(&q9.stack.headline_per_mwh_delivered)[i] - band(&q9.plant_gate_lcoe_mean)[i],
            "band point {i}: gap is not headline − mean"
        );
    }
}

#[test]
fn q9_is_deterministic() {
    let reference = reference();
    let first = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    let second = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert!(first == second, "Q9 decomposition differs between reruns");
}

// ---------------------------------------------------------------------
// Wiring into the cost-stack output: the embedded stack IS the rule-1
// stack, and the stamps travel (rule 3 + rule 4 + quarantine).
// ---------------------------------------------------------------------

#[test]
fn q9_embeds_the_rule_1_cost_stack_and_its_stamps() {
    let reference = reference();
    let result = run_result();
    let pricing = pricing_inputs();
    let spec = spec_with_battery();
    let q9 = q9_decomposition(&result, &pricing, &reference, &spec).unwrap();
    let stack = cost_stack(&result, &pricing, &reference, &spec).unwrap();
    // The embedded stack is exactly the rule-1 stack (bitwise).
    assert!(q9.stack == stack, "embedded stack differs from cost_stack");
    // Rule-3 reliability stamp travels on the artefact.
    assert_eq!(
        q9.stack.metadata.reliability.unserved_energy,
        Energy::gigawatt_hours(0.5)
    );
    // Rule-4 WACC band: the capex-bearing wedges vary across the band.
    assert!(
        q9.utilisation_wedge.low < q9.utilisation_wedge.central
            && q9.utilisation_wedge.central < q9.utilisation_wedge.high,
        "the utilisation wedge carries capex content and must rise with the WACC"
    );
    assert_eq!(q9.stack.metadata.wacc.low, PerUnit::new(0.045));
    assert_eq!(q9.stack.metadata.wacc.central, PerUnit::new(0.075));
    assert_eq!(q9.stack.metadata.wacc.high, PerUnit::new(0.100));
}

#[test]
fn q9_propagates_quarantine_and_refuses_publication() {
    // The MACHINERY test, on the re-quarantined in-memory reference
    // (the committed battery row was lifted 2026-07-06 as a reviewed
    // act): a quarantined consumed row stamps the Q9 artefact
    // non-quotable and the publish path refuses.
    let reference = requarantined_battery_reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert!(!q9.stack.metadata.quotable);
    match q9.ensure_publishable() {
        Err(GridError::NonQuotableResult { reason }) => {
            assert!(reason.contains("battery"), "reason: {reason}");
        }
        other => panic!("expected NonQuotableResult, got {other:?}"),
    }
}

#[test]
fn q9_with_the_lifted_battery_row_is_publishable_and_keeps_the_staleness_stamp() {
    // The new truth on the COMMITTED reference: the battery row is
    // quotable (condition 3.i discharged 2026-07-06), so a
    // battery-bearing Q9 artefact publishes — while caveat 3.iii (the
    // 2018-vintage staleness stamp) still travels.
    let reference = reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert!(q9.stack.metadata.quotable);
    assert!(q9.stack.metadata.consumed_quarantined_rows.is_empty());
    assert!(
        q9.stack
            .metadata
            .staleness_stamps
            .iter()
            .any(|s| s.contains("storage.battery_li_ion") && s.contains("2018")),
        "caveat 3.iii must survive the lift — staleness stamps: {:?}",
        q9.stack.metadata.staleness_stamps
    );
    q9.ensure_publishable().unwrap();
}

#[test]
fn q9_without_quarantined_rows_is_publishable() {
    let reference = reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_without_battery(),
    )
    .unwrap();
    assert!(q9.stack.metadata.quotable);
    q9.ensure_publishable().unwrap();
}

// ---------------------------------------------------------------------
// Rule-6a mandatory statements: the weighting basis, the term-to-label
// mapping (with the ADR-12 "transmission" meaning), and the per-tech
// bridge terms.
// ---------------------------------------------------------------------

#[test]
fn q9_states_its_weighting_basis_and_label_mapping() {
    let reference = reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert!(
        q9.weighting_basis.contains("potential"),
        "the weighting basis must state the potential-output convention: {}",
        q9.weighting_basis
    );
    assert!(
        q9.label_mapping.contains("constraint") && q9.label_mapping.contains("interconnection"),
        "the label mapping must state what 'transmission' means here: {}",
        q9.label_mapping
    );
    assert!(
        q9.label_mapping.contains("ADR-12") || q9.label_mapping.contains("no network model"),
        "the ADR-12 disclosure must travel on the artefact: {}",
        q9.label_mapping
    );
    // The costed-coverage statement is mandatory and CO-EMITTED with
    // the denominator wedge — the wedge has no coverage-free accessor
    // (review condition 2). On this fixture every supply series is
    // costed, and the statement says so.
    let (_, coverage) = q9.denominator_wedge();
    assert!(coverage.complete);
    assert!(coverage.uncosted.is_empty());
    assert!(
        coverage.statement.contains("COMPLETE"),
        "coverage statement: {}",
        coverage.statement
    );
}

#[test]
fn q9_reports_per_tech_bridge_terms_with_their_cf_bases() {
    let reference = reference();
    let q9 = q9_decomposition(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    let wind = q9
        .plant_gate
        .iter()
        .find(|t| t.tech == "onshore_wind")
        .unwrap();
    assert_eq!(wind.assumed_cf, Some(PerUnit::new(0.36)));
    assert!((wind.realised_cf.value() - 0.25).abs() < 1e-12);
    assert_eq!(wind.generated_energy, Energy::gigawatt_hours(72.0));
    // Wind plant-gate LCOE at the assumed CF: fixed / (12 GW × 0.36 ×
    // 24 h) + zero VOM.
    let expected = fixed_wind(0.075) / 103_680.0;
    assert_close(
        wind.plant_gate_lcoe.as_ref().unwrap().central,
        expected,
        "wind plant-gate LCOE (central)",
    );

    let ccgt = q9.plant_gate.iter().find(|t| t.tech == "ccgt").unwrap();
    assert_eq!(ccgt.assumed_cf, None, "the ccgt row publishes no LF");
    // Realised-CF convention: fixed / realised generation + realised
    // variable per MWh.
    let expected = fixed_ccgt(0.075) / 144_000.0 + 55.0;
    assert_close(
        ccgt.plant_gate_lcoe.as_ref().unwrap().central,
        expected,
        "ccgt plant-gate LCOE at realised CF (central)",
    );
}

// ---------------------------------------------------------------------
// Structured input errors: the identity's attribution preconditions.
// ---------------------------------------------------------------------

#[test]
fn q9_refuses_an_srmc_bearing_technology_that_is_not_costed() {
    let reference = reference();
    let mut spec = spec_with_battery();
    spec.generation.retain(|g| g.tech != "ccgt"); // ccgt carries SRMC
    match q9_decomposition(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("ccgt"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

#[test]
fn q9_refuses_duplicate_costed_technologies() {
    let reference = reference();
    let mut spec = spec_with_battery();
    let duplicate = spec.generation[0].clone();
    spec.generation.push(duplicate);
    match q9_decomposition(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("ccgt"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

#[test]
fn q9_refuses_a_costed_asset_with_no_capacity() {
    let reference = reference();
    let mut spec = spec_with_battery();
    spec.generation[1].capacity = Power::gigawatts(0.0);
    match q9_decomposition(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("onshore_wind"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// The 2024 reference: the identity closes on the real instrument, with
// the stamps and the coverage statement travelling. Needs the locally
// built 2024 pack — fails loudly without it (audit standing rule).
// ---------------------------------------------------------------------

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

/// The 2024 reference costed spec: every fleet technology with a
/// costs-reference row (ccgt, ocgt, nuclear, biomass, onshore/offshore
/// wind, solar→solar_pv), capacities read from the scenario. Coal and
/// hydro have no reference row and stay uncosted — the coverage
/// statement on the artefact owns this.
fn reference_2024_spec(scenario: &Scenario) -> CostStackSpec {
    let rows = [
        ("ccgt", "ccgt"),
        ("ocgt", "ocgt"),
        ("nuclear", "nuclear"),
        ("biomass", "biomass"),
        ("onshore_wind", "onshore_wind"),
        ("offshore_wind", "offshore_wind"),
        ("solar", "solar_pv"),
    ];
    let zone = &scenario.zones[0];
    let generation = rows
        .iter()
        .map(|(tech, row)| {
            let entry = zone
                .fleet
                .iter()
                .find(|f| f.technology.as_str() == *tech)
                .unwrap_or_else(|| panic!("2024 reference fleet has no {tech}"));
            CostedGeneration {
                tech: (*tech).to_owned(),
                cost_row: (*row).to_owned(),
                capacity: entry.capacity_gw,
            }
        })
        .collect();
    CostStackSpec {
        framing: CostFraming::Forward,
        adequacy_standard: "2024 observed-fleet validation year (not solved to a standard)"
            .to_owned(),
        generation,
        batteries: vec![CostedBattery {
            label: "battery".to_owned(),
            power: Power::gigawatts(4.7),
            energy: Energy::gigawatt_hours(6.6),
            vintage: StoreVintage::Build2030,
        }],
        interconnectors: vec![CostedLink {
            row: "viking_link".to_owned(),
            life_years: 40,
        }],
        holdings: vec![],
    }
}

#[test]
fn q9_identity_closes_on_the_2024_reference() {
    require_pack();
    let root = repo_root();
    let scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let inputs = load_run_inputs(&scenario, &root).unwrap();
    let result = run(&scenario, &inputs).unwrap();
    let pricing_spec = scenario.pricing.as_ref().unwrap();
    let pricing = load_pricing_inputs(&scenario, pricing_spec, &root).unwrap();
    let reference = reference();
    let spec = reference_2024_spec(&scenario);

    let q9 = q9_decomposition(&result, &pricing, &reference, &spec).unwrap();

    // The identity closes at every WACC band point.
    let (denominator_wedge, coverage) = q9.denominator_wedge();
    for i in 0..3 {
        let l_mean = band(&q9.plant_gate_lcoe_mean)[i].as_pounds_per_megawatt_hour();
        let w_util = band(&q9.utilisation_wedge)[i].as_pounds_per_megawatt_hour();
        let w_denom = band(&denominator_wedge)[i].as_pounds_per_megawatt_hour();
        let w_miss = band(&q9.missing_line_wedge)[i].as_pounds_per_megawatt_hour();
        let headline = band(&q9.stack.headline_per_mwh_delivered)[i].as_pounds_per_megawatt_hour();
        let reconstructed = l_mean + w_util + w_denom + w_miss;
        assert!(
            ((reconstructed - headline) / headline.abs().max(1.0)).abs() < 1e-9,
            "band point {i}: identity residual {} £/MWh",
            reconstructed - headline
        );
        assert_eq!(
            band(&q9.gap)[i],
            band(&q9.stack.headline_per_mwh_delivered)[i] - band(&q9.plant_gate_lcoe_mean)[i]
        );
    }

    // Quotability on the real artefact: since the 2026-07-06 battery
    // lift no consumed row is quarantined, so the artefact is quotable
    // and publishes; the battery staleness stamp (caveat 3.iii) and
    // the nuclear bracket rule still travel as rendering obligations.
    assert!(q9.stack.metadata.quotable);
    assert!(q9.stack.metadata.consumed_quarantined_rows.is_empty());
    q9.ensure_publishable().unwrap();
    assert!(
        q9.stack
            .metadata
            .staleness_stamps
            .iter()
            .any(|s| s.contains("storage.battery_li_ion") && s.contains("2018")),
        "staleness stamps: {:?}",
        q9.stack.metadata.staleness_stamps
    );
    assert!(
        q9.stack
            .metadata
            .bracket_rules
            .iter()
            .any(|r| r.contains("nuclear")),
        "bracket rules: {:?}",
        q9.stack.metadata.bracket_rules
    );

    // Costed-coverage boundary, PINNED BY NAME (review condition 2a):
    // on the 2024 reference the costed set excludes hydro, coal and the
    // three exogenous must-take traces — ~41 TWh of demand service
    // inside E but outside G. A silent coverage change (a series
    // gained, lost or renamed) must fail this pin.
    assert!(!coverage.complete);
    assert_eq!(
        coverage.uncosted,
        vec![
            "hydro".to_owned(),
            "coal".to_owned(),
            "exogenous:net_imports".to_owned(),
            "exogenous:pumped_storage_net".to_owned(),
            "exogenous:other".to_owned(),
        ],
        "the uncosted-supply set moved — re-adjudicate the coverage statement"
    );
    // The measured 2024 reality (review §C): the uncosted supply makes
    // E > G, so the denominator wedge — labelled 'curtailment'/
    // 'balancing' in the docs/07 mapping — is NEGATIVE (−16.63 £/MWh
    // central; whole gap −3.04). That sign must never appear
    // unexplained: it is co-emitted with the coverage statement (the
    // wedge has no coverage-free accessor), and the statement names
    // every uncosted series and owns the sign.
    assert!(
        denominator_wedge.central < Price::pounds_per_megawatt_hour(0.0),
        "the 2024-reference denominator wedge should be negative (measured −16.63 central); \
         if this changed, the coverage situation changed — re-adjudicate"
    );
    for name in &coverage.uncosted {
        assert!(
            coverage.statement.contains(name),
            "coverage statement must name {name}: {}",
            coverage.statement
        );
    }
    assert!(
        coverage.statement.contains("NEGATIVE"),
        "coverage statement must own the sign hazard: {}",
        coverage.statement
    );

    // The rule-3 stamp carries the run's unserved energy (zero on the
    // validated 2024 reference).
    assert_eq!(
        q9.stack.metadata.reliability.unserved_energy,
        Energy::gigawatt_hours(0.0)
    );
}
