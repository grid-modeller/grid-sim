//! Stage 2 pricing layer (ADR-9): SRMC per technology, system marginal
//! price per period, revenue and capture-price accounting, emissions
//! totals, and the reported price-realism statistics.
//!
//! # The SRMC recipe (docs/04 Stage 2; data/reference/prices-2024.toml)
//!
//! ```text
//! SRMC [£/MWh_e] = fuel_price / η + (EF_CO2 / η) × carbon_price
//! ```
//!
//! with the fuel price in £/MWh-thermal, the emission factor in
//! tCO2/MWh-thermal and the efficiency η all on a **consistent gross
//! calorific value (HHV) basis** — GB gas is traded and billed gross, and
//! the committed reference file's efficiencies and emission factor are
//! gross-CV. Callers must not mix bases; the reference file is the pinned
//! HHV-consistent source. The carbon price is the UKA step series plus
//! the Carbon Price Support, both charging **CO₂ only** (not CO₂e): the
//! UK ETS and CPS levy combustion CO₂. The CO₂e factor exists for
//! *emissions accounting*, never for pricing (reference file note).
//!
//! # The system-marginal-price conventions, in prose
//!
//! These are modelling choices, documented here because they are
//! contestable (same discipline as the dispatch rules, docs/06):
//!
//! 1. **Only SRMC-bearing technologies can set the price.** A technology
//!    sets the system marginal price in a period only if it has an SRMC
//!    model and strictly positive dispatch in that period. The SMP is the
//!    *maximum* SRMC over those technologies (under merit-order dispatch
//!    the most expensive running unit is the marginal one).
//! 2. **Must-take-only periods price at zero.** When no SRMC-bearing
//!    technology is dispatched — demand is covered by renewables,
//!    must-run/calibrated plant (nuclear, biomass, hydro, coal carry no
//!    SRMC model in the 2024 reference inputs) and exogenous must-take
//!    traces — SMP = £0/MWh with no price-setter. Zero is the marginal
//!    cost of the marginal unit in such a period under this model's cost
//!    structure. *Limits, stated out loud:* real GB prices in such
//!    periods are set by bids, interconnector arbitrage and subsidy
//!    opportunity costs, and were **negative** in 495 half-hours of 2024
//!    (down to −£61/MWh; price pack report §2,
//!    `docs/notes/2024-price-pack-report.md`); this model cannot
//!    produce negative prices, and
//!    exogenous imports/pumped-storage/"other" traces never set the
//!    price even though real imports and pumped storage do set GB prices
//!    in some periods.
//! 3. **Unserved periods price at the fleet's SRMC ceiling**: the highest
//!    SRMC among all SRMC-bearing technologies in that period, dispatched
//!    or not. This is a documented *floor* on the true scarcity price (no
//!    value-of-lost-load model exists until the cost-synthesis stage); an
//!    unserved period in a fleet with no SRMC-bearing technology at all
//!    is an error rather than a silent zero. No unserved periods occur in
//!    the 2024 reference run.
//!
//! # Deliberate non-features
//!
//! No VOM adder (excluded from the pinned recipe; the reference file
//! carries the sensitivity note), no start costs, no bid/offer behaviour,
//! no scarcity rent. The SMP is an SRMC stack price, nothing more.

use crate::GridError;
use crate::scenario::TechId;
use crate::time::UtcInstant;
use crate::trace::Trace;
use crate::units::{
    CarbonPrice, Duration, Emissions, EmissionsRate, Energy, Money, PerUnit, Power, Price,
};

/// One UK ETS allowance auction: clearing date and price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarbonAuction {
    /// Auction date (00:00 UTC of the clearing day; the step series
    /// applies the price from this instant onward).
    pub date: UtcInstant,
    /// Clearing price, £/tCO2.
    pub clearing_price: CarbonPrice,
}

