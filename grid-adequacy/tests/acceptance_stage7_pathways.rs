//! Stage 7 acceptance: the costed published-pathway scenarios
//! (docs/04 Stage 7 scope line "scenario pack for published pathways
//! (FES, CCC, Royal Society)" — the Royal Society leg is the committed
//! `acceptance_stage3_rs37y.rs`; this file delivers the FES 2025
//! Electric Engagement and CCC CB7 Balanced Pathway legs, 2035 + 2050,
//! as runnable, costed, PINNED scenarios).
//!
//! # THE STAGE 7 ACCEPTANCE-LINE INDEX (docs/04 Stage 7, the named test list)
//!
//! 1. **"LP mode on a small hand-checkable scenario matches manual
//!    optimum"** — committed:
//!    `lp_dispatch.rs::lp_storage_feasibility_matches_the_hand_computed_minimum`,
//!    `lp_dispatch.rs::lp_soc_convention_matches_rule_based_when_dispatch_is_forced`,
//!    `lp_dispatch.rs::lp_wheels_north_through_middle_to_south_for_zero_unserved`,
//!    `lp_solve.rs::lp_bisection_recovers_the_single_zone_requirement`.
//! 2. **"LP storage requirement ≤ rule-based on every scenario (sanity
//!    invariant); gap reported per scenario"** — committed:
//!    `lp_solve.rs::lp_requirement_is_at_most_rule_based_requirement`,
//!    `lp_solve.rs::lp_needs_strictly_less_storage_than_rule_based_when_wheeling_helps`,
//!    `lp_gap_report.rs::gap_report_pins_the_strict_wheeling_gap`,
//!    `lp_gap_report.rs::gap_report_holds_at_equality_where_rule_based_is_optimal`,
//!    `lp_gap_report.rs::gap_invariant_violation_is_a_structured_error`.
//! 3. **"Cost stack reconciles: Σ components = total, LCOE vs.
//!    delivered £/MWh gap fully decomposed (Q9)"** — committed:
//!    `cost_stack.rs` (independent-recomputation reconciliation),
//!    `q9_decomposition.rs` (the rule-6a identity, synthetic + the 2024
//!    reference); NEW in this file: the four pathway scenarios' cost
//!    stacks and Q9 decompositions — reconciliation asserted
//!    (`cost_stack_reconciles_on_every_pathway_scenario`) and every
//!    headline pinned (`*_pins_are_exact`).
//!
//! # What these scenarios are (pre-registered reporting shape)
//!
//! Each scenario answers "how does this PUBLISHED fleet perform under
//! the OBSERVED 2024 weather year?" — a single-weather-year instrument.
//! Unserved energy on a published fleet is THE FINDING, not a defect,
//! and no fleet is tuned to fix it (the RS precedent). Every quoted
//! number carries the scenario-header conventions: autarky
//! (interconnection excluded with magnitude, adequacy-ADVERSE), no
//! outage model + unlimited-fuel hydrogen/LCD (adequacy-FAVOURABLE),
//! no electrification reprofiling of the 2024 demand shape
//! (adequacy-FAVOURABLE), the CCC UK-as-GB convention, and the CCC
//! electrolysis demand-basis wedge (CCC curtailment is overstated as
//! waste by up to 29/89 TWh of intended electrolysis feedstock; CCC
//! adequacy is favourable by the same under-loading; any FES-vs-CCC
//! comparison carries the wedge — review condition 7).
//!
//! # Cost-stack conventions (this file's spec, stated)
//!
//! - Framing: GREENFIELD (a pathway fleet is a future build).
//! - Costed set = every fleet technology with a costs-reference row
//!   (ccgt, ocgt, nuclear, biomass, onshore/offshore wind,
//!   solar→solar_pv). Open-set pathway ids (beccs, ccgt_ccs,
//!   low_carbon_dispatchable, other_generation, waste, oil,
//!   hydrogen_turbine, hydro) have NO honest reference row and stay
//!   UNCOSTED — named per scenario in the pinned Q9 coverage
//!   statement; costing them at unabated-technology rates would
//!   misprice abated/novel plant. The pathway headline £/MWh is
//!   therefore a PARTIAL-COVERAGE figure, quotable only with its
//!   coverage statement (the machinery co-emits it).
//! - Battery stores are costed (2030-build vintage). The battery row's
//!   condition-3 quarantine was LIFTED 2026-07-06 (reviewed act;
//!   condition 3.i discharged against the NREL primary,
//!   NREL/TP-6A40-93281 — the committed numbers were CONFIRMED, so no
//!   pin below moved), and no other consumed row is quarantined
//!   (interconnection is not costed here — line 4 is a structural
//!   zero, so the quarantined NSL/NeuConnect/Greenlink rows are never
//!   consumed): every pathway stack is now QUOTABLE with an
//!   affirmatively-EMPTY consumed-quarantine declaration (the Q9
//!   review condition 3 empty-or-not declaration, by name). Caveats
//!   3.ii/3.iii REMAIN: the battery staleness stamp still travels on
//!   every stack. The pumped_hydro/LDES stores are NOT costed (no
//!   reference row for the fold) — a named limitation with the GW/GWh
//!   magnitude in the scenario files. The CB7 storage rounding stamps
//!   (energy_precision) are asserted present on the parsed reference.
//! - Interconnection and holdings: not costed (autarky; no holdings
//!   modelled) — lines 4 and 5 are structural zeros here.
//! - SRMC = 2024 actuals (D8 rule 1.2 chain), a stated convention;
//!   ccgt_ccs / hydrogen_turbine / LCD carry no priced fuel chain.
//!
//! # Pins
//!
//! Full precision (bit-exact f64), pack-gated: the tests FAIL LOUDLY
//! without the locally built 2024 pack (the audit standing rule).
//! HiGHS is not involved (rule-based single-zone runs), so the pins
//! carry no cross-machine solver caveat; they are pure-arithmetic
//! digest-grade values.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::costs::{
    CostFraming, CostStackSpec, CostedBattery, CostedGeneration, Q9Decomposition, StoreVintage,
    q9_decomposition,
};
use grid_adequacy::result::RunResult;
use grid_adequacy::{load_pricing_inputs, load_run_inputs, run};
use grid_core::costs::WaccBand;
use grid_core::costs_reference::CostsReference;
use grid_core::pathways_published::{ExclusionRecord, PathwaysPublished};
use grid_core::scenario::{Scenario, StorageKind};
use grid_core::units::{Duration, Energy, Money, Power, Price};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Fail loudly if the 2024 data pack has not been built locally (the
/// audit standing rule: no silent self-skip on the only guard).
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

const FES_2035: &str = "scenarios/fes2025-ee-2035.toml";
const FES_2050: &str = "scenarios/fes2025-ee-2050.toml";
const CCC_2035: &str = "scenarios/ccc-cb7-bp-2035.toml";
const CCC_2050: &str = "scenarios/ccc-cb7-bp-2050.toml";
const SCENARIOS: [&str; 4] = [FES_2035, FES_2050, CCC_2035, CCC_2050];

fn load(path: &str) -> Scenario {
    Scenario::load(&repo_root().join(path)).unwrap()
}

fn pathways() -> PathwaysPublished {
    PathwaysPublished::load(&repo_root().join("data/reference/pathways-published.toml")).unwrap()
}

fn run_scenario(scenario: &Scenario) -> RunResult {
    let root = repo_root();
    let inputs = load_run_inputs(scenario, &root).unwrap();
    run(scenario, &inputs).unwrap()
}

/// The costed generation set: every fleet technology with a
/// costs-reference row (module docs), capacities read from the
/// scenario itself.
fn costed_spec(scenario: &Scenario) -> CostStackSpec {
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
    let generation = zone
        .fleet
        .iter()
        .filter_map(|entry| {
            rows.iter()
                .find(|(tech, _)| *tech == entry.technology.as_str())
                .map(|(tech, row)| CostedGeneration {
                    tech: (*tech).to_owned(),
                    cost_row: (*row).to_owned(),
                    capacity: entry.capacity_gw,
                })
        })
        .collect();
    let batteries = zone
        .storage
        .iter()
        .filter(|s| s.kind == StorageKind::Battery)
        .map(|s| CostedBattery {
            label: "battery".to_owned(),
            power: s.power_gw,
            energy: s.energy_gwh,
            vintage: StoreVintage::Build2030,
        })
        .collect();
    CostStackSpec {
        framing: CostFraming::Greenfield,
        adequacy_standard:
            "not solved to a standard — published-pathway fleet as-is under 2024 weather \
             (unserved energy is the finding)"
                .to_owned(),
        generation,
        batteries,
        interconnectors: vec![],
        holdings: vec![],
    }
}

fn q9_for(path: &str) -> (Scenario, RunResult, Q9Decomposition) {
    let root = repo_root();
    let scenario = load(path);
    let result = run_scenario(&scenario);
    let pricing_spec = scenario.pricing.as_ref().unwrap();
    let pricing = load_pricing_inputs(&scenario, pricing_spec, &root).unwrap();
    let reference = CostsReference::load(&root.join("data/reference/costs-gb.toml")).unwrap();
    let spec = costed_spec(&scenario);
    let q9 = q9_decomposition(&result, &pricing, &reference, &spec).unwrap();
    (scenario, result, q9)
}

fn energy_of(series: &[Power]) -> Energy {
    series
        .iter()
        .map(|&p| p * Duration::half_hour())
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

fn thermal_energy(result: &RunResult, tech: &str) -> Energy {
    energy_of(
        &result
            .thermal
            .iter()
            .find(|s| s.tech.as_str() == tech)
            .unwrap_or_else(|| panic!("no thermal series {tech}"))
            .power,
    )
}

fn store_discharge(result: &RunResult, kind: StorageKind) -> Energy {
    energy_of(
        &result
            .stores
            .iter()
            .find(|s| s.kind == kind)
            .unwrap_or_else(|| panic!("no store of kind {kind}"))
            .discharge,
    )
}

fn gwh(value: f64) -> Energy {
    Energy::gigawatt_hours(value)
}

fn pounds(value: f64) -> Money {
    Money::pounds(value)
}

fn per_mwh(value: f64) -> Price {
    Price::pounds_per_megawatt_hour(value)
}

fn scenario_capacity(scenario: &Scenario, tech: &str) -> Option<Power> {
    scenario.zones[0]
        .fleet
        .iter()
        .find(|f| f.technology.as_str() == tech)
        .map(|f| f.capacity_gw)
}

// ---------------------------------------------------------------------
// Scenario ↔ published-reference consistency: every capacity is the
// parsed reference value or a DECLARED, documented split of it (the
// review condition 5/8 discipline — the parser refuses to hand
// aggregates over as fleet, so each consumption here is explicit).
// ---------------------------------------------------------------------

#[test]
fn fes_scenario_fleets_match_the_published_reference_and_declared_fold_splits() {
    require_pack();
    let reference = pathways();
    let fes = &reference.pathways["fes2025_electric_engagement"];

    for (path, year) in [(FES_2035, 2035), (FES_2050, 2050)] {
        let scenario = load(path);
        let published = fes.year(year).unwrap();
        let cap = |tech: &str| {
            published
                .fleet
                .iter()
                .find(|f| f.technology.as_str() == tech)
                .unwrap_or_else(|| panic!("no published {tech}"))
                .capacity
        };

        // Direct mappings, bit-equal to the reference.
        for tech in [
            "nuclear",
            "waste",
            "hydro",
            "ocgt",
            "oil",
            "hydrogen_turbine",
            "onshore_wind",
            "offshore_wind",
            "solar",
        ] {
            assert_eq!(
                scenario_capacity(&scenario, tech),
                Some(cap(tech)),
                "{path}: {tech} deviates from pathways-published"
            );
        }

        // Declared fold splits (scenario convention 6): components are
        // the reference's own e1/e2 exclusion magnitudes; the split
        // halves reassemble the published fold.
        let e1 = match &fes.exclusions["ccs_gas_in_ccgt_gw"] {
            ExclusionRecord::CapacityByYear(map) => map[&year],
            other => panic!("e1: {other:?}"),
        };
        let e2 = match &fes.exclusions["ccs_biomass_in_biomass_gw"] {
            ExclusionRecord::CapacityByYear(map) => map[&year],
            other => panic!("e2: {other:?}"),
        };
        let ccgt = scenario_capacity(&scenario, "ccgt").unwrap();
        let ccgt_ccs = scenario_capacity(&scenario, "ccgt_ccs").unwrap();
        assert_eq!(ccgt_ccs, e1, "{path}: ccgt_ccs must be the e1 magnitude");
        assert!(
            ((ccgt + ccgt_ccs).as_gigawatts() - cap("ccgt").as_gigawatts()).abs() < 1e-9,
            "{path}: ccgt split does not reassemble the published fold"
        );
        let biomass = scenario_capacity(&scenario, "biomass").unwrap();
        let beccs = scenario_capacity(&scenario, "beccs").unwrap();
        assert_eq!(beccs, e2, "{path}: beccs must be the e2 magnitude");
        assert!(
            ((biomass + beccs).as_gigawatts() - cap("biomass").as_gigawatts()).abs() < 1e-9,
            "{path}: biomass split does not reassemble the published fold"
        );

        // Named exclusions (convention 7): marine and geothermal-CHP
        // "other" exist in the reference and are ABSENT from the
        // scenario — the exclusion-with-magnitude, machine-checked.
        for excluded in ["marine", "other"] {
            assert!(cap(excluded).as_gigawatts() > 0.0);
            assert_eq!(
                scenario_capacity(&scenario, excluded),
                None,
                "{path}: {excluded} must stay a named exclusion, not fleet capacity"
            );
        }

        // Storage bit-equal; interconnection carried inert at the
        // published GW with availability zero (autarky convention 3).
        for kind in [StorageKind::Battery, StorageKind::PumpedHydro] {
            let published_store = published.storage.iter().find(|s| s.kind == kind).unwrap();
            let store = scenario.zones[0]
                .storage
                .iter()
                .find(|s| s.kind == kind)
                .unwrap();
            assert_eq!(store.power_gw, published_store.power);
            assert_eq!(store.energy_gwh, published_store.energy);
        }
        let link = &scenario.links[0];
        assert_eq!(link.capacity_gw, cap("interconnector"));
        assert_eq!(link.availability.value(), 0.0, "{path}: autarky");
    }
}

#[test]
fn ccc_scenario_fleets_match_the_published_reference_and_declared_split_rules() {
    require_pack();
    let reference = pathways();
    let ccc = &reference.pathways["ccc_cb7_balanced"];
    assert_eq!(ccc.geography, "UK");

    for (path, year) in [(CCC_2035, 2035), (CCC_2050, 2050)] {
        let scenario = load(path);
        // The UK-as-GB declaration travels on the scenario itself
        // (review condition 6).
        assert!(
            scenario
                .description
                .as_deref()
                .unwrap()
                .contains("UK-as-GB"),
            "{path}: the UK-as-GB convention must be declared on the scenario"
        );

        let published = ccc.year(year).unwrap();
        let cap = |tech: &str| {
            published
                .fleet
                .iter()
                .find(|f| f.technology.as_str() == tech)
                .unwrap()
                .capacity
        };
        for tech in ["nuclear", "onshore_wind", "offshore_wind", "solar"] {
            assert_eq!(
                scenario_capacity(&scenario, tech),
                Some(cap(tech)),
                "{path}: {tech}"
            );
        }

        let aggregate = |name: &str| {
            ccc.aggregates
                .iter()
                .find(|a| a.name == name)
                .unwrap()
                .capacity_by_year[&year]
        };

        // Split decision S1 (2035 only): unabated_gas split by the FES
        // EE 2035 unabated CCGT-class : OCGT-class ratio, recomputed
        // here from the reference itself (fold − e1 : ocgt), matching
        // the scenario at its stated 4 dp rounding; the halves
        // reassemble the published bucket exactly.
        let unabated_gas = aggregate("unabated_gas");
        if year == 2035 {
            let fes = &reference.pathways["fes2025_electric_engagement"];
            let fes_2035 = fes.year(2035).unwrap();
            let e1 = match &fes.exclusions["ccs_gas_in_ccgt_gw"] {
                ExclusionRecord::CapacityByYear(map) => map[&2035],
                other => panic!("e1: {other:?}"),
            };
            let fes_ccgt_class = fes_2035
                .fleet
                .iter()
                .find(|f| f.technology.as_str() == "ccgt")
                .unwrap()
                .capacity
                - e1;
            let fes_ocgt_class = fes_2035
                .fleet
                .iter()
                .find(|f| f.technology.as_str() == "ocgt")
                .unwrap()
                .capacity;
            let rule_ccgt = unabated_gas.as_gigawatts() * fes_ccgt_class.as_gigawatts()
                / (fes_ccgt_class + fes_ocgt_class).as_gigawatts();
            let ccgt = scenario_capacity(&scenario, "ccgt").unwrap();
            let ocgt = scenario_capacity(&scenario, "ocgt").unwrap();
            assert!(
                (ccgt.as_gigawatts() - rule_ccgt).abs() < 5e-5,
                "{path}: S1 ccgt {} deviates from the declared rule value {rule_ccgt}",
                ccgt.as_gigawatts()
            );
            assert!(
                ((ccgt + ocgt).as_gigawatts() - unabated_gas.as_gigawatts()).abs() < 1e-9,
                "{path}: S1 halves do not reassemble the published bucket"
            );
        } else {
            assert_eq!(unabated_gas, Power::gigawatts(0.0));
            assert_eq!(scenario_capacity(&scenario, "ccgt"), None);
            assert_eq!(scenario_capacity(&scenario, "ocgt"), None);
        }

        // S2/S3 + other_generation: consumed at published GW under
        // their open-set ids — no component split invented.
        assert_eq!(
            scenario_capacity(&scenario, "low_carbon_dispatchable"),
            Some(aggregate("low_carbon_dispatchable")),
            "{path}: S2"
        );
        assert_eq!(
            scenario_capacity(&scenario, "beccs"),
            Some(aggregate("ccs_biomass")),
            "{path}: S3"
        );
        assert_eq!(
            scenario_capacity(&scenario, "other_generation"),
            Some(aggregate("other_generation")),
            "{path}: other_generation"
        );
        // smart_demand_flexibility stays a named exclusion (e5 ruling).
        assert!(aggregate("smart_demand_flexibility").as_gigawatts() > 0.0);
        assert_eq!(
            scenario_capacity(&scenario, "smart_demand_flexibility"),
            None
        );

        // Storage bit-equal, and the CB7 rounding stamps are present on
        // the parsed reference (condition 5 propagation basis).
        for kind in [StorageKind::Battery, StorageKind::PumpedHydro] {
            let published_store = published.storage.iter().find(|s| s.kind == kind).unwrap();
            let store = scenario.zones[0]
                .storage
                .iter()
                .find(|s| s.kind == kind)
                .unwrap();
            assert_eq!(store.power_gw, published_store.power);
            assert_eq!(store.energy_gwh, published_store.energy);
            assert!(
                published_store
                    .energy_precision
                    .as_deref()
                    .unwrap()
                    .contains("rounded integer"),
                "{path}: the CB7 rounding stamp must travel"
            );
        }
        let link = &scenario.links[0];
        assert_eq!(link.capacity_gw, cap("interconnector"));
        assert_eq!(link.availability.value(), 0.0, "{path}: autarky");

        // The electrolysis wedge is machine-visible on the reference
        // this scenario declares its basis against (condition 7).
        let wedge = published.surplus_electrolysis_excluded.unwrap();
        let expected = if year == 2035 { 29_000.0 } else { 89_000.0 };
        assert_eq!(wedge, Energy::gigawatt_hours(expected));
    }
}

#[test]
fn scenario_demand_totals_land_on_the_published_pathway_demand() {
    require_pack();
    let reference = pathways();
    for (path, pathway, year) in [
        (FES_2035, "fes2025_electric_engagement", 2035),
        (FES_2050, "fes2025_electric_engagement", 2050),
        (CCC_2035, "ccc_cb7_balanced", 2035),
        (CCC_2050, "ccc_cb7_balanced", 2050),
    ] {
        let scenario = load(path);
        let result = run_scenario(&scenario);
        let target = reference.pathways[pathway].year(year).unwrap().demand;
        let got = result.total_demand_energy().as_gigawatt_hours();
        let want = target.as_gigawatt_hours();
        assert!(
            ((got - want) / want).abs() < 1e-9,
            "{path}: run demand {got} GWh vs published {want} GWh"
        );
    }
}

// ---------------------------------------------------------------------
// Reconciliation (acceptance line 3 on the pathway instruments): the
// six lines re-fold to the total exactly, at every WACC band point.
// ---------------------------------------------------------------------

#[test]
fn cost_stack_reconciles_on_every_pathway_scenario() {
    require_pack();
    for path in SCENARIOS {
        let (_, _, q9) = q9_for(path);
        let stack = &q9.stack;
        let fold = |pick: &dyn Fn(&WaccBand<Money>) -> Money| {
            pick(&stack.generation_capex_fom)
                + pick(&stack.variable_om_fuel_carbon)
                + pick(&stack.storage_capex_om)
                + pick(&stack.interconnection)
                + pick(&stack.stability_services)
                + pick(&stack.constraint_costs.value)
        };
        assert_eq!(fold(&|b| b.low), stack.total.low, "{path}: low");
        assert_eq!(fold(&|b| b.central), stack.total.central, "{path}: central");
        assert_eq!(fold(&|b| b.high), stack.total.high, "{path}: high");
        // Lines 4 and 5 are structural zeros here (module docs).
        assert_eq!(stack.interconnection.central, Money::pounds(0.0));
        assert_eq!(stack.stability_services.central, Money::pounds(0.0));
        assert!(stack.constraint_costs.pending_d6);

        // The Q9 identity closes (the committed q9_decomposition.rs
        // bound) on every pathway instrument.
        let (denominator_wedge, _) = q9.denominator_wedge();
        for (mean, util, denom, miss, headline) in [
            (
                q9.plant_gate_lcoe_mean.low,
                q9.utilisation_wedge.low,
                denominator_wedge.low,
                q9.missing_line_wedge.low,
                stack.headline_per_mwh_delivered.low,
            ),
            (
                q9.plant_gate_lcoe_mean.central,
                q9.utilisation_wedge.central,
                denominator_wedge.central,
                q9.missing_line_wedge.central,
                stack.headline_per_mwh_delivered.central,
            ),
            (
                q9.plant_gate_lcoe_mean.high,
                q9.utilisation_wedge.high,
                denominator_wedge.high,
                q9.missing_line_wedge.high,
                stack.headline_per_mwh_delivered.high,
            ),
        ] {
            let reconstructed = mean.as_pounds_per_megawatt_hour()
                + util.as_pounds_per_megawatt_hour()
                + denom.as_pounds_per_megawatt_hour()
                + miss.as_pounds_per_megawatt_hour();
            let target = headline.as_pounds_per_megawatt_hour();
            assert!(
                ((reconstructed - target) / target.abs().max(1.0)).abs() < 1e-9,
                "{path}: Q9 identity residual {}",
                reconstructed - target
            );
        }
    }
}

// ---------------------------------------------------------------------
// Quotability, stamps and coverage: the affirmative declarations
// (Q9 review condition 3 — never a silent empty). Updated 2026-07-06
// for the reviewed battery-quarantine lift (condition 3.i discharged):
// the declaration is now affirmatively EMPTY and the stacks publish.
// ---------------------------------------------------------------------

#[test]
fn quarantine_declaration_is_affirmatively_empty_and_stacks_are_quotable() {
    require_pack();
    for path in SCENARIOS {
        let (_, _, q9) = q9_for(path);
        // AFFIRMATIVE declaration of the post-lift quarantine set: EMPTY
        // by name — the battery row was lifted (2026-07-06 reviewed
        // act), and the still-quarantined interconnector rows
        // (NSL/NeuConnect/Greenlink) are never consumed here (autarky:
        // the spec costs no interconnector, line 4 is a structural
        // zero). Any row appearing here means a quarantined input crept
        // into the pathway stacks — re-adjudicate before quoting.
        assert_eq!(
            q9.stack.metadata.consumed_quarantined_rows,
            Vec::<String>::new(),
            "{path}: the consumed-quarantine declaration moved"
        );
        assert!(q9.stack.metadata.quotable);
        q9.ensure_publishable()
            .unwrap_or_else(|e| panic!("{path}: expected publishable, got {e:?}"));
        // Caveats 3.ii/3.iii REMAIN: the battery staleness stamp and
        // the nuclear bracket rule travel on every pathway artefact.
        assert!(
            q9.stack
                .metadata
                .staleness_stamps
                .iter()
                .any(|s| s.contains("battery")),
            "{path}: staleness stamps {:?}",
            q9.stack.metadata.staleness_stamps
        );
        assert!(
            q9.stack
                .metadata
                .bracket_rules
                .iter()
                .any(|r| r.contains("nuclear")),
            "{path}: bracket rules {:?}",
            q9.stack.metadata.bracket_rules
        );
        // The reliability stamp carries the run's (nonzero) unserved.
        assert_eq!(
            q9.stack.metadata.reliability.adequacy_standard,
            "not solved to a standard — published-pathway fleet as-is under 2024 weather \
             (unserved energy is the finding)"
        );
    }
}

#[test]
fn costed_coverage_is_pinned_by_name_per_scenario() {
    require_pack();
    let expectations = [
        (
            FES_2035,
            vec![
                "beccs",
                "waste",
                "hydro",
                "ccgt_ccs",
                "oil",
                "hydrogen_turbine",
            ],
        ),
        (
            FES_2050,
            vec![
                "beccs",
                "waste",
                "hydro",
                "ccgt_ccs",
                "oil",
                "hydrogen_turbine",
            ],
        ),
        (
            CCC_2035,
            vec!["beccs", "other_generation", "low_carbon_dispatchable"],
        ),
        (
            CCC_2050,
            vec!["beccs", "other_generation", "low_carbon_dispatchable"],
        ),
    ];
    for (path, expected) in expectations {
        let (_, _, q9) = q9_for(path);
        let (_, coverage) = q9.denominator_wedge();
        assert!(!coverage.complete);
        let expected: Vec<String> = expected.into_iter().map(str::to_owned).collect();
        assert_eq!(
            coverage.uncosted, expected,
            "{path}: the uncosted-supply set moved — re-adjudicate the coverage statement"
        );
        for name in &coverage.uncosted {
            assert!(coverage.statement.contains(name.as_str()));
        }
    }
}

// ---------------------------------------------------------------------
// THE PINS: per-scenario headlines, full precision, characterised on
// the runs of 2026-07-06 (pack 2024.sha256). Any movement is a
// deliberate engine/pack/scenario change requiring a knowing re-pin.
// ---------------------------------------------------------------------

struct Pins {
    demand_gwh: f64,
    unserved_gwh: f64,
    curtailment_gwh: f64,
    battery_discharge_gwh: f64,
    pumped_discharge_gwh: f64,
    /// (technology, dispatched GWh) — the fuel-bearing / novel rungs.
    thermal_gwh: &'static [(&'static str, f64)],
    total_low: f64,
    total_central: f64,
    total_high: f64,
    headline_low: f64,
    headline_central: f64,
    headline_high: f64,
    plant_gate_mean_central: f64,
    gap_central: f64,
    utilisation_wedge_central: f64,
    denominator_wedge_central: f64,
    missing_line_wedge_central: f64,
}

