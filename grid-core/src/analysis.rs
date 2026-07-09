//! Residual-load analysis utilities and the timescale decomposition
//! (ADR-1: analysis utilities live in grid-core; docs/03 "Analysis
//! utilities"; docs/07 Module 2 and Module 3(c)).
//!
//! # Residual load (Module 2)
//!
//! `residual(t) = demand(t) − Σ must-take(t)` — what the dispatchable
//! fleet and storage actually have to do. Negative values are system
//! surplus. [`residual_load`] subtracts the must-take total computed in
//! the dispatch engine's rule-1 summation order (sum the series first,
//! then subtract from demand), so downstream consumers that replay the
//! residual through the engine reproduce its arithmetic exactly.
//! [`duration_curve`], [`ramps`] and [`ramp_stats`] are the Module 2
//! machinery.
//!
//! # Timescale decomposition (Module 3(c))
//!
//! The residual series is split into four bands — **diurnal, synoptic,
//! seasonal, inter-annual** — by successive differences of centred
//! moving averages at three widening windows (the classic band-pass-by-
//! smoothing construction, chosen for being simple to explain and
//! exactly summing):
//!
//! ```text
//! ℓ₁ = MA(r, diurnal window)      ℓ₂ = MA(r, synoptic window)
//! ℓ₃ = MA(r, seasonal window)
//!
//! diurnal      = r  − ℓ₁     (faster than the diurnal window)
//! synoptic     = ℓ₁ − ℓ₂     (diurnal .. synoptic window)
//! seasonal     = ℓ₂ − ℓ₃     (synoptic .. seasonal window)
//! inter-annual = ℓ₃          (slower than the seasonal window,
//!                             including the mean)
//! ```
//!
//! **The bands sum back to the original series identically by
//! construction** — the sum telescopes:
//! `(r − ℓ₁) + (ℓ₁ − ℓ₂) + (ℓ₂ − ℓ₃) + ℓ₃ = r` — up to f64
//! re-association only (a few ulps; the acceptance suite measures the
//! worst per-period error on the real 40-year record). This is the
//! docs/08 kill-criterion-2 "bands approximately sum" property, exact
//! here by design.
//!
//! Every level is a moving average **of the original series**, not a
//! cascade of re-smoothed levels: this makes each band attribution
//! depend only on the two windows bounding it, so perturbing one window
//! (the kill-criterion-2 stability test) moves exactly the two adjacent
//! bands and provably nothing else.
//!
//! Moving-average convention: a centred window of half-width `h`
//! periods (`h` = half the nominal window duration), clipped at the
//! series boundaries (edge averages use the available samples).
//! Implemented with prefix sums; the cumulative-sum rounding error is
//! ≲ 1e-6 GW on a 40-year, tens-of-GW series — negligible against the
//! GWh-scale storage-attribution tolerances downstream.

use crate::GridError;
use crate::units::{Duration, Power, UnitScalar};

/// Residual load: `demand(t) − Σ must_take(t)` per period. Negative
/// values are surplus. The must-take total is accumulated series-by-
/// series from zero, then subtracted from demand — the dispatch
/// engine's own rule-1 order, so `residual = −net` bit-for-bit.
///
/// Errors with [`GridError::InvalidAnalysisInput`] if any must-take
/// series has a different length than demand.
pub fn residual_load(demand: &[Power], must_take: &[&[Power]]) -> Result<Vec<Power>, GridError> {
    for (index, series) in must_take.iter().enumerate() {
        if series.len() != demand.len() {
            return Err(GridError::InvalidAnalysisInput {
                reason: format!(
                    "must-take series {index} has {} periods; demand has {}",
                    series.len(),
                    demand.len()
                ),
            });
        }
    }
    Ok(demand
        .iter()
        .enumerate()
        .map(|(t, &d)| {
            let mut total = Power::gigawatts(0.0);
            for series in must_take {
                total = total + series[t];
            }
            d - total
        })
        .collect())
}

/// Duration curve: the series sorted descending (the classic
/// exceedance view — index/length is the fraction of time the value is
/// exceeded). NaNs never occur in engine output; if present they sort
/// last.
#[must_use]
pub fn duration_curve(series: &[Power]) -> Vec<Power> {
    let mut sorted = series.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(core::cmp::Ordering::Equal));
    sorted
}

