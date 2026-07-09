//! Pinned per-zone dispatch digests of the 5-zone Stage 5 acceptance
//! scenario — the EXPLICIT re-verification the D9 heating package
//! requires (d9-heating-overlay.md rule 2/rule 5 test 2): both live
//! reference scenarios were migrated to schema v5 in the same commit
//! (version line + removal of the engine-inert v1–v4
//! `[zones.demand.heating]` sketch block), and the dispatch must be
//! bit-identical across that edit — a measured check, not an
//! assumption.
//!
//! Measurement record: the digests below were measured on the
//! pre-migration v4 scenario (2026-07-03, current engine + pack) and
//! re-asserted here against the migrated v5 file. The single-zone
//! counterpart is `regression_2024.rs` (the pinned `779d7444…`), which
//! runs the migrated reference scenario directly. The Stage 5 A2
//! pinned match counts (grid-adequacy/tests/acceptance_stage5_2024.rs)
//! independently guard the same dispatch.
//!
//! Requires the locally built 2024 + entsoe-2024 + CF data packs;
//! fails loudly if absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

/// Per-zone dispatch digests measured pre-migration (v4 file,
/// 2026-07-03) — the v5 migration must not move any of them. The
/// migration-equivalence record (v4 == v5 bit-for-bit) was measured
/// and held on the pre-R7 engine.
///
/// Re-pinned 2026-07-06 for the R7 flow-walk stall fix (docs/08 R7 —
/// the pre-fix walk silently cap-truncated boundary-sliver stalls).
/// DK1 alone is unmoved by the fix (no stall released in its
/// dispatch). Old digests:
///   GB      c783b306737eb4854b951d023c106578a9bfc5d428a6588c8e73e85ed1b03e5a
///   FR      91191dc801a91de720e8bb4012ba6ebe190f547112311fdc9f4dba8500d7b66c
///   CONT-NW e5f376063d57b02522fcdb061f920f1ee297f2375f4daa9f6d7802a7e84b5720
///   NO2     fba1fb7c7edd6c96d8d8283c7ac16d82a01fe33ee0c065da49810f6595d820a8
///   DK1     00065d896478c5e28a8727fb3e1bca924e6a99e52ee17f98e6b6f710b5b680ff
///   IE-SEM  1956cd892945155437d7f2beb3d9b26951c04caa8257d85bccc6b43282dd4776
const PINNED_ZONE_DIGESTS: [(&str, &str); 6] = [
    (
        "GB",
        "849c498a047c10f4d43cc7b097296bffcc431a379701cc7d3ef841256b80523d",
    ),
    (
        "FR",
        "9878aff10c923c9d1936fda12f30e9c6332bb91bd17ea769d35375289e9a28b7",
    ),
    (
        "CONT-NW",
        "f3a3c6d0a9ee771507d4ca5091fa5a7b8c63fa317d358941bbe9cf5c6aa1515e",
    ),
    (
        "NO2",
        "8ecbf13b98fc3c0616964a84c1e9648b2d22cf0f7294d98d67703f7f170401ae",
    ),
    (
        "DK1",
        "00065d896478c5e28a8727fb3e1bca924e6a99e52ee17f98e6b6f710b5b680ff",
    ),
    (
        "IE-SEM",
        "bd62a18b0f934c60d335fc857d0358892ccb6fe40bddca47b42e119422082afa",
    ),
];

/// The link-flow digest, measured pre-migration alongside. Re-pinned
/// 2026-07-06 (R7 stall fix); old: 371aa2571c35aa40af68e58337afa698
/// 46d5ef36e18ad08ae7c7c23dc7b97cad.
const PINNED_LINKS_DIGEST: &str =
    "0d9e44355d53ce2ca9ff2723100a1b4b7f76478f466d2bf99d52d84f9f3405cf";

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

#[test]
fn migrated_5zone_scenario_dispatch_digests_are_unmoved() {
    let probe = repo_root().join("data/packs/entsoe-2024/processed/load_fr_2024.parquet");
    assert!(
        probe.exists(),
        "entsoe-2024 data pack is missing ({}) — build the packs first (scripts/fetch-2024, \
         scripts/fetch-entsoe, scripts/era5-cf)",
        probe.display()
    );

    let out_dir = std::env::temp_dir()
        .join("grid-cli-heating-tests")
        .join("regression-5zone");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_grid-cli"))
        .args([
            "run",
            "--scenario",
            "scenarios/gb-2024-5zone.toml",
            "--out",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "5-zone run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary = std::fs::read_to_string(out_dir.join("summary.toml")).unwrap();

    // Extract each zone's digest from its [results.zones."<id>"] block.
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
        let digest = digest_of(zone);
        assert_eq!(
            digest, pinned,
            "zone {zone}: dispatch digest moved across the v5 migration — the removed \
             heating sketch block was engine-inert, so any move means the engine or the \
             pack changed; re-pin only with the record"
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
