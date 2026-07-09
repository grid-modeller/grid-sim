//! D13 PACKAGE 2 acceptance — the 60 GW COMPOSED measurement on
//! `scenarios/gb-2024-8zone.toml`, per the ADOPTED design
//! `docs/notes/d13-composed-boundary-trade.md` as RE-SCOPED by the
//! package-1 adjudication (`d13-composed-boundary-trade-review.md`
//! addendum, rulings A–E; edits P2-1…P2-6 applied to the note). The
//! package-1 anchor record and its pins live in
//! `acceptance_d13_composed.rs` and are NOT touched here; this file
//! carries only NEW pins (the 60 GW point, plus the anchor LP
//! minimum-forced-waste baseline that ruling C names quotable —
//! "Quotable composed axes for package 2 (60 GW + anchor)").
//!
//! # The RE-SCOPED instrument set (adjudication ruling C — BINDING)
//!
//! 1. **B4/B6 binding bands (LP)** — quoted under BOTH floor
//!    conventions, and EVERY quote names its floor: `floor_internal`
//!    (committed-comparable: internal downstream zones only) and
//!    `floor_full` (externals included). Caveat (n): floor_full tests
//!    downstream curtailment without checking link saturation, so it
//!    OVER-excludes — a deliberately loose lower bound on the artifact
//!    class, not a tight physics floor.
//! 2. **LP MINIMUM FORCED WASTE** — the well-determined optimum. The
//!    objective's optimal VALUE is unique; its components (curtailment
//!    vs link-loss vs storage-loss, and the split of curtailment across
//!    zones) are mutually degenerate. TOTAL waste is the primary
//!    quantity; curtailment is quoted only as the band over the
//!    degenerate loss channels (the d12 band discipline). The
//!    dispatch-independent test: if the composed LP minimum waste at
//!    60 GW exceeds the copper-plate rule-based 4.01 TWh, the geometry
//!    NECESSARILY forces more waste than the tier-2 central reported,
//!    under ANY dispatch — one-sided, dispatch-independent.
//! 3. **Rule-based trade axes as ONE-SIDED disclosed bounds** (exports
//!    = FLOOR, curtailment = CEILING, net trade =
//!    most-pessimistic-for-exports), quotable only with the caveat-(l)
//!    anchor-red disclosure attached verbatim: **+4.41 % gas / +18.2 %
//!    imports vs the committed 5-zone anchor; +27.5 % imports vs
//!    observed — the outright A1 miss** (pre-R7-fix: +4.49/+18.1/
//!    +27.4 %). The composed rule-based leg is
//!    NOT anchor-validated on national trade axes.
//!
//! **Asymmetric evidential rule, mandatory and verbatim** (design note
//! rule 4; adjudication ruling C): "a 60 GW rule-based net-export
//! reading is evidence FOR export survival (a fortiori, through the
//! artefact); a collapse reading is NOT evidence of collapse
//! (artefact-confounded) — it leaves the question to the bracket."
//!
//! **NO capture is measured on ANY leg at ANY point** (ruling D: the
//! composed family currently has no capture instrument; the LP reports
//! no capture and its gas/trade aggregates are non-instruments —
//! caveat (m): LP gas is the thermal-split objective-degeneracy made
//! concrete, and under the loss-as-waste term LP net imports measure
//! loss-minimising autarky. They are reported once below as
//! diagnostics with mechanism, never pinned).
//!
//! # Pre-registered 60 GW branches (design note rule 8, RE-REGISTERED
//! # over the valid instruments — quoted here so the verdict test
//! # below is checkable against the registration)
//!
//! - **Branch A — the geometry forces the waste**: composed LP minimum
//!   forced waste at 60 GW MATERIALLY EXCEEDS 4.01 TWh (with the LP
//!   binding bands high). Dispatch-independent and one-sided; caveat
//!   (e)'s curtailment component resolves AGAINST the tier-2 level.
//!   Export survival is then decided only per the asymmetric rule.
//! - **Branch B — the finding survives**: LP min waste at or near
//!   4.01 TWh AND the rule-based export floor still shows net exports.
//! - **Branch C — bounded split (a-priori expected)**: LP min waste at
//!   or near 4.01 TWh while the rule-based leg collapses; the record is
//!   the bracket quoted whole; export survival OPEN.
//! - **Anomaly catch-all**: any outcome outside A/B/C — including an
//!   LP min-waste reading above the rule-based curtailment ceiling —
//!   → stop, characterise, report before anything is quoted.
//!
//! **MEASURED (2026-07-05): the anomaly catch-all's named shape FIRED
//! (LP min waste 36.224 TWh > rule-based GB ceiling 29.910 TWh on the
//! post-R7 re-pinned record; pre-R7 as first measured: ceiling 30.175,
//! rule-based system-waste analogue 36.929 → 36.666 — the shape holds
//! on both) while
//! branch A's own conditions are simultaneously measured true — the
//! VERDICT IS WITHHELD for the reviewer; see
//! `branch_adjudication_anomaly_shape_measured_verdict_withheld` and
//! the conventions-wedge characterisation
//! (`lp_60gw_feasibility_measured_and_the_conventions_wedge_characterised`).**
//!
//! # Conventions carried verbatim from package 1
//!
//! - GB wind scales by ONE shared national factor (target ÷ the
//!   committed 29.1 GW fleet) across the onshore and offshore entries
//!   of all three GB zones (design rule 6) — asserted identical to the
//!   committed `wind_capacity_sweep_multi_group` helper's scaling.
//! - LP leg surgeries (rule 3): drop `pumped_hydro` from EVERY zone in
//!   memory (the acceptance_b4_lp PS de-dup precedent); convert the
//!   FR/NO2 budgeted hydro to must-take exogenous traces at observed
//!   2024 generation (hydro-as-history, identity-asserted in package 1).
//! - PS inertness is ASSERTED on the rule-based leg at 60 GW (review
//!   edit 4: BOTH points) — if the stores wake, this file goes red and
//!   the double-count is disclosed, never silently de-duplicated.
//! - LP binding statistics pin at the b4-lp ±0.01 cross-platform
//!   convention; rule-based physical quantities at engine determinism
//!   tolerance. LP waste COMPONENTS pin at ±0.02 TWh (degenerate
//!   channels — a cross-platform vertex shift is a
//!   re-pin-with-disclosure event, not silent); the TOTAL pins tight.
//!
//! Requires the same fetched packs as `acceptance_d13_composed.rs`;
//! FAILS LOUDLY with build instructions if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::sync::OnceLock;

use grid_adequacy::{
    Execution, MultiZoneInputs, MultiZoneRunResult, load_multi_zone_inputs, run_multi,
    run_multi_lp_min_curtailment, wind_capacity_sweep_multi_group,
};
use grid_core::scenario::{Scenario, StorageKind, ZoneSpec};
use grid_core::time::{HALF_HOUR_MICROS, UtcInstant};
use grid_core::trace::load_sparse_power_trace_mw;
use grid_core::units::{Energy, Power};

const SCENARIO_8: &str = "scenarios/gb-2024-8zone.toml";
const B4_TRACE: &str = "data/packs/b6/processed/b4_da_flows_limits.parquet";
const B6_TRACE: &str = "data/packs/b6/processed/b6_da_flows_limits.parquet";
const PERIODS_2024: usize = 17_568;

const GB_ZONES: [&str; 3] = ["NSCO", "SSCO", "RGB"];

/// The 60 GW point (the tier-2 central's wind capacity, design rule 6).
const TARGET_WIND_GW: f64 = 60.0;

// ---------------------------------------------------------------------
// COMMITTED COMPARATORS (never re-pinned here; the committed pins live
// in their own files and stay unmoved — these constants are quoting
// copies for the branch definitions and deltas).
// ---------------------------------------------------------------------

/// The copper-plate tier-2 60 GW rule-based curtailment (d11 sweep
/// record, run-report §3) — the dispatch-independent comparator.
/// Quoting copy re-pinned 2026-07-06 with its source
/// (acceptance_d11_sweep.rs, R7 stall fix): was 4.007462807827.
const TIER2_60GW_CURTAILMENT_TWH: f64 = 3.982736889304;
/// The copper-plate tier-2 60 GW gas / net imports (context only —
/// the rule-based trade axes are one-sided bounds, caveat (l)).
/// R7 re-pin: was 40.695234239837 / -6.456015207006.
const TIER2_60GW_GAS_TWH: f64 = 40.670313638111;
const TIER2_60GW_NET_IMPORTS_TWH: f64 = -6.462858359731;

