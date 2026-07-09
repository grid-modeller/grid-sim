//! D11 priced-ladder acceptance (docs/notes/d11-priced-dispatch.md
//! rule 4): the tier-2 priced flow signal on the 2024 five-zone
//! scenario, measured against the superseded Stage 5 ≥ 95 % A2a
//! direction-match target (expectation 97.4 % — the priced-ladder
//! target docs/04 retained when the two-limb A2 gate was re-pinned).
//!
//! The committed scenario keeps `flow_signal = "scarcity"` (its
//! dispatch is byte-identical to the v6 pins); the acceptance run here
//! flips the signal in-memory — the B4-LP in-memory-variant precedent.
//!
//! Phase-0 context (docs/notes/d11-a2a-mismatch-characterisation.md):
//! the 2024 GB-vs-EU carbon wedge is ~nil (+£0.17/tCO2 — carbon-parity
//! year), so the ladder's lever on the dominant both-gas-marginal
//! mismatch class is EMPTY, and the committed data's granularity
//! asymmetry (a stepped GB UKA+CPS series against a flat annual EUA
//! mean) makes the residual per-period wedge SIGN-FLIP through the
//! year. The ≥ 95 % target is therefore expected UNREACHABLE on 2024
//! prices — the pre-registered D11 rule-4 outcome, named, not tuned
//! away.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::OnceLock;

use grid_adequacy::{MultiZoneRunResult, load_multi_zone_inputs, run_multi};
use grid_core::scenario::{FlowSignal, Scenario, ZonePricingSpec};
use grid_core::trace::load_power_trace_mw;
use grid_core::units::{Duration, Energy, Power};

const SCENARIO: &str = "scenarios/gb-2024-5zone.toml";
const PERIODS: usize = 17_568;
/// The A2 dead-band, GW (50 MW) — as acceptance_stage5_2024.rs.
const DEAD_BAND_GW: f64 = 0.05;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Fail loudly (red, not skipped) if the packs are missing — the
/// Stage 5 discipline (see acceptance_stage5_2024.rs for the full
/// build instructions).
fn require_packs() {
    let root = repo_root();
    for rel in [
        "data/packs/2024/processed/demand_2024.parquet",
        "data/packs/2024/processed/gas_sap_daily_2024.parquet",
        "data/packs/entsoe-2024/processed/load_fr_2024.parquet",
        "data/packs/entsoe-2024/processed/flows_gb_entsoe_2024.parquet",
        "data/packs/cf-eu-1985-2024.sha256",
    ] {
        assert!(
            root.join(rel).exists(),
            "data pack file missing: {rel} — build the 2024 / entsoe-2024 / cf-eu packs \
             first (scripts/fetch-2024, scripts/era5-cf, scripts/fetch-entsoe). These D11 \
             acceptance tests stay RED until the packs exist."
        );
    }
}

/// The 5-zone scenario with the flow signal flipped to the priced
/// ladder in-memory (the committed file stays on the scarcity default
/// so every v6-era pin is byte-identical).
fn priced_scenario() -> Scenario {
    let mut scenario = Scenario::load(&repo_root().join(SCENARIO)).unwrap();
    scenario.dispatch.flow_signal = FlowSignal::PricedLadder;
    scenario
}

/// The priced-ladder acceptance run.
fn run_priced() -> &'static MultiZoneRunResult {
    static RESULT: OnceLock<MultiZoneRunResult> = OnceLock::new();
    RESULT.get_or_init(|| {
        require_packs();
        let root = repo_root();
        let scenario = priced_scenario();
        let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
        run_multi(&scenario, &inputs).unwrap()
    })
}

fn twh(energy: Energy) -> f64 {
    energy.as_gigawatt_hours() / 1000.0
}

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

