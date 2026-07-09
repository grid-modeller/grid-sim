//! Parser for the committed published-pathway reference file
//! (`data/reference/pathways-published.toml`, schema
//! `pathways-published-v1`) — the Stage 7 cited capacity/demand tables
//! behind the FES 2025 Electric Engagement and CCC CB7 Balanced Pathway
//! scenarios (evidence: `docs/notes/stage7-pathways-data-report.md`,
//! adjudicated ACCEPT-WITH-CONDITIONS in
//! `docs/notes/stage7-pathways-data-review.md`).
//!
//! Parsing is strict, the `costs-reference-v1` pattern: the `schema`
//! string is probed first, unknown fields are rejected everywhere, and
//! semantic validation returns structured errors naming the offending
//! table and field.
//!
//! # Machine-enforced exclusion of unmappable aggregates (review condition 5)
//!
//! The CCC publishes several capacity buckets the engine's technology
//! ids cannot take without a split or modelling decision (unabated gas
//! CCGT:OCGT, low-carbon dispatchable, other generation, BECCS, smart
//! demand flexibility). The reference file carries them with
//! `mappable = false`, and this parser makes that flag LOAD-BEARING:
//!
//! - aggregates are parsed into their own type, [`ExcludedAggregate`] —
//!   a **named exclusion with magnitude**, never a fleet entry. No API
//!   of this module merges an aggregate into [`PathwayYear::fleet`], so
//!   a consumer cannot silently obtain aggregate capacity as fleet
//!   capacity; consuming one requires a declared, reviewed split rule
//!   in the consumer's own artefact (the Stage 7 scenario files carry
//!   theirs in prose, pinned by the acceptance tests).
//! - `mappable = true` on an aggregate is a structured parse error (a
//!   mappable row belongs in `fleet`), as is an aggregate whose name
//!   collides with a fleet technology (a double-count).
//!
//! # Stamps and load-bearing flags that travel
//!
//! - `energy_precision` on CB7 storage rows (Table 7.5.1 publishes GWh
//!   only as rounded integers) is parsed into
//!   [`PathwayStorageEntry::energy_precision`] so any artefact quoting
//!   CB7 storage energy can propagate the rounding caveat (review
//!   condition 5).
//! - `geography` is carried verbatim (`"UK"` on the CCC pathway — the
//!   UK-not-GB scope flag behind review condition 6).
//! - The CCC surplus-electrolysis exclusion (29 TWh 2035 / 89 TWh 2050,
//!   review condition 1) is machine-visible at BOTH its sites — the
//!   per-year `surplus_electrolysis_excluded_twh` field and the
//!   pathway-level exclusions register — and the parser verifies the
//!   two sites agree (the committed file's own "must stay in step"
//!   rule).
//!
//! # Units
//!
//! TWh, GW and GWh in the file become [`Energy`] / [`Power`] at parse
//! (TWh × 1000 → the canonical GWh carrier); raw `f64` does not cross
//! this API for a physical quantity. Exclusion-register year-maps must
//! state their unit through a `_gw` / `_twh` key suffix — an unsuffixed
//! magnitude key is a parse error, not a guess.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::GridError;
use crate::scenario::{StorageKind, TechId};
use crate::units::{Energy, Power};

/// The reference-file schema string this parser reads.
pub const PATHWAYS_PUBLISHED_SCHEMA: &str = "pathways-published-v1";

/// A cited source record (provenance; every number in the file names
/// its source in a comment, and every source is checksummed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathwaySource {
    /// Source title, including edition.
    pub title: String,
    /// Publication URL.
    pub url: String,
    /// Pinned snapshot checksum (the pack manifests re-verify it).
    pub sha256: String,
    /// Retrieval date, as written.
    pub retrieved: String,
    /// Licence statement.
    pub licence: String,
}

