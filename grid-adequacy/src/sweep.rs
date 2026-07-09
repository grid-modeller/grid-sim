//! The parameter sweep runner (ADR-1: sweeps live in grid-adequacy;
//! ADR-10: brute-force sweeps parallelised with rayon, **full response
//! surfaces kept**, never just optima) and the Q4 per-year batch mode.
//!
//! # Sweep model
//!
//! A sweep is a scenario plus one or two [`Dimension`]s, each a list of
//! values for one scenario knob. The runner evaluates a full dispatch
//! run at every grid point (row-major over the dimension value lists)
//! and records summary metrics per point ([`SweepPoint`]) — the full
//! response surface. Each point is a pure function of its scenario
//! variant (ADR-5), so parallel execution with rayon's order-preserving
//! `collect` is **bit-identical to serial execution**; the Stage 4
//! acceptance test asserts this, and [`Execution::Serial`] keeps the
//! serial path callable forever.
//!
//! Sweep specs are TOML files ([`SweepSpec`]) — not scenarios; the
//! scenario schema is untouched. Dimension values enter the unit system
//! at the spec boundary via unit-named fields (`values_gw`,
//! `values_gwh`; ADR-4).
//!
//! # Input reuse (documented performance decision)
//!
//! Traces are loaded once. The demand trace is loaded unscaled
//! (`annual_scale = 1`, `extra_demand_gw = 0`) and the scenario's own
//! scaling applied in memory. For a scenario WITHOUT a heating overlay
//! this is bit-identical to loading with the scaling applied, since
//! both compute `base × scale + extra` per period. With a schema-v5
//! heating overlay it is NOT a bit-level guarantee: the loader folds
//! the overlay into the neutral load, so the rescale round-trips
//! `(neutral − h) × scale + extra + h` where a direct load computes
//! `base × scale + extra + h`, and `(base + h) − h` can differ from
//! `base` by IEEE rounding — ULP-scale of the neutral load per period
//! (relative ~1e-16). No digest may be assumed shared between a sweep
//! point and a direct run on a heating scenario. (No committed sweep
//! artefact exercises a heating overlay today; the committed sweeps run
//! non-heating scenarios.) Points that change neither demand knob share
//! the loaded inputs by reference; an `annual_scale` sweep rebuilds the
//! demand trace (and clones the other inputs) per point.
//!
//! # Per-year batch mode (Q4: "one year or forty?")
//!
//! [`per_year_requirements`] solves `min_storage_for_zero_unserved` for
//! every weather year as an *independent single-year scenario*: the
//! horizon is clamped to the year and every multi-file trace list is
//! filtered to the file(s) naming that year — which is why batch mode
//! requires per-year trace files (the repo's `data/packs/cf` and
//! `data/packs/demand-tiled` layout). Infeasible years are recorded as
//! results, not errors: a year the fleet cannot serve at any store size
//! is a finding.
//!
//! # Multi-zone wind-capacity sweep (D11 rule 2)
//!
//! [`wind_capacity_sweep_multi`] runs the Module 1 wind-capacity sweep
//! on the multi-zone engine ([`crate::multizone::run_multi`]), so
//! imports respond **endogenously** to the swept fleet instead of the
//! frozen-2024-imports convention (the tier-2 fix of the tracked
//! frozen-imports deviation; docs/notes/d11-priced-dispatch.md rule 2).
//! Conventions, stated in full at the function; the headline one is
//! that ONLY the swept zone's wind fleet scales — every external
//! zone's fleet, demand and traces stay at their committed 2024 basis
//! (external fleets are NOT projected).

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::Path;

use rayon::prelude::*;
use serde::Deserialize;

use grid_core::GridError;
use grid_core::pricing::{
    PricedSeries, capture_ratio, price_setting_share, system_marginal_price,
    time_weighted_mean_price,
};
use grid_core::scenario::{Scenario, TechId, TraceFiles, WeatherYears, ZoneId};
use grid_core::time::UtcInstant;
use grid_core::trace::Trace;
use grid_core::units::{Energy, Power, Price};

use crate::dispatch::run;
use crate::inputs::{MultiZoneInputs, RunInputs, ZonePricingInputs, load_run_inputs, single_zone};
use crate::multizone::run_multi;
use crate::pricing::delivered_renewable_power;
use crate::result::RunResult;
use crate::solve::{SolveOptions, min_storage_for_zero_unserved};

// ---------------------------------------------------------------------
// Sweep specification (TOML).
// ---------------------------------------------------------------------

/// A sweep specification file: the scenario to sweep and one or two
/// dimensions. Strictly parsed (`deny_unknown_fields`), like every
/// input file in this project.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SweepSpec {
    /// Scenario TOML path, resolved against the run's base directory.
    pub scenario: String,
    /// Optional human-readable name for output metadata.
    #[serde(default)]
    pub name: Option<String>,
    /// One or two swept dimensions.
    pub dimensions: Vec<DimensionSpec>,
}

impl SweepSpec {
    /// Parse a sweep spec from TOML text.
    pub fn from_toml_str(text: &str) -> Result<Self, GridError> {
        toml::from_str(text).map_err(|source| GridError::ScenarioParse {
            source: Box::new(source),
        })
    }

    /// Read and parse a sweep spec file.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InScenarioFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    /// Resolve the spec's dimensions against a loaded scenario:
    /// validates the dimension count (1 or 2), the value lists, and
    /// that every referenced technology / store exists.
    pub fn resolve(&self, scenario: &Scenario) -> Result<Vec<Dimension>, GridError> {
        if self.dimensions.is_empty() || self.dimensions.len() > 2 {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "a sweep needs one or two dimensions; the spec has {}",
                    self.dimensions.len()
                ),
            });
        }
        self.dimensions
            .iter()
            .map(|d| d.resolve(scenario))
            .collect()
    }
}

/// An evenly spaced inclusive value range: `count` values from `start`
/// to `stop` (`count ≥ 2`).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Range<T> {
    /// First value (inclusive).
    pub start: T,
    /// Last value (inclusive).
    pub stop: T,
    /// Number of values.
    pub count: usize,
}

/// Inclusive linspace on raw values (unit conversion happens at the
/// dimension variant, where the unit is known).
fn linspace(start: f64, stop: f64, count: usize) -> Result<Vec<f64>, GridError> {
    if count < 2 || !start.is_finite() || !stop.is_finite() {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "a sweep range needs finite endpoints and count ≥ 2 \
                 (got start {start}, stop {stop}, count {count})"
            ),
        });
    }
    Ok((0..count)
        .map(|i| start + (stop - start) * i as f64 / (count - 1) as f64)
        .collect())
}

/// One swept dimension, as written in the spec TOML. Each target names
/// its values with the unit in the field name (ADR-4): either an
/// explicit list or an evenly spaced range, exactly one of the two.
///
/// ```toml
/// [[dimensions]]
/// target = "store_energy"
/// store_index = 0
/// range_gwh = { start = 10000.0, stop = 80000.0, count = 8 }
///
/// [[dimensions]]
/// target = "fleet_scale"
/// technologies = ["offshore_wind", "onshore_wind", "solar"]
/// values = [0.85, 1.0, 1.15, 1.3, 1.45]
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case", deny_unknown_fields)]
pub enum DimensionSpec {
    /// One fleet entry's `capacity_gw`.
    FleetCapacity {
        /// The technology whose capacity is swept.
        technology: String,
        /// Explicit capacities, GW.
        #[serde(default)]
        values_gw: Option<Vec<Power>>,
        /// Evenly spaced capacities, GW.
        #[serde(default)]
        range_gw: Option<Range<Power>>,
    },
    /// One store's `energy_gwh`.
    StoreEnergy {
        /// Index into the zone's storage list (scenario order).
        store_index: usize,
        /// Explicit capacities, GWh.
        #[serde(default)]
        values_gwh: Option<Vec<Energy>>,
        /// Evenly spaced capacities, GWh.
        #[serde(default)]
        range_gwh: Option<Range<Energy>>,
    },
    /// One store's `power_gw`.
    StorePower {
        /// Index into the zone's storage list (scenario order).
        store_index: usize,
        /// Explicit ratings, GW.
        #[serde(default)]
        values_gw: Option<Vec<Power>>,
        /// Evenly spaced ratings, GW.
        #[serde(default)]
        range_gw: Option<Range<Power>>,
    },
    /// The zone's demand `annual_scale` (dimensionless multiplier).
    AnnualScale {
        /// Explicit scale factors.
        #[serde(default)]
        values: Option<Vec<f64>>,
        /// Evenly spaced scale factors.
        #[serde(default)]
        range: Option<Range<f64>>,
    },
    /// A dimensionless multiplier applied to the `capacity_gw` of every
    /// named technology — the overbuild axis (Module 4).
    FleetScale {
        /// The technologies scaled together.
        technologies: Vec<String>,
        /// Explicit multipliers.
        #[serde(default)]
        values: Option<Vec<f64>>,
        /// Evenly spaced multipliers.
        #[serde(default)]
        range: Option<Range<f64>>,
    },
}

/// Exactly one of `values` / `range` must be given; returns the raw
/// value list.
fn values_or_range(
    what: &str,
    values: Option<Vec<f64>>,
    range: Option<(f64, f64, usize)>,
) -> Result<Vec<f64>, GridError> {
    match (values, range) {
        (Some(values), None) => {
            if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
                return Err(GridError::InvalidScenario {
                    reason: format!("{what}: sweep values must be non-empty and finite"),
                });
            }
            Ok(values)
        }
        (None, Some((start, stop, count))) => linspace(start, stop, count),
        _ => Err(GridError::InvalidScenario {
            reason: format!("{what}: give exactly one of an explicit value list or a range"),
        }),
    }
}

