//! Parser for the committed fuel/carbon prices-reference file
//! (`data/reference/prices-2024.toml`, schema `prices-reference-v1`) —
//! the pinned, HHV-consistent source of the Stage 2 SRMC recipe's
//! constants: monthly gas SAP (cross-check only; the per-period fuel
//! price is the pack's daily SAP trace), the 25 UKA auction clearing
//! prices, the Carbon Price Support rate, the natural-gas emission
//! factors (CO₂-only for pricing, CO₂e for accounting), and the fleet
//! thermal efficiencies (gross-CV basis).
//!
//! Parsing is strict, like the scenario schema: the `schema` string is
//! probed first (so a future reference-file revision fails with a clear
//! message), and unknown fields are rejected.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::GridError;
use crate::pricing::CarbonAuction;
use crate::time::UtcInstant;
use crate::units::{CarbonPrice, EmissionsRate, PerUnit, Price};

/// The reference-file schema string this parser reads.
pub const PRICES_REFERENCE_SCHEMA: &str = "prices-reference-v1";

/// The validated contents of a prices-reference file. Every number
/// carries its citation in the TOML source; this struct holds only what
/// the pricing layer consumes.
#[derive(Debug, Clone, PartialEq)]
pub struct PricesReference {
    /// The calendar year the reference describes.
    pub year: i64,
    /// Monthly gas System Average Price, `("YYYY-MM", £/MWh-thermal
    /// HHV)` — a cross-check series; runs use the daily SAP trace.
    pub gas_monthly_sap: Vec<(String, Price)>,
    /// UKA auction clearing prices, ascending by date.
    pub uka_auctions: Vec<CarbonAuction>,
    /// Carbon Price Support rate, £/tCO2, charged on top of the UKA.
    pub cps: CarbonPrice,
    /// Natural-gas CO₂-only emission factor, tCO2/MWh-thermal HHV — the
    /// **pricing** factor (UK ETS and CPS charge combustion CO₂ only).
    pub ef_co2_thermal: EmissionsRate,
    /// Natural-gas CO₂e emission factor (incl. CH₄, N₂O), tCO2e/
    /// MWh-thermal HHV — the **emissions-accounting** factor, never used
    /// for pricing.
    pub ef_co2e_thermal: EmissionsRate,
    /// Fleet thermal efficiency per technology key (`ccgt`, `ocgt`),
    /// gross-CV (HHV) basis.
    pub efficiency_hhv: BTreeMap<String, PerUnit>,
}

// ---------------------------------------------------------------------
// TOML-facing raw structures (strict).
// ---------------------------------------------------------------------

