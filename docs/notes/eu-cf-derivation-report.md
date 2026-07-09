# EU external-zone CF derivation — evidence note (Stage 5)

Built 2026-07-03 per the Stage 5 external-zone work order. Scripts:
`scripts/era5-cf/derive_cf_eu.py` (derivation) and
`scripts/era5-cf/validate_cf_eu.py` (validator, incl. the committed EU
pack geometry re-assertion that `docs/notes/eu-pack-box-review.md` note 3
obliges). Anchor builder: `scripts/fetch-entsoe/build_gen_agg.py`.
NL recalibration anchor (2026-07-03 addendum below): CBS pack,
`scripts/fetch-cbs/{fetch,build}.py`, manifest `data/packs/cbs-2024.sha256`.
Machine-readable numbers: `data/packs/cf-eu/eu_cf_report.json`.

Deliverables (git-ignored data; committed manifest
`data/packs/cf-eu-1985-2024.sha256`):

    data/packs/cf-eu/{fr,be,nl,de,dk1}/<c>_{onshore,offshore,solar}_cf_<Y>.{parquet,csv}
    data/packs/cf-eu/ie/ie_{onshore,solar}_cf_<Y>.{parquet,csv}
    data/packs/cf-eu/{fr,be,nl,de,dk1,ie,no2}/<c>_t2m_<Y>.{parquet,csv}

Y = 1985–2024 (40 years). CF traces: single float64 `cf` in [0, 1],
half-hourly UTC `utc_start` index (17,520 periods; 17,568 leap) — the GB
trace format. Temperature traces: float64 `t2m_c` (Celsius), same index,
population-weighted (new format; GB has no temperature trace yet).
Countries are derived SEPARATELY so the D5 zone decision can aggregate
later without re-derivation.

## Sources, licences, retrieval

| Item | Value |
|---|---|
| Weather | `data/packs/era5-eu/` (committed manifest `era5-eu-1985-2024.sha256`; Earthmover icechunk ERA5, snapshot `39TK56WX185WZ1HP9WNG`, fetched 2026-07-03; box 42–72°N 11°W–16°E, 13,189 cells, single-source, no decode seam). ERA5 is CC-BY 4.0; attribution: "Contains modified Copernicus Climate Change Service information [2024]. Neither the European Commission nor ECMWF is responsible for any use that may be made of the Copernicus information or data it contains." |
| Calibration anchor | ENTSO-E Transparency Platform A75 actual generation per production type + A68 installed capacity, 2024, per zone (fr, be, nl, delu, dk1, ie), fetched 2026-07-03 (`scripts/fetch-entsoe/fetch.py`, extended the same day; built to `aggregation_gen_2024.{parquet,csv}` in the entsoe pack, manifest `entsoe-2024.sha256` now 31 entries). **A75 generation and A68 capacity are NOT on the ENTSO-E CC-BY free-re-use list** — this use is the GTC clause-3.1 case (use for any purpose with source acknowledgement): an internal calibration anchor, fetched-and-built locally, git-ignored, never redistributed. Attribution: "Source: ENTSO-E Transparency Platform". |
| NL recalibration anchor (addendum) | CBS (Statistics Netherlands) StatLine tables **82610NED** ("Hernieuwbare elektriciteit; productie en vermogen") and **85005NED** ("Zonnestroom; vermogen en vermogensklasse"), via the CBS OData API (datasets.cbs.nl), retrieved 2026-07-03; 2024 rows carry CBS status **"NaderVoorlopig"** (revised provisional — stated, not hidden). Licence: **CC BY 4.0**, verified 2026-07-03 at https://www.cbs.nl/en-gb/about-us/website/copyright ("the content of this website is subject to Creative Commons Attribution (CC BY 4.0)"; naming CBS as source mandatory). Attribution: "Source: CBS (Statistics Netherlands)". Fetched-and-built (`scripts/fetch-cbs/`), git-ignored, manifest `cbs-2024.sha256` committed. |
| GB comparator | `data/packs/cf/gb_*_cf_<Y>` (pinned Phase B traces) for the correlation matrix. |

Determinism: no network in derive/validate; outputs are a pure function
of the two packs and the constants in `derive_cf_eu.py`; checksum
reproducibility is conditional on `scripts/era5-cf/requirements.txt`
versions (the era5-venv precedent) and the pinned snapshot ID.

