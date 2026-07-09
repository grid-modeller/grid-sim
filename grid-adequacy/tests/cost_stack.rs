//! Stage 7 package 1 acceptance tests: the D8 rule-1 cost stack on an
//! adequacy [`RunResult`], the rule-2 reconciliation with component
//! lines RECOMPUTED INDEPENDENTLY of the library's code path (the
//! adjudication anti-tautology rule), the quarantine
//! propagate-then-refuse form, and the WACC banding + rule-3
//! reliability stamp on every cost output.
//!
//! Independence structure of the reconciliation test: the expected
//! value of every component line is recomputed here from the parsed
//! reference numbers and the raw run series using test-local
//! arithmetic (including a test-local CRF), never by calling the
//! library's line or annuity functions. The library total must then
//! (a) equal the bitwise sum of its own reported lines in the
//! documented order, and (b) match the independent recomputation. A
//! shared-accumulator bug — a line dropped from or double-counted into
//! the total, or a line value corrupted after accumulation — fails
//! (a); a wrong line that keeps the total self-consistent fails (b).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use grid_adequacy::costs::{
    CostFraming, CostMetadata, CostStackSpec, CostedBattery, CostedGeneration, CostedLink,
    ReliabilityStamp, ServiceHolding, StoreVintage, cost_stack,
};
use grid_adequacy::pricing::PricingInputs;
use grid_adequacy::result::{RunResult, TechSeries};
use grid_core::GridError;
use grid_core::costs::WaccBand;
use grid_core::costs_reference::CostsReference;
use grid_core::scenario::{Reliability, TechId};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, Money, PerUnit, Power, Price};

fn reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/reference/costs-gb.toml")
}

fn reference() -> CostsReference {
    CostsReference::load(&reference_path()).unwrap()
}

/// The committed reference with the battery row RE-QUARANTINED in
/// memory. The real row's condition-3 quarantine was lifted 2026-07-06
/// as a reviewed act (condition 3.i discharged, NREL/TP-6A40-93281), so
/// no committed row any longer quarantines a stack — the
/// propagate-then-refuse machinery is exercised on this synthetically
/// quarantined variant instead.
fn requarantined_battery_reference() -> CostsReference {
    let text = std::fs::read_to_string(reference_path()).unwrap();
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

/// A hand-checkable synthetic day: 10 GW flat demand; 3 GW onshore wind
/// potential; 6 GW CCGT and 1 GW nuclear dispatched flat; 0.5 GW
/// unserved in two periods; no curtailment; no store series (storage is
/// costed from the spec's capacities, not from dispatch).
fn run_result() -> RunResult {
    let flat = |gw: f64| vec![Power::gigawatts(gw); PERIODS];
    let mut unserved = flat(0.0);
    unserved[10] = Power::gigawatts(0.5);
    unserved[11] = Power::gigawatts(0.5);
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
        stores: vec![],
        curtailment: flat(0.0),
        unserved,
    }
}

/// Pricing inputs carrying one SRMC series (CCGT, flat £50/MWh) — the
/// Stage 2 chain the fuel+carbon line consumes (D8 rule 1.2).
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
                capacity: Power::gigawatts(3.0),
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

/// Test-local independent recomputation of the six lines at one WACC,
/// in £, from the parsed reference numbers and the raw series.
fn independent_lines(rate: f64) -> [f64; 6] {
    // Line 1 — generation capex annualised + fixed O&M (fom + insurance
    // + connection), central overnight capex incl. site infrastructure.
    let generation = ((1020.0 + 14.6) * 1000.0 * crf(rate, 25) + 22_900.0) * 8_000.0
        + ((1380.0 + 315.9) * 1000.0 * crf(rate, 35) + 39_900.0) * 3_000.0
        + ((5820.0 + 4.7) * 1000.0 * crf(rate, 60) + 111_840.0) * 1_000.0;
    // Line 2 — variable O&M + fuel + carbon: CCGT 6 GW × 24 h =
    // 144,000 MWh at SRMC £50 and VOM £5; nuclear 1 GW × 24 h =
    // 24,000 MWh at VOM 6.7 + fuel 6.7 + decommissioning 2.7 = £16.1;
    // onshore VOM is zero.
    let variable = 144_000.0 * (50.0 + 5.0) + 24_000.0 * 16.1;
    // Line 3 — storage capex/O&M, power and energy legs priced
    // separately (2030-build battery row): 1 GW power leg, 4 GWh energy
    // leg, £12.9/kW/yr fixed O&M.
    let storage = 262.0 * 1000.0 * crf(rate, 15) * 1_000.0
        + 135.0 * crf(rate, 15) * 4.0e6
        + 12_900.0 * 1_000.0;
    // Line 4 — interconnection: Viking Link £1.7bn over a caller-stated
    // 40-year life.
    let interconnection = 1.7e9 * crf(rate, 40);
    // Line 5 — stability services: 1.178 GW held × 24 h × £3.31/MW/h.
    let stability = 1_178.0 * 24.0 * 3.31;
    // Line 6 — constraint costs: named zero pending D6.
    let constraints = 0.0;
    [
        generation * YEARS,
        variable,
        storage * YEARS,
        interconnection * YEARS,
        stability,
        constraints,
    ]
}

