//! Three-zone (N-Scotland / S-Scotland / E+W) ROBUSTNESS demonstration
//! (work-order deliverable 3): the Royal-Society-style 40-year storage
//! requirement (single-zone pin 23,872 GWh; two-zone copper 26,480 / B6
//! 35,648 — acceptance_stage3_rs37y.rs, acceptance_b6_robustness.rs)
//! recomputed under the three-zone split with the B4 + B6 links at their
//! non-2024 central capabilities (B4 1.8/4.0, B6 4.1/3.5 — no synthesised
//! limit series off-2024). Plus the Cruachan N↔S placement sensitivity
//! on the 2024 fleet (design-review Edit 3, obligation 3).
//!
//! # THE FINDING — framed per design-review item 5 verbatim (carries on
//! # EVERY quote of these numbers)
//!
//! Under the three-zone split with the demand-share store placement (the
//! two-zone headline convention), the RS 40-year storage requirement is
//! **37,824 GWh (+58.4% vs the single-zone 23,872; +6.1% vs the two-zone
//! B6 35,648)** at the ruling's central capabilities. The DIRECTION is
//! confirmed, but it is NOT a single clean monotone chain — the
//! zone-count/dispatch effect and the boundary-capacity effect
//! interleave. Ascending by VALUE:
//!
//!   single 23,872 < 2-zone copper 26,480 < 2-zone B6 35,648
//!                 < 3-zone copper 35,968 < 3-zone B6-only 36,416
//!                 < 3-zone B4+B6 37,824
//!
//! The two load-bearing facts (NOT a linear chain — 3-zone copper
//! 35,968 EXCEEDS 2-zone B6 35,648 with its boundary, because the
//! zone-count/dispatch effect dominates boundary capacity): (a) more
//! zones ⇒ more storage at every step (single < two-zone < three-zone);
//! (b) within a fixed zone count, adding the boundary raises it. VERIFIED
//! (work order): three-zone > two-zone > single-zone at the
//! demand-share placement — NO inversion here (unlike the two-zone
//! es=0.03 placement, where B6 < copper). The +58.4% headline carries
//! THREE conditions, always:
//!
//!  1. STRESS CONVENTION: end-2024 zonal shares + 2024 boundary
//!     capability projected onto a 520 GW fleet — a statement about
//!     TODAY's network under that fleet, never a future-network forecast.
//!  2. DISPATCH CONVENTION: the three-zone copper-plate baseline
//!     (35,968) is ALREADY +50.7% over single-zone — dominated by the
//!     RULE-BASED FLOW CONVENTION (flows clear before storage by
//!     surplus-depth equalisation, blind to store headroom;
//!     [`grid_adequacy::flow`] rules 1/3), NOT by the zone count. Adding
//!     the finite B4+B6 boundaries lifts it only a further +5.2%
//!     (35,968 -> 37,824). The "boundary effect" is NOT separable from
//!     the dispatch convention (item 5); every quote carries "rule-based
//!     dispatch, upper-bias" and the Stage-7 LP is the named resolver.
//!  3. LOWER-BOUND DUTY: B5 folded into the S-Scotland copper-plate
//!     under-states the S-internal constraint, so the three-zone result
//!     is a TIGHTER LOWER BOUND on the Scottish constraint phenomenon,
//!     not an adequate/complete representation (design-review item 1
//!     failure-mode A).
//!
//! Per obligation 2 / item 5: the model quotes the raw total-delta
//! DIRECTION and PINNED TOTALS under stated conventions ONLY — NO "B4
//! effect proper" %, NO B4-vs-B6 or boundary-vs-dispatch decomposition.
//!
//! # Cruachan N↔S sensitivity (obligation 3, on the 2024 fleet)
//!
//! Moving Cruachan (440 MW / 7.1 GWh pumped storage) from N-Scotland
//! (default; Y=728,674, ~18.7k north of the 710k line, SSEN-connected) to
//! S-Scotland changes the 2024 total Scottish curtailment by only ~5 GWh
//! (7.0078 -> 7.0024 TWh, −0.08%) — IMMATERIAL. The finding: PS placement
//! is second-order because the rule-based flow already strands the
//! northern surplus in N regardless of where the 440 MW store sits. Both
//! placements pinned. (On the RS synthetic fleet the store is the 100 GW
//! demand-share hydrogen store, so the 740 MW PS is a fortiori immaterial
//! there — the sensitivity is reported on the 2024 fleet, which carries
//! the actual PS.)
//!
//! # Bisection convention (acceptance_b6_robustness.rs verbatim)
//!
//! Doubling search from 1 GWh, halving to max(0.1, 1e-3×hi); requirement
//! = smallest known-feasible; feasible = total unserved across ALL zones
//! ≤ 1e-9 GWh. Convention-comparable with the single-zone 23,872 and the
//! two-zone pins.
//!
//! Requires the per-year 1985–2024 packs (demand-tiled + cf-gb2 rgb +
//! cf-gb3 nsco/ssco); fails loudly with build instructions if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{MultiZoneInputs, load_multi_zone_inputs, run_multi};
use grid_core::scenario::{
    DemandSpec, Dispatch, DispatchPolicyKind, ExogenousReliability, ExogenousSupplySpec,
    FleetEntry, Horizon, LinkSpec, Scenario, StorageKind, StorageSpec, TechId, TraceFiles,
    WeatherYears, ZoneId, ZoneSpec,
};
use grid_core::units::{Energy, PerUnit, Power};