fn observed_border_net_gw(column: &str) -> Vec<f64> {
    let path = repo_root().join("data/packs/entsoe-2024/processed/flows_gb_entsoe_2024.parquet");
    load_power_trace_mw(&path, column, PERIODS)
        .unwrap()
        .values()
        .iter()
        .map(|p| p.as_gigawatts())
        .collect()
}

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
// The A2a ≥ 95 % priced-ladder target (docs/04, superseded Stage 5
// pin retained as the tier-2 acceptance bar) — MEASURED, MISSED, and
// converted per D11 rule 4: "Miss → the priced ladder is not adequate
// and the finding is named (not re-pinned)."
//
// THE PRE-REGISTERED FINDING (named in full in
// docs/notes/d11-a2a-mismatch-characterisation.md): on committed 2024
// price data the priced ladder measures A2a = 12,595/17,568 =
// **71.69 %** — a REGRESSION from the scarcity rule's 90.07 %, not an
// improvement toward 97.4 %. The target was written when the GB
// carbon-floor premium was assumed real; the D11 data package showed
// 2024 is a carbon-parity year (wedge +£0.17/tCO2 ≈ nil), and Phase 0
// showed the A2a residual is dominated by genuinely-both-gas-marginal
// periods where the only surviving per-zone price difference is a
// GRANULARITY ARTIFACT: the committed GB carbon is a fortnightly
// UKA-auction step series (+CPS) while the only licence-clean EUA
// figure is a flat annual mean, so the sub-noise wedge SIGN-FLIPS
// through the year (GB dearer in 44.5 % of periods) and — bang-bang on
// flat bands being the ladder's intended behaviour — decides thousands
// of both-gas direction calls on noise. The target is UNREACHABLE on
// 2024 prices; the honest cause is the year's carbon parity plus the
// data granularity, not an engine defect. The committed Stage 5 A2
// two-limb band-gate (scarcity rule) is untouched and still passing.
// ---------------------------------------------------------------------

/// Pinned priced-ladder A2a match count on committed 2024 data
/// (2026-07-05): 12,595/17,568 = 71.69 %.
const PINNED_A2A_PRICED_MATCHES: usize = 12_595;

#[test]
fn a2a_under_the_priced_ladder_misses_the_target_and_is_pinned_as_the_finding() {
    let modelled = border_net_gw(run_priced(), &["IFA", "IFA2", "ElecLink"]);
    let observed = observed_border_net_gw("fr_net");
    let mut confusion = [[0usize; 3]; 3];
    for (m, o) in modelled.iter().zip(&observed) {
        confusion[(direction(*m) + 1) as usize][(direction(*o) + 1) as usize] += 1;
    }
    let matches = confusion[0][0] + confusion[1][1] + confusion[2][2];
    let rate = matches as f64 / PERIODS as f64;
    eprintln!(
        "D11 A2a (priced ladder): {matches}/{PERIODS} = {:.2} % (scarcity rule: 90.07 %; \
         target ≥ 95 % MISSED — the pre-registered rule-4 finding)",
        100.0 * rate
    );
    eprintln!("confusion (rows model exp/idle/imp; cols observed):");
    for (label, row) in ["export", "idle  ", "import"].iter().zip(&confusion) {
        eprintln!("  model {label}: {:>6} {:>6} {:>6}", row[0], row[1], row[2]);
    }
    // The regression pin: a deliberate change to scenario, packs or
    // engine is a knowing re-pin (the Stage 5 A2a pin discipline).
    assert_eq!(
        matches, PINNED_A2A_PRICED_MATCHES,
        "priced-ladder A2a moved from the pinned finding value"
    );
    // The finding's shape, asserted so the record cannot silently
    // rot: the ladder measures BELOW both the target and the scarcity
    // rule on 2024 data.
    assert!(
        rate < 0.95,
        "the rule-4 finding no longer holds: re-adjudicate"
    );
}