/// Period-to-period ramps: `series[t+1] − series[t]` (length
/// `len − 1`; empty for a series of fewer than two periods).
#[must_use]
pub fn ramps(series: &[Power]) -> Vec<Power> {
    series.windows(2).map(|pair| pair[1] - pair[0]).collect()
}

/// Summary ramp statistics of a series (per half-hour step).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RampStats {
    /// Largest upward ramp (most positive step).
    pub max_up: Power,
    /// Largest downward ramp (most negative step, reported negative).
    pub max_down: Power,
    /// Mean absolute ramp.
    pub mean_abs: Power,
}

/// Ramp statistics of a series; `None` for fewer than two periods.
#[must_use]
pub fn ramp_stats(series: &[Power]) -> Option<RampStats> {
    let deltas = ramps(series);
    if deltas.is_empty() {
        return None;
    }
    let mut max_up = deltas[0];
    let mut max_down = deltas[0];
    let mut abs_sum = 0.0f64;
    for &delta in &deltas {
        if delta > max_up {
            max_up = delta;
        }
        if delta < max_down {
            max_down = delta;
        }
        abs_sum += delta.as_gigawatts().abs();
    }
    Some(RampStats {
        max_up,
        max_down,
        mean_abs: Power::gigawatts(abs_sum / deltas.len() as f64),
    })
}

/// The three smoothing windows bounding the four bands. Windows are
/// nominal durations; each becomes a centred moving average of
/// half-width `round(window / 2)` periods (module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecompositionWindows {
    /// Diurnal cutoff (canonical: 24 h).
    pub diurnal: Duration,
    /// Synoptic cutoff (canonical: 14 d — weather-system passage).
    pub synoptic: Duration,
    /// Seasonal cutoff (canonical: 365 d — the annual cycle).
    pub seasonal: Duration,
}

impl DecompositionWindows {
    /// The canonical windows: 24 h / 14 d / 365 d.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            diurnal: Duration::hours(24.0),
            synoptic: Duration::hours(14.0 * 24.0),
            seasonal: Duration::hours(365.0 * 24.0),
        }
    }

    /// Half-widths in half-hourly periods, validated: every window at
    /// least one hour, finite, and strictly increasing.
    fn half_widths(&self) -> Result<[usize; 3], GridError> {
        let mut out = [0usize; 3];
        for (slot, (name, window)) in out.iter_mut().zip([
            ("diurnal", self.diurnal),
            ("synoptic", self.synoptic),
            ("seasonal", self.seasonal),
        ]) {
            let hours = window.as_hours();
            if !hours.is_finite() || hours < 1.0 {
                return Err(GridError::InvalidAnalysisInput {
                    reason: format!(
                        "{name} window is {hours} h; decomposition windows must be finite \
                         and at least one hour"
                    ),
                });
            }
            // Half-width in periods = half the nominal window: a 24 h
            // window averages ±12 h around each period.
            *slot = (hours).round() as usize;
        }
        if !(out[0] < out[1] && out[1] < out[2]) {
            return Err(GridError::InvalidAnalysisInput {
                reason: format!(
                    "decomposition windows must be strictly increasing: diurnal {} h, \
                     synoptic {} h, seasonal {} h",
                    self.diurnal.as_hours(),
                    self.synoptic.as_hours(),
                    self.seasonal.as_hours()
                ),
            });
        }
        Ok(out)
    }
}

/// The four bands plus the smoothing levels they difference (module
/// docs). `bands` sum back to the input series by construction.
#[derive(Debug, Clone, PartialEq)]
pub struct Decomposition {
    /// Variation faster than the diurnal window: `r − ℓ₁`.
    pub diurnal: Vec<Power>,
    /// Diurnal-to-synoptic variation: `ℓ₁ − ℓ₂`.
    pub synoptic: Vec<Power>,
    /// Synoptic-to-seasonal variation: `ℓ₂ − ℓ₃`.
    pub seasonal: Vec<Power>,
    /// Variation slower than the seasonal window, including the mean:
    /// `ℓ₃`.
    pub inter_annual: Vec<Power>,
    /// The smoothing levels `[ℓ₁, ℓ₂, ℓ₃]`: the series with variation
    /// faster than the diurnal / synoptic / seasonal window removed.
    pub levels: [Vec<Power>; 3],
}

