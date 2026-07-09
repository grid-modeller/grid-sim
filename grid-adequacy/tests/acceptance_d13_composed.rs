//! D13 PACKAGE 1 acceptance — the COMPOSED boundary-trade scenario
//! (`scenarios/gb-2024-8zone.toml`): the committed 3-zone GB boundary
//! family (NSCO/SSCO/RGB, B4+B6) joined to the committed 5-zone
//! external set (FR, CONT-NW, NO2, DK1, IE-SEM), per the ADOPTED design
//! `docs/notes/d13-composed-boundary-trade.md` (adjudication
//! `d13-composed-boundary-trade-review.md`). This file carries the
//! design's rule-8 pre-registered acceptance criteria (i)–(v) for the
//! ANCHOR (factor 1.0); the 60 GW measurement is package 2, gated on
//! this package's review.
//!
//! # >>> GATES 8(i) AND 8(ii)-B4 MEASURED RED (2026-07-05) — LINE
//! # STOPPED; VERDICT WITHHELD PENDING REVIEWER ADJUDICATION <<<
//!
//! The pre-registered anchor gates FAILED on first measurement, and per
//! rule 8 ("a red stops the line — the D11 withhold-and-report
//! discipline") the composed record is NOT validated and NO 60 GW run
//! may happen until the reviewer adjudicates. Following the D11
//! conversion precedent (acceptance_d11_sweep.rs: the pre-registered
//! miss branch fired → the measured values are pinned IN THAT SHAPE and
//! the framing withheld), this file pins the measured deviations so the
//! record cannot silently rot:
//!
//! - **8(i) RED**: composed-anchor GB-aggregate gas 74.960 TWh =
//!   **+4.41 %** vs the pre-fix committed 5-zone anchor (band ±2 %;
//!   75.019/+4.49 % pre-R7-fix); net imports +42.473 TWh = **+18.2 %**
//!   vs the pre-registered +35.935 (band ±5 %; +42.428/+18.1 %
//!   pre-fix) and **+27.5 %** vs the observed 33.30 (the outright A1
//!   ±10 % gate also fails; +27.4 % pre-fix). DIAGNOSIS (mechanical decomposition, this session): NOT a
//!   composition defect — the composition identities below all hold,
//!   and the committed 3-zone family run STANDALONE already reads
//!   GB-aggregate gas 82.42 TWh / curtailment 7.01 TWh: the committed
//!   single-pass equal-depth dispatcher strands ~7 TWh of northern
//!   surplus AT COPPER PLATE (the committed 3-zone finding 1 — 6.90 TWh
//!   stranded with UNBOUNDED B4), which the 5-zone copper-plate GB
//!   anchor never saw. Composing the externals recovers part of it
//!   (imports +6.5 TWh, gas −7.4 TWh vs standalone), landing outside
//!   the band. The design's ±2 % derivation bounded only
//!   BOUNDARY-BINDING energy (~1.5 TWh) and missed the committed
//!   copper-plate stranding artefact; the tolerance is pre-registered
//!   and NOT re-pinned here — the gate verdict is the reviewer's.
//! - **8(ii) branch (b) RED for B4**: composed-anchor B4 rule-based
//!   binding 0.0107 DECREASED from the committed 0.0195 (B6 rose
//!   modestly, 0.0385 vs 0.0335 — its branch (a)). DIAGNOSIS: the
//!   decomposition test below proves the modelled external links have
//!   ZERO effect on B4/B6 rule-based binding (bit-identical binding
//!   with the ten external links deleted): under the committed
//!   single-pass walk with the adopted declaration order, B4 and B6
//!   clear BEFORE any external border, so the design's expected-UP
//!   mechanism ("the export channel drains RGB in the £0 periods where
//!   B4/B6 bind") is UNREACHABLE by construction on the rule-based
//!   leg. The whole shift comes from the OTHER stated surgery —
//!   removing the observed net_imports padding from SSCO/RGB (RGB
//!   scarcer → B6 UP; SSCO's position shifts → B4 DOWN).
//!
//! What this file therefore asserts is the MEASURED STATE: the
//! composition identities (all green), PS inertness (green), the
//! budget-conversion identities (green), determinism (green), the
//! deviation SHAPES and full-precision pins for every gate quantity,
//! and the zero-external-effect decomposition. If any deviation moves
//! back INSIDE its pre-registered band, the shape assertions go red —
//! that is a re-adjudication event, not a green light.
//!
//! # What is asserted (rule 8, as measured)
//!
//! - **(iii) Composition identities**: every zone/link is value-checked
//!   against the two committed files — externals byte-identical
//!   (structural equality), GB zones identical to the 3-zone family
//!   modulo the two STATED departures (exogenous `net_imports` blocks
//!   removed — external trade is modelled through `[[links]]`; each GB
//!   zone gains the committed GB `[zones.pricing]` SRMC chain), links
//!   B4/B6 byte-identical and the ten external links carried with only
//!   the GB endpoint renamed per the committed landing-point mapping
//!   (Moyle → SSCO, all others → RGB). GB fleet/demand/storage sums,
//!   and the zonal CF reconstruction identity, asserted mechanically.
//! - **(i) Anchor self-validation**: MEASURED RED (banner above) —
//!   pinned in the deviation shape.
//! - **(ii) Composed-anchor B4/B6 binding**: B4 MEASURED RED, branch
//!   (b) (banner above) — pinned in the deviation shape, with the
//!   zero-external-effect decomposition asserted. The composed-anchor
//!   LP legs' B4/B6 [floor, point] bands are measured under the
//!   committed b4-lp mask convention and pinned (±0.01 cross-platform)
//!   as characterisation evidence for the adjudication.
//! - **PS-store inertness ASSERTED** (review edit 4): zero
//!   `pumped_hydro` cycling on the rule-based anchor leg. If the
//!   stores wake, this test is RED — do NOT silently de-duplicate the
//!   rule-based leg (that would break like-for-like with the tier-2
//!   comparator); stop and report.
//! - **LP-leg conventions** (rule 3): every LP run first drops the
//!   `pumped_hydro` store from EVERY zone in memory (the
//!   acceptance_b4_lp precedent — the committed scenario file stays
//!   byte-fixed), and converts the FR/NO2 budgeted hydro to MUST-TAKE
//!   exogenous traces at their observed 2024 generation (the same A75
//!   columns the budgets read) — the identical observed-operation-as-
//!   history posture, RATIFIED by adjudication B(iii). The conversion
//!   is mechanical, not editorial: identity asserts check the
//!   substituted traces per-period against the budgets' own A75
//!   columns and their annual sums against the committed budget
//!   energies (FR 24.37 TWh; NO2 43.67 TWh). The FR pumping DEMAND leg
//!   stays the committed `extra_profiles` trace, untouched — the
//!   conversion is visibly one-sided (generation-side only).
//! - **(iv) Determinism**: rerun bit-identity on the rule-based leg
//!   (parallel ≡ serial is asserted through the group-sweep helper);
//!   the LP leg carries the b4-lp ±0.01 cross-platform tolerance on
//!   binding statistics (ADR-5 single-threaded HiGHS).
//! - **(v) Pins**: new anchor pins live here; the per-zone dispatch
//!   digests / links digest / scenario sha256 are pinned in
//!   `grid-cli/tests/regression_8zone.rs`; ALL committed pins are
//!   untouched files re-run green by the full suite.
//! - **Caveat-(c) diagnostic**: the flow-walk stall-signature count at
//!   the composed anchor is REPORTED (≤-bound convention: GB
//!   curtailment periods with at least one unsaturated export link
//!   toward a non-curtailing external counterparty — an over-count
//!   that includes the exporter-bound class), never asserted as a
//!   magnitude. MEASURED: 9,473 of 9,473 GB-curtailment periods — the
//!   bound is VACUOUS at composed-anchor scale because the committed
//!   stranding artefact makes NSCO curtail behind an unbound B4 in
//!   over half of all periods; stated so the diagnostic is not quoted
//!   as if it measured the stall class.
//!
//! Requires the fetched 2024 + cf-gb2/cf-gb3 + b4/b6 + entsoe-2024 +
//! cf-eu packs; FAILS LOUDLY with build instructions if absent (the
//! acceptance_b4_lp discipline — no silent skip).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::OnceLock;