impl DimensionSpec {
    /// Validate against the scenario and produce the typed dimension.
    pub fn resolve(&self, scenario: &Scenario) -> Result<Dimension, GridError> {
        let zone = single_zone(scenario)?;
        let has_tech = |name: &str| zone.fleet.iter().any(|e| e.technology.as_str() == name);
        let check_store = |index: usize| -> Result<(), GridError> {
            if index >= zone.storage.len() {
                return Err(GridError::InvalidScenario {
                    reason: format!(
                        "sweep dimension names store index {index}, but zone {} has {} stores",
                        zone.id,
                        zone.storage.len()
                    ),
                });
            }
            Ok(())
        };
        match self {
            Self::FleetCapacity {
                technology,
                values_gw,
                range_gw,
            } => {
                if !has_tech(technology) {
                    return Err(GridError::InvalidScenario {
                        reason: format!(
                            "sweep dimension names technology {technology}, absent from the fleet"
                        ),
                    });
                }
                let raw = values_or_range(
                    "fleet_capacity",
                    values_gw
                        .as_ref()
                        .map(|v| v.iter().map(|p| p.as_gigawatts()).collect()),
                    range_gw.map(|r| (r.start.as_gigawatts(), r.stop.as_gigawatts(), r.count)),
                )?;
                Ok(Dimension::FleetCapacity {
                    technology: TechId::new(technology.clone()),
                    values: raw.into_iter().map(Power::gigawatts).collect(),
                })
            }
            Self::StoreEnergy {
                store_index,
                values_gwh,
                range_gwh,
            } => {
                check_store(*store_index)?;
                let raw = values_or_range(
                    "store_energy",
                    values_gwh
                        .as_ref()
                        .map(|v| v.iter().map(|e| e.as_gigawatt_hours()).collect()),
                    range_gwh.map(|r| {
                        (
                            r.start.as_gigawatt_hours(),
                            r.stop.as_gigawatt_hours(),
                            r.count,
                        )
                    }),
                )?;
                Ok(Dimension::StoreEnergy {
                    store_index: *store_index,
                    values: raw.into_iter().map(Energy::gigawatt_hours).collect(),
                })
            }
            Self::StorePower {
                store_index,
                values_gw,
                range_gw,
            } => {
                check_store(*store_index)?;
                let raw = values_or_range(
                    "store_power",
                    values_gw
                        .as_ref()
                        .map(|v| v.iter().map(|p| p.as_gigawatts()).collect()),
                    range_gw.map(|r| (r.start.as_gigawatts(), r.stop.as_gigawatts(), r.count)),
                )?;
                Ok(Dimension::StorePower {
                    store_index: *store_index,
                    values: raw.into_iter().map(Power::gigawatts).collect(),
                })
            }
            Self::AnnualScale { values, range } => {
                let raw = values_or_range(
                    "annual_scale",
                    values.clone(),
                    range.map(|r| (r.start, r.stop, r.count)),
                )?;
                Ok(Dimension::AnnualScale { values: raw })
            }
            Self::FleetScale {
                technologies,
                values,
                range,
            } => {
                if technologies.is_empty() {
                    return Err(GridError::InvalidScenario {
                        reason: "fleet_scale: at least one technology is required".to_owned(),
                    });
                }
                for name in technologies {
                    if !has_tech(name) {
                        return Err(GridError::InvalidScenario {
                            reason: format!(
                                "sweep dimension names technology {name}, absent from the fleet"
                            ),
                        });
                    }
                }
                let raw = values_or_range(
                    "fleet_scale",
                    values.clone(),
                    range.map(|r| (r.start, r.stop, r.count)),
                )?;
                Ok(Dimension::FleetScale {
                    technologies: technologies.iter().cloned().map(TechId::new).collect(),
                    values: raw,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------
// Resolved dimensions and the runner.
// ---------------------------------------------------------------------

/// A resolved, typed sweep dimension: a scenario knob plus its value
/// list.
#[derive(Debug, Clone, PartialEq)]
pub enum Dimension {
    /// One fleet entry's capacity.
    FleetCapacity {
        /// The swept technology.
        technology: TechId,
        /// Capacities to evaluate.
        values: Vec<Power>,
    },
    /// One store's energy capacity.
    StoreEnergy {
        /// Index into the zone's storage list.
        store_index: usize,
        /// Capacities to evaluate.
        values: Vec<Energy>,
    },
    /// One store's power rating.
    StorePower {
        /// Index into the zone's storage list.
        store_index: usize,
        /// Ratings to evaluate.
        values: Vec<Power>,
    },
    /// The demand `annual_scale` multiplier.
    AnnualScale {
        /// Scale factors to evaluate.
        values: Vec<f64>,
    },
    /// A common multiplier on the named technologies' capacities.
    FleetScale {
        /// The technologies scaled together.
        technologies: Vec<TechId>,
        /// Multipliers to evaluate.
        values: Vec<f64>,
    },
}

impl Dimension {
    /// Number of values along this dimension.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::FleetCapacity { values, .. } => values.len(),
            Self::StoreEnergy { values, .. } => values.len(),
            Self::StorePower { values, .. } => values.len(),
            Self::AnnualScale { values } => values.len(),
            Self::FleetScale { values, .. } => values.len(),
        }
    }

    /// Whether the dimension has no values (never true after
    /// resolution).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Output column name, unit-suffixed where physical.
    #[must_use]
    pub fn column(&self) -> String {
        match self {
            Self::FleetCapacity { technology, .. } => format!("{technology}_capacity_gw"),
            Self::StoreEnergy { store_index, .. } => format!("store{store_index}_energy_gwh"),
            Self::StorePower { store_index, .. } => format!("store{store_index}_power_gw"),
            Self::AnnualScale { .. } => "annual_scale".to_owned(),
            Self::FleetScale { .. } => "fleet_scale".to_owned(),
        }
    }

    /// The `index`-th coordinate in the column's declared unit (for
    /// output tables and charts — the labelled conversion point,
    /// ADR-4).
    #[must_use]
    pub fn coordinate(&self, index: usize) -> f64 {
        match self {
            Self::FleetCapacity { values, .. } => values[index].as_gigawatts(),
            Self::StoreEnergy { values, .. } => values[index].as_gigawatt_hours(),
            Self::StorePower { values, .. } => values[index].as_gigawatts(),
            Self::AnnualScale { values } => values[index],
            Self::FleetScale { values, .. } => values[index],
        }
    }

    /// Apply the `index`-th value to a scenario variant.
    fn apply(&self, scenario: &mut Scenario, index: usize) {
        let zone = &mut scenario.zones[0];
        match self {
            Self::FleetCapacity { technology, values } => {
                for entry in &mut zone.fleet {
                    if entry.technology == *technology {
                        entry.capacity_gw = values[index];
                    }
                }
            }
            Self::StoreEnergy {
                store_index,
                values,
            } => {
                zone.storage[*store_index].energy_gwh = values[index];
            }
            Self::StorePower {
                store_index,
                values,
            } => {
                zone.storage[*store_index].power_gw = values[index];
            }
            Self::AnnualScale { values } => {
                zone.demand.annual_scale = values[index];
            }
            Self::FleetScale {
                technologies,
                values,
            } => {
                for entry in &mut zone.fleet {
                    if technologies.contains(&entry.technology) {
                        entry.capacity_gw = entry.capacity_gw * values[index];
                    }
                }
            }
        }
    }
}

/// How to execute the grid: rayon (the default) or the forced serial
/// path (kept callable so determinism under parallelism stays
/// testable — docs/04 Stage 4 acceptance).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    /// rayon `par_iter` over the grid points (ADR-10).
    Parallel,
    /// Plain sequential iteration, same order.
    Serial,
}

/// One grid point of the response surface: the dimension indices and
/// the run's summary metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepPoint {
    /// Value index per dimension (`indices[d]` indexes
    /// `dimensions[d]`'s value list).
    pub indices: Vec<usize>,
    /// Total adjusted demand energy.
    pub demand: Energy,
    /// Total weather-driven potential energy (pre-curtailment).
    pub renewable_potential: Energy,
    /// Total unserved energy.
    pub unserved: Energy,
    /// Number of periods with unserved energy.
    pub unserved_periods: usize,
    /// Total pooled curtailment energy.
    pub curtailment: Energy,
    /// Minimum end-of-period SoC per store (label, SoC), dispatch
    /// order.
    pub store_min_soc: Vec<(String, Energy)>,
}

/// A completed sweep: the resolved dimensions and every point of the
/// response surface, row-major (last dimension fastest).
#[derive(Debug, Clone, PartialEq)]
pub struct SweepResult {
    /// The swept dimensions.
    pub dimensions: Vec<Dimension>,
    /// The full response surface (ADR-10: never just the optimum).
    pub points: Vec<SweepPoint>,
}

/// Run the sweep: one dispatch run per grid point over the cartesian
/// product of the dimension values, metrics recorded per point. See the
/// module docs for the determinism and input-reuse guarantees.
pub fn run_sweep(
    scenario: &Scenario,
    base_dir: &Path,
    dimensions: &[Dimension],
    execution: Execution,
) -> Result<SweepResult, GridError> {
    if dimensions.is_empty() || dimensions.len() > 2 {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "a sweep needs one or two dimensions; got {}",
                dimensions.len()
            ),
        });
    }
    for dimension in dimensions {
        if dimension.is_empty() {
            return Err(GridError::InvalidScenario {
                reason: format!("sweep dimension {} has no values", dimension.column()),
            });
        }
    }
    scenario.validate()?;
    let zone = single_zone(scenario)?;
    let base_scale = zone.demand.annual_scale;
    let base_extra = zone.demand.extra_demand_gw;

    // Load traces once, demand unscaled (module docs), then apply the
    // scenario's own scaling in memory — bit-identical to loading with
    // it applied for scenarios without a heating overlay; ULP-scale
    // rounding may differ on the heating path (module docs).
    let mut neutral = scenario.clone();
    neutral.zones[0].demand.annual_scale = 1.0;
    neutral.zones[0].demand.extra_demand_gw = Power::gigawatts(0.0);
    let raw = load_run_inputs(&neutral, base_dir)?;
    let default_inputs = RunInputs {
        demand: scale_demand(&raw.demand, base_scale, base_extra, raw.heating.as_ref())?,
        ..raw.clone()
    };

    // The grid, row-major (last dimension fastest).
    let grid: Vec<Vec<usize>> = match dimensions {
        [d0] => (0..d0.len()).map(|i| vec![i]).collect(),
        [d0, d1] => (0..d0.len())
            .flat_map(|i| (0..d1.len()).map(move |j| vec![i, j]))
            .collect(),
        _ => unreachable!("dimension count validated above"),
    };

    let evaluate = |indices: &Vec<usize>| -> Result<SweepPoint, GridError> {
        let mut variant = scenario.clone();
        for (dimension, &index) in dimensions.iter().zip(indices) {
            dimension.apply(&mut variant, index);
        }
        // Rebuild the demand trace only when `annual_scale` changed —
        // the only sweepable demand knob (`extra_demand_gw` has no
        // `Dimension` variant); otherwise share the preloaded inputs.
        let variant_scale = variant.zones[0].demand.annual_scale;
        let rebuilt;
        let inputs = if variant_scale == base_scale {
            &default_inputs
        } else {
            rebuilt = RunInputs {
                demand: scale_demand(&raw.demand, variant_scale, base_extra, raw.heating.as_ref())?,
                ..raw.clone()
            };
            &rebuilt
        };
        let result = run(&variant, inputs)?;
        Ok(point_metrics(indices.clone(), &result))
    };

    let points = match execution {
        Execution::Parallel => grid
            .par_iter()
            .map(evaluate)
            .collect::<Result<Vec<_>, _>>()?,
        Execution::Serial => grid.iter().map(evaluate).collect::<Result<Vec<_>, _>>()?,
    };

    Ok(SweepResult {
        dimensions: dimensions.to_vec(),
        points,
    })
}