/// Composed-ANCHOR rule-based comparators (package-1 pins, quoted).
/// Quoting copies re-pinned 2026-07-06 with their source
/// (acceptance_d13_composed.rs, R7 stall fix): were 75.018859657887 /
/// 42.427578713250 / 7.466489326179.
const ANCHOR_GB_GAS_TWH: f64 = 74.960300603031;
const ANCHOR_GB_NET_IMPORTS_TWH: f64 = 42.472774891030;
const ANCHOR_GB_CURTAILMENT_TWH: f64 = 7.452365995350;
/// Composed-ANCHOR rule-based B4/B6 binding (package-1 pins; the
/// re-registered rule-8 expectation: these measure the
/// import-padding-removal surgery, external links proven zero-effect
/// on the rule-based walk by construction — that structural diagnosis
/// carries to 60 GW unchanged: B4/B6 clear before any external border).
/// (R7 re-pin 2026-07-06 with acceptance_d13_composed.rs: were
/// 185/17,277 and 662/17,211.)
const ANCHOR_B4_RB_BINDING: f64 = 187.0 / 17_277.0;
const ANCHOR_B6_RB_BINDING: f64 = 671.0 / 17_211.0;
/// Composed-ANCHOR LP band comparators (package-1 pins, floor named).
const ANCHOR_B4_LP_POINT: f64 = 4_849.0 / 17_235.0;
const ANCHOR_B4_LP_FLOOR_INTERNAL: f64 = 4_107.0 / 17_235.0;
const ANCHOR_B6_LP_POINT: f64 = 1_671.0 / 17_042.0;

// ---------------------------------------------------------------------
// NEW PINS — the 60 GW MEASURED record (2026-07-05, first full run;
// deterministic per ADR-5). Rule-based physical quantities pin at
// engine determinism tolerance (1e-6 TWh); LP binding statistics at
// the b4-lp ±0.01 cross-platform convention; the LP waste TOTAL at
// 1e-3 TWh (the well-determined optimum); LP waste COMPONENTS at
// ±0.02 TWh (mutually degenerate channels, see module banner).
// ---------------------------------------------------------------------

// --- Rule-based leg (ONE-SIDED bounds, caveat (l) attached) ---

// R7 flow-walk stall fix re-pin, 2026-07-06 (docs/08 R7): the whole
// rule-based leg below moved (the composed family was the most
// stall-exposed — the package-1 anchor diagnostic found the signature
// on 9,473/9,473 GB-curtailment periods); the LP leg is untouched.
// Old values are recorded per pin.
/// GB-aggregate gas at 60 GW, TWh. (R7 re-pin: was 46.874253432776.)
const PIN_60_GB_GAS_TWH: f64 = 46.769439495118;
/// GB-aggregate net imports at 60 GW, TWh (positive = imports). The
/// net-trade axis, read under the asymmetric evidential rule: this
/// POSITIVE reading (no net exports) is NOT evidence of export
/// collapse — it is the artefact-confounded pessimistic bound, and it
/// leaves export survival OPEN (module banner).
/// (R7 re-pin: was 11.868791000907.)
const PIN_60_GB_NET_IMPORTS_TWH: f64 = 11.702004163523;
/// GB-aggregate pooled curtailment at 60 GW, TWh — the CEILING.
/// (R7 re-pin: was 30.174654042171.)
const PIN_60_GB_CURTAILMENT_TWH: f64 = 29.909919155389;
/// GB-aggregate unserved at 60 GW, GWh (zero: the 60 GW fleet clears
/// the anchor's 1.355 GWh SSCO walk-staleness residue).
const PIN_60_GB_UNSERVED_GWH: f64 = 0.0;
/// The zonal stranding split of the curtailment ceiling, TWh: the
/// stranding sits overwhelmingly NORTH of the boundaries (NSCO 83.2 %,
/// SSCO 16.3 %, RGB 0.5 % of the GB ceiling). (R7 re-pin: were
/// 24.917339725736 / 5.114250090157 / 0.143064226278 — RGB unmoved.)
const PIN_60_CURT_NSCO_TWH: f64 = 24.898333204486;
const PIN_60_CURT_SSCO_TWH: f64 = 4.868521724625;
const PIN_60_CURT_RGB_TWH: f64 = 0.143064226278;
/// Net southward boundary transfers at 60 GW, TWh. (R7 re-pin: were
/// 5.506629009292 / 21.176151704704.)
const PIN_60_B4_SOUTH_TWH: f64 = 5.520960423026;
const PIN_60_B6_SOUTH_TWH: f64 = 21.504232530043;
/// GB gross external trade at 60 GW, TWh (sending-end exports; GB-side
/// received imports).
/// (R7 re-pin: were 23.858666863965 / 35.727457864872.)
const PIN_60_GROSS_EXPORTS_TWH: f64 = 24.047637304912;
const PIN_60_GROSS_IMPORTS_TWH: f64 = 35.749641468435;
/// Per-external-link saturation counts over the 17,568 periods:
/// (name, export-saturated periods, import-saturated periods) —
/// saturation = sending-end flow ≥ 99 % of capacity × availability.
/// Greenlink carries availability 0.0 (2024 commissioning) and is
/// asserted inert instead of counted.
/// (R7 re-pin, old counts: IFA-family 2,286/5,487; Nemo/BritNed
/// 7,066/4,922; NSL 1,815/12,625; Viking 1,161/4,370 — unmoved;
/// Moyle 5,405/131; EWIC 10,768/2,592.)
const PIN_60_LINK_SATURATION: [(&str, usize, usize); 10] = [
    ("IFA", 2_307, 5_562),
    ("IFA2", 2_307, 5_562),
    ("ElecLink", 2_307, 5_562),
    ("Nemo", 7_215, 4_886),
    ("BritNed", 7_215, 4_886),
    ("NSL", 1_816, 12_594),
    ("Viking", 1_161, 4_370),
    ("Moyle", 5_323, 134),
    ("EWIC", 10_786, 2_711),
    ("Greenlink", 0, 0),
];
/// Rule-based B4/B6 binding at 60 GW (gate-(iii) mask convention;
/// pinned as exact binding-period counts over the committed
/// denominators): B4 1,718/17,277 = 0.0994 (anchor 0.0108, committed
/// 3-zone 0.0201); B6 5,161/17,211 = 0.2999 (anchor 0.0390).
/// (R7 re-pin: were 1,706 and 4,769.)
const PIN_60_B4_RB_BINDING_COUNT: usize = 1_718;
const PIN_60_B6_RB_BINDING_COUNT: usize = 5_161;

// --- LP leg (MinCurtailment + loss-as-waste; PS de-dup; FR/NO2
// hydro-as-history) ---

/// LP B4/B6 binding-band counts at 60 GW on the b4-lp sentinel-dropped
/// masks (B4: 17,235 periods; B6: 17,042), ±0.01 on the fractions.
/// B4: point 0.5712, floor_internal 0.2753, floor_full 0.0600 (of
/// 17,235); B6: point 0.3880, floor_internal 0.3709, floor_full 0.0685
/// (of 17,042). Anchor comparators (floor named): B4
/// [floor_internal 0.2383, point 0.2813]; B6 [0.0981, 0.0981].
const PIN_60_LP_B4_POINT_COUNT: usize = 9_845;
const PIN_60_LP_B4_FLOOR_INTERNAL_COUNT: usize = 4_744;
const PIN_60_LP_B4_FLOOR_FULL_COUNT: usize = 1_034;
const PIN_60_LP_B6_POINT_COUNT: usize = 6_612;
const PIN_60_LP_B6_FLOOR_INTERNAL_COUNT: usize = 6_321;
const PIN_60_LP_B6_FLOOR_FULL_COUNT: usize = 1_167;