use grid_adequacy::{
    Execution, MultiZoneInputs, MultiZoneRunResult, load_multi_zone_inputs, run_multi,
    run_multi_lp_min_curtailment, wind_capacity_sweep_multi_group,
};
use grid_core::scenario::{Scenario, StorageKind, ZoneSpec};
use grid_core::time::{HALF_HOUR_MICROS, UtcInstant};
use grid_core::trace::{
    load_per_unit_trace_concat, load_power_trace_mw, load_sparse_power_trace_mw,
};
use grid_core::units::{Energy, Power};

const SCENARIO_8: &str = "scenarios/gb-2024-8zone.toml";
const SCENARIO_3: &str = "scenarios/gb-2024-3zone.toml";
const SCENARIO_5: &str = "scenarios/gb-2024-5zone.toml";
const B4_TRACE: &str = "data/packs/b6/processed/b4_da_flows_limits.parquet";
const B6_TRACE: &str = "data/packs/b6/processed/b6_da_flows_limits.parquet";
const PERIODS_2024: usize = 17_568;

const GB_ZONES: [&str; 3] = ["NSCO", "SSCO", "RGB"];
const EXTERNAL_ZONES: [&str; 5] = ["FR", "CONT-NW", "NO2", "DK1", "IE-SEM"];

// ---------------------------------------------------------------------
// Rule 8(i) — pre-registered anchor tolerances (NOT re-pinnable knobs).
// ---------------------------------------------------------------------

/// The committed 5-zone anchor GB gas (d11 sweep record, run-report §2).
///
/// R7 DISCLOSURE (2026-07-06, reviewer-ruled — r7-fix-review.md §7):
/// the committed 5-zone record moved with the R7 stall fix (gas
/// 71.797411264632 → 71.700788341640; imports 35.935152502942 →
/// 36.025896904243), but these comparators STAY at the pre-registered
/// pre-fix values — re-basing a pre-registered gate after seeing the
/// measurement is exactly what rule 8 prevents. The mixed-engine
/// wedge is 0.13–0.25 %, an order below the band widths, and the
/// 8(i) RED verdict holds under both bases (+4.41 % / +4.55 % gas vs
/// ±2 %; +18.2 % / +17.9 % imports vs ±5 %).
const GAS_5ZONE_ANCHOR_TWH: f64 = 71.797411264632;
/// The committed 5-zone anchor GB net imports (same record; the R7
/// disclosure above applies).
const IMPORTS_5ZONE_ANCHOR_TWH: f64 = 35.935152502942;
/// Stage 5 A1 outright gates (observed 2024).
const GAS_ACTUAL_TWH: f64 = 72.79;
const IMPORTS_ACTUAL_TWH: f64 = 33.30;

// ---------------------------------------------------------------------
// Rule 8(ii) — committed 3-zone B4/B6 rule-based binding pins
// (acceptance_b4_3zone.rs, gate-(iii) mask denominator) and the
// committed 3-zone LP band (acceptance_b4_lp.rs, sentinel-dropped mask).
// ---------------------------------------------------------------------

// R7 DISCLOSURE (2026-07-06, reviewer-ruled — r7-fix-review.md §7):
// the committed 3-zone B4 pin moved with the R7 stall fix
// (0.019506 → 0.020085, i.e. 337 → 347/17,277; B6 unmoved at
// 577/17,211 = 0.033525). These comparators STAY at the
// pre-registered pre-fix values (rule 8: never re-based post-hoc);
// the branch-(b) RED (187 well below either basis) and branch-(a)
// verdicts hold under both.
const B4_RB_BINDING_3ZONE: f64 = 0.01950570122127684;
const B6_RB_BINDING_3ZONE: f64 = 0.03352507117541107;
const B4_LP_POINT_3ZONE: f64 = 0.2816;
const B4_LP_FLOOR_3ZONE: f64 = 0.2346;

// ---------------------------------------------------------------------
// NEW PINS — the composed-anchor MEASURED record (2026-07-05, first
// full run; deterministic per ADR-5). These are CHARACTERISATION pins
// of a gate-red state (module banner), NOT a validated anchor record.
// Rule-based physical quantities pin at engine determinism tolerance;
// LP binding statistics pin at the b4-lp ±0.01 cross-platform
// convention (degenerate-vertex sensitivity).
// ---------------------------------------------------------------------

// R7 flow-walk stall fix re-pin, 2026-07-06 (docs/08 R7): the
// rule-based leg below moved (the stall signature was pervasive on
// this family — 9,473/9,473 GB-curtailment periods at the anchor);
// the LP leg is untouched. Old values are recorded per pin. The
// 8(i)/8(ii) RED verdicts are unchanged in shape.
/// GB-aggregate (NSCO+SSCO+RGB) gas at the composed anchor, TWh —
/// +4.41 % vs the committed 5-zone anchor (8(i) RED). (R7 re-pin:
/// was 75.018859657887, +4.49 % vs the pre-fix committed anchor.)
const PIN_ANCHOR_GB_GAS_TWH: f64 = 74.960300603031;
/// GB-aggregate net imports at the composed anchor, TWh (internal B4/B6
/// positions cancel in the sum — loss 0.0 — so this is the external
/// border position). +18.2 % vs the committed anchor; +27.5 % vs the
/// observed 33.30 (8(i) and A1 both RED). (R7 re-pin: was
/// 42.427578713250.)
const PIN_ANCHOR_GB_NET_IMPORTS_TWH: f64 = 42.472774891030;
/// GB-aggregate pooled curtailment at the composed anchor, TWh (the
/// committed 3-zone stranding artefact carried: standalone 3-zone reads
/// 6.975 post-R7-fix; the 5-zone copper-plate GB anchor reads 0.005).
/// (R7 re-pin: was 7.466489326179.)
const PIN_ANCHOR_GB_CURTAILMENT_TWH: f64 = 7.452365995350;
/// GB-aggregate unserved at the composed anchor, GWh (SSCO, single-pass
/// walk staleness; the 5-zone anchor read 0.0).
const PIN_ANCHOR_GB_UNSERVED_GWH: f64 = 1.355087867608;

/// Composed-anchor rule-based B4/B6 binding (gate-(iii) denominator;
/// pinned as exact binding-period counts over the committed
/// denominators). B4 DECREASED from the committed 3-zone 0.020085
/// (347/17,277 — branch (b) RED, 187 vs 347 binding periods); B6 rose
/// modestly from the committed 0.033525 (577/17,211, unmoved by the
/// R7 fix — its branch (a): 671 vs 577).
/// Measured 2026-07-05; R7 re-pin 2026-07-06 (were 185 and 662
/// against the pre-fix committed 337/0.019506 and 577/0.033525 —
/// both branch verdicts unchanged in shape).
const PIN_ANCHOR_B4_RB_BINDING: f64 = 187.0 / 17_277.0;
const PIN_ANCHOR_B6_RB_BINDING: f64 = 671.0 / 17_211.0;

/// Composed-anchor LP (MinCurtailment + loss-as-waste, PS de-dup,
/// FR/NO2 hydro-as-history) B4/B6 binding bands on the b4-lp
/// sentinel-dropped mask, ±0.01 (HiGHS cross-platform); pinned as
/// exact binding-period counts. TWO floors are pinned per boundary:
/// the committed-convention floor (internal downstream zones only —
/// comparable to the committed 3-zone [0.2346, 0.2816]) and the
/// FULL-downstream floor (external zones included — the honest
/// composed degeneracy class: with lossy links under the loss-as-waste
/// term, spill relocation into ANY curtailing downstream zone is
/// exactly objective-indifferent, and the externals curtail often, so
/// the composed band is much wider).
const PIN_ANCHOR_B4_LP_POINT: f64 = 4_849.0 / 17_235.0; // 0.281346
const PIN_ANCHOR_B4_LP_FLOOR_INTERNAL: f64 = 4_107.0 / 17_235.0; // 0.238294
const PIN_ANCHOR_B4_LP_FLOOR_FULL: f64 = 905.0 / 17_235.0; // 0.052509
const PIN_ANCHOR_B6_LP_POINT: f64 = 1_671.0 / 17_042.0; // 0.098052
const PIN_ANCHOR_B6_LP_FLOOR_INTERNAL: f64 = 1_671.0 / 17_042.0; // = the point: RGB never curtails in a B6-binding LP period
const PIN_ANCHOR_B6_LP_FLOOR_FULL: f64 = 290.0 / 17_042.0; // 0.017017