fn assert_pins(path: &str, pins: &Pins) {
    let (_, result, q9) = q9_for(path);
    assert_eq!(
        result.total_demand_energy(),
        gwh(pins.demand_gwh),
        "{path}: demand"
    );
    assert_eq!(
        result.total_unserved(),
        gwh(pins.unserved_gwh),
        "{path}: unserved"
    );
    assert_eq!(
        result.total_curtailment(),
        gwh(pins.curtailment_gwh),
        "{path}: curtailment"
    );
    assert_eq!(
        store_discharge(&result, StorageKind::Battery),
        gwh(pins.battery_discharge_gwh),
        "{path}: battery cycling"
    );
    assert_eq!(
        store_discharge(&result, StorageKind::PumpedHydro),
        gwh(pins.pumped_discharge_gwh),
        "{path}: pumped/LDES cycling"
    );
    for (tech, energy) in pins.thermal_gwh {
        assert_eq!(
            thermal_energy(&result, tech),
            gwh(*energy),
            "{path}: {tech} dispatch"
        );
    }
    let stack = &q9.stack;
    assert_eq!(stack.total.low, pounds(pins.total_low), "{path}: total low");
    assert_eq!(
        stack.total.central,
        pounds(pins.total_central),
        "{path}: total central"
    );
    assert_eq!(
        stack.total.high,
        pounds(pins.total_high),
        "{path}: total high"
    );
    assert_eq!(
        stack.headline_per_mwh_delivered.low,
        per_mwh(pins.headline_low),
        "{path}: headline low"
    );
    assert_eq!(
        stack.headline_per_mwh_delivered.central,
        per_mwh(pins.headline_central),
        "{path}: headline central"
    );
    assert_eq!(
        stack.headline_per_mwh_delivered.high,
        per_mwh(pins.headline_high),
        "{path}: headline high"
    );
    assert_eq!(
        q9.plant_gate_lcoe_mean.central,
        per_mwh(pins.plant_gate_mean_central),
        "{path}: mean plant-gate LCOE"
    );
    assert_eq!(q9.gap.central, per_mwh(pins.gap_central), "{path}: Q9 gap");
    assert_eq!(
        q9.utilisation_wedge.central,
        per_mwh(pins.utilisation_wedge_central),
        "{path}: utilisation wedge"
    );
    let (denominator_wedge, _) = q9.denominator_wedge();
    assert_eq!(
        denominator_wedge.central,
        per_mwh(pins.denominator_wedge_central),
        "{path}: denominator wedge"
    );
    assert_eq!(
        q9.missing_line_wedge.central,
        per_mwh(pins.missing_line_wedge_central),
        "{path}: missing-line wedge"
    );
}

