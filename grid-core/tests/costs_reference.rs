//! Stage 7 package 1 acceptance tests: the costs-reference-v1 strict
//! parser (`grid_core::costs_reference`) over the committed
//! `data/reference/costs-gb.toml`, and the D8 rule-4 annualisation
//! arithmetic (`grid_core::costs`).
//!
//! Pinned regression tests (review condition 12 / D8 rule 9): the WACC
//! set, one capex row (CCGT), one trajectory point (gas central 2030),
//! and one quarantine flag (battery split) are pinned to the committed
//! file's exact values — a silent edit to the reference fails here.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_core::GridError;
use grid_core::costs::{WaccBand, annuity_per_mw, capital_recovery_factor};
use grid_core::costs_reference::{COSTS_REFERENCE_SCHEMA, CostsReference};
use grid_core::units::{
    AnnualCapacityCost, CapacityCost, CarbonPrice, EmissionsRate, EnergyCapacityCost, Money,
    PerUnit, Power, Price,
};

fn reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/reference/costs-gb.toml")
}

fn reference_text() -> String {
    std::fs::read_to_string(reference_path()).unwrap()
}

fn load() -> CostsReference {
    CostsReference::load(&reference_path()).unwrap()
}

// ---------------------------------------------------------------------
// Pinned regression tests on the committed file (review condition 12).
// ---------------------------------------------------------------------

#[test]
fn schema_string_is_the_adopted_v1() {
    assert_eq!(COSTS_REFERENCE_SCHEMA, "costs-reference-v1");
    // The committed file is the consumed (de-DRAFTed) reference.
    let reference = load();
    assert_eq!(reference.price_base, "real 2024 GBP");
}

#[test]
fn pinned_wacc_set_and_anchors() {
    let reference = load();
    // The docs/04 Stage 7 pin: 4.5 / 7.5 / 10.0 % real, uniform.
    assert_eq!(reference.wacc.set.low, PerUnit::new(0.045));
    assert_eq!(reference.wacc.set.central, PerUnit::new(0.075));
    assert_eq!(reference.wacc.set.high, PerUnit::new(0.100));
    // Unrounded anchors recorded (review condition 1).
    assert_eq!(
        reference.wacc.anchors_unrounded.low_range,
        [PerUnit::new(0.0446), PerUnit::new(0.0467)]
    );
    assert_eq!(
        reference.wacc.anchors_unrounded.central,
        PerUnit::new(0.076)
    );
    assert_eq!(reference.wacc.anchors_unrounded.high, PerUnit::new(0.101));
    // Per-tech sensitivity: all 14 assignments, labelled-sensitivity only.
    assert_eq!(reference.wacc.per_tech_sensitivity.len(), 14);
    assert_eq!(
        reference.wacc.per_tech_sensitivity["offshore_wind_floating"],
        PerUnit::new(0.114)
    );
    assert_eq!(
        reference.wacc.per_tech_sensitivity["interconnector_cap_floor"],
        PerUnit::new(0.076)
    );
}

#[test]
fn pinned_ccgt_capex_row() {
    let reference = load();
    let ccgt = &reference.technologies["ccgt"];
    assert_eq!(
        ccgt.capex_per_kw.low,
        CapacityCost::pounds_per_kilowatt(810.0)
    );
    assert_eq!(
        ccgt.capex_per_kw.central,
        CapacityCost::pounds_per_kilowatt(1020.0)
    );
    assert_eq!(
        ccgt.capex_per_kw.high,
        CapacityCost::pounds_per_kilowatt(1120.0)
    );
    assert!(!ccgt.capex_converted_to_gbp2024); // published 2024 GBP directly
    assert_eq!(
        ccgt.infrastructure_per_kw,
        Some(CapacityCost::pounds_per_kilowatt(14.6))
    );
    assert_eq!(
        ccgt.fom_per_mw_yr,
        AnnualCapacityCost::pounds_per_megawatt_year(16_000.0)
    );
    assert_eq!(
        ccgt.insurance_per_mw_yr,
        AnnualCapacityCost::pounds_per_megawatt_year(2_500.0)
    );
    assert_eq!(
        ccgt.connection_per_mw_yr,
        AnnualCapacityCost::pounds_per_megawatt_year(4_400.0)
    );
    assert_eq!(ccgt.vom_per_mwh, Price::pounds_per_megawatt_hour(5.0));
    assert_eq!(ccgt.efficiency_hhv_new_plant, Some(PerUnit::new(0.54)));
    assert_eq!(ccgt.life_years, 25);
    assert_eq!(ccgt.build_years, 3);
    assert_eq!(ccgt.predev_years, Some(2));
    // Build phasing carried for the (out-of-scope) IDC escalation.
    let phasing: Vec<f64> = ccgt.build_phasing.iter().map(|f| f.value()).collect();
    assert_eq!(phasing, vec![0.35, 0.45, 0.20]);
    assert!(ccgt.quotable);
    assert!(ccgt.bracket_rule.is_none());
}

