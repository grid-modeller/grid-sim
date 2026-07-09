//! Pinned regression for the B6 two-zone validation scenario
//! (scenarios/gb-2024-2zone.toml): per-zone dispatch digests, the
//! links digest, the schema-v6 B6 capability/binding output columns,
//! and the ruling's convention/quote-duty assumption lines on the
//! artefact (docs/notes/b6-two-zone-data-review.md §6; the
//! 5-zone precedent is regression_5zone.rs).
//!
//! Requires the locally built 2024 + cf-gb2 + b6 data packs; fails
//! loudly with build instructions if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Per-zone dispatch digests, measured on the first run (2026-07-04).
/// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7):
/// the pre-fix walk silently cap-truncated boundary-sliver stalls in
/// 2,795/17,568 periods of this run (net −0.618 TWh southward B6 flow
/// withheld). Old digests: SCO 23d9eac57c447df974bd5ec33686fc4d07
/// 1add2e3a40cb04fd52a4f9a5d55e46, RGB 84135d259e8634f2731f5abca277
/// 665e1ec3691bc2b242e2a0297fddd271f5d9.
const PINNED_ZONE_DIGESTS: [(&str, &str); 2] = [
    (
        "SCO",
        "20b7f763bd10b890cf722ba795e3aeda8e81aee3f4af03b12c05efb259f40199",
    ),
    (
        "RGB",
        "0a260756eb527600a73aad5ea325f9af8ae65a65f95f19d67760a3ca028ab1cc",
    ),
];

/// The link-flow digest (covers the B6 flow, capability and binding
/// columns), measured alongside. Re-pinned 2026-07-06 (R7 stall fix);
/// old: 905efb55ab8e4fb2b27024772a1b8bce267c281c52249cb9383235cdfd7a8ea3.
const PINNED_LINKS_DIGEST: &str =
    "46781178e169bcbc600c75727fa3a9726a17bd7bb9ccaa1271cd10a350a8e166";

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn require_packs() {
    for (rel, hint) in [
        (
            "data/packs/2024/processed/demand_2024.parquet",
            "scripts/fetch-2024",
        ),
        (
            "data/packs/cf-gb2/sco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb2zone.py",
        ),
        (
            "data/packs/b6/processed/b6_da_flows_limits.parquet",
            "scripts/fetch-b6; verify data/packs/b6.sha256",
        ),
    ] {
        let path = repo_root().join(rel);
        assert!(
            path.exists(),
            "data pack file missing: {} — build it first: {hint}",
            path.display()
        );
    }
}

/// Run the scenario once per test process; return (summary, links.csv).
fn outputs() -> &'static (String, String) {
    static OUT: OnceLock<(String, String)> = OnceLock::new();
    OUT.get_or_init(|| {
        require_packs();
        let out_dir = std::env::temp_dir()
            .join("grid-cli-b6-tests")
            .join("regression-2zone");
        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
            .args([
                "run",
                "--scenario",
                "scenarios/gb-2024-2zone.toml",
                "--out",
                out_dir.to_str().unwrap(),
            ])
            .current_dir(repo_root())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "2-zone run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            std::fs::read_to_string(out_dir.join("summary.toml")).unwrap(),
            std::fs::read_to_string(out_dir.join("links.csv")).unwrap(),
        )
    })
}

#[test]
fn two_zone_dispatch_digests_are_pinned() {
    let (summary, _) = outputs();
    let digest_of = |zone: &str| -> String {
        let header = format!("[results.zones.\"{zone}\"]");
        let block = summary
            .split(&header)
            .nth(1)
            .unwrap_or_else(|| panic!("summary has no {header}"));
        block
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == "result_digest_sha256").then(|| v.trim().trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| panic!("no digest under {header}"))
    };
    for (zone, pinned) in PINNED_ZONE_DIGESTS {
        assert_eq!(
            digest_of(zone),
            pinned,
            "zone {zone}: dispatch digest moved — a deliberate engine/pack/scenario change \
             requires a knowing re-pin with the record"
        );
    }
    let links_digest = summary
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k.trim() == "links_digest_sha256").then(|| v.trim().trim_matches('"').to_owned())
        })
        .expect("summary has no links digest");
    assert_eq!(links_digest, PINNED_LINKS_DIGEST, "link-flow digest moved");
}

