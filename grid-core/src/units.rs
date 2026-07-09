//! Newtype unit system (ADR-4).
//!
//! Physical quantities are newtype wrappers over `f64` with **only
//! physically meaningful arithmetic** implemented. GW/GWh confusion is the
//! classic energy-model failure mode; the type system eliminates it. Raw
//! `f64` for a physical quantity never crosses a public API boundary — the
//! named constructors and accessors here (`Power::gigawatts`,
//! `Energy::as_gigawatt_hours`, …) are the single, explicitly-labelled
//! conversion point.
//!
//! Cross-unit arithmetic implemented in Stage 0:
//!
//! - [`Power`] × [`Duration`] = [`Energy`] (and commuted)
//! - [`Energy`] ÷ [`Duration`] = [`Power`]
//! - [`Energy`] ÷ [`Power`] = [`Duration`]
//! - [`PerUnit`] × [`Power`] = [`Power`], [`PerUnit`] × [`Energy`] =
//!   [`Energy`] (and commuted) — availability, efficiency, capacity factors
//!
//! Stage 6 stability arithmetic (ADR-2/ADR-9):
//!
//! - [`InertiaConstant`] × [`ApparentPower`] = [`Inertia`] (and
//!   commuted) — H (s, machine-MVA base) × synchronised GVA = stored
//!   kinetic energy in GVA·s
//! - [`Power::apparent`] — the single MW→MVA conversion point (divide
//!   by a power factor)
//! - [`Frequency`] ÷ [`Duration`] = [`Rocof`], [`Rocof`] × [`Duration`]
//!   = [`Frequency`] (a frequency *delta*; the Hz-per-second ↔ hours
//!   factor is applied here, at the single defined conversion point)
//!
//! Every unit supports same-type `+`, `-`, unary `-`, and scaling by a
//! dimensionless `f64`. Dimensionally invalid operations do not compile:
//!
//! ```compile_fail
//! use grid_core::units::Power;
//! // Power × Power has no meaning in this model.
//! let _ = Power::gigawatts(2.0) * Power::gigawatts(3.0);
//! ```
//!
//! ```compile_fail
//! use grid_core::units::{Energy, Power};
//! // Adding energy to power is dimensionally invalid.
//! let _ = Power::gigawatts(2.0) + Energy::gigawatt_hours(1.0);
//! ```
//!
//! ```compile_fail
//! use grid_core::units::{Duration, Energy};
//! // Energy × Duration (GWh·h) has no meaning in this model.
//! let _ = Energy::gigawatt_hours(1.0) * Duration::half_hour();
//! ```
//!
//! ```compile_fail
//! use grid_core::units::Power;
//! // A raw f64 is not a Power; there is no implicit conversion.
//! fn takes_power(p: Power) {}
//! takes_power(30.0);
//! ```
//!
//! ```compile_fail
//! use grid_core::units::{InertiaConstant, Power};
//! // H is quoted per MVA of machine rating: multiplying it by a real
//! // Power (GW) skips the power-factor conversion and understates the
//! // kinetic energy — convert with `Power::apparent` first.
//! let _ = InertiaConstant::seconds(4.5) * Power::gigawatts(5.0);
//! ```

use serde::{Deserialize, Serialize};

/// Crate-internal escape hatch between a unit newtype and its raw `f64`
/// representation, for trace loading and statistics. Deliberately not
/// public: outside this crate the named constructors/accessors are the
/// only conversion points (ADR-4).
pub(crate) trait UnitScalar: Copy {
    /// Wrap a raw value expressed in the type's canonical unit.
    fn from_raw(value: f64) -> Self;
    /// The raw value in the type's canonical unit.
    fn raw(self) -> f64;
}