/// THE HEADLINE INSTRUMENT: LP minimum forced waste at 60 GW, TWh —
/// the weight-1 waste terms of the MinCurtailment objective (all-zone
/// curtailment + storage round-trip loss + link loss), excluding the
/// 1e-6 cycling tie-break and the 1e6-weighted unserved term. The
/// TOTAL is the well-determined optimum; the components below are the
/// solved vertex's degenerate split (characterisation only).
const PIN_60_LP_TOTAL_WASTE_TWH: f64 = 36.223998953964;
const PIN_60_LP_CURTAILMENT_ALL_ZONES_TWH: f64 = 35.638937973451;
const PIN_60_LP_CURTAILMENT_GB_TWH: f64 = 26.217733834871;
const PIN_60_LP_CURTAILMENT_EXTERNAL_TWH: f64 = 9.421204138580;
const PIN_60_LP_STORAGE_LOSS_TWH: f64 = 0.046226069891;
const PIN_60_LP_LINK_LOSS_TWH: f64 = 0.538834910621;
/// LP unserved at 60 GW, GWh (all zones; GB carries ZERO of it — see
/// the conventions-wedge characterisation test).
const PIN_60_LP_UNSERVED_GWH: f64 = 785.086485280863;
/// Rule-based unserved at 60 GW, GWh, ALL zones (GB carries zero; the
/// remainder is external scarcity the walk leaves unserved).
/// (R7 re-pin: was 207.925547466578.)
const PIN_60_RB_UNSERVED_ALL_ZONES_GWH: f64 = 207.683928076314;
/// Rule-based ALL-ZONE curtailment and the rule-based SYSTEM-WASTE
/// analogue at 60 GW (all-zone curtailment + storage round-trip loss
/// on charged energy + link loss — the SAME accounting the LP
/// min-waste instrument uses), TWh. Measured for the bracket-inversion
/// characterisation in the branch-adjudication test.
/// (R7 re-pin: were 35.520189138549 / 36.928918483464.)
const PIN_60_RB_CURTAILMENT_ALL_ZONES_TWH: f64 = 35.252936891843;
const PIN_60_RB_SYSTEM_WASTE_TWH: f64 = 36.666330839207;

/// The ANCHOR LP minimum-forced-waste baseline (same conventions, the
/// composed scenario at the committed 29.1 GW fleet) — quotable per
/// ruling C ("60 GW + anchor") and the context for the 60 GW margin:
/// it carries the composed family's baseline waste (the externals'
/// own curtailment + link losses at the 2024 fleet) that the
/// copper-plate 4.01 TWh comparator never counted.
const PIN_ANCHOR_LP_TOTAL_WASTE_TWH: f64 = 12.196896137008;
const PIN_ANCHOR_LP_CURTAILMENT_ALL_ZONES_TWH: f64 = 11.758631592405;
const PIN_ANCHOR_LP_CURTAILMENT_GB_TWH: f64 = 2.619408148203;
const PIN_ANCHOR_LP_STORAGE_LOSS_TWH: f64 = 0.010582720888;
const PIN_ANCHOR_LP_LINK_LOSS_TWH: f64 = 0.427681823715;

const CURTAILMENT_TOL_GW: f64 = 1e-6;
const TWH_TOL: f64 = 1e-6;
/// Degenerate-channel pin tolerance (module banner).
const WASTE_COMPONENT_TOL_TWH: f64 = 0.02;
/// The well-determined LP optimum pin tolerance.
const WASTE_TOTAL_TOL_TWH: f64 = 1e-3;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Loud pack-presence check with build instructions (identical to the
/// package-1 file — this measurement composes the same data).
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
            "data pack file missing: {} — build it first: {hint}. The D13 60 GW \
             acceptance tests stay RED until the packs exist.",
            path.display()
        );
    }
}

fn load_composed() -> Scenario {
    Scenario::load(&repo_root().join(SCENARIO_8)).unwrap()
}

fn zone<'a>(s: &'a Scenario, id: &str) -> &'a ZoneSpec {
    s.zones.iter().find(|z| z.id.as_str() == id).unwrap()
}

fn twh(e: Energy) -> f64 {
    e.as_gigawatt_hours() / 1000.0
}

fn is_wind(tech: &str) -> bool {
    matches!(tech, "offshore_wind" | "onshore_wind")
}

/// Scale the GB zone group's wind to 60 GW with ONE shared factor —
/// bit-identical arithmetic to the committed
/// `apply_zone_group_wind_capacity` (design rule 6): reference = the
/// group wind sum in scenario order, factor = target ÷ reference,
/// every onshore/offshore entry of the three GB zones multiplied by
/// it. The helper-equality test below asserts the reproduction.
fn scale_gb_wind(scenario: &mut Scenario, target_gw: f64) {
    let reference: f64 = scenario
        .zones
        .iter()
        .filter(|z| GB_ZONES.contains(&z.id.as_str()))
        .flat_map(|z| z.fleet.iter())
        .filter(|e| is_wind(e.technology.as_str()))
        .map(|e| e.capacity_gw.as_gigawatts())
        .sum();
    assert!(
        (reference - 29.1).abs() < 1e-9,
        "the committed 29.1 GW GB wind fleet: {reference}"
    );
    let factor = target_gw / reference;
    for z in scenario
        .zones
        .iter_mut()
        .filter(|z| GB_ZONES.contains(&z.id.as_str()))
    {
        for entry in &mut z.fleet {
            if is_wind(entry.technology.as_str()) {
                entry.capacity_gw = entry.capacity_gw * factor;
            }
        }
    }
}

/// The rule-3 LP surgeries, verbatim from the package-1 file (the
/// committed scenario file stays byte-fixed; identity asserts for the
/// budget conversion are committed there and not repeated): drop the
/// `pumped_hydro` store from EVERY zone, convert the FR/NO2 budgeted
/// hydro to must-take exogenous traces at observed 2024 generation.
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

// ---------------------------------------------------------------------
// Shared runs, each dispatched once per test binary.
// ---------------------------------------------------------------------

fn rb_60() -> &'static (Scenario, MultiZoneInputs, MultiZoneRunResult) {
    static RUN: OnceLock<(Scenario, MultiZoneInputs, MultiZoneRunResult)> = OnceLock::new();
    RUN.get_or_init(|| {
        require_packs();
        let mut scenario = load_composed();
        scale_gb_wind(&mut scenario, TARGET_WIND_GW);
        let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();
        let result = run_multi(&scenario, &inputs).unwrap();
        (scenario, inputs, result)
    })
}

fn lp_60() -> &'static (Scenario, MultiZoneInputs, MultiZoneRunResult) {
    static RUN: OnceLock<(Scenario, MultiZoneInputs, MultiZoneRunResult)> = OnceLock::new();
    RUN.get_or_init(|| {
        require_packs();
        let mut composed = load_composed();
        scale_gb_wind(&mut composed, TARGET_WIND_GW);
        let scenario = lp_scenario(&composed);
        let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();
        let estimated = grid_adequacy::estimate_lp_variables(&scenario, PERIODS_2024);
        assert!(
            estimated <= grid_adequacy::LP_VARIABLE_CAP,
            "STOP AND REPORT: the 60 GW composed LP exceeds the variable cap ({estimated})"
        );
        let result = run_multi_lp_min_curtailment(&scenario, &inputs).unwrap();
        (scenario, inputs, result)
    })
}

fn lp_anchor() -> &'static (Scenario, MultiZoneInputs, MultiZoneRunResult) {
    static RUN: OnceLock<(Scenario, MultiZoneInputs, MultiZoneRunResult)> = OnceLock::new();
    RUN.get_or_init(|| {
        require_packs();
        let composed = load_composed();
        let scenario = lp_scenario(&composed);
        let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();
        let result = run_multi_lp_min_curtailment(&scenario, &inputs).unwrap();
        (scenario, inputs, result)
    })
}

/// The composed-anchor RULE-BASED run (the package-1 record repeated,
/// diagnostics only — its pins live in acceptance_d13_composed.rs):
/// needed by the conventions-wedge characterisation below.
fn rb_anchor() -> &'static MultiZoneRunResult {
    static RUN: OnceLock<MultiZoneRunResult> = OnceLock::new();
    RUN.get_or_init(|| {
        require_packs();
        let scenario = load_composed();
        let inputs = load_multi_zone_inputs(&scenario, &repo_root()).unwrap();
        run_multi(&scenario, &inputs).unwrap()
    })
}

// ---------------------------------------------------------------------
// Aggregates (identical recipes to the package-1 file).
// ---------------------------------------------------------------------

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