/// The B6 flow, capability and binding columns ship in the links
/// artefact (work order deliverable 6), and the summary carries the
/// gate-(iii)-convention binding statistics.
#[test]
fn b6_capability_and_binding_columns_ship_in_the_artefact() {
    let (summary, links_csv) = outputs();
    let header = links_csv
        .lines()
        .find(|l| l.starts_with("utc_start"))
        .expect("links.csv has no data header");
    for column in ["B6_home_gw", "B6_away_gw", "B6_fwd_cap_gw", "B6_binding"] {
        assert!(header.contains(column), "links.csv lacks column {column}");
    }
    for key in [
        "forward_capability_observed_periods",
        "forward_binding_periods",
        "forward_binding_share_of_capability_observed",
    ] {
        assert!(summary.contains(key), "summary lacks {key}");
    }
    // Engine-review condition 5: the artefact's binding share is
    // labelled to its CAPABILITY-OBSERVED denominator and explicitly
    // disclaims the gate-(iii) DA-flow-mask statistic — the two shares
    // (~0.25096 vs ~0.25019 post-R7-fix; ~0.23301 vs ~0.23229 pre-fix)
    // must never be quoted interchangeably.
    assert!(
        summary.contains("NOT the gate-(iii) DA-flow-mask")
            || summary.contains("NOT the gate-(iii) DA-flow-mask statistic"),
        "summary must disclaim the gate-(iii) denominator on the capability binding share"
    );
    // The old ambiguous key names must be gone (they conflated the two
    // denominators).
    assert!(
        !summary.contains("forward_binding_share_of_observed ")
            && !summary.contains("forward_observed_periods "),
        "the ambiguous pre-condition-5 key names must not reappear"
    );

    // DIAGNOSTIC — not a quotable "boundary effect". The
    // capability-observed binding share is a grid-cli summary statistic
    // outside the pinned dispatch/link digests, so it can drift
    // silently; pin its EXACT value (and the two integer counts it
    // derives from) so a move is a knowing re-pin. It is NOT a headline:
    // it is one of two easily-conflated binding shares (this one has the
    // capability-observed denominator 17,158; the gate-(iii) DA-flow-mask
    // statistic is ~0.23229 on 17,211 rows — never quoted
    // interchangeably) and the B6 requirement itself is a rule-based
    // upper-biased lower bound, not a clean boundary measurement. Values
    // first measured 2026-07-04. Re-pinned 2026-07-06 (R7 stall fix,
    // docs/08: released flows push more periods to the 99% line): was
    // 3,998 binding periods / share 0.23301084042429188 on the same
    // 17,158 denominator.
    let summary_f64 = |key: &str| -> f64 {
        summary
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == key)
                    .then(|| v.split('#').next().unwrap().trim().parse::<f64>().unwrap())
            })
            .unwrap_or_else(|| panic!("summary has no numeric key {key}"))
    };
    assert_eq!(
        summary_f64("forward_capability_observed_periods"),
        17_158.0,
        "DIAGNOSTIC capability-observed denominator moved"
    );
    assert_eq!(
        summary_f64("forward_binding_periods"),
        4_306.0,
        "DIAGNOSTIC forward-binding period count moved"
    );
    assert!(
        (summary_f64("forward_binding_share_of_capability_observed") - 0.250_961_650_542_021_24)
            .abs()
            < 1e-12,
        "DIAGNOSTIC capability-observed binding share moved (pinned 0.25096165…; NOT a \
         quotable boundary effect — see the §3 correction in the b6 run report)"
    );
}

/// The ruling's conventions travel on the artefact itself: sentinel
/// handling, masked-fill, the per-direction capabilities, and the B6
/// quote duty verbatim (ruling (c)).
#[test]
fn ruling_convention_assumption_lines_ship_in_links_csv() {
    let (_, links_csv) = outputs();
    for needle in [
        "link B6 capability conventions",
        "no-constraint sentinels replaced by the pinned upper bound 6.7 GW",
        "MASKED out of validation-gate arithmetic",
        "pinned central fill 4.1 GW",
        "reverse (RGB -> SCO) capability = 3.5 GW",
        "B6 QUOTE DUTY",
        "LOWER BOUND on the Scottish constraint phenomenon",
        "B4/B5 boundaries are structurally invisible",
        "GBP 90.5m",
        "GBP 525.8m",
        "CONTEXT ONLY, never a tuning target",
        // Engine-review conditions 7 and §1d.
        "B6-ATTRIBUTABLE SUBTRACTION RULE",
        "rest-of-GB zone moves the OPPOSITE way",
        "rule-based dispatch, upper-bias",
        "LP/Stage-7 is the",
        "store-placement conditioning",
    ] {
        assert!(
            links_csv.contains(needle),
            "links.csv assumption lines lack {needle:?}"
        );
    }
}