/// `base × scale + extra` per period — the demand adjustment the input
/// loader applies (`crate::inputs`). Without an overlay the arithmetic
/// is the pre-v5 expression, reproduced bit-identically.
///
/// When the loaded inputs carry a heating overlay (schema v5), the
/// neutral-load demand already INCLUDES the overlay's electrical
/// total, and heating carries its own quantum — it is never subject to
/// `annual_scale` (the loader convention). The overlay is therefore
/// subtracted before scaling and added back after:
/// `(neutral − heating) × scale + extra + heating`. This is NOT
/// bit-identical to the loader's direct `base × scale + extra +
/// heating`: `(base + h) − h` may differ from `base` by IEEE rounding,
/// so a heating-scenario sweep point can differ from a direct run at
/// ULP scale of the neutral load per period (relative ~1e-16) — do not
/// assume shared digests across the two paths (module docs).
fn scale_demand(
    base: &Trace<Power>,
    scale: f64,
    extra: Power,
    heating: Option<&grid_core::heating::HeatingOverlay>,
) -> Result<Trace<Power>, GridError> {
    match heating {
        None => Trace::from_parts(
            base.start(),
            base.values().iter().map(|&p| p * scale + extra).collect(),
        ),
        Some(overlay) => Trace::from_parts(
            base.start(),
            base.values()
                .iter()
                .zip(&overlay.electrical_total)
                .map(|(&p, &h)| (p - h) * scale + extra + h)
                .collect(),
        ),
    }
}

/// Summarise one run into its sweep-point metrics.
fn point_metrics(indices: Vec<usize>, result: &RunResult) -> SweepPoint {
    let zero = Energy::gigawatt_hours(0.0);
    let renewable_potential = result
        .renewables
        .iter()
        .map(|s| RunResult::total_energy(&s.power))
        .fold(zero, |acc, e| acc + e);
    SweepPoint {
        indices,
        demand: result.total_demand_energy(),
        renewable_potential,
        unserved: result.total_unserved(),
        unserved_periods: result
            .unserved
            .iter()
            .filter(|p| p.as_gigawatts() > 0.0)
            .count(),
        curtailment: result.total_curtailment(),
        store_min_soc: result
            .stores
            .iter()
            .map(|s| (s.label.clone(), s.min_soc().map_or(zero, |(_, soc)| soc)))
            .collect(),
    }
}

// ---------------------------------------------------------------------
// Multi-zone wind-capacity sweep (D11 rule 2).
// ---------------------------------------------------------------------

/// One point of the multi-zone wind-capacity sweep: the swept zone's
/// Module 1 metrics with imports **endogenous** (modelled through the
/// scenario's `[[links]]` by the flow rule), replacing the single-zone
/// sweep's frozen-2024-imports convention. Metric definitions match the
/// Module 1 / Package A/B sweep (`grid-cli sweep wind-capacity`)
/// exactly, so the numbers are comparable against the pinned
/// single-zone bracket:
///
/// - `curtailment` — the zone's pooled curtailment (Stage 1
///   convention, no per-source attribution).
/// - `gas` — the zone's `ccgt` + `ocgt` dispatched energy (the Module 1
///   `gas_twh` definition).
/// - `net_imports` — the zone's net import energy; in a multi-zone run
///   this is the sum of the link net positions (positive = import),
///   the endogenous quantity this mode exists to measure.
/// - `gas_price_setting_share` — fraction of periods where a priced
///   (SRMC-modelled) technology sets the zone's SMP, over the zone's
///   `[zones.pricing]` SRMC set ({ccgt, ocgt} = gas on the GB
///   scenarios).
/// - `mean_smp` — time-weighted mean of the zone's SMP under the
///   Stage 2 conventions (`grid_core::pricing::system_marginal_price`
///   over the zone's thermal dispatch: £0 in must-take-only periods,
///   fleet-SRMC ceiling when unserved; renewables, storage and link
///   flows never set the price).
/// - `wind_capture_ratio` / `wind_capture_ratio_delivered` — total
///   (onshore + offshore) wind capture ratio on the potential basis
///   (pooled-curtailment convention) and the delivered basis
///   (pro-rata post-curtailment, `crate::pricing` prose) — the
///   Package A pair.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiZoneWindPoint {
    /// The swept total wind capacity (onshore + offshore).
    pub wind_capacity: Power,
    /// The zone's pooled curtailment energy.
    pub curtailment: Energy,
    /// The zone's gas (`ccgt` + `ocgt`) dispatched energy.
    pub gas: Energy,
    /// The zone's net import energy (positive = import) — endogenous.
    pub net_imports: Energy,
    /// The zone's unserved energy.
    pub unserved: Energy,
    /// Fraction of periods with a priced technology setting the SMP
    /// (dimensionless share, the `price_setting_share` convention).
    pub gas_price_setting_share: f64,
    /// Time-weighted mean SMP.
    pub mean_smp: Price,
    /// Wind capture ratio, potential basis (dimensionless; `None` for
    /// zero wind output or a degenerate all-£0 SMP series, where the
    /// quotient is 0/0).
    pub wind_capture_ratio: Option<f64>,
    /// Wind capture ratio, delivered basis (dimensionless; same `None`
    /// convention).
    pub wind_capture_ratio_delivered: Option<f64>,
}

/// A completed multi-zone wind-capacity sweep: one
/// [`MultiZoneWindPoint`] per requested capacity, in request order
/// (the full response, ADR-10).
#[derive(Debug, Clone, PartialEq)]
pub struct MultiZoneWindSweep {
    /// The swept zone.
    pub zone: ZoneId,
    /// One point per requested capacity, in request order.
    pub points: Vec<MultiZoneWindPoint>,
}

/// Module 1 wind identity (the sweep convention shared with
/// `grid-cli sweep wind-capacity`).
fn is_wind(tech: &str) -> bool {
    matches!(tech, "offshore_wind" | "onshore_wind")
}

/// Scale the named zone's wind fleet to `target` total capacity, the
/// Module 1 convention: onshore and offshore scale PROPORTIONALLY from
/// their reference split, keeping the CF trace shapes; every other
/// fleet entry — and every other zone — is untouched.
fn apply_zone_wind_capacity(
    scenario: &mut Scenario,
    zone_id: &str,
    target: Power,
) -> Result<(), GridError> {
    let zone = scenario
        .zones
        .iter_mut()
        .find(|z| z.id.as_str() == zone_id)
        .ok_or_else(|| GridError::InvalidScenario {
            reason: format!("multi-zone wind sweep: the scenario has no zone {zone_id}"),
        })?;
    let reference: f64 = zone
        .fleet
        .iter()
        .filter(|e| is_wind(e.technology.as_str()))
        .map(|e| e.capacity_gw.as_gigawatts())
        .sum();
    if reference <= 0.0 {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "multi-zone wind sweep: zone {zone_id} has no onshore/offshore wind \
                 capacity to scale"
            ),
        });
    }
    let factor = target.as_gigawatts() / reference;
    for entry in &mut zone.fleet {
        if is_wind(entry.technology.as_str()) {
            entry.capacity_gw = entry.capacity_gw * factor;
        }
    }
    Ok(())
}