/// Declares a unit newtype: docs, canonical constructor/accessor, the
/// always-valid same-type and dimensionless-scalar arithmetic, and the
/// crate-internal [`UnitScalar`] impl.
macro_rules! unit {
    (
        $(#[$meta:meta])*
        $name:ident, $ctor:ident, $accessor:ident,
        ctor_doc: $ctor_doc:literal,
        accessor_doc: $accessor_doc:literal
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(f64);

        impl $name {
            #[doc = $ctor_doc]
            #[must_use]
            pub const fn $ctor(value: f64) -> Self {
                Self(value)
            }

            #[doc = $accessor_doc]
            #[must_use]
            pub const fn $accessor(self) -> f64 {
                self.0
            }
        }

        impl UnitScalar for $name {
            fn from_raw(value: f64) -> Self {
                Self(value)
            }
            fn raw(self) -> f64 {
                self.0
            }
        }

        impl core::ops::Add for $name {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl core::ops::Neg for $name {
            type Output = Self;
            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        // Scaling by a dimensionless factor. The `f64` here is a pure
        // number, not a physical quantity, so this does not breach ADR-4.
        impl core::ops::Mul<f64> for $name {
            type Output = Self;
            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }

        impl core::ops::Mul<$name> for f64 {
            type Output = $name;
            fn mul(self, rhs: $name) -> $name {
                $name(self * rhs.0)
            }
        }

        impl core::ops::Div<f64> for $name {
            type Output = Self;
            fn div(self, rhs: f64) -> Self {
                Self(self.0 / rhs)
            }
        }
    };
}

unit!(
    /// Electrical power in gigawatts (GW).
    Power, gigawatts, as_gigawatts,
    ctor_doc: "Power from a value in gigawatts.",
    accessor_doc: "The power in gigawatts."
);

unit!(
    /// Electrical energy in gigawatt-hours (GWh).
    Energy, gigawatt_hours, as_gigawatt_hours,
    ctor_doc: "Energy from a value in gigawatt-hours.",
    accessor_doc: "The energy in gigawatt-hours."
);

unit!(
    /// Price in pounds sterling per megawatt-hour (£/MWh).
    Price, pounds_per_megawatt_hour, as_pounds_per_megawatt_hour,
    ctor_doc: "Price from a value in £/MWh.",
    accessor_doc: "The price in £/MWh."
);

unit!(
    /// System inertia in gigavolt-ampere seconds (GVA·s).
    Inertia, gigavolt_ampere_seconds, as_gigavolt_ampere_seconds,
    ctor_doc: "Inertia from a value in GVA·s.",
    accessor_doc: "The inertia in GVA·s."
);

unit!(
    /// Carbon emissions in tonnes of CO₂ (tCO2).
    Emissions, tonnes_co2, as_tonnes_co2,
    ctor_doc: "Emissions from a value in tonnes of CO₂.",
    accessor_doc: "The emissions in tonnes of CO₂."
);

unit!(
    /// An inertia constant H in seconds (equivalently GVA·s per GVA of
    /// machine rating) — stored kinetic energy at rated speed divided by
    /// the machine's MVA rating, the universal convention
    /// (`data/reference/inertia-constants.toml`). Quoted on the machine
    /// **MVA** base: multiply by an [`ApparentPower`], never a [`Power`]
    /// (Stage 6, ADR-9).
    InertiaConstant, seconds, as_seconds,
    ctor_doc: "Inertia constant from a value in seconds (GVA·s per GVA).",
    accessor_doc: "The inertia constant in seconds (GVA·s per GVA)."
);

unit!(
    /// Apparent power in gigavolt-amperes (GVA) — the synchronised
    /// machine-rating base on which inertia constants are quoted.
    /// Distinct from [`Power`] (GW, real power): the two are related
    /// only through a power factor ([`Power::apparent`]).
    ApparentPower, gigavolt_amperes, as_gigavolt_amperes,
    ctor_doc: "Apparent power from a value in gigavolt-amperes.",
    accessor_doc: "The apparent power in gigavolt-amperes."
);

unit!(
    /// System frequency (or a frequency deviation) in hertz (Hz).
    Frequency, hertz, as_hertz,
    ctor_doc: "Frequency from a value in hertz.",
    accessor_doc: "The frequency in hertz."
);

unit!(
    /// Rate of change of frequency (RoCoF) in hertz per second (Hz/s).
    Rocof, hertz_per_second, as_hertz_per_second,
    ctor_doc: "RoCoF from a value in hertz per second.",
    accessor_doc: "The RoCoF in hertz per second."
);

unit!(
    /// A load-damping coefficient in percent of demand per hertz of
    /// frequency deviation (%/Hz) — the frequency dependence of
    /// aggregate load (Stage 6 swing model; literature span 1–2.5 %/Hz,
    /// `docs/notes/stage-6-evidence-report.md` §3.5).
    Damping, percent_of_demand_per_hertz, as_percent_of_demand_per_hertz,
    ctor_doc: "Damping from a value in percent of demand per hertz.",
    accessor_doc: "The damping in percent of demand per hertz."
);

unit!(
    /// A carbon price in pounds sterling per tonne of CO₂ (£/tCO2) —
    /// UKA allowance prices and the Carbon Price Support rate (Stage 2,
    /// ADR-9). Distinct from [`Price`] (£/MWh): the two are related only
    /// through an [`EmissionsRate`].
    CarbonPrice, pounds_per_tonne_co2, as_pounds_per_tonne_co2,
    ctor_doc: "Carbon price from a value in £/tCO2.",
    accessor_doc: "The carbon price in £/tCO2."
);

unit!(
    /// An emissions intensity in tonnes of CO₂ per megawatt-hour
    /// (tCO2/MWh). Whether the megawatt-hour is thermal (fuel burned,
    /// HHV) or electric (generation) is a documented property of each
    /// value, converted between bases by dividing by an efficiency
    /// [`PerUnit`]; likewise CO₂-only vs CO₂e is carried in the
    /// surrounding field names (the type records the dimension, not the
    /// accounting basis).
    EmissionsRate, tonnes_per_megawatt_hour, as_tonnes_per_megawatt_hour,
    ctor_doc: "Emissions rate from a value in tCO2/MWh.",
    accessor_doc: "The emissions rate in tCO2/MWh."
);

unit!(
    /// An amount of money in pounds sterling (£) — revenue accounting
    /// (Stage 2, ADR-9). `Energy × Price = Money`; `Money / Energy` is
    /// the capture-price operation.
    Money, pounds, as_pounds,
    ctor_doc: "Money from a value in pounds sterling.",
    accessor_doc: "The amount in pounds sterling."
);

unit!(
    /// A dimensionless per-unit quantity: availability, round-trip
    /// efficiency, capacity factor. Typically in `0.0..=1.0`; range is not
    /// enforced by the type (a capacity factor trace is validated at load,
    /// a scenario field at scenario validation).
    PerUnit, new, value,
    ctor_doc: "Per-unit value from a dimensionless fraction.",
    accessor_doc: "The dimensionless fraction."
);

// ---------------------------------------------------------------------
// Stage 7 cost units (ADR-9, D8): overnight capex, annualised costs and
// their conversions to money over a run.
// ---------------------------------------------------------------------

unit!(
    /// An overnight capital cost per unit of power capacity, in pounds
    /// sterling per kilowatt (£/kW) — the DESNZ Electricity Generation
    /// Costs publication convention carried by
    /// `data/reference/costs-gb.toml` (Stage 7, D8 rule 4).
    CapacityCost, pounds_per_kilowatt, as_pounds_per_kilowatt,
    ctor_doc: "Capacity cost from a value in £/kW.",
    accessor_doc: "The capacity cost in £/kW."
);

unit!(
    /// An overnight capital cost per unit of energy capacity, in pounds
    /// sterling per kilowatt-hour (£/kWh) — the storage energy leg of
    /// the D8 rule-1.3 power/energy split.
    EnergyCapacityCost, pounds_per_kilowatt_hour, as_pounds_per_kilowatt_hour,
    ctor_doc: "Energy-capacity cost from a value in £/kWh.",
    accessor_doc: "The energy-capacity cost in £/kWh."
);

unit!(
    /// An annual cost per unit of power capacity, in pounds sterling
    /// per megawatt per year (£/MW/yr) — fixed O&M rates and rule-4
    /// capex annuities.
    AnnualCapacityCost, pounds_per_megawatt_year, as_pounds_per_megawatt_year,
    ctor_doc: "Annual capacity cost from a value in £/MW/yr.",
    accessor_doc: "The annual capacity cost in £/MW/yr."
);

unit!(
    /// An annual cost per unit of energy capacity, in pounds sterling
    /// per kilowatt-hour per year (£/kWh/yr) — the annuitised storage
    /// energy leg.
    AnnualEnergyCapacityCost, pounds_per_kilowatt_hour_year,
    as_pounds_per_kilowatt_hour_year,
    ctor_doc: "Annual energy-capacity cost from a value in £/kWh/yr.",
    accessor_doc: "The annual energy-capacity cost in £/kWh/yr."
);

unit!(
    /// A money flow in pounds sterling per year (£/yr) — annualised
    /// system-cost lines before they are accrued over a run horizon
    /// ([`MoneyRate::over_years`]).
    MoneyRate, pounds_per_year, as_pounds_per_year,
    ctor_doc: "Money rate from a value in £/yr.",
    accessor_doc: "The money rate in £/yr."
);

unit!(
    /// A cost per unit of mass in pounds sterling per kilogram (£/kg) —
    /// the levelised hydrogen-storage figure of
    /// `data/reference/costs-gb.toml` (its per-MWh companion is a
    /// [`Price`]).
    CostPerMass, pounds_per_kilogram, as_pounds_per_kilogram,
    ctor_doc: "Cost per mass from a value in £/kg.",
    accessor_doc: "The cost in £/kg."
);

unit!(
    /// A length in kilometres (km) — interconnector route lengths in
    /// the costs reference.
    Length, kilometres, as_kilometres,
    ctor_doc: "Length from a value in kilometres.",
    accessor_doc: "The length in kilometres."
);

// ---------------------------------------------------------------------
// Q5 heating-overlay units (D9 rules 3–4): temperatures, ground thermal
// diffusivity, and the pinned heat-intensity coefficient k.
// ---------------------------------------------------------------------

unit!(
    /// A temperature in degrees Celsius (°C) — air/ground temperatures
    /// and sink temperatures of the heating overlay (D9 rule 4). A
    /// difference of two `Temperature`s is a temperature difference in
    /// kelvin (numerically identical to °C differences), which is what
    /// the COP curves consume; whether a value is an absolute
    /// temperature or a difference is carried in the surrounding field
    /// names.
    Temperature, celsius, as_celsius,
    ctor_doc: "Temperature from a value in degrees Celsius.",
    accessor_doc: "The temperature in degrees Celsius."
);

unit!(
    /// A soil thermal diffusivity in square metres per second (m²/s) —
    /// the Kusuda–Achenbach ground-model α of
    /// `data/reference/heating-cop.toml` (D9 rule 4).
    Diffusivity, square_metres_per_second, as_square_metres_per_second,
    ctor_doc: "Diffusivity from a value in m²/s.",
    accessor_doc: "The diffusivity in m²/s."
);

unit!(
    /// The heating overlay's pinned space-heat intensity coefficient k
    /// in gigawatts of delivered heat per kelvin of heat need
    /// (GW/K = GWh per °C·h) — D9 rule 3:
    /// `k = electrified space-heat quantum ÷ mean annual degree-hours`.
    /// `HeatIntensity × Temperature` (the heat-need degree value) is a
    /// [`Power`] of delivered heat.
    HeatIntensity, gigawatts_per_kelvin, as_gigawatts_per_kelvin,
    ctor_doc: "Heat intensity from a value in GW per kelvin of heat need.",
    accessor_doc: "The heat intensity in GW per kelvin."
);

unit!(
    /// An integral of heat need over time in degree-hours (°C·h) — the
    /// UK degree-day convention's native quantity and the denominator
    /// of the heating overlay's intensity k (D9 rule 3).
    /// `Temperature × Duration = DegreeHours`;
    /// `Energy ÷ DegreeHours = HeatIntensity`.
    DegreeHours, celsius_hours, as_celsius_hours,
    ctor_doc: "Degree-hours from a value in °C·h.",
    accessor_doc: "The integral in °C·h."
);

unit!(
    /// A vertical temperature gradient in °C per kilometre of depth —
    /// the D16 geothermal gradient of
    /// `data/reference/heating-cop.toml` `[geothermal]`.
    /// `TemperatureGradient × Length` is the mean-source warming at a
    /// resource depth (a temperature DIFFERENCE, applied from the
    /// committed shallow `loop_depth_m` datum).
    TemperatureGradient, celsius_per_kilometre, as_celsius_per_kilometre,
    ctor_doc: "Temperature gradient from a value in °C/km.",
    accessor_doc: "The gradient in °C/km."
);

unit!(
    /// A span of model time in hours, suited to half-hourly settlement
    /// periods (ADR-3). Distinct from `std::time::Duration` (wall-clock),
    /// which library crates never read.
    Duration, hours, as_hours,
    ctor_doc: "Duration from a value in hours.",
    accessor_doc: "The duration in hours."
);

impl Duration {
    /// One half-hourly settlement period (ADR-3), the engine's native
    /// timestep.
    ///
    /// ```
    /// use grid_core::units::{Duration, Energy, Power};
    /// assert_eq!(Power::gigawatts(2.0) * Duration::half_hour(),
    ///            Energy::gigawatt_hours(1.0));
    /// ```
    #[must_use]
    pub const fn half_hour() -> Self {
        Self::hours(0.5)
    }
}

impl Power {
    /// Power from a value in megawatts (converted to the canonical GW).
    /// Data-pack demand traces are published in MW.
    #[must_use]
    pub const fn megawatts(value: f64) -> Self {
        Self::gigawatts(value / 1000.0)
    }

    /// The apparent power (GVA) behind this real power at the given
    /// power factor — the single MW→MVA conversion point (ADR-4).
    /// Inertia constants are quoted per MVA of machine rating, while
    /// scenario capacities and dispatch are real GW; the Stage 6
    /// convention (`grid_core::inertia::DEFAULT_POWER_FACTOR`) is
    /// pf = 0.9. The caller guarantees a nonzero power factor (scenario
    /// validation rejects pf outside (0, 1]).
    #[must_use]
    pub fn apparent(self, power_factor: PerUnit) -> ApparentPower {
        ApparentPower::gigavolt_amperes(self.as_gigawatts() / power_factor.value())
    }
}

impl Duration {
    /// Duration from a value in seconds (converted to the canonical
    /// hours) — stability-event timescales (Stage 6).
    #[must_use]
    pub const fn from_seconds(value: f64) -> Self {
        Self::hours(value / 3600.0)
    }

    /// The duration in seconds.
    #[must_use]
    pub const fn as_seconds(self) -> f64 {
        self.as_hours() * 3600.0
    }
}

impl Length {
    /// Length from a value in metres (converted to the canonical km) —
    /// the heating ground-model loop depth is quoted in metres.
    #[must_use]
    pub const fn metres(value: f64) -> Self {
        Self::kilometres(value / 1000.0)
    }

    /// The length in metres.
    #[must_use]
    pub const fn as_metres(self) -> f64 {
        self.as_kilometres() * 1000.0
    }
}

/// GW/K × K (of heat need) = GW of delivered heat — the D9 rule-3
/// half-hourly heat computation `k · heat_need(t)`.
impl core::ops::Mul<Temperature> for HeatIntensity {
    type Output = Power;
    fn mul(self, rhs: Temperature) -> Power {
        Power::gigawatts(self.as_gigawatts_per_kelvin() * rhs.as_celsius())
    }
}

/// K × GW/K = GW (commuted).
impl core::ops::Mul<HeatIntensity> for Temperature {
    type Output = Power;
    fn mul(self, rhs: HeatIntensity) -> Power {
        rhs * self
    }
}

/// °C (of heat need) × h = °C·h — degree-hour accumulation, the single
/// conversion point for the D9 rule-3 record integral.
impl core::ops::Mul<Duration> for Temperature {
    type Output = DegreeHours;
    fn mul(self, rhs: Duration) -> DegreeHours {
        DegreeHours::celsius_hours(self.as_celsius() * rhs.as_hours())
    }
}

/// h × °C = °C·h (commuted).
impl core::ops::Mul<Temperature> for Duration {
    type Output = DegreeHours;
    fn mul(self, rhs: Temperature) -> DegreeHours {
        rhs * self
    }
}

/// GWh ÷ °C·h = GW/K — the D9 rule-3 intensity definition
/// `k = space-heat quantum ÷ mean annual degree-hours` (the caller
/// guarantees non-zero degree-hours; the overlay validates it).
impl core::ops::Div<DegreeHours> for Energy {
    type Output = HeatIntensity;
    fn div(self, rhs: DegreeHours) -> HeatIntensity {
        HeatIntensity::gigawatts_per_kelvin(self.as_gigawatt_hours() / rhs.as_celsius_hours())
    }
}

/// GW × h = GWh.
impl core::ops::Mul<Duration> for Power {
    type Output = Energy;
    fn mul(self, rhs: Duration) -> Energy {
        Energy::gigawatt_hours(self.as_gigawatts() * rhs.as_hours())
    }
}

/// h × GW = GWh.
impl core::ops::Mul<Power> for Duration {
    type Output = Energy;
    fn mul(self, rhs: Power) -> Energy {
        rhs * self
    }
}

/// GWh ÷ h = GW.
impl core::ops::Div<Duration> for Energy {
    type Output = Power;
    fn div(self, rhs: Duration) -> Power {
        Power::gigawatts(self.as_gigawatt_hours() / rhs.as_hours())
    }
}

/// GWh ÷ GW = h.
impl core::ops::Div<Power> for Energy {
    type Output = Duration;
    fn div(self, rhs: Power) -> Duration {
        Duration::hours(self.as_gigawatt_hours() / rhs.as_gigawatts())
    }
}

/// Derating / efficiency applied to power.
impl core::ops::Mul<PerUnit> for Power {
    type Output = Power;
    fn mul(self, rhs: PerUnit) -> Power {
        Power::gigawatts(self.as_gigawatts() * rhs.value())
    }
}

/// Derating / efficiency applied to power (commuted).
impl core::ops::Mul<Power> for PerUnit {
    type Output = Power;
    fn mul(self, rhs: Power) -> Power {
        rhs * self
    }
}

/// Efficiency applied to energy.
impl core::ops::Mul<PerUnit> for Energy {
    type Output = Energy;
    fn mul(self, rhs: PerUnit) -> Energy {
        Energy::gigawatt_hours(self.as_gigawatt_hours() * rhs.value())
    }
}

/// Efficiency applied to energy (commuted).
impl core::ops::Mul<Energy> for PerUnit {
    type Output = Energy;
    fn mul(self, rhs: Energy) -> Energy {
        rhs * self
    }
}

// ---------------------------------------------------------------------
// Stage 6 stability arithmetic (ADR-2/ADR-9): system inertia is
// Σ(H × MVA) over synchronised plant; RoCoF relates frequency deltas to
// event time.
// ---------------------------------------------------------------------

/// s × GVA = GVA·s — one synchronised machine's (or aggregated
/// technology tranche's) stored kinetic energy.
impl core::ops::Mul<ApparentPower> for InertiaConstant {
    type Output = Inertia;
    fn mul(self, rhs: ApparentPower) -> Inertia {
        Inertia::gigavolt_ampere_seconds(self.as_seconds() * rhs.as_gigavolt_amperes())
    }
}

/// GVA × s = GVA·s (commuted).
impl core::ops::Mul<InertiaConstant> for ApparentPower {
    type Output = Inertia;
    fn mul(self, rhs: InertiaConstant) -> Inertia {
        rhs * self
    }
}

/// Hz ÷ duration = Hz/s — a frequency delta over an interval is a mean
/// RoCoF (the hours→seconds factor is applied here).
impl core::ops::Div<Duration> for Frequency {
    type Output = Rocof;
    fn div(self, rhs: Duration) -> Rocof {
        Rocof::hertz_per_second(self.as_hertz() / rhs.as_seconds())
    }
}

/// Hz/s × duration = Hz — the frequency delta a RoCoF accumulates over
/// an interval.
impl core::ops::Mul<Duration> for Rocof {
    type Output = Frequency;
    fn mul(self, rhs: Duration) -> Frequency {
        Frequency::hertz(self.as_hertz_per_second() * rhs.as_seconds())
    }
}

/// duration × Hz/s = Hz (commuted).
impl core::ops::Mul<Rocof> for Duration {
    type Output = Frequency;
    fn mul(self, rhs: Rocof) -> Frequency {
        rhs * self
    }
}

/// %/Hz × Hz = a per-unit fraction of demand — the load-damping relief
/// at a frequency deviation (the percent→fraction factor is applied
/// here, at the single defined conversion point).
impl core::ops::Mul<Frequency> for Damping {
    type Output = PerUnit;
    fn mul(self, rhs: Frequency) -> PerUnit {
        PerUnit::new(self.as_percent_of_demand_per_hertz() / 100.0 * rhs.as_hertz())
    }
}

/// Hz × %/Hz = a per-unit fraction of demand (commuted).
impl core::ops::Mul<Damping> for Frequency {
    type Output = PerUnit;
    fn mul(self, rhs: Damping) -> PerUnit {
        rhs * self
    }
}

// ---------------------------------------------------------------------
// Stage 2 pricing arithmetic (ADR-9). The SRMC chain is
// `fuel_price / η + (ef / η) × carbon_price`, all £/MWh-electric; revenue
// accounting is `energy × price`. MWh↔GWh factors are applied here, at
// the single defined conversion point for each operation.
// ---------------------------------------------------------------------

/// £/MWh ÷ efficiency = £/MWh — converts a thermal-basis fuel price to an
/// electric-basis cost (the caller guarantees a non-zero efficiency; the
/// pricing layer validates efficiencies into `(0, 1]`).
impl core::ops::Div<PerUnit> for Price {
    type Output = Price;
    fn div(self, rhs: PerUnit) -> Price {
        Price::pounds_per_megawatt_hour(self.as_pounds_per_megawatt_hour() / rhs.value())
    }
}

/// tCO2/MWh ÷ efficiency = tCO2/MWh — converts a thermal-basis emissions
/// factor to an electric-basis intensity.
impl core::ops::Div<PerUnit> for EmissionsRate {
    type Output = EmissionsRate;
    fn div(self, rhs: PerUnit) -> EmissionsRate {
        EmissionsRate::tonnes_per_megawatt_hour(self.as_tonnes_per_megawatt_hour() / rhs.value())
    }
}

/// tCO2/MWh × £/tCO2 = £/MWh — the carbon term of an SRMC.
impl core::ops::Mul<CarbonPrice> for EmissionsRate {
    type Output = Price;
    fn mul(self, rhs: CarbonPrice) -> Price {
        Price::pounds_per_megawatt_hour(
            self.as_tonnes_per_megawatt_hour() * rhs.as_pounds_per_tonne_co2(),
        )
    }
}

/// £/tCO2 × tCO2/MWh = £/MWh (commuted).
impl core::ops::Mul<EmissionsRate> for CarbonPrice {
    type Output = Price;
    fn mul(self, rhs: EmissionsRate) -> Price {
        rhs * self
    }
}

/// tCO2/MWh × GWh = tCO2 (1 GWh = 1000 MWh).
impl core::ops::Mul<Energy> for EmissionsRate {
    type Output = Emissions;
    fn mul(self, rhs: Energy) -> Emissions {
        Emissions::tonnes_co2(self.as_tonnes_per_megawatt_hour() * rhs.as_gigawatt_hours() * 1000.0)
    }
}

/// GWh × tCO2/MWh = tCO2 (commuted).
impl core::ops::Mul<EmissionsRate> for Energy {
    type Output = Emissions;
    fn mul(self, rhs: EmissionsRate) -> Emissions {
        rhs * self
    }
}

/// GWh × £/MWh = £ (1 GWh = 1000 MWh) — per-technology revenue.
impl core::ops::Mul<Price> for Energy {
    type Output = Money;
    fn mul(self, rhs: Price) -> Money {
        Money::pounds(self.as_gigawatt_hours() * 1000.0 * rhs.as_pounds_per_megawatt_hour())
    }
}

/// £/MWh × GWh = £ (commuted).
impl core::ops::Mul<Energy> for Price {
    type Output = Money;
    fn mul(self, rhs: Energy) -> Money {
        rhs * self
    }
}

/// £ ÷ GWh = £/MWh — the capture-price operation (the caller guarantees
/// non-zero energy; the pricing layer returns `None` for zero output).
impl core::ops::Div<Energy> for Money {
    type Output = Price;
    fn div(self, rhs: Energy) -> Price {
        Price::pounds_per_megawatt_hour(self.as_pounds() / (rhs.as_gigawatt_hours() * 1000.0))
    }
}

// ---------------------------------------------------------------------
// Stage 7 cost arithmetic (ADR-9, D8 rules 1/4). The kW↔GW, kWh↔GWh and
// MW↔GW factors are applied here, at the single defined conversion
// point for each operation.
// ---------------------------------------------------------------------

/// £/kW × GW = £ (1 GW = 10⁶ kW) — total overnight capex of a fleet.
impl core::ops::Mul<Power> for CapacityCost {
    type Output = Money;
    fn mul(self, rhs: Power) -> Money {
        Money::pounds(self.as_pounds_per_kilowatt() * rhs.as_gigawatts() * 1.0e6)
    }
}

/// GW × £/kW = £ (commuted).
impl core::ops::Mul<CapacityCost> for Power {
    type Output = Money;
    fn mul(self, rhs: CapacityCost) -> Money {
        rhs * self
    }
}

/// £/kWh × GWh = £ (1 GWh = 10⁶ kWh) — total capex of a storage energy
/// leg.
impl core::ops::Mul<Energy> for EnergyCapacityCost {
    type Output = Money;
    fn mul(self, rhs: Energy) -> Money {
        Money::pounds(self.as_pounds_per_kilowatt_hour() * rhs.as_gigawatt_hours() * 1.0e6)
    }
}

/// GWh × £/kWh = £ (commuted).
impl core::ops::Mul<EnergyCapacityCost> for Energy {
    type Output = Money;
    fn mul(self, rhs: EnergyCapacityCost) -> Money {
        rhs * self
    }
}

/// £/MW/yr × GW = £/yr (1 GW = 10³ MW) — a fleet's annual fixed cost or
/// capex annuity.
impl core::ops::Mul<Power> for AnnualCapacityCost {
    type Output = MoneyRate;
    fn mul(self, rhs: Power) -> MoneyRate {
        MoneyRate::pounds_per_year(self.as_pounds_per_megawatt_year() * rhs.as_gigawatts() * 1.0e3)
    }
}

/// GW × £/MW/yr = £/yr (commuted).
impl core::ops::Mul<AnnualCapacityCost> for Power {
    type Output = MoneyRate;
    fn mul(self, rhs: AnnualCapacityCost) -> MoneyRate {
        rhs * self
    }
}

/// £/kWh/yr × GWh = £/yr (1 GWh = 10⁶ kWh) — a storage energy leg's
/// annuity.
impl core::ops::Mul<Energy> for AnnualEnergyCapacityCost {
    type Output = MoneyRate;
    fn mul(self, rhs: Energy) -> MoneyRate {
        MoneyRate::pounds_per_year(
            self.as_pounds_per_kilowatt_hour_year() * rhs.as_gigawatt_hours() * 1.0e6,
        )
    }
}

/// GWh × £/kWh/yr = £/yr (commuted).
impl core::ops::Mul<AnnualEnergyCapacityCost> for Energy {
    type Output = MoneyRate;
    fn mul(self, rhs: AnnualEnergyCapacityCost) -> MoneyRate {
        rhs * self
    }
}

impl MoneyRate {
    /// The money this rate accrues over a span of years — the single
    /// £/yr → £ conversion point. `years` is a dimensionless count of
    /// calendar years (fractional for partial years); calendar years
    /// vary in length, so a year count is not a [`Duration`] in this
    /// unit system and the caller states its convention (the cost stack
    /// pro-rates by calendar-year coverage).
    #[must_use]
    pub fn over_years(self, years: f64) -> Money {
        Money::pounds(self.as_pounds_per_year() * years)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // Stage 0 acceptance test (docs/04): Power(2 GW) over half an hour is
    // 1 GWh.
    #[test]
    fn power_times_duration_is_energy() {
        assert_eq!(
            Power::gigawatts(2.0) * Duration::half_hour(),
            Energy::gigawatt_hours(1.0)
        );
        // Commutes.
        assert_eq!(
            Duration::half_hour() * Power::gigawatts(2.0),
            Energy::gigawatt_hours(1.0)
        );
    }

    #[test]
    fn energy_divided_by_duration_is_power() {
        assert_eq!(
            Energy::gigawatt_hours(1.0) / Duration::half_hour(),
            Power::gigawatts(2.0)
        );
    }

    #[test]
    fn energy_divided_by_power_is_duration() {
        assert_eq!(
            Energy::gigawatt_hours(6.0) / Power::gigawatts(2.0),
            Duration::hours(3.0)
        );
    }

    #[test]
    fn per_unit_scales_power_and_energy() {
        assert_eq!(
            Power::gigawatts(10.0) * PerUnit::new(0.5),
            Power::gigawatts(5.0)
        );
        assert_eq!(
            PerUnit::new(0.5) * Power::gigawatts(10.0),
            Power::gigawatts(5.0)
        );
        assert_eq!(
            Energy::gigawatt_hours(40.0) * PerUnit::new(0.88),
            Energy::gigawatt_hours(35.2)
        );
    }

    #[test]
    fn same_unit_add_sub_neg() {
        assert_eq!(
            Power::gigawatts(1.5) + Power::gigawatts(0.5),
            Power::gigawatts(2.0)
        );
        assert_eq!(
            Power::gigawatts(1.5) - Power::gigawatts(0.5),
            Power::gigawatts(1.0)
        );
        assert_eq!(-Power::gigawatts(1.5), Power::gigawatts(-1.5));
        assert_eq!(
            Energy::gigawatt_hours(1.0) + Energy::gigawatt_hours(2.0),
            Energy::gigawatt_hours(3.0)
        );
    }

    #[test]
    fn dimensionless_scalar_scaling() {
        assert_eq!(Power::gigawatts(2.0) * 3.0, Power::gigawatts(6.0));
        assert_eq!(3.0 * Power::gigawatts(2.0), Power::gigawatts(6.0));
        assert_eq!(Power::gigawatts(6.0) / 3.0, Power::gigawatts(2.0));
    }

    #[test]
    fn megawatt_constructor_converts_to_gigawatts() {
        assert_eq!(Power::megawatts(1500.0), Power::gigawatts(1.5));
        assert_eq!(Power::megawatts(1500.0).as_gigawatts(), 1.5);
    }

    // -----------------------------------------------------------------
    // Stage 2 pricing arithmetic (ADR-9): SRMC chain and revenue
    // accounting.
    // -----------------------------------------------------------------

    #[test]
    fn price_divided_by_efficiency_converts_thermal_to_electric_basis() {
        // £30/MWh_th at 50 % efficiency costs £60/MWh_e.
        assert_eq!(
            Price::pounds_per_megawatt_hour(30.0) / PerUnit::new(0.5),
            Price::pounds_per_megawatt_hour(60.0)
        );
    }

    #[test]
    fn emissions_rate_divided_by_efficiency_converts_thermal_to_electric_basis() {
        // 0.2 tCO2/MWh_th at 50 % efficiency emits 0.4 tCO2/MWh_e.
        assert_eq!(
            EmissionsRate::tonnes_per_megawatt_hour(0.2) / PerUnit::new(0.5),
            EmissionsRate::tonnes_per_megawatt_hour(0.4)
        );
    }

    #[test]
    fn emissions_rate_times_carbon_price_is_a_price() {
        // 0.2 tCO2/MWh × £50/tCO2 = £10/MWh.
        assert_eq!(
            EmissionsRate::tonnes_per_megawatt_hour(0.2) * CarbonPrice::pounds_per_tonne_co2(50.0),
            Price::pounds_per_megawatt_hour(10.0)
        );
        assert_eq!(
            CarbonPrice::pounds_per_tonne_co2(50.0) * EmissionsRate::tonnes_per_megawatt_hour(0.2),
            Price::pounds_per_megawatt_hour(10.0)
        );
    }

    #[test]
    fn emissions_rate_times_energy_is_emissions() {
        // 0.4 tCO2/MWh × 2 GWh (= 2000 MWh) = 800 tCO2.
        assert_eq!(
            EmissionsRate::tonnes_per_megawatt_hour(0.4) * Energy::gigawatt_hours(2.0),
            Emissions::tonnes_co2(800.0)
        );
        assert_eq!(
            Energy::gigawatt_hours(2.0) * EmissionsRate::tonnes_per_megawatt_hour(0.4),
            Emissions::tonnes_co2(800.0)
        );
    }

    #[test]
    fn energy_times_price_is_money() {
        // 2 GWh (= 2000 MWh) × £70/MWh = £140,000.
        assert_eq!(
            Energy::gigawatt_hours(2.0) * Price::pounds_per_megawatt_hour(70.0),
            Money::pounds(140_000.0)
        );
        assert_eq!(
            Price::pounds_per_megawatt_hour(70.0) * Energy::gigawatt_hours(2.0),
            Money::pounds(140_000.0)
        );
    }

    #[test]
    fn money_divided_by_energy_is_a_price() {
        // £140,000 over 2 GWh (= 2000 MWh) is £70/MWh — the capture-price
        // operation.
        assert_eq!(
            Money::pounds(140_000.0) / Energy::gigawatt_hours(2.0),
            Price::pounds_per_megawatt_hour(70.0)
        );
    }

    // -----------------------------------------------------------------
    // Stage 6 stability arithmetic (ADR-2/ADR-9).
    // -----------------------------------------------------------------

    #[test]
    fn inertia_constant_times_apparent_power_is_inertia() {
        // H = 4.5 s on a 6 GVA synchronised base stores 27 GVA·s.
        assert_eq!(
            InertiaConstant::seconds(4.5) * ApparentPower::gigavolt_amperes(6.0),
            Inertia::gigavolt_ampere_seconds(27.0)
        );
        assert_eq!(
            ApparentPower::gigavolt_amperes(6.0) * InertiaConstant::seconds(4.5),
            Inertia::gigavolt_ampere_seconds(27.0)
        );
    }

    #[test]
    fn power_apparent_divides_by_the_power_factor() {
        // 4.5 GW real at pf 0.9 is 5 GVA apparent — the MW→MVA
        // conversion the Stage 6 inertia sum uses.
        assert_eq!(
            Power::gigawatts(4.5).apparent(PerUnit::new(0.9)),
            ApparentPower::gigavolt_amperes(5.0)
        );
    }

    #[test]
    fn frequency_delta_over_duration_is_rocof() {
        // 0.288 Hz over 2 s is 0.144 Hz/s.
        assert_eq!(
            Frequency::hertz(0.288) / Duration::from_seconds(2.0),
            Rocof::hertz_per_second(0.144)
        );
        assert_eq!(
            Rocof::hertz_per_second(0.144) * Duration::from_seconds(2.0),
            Frequency::hertz(0.288)
        );
        assert_eq!(
            Duration::from_seconds(2.0) * Rocof::hertz_per_second(0.144),
            Frequency::hertz(0.288)
        );
    }

    #[test]
    fn duration_second_conversions_round_trip() {
        assert_eq!(Duration::from_seconds(1800.0), Duration::half_hour());
        assert_eq!(Duration::half_hour().as_seconds(), 1800.0);
    }

    #[test]
    fn damping_times_frequency_deviation_is_a_demand_fraction() {
        // 2 %/Hz at a 0.5 Hz deviation relieves 1 % of demand.
        assert_eq!(
            Damping::percent_of_demand_per_hertz(2.0) * Frequency::hertz(0.5),
            PerUnit::new(0.01)
        );
        assert_eq!(
            Frequency::hertz(0.5) * Damping::percent_of_demand_per_hertz(2.0),
            PerUnit::new(0.01)
        );
    }

    // -----------------------------------------------------------------
    // Stage 7 cost arithmetic (ADR-9, D8).
    // -----------------------------------------------------------------

    #[test]
    fn capacity_cost_times_power_is_money() {
        // £1,020/kW × 2 GW (= 2×10⁶ kW) = £2.04bn.
        assert_eq!(
            CapacityCost::pounds_per_kilowatt(1020.0) * Power::gigawatts(2.0),
            Money::pounds(2.04e9)
        );
        assert_eq!(
            Power::gigawatts(2.0) * CapacityCost::pounds_per_kilowatt(1020.0),
            Money::pounds(2.04e9)
        );
    }

    #[test]
    fn energy_capacity_cost_times_energy_is_money() {
        // £135/kWh × 4 GWh (= 4×10⁶ kWh) = £540m.
        assert_eq!(
            EnergyCapacityCost::pounds_per_kilowatt_hour(135.0) * Energy::gigawatt_hours(4.0),
            Money::pounds(5.4e8)
        );
        assert_eq!(
            Energy::gigawatt_hours(4.0) * EnergyCapacityCost::pounds_per_kilowatt_hour(135.0),
            Money::pounds(5.4e8)
        );
    }

    #[test]
    fn annual_capacity_cost_times_power_is_a_money_rate() {
        // £16,000/MW/yr × 2 GW (= 2,000 MW) = £32m/yr.
        assert_eq!(
            AnnualCapacityCost::pounds_per_megawatt_year(16_000.0) * Power::gigawatts(2.0),
            MoneyRate::pounds_per_year(3.2e7)
        );
        assert_eq!(
            Power::gigawatts(2.0) * AnnualCapacityCost::pounds_per_megawatt_year(16_000.0),
            MoneyRate::pounds_per_year(3.2e7)
        );
    }

    #[test]
    fn annual_energy_capacity_cost_times_energy_is_a_money_rate() {
        // £15/kWh/yr × 4 GWh (= 4×10⁶ kWh) = £60m/yr.
        assert_eq!(
            AnnualEnergyCapacityCost::pounds_per_kilowatt_hour_year(15.0)
                * Energy::gigawatt_hours(4.0),
            MoneyRate::pounds_per_year(6.0e7)
        );
        assert_eq!(
            Energy::gigawatt_hours(4.0)
                * AnnualEnergyCapacityCost::pounds_per_kilowatt_hour_year(15.0),
            MoneyRate::pounds_per_year(6.0e7)
        );
    }

    #[test]
    fn money_rate_over_years_is_money() {
        assert_eq!(
            MoneyRate::pounds_per_year(1.0e6).over_years(2.5),
            Money::pounds(2.5e6)
        );
    }

    // -----------------------------------------------------------------
    // Q5 heating-overlay arithmetic (D9 rules 3–4).
    // -----------------------------------------------------------------

    #[test]
    fn heat_intensity_times_heat_need_is_power() {
        // k = 4 GW/K at 5 K of heat need delivers 20 GW of heat.
        assert_eq!(
            HeatIntensity::gigawatts_per_kelvin(4.0) * Temperature::celsius(5.0),
            Power::gigawatts(20.0)
        );
        assert_eq!(
            Temperature::celsius(5.0) * HeatIntensity::gigawatts_per_kelvin(4.0),
            Power::gigawatts(20.0)
        );
    }

    #[test]
    fn temperature_differences_are_kelvin_valued() {
        // T_sink − T_source: 40 °C − (−5 °C) = 45 K of lift.
        assert_eq!(
            Temperature::celsius(40.0) - Temperature::celsius(-5.0),
            Temperature::celsius(45.0)
        );
    }

    #[test]
    fn length_metre_conversions_round_trip() {
        assert_eq!(Length::metres(1.0), Length::kilometres(0.001));
        assert_eq!(Length::metres(1.2).as_metres(), 1.2);
    }

    #[test]
    fn degree_hour_arithmetic_is_dimensional() {
        // 5 K of heat need over half an hour is 2.5 °C·h.
        assert_eq!(
            Temperature::celsius(5.0) * Duration::half_hour(),
            DegreeHours::celsius_hours(2.5)
        );
        assert_eq!(
            Duration::half_hour() * Temperature::celsius(5.0),
            DegreeHours::celsius_hours(2.5)
        );
        // k = 170,357 GWh over 50,454 °C·h ≈ 3.3765 GW/K (the D9
        // rule-3 definition).
        assert_eq!(
            Energy::gigawatt_hours(100.0) / DegreeHours::celsius_hours(50.0),
            HeatIntensity::gigawatts_per_kelvin(2.0)
        );
    }

    #[test]
    fn accessors_return_declared_units() {
        assert_eq!(Energy::gigawatt_hours(2.5).as_gigawatt_hours(), 2.5);
        assert_eq!(
            Price::pounds_per_megawatt_hour(80.0).as_pounds_per_megawatt_hour(),
            80.0
        );
        assert_eq!(
            Inertia::gigavolt_ampere_seconds(120.0).as_gigavolt_ampere_seconds(),
            120.0
        );
        assert_eq!(Emissions::tonnes_co2(1e6).as_tonnes_co2(), 1e6);
        assert_eq!(PerUnit::new(0.95).value(), 0.95);
        assert_eq!(Duration::hours(0.5), Duration::half_hour());
        assert_eq!(Duration::half_hour().as_hours(), 0.5);
    }
}