#[test]
fn fes_2035_pins_are_exact() {
    require_pack();
    assert_pins(
        FES_2035,
        &Pins {
            demand_gwh: 450_075.999_999_999,
            unserved_gwh: 1_716.936_985_664_870_5,
            curtailment_gwh: 38_469.743_557_467_846,
            battery_discharge_gwh: 407.274_163_576_446_7,
            pumped_discharge_gwh: 1_115.234_252_689_569,
            thermal_gwh: &[
                ("ccgt_ccs", 24_778.957_477_079_344),
                ("ccgt", 23_854.700_114_880_67),
                ("ocgt", 5_239.847_266_216_05),
                ("hydrogen_turbine", 466.181_384_842_209_2),
            ],
            total_low: 38_064_688_071.790_985,
            total_central: 46_531_068_925.502_15,
            total_high: 54_270_255_129.808_495,
            headline_low: 84.897_777_722_793_68,
            headline_central: 103.780_814_895_704_57,
            headline_high: 121.041_949_648_452_77,
            plant_gate_mean_central: 83.559_291_079_998,
            gap_central: 20.221_523_815_706_56,
            utilisation_wedge_central: 21.559_838_872_237_098,
            denominator_wedge_central: -6.001_463_058_064_459,
            missing_line_wedge_central: 4.663_148_001_533_885,
        },
    );
}