/// SENSITIVITY (measured for the characterisation note, pinned): with
/// BOTH carbon bases flattened to their 2024 annual means (GB
/// UKA-mean + CPS = £55.18 vs EUA £55.01 — the equally-defensible
/// granularity-consistent convention), the +£0.17/tCO2 wedge is
/// constant and GB-dearer, so the ladder pulls FR→GB in every
/// both-gas period. MEASURED: 16,370/17,568 = **93.18 %** — better
/// than the committed-convention 71.69 %, still BELOW both the 95 %
/// target and the static 97.4 % expectation (the bang-bang cap-pinned
/// imports the constant wedge forces also flip previously-matching
/// export/idle periods). Two conclusions, both part of the rule-4
/// finding: (i) the A2a outcome on 2024 data is decided by the
/// carbon-series CONVENTION — a wedge an order of magnitude below the
/// data's own uncertainty moves A2a by ~21.5 pp — so neither number
/// validates a price model; (ii) the ≥ 95 % target is unreachable
/// under EITHER convention.
#[test]
fn a2a_sensitivity_flat_flat_carbon_is_pinned() {
    require_packs();
    let root = repo_root();
    let mut scenario = priced_scenario();
    // GB flat carbon = the committed UKA 2024 auction mean + CPS
    // (prices-eu-2024.toml [carbon.wedge] gb_carbon_gbp_per_tco2).
    for zone in &mut scenario.zones {
        if zone.id.as_str() == "GB" {
            zone.pricing.as_mut().unwrap().carbon_flat_gbp_per_tco2 =
                Some(grid_core::units::CarbonPrice::pounds_per_tonne_co2(55.18));
        }
    }
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let result = run_multi(&scenario, &inputs).unwrap();
    let modelled = border_net_gw(&result, &["IFA", "IFA2", "ElecLink"]);
    let observed = observed_border_net_gw("fr_net");
    let matches = modelled
        .iter()
        .zip(&observed)
        .filter(|(m, o)| direction(**m) == direction(**o))
        .count();
    eprintln!(
        "D11 A2a sensitivity (flat-flat carbon, GB 55.18 vs EUA 55.01): {matches}/{PERIODS} \
         = {:.2} %",
        100.0 * matches as f64 / PERIODS as f64
    );
    // Pinned: 16,370/17,568 = 93.18 % (2026-07-05) — below the 95 %
    // target under the ladder-favourable convention too.
    assert_eq!(
        matches, 16_370,
        "flat-flat sensitivity moved from the pinned finding value"
    );
}

// ---------------------------------------------------------------------
// Ladder-run gate measurements (D11 rule 4: the ladder must not
// regress the Stage 5 gates — measured and reported here).
// ---------------------------------------------------------------------

