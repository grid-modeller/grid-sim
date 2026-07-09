//! Pinned characterisation tests for the Package B import-convention
//! bracket on the Module 1 sweep (2024 reference scenario, 40 and
//! 60 GW points; first measured 2026-07-03).
//!
//! Conventions (prose at `grid_adequacy::import_convention`): FROZEN
//! (imports at observed values — the unchanged default),
//! ZERO-IN-SURPLUS, EXPORT-IN-SURPLUS (cap = min(export capability,
//! surplus); capability from the scenario's own links: 9.8 GW
//! nameplate ex-Greenlink × 0.95 availability = 9.31 GW).
//!
//! # Measured directions, worked out rather than assumed
//!
//! - **Potential-basis capture is IDENTICAL across the conventions**
//!   at these points: the transformations only touch pre-import
//!   surplus periods, which stay must-take-only (£0-priced) under all
//!   three conventions, so the SMP series — and hence wind revenue and
//!   the mean price — never move. (Zero vs export is structural: both
//!   keep the post-import balance ≥ 0 in mask periods. Frozen vs zero
//!   could in principle differ, if observed exports exceeded the swept
//!   surplus in a mask period and flipped it to gas; no such period
//!   exists in the 2024 data at 40–60 GW.) The bracket therefore acts
//!   on CURTAILMENT and delivered energy, not on price formation —
//!   the model has no export-price channel (tier 2).
//! - **Curtailment bracket at the high-curtailment point (60 GW):
//!   frozen ≥ zero ≥ export holds** (21.85 / 17.80 / 5.33 TWh). At
//!   40 GW it INVERTS between frozen and zero (0.77 vs 1.18 TWh):
//!   the 40 GW mask covers only the deepest-wind periods, where the
//!   OBSERVED 2024 trace was already exporting — zeroing it removes
//!   real export relief and deepens the surplus. Documented, not
//!   asserted as an invariant.
//! - **The work order's naive capture ordering (frozen ≤ zero ≤
//!   export) INVERTS**: at 60 GW the delivered capture ratio runs
//!   frozen 0.6106 ≥ zero 0.5953 ≥ export 0.5514. Mechanism: with
//!   prices pinned (above), more export relief means less curtailment,
//!   i.e. MORE delivered energy earning £0 in the still-£0 surplus
//!   periods — a larger zero-revenue denominator, hence a LOWER
//!   delivered capture ratio. The true invariant asserted here is
//!   zero ≥ export (structural: identical revenue and mean SMP, and
//!   export curtails less). The bracket's capture message is therefore
//!   a WIDTH (±0.03 around zero at 60 GW), not a one-sided correction.
//!
//! Requires the locally built 2024 data pack; fails loudly if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// One point's pinned values: (potential ratio, delivered ratio,
/// curtailment TWh) per convention, measured 2026-07-03.
struct Pinned {
    wind_gw: f64,
    // Frozen — MUST equal the Package A values bit-for-bit (the frozen
    // default is unchanged; a move here is a defect, not a re-pin).
    frozen: (f64, f64, f64),
    zero: (f64, f64, f64),
    export: (f64, f64, f64),
}

const PINNED: [Pinned; 2] = [
    Pinned {
        wind_gw: 40.0,
        frozen: (0.7128503657378394, 0.717504618821603, 0.7747882502778707),
        zero: (0.7128503657378394, 0.7200840143439774, 1.1755367901103146),
        export: (0.7128503657378394, 0.7140435797728879, 0.19847387677560588),
    },
    Pinned {
        wind_gw: 60.0,
        frozen: (0.5347799945293277, 0.6106059846371504, 21.845913344574633),
        zero: (0.5347799945293277, 0.5952510429390278, 17.79769682262425),
        export: (0.5347799945293277, 0.5514484407085398, 5.3280243997597205),
    },
];

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

/// Run the pinned bracket sweep (40 and 60 GW) once per test process
/// and return the CSV text.
fn pinned_sweep_csv() -> &'static str {
    static CSV: OnceLock<String> = OnceLock::new();
    CSV.get_or_init(|| {
        require_pack();
        let out_dir = std::env::temp_dir()
            .join("grid-cli-imports-bracket-tests")
            .join("pinned-regression");
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
                "40",
                "--max-gw",
                "60",
                "--step-gw",
                "20",
            ])
            .current_dir(repo_root())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "pinned bracket sweep failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(out_dir.join("module1_gas_marginal_vs_wind.csv")).unwrap()
    })
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

const RATIO_TOL: f64 = 1e-7; // bit-deterministic engine (ADR-5)
const TWH_TOL: f64 = 1e-6;

fn assert_pinned(what: &str, measured: f64, pinned: f64, tolerance: f64) {
    assert!(
        (measured - pinned).abs() <= tolerance,
        "{what}: measured {measured:.12} vs pinned {pinned:.12} (±{tolerance:e}) — if the \
         change is intentional, update this pin and the Package B record together"
    );
}

