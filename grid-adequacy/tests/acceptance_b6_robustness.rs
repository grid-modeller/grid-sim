//! B6 two-zone ROBUSTNESS demonstration (work order deliverable 4):
//! the Royal-Society-style 37+-year storage requirement (pinned
//! 23,872 GWh at its conventions — acceptance_stage3_rs37y.rs)
//! recomputed under the two-zone Scotland/rest-of-GB split with the B6
//! link at the ruling's non-2024 central capabilities (export 4.1 GW /
//! import 3.5 GW — no synthesised limit series for non-2024 years),
//! pinned with its deltas AND the control/placement decomposition the
//! engine review demanded.
//!
//! # THE FINDING OF RECORD — framed per the engine review §1d verbatim
//! # (docs/notes/b6-two-zone-engine-review.md; carries on EVERY quote
//! # of these numbers)
//!
//! Under the two-zone split with B6 frozen at the ruling's 2024 central
//! capabilities (export 4.1 / import 3.5 GW), the RS-fleet 40-year
//! storage requirement rises from the single-zone 23,872 GWh to
//! **35,648 GWh (+49.3 %)** at the demand-share store placement. THREE
//! conditions travel with the number, always:
//!
//! 1. **STRESS CONVENTION**: end-2024 zonal capacity shares and 2024
//!    boundary capability projected onto a 520 GW fleet — a statement
//!    about *today's network under that fleet*, NEVER a projection of a
//!    credible future network.
//! 2. **PLACEMENT**: the hydrogen store's ENERGY placement moves the
//!    requirement from **33,056 GWh (+38.5 %, measured optimum near a
//!    3 % Scottish energy share)** through **35,648 (+49.3 %, demand
//!    share)** to **49,152 (+106 %, wind share)**. Power placement is
//!    NOT free — the 100 GW rating must track zonal peak demand, so the
//!    named wind-share / all-south alternatives are infeasible as
//!    power splits and only the ENERGY placement is a free knob.
//!    **CORRECTION (2026-07-04, beta-readiness audit): there is NO
//!    placement-stable "boundary effect proper".** The earlier claim —
//!    "boundary-attributable effect stable at ~+33–35 % across
//!    placements" — is WITHDRAWN, contradicted by this file's own
//!    pinned constants: at the 3 % energy-optimum placement the
//!    B6-constrained solve (33,056 GWh) sits BELOW copper-plate at the
//!    same placement (33,632 GWh), a −1.7 % "boundary effect" that is
//!    physically impossible under optimal dispatch and a symptom of the
//!    rule-based flow artefact. The boundary-vs-copper delta at the
//!    SAME placement is not stable: it ranges from **−1.7 % (3 %
//!    placement)** to **+34.6 % (demand-share)**. The +38.5 % lower
//!    bound of the total headline is therefore CONTAMINATED by the
//!    copper-plate rule-based-flow artefact, not a clean boundary term.
//!    **What survives, safely quotable:** only the raw total-delta
//!    DIRECTION (single-zone 23,872 < every two-zone/B6 configuration
//!    measured), plus the pinned totals with all three conditions. No
//!    single "boundary effect proper" percentage is quotable; the clean
//!    boundary-vs-dispatch separation awaits the LP (Stage 7).
//! 3. **DISPATCH CONVENTION**: the two-zone copper-plate baseline sits
//!    +4 to +11 % above the single-zone pin depending on placement, and
//!    controls show this is dominated by the **RULE-BASED FLOW
//!    CONVENTION** (flows clear before storage by surplus-depth
//!    equalisation, blind to store headroom — [`grid_adequacy::flow`]
//!    rule 1/3), NOT by the zonal split. The same fleet, traces and a
//!    two-store split on a SINGLE BUS need **24,112 GWh (+1.0 %)**, and
//!    pooled zonal traces on a single bus need **23,808 GWh** (trace
//!    substitution NIL, −0.3 %, the +0.22 % onshore-split energy wobble
//!    with sign). Every quote therefore carries **"rule-based dispatch,
//!    upper-bias"** alongside the "B6-only slice, lower bound on network
//!    effects" duty; the two biases run in OPPOSITE directions, and the
//!    **LP comparison (Stage 7, `perfect_foresight`) is the named
//!    resolver**.
//!
//! The ratified expectation "flagship storage numbers barely move" is
//! CONTRADICTED as measured, at full prominence.
//!
//! ## Mechanism (engine review §1b, controls-adjudicated)
//!
//! The earlier attribution shipping in this file — *"the split alone
//! (zonal traces + demand-share store placement) moves the number by
//! ~11 %"* — was WRONG and is withdrawn. The controls show trace
//! substitution contributes ~nothing (23,808 ≈ 23,872) and the
//! two-store split on a single bus ~+1 % (24,112); the copper-plate
//! two-zone +10.9 % is dominated by the rule-based flow convention, not
//! the split. Drought DEPTH is GB-wide (worst year 2010 in both zones,
//! no unserved anywhere at the pinned sizes); it is the **inter-drought
//! RECHARGE** that the boundary throttles. But the boundary term does
//! NOT separate cleanly from the dispatch convention: the B6-vs-copper
//! delta at the same placement swings from −1.7 % to +34.6 % (see the
//! §2 CORRECTION), so there is no placement-stable "B6 leg proper" to
//! quote — only the total-delta direction is safe until the LP resolves
//! the entanglement.
//!
//! **A finite B6 does NOT monotonically raise the requirement in this
//! engine.** At the 3 % energy placement the B6-constrained solve needs
//! LESS storage than copper-plate (**33,056 < 33,632**) — physically
//! impossible under optimal dispatch, a direct artefact of the
//! rule-based flow convention (the equal-surplus-depth rule ships the
//! southern zone's absolutely-deeper surplus NORTH into a store that
//! can charge at only 10.1 GW). So the old *"direction is structural:
//! a finite boundary can only increase the requirement"* comment on the
//! assert is FALSE for this engine and has been removed; the assert is
//! re-scoped to the demand-share placement it actually tests.
//!
//! QUOTE DUTY (ruling (c)): B6-only slice; a LOWER BOUND on network
//! effects (the intra-Scottish B4/B5 boundaries are invisible).
//!
//! # Bisection convention
//!
//! The bisection mirrors `min_storage_for_zero_unserved` exactly
//! (doubling search from 1 GWh, halving to max(0.1 GWh, 1e-3 × hi),
//! requirement = smallest known-feasible; feasible = total unserved
//! across ALL zones ≤ 1e-9 GWh), so every number here is convention-
//! comparable with the pinned single-zone 23,872. Controls run through
//! the same [`run_multi`] path (single-zone run_multi is bit-identical
//! to `run`).
//!
//! Requires the per-year 1985–2024 packs (demand-tiled + cf-gb2);
//! fails loudly with build instructions if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{MultiZoneInputs, load_multi_zone_inputs, run_multi};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, LinkSpec, Scenario, StorageKind,
    StorageSpec, TechId, TraceFiles, WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::units::{Energy, PerUnit, Power};

