//! B6 two-zone acceptance and validation gates (the Richard-promoted
//! beta item; work order: memory/project-state V1/BETA item 2).
//!
//! THE RULING (docs/notes/b6-two-zone-data-review.md §6, implemented
//! verbatim — the scenario header restates it): the link is B6, not a
//! group aggregate; the 2024 validation configuration takes the
//! observed half-hourly DA limit series as the export capability
//! (sentinels: ≥ 9,999 MW → ETYS 6.7 GW; 0 → masked; missing stays
//! missing and masked out of gates); import 3.5 GW flat. Gates:
//!
//! - **(i)** modelled PRE-CONSTRAINT (copper-plate) B6 flow vs the DA
//!   flow series: correlation + net ≈ 22.6 TWh southward over the same
//!   observed-period mask;
//! - **(ii)** modelled CONSTRAINED export vs the 17 TWh Energy Trends
//!   outturn, carrying the ~2 TWh irreducible DA-vs-outturn wedge in
//!   the tolerance;
//! - **(iii)** binding frequency vs 23.6 % of periods at ≥ 99 % of the
//!   limit.
//!
//! Anchor conventions, recomputed from the pack (reviewer-verified
//! b6_report.json values reproduced by this file's own arithmetic):
//! 2024 has 17,214 observed rows of 17,568 periods (354 missing), 3
//! NaN rows → the **flow mask** is the 17,211 rows with valid flow AND
//! limit; net DA flow over it = 22.627189 TWh southward; the binding
//! share 0.236011852884783 counts rows with limit > 0 and
//! flow ≥ 0.99 × limit over the SAME 17,211-row denominator (the 53
//! zero-limit sentinel rows stay in the denominator, never the
//! numerator — reproduced exactly below before any model comparison).
//!
//! Tolerances (pinned AFTER the first runs quantified the wedges — the
//! ruling's deferral): each gate assert names the wedge budget it
//! carries. The named wedges (report §3/§6): ~2.1 TWh irreducible
//! DA-vs-outturn basis wedge; up to ~3 TWh 2024-specific Scottish
//! offshore overstatement (end-2024-fleet full-year convention, Moray
//! West/NnG); second-order flat-demand-share and zonal-availability
//! approximations.
//!
//! QUOTE DUTY (ruling (c), carried on every output of this file):
//! model curtailment/constraint numbers are a LOWER BOUND on the
//! Scottish constraint phenomenon — the two-zone geometry sees only
//! B6; B4/B5 (B4 cost 4× B6 in 2024) are structurally invisible. The
//! like-for-like cost anchor is B6 (£90.5m calendar 2024); the
//! Scottish group (£525.8m) is context only, NEVER a tuning target.
//!
//! Requires the locally built 2024 + cf-gb2 + b6 data packs; fails
//! loudly with build instructions if absent (trace-test precedent).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use grid_adequacy::{MultiZoneRunResult, load_multi_zone_inputs, run_multi};
use grid_core::scenario::Scenario;
use grid_core::time::UtcInstant;
use grid_core::trace::load_sparse_power_trace_mw;
use grid_core::units::Power;