## Method

The GB method (`derive_cf.py`, reviewed Phase A/B), imported — not
reimplemented: same logistic aggregate power curves (offshore V0 9.5,
S 1.9; onshore V0 8.5, S 1.7, sheared 100→80 m; PMAX 0.95, LOSSES 0.90;
25–30 m/s storm taper), same GHI-proportional PV model with temperature
derate (PR 0.85, γ −0.0037/K), same hourly→half-hourly linear
interpolation and solar half-hour centring (ssrd hour-ending convention
re-verified empirically on the EU pack: clear day 2024-06-07 at 48°N 2°E,
irradiance centroid +29.8 min after true solar noon). `derive_cf.py` is
byte-unchanged; the GB path and all committed GB manifests are untouched.

Documented deviations: (1) per-country spatial weights (below);
(2) ENTSO-E calibration anchor; (3) the out-of-band → uncalibrated
honesty policy (below); (4) new temperature series; (5) an EU-layout
cutout loader (same 3×3-cell box means, float64).

### Zone geometry decisions

- **de** = DE-LU bidding zone (Luxembourg inside; LU temperature point
  included).
- **dk1** = Jutland + Funen only (the Viking Link zone). Onshore/solar
  points all west of the Great Belt; offshore = Horns Rev cluster,
  Vesterhav, and **Anholt (Kattegat), which grid-connects to Jutland**;
  Zealand and Bornholm (DK2) excluded.
- **ie** = island of Ireland (all-island SEM, matching ENTSO-E's IE(SEM)
  zone); points cover IE and NI. **No offshore series** — SEM offshore in
  2024 is Arklow Bank (~25 MW, immaterial); ENTSO-E lists no SEM B18.
- **no2** = temperature only (Stavanger/Kristiansand/Haugesund,
  southwest-Norway population); the Norwegian zone is hydro-driven from
  ENTSO-E data (`entsoe-stage5-pack-report.md` §6). NO2 wind is
  deliberately out of scope (D5 note).

### Spatial weights (APPROXIMATE — reviewer scrutiny list)

Public-knowledge regional statistics and named offshore clusters, not a
licensed site database — the same honesty level as the GB UKWED-style
weights. Every point (name, lat, lon, GW weight) is in
`derive_cf_eu.py`; weights are normalised, so only relative sizes
matter. Summary of what was assumed:

- **FR onshore** (9 regions): Hauts-de-France + Grand Est ≈ half the
  fleet; Occitanie (Aude, Mediterranean regime) 1.7 GW; the west 3–4 GW.
- **FR offshore** (3 farms): Saint-Nazaire, Fécamp, Saint-Brieuc —
  Atlantic/Channel, not North Sea.
- **FR solar** (7 regions): strongly south-heavy (Nouvelle-Aquitaine,
  Occitanie, PACA ≈ 55 %).
- **BE**: Flanders/Wallonia split onshore (1.7/1.4) and solar (6.0/2.8);
  one compact offshore zone off Zeebrugge.
- **NL onshore** (5): Flevoland/IJsselmeer, Groningen/Friesland,
  Zeeland/Zuid-Holland delta, Noord-Holland. **NL offshore** (4):
  Borssele, Hollandse Kust, Gemini (far north), older IJmuiden farms.
  **NL solar** (5): broadly distributed, Brabant/east-heavy.
- **DE onshore** (10): strongly north-heavy (SH 8.5, Niedersachsen 12.5,
  Brandenburg 8.0, Sachsen-Anhalt 5.4, NRW 6.9, MV 3.7; thin south).
  **DE offshore**: German Bight 7.0 vs Baltic (Arkona area) 1.5.
  **DE solar** (9): Bavaria 19.5 + BW 9.0, substantial east and NRW.
- **DK1**: west-coast-heavy onshore; offshore per farm; solar southern
  Jutland-heavy.
- **IE onshore** (8): the TRUE all-island distribution (~5.9 GW,
  Atlantic-facing counties + NI Tyrone/Antrim) — deliberately NOT scaled
  to ENTSO-E's stale 3.0 GW (weights are relative only).
- **Temperature**: city/metro population weights per country
  (approximate millions, listed in the script).