/// The physical energy-conservation identity (package-1 recipe).
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

// ---------------------------------------------------------------------
// Boundary-binding statistics (the committed conventions, verbatim
// from the package-1 file).
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

/// The committed gate-(iii) rule-based binding statistic
/// (acceptance_b4_3zone.rs convention), returned as (count, mask).
fn rule_based_binding(result: &MultiZoneRunResult, name: &str, trace: &str) -> (usize, usize) {
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
    (binding, mask_count)
}

/// The b4-lp sentinel-dropped mask convention (package-1 file), with
/// the TWO floors: `floor_internal` (committed-comparable) and
/// `floor_full` (externals included — caveat (n): a deliberately loose
/// lower bound on the artifact class, not a tight physics floor).
struct LpBand {
    point_count: usize,
    floor_internal_count: usize,
    floor_full_count: usize,
    mask_count: usize,
}

impl LpBand {
    fn point(&self) -> f64 {
        self.point_count as f64 / self.mask_count as f64
    }
    fn floor_internal(&self) -> f64 {
        self.floor_internal_count as f64 / self.mask_count as f64
    }
    fn floor_full(&self) -> f64 {
        self.floor_full_count as f64 / self.mask_count as f64
    }
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
    let floor = |curt: &[Vec<f64>]| -> usize {
        binding
            .iter()
            .filter(|&&t| curt.iter().all(|c| c[t] <= CURTAILMENT_TOL_GW))
            .count()
    };
    let internal = curt_series(downstream_internal);
    let full: Vec<Vec<f64>> = internal
        .iter()
        .cloned()
        .chain(curt_series(downstream_external))
        .collect();
    LpBand {
        point_count: binding.len(),
        floor_internal_count: floor(&internal),
        floor_full_count: floor(&full),
        mask_count,
    }
}

// ---------------------------------------------------------------------
// The LP minimum-forced-waste decomposition (ruling C instrument 2).
// ---------------------------------------------------------------------

/// The weight-1 waste terms of the MinCurtailment objective,
/// reconstructed from the result series (all in TWh unless stated):
/// - curtailment: the per-zone curtailment series (the LP curtailment
///   variables), summed;
/// - storage loss: `(1 − round_trip_efficiency) × charged energy` per
///   store — the objective's own term (lp.rs: `loss_rate × dt × c`);
/// - link loss: `loss × sent` both directions = −Σ(home_end +
///   away_end) × dt, exact for any direction split.
///
/// Excluded, stated: the 1e-6 cycling tie-break and the 1e6-weighted
/// unserved term (unserved is reported separately).
struct WasteDecomposition {
    curtailment_by_zone_twh: Vec<(String, f64)>,
    curtailment_gb_twh: f64,
    curtailment_external_twh: f64,
    storage_loss_twh: f64,
    link_loss_twh: f64,
    unserved_gwh: f64,
}

impl WasteDecomposition {
    fn curtailment_all_zones_twh(&self) -> f64 {
        self.curtailment_gb_twh + self.curtailment_external_twh
    }
    fn total_twh(&self) -> f64 {
        self.curtailment_all_zones_twh() + self.storage_loss_twh + self.link_loss_twh
    }
}

fn waste_decomposition(scenario: &Scenario, result: &MultiZoneRunResult) -> WasteDecomposition {
    let mut curtailment_by_zone_twh = Vec::new();
    let mut curtailment_gb_twh = 0.0;
    let mut curtailment_external_twh = 0.0;
    let mut storage_loss_twh = 0.0;
    let mut unserved_gwh = 0.0;
    for (spec, zr) in scenario.zones.iter().zip(&result.zones) {
        assert_eq!(spec.id, zr.id, "zone order");
        let curt = twh(zr.result.total_curtailment());
        curtailment_by_zone_twh.push((zr.id.as_str().to_owned(), curt));
        if GB_ZONES.contains(&zr.id.as_str()) {
            curtailment_gb_twh += curt;
        } else {
            curtailment_external_twh += curt;
        }
        unserved_gwh += zr.result.total_unserved().as_gigawatt_hours();
        assert_eq!(spec.storage.len(), zr.result.stores.len());
        for (st, series) in spec.storage.iter().zip(&zr.result.stores) {
            assert_eq!(st.kind, series.kind);
            let charged_gwh: f64 = series.charge.iter().map(|p| p.as_gigawatts() * 0.5).sum();
            storage_loss_twh += (1.0 - st.round_trip_efficiency.value()) * charged_gwh / 1000.0;
        }
    }
    let mut link_loss_twh = 0.0;
    for link in &result.links {
        let loss_gwh: f64 = link
            .home_end
            .iter()
            .zip(&link.away_end)
            .map(|(h, a)| -(h.as_gigawatts() + a.as_gigawatts()) * 0.5)
            .sum();
        assert!(
            loss_gwh > -1e-6,
            "link {} shows negative loss energy ({loss_gwh} GWh)",
            link.name
        );
        link_loss_twh += loss_gwh / 1000.0;
    }
    WasteDecomposition {
        curtailment_by_zone_twh,
        curtailment_gb_twh,
        curtailment_external_twh,
        storage_loss_twh,
        link_loss_twh,
        unserved_gwh,
    }
}

fn assert_pinned(what: &str, measured: f64, pinned: f64, tol: f64) {
    assert!(
        (measured - pinned).abs() <= tol,
        "60gw record: {what} measured {measured:.12} vs pinned {pinned:.12} (tol {tol})"
    );
}

// ---------------------------------------------------------------------
// The 60 GW scaling convention + helper reproduction + determinism.
// ---------------------------------------------------------------------

