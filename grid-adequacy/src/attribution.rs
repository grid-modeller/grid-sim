//! Storage attribution by timescale band — the machinery behind the
//! Module 3(c) decomposition chart, the project's single most
//! persuasive artefact (docs/07), and the subject of kill criterion 2
//! (docs/08).
//!
//! # Method (normative)
//!
//! The zone's residual load `r = demand − must-take` is decomposed into
//! diurnal / synoptic / seasonal / inter-annual bands by successive
//! differences of moving averages (`grid_core::analysis::decompose`;
//! bands sum back to `r` by construction). The storage requirement is
//! attributed to the bands **by telescoping the requirement across the
//! smoothing cascade**:
//!
//! ```text
//! M(x) = the min store energy for zero unserved when the store faces
//!        the series x alone (bisection, crate::solve, naive figure)
//!
//! total          = M(r)
//! A(diurnal)     = M(r)  − M(ℓ₁)     ℓ₁ = MA(r, diurnal window)
//! A(synoptic)    = M(ℓ₁) − M(ℓ₂)     ℓ₂ = MA(r, synoptic window)
//! A(seasonal)    = M(ℓ₂) − M(ℓ₃)     ℓ₃ = MA(r, seasonal window)
//! A(inter-annual)= M(ℓ₃)
//! ```
//!
//! Each band's attribution is *the storage the requirement drops by
//! when variation faster than that band's cutoff is smoothed away* —
//! answering exactly the Module 3 question ("what drives the
//! requirement — daily cycling or inter-annual drought?"). The four
//! attributions **sum to the total identically by telescoping**; no
//! tolerance is consumed by the sum itself (only f64 re-association,
//! a few ulps).
//!
//! # `M(x)`: the bare-residual replay
//!
//! `M` is evaluated by the same bisection solver that produced the
//! Stage 3 pinned requirements, on a synthetic variant of the scenario:
//! the fleet and exogenous supply are removed and the series `x` is fed
//! as the demand trace. The dispatch engine computes
//! `net = must-take − demand = −x`, so the store faces surpluses and
//! deficits identical (bit-for-bit) to the real run's — negative
//! residual periods become charging surplus, positive become deficit —
//! with the same power rating, efficiency split, initial-SoC rule and
//! greedy policy. For the unfiltered residual this **replays the real
//! scenario's solve exactly** (asserted by the Stage 4 acceptance
//! suite), which is what lets the attribution total be compared to the
//! Stage 3 pinned number within the bisection's own convergence
//! tolerance (`max(0.1 GWh, 1e-3 × requirement)`), the only tolerance
//! the method needs.
//!
//! # Scope (stated, enforced)
//!
//! Attribution is defined for fleets where storage faces the raw
//! residual: every fleet entry weather-driven (must-take), no
//! dispatchable stack — the Royal-Society-style wind+solar+storage
//! scenarios. With a thermal stack the store sees the *post-stack*
//! deficit, a different series, and the decomposition-of-residual
//! attribution would not reconcile with the scenario's requirement;
//! such scenarios are rejected with a structured error rather than
//! silently mis-attributed. The designated store must also be the
//! zone's only store (the Stage 3 RS scenarios' shape); portfolio
//! attribution is future work.
//!
//! # Kill-criterion-2 posture
//!
//! The construction makes "bands sum to total" exact, so the criterion
//! lives entirely in the *stability* of the attribution under window
//! perturbation. Because every smoothing level averages the ORIGINAL
//! series, perturbing one window can only move the two bands it bounds:
//! the total, the diurnal band, the inter-annual band, and the
//! synoptic+seasonal aggregate are window-invariant BY CONSTRUCTION,
//! and the Stage 4 acceptance suite gates them at f64 dust (1e-9 GWh).
//! The adjacent synoptic↔seasonal trade — the band *definition* moving
//! with its boundary — is gated at 10 % of total, a regression guard
//! set at 1.5× the measured worst (6.57 % at a 21 d window), not an
//! independently derived threshold (evidence pinned in the test and in
//! docs/notes/stage-4-decomposition-run-report.md). Publication rule:
//! the synoptic-vs-seasonal ranking is window-sensitive and must never
//! be quoted without stating the window. If the invariant gates ever
//! fail, the decomposition is not publishable; report it.

use std::collections::BTreeMap;

use grid_core::GridError;
use grid_core::analysis::{DecompositionWindows, decompose, residual_load};
use grid_core::scenario::Scenario;
use grid_core::trace::Trace;
use grid_core::units::{Energy, Power};

use crate::dispatch::run;
use crate::inputs::RunInputs;
use crate::solve::{SolveOptions, min_storage_for_zero_unserved};

/// The four timescale bands, slowest-cutoff last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Variation faster than the diurnal window (within-day cycling —
    /// the band the "few days of storage" claim answers).
    Diurnal,
    /// Diurnal-to-synoptic variation (weather-system passage,
    /// Dunkelflaute onset).
    Synoptic,
    /// Synoptic-to-seasonal variation (the annual cycle).
    Seasonal,
    /// Slower than seasonal (year-to-year weather, multi-year
    /// droughts), including the mean.
    InterAnnual,
}