/// Run the wind-capacity sweep on the multi-zone engine (D11 rule 2:
/// the tier-2 fix of the frozen-imports-under-sweep deviation).
///
/// # Conventions (stated per the D11 work order)
///
/// 1. **Only the swept zone's wind fleet scales** (proportionally,
///    [`apply_zone_wind_capacity`]). Every EXTERNAL zone's fleet,
///    demand, traces and budgets stay at their committed 2024 basis —
///    external fleets are NOT projected alongside the swept fleet.
///    Their response to the swept capacity is purely operational
///    (the flow rule redispatches their committed fleets).
/// 2. **Imports are endogenous**: the swept zone's `net_imports` is the
///    modelled link position at each capacity, replacing the
///    single-zone sweep's frozen observed-2024 trace. The Package B
///    frozen/zero/export bracket remains the disclosed uncertainty
///    band around this central estimate.
/// 3. **The flow signal is the scenario's** (`dispatch.flow_signal`).
///    The tier-2 CENTRAL estimate runs the committed `scarcity`
///    default — the configuration that passes the Stage 5 A-gates at
///    the 2024 anchor; the priced ladder is a named SENSITIVITY only
///    (the d11-engine-review §G ruling: on 2024 prices its both-gas
///    directions are convention noise).
/// 4. `inputs` are loaded once by the caller
///    ([`crate::inputs::load_multi_zone_inputs`]) and shared across
///    points — capacity scaling enters at dispatch, not at trace
///    loading, so the shared inputs are bit-identical to a per-point
///    reload.
///
/// The swept zone must exist, carry wind, and carry loaded
/// `[zones.pricing]` inputs (its SRMC chain prices the Module 1
/// metrics). Parallel execution is bit-identical to serial (ADR-10;
/// rayon's order-preserving collect).
pub fn wind_capacity_sweep_multi(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    zone_id: &str,
    capacities: &[Power],
    execution: Execution,
) -> Result<MultiZoneWindSweep, GridError> {
    if capacities.is_empty() {
        return Err(GridError::InvalidScenario {
            reason: "multi-zone wind sweep: at least one capacity is required".to_owned(),
        });
    }
    for capacity in capacities {
        let gw = capacity.as_gigawatts();
        if !(gw.is_finite() && gw > 0.0) {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "multi-zone wind sweep: capacities must be finite and positive (got {gw} GW)"
                ),
            });
        }
    }
    // Validate the swept zone once, up front (a clean error before any
    // dispatch): it exists and carries wind…
    {
        let mut probe = scenario.clone();
        apply_zone_wind_capacity(&mut probe, zone_id, capacities[0])?;
    }
    // …and its pricing inputs are loaded (the SRMC chain for the
    // priced Module 1 metrics).
    let zone_pricing = inputs
        .zones
        .iter()
        .find(|z| z.id.as_str() == zone_id)
        .and_then(|z| z.pricing.as_ref())
        .ok_or_else(|| GridError::InvalidRunInputs {
            reason: format!(
                "multi-zone wind sweep: zone {zone_id} has no loaded [zones.pricing] \
                 inputs; the Module 1 metrics need its SRMC chain"
            ),
        })?;

    let evaluate = |&target: &Power| -> Result<MultiZoneWindPoint, GridError> {
        let mut variant = scenario.clone();
        apply_zone_wind_capacity(&mut variant, zone_id, target)?;
        let result = run_multi(&variant, inputs)?;
        let zone_result = result
            .zone(zone_id)
            .ok_or_else(|| GridError::InvalidScenario {
                reason: format!("multi-zone wind sweep: no result for zone {zone_id}"),
            })?;
        multi_zone_point_metrics(target, zone_result, zone_pricing)
    };

    let points = match execution {
        Execution::Parallel => capacities
            .par_iter()
            .map(evaluate)
            .collect::<Result<Vec<_>, _>>()?,
        Execution::Serial => capacities
            .iter()
            .map(evaluate)
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(MultiZoneWindSweep {
        zone: ZoneId::new(zone_id),
        points,
    })
}

/// Summarise the swept zone's result into its Module 1 metrics (the
/// definitions on [`MultiZoneWindPoint`]; the SMP arithmetic is exactly
/// `crate::pricing::price_run`'s series construction, applied to the
/// zone's thermal dispatch with its `[zones.pricing]` SRMC chain).
fn multi_zone_point_metrics(
    wind_capacity: Power,
    result: &RunResult,
    pricing: &ZonePricingInputs,
) -> Result<MultiZoneWindPoint, GridError> {
    let series: Vec<PricedSeries<'_>> = result
        .thermal
        .iter()
        .map(|thermal| PricedSeries {
            tech: thermal.tech.clone(),
            power: &thermal.power,
            srmc: pricing.srmc.get(&thermal.tech).map(|t| t.values()),
        })
        .collect();
    let prices = system_marginal_price(&series, &result.unserved)?;
    let mean_smp =
        time_weighted_mean_price(&prices.smp).ok_or_else(|| GridError::InvalidPricing {
            reason: "cannot price a run with no periods".to_owned(),
        })?;
    let priced_techs: Vec<&str> = pricing.srmc.keys().map(|t| t.as_str()).collect();

    // Total wind on both capture bases (the Package A pair).
    let zero = Power::gigawatts(0.0);
    let mut potential = vec![zero; result.periods()];
    for series in result
        .renewables
        .iter()
        .filter(|s| is_wind(s.tech.as_str()))
    {
        for (acc, &p) in potential.iter_mut().zip(&series.power) {
            *acc = *acc + p;
        }
    }
    let delivered_all = delivered_renewable_power(result)?;
    let mut delivered = vec![zero; result.periods()];
    for (series, delivered_power) in result.renewables.iter().zip(&delivered_all) {
        if !is_wind(series.tech.as_str()) {
            continue;
        }
        for (acc, &p) in delivered.iter_mut().zip(delivered_power) {
            *acc = *acc + p;
        }
    }

    let zero_energy = Energy::gigawatt_hours(0.0);
    let gas = result.thermal_energy("ccgt").unwrap_or(zero_energy)
        + result.thermal_energy("ocgt").unwrap_or(zero_energy);

    // A degenerate all-£0 SMP series (every period must-take-only)
    // makes the capture quotient 0/0; there is no meaningful ratio, so
    // it is reported as None — which also keeps the sweep points
    // NaN-free (PartialEq-comparable, the determinism assertions).
    let finite = |ratio: Option<f64>| ratio.filter(|r| r.is_finite());

    Ok(MultiZoneWindPoint {
        wind_capacity,
        curtailment: result.total_curtailment(),
        gas,
        net_imports: result.net_imports_energy(),
        unserved: result.total_unserved(),
        gas_price_setting_share: price_setting_share(&prices.setter, &priced_techs),
        mean_smp,
        wind_capture_ratio: finite(capture_ratio(&potential, &prices.smp)?),
        wind_capture_ratio_delivered: finite(capture_ratio(&delivered, &prices.smp)?),
    })
}

// ---------------------------------------------------------------------
// Zone-GROUP wind-capacity sweep + GB-aggregate metrics (D13 rules
// 1/5/6) — the small ADDITIVE measurement helpers the composed
// boundary-trade measurement needs: `wind_capacity_sweep_multi` scales
// exactly one named zone and prices one zone's result, but the composed
// scenario scales GB wind across THREE zones (one shared national
// factor, rule 6) and aggregates GB metrics across them (the basis-(A)
// recipe, rule 5). No dispatch-engine change.
// ---------------------------------------------------------------------

/// A completed zone-group wind-capacity sweep: the group's aggregate
/// [`MultiZoneWindPoint`] per requested capacity, in request order.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiZoneGroupWindSweep {
    /// The swept zone group, in the caller's order.
    pub zones: Vec<ZoneId>,
    /// One aggregate point per requested capacity, in request order.
    pub points: Vec<MultiZoneWindPoint>,
}

/// Scale the GROUP's wind fleet to `target` total capacity with **one
/// shared factor** `target ÷ Σ group wind` applied to the onshore and
/// offshore entries of every named zone — the D13 rule-6 convention:
/// the committed zonal splits and each zone's onshore/offshore mix are
/// preserved exactly (a single-zone group degenerates to
/// [`apply_zone_wind_capacity`]). Every other fleet entry — and every
/// zone outside the group — is untouched.
fn apply_zone_group_wind_capacity(
    scenario: &mut Scenario,
    zone_ids: &[&str],
    target: Power,
) -> Result<(), GridError> {
    if zone_ids.is_empty() {
        return Err(GridError::InvalidScenario {
            reason: "zone-group wind sweep: the group names no zones".to_owned(),
        });
    }
    for (index, id) in zone_ids.iter().enumerate() {
        if zone_ids[..index].contains(id) {
            return Err(GridError::InvalidScenario {
                reason: format!("zone-group wind sweep: zone {id} is named more than once"),
            });
        }
        if !scenario.zones.iter().any(|z| z.id.as_str() == *id) {
            return Err(GridError::InvalidScenario {
                reason: format!("zone-group wind sweep: the scenario has no zone {id}"),
            });
        }
    }
    let reference: f64 = scenario
        .zones
        .iter()
        .filter(|z| zone_ids.contains(&z.id.as_str()))
        .flat_map(|z| z.fleet.iter())
        .filter(|e| is_wind(e.technology.as_str()))
        .map(|e| e.capacity_gw.as_gigawatts())
        .sum();
    if reference <= 0.0 {
        return Err(GridError::InvalidScenario {
            reason: format!(
                "zone-group wind sweep: the group {zone_ids:?} has no onshore/offshore \
                 wind capacity to scale"
            ),
        });
    }
    // ONE shared factor — requesting the reference capacity yields
    // factor x/x = 1.0 exactly (IEEE), so the anchor dispatch is
    // bit-identical to the unswept scenario.
    let factor = target.as_gigawatts() / reference;
    for zone in scenario
        .zones
        .iter_mut()
        .filter(|z| zone_ids.contains(&z.id.as_str()))
    {
        for entry in &mut zone.fleet {
            if is_wind(entry.technology.as_str()) {
                entry.capacity_gw = entry.capacity_gw * factor;
            }
        }
    }
    Ok(())
}