/// Build the per-period carbon price series: the UKA auction prices
/// step-interpolated (forward-fill from the most recent auction; before
/// the first auction, the first auction's price — the reference-file
/// convention) plus the Carbon Price Support added to every period.
///
/// Errors with [`GridError::InvalidPricing`] if `auctions` is empty.
pub fn carbon_price_step_series(
    auctions: &[CarbonAuction],
    cps: CarbonPrice,
    start: UtcInstant,
    periods: usize,
) -> Result<Trace<CarbonPrice>, GridError> {
    if auctions.is_empty() {
        return Err(GridError::InvalidPricing {
            reason: "carbon price step series needs at least one auction".to_owned(),
        });
    }
    let mut sorted: Vec<CarbonAuction> = auctions.to_vec();
    sorted.sort_by_key(|auction| auction.date);

    let mut values = Vec::with_capacity(periods);
    let mut next = 0usize; // index of the next auction not yet in force
    let mut current = sorted[0].clearing_price; // before-first-auction convention
    for t in 0..periods {
        let instant = start.plus_periods(t as i64);
        while next < sorted.len() && sorted[next].date <= instant {
            current = sorted[next].clearing_price;
            next += 1;
        }
        values.push(current + cps);
    }
    Trace::from_parts(start, values)
}

/// Compute a per-period SRMC series from an HHV-consistent fuel-price
/// trace (£/MWh-thermal), a carbon-price trace (£/tCO2, CO₂-only basis),
/// a thermal efficiency (HHV) and a CO₂ emission factor
/// (tCO2/MWh-thermal, HHV). See the module docs for the recipe and the
/// basis rules.
///
/// Errors with [`GridError::InvalidPricing`] if the efficiency is outside
/// `(0, 1]`, the emission factor is negative, or the traces are
/// misaligned.
pub fn srmc_series(
    fuel_price_thermal: &Trace<Price>,
    carbon_price: &Trace<CarbonPrice>,
    efficiency_hhv: PerUnit,
    emission_factor_co2_thermal: EmissionsRate,
) -> Result<Trace<Price>, GridError> {
    let eta = efficiency_hhv.value();
    if !(eta > 0.0 && eta <= 1.0) {
        return Err(GridError::InvalidPricing {
            reason: format!("efficiency {eta} is outside (0, 1]"),
        });
    }
    if emission_factor_co2_thermal.as_tonnes_per_megawatt_hour() < 0.0 {
        return Err(GridError::InvalidPricing {
            reason: format!(
                "emission factor {} tCO2/MWh is negative",
                emission_factor_co2_thermal.as_tonnes_per_megawatt_hour()
            ),
        });
    }
    if fuel_price_thermal.len() != carbon_price.len()
        || fuel_price_thermal.start() != carbon_price.start()
    {
        return Err(GridError::InvalidPricing {
            reason: format!(
                "fuel-price trace ({} periods from {}) and carbon-price trace ({} periods \
                 from {}) are misaligned",
                fuel_price_thermal.len(),
                fuel_price_thermal.start(),
                carbon_price.len(),
                carbon_price.start(),
            ),
        });
    }
    let ef_electric = emission_factor_co2_thermal / efficiency_hhv;
    let values = fuel_price_thermal
        .values()
        .iter()
        .zip(carbon_price.values())
        .map(|(&fuel, &carbon)| fuel / efficiency_hhv + ef_electric * carbon)
        .collect();
    Trace::from_parts(fuel_price_thermal.start(), values)
}

/// One technology's dispatch series and, if it has one, its SRMC series
/// — the input row to [`system_marginal_price`]. Technologies without an
/// SRMC model (must-run/calibrated plant) never set the price
/// (convention 1 in the module docs).
#[derive(Debug, Clone)]
pub struct PricedSeries<'a> {
    /// The technology.
    pub tech: TechId,
    /// Dispatched output per period.
    pub power: &'a [Power],
    /// Per-period SRMC, or `None` for technologies with no SRMC model.
    pub srmc: Option<&'a [Price]>,
}

/// The system marginal price series and its per-period price-setter.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemPrices {
    /// System marginal price per period.
    pub smp: Vec<Price>,
    /// The technology that set the price, or `None` in must-take-only
    /// periods (convention 2 in the module docs).
    pub setter: Vec<Option<TechId>>,
}