/// Decompose a series into the four timescale bands (module docs:
/// differences of centred moving averages at the given windows; the
/// bands sum back to the series identically by construction).
///
/// Errors with [`GridError::InvalidAnalysisInput`] on an empty series
/// or invalid windows.
pub fn decompose(
    series: &[Power],
    windows: &DecompositionWindows,
) -> Result<Decomposition, GridError> {
    if series.is_empty() {
        return Err(GridError::InvalidAnalysisInput {
            reason: "cannot decompose an empty series".to_owned(),
        });
    }
    let [h1, h2, h3] = windows.half_widths()?;
    let l1 = moving_average(series, h1);
    let l2 = moving_average(series, h2);
    let l3 = moving_average(series, h3);

    let difference = |a: &[Power], b: &[Power]| -> Vec<Power> {
        a.iter().zip(b).map(|(&x, &y)| x - y).collect()
    };
    Ok(Decomposition {
        diurnal: difference(series, &l1),
        synoptic: difference(&l1, &l2),
        seasonal: difference(&l2, &l3),
        inter_annual: l3.clone(),
        levels: [l1, l2, l3],
    })
}

/// Centred moving average of half-width `h` periods, clipped at the
/// series boundaries (edge averages use the available samples).
/// Prefix-sum implementation, O(n).
fn moving_average(series: &[Power], half_width: usize) -> Vec<Power> {
    let n = series.len();
    let mut prefix = Vec::with_capacity(n + 1);
    prefix.push(0.0f64);
    for value in series {
        // Infallible: prefix always has at least one element.
        let last = prefix.last().copied().unwrap_or(0.0);
        prefix.push(last + value.raw());
    }
    (0..n)
        .map(|t| {
            let lo = t.saturating_sub(half_width);
            let hi = (t + half_width).min(n - 1);
            Power::from_raw((prefix[hi + 1] - prefix[lo]) / (hi + 1 - lo) as f64)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn gw(values: &[f64]) -> Vec<Power> {
        values.iter().map(|&v| Power::gigawatts(v)).collect()
    }

    #[test]
    fn residual_subtracts_must_take_from_demand() {
        let demand = gw(&[30.0, 40.0, 20.0]);
        let wind = gw(&[10.0, 35.0, 25.0]);
        let solar = gw(&[5.0, 10.0, 0.0]);
        let residual = residual_load(&demand, &[&wind, &solar]).unwrap();
        assert_eq!(residual, gw(&[15.0, -5.0, -5.0]));
        // No must-take: residual is demand.
        assert_eq!(residual_load(&demand, &[]).unwrap(), demand);
    }

    #[test]
    fn residual_rejects_misaligned_series() {
        let demand = gw(&[30.0, 40.0]);
        let short = gw(&[10.0]);
        let err = residual_load(&demand, &[&short]).unwrap_err();
        assert!(matches!(err, GridError::InvalidAnalysisInput { .. }));
    }

    #[test]
    fn duration_curve_sorts_descending() {
        let series = gw(&[5.0, -2.0, 30.0, 7.0]);
        assert_eq!(duration_curve(&series), gw(&[30.0, 7.0, 5.0, -2.0]));
    }

    #[test]
    fn ramps_and_stats_hand_check() {
        let series = gw(&[10.0, 14.0, 8.0, 8.0]);
        assert_eq!(ramps(&series), gw(&[4.0, -6.0, 0.0]));
        let stats = ramp_stats(&series).unwrap();
        assert_eq!(stats.max_up, Power::gigawatts(4.0));
        assert_eq!(stats.max_down, Power::gigawatts(-6.0));
        assert!((stats.mean_abs.as_gigawatts() - 10.0 / 3.0).abs() < 1e-12);
        // Fewer than two periods: no ramps.
        assert!(ramp_stats(&gw(&[1.0])).is_none());
    }

    #[test]
    fn moving_average_clips_at_the_edges() {
        let series = gw(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        // Half-width 1: centred over three samples, two at the edges.
        let ma = moving_average(&series, 1);
        let expected = [1.5, 2.0, 3.0, 4.0, 4.5];
        for (value, want) in ma.iter().zip(expected) {
            assert!((value.as_gigawatts() - want).abs() < 1e-12);
        }
        // Half-width beyond the series: every value is the global mean.
        let ma = moving_average(&series, 10);
        for value in ma {
            assert!((value.as_gigawatts() - 3.0).abs() < 1e-12);
        }
    }

    fn small_windows() -> DecompositionWindows {
        DecompositionWindows {
            diurnal: Duration::hours(2.0),
            synoptic: Duration::hours(6.0),
            seasonal: Duration::hours(24.0),
        }
    }

    #[test]
    fn decompose_bands_sum_to_the_series() {
        // A deliberately messy deterministic series.
        let series: Vec<Power> = (0..500)
            .map(|t| {
                let t = t as f64;
                Power::gigawatts(
                    30.0 + 10.0 * (t / 7.3).sin() + 5.0 * (t / 91.0).cos() + (t * 37.0 % 11.0),
                )
            })
            .collect();
        let d = decompose(&series, &small_windows()).unwrap();
        for (t, &value) in series.iter().enumerate() {
            let sum = d.diurnal[t] + d.synoptic[t] + d.seasonal[t] + d.inter_annual[t];
            assert!(
                (sum - value).as_gigawatts().abs() < 1e-9,
                "period {t}: bands sum to {} but the series is {}",
                sum.as_gigawatts(),
                value.as_gigawatts()
            );
        }
    }

    #[test]
    fn decompose_constant_series_is_all_inter_annual() {
        let series = vec![Power::gigawatts(42.0); 100];
        let d = decompose(&series, &small_windows()).unwrap();
        for t in 0..100 {
            assert!(d.diurnal[t].as_gigawatts().abs() < 1e-12);
            assert!(d.synoptic[t].as_gigawatts().abs() < 1e-12);
            assert!(d.seasonal[t].as_gigawatts().abs() < 1e-12);
            assert!((d.inter_annual[t].as_gigawatts() - 42.0).abs() < 1e-12);
        }
    }

    #[test]
    fn decompose_puts_a_fast_sine_mostly_in_the_diurnal_band() {
        // A 2 h sine under a 2 h diurnal window: the diurnal band must
        // carry (nearly) all the variance.
        let series: Vec<Power> = (0..2000)
            .map(|t| Power::gigawatts(20.0 + 8.0 * (t as f64 * core::f64::consts::TAU / 4.0).sin()))
            .collect();
        let d = decompose(&series, &small_windows()).unwrap();
        let variance = |band: &[Power]| -> f64 {
            let mean = band.iter().map(|p| p.as_gigawatts()).sum::<f64>() / band.len() as f64;
            band.iter()
                .map(|p| (p.as_gigawatts() - mean).powi(2))
                .sum::<f64>()
                / band.len() as f64
        };
        let total = variance(&series);
        assert!(
            variance(&d.diurnal) > 0.95 * total,
            "diurnal band carries {} of {} total variance",
            variance(&d.diurnal),
            total
        );
    }

    #[test]
    fn decompose_rejects_bad_inputs() {
        let series = gw(&[1.0, 2.0]);
        // Empty series.
        assert!(matches!(
            decompose(&[], &small_windows()).unwrap_err(),
            GridError::InvalidAnalysisInput { .. }
        ));
        // Non-increasing windows.
        let bad = DecompositionWindows {
            diurnal: Duration::hours(24.0),
            synoptic: Duration::hours(24.0),
            seasonal: Duration::hours(48.0),
        };
        assert!(matches!(
            decompose(&series, &bad).unwrap_err(),
            GridError::InvalidAnalysisInput { .. }
        ));
        // Sub-hour window.
        let bad = DecompositionWindows {
            diurnal: Duration::hours(0.4),
            synoptic: Duration::hours(6.0),
            seasonal: Duration::hours(24.0),
        };
        assert!(matches!(
            decompose(&series, &bad).unwrap_err(),
            GridError::InvalidAnalysisInput { .. }
        ));
    }

    proptest! {
        // Property (docs/06: conservation laws get property tests): the
        // bands sum back to the series for arbitrary inputs.
        #[test]
        fn bands_always_sum_to_the_series(
            values in proptest::collection::vec(-100.0f64..100.0, 1..400)
        ) {
            let series: Vec<Power> = values.iter().map(|&v| Power::gigawatts(v)).collect();
            let d = decompose(&series, &small_windows()).unwrap();
            for (t, &value) in series.iter().enumerate() {
                let sum = d.diurnal[t] + d.synoptic[t] + d.seasonal[t] + d.inter_annual[t];
                prop_assert!((sum - value).as_gigawatts().abs() < 1e-9);
            }
        }
    }
}