/// Run the wind-capacity sweep on a ZONE GROUP (D13 rules 1/5/6): one
/// shared scaling factor across the group's wind fleets
/// ([`apply_zone_group_wind_capacity`]), metrics on the group AGGREGATE
/// under the basis-(A) recipe (rule 5, stated mechanically at
/// [`multi_zone_group_point_metrics`]). External zones stay frozen at
/// their committed basis, exactly as [`wind_capacity_sweep_multi`].
/// Parallel execution is bit-identical to serial (ADR-10).
pub fn wind_capacity_sweep_multi_group(
    scenario: &Scenario,
    inputs: &MultiZoneInputs,
    zone_ids: &[&str],
    capacities: &[Power],
    execution: Execution,
) -> Result<MultiZoneGroupWindSweep, GridError> {
    if capacities.is_empty() {
        return Err(GridError::InvalidScenario {
            reason: "zone-group wind sweep: at least one capacity is required".to_owned(),
        });
    }
    for capacity in capacities {
        let gw = capacity.as_gigawatts();
        if !(gw.is_finite() && gw > 0.0) {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "zone-group wind sweep: capacities must be finite and positive (got {gw} GW)"
                ),
            });
        }
    }
    // Validate the group once, up front (exists, unique, carries wind).
    {
        let mut probe = scenario.clone();
        apply_zone_group_wind_capacity(&mut probe, zone_ids, capacities[0])?;
    }
    // The group's merged SRMC chain (basis (A) is well-defined because
    // the group zones carry the IDENTICAL committed chain — checked, not
    // assumed): union of the group zones' loaded [zones.pricing] series,
    // duplicated technologies required trace-identical.
    let mut srmc: BTreeMap<TechId, &Trace<Price>> = BTreeMap::new();
    for id in zone_ids {
        let Some(pricing) = inputs
            .zones
            .iter()
            .find(|z| z.id.as_str() == *id)
            .and_then(|z| z.pricing.as_ref())
        else {
            continue;
        };
        for (tech, trace) in &pricing.srmc {
            match srmc.get(tech) {
                None => {
                    srmc.insert(tech.clone(), trace);
                }
                Some(existing) if *existing == trace => {}
                Some(_) => {
                    return Err(GridError::InvalidRunInputs {
                        reason: format!(
                            "zone-group wind sweep: zone {id} carries an SRMC series for \
                             {tech} that CONFLICTS with another group zone's — the \
                             basis-(A) aggregate needs one identical chain"
                        ),
                    });
                }
            }
        }
    }
    if srmc.is_empty() {
        return Err(GridError::InvalidRunInputs {
            reason: format!(
                "zone-group wind sweep: no zone of the group {zone_ids:?} carries loaded \
                 [zones.pricing] inputs; the aggregate Module 1 metrics need the group's \
                 SRMC chain"
            ),
        });
    }

    let evaluate = |&target: &Power| -> Result<MultiZoneWindPoint, GridError> {
        let mut variant = scenario.clone();
        apply_zone_group_wind_capacity(&mut variant, zone_ids, target)?;
        let result = run_multi(&variant, inputs)?;
        let mut group_results = Vec::with_capacity(zone_ids.len());
        for id in zone_ids {
            group_results.push(result.zone(id).ok_or_else(|| GridError::InvalidScenario {
                reason: format!("zone-group wind sweep: no result for zone {id}"),
            })?);
        }
        multi_zone_group_point_metrics(target, &group_results, &srmc)
    };

    let points = match execution {
        Execution::Parallel => capacities
            .par_iter()
            .map(evaluate)
            .collect::<Result<Vec<_>, _>>()?,
        Execution::Serial => capacities
            .iter()
            .map(evaluate)
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(MultiZoneGroupWindSweep {
        zones: zone_ids.iter().map(|id| ZoneId::new(*id)).collect(),
        points,
    })
}

/// Summarise a zone GROUP's results into aggregate Module 1 metrics —
/// the D13 basis-(A) recipe, stated mechanically (design rule 5, review
/// edit 9) so the parity claim with the committed single-zone recipe is
/// checkable:
///
/// - **per-technology thermal series SUMMED across the group zones**
///   (well-defined because the group carries one identical SRMC chain),
///   feeding the existing single-zone [`PricedSeries`] construction;
/// - **aggregate unserved = the group sum** — the second argument of
///   [`system_marginal_price`], so the unserved→ceiling convention
///   fires on the aggregate exactly as it did on the single GB zone;
/// - **delivered wind = the per-zone pro-rata
///   [`delivered_renewable_power`], summed**; potential = capacity ×
///   CF, summed (each zone's pre-curtailment renewable series);
/// - capture / mean / setter share = the committed quotient recipes
///   ([`capture_ratio`], [`time_weighted_mean_price`],
///   [`price_setting_share`]) on the aggregate series;
/// - curtailment / gas / net imports / unserved are the plain group
///   sums of the committed per-zone definitions.
fn multi_zone_group_point_metrics(
    wind_capacity: Power,
    results: &[&RunResult],
    srmc: &BTreeMap<TechId, &Trace<Price>>,
) -> Result<MultiZoneWindPoint, GridError> {
    let periods = results.first().map(|r| r.periods()).unwrap_or(0);
    if results.iter().any(|r| r.periods() != periods) {
        return Err(GridError::InvalidRunInputs {
            reason: "zone-group metrics: group results cover different horizons".to_owned(),
        });
    }
    let zero = Power::gigawatts(0.0);

    // Per-technology thermal, summed across the group in
    // first-appearance order (each zone's thermal is merit-ordered, so
    // the aggregate order is deterministic and merit-consistent).
    let mut order: Vec<TechId> = Vec::new();
    let mut summed: BTreeMap<TechId, Vec<Power>> = BTreeMap::new();
    for result in results {
        for thermal in &result.thermal {
            let series = summed.entry(thermal.tech.clone()).or_insert_with(|| {
                order.push(thermal.tech.clone());
                vec![zero; periods]
            });
            for (acc, &p) in series.iter_mut().zip(&thermal.power) {
                *acc = *acc + p;
            }
        }
    }
    let series: Vec<PricedSeries<'_>> = order
        .iter()
        .map(|tech| PricedSeries {
            tech: tech.clone(),
            power: &summed[tech],
            srmc: srmc.get(tech).map(|t| t.values()),
        })
        .collect();

    // Aggregate unserved: the group sum, fed to the SMP convention.
    let mut unserved_sum = vec![zero; periods];
    for result in results {
        for (acc, &p) in unserved_sum.iter_mut().zip(&result.unserved) {
            *acc = *acc + p;
        }
    }
    let prices = system_marginal_price(&series, &unserved_sum)?;
    let mean_smp =
        time_weighted_mean_price(&prices.smp).ok_or_else(|| GridError::InvalidPricing {
            reason: "cannot price a run with no periods".to_owned(),
        })?;
    let priced_techs: Vec<&str> = srmc.keys().map(|t| t.as_str()).collect();

    // Total wind on both capture bases, summed across the group.
    let mut potential = vec![zero; periods];
    let mut delivered = vec![zero; periods];
    for result in results {
        for series in result
            .renewables
            .iter()
            .filter(|s| is_wind(s.tech.as_str()))
        {
            for (acc, &p) in potential.iter_mut().zip(&series.power) {
                *acc = *acc + p;
            }
        }
        let delivered_all = delivered_renewable_power(result)?;
        for (series, delivered_power) in result.renewables.iter().zip(&delivered_all) {
            if !is_wind(series.tech.as_str()) {
                continue;
            }
            for (acc, &p) in delivered.iter_mut().zip(delivered_power) {
                *acc = *acc + p;
            }
        }
    }

    let zero_energy = Energy::gigawatt_hours(0.0);
    let sum_energy = |f: &dyn Fn(&RunResult) -> Energy| -> Energy {
        results.iter().fold(zero_energy, |acc, r| acc + f(r))
    };
    let gas = sum_energy(&|r| {
        r.thermal_energy("ccgt").unwrap_or(zero_energy)
            + r.thermal_energy("ocgt").unwrap_or(zero_energy)
    });

    let finite = |ratio: Option<f64>| ratio.filter(|r| r.is_finite());

    Ok(MultiZoneWindPoint {
        wind_capacity,
        curtailment: sum_energy(&|r| r.total_curtailment()),
        gas,
        net_imports: sum_energy(&|r| r.net_imports_energy()),
        unserved: sum_energy(&|r| r.total_unserved()),
        gas_price_setting_share: price_setting_share(&prices.setter, &priced_techs),
        mean_smp,
        wind_capture_ratio: finite(capture_ratio(&potential, &prices.smp)?),
        wind_capture_ratio_delivered: finite(capture_ratio(&delivered, &prices.smp)?),
    })
}

// ---------------------------------------------------------------------
// Per-year batch mode (Q4).
// ---------------------------------------------------------------------

/// One weather year's solve outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum YearOutcome {
    /// The bisection found a requirement.
    Feasible {
        /// Minimum store energy for zero unserved (the naive solve;
        /// single-year horizons cannot run the D4 burn-in).
        requirement: Energy,
        /// Minimum SoC of the designated store at the requirement.
        min_soc: Energy,
        /// When the minimum first occurs.
        min_soc_at: UtcInstant,
        /// The D4 initial-SoC guard flag. On a single-year horizon the
        /// burn-in re-run is skipped by design, so a flagged year leans
        /// on the initial-full store — report it, never hide it.
        initial_condition_sensitive: bool,
    },
    /// No store size achieves zero unserved (a finding, not an error).
    Infeasible {
        /// The solver's structured reason.
        reason: String,
    },
}

/// One year of the Q4 batch.
#[derive(Debug, Clone, PartialEq)]
pub struct YearRequirement {
    /// The weather year.
    pub year: i32,
    /// Its solve outcome.
    pub outcome: YearOutcome,
}

/// Solve `min_storage_for_zero_unserved` for every weather year in
/// `years` as an independent single-year scenario (module docs: the
/// horizon is clamped to the year and per-year trace files selected by
/// the year in their file name). Results are returned in year order;
/// parallel and serial execution are bit-identical.
pub fn per_year_requirements(
    scenario: &Scenario,
    base_dir: &Path,
    years: RangeInclusive<i32>,
    store_index: usize,
    options: &SolveOptions,
    execution: Execution,
) -> Result<Vec<YearRequirement>, GridError> {
    let years: Vec<i32> = years.collect();
    if years.is_empty() {
        return Err(GridError::InvalidScenario {
            reason: "per-year batch: empty year range".to_owned(),
        });
    }

    let solve_year = |&year: &i32| -> Result<YearRequirement, GridError> {
        let variant = scenario_for_year(scenario, year)?;
        let inputs = load_run_inputs(&variant, base_dir)?;
        let outcome = match min_storage_for_zero_unserved(&variant, &inputs, store_index, options) {
            Ok(result) => YearOutcome::Feasible {
                requirement: result.naive.requirement,
                min_soc: result.min_soc,
                min_soc_at: result.min_soc_at,
                initial_condition_sensitive: result.initial_condition_sensitive,
            },
            Err(GridError::SolveInfeasible { reason }) => YearOutcome::Infeasible { reason },
            Err(other) => return Err(other),
        };
        Ok(YearRequirement { year, outcome })
    };

    match execution {
        Execution::Parallel => years.par_iter().map(solve_year).collect(),
        Execution::Serial => years.iter().map(solve_year).collect(),
    }
}