/// The single-zone RS pin (acceptance_stage3_rs37y.rs) — the
/// single-bus GB baseline the deltas are quoted against.
const SINGLE_ZONE_PIN_GWH: f64 = 23_872.0;

/// PINNED two-zone headline requirements (first pass, 2026-07-04;
/// deterministic ADR-5 — a move is a knowing re-pin with the record).
/// Exact whole GWh: every bisection candidate is doubling/halving
/// arithmetic (binary-exact f64), the single-zone pin's own property.
const PIN_2ZONE_COPPER_GWH: f64 = 26_480.0;
const PIN_2ZONE_B6_GWH: f64 = 35_648.0;

/// PINNED mechanism controls (engine review §1b/§1c, reproduced with
/// this file's own bisection; measured-then-pinned 2026-07-04). These
/// are the numbers the finding-of-record framing quotes.
const PIN_POOLED_SINGLE_BUS_GWH: f64 = 23_808.0; // trace substitution ≈ nil
const PIN_TWO_STORE_SINGLE_BUS_GWH: f64 = 24_112.0; // store split per se ≈ +1.0 %
const PIN_B6_ES003_GWH: f64 = 33_056.0; // B6, 3 % Scottish-energy placement (optimum)
const PIN_COPPER_ES003_GWH: f64 = 33_632.0; // copper, same placement (> B6: the inversion)
const PIN_B6_WINDSHARE_GWH: f64 = 49_152.0; // B6, 34.8 % wind-share placement