/// One published pathway (FES 2025 Electric Engagement, CCC CB7
/// Balanced Pathway).
#[derive(Debug, Clone, PartialEq)]
pub struct Pathway {
    /// The pathway's published name.
    pub name: String,
    /// Edition statement.
    pub edition: String,
    /// Geographic scope, verbatim (`"GB"`; `"UK"` for the CCC — the
    /// scenario package must declare the UK-as-GB convention, review
    /// condition 6).
    pub geography: String,
    /// Required attribution wording.
    pub attribution: String,
    /// Snapshot years, ascending; exactly the file's `snapshot_years`.
    pub years: Vec<PathwayYear>,
    /// Published buckets the engine cannot take without a declared
    /// split rule — named exclusions with magnitude, NEVER fleet
    /// capacity (module docs).
    pub aggregates: Vec<ExcludedAggregate>,
    /// The pathway's exclusions register: every excluded quantity with
    /// its magnitude, plus prose notes.
    pub exclusions: BTreeMap<String, ExclusionRecord>,
}

impl Pathway {
    /// The snapshot block for one year.
    #[must_use]
    pub fn year(&self, year: i64) -> Option<&PathwayYear> {
        self.years.iter().find(|y| y.year == year)
    }
}

/// One snapshot year of a pathway.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayYear {
    /// Calendar (CCC) or fiscal-labelled (FES) snapshot year — the
    /// basis wrinkle is stated in the file header, carried as
    /// published.
    pub year: i64,
    /// Annual electricity demand (the pathway's own published basis —
    /// see the demand-basis wedge in the file's exclusions).
    pub demand: Energy,
    /// Published peak demand; `None` where the source publishes none
    /// (CCC — the named quarantine gap).
    pub peak_demand: Option<Power>,
    /// Surplus-driven electrolysis demand EXCLUDED from `demand`
    /// (CCC c1 exclusion; `None` where the demand basis has no such
    /// exclusion).
    pub surplus_electrolysis_excluded: Option<Energy>,
    /// Demand decomposition, TWh entries keyed as published (component
    /// sets differ per source; FES also carries non-additive
    /// electrification markers — see the file's comments).
    pub demand_components: BTreeMap<String, Energy>,
    /// Fleet capacities — ONLY the unambiguous engine-technology
    /// mappings. Unmappable buckets are in [`Pathway::aggregates`].
    pub fleet: Vec<PathwayFleetEntry>,
    /// Storage entries (power + energy pairs).
    pub storage: Vec<PathwayStorageEntry>,
}

/// One unambiguous fleet mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayFleetEntry {
    /// Engine technology id.
    pub technology: TechId,
    /// Installed capacity.
    pub capacity: Power,
}

/// One storage entry.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwayStorageEntry {
    /// Storage kind (fes-pathway-v1 folds: CCC medium-duration and the
    /// FES LDES fold are carried as `pumped_hydro`).
    pub kind: StorageKind,
    /// Output power capacity.
    pub power: Power,
    /// Energy capacity.
    pub energy: Energy,
    /// Precision stamp on the energy figure, where the source publishes
    /// only rounded values (CB7 Table 7.5.1) — MUST travel onto any
    /// artefact quoting it (review condition 5).
    pub energy_precision: Option<String>,
}

/// A published capacity bucket that is NOT mappable to engine
/// technology ids without a declared, reviewed split rule — surfaced as
/// a named exclusion with magnitude, never as fleet capacity (module
/// docs; review condition 5).
#[derive(Debug, Clone, PartialEq)]
pub struct ExcludedAggregate {
    /// Bucket name as published (`unabated_gas`, …).
    pub name: String,
    /// Published capacity per snapshot year.
    pub capacity_by_year: BTreeMap<i64, Power>,
    /// The bucket's published definition and citation.
    pub definition: String,
    /// The data package's suggested treatment — a suggestion, not an
    /// adoption (review condition 8: splits are the scenario package's
    /// reviewed decision).
    pub suggested_treatment: String,
}