#[test]
fn ladder_run_gate_quantities_are_measured_and_reported() {
    let result = run_priced();
    let gb = result.zone("GB").unwrap();
    let imports = twh(gb.net_imports_energy());
    let gas = twh(gb.thermal_energy("ccgt").unwrap()) + twh(gb.thermal_energy("ocgt").unwrap());
    eprintln!("D11 ladder run: GB net imports {imports:+.2} TWh (A1 band 33.30 ± 10 %)");
    eprintln!("D11 ladder run: GB gas {gas:.2} TWh (A1 band 72.79 ± 5 %)");
    // Pinned (2026-07-05, the characterisation note's regression
    // record — every published number gets a pin): both A1 quantities
    // sit OUTSIDE their bands under the ladder; that is the finding.
    // Re-pinned 2026-07-06 (R7 flow-walk stall fix, docs/08 R7): was
    // imports +25.70 / gas 82.20 — the finding (outside the A1 bands)
    // is unchanged.
    assert!(
        (imports - 25.72).abs() < 0.01,
        "ladder GB net imports {imports:.3} moved from the pinned +25.72 TWh"
    );
    assert!(
        (gas - 82.17).abs() < 0.01,
        "ladder GB gas {gas:.3} moved from the pinned 82.17 TWh"
    );
    let energy = |names: &[&str]| -> f64 {
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
    // FR re-pinned 2026-07-06 (R7 stall fix, docs/08 R7): was +12.99;
    // the other five borders moved within the ±0.01 pin tolerance and
    // the A2b recall record (838/1,312) did not move.
    for (name, links, actual, pinned) in [
        ("FR", &["IFA", "IFA2", "ElecLink"][..], 19.45, 13.01),
        ("BE (Nemo)", &["Nemo"][..], 4.16, 0.79),
        ("NL (BritNed)", &["BritNed"][..], 1.59, 0.79),
        ("NO2 (NSL)", &["NSL"][..], 9.62, 9.49),
        ("DK1 (Viking)", &["Viking"][..], 3.66, 2.53),
        ("IE-SEM", &["Moyle", "EWIC", "Greenlink"][..], -5.18, -0.90),
    ] {
        let modelled = energy(links);
        eprintln!("D11 ladder run: {name:<14} {modelled:+7.2} vs NESO {actual:+7.2} TWh");
        assert!(
            (modelled - pinned).abs() < 0.01,
            "{name}: ladder net {modelled:.3} moved from the pinned {pinned:+.2} TWh"
        );
    }
    // A2b export recall under the ladder (band ≥ 70 % on the scarcity
    // rule).
    let modelled = border_net_gw(result, &["IFA", "IFA2", "ElecLink"]);
    let observed = observed_border_net_gw("fr_net");
    let (mut exports, mut recalled) = (0usize, 0usize);
    for (m, o) in modelled.iter().zip(&observed) {
        if direction(*o) == -1 {
            exports += 1;
            if direction(*m) == -1 {
                recalled += 1;
            }
        }
    }
    eprintln!(
        "D11 ladder run: A2b export recall {recalled}/{exports} = {:.2} % (band ≥ 70 %)",
        100.0 * recalled as f64 / exports as f64
    );
    // Pinned: 838/1,312 = 63.87 % — below the ≥ 70 % band under the
    // ladder (part of the rule-4 regression record).
    assert_eq!(
        (recalled, exports),
        (838, 1_312),
        "ladder A2b counts moved from the pinned finding values"
    );
    // A3 continental-imports anticorrelation under the ladder (band
    // r ≤ −0.25 on the scarcity rule).
    let continental = border_net_gw(result, &["IFA", "IFA2", "ElecLink", "Nemo", "BritNed"]);
    let wind: Vec<f64> = grid_core::trace::load_per_unit_trace(
        &repo_root().join("data/packs/2024/processed/wind_cf_2024.parquet"),
        "wind_cf",
        PERIODS,
    )
    .unwrap()
    .values()
    .iter()
    .map(|v| v.value())
    .collect();
    let pearson = |x: &[f64], y: &[f64]| -> f64 {
        let n = x.len() as f64;
        let mx = x.iter().sum::<f64>() / n;
        let my = y.iter().sum::<f64>() / n;
        let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
        let sx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
        let sy: f64 = y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt();
        sxy / (sx * sy)
    };
    let r = pearson(&continental, &wind);
    eprintln!("D11 ladder run: A3 continental imports vs GB wind r = {r:.3} (band ≤ −0.25)");
    // Pinned: r = −0.185 — outside the ≤ −0.25 band under the ladder
    // (part of the rule-4 regression record). The committed Stage 5
    // gates continue to run (and pass) on the scarcity rule.
    assert!(
        (r - (-0.185)).abs() < 0.001,
        "ladder A3 r {r:.4} moved from the pinned −0.185"
    );
}

// ---------------------------------------------------------------------
// Property (c) of the work order: the single-zone reference digest
// (779d7444…, grid-cli/tests/regression_2024.rs) CANNOT move under the
// priced ladder — a single-zone scenario has zero borders, so the flow
// rule (either signal) is UNREACHABLE (`multizone.rs` links_live =
// zones.len() > 1), not merely "byte-untouched by convention". Proven
// here by running the pinned reference scenario under the ladder and
// asserting bit-identity with the frozen single-zone path.
// ---------------------------------------------------------------------

#[test]
fn priced_ladder_on_the_single_zone_reference_is_bit_identical_to_run() {
    require_packs();
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join("scenarios/gb-2024-reference.toml")).unwrap();
    let reference = {
        let inputs = grid_adequacy::load_run_inputs(&scenario, &root).unwrap();
        grid_adequacy::run(&scenario, &inputs).unwrap()
    };
    // Flip to the priced ladder, mirroring the top-level [pricing]
    // block into the zone (validation requires it; the flow rule can
    // never consult it — zero borders).
    scenario.dispatch.flow_signal = FlowSignal::PricedLadder;
    let pricing = scenario.pricing.clone().unwrap();
    scenario.zones[0].pricing = Some(ZonePricingSpec {
        reference: pricing.reference,
        carbon_flat_gbp_per_tco2: None,
        fuel_price: pricing.fuel_price,
        srmc: pricing.srmc,
    });
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let result = run_multi(&scenario, &inputs).unwrap();
    assert_eq!(result.zones.len(), 1);
    assert!(
        result.zones[0].result == reference,
        "priced ladder perturbed a single-zone run — the flow rule must be unreachable"
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
// Property (b) at acceptance scale: the full 5-zone priced-ladder run
// is bit-identical on rerun (ADR-5).
// ---------------------------------------------------------------------

#[test]
fn five_zone_priced_ladder_rerun_is_bit_identical() {
    require_packs();
    let root = repo_root();
    let scenario = priced_scenario();
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let first = run_multi(&scenario, &inputs).unwrap();
    let second = run_multi(&scenario, &inputs).unwrap();
    assert!(first == second, "priced-ladder reruns differ (ADR-5)");
}

// ---------------------------------------------------------------------
// Drift guard (the Stage 1 scenario-calibration pattern): the flat
// per-zone carbon level written in the scenario must equal the
// committed EU price reference — the single committed source cannot
// drift from a second transcription.
// ---------------------------------------------------------------------

#[test]
fn scenario_flat_carbon_matches_the_committed_eu_reference() {
    let root = repo_root();
    let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    // Parse the committed reference (a documentation-first TOML; only
    // the one load-bearing value is read here).
    let text = std::fs::read_to_string(root.join("data/reference/prices-eu-2024.toml")).unwrap();
    let value: toml::Value = toml::from_str(&text).unwrap();
    let eua_gbp = value["carbon"]["eua"]["average_2024_gbp"]
        .as_float()
        .unwrap();
    for zone in &scenario.zones {
        let Some(pricing) = &zone.pricing else {
            panic!("zone {} has no pricing block", zone.id);
        };
        match zone.id.as_str() {
            "GB" => assert_eq!(
                pricing.carbon_flat_gbp_per_tco2, None,
                "GB prices carbon at the reference UKA+CPS step series"
            ),
            _ => {
                let flat = pricing
                    .carbon_flat_gbp_per_tco2
                    .unwrap_or_else(|| panic!("zone {}: flat carbon missing", zone.id))
                    .as_pounds_per_tonne_co2();
                assert!(
                    (flat - eua_gbp).abs() < 1e-12,
                    "zone {}: scenario flat carbon {flat} != committed EUA {eua_gbp}",
                    zone.id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// The energy identity of the priced run still closes (per-zone
// conservation is engine-enforced; this asserts the run completes and
// the border energies are finite — the smoke half of the gate table).
// ---------------------------------------------------------------------

#[test]
fn priced_run_border_energies_are_finite() {
    let result = run_priced();
    for link in &result.links {
        let net = link
            .home_end
            .iter()
            .map(|&p| p * Duration::half_hour())
            .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e);
        assert!(net.as_gigawatt_hours().is_finite(), "{}", link.name);
    }
}