/// Convention-comparable baselines (pinned elsewhere).
const SINGLE_ZONE_PIN_GWH: f64 = 23_872.0;
const PIN_2ZONE_COPPER_GWH: f64 = 26_480.0;
const PIN_2ZONE_B6_GWH: f64 = 35_648.0;

/// PINNED three-zone headline requirements (first pass 2026-07-04;
/// deterministic ADR-5). Exact whole GWh (binary-exact bisection).
const PIN_3ZONE_COPPER_GWH: f64 = 35_968.0;
const PIN_3ZONE_B6_ONLY_GWH: f64 = 36_416.0;
const PIN_3ZONE_FULL_GWH: f64 = 37_824.0;

/// PINNED Cruachan-both-ways 2024 total Scottish curtailment (TWh).
// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7 —
// released boundary flow wheels more northern surplus south, total
// Scottish curtailment falls ~0.03 TWh; the N-vs-S immateriality
// finding is unchanged). Was N 7.007757423828873 / S 7.002415475291746.
const PIN_CRUACHAN_N_TOTAL_TWH: f64 = 6.9746937532860835;
const PIN_CRUACHAN_S_TOTAL_TWH: f64 = 6.97416779108054;

// Scotland-of-GB shares (2-zone header) and N-of-Scotland fractions.
const SCO_DEMAND: f64 = 0.101;
const SCO_ONSHORE: f64 = 0.6997;
const SCO_OFFSHORE: f64 = 0.209150;
const SCO_SOLAR: f64 = 0.026738;
const N_ONSHORE: f64 = 0.4077;
const N_OFFSHORE: f64 = 0.9393;
const N_SOLAR: f64 = 0.6941;
const N_DEMAND: f64 = 0.33;

const SCENARIO_2024: &str = "scenarios/gb-2024-3zone.toml";

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
            format!("data/packs/cf-gb2/nsco_onshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/ssco_onshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/nsco_offshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/ssco_offshore_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/nsco_solar_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/ssco_solar_cf_{year}.parquet"),
            format!("data/packs/cf-gb2/rgb_onshore_cf_{year}.parquet"),
        ] {
            if !root.join(&rel).exists() {
                missing.push(rel);
            }
        }
    }
    assert!(
        missing.is_empty(),
        "per-year pack incomplete: {} file(s) missing, first {} — build with scripts/fetch-2024 \
         (tiled demand) and scripts/era5-cf/derive_cf_gb3zone.py (nsco/ssco traces; manifest \
         data/packs/cf-gb3-1985-2024.sha256)",
        missing.len(),
        missing[0]
    );
}

fn per_year_traces(pattern: &dyn Fn(i32) -> String) -> TraceFiles {
    TraceFiles::from_paths((1985..=2024).map(pattern).collect())
}

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