/// Committed budget energies the FR/NO2 conversions must reproduce
/// (5-zone header values, TWh).
const FR_BUDGET_TWH: f64 = 24.37;
const NO2_BUDGET_TWH: f64 = 43.67;

const CURTAILMENT_TOL_GW: f64 = 1e-6;
const TWH_TOL: f64 = 1e-6;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Loud pack-presence check with build instructions (the union of the
/// 3-zone and 5-zone acceptance requirements — this scenario composes
/// both families' data).
fn require_packs() {
    let root = repo_root();
    for (rel, hint) in [
        (
            "data/packs/2024/processed/demand_2024.parquet",
            "scripts/fetch-2024 (fetch.py, build.py)",
        ),
        (
            "data/packs/2024/processed/gas_sap_daily_2024.parquet",
            "scripts/fetch-prices",
        ),
        (
            "data/packs/cf-gb2/nsco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb3zone.py; verify data/packs/cf-gb3-1985-2024.sha256",
        ),
        (
            B4_TRACE,
            "scripts/fetch-b6 (build.py --three-zone); verify data/packs/b4.sha256",
        ),
        (
            B6_TRACE,
            "scripts/fetch-b6 (build.py); verify data/packs/b6.sha256",
        ),
        (
            "data/packs/entsoe-2024/processed/load_fr_2024.parquet",
            "scripts/fetch-entsoe",
        ),
        (
            "data/packs/cf-eu-1985-2024.sha256",
            "scripts/era5-cf (EU derivation)",
        ),
    ] {
        let path = root.join(rel);
        assert!(
            path.exists(),
            "data pack file missing: {} — build it first: {hint}. The D13 composed \
             acceptance tests stay RED until the packs exist.",
            path.display()
        );
    }
}

fn load(rel: &str) -> Scenario {
    Scenario::load(&repo_root().join(rel)).unwrap()
}

fn zone<'a>(s: &'a Scenario, id: &str) -> &'a ZoneSpec {
    s.zones.iter().find(|z| z.id.as_str() == id).unwrap()
}

fn twh(e: Energy) -> f64 {
    e.as_gigawatt_hours() / 1000.0
}

// ---------------------------------------------------------------------
// Shared anchor run (rule-based leg) — loaded and dispatched once.
// ---------------------------------------------------------------------

fn anchor() -> &'static (Scenario, MultiZoneInputs, MultiZoneRunResult) {
    static RUN: OnceLock<(Scenario, MultiZoneInputs, MultiZoneRunResult)> = OnceLock::new();
    RUN.get_or_init(|| {
        require_packs();
        let scenario = load(SCENARIO_8);
        let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();
        let result = run_multi(&scenario, &inputs).unwrap();
        (scenario, inputs, result)
    })
}

fn gb_aggregate_gas_twh(result: &MultiZoneRunResult) -> f64 {
    let zero = Energy::gigawatt_hours(0.0);
    GB_ZONES
        .iter()
        .map(|id| {
            let z = result.zone(id).unwrap();
            twh(z.thermal_energy("ccgt").unwrap_or(zero) + z.thermal_energy("ocgt").unwrap_or(zero))
        })
        .sum()
}

fn gb_aggregate_net_imports_twh(result: &MultiZoneRunResult) -> f64 {
    GB_ZONES
        .iter()
        .map(|id| twh(result.zone(id).unwrap().net_imports_energy()))
        .sum()
}

fn gb_aggregate_curtailment_twh(result: &MultiZoneRunResult) -> f64 {
    GB_ZONES
        .iter()
        .map(|id| twh(result.zone(id).unwrap().total_curtailment()))
        .sum()
}

fn gb_aggregate_unserved_gwh(result: &MultiZoneRunResult) -> f64 {
    GB_ZONES
        .iter()
        .map(|id| {
            result
                .zone(id)
                .unwrap()
                .total_unserved()
                .as_gigawatt_hours()
        })
        .sum()
}

// ---------------------------------------------------------------------
// Rule 8(iii) — composition identities against the two committed files.
// ---------------------------------------------------------------------