fn assert_close(got: Money, expected: f64, what: &str) {
    let got = got.as_pounds();
    let scale = expected.abs().max(1.0);
    assert!(
        ((got - expected) / scale).abs() < 1e-9,
        "{what}: got £{got}, independently recomputed £{expected}"
    );
}

// ---------------------------------------------------------------------
// Rule 2: Σ component lines = total, lines recomputed independently.
// ---------------------------------------------------------------------

#[test]
fn cost_stack_reconciles_with_independently_recomputed_lines() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    let rates = [0.045, 0.075, 0.100];
    let pick = |band: &WaccBand<Money>| [band.low, band.central, band.high];

    for (i, rate) in rates.into_iter().enumerate() {
        let expected = independent_lines(rate);
        let lines = [
            pick(&stack.generation_capex_fom)[i],
            pick(&stack.variable_om_fuel_carbon)[i],
            pick(&stack.storage_capex_om)[i],
            pick(&stack.interconnection)[i],
            pick(&stack.stability_services)[i],
            pick(&stack.constraint_costs.value)[i],
        ];
        // (b) Every line matches the test's own arithmetic.
        for (line, (&got, &want)) in lines.iter().zip(expected.iter()).enumerate() {
            assert_close(got, want, &format!("line {line} at WACC {rate}"));
        }
        // (a) The total is the bitwise sum of the reported lines in the
        // documented order (exact under determinism, D8 rule 2).
        let summed = lines
            .iter()
            .fold(Money::pounds(0.0), |acc, &line| acc + line);
        assert_eq!(
            summed,
            pick(&stack.total)[i],
            "total at WACC {rate} is not the exact sum of its lines"
        );
        // And the total matches the independent recomputation too.
        assert_close(
            pick(&stack.total)[i],
            expected.iter().sum(),
            &format!("total at WACC {rate}"),
        );
    }
}

// ---------------------------------------------------------------------
// Rule 1: the headline denominator is delivered-TO-DEMAND energy
// (GB demand − unserved, D3 convention) — a different object from the
// Package A per-technology delivered series.
// ---------------------------------------------------------------------

#[test]
fn headline_uses_the_delivered_to_demand_denominator() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    // 10 GW × 24 h = 240 GWh demand, minus 0.5 GWh unserved.
    assert_eq!(
        stack.delivered_to_demand_energy,
        Energy::gigawatt_hours(239.5)
    );
    // Headline = total / delivered, per WACC band point.
    let expected_central = stack.total.central / stack.delivered_to_demand_energy;
    assert_eq!(stack.headline_per_mwh_delivered.central, expected_central);
    assert!(
        stack.headline_per_mwh_delivered.low < stack.headline_per_mwh_delivered.central
            && stack.headline_per_mwh_delivered.central < stack.headline_per_mwh_delivered.high
    );
}