#[test]
fn scaling_matches_the_group_convention_and_the_helper_reproduces_the_run() {
    let (scenario, inputs, result) = rb_60();

    // The shared-factor convention (design rule 6): total 60 GW, zonal
    // splits preserved (each entry scaled by exactly 60/29.1 relative
    // to the committed fleet).
    let committed = load_composed();
    let factor = TARGET_WIND_GW / 29.1;
    let mut total = 0.0;
    for id in GB_ZONES {
        for (entry, base) in zone(scenario, id)
            .fleet
            .iter()
            .zip(zone(&committed, id).fleet.iter())
        {
            assert_eq!(entry.technology, base.technology);
            if is_wind(entry.technology.as_str()) {
                total += entry.capacity_gw.as_gigawatts();
                let expected = base.capacity_gw.as_gigawatts() * (TARGET_WIND_GW / 29.1);
                assert!(
                    (entry.capacity_gw.as_gigawatts() - expected).abs() < 1e-9 * factor,
                    "{id} {} scaled off-convention",
                    entry.technology
                );
            } else {
                assert_eq!(entry.capacity_gw, base.capacity_gw);
            }
        }
    }
    assert!(
        (total - TARGET_WIND_GW).abs() < 1e-9,
        "GB wind total {total}"
    );

    // The committed group-sweep helper at 60 GW must reproduce the
    // direct run's aggregates EXACTLY (same scaling arithmetic, an
    // independent second dispatch — this is also the 60 GW rerun
    // determinism check, rule 8(iv)), and parallel ≡ serial (ADR-10).
    // Capture/SMP fields of the helper's point are NOT read: ruling D —
    // no capture is measured on any leg.
    let capacities = [Power::gigawatts(TARGET_WIND_GW)];
    let committed_inputs = inputs;
    let parallel = wind_capacity_sweep_multi_group(
        &committed,
        committed_inputs,
        &GB_ZONES,
        &capacities,
        Execution::Parallel,
    )
    .unwrap();
    let serial = wind_capacity_sweep_multi_group(
        &committed,
        committed_inputs,
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
    assert!((twh(point.gas) - gb_aggregate_gas_twh(result)).abs() < 1e-12);
    assert!((twh(point.net_imports) - gb_aggregate_net_imports_twh(result)).abs() < 1e-12);
    assert!((twh(point.curtailment) - gb_aggregate_curtailment_twh(result)).abs() < 1e-12);
    assert!((point.unserved.as_gigawatt_hours() - gb_aggregate_unserved_gwh(result)).abs() < 1e-12);
}

// ---------------------------------------------------------------------
// Rule-based leg: ONE-SIDED bounds (caveat (l) verbatim in the module
// banner), zonal stranding split, conservation, PS inertness.
// ---------------------------------------------------------------------

/// The rule-based 60 GW GB aggregates — quotable ONLY as one-sided
/// bounds with the caveat-(l) disclosure attached (module banner):
/// exports = FLOOR, curtailment = CEILING, net trade =
/// most-pessimistic-for-exports. The asymmetric evidential rule
/// applies to the net-trade reading (verbatim in the module banner).
#[test]
fn rule_based_60gw_one_sided_bounds_measured_and_pinned() {
    let (_, _, result) = rb_60();

    let gas = gb_aggregate_gas_twh(result);
    let imports = gb_aggregate_net_imports_twh(result);
    let curtailment = gb_aggregate_curtailment_twh(result);
    let unserved = gb_aggregate_unserved_gwh(result);
    let curt_zone = |id: &str| twh(result.zone(id).unwrap().total_curtailment());
    let (nsco, ssco, rgb) = (curt_zone("NSCO"), curt_zone("SSCO"), curt_zone("RGB"));
    eprintln!(
        "COMPOSED 60 GW (rule-based, ONE-SIDED bounds — caveat (l)): GB gas {gas:.12} TWh \
         (anchor {ANCHOR_GB_GAS_TWH}; copper-plate 60 GW comparator {TIER2_60GW_GAS_TWH}) | \
         net imports {imports:+.12} TWh (anchor {ANCHOR_GB_NET_IMPORTS_TWH:+}; copper-plate \
         {TIER2_60GW_NET_IMPORTS_TWH:+}) | curtailment CEILING {curtailment:.12} TWh \
         (anchor {ANCHOR_GB_CURTAILMENT_TWH}; copper-plate {TIER2_60GW_CURTAILMENT_TWH}) | \
         unserved {unserved:.12} GWh"
    );
    eprintln!(
        "COMPOSED 60 GW zonal stranding split (curtailment TWh): NSCO {nsco:.12} | SSCO \
         {ssco:.12} | RGB {rgb:.12}"
    );

    assert_pinned("GB gas (TWh)", gas, PIN_60_GB_GAS_TWH, TWH_TOL);
    assert_pinned(
        "GB net imports (TWh)",
        imports,
        PIN_60_GB_NET_IMPORTS_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "GB curtailment ceiling (TWh)",
        curtailment,
        PIN_60_GB_CURTAILMENT_TWH,
        TWH_TOL,
    );
    assert_pinned("GB unserved (GWh)", unserved, PIN_60_GB_UNSERVED_GWH, 1e-6);
    assert_pinned(
        "NSCO curtailment (TWh)",
        nsco,
        PIN_60_CURT_NSCO_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "SSCO curtailment (TWh)",
        ssco,
        PIN_60_CURT_SSCO_TWH,
        TWH_TOL,
    );
    assert_pinned("RGB curtailment (TWh)", rgb, PIN_60_CURT_RGB_TWH, TWH_TOL);
    assert!(
        (nsco + ssco + rgb - curtailment).abs() < 1e-9,
        "zonal split sums to the GB ceiling"
    );

    assert_conservation(result);
}

#[test]
fn rule_based_60gw_pumped_hydro_stores_stay_inert() {
    let (_, _, result) = rb_60();
    // Review edit 4, the 60 GW half: inertness ASSERTED, not assumed.
    // If the stores wake, do NOT silently de-duplicate the rule-based
    // leg — stop and report; the active double-count would then be a
    // disclosed carried tier-2 convention (caveat (i)).
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
                 rule-based dispatch at 60 GW — the committed harmless-double-count claim \
                 no longer holds; do not de-dup silently",
                zr.id
            );
        }
    }
    assert_eq!(checked, 2, "NSCO and RGB carry the pumped_hydro stores");
}

/// Gross external trade and per-link saturation at 60 GW (rule-based;
/// one-sided bounds, caveat (l)). Saturation = sending-end flow ≥ 99 %
/// of capacity × availability, counted over all 17,568 periods, per
/// direction. Boundary transfers (B4/B6 net southward) are pinned as
/// the wheeling record.
#[test]
fn rule_based_60gw_external_trade_gross_flows_and_link_saturation_pinned() {
    let (scenario, _, result) = rb_60();

    let mut gross_exports_twh = 0.0;
    let mut gross_imports_twh = 0.0;
    let mut measured: Vec<(String, usize, usize)> = Vec::new();
    for (i, link) in scenario.links.iter().enumerate().skip(2) {
        let name = link.name.clone().unwrap();
        let cap = link.capacity_gw.as_gigawatts() * link.availability.value();
        let loss = link.loss.value();
        let series = &result.links[i];
        assert_eq!(series.name, name);
        let mut export_gwh = 0.0;
        let mut import_gwh = 0.0;
        let mut export_sat = 0usize;
        let mut import_sat = 0usize;
        for t in 0..PERIODS_2024 {
            let home = series.home_end[t].as_gigawatts();
            if home < 0.0 {
                // GB exporting: sending-end power = −home_end.
                let sent = -home;
                export_gwh += sent * 0.5;
                if cap > 0.0 && sent >= 0.99 * cap {
                    export_sat += 1;
                }
            } else if home > 0.0 {
                // GB importing: received = home_end; sending-end =
                // received ÷ (1 − loss).
                import_gwh += home * 0.5;
                if cap > 0.0 && home / (1.0 - loss) >= 0.99 * cap {
                    import_sat += 1;
                }
            }
        }
        if cap == 0.0 {
            // Greenlink (availability 0.0, 2024 commissioning basis):
            // asserted inert rather than counted as saturated.
            assert!(
                export_gwh == 0.0 && import_gwh == 0.0,
                "{name}: zero-capability link carried flow"
            );
        }
        gross_exports_twh += export_gwh / 1000.0;
        gross_imports_twh += import_gwh / 1000.0;
        eprintln!(
            "COMPOSED 60 GW link {name}: gross export {:.6} TWh / gross import {:.6} TWh | \
             export-saturated {export_sat} periods, import-saturated {import_sat} periods \
             (of {PERIODS_2024})",
            export_gwh / 1000.0,
            import_gwh / 1000.0,
        );
        measured.push((name, export_sat, import_sat));
    }
    let b4_south = southward_gw(result, "B4").iter().sum::<f64>() * 0.5 / 1000.0;
    let b6_south = southward_gw(result, "B6").iter().sum::<f64>() * 0.5 / 1000.0;
    eprintln!(
        "COMPOSED 60 GW external trade: GB gross exports {gross_exports_twh:.12} TWh \
         (sending-end) | gross imports {gross_imports_twh:.12} TWh (received) | B4 net \
         southward {b4_south:.12} TWh | B6 net southward {b6_south:.12} TWh"
    );

    assert_pinned(
        "gross exports (TWh)",
        gross_exports_twh,
        PIN_60_GROSS_EXPORTS_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "gross imports (TWh)",
        gross_imports_twh,
        PIN_60_GROSS_IMPORTS_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "B4 net southward (TWh)",
        b4_south,
        PIN_60_B4_SOUTH_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "B6 net southward (TWh)",
        b6_south,
        PIN_60_B6_SOUTH_TWH,
        TWH_TOL,
    );
    assert_eq!(measured.len(), PIN_60_LINK_SATURATION.len());
    for ((name, export_sat, import_sat), (pin_name, pin_export, pin_import)) in
        measured.iter().zip(PIN_60_LINK_SATURATION)
    {
        assert_eq!(
            name, pin_name,
            "link order departs from the committed record"
        );
        assert_eq!(
            (*export_sat, *import_sat),
            (pin_export, pin_import),
            "{name}: saturation counts moved"
        );
    }
}