#[test]
fn pinned_nuclear_row_carries_the_bracket_rule() {
    let reference = load();
    let nuclear = &reference.technologies["nuclear"];
    assert!(nuclear.capex_converted_to_gbp2024); // 2014 GBP × 1.3410
    assert_eq!(
        nuclear.capex_per_kw.central,
        CapacityCost::pounds_per_kilowatt(5820.0)
    );
    assert_eq!(
        nuclear.fuel_per_mwh,
        Some(Price::pounds_per_megawatt_hour(6.7))
    );
    assert_eq!(
        nuclear.decommissioning_waste_per_mwh,
        Some(Price::pounds_per_megawatt_hour(2.7))
    );
    assert_eq!(nuclear.life_years, 60);
    let rule = nuclear.bracket_rule.as_deref().unwrap();
    assert!(rule.contains("nuclear_observed"), "bracket rule: {rule}");

    // The other half of the mandatory bracket.
    let observed = &reference.nuclear_observed;
    assert_eq!(
        observed.capex_per_kw_project_total,
        CapacityCost::pounds_per_kilowatt(11_875.0)
    );
    assert_eq!(observed.capacity, Power::gigawatts(3.2));
    assert!(observed.basis.contains("not overnight"));
    assert!(!observed.bracket_rule.is_empty());
}

#[test]
fn pinned_gas_trajectory_point() {
    let reference = load();
    let gas = &reference.gas_trajectory;
    assert_eq!(gas.years.len(), 18);
    assert_eq!(gas.years.first(), Some(&2025));
    assert_eq!(gas.years.last(), Some(&2050));
    assert_eq!(gas.p_per_therm_to_gbp_per_mwh_hhv, 0.341214);
    // Central 2030 = 71 p/therm × 0.341214 = £24.226194/MWh (HHV).
    let index_2030 = gas.years.iter().position(|&y| y == 2030).unwrap();
    let central_2030 = gas.central[index_2030].as_pounds_per_megawatt_hour();
    assert!(
        (central_2030 - 24.226_194).abs() < 1e-9,
        "gas central 2030 = {central_2030}"
    );
    // Pointwise low ≤ central ≤ high held for every year.
    for i in 0..gas.years.len() {
        assert!(gas.low[i] <= gas.central[i] && gas.central[i] <= gas.high[i]);
    }
}

#[test]
fn pinned_carbon_trajectory_and_cps_convention() {
    let reference = load();
    let carbon = &reference.carbon_trajectory;
    assert_eq!(carbon.years, vec![2025, 2030, 2035, 2040, 2045, 2050]);
    assert_eq!(
        carbon.central.last(),
        Some(&CarbonPrice::pounds_per_tonne_co2(227.8))
    );
    assert_eq!(carbon.cps_nominal, CarbonPrice::pounds_per_tonne_co2(18.0));
    assert!(carbon.cps_convention.contains("deflate"));
}

