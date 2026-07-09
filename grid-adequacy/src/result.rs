//! Dispatch run results: per-period series plus aggregation helpers
//! (docs/03 outputs: dispatch per technology, storage state of charge
//! per store, unserved energy, curtailment, imports; pricing and
//! emissions are layered on by `grid_core::pricing`), and the
//! firm/variable reliability accounting (the owner's gb-grid-margin
//! methodology — see `grid_core::scenario::Reliability`). The
//! classification is pure accounting over a completed run: it cannot
//! perturb dispatch.

use std::collections::BTreeMap;

use grid_core::scenario::{ExogenousReliability, Reliability, StorageKind, TechId};
use grid_core::time::UtcInstant;
use grid_core::units::{Duration, Energy, Power};

/// Per-period output series of one technology.
#[derive(Debug, Clone, PartialEq)]
pub struct TechSeries {
    /// The technology.
    pub tech: TechId,
    /// Reliability classification (gb-grid-margin methodology): the
    /// scenario's explicit override when given, else the derived
    /// default (weather-driven ⇒ variable, dispatchable ⇒ firm).
    pub reliability: Reliability,
    /// Whether `reliability` overrides the derived default — emitted
    /// into outputs so overrides cannot hide.
    pub reliability_overridden: bool,
    /// Output per settlement period. For weather-driven technologies
    /// this is *potential* (pre-curtailment) output; system surplus is
    /// reported in [`RunResult::curtailment`] as a pooled quantity.
    pub power: Vec<Power>,
}

/// Per-period series of one exogenous must-take supply.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelledSeries {
    /// Series label from the scenario.
    pub label: String,
    /// Whether this series counts toward the imports accounting.
    pub imports: bool,
    /// Reliability classification (always explicit on exogenous
    /// entries; `excluded` = counted in neither bucket).
    pub reliability: ExogenousReliability,
    /// Net supply per period (negative = export / pumping load).
    pub power: Vec<Power>,
}

/// Per-period series of one store (Stage 3): grid-side charge and
/// discharge power, and end-of-period state of charge.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreSeries {
    /// Output label: the store kind, disambiguated with
    /// `_<dispatch_order>` when the zone repeats a kind.
    pub label: String,
    /// Storage kind.
    pub kind: StorageKind,
    /// Grid-side charging power per period (drawn from surplus).
    pub charge: Vec<Power>,
    /// Grid-side discharge power per period (delivered to the grid).
    pub discharge: Vec<Power>,
    /// State of charge at the END of each period (after that period's
    /// charge/discharge is applied).
    pub soc: Vec<Energy>,
}

impl StoreSeries {
    /// Smallest end-of-period SoC and the index of its first
    /// occurrence; `None` for a zero-period run (never produced).
    #[must_use]
    pub fn min_soc(&self) -> Option<(usize, Energy)> {
        let mut best: Option<(usize, Energy)> = None;
        for (index, &soc) in self.soc.iter().enumerate() {
            if best.is_none_or(|(_, current)| soc < current) {
                best = Some((index, soc));
            }
        }
        best
    }

    /// Largest end-of-period SoC (first occurrence).
    #[must_use]
    pub fn max_soc(&self) -> Option<Energy> {
        let mut best: Option<Energy> = None;
        for &soc in &self.soc {
            if best.is_none_or(|current| soc > current) {
                best = Some(soc);
            }
        }
        best
    }
}

/// The complete result of a dispatch run. Two runs of identical inputs
/// produce bit-identical results (ADR-5); `PartialEq` compares exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    /// Start of the first settlement period.
    pub start: UtcInstant,
    /// Adjusted demand per period.
    pub demand: Vec<Power>,
    /// Weather-driven must-take technologies (potential output).
    pub renewables: Vec<TechSeries>,
    /// Exogenous must-take supply series.
    pub exogenous: Vec<LabelledSeries>,
    /// Thermal technologies in merit order.
    pub thermal: Vec<TechSeries>,
    /// Storage portfolio series, in ascending dispatch order (Stage 3).
    pub stores: Vec<StoreSeries>,
    /// Pooled system surplus per period (post-storage: surplus no store
    /// could absorb).
    pub curtailment: Vec<Power>,
    /// Unserved energy (as power) per period (post-storage: deficit no
    /// store could cover).
    pub unserved: Vec<Power>,
}

/// Sum a per-period power series into energy (half-hour periods).
fn energy_of(series: &[Power]) -> Energy {
    series
        .iter()
        .map(|&p| p * Duration::half_hour())
        .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
}

impl RunResult {
    /// Number of settlement periods.
    #[must_use]
    pub fn periods(&self) -> usize {
        self.demand.len()
    }

    /// Start of settlement period `index`.
    #[must_use]
    pub fn timestamp_at(&self, index: usize) -> UtcInstant {
        self.start.plus_periods(index as i64)
    }

    /// Total energy of any per-period power series of this run.
    #[must_use]
    pub fn total_energy(series: &[Power]) -> Energy {
        energy_of(series)
    }

    /// Energy per UTC calendar month (keyed `(year, month)`, ascending)
    /// of any per-period power series of this run.
    #[must_use]
    pub fn monthly_energy(&self, series: &[Power]) -> BTreeMap<(i64, u8), Energy> {
        let mut months: BTreeMap<(i64, u8), Energy> = BTreeMap::new();
        for (index, &power) in series.iter().enumerate() {
            let (year, month, _) = self.timestamp_at(index).civil_date();
            let entry = months
                .entry((year, month))
                .or_insert(Energy::gigawatt_hours(0.0));
            *entry = *entry + power * Duration::half_hour();
        }
        months
    }