Cluster points are 3×3-cell ERA5 box means (±0.375°); coastal points
include sea cells (as in GB).

## Calibration (the honesty metric)

One multiplicative factor per technology per country: 2024 annual energy
of trace × A68 capacity must equal the A75 2024 energy. Target CF =
gen_GWh / (capacity_MW × 8,784 h). **Honesty policy (extends the GB
band rule): a factor outside [0.7, 1.3] is treated as an anchor data
finding and is NOT applied — the trace ships uncalibrated (factor 1.0)
with the diagnosis recorded.** Pinned factors + drift guard:
`derive_cf_eu.PINNED_FACTORS_EU` (mechanism identical to the GB
`PINNED_FACTORS_2024`).

| Country | Tech | Raw CF 2024 | Target CF | Computed factor | Verdict |
|---|---|---|---|---|---|
| fr | onshore | 0.2180 | 0.2199 | **1.0087** | honest, applied |
| fr | offshore | 0.3036 | 0.4490 | 1.5580 | **OUT OF BAND — uncalibrated** (anchor fault, see below) |
| fr | solar | 0.1246 | 0.1492 | **1.1974** | honest, applied (tilt-gain under-modelling pulls >1, opposite of GB's 0.88 — southern latitudes + A68 capacity likely below true fleet) |
| be | onshore | 0.2474 | 0.2064 | **0.8342** | honest, applied |
| be | offshore | 0.3675 | 0.3718 | **1.0117** | honest, applied |
| be | solar | 0.0991 | 0.1078 | **1.0883** | honest, applied |
| nl | onshore | 0.3220 | 0.2890 (CBS) | **0.8975** | honest, applied — **CBS-recalibrated 2026-07-03** (addendum); the A75 anchor (target 0.1253, factor 0.3891) remains an anchor fault of record |
| nl | offshore | 0.3806 | 0.3652 | **0.9597** | honest, applied |
| nl | solar | 0.1016 | 0.0888 (CBS) | **0.8735** | honest, applied — **CBS-recalibrated 2026-07-03** (addendum); the A75 anchor (target 0.0020, factor 0.0195) remains an anchor fault of record |
| de | onshore | 0.2205 | 0.2143 | **0.9716** | honest, applied |
| de | offshore | 0.4216 | 0.3455 | **0.8195** | honest, applied |
| de | solar | 0.1072 | 0.0938 | **0.8750** | honest, applied |
| dk1 | onshore | 0.3411 | 0.2538 | **0.7441** | honest, applied — near the band edge; the DK1 fleet is old/low-hub-height stock our GB-style curve over-produces; physically explicable, kept |
| dk1 | offshore | 0.4058 | 0.4362 | **1.0749** | honest, applied |
| dk1 | solar | 0.0990 | 0.1090 | **1.1015** | honest, applied |
| ie | onshore | 0.2850 | 0.5072 | 1.9716 | **OUT OF BAND — uncalibrated** (anchor fault) |
| ie | solar | 0.0945 | — | — | **NO ANCHOR — uncalibrated** |

14 of 16 anchored series calibrate inside the band (0.74–1.20; 12 on
ENTSO-E anchors, 2 — NL onshore and NL solar — on the CBS anchors of the
2026-07-03 addendum. GB precedent 0.88–1.04, wider here as expected for
six foreign fleets).

### Anchor-fault diagnoses (the findings, not absorbed)

1. **FR offshore.** A68 lists 1,003 MW but the physical end-2024 fleet
   is ≈1,473 MW (Saint-Nazaire 480 + Fécamp 497 + Saint-Brieuc 496;
   the latter two commissioned 2023–24). A75 energy over the REAL fleet
   implies CF 0.306 — vs raw model 0.304. **The physical model is right
   to <1 %; the A68 denominator is stale.** Shipped uncalibrated.
2. **IE onshore.** A75 13,368 GWh is the plausible all-island SEM wind
   outturn, but A68 3,000 MW is years-stale (true all-island ≈5.9 GW).
   Implied CF 0.507 is non-physical. Inverting: the generation and the
   raw CF level (0.285) together imply a ≈5.3 GW fleet — matching the
   known fleet within ~10 %. Same disease, same verdict.
3. **NL onshore / NL solar — RESOLVED by CBS recalibration
   (2026-07-03 addendum; closes eu-cf-review defect D3).** ENTSO-E A75
   under-reports NL distribution-connected generation: onshore
   7,653.9 GWh reported vs **17,657 GWh** CBS net national outturn
   (43 % captured), solar 487.4 GWh vs **21,822 GWh** (2 % captured) —
   CBS StatLine 82610NED, retrieved 2026-07-03, status "NaderVoorlopig"
   (revised provisional), CC BY 4.0. Implied CFs 0.125 / 0.002 are
   artefacts of the numerator. The uncited "≈15–16 TWh" onshore
   magnitude that stood here (the D3 defect) was itself wrong: CBS net
   onshore is 17.66 TWh. Originally shipped uncalibrated; now
   CBS-calibrated (factors 0.8975 / 0.8735, in band).
4. **IE solar.** The SEM B16 A75 series first appears **2024-11-13**
   (platform onboarding), and A68 lists no SEM solar capacity — no
   full-year anchor exists. An anchor series absent for whole months is
   excluded, never zero-filled (`build_gen_agg.py` rule). Shipped
   uncalibrated.

**Uncalibrated-trace bias guidance (stated, not hidden; NL entries
superseded by the 2026-07-03 CBS recalibration — addendum below):**
three series remain uncalibrated. **FR offshore**: raw level right to
<1 % against the real 1,473 MW fleet (diagnosis 1). **IE onshore is
likely close to right** (GB onshore factor 1.04; IE raw annual CF 0.285
vs the ~0.28–0.30 all-island historical norm). **IE solar likely
~10–15 % high** — raw solar runs high without a tilt/calibration
correction in the north (GB 0.88, DE 0.8750, and now NL 0.8735 applied
factors). The old NL guidance ("onshore ~20–30 % high, solar ~15 %
high") was based on the wrong uncited ~15–16 TWh onshore magnitude; the
CBS-measured corrections are **onshore +11.4 % high** (factor 0.8975)
and **solar +14.5 % high on the DC-referenced CF** (factor 0.8735).
Recalibrating IE would need an adopted national anchor (SEAI/EirGrid —
separate source, separate licence check, NOT adopted here).

### Calibration semantics (restating the GB Phase B decision)

The 2024 factors apply unchanged to all years. A year-Y trace answers
"what would the A68-2024 fleet have produced in year Y's weather?" —
NOT "what did year Y's fleet produce?". The calibrated traces are
energy-matching CFs for the **A68-2024 capacities and must be paired
with those capacities** (or deliberately rescaled) in scenario work —
e.g. de solar trace × 77,015.56 MW reproduces 63.44 TWh.

## 2024 energy reconciliation (derived × A68 capacity vs ENTSO-E A75)

Annual energy reproduces the anchor exactly for every calibrated series
(by construction; validator asserts to 1e-6 relative). Monthly shape is
the genuine skill measure:

| Country | Onshore r | Offshore r | Solar r |
|---|---|---|---|
| fr | 0.9905 | 0.6127 (uncal.) | 0.9780 |
| be | 0.9928 | 0.9862 | 0.9814 |
| nl | 0.9945 (CBS-cal.) | 0.9544 | 0.9923 (CBS-cal.) |
| de | 0.9944 | 0.9414 | 0.9928 |
| dk1 | 0.9852 | 0.9422 | 0.9827 |
| ie | 0.9373 (uncal.) | — | — (no anchor) |

(GB precedent: monthly wind r 0.986, solar 0.995.) Notes: FR offshore
0.61 reflects the observed series' within-year commissioning ramp
(Fécamp/Saint-Brieuc), absent from a fixed-fleet trace — expected, not a
model defect. NL onshore/solar monthly r ≈0.99 despite the level fault
shows the platform's NL feed is shape-faithful but level-deficient
(r is scale-invariant, so it is unchanged by the CBS recalibration;
their ANNUAL energies now reproduce the CBS anchors, not A75 — the
table's annual mismatch for those two rows IS the A75 under-report).
Monthly tables per country/tech: `eu_cf_report.json`
(`reconciliation_2024`).

## 40-year statistics (fixed A68-2024 fleet, weather 1985–2024)

Total wind (A68-capacity-weighted onshore+offshore; ie = onshore):

| Country | Mean CF | Worst year | Best year |
|---|---|---|---|
| fr | 0.2336 | **2010** (0.2131) | 2023 (0.2631) |
| be | 0.2775 | **2010** (0.2419) | 1986 (0.3125) |
| nl | 0.3192 | **2010** (0.2754) | 1986 (0.3537) |
| de | 0.2322 | **2003** (0.2034) | 1990 (0.2614) |
| dk1 | 0.3054 | **2003** (0.2718) | 2015 (0.3400) |
| ie | 0.2921 | **2010** (0.2265) | 1986 (0.3378) |

GB's worst wind year on the pinned GB traces is also **2010** (onshore
0.2318 / offshore 0.2999): the 2010 drought is a shared GB–FR–BE–NL–IE
event, while DE/DK1 bottom out in 2003 — a real east–west split in the
drought climatology, exactly the structure a multi-zone adequacy model
must capture. Per-technology annual means for every year:
`eu_cf_report.json` (`annual_cf`); temperature summaries
(`t2m_annual_mean_c`): fr 12.1 °C, be 10.6, nl 10.6, de 10.0, dk1 8.8,
ie 9.9, no2 8.0 — all climatologically plausible.

## Cross-country wind correlation vs GB (the Module 5 anticyclone evidence)

Pearson r of total-wind CF, 40 years. Half-hourly / daily-mean:

| | gb | fr | be | nl | de | dk1 | ie |
|---|---|---|---|---|---|---|---|
| **gb** | 1 | .431/.478 | .597/.656 | .645/.714 | .522/.601 | .445/.537 | **.708/.793** |
| **fr** | | 1 | .813/.852 | .645/.703 | .542/.604 | .280/.334 | .319/.391 |
| **be** | | | 1 | .899/.928 | .668/.736 | .390/.461 | .366/.466 |
| **nl** | | | | 1 | .790/.842 | .525/.596 | .360/.463 |
| **de** | | | | | 1 | .715/.759 | .315/.394 |
| **dk1** | | | | | | 1 | .254/.331 |
| **ie** | | | | | | | 1 |

Headline: **every import counterparty's wind is substantially positively
correlated with GB's** — daily r from 0.48 (FR) through 0.66 (BE), 0.71
(NL), 0.60 (DE), 0.54 (DK1) to 0.79 (IE). A GB wind drought is a
NW-European wind drought, strongest across the Irish Sea and the
southern North Sea, weakest (but still ~0.5) toward France and western
Denmark. This is the resource-level basis for Stage 5's anticyclone
mechanism (flow-level evidence: `entsoe-stage5-pack-report.md` §5).

## Validation summary

`validate_cf_eu.py` (full run, exit 0):
- **EU pack geometry re-asserted** (the eu-pack-box-review note 3
  obligation, now a committed validator): 480 files, rows = hours ×
  13,189 cells, full 121×109 lattice, no NaNs, per-month calendar time
  spans, 350,640 hours total.
- **Traces**: 1,920 files (960 Parquet + 960 CSV), complete 40-year
  families; per file: period counts (17,520/17,568 — UTC-clean through
  every March/October clock change, asserted via strict 30-min
  uniformity + year boundaries), no duplicates, `utc_start` as
  timestamp[us, tz=UTC], float64 single column, cf in [0, 1], t2m_c in
  [−40, 45] °C, no NaNs, CSV/Parquet agreement to 1e-9, cross-year
  30-min continuity 1985→2024 per family (concat-loader contract).
- **Calibration reproduction**: every calibrated series' 2024 mean CF
  reproduces the ENTSO-E anchor target to <1e-6 relative; applied
  factors match `PINNED_FACTORS_EU`; uncalibrated series carry factor
  1.0 exactly.

Manifest: `data/packs/cf-eu-1985-2024.sha256` (1,921 entries: 1,920
trace files + `eu_cf_report.json`), written after validation.

## Limitations (stated, not hidden)

1. Spatial weights are approximate regional points, not site databases;
   adequate for zone-aggregate traces, revisit only if sub-national
   splits are ever needed.
2. Three of 17 series are uncalibrated (fr offshore, ie onshore, ie
   solar) with stated bias directions; the anchor faults are ENTSO-E
   data quality, documented above. NL onshore/solar were recalibrated
   against CBS national statistics on 2026-07-03 (addendum below).
3. No curtailment, no outage structure, no within-year capacity growth
   (as GB); FR offshore's monthly r 0.61 is the visible consequence.
4. Anchored to a single year (2024) of ENTSO-E data; a second anchor
   year would tighten the factors (GB has the same single-year anchor).
5. The IE onshore weights (all-island true distribution) are
   deliberately inconsistent with ENTSO-E's stale 3.0 GW capacity row —
   scenario capacity for ie must NOT be taken from A68.
6. Temperature series are population-weighted (approximate weights),
   not demand-weighted; fine for temperature-demand regression, stated
   here.
7. DK1 onshore factor 0.744 sits near the band edge — kept because the
   old-stock explanation is physical, but it is the weakest calibrated
   factor in the set.
8. No NO2 wind/hydro, no zone aggregation, no scenario files — Stage 5
   design work, out of scope here by the work order.
---

## Addendum — 2026-07-03 CBS recalibration of NL onshore + NL solar

**Trigger (adjudicated, not discretionary):** eu-cf-review ruling 1 made
a CBS national-statistics recalibration mandatory if any acceptance
verdict flipped inside the NL sensitivity bracket; the stage-5-review
escalation adjudication (ruling 3, condition 1) found the A4-BE verdict
flips at the w=1 configuration (Nemo error −1.44 shipped / −1.67 TWh at
the bias-corrected end, outside ±1.5). This addendum is that package:
observed-statistics anchoring, not input tuning. It also closes
eu-cf-review defect D3 (the uncited ~15.5/~21 TWh magnitudes).

### Licence verdict (checked first)

CBS StatLine content is **Creative Commons Attribution 4.0 (CC BY 4.0)**
— verified 2026-07-03 at
https://www.cbs.nl/en-gb/about-us/website/copyright: "the content of
this website is subject to Creative Commons Attribution (CC BY 4.0)";
naming CBS as the source is mandatory. Redistribution of derived traces
is therefore permitted with attribution ("Source: CBS (Statistics
Netherlands)"), carried here, in `scripts/fetch-cbs/`, and in the pack
build report.

### CBS anchor evidence (both tables retrieved 2026-07-03 via the CBS
OData API, datasets.cbs.nl; raw JSON + processed table in
`data/packs/cbs-2024/`, manifest `data/packs/cbs-2024.sha256`; build
double-run byte-identical)

**2024 rows carry CBS status "NaderVoorlopig" (revised provisional).**
Stated plainly: a provisional national statistic still beats a
known-biased A75 anchor, but CBS revises in place — a re-fetch after a
revision fails the committed manifest by design (drift visible, never
silent). 2023 is "Definitief" for comparison.

| Quantity | Value | CBS table |
|---|---|---|
| NL onshore wind net generation 2024 | **17,657 GWh** (gross 18,021) | 82610NED (E006637) |
| NL onshore wind capacity end-2024 | **6,955 MW** | 82610NED |
| NL solar PV generation 2024, all sectors | **21,822 GWh** (dwellings 9,589 + economic activities 12,233, 85005NED sector split — sums exactly; 82610NED reports the identical national total, single all-sector row) | 82610NED (E006590) / 85005NED |
| NL solar panel capacity end-2024 (DC) | **27,979.732 MWp** | 85005NED (E007161) |
| NL solar inverter capacity end-2024 (AC) | 24,920.094 MW | 85005NED |
| NL solar capacity end-2024 (82610NED convention, AC-side) | 24,772 MW | 82610NED |
| NL offshore wind net generation 2024 (corroboration only) | 15,182 GWh | 82610NED (E006638) |

### The under-report, quantified (D3 closed)

| Series | ENTSO-E A75 2024 | CBS 2024 | A75 captures |
|---|---|---|---|
| NL onshore | 7,653.9 GWh | 17,657 GWh (net) | **43.3 %** |
| NL solar | 487.4 GWh | 21,822 GWh | **2.2 %** |
| NL offshore (trusted, control) | 15,203.9 GWh | 15,182 GWh (net) | 100.1 % |

The offshore control also settles the gross-vs-net choice: A75 sits
0.14 % from CBS **net** and 2.0 % from gross, so the onshore anchor is
CBS net generation (matching the A75 convention every other calibrated
series uses). Note the old bias guidance's "~15–16 TWh" onshore
magnitude was wrong; the true figure is higher, so the old ×0.78
bracket end OVERSTATED the onshore bias.

### Capacity denominators (the pairing decision)

- **NL onshore: 6,955 MW** — CBS 82610NED end-2024 capacity, which is
  **numerically identical to A68's value**. The A68 onshore denominator
  was never the fault; no pairing change.
- **NL solar: 27,979.732 MWp (DC panel capacity, 85005NED)** — this
  reveals A68's 27,980 MW as exactly the CBS DC panel figure (rounded
  to MW). The DC denominator is retained because (a) it is what A68
  carries, so the trace still pairs with the A68 capacity the scenario
  already uses (trace × 27,980 MW reproduces 21,822 GWh to <0.001 %),
  and (b) it is physically consistent with the PV model, whose CF is
  referenced to STC panel (DC) rating. CBS's AC-side figures (inverter
  24,920 MW; 82610NED 24,772 MW) are recorded above and are **NOT** the
  paired capacities — pairing the trace with an AC capacity would
  understate NL solar energy by ~11 %.