/// Store energy-placement shares: the measured B6 optimum (~3 %
/// Scottish energy) and the Scottish wind-energy share (~34.8 %).
const ES_OPTIMUM: f64 = 0.03;
const ES_WIND_SHARE: f64 = 0.348;

/// Zonal shares (the adopted scenario conventions — the 2-zone
/// scenario header cites each): demand 10.1 %, onshore 0.6997 (DESNZ),
/// offshore 0.209150 (cluster), solar 0.026738 (cluster).
const SCO_DEMAND: f64 = 0.101;
const SCO_ONSHORE: f64 = 0.6997;
const SCO_OFFSHORE: f64 = 0.209150;
const SCO_SOLAR: f64 = 0.026738;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn require_packs() {
    let root = repo_root();
    let mut missing: Vec<String> = Vec::new();
    for year in 1985..=2024 {
        for rel in [
            format!("data/packs/demand-tiled/demand_{year}.parquet"),
            format!("data/packs/cf-gb2/sco_onshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/rgb_onshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/sco_offshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/rgb_offshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/sco_solar_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/rgb_solar_cf_{year}.parquet"),
        ] {
            if !root.join(&rel).exists() {
                missing.push(rel);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "per-year pack incomplete: {} file(s) missing, first {} — build with \
         scripts/fetch-2024 (tiled demand) and scripts/era5-cf/derive_cf_gb2zone.py \
         (zonal traces; manifest data/packs/cf-gb2-1985-2024.sha256)",
        missing.len(),
        missing[0]
    );
}

fn per_year_traces(pattern: &dyn Fn(i32) -> String) -> TraceFiles {
    TraceFiles::from_paths((1985..=2024).map(pattern).collect())
}

/// A CF fleet entry with an explicit technology id, zone-file prefix
/// and CF file stem — the building block for both the per-zone fleet
/// (`offshore_wind` etc.) and the pooled single-bus fleet
/// (`offshore_wind_sco` + `offshore_wind_rgb`, whose share-weighted sum
/// reconstructs the pooled GB trace × capacity exactly).
fn cf_entry(tech_id: &str, capacity_gw: f64, zone: &'static str, file_tech: &str) -> FleetEntry {
    FleetEntry {
        technology: TechId::new(tech_id),
        capacity_gw: Power::gigawatts(capacity_gw),
        capacity_factor_trace: Some(per_year_traces(&|y| {
            format!("data/packs/cf-gb2/{zone}_{file_tech}_cf_{y}.parquet")
        })),
        availability: None,
        reliability: None,
        inertia_h: None,
        synchronous: None,
        energy_budget: None,
    }
}

/// One zone's slice of the RS fleet at the zonal capacity shares, plus
/// its store (POWER = 100 GW × the zonal DEMAND share — the rating must
/// track zonal peak demand, so power placement is not a free knob;
/// ENERGY given, the free placement knob).
fn rs_zone(id: &'static str, energy_gwh: f64) -> ZoneSpec {
    let zone = if id == "SCO" { "sco" } else { "rgb" };
    let (onshore, offshore, solar) = if id == "SCO" {
        (SCO_ONSHORE, SCO_OFFSHORE, SCO_SOLAR)
    } else {
        (1.0 - SCO_ONSHORE, 1.0 - SCO_OFFSHORE, 1.0 - SCO_SOLAR)
    };
    let demand_share = demand_share_of(id);
    ZoneSpec {
        pricing: None,
        id: ZoneId::new(id),
        demand: DemandSpec {
            base_profile: per_year_traces(&|y| {
                format!("data/packs/demand-tiled/demand_{y}.parquet")
            }),
            column: "underlying_demand".to_owned(),
            extra_profiles: vec![],
            // The RS 570 TWh/yr level (annual_scale 2.177 on the tiled
            // 2024 profile) × the flat zonal demand share.
            annual_scale: 2.177 * demand_share,
            extra_demand_gw: Power::gigawatts(0.0),
            heating: None,
        },
        exogenous_supply: vec![],
        fleet: vec![
            cf_entry("offshore_wind", 240.0 * offshore, zone, "offshore"),
            cf_entry("onshore_wind", 80.0 * onshore, zone, "onshore"),
            cf_entry("solar", 200.0 * solar, zone, "solar"),
        ],
        storage: vec![hydrogen_store(100.0 * demand_share, energy_gwh)],
    }
}

fn demand_share_of(id: &str) -> f64 {
    if id == "SCO" {
        SCO_DEMAND
    } else {
        1.0 - SCO_DEMAND
    }
}

/// A hydrogen store at the RS conventions (η 0.40, initial full).
fn hydrogen_store(power_gw: f64, energy_gwh: f64) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Hydrogen,
        power_gw: Power::gigawatts(power_gw),
        energy_gwh: Energy::gigawatt_hours(energy_gwh),
        round_trip_efficiency: PerUnit::new(0.40),
        dispatch_order: 1,
        initial_soc: None, // full — the D4 default, as in the RS pin
        shift_duration: None,
        daily_volume_limit: None,
    }
}