#[test]
fn composition_is_value_identical_to_the_committed_families() {
    require_packs();
    let composed = load(SCENARIO_8);
    let three = load(SCENARIO_3);
    let five = load(SCENARIO_5);

    assert_eq!(composed.schema_version, 8);
    assert_eq!(composed.horizon.start, three.horizon.start);
    assert_eq!(composed.horizon.end, three.horizon.end);
    assert_eq!(
        composed
            .zones
            .iter()
            .map(|z| z.id.as_str())
            .collect::<Vec<_>>(),
        [
            "NSCO", "SSCO", "RGB", "FR", "CONT-NW", "NO2", "DK1", "IE-SEM"
        ],
        "zone declaration order: the three GB zones then the committed 5-zone external order"
    );

    // External zones: byte-identical to the committed 5-zone entries
    // (structural equality of the parsed specs — demand, wedges,
    // extra_profiles, calibrated fleets, budgets, pricing, everything).
    for id in EXTERNAL_ZONES {
        assert_eq!(
            zone(&composed, id),
            zone(&five, id),
            "external zone {id} is not byte-identical to the committed 5-zone entry"
        );
    }

    // GB zones: value-identical to the committed 3-zone family modulo
    // the two STATED departures (design rule 1) — the exogenous
    // `net_imports` blocks removed, a [zones.pricing] block added.
    for id in GB_ZONES {
        let mut expected = zone(&three, id).clone();
        expected
            .exogenous_supply
            .retain(|s| s.label != "net_imports");
        expected.pricing = zone(&composed, id).pricing.clone();
        assert_eq!(
            zone(&composed, id),
            &expected,
            "GB zone {id} departs from the committed 3-zone family beyond the two stated \
             surgeries (net_imports removal; pricing addition)"
        );
    }
    // The removed blocks really existed in the committed file (SSCO's
    // intirl column; RGB's nine-column block) and are gone here.
    for id in ["SSCO", "RGB"] {
        assert!(
            zone(&three, id)
                .exogenous_supply
                .iter()
                .any(|s| s.label == "net_imports"),
            "the committed 3-zone {id} zone should carry a net_imports block"
        );
        assert!(
            zone(&composed, id)
                .exogenous_supply
                .iter()
                .all(|s| s.label != "net_imports"),
            "composed {id} must not double-carry the observed import trace"
        );
    }

    // The added pricing blocks are the committed GB chain (the 5-zone GB
    // zone's block), restricted per zone to its own SRMC-bearing fleet —
    // the committed validator requires srmc entries to name THIS zone's
    // fleet (a stated realisation of the design's "identical in all
    // three zones": the CHAIN — reference, fuel price, recipes — is
    // identical; the srmc listing covers each zone's gas plant).
    let gb5 = zone(&five, "GB").pricing.as_ref().unwrap();
    for id in GB_ZONES {
        let p = zone(&composed, id)
            .pricing
            .as_ref()
            .unwrap_or_else(|| panic!("GB zone {id} must carry [zones.pricing] (rule 5)"));
        assert_eq!(p.reference, gb5.reference, "{id}: pricing reference");
        assert_eq!(
            p.carbon_flat_gbp_per_tco2, gb5.carbon_flat_gbp_per_tco2,
            "{id}: GB carbon basis (UKA+CPS step series, no flat override)"
        );
        assert_eq!(p.fuel_price, gb5.fuel_price, "{id}: gas SAP fuel price");
        for (tech, recipe) in &p.srmc {
            assert_eq!(
                recipe,
                gb5.srmc.get(tech).unwrap(),
                "{id}: srmc.{tech} recipe differs from the committed GB chain"
            );
        }
    }
    let srmc_techs = |id: &str| -> Vec<String> {
        zone(&composed, id)
            .pricing
            .as_ref()
            .unwrap()
            .srmc
            .keys()
            .cloned()
            .collect()
    };
    assert_eq!(srmc_techs("NSCO"), ["ccgt"], "NSCO fleet carries ccgt only");
    assert!(
        srmc_techs("SSCO").is_empty(),
        "SSCO carries no gas plant, so no srmc entries"
    );
    assert_eq!(srmc_techs("RGB"), ["ccgt", "ocgt"]);

    // Links: B4, then B6, byte-identical from the 3-zone file; then the
    // ten external links in committed 5-zone order with ONLY the GB
    // endpoint renamed per the committed landing-point mapping
    // (Moyle → SSCO at Auchencrosh; every other link → RGB).
    assert_eq!(composed.links.len(), 12);
    assert_eq!(
        &composed.links[0], &three.links[0],
        "B4 must carry byte-identical"
    );
    assert_eq!(
        &composed.links[1], &three.links[1],
        "B6 must carry byte-identical"
    );
    assert_eq!(composed.links[0].name.as_deref(), Some("B4"));
    assert_eq!(composed.links[1].name.as_deref(), Some("B6"));
    for (i, five_link) in five.links.iter().enumerate() {
        let mut expected = five_link.clone();
        let landing = if expected.name.as_deref() == Some("Moyle") {
            "SSCO"
        } else {
            "RGB"
        };
        expected.from = grid_core::scenario::ZoneId::new(landing);
        assert_eq!(
            &composed.links[2 + i],
            &expected,
            "external link {:?} departs from the committed 5-zone entry + landing rename",
            five_link.name
        );
    }

    // GB conservation sums (rule 8(iii)): the three GB zones' fleets sum
    // to the committed GB totals per technology, and to the 3-zone
    // family's own sums.
    let fleet_sum = |tech: &str| -> f64 {
        GB_ZONES
            .iter()
            .flat_map(|id| zone(&composed, id).fleet.iter())
            .filter(|e| e.technology.as_str() == tech)
            .map(|e| e.capacity_gw.as_gigawatts())
            .sum()
    };
    for (tech, total) in [
        ("onshore_wind", 14.4),
        ("offshore_wind", 14.7),
        ("solar", 18.7),
        ("ccgt", 30.0),
        ("ocgt", 1.0),
        ("nuclear", 5.9),
        ("biomass", 3.5),
        ("coal", 2.0),
        ("hydro", 1.9),
    ] {
        assert!(
            (fleet_sum(tech) - total).abs() < 1e-9,
            "GB {tech} fleet sum {} != committed total {total}",
            fleet_sum(tech)
        );
    }
    // Demand shares sum to GB (annual_scale 1.0; station-transformer
    // wedge 0.667 GW), PS trace split shares to 1.0, batteries to the
    // committed GB portfolio.
    let demand_scale: f64 = GB_ZONES
        .iter()
        .map(|id| zone(&composed, id).demand.annual_scale)
        .sum();
    assert!(
        (demand_scale - 1.0).abs() < 1e-9,
        "demand shares: {demand_scale}"
    );
    let extra_demand: f64 = GB_ZONES
        .iter()
        .map(|id| zone(&composed, id).demand.extra_demand_gw.as_gigawatts())
        .sum();
    assert!(
        (extra_demand - 0.667).abs() < 1e-9,
        "station wedge: {extra_demand}"
    );
    let ps_scale: f64 = GB_ZONES
        .iter()
        .flat_map(|id| zone(&composed, id).exogenous_supply.iter())
        .filter(|s| s.label == "pumped_storage_net")
        .map(|s| s.scale)
        .sum();
    assert!((ps_scale - 1.0).abs() < 1e-9, "PS split shares: {ps_scale}");
    let store_sum = |kind: StorageKind, energy: bool| -> f64 {
        GB_ZONES
            .iter()
            .flat_map(|id| zone(&composed, id).storage.iter())
            .filter(|s| s.kind == kind)
            .map(|s| {
                if energy {
                    s.energy_gwh.as_gigawatt_hours()
                } else {
                    s.power_gw.as_gigawatts()
                }
            })
            .sum()
    };
    assert!((store_sum(StorageKind::Battery, false) - 4.7).abs() < 1e-9);
    assert!((store_sum(StorageKind::Battery, true) - 6.6).abs() < 1e-9);
    // Pumped hydro sums to the 3-zone station basis (Cruachan+Foyers +
    // Dinorwig+Ffestiniog = 2.828 GW / 23.9 GWh) — the committed 3-zone
    // convention, carried with its double-count warning.
    assert!((store_sum(StorageKind::PumpedHydro, false) - 2.828).abs() < 1e-9);
    assert!((store_sum(StorageKind::PumpedHydro, true) - 23.9).abs() < 1e-9);

    // Zonal CF reconstruction identity (rule 6, asserted at
    // composition): the nsco/ssco CF traces, weighted by the
    // DERIVATION's subzone weight shares (gb3_cf_report.json
    // `subzone_weight_shares_of_sco` — the cluster trace weights, NOT
    // the REPD capacity split, which allocates CAPACITY while traces
    // follow clusters, data report §3), reconstruct the committed sco
    // trace (3-zone header: max residual 2.4e-07 over 40 years — the
    // split loses no information).
    let root = repo_root();
    for (tech, w_nsco, sco_file) in [
        ("onshore", 0.3113207547169811, "sco_onshore_cf_2024.parquet"),
        (
            "offshore",
            0.9975624999999999,
            "sco_offshore_cf_2024.parquet",
        ),
        ("solar", 0.694, "sco_solar_cf_2024.parquet"),
    ] {
        let trace = |rel: String| {
            load_per_unit_trace_concat(
                &[root.join("data/packs/cf-gb2").join(rel)],
                "cf",
                PERIODS_2024,
            )
            .unwrap()
        };
        let n = trace(format!("nsco_{tech}_cf_2024.parquet"));
        let s = trace(format!("ssco_{tech}_cf_2024.parquet"));
        let sco = trace(sco_file.to_owned());
        let mut max_residual: f64 = 0.0;
        for t in 0..PERIODS_2024 {
            let reconstructed =
                w_nsco * n.values()[t].value() + (1.0 - w_nsco) * s.values()[t].value();
            max_residual = max_residual.max((reconstructed - sco.values()[t].value()).abs());
        }
        assert!(
            max_residual < 5e-7,
            "{tech}: weighted nsco/ssco CF does not reconstruct the committed sco trace \
             (max residual {max_residual:e}; pinned identity 2.4e-07)"
        );
    }
}

// ---------------------------------------------------------------------
// Rule 8(i) + PS inertness + conservation + determinism — the anchor
// rule-based leg.
// ---------------------------------------------------------------------

/// The physical energy-conservation identity over a multi-zone result
/// (link flows folded into the zone's exogenous series).
fn assert_conservation(result: &MultiZoneRunResult) {
    for zr in &result.zones {
        let r = &zr.result;
        for t in 0..r.periods() {
            let supply: f64 = r
                .renewables
                .iter()
                .chain(&r.thermal)
                .map(|s| s.power[t].as_gigawatts())
                .sum::<f64>()
                + r.exogenous
                    .iter()
                    .map(|s| s.power[t].as_gigawatts())
                    .sum::<f64>()
                + r.stores
                    .iter()
                    .map(|s| s.discharge[t].as_gigawatts())
                    .sum::<f64>();
            let uses = r.demand[t].as_gigawatts() - r.unserved[t].as_gigawatts()
                + r.stores
                    .iter()
                    .map(|s| s.charge[t].as_gigawatts())
                    .sum::<f64>()
                + r.curtailment[t].as_gigawatts();
            assert!(
                (supply - uses).abs() < 1e-6,
                "zone {} period {t}: supply {supply} != uses {uses}",
                zr.id
            );
            assert!(r.curtailment[t].as_gigawatts() >= -1e-9);
            assert!(r.unserved[t].as_gigawatts() >= -1e-9);
        }
    }
}

