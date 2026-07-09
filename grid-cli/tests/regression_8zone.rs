//! Pinned per-zone dispatch digests, links digest and scenario sha256
//! of the D13 COMPOSED scenario (`scenarios/gb-2024-8zone.toml`) — the
//! rule-8(v) pins of the composed boundary-trade package
//! (`docs/notes/d13-composed-boundary-trade.md`;
//! grid-adequacy/tests/acceptance_d13_composed.rs is the companion
//! acceptance file and carries the measured-red gate record).
//!
//! These are CHARACTERISATION pins of the measured composed-anchor
//! dispatch (first measured 2026-07-05), NOT a validated-anchor record:
//! the pre-registered 8(i)/8(ii)-B4 gates measured RED (see the
//! acceptance file's banner) and the verdict is withheld pending
//! reviewer adjudication. The pins exist so the composed dispatch
//! cannot silently move underneath that adjudication. All COMMITTED
//! digests (779d7444…, the 2/3/5-zone families) live in their own
//! untouched regression files.
//!
//! Requires the locally built 2024 + cf-gb2/cf-gb3 + b4/b6 +
//! entsoe-2024 + cf-eu data packs; fails loudly if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

/// The composed scenario file's sha256 (the composition is data — a
/// file change is a re-adjudication event, not a drift).
/// Re-pinned 2026-07-06 for the schema v7 → v8 version-line migration
/// (D16, docs/03 migration note — the ONLY byte change is the
/// `schema_version` line; the per-zone result digests below are
/// re-verified unmoved). Old sha:
///   23d51777a935cfc92c6863520759e4be460cf1ae241cd4c24d801f25986981f9
const PINNED_SCENARIO_SHA256: &str =
    "db7c91db8cb7db3fe86eb57e78c7c7b120a8ec5cda36f5d41347c65385bcfdac";

/// Per-zone dispatch digests (rule-based leg, measured 2026-07-05).
/// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7 —
/// the pre-fix walk silently cap-truncated boundary-sliver stalls;
/// the composed family was the most exposed: the package-1 anchor
/// diagnostic found the stall signature on 9,473/9,473 GB-curtailment
/// periods). Old digests:
///   NSCO    d14dbb9a05d47553396f5bbf144bad6ad317975afca9051d75c895a803eb803f
///   SSCO    b270c75f32ef1702e30d30a465405d89f1e37ec35e9b71001f748ec46d6a45cf
///   RGB     75ac825fb01d1d98123f7531dfc3207339f117e2613251a2ccc69e4842089aa2
///   FR      df369fe52692de8af3aae3c8bb4075936fb05f749f8024b621a30c31f95d1d2f
///   CONT-NW 105adc7a5f2ea2da7ae4ac81bd88e9f872d872e01ebe962950ca8dcde31a5258
///   NO2     9ce180133f15554adfe4ab5ab6b7b16dc8c8b0e7e186abb9447dede60a7ae247
///   DK1     8c2415f400f0ec41323b030fe866ea1bdfc405781da10a509bba4bccbf8e494b
///   IE-SEM  29eb5c6e7d298fa53610ad84a4fe6818b2f6b0eeddd054cbcb9379b38cd4a097
const PINNED_ZONE_DIGESTS: [(&str, &str); 8] = [
    (
        "NSCO",
        "5b6f066af10e57be3a3087c0b48951a78ace54acdeb06e84acf9435bb406a2d9",
    ),
    (
        "SSCO",
        "55fa43b62806968556c5937070001181d5a8aa4923f6f38209fc5a0c16a758ae",
    ),
    (
        "RGB",
        "871bc1caf654e461950831dd32ae4bf4c5cc16c34779516e7de56553774180fe",
    ),
    (
        "FR",
        "59564e84583d98a9c92401f5cdb643b63980767c57cec6b1e0f64f19a4259ff8",
    ),
    (
        "CONT-NW",
        "ec60921910bee2acc2b9024d9686917f93cc9e1af32546d1ab56b3c37c571488",
    ),
    (
        "NO2",
        "0f55804f7d1f888372e94c80115b277b83b82faaf1ee953b6fc3495dd5de85a7",
    ),
    (
        "DK1",
        "2cfff5a7a33a5cfa7969cf00cbca73fa7d5741ad958e443a9c234404a1af2e13",
    ),
    (
        "IE-SEM",
        "9ea88862fb70d8658bd497327c2313635ded470c918e30192312d37593042a07",
    ),
];

/// The link-flow digest (12 links: B4, B6, then the ten external links
/// in the committed 5-zone order). Re-pinned 2026-07-06 (R7 stall
/// fix); old: a7c5f5f67b02b48c346da40e1efaebb06a3d69ade32bebb7b0f2224
/// 9331e873b.
const PINNED_LINKS_DIGEST: &str =
    "e6f0ddbcd6bc1f03b296d59c89ac04a48c00618882212064fbeafbfd6764dbb2";

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

#[test]
fn composed_8zone_scenario_dispatch_digests_are_pinned() {
    for probe in [
        "data/packs/entsoe-2024/processed/load_fr_2024.parquet",
        "data/packs/cf-gb2/nsco_onshore_cf_2024.parquet",
        "data/packs/b6/processed/b4_da_flows_limits.parquet",
    ] {
        assert!(
            repo_root().join(probe).exists(),
            "data pack file missing ({probe}) — build the packs first (scripts/fetch-2024, \
             scripts/fetch-entsoe, scripts/era5-cf, scripts/fetch-b6)"
        );
    }

    let out_dir = std::env::temp_dir()
        .join("grid-cli-d13-tests")
        .join("regression-8zone");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "run",
            "--scenario",
            "scenarios/gb-2024-8zone.toml",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "8-zone run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary = std::fs::read_to_string(out_dir.join("summary.toml")).unwrap();

    let field = |key: &str, from: &str| -> String {
        from.lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                (k.trim() == key).then(|| v.trim().trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| panic!("summary has no {key}"))
    };
    assert_eq!(
        field("scenario_sha256", &summary),
        PINNED_SCENARIO_SHA256,
        "the composed scenario file changed — a re-adjudication event, not a drift"
    );
    for (zone, pinned) in PINNED_ZONE_DIGESTS {
        let header = format!("[results.zones.\"{zone}\"]");
        let block = summary
            .split(&header)
            .nth(1)
            .unwrap_or_else(|| panic!("summary has no {header}"));
        assert_eq!(
            field("result_digest_sha256", block),
            pinned,
            "zone {zone}: composed dispatch digest moved — re-pin only with the D13 record \
             (and re-run acceptance_d13_composed)"
        );
    }
    assert_eq!(
        field("links_digest_sha256", &summary),
        PINNED_LINKS_DIGEST,
        "composed link-flow digest moved"
    );
}