/// The scenario restricted to one weather year: horizon clamped to the
/// calendar year, every multi-file trace list filtered to the file(s)
/// whose name contains the year.
fn scenario_for_year(scenario: &Scenario, year: i32) -> Result<Scenario, GridError> {
    let mut variant = scenario.clone();
    variant.horizon.start = format!("{year:04}-01-01T00:00:00Z");
    variant.horizon.end = format!("{year:04}-12-31T23:30:00Z");
    variant.horizon.weather_years = WeatherYears::Years(vec![year]);

    let token = format!("{year:04}");
    let filter = |files: &mut TraceFiles, what: &str| -> Result<(), GridError> {
        let kept: Vec<String> = files
            .paths()
            .iter()
            .filter(|p| p.contains(&token))
            .cloned()
            .collect();
        if kept.is_empty() {
            return Err(GridError::InvalidScenario {
                reason: format!(
                    "per-year batch mode needs per-year trace files: no path of {what} names \
                     the year {year} ({files})"
                ),
            });
        }
        *files = TraceFiles::from_paths(kept);
        Ok(())
    };

    let zone = &mut variant.zones[0];
    filter(&mut zone.demand.base_profile, "the demand base_profile")?;
    for entry in &mut zone.fleet {
        if let Some(trace) = &mut entry.capacity_factor_trace {
            filter(trace, &format!("the {} CF trace", entry.technology))?;
        }
    }
    for supply in &mut zone.exogenous_supply {
        filter(
            &mut supply.path,
            &format!("the exogenous supply {:?}", supply.label),
        )?;
    }
    Ok(variant)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use grid_core::scenario::{
        DemandSpec, Dispatch, DispatchPolicyKind, FleetEntry, Horizon, LinkSpec, ZoneId, ZoneSpec,
    };
    use grid_core::units::{PerUnit, Price};

    use crate::inputs::{MultiZoneInputs, ZoneInputs, ZonePricingInputs};

    // -----------------------------------------------------------------
    // Multi-zone wind-capacity sweep (D11 rule 2) — synthetic fixtures,
    // the multizone.rs test idiom.
    // -----------------------------------------------------------------

    const START: &str = "2024-01-01T00:00:00Z";
    const PERIODS: usize = 4;

    fn start() -> UtcInstant {
        UtcInstant::parse(START).unwrap()
    }

    fn thermal_entry(tech: &str, capacity_gw: f64) -> FleetEntry {
        FleetEntry {
            technology: TechId::new(tech),
            capacity_gw: Power::gigawatts(capacity_gw),
            capacity_factor_trace: None,
            availability: None,
            reliability: None,
            inertia_h: None,
            synchronous: None,
            energy_budget: None,
        }
    }

    fn renewable_entry(tech: &str, capacity_gw: f64) -> FleetEntry {
        FleetEntry {
            capacity_factor_trace: Some(format!("synthetic/{tech}.parquet").into()),
            ..thermal_entry(tech, capacity_gw)
        }
    }

    fn synthetic_zone(id: &str, fleet: Vec<FleetEntry>) -> ZoneSpec {
        ZoneSpec {
            pricing: None,
            id: ZoneId::new(id),
            demand: DemandSpec {
                base_profile: "unused-in-synthetic-runs".into(),
                column: "underlying_demand".to_owned(),
                extra_profiles: vec![],
                annual_scale: 1.0,
                extra_demand_gw: Power::gigawatts(0.0),
                heating: None,
            },
            exogenous_supply: vec![],
            fleet,
            storage: vec![],
        }
    }

    /// A two-zone scenario: an importing wind zone ("GB": ccgt 2 GW,
    /// onshore 2 GW + offshore 1 GW, demand 5 GW) linked to a thermal
    /// exporter ("EX": ccgt 10 GW, demand 1 GW), 3 GW lossless link.
    fn two_zone_scenario() -> Scenario {
        Scenario {
            schema_version: 8,
            name: "synthetic-multizone-wind-sweep".to_owned(),
            description: None,
            horizon: Horizon {
                start: START.to_owned(),
                end: start().plus_periods(PERIODS as i64 - 1).to_string(),
                weather_years: WeatherYears::Years(vec![2024]),
            },
            zones: vec![
                synthetic_zone(
                    "GB",
                    vec![
                        thermal_entry("ccgt", 2.0),
                        renewable_entry("onshore_wind", 2.0),
                        renewable_entry("offshore_wind", 1.0),
                    ],
                ),
                synthetic_zone("EX", vec![thermal_entry("ccgt", 10.0)]),
            ],
            links: vec![LinkSpec {
                name: Some("L".to_owned()),
                from: ZoneId::new("GB"),
                to: ZoneId::new("EX"),
                capacity_gw: Power::gigawatts(3.0),
                reverse_capacity_gw: None,
                capability_trace: None,
                availability: PerUnit::new(1.0),
                loss: PerUnit::new(0.0),
            }],
            dispatch: Dispatch {
                flow_signal: Default::default(),
                policy: DispatchPolicyKind::RuleBased,
            },
            constraints: None,
            solver: None,
            pricing: None,
        }
    }

    fn two_zone_inputs(gb_pricing: Option<ZonePricingInputs>) -> MultiZoneInputs {
        let power = |values: &[f64]| {
            Trace::from_parts(
                start(),
                values.iter().map(|&v| Power::gigawatts(v)).collect(),
            )
            .unwrap()
        };
        let cf_one = Trace::from_parts(start(), vec![PerUnit::new(1.0); PERIODS]).unwrap();
        MultiZoneInputs {
            zones: vec![
                ZoneInputs {
                    id: ZoneId::new("GB"),
                    inputs: RunInputs {
                        demand: power(&[5.0; 4]),
                        capacity_factors: [
                            (TechId::new("onshore_wind"), cf_one.clone()),
                            (TechId::new("offshore_wind"), cf_one),
                        ]
                        .into_iter()
                        .collect(),
                        exogenous: vec![],
                        availability: BTreeMap::new(),
                        heating: None,
                    },
                    budgets: BTreeMap::new(),
                    pricing: gb_pricing,
                },
                ZoneInputs {
                    id: ZoneId::new("EX"),
                    inputs: RunInputs {
                        demand: power(&[1.0; 4]),
                        capacity_factors: BTreeMap::new(),
                        exogenous: vec![],
                        availability: BTreeMap::new(),
                        heating: None,
                    },
                    budgets: BTreeMap::new(),
                    pricing: None,
                },
            ],
            link_capabilities: vec![],
        }
    }

    fn gb_pricing() -> ZonePricingInputs {
        ZonePricingInputs {
            srmc: [(
                TechId::new("ccgt"),
                Trace::from_parts(
                    start(),
                    vec![Price::pounds_per_megawatt_hour(50.0); PERIODS],
                )
                .unwrap(),
            )]
            .into_iter()
            .collect(),
        }
    }

    /// The Module 1 scaling convention on the named zone: onshore and
    /// offshore wind scale PROPORTIONALLY from their reference split;
    /// every other fleet entry — and every other zone — is untouched.
    #[test]
    fn apply_zone_wind_capacity_scales_proportionally_in_the_named_zone_only() {
        let mut scenario = two_zone_scenario();
        // Give EX a wind entry to prove other zones stay untouched.
        scenario.zones[1]
            .fleet
            .push(renewable_entry("onshore_wind", 7.0));
        apply_zone_wind_capacity(&mut scenario, "GB", Power::gigawatts(6.0)).unwrap();
        let cap = |zone: usize, tech: &str| -> f64 {
            scenario.zones[zone]
                .fleet
                .iter()
                .find(|e| e.technology.as_str() == tech)
                .unwrap()
                .capacity_gw
                .as_gigawatts()
        };
        assert!((cap(0, "onshore_wind") - 4.0).abs() < 1e-12);
        assert!((cap(0, "offshore_wind") - 2.0).abs() < 1e-12);
        assert!((cap(0, "ccgt") - 2.0).abs() < 1e-12); // non-wind untouched
        assert!((cap(1, "onshore_wind") - 7.0).abs() < 1e-12); // other zone untouched

        // Unknown zone and a windless zone are structured errors.
        let mut scenario = two_zone_scenario();
        assert!(matches!(
            apply_zone_wind_capacity(&mut scenario, "XX", Power::gigawatts(6.0)),
            Err(GridError::InvalidScenario { .. })
        ));
        scenario.zones[1].fleet = vec![thermal_entry("ccgt", 10.0)];
        assert!(matches!(
            apply_zone_wind_capacity(&mut scenario, "EX", Power::gigawatts(6.0)),
            Err(GridError::InvalidScenario { .. })
        ));
    }

    #[test]
    fn multi_zone_wind_sweep_rejects_bad_arguments() {
        let scenario = two_zone_scenario();
        let inputs = two_zone_inputs(Some(gb_pricing()));

        // No capacities.
        assert!(matches!(
            wind_capacity_sweep_multi(&scenario, &inputs, "GB", &[], Execution::Serial),
            Err(GridError::InvalidScenario { .. })
        ));
        // Non-positive capacity.
        assert!(matches!(
            wind_capacity_sweep_multi(
                &scenario,
                &inputs,
                "GB",
                &[Power::gigawatts(0.0)],
                Execution::Serial
            ),
            Err(GridError::InvalidScenario { .. })
        ));
        // Unknown zone.
        assert!(matches!(
            wind_capacity_sweep_multi(
                &scenario,
                &inputs,
                "XX",
                &[Power::gigawatts(6.0)],
                Execution::Serial
            ),
            Err(GridError::InvalidScenario { .. })
        ));
        // A windless zone cannot be swept.
        assert!(matches!(
            wind_capacity_sweep_multi(
                &scenario,
                &inputs,
                "EX",
                &[Power::gigawatts(6.0)],
                Execution::Serial
            ),
            Err(GridError::InvalidScenario { .. })
        ));
        // The swept zone must carry loaded pricing inputs (the priced
        // Module 1 metrics have no basis without its SRMC chain).
        let unpriced = two_zone_inputs(None);
        assert!(matches!(
            wind_capacity_sweep_multi(
                &scenario,
                &unpriced,
                "GB",
                &[Power::gigawatts(6.0)],
                Execution::Serial
            ),
            Err(GridError::InvalidRunInputs { .. })
        ));
    }

    /// The point of the D11 rule-2 mode: imports respond ENDOGENOUSLY
    /// to the swept fleet. At 1 GW wind the zone is short and imports;
    /// at 6 GW it is surplus and exports — the frozen-imports
    /// convention could produce neither response.
    #[test]
    fn multi_zone_wind_sweep_imports_respond_endogenously() {
        let scenario = two_zone_scenario();
        let inputs = two_zone_inputs(Some(gb_pricing()));
        let sweep = wind_capacity_sweep_multi(
            &scenario,
            &inputs,
            "GB",
            &[Power::gigawatts(1.0), Power::gigawatts(6.0)],
            Execution::Serial,
        )
        .unwrap();
        assert_eq!(sweep.points.len(), 2);
        let short = &sweep.points[0];
        let surplus = &sweep.points[1];
        assert!(
            short.net_imports.as_gigawatt_hours() > 0.0,
            "the short zone must import (got {:?})",
            short.net_imports
        );
        assert!(
            surplus.net_imports.as_gigawatt_hours() < 0.0,
            "the surplus zone must export (got {:?})",
            surplus.net_imports
        );
        // Gas dispatch falls with wind; at 1 GW ccgt is marginal in
        // every period, so the priced metrics carry the SRMC.
        assert!(short.gas > surplus.gas);
        assert!(short.gas_price_setting_share > 0.99);
        assert!((short.mean_smp.as_pounds_per_megawatt_hour() - 50.0).abs() < 1e-9);
        // Wind earns the mean price when gas sets it in every period.
        assert!((short.wind_capture_ratio.unwrap() - 1.0).abs() < 1e-9);
        assert!((short.wind_capture_ratio_delivered.unwrap() - 1.0).abs() < 1e-9);
    }

    /// ADR-10/Stage-4 discipline carried over: rayon execution is
    /// bit-identical to serial.
    #[test]
    fn multi_zone_wind_sweep_parallel_matches_serial_bit_for_bit() {
        let scenario = two_zone_scenario();
        let inputs = two_zone_inputs(Some(gb_pricing()));
        let capacities = [
            Power::gigawatts(1.0),
            Power::gigawatts(3.0),
            Power::gigawatts(6.0),
        ];
        let serial =
            wind_capacity_sweep_multi(&scenario, &inputs, "GB", &capacities, Execution::Serial)
                .unwrap();
        let parallel =
            wind_capacity_sweep_multi(&scenario, &inputs, "GB", &capacities, Execution::Parallel)
                .unwrap();
        assert!(serial == parallel, "parallel sweep differs from serial");
    }

    // -----------------------------------------------------------------
    // D13 zone-GROUP sweep + basis-(A) aggregate metrics — red-first
    // hand-computable fixtures (d13-composed-boundary-trade.md rules
    // 1/5/6).
    // -----------------------------------------------------------------

    const GROUP_PERIODS: usize = 3;

    /// Two "GB-like" group zones with no links: N carries wind only
    /// (3 GW, cf [1.0, 0.5, 0.0], demand 1 GW flat — P0 curtails 2 GW,
    /// P1 curtails 0.5 GW, P2 is 1 GW unserved); S carries ccgt 4 GW
    /// against demand 2 GW flat (dispatches 2 GW every period). Only S
    /// carries a pricing chain (the SSCO pattern: a group zone with no
    /// SRMC-bearing plant carries none).
    fn group_scenario() -> Scenario {
        Scenario {
            schema_version: 8,
            name: "synthetic-group-sweep".to_owned(),
            description: None,
            horizon: Horizon {
                start: START.to_owned(),
                end: start().plus_periods(GROUP_PERIODS as i64 - 1).to_string(),
                weather_years: WeatherYears::Years(vec![2024]),
            },
            zones: vec![
                synthetic_zone(
                    "N",
                    vec![
                        renewable_entry("onshore_wind", 2.0),
                        renewable_entry("offshore_wind", 1.0),
                    ],
                ),
                synthetic_zone("S", vec![thermal_entry("ccgt", 4.0)]),
            ],
            links: vec![],
            dispatch: Dispatch {
                flow_signal: Default::default(),
                policy: DispatchPolicyKind::RuleBased,
            },
            constraints: None,
            solver: None,
            pricing: None,
        }
    }

    fn group_srmc(values: &[f64]) -> ZonePricingInputs {
        ZonePricingInputs {
            srmc: [(
                TechId::new("ccgt"),
                Trace::from_parts(
                    start(),
                    values
                        .iter()
                        .map(|&v| Price::pounds_per_megawatt_hour(v))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            )]
            .into_iter()
            .collect(),
        }
    }

    fn group_inputs(
        n_pricing: Option<ZonePricingInputs>,
        s_pricing: Option<ZonePricingInputs>,
    ) -> MultiZoneInputs {
        let power = |values: &[f64]| {
            Trace::from_parts(
                start(),
                values.iter().map(|&v| Power::gigawatts(v)).collect(),
            )
            .unwrap()
        };
        let cf = |values: &[f64]| {
            Trace::from_parts(
                start(),
                values.iter().map(|&v| PerUnit::new(v)).collect::<Vec<_>>(),
            )
            .unwrap()
        };
        MultiZoneInputs {
            zones: vec![
                ZoneInputs {
                    id: ZoneId::new("N"),
                    inputs: RunInputs {
                        demand: power(&[1.0; GROUP_PERIODS]),
                        capacity_factors: [
                            (TechId::new("onshore_wind"), cf(&[1.0, 0.5, 0.0])),
                            (TechId::new("offshore_wind"), cf(&[1.0, 0.5, 0.0])),
                        ]
                        .into_iter()
                        .collect(),
                        exogenous: vec![],
                        availability: BTreeMap::new(),
                        heating: None,
                    },
                    budgets: BTreeMap::new(),
                    pricing: n_pricing,
                },
                ZoneInputs {
                    id: ZoneId::new("S"),
                    inputs: RunInputs {
                        demand: power(&[2.0; GROUP_PERIODS]),
                        capacity_factors: BTreeMap::new(),
                        exogenous: vec![],
                        availability: BTreeMap::new(),
                        heating: None,
                    },
                    budgets: BTreeMap::new(),
                    pricing: s_pricing,
                },
            ],
            link_capabilities: vec![],
        }
    }

    /// The rule-6 convention: ONE shared factor across the whole group,
    /// preserving the zonal split and each zone's onshore/offshore mix.
    #[test]
    fn apply_zone_group_wind_capacity_uses_one_shared_factor() {
        let mut scenario = group_scenario();
        // Give S wind too, and a third zone outside the group.
        scenario.zones[1]
            .fleet
            .push(renewable_entry("onshore_wind", 1.0));
        scenario.zones.push(synthetic_zone(
            "EX",
            vec![renewable_entry("onshore_wind", 7.0)],
        ));
        // Group reference = N 2+1 + S 1 = 4 GW; target 8 → factor 2.
        apply_zone_group_wind_capacity(&mut scenario, &["N", "S"], Power::gigawatts(8.0)).unwrap();
        let cap = |zone: usize, tech: &str| -> f64 {
            scenario.zones[zone]
                .fleet
                .iter()
                .find(|e| e.technology.as_str() == tech)
                .unwrap()
                .capacity_gw
                .as_gigawatts()
        };
        assert!((cap(0, "onshore_wind") - 4.0).abs() < 1e-12);
        assert!((cap(0, "offshore_wind") - 2.0).abs() < 1e-12);
        assert!((cap(1, "onshore_wind") - 2.0).abs() < 1e-12);
        assert!((cap(1, "ccgt") - 4.0).abs() < 1e-12); // non-wind untouched
        assert!((cap(2, "onshore_wind") - 7.0).abs() < 1e-12); // outside the group

        // Structured errors: empty group, unknown zone, duplicate zone,
        // a group with no wind.
        let base = group_scenario();
        for (group, why) in [
            (vec![], "empty group"),
            (vec!["N", "XX"], "unknown zone"),
            (vec!["N", "N"], "duplicate zone"),
            (vec!["S"], "windless group"),
        ] {
            let mut s = base.clone();
            assert!(
                matches!(
                    apply_zone_group_wind_capacity(&mut s, &group, Power::gigawatts(8.0)),
                    Err(GridError::InvalidScenario { .. })
                ),
                "{why} must be a structured error"
            );
        }
    }

    /// The basis-(A) aggregate recipe, hand-computed end to end
    /// (design rule 5, review edit 9). dt = 0.5 h; SRMC [50, 80, 100].
    ///
    /// N: wind 3 GW × cf [1, .5, 0] vs demand 1 → curtails [2, .5, 0],
    ///    P2 unserved 1 GW. S: ccgt runs 2 GW flat.
    /// Aggregate ccgt = [2, 2, 2] (gas 3.0 GWh); aggregate unserved =
    /// [0, 0, 1] so the unserved→ceiling convention fires ON THE
    /// AGGREGATE at P2: SMP = [50, 80, 100], mean 76.667, ccgt sets all
    /// three periods (share 1.0).
    /// Potential wind [3, 1.5, 0]: energy 2.25 GWh, revenue £135k →
    /// capture 60/(230/3) = 18/23. Delivered wind [1, 1, 0]: energy
    /// 1.0 GWh, revenue £65k → capture 65/(230/3) = 19.5/23.
    #[test]
    fn group_sweep_aggregate_metrics_match_the_hand_computation() {
        let scenario = group_scenario();
        let inputs = group_inputs(None, Some(group_srmc(&[50.0, 80.0, 100.0])));
        // Reference group wind = 3 GW; request exactly 3 → factor 1.0.
        let sweep = wind_capacity_sweep_multi_group(
            &scenario,
            &inputs,
            &["N", "S"],
            &[Power::gigawatts(3.0)],
            Execution::Serial,
        )
        .unwrap();
        assert_eq!(
            sweep.zones,
            vec![ZoneId::new("N"), ZoneId::new("S")],
            "the group, in caller order"
        );
        let point = &sweep.points[0];
        assert!((point.wind_capacity.as_gigawatts() - 3.0).abs() < 1e-12);
        assert!((point.gas.as_gigawatt_hours() - 3.0).abs() < 1e-9);
        assert!((point.curtailment.as_gigawatt_hours() - 1.25).abs() < 1e-9);
        assert!((point.unserved.as_gigawatt_hours() - 0.5).abs() < 1e-9);
        assert!((point.net_imports.as_gigawatt_hours() - 0.0).abs() < 1e-9);
        assert!(
            (point.mean_smp.as_pounds_per_megawatt_hour() - 230.0 / 3.0).abs() < 1e-9,
            "mean SMP: {:?}",
            point.mean_smp
        );
        assert!(
            (point.gas_price_setting_share - 1.0).abs() < 1e-12,
            "ccgt sets every period (incl. the aggregate-unserved ceiling period)"
        );
        assert!(
            (point.wind_capture_ratio.unwrap() - 18.0 / 23.0).abs() < 1e-9,
            "potential capture: {:?}",
            point.wind_capture_ratio
        );
        assert!(
            (point.wind_capture_ratio_delivered.unwrap() - 19.5 / 23.0).abs() < 1e-9,
            "delivered capture: {:?}",
            point.wind_capture_ratio_delivered
        );
    }

    /// The group's SRMC chains must be COHERENT (basis (A) is
    /// well-defined "because the three zones carry the identical
    /// committed SRMC chain"): duplicated technologies with identical
    /// traces merge; conflicting traces are a structured error; a group
    /// with no pricing at all is a structured error.
    #[test]
    fn group_sweep_requires_a_coherent_group_srmc_chain() {
        let scenario = group_scenario();
        // No pricing anywhere in the group.
        assert!(matches!(
            wind_capacity_sweep_multi_group(
                &scenario,
                &group_inputs(None, None),
                &["N", "S"],
                &[Power::gigawatts(3.0)],
                Execution::Serial,
            ),
            Err(GridError::InvalidRunInputs { .. })
        ));
        // Identical chains in both zones: fine (they merge).
        let same = group_inputs(
            Some(group_srmc(&[50.0, 80.0, 100.0])),
            Some(group_srmc(&[50.0, 80.0, 100.0])),
        );
        assert!(
            wind_capacity_sweep_multi_group(
                &scenario,
                &same,
                &["N", "S"],
                &[Power::gigawatts(3.0)],
                Execution::Serial,
            )
            .is_ok()
        );
        // Conflicting chains for the same technology: refused.
        let conflicting = group_inputs(
            Some(group_srmc(&[50.0, 80.0, 100.0])),
            Some(group_srmc(&[50.0, 80.0, 999.0])),
        );
        assert!(matches!(
            wind_capacity_sweep_multi_group(
                &scenario,
                &conflicting,
                &["N", "S"],
                &[Power::gigawatts(3.0)],
                Execution::Serial,
            ),
            Err(GridError::InvalidRunInputs { .. })
        ));
    }

    #[test]
    fn group_sweep_parallel_matches_serial_bit_for_bit() {
        let scenario = group_scenario();
        let inputs = group_inputs(None, Some(group_srmc(&[50.0, 80.0, 100.0])));
        let capacities = [
            Power::gigawatts(1.5),
            Power::gigawatts(3.0),
            Power::gigawatts(6.0),
        ];
        let serial = wind_capacity_sweep_multi_group(
            &scenario,
            &inputs,
            &["N", "S"],
            &capacities,
            Execution::Serial,
        )
        .unwrap();
        let parallel = wind_capacity_sweep_multi_group(
            &scenario,
            &inputs,
            &["N", "S"],
            &capacities,
            Execution::Parallel,
        )
        .unwrap();
        assert!(
            serial == parallel,
            "group sweep parallel differs from serial"
        );
    }

    #[test]
    fn linspace_is_inclusive_and_even() {
        assert_eq!(
            linspace(10.0, 60.0, 11).unwrap(),
            (0..11).map(|i| 10.0 + 5.0 * i as f64).collect::<Vec<_>>()
        );
        assert!(linspace(0.0, 1.0, 1).is_err());
        assert!(linspace(f64::NAN, 1.0, 3).is_err());
    }

    #[test]
    fn spec_parses_both_value_forms_and_rejects_unknown_fields() {
        let spec = SweepSpec::from_toml_str(
            r#"
scenario = "scenarios/example.toml"

[[dimensions]]
target = "fleet_scale"
technologies = ["offshore_wind"]
values = [0.8, 1.0]

[[dimensions]]
target = "store_energy"
store_index = 0
range_gwh = { start = 10.0, stop = 20.0, count = 3 }
"#,
        )
        .unwrap();
        assert_eq!(spec.dimensions.len(), 2);

        let err = SweepSpec::from_toml_str("scenario = \"x\"\nsurprise = 1\ndimensions = []")
            .unwrap_err();
        assert!(matches!(err, GridError::ScenarioParse { .. }));
    }

    #[test]
    fn values_or_range_requires_exactly_one() {
        assert!(values_or_range("d", None, None).is_err());
        assert!(values_or_range("d", Some(vec![1.0]), Some((0.0, 1.0, 2))).is_err());
        assert!(values_or_range("d", Some(vec![]), None).is_err());
        assert_eq!(
            values_or_range("d", Some(vec![1.0, 2.0]), None).unwrap(),
            vec![1.0, 2.0]
        );
    }

    /// The heating-aware demand rescale (schema v5): heating carries
    /// its own quantum and is never subject to `annual_scale` — the
    /// rescale is `(neutral − h) × scale + extra + h`, and without an
    /// overlay it is the pre-v5 expression bit-identically.
    #[test]
    fn scale_demand_scales_the_base_but_never_the_heating_overlay() {
        use grid_core::heating::{HeatingCopReference, compute_overlay};
        use grid_core::scenario::{HeatingEntry, HeatingKind, HeatingSpec, TraceRefSpec};
        use grid_core::time::UtcInstant;
        use grid_core::units::{PerUnit, Temperature};

        // A synthetic constant year at 10.5 °C and an all-ASHP spec.
        let start = UtcInstant::parse("2023-01-01T00:00:00Z").unwrap();
        let t_pop = Trace::from_parts(start, vec![Temperature::celsius(10.5); 17_520]).unwrap();
        let reference = HeatingCopReference::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(grid_core::heating::HEATING_COP_REFERENCE_PATH),
        )
        .unwrap();
        let spec = HeatingSpec {
            delivered_heat_twh: Energy::gigawatt_hours(410_500.0),
            electrified_share: PerUnit::new(0.5),
            dhw_fraction: PerUnit::new(0.170),
            temperature_trace: TraceRefSpec {
                path: "unused".to_owned(),
                column: "unused".to_owned(),
            },
            entries: vec![HeatingEntry {
                kind: HeatingKind::Ashp,
                share: PerUnit::new(1.0),
                cop_curve: None,
                correction_factor: None,
                rhpp_derating: None,
                cop_const: None,
                resource_depth_m: None,
            }],
        };
        let overlay = compute_overlay(&spec, &reference, &t_pop, start, 48).unwrap();

        // Neutral demand = base (at scale 1, extra 0) + heating.
        let base_gw = 20.0;
        let neutral = Trace::from_parts(
            start,
            overlay
                .electrical_total
                .iter()
                .map(|&h| Power::gigawatts(base_gw) + h)
                .collect::<Vec<_>>(),
        )
        .unwrap();

        let scaled = scale_demand(&neutral, 2.0, Power::gigawatts(0.5), Some(&overlay)).unwrap();
        for (t, &value) in scaled.values().iter().enumerate() {
            let expected = Power::gigawatts(base_gw * 2.0 + 0.5) + overlay.electrical_total[t];
            assert_eq!(value, expected, "period {t}");
        }

        // Without an overlay: the pre-v5 arithmetic, bit-identically.
        let plain = scale_demand(&neutral, 2.0, Power::gigawatts(0.5), None).unwrap();
        for (t, &value) in plain.values().iter().enumerate() {
            assert_eq!(
                value,
                neutral.values()[t] * 2.0 + Power::gigawatts(0.5),
                "period {t}"
            );
        }
    }

    #[test]
    fn scenario_for_year_filters_trace_lists_and_clamps_the_horizon() {
        let mut scenario = minimal_scenario();
        scenario.zones[0].demand.base_profile = TraceFiles::from_paths(vec![
            "demand_1985.parquet".to_owned(),
            "demand_1986.parquet".to_owned(),
        ]);
        let variant = scenario_for_year(&scenario, 1986).unwrap();
        assert_eq!(variant.horizon.start, "1986-01-01T00:00:00Z");
        assert_eq!(variant.horizon.end, "1986-12-31T23:30:00Z");
        assert_eq!(
            variant.zones[0].demand.base_profile.paths(),
            ["demand_1986.parquet"]
        );
        // A trace with no per-year files is refused with a clear error.
        let err = scenario_for_year(&scenario, 1999).unwrap_err();
        assert!(matches!(err, GridError::InvalidScenario { .. }));
    }

    fn minimal_scenario() -> Scenario {
        Scenario::from_toml_str(
            r#"
schema_version = 8
name = "minimal"

[horizon]
start = "1985-01-01T00:00:00Z"
end = "1986-12-31T23:30:00Z"
weather_years = "all"

[[zones]]
id = "GB"

[zones.demand]
base_profile = "demand.parquet"
annual_scale = 1.0

[dispatch]
policy = "rule_based"
"#,
        )
        .unwrap()
    }
}