fn hydrogen_store(power_gw: f64, energy_gwh: f64) -> StorageSpec {
    StorageSpec {
        kind: StorageKind::Hydrogen,
        power_gw: Power::gigawatts(power_gw),
        energy_gwh: Energy::gigawatt_hours(energy_gwh),
        round_trip_efficiency: PerUnit::new(0.40),
        dispatch_order: 1,
        initial_soc: None,
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

/// GB-shares (onshore, offshore, solar, demand) of one of the three zones.
fn zone_shares(id: &str) -> (f64, f64, f64, f64) {
    match id {
        "NSCO" => (
            SCO_ONSHORE * N_ONSHORE,
            SCO_OFFSHORE * N_OFFSHORE,
            SCO_SOLAR * N_SOLAR,
            SCO_DEMAND * N_DEMAND,
        ),
        "SSCO" => (
            SCO_ONSHORE * (1.0 - N_ONSHORE),
            SCO_OFFSHORE * (1.0 - N_OFFSHORE),
            SCO_SOLAR * (1.0 - N_SOLAR),
            SCO_DEMAND * (1.0 - N_DEMAND),
        ),
        _ => (
            1.0 - SCO_ONSHORE,
            1.0 - SCO_OFFSHORE,
            1.0 - SCO_SOLAR,
            1.0 - SCO_DEMAND,
        ),
    }
}

/// One zone's slice of the RS fleet at the zonal shares, plus its store
/// (POWER = 100 GW × the zonal DEMAND share; ENERGY the free knob).
fn rs_zone(id: &'static str, energy_gwh: f64) -> ZoneSpec {
    let zone = match id {
        "NSCO" => "nsco",
        "SSCO" => "ssco",
        _ => "rgb",
    };
    let (onshore, offshore, solar, demand) = zone_shares(id);
    ZoneSpec {
        pricing: None,
        id: ZoneId::new(id),
        demand: DemandSpec {
            base_profile: per_year_traces(&|y| {
                format!("data/packs/demand-tiled/demand_{y}.parquet")
            }),
            column: "underlying_demand".to_owned(),
            extra_profiles: vec![],
            annual_scale: 2.177 * demand,
            extra_demand_gw: Power::gigawatts(0.0),
            heating: None,
        },
        exogenous_supply: vec![],
        fleet: vec![
            cf_entry("offshore_wind", 240.0 * offshore, zone, "offshore"),
            cf_entry("onshore_wind", 80.0 * onshore, zone, "onshore"),
            cf_entry("solar", 200.0 * solar, zone, "solar"),
        ],
        storage: vec![hydrogen_store(100.0 * demand, energy_gwh)],
    }
}

/// The three-zone RS scenario at total hydrogen energy split by DEMAND
/// share, with the given B4/B6 capabilities (GW).
fn rs_three_zone(total_gwh: f64, b4_fwd: f64, b4_rev: f64, b6_fwd: f64, b6_rev: f64) -> Scenario {
    let dn = zone_shares("NSCO").3;
    let ds = zone_shares("SSCO").3;
    let de = zone_shares("RGB").3;
    Scenario {
        schema_version: 6,
        name: "royal-society-37y-3zone".to_owned(),
        description: None,
        horizon: full_horizon(),
        zones: vec![
            rs_zone("NSCO", total_gwh * dn),
            rs_zone("SSCO", total_gwh * ds),
            rs_zone("RGB", total_gwh * de),
        ],
        links: vec![
            LinkSpec {
                name: Some("B4".to_owned()),
                from: ZoneId::new("NSCO"),
                to: ZoneId::new("SSCO"),
                capacity_gw: Power::gigawatts(b4_fwd),
                reverse_capacity_gw: Some(Power::gigawatts(b4_rev)),
                capability_trace: None,
                availability: PerUnit::new(1.0),
                loss: PerUnit::new(0.0),
            },
            LinkSpec {
                name: Some("B6".to_owned()),
                from: ZoneId::new("SSCO"),
                to: ZoneId::new("RGB"),
                capacity_gw: Power::gigawatts(b6_fwd),
                reverse_capacity_gw: Some(Power::gigawatts(b6_rev)),
                capability_trace: None,
                availability: PerUnit::new(1.0),
                loss: PerUnit::new(0.0),
            },
        ],
        dispatch: Dispatch {
            flow_signal: Default::default(),
            policy: DispatchPolicyKind::RuleBased,
        },
        constraints: None,
        solver: None,
        pricing: None,
    }
}

fn total_unserved_gwh(scenario: &Scenario, inputs: &MultiZoneInputs) -> f64 {
    let result = run_multi(scenario, inputs).unwrap();
    result
        .zones
        .iter()
        .map(|z| z.result.total_unserved().as_gigawatt_hours())
        .sum()
}

fn bisect(feasible: impl Fn(f64) -> bool) -> f64 {
    if feasible(0.0) {
        return 0.0;
    }
    let mut lo = 0.0;
    let mut hi = 1.0;
    loop {
        assert!(
            hi <= 1e6,
            "no storage size achieves zero unserved at the 10^6 GWh cap"
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

fn min_three_zone(
    b4_fwd: f64,
    b4_rev: f64,
    b6_fwd: f64,
    b6_rev: f64,
    inputs: &MultiZoneInputs,
) -> f64 {
    bisect(|total| {
        total_unserved_gwh(
            &rs_three_zone(total, b4_fwd, b4_rev, b6_fwd, b6_rev),
            inputs,
        ) <= 1e-9
    })
}

/// The robustness headline: the three-zone requirement, its deltas, and
/// the monotone DIRECTION, all pinned. One test so the 40-year input load
/// happens once.
#[test]
fn rs_requirement_under_the_three_zone_split_is_pinned_with_its_direction() {
    require_packs();
    let root = repo_root();
    let scenario = rs_three_zone(1.0, 1.8, 4.0, 4.1, 3.5);
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();

    // Copper (both links unbounded): the split + flow convention, no
    // boundary.
    let copper = min_three_zone(1000.0, 1000.0, 1000.0, 1000.0, &inputs);
    // B6 only (B4 unbounded, B6 constrained): the two-zone analogue.
    let b6_only = min_three_zone(1000.0, 1000.0, 4.1, 3.5, &inputs);
    // Full three-zone (B4 + B6 constrained): the headline.
    let full = min_three_zone(1.8, 4.0, 4.1, 3.5, &inputs);

    eprintln!(
        "RS 3-zone (demand-share placement): copper {copper} GWh ({:+.1}% vs single 23,872 — \
         DISPATCH-CONVENTION-dominated, NOT the zone count); B6-only {b6_only}; B4+B6 {full} GWh \
         ({:+.1}% vs single, {:+.1}% vs two-zone B6 35,648, {:+.1}% vs 3-zone copper). DIRECTION \
         (obligation 2, NOT a linear chain — the effects interleave): single 23,872 < 2z-copper \
         26,480 < 2z-B6 35,648 < 3z-copper {copper} < 3z-B6-only {b6_only} < 3z-B4+B6 {full}. \
         3z-copper EXCEEDS 2z-B6 (zone-count/dispatch dominates boundary capacity). Two safe facts: \
         (a) three-zone > two-zone > single-zone; (b) within a zone count, the boundary raises it. \
         Rule-based dispatch upper-bias, B5-folded lower bound; LP is the Stage-7 resolver. No \
         'B4 effect proper' %.",
        100.0 * (copper - SINGLE_ZONE_PIN_GWH) / SINGLE_ZONE_PIN_GWH,
        100.0 * (full - SINGLE_ZONE_PIN_GWH) / SINGLE_ZONE_PIN_GWH,
        100.0 * (full - PIN_2ZONE_B6_GWH) / PIN_2ZONE_B6_GWH,
        100.0 * (full - copper) / copper,
    );

    // PINNED totals (the magnitudes are the FINDING; a move is a knowing
    // re-pin with the record).
    assert!(
        (copper - PIN_3ZONE_COPPER_GWH).abs() < 1e-6,
        "PINNED 3-zone copper moved: {copper}"
    );
    assert!(
        (b6_only - PIN_3ZONE_B6_ONLY_GWH).abs() < 1e-6,
        "PINNED 3-zone B6-only moved: {b6_only}"
    );
    assert!(
        (full - PIN_3ZONE_FULL_GWH).abs() < 1e-6,
        "PINNED 3-zone B4+B6 moved: {full}"
    );

    // The DIRECTION that survives (design-review item 5): three-zone >
    // two-zone > single-zone. VERIFIED, not assumed. Monotone at this
    // placement (no inversion — reported if it ever moves).
    assert!(
        SINGLE_ZONE_PIN_GWH < PIN_2ZONE_COPPER_GWH
            && PIN_2ZONE_COPPER_GWH < copper
            && copper <= b6_only
            && b6_only <= full,
        "the three-zone requirement must exceed the two-zone and single-zone (monotone): \
         single {SINGLE_ZONE_PIN_GWH} < 2z-copper {PIN_2ZONE_COPPER_GWH} < 3z-copper {copper} \
         <= 3z-B6-only {b6_only} <= 3z-B4+B6 {full}"
    );
    assert!(
        full > PIN_2ZONE_B6_GWH,
        "the three-zone B4+B6 requirement must exceed the two-zone B6 {PIN_2ZONE_B6_GWH}: {full}"
    );
}

// =====================================================================
// Cruachan N↔S placement sensitivity on the 2024 fleet (obligation 3,
// design-review Edit 3). Both placements pinned; the finding is that PS
// placement is immaterial under the rule-based flow convention.
// =====================================================================

fn zone_curt(result: &grid_adequacy::MultiZoneRunResult, id: &str) -> f64 {
    result
        .zone(id)
        .unwrap()
        .total_curtailment()
        .as_gigawatt_hours()
        / 1000.0
}

fn run_2024(scenario: &Scenario) -> grid_adequacy::MultiZoneRunResult {
    let inputs = load_multi_zone_inputs(scenario, &repo_root()).unwrap();
    run_multi(scenario, &inputs).unwrap()
}

#[test]
fn cruachan_n_vs_s_sensitivity_is_pinned_both_ways() {
    require_packs();
    let root = repo_root();
    // Default: Cruachan in N (the committed scenario).
    let default_n = Scenario::load(&root.join(SCENARIO_2024)).unwrap();
    let rn = run_2024(&default_n);
    let n_total = zone_curt(&rn, "NSCO") + zone_curt(&rn, "SSCO") + zone_curt(&rn, "RGB");

    // Sensitivity: Cruachan (440 MW / 7.1 GWh) N -> S. N keeps Foyers
    // (300 MW / 6.3 GWh); the exogenous PS split follows the station MW.
    let mut cruachan_s = default_n.clone();
    {
        let n = cruachan_s
            .zones
            .iter_mut()
            .find(|z| z.id.as_str() == "NSCO")
            .unwrap();
        let ps = n
            .storage
            .iter_mut()
            .find(|s| s.kind == StorageKind::PumpedHydro)
            .unwrap();
        ps.power_gw = Power::gigawatts(0.30);
        ps.energy_gwh = Energy::gigawatt_hours(6.3);
        let ex = n
            .exogenous_supply
            .iter_mut()
            .find(|e| e.label == "pumped_storage_net")
            .unwrap();
        ex.scale = 300.0 / 2828.0;
    }
    {
        let s = cruachan_s
            .zones
            .iter_mut()
            .find(|z| z.id.as_str() == "SSCO")
            .unwrap();
        s.storage.insert(
            0,
            StorageSpec {
                kind: StorageKind::PumpedHydro,
                power_gw: Power::gigawatts(0.44),
                energy_gwh: Energy::gigawatt_hours(7.1),
                round_trip_efficiency: PerUnit::new(0.76),
                dispatch_order: 2,
                initial_soc: None,
                shift_duration: None,
                daily_volume_limit: None,
            },
        );
        s.exogenous_supply.push(ExogenousSupplySpec {
            label: "pumped_storage_net".to_owned(),
            path: TraceFiles::from_paths(vec![
                "data/packs/2024/processed/generation_by_fuel_2024.parquet".to_owned(),
            ]),
            columns: vec!["ps".to_owned()],
            imports: false,
            reliability: ExogenousReliability::Excluded,
            scale: 440.0 / 2828.0,
        });
    }
    let rs = run_2024(&cruachan_s);
    let s_total = zone_curt(&rs, "NSCO") + zone_curt(&rs, "SSCO") + zone_curt(&rs, "RGB");

    eprintln!(
        "Cruachan sensitivity (2024 fleet): total Scottish curtailment N-placement {n_total} TWh \
         vs S-placement {s_total} TWh (Δ {:.4} GWh) — IMMATERIAL: PS placement is second-order \
         because the rule-based flow strands the northern surplus in N regardless of the 440 MW \
         store's side. Both pinned.",
        (n_total - s_total) * 1000.0
    );

    assert!(
        (n_total - PIN_CRUACHAN_N_TOTAL_TWH).abs() < 1e-6,
        "PINNED Cruachan-N total moved: {n_total}"
    );
    assert!(
        (s_total - PIN_CRUACHAN_S_TOTAL_TWH).abs() < 1e-6,
        "PINNED Cruachan-S total moved: {s_total}"
    );
    // The finding: the placement changes the total by < 0.1%.
    assert!(
        (n_total - s_total).abs() / n_total < 0.001,
        "Cruachan N↔S placement should be immaterial (<0.1%): N {n_total} vs S {s_total}"
    );
}