fn full_horizon() -> Horizon {
    Horizon {
        start: "1985-01-01T00:00:00Z".to_owned(),
        end: "2024-12-31T23:30:00Z".to_owned(),
        weather_years: WeatherYears::All,
    }
}

/// The two-zone RS scenario at total hydrogen energy `total_gwh` split
/// by `es_share` (Scottish ENERGY placement; POWER stays demand-share),
/// with the given B6 capabilities (GW).
fn rs_two_zone(total_gwh: f64, es_share: f64, export_gw: f64, import_gw: f64) -> Scenario {
    Scenario {
        schema_version: 6,
        name: "royal-society-37y-2zone".to_owned(),
        description: None,
        horizon: full_horizon(),
        zones: vec![
            rs_zone("SCO", total_gwh * es_share),
            rs_zone("RGB", total_gwh * (1.0 - es_share)),
        ],
        links: vec![LinkSpec {
            name: Some("B6".to_owned()),
            from: ZoneId::new("SCO"),
            to: ZoneId::new("RGB"),
            capacity_gw: Power::gigawatts(export_gw),
            reverse_capacity_gw: Some(Power::gigawatts(import_gw)),
            capability_trace: None, // the ruling: no synthesised series off-2024
            availability: PerUnit::new(1.0),
            loss: PerUnit::new(0.0),
        }],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

/// A single-bus (one GB zone) RS scenario with the POOLED fleet (six CF
/// entries whose share-weighted sum reconstructs the pooled GB trace ×
/// capacity) and the given store portfolio. No links.
fn rs_single_bus(stores: Vec<StorageSpec>) -> Scenario {
    Scenario {
        schema_version: 6,
        name: "royal-society-37y-single-bus".to_owned(),
        description: None,
        horizon: full_horizon(),
        zones: vec![ZoneSpec {
            pricing: None,
            id: ZoneId::new("GB"),
            demand: DemandSpec {
                base_profile: per_year_traces(&|y| {
                    format!("data/packs/demand-tiled/demand_{y}.parquet")
                }),
                column: "underlying_demand".to_owned(),
                extra_profiles: vec![],
                annual_scale: 2.177,
                extra_demand_gw: Power::gigawatts(0.0),
                heating: None,
            },
            exogenous_supply: vec![],
            fleet: vec![
                cf_entry("offshore_wind_sco", 240.0 * SCO_OFFSHORE, "sco", "offshore"),
                cf_entry(
                    "offshore_wind_rgb",
                    240.0 * (1.0 - SCO_OFFSHORE),
                    "rgb",
                    "offshore",
                ),
                cf_entry("onshore_wind_sco", 80.0 * SCO_ONSHORE, "sco", "onshore"),
                cf_entry(
                    "onshore_wind_rgb",
                    80.0 * (1.0 - SCO_ONSHORE),
                    "rgb",
                    "onshore",
                ),
                cf_entry("solar_sco", 200.0 * SCO_SOLAR, "sco", "solar"),
                cf_entry("solar_rgb", 200.0 * (1.0 - SCO_SOLAR), "rgb", "solar"),
            ],
            storage: stores,
        }],
        links: vec![],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

/// Total unserved energy across all zones, GWh.
fn total_unserved_gwh(scenario: &Scenario, inputs: &MultiZoneInputs) -> f64 {
    let result = run_multi(scenario, inputs).unwrap();
    result
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum()
}

/// The shared bisection (module docs; `min_storage_for_zero_unserved`
/// conventions verbatim) over a caller-supplied feasibility predicate
/// on total store energy.
fn bisect(feasible: impl Fn(f64) -> bool) -> f64 {
    if feasible(0.0) {
        return 0.0;
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    loop {
        assert!(
            hi <= 1e6,
            "no storage size achieves zero unserved at the 10^6 GWh cap (the cliff)"
        );
        if feasible(hi) {
            break;
        }
        lo = hi;
        hi *= 2.0;
    }
    let tolerance = |hi: f64| (hi * 1e-3).max(0.1);
    while hi - lo > tolerance(hi) {
        let mid = (lo + hi) / 2.0;
        if feasible(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// Two-zone minimum total storage at the given energy placement and B6
/// capabilities (power split stays demand-share).
fn min_two_zone(es_share: f64, export_gw: f64, import_gw: f64, inputs: &MultiZoneInputs) -> f64 {
    bisect(|total| {
        total_unserved_gwh(&rs_two_zone(total, es_share, export_gw, import_gw), inputs) <= 1e-9
    })
}

/// Single-bus minimum total storage for a store portfolio built from
/// the total energy.
fn min_single_bus(stores: impl Fn(f64) -> Vec<StorageSpec>, inputs: &MultiZoneInputs) -> f64 {
    bisect(|total| total_unserved_gwh(&rs_single_bus(stores(total)), inputs) <= 1e-9)
}

/// The robustness demonstration (module docs): the headline two-zone
/// requirements, deltas quoted against the single-zone pin, everything
/// pinned. One test so the two-zone input load happens once.
#[test]
fn rs_requirement_under_the_two_zone_split_is_pinned_with_its_deltas() {
    require_packs();
    let root = repo_root();
    // Inputs are independent of store sizing and link capability: load
    // once from the B6-linked scenario shape.
    let scenario = rs_two_zone(1.0, SCO_DEMAND, 4.1, 3.5);
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();

    // Copper-plate two-zone (demand-share placement): the split + flow
    // convention, no boundary.
    let copper_gwh = min_two_zone(SCO_DEMAND, 1000.0, 1000.0, &inputs);
    // B6-linked at the ruling's central capabilities: + the boundary.
    let b6_gwh = min_two_zone(SCO_DEMAND, 4.1, 3.5, &inputs);

    let split_delta = 100.0 * (copper_gwh - SINGLE_ZONE_PIN_GWH) / SINGLE_ZONE_PIN_GWH;
    let link_delta_vs_single = 100.0 * (b6_gwh - SINGLE_ZONE_PIN_GWH) / SINGLE_ZONE_PIN_GWH;
    let link_delta_vs_copper = 100.0 * (b6_gwh - copper_gwh) / copper_gwh;
    eprintln!(
        "RS 2-zone robustness: copper-plate {copper_gwh} GWh ({split_delta:+.1} % vs the \
         single-zone 23,872; DISPATCH-CONVENTION-dominated, NOT the split — see the \
         controls test); B6 4.1/3.5 GW {b6_gwh} GWh ({link_delta_vs_single:+.1} % vs \
         single-zone, {link_delta_vs_copper:+.1} % vs 2-zone copper-plate at the SAME \
         placement — a DIAGNOSTIC, not a placement-stable 'boundary effect proper'; see \
         the §2 correction) — B6-only slice, LOWER BOUND on network effects, \
         rule-based dispatch upper-bias (LP is the Stage-7 resolver)"
    );

    // PINNED (first pass, 2026-07-04). The magnitude of the B6 delta is
    // the FINDING (module docs) — at full prominence, never a retune.
    assert!(
        (copper_gwh - PIN_2ZONE_COPPER_GWH).abs() < 1e-6,
        "PINNED 2-zone copper-plate requirement moved: measured {copper_gwh} GWh"
    );
    assert!(
        (b6_gwh - PIN_2ZONE_B6_GWH).abs() < 1e-6,
        "PINNED 2-zone B6 requirement moved: measured {b6_gwh} GWh"
    );
    // DIAGNOSTIC — not a quotable "boundary effect". Pin the exact
    // B6-vs-copper delta at the demand-share placement so it cannot
    // drift silently, but it is EXPLICITLY not a headline: it is one
    // point on the −1.7 %…+34.6 % same-placement swing (§2 CORRECTION),
    // entangled with the rule-based flow convention, and no single
    // "boundary effect proper" percentage is quotable until the Stage-7
    // LP separates boundary from dispatch. Value first measured
    // 2026-07-04 (35,648 vs 26,480 = +34.62 %).
    assert!(
        (link_delta_vs_copper - 34.622_356_495_468_28).abs() < 1e-6,
        "DIAGNOSTIC same-placement B6-vs-copper delta moved: measured {link_delta_vs_copper} % \
         (pinned +34.622 %; not a quotable boundary effect — see §2 correction)"
    );
    // NOTE (engine review §1b): "direction is structural" is FALSE for
    // this engine (at the 3 % placement B6 < copper — the controls test
    // pins the inversion). This assert is scoped ONLY to the
    // demand-share placement it measures, where B6 does exceed copper.
    assert!(
        b6_gwh >= copper_gwh,
        "at the demand-share placement the B6 requirement should exceed copper-plate \
         (the inversion lives at other placements — see the controls test)"
    );
}

/// The control/placement decomposition (engine review §1b/§1c/§1d):
/// the mechanism claims of the finding-of-record, each measured and
/// pinned. Separate test so the single-bus input load is isolated; the
/// two-zone placement points reuse one two-zone load.
#[test]
fn rs_mechanism_controls_and_placement_spread_are_pinned() {
    require_packs();
    let root = repo_root();

    // --- Single-bus controls (isolate trace substitution and the
    //     store split from the flow convention). ---
    let single_bus_inputs = load_multi_zone_inputs(&rs_single_bus(vec![]), &root).unwrap();
    // Control 1: pooled zonal traces, one 100 GW store — trace
    // substitution alone.
    let pooled = min_single_bus(
        |total| vec![hydrogen_store(100.0, total)],
        &single_bus_inputs,
    );
    // Control 2: same traces, store split 10.1/89.9 of power AND energy
    // on the single bus — the store split per se.
    let two_store = min_single_bus(
        |total| {
            vec![
                StorageSpec {
                    dispatch_order: 1,
                    ..hydrogen_store(100.0 * SCO_DEMAND, total * SCO_DEMAND)
                },
                StorageSpec {
                    dispatch_order: 2,
                    ..hydrogen_store(100.0 * (1.0 - SCO_DEMAND), total * (1.0 - SCO_DEMAND))
                },
            ]
        },
        &single_bus_inputs,
    );

    // --- Two-zone energy-placement spread (power fixed demand-share). ---
    let two_zone_inputs =
        load_multi_zone_inputs(&rs_two_zone(1.0, SCO_DEMAND, 4.1, 3.5), &root).unwrap();
    // B6 at the measured optimum (~3 % Scottish energy) and copper at
    // the SAME placement: the inversion (B6 < copper) that falsifies
    // "direction is structural".
    let b6_es003 = min_two_zone(ES_OPTIMUM, 4.1, 3.5, &two_zone_inputs);
    let copper_es003 = min_two_zone(ES_OPTIMUM, 1000.0, 1000.0, &two_zone_inputs);
    // B6 at the wind-share placement: the +106 % top of the spread.
    let b6_windshare = min_two_zone(ES_WIND_SHARE, 4.1, 3.5, &two_zone_inputs);

    eprintln!(
        "controls: pooled single-bus {pooled} GWh (trace substitution ~nil vs 23,872); \
         two-store single-bus {two_store} GWh (store split ~+1 %). \
         placement (B6 4.1/3.5): es=0.03 {b6_es003} GWh (optimum) vs copper es=0.03 \
         {copper_es003} GWh (INVERSION: B6 < copper — direction NOT structural); \
         wind-share es=0.348 {b6_windshare} GWh (+106 %). The B6-vs-copper same-placement \
         delta is NOT placement-stable: −1.7 % here (es=0.03) vs +34.6 % at demand-share \
         (35,648 vs 26,480) — DIAGNOSTIC, no single 'boundary effect proper' is quotable \
         (§2 correction; LP resolves at Stage 7)."
    );

    // PINNED (measured-then-pinned 2026-07-04; deterministic ADR-5).
    let pins = [
        ("pooled single-bus", pooled, PIN_POOLED_SINGLE_BUS_GWH),
        (
            "two-store single-bus",
            two_store,
            PIN_TWO_STORE_SINGLE_BUS_GWH,
        ),
        ("B6 es=0.03", b6_es003, PIN_B6_ES003_GWH),
        ("copper es=0.03", copper_es003, PIN_COPPER_ES003_GWH),
        ("B6 wind-share es=0.348", b6_windshare, PIN_B6_WINDSHARE_GWH),
    ];
    for (what, measured, pinned) in pins {
        assert!(
            (measured - pinned).abs() < 1e-6,
            "PINNED control/placement {what} moved: measured {measured} GWh, pinned {pinned}"
        );
    }

    // The mechanism claims, as asserts:
    // (a) trace substitution is ~nil (control 1 within ~1 % of the pin).
    assert!(
        (pooled - SINGLE_ZONE_PIN_GWH).abs() / SINGLE_ZONE_PIN_GWH < 0.01,
        "pooled single-bus should be ~= the 23,872 pin (trace substitution nil): {pooled}"
    );
    // (b) the store split alone is small (control 2 within ~2 %).
    assert!(
        (two_store - SINGLE_ZONE_PIN_GWH).abs() / SINGLE_ZONE_PIN_GWH < 0.02,
        "two-store single-bus should be ~+1 % (store split per se): {two_store}"
    );
    // (c) the inversion: a finite B6 is NOT monotone — at the 3 %
    //     placement it needs LESS than copper-plate (direction is NOT
    //     structural in this rule-based engine).
    assert!(
        b6_es003 < copper_es003,
        "expected the B6 < copper inversion at the 3 % placement: B6 {b6_es003} vs copper \
         {copper_es003}"
    );
    // (d) placement moves the B6 requirement materially (optimum ->
    //     wind-share is a large spread).
    assert!(
        b6_windshare > b6_es003 * 1.4,
        "the energy placement must move the B6 requirement materially: {b6_es003} -> \
         {b6_windshare}"
    );
}