/// Compute the system marginal price per period under the conventions
/// documented in the module docs: the most expensive dispatched
/// SRMC-bearing technology sets the price; must-take-only periods price
/// at zero with no setter; unserved periods price at the fleet's SRMC
/// ceiling.
///
/// Errors with [`GridError::InvalidPricing`] on misaligned series or an
/// unserved period with no SRMC-bearing technology.
pub fn system_marginal_price(
    series: &[PricedSeries<'_>],
    unserved: &[Power],
) -> Result<SystemPrices, GridError> {
    let periods = unserved.len();
    for entry in series {
        if entry.power.len() != periods {
            return Err(GridError::InvalidPricing {
                reason: format!(
                    "dispatch series for {} has {} periods; expected {periods}",
                    entry.tech,
                    entry.power.len()
                ),
            });
        }
        if let Some(srmc) = entry.srmc
            && srmc.len() != periods
        {
            return Err(GridError::InvalidPricing {
                reason: format!(
                    "SRMC series for {} has {} periods; expected {periods}",
                    entry.tech,
                    srmc.len()
                ),
            });
        }
    }

    let mut smp = Vec::with_capacity(periods);
    let mut setter = Vec::with_capacity(periods);
    for t in 0..periods {
        // Convention 1: max SRMC over dispatched SRMC-bearing techs.
        let mut best: Option<(Price, &TechId)> = None;
        for entry in series {
            let Some(srmc) = entry.srmc else { continue };
            if entry.power[t] > Power::gigawatts(0.0)
                && best.is_none_or(|(price, _)| srmc[t] > price)
            {
                best = Some((srmc[t], &entry.tech));
            }
        }

        // Convention 3: unserved periods price at the fleet SRMC ceiling.
        if unserved[t] > Power::gigawatts(0.0) {
            let mut ceiling: Option<(Price, &TechId)> = None;
            for entry in series {
                let Some(srmc) = entry.srmc else { continue };
                if ceiling.is_none_or(|(price, _)| srmc[t] > price) {
                    ceiling = Some((srmc[t], &entry.tech));
                }
            }
            let Some((price, tech)) = ceiling else {
                return Err(GridError::InvalidPricing {
                    reason: format!(
                        "period {t} has unserved energy but no SRMC-bearing technology to \
                         bound its price (scarcity pricing arrives with the cost-synthesis \
                         stage)"
                    ),
                });
            };
            smp.push(price);
            setter.push(Some(tech.clone()));
            continue;
        }

        match best {
            Some((price, tech)) => {
                smp.push(price);
                setter.push(Some(tech.clone()));
            }
            None => {
                // Convention 2: must-take-only period.
                smp.push(Price::pounds_per_megawatt_hour(0.0));
                setter.push(None);
            }
        }
    }
    Ok(SystemPrices { smp, setter })
}

/// Check two per-period series have the same length.
fn check_aligned(context: &str, a: usize, b: usize) -> Result<(), GridError> {
    if a != b {
        return Err(GridError::InvalidPricing {
            reason: format!("{context}: series lengths differ ({a} vs {b})"),
        });
    }
    Ok(())
}

/// Total energy of a half-hourly dispatch series.
fn series_energy(power: &[Power]) -> Energy {
    power
        .iter()
        .map(|&p| p * Duration::half_hour())
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

/// A technology's revenue over the run: `Σ_t dispatch(t) × Δt × SMP(t)`.
pub fn revenue(power: &[Power], smp: &[Price]) -> Result<Money, GridError> {
    check_aligned("revenue", power.len(), smp.len())?;
    Ok(power
        .iter()
        .zip(smp)
        .map(|(&p, &price)| (p * Duration::half_hour()) * price)
        .fold(Money::pounds(0.0), |acc, m| acc + m))
}

/// A technology's capture price: revenue / energy, or `None` for zero
/// output (never NaN).
pub fn capture_price(power: &[Power], smp: &[Price]) -> Result<Option<Price>, GridError> {
    let energy = series_energy(power);
    if energy == Energy::gigawatt_hours(0.0) {
        // Still validate alignment so a zero-output series with a wrong
        // SMP length is not silently accepted.
        check_aligned("capture price", power.len(), smp.len())?;
        return Ok(None);
    }
    Ok(Some(revenue(power, smp)? / energy))
}

/// The time-weighted mean price (uniform half-hourly periods, so the
/// arithmetic mean); `None` for an empty series.
#[must_use]
pub fn time_weighted_mean_price(smp: &[Price]) -> Option<Price> {
    if smp.is_empty() {
        return None;
    }
    let sum: f64 = smp.iter().map(|p| p.as_pounds_per_megawatt_hour()).sum();
    Some(Price::pounds_per_megawatt_hour(sum / smp.len() as f64))
}

/// Capture ratio: capture price / time-weighted mean SMP — a
/// dimensionless quotient of two prices (may exceed 1). `None` for zero
/// output or an empty series.
pub fn capture_ratio(power: &[Power], smp: &[Price]) -> Result<Option<f64>, GridError> {
    let (Some(capture), Some(mean)) = (capture_price(power, smp)?, time_weighted_mean_price(smp))
    else {
        return Ok(None);
    };
    Ok(Some(
        capture.as_pounds_per_megawatt_hour() / mean.as_pounds_per_megawatt_hour(),
    ))
}

/// Total emissions of a dispatch series at an **electric-basis**
/// emissions intensity (tCO2/MWh-electric — a thermal-basis factor must
/// first be divided by the efficiency). Whether the intensity is CO₂
/// (pricing basis) or CO₂e (accounting basis) is the caller's labelled
/// choice.
#[must_use]
pub fn total_emissions(power: &[Power], rate_electric: EmissionsRate) -> Emissions {
    series_energy(power) * rate_electric
}

/// The fraction of periods (0–1) in which one of the named technologies
/// set the price.
#[must_use]
pub fn price_setting_share(setter: &[Option<TechId>], techs: &[&str]) -> f64 {
    if setter.is_empty() {
        return 0.0;
    }
    let count = setter
        .iter()
        .filter(|s| {
            s.as_ref()
                .is_some_and(|tech| techs.contains(&tech.as_str()))
        })
        .count();
    count as f64 / setter.len() as f64
}

/// Median of the per-period `model / observed` price ratios — the
/// reported (not gated) realism statistic of docs/04 Stage 2. All
/// periods enter, including negative observed prices (the median is
/// robust to the resulting outlier ratios; the 2024 MID has 495 negative
/// periods — price pack report §2,
/// `docs/notes/2024-price-pack-report.md`).
pub fn median_ratio(model: &[Price], observed: &[Price]) -> Result<f64, GridError> {
    check_aligned("median ratio", model.len(), observed.len())?;
    if model.is_empty() {
        return Err(GridError::InvalidPricing {
            reason: "median ratio of empty series".to_owned(),
        });
    }
    let mut ratios: Vec<f64> = model
        .iter()
        .zip(observed)
        .map(|(m, o)| m.as_pounds_per_megawatt_hour() / o.as_pounds_per_megawatt_hour())
        .collect();
    ratios.sort_by(f64::total_cmp);
    let n = ratios.len();
    Ok(if n % 2 == 1 {
        ratios[n / 2]
    } else {
        (ratios[n / 2 - 1] + ratios[n / 2]) / 2.0
    })
}

/// Pearson correlation of calendar-month mean prices between a model
/// series and an observed series starting at the same instant — the
/// second reported realism statistic of docs/04 Stage 2.
///
/// Errors with [`GridError::InvalidPricing`] on misaligned series or
/// fewer than two calendar months.
pub fn monthly_mean_correlation(
    start: UtcInstant,
    model: &[Price],
    observed: &[Price],
) -> Result<f64, GridError> {
    check_aligned("monthly correlation", model.len(), observed.len())?;
    // Group by (year, month); the trace is chronological, so months form
    // contiguous runs and a Vec keyed by change-of-month suffices.
    let mut model_means: Vec<f64> = Vec::new();
    let mut observed_means: Vec<f64> = Vec::new();
    let mut current_month: Option<(i64, u8)> = None;
    let (mut sum_m, mut sum_o, mut count) = (0.0f64, 0.0f64, 0usize);
    for (t, (m, o)) in model.iter().zip(observed).enumerate() {
        let (year, month, _) = start.plus_periods(t as i64).civil_date();
        if current_month != Some((year, month)) {
            if count > 0 {
                model_means.push(sum_m / count as f64);
                observed_means.push(sum_o / count as f64);
            }
            current_month = Some((year, month));
            (sum_m, sum_o, count) = (0.0, 0.0, 0);
        }
        sum_m += m.as_pounds_per_megawatt_hour();
        sum_o += o.as_pounds_per_megawatt_hour();
        count += 1;
    }
    if count > 0 {
        model_means.push(sum_m / count as f64);
        observed_means.push(sum_o / count as f64);
    }
    if model_means.len() < 2 {
        return Err(GridError::InvalidPricing {
            reason: format!(
                "monthly correlation needs at least two calendar months, got {}",
                model_means.len()
            ),
        });
    }
    Ok(pearson(&model_means, &observed_means))
}

/// Pearson correlation coefficient of two equal-length samples.
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
    let sy: f64 = y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt();
    sxy / (sx * sy)
}