#[test]
fn fes_2050_pins_are_exact() {
    require_pack();
    assert_pins(
        FES_2050,
        &Pins {
            demand_gwh: 784_735.999_999_998_3,
            unserved_gwh: 869.657_631_739_639_7,
            curtailment_gwh: 21_610.187_317_459_982,
            battery_discharge_gwh: 176.350_391_853_870_83,
            pumped_discharge_gwh: 1_090.008_654_087_286,
            thermal_gwh: &[
                ("ccgt_ccs", 87_906.959_501_198_65),
                ("ccgt", 7_196.375_446_273_965),
                ("ocgt", 2_735.050_594_390_898_4),
                ("hydrogen_turbine", 18_217.951_123_177_983),
            ],
            total_low: 55_901_962_509.084_93,
            total_central: 69_647_094_028.999_88,
            total_high: 82_142_970_540.825_59,
            headline_low: 71.315_681_625_251_26,
            headline_central: 88.850_726_539_142_36,
            headline_high: 104.792_062_244_503_17,
            plant_gate_mean_central: 81.645_489_486_315_9,
            gap_central: 7.205_237_052_826_462,
            utilisation_wedge_central: 19.609_686_313_076_843,
            denominator_wedge_central: -15.777_156_580_514_33,
            missing_line_wedge_central: 3.372_707_320_263_986,
        },
    );
}