#[test]
fn pinned_battery_quarantine_flag() {
    let reference = load();
    let battery = &reference.battery;
    // Review condition 3 quarantine LIFTED 2026-07-06 (reviewed act:
    // condition 3.i discharged against the NREL primary,
    // NREL/TP-6A40-93281 — evidence committed 23676f1; this revision is
    // the coordinated lift). The numeric pins below are UNMOVED: the
    // re-verification CONFIRMED the committed values.
    assert!(battery.quotable);
    assert_eq!(
        battery.power_per_kw_2030_build,
        CapacityCost::pounds_per_kilowatt(262.0)
    );
    assert_eq!(
        battery.energy_per_kwh_2030_build,
        EnergyCapacityCost::pounds_per_kilowatt_hour(135.0)
    );
    // £12.9/kW/yr converted to the canonical £/MW/yr at parse.
    assert_eq!(
        battery.fom_per_mw_yr,
        AnnualCapacityCost::pounds_per_megawatt_year(12_900.0)
    );
    assert_eq!(battery.life_years, 15);
    assert_eq!(battery.round_trip_efficiency, PerUnit::new(0.85));
    assert!(battery.staleness_stamp.contains("2018"));
}

#[test]
fn interconnector_rows_carry_machine_readable_quarantine() {
    let reference = load();
    let viking = &reference.interconnectors["viking_link"];
    assert!(viking.verified && viking.quotable);
    assert_eq!(viking.capex, Some(Money::pounds(1.7e9)));
    assert_eq!(viking.capacity, Power::gigawatts(1.4));

    for key in ["north_sea_link", "neuconnect", "greenlink"] {
        let row = &reference.interconnectors[key];
        assert!(!row.verified && !row.quotable, "{key} must be quarantined");
        assert_eq!(row.capex, None, "{key} carries no consumable GBP capex");
    }
}

#[test]
fn holding_costs_join_the_response_holdings_keys() {
    let reference = load();
    let holding = &reference.holding_costs;
    assert_eq!(holding.unit, "GBP/MW/h");
    let dcl = &holding.services["dynamic_containment_lf"];
    assert_eq!(dcl.central, Price::pounds_per_megawatt_hour(3.31));
    assert_eq!(
        dcl.range_p5_p95,
        [
            Price::pounds_per_megawatt_hour(0.89),
            Price::pounds_per_megawatt_hour(7.29)
        ]
    );
    assert_eq!(
        holding.services["dynamic_regulation_lf"].central,
        Price::pounds_per_megawatt_hour(14.19)
    );
    // Negative DRH is genuine and preserved.
    assert_eq!(
        holding.high_frequency.drh,
        Price::pounds_per_megawatt_hour(-3.42)
    );
}

#[test]
fn ocht_row_carries_the_publication_gate_and_labelled_sensitivity() {
    let reference = load();
    let ocht = &reference.hydrogen_reconversion_ocht;
    assert!(ocht.publication_gate.contains("Baringa"));
    assert_eq!(ocht.efficiency_hhv_default, PerUnit::new(0.25));
    assert_eq!(
        ocht.efficiency_hhv_sensitivity_labelled,
        PerUnit::new(0.296)
    );
    assert_eq!(
        ocht.capex_per_kw.central,
        CapacityCost::pounds_per_kilowatt(850.0)
    );
}

#[test]
fn cavern_row_carries_the_binding_cycling_convention() {
    let reference = load();
    let cavern = &reference.hydrogen_cavern;
    assert_eq!(
        cavern.levelised_per_mwh_h2_hhv,
        Price::pounds_per_megawatt_hour(6.9)
    );
    assert_eq!(cavern.cycles_per_year_assumed, 9);
    assert!(cavern.binding_convention.contains("9 cycles/yr"));
}

#[test]
fn electrolyser_row_parses() {
    let reference = load();
    let electrolyser = &reference.electrolyser;
    assert_eq!(
        electrolyser.capex_per_kwe_2030_central,
        CapacityCost::pounds_per_kilowatt(518.0)
    );
    assert_eq!(
        electrolyser.vom_per_mwh_h2,
        Price::pounds_per_megawatt_hour(3.6)
    );
    assert_eq!(electrolyser.life_years, 30);
}

