//! Stability metadata defaults (Stage 6, ADR-9: inertia constant H and
//! `synchronous` per technology live in grid-core so `grid-stability`
//! consumes adequacy output directly).
//!
//! Every value here transcribes the committed, per-number-cited
//! evidence file `data/reference/inertia-constants.toml` (assembled
//! 2026-07-03; ERCOT 2018 registration data, Fernández-Guillamón et
//! al. 2019 survey, Kraljič 2022 GB reconstruction — full citations in
//! the file). A characterisation test
//! (`grid-core/tests/inertia_defaults.rs`) fails if this module and the
//! file drift apart. Scenario authors can override any entry per fleet
//! entry (`inertia_h`, `synchronous` — schema v3); overrides are
//! surfaced in run outputs, exactly like the reliability field.
//!
//! ## The MW→MVA convention (decided at Stage 6 design, as the evidence
//! file requires)
//!
//! H is quoted on the machine **MVA** base; scenario capacities and
//! dispatch are real **GW**. The adopted convention is a single
//! documented fleet power factor, [`DEFAULT_POWER_FACTOR`] = 0.9
//! (inertia-constants.toml CONVENTIONS: "typically 0.85–0.95 for large
//! sets"; 0.9 is the centre of that band), applied at the one
//! conversion point `Power::apparent`. So the Stage 6 inertia sum is
//!
//! `E = Σ H_i × (dispatched GW_i / 0.9)` over synchronised plant.
//!
//! A per-entry power-factor override was considered and rejected for
//! v1: no public GB per-unit MVA register exists to populate it (the
//! evidence file's NOT FOUND note), so it would be a free parameter
//! pretending to be data. Revisit if unit-level data ever lands.
//!
//! ## Non-synchronous defaults
//!
//! Wind, solar, batteries and HVDC interconnectors are
//! inverter/HVDC-coupled: physical rotor energy (where any exists) is
//! decoupled, and synthetic inertia is a *control service*, never an H
//! (the file's per-entry notes; Hornsea One's 9 Aug 2019 behaviour is
//! the case study). Unknown technology ids also default to
//! non-synchronous with no H — the honest default for plant this model
//! knows nothing about; scenarios claiming otherwise must say so
//! explicitly.

use crate::scenario::{StorageKind, TechId};
use crate::units::{InertiaConstant, PerUnit};

/// The documented fleet power factor for the MW→MVA conversion:
/// MVA = MW / 0.9 (see the module docs for the adoption rationale and
/// citation).
pub const DEFAULT_POWER_FACTOR: PerUnit = PerUnit::new(0.9);

/// A technology's (or storage kind's) default stability metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertiaDefault {
    /// Default inertia constant, machine-MVA base; `None` for
    /// non-synchronous plant (docs/03: `None` if non-sync).
    pub h: Option<InertiaConstant>,
    /// Whether the plant is synchronously coupled by default.
    pub synchronous: bool,
}

impl InertiaDefault {
    const fn synchronous(h_seconds: f64) -> Self {
        Self {
            h: Some(InertiaConstant::seconds(h_seconds)),
            synchronous: true,
        }
    }

    const fn non_synchronous() -> Self {
        Self {
            h: None,
            synchronous: false,
        }
    }
}

/// The default stability metadata of a fleet technology id
/// (transcribed from `data/reference/inertia-constants.toml`; see the
/// module docs). Unknown ids are non-synchronous with no H.
#[must_use]
pub fn technology_default(tech: &TechId) -> InertiaDefault {
    match tech.as_str() {
        // [FG19] Table 1; [ERCOT18] Table 1; [KRA22] GB thermal 3–10 s.
        "ccgt" => InertiaDefault::synchronous(5.0),
        // [ERCOT18] combustion turbines 1–12.5 s; low confidence.
        "ocgt" => InertiaDefault::synchronous(4.0),
        // [ERCOT18] 3.8–4.34 s; GB AGR 2-pole band + Sizewell B.
        "nuclear" => InertiaDefault::synchronous(4.5),
        // [ERCOT18] coal 2.9–4.5 s; closed GB fleet, historical runs.
        "coal" => InertiaDefault::synchronous(4.0),
        // Converted coal steam sets (Drax, Lynemouth) — same machines.
        "biomass" => InertiaDefault::synchronous(4.0),
        // Kundur/[FG19] hydro 2–4 s; GB small hydro sits above the band
        // ([KRA22]), so 3.0 is conservative.
        "hydro" => InertiaDefault::synchronous(3.0),
        // Inverter-coupled ([ERCOT18]: wind/solar H = 0); HVDC links
        // transfer no inertia from the neighbouring synchronous area.
        "onshore_wind" | "offshore_wind" | "solar" | "interconnector" => {
            InertiaDefault::non_synchronous()
        }
        _ => InertiaDefault::non_synchronous(),
    }
}

/// The default stability metadata of a storage kind (storage is not a
/// fleet technology in this schema, so its mapping lives here rather
/// than on a scenario field).
///
/// - Pumped storage: synchronous, H = 4.5 s ([KRA22]/Stability
///   Pathfinder ⇒ ~4.8 s derived; Dinorwig-class 500 rpm sets sit
///   above the generic hydro band). It contributes inertia in **both**
///   pumping and generating modes while synchronised — a dispatch-keyed
///   inertia sum must include pumping hours.
/// - Battery: inverter-coupled, zero inertia — but the premier
///   fast-response provider (472 MW of battery response on 9 Aug
///   2019); model it as a response service, never as H.
/// - Hydrogen: **non-synchronous by v1 modelling choice.** The
///   reconversion technology is unspecified in the schema (fuel cells
///   are inverter-coupled; hydrogen turbines would be synchronous), so
///   the v1 default claims nothing — which makes the Royal-Society
///   finding ("an all-variable fleet has zero system inertia without
///   synthetic provision") an honest output rather than an assumption
///   smuggled in. A hydrogen-turbine scenario should model the turbines
///   as a fleet entry with an explicit H.
/// - DSR: demand, no machine.
#[must_use]
pub fn storage_kind_default(kind: StorageKind) -> InertiaDefault {
    match kind {
        StorageKind::PumpedHydro => InertiaDefault::synchronous(4.5),
        StorageKind::Battery | StorageKind::Hydrogen | StorageKind::Dsr => {
            InertiaDefault::non_synchronous()
        }
    }
}
