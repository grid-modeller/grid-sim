//! Pinned characterisation tests for the capture-wedge figure rows
//! (Module 1 sweep, 2024 reference scenario; quoted in the CfD
//! cannibalisation essay and destined for the book figure P-Q10).
//!
//! Published-number rule (CLAUDE.md): every quoted number gets a pinned
//! regression test before it is quoted anywhere. The quoted rows are
//! the 25, 30, 60 and 80 GW points of
//! `sweep wind-capacity --scenario scenarios/gb-2024-reference.toml`
//! (frozen import convention, unsuffixed columns, POTENTIAL capture
//! basis): mean SMP, potential capture ratio, curtailment, and the
//! 80 GW curtailment share of renewable potential. The downstream
//! capture price (ratio × mean SMP) and the £91-strike wedge are
//! derived in the figure repo, so pinning these columns pins them.
//!
//! Re-pinned 2026-07-07 on a clean engine (the first measurement ran
//! on a dirty tree whose hash did not survive the 2026-07-07 history
//! rewrite; the clean re-run reproduced every data row bit-for-bit).
//! Scenario sha256 c4dbdd44…, 60 GW row identical to the Package A/B
//! pins in `regression_imports_bracket_2024.rs`.
//!
//! Interpretation caveats travel with the figure, not this test: the
//! £0 SMP floor understates cannibalisation but overstates the AR4+
//! top-up in floored hours, and the 2024-frozen fleet overstates it
//! (sweep CSV assumption block; grid-core pricing conventions).
//!
//! Requires the locally built 2024 data pack; fails loudly if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// One quoted row's pinned values, measured 2026-07-07 (engine
/// a6bbf35, clean tree).
struct Pinned {
    wind_gw: f64,
    mean_smp_gbp_per_mwh: f64,
    /// Potential-basis capture ratio (the frozen-convention
    /// unsuffixed column — the figure's value factor).
    capture_ratio: f64,
    curtailment_twh: f64,
}

const PINNED: [Pinned; 4] = [
    Pinned {
        wind_gw: 25.0,
        mean_smp_gbp_per_mwh: 78.74374304205612,
        capture_ratio: 0.990508807173759,
        curtailment_twh: 0.0,
    },
    Pinned {
        wind_gw: 30.0,
        mean_smp_gbp_per_mwh: 72.95010153172663,
        capture_ratio: 0.9226965776690815,
        curtailment_twh: 0.0016276638716311708,
    },
    Pinned {
        wind_gw: 60.0,
        mean_smp_gbp_per_mwh: 37.13886422215412,
        capture_ratio: 0.5347799945293277,
        curtailment_twh: 21.845913344574633,
    },
    Pinned {
        wind_gw: 80.0,
        mean_smp_gbp_per_mwh: 26.569293428772706,
        capture_ratio: 0.4284163801683807,
        curtailment_twh: 58.57495702560469,
    },
];

/// The 80 GW curtailment share of renewable potential quoted in prose
/// ("24% of renewable potential").
const PIN_80_CURTAILMENT_PCT_OF_POTENTIAL: f64 = 24.299260457766405;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn require_pack() {
    let probe = repo_root().join("data/packs/2024/processed/demand_2024.parquet");
    assert!(
        probe.exists(),
        "2024 data pack is missing ({}) — build the pack first",
        probe.display()
    );
}

/// Run one sweep covering the given range and return the CSV text,
/// cached per (min, max, step) for the test process.
fn sweep_csv(
    min_gw: &str,
    max_gw: &str,
    step_gw: &str,
    cache: &'static OnceLock<String>,
) -> &'static str {
    cache.get_or_init(|| {
        require_pack();
        let out_dir = std::env::temp_dir()
            .join("grid-cli-capture-wedge-tests")
            .join(format!("pinned-{min_gw}-{max_gw}-{step_gw}"));
        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
            .args([
                "sweep",
                "wind-capacity",
                "--scenario",
                "scenarios/gb-2024-reference.toml",
                "--out",
                out_dir.to_str().unwrap(),
                "--min-gw",
                min_gw,
                "--max-gw",
                max_gw,
                "--step-gw",
                step_gw,
            ])
            .current_dir(repo_root())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "capture-wedge pin sweep failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(out_dir.join("module1_gas_marginal_vs_wind.csv")).unwrap()
    })
}