/// Rule-based B4/B6 binding at 60 GW (gate-(iii) mask convention).
/// Comparators: the package-1 anchor pins (B4 185/17,277; B6
/// 662/17,211), which the re-registered rule-8 expectation reads as
/// the import-padding-removal surgery; the package-1 decomposition
/// proved the external links have ZERO effect on the rule-based walk
/// (B4/B6 clear before any external border — structural, so it
/// carries to 60 GW unchanged). The rule-based figure is the
/// disclosed myopic comparator and is NEVER a central on this axis
/// (framing regime (b)).
#[test]
fn rule_based_60gw_b4_b6_binding_measured_and_pinned() {
    let (_, _, result) = rb_60();

    let (b4_count, b4_n) = rule_based_binding(result, "B4", B4_TRACE);
    let (b6_count, b6_n) = rule_based_binding(result, "B6", B6_TRACE);
    assert_eq!(b4_n, 17_277, "the committed B4 gate-(iii) denominator");
    assert_eq!(b6_n, 17_211, "the committed B6 gate-(iii) denominator");
    eprintln!(
        "COMPOSED 60 GW rule-based binding: B4 {b4_count}/{b4_n} = {:.12} (anchor \
         {ANCHOR_B4_RB_BINDING:.6}) | B6 {b6_count}/{b6_n} = {:.12} (anchor \
         {ANCHOR_B6_RB_BINDING:.6})",
        b4_count as f64 / b4_n as f64,
        b6_count as f64 / b6_n as f64,
    );
    assert_eq!(
        b4_count, PIN_60_B4_RB_BINDING_COUNT,
        "B4 rule-based binding moved"
    );
    assert_eq!(
        b6_count, PIN_60_B6_RB_BINDING_COUNT,
        "B6 rule-based binding moved"
    );
}

// ---------------------------------------------------------------------
// LP leg: feasibility + anomaly guards, minimum forced waste, bands.
// ---------------------------------------------------------------------