#[test]
fn emission_factors_close_the_rule_9_gap() {
    let reference = load();
    let factors = &reference.emission_factors;
    assert_eq!(
        factors.coal_co2_per_mwh_th_hhv,
        EmissionsRate::tonnes_per_megawatt_hour(0.31530)
    );
    // Biogenic CO2 zero-rated for pricing; non-CO2 carried for accounting.
    assert_eq!(
        factors.biomass_co2_for_pricing,
        EmissionsRate::tonnes_per_megawatt_hour(0.0)
    );
    assert_eq!(
        factors.biomass_co2e_non_co2,
        EmissionsRate::tonnes_per_megawatt_hour(0.01132)
    );
    // The mandatory "other"-residual convention (review condition 7).
    assert!(factors.other_convention.contains("residual"));
}

#[test]
fn deflator_table_parses() {
    let reference = load();
    assert_eq!(reference.deflator.to_2024["y2014"], PerUnit::new(1.3410));
    assert_eq!(reference.deflator.to_2024.len(), 6);
    assert!(reference.deflator.licence.contains("OGL"));
}

// ---------------------------------------------------------------------
// Structured error cases: the parser names what is wrong.
// ---------------------------------------------------------------------

fn expect_invalid(text: &str, needle: &str) {
    match CostsReference::from_toml_str(text) {
        Err(GridError::InvalidCostsReference { reason }) => {
            assert!(
                reason.contains(needle),
                "error {reason:?} does not name {needle:?}"
            );
        }
        Err(other) => panic!("expected InvalidCostsReference, got {other}"),
        Ok(_) => panic!("expected an error naming {needle:?}"),
    }
}

#[test]
fn missing_schema_is_a_structured_error() {
    expect_invalid("price_base = \"real 2024 GBP\"\n", "schema");
}

#[test]
fn wrong_schema_is_reported_as_such() {
    let text = reference_text().replace(
        "schema = \"costs-reference-v1\"",
        "schema = \"costs-reference-v2\"",
    );
    expect_invalid(&text, "costs-reference-v2");
}

#[test]
fn unknown_fields_are_rejected() {
    let mut text = reference_text();
    text.push_str("\nsurprise_field = 1\n");
    match CostsReference::from_toml_str(&text) {
        Err(GridError::CostsReferenceParse { .. }) => {}
        other => panic!("expected CostsReferenceParse, got {other:?}"),
    }
}

#[test]
fn phasing_array_must_sum_to_one_within_tolerance() {
    // Break the CCGT build phasing (0.35+0.45+0.20 → 0.35+0.45+0.10).
    let text = reference_text().replace(
        "build_phasing = [0.35, 0.45, 0.20]",
        "build_phasing = [0.35, 0.45, 0.10]",
    );
    expect_invalid(&text, "build_phasing");
}

#[test]
fn as_published_phasing_rounding_is_tolerated() {
    // The committed file parses even though onshore predev phasing sums
    // to 0.98 and biomass to 0.99 (as published) — the tolerance is a
    // documented parser constant, not silent laxity.
    let _ = load();
}