#[test]
fn anchor_rule_based_gate_8i_measured_red_and_pinned_in_deviation_shape() {
    let (_, _, result) = anchor();

    let gas = gb_aggregate_gas_twh(result);
    let imports = gb_aggregate_net_imports_twh(result);
    let curtailment = gb_aggregate_curtailment_twh(result);
    let unserved = gb_aggregate_unserved_gwh(result);
    let gas_move = 100.0 * (gas - GAS_5ZONE_ANCHOR_TWH) / GAS_5ZONE_ANCHOR_TWH;
    let import_move = 100.0 * (imports - IMPORTS_5ZONE_ANCHOR_TWH) / IMPORTS_5ZONE_ANCHOR_TWH;
    let a1_import_move = 100.0 * (imports - IMPORTS_ACTUAL_TWH) / IMPORTS_ACTUAL_TWH;
    eprintln!(
        "COMPOSED ANCHOR (rule-based): GB-aggregate gas {gas:.12} TWh ({gas_move:+.2} % vs \
         committed anchor; band ±2 %) | net imports {imports:+.12} TWh ({import_move:+.2} % \
         vs committed, band ±5 %; {a1_import_move:+.2} % vs observed, A1 band ±10 %) | \
         curtailment {curtailment:.12} TWh | unserved {unserved:.12} GWh"
    );

    // THE MEASURED 8(i) VERDICT, asserted in its deviation SHAPE (the
    // D11 conversion precedent — module banner): gas sits ABOVE the
    // pre-registered +2 % edge, imports ABOVE the +5 % edge AND above
    // the outright A1 +10 % gate. If any of these moves back INSIDE its
    // band, that is a re-adjudication event — do not silently re-frame.
    assert!(
        gas_move > 2.0,
        "composed-anchor GB gas {gas:.4} TWh ({gas_move:+.2} %) no longer sits ABOVE the \
         pre-registered ±2 % band — the pinned 8(i) RED record changed shape: re-adjudicate"
    );
    assert!(
        import_move > 5.0,
        "composed-anchor GB net imports {imports:.4} TWh ({import_move:+.2} %) no longer \
         sits ABOVE the ±5 % band — re-adjudicate"
    );
    assert!(
        a1_import_move > 10.0,
        "composed-anchor GB net imports no longer sits ABOVE the outright A1 ±10 % gate — \
         re-adjudicate"
    );
    // The gas A1 gate (±5 % of the OBSERVED 72.79) does pass in its own
    // terms — stated so the record carries both facts.
    assert!(
        (100.0 * (gas - GAS_ACTUAL_TWH) / GAS_ACTUAL_TWH).abs() <= 5.0,
        "A1 gas gate (observed basis): {gas:.4} TWh vs {GAS_ACTUAL_TWH} ±5 %"
    );

    // Full-precision pins of the measured state.
    for (what, measured, pinned) in [
        ("GB gas (TWh)", gas, PIN_ANCHOR_GB_GAS_TWH),
        (
            "GB net imports (TWh)",
            imports,
            PIN_ANCHOR_GB_NET_IMPORTS_TWH,
        ),
        (
            "GB curtailment (TWh)",
            curtailment,
            PIN_ANCHOR_GB_CURTAILMENT_TWH,
        ),
    ] {
        assert!(
            (measured - pinned).abs() <= TWH_TOL,
            "composed-anchor {what}: measured {measured:.12} vs pinned {pinned:.12}"
        );
    }
    assert!(
        (unserved - PIN_ANCHOR_GB_UNSERVED_GWH).abs() <= 1e-6,
        "composed-anchor GB unserved moved: {unserved} GWh"
    );

    // Per-zone conservation (rule 8(iii)).
    assert_conservation(result);
}

/// The 8(i) diagnosis, asserted mechanically: the committed 3-zone
/// family run STANDALONE (its observed exogenous imports intact)
/// already carries the GB-aggregate stranding the 8(i) tolerance
/// derivation missed — its gas sits FURTHER from the 5-zone anchor
/// than the composed scenario's, and its curtailment is the same
/// ~7 TWh artefact class. The composition MOVES TOWARD the anchor
/// relative to its committed GB-side parent; the residual deviation is
/// the parent's committed dispatcher artefact, not new composition
/// physics.
#[test]
fn anchor_8i_diagnosis_the_committed_3zone_parent_carries_the_deviation() {
    require_packs();
    let three = load(SCENARIO_3);
    let inputs = load_multi_zone_inputs(&three, &repo_root()).unwrap();
    let parent = run_multi(&three, &inputs).unwrap();
    let parent_gas = gb_aggregate_gas_twh(&parent);
    let parent_curt = gb_aggregate_curtailment_twh(&parent);
    let (_, _, composed) = anchor();
    let composed_gas = gb_aggregate_gas_twh(composed);
    eprintln!(
        "8(i) diagnosis: committed 3-zone standalone GB gas {parent_gas:.6} TWh / \
         curtailment {parent_curt:.6} TWh; composed {composed_gas:.6} TWh (5-zone anchor \
         {GAS_5ZONE_ANCHOR_TWH})"
    );
    assert!(
        (parent_gas - GAS_5ZONE_ANCHOR_TWH).abs() > (composed_gas - GAS_5ZONE_ANCHOR_TWH).abs(),
        "the committed 3-zone parent should sit FURTHER from the 5-zone anchor than the \
         composed scenario (composing externals moves TOWARD it)"
    );
    assert!(
        parent_curt > 6.5,
        "the committed 3-zone stranding artefact (~7 TWh) should already be in the parent: \
         {parent_curt}"
    );
}

#[test]
fn anchor_pumped_hydro_stores_are_inert_under_rule_based_dispatch() {
    let (_, _, result) = anchor();
    // Review edit 4: inertness is ASSERTED, not assumed. If this test
    // goes red the stores woke — do NOT silently de-duplicate the
    // rule-based leg; stop and report (the double-count would then be
    // an ACTIVE carried tier-2 convention needing disclosure).
    let mut checked = 0;
    for zr in &result.zones {
        for store in &zr.result.stores {
            if store.kind != StorageKind::PumpedHydro {
                continue;
            }
            checked += 1;
            let cycled_gwh: f64 = store
                .charge
                .iter()
                .chain(&store.discharge)
                .map(|p| p.as_gigawatts() * 0.5)
                .sum();
            assert!(
                cycled_gwh.abs() < 1e-9,
                "STOP AND REPORT: zone {} pumped_hydro store CYCLED {cycled_gwh} GWh under \
                 rule-based dispatch at the anchor — the committed harmless-double-count \
                 claim no longer holds; do not de-dup silently",
                zr.id
            );
        }
    }
    assert_eq!(checked, 2, "NSCO and RGB carry the pumped_hydro stores");
}

#[test]
fn anchor_rule_based_leg_is_deterministic_and_group_sweep_reproduces_it() {
    let (scenario, inputs, result) = anchor();

    // Rerun bit-identity (rule 8(iv)).
    let again = run_multi(scenario, inputs).unwrap();
    assert!(again == *result, "rule-based rerun differs (ADR-5)");

    // The D13 group-sweep helper at factor EXACTLY 1.0 (the anchor wind
    // capacity summed from the scenario, so factor = x/x = 1.0) must
    // reproduce the committed dispatch through the GB-aggregate metrics
    // recipe, parallel ≡ serial.
    let anchor_wind: f64 = GB_ZONES
        .iter()
        .flat_map(|id| zone(scenario, id).fleet.iter())
        .filter(|e| matches!(e.technology.as_str(), "onshore_wind" | "offshore_wind"))
        .map(|e| e.capacity_gw.as_gigawatts())
        .sum();
    assert!(
        (anchor_wind - 29.1).abs() < 1e-9,
        "the committed 29.1 GW GB wind fleet"
    );
    let capacities = [Power::gigawatts(anchor_wind)];
    let parallel = wind_capacity_sweep_multi_group(
        scenario,
        inputs,
        &GB_ZONES,
        &capacities,
        Execution::Parallel,
    )
    .unwrap();
    let serial = wind_capacity_sweep_multi_group(
        scenario,
        inputs,
        &GB_ZONES,
        &capacities,
        Execution::Serial,
    )
    .unwrap();
    assert!(
        parallel == serial,
        "group sweep parallel != serial (ADR-10)"
    );

    let point = &parallel.points[0];
    eprintln!(
        "ANCHOR (group-sweep basis-(A) metrics): delivered capture {:?} | potential {:?} | \
         gas {:.12} TWh | net imports {:+.12} TWh | curtailment {:.12} TWh | mean SMP \
         £{:.12}/MWh | gas price-setting {:.12} %",
        point.wind_capture_ratio_delivered,
        point.wind_capture_ratio,
        twh(point.gas),
        twh(point.net_imports),
        twh(point.curtailment),
        point.mean_smp.as_pounds_per_megawatt_hour(),
        100.0 * point.gas_price_setting_share,
    );
    // The helper's aggregates must equal the direct run's sums exactly
    // (factor 1.0 ⇒ the dispatch IS the committed dispatch).
    assert!((twh(point.gas) - gb_aggregate_gas_twh(result)).abs() < 1e-12);
    assert!((twh(point.net_imports) - gb_aggregate_net_imports_twh(result)).abs() < 1e-12);
    assert!((twh(point.curtailment) - gb_aggregate_curtailment_twh(result)).abs() < 1e-12);
}