    /// Total energy of one thermal technology, or `None` if it is not in
    /// the fleet.
    #[must_use]
    pub fn thermal_energy(&self, tech: &str) -> Option<Energy> {
        self.thermal
            .iter()
            .find(|series| series.tech.as_str() == tech)
            .map(|series| energy_of(&series.power))
    }

    /// Net annual imports: total energy of the exogenous series flagged
    /// `imports` (zero if none is).
    #[must_use]
    pub fn net_imports_energy(&self) -> Energy {
        self.exogenous
            .iter()
            .filter(|series| series.imports)
            .map(|series| energy_of(&series.power))
            .fold(Energy::gigawatt_hours(0.0), |acc, e| acc + e)
    }

    /// Total adjusted demand energy.
    #[must_use]
    pub fn total_demand_energy(&self) -> Energy {
        energy_of(&self.demand)
    }

    /// Total unserved energy.
    #[must_use]
    pub fn total_unserved(&self) -> Energy {
        energy_of(&self.unserved)
    }

    /// Total pooled curtailment energy.
    #[must_use]
    pub fn total_curtailment(&self) -> Energy {
        energy_of(&self.curtailment)
    }

    // -----------------------------------------------------------------
    // Reliability accounting (gb-grid-margin methodology, implemented
    // as published: binary, no derating; storage discharge is its own
    // fourth category, never folded into firm — the module docs).
    // -----------------------------------------------------------------

    /// Sum the fleet and exogenous series in one [`Reliability`] bucket
    /// per period. `excluded` exogenous series (pumped-storage net
    /// traces) land in neither bucket; storage discharge in neither
    /// (see [`RunResult::storage_discharge`]).
    fn reliability_bucket(&self, bucket: Reliability) -> Vec<Power> {
        let mut total = vec![Power::gigawatts(0.0); self.periods()];
        for series in self.renewables.iter().chain(&self.thermal) {
            if series.reliability != bucket {
                continue;
            }
            for (acc, &p) in total.iter_mut().zip(&series.power) {
                *acc = *acc + p;
            }
        }
        let wanted = match bucket {
            Reliability::Firm => ExogenousReliability::Firm,
            Reliability::Variable => ExogenousReliability::Variable,
        };
        for series in &self.exogenous {
            if series.reliability != wanted {
                continue;
            }
            for (acc, &p) in total.iter_mut().zip(&series.power) {
                *acc = *acc + p;
            }
        }
        total
    }

    /// Firm supply per period: dispatchable fleet output plus `firm`
    /// exogenous series.
    #[must_use]
    pub fn firm_supply(&self) -> Vec<Power> {
        self.reliability_bucket(Reliability::Firm)
    }

    /// Variable ("weather & imports") supply per period: weather-driven
    /// potential output plus `variable` exogenous series.
    #[must_use]
    pub fn variable_supply(&self) -> Vec<Power> {
        self.reliability_bucket(Reliability::Variable)
    }

    /// Total storage discharge per period — the fourth category, never
    /// folded into firm (the published methodology has no storage
    /// representation; whether storage-backed supply counts as reliable
    /// is a visibly open question).
    #[must_use]
    pub fn storage_discharge(&self) -> Vec<Power> {
        let mut total = vec![Power::gigawatts(0.0); self.periods()];
        for store in &self.stores {
            for (acc, &p) in total.iter_mut().zip(&store.discharge) {
                *acc = *acc + p;
            }
        }
        total
    }

    /// The headline metric: firm share of demand per period,
    /// **unclamped** (net-export periods legitimately exceed 1.0).
    /// Dimensionless ratio; demand is assumed nonzero (real demand
    /// traces are tens of GW every period).
    #[must_use]
    pub fn firm_share(&self) -> Vec<f64> {
        self.firm_supply()
            .iter()
            .zip(&self.demand)
            .map(|(firm, demand)| firm.as_gigawatts() / demand.as_gigawatts())
            .collect()
    }

    /// Summary statistics of [`RunResult::firm_share`]; `None` only for
    /// a zero-period run (never produced).
    #[must_use]
    pub fn firm_share_stats(&self) -> Option<FirmShareStats> {
        let shares = self.firm_share();
        if shares.is_empty() {
            return None;
        }
        let mean = shares.iter().sum::<f64>() / shares.len() as f64;
        let min = shares.iter().copied().fold(f64::INFINITY, f64::min);
        let mut sorted = shares.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        // 25th percentile by linear interpolation between closest
        // ranks (the numpy default), documented here as the pinned
        // definition.
        let rank = 0.25 * (sorted.len() - 1) as f64;
        let lower = rank.floor() as usize;
        let fraction = rank - lower as f64;
        let p25 = if lower + 1 < sorted.len() {
            sorted[lower] + fraction * (sorted[lower + 1] - sorted[lower])
        } else {
            sorted[lower]
        };
        let below_threshold = shares
            .iter()
            .filter(|&&s| s < FIRM_SHARE_ALARM_THRESHOLD)
            .count();
        Some(FirmShareStats {
            mean,
            min,
            p25,
            below_threshold,
        })
    }
}

/// The gb-grid-margin alarm threshold: a period is alarming when firm
/// supply covers less than half of demand.
pub const FIRM_SHARE_ALARM_THRESHOLD: f64 = 0.5;

/// Summary statistics of the per-period firm share (unclamped ratios).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirmShareStats {
    /// Arithmetic mean over all periods (periods are equal-length, so
    /// this is also the time-weighted mean).
    pub mean: f64,
    /// Smallest per-period share.
    pub min: f64,
    /// 25th percentile (linear interpolation between closest ranks).
    pub p25: f64,
    /// Number of periods with share < [`FIRM_SHARE_ALARM_THRESHOLD`].
    pub below_threshold: usize,
}