#[test]
fn every_phasing_array_is_pinned() {
    // The ±0.025 sum tolerance alone would let a silently edited array
    // summing to ~1 parse (pkg-1 review note 5), so every phasing array
    // in the committed file is pinned exactly here — a silent edit
    // fails this test even when it passes the sum check. The arrays
    // are the rule-4 IDC escalation inputs (review condition 11).
    let reference = load();
    let assert_phasing = |what: &str, got: Option<&Vec<PerUnit>>, pinned: &[f64]| {
        let got: Vec<f64> = got
            .unwrap_or_else(|| panic!("{what}: phasing array missing"))
            .iter()
            .map(|f| f.value())
            .collect();
        assert_eq!(got, pinned, "{what}");
    };
    let tech = |key: &str| &reference.technologies[key];

    assert_phasing(
        "ccgt predev",
        tech("ccgt").predev_phasing.as_ref(),
        &[0.43, 0.57],
    );
    assert_phasing(
        "ccgt build",
        Some(&tech("ccgt").build_phasing),
        &[0.35, 0.45, 0.20],
    );
    assert_phasing(
        "ocgt predev",
        tech("ocgt").predev_phasing.as_ref(),
        &[0.66, 0.34],
    );
    assert_phasing(
        "ocgt build",
        Some(&tech("ocgt").build_phasing),
        &[0.66, 0.34],
    );
    assert_phasing(
        "nuclear predev",
        tech("nuclear").predev_phasing.as_ref(),
        &[0.20, 0.20, 0.20, 0.20, 0.20],
    );
    assert_phasing(
        "nuclear build",
        Some(&tech("nuclear").build_phasing),
        &[0.05, 0.05, 0.20, 0.20, 0.20, 0.20, 0.05, 0.05],
    );
    assert_phasing(
        "onshore_wind predev",
        tech("onshore_wind").predev_phasing.as_ref(),
        &[0.13, 0.13, 0.13, 0.13, 0.13, 0.33],
    );
    assert_phasing(
        "onshore_wind build",
        Some(&tech("onshore_wind").build_phasing),
        &[0.57, 0.43],
    );
    assert_phasing(
        "offshore_wind predev",
        tech("offshore_wind").predev_phasing.as_ref(),
        &[0.10, 0.10, 0.15, 0.15, 0.20, 0.30],
    );
    assert_phasing(
        "offshore_wind build",
        Some(&tech("offshore_wind").build_phasing),
        &[0.35, 0.30, 0.35],
    );
    assert_phasing(
        "solar_pv predev",
        tech("solar_pv").predev_phasing.as_ref(),
        &[0.37, 0.37, 0.26],
    );
    assert_phasing(
        "solar_pv build",
        Some(&tech("solar_pv").build_phasing),
        &[0.80, 0.20],
    );
    assert_phasing(
        "biomass predev",
        tech("biomass").predev_phasing.as_ref(),
        &[0.33, 0.33, 0.33],
    );
    assert_phasing(
        "biomass build",
        Some(&tech("biomass").build_phasing),
        &[0.46, 0.54],
    );
    assert_phasing(
        "battery build",
        Some(&reference.battery.build_phasing),
        &[1.0],
    );
    assert_phasing(
        "electrolyser build",
        Some(&reference.electrolyser.build_phasing),
        &[0.3333, 0.3333, 0.3334],
    );
    assert_phasing(
        "ocht predev",
        Some(&reference.hydrogen_reconversion_ocht.predev_phasing),
        &[0.56, 0.44],
    );
    assert_phasing(
        "ocht build",
        Some(&reference.hydrogen_reconversion_ocht.build_phasing),
        &[0.66, 0.34],
    );
}

// ---------------------------------------------------------------------
// The two load-bearing quarantine guards (pkg-1 review condition 1):
// the machine-readable flags of the docs/04 Stage 7 pin cannot be
// weakened by a silent reference edit.
// ---------------------------------------------------------------------

#[test]
fn battery_quotability_is_byte_pinned_to_the_discharge_citation() {
    // SUCCESSOR to `battery_quarantine_cannot_be_lifted_silently`: the
    // condition-3 quarantine was lifted 2026-07-06 as a reviewed act
    // (condition 3.i discharged against the NREL primary,
    // NREL/TP-6A40-93281; evidence committed 23676f1), so the parser
    // guard that test relied on is gone BY THAT SAME REVIEWED ACT. The
    // anti-silent-change discipline now points at the new state: the
    // lifted flag line, WITH its discharge citation, is byte-pinned —
    // re-quarantining the row, dropping the citation, or any other
    // edit to this line is a knowing re-review, not an edit.
    let flag_line = "quotable = true                           # condition 3.i DISCHARGED \
                     2026-07-06: NREL/TP-6A40-93281 primary re-verified \
                     (costs-evidence.sha256); lift = this reviewed revision; caveats \
                     3.ii/3.iii REMAIN";
    let occurrences = reference_text().matches(flag_line).count();
    assert_eq!(
        occurrences, 1,
        "the battery quotable flag line moved (found {occurrences} exact matches) — \
         changing it requires a reviewed reference revision"
    );
    // And the parsed flag agrees with the pinned bytes.
    assert!(load().battery.quotable);
}