// ---------------------------------------------------------------------
// Rule 8(ii) — composed-anchor B4/B6 rule-based binding vs the committed
// 3-zone pins (gate-(iii) mask convention: observed flow+limit rows in
// the denominator, sentinel rows excluded from the numerator via
// forward_observed).
// ---------------------------------------------------------------------

struct Observed {
    flow_mw: Vec<Option<f64>>,
    limit_mw: Vec<Option<f64>>,
}

fn observed(rel: &str) -> Observed {
    let path = repo_root().join(rel);
    let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
    let align = |points: Vec<(UtcInstant, Option<Power>)>| -> Vec<Option<f64>> {
        let mut out = vec![None; PERIODS_2024];
        for (t, v) in points {
            let offset = t.unix_micros() - start.unix_micros();
            if offset < 0 || offset % HALF_HOUR_MICROS != 0 {
                continue;
            }
            let index = (offset / HALF_HOUR_MICROS) as usize;
            if index < PERIODS_2024 {
                out[index] = v.map(|p| p.as_gigawatts() * 1000.0);
            }
        }
        out
    };
    Observed {
        flow_mw: align(load_sparse_power_trace_mw(&path, "flow_mw").unwrap()),
        limit_mw: align(load_sparse_power_trace_mw(&path, "limit_mw").unwrap()),
    }
}

fn southward_gw(result: &MultiZoneRunResult, name: &str) -> Vec<f64> {
    result
        .links
        .iter()
        .find(|l| l.name == name)
        .unwrap()
        .home_end
        .iter()
        .map(|p| -p.as_gigawatts())
        .collect()
}

/// The committed gate-(iii) statistic (acceptance_b4_3zone.rs): binding
/// = modelled southward flow ≥ 99 % of the capability the model
/// dispatched against, over the observed flow mask (denominator), with
/// unobserved (sentinel/masked) rows excluded from the numerator.
fn rule_based_binding(result: &MultiZoneRunResult, name: &str, trace: &str) -> (f64, usize) {
    let obs = observed(trace);
    let mask: Vec<bool> = (0..PERIODS_2024)
        .map(|t| obs.flow_mw[t].is_some() && obs.limit_mw[t].is_some())
        .collect();
    let mask_count = mask.iter().filter(|&&m| m).count();
    let model = southward_gw(result, name);
    let cap = result
        .links
        .iter()
        .find(|l| l.name == name)
        .unwrap()
        .capability
        .as_ref()
        .unwrap();
    let binding = (0..PERIODS_2024)
        .filter(|&t| mask[t] && cap.forward_observed[t])
        .filter(|&t| model[t] >= 0.99 * cap.forward[t].as_gigawatts())
        .count();
    (binding as f64 / mask_count as f64, mask_count)
}

#[test]
fn anchor_b4_b6_rule_based_binding_measured_b4_branch_b_red_and_pinned() {
    let (_, _, result) = anchor();

    let (b4, b4_n) = rule_based_binding(result, "B4", B4_TRACE);
    let (b6, b6_n) = rule_based_binding(result, "B6", B6_TRACE);
    assert_eq!(b4_n, 17_277, "the committed B4 gate-(iii) denominator");
    assert_eq!(b6_n, 17_211, "the committed B6 gate-(iii) denominator");
    eprintln!(
        "COMPOSED ANCHOR rule-based binding: B4 {b4:.12} (committed 3-zone \
         {B4_RB_BINDING_3ZONE:.6}) | B6 {b6:.12} (committed {B6_RB_BINDING_3ZONE:.6})"
    );

    // Rule 8(ii) pre-registered branches: (a) small increase; (b)
    // decrease; (c) order-of-magnitude jump. MEASURED (module banner):
    // B4 fired branch (b) — DECREASED — and the shape is asserted so
    // the record cannot rot; B6 fired branch (a) — a modest increase.
    // The diagnosis (zero external effect; the shift is entirely the
    // removed observed-import padding) is asserted by the
    // decomposition test below.
    assert!(
        b4 < B4_RB_BINDING_3ZONE,
        "composed-anchor B4 rule-based binding {b4:.6} no longer sits BELOW the committed \
         {B4_RB_BINDING_3ZONE:.6} — the pinned branch-(b) RED record changed shape: \
         re-adjudicate"
    );
    assert!(
        b6 > B6_RB_BINDING_3ZONE && b6 < 10.0 * B6_RB_BINDING_3ZONE,
        "composed-anchor B6 rule-based binding {b6:.6} left its measured branch-(a) shape \
         (modest increase over {B6_RB_BINDING_3ZONE:.6}) — re-adjudicate"
    );

    assert!(
        (b4 - PIN_ANCHOR_B4_RB_BINDING).abs() < 1e-12,
        "PINNED composed-anchor B4 rule-based binding moved: {b4}"
    );
    assert!(
        (b6 - PIN_ANCHOR_B6_RB_BINDING).abs() < 1e-12,
        "PINNED composed-anchor B6 rule-based binding moved: {b6}"
    );
}

/// The 8(ii) diagnosis, asserted mechanically: under the committed
/// single-pass flow walk with the adopted declaration order (B4, B6,
/// then the externals), B4 and B6 clear BEFORE any external border in
/// every period, so the modelled external links have NO effect on the
/// rule-based B4/B6 binding statistics — deleting all ten external
/// links reproduces them exactly. The design's expected-UP mechanism
/// (export drain deepening the north→south gradient) is therefore
/// unreachable on the rule-based leg by construction; the measured
/// B4-down/B6-up shift is entirely the OTHER stated surgery (the
/// removed observed net_imports padding on SSCO/RGB).
#[test]
fn anchor_8ii_diagnosis_external_links_cannot_move_rule_based_binding() {
    require_packs();
    let (scenario, _, composed) = anchor();
    let mut internal_only = scenario.clone();
    internal_only.links.truncate(2); // keep B4, B6; drop the externals
    let inputs = load_multi_zone_inputs(&internal_only, &repo_root()).unwrap();
    let result = run_multi(&internal_only, &inputs).unwrap();

    let (b4_composed, _) = rule_based_binding(composed, "B4", B4_TRACE);
    let (b6_composed, _) = rule_based_binding(composed, "B6", B6_TRACE);
    let (b4_internal, _) = rule_based_binding(&result, "B4", B4_TRACE);
    let (b6_internal, _) = rule_based_binding(&result, "B6", B6_TRACE);
    eprintln!(
        "8(ii) diagnosis: B4 composed {b4_composed:.12} vs externals-deleted \
         {b4_internal:.12}; B6 composed {b6_composed:.12} vs {b6_internal:.12}"
    );
    assert!(
        (b4_composed - b4_internal).abs() < 1e-12 && (b6_composed - b6_internal).abs() < 1e-12,
        "the external links moved the rule-based B4/B6 binding — the single-pass \
         order-precedence diagnosis no longer holds; re-diagnose the 8(ii) red"
    );
}

// ---------------------------------------------------------------------
// Caveat-(c) diagnostic: flow-walk stall-signature count at the anchor
// (≤-bound convention; REPORTED, never asserted as a magnitude).
// ---------------------------------------------------------------------

#[test]
fn anchor_stall_signature_diagnostic_is_reported() {
    let (scenario, _, result) = anchor();

    // A GB curtailment period carries the stall SIGNATURE (≤-bound)
    // when at least one external link still has export headroom
    // (sending-end flow < 99 % of capacity × availability) toward a
    // counterparty that is not itself curtailing — the R7 walk-stall
    // class PLUS the legitimate exporter-bound class (this is an
    // over-count by convention, matching the committed record's
    // ≤-bound posture).
    let export_links: Vec<(usize, String, f64)> = scenario
        .links
        .iter()
        .enumerate()
        .skip(2) // B4, B6 are internal
        .map(|(i, l)| {
            (
                i,
                l.to.as_str().to_owned(),
                l.capacity_gw.as_gigawatts() * l.availability.value(),
            )
        })
        .collect();
    let zone_curt: Vec<(String, Vec<f64>)> = result
        .zones
        .iter()
        .map(|z| {
            (
                z.id.as_str().to_owned(),
                z.result
                    .curtailment
                    .iter()
                    .map(|p| p.as_gigawatts())
                    .collect(),
            )
        })
        .collect();
    let curt =
        |id: &str, t: usize| -> f64 { zone_curt.iter().find(|(z, _)| z == id).unwrap().1[t] };
    let mut gb_curtailment_periods = 0usize;
    let mut stall_signature = 0usize;
    for t in 0..PERIODS_2024 {
        let gb_curtailing = GB_ZONES.iter().any(|id| curt(id, t) > CURTAILMENT_TOL_GW);
        if !gb_curtailing {
            continue;
        }
        gb_curtailment_periods += 1;
        let stalled = export_links.iter().any(|(i, to, cap)| {
            // Export = GB-side sending end (home is the GB landing zone).
            let export = -result.links[*i].home_end[t].as_gigawatts();
            export < 0.99 * cap && curt(to, t) <= CURTAILMENT_TOL_GW
        });
        if stalled {
            stall_signature += 1;
        }
    }
    eprintln!(
        "COMPOSED ANCHOR stall diagnostic (caveat c, ≤-bound convention): \
         {stall_signature} of {gb_curtailment_periods} GB-curtailment periods carry the \
         stall signature (unsaturated export link toward a non-curtailing counterparty; \
         includes the exporter-bound class by construction)"
    );
}