/// One entry of a pathway's exclusions register: a magnitude per
/// snapshot year (unit from the key's `_gw`/`_twh` suffix) or a prose
/// note.
#[derive(Debug, Clone, PartialEq)]
pub enum ExclusionRecord {
    /// A capacity magnitude per snapshot year (`*_gw` keys).
    CapacityByYear(BTreeMap<i64, Power>),
    /// An energy magnitude per snapshot year (`*_twh` keys).
    EnergyByYear(BTreeMap<i64, Energy>),
    /// A prose note (a ruling with no single magnitude, e.g. DSR/V2G).
    Note(String),
}

/// The validated contents of a pathways-published reference file.
#[derive(Debug, Clone, PartialEq)]
pub struct PathwaysPublished {
    /// Assembly date, as written.
    pub assembled: String,
    /// The snapshot years every pathway carries, ascending.
    pub snapshot_years: Vec<i64>,
    /// Cited sources by key.
    pub sources: BTreeMap<String, PathwaySource>,
    /// Pathways by key (`fes2025_electric_engagement`,
    /// `ccc_cb7_balanced`).
    pub pathways: BTreeMap<String, Pathway>,
}

// ---------------------------------------------------------------------
// TOML-facing raw structures (strict: deny_unknown_fields on every
// fixed-shape table).
// ---------------------------------------------------------------------