#[derive(Deserialize)]
struct SchemaProbe {
    schema: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReference {
    #[allow(dead_code, reason = "consumed by the schema probe")]
    schema: String,
    year: i64,
    gas: RawGas,
    carbon: RawCarbon,
    emission_factor: RawEmissionFactor,
    efficiency: BTreeMap<String, RawEfficiency>,
    // Present in the file for documentation; VOM is excluded from the
    // pinned SRMC recipe, so only its exclusion flag is checked.
    vom: RawVom,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGas {
    #[allow(dead_code, reason = "documentation field")]
    unit: String,
    monthly_sap: Vec<RawMonthlySap>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMonthlySap {
    month: String,
    price: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCarbon {
    uka: RawUka,
    cps: RawCps,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUka {
    #[allow(dead_code, reason = "documentation field")]
    unit: String,
    auctions: Vec<RawAuction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAuction {
    date: toml::value::Datetime,
    clearing_price: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCps {
    rate: f64,
    #[allow(dead_code, reason = "documentation field")]
    unit: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEmissionFactor {
    natural_gas: RawNaturalGasFactors,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNaturalGasFactors {
    co2_tonnes_per_mwh_th_hhv: f64,
    co2e_tonnes_per_mwh_th_hhv: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEfficiency {
    hhv: f64,
    #[allow(dead_code, reason = "sensitivity band, documentation only")]
    hhv_sensitivity: Option<Vec<f64>>,
    #[allow(dead_code, reason = "net-CV source value, documentation only")]
    lhv: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVom {
    #[allow(dead_code, reason = "sensitivity value, documentation only")]
    ccgt_typical_gbp_per_mwh: f64,
    included_in_srmc: bool,
}

impl PricesReference {
    /// Parse a prices-reference file from TOML text (strict; see module
    /// docs).
    pub fn from_toml_str(toml_text: &str) -> Result<Self, GridError> {
        let parse_err = |source: toml::de::Error| GridError::PricesReferenceParse {
            source: Box::new(source),
        };
        // Schema first, leniently, so a revision mismatch is reported as
        // such rather than as an arbitrary field error.
        let probe: SchemaProbe = toml::from_str(toml_text).map_err(parse_err)?;
        match probe.schema.as_deref() {
            None => {
                return Err(GridError::InvalidPricesReference {
                    reason: format!(
                        "missing mandatory `schema` field (this engine reads \
                         {PRICES_REFERENCE_SCHEMA:?})"
                    ),
                });
            }
            Some(found) if found != PRICES_REFERENCE_SCHEMA => {
                return Err(GridError::InvalidPricesReference {
                    reason: format!(
                        "unsupported schema {found:?}: this engine reads \
                         {PRICES_REFERENCE_SCHEMA:?}"
                    ),
                });
            }
            Some(_) => {}
        }
        let raw: RawReference = toml::from_str(toml_text).map_err(parse_err)?;
        Self::validate(raw)
    }

    /// Read and parse a prices-reference file, attaching the path to any
    /// error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InPricesReferenceFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    fn validate(raw: RawReference) -> Result<Self, GridError> {
        let invalid = |reason: String| GridError::InvalidPricesReference { reason };

        if raw.vom.included_in_srmc {
            return Err(invalid(
                "vom.included_in_srmc = true contradicts the pinned SRMC recipe \
                 (VOM is excluded; docs/notes/2024-price-pack-report.md §4)"
                    .to_owned(),
            ));
        }

        let mut uka_auctions = Vec::with_capacity(raw.carbon.uka.auctions.len());
        for auction in &raw.carbon.uka.auctions {
            let Some(date) = auction.date.date else {
                return Err(invalid(format!(
                    "UKA auction date {} is not a plain calendar date",
                    auction.date
                )));
            };
            let instant = UtcInstant::parse(&format!(
                "{:04}-{:02}-{:02}T00:00:00Z",
                date.year, date.month, date.day
            ))?;
            if auction.clearing_price <= 0.0 {
                return Err(invalid(format!(
                    "UKA auction {} has non-positive clearing price {}",
                    auction.date, auction.clearing_price
                )));
            }
            uka_auctions.push(CarbonAuction {
                date: instant,
                clearing_price: CarbonPrice::pounds_per_tonne_co2(auction.clearing_price),
            });
        }
        if uka_auctions.is_empty() {
            return Err(invalid("no UKA auctions listed".to_owned()));
        }
        uka_auctions.sort_by_key(|auction| auction.date);

        if raw.carbon.cps.rate < 0.0 {
            return Err(invalid(format!(
                "CPS rate {} is negative",
                raw.carbon.cps.rate
            )));
        }

        let factors = &raw.emission_factor.natural_gas;
        if factors.co2_tonnes_per_mwh_th_hhv < 0.0 || factors.co2e_tonnes_per_mwh_th_hhv < 0.0 {
            return Err(invalid("negative emission factor".to_owned()));
        }
        if factors.co2e_tonnes_per_mwh_th_hhv < factors.co2_tonnes_per_mwh_th_hhv {
            return Err(invalid(
                "CO2e factor is below the CO2-only factor (CO2e includes CO2)".to_owned(),
            ));
        }

        let mut efficiency_hhv = BTreeMap::new();
        for (key, entry) in &raw.efficiency {
            if !(entry.hhv > 0.0 && entry.hhv <= 1.0) {
                return Err(invalid(format!(
                    "efficiency.{key}.hhv = {} is outside (0, 1]",
                    entry.hhv
                )));
            }
            efficiency_hhv.insert(key.clone(), PerUnit::new(entry.hhv));
        }

        Ok(Self {
            year: raw.year,
            gas_monthly_sap: raw
                .gas
                .monthly_sap
                .iter()
                .map(|entry| {
                    (
                        entry.month.clone(),
                        Price::pounds_per_megawatt_hour(entry.price),
                    )
                })
                .collect(),
            uka_auctions,
            cps: CarbonPrice::pounds_per_tonne_co2(raw.carbon.cps.rate),
            ef_co2_thermal: EmissionsRate::tonnes_per_megawatt_hour(
                factors.co2_tonnes_per_mwh_th_hhv,
            ),
            ef_co2e_thermal: EmissionsRate::tonnes_per_megawatt_hour(
                factors.co2e_tonnes_per_mwh_th_hhv,
            ),
            efficiency_hhv,
        })
    }
}
