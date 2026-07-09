//! Bottom-up inertia from an actual-generation table (Stage 6 NESO
//! enrichment, Task 5): an independent estimate of system inertia,
//! computed directly from per-fuel generation MW rather than from a
//! scenario dispatch result (`inertia.rs`'s `system_inertia`). This is
//! the method-characterisation counterpart used to compare against the
//! published NESO System Inertia outturn series.
//!
//! Mirrors the aggregation convention documented in `inertia.rs`: `E =
//! Σ H_i × (dispatched GW_i / PF)` over synchronous plant, with `PF`
//! the fleet power factor default
//! ([`grid_core::inertia::DEFAULT_POWER_FACTOR`]). Here "dispatched" is
//! the fuel's per-period generation MW column directly, and technology
//! H comes from `grid_core::inertia::technology_default` via the fuel
//! name → `TechId` mapping. Non-synchronous fuels (wind, solar,
//! interconnectors) and unrecognised fuel names both contribute 0 —
//! the same honest-default convention as `technology_default` itself.

use grid_core::GridError;
use grid_core::inertia::{DEFAULT_POWER_FACTOR, technology_default};
use grid_core::scenario::TechId;
use grid_core::units::{Inertia, Power};

/// Per-period system inertia computed bottom-up from a generation-by-fuel
/// table: `fuels` is `(fuel name, per-period generation in MW)` pairs.
/// Each fuel name is mapped to a `TechId` and its default inertia
/// constant via `grid_core::inertia::technology_default`; non-synchronous
/// or unrecognised fuels contribute 0 for every period.
///
/// Returns `GridError::InvalidStabilityInput` if the fuel columns are
/// ragged (not all the same length) — never panics.
pub fn inertia_from_generation(fuels: &[(String, Vec<f64>)]) -> Result<Vec<Inertia>, GridError> {
    let periods = fuels.first().map_or(0, |(_, series)| series.len());
    for (name, series) in fuels {
        if series.len() != periods {
            return Err(GridError::InvalidStabilityInput {
                reason: format!(
                    "fuel column {name:?} has {} periods, expected {periods} (all columns \
                     must be the same length)",
                    series.len()
                ),
            });
        }
    }

    let pf = DEFAULT_POWER_FACTOR;
    let mut totals = vec![Inertia::gigavolt_ampere_seconds(0.0); periods];
    for (name, series) in fuels {
        let tech = TechId::new(name.as_str());
        let Some(h) = technology_default(&tech).h else {
            continue;
        };
        for (period, &mw) in series.iter().enumerate() {
            totals[period] = totals[period] + h * Power::megawatts(mw).apparent(pf);
        }
    }
    Ok(totals)
}

/// Result of [`correlate`]: how closely a bottom-up inertia series tracks
/// a reference (NESO) series, expressed as a Pearson correlation, an
/// ordinary-least-squares affine fit (`neso = slope·ours + intercept`),
/// and a robust ratio summary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Number of paired observations the fit was computed over.
    pub n: usize,
    /// Pearson correlation coefficient of `ours` vs `neso`.
    pub pearson_r: f64,
    /// OLS slope of `neso` regressed on `ours`.
    pub slope: f64,
    /// OLS intercept of `neso` regressed on `ours`.
    pub intercept: f64,
    /// Median of `neso[i] / ours[i]` over pairs with a non-zero
    /// denominator (see [`correlate`] for why zero-denominator pairs are
    /// excluded rather than erroring).
    pub median_ratio: f64,
}

/// Pearson correlation and OLS affine fit of a reference (NESO) inertia
/// series against our bottom-up estimate, plus a median-ratio summary.
///
/// `neso ≈ slope·ours + intercept`; `pearson_r` measures how tight that
/// linear relationship is.
///
/// `median_ratio` is the median of `neso[i]/ours[i]`, skipping any pair
/// where `ours[i]` is exactly zero (the ratio is undefined there — a
/// zero bottom-up estimate means no synchronous generation was dispatched
/// that period, not that NESO's outturn should be treated as infinite).
/// This can never discard every pair: the zero-variance guard below
/// already rejects any input where every `ours[i]` is identical
/// (including all-zero), so at least one non-zero denominator survives.
///
/// Returns `GridError::InvalidStabilityInput` if `ours` and `neso` have
/// different lengths, if there are fewer than 2 points, or if `ours` or
/// `neso` has zero variance (an OLS slope against a constant, or a
/// correlation against a constant, is undefined).
pub fn correlate(ours: &[Inertia], neso: &[Inertia]) -> Result<Fit, GridError> {
    if ours.len() != neso.len() {
        return Err(GridError::InvalidStabilityInput {
            reason: format!(
                "ours has {} points, neso has {} (must be paired and equal length)",
                ours.len(),
                neso.len()
            ),
        });
    }
    let n = ours.len();
    if n < 2 {
        return Err(GridError::InvalidStabilityInput {
            reason: format!("correlate needs at least 2 points, got {n}"),
        });
    }

    let x: Vec<f64> = ours
        .iter()
        .map(|i| i.as_gigavolt_ampere_seconds())
        .collect();
    let y: Vec<f64> = neso
        .iter()
        .map(|i| i.as_gigavolt_ampere_seconds())
        .collect();

    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    cov /= n as f64;
    var_x /= n as f64;
    var_y /= n as f64;

    if var_x <= 0.0 {
        return Err(GridError::InvalidStabilityInput {
            reason: "ours has zero variance; cannot fit a slope against a constant series"
                .to_string(),
        });
    }
    if var_y <= 0.0 {
        return Err(GridError::InvalidStabilityInput {
            reason: "neso has zero variance; correlation is undefined against a constant series"
                .to_string(),
        });
    }

    let slope = cov / var_x;
    let intercept = mean_y - slope * mean_x;
    let pearson_r = cov / (var_x.sqrt() * var_y.sqrt());

    let mut ratios: Vec<f64> = x
        .iter()
        .zip(y.iter())
        .filter(|(ox, _)| **ox != 0.0)
        .map(|(ox, ny)| ny / ox)
        .collect();
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_ratio = if ratios.is_empty() {
        // Unreachable given the zero-variance guard above (it requires at
        // least two distinct `ours` values, so not all can be zero), but
        // handled explicitly rather than indexing into an empty slice.
        0.0
    } else if ratios.len().is_multiple_of(2) {
        let mid = ratios.len() / 2;
        (ratios[mid - 1] + ratios[mid]) / 2.0
    } else {
        ratios[ratios.len() / 2]
    };

    Ok(Fit {
        n,
        pearson_r,
        slope,
        intercept,
        median_ratio,
    })
}