/// The shelf points (25, 30 GW) and the deep-penetration points
/// (60, 80 GW), two 2-point sweeps rather than one 12-point sweep.
fn shelf_csv() -> &'static str {
    static CSV: OnceLock<String> = OnceLock::new();
    sweep_csv("25", "30", "5", &CSV)
}

fn deep_csv() -> &'static str {
    static CSV: OnceLock<String> = OnceLock::new();
    sweep_csv("60", "80", "20", &CSV)
}

/// Parse the CSV into (header, data rows).
fn table(csv: &str) -> (Vec<&str>, Vec<Vec<f64>>) {
    let mut lines = csv.lines().filter(|l| !l.starts_with('#'));
    let header: Vec<&str> = lines.next().unwrap().split(',').collect();
    let rows = lines
        .map(|l| l.split(',').map(|v| v.parse::<f64>().unwrap()).collect())
        .collect();
    (header, rows)
}

fn column(header: &[&str], rows: &[Vec<f64>], name: &str, wind_gw: f64) -> f64 {
    let col = header
        .iter()
        .position(|c| *c == name)
        .unwrap_or_else(|| panic!("CSV has no column {name:?}"));
    let wind_col = header
        .iter()
        .position(|c| *c == "wind_capacity_gw")
        .unwrap();
    rows.iter()
        .find(|r| r[wind_col] == wind_gw)
        .unwrap_or_else(|| panic!("no row at {wind_gw} GW"))[col]
}

fn row_csv(wind_gw: f64) -> &'static str {
    if wind_gw < 60.0 {
        shelf_csv()
    } else {
        deep_csv()
    }
}

const RATIO_TOL: f64 = 1e-7; // bit-deterministic engine (ADR-5)
const TWH_TOL: f64 = 1e-6;
const SMP_TOL: f64 = 1e-6;
const PCT_TOL: f64 = 1e-6;

fn assert_pinned(what: &str, measured: f64, pinned: f64, tolerance: f64) {
    assert!(
        (measured - pinned).abs() <= tolerance,
        "{what}: measured {measured:.12} vs pinned {pinned:.12} (±{tolerance:e}) — if the \
         change is intentional, update this pin AND the published figure together"
    );
}

/// The four quoted rows: mean SMP, potential capture ratio,
/// curtailment.
#[test]
fn quoted_capture_wedge_rows_are_pinned() {
    for pin in &PINNED {
        let (header, rows) = table(row_csv(pin.wind_gw));
        assert_pinned(
            &format!("{} GW mean SMP", pin.wind_gw),
            column(&header, &rows, "mean_smp_gbp_per_mwh", pin.wind_gw),
            pin.mean_smp_gbp_per_mwh,
            SMP_TOL,
        );
        assert_pinned(
            &format!("{} GW potential capture ratio", pin.wind_gw),
            column(&header, &rows, "wind_capture_ratio", pin.wind_gw),
            pin.capture_ratio,
            RATIO_TOL,
        );
        assert_pinned(
            &format!("{} GW curtailment", pin.wind_gw),
            column(&header, &rows, "curtailment_twh", pin.wind_gw),
            pin.curtailment_twh,
            TWH_TOL,
        );
    }
}

/// The prose claim "58.6 TWh is 24% of renewable potential" at 80 GW.
#[test]
fn curtailment_share_of_potential_at_80_gw_is_pinned() {
    let (header, rows) = table(deep_csv());
    assert_pinned(
        "80 GW curtailment % of renewable potential",
        column(
            &header,
            &rows,
            "curtailment_pct_of_renewable_potential",
            80.0,
        ),
        PIN_80_CURTAILMENT_PCT_OF_POTENTIAL,
        PCT_TOL,
    );
}

/// The figure's shape claim: the capture ratio declines strictly
/// across the quoted points (flat shelf into structural decline).
#[test]
fn capture_ratio_declines_across_the_quoted_points() {
    let ratios: Vec<f64> = PINNED
        .iter()
        .map(|pin| {
            let (header, rows) = table(row_csv(pin.wind_gw));
            column(&header, &rows, "wind_capture_ratio", pin.wind_gw)
        })
        .collect();
    assert!(
        ratios.windows(2).all(|w| w[0] > w[1]),
        "capture ratio no longer declines across 25/30/60/80 GW: {ratios:?}"
    );
}