#[test]
fn ccc_2035_pins_are_exact() {
    require_pack();
    assert_pins(
        CCC_2035,
        &Pins {
            demand_gwh: 443_541.000_000_000_7,
            unserved_gwh: 20.259_298_341_906_707,
            curtailment_gwh: 33_629.950_985_939_37,
            battery_discharge_gwh: 66.857_344_773_912_39,
            pumped_discharge_gwh: 422.821_677_047_727_6,
            thermal_gwh: &[
                ("low_carbon_dispatchable", 28_013.215_827_597_21),
                ("ccgt", 24_915.269_813_936_262),
                ("ocgt", 2_789.055_521_425_407_7),
            ],
            total_low: 37_898_913_004.852_96,
            total_central: 46_389_926_743.372_795,
            total_high: 54_149_368_933.277_27,
            headline_low: 85.450_148_159_691_72,
            headline_central: 104.594_717_870_426_96,
            headline_high: 122.089_823_460_368_21,
            plant_gate_mean_central: 87.593_122_766_409_14,
            gap_central: 17.001_595_104_017_824,
            utilisation_wedge_central: 22.155_386_439_345_016,
            denominator_wedge_central: -9.035_850_094_689_067,
            missing_line_wedge_central: 3.882_058_759_361_888,
        },
    );
}