#[test]
fn zero_delivered_energy_is_a_structured_error() {
    let reference = reference();
    let mut result = run_result();
    result.demand = vec![Power::gigawatts(0.0); PERIODS];
    result.unserved = vec![Power::gigawatts(0.0); PERIODS];
    match cost_stack(&result, &pricing_inputs(), &reference, &spec_with_battery()) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("delivered"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Rule 4: every cost output carries all three WACC values; lines with
// no capex content are flat across the band.
// ---------------------------------------------------------------------

#[test]
fn every_cost_output_carries_the_three_wacc_band() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    for band in [
        &stack.generation_capex_fom,
        &stack.storage_capex_om,
        &stack.interconnection,
        &stack.total,
    ] {
        assert!(
            band.low < band.central && band.central < band.high,
            "capex-bearing line must rise with the WACC"
        );
    }
    // Operating lines carry the band too, flat by construction.
    let variable = &stack.variable_om_fuel_carbon;
    assert!(variable.low == variable.central && variable.central == variable.high);
    let stability = &stack.stability_services;
    assert!(stability.low == stability.central && stability.central == stability.high);
    // The WACC values themselves are stamped on the metadata.
    assert_eq!(stack.metadata.wacc.low, PerUnit::new(0.045));
    assert_eq!(stack.metadata.wacc.central, PerUnit::new(0.075));
    assert_eq!(stack.metadata.wacc.high, PerUnit::new(0.100));
}

// ---------------------------------------------------------------------
// Rule 3 stamp: unserved energy + the standard the scenario was solved
// to, on every cost artefact. Unserved energy is NOT priced.
// ---------------------------------------------------------------------

#[test]
fn reliability_stamp_carries_unserved_energy_and_the_standard() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert_eq!(
        stack.metadata.reliability.unserved_energy,
        Energy::gigawatt_hours(0.5)
    );
    assert!(
        stack
            .metadata
            .reliability
            .adequacy_standard
            .contains("not solved")
    );
}

// ---------------------------------------------------------------------
// Rule 1.6: constraint costs are a named zero-with-flag line pending
// D6, never silently pooled.
// ---------------------------------------------------------------------

#[test]
fn constraint_line_is_a_named_zero_with_flag() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert!(stack.constraint_costs.pending_d6);
    assert_eq!(stack.constraint_costs.value.low, Money::pounds(0.0));
    assert_eq!(stack.constraint_costs.value.central, Money::pounds(0.0));
    assert_eq!(stack.constraint_costs.value.high, Money::pounds(0.0));
    assert!(stack.constraint_costs.note.contains("D6"));
}

// ---------------------------------------------------------------------
// Quarantine propagation (docs/04 Stage 7 pin, corrected form):
// consuming a quotable = false row is LEGAL and stamps the result
// non-quotable; the publish path refuses with a structured error.
// Exercised on the re-quarantined in-memory reference since the
// 2026-07-06 battery lift (see `requarantined_battery_reference`).
// ---------------------------------------------------------------------

#[test]
fn consuming_a_quarantined_battery_row_is_legal_and_stamps_non_quotable() {
    let reference = requarantined_battery_reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();

    assert!(!stack.metadata.quotable);
    assert_eq!(
        stack.metadata.consumed_quarantined_rows,
        vec!["storage.battery_li_ion".to_owned()]
    );
    // The staleness stamp propagates to the quoting artefact.
    assert!(
        stack
            .metadata
            .staleness_stamps
            .iter()
            .any(|s| s.contains("2018"))
    );
    // The publish path refuses.
    match stack.ensure_publishable() {
        Err(GridError::NonQuotableResult { reason }) => {
            assert!(
                reason.contains("storage.battery_li_ion"),
                "reason: {reason}"
            );
        }
        other => panic!("expected NonQuotableResult, got {other:?}"),
    }
}

#[test]
fn lifted_battery_row_is_quotable_and_the_staleness_stamp_still_travels() {
    // The committed battery row is quotable since the 2026-07-06
    // reviewed lift (condition 3.i discharged). Caveat 3.iii is NOT
    // lifted: the 2018-projection-vintage staleness stamp is a property
    // of the ROW, not of the quarantine, and must still travel on every
    // battery-containing artefact.
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_with_battery(),
    )
    .unwrap();
    assert!(stack.metadata.quotable);
    assert!(stack.metadata.consumed_quarantined_rows.is_empty());
    assert!(
        stack
            .metadata
            .staleness_stamps
            .iter()
            .any(|s| s.contains("storage.battery_li_ion") && s.contains("2018")),
        "caveat 3.iii must survive the lift — staleness stamps: {:?}",
        stack.metadata.staleness_stamps
    );
    stack.ensure_publishable().unwrap();
}

