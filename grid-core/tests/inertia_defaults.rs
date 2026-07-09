//! Characterisation tests: the code's built-in inertia defaults
//! (`grid_core::inertia`, ADR-9) must match the committed evidence file
//! `data/reference/inertia-constants.toml` exactly — one cited source,
//! no drift. If the evidence file is revised, this test forces the code
//! mapping (and its citations) to move with it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use grid_core::inertia::{DEFAULT_POWER_FACTOR, storage_kind_default, technology_default};
use grid_core::scenario::{StorageKind, TechId};

fn reference() -> toml::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("data/reference/inertia-constants.toml");
    toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Fleet technology ids ↔ reference-file `[technologies.*]` keys.
/// `pumped_storage` is a storage kind in this model, checked separately.
const TECHNOLOGY_KEYS: [(&str, &str); 10] = [
    ("ccgt", "ccgt"),
    ("ocgt", "ocgt"),
    ("nuclear", "nuclear"),
    ("coal", "coal"),
    ("biomass", "biomass"),
    ("hydro", "hydro"),
    ("onshore_wind", "onshore_wind"),
    ("offshore_wind", "offshore_wind"),
    ("solar", "solar"),
    ("interconnector", "interconnector"),
];

#[test]
fn technology_defaults_match_the_committed_reference_file() {
    let reference = reference();
    for (tech, key) in TECHNOLOGY_KEYS {
        let entry = &reference["technologies"][key];
        let expected_h = entry["h_s"].as_float().unwrap();
        let expected_sync = entry["synchronous"].as_bool().unwrap();
        let default = technology_default(&TechId::new(tech));
        assert_eq!(
            default.synchronous, expected_sync,
            "technology {tech}: synchronous flag drifted from the reference file"
        );
        let h = default.h.map_or(0.0, |h| h.as_seconds());
        assert_eq!(
            h, expected_h,
            "technology {tech}: H default drifted from the reference file"
        );
        // Non-synchronous defaults carry NO inertia constant (docs/03:
        // `None` if non-sync — the reference file's 0.0 is spelled as
        // absence in the schema).
        assert_eq!(default.h.is_some(), expected_sync);
    }
}

#[test]
fn unknown_technologies_default_to_non_synchronous_with_no_h() {
    let default = technology_default(&TechId::new("fusion_prototype"));
    assert!(!default.synchronous);
    assert!(default.h.is_none());
}

#[test]
fn storage_kind_defaults_match_the_committed_reference_file() {
    let reference = reference();
    // Pumped storage: synchronous with real H in BOTH modes (the
    // reference file's pumped_storage entry).
    let ps = storage_kind_default(StorageKind::PumpedHydro);
    assert!(ps.synchronous);
    assert_eq!(
        ps.h.unwrap().as_seconds(),
        reference["technologies"]["pumped_storage"]["h_s"]
            .as_float()
            .unwrap()
    );
    // Battery: inverter-coupled, zero inertia (reference battery entry).
    let battery = storage_kind_default(StorageKind::Battery);
    assert!(!battery.synchronous);
    assert!(battery.h.is_none());
    assert!(
        !reference["technologies"]["battery"]["synchronous"]
            .as_bool()
            .unwrap()
    );
    // Hydrogen and DSR: non-synchronous by v1 modelling choice
    // (documented in grid_core::inertia — reconversion technology
    // unspecified; the RS zero-inertia finding depends on this default
    // being visible, not hidden).
    for kind in [StorageKind::Hydrogen, StorageKind::Dsr] {
        let default = storage_kind_default(kind);
        assert!(!default.synchronous, "{kind}: v1 default is non-sync");
        assert!(default.h.is_none());
    }
}

#[test]
fn default_power_factor_is_the_documented_convention() {
    // MVA = GW / 0.9 (inertia-constants.toml CONVENTIONS: pf typically
    // 0.85–0.95 for large sets; 0.9 adopted as the Stage 6 convention).
    assert_eq!(DEFAULT_POWER_FACTOR.value(), 0.9);
}
