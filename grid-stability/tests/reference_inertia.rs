//! Acceptance tests for the bottom-up inertia-from-generation path
//! (Stage 6 NESO enrichment, Task 5): `inertia_from_generation` sums
//! `H(fuel) × (MW/PF)` per period from a generation-by-fuel table,
//! independent of the scenario-dispatch path in `inertia.rs`.

// Tests may unwrap/panic freely (workspace lints deny these in library code).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use grid_core::GridError;
use grid_core::units::Inertia;
use grid_stability::{correlate, inertia_from_generation};

fn gv(v: &[f64]) -> Vec<Inertia> {
    v.iter()
        .map(|x| Inertia::gigavolt_ampere_seconds(*x))
        .collect()
}

#[test]
fn ccgt_only_period_matches_h_times_mva() {
    // 1000 MW CCGT, H=5.0s, PF=0.9 -> 5.0 * (1.0/0.9) GVA.s = 5.5556 GVA.s
    let fuels = vec![("ccgt".to_string(), vec![1000.0])];
    let e = inertia_from_generation(&fuels).unwrap();
    assert!((e[0].as_gigavolt_ampere_seconds() - 5.0 * (1.0 / 0.9)).abs() < 1e-9);
}

#[test]
fn wind_contributes_zero() {
    let fuels = vec![("wind".to_string(), vec![5000.0])];
    let e = inertia_from_generation(&fuels).unwrap();
    assert_eq!(e[0].as_gigavolt_ampere_seconds(), 0.0);
}

#[test]
fn unknown_fuel_contributes_zero() {
    let fuels = vec![("mystery_fuel".to_string(), vec![1234.0])];
    let e = inertia_from_generation(&fuels).unwrap();
    assert_eq!(e[0].as_gigavolt_ampere_seconds(), 0.0);
}

#[test]
fn multiple_fuels_sum_per_period() {
    // ccgt (H=5.0) + nuclear (H=4.5), two periods.
    let fuels = vec![
        ("ccgt".to_string(), vec![1000.0, 2000.0]),
        ("nuclear".to_string(), vec![1000.0, 0.0]),
    ];
    let e = inertia_from_generation(&fuels).unwrap();
    assert_eq!(e.len(), 2);
    let expected_p0 = 5.0 * (1.0 / 0.9) + 4.5 * (1.0 / 0.9);
    let expected_p1 = 5.0 * (2.0 / 0.9);
    assert!((e[0].as_gigavolt_ampere_seconds() - expected_p0).abs() < 1e-9);
    assert!((e[1].as_gigavolt_ampere_seconds() - expected_p1).abs() < 1e-9);
}

#[test]
fn ragged_columns_return_invalid_stability_input() {
    let fuels = vec![
        ("ccgt".to_string(), vec![1000.0, 2000.0]),
        ("nuclear".to_string(), vec![1000.0]),
    ];
    let result = inertia_from_generation(&fuels);
    assert!(matches!(
        result,
        Err(GridError::InvalidStabilityInput { .. })
    ));
}

#[test]
fn empty_fuel_list_returns_empty_series() {
    let fuels: Vec<(String, Vec<f64>)> = vec![];
    let e = inertia_from_generation(&fuels).unwrap();
    assert!(e.is_empty());
}

#[test]
fn affine_relationship_is_recovered_exactly() {
    let ours = gv(&[10.0, 20.0, 30.0, 40.0]);
    let neso = gv(&[62.0, 76.0, 90.0, 104.0]); // = 1.4*ours + 48
    let f = correlate(&ours, &neso).unwrap();
    assert!((f.pearson_r - 1.0).abs() < 1e-12);
    assert!((f.slope - 1.4).abs() < 1e-9);
    assert!((f.intercept - 48.0).abs() < 1e-9);
}

#[test]
fn correlate_reports_n_and_median_ratio() {
    let ours = gv(&[10.0, 20.0, 30.0, 40.0]);
    let neso = gv(&[62.0, 76.0, 90.0, 104.0]);
    let f = correlate(&ours, &neso).unwrap();
    assert_eq!(f.n, 4);
    // ratios: 6.2, 3.8, 3.0, 2.6 -> sorted 2.6,3.0,3.8,6.2 -> median (3.0+3.8)/2 = 3.4
    assert!((f.median_ratio - 3.4).abs() < 1e-9);
}

#[test]
fn correlate_rejects_length_mismatch() {
    let ours = gv(&[10.0, 20.0, 30.0]);
    let neso = gv(&[62.0, 76.0]);
    let result = correlate(&ours, &neso);
    assert!(matches!(
        result,
        Err(GridError::InvalidStabilityInput { .. })
    ));
}

#[test]
fn correlate_rejects_fewer_than_two_points() {
    let ours = gv(&[10.0]);
    let neso = gv(&[62.0]);
    let result = correlate(&ours, &neso);
    assert!(matches!(
        result,
        Err(GridError::InvalidStabilityInput { .. })
    ));
}

#[test]
fn correlate_rejects_zero_variance_in_ours() {
    let ours = gv(&[10.0, 10.0, 10.0]);
    let neso = gv(&[62.0, 76.0, 90.0]);
    let result = correlate(&ours, &neso);
    assert!(matches!(
        result,
        Err(GridError::InvalidStabilityInput { .. })
    ));
}

#[test]
fn correlate_errors_on_zero_variance_neso() {
    let ours = gv(&[10.0, 20.0, 30.0]);
    let neso = gv(&[62.0, 62.0, 62.0]);
    let result = correlate(&ours, &neso);
    assert!(matches!(
        result,
        Err(GridError::InvalidStabilityInput { .. })
    ));
}

#[test]
fn correlate_skips_zero_denominator_pairs_in_median_ratio() {
    // ours has a zero entry; that pair is excluded from the ratio median
    // (division by zero is undefined), but the fit itself still uses all points.
    let ours = gv(&[0.0, 10.0, 20.0, 30.0]);
    let neso = gv(&[48.0, 62.0, 76.0, 90.0]); // = 1.4*ours + 48
    let f = correlate(&ours, &neso).unwrap();
    assert_eq!(f.n, 4);
    // valid ratios: 62/10=6.2, 76/20=3.8, 90/30=3.0 -> sorted 3.0,3.8,6.2 -> median 3.8
    assert!((f.median_ratio - 3.8).abs() < 1e-9);
}