#[test]
fn quarantined_interconnector_with_a_point_capex_is_rejected() {
    // A quarantined (verified = false / quotable = false) interconnector
    // row must not expose a consumable point GBP capex (review
    // condition 9): give North Sea Link one and the parse must fail,
    // naming the row. (Literal updated 2026-07-06 with the recorded
    // NSL estimate correction 2.0 -> 1.6, Statnett owner primary — the
    // row itself stays quarantined; see the TOML comment.)
    let text = reference_text().replace("capex_eur_bn_estimated = 1.6", "capex_gbp_bn = 1.6");
    assert_ne!(text, reference_text(), "mutation did not apply");
    expect_invalid(&text, "north_sea_link");
}

#[test]
fn wacc_set_must_be_ordered() {
    let text = reference_text().replace("real_low = 0.045", "real_low = 0.145");
    expect_invalid(&text, "wacc");
}

#[test]
fn negative_fixed_costs_are_rejected() {
    let text = reference_text().replace("fom_gbp_per_mw_yr = 16000", "fom_gbp_per_mw_yr = -16000");
    expect_invalid(&text, "fom");
}

#[test]
fn capex_bracket_must_be_ordered() {
    let text = reference_text().replace(
        "capex_gbp_per_kw = [810, 1020, 1120]",
        "capex_gbp_per_kw = [1810, 1020, 1120]",
    );
    expect_invalid(&text, "capex");
}

#[test]
fn trajectory_arrays_must_match_the_year_axis() {
    let text = reference_text().replace(
        "years =   [2025, 2030, 2035, 2040, 2045, 2050]",
        "years =   [2025, 2030, 2035, 2040, 2045]",
    );
    expect_invalid(&text, "carbon");
}

// ---------------------------------------------------------------------
// Annualisation (D8 rule 4): CRF = r(1+r)^n / ((1+r)^n − 1).
// ---------------------------------------------------------------------

#[test]
fn crf_hand_checkable_values() {
    // One-year life: repay principal plus one year's interest.
    let one_year = capital_recovery_factor(PerUnit::new(0.10), 1).unwrap();
    assert!((one_year.value() - 1.10).abs() < 1e-12);
    // Pinned central-WACC CCGT-life value.
    let central_25 = capital_recovery_factor(PerUnit::new(0.075), 25).unwrap();
    assert!((central_25.value() - 0.089_710_671_649_444_02).abs() < 1e-15);
}

#[test]
fn crf_rejects_degenerate_inputs() {
    assert!(capital_recovery_factor(PerUnit::new(0.0), 25).is_err());
    assert!(capital_recovery_factor(PerUnit::new(-0.05), 25).is_err());
    assert!(capital_recovery_factor(PerUnit::new(1.5), 25).is_err());
    assert!(capital_recovery_factor(PerUnit::new(0.075), 0).is_err());
}

#[test]
fn pinned_ccgt_annuity_at_the_three_waccs() {
    // CCGT central overnight capex incl. site infrastructure:
    // (1020 + 14.6) £/kW over 25 years.
    let reference = load();
    let ccgt = &reference.technologies["ccgt"];
    let capex = ccgt.total_overnight_capex_central();
    assert_eq!(capex, CapacityCost::pounds_per_kilowatt(1034.6));

    let band = WaccBand {
        low: annuity_per_mw(capex, reference.wacc.set.low, ccgt.life_years).unwrap(),
        central: annuity_per_mw(capex, reference.wacc.set.central, ccgt.life_years).unwrap(),
        high: annuity_per_mw(capex, reference.wacc.set.high, ccgt.life_years).unwrap(),
    };
    let expect = |value: AnnualCapacityCost, pinned: f64| {
        let got = value.as_pounds_per_megawatt_year();
        assert!(
            ((got - pinned) / pinned).abs() < 1e-9,
            "annuity {got} vs pinned {pinned}"
        );
    };
    expect(band.low, 69_772.418_409);
    expect(band.central, 92_814.660_889);
    expect(band.high, 113_979.887_488);
    assert!(band.low < band.central && band.central < band.high);
}