impl Band {
    /// Stable lower-case name for outputs.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Diurnal => "diurnal",
            Self::Synoptic => "synoptic",
            Self::Seasonal => "seasonal",
            Self::InterAnnual => "inter_annual",
        }
    }
}

/// One band's share of the storage requirement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandAttribution {
    /// The band.
    pub band: Band,
    /// The requirement attributed to it (module docs: the telescoping
    /// difference of level requirements). Can in principle be negative
    /// if smoothing *raises* the requirement — never observed; a
    /// negative value would itself be a reportable anomaly.
    pub requirement: Energy,
}

/// The complete attribution: total, per-level requirements, and the
/// four band attributions (which sum to the total by telescoping).
#[derive(Debug, Clone, PartialEq)]
pub struct StorageAttribution {
    /// The windows used.
    pub windows: DecompositionWindows,
    /// `M(r)`: the requirement on the unfiltered residual — the total
    /// being attributed.
    pub total: Energy,
    /// `[M(r), M(ℓ₁), M(ℓ₂), M(ℓ₃)]`: the requirement at each
    /// smoothing level.
    pub level_requirements: [Energy; 4],
    /// The band attributions, `[diurnal, synoptic, seasonal,
    /// inter_annual]`.
    pub bands: [BandAttribution; 4],
}

/// Attribute the scenario's storage requirement to the four timescale
/// bands (module docs: telescoping bisection requirements across the
/// smoothing cascade; scope restrictions enforced here).
pub fn attribute_storage_by_band(
    scenario: &Scenario,
    inputs: &RunInputs,
    store_index: usize,
    windows: &DecompositionWindows,
    options: &SolveOptions,
) -> Result<StorageAttribution, GridError> {
    scenario.validate()?;
    let zone = crate::inputs::single_zone(scenario)?;
    if let Some(entry) = zone
        .fleet
        .iter()
        .find(|e| e.capacity_factor_trace.is_none())
    {
        return Err(GridError::UnsupportedFeature {
            feature: format!(
                "storage attribution on a fleet with a dispatchable stack ({}): storage \
                 would face the post-stack deficit, not the residual load, and the \
                 decomposition-of-residual attribution would not reconcile — attribution \
                 is defined for all-must-take (wind+solar+storage) fleets",
                entry.technology
            ),
        });
    }
    if zone.storage.len() != 1 || store_index != 0 {
        return Err(GridError::UnsupportedFeature {
            feature: format!(
                "storage attribution on a portfolio ({} stores, designated index \
                 {store_index}): attribution is defined for single-store scenarios",
                zone.storage.len()
            ),
        });
    }

    // The residual, in the engine's own arithmetic order (so the
    // replay below is bit-identical to the real run).
    let result = run(scenario, inputs)?;
    let must_take: Vec<&[Power]> = result
        .renewables
        .iter()
        .map(|s| s.power.as_slice())
        .chain(result.exogenous.iter().map(|s| s.power.as_slice()))
        .collect();
    let residual = residual_load(&result.demand, &must_take)?;
    let decomposition = decompose(&residual, windows)?;

    // The synthetic bare-residual scenario: store only, series as
    // demand (module docs).
    let mut synthetic = scenario.clone();
    {
        let zone = &mut synthetic.zones[0];
        zone.fleet.clear();
        zone.exogenous_supply.clear();
        // The series below are prebuilt traces; neutralise the loader
        // knobs so nothing double-scales if these inputs are ever
        // reloaded from the scenario.
        zone.demand.annual_scale = 1.0;
        zone.demand.extra_demand_gw = Power::gigawatts(0.0);
    }
    synthetic.pricing = None;

    let start = result.start;
    let requirement_of = |series: &[Power]| -> Result<Energy, GridError> {
        let synthetic_inputs = RunInputs {
            demand: Trace::from_parts(start, series.to_vec())?,
            capacity_factors: BTreeMap::new(),
            exogenous: Vec::new(),
            availability: BTreeMap::new(),
            heating: None,
        };
        let solved = min_storage_for_zero_unserved(&synthetic, &synthetic_inputs, 0, options)?;
        Ok(solved.naive.requirement)
    };

    let m_r = requirement_of(&residual)?;
    let m_l1 = requirement_of(&decomposition.levels[0])?;
    let m_l2 = requirement_of(&decomposition.levels[1])?;
    let m_l3 = requirement_of(&decomposition.levels[2])?;

    Ok(StorageAttribution {
        windows: *windows,
        total: m_r,
        level_requirements: [m_r, m_l1, m_l2, m_l3],
        bands: [
            BandAttribution {
                band: Band::Diurnal,
                requirement: m_r - m_l1,
            },
            BandAttribution {
                band: Band::Synoptic,
                requirement: m_l1 - m_l2,
            },
            BandAttribution {
                band: Band::Seasonal,
                requirement: m_l2 - m_l3,
            },
            BandAttribution {
                band: Band::InterAnnual,
                requirement: m_l3,
            },
        ],
    })
}