/// LP feasibility at 60 GW, and the MEASURED CONVENTIONS WEDGE —
/// characterised per the anomaly catch-all's own discipline (stop,
/// characterise, report before anything is quoted) and REPORTED to the
/// reviewer as a finding, not silently narrowed away.
///
/// MEASURED (2026-07-05, first run): the naive all-zone invariant
/// "perfect-foresight LP unserved ≤ rule-based unserved" FAILS at
/// 60 GW — LP 785.086 GWh vs rule-based 207.926 GWh. Characterisation,
/// asserted mechanically below:
/// - GB carries ZERO of the LP unserved (and zero rule-based unserved)
///   — the GB-side invariant holds; the wedge is entirely EXTERNAL.
/// - The LP total is IDENTICAL at the anchor and at 60 GW (the pinned
///   equality below, per zone): it is wind-independent, i.e. a
///   property of the LP-leg CONVENTIONS plus link capacity, not of the
///   60 GW point (in the external scarce hours the perfect-foresight
///   LP already fills the import links at the ANCHOR fleet; extra GB
///   wind adds no link capacity).
/// - The wedge sits ENTIRELY in NO2 (LP 592.793 vs rule-based
///   15.632 GWh; DK1 191.309 and CONT-NW 0.985 GWh are BIT-IDENTICAL
///   on both legs, FR zero on both). Mechanism: NO2's committed WEEKLY
///   energy budget (336-period windows) gives the rule-based leg real
///   within-week flexibility — observed hydro energy shifts into the
///   scarce hours — while the ratified LP-leg conversion (rule 3,
///   adjudication B(iii)) fixes 2024 observed operation as history, a
///   floor no foresight can remove. FR shows NO wedge because its
///   budget has `window_periods = 1`: a per-period budget IS a trace,
///   so the conversion loses nothing there. At the ANCHOR the
///   package-1 all-zone guard passed only because rule-based NO2
///   unserved (694.583 GWh) sat above the LP floor; at 60 GW the extra
///   GB wind relieves NO2 through the walk (15.632 GWh) while the LP's
///   must-take convention cannot adapt.
/// - Consequence for the instruments: this is the caveat-(i)
///   hydro-as-history judgment made visible on the unserved axis
///   (external adaptation denied — the direction the adjudication
///   already ruled conservative). It does not touch the GB boundary
///   bands; its bearing on the min-waste margin goes to the reviewer
///   with the anchor-baseline decomposition (the margin dwarfs the
///   wedge by two orders of magnitude: 0.577 TWh of denied NO2
///   flexibility vs a +32.2 TWh exceedance).
///
/// If the wedge CLOSES (LP ≤ rule-based all-zone at 60 GW) or moves
/// zones, this test goes red — a re-adjudication event, not a fix.
#[test]
fn lp_60gw_feasibility_measured_and_the_conventions_wedge_characterised() {
    let (_, _, lp60) = lp_60();
    let (_, _, rb60) = rb_60();
    let (_, _, lpa) = lp_anchor();
    let rba = rb_anchor();

    let per_zone = |result: &MultiZoneRunResult| -> Vec<(String, f64)> {
        result
            .zones
            .iter()
            .map(|z| {
                (
                    z.id.as_str().to_owned(),
                    z.result.total_unserved().as_gigawatt_hours(),
                )
            })
            .collect()
    };
    let total = |zones: &[(String, f64)]| zones.iter().map(|(_, u)| u).sum::<f64>();
    let lp60_zones = per_zone(lp60);
    let rb60_zones = per_zone(rb60);
    let lpa_zones = per_zone(lpa);
    let rba_zones = per_zone(rba);
    for (label, zones) in [
        ("LP 60 GW", &lp60_zones),
        ("rule-based 60 GW", &rb60_zones),
        ("LP anchor", &lpa_zones),
        ("rule-based anchor", &rba_zones),
    ] {
        eprintln!(
            "UNSERVED per zone ({label}): total {:.9} GWh | {}",
            total(zones),
            zones
                .iter()
                .map(|(id, u)| format!("{id} {u:.9}"))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    // GB-side feasibility: zero on both legs at 60 GW.
    for id in GB_ZONES {
        let gb_lp = lp60_zones.iter().find(|(z, _)| z == id).unwrap().1;
        let gb_rb = rb60_zones.iter().find(|(z, _)| z == id).unwrap().1;
        assert!(
            gb_lp.abs() < 1e-9 && gb_rb.abs() < 1e-9,
            "GB zone {id} carries unserved at 60 GW (LP {gb_lp} / rule-based {gb_rb} GWh)"
        );
    }

    // The wedge, pinned in its measured shape (all-zone LP ABOVE
    // all-zone rule-based at 60 GW — the conventions artefact).
    let lp_total = total(&lp60_zones);
    let rb_total = total(&rb60_zones);
    assert!(
        lp_total > rb_total,
        "the measured conventions wedge closed (LP {lp_total} ≤ rule-based {rb_total} \
         GWh all-zone unserved at 60 GW) — re-adjudicate, do not re-frame"
    );
    assert_pinned("LP unserved (GWh)", lp_total, PIN_60_LP_UNSERVED_GWH, 1e-3);
    assert_pinned(
        "rule-based all-zone unserved (GWh)",
        rb_total,
        PIN_60_RB_UNSERVED_ALL_ZONES_GWH,
        1e-3,
    );
    // Wind-independence: the LP unserved floor is IDENTICAL at the
    // anchor and 60 GW (the conventions, not the geometry).
    assert!(
        (lp_total - total(&lpa_zones)).abs() < 1e-3,
        "the LP unserved floor moved with GB wind ({lp_total} vs {} GWh) — the \
         wind-independence characterisation no longer holds",
        total(&lpa_zones)
    );
    // The wedge is NO2-concentrated: every other zone's unserved is
    // identical across the two legs at 60 GW, so the whole all-zone
    // wedge equals the NO2 wedge (the weekly-budget flexibility the
    // LP-leg conversion removes).
    let unserved_of = |zones: &[(String, f64)], id: &str| -> f64 {
        zones.iter().find(|(z, _)| z == id).unwrap().1
    };
    let no2_wedge = unserved_of(&lp60_zones, "NO2") - unserved_of(&rb60_zones, "NO2");
    assert!(
        (no2_wedge - (lp_total - rb_total)).abs() < 1e-6,
        "the conventions wedge is no longer NO2-concentrated (NO2 wedge {no2_wedge} vs \
         total wedge {} GWh) — re-characterise",
        lp_total - rb_total
    );
    // At the anchor the package-1 all-zone guard passed: rule-based
    // anchor unserved sits ABOVE the LP floor.
    assert!(
        total(&rba_zones) >= total(&lpa_zones) - 1e-6,
        "anchor ordering changed — the package-1 guard's premise no longer holds"
    );
}

/// THE HEADLINE INSTRUMENT (ruling C instrument 2): LP minimum forced
/// waste at 60 GW, pinned as the objective decomposition — TOTAL waste
/// primary (well-determined), components as the solved vertex's
/// degenerate split (curtailment is quotable only as the band over the
/// degenerate loss channels; its ZONE split is likewise degenerate —
/// with the loss-as-waste term, relocating spill into any curtailing
/// downstream zone is exactly objective-indifferent).
///
/// THE DISPATCH-INDEPENDENT TEST: composed 60 GW minimum waste vs the
/// copper-plate rule-based 3.982736889304 TWh (post-R7-fix; the
/// pre-fix comparator read 4.007462807827). If the minimum exceeds
/// the comparator, the geometry NECESSARILY forces more waste than the
/// tier-2 central reported, under ANY dispatch. The anchor LP baseline
/// (pinned below) is the disclosed context: the composed family
/// carries baseline waste (external curtailment + link losses at the
/// 2024 fleet) that the copper-plate comparator never counted.
#[test]
fn lp_60gw_minimum_forced_waste_decomposition_pinned_and_dispatch_independent_test() {
    let (scenario, _, lp) = lp_60();
    let waste = waste_decomposition(scenario, lp);

    for (id, curt) in &waste.curtailment_by_zone_twh {
        eprintln!("COMPOSED 60 GW LP curtailment (degenerate split): {id} {curt:.12} TWh");
    }
    eprintln!(
        "COMPOSED 60 GW LP MINIMUM FORCED WASTE: total {:.12} TWh = curtailment \
         {:.12} TWh (GB {:.12} + external {:.12}) + storage loss {:.12} TWh + link loss \
         {:.12} TWh | unserved {:.6} GWh | copper-plate comparator \
         {TIER2_60GW_CURTAILMENT_TWH} TWh | margin {:+.12} TWh",
        waste.total_twh(),
        waste.curtailment_all_zones_twh(),
        waste.curtailment_gb_twh,
        waste.curtailment_external_twh,
        waste.storage_loss_twh,
        waste.link_loss_twh,
        waste.unserved_gwh,
        waste.total_twh() - TIER2_60GW_CURTAILMENT_TWH,
    );
    // The LP GB gas/trade aggregates — DIAGNOSTIC ONLY, never pinned
    // (caveat (m): thermal-split degeneracy; loss-minimising autarky).
    eprintln!(
        "COMPOSED 60 GW LP GB aggregates (DIAGNOSTIC — objective-degenerate, do not \
         quote): gas {:.6} TWh | net imports {:+.6} TWh",
        gb_aggregate_gas_twh(lp),
        gb_aggregate_net_imports_twh(lp),
    );

    assert_pinned(
        "LP total waste (TWh)",
        waste.total_twh(),
        PIN_60_LP_TOTAL_WASTE_TWH,
        WASTE_TOTAL_TOL_TWH,
    );
    assert_pinned(
        "LP curtailment, all zones (TWh)",
        waste.curtailment_all_zones_twh(),
        PIN_60_LP_CURTAILMENT_ALL_ZONES_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "LP curtailment, GB (TWh)",
        waste.curtailment_gb_twh,
        PIN_60_LP_CURTAILMENT_GB_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "LP curtailment, external (TWh)",
        waste.curtailment_external_twh,
        PIN_60_LP_CURTAILMENT_EXTERNAL_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "LP storage loss (TWh)",
        waste.storage_loss_twh,
        PIN_60_LP_STORAGE_LOSS_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "LP link loss (TWh)",
        waste.link_loss_twh,
        PIN_60_LP_LINK_LOSS_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
}

/// The 60 GW LP B4/B6 binding bands — EVERY quantity names its floor:
/// `[floor_internal, point]` is the committed-comparable band;
/// `floor_full` (externals included) is caveat (n)'s deliberately
/// loose lower bound on the artifact class, not a tight physics floor.
#[test]
fn lp_60gw_b4_b6_binding_bands_measured_and_pinned() {
    let (_, _, lp) = lp_60();
    let externals = ["FR", "CONT-NW", "NO2", "DK1", "IE-SEM"];
    let b4 = lp_binding_band(lp, "B4", B4_TRACE, &["SSCO", "RGB"], &externals);
    let b6 = lp_binding_band(lp, "B6", B6_TRACE, &["RGB"], &externals);
    assert_eq!(
        b4.mask_count, 17_235,
        "the committed b4-lp sentinel-dropped mask"
    );
    assert_eq!(
        b6.mask_count, 17_042,
        "the committed B6 sentinel-dropped mask"
    );
    eprintln!(
        "COMPOSED 60 GW LP bands: B4 point {:.12} ({}/{}) | floor_internal {:.12} ({}) | \
         floor_full {:.12} ({}) — anchor comparators point {ANCHOR_B4_LP_POINT:.6}, \
         floor_internal {ANCHOR_B4_LP_FLOOR_INTERNAL:.6}",
        b4.point(),
        b4.point_count,
        b4.mask_count,
        b4.floor_internal(),
        b4.floor_internal_count,
        b4.floor_full(),
        b4.floor_full_count,
    );
    eprintln!(
        "COMPOSED 60 GW LP bands: B6 point {:.12} ({}/{}) | floor_internal {:.12} ({}) | \
         floor_full {:.12} ({}) — anchor comparator point {ANCHOR_B6_LP_POINT:.6}",
        b6.point(),
        b6.point_count,
        b6.mask_count,
        b6.floor_internal(),
        b6.floor_internal_count,
        b6.floor_full(),
        b6.floor_full_count,
    );

    for (what, measured, pinned_count, mask) in [
        (
            "B4 LP point",
            b4.point(),
            PIN_60_LP_B4_POINT_COUNT,
            b4.mask_count,
        ),
        (
            "B4 LP floor_internal",
            b4.floor_internal(),
            PIN_60_LP_B4_FLOOR_INTERNAL_COUNT,
            b4.mask_count,
        ),
        (
            "B4 LP floor_full",
            b4.floor_full(),
            PIN_60_LP_B4_FLOOR_FULL_COUNT,
            b4.mask_count,
        ),
        (
            "B6 LP point",
            b6.point(),
            PIN_60_LP_B6_POINT_COUNT,
            b6.mask_count,
        ),
        (
            "B6 LP floor_internal",
            b6.floor_internal(),
            PIN_60_LP_B6_FLOOR_INTERNAL_COUNT,
            b6.mask_count,
        ),
        (
            "B6 LP floor_full",
            b6.floor_full(),
            PIN_60_LP_B6_FLOOR_FULL_COUNT,
            b6.mask_count,
        ),
    ] {
        let pinned = pinned_count as f64 / mask as f64;
        assert!(
            (measured - pinned).abs() <= 0.01,
            "60 GW {what} moved from pinned {pinned:.6}: measured {measured:.6} (±0.01 \
             cross-platform convention)"
        );
    }
}

/// The ANCHOR LP minimum-forced-waste baseline (ruling C: the
/// min-waste instrument is quotable at "60 GW + anchor"). Same
/// conventions as the 60 GW leg; the composed anchor LP run is the
/// package-1 run repeated here for the decomposition, which package 1
/// did not pin (its pins — the anchor bands — live in
/// acceptance_d13_composed.rs and are not touched).
#[test]
fn lp_anchor_minimum_forced_waste_decomposition_pinned() {
    let (scenario, _, lp) = lp_anchor();
    let waste = waste_decomposition(scenario, lp);
    eprintln!(
        "COMPOSED ANCHOR LP MINIMUM FORCED WASTE (baseline): total {:.12} TWh = \
         curtailment {:.12} TWh (GB {:.12} + external {:.12}) + storage loss {:.12} TWh + \
         link loss {:.12} TWh | unserved {:.6} GWh",
        waste.total_twh(),
        waste.curtailment_all_zones_twh(),
        waste.curtailment_gb_twh,
        waste.curtailment_external_twh,
        waste.storage_loss_twh,
        waste.link_loss_twh,
        waste.unserved_gwh,
    );
    assert_pinned(
        "anchor LP total waste (TWh)",
        waste.total_twh(),
        PIN_ANCHOR_LP_TOTAL_WASTE_TWH,
        WASTE_TOTAL_TOL_TWH,
    );
    assert_pinned(
        "anchor LP curtailment, all zones (TWh)",
        waste.curtailment_all_zones_twh(),
        PIN_ANCHOR_LP_CURTAILMENT_ALL_ZONES_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "anchor LP curtailment, GB (TWh)",
        waste.curtailment_gb_twh,
        PIN_ANCHOR_LP_CURTAILMENT_GB_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "anchor LP storage loss (TWh)",
        waste.storage_loss_twh,
        PIN_ANCHOR_LP_STORAGE_LOSS_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
    assert_pinned(
        "anchor LP link loss (TWh)",
        waste.link_loss_twh,
        PIN_ANCHOR_LP_LINK_LOSS_TWH,
        WASTE_COMPONENT_TOL_TWH,
    );
}

// ---------------------------------------------------------------------
// Branch adjudication (rule 8, re-registered branches — the verdict
// record, asserted in its measured shape so it cannot silently rot).
// ---------------------------------------------------------------------

/// >>> THE MEASURED BRANCH VERDICT (2026-07-05) — ANOMALY SHAPE FIRED;
/// >>> VERDICT WITHHELD PENDING REVIEWER ADJUDICATION (the D11
/// >>> withhold-and-report discipline, exactly as package 1 applied it
/// >>> to the anchor reds). <<<
///
/// Measured against the re-registered branches:
/// - **Branch A's conditions are measured TRUE on its own axes**: LP
///   minimum forced waste 36.224 TWh exceeds the copper-plate
///   comparator 3.983 TWh by +32.241 TWh (and exceeds it by +20.04 TWh
///   even after subtracting the ENTIRE composed-anchor LP baseline of
///   12.197 TWh — the externals' own curtailment and losses the
///   copper-plate number never counted), with the LP binding bands
///   high and UP from the anchor (B4 point 0.5712 vs 0.2813;
///   floor_internal 0.2753 vs 0.2383; B6 point 0.3880 vs 0.0981 —
///   floors named). The rule-based floor shows NO net exports
///   (+11.70 TWh net imports), so under the asymmetric evidential rule
///   export survival is OPEN (the collapse-side reading is NOT
///   evidence of collapse).
/// - **AND the anomaly catch-all's one NAMED shape also fired**: the
///   LP min-waste reading (36.224 TWh) sits ABOVE the rule-based GB
///   curtailment ceiling (29.910 TWh), so the registered bracket
///   "curtailment ∈ [LP min-waste, rule-based ceiling]" INVERTS as
///   registered. Characterisation, asserted below: the inversion is an
///   ACCOUNTING-BASIS mismatch in the registered bracket, not the LP
///   out-wasting the dispatcher — the min-waste instrument counts the
///   whole 8-zone system's waste (external curtailment 9.42 TWh at the
///   vertex + link loss 0.54 + storage loss 0.05) while the ceiling
///   counts GB curtailment only. On the GB-attributed vertex split the
///   bracket holds (LP GB curtailment 26.22 < ceiling 29.91, itself
///   degenerate); on the LIKE-basis system comparison the LP optimum
///   sits BELOW the rule-based dispatch's own system waste (36.224 <
///   36.666 TWh = 35.253 all-zone curtailment + 1.413 link loss +
///   0.000 storage loss), as an optimum should. Which bracket
///   convention the record quotes is a FRAMING question — per the
///   pre-registration it goes to the reviewer before anything is
///   quoted. No spin: this test pins BOTH facts in their measured
///   shape.
#[test]
fn branch_adjudication_anomaly_shape_measured_verdict_withheld() {
    let (lp_scenario_60, _, lp) = lp_60();
    let (rb_scenario_60, _, rb) = rb_60();
    let lp_waste = waste_decomposition(lp_scenario_60, lp);
    let rb_waste = waste_decomposition(rb_scenario_60, rb);
    let margin = lp_waste.total_twh() - TIER2_60GW_CURTAILMENT_TWH;
    let rb_ceiling = gb_aggregate_curtailment_twh(rb);
    let rb_net_imports = gb_aggregate_net_imports_twh(rb);
    eprintln!(
        "BRANCH INPUTS: LP min waste {:.12} TWh (margin {margin:+.12} vs copper-plate \
         {TIER2_60GW_CURTAILMENT_TWH}; anchor LP baseline {PIN_ANCHOR_LP_TOTAL_WASTE_TWH}) \
         | rule-based GB curtailment ceiling {rb_ceiling:.12} TWh | rule-based ALL-ZONE \
         curtailment {:.12} TWh | rule-based SYSTEM-WASTE analogue {:.12} TWh (storage \
         loss {:.12} + link loss {:.12}) | rule-based net imports {rb_net_imports:+.12} \
         TWh (export floor: net exports iff negative)",
        lp_waste.total_twh(),
        rb_waste.curtailment_all_zones_twh(),
        rb_waste.total_twh(),
        rb_waste.storage_loss_twh,
        rb_waste.link_loss_twh,
    );

    // THE DISPATCH-INDEPENDENT TEST (branch-A axis), in its measured
    // shape: minimum forced waste exceeds the copper-plate comparator —
    // and still exceeds it after subtracting the entire anchor
    // baseline (the conservative deconfounded reading).
    assert!(
        margin > 0.0
            && lp_waste.total_twh() - PIN_ANCHOR_LP_TOTAL_WASTE_TWH > TIER2_60GW_CURTAILMENT_TWH,
        "the dispatch-independent exceedance changed shape (waste {:.6} TWh, anchor \
         baseline {PIN_ANCHOR_LP_TOTAL_WASTE_TWH}, comparator \
         {TIER2_60GW_CURTAILMENT_TWH}) — re-adjudicate",
        lp_waste.total_twh()
    );
    // Bands high and UP from the anchor (beyond the ±0.01 convention),
    // on BOTH floors' band ends — floor named on every quantity.
    let externals = ["FR", "CONT-NW", "NO2", "DK1", "IE-SEM"];
    let b4 = lp_binding_band(lp, "B4", B4_TRACE, &["SSCO", "RGB"], &externals);
    let b6 = lp_binding_band(lp, "B6", B6_TRACE, &["RGB"], &externals);
    assert!(
        b4.point() > ANCHOR_B4_LP_POINT + 0.01
            && b4.floor_internal() > ANCHOR_B4_LP_FLOOR_INTERNAL + 0.01
            && b6.point() > ANCHOR_B6_LP_POINT + 0.01,
        "the 60 GW LP bands no longer sit UP from the anchor — re-adjudicate"
    );
    // The rule-based export floor shows NO net exports: under the
    // asymmetric evidential rule this leaves export survival OPEN
    // (it is NOT evidence of collapse).
    assert!(
        rb_net_imports > 0.0,
        "the rule-based net-trade reading changed sign (now {rb_net_imports:+.6} TWh) — \
         a net-export floor reading is evidence FOR survival: re-adjudicate the branch"
    );

    // THE ANOMALY SHAPE, pinned as measured: the registered bracket
    // [LP min-waste, rule-based GB ceiling] INVERTS on the
    // as-registered accounting bases…
    assert!(
        lp_waste.total_twh() > rb_ceiling,
        "the bracket inversion closed (LP min waste {:.6} ≤ rule-based GB ceiling \
         {rb_ceiling:.6} TWh) — the pinned anomaly record changed shape: re-adjudicate",
        lp_waste.total_twh()
    );
    // …while BOTH like-basis orderings hold: GB-attributed (vertex,
    // degenerate) LP curtailment below the GB ceiling, and the LP
    // system optimum below the rule-based system waste.
    assert!(
        lp_waste.curtailment_gb_twh < rb_ceiling,
        "GB-basis ordering broke: LP GB-vertex curtailment {:.6} vs ceiling {rb_ceiling:.6}",
        lp_waste.curtailment_gb_twh
    );
    assert!(
        lp_waste.total_twh() < rb_waste.total_twh(),
        "like-basis ordering broke: LP system optimum {:.6} vs rule-based system waste \
         {:.6} TWh — that WOULD be a genuine optimality anomaly",
        lp_waste.total_twh(),
        rb_waste.total_twh()
    );
    assert_pinned(
        "rule-based all-zone curtailment (TWh)",
        rb_waste.curtailment_all_zones_twh(),
        PIN_60_RB_CURTAILMENT_ALL_ZONES_TWH,
        TWH_TOL,
    );
    assert_pinned(
        "rule-based system-waste analogue (TWh)",
        rb_waste.total_twh(),
        PIN_60_RB_SYSTEM_WASTE_TWH,
        TWH_TOL,
    );
}