#[test]
fn ccc_2050_pins_are_exact() {
    require_pack();
    assert_pins(
        CCC_2050,
        &Pins {
            demand_gwh: 692_025.000_000_002_9,
            unserved_gwh: 5_251.239_356_108_109,
            curtailment_gwh: 59_041.077_339_180_76,
            battery_discharge_gwh: 2_102.217_755_337_009_3,
            pumped_discharge_gwh: 3_564.230_158_074_912,
            thermal_gwh: &[
                ("low_carbon_dispatchable", 92_039.194_374_847_04),
                ("nuclear", 60_324.315_995_260_48),
            ],
            total_low: 56_454_299_513.548_4,
            total_central: 69_788_536_072.362_3,
            total_high: 81_965_005_593.956_73,
            headline_low: 82.202_178_866_907_86,
            headline_central: 101.617_941_848_755_32,
            headline_high: 119.347_899_251_580_66,
            plant_gate_mean_central: 81.584_856_463_448_28,
            gap_central: 20.033_085_385_307_047,
            utilisation_wedge_central: 24.536_354_889_036_85,
            denominator_wedge_central: -9.775_548_513_799_82,
            missing_line_wedge_central: 5.272_279_010_070_012_5,
        },
    );
}

// ---------------------------------------------------------------------
// Determinism (ADR-5): bit-identical artefacts across reruns.
// ---------------------------------------------------------------------

#[test]
fn pathway_runs_and_decompositions_are_deterministic() {
    require_pack();
    let (_, first_result, first_q9) = q9_for(FES_2035);
    let (_, second_result, second_q9) = q9_for(FES_2035);
    assert!(first_result == second_result, "run differs between reruns");
    assert!(first_q9 == second_q9, "Q9 differs between reruns");
}