// ---------------------------------------------------------------------
// The composed-anchor LP leg (MinCurtailment + the D13 loss-as-waste
// term): PS de-dup + FR/NO2 budget-to-history conversion, identity
// asserts, tractability check, B4/B6 [floor, point] bands.
// ---------------------------------------------------------------------

/// The rule-3 LP surgeries, in memory (the committed file stays
/// byte-fixed): drop the `pumped_hydro` store from EVERY zone
/// (acceptance_b4_lp precedent), and convert the FR/NO2 budgeted hydro
/// fleet entries to must-take exogenous traces at their observed 2024
/// generation (the same A75 columns the budgets read).
fn lp_scenario(composed: &Scenario) -> Scenario {
    let mut s = composed.clone();
    for z in &mut s.zones {
        z.storage.retain(|st| st.kind != StorageKind::PumpedHydro);
    }
    for id in ["FR", "NO2"] {
        let z = s.zones.iter_mut().find(|z| z.id.as_str() == id).unwrap();
        let budgeted: Vec<grid_core::scenario::FleetEntry> = z
            .fleet
            .iter()
            .filter(|e| e.energy_budget.is_some())
            .cloned()
            .collect();
        assert_eq!(budgeted.len(), 1, "{id} carries exactly one budgeted entry");
        let budget = budgeted[0].energy_budget.as_ref().unwrap();
        z.exogenous_supply
            .push(grid_core::scenario::ExogenousSupplySpec {
                label: format!("{}_hydro_observed", id.to_lowercase()),
                path: budget.trace.clone(),
                columns: budget.columns.clone(),
                scale: 1.0,
                imports: false,
                reliability: grid_core::scenario::ExogenousReliability::Firm,
            });
        z.fleet.retain(|e| e.energy_budget.is_none());
    }
    s
}

fn lp_anchor() -> &'static (Scenario, MultiZoneInputs, MultiZoneRunResult) {
    static RUN: OnceLock<(Scenario, MultiZoneInputs, MultiZoneRunResult)> = OnceLock::new();
    RUN.get_or_init(|| {
        require_packs();
        let composed = load(SCENARIO_8);
        let scenario = lp_scenario(&composed);
        let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();

        // Tractability (rule 7): the surgered LP is 72 variables/period
        // = 1,264,896 (51 % of LP_VARIABLE_CAP). A rejection here is a
        // stop-and-report event, not a fallback.
        let estimated = grid_adequacy::estimate_lp_variables(&scenario, PERIODS_2024);
        eprintln!(
            "COMPOSED LP leg: estimated {estimated} variables \
             ({:.1} % of the {} cap)",
            100.0 * estimated as f64 / grid_adequacy::LP_VARIABLE_CAP as f64,
            grid_adequacy::LP_VARIABLE_CAP
        );
        assert!(
            estimated <= grid_adequacy::LP_VARIABLE_CAP,
            "STOP AND REPORT: the composed LP exceeds the variable cap ({estimated})"
        );

        let result = run_multi_lp_min_curtailment(&scenario, &inputs).unwrap();
        (scenario, inputs, result)
    })
}

#[test]
fn lp_leg_budget_conversion_is_mechanically_identical_to_the_budgets() {
    require_packs();
    let root = repo_root();
    let composed = load(SCENARIO_8);
    let surgered = lp_scenario(&composed);
    let lp_inputs = load_multi_zone_inputs(&surgered, &root).unwrap();
    let base_inputs = load_multi_zone_inputs(&composed, &root).unwrap();

    for (id, committed_twh) in [("FR", FR_BUDGET_TWH), ("NO2", NO2_BUDGET_TWH)] {
        let zin = lp_inputs
            .zones
            .iter()
            .find(|z| z.id.as_str() == id)
            .unwrap();
        let label = format!("{}_hydro_observed", id.to_lowercase());
        let substituted = &zin
            .inputs
            .exogenous
            .iter()
            .find(|s| s.label == label)
            .unwrap_or_else(|| panic!("{id}: substituted must-take trace missing"))
            .trace;

        // Per-period identity against the budgets' own A75 columns
        // (review edit 6): reload the columns independently and compare
        // every period.
        let spec = zone(&composed, id)
            .fleet
            .iter()
            .find_map(|e| e.energy_budget.as_ref())
            .unwrap();
        let paths: Vec<PathBuf> = spec.trace.paths().iter().map(|p| root.join(p)).collect();
        assert_eq!(paths.len(), 1);
        let mut expected = vec![0.0; PERIODS_2024];
        for column in &spec.columns {
            let trace = load_power_trace_mw(&paths[0], column, PERIODS_2024).unwrap();
            for (acc, &p) in expected.iter_mut().zip(trace.values()) {
                *acc += p.as_gigawatts();
            }
        }
        for (t, (&sub, &exp)) in substituted.values().iter().zip(&expected).enumerate() {
            assert!(
                (sub.as_gigawatts() - exp).abs() < 1e-12,
                "{id} period {t}: substituted trace differs from the A75 columns"
            );
        }
        assert_eq!(substituted.values().len(), PERIODS_2024);

        // Window-sum identity against the loaded budget schedule (the
        // budgets' own view of the same data), and the committed annual
        // energy (5-zone header: FR 24.37 / NO2 43.67 TWh).
        let base_zin = base_inputs
            .zones
            .iter()
            .find(|z| z.id.as_str() == id)
            .unwrap();
        let schedule = base_zin.budgets.values().next().unwrap();
        let mut cursor = 0usize;
        for (w, window) in schedule.windows.iter().enumerate() {
            let len = schedule.window_periods.min(PERIODS_2024 - cursor);
            let sum_gwh: f64 = substituted.values()[cursor..cursor + len]
                .iter()
                .map(|p| p.as_gigawatts() * 0.5)
                .sum();
            assert!(
                (sum_gwh - window.as_gigawatt_hours()).abs() < 1e-9,
                "{id} window {w}: substituted trace {sum_gwh} GWh != budget window {window:?}"
            );
            cursor += len;
        }
        let annual_twh: f64 = substituted
            .values()
            .iter()
            .map(|p| p.as_gigawatts() * 0.5)
            .sum::<f64>()
            / 1000.0;
        assert!(
            (annual_twh - committed_twh).abs() < 0.005,
            "{id}: substituted annual energy {annual_twh:.4} TWh != committed {committed_twh}"
        );

        // The FR pumping DEMAND leg stays the committed extra_profiles
        // trace, untouched (one-sided conversion).
        if id == "FR" {
            assert_eq!(
                zone(&surgered, "FR").demand.extra_profiles,
                zone(&composed, "FR").demand.extra_profiles,
                "the FR pumping demand leg must carry unchanged"
            );
        }
    }

    // The de-dup dropped every pumped_hydro store and nothing else.
    for z in &surgered.zones {
        assert!(z.storage.iter().all(|s| s.kind != StorageKind::PumpedHydro));
    }
    assert_eq!(
        surgered
            .zones
            .iter()
            .map(|z| z.storage.len())
            .sum::<usize>(),
        composed
            .zones
            .iter()
            .map(|z| z.storage.len())
            .sum::<usize>()
            - 2
    );
}