const SCENARIO: &str = "scenarios/gb-2024-2zone.toml";
const PERIODS_2024: usize = 17_568;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Loud pack-presence check with build instructions (the trace-test
/// precedent): the 2024 pack, the cf-gb2 zonal traces and the b6 pack
/// are fetched-and-built, never committed.
fn require_packs() {
    let root = repo_root();
    for (rel, hint) in [
        (
            "data/packs/2024/processed/demand_2024.parquet",
            "scripts/fetch-2024 (fetch.py, build.py)",
        ),
        (
            "data/packs/cf-gb2/sco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb2zone.py",
        ),
        (
            "data/packs/b6/processed/b6_da_flows_limits.parquet",
            "scripts/fetch-b6 (fetch.py, build.py); verify data/packs/b6.sha256",
        ),
    ] {
        let path = root.join(rel);
        assert!(
            path.exists(),
            "data pack file missing: {} — build it first: {hint}",
            path.display()
        );
    }
}

/// The observed 2024 DA series aligned to the horizon: per period,
/// `Some(mw)` where the row exists with a valid value.
struct Observed {
    flow_mw: Vec<Option<f64>>,
    limit_mw: Vec<Option<f64>>,
}

fn observed() -> Observed {
    let root = repo_root();
    let path = root.join("data/packs/b6/processed/b6_da_flows_limits.parquet");
    let start = UtcInstant::parse("2024-01-01T00:00:00Z").unwrap();
    let align = |points: Vec<(UtcInstant, Option<Power>)>| -> Vec<Option<f64>> {
        let mut out = vec![None; PERIODS_2024];
        for (t, v) in points {
            let offset = t.unix_micros() - start.unix_micros();
            if offset < 0 || offset % grid_core::time::HALF_HOUR_MICROS != 0 {
                continue;
            }
            let index = (offset / grid_core::time::HALF_HOUR_MICROS) as usize;
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

/// The gate mask: periods with BOTH flow and limit observed (17,211 in
/// this pack retrieval — 354 missing rows + 3 NaN rows excluded).
fn flow_mask(obs: &Observed) -> Vec<bool> {
    (0..PERIODS_2024)
        .map(|t| obs.flow_mw[t].is_some() && obs.limit_mw[t].is_some())
        .collect()
}

/// Model southward (SCO→RGB) sending-end flow, GW per period: the B6
/// link's home end is SCO, so southward = −home_end (loss = 0).
fn southward_gw(result: &MultiZoneRunResult) -> Vec<f64> {
    let b6 = &result.links[0];
    assert_eq!(b6.name, "B6");
    assert_eq!(b6.from.as_str(), "SCO");
    b6.home_end.iter().map(|p| -p.as_gigawatts()).collect()
}

fn net_twh(southward_gw: &[f64], mask: Option<&[bool]>) -> f64 {
    southward_gw
        .iter()
        .enumerate()
        .filter(|(t, _)| mask.is_none_or(|m| m[*t]))
        .map(|(_, gw)| gw * 0.5)
        .sum::<f64>()
        / 1000.0
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let (va, vb): (f64, f64) = (
        a.iter().map(|x| (x - ma).powi(2)).sum(),
        b.iter().map(|y| (y - mb).powi(2)).sum(),
    );
    cov / (va.sqrt() * vb.sqrt())
}

fn validation_run() -> MultiZoneRunResult {
    let root = repo_root();
    let scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    run_multi(&scenario, &inputs).unwrap()
}

/// The copper-plate (pre-constraint) variant: the same scenario with
/// the B6 link unbounded — the model's unconstrained boundary flow.
fn copper_plate_run() -> MultiZoneRunResult {
    let root = repo_root();
    let mut scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    scenario.links[0].capability_trace = None;
    scenario.links[0].capacity_gw = Power::gigawatts(1000.0);
    scenario.links[0].reverse_capacity_gw = Some(Power::gigawatts(1000.0));
    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    run_multi(&scenario, &inputs).unwrap()
}

// ---------------------------------------------------------------------
// Anchor reproduction: this file's own arithmetic must land exactly on
// the reviewer-verified pack statistics before any model comparison.
// ---------------------------------------------------------------------

#[test]
fn observed_anchors_reproduce_the_reviewed_pack_statistics() {
    require_packs();
    let obs = observed();
    let mask = flow_mask(&obs);
    let mask_count = mask.iter().filter(|&&m| m).count();
    assert_eq!(
        mask_count, 17_211,
        "the 2024 flow mask (17,214 rows − 3 NaN)"
    );

    // Net DA flow over the mask: 22.627189 TWh southward.
    let net: f64 = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| obs.flow_mw[t].unwrap() * 0.5)
        .sum::<f64>()
        / 1e6;
    assert!((net - 22.627189).abs() < 1e-6, "net DA flow {net} TWh");

    // Binding share: limit > 0 AND flow ≥ 0.99 × limit, over the same
    // 17,211-row denominator = 0.236011852884783 (b6_report.json).
    let binding = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .filter(|&t| {
            let limit = obs.limit_mw[t].unwrap();
            limit > 0.0 && obs.flow_mw[t].unwrap() >= 0.99 * limit
        })
        .count();
    let share = binding as f64 / mask_count as f64;
    assert!(
        (share - 0.236011852884783).abs() < 1e-12,
        "observed binding share {share}"
    );
}

// ---------------------------------------------------------------------
// Gate (i): pre-constraint (copper-plate) B6 flow vs the DA flow
// series over the observed mask.
// ---------------------------------------------------------------------

#[test]
fn gate_i_pre_constraint_flow_matches_the_da_series() {
    require_packs();
    let obs = observed();
    let mask = flow_mask(&obs);
    let model = southward_gw(&copper_plate_run());

    let net = net_twh(&model, Some(&mask));
    let observed_masked: Vec<f64> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| obs.flow_mw[t].unwrap() / 1000.0)
        .collect();
    let model_masked: Vec<f64> = (0..PERIODS_2024)
        .filter(|&t| mask[t])
        .map(|t| model[t])
        .collect();
    let r = pearson(&model_masked, &observed_masked);
    eprintln!("gate (i): copper-plate net southward {net} TWh over the mask; r = {r}");

    // Anchor 22.627 TWh; measured 19.898 (−2.73; 18.809 pre-R7-fix —
    // docs/08 R7: released stall flows move TOWARD the anchor).
    // Tolerance budget
    // ±4.5 TWh, decomposed from the named wedges (first-run
    // quantification, per the ruling's deferral):
    //   − the demand-basis wedge: the adopted 10.1 % Scottish share is
    //     the Energy Trends CONSUMPTION basis; the metered basis is
    //     8.7 % — up to (10.1−8.7) % × 261.8 TWh ≈ −3.7 TWh of
    //     southward pressure sits between the two bases (report §4);
    //   − the flow-rule surplus-sharing convention: in joint-surplus
    //     periods the equalising rule splits curtailment between the
    //     zones instead of wheeling the full Scottish surplus south
    //     (flow-module prose rule 1) — direction negative;
    //   + the offshore commissioning wedge: the end-2024 fleet runs
    //     Moray West/NnG all year, up to +3 TWh (report §3);
    //   ± the DA series is NESO's forecast dispatch, not physics
    //     (3.51 TWh of it sits ABOVE the DA limit).
    // A miss beyond ±4.5 means the zonal split or the flow rule broke,
    // not a wedge.
    assert!(
        (net - 22.627189).abs() < 4.5,
        "gate (i) FAILED: copper-plate net southward {net:.3} TWh vs DA anchor 22.627 TWh \
         (±4.5 TWh wedge budget)"
    );
    // Correlation floor: measured 0.7443 on the first pass — the DA
    // series embeds NESO's forecast dispatch and outage schedule, so
    // r ≈ 1 is not attainable; a fall below 0.70 means the flow
    // structure broke, not a wedge.
    assert!(
        r > 0.70,
        "gate (i) FAILED: correlation {r:.4} with the DA flow series (floor 0.70)"
    );

    // PINNED measured values (first pass, 2026-07-04; deterministic
    // ADR-5 — a move is a knowing re-pin with the record).
    assert!(
        (net - PIN_GATE_I_NET_TWH).abs() < 1e-6,
        "PINNED gate (i) net moved: measured {net}"
    );
    assert!(
        (r - PIN_GATE_I_R).abs() < 1e-9,
        "PINNED gate (i) correlation moved: measured {r}"
    );
}

/// Gate (i) pins (first pass, 2026-07-04). Re-pinned 2026-07-06 for
/// the R7 flow-walk stall fix (docs/08 R7): the pre-fix walk silently
/// cap-truncated boundary-sliver stalls, withholding southward B6
/// flow; the released flow moves the copper-plate net TOWARD the DA
/// anchor. Was net 18.808794819925264 / r 0.7443336620570405.
const PIN_GATE_I_NET_TWH: f64 = 19.89819023464491;
const PIN_GATE_I_R: f64 = 0.7403737529621567;

// ---------------------------------------------------------------------
// Gate (ii): constrained export vs the 17 TWh Energy Trends outturn.
// ---------------------------------------------------------------------

#[test]
fn gate_ii_constrained_export_matches_the_energy_trends_outturn() {
    require_packs();
    let model = southward_gw(&validation_run());
    // Energy Trends: "Scotland transferred 17 TWh to England in 2024"
    // — a full-year ledger quantity, so the model total runs over ALL
    // periods (masked periods dispatch against the pinned 4.1 GW fill).
    let net = net_twh(&model, None);
    eprintln!("gate (ii): constrained net southward {net} TWh (anchor 17)");

    // Anchor 17 TWh; measured 16.406 (−0.59; 15.788 pre-R7-fix —
    // TOWARD the outturn). Tolerance budget
    // ±2.5 TWh: the ~2.1 TWh irreducible DA-vs-outturn basis wedge
    // (reviewer decomposition: clipping the DA flow at the DA limit
    // gives 19.12 TWh vs the 17 TWh outturn) is carried in full, plus
    // margin for the 354 missing periods dispatched at the 4.1 GW
    // fill. The gate-(i) demand-basis and offshore wedges largely
    // cancel against the constraint clipping here — the model lands
    // BETWEEN the two anchors, which is the expected position (the
    // anchors legitimately bracket the model: 22.6 unconstrained /
    // 17 constrained).
    assert!(
        (net - 17.0).abs() < 2.5,
        "gate (ii) FAILED: constrained net southward {net:.3} TWh vs the 17 TWh outturn \
         (±2.5 TWh wedge budget)"
    );

    // PINNED measured value (first pass, 2026-07-04).
    assert!(
        (net - PIN_GATE_II_NET_TWH).abs() < 1e-6,
        "PINNED gate (ii) net moved: measured {net}"
    );
}

/// Gate (ii) pin (first pass, 2026-07-04). Re-pinned 2026-07-06 (R7
/// stall fix — released southward flow moves the model TOWARD the
/// 17 TWh outturn anchor). Was 15.787702182212668.
const PIN_GATE_II_NET_TWH: f64 = 16.405946369726383;

// ---------------------------------------------------------------------
// Gate (iii): binding frequency vs 23.6 % of periods at ≥ 99 % of the
// limit.
// ---------------------------------------------------------------------

#[test]
fn gate_iii_binding_frequency_matches_the_observed_share() {
    require_packs();
    let result = validation_run();
    let model = southward_gw(&result);
    let capability = result.links[0].capability.as_ref().unwrap();
    let obs = observed();
    let mask = flow_mask(&obs);
    let mask_count = mask.iter().filter(|&&m| m).count();

    // The model analogue of the observed convention (module docs):
    // numerator = observed-capability periods (zero-limit sentinels
    // excluded — forward_observed carries exactly that mask) where the
    // modelled southward flow reaches 99 % of the capability the model
    // dispatched against; denominator = the same 17,211-row flow mask.
    let binding = (0..PERIODS_2024)
        .filter(|&t| mask[t] && capability.forward_observed[t])
        .filter(|&t| model[t] >= 0.99 * capability.forward[t].as_gigawatts())
        .count();
    let share = binding as f64 / mask_count as f64;
    eprintln!("gate (iii): model binding share {share} (anchor 0.236011852884783)");

    // Anchor 0.2360; measured 0.2323 (−0.4 pp). Tolerance ±0.04
    // absolute: the binding share inherits the gate-(i)/(ii) flow
    // wedges (a few TWh of southward-pressure error moves ~2–4 pp of
    // periods across the 99 % line at the observed limit
    // distribution). A miss beyond it means the capability series or
    // the flow rule is mis-wired, not a wedge.
    assert!(
        (share - 0.236011852884783).abs() < 0.04,
        "gate (iii) FAILED: model binding share {share:.4} vs observed 0.2360 (±0.04)"
    );

    // PINNED measured value (first pass, 2026-07-04). Re-pinned
    // 2026-07-06 (R7 stall fix, docs/08: released flows push more
    // periods to the 99 % line). Was 0.23229330079600255.
    assert!(
        (share - 0.25018883272325837).abs() < 1e-12,
        "PINNED gate (iii) binding share moved: measured {share}"
    );
}

// ---------------------------------------------------------------------
// The Q2/Q10 measurement: two-zone Scottish curtailment and B6-binding
// statistics vs copper-plate, at the reference fleet and at the 60 GW
// Module-1 high-wind point — the measured bracket the papers upgrade
// to.
//
// QUOTE DUTIES on every number (module docs; engine-review conditions):
//  - LOWER BOUND on the Scottish constraint phenomenon (B6-only slice);
//    the £525.8m Scottish-group cost is context, never a tuning target.
//  - "B6-ATTRIBUTABLE" SUBTRACTION RULE (engine review condition 7):
//    the constrained-minus-copper SCO curtailment delta must NEVER be
//    quoted alone. Blocking Scottish surplus at B6 shuffles curtailment
//    between zones: the RGB zone MOVES IN THE OPPOSITE DIRECTION in the
//    same comparison (at 60 GW: SCO +13.371 / RGB −6.725 TWh), so the
//    SYSTEM-NET B6 effect is +6.65 TWh. Any B6-attributable quote
//    carries BOTH legs, or quotes the system net.
// ---------------------------------------------------------------------

/// Curtailment TWh of one zone.
fn zone_curtailment_twh(result: &MultiZoneRunResult, id: &str) -> f64 {
    result
        .zone(id)
        .unwrap()
        .total_curtailment()
        .as_gigawatt_hours()
        / 1000.0
}

/// The B6-attributable curtailment subtraction, all three legs
/// (condition 7): SCO delta, RGB counter-movement, system net.
struct B6Attribution {
    sco_delta: f64,
    rgb_delta: f64,
    system_net: f64,
}

fn b6_attribution(
    sco_constrained: f64,
    sco_copper: f64,
    rgb_constrained: f64,
    rgb_copper: f64,
) -> B6Attribution {
    let sco_delta = sco_constrained - sco_copper;
    let rgb_delta = rgb_constrained - rgb_copper;
    B6Attribution {
        sco_delta,
        rgb_delta,
        system_net: sco_delta + rgb_delta,
    }
}

#[test]
fn q2_q10_reference_fleet_curtailment_bracket_is_pinned() {
    require_packs();
    let constrained = validation_run();
    let copper = copper_plate_run();

    let sco_constrained = zone_curtailment_twh(&constrained, "SCO");
    let rgb_constrained = zone_curtailment_twh(&constrained, "RGB");
    let sco_copper = zone_curtailment_twh(&copper, "SCO");
    let rgb_copper = zone_curtailment_twh(&copper, "RGB");
    // The B6-attributable subtraction, all three legs (condition 7):
    // never quote the SCO delta alone.
    let attr = b6_attribution(sco_constrained, sco_copper, rgb_constrained, rgb_copper);
    eprintln!(
        "Q2/Q10 reference fleet: SCO curtailment {sco_constrained} TWh constrained vs \
         {sco_copper} copper-plate; RGB {rgb_constrained} vs {rgb_copper}. \
         B6-ATTRIBUTABLE (condition 7): SCO {:+} / RGB {:+} / system net {:+} TWh \
         (LOWER BOUND on the Scottish constraint phenomenon — B6-only slice)",
        attr.sco_delta, attr.rgb_delta, attr.system_net
    );

    // Direction: the B6 limit can only ADD Scottish curtailment.
    assert!(
        sco_constrained >= sco_copper - 1e-9,
        "B6 constraint must not reduce Scottish curtailment"
    );

    // PINNED measured values (first pass, 2026-07-04).
    let pins = [
        ("SCO constrained", sco_constrained, PIN_REF_SCO_CONSTRAINED),
        ("RGB constrained", rgb_constrained, PIN_REF_RGB_CONSTRAINED),
        ("SCO copper-plate", sco_copper, PIN_REF_SCO_COPPER),
        ("RGB copper-plate", rgb_copper, PIN_REF_RGB_COPPER),
        // Condition 7: the system-net B6 effect is pinned alongside the
        // per-zone legs, so the subtraction can never drift to a
        // one-sided SCO quote.
        ("B6 system-net", attr.system_net, PIN_REF_SYSTEM_NET),
    ];
    for (what, measured, pinned) in pins {
        assert!(
            (measured - pinned).abs() < 1e-6,
            "PINNED Q2/Q10 reference {what} moved: measured {measured} TWh, pinned {pinned}"
        );
    }
}

#[test]
fn q2_q10_sixty_gw_high_wind_point_is_pinned() {
    require_packs();
    let root = repo_root();

    // The Module 1 convention: onshore + offshore wind scaled
    // PROPORTIONALLY to 60 GW total (from the 29.1 GW end-2024 GB
    // fleet), preserving both the on/offshore split and the zonal
    // shares. A SCALED run per the ruling takes the flat central
    // capabilities: export 4.1 / import 3.5 GW (no limit series exists
    // for a hypothetical fleet).
    let factor = 60.0 / 29.1;
    let mut scenario = Scenario::load(&root.join(SCENARIO)).unwrap();
    for zone in &mut scenario.zones {
        for entry in &mut zone.fleet {
            let tech = entry.technology.as_str();
            if tech == "onshore_wind" || tech == "offshore_wind" {
                entry.capacity_gw = entry.capacity_gw * factor;
            }
        }
    }
    scenario.links[0].capability_trace = None; // scaled run: flat 4.1/3.5

    let inputs = load_multi_zone_inputs(&scenario, &root).unwrap();
    let constrained = run_multi(&scenario, &inputs).unwrap();

    let mut copper = scenario.clone();
    copper.links[0].capacity_gw = Power::gigawatts(1000.0);
    copper.links[0].reverse_capacity_gw = Some(Power::gigawatts(1000.0));
    let copper_inputs = load_multi_zone_inputs(&copper, &root).unwrap();
    let copper = run_multi(&copper, &copper_inputs).unwrap();

    let sco_constrained = zone_curtailment_twh(&constrained, "SCO");
    let rgb_constrained = zone_curtailment_twh(&constrained, "RGB");
    let sco_copper = zone_curtailment_twh(&copper, "SCO");
    let rgb_copper = zone_curtailment_twh(&copper, "RGB");

    // Binding share at the flat 4.1 GW export capability (all periods
    // observed — no trace).
    let model = southward_gw(&constrained);
    let capability = constrained.links[0].capability.as_ref().unwrap();
    let binding = (0..PERIODS_2024)
        .filter(|&t| model[t] >= 0.99 * capability.forward[t].as_gigawatts())
        .count();
    let binding_share = binding as f64 / PERIODS_2024 as f64;
    let net = net_twh(&model, None);

    // The B6-attributable subtraction, all three legs (condition 7).
    let attr = b6_attribution(sco_constrained, sco_copper, rgb_constrained, rgb_copper);
    eprintln!(
        "Q2/Q10 at 60 GW wind (flat 4.1/3.5 GW B6): SCO curtailment {sco_constrained} \
         TWh vs copper-plate {sco_copper}; RGB {rgb_constrained} vs {rgb_copper}; \
         B6 export binding share {binding_share}; net southward {net} TWh. \
         B6-ATTRIBUTABLE (condition 7): SCO {:+} / RGB {:+} / system net {:+} TWh — the \
         SCO delta is NEVER quoted alone; blocked northward surplus-shuffling moves RGB \
         the opposite way (LOWER BOUND on the Scottish constraint phenomenon — B6-only \
         slice)",
        attr.sco_delta, attr.rgb_delta, attr.system_net
    );

    // PINNED measured values (first pass, 2026-07-04).
    let pins = [
        (
            "SCO constrained curtailment",
            sco_constrained,
            PIN_60GW_SCO_CONSTRAINED,
        ),
        (
            "RGB constrained curtailment",
            rgb_constrained,
            PIN_60GW_RGB_CONSTRAINED,
        ),
        (
            "SCO copper-plate curtailment",
            sco_copper,
            PIN_60GW_SCO_COPPER,
        ),
        (
            "RGB copper-plate curtailment",
            rgb_copper,
            PIN_60GW_RGB_COPPER,
        ),
        ("binding share", binding_share, PIN_60GW_BINDING_SHARE),
        ("net southward TWh", net, PIN_60GW_NET_TWH),
        // Condition 7: the three subtraction legs are pinned together.
        (
            "B6-attributable SCO delta",
            attr.sco_delta,
            PIN_60GW_SCO_DELTA,
        ),
        (
            "B6-attributable RGB delta",
            attr.rgb_delta,
            PIN_60GW_RGB_DELTA,
        ),
        (
            "B6-attributable system net",
            attr.system_net,
            PIN_60GW_SYSTEM_NET,
        ),
    ];
    for (what, measured, pinned) in pins {
        assert!(
            (measured - pinned).abs() < 1e-6,
            "PINNED Q2/Q10 60 GW {what} moved: measured {measured}, pinned {pinned}"
        );
    }
    // The counter-movement is real and opposite: RGB curtailment FALLS
    // as SCO's rises (the surplus-shuffling the subtraction rule names).
    assert!(
        attr.sco_delta > 0.0 && attr.rgb_delta < 0.0,
        "expected the opposite-sign B6 counter-movement (SCO up, RGB down)"
    );
}

// Q2/Q10 pins (first pass, 2026-07-04). QUOTE DUTY (ruling (c)):
// LOWER BOUND on the Scottish constraint phenomenon — B6-only slice.
//
// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7):
// the pre-fix walk silently cap-truncated boundary-sliver stalls; the
// released flow wheels more Scottish surplus south, so SCO curtailment
// FALLS and the copper-plate split lands at near-exact equal depth.
// Old values (pre-fix):
//   REF_SCO_CONSTRAINED 1.684001587134434 | REF_SCO_COPPER 0.048634694285419715
//   REF_RGB_COPPER 0.000071846378969326 (unmoved) | REF_SYSTEM_NET 1.635295046470045
//   60GW_SCO_CONSTRAINED 27.13942924577101 | 60GW_RGB_CONSTRAINED 3.695939609809804 (unmoved)
//   60GW_SCO_COPPER 13.76830342692668 | 60GW_RGB_COPPER 10.420581614357106
//   60GW_BINDING_SHARE 0.46738387978142076 | 60GW_NET_TWH 22.885651471726526
//   60GW_SCO_DELTA 13.371125818844328 | 60GW_RGB_DELTA -6.724642004547302
//   60GW_SYSTEM_NET 6.646483814297026
const PIN_REF_SCO_CONSTRAINED: f64 = 1.678730303423035;
const PIN_REF_RGB_CONSTRAINED: f64 = 0.0;
const PIN_REF_SCO_COPPER: f64 = 0.000071846378969326;
const PIN_REF_RGB_COPPER: f64 = 0.000071846378969326;
const PIN_60GW_SCO_CONSTRAINED: f64 = 27.02512748182898;
const PIN_60GW_RGB_CONSTRAINED: f64 = 3.695939609809804;
const PIN_60GW_SCO_COPPER: f64 = 10.934132203631213;
const PIN_60GW_RGB_COPPER: f64 = 10.934132203631211;
const PIN_60GW_BINDING_SHARE: f64 = 0.49237249544626593;
const PIN_60GW_NET_TWH: f64 = 23.410916797556098;
// The B6-attributable subtraction legs (condition 7), pinned together
// so the SCO delta can never be quoted without its RGB counter-move.
const PIN_REF_SYSTEM_NET: f64 = 1.6785866106650964;
const PIN_60GW_SCO_DELTA: f64 = 16.090995278197767;
const PIN_60GW_RGB_DELTA: f64 = -7.238192593821408;
const PIN_60GW_SYSTEM_NET: f64 = 8.852802684376359;