#[test]
fn stack_without_quarantined_rows_is_publishable() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_without_battery(),
    )
    .unwrap();
    assert!(stack.metadata.quotable);
    assert!(stack.metadata.consumed_quarantined_rows.is_empty());
    stack.ensure_publishable().unwrap();
}

#[test]
fn nuclear_bracket_rule_surfaces_in_metadata() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_without_battery(),
    )
    .unwrap();
    // The bracket rule is a rendering obligation, not a quarantine: the
    // result stays quotable but the rule travels with it.
    assert!(stack.metadata.quotable);
    assert!(
        stack
            .metadata
            .bracket_rules
            .iter()
            .any(|r| r.contains("nuclear_observed")),
        "bracket rules: {:?}",
        stack.metadata.bracket_rules
    );
}

#[test]
fn an_unmet_publication_gate_refuses_publication() {
    // The OCHT row is not yet consumable by the stack (hydrogen store
    // costing is a later package), so the gate half of the refuse path
    // is exercised at the metadata level directly.
    let metadata = CostMetadata {
        framing: CostFraming::Greenfield,
        wacc: WaccBand {
            low: PerUnit::new(0.045),
            central: PerUnit::new(0.075),
            high: PerUnit::new(0.100),
        },
        reliability: ReliabilityStamp {
            unserved_energy: Energy::gigawatt_hours(0.0),
            adequacy_standard: "zero unserved (test)".to_owned(),
        },
        quotable: true,
        consumed_quarantined_rows: vec![],
        publication_gates: vec![
            "Baringa H2P primary check required before any OCHT-containing number is published"
                .to_owned(),
        ],
        bracket_rules: vec![],
        staleness_stamps: vec![],
        limitations: vec![],
    };
    match metadata.ensure_publishable() {
        Err(GridError::NonQuotableResult { reason }) => {
            assert!(reason.contains("Baringa"), "reason: {reason}");
        }
        other => panic!("expected NonQuotableResult, got {other:?}"),
    }
}

#[test]
fn quarantined_interconnector_rows_cannot_be_consumed() {
    let reference = reference();
    let mut spec = spec_without_battery();
    spec.interconnectors = vec![CostedLink {
        row: "north_sea_link".to_owned(),
        life_years: 40,
    }];
    match cost_stack(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(
                reason.contains("north_sea_link"),
                "error must name the row: {reason}"
            );
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Structured input errors.
// ---------------------------------------------------------------------

#[test]
fn unknown_cost_row_is_a_structured_error() {
    let reference = reference();
    let mut spec = spec_without_battery();
    spec.generation[0].cost_row = "fusion".to_owned();
    match cost_stack(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("fusion"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

#[test]
fn costed_generation_must_match_a_result_series() {
    let reference = reference();
    let mut spec = spec_without_battery();
    spec.generation[0].tech = "ocgt".to_owned(); // not dispatched in the fixture
    match cost_stack(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("ocgt"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

#[test]
fn unknown_holding_service_is_a_structured_error() {
    let reference = reference();
    let mut spec = spec_without_battery();
    spec.holdings = vec![ServiceHolding {
        service: "static_firm_response".to_owned(),
        held: Power::gigawatts(1.0),
    }];
    match cost_stack(&run_result(), &pricing_inputs(), &reference, &spec) {
        Err(GridError::InvalidCostInputs { reason }) => {
            assert!(reason.contains("static_firm_response"), "reason: {reason}");
        }
        other => panic!("expected InvalidCostInputs, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// Documented limitations are stamped on the metadata (basis stamp for
// the out-of-scope IDC escalation; rule-8 import settlement).
// ---------------------------------------------------------------------

#[test]
fn documented_limitations_are_stamped() {
    let reference = reference();
    let stack = cost_stack(
        &run_result(),
        &pricing_inputs(),
        &reference,
        &spec_without_battery(),
    )
    .unwrap();
    assert!(
        stack
            .metadata
            .limitations
            .iter()
            .any(|l| l.contains("overnight")),
        "the overnight-capex / no-IDC basis stamp must be carried"
    );
    assert!(
        stack
            .metadata
            .limitations
            .iter()
            .any(|l| l.contains("settlement")),
        "the rule-8 import/export settlement limitation must be carried"
    );
}