- End-of-year denominators overstate the within-year average fleet
  (CBS solar capacity grew 21,957 → 24,772 MW AC / to 27,980 MWp DC
  during 2024; onshore 6,692 → 6,955 MW): the factor absorbs this,
  exactly the GB Phase B / FR-offshore semantics — the pairing rule is
  what keeps it honest.

### Factors (old → new; drift guard re-pinned)

| Series | Old (uncalibrated sentinel) | New (CBS-anchored) | Honesty band [0.7, 1.3] |
|---|---|---|---|
| nl onshore | 1.0 | **0.8975** (raw CF 0.3220 → target 0.2890) | **IN BAND** — raw trace was +11.4 % high |
| nl solar | 1.0 | **0.8735** (raw CF 0.1016 → target 0.0888) | **IN BAND** — raw +14.5 % high on the DC-referenced CF, in line with GB 0.88 / DE 0.8750 tilt-correction factors |

Both factors sit inside the calibrated point of the old sensitivity
bracket (onshore [×0.78, ×1.0]; solar [×0.85, ×1.0]) — the bracket
collapses to this point for the Stage 5 re-verification. The derive
script now hard-stops (never forces) if a CBS-anchored factor leaves
the band.

### Mechanics, verification, manifest

- Same pinned machinery (`derive_cf_eu.py`; the GB functions untouched):
  only the two NL anchors changed, read from the CBS pack via
  `load_cbs_anchors`. `PINNED_FACTORS_EU` re-pinned for the two NL
  entries; all 15 other factors byte-for-byte unchanged.