/// The b4-lp mask convention (acceptance_b4_lp.rs, generalised to B6):
/// observed flow present AND a real (non-sentinel) limit posted —
/// strictly inside (0.001, 9.0) GW, which drops zero-limit rows on both
/// boundaries and B6's ≥9999 "no constraint recorded" rows from BOTH
/// numerator and denominator (a sentinel posts no real limit to bind
/// against).
struct LpBand {
    point: f64,
    /// Floor under the COMMITTED 3-zone convention: binding periods
    /// with an INTERNAL downstream zone (SSCO/RGB as applicable)
    /// curtailing are excluded — comparable to the committed
    /// acceptance_b4_lp floor.
    floor_internal: f64,
    /// Floor under the FULL composed downstream set (external zones
    /// included): with lossy links under the loss-as-waste term, spill
    /// relocation into ANY curtailing downstream zone is exactly
    /// objective-indifferent, so this is the honest composed
    /// degeneracy-purged floor.
    floor_full: f64,
    mask_count: usize,
}

fn lp_binding_band(
    result: &MultiZoneRunResult,
    name: &str,
    trace: &str,
    downstream_internal: &[&str],
    downstream_external: &[&str],
) -> LpBand {
    let obs = observed(trace);
    let mask: Vec<bool> = (0..PERIODS_2024)
        .map(|t| obs.flow_mw[t].is_some() && obs.limit_mw[t].is_some_and(|l| l > 1.0 && l < 9000.0))
        .collect();
    let mask_count = mask.iter().filter(|&&m| m).count();
    let model = southward_gw(result, name);
    let binding: Vec<usize> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .filter(|&t| model[t] * 1000.0 >= 0.99 * obs.limit_mw[t].unwrap())
        .collect();
    let point = binding.len() as f64 / mask_count as f64;

    // The floors: exclude binding periods in which a DOWNSTREAM zone is
    // itself curtailing — there the objective was indifferent to where
    // the spill sat (lossless internal links; the D13 loss-as-waste term
    // renders lossy-link disposal indifferent too), so the binding is a
    // solver-vertex artifact, not physics.
    let curt_series = |ids: &[&str]| -> Vec<Vec<f64>> {
        ids.iter()
            .map(|id| {
                result
                    .zone(id)
                    .unwrap()
                    .curtailment
                    .iter()
                    .map(|p| p.as_gigawatts())
                    .collect()
            })
            .collect()
    };
    let floor = |curt: &[Vec<f64>]| -> f64 {
        binding
            .iter()
            .filter(|&&t| curt.iter().all(|c| c[t] <= CURTAILMENT_TOL_GW))
            .count() as f64
            / mask_count as f64
    };
    let internal = curt_series(downstream_internal);
    let full: Vec<Vec<f64>> = internal
        .iter()
        .cloned()
        .chain(curt_series(downstream_external))
        .collect();
    LpBand {
        point,
        floor_internal: floor(&internal),
        floor_full: floor(&full),
        mask_count,
    }
}

#[test]
fn anchor_lp_leg_bands_are_measured_and_pinned() {
    let (_, _, lp) = lp_anchor();
    let (_, _, rb) = anchor();

    // Anomaly guard (rule 8, the pre-registered catch-all): the LP leg
    // must not read WORSE than rule-based on the unserved axis — a
    // worse reading would indicate the LP-leg conventions (PS de-dup,
    // hydro-as-history) or a defect, not geometry.
    let lp_unserved: f64 = lp
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum();
    let rb_unserved: f64 = rb
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum();
    eprintln!(
        "COMPOSED ANCHOR LP: total unserved {lp_unserved:.6} GWh (rule-based \
         {rb_unserved:.6} GWh)"
    );
    assert!(
        lp_unserved <= rb_unserved + 1e-6,
        "STOP AND REPORT (anomaly branch): the perfect-foresight LP leg reads WORSE than \
         rule-based on unserved ({lp_unserved} vs {rb_unserved} GWh) — conventions or \
         defect, not geometry"
    );

    let externals = ["FR", "CONT-NW", "NO2", "DK1", "IE-SEM"];
    let b4 = lp_binding_band(lp, "B4", B4_TRACE, &["SSCO", "RGB"], &externals);
    let b6 = lp_binding_band(lp, "B6", B6_TRACE, &["RGB"], &externals);
    assert_eq!(
        b4.mask_count, 17_235,
        "the committed b4-lp sentinel-dropped mask"
    );
    eprintln!(
        "COMPOSED ANCHOR LP bands: B4 point {:.12}, floor(internal) {:.12}, floor(full) \
         {:.12} on {} periods (committed 3-zone [{B4_LP_FLOOR_3ZONE}, \
         {B4_LP_POINT_3ZONE}]); B6 point {:.12}, floor(internal) {:.12}, floor(full) \
         {:.12} on {} periods",
        b4.point,
        b4.floor_internal,
        b4.floor_full,
        b4.mask_count,
        b6.point,
        b6.floor_internal,
        b6.floor_full,
        b6.mask_count,
    );

    // The LP GB aggregates — REPORTED AS DIAGNOSTICS ONLY, never
    // pinned or quoted as measurements. Ground (measured 2026-07-05,
    // the adjudication B(ii) degeneracy made concrete): MinCurtailment
    // carries no thermal-cost term, so the split of thermal service
    // across technologies is objective-degenerate — the LP leg read GB
    // gas 160.2 TWh (+123 % vs the committed anchor) because HiGHS
    // parked the load on ccgt instead of the merit-order
    // nuclear/biomass/coal mix; and the loss-as-waste term makes lossy
    // IMPORTS strictly dominated by free domestic thermal wherever
    // thermal headroom exists, so LP net imports (+9.6 TWh vs the
    // anchor's +35.9) measure loss-minimising autarky, not trade
    // capacity. The LP leg's quotable statistics on this scenario are
    // the BINDING BANDS below; its trade/gas aggregates are vertex
    // artifacts and stay out of the record (this extends the
    // adjudication's LP-shadow-capture suppression to the gas/trade
    // axes — flagged for the reviewer in the package report).
    let lp_gas = gb_aggregate_gas_twh(lp);
    let lp_imports = gb_aggregate_net_imports_twh(lp);
    let lp_curtailment = gb_aggregate_curtailment_twh(lp);
    eprintln!(
        "COMPOSED ANCHOR LP GB aggregates (DIAGNOSTIC — objective-degenerate, do not \
         quote): gas {lp_gas:.6} TWh ({:+.2} % vs the committed 5-zone anchor) | net \
         imports {lp_imports:+.6} TWh | curtailment {lp_curtailment:.6} TWh",
        100.0 * (lp_gas - GAS_5ZONE_ANCHOR_TWH) / GAS_5ZONE_ANCHOR_TWH
    );

    // Rule 8(ii), LP half: the expected direction was UP vs the
    // committed 3-zone point; MEASURED: flat (−0.0003, well inside the
    // ±0.01 convention). The guard below keeps the not-DOWN half.
    assert!(
        b4.point >= B4_LP_POINT_3ZONE - 0.01,
        "STOP AND REPORT: composed-anchor B4 LP point {:.4} reads BELOW the committed \
         3-zone point {B4_LP_POINT_3ZONE} — beyond the ±0.01 convention",
        b4.point
    );

    // THE PINNED BANDS (quote a band, never the point alone; ±0.01
    // cross-platform) and the pinned LP aggregates.
    for (what, measured, pinned) in [
        ("B4 LP point", b4.point, PIN_ANCHOR_B4_LP_POINT),
        (
            "B4 LP floor (internal convention)",
            b4.floor_internal,
            PIN_ANCHOR_B4_LP_FLOOR_INTERNAL,
        ),
        (
            "B4 LP floor (full downstream set)",
            b4.floor_full,
            PIN_ANCHOR_B4_LP_FLOOR_FULL,
        ),
        ("B6 LP point", b6.point, PIN_ANCHOR_B6_LP_POINT),
        (
            "B6 LP floor (internal convention)",
            b6.floor_internal,
            PIN_ANCHOR_B6_LP_FLOOR_INTERNAL,
        ),
        (
            "B6 LP floor (full downstream set)",
            b6.floor_full,
            PIN_ANCHOR_B6_LP_FLOOR_FULL,
        ),
    ] {
        assert!(
            (measured - pinned).abs() <= 0.01,
            "composed-anchor {what} moved from pinned {pinned}: measured {measured:.6}"
        );
    }
}