#[derive(Deserialize)]
struct SchemaProbe {
    schema: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[allow(dead_code, reason = "consumed by the schema probe")]
    schema: String,
    assembled: toml::value::Datetime,
    snapshot_years: Vec<i64>,
    sources: BTreeMap<String, RawSource>,
    pathways: BTreeMap<String, RawPathway>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    title: String,
    url: String,
    sha256: String,
    retrieved: toml::value::Datetime,
    licence: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPathway {
    name: String,
    edition: String,
    geography: String,
    attribution: String,
    years: Vec<RawYear>,
    #[serde(default)]
    aggregates: Vec<RawAggregate>,
    #[serde(default)]
    exclusions: BTreeMap<String, RawExclusion>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawYear {
    year: i64,
    demand_twh: f64,
    peak_demand_gw: Option<f64>,
    surplus_electrolysis_excluded_twh: Option<f64>,
    #[serde(default)]
    demand_components: BTreeMap<String, f64>,
    #[serde(default)]
    fleet: Vec<RawFleet>,
    #[serde(default)]
    storage: Vec<RawStorage>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFleet {
    technology: String,
    capacity_gw: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStorage {
    kind: StorageKind,
    power_gw: f64,
    energy_gwh: f64,
    energy_precision: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAggregate {
    name: String,
    mappable: bool,
    capacity_gw: BTreeMap<String, f64>,
    definition: String,
    suggested_treatment: String,
}

/// A magnitude year-map (`{ y2035 = …, y2050 = … }`) or a prose note.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawExclusion {
    YearMap(BTreeMap<String, f64>),
    Note(String),
}

// ---------------------------------------------------------------------
// Validation.
// ---------------------------------------------------------------------

fn invalid(reason: String) -> GridError {
    GridError::InvalidPathwaysReference { reason }
}

/// A finite, non-negative magnitude; `what` names the field.
fn non_negative(what: &str, value: f64) -> Result<f64, GridError> {
    if !value.is_finite() || value < 0.0 {
        return Err(invalid(format!("{what} = {value} must be non-negative")));
    }
    Ok(value)
}

/// A finite, strictly positive magnitude; `what` names the field.
fn positive(what: &str, value: f64) -> Result<f64, GridError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(invalid(format!("{what} = {value} must be positive")));
    }
    Ok(value)
}

/// Parse a `y<year>` key of a magnitude year-map against the snapshot
/// years; the map must cover EXACTLY the snapshot years.
fn year_map(
    what: &str,
    raw: &BTreeMap<String, f64>,
    snapshot_years: &[i64],
) -> Result<BTreeMap<i64, f64>, GridError> {
    let mut out = BTreeMap::new();
    for (key, &value) in raw {
        let year: i64 = key
            .strip_prefix('y')
            .and_then(|digits| digits.parse().ok())
            .ok_or_else(|| {
                invalid(format!(
                    "{what}: key {key:?} is not a y<year> snapshot-year key"
                ))
            })?;
        if !snapshot_years.contains(&year) {
            return Err(invalid(format!(
                "{what}: {year} is not a snapshot year (snapshot_years = {snapshot_years:?})"
            )));
        }
        out.insert(year, non_negative(&format!("{what}.{key}"), value)?);
    }
    for year in snapshot_years {
        if !out.contains_key(year) {
            return Err(invalid(format!(
                "{what}: missing snapshot year {year} (every magnitude travels with \
                 every snapshot year)"
            )));
        }
    }
    Ok(out)
}

impl PathwaysPublished {
    /// Parse a pathways-published reference file from TOML text
    /// (strict; see the module docs for the enforced semantics).
    pub fn from_toml_str(toml_text: &str) -> Result<Self, GridError> {
        let parse_err = |source: toml::de::Error| GridError::PathwaysReferenceParse {
            source: Box::new(source),
        };
        // Schema first, leniently, so a revision mismatch is reported
        // as such rather than as an arbitrary field error.
        let probe: SchemaProbe = toml::from_str(toml_text).map_err(parse_err)?;
        match probe.schema.as_deref() {
            None => {
                return Err(invalid(format!(
                    "missing mandatory `schema` field (this engine reads \
                     {PATHWAYS_PUBLISHED_SCHEMA:?})"
                )));
            }
            Some(found) if found != PATHWAYS_PUBLISHED_SCHEMA => {
                return Err(invalid(format!(
                    "unsupported schema {found:?}: this engine reads \
                     {PATHWAYS_PUBLISHED_SCHEMA:?}"
                )));
            }
            Some(_) => {}
        }
        let raw: RawFile = toml::from_str(toml_text).map_err(parse_err)?;
        Self::validate(raw)
    }

    /// Read and parse a pathways-published reference file, attaching
    /// the path to any error.
    pub fn load(path: &Path) -> Result<Self, GridError> {
        let in_file = |source: GridError| GridError::InPathwaysReferenceFile {
            path: path.to_path_buf(),
            source: Box::new(source),
        };
        let text =
            std::fs::read_to_string(path).map_err(|source| in_file(GridError::Io { source }))?;
        Self::from_toml_str(&text).map_err(in_file)
    }

    fn validate(raw: RawFile) -> Result<Self, GridError> {
        if raw.snapshot_years.is_empty() {
            return Err(invalid("snapshot_years is empty".to_owned()));
        }
        if !raw.snapshot_years.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(invalid(format!(
                "snapshot_years {:?} must be strictly ascending",
                raw.snapshot_years
            )));
        }

        let sources = raw
            .sources
            .into_iter()
            .map(|(key, source)| {
                (
                    key,
                    PathwaySource {
                        title: source.title,
                        url: source.url,
                        sha256: source.sha256,
                        retrieved: source.retrieved.to_string(),
                        licence: source.licence,
                    },
                )
            })
            .collect();

        let mut pathways = BTreeMap::new();
        for (key, pathway) in raw.pathways {
            let validated = validate_pathway(&key, pathway, &raw.snapshot_years)?;
            pathways.insert(key, validated);
        }

        Ok(Self {
            assembled: raw.assembled.to_string(),
            snapshot_years: raw.snapshot_years,
            sources,
            pathways,
        })
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one linear pass over a pathway's tables"
)]
fn validate_pathway(
    key: &str,
    raw: RawPathway,
    snapshot_years: &[i64],
) -> Result<Pathway, GridError> {
    let ctx = |field: &str| format!("pathways.{key}: {field}");

    // Years: exactly the snapshot years, in order.
    let year_labels: Vec<i64> = raw.years.iter().map(|y| y.year).collect();
    if year_labels != snapshot_years {
        return Err(invalid(ctx(&format!(
            "years {year_labels:?} must be exactly the snapshot_years {snapshot_years:?}"
        ))));
    }

    let mut years = Vec::with_capacity(raw.years.len());
    for year in &raw.years {
        let yctx = |field: &str| format!("pathways.{key}, year {}: {field}", year.year);

        let demand = positive(&yctx("demand_twh"), year.demand_twh)?;
        let peak_demand = year
            .peak_demand_gw
            .map(|value| positive(&yctx("peak_demand_gw"), value))
            .transpose()?
            .map(Power::gigawatts);
        let surplus = year
            .surplus_electrolysis_excluded_twh
            .map(|value| positive(&yctx("surplus_electrolysis_excluded_twh"), value))
            .transpose()?;

        let mut demand_components = BTreeMap::new();
        for (name, &value) in &year.demand_components {
            if !name.ends_with("_twh") {
                return Err(invalid(yctx(&format!(
                    "demand component {name:?} must carry the _twh unit suffix"
                ))));
            }
            demand_components.insert(
                name.clone(),
                Energy::gigawatt_hours(
                    non_negative(&yctx(&format!("demand_components.{name}")), value)? * 1000.0,
                ),
            );
        }

        let mut fleet = Vec::with_capacity(year.fleet.len());
        for entry in &year.fleet {
            if entry.technology.is_empty() {
                return Err(invalid(yctx("fleet entry with an empty technology id")));
            }
            if fleet
                .iter()
                .any(|f: &PathwayFleetEntry| f.technology.as_str() == entry.technology)
            {
                return Err(invalid(yctx(&format!(
                    "duplicate fleet technology {:?}",
                    entry.technology
                ))));
            }
            fleet.push(PathwayFleetEntry {
                technology: TechId::new(&entry.technology),
                capacity: Power::gigawatts(non_negative(
                    &yctx(&format!("fleet.{}.capacity_gw", entry.technology)),
                    entry.capacity_gw,
                )?),
            });
        }

        let mut storage = Vec::with_capacity(year.storage.len());
        for entry in &year.storage {
            if storage
                .iter()
                .any(|s: &PathwayStorageEntry| s.kind == entry.kind)
            {
                return Err(invalid(yctx(&format!(
                    "duplicate storage kind {}",
                    entry.kind
                ))));
            }
            storage.push(PathwayStorageEntry {
                kind: entry.kind,
                power: Power::gigawatts(positive(
                    &yctx(&format!("storage.{}.power_gw", entry.kind)),
                    entry.power_gw,
                )?),
                energy: Energy::gigawatt_hours(positive(
                    &yctx(&format!("storage.{}.energy_gwh", entry.kind)),
                    entry.energy_gwh,
                )?),
                energy_precision: entry.energy_precision.clone(),
            });
        }

        years.push(PathwayYear {
            year: year.year,
            demand: Energy::gigawatt_hours(demand * 1000.0),
            peak_demand,
            surplus_electrolysis_excluded: surplus.map(|twh| Energy::gigawatt_hours(twh * 1000.0)),
            demand_components,
            fleet,
            storage,
        });
    }

    // Aggregates: mappable = false is LOAD-BEARING (module docs).
    let mut aggregates = Vec::with_capacity(raw.aggregates.len());
    for aggregate in &raw.aggregates {
        let actx = |field: &str| format!("pathways.{key}, aggregate {}: {field}", aggregate.name);
        if aggregate.mappable {
            return Err(invalid(actx(
                "mappable = true is a contradiction — an aggregate is by definition a \
                 bucket the engine cannot take without a declared, reviewed split rule; \
                 unambiguous mappings belong under fleet (review condition 5)",
            )));
        }
        if aggregates
            .iter()
            .any(|a: &ExcludedAggregate| a.name == aggregate.name)
        {
            return Err(invalid(actx("duplicate aggregate name")));
        }
        for year in &years {
            if year
                .fleet
                .iter()
                .any(|f| f.technology.as_str() == aggregate.name)
            {
                return Err(invalid(actx(&format!(
                    "collides with a year-{} fleet technology of the same name — the \
                     capacity would be double-counted",
                    year.year
                ))));
            }
        }
        let capacity_by_year =
            year_map(&actx("capacity_gw"), &aggregate.capacity_gw, snapshot_years)?
                .into_iter()
                .map(|(year, value)| (year, Power::gigawatts(value)))
                .collect();
        aggregates.push(ExcludedAggregate {
            name: aggregate.name.clone(),
            capacity_by_year,
            definition: aggregate.definition.clone(),
            suggested_treatment: aggregate.suggested_treatment.clone(),
        });
    }

    // Exclusions register: unit from the key suffix; notes verbatim.
    let mut exclusions = BTreeMap::new();
    for (name, record) in &raw.exclusions {
        let ectx = format!("pathways.{key}, exclusion {name}");
        let validated = match record {
            RawExclusion::Note(text) => ExclusionRecord::Note(text.clone()),
            RawExclusion::YearMap(map) => {
                let magnitudes = year_map(&ectx, map, snapshot_years)?;
                if name.ends_with("_gw") {
                    ExclusionRecord::CapacityByYear(
                        magnitudes
                            .into_iter()
                            .map(|(year, value)| (year, Power::gigawatts(value)))
                            .collect(),
                    )
                } else if name.ends_with("_twh") {
                    ExclusionRecord::EnergyByYear(
                        magnitudes
                            .into_iter()
                            .map(|(year, value)| (year, Energy::gigawatt_hours(value * 1000.0)))
                            .collect(),
                    )
                } else {
                    return Err(invalid(format!(
                        "{ectx}: a magnitude year-map must state its unit through a \
                         _gw or _twh key suffix"
                    )));
                }
            }
        };
        exclusions.insert(name.clone(), validated);
    }

    // The surplus-electrolysis pair must stay in step (review
    // condition 1 as committed: the per-year field and the exclusions
    // register are two sites of ONE exclusion).
    const SURPLUS_KEY: &str = "surplus_electrolysis_demand_twh";
    let register = exclusions.get(SURPLUS_KEY);
    for year in &years {
        match (year.surplus_electrolysis_excluded, register) {
            (None, None) => {}
            (Some(field), Some(ExclusionRecord::EnergyByYear(map))) => {
                if map.get(&year.year) != Some(&field) {
                    return Err(invalid(ctx(&format!(
                        "surplus-electrolysis sites out of step at year {}: the year \
                         block says {} GWh, exclusions.{SURPLUS_KEY} says {:?} GWh",
                        year.year,
                        field.as_gigawatt_hours(),
                        map.get(&year.year).map(|e| e.as_gigawatt_hours()),
                    ))));
                }
            }
            (Some(_), None) => {
                return Err(invalid(ctx(&format!(
                    "year {} carries surplus_electrolysis_excluded_twh but the \
                     exclusions register has no {SURPLUS_KEY} entry (the two sites \
                     must stay in step)",
                    year.year
                ))));
            }
            (None, Some(_)) => {
                return Err(invalid(ctx(&format!(
                    "exclusions.{SURPLUS_KEY} exists but year {} carries no \
                     surplus_electrolysis_excluded_twh (the two sites must stay in \
                     step)",
                    year.year
                ))));
            }
            (Some(_), Some(_)) => {
                return Err(invalid(ctx(&format!(
                    "exclusions.{SURPLUS_KEY} must be an energy year-map"
                ))));
            }
        }
    }

    Ok(Pathway {
        name: raw.name,
        edition: raw.edition,
        geography: raw.geography,
        attribution: raw.attribution,
        years,
        aggregates,
        exclusions,
    })
}