- Full 1985–2024 re-derivation, run twice: **double-run
  byte-identical** (all 1,921 manifest files).
- Manifest `cf-eu-1985-2024.sha256` re-pinned: exactly **161 entries
  changed** (nl_onshore × 40 y × 2 formats + nl_solar × 40 y × 2
  formats + `eu_cf_report.json`); the other **1,760 entries verified
  byte-unchanged** against the previous manifest (full `shasum -c`,
  zero non-NL mismatches).
- `validate_cf_eu.py` updated (NL targets re-asserted against the CBS
  pack, report anchor fields cross-checked): full run **exit 0**; NL
  onshore/solar 2024 energies reproduce the CBS anchors to ≤4e-16
  relative.
- 40-y statistics moved as expected: NL total-wind mean CF 0.3390 →
  0.3192 (worst year still 2010, best still 1986) — level only, shape
  untouched.
- **Correction of record found during re-generation:** the original
  correlation table's NL–DK1 cell (.715/.759) was a transcription
  error duplicating the DE–DK1 cell; the true pre-recalibration value
  was .527/.598 (reconstructed from the shipped traces by inverting
  the factors), now .525/.596 at the calibrated point. No other cell
  moved by more than 0.002; the anticyclone headline is unaffected.

### What this addendum does NOT do

No scenario edits (the Stage 5 implementer re-runs A1/A4 at the
calibrated point under w=1 per stage-5-review ruling 3), no change to
any other country's factors or traces, no engine code. IE remains
uncalibrated with stated bias; a SEAI/EirGrid anchor would be a
separate source and licence check.