/// Requirement (1): the FROZEN default is bit-identical to the
/// Package A outputs — same column names, same values.
#[test]
fn frozen_default_columns_are_bit_identical_to_package_a() {
    let (header, rows) = table(pinned_sweep_csv());
    for pin in &PINNED {
        let (potential, delivered, curtailment) = pin.frozen;
        assert_pinned(
            &format!("{} GW frozen potential capture", pin.wind_gw),
            column(&header, &rows, "wind_capture_ratio", pin.wind_gw),
            potential,
            RATIO_TOL,
        );
        assert_pinned(
            &format!("{} GW frozen delivered capture", pin.wind_gw),
            column(&header, &rows, "wind_capture_ratio_delivered", pin.wind_gw),
            delivered,
            RATIO_TOL,
        );
        assert_pinned(
            &format!("{} GW frozen curtailment", pin.wind_gw),
            column(&header, &rows, "curtailment_twh", pin.wind_gw),
            curtailment,
            TWH_TOL,
        );
    }
}

/// Requirement (4): characterisation pins for the 40 and 60 GW points
/// under the zero-in-surplus and export-in-surplus conventions, both
/// capture bases plus curtailment.
#[test]
fn pinned_bracket_values_at_40_and_60_gw() {
    let (header, rows) = table(pinned_sweep_csv());
    for pin in &PINNED {
        for (suffix, (potential, delivered, curtailment)) in
            [("imports_zero", pin.zero), ("imports_export", pin.export)]
        {
            assert_pinned(
                &format!("{} GW {suffix} potential capture", pin.wind_gw),
                column(
                    &header,
                    &rows,
                    &format!("wind_capture_ratio_{suffix}"),
                    pin.wind_gw,
                ),
                potential,
                RATIO_TOL,
            );
            assert_pinned(
                &format!("{} GW {suffix} delivered capture", pin.wind_gw),
                column(
                    &header,
                    &rows,
                    &format!("wind_capture_ratio_delivered_{suffix}"),
                    pin.wind_gw,
                ),
                delivered,
                RATIO_TOL,
            );
            assert_pinned(
                &format!("{} GW {suffix} curtailment", pin.wind_gw),
                column(
                    &header,
                    &rows,
                    &format!("curtailment_twh_{suffix}"),
                    pin.wind_gw,
                ),
                curtailment,
                TWH_TOL,
            );
        }
    }
}

/// Requirement (3), as the TRUE invariants (module docs: the work
/// order's naive capture ordering inverts; the curtailment ordering
/// holds at the high-curtailment point but inverts frozen-vs-zero at
/// 40 GW where the observed trace already exports).
#[test]
fn bracket_orderings_at_the_high_curtailment_point() {
    let (header, rows) = table(pinned_sweep_csv());
    let value = |name: &str| column(&header, &rows, name, 60.0);

    // Curtailment: frozen ≥ zero ≥ export, strictly at 60 GW.
    let (frozen, zero, export) = (
        value("curtailment_twh"),
        value("curtailment_twh_imports_zero"),
        value("curtailment_twh_imports_export"),
    );
    assert!(
        frozen > zero && zero > export,
        "60 GW curtailment bracket broken: frozen {frozen} / zero {zero} / export {export}"
    );

    // Delivered capture: the INVERTED (true) ordering — prices are
    // pinned by the £0-floor, so more export relief = more delivered
    // £0 energy = lower delivered capture (module docs).
    let (frozen, zero, export) = (
        value("wind_capture_ratio_delivered"),
        value("wind_capture_ratio_delivered_imports_zero"),
        value("wind_capture_ratio_delivered_imports_export"),
    );
    assert!(
        frozen > zero && zero > export,
        "60 GW delivered-capture ordering broken (expected frozen ≥ zero ≥ export under \
         the £0-floor mechanism): frozen {frozen} / zero {zero} / export {export}"
    );

    // Potential capture: identical across conventions (the conventions
    // never move price formation in this data — zero vs export is
    // structural, frozen matches at these points).
    let (frozen, zero, export) = (
        value("wind_capture_ratio"),
        value("wind_capture_ratio_imports_zero"),
        value("wind_capture_ratio_imports_export"),
    );
    assert!(
        (frozen - zero).abs() <= RATIO_TOL && (zero - export).abs() <= RATIO_TOL,
        "potential capture moved across conventions — an import convention has started \
         changing price formation: frozen {frozen} / zero {zero} / export {export}"
    );
}

/// The export capability actually used is recorded in the CSV
/// assumptions: the reference scenario's own links, 9.8 GW nameplate
/// ex-Greenlink × 0.95 = 9.31 GW (no CLI parameter, no silent
/// default). Recorded at 3 dp (Package B review float-noise note; the
/// f64-exact value is used in the computation and pinned by the
/// bracket-value tests, which would move if it changed).
#[test]
fn export_capacity_is_recorded_and_scenario_derived() {
    let csv = pinned_sweep_csv();
    assert!(
        csv.contains("export_capacity = 9.310 GW"),
        "CSV assumptions do not record the 9.31 GW scenario-derived export capability"
    );
    assert!(
        csv.contains("scenario links"),
        "CSV assumptions do not record the export-capability source"
    );
}

/// Package B review condition 1: the interpretation guard must ship in
/// the artefact itself — the conventions move curtailment, not price
/// formation, and the delivered-capture spread is a WIDTH, not a
/// correction direction.
#[test]
fn interpretation_guard_ships_in_the_csv_assumptions() {
    let csv = pinned_sweep_csv();
    for needle in [
        "interpretation guard",
        "NOT price formation",
        "convention WIDTH",
        "not a correction direction",
        "missing export-price channel",
    ] {
        assert!(
            csv.contains(needle),
            "CSV assumption 6 (interpretation guard) lacks {needle:?}"
        );
    }
}
