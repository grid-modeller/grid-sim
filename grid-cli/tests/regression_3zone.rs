//! Pinned regression for the three-zone Scottish-group validation
//! scenario (scenarios/gb-2024-3zone.toml): per-zone dispatch digests,
//! the links digest, and the schema-v6 B4 + B6 capability/binding output
//! columns. The two-zone precedent is regression_2zone.rs; the five-zone
//! precedent is regression_5zone.rs.
//!
//! Requires the locally built 2024 + cf-gb2 + cf-gb3 + b4 data packs;
//! fails loudly with build instructions if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// Per-zone dispatch digests, measured on the first run (2026-07-04).
/// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7 —
/// the pre-fix walk silently cap-truncated boundary-sliver stalls).
/// Old digests: NSCO 3149233ec787b0d50c6d3532af6869a1769fbc09e23b80
/// 10035d72648bc52378, SSCO 3939b2fc3215d1c556a09e674bcfae964d975da1
/// 4c6832323ef4e979de397fe8, RGB 5960cc9bc6fb10b379d416bd73f66a62f9b
/// 9857ba73bc84e4aef1cf96479789b.
const PINNED_ZONE_DIGESTS: [(&str, &str); 3] = [
    (
        "NSCO",
        "0b553db9195a0c4f9b280c05dd13d1c095b1921d8ca49674ccbf4061e87fb717",
    ),
    (
        "SSCO",
        "124c3e01bbde98f17eb2907a28134b6b0117bcff24e4a9834b40337a4c11276d",
    ),
    (
        "RGB",
        "781d980dde848abc153c8cd7d33f6a7ce1f70e56bf382d7cafc6c7f40c5fd113",
    ),
];

/// The link-flow digest (covers both B4 and B6 flow, capability and
/// binding columns), measured alongside. Re-pinned 2026-07-06 (R7
/// stall fix); old: 085ac5d146f60eb8906d9926f1e804014094e73a06d0a683
/// 059943308356bae9.
const PINNED_LINKS_DIGEST: &str =
    "bfd6d3271b77ae669024193521fd5160d10fd53116221fe0e000ff6fb8c94a5f";

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
            "data/packs/cf-gb2/nsco_onshore_cf_2024.parquet",
            "scripts/era5-cf/derive_cf_gb3zone.py; verify data/packs/cf-gb3-1985-2024.sha256",
        ),
        (
            "data/packs/b6/processed/b4_da_flows_limits.parquet",
            "scripts/fetch-b6 (build.py --three-zone); verify data/packs/b4.sha256",
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
            .join("grid-cli-b4-tests")
            .join("regression-3zone");
        if out_dir.exists() {
            std::fs::remove_dir_all(&out_dir).unwrap();
        }
        let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
            .args([
                "run",
                "--scenario",
                "scenarios/gb-2024-3zone.toml",
                "--out",
                out_dir.to_str().unwrap(),
            ])
            .current_dir(repo_root())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "3-zone run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            std::fs::read_to_string(out_dir.join("summary.toml")).unwrap(),
            std::fs::read_to_string(out_dir.join("links.csv")).unwrap(),
        )
    })
}

#[test]
fn three_zone_dispatch_digests_are_pinned() {
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

/// Both B4 and B6 flow/capability/binding columns ship generically in the
/// links artefact, and the summary carries per-link binding statistics.
#[test]
fn b4_and_b6_capability_columns_ship_in_the_artefact() {
    let (summary, links_csv) = outputs();
    let header = links_csv
        .lines()
        .find(|l| l.starts_with("utc_start"))
        .expect("links.csv has no data header");
    for column in [
        "B4_home_gw",
        "B4_away_gw",
        "B4_fwd_cap_gw",
        "B4_binding",
        "B6_home_gw",
        "B6_away_gw",
        "B6_fwd_cap_gw",
        "B6_binding",
    ] {
        assert!(header.contains(column), "links.csv lacks column {column}");
    }
    for link in ["B4", "B6"] {
        let block_header = format!("[results.link_flows.\"{link}\"]");
        assert!(
            summary.contains(&block_header),
            "summary lacks the {link} link-flow block"
        );
    }
    // The B4 capability-observed binding count (DIAGNOSTIC — the model
    // barely binds B4 vs the observed 35.8%; see acceptance_b4_3zone.rs
    // finding 2). Pin the integer count so a drift is a knowing re-pin.
    let summary_f64 = |key: &str, after: &str| -> f64 {
        let block = summary.split(after).nth(1).unwrap();
        block
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == key)
                    .then(|| v.split('#').next().unwrap().trim().parse::<f64>().unwrap())
            })
            .unwrap_or_else(|| panic!("summary block after {after} has no numeric key {key}"))
    };
    // Re-pinned 2026-07-06 (R7 stall fix): B4 binding count was 337
    // pre-fix; the B6 count (577) did not move.
    assert_eq!(
        summary_f64("forward_binding_periods", "[results.link_flows.\"B4\"]"),
        347.0,
        "DIAGNOSTIC B4 forward-binding period count moved (model B4 barely binds — the \
         flow-convention pre-emption finding)"
    );
    assert_eq!(
        summary_f64("forward_binding_periods", "[results.link_flows.\"B6\"]"),
        577.0,
        "DIAGNOSTIC B6 forward-binding period count moved"
    );
}
