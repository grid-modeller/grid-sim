# ENTSO-E Stage 5 Pack — Licence Verdict and Evidence Report

Assembled 2026-07-03 per the Stage 5 data work order (docs/04 Stage 5;
docs/05 data table row "Continental demand/fleet (Stage 5)"). All sources
accessed **2026-07-03**; quotes verbatim.

Pack: `data/packs/entsoe-2024/` (git-ignored; checksums in
`data/packs/entsoe-2024.sha256`, processed files, 28 entries). Scripts:
`scripts/fetch-entsoe/` (fetch → build → validate → analyze; the README
records API details, data semantics and repair rules). Machine-readable
numbers: `data/packs/entsoe-2024/processed/analysis_entsoe_2024.json`;
per-series gap/repair record: `build_report_entsoe_2024.json` (in the
pack). `validate.py` exits 0 on the built pack.

## 1. Licence verdict (blocking check — done first)

**Verdict: adopted.** Fetch-and-build use of every item in this pack is
permitted with source acknowledgement; the physical-flows item is
additionally CC-BY 4.0 open data except for the IFA and Nemo Link
carve-outs. Nothing in this project's architecture depends on the rights
we do NOT have (raw redistribution of non-listed items).

Sources:
- Terms of Use: "General Terms and Conditions for the use of the ENTSO-E
  Transparency Platform", approved by the ENTSO-E Market Committee
  29 March 2023, applicable as from 1 November 2023. Published on the
  TP help centre "Legal Terms and Conditions" article (id 40921911218961,
  attachment 40921869376401), <https://transparencyplatform.zendesk.com/hc/en-us/articles/40921911218961-Legal-Terms-and-Conditions>.
- Open-data list: "List of Data Available for Free Re-Use", last modified
  18 October 2023 (attachment 40921869379729 on the same article).

What the Terms of Use say (clause 3.1, "Use of the Transparency Platform
Data" — applies to ALL TP data, listed or not): the Data User may use TP
data "for any purpose whatsoever" provided they

> "use the Transparency Platform Data in good faith and always comply
> with good business practices regarding the re-use of publicly available
> data; [...] mention the ENTSO-E Transparency Platform as the source of
> publication of the data [...] not cause prejudice to the copyright or
> related right on a Transparency Platform Data, which may be owned by
> the concerned Primary Owner of Data. In case of a risk to cause
> prejudice to said right, the Data User shall seek the prior agreement
> of the holder of the copyright or related right."

Clause 2.5 ("Open Source License"):

> "ENTSO-E currently publishes a subset of the Transparency Platform data
> available under an open source license (CC-BY 4.0) for the free use of
> Data Users (hereinafter 'Open Data')."

The list document operationalises this:

> "Data Users may freely copy, redistribute, and adapt the listed data
> for any purpose, by giving appropriate credit (attribution) to its
> source and indicating if they have made any changes, with no need to
> seek for the prior agreement of the respective Primary Owner of Data."

**What is on the CC-BY list (relevant to this pack):** Physical flows
(item 18, Transparency Regulation Art. 12.1.g). **What is NOT on the
list:** actual total load (6.1.a — only the load *forecasts* 6.1.b–e are
listed), installed capacity per production type (14.1.a), actual
generation per type (16.1), and reservoir filling (16.1.d). Those items
fall back to clause 3.1: use for any purpose with attribution, but
redistribution that could prejudice a Primary Owner's copyright requires
that owner's agreement.

**Named exceptions to the CC-BY list** (verbatim: "The below list, and
the Creative Commons Attribution 4.0 International License, does not
encompass the following data"):
- "data provided by interconnectors Interconnexion France-Angleterre
  (interconnector between France and the United Kingdom) and Nemo Link
  (interconnector between Belgium and the United Kingdom)" — i.e. **IFA
  and Nemo Link data are excluded entirely** (note: the wording names
  IFA; whether IFA2 and ElecLink are covered by the GB↔FR border series'
  provider is not resolvable from the document — treated conservatively
  as excluded, since the platform serves one combined GB↔FR series);
- BritNed: only balancing data items #24, 26, 29, 30, 32–35 are excluded
  (BritNed physical flows were explicitly opened for free re-use in the
  28 Oct 2019 list revision);
- Moldova, Turkey; Ukraine balancing items.

**Consequences for this project (mirrors the D1 discipline):**
1. **Fetching and building the pack locally** — permitted for all items
   (clause 3.1). Each user runs `fetch.py` with their own free token.
2. **Committing the pack** — does not happen (git-ignored; only sha256
   checksums and scripts are committed). No redistribution occurs.
3. **Publishing derived aggregate statistics** (this note, the book):
   covered by clause 3.1 use-for-any-purpose with the attribution
   "Source: ENTSO-E Transparency Platform". For GB↔FR and GB↔BE
   per-link numbers, the publication-grade source remains the
   Elexon/NESO 2024 pack (fully open licences, per-link resolution);
   ENTSO-E is the neighbour-side cross-check. This keeps every published
   flow number clear of the IFA/Nemo carve-out entirely.
4. **A future hosted pre-built pack (phase 2)** could include the
   physical-flow traces EXCEPT the GB↔FR and GB↔BE borders, and should
   not include the load/capacity/generation/reservoir traces without
   further diligence — flag for the phase-2 decision; not a Stage 5
   problem.

Registration/automated access: clause 3.3 provides the RESTful API for
exactly this ("To facilitate access to all interested parties using
automated tools ... ENTSO-E provides a specific M2M Interface"); the
platform documents a 400 req/min cap; `fetch.py` throttles to ~170.

## 2. What was fetched and built

261 API documents (2026-07-03, token-authenticated, zero
"no data" acknowledgements, zero 429s). Raw XML retained in the pack.

| Trace | Zones/borders | Native resolution (2024) | Built output |
|---|---|---|---|
| A11 physical flows per direction | GB↔FR, BE, NL, NO2, DK1, IE(SEM) | fr/dk1/ie PT60M; be/nl PT15M; **no2 PT15M Jan–Sep, PT60M after** | `flows_gb_entsoe_2024.*`, 17,568 half-hourly rows × 18 cols (imp/exp/net per border, MW, + = import to GB) |
| A65 actual load | FR, BE, NL, DE-LU, NO2, DK1, IE(SEM) | fr/no2/dk1 PT60M (fr has one PT15M doc); be/nl/delu PT15M; ie PT30M | `load_{zone}_2024.*`, 17,568 rows each |
| A68 installed capacity per type | same 7 zones | P1Y | `capacity_2024.*`, 79 rows (zone, psr, mapped technology, MW) |
| A75 generation per type | NO2, NO aggregate | PT60M | `{no2,no}_generation_2024.*`, 17,568 rows |
| A72 reservoir filling | NO2, NO | P7D (Norwegian weeks, Mon 00:00 Oslo) | `reservoir_{no2,no}_2024.*`, 53 weekly rows, MWh + inflow proxy |

Resampling to the 30-min UTC grid: PT15M → mean of the two slots; PT60M
→ repeat into both half-hours (both energy-preserving; MW are
market-time-unit averages). The API is UTC end-to-end — the March/October
clock changes required no handling. Curve semantics are parsed
explicitly from each TimeSeries' declared curveType; in the 2024 fetch
every series — flows AND load/generation/reservoir — declared A03
repeat-blocks (review-verified across all 261 documents; hold-forward
exercised on load and generation), so never assume A01 fixed-blocks for
any document type. See the scripts README.

**Gaps and repairs** (full timestamp lists in the build report):
- FR/BE/NL flows and be/nl/delu/fr/no2/dk1 loads: **zero gaps**.
- DK1 flows: 2 slots, interpolated.
- IE(SEM) flows: 718 missing slots (~283 short EirGrid publication
  outages; longest 14 h); 638 interpolated (≤ 2 h rule), 80 filled from
  the NESO GB-side per-link actuals (same physical quantity, GB end of
  the same cables — a flagged repair from the already-adopted primary
  source, not a substitution).
- IE(SEM) load: 592 missing; 518 interpolated, 74 filled with the mean
  of the same half-hour one day earlier/later (preserves diurnal shape;
  longest run 22 h on 2024-09-26/27).
- NO-aggregate solar generation: absent outside May–Nov (platform omits
  empty series; Norwegian winter solar ≈ 0) — zero-filled, 6,092 slots
  recorded. NO2 generation needed **no** repair.

## 3. Per-border 2024 flows and the 33.30 TWh reconciliation

ENTSO-E border series vs the NESO per-link actuals (2024 validation pack,
`docs/notes/2024-validation-pack-report.md` §2; per-link sums mapped:
fr = IFA+IFA2+ElecLink, ie = EWIC+Moyle+Greenlink):

| Border | ENTSO-E import GWh | ENTSO-E export GWh | ENTSO-E net TWh | NESO net TWh | wedge TWh |
|---|---|---|---|---|---|
| FR | 20,664.6 | 782.5 | +19.88 | +19.45 | +0.43 |
| BE (Nemo) | 5,192.2 | 891.5 | +4.30 | +4.16 | +0.14 |
| NL (BritNed) | 3,289.7 | 1,614.0 | +1.68 | +1.59 | +0.09 |
| NO2 (NSL) | 10,188.7 | 273.3 | +9.92 | +9.62 | +0.29 |
| DK1 (Viking) | 5,145.9 | 1,404.1 | +3.74 | +3.66 | +0.08 |
| IE-SEM | 382.6 | 5,556.4 | −5.17 | −5.18 | +0.01 |
| **Total** | | | **+34.34** | **+33.30** | **+1.04** |

The wedge is one-signed and proportional to gross import volume (FR
+2.2 %, NO2 +3.0 %, BE +3.3 %, NL +5.7 % of net, IE ~0 where GB is the
sender): **ENTSO-E border flows are metered at the sending end, NESO at
the GB end; HVDC cable losses land between them.** This is the
irreducible cross-source discrepancy — the two sources agree on direction
99.8–100 % of active periods (§4). Consequence for Stage 5 validation:
the modelled-imports tolerance must name its reference (the Stage 1 pack
uses NESO; keep that), and a modelled-flow comparison against ENTSO-E
would inherit ~2–3 % of systematic loss wedge. Greenlink caveat from the
2024 pack report applies unchanged: commissioning only, −0.00 TWh, inside
the IE aggregate here. Per-asset GB↔FR splitting is **not available**
from ENTSO-E (combined border series only; the GB(IFA)/GB(IFA2)/
GB(ElecLink) virtual zones answer nothing for A11) — per-link stays
NESO/Elexon.

## 4. Flow-direction base rates (evidence for the Stage 5 TBD-DATA)

% of 17,568 half-hourly periods (ENTSO-E net):

| Border | import | export | exact 0 | import (>50 MW) | export (<−50 MW) | idle (±50 MW) |
|---|---|---|---|---|---|---|
| FR | 92.30 | 7.70 | 0.00 | 92.04 | 7.47 | 0.49 |
| BE | 77.97 | 20.84 | 1.19 | 76.15 | 19.25 | 4.60 |
| NL | 64.40 | 35.60 | 0.00 | 49.96 | 28.85 | 21.19 |
| NO2 | 90.98 | 5.85 | 3.16 | 89.77 | 4.70 | 5.53 |
| DK1 | 65.33 | 28.84 | 5.83 | 63.70 | 24.88 | 11.42 |
| IE-SEM | 12.27 | 87.72 | 0.01 | 10.79 | 86.01 | 3.20 |

ENTSO-E vs NESO direction agreement (periods where either source is
>50 MW active — the analyze.py mask): FR 99.86 %,
BE 100.00 %, NL 100.00 %, NO2 99.98 %, DK1 99.82 %, IE 98.12 % — either
source can serve as the direction target; the numbers above are stable
against source choice.

**Pinning guidance for the GB↔FR direction-match acceptance test:** a
constant "always importing" predictor scores **92.3 %** on 2024 GB↔FR.
The acceptance threshold must sit meaningfully above that base rate to
test anything (e.g. a threshold in the low-90s is vacuous; the
discriminating band is the 7.7 % of exporting periods). Number is the
supervisor's call (D2 discipline); the base rate is what this pack pins.

## 5. Sign-test first cut (the anticyclone question)

Pearson r of border net imports vs the GB observed fleet-wide wind CF
(`wind_cf_2024`, 2024 pack):

| Series | r (half-hourly) | r (daily means) |
|---|---|---|
| FR net imports | −0.376 | −0.425 |
| BE net imports | −0.241 | −0.321 |
| NL net imports | −0.058 | −0.026 |
| Continental (FR+BE+NL) | −0.352 | −0.430 |
| NO2 (NSL) net imports | −0.399 | −0.458 |
| DK1 net imports | +0.002 | +0.096 |
| IE-SEM net imports | +0.263 | +0.418 |
| **NO2 hydro generation** (reservoir+RoR) | **−0.087** | **−0.088** |

Mean net import in GB's lowest-wind decile vs all periods: FR 2,897 vs
2,263 MW; BE 683 vs 490; NL 239 vs 191; NO2 1,340 vs 1,129; DK1 489 vs
426; IE −749 vs −589 (GB exports *more* to SEM when GB wind is low —
SEM's wind is correlated with GB's and SEM leans on GB precisely then).

**Contrary finding, pinned before anyone quotes the acceptance test as
written:** docs/04 Stage 5 expects "Norwegian hydro exports uncorrelated
with GB wind; continental exports correlated". At the FLOW level this is
false in 2024: NSL net imports are the *most* wind-anticorrelated series
in the table (r ≈ −0.40/−0.46) — dispatchable reservoir hydro chases GB
scarcity prices, so its flows track GB wind harder than the continent's
do. The claim is true at the RESOURCE level: NO2 hydro *generation* is
uncorrelated with GB wind (r ≈ −0.09), whereas continental supply is
weather-correlated with GB (the anticyclone mechanism), which is why
continental *flows* correlate with GB wind even after price response.
The Stage 5 acceptance test should therefore be formulated as: (a) NO2
hydro resource/generation uncorrelated with GB wind — passes on this
evidence; (b) continental net exports to GB correlated with GB wind
(negative r on imports) — passes; and NOT as "NSL flows uncorrelated",
which would fail against observation. Engine-side, a correct Stage 5
model should REPRODUCE the strong NSL anticorrelation (it emerges from
relative scarcity/price), and its Norwegian-side driver (hydro
availability) should be wind-independent.

## 6. Norwegian hydro evidence (NO2, the NSL counterparty)

Added to this package because the EU weather pack carries no
hydro-capable variables (`docs/notes/eu-pack-box-review.md`).

2024 annual generation (TWh, transmission-metered, A75):

| Type | NO2 | NO (aggregate) |
|---|---|---|
| Hydro water reservoir | 42.56 | 106.18 |
| Hydro run-of-river | 7.09 | 30.19 |
| Hydro pumped (gen) | 1.11 | 2.18 |
| Wind onshore | 4.52 | 14.53 |
| Fossil gas | 0.08 | 1.63 |
| Other (waste/biomass/solar/…) | 0.04 | 0.71 |

- NO2 installed (A68): reservoir 9.82 GW, RoR 1.41 GW, pumped 0.52 GW,
  wind 1.45 GW, gas 0.02 GW. NSL (1.4 GW) is ~14 % of NO2 reservoir
  capacity alone — the zone's hydro dwarfs the link.
- Reservoir filling (A72, weekly): NO2 range 12.48–29.26 TWh stored
  (mean NO2 share of NO storage: 43.7 %); seasonal drawdown to
  late-April minimum, refill to late-autumn maximum, as expected.
- **Inflow proxy** (stated, not measured): weekly ΔStorage + reservoir
  generation energy (B12 only; pumped recharge ignored — NO2 pumped gen
  is 1.1 TWh/yr against 42.6 reservoir). 2024 NO2 total ≈ 46.2 TWh.
  Weekly proxy is noisy (four negative NO proxy weeks); weekly
  generation vs inflow r = −0.34 — generation dispatches against price
  and season, not against contemporaneous inflow. Good enough for a
  seasonal-budget constraint on an external NO2 zone; NOT a
  high-frequency forcing series. If Stage 5 needs true inflow, NVE
  publishes it (separate source, separate licence check).
- A75 publishes no pumping-consumption series for NO zones; pumped
  storage in NO2 must be modelled without an observed pumping trace
  (or from price, or ignored at 0.5 GW).

## 7. D5-relevant observations (zone granularity)

- **NO2 vs NO:** NO2 carries 40 % of Norwegian hydro energy and 44 % of
  reservoir storage, all of NSL, and its load trace (36.1 TWh) is
  available. Internal NO1/NO5→NO2 transfers exist but the GB-relevant
  scarcity signal is NO2's. A single NO2 external zone is defensible for
  Stage 5; the NO-aggregate traces are in the pack if the aggregate is
  preferred. Note from the weather-pack review: the EU ERA5 box covers
  Norway only west of 16°E (NO2 and Fosen in; NO4 out) — irrelevant if
  the Norwegian zone is hydro-driven from ENTSO-E data rather than
  weather-derived, which this package enables.
- **DE-LU:** one bidding zone on the platform (10Y1001A1001A82H), one
  load trace (470.4 TWh), one capacity table. There is no GB↔DE border
  (Viking lands in DK1): DE-LU enters Stage 5 only inside the
  continental aggregate, so single-zone DE-LU is sufficient.
- **IE-SEM:** one zone spanning IE+NI matches both the market (SEM) and
  the platform's border definition (EWIC+Moyle+Greenlink combined).
- **DK1:** included (Viking counterparty); load 22.6 TWh; fleet is
  wind-heavy (4.1 GW onshore, 1.6 GW offshore vs no listed thermal
  beyond 'other') — DK1 behaves "continental-correlated" but its flow
  shows r ≈ 0 with GB wind (Viking commissioned Dec 2023; flows
  price-arbitrage between two wind-heavy zones).
- **Neighbour-zone annual loads (2024):** FR 429.7, DE-LU 470.4,
  NL 115.1, BE 81.0, NO2 36.1, DK1 22.6, IE-SEM 40.9 TWh.

## 8. Installed-capacity technology mapping (lossy entries flagged)

PSR→scenario-technology mapping used in `capacity_2024.*` (empty = no
clean mapping; the column is informational D5 evidence, not a fleet
file): B01→biomass; B02→coal (**lossy**: lignite folded into hard coal);
B04→ccgt (**lossy**: ENTSO-E "Fossil Gas" includes OCGT and CHP; the
scenario splits ccgt/ocgt); B05→coal; B11→hydro, B12→hydro (**lossy**:
the GB scenario's `hydro` has no reservoir/RoR split — for a Norwegian
zone that split is the whole point, so the pack keeps B11/B12 separate
columns); B14→nuclear; B16→solar; B18→offshore_wind; B19→onshore_wind;
B10 (pumped) maps to scenario *storage*, not fleet; B03/B06/B07/B08/B09/
B13/B15/B17/B20 unmapped (oil, waste, geothermal, marine, other — 2024
totals per zone in the pack; largest unmapped block: IE fossil_oil
1.59 GW, DE 'other' groups).

## 9. Data-quality issues (complete list)

1. IE(SEM) flows/load publication outages and repairs (§2) — all
   timestamps in the build report.
2. NO2 flow resolution switches PT15M→PT60M during 2024 (handled
   per-document; recorded).
3. ENTSO-E vs NESO sending-end/receiving-end wedge, +1.04 TWh on the
   year (§3) — systematic, not noise; do not mix sources in one energy
   identity.
4. NO-aggregate solar reported May–Nov only; zero-filled, recorded
   (D5-note column only).
5. A72 weeks are Norwegian local weeks (Sunday 23:00 UTC starts); the
   two year-boundary weeks have no computable inflow proxy.
6. Exact-zero flow periods exist (NSL 3.2 %, DK1 5.8 % of periods —
   outages/commissioning); the 50 MW dead-band rates in §4 are the
   robust direction statistics.
7. IFA/Nemo CC-BY carve-out (§1): publication-grade GB↔FR / GB↔BE
   per-link numbers should cite the Elexon/NESO pack, not this one.

## 10. ADDENDUM (2026-07-03) — FR per-type generation and reservoir traces (Stage 5 A2 remediation)

Added the same day, after the Stage 5 review, per ruling 1 of
`docs/notes/stage-5-review.md`: the A2 direction-match red (82.19 %) is a
remediable input deficiency — the 5-zone scenario models FR hydro as
20.497 GW at a FLAT 0.3518 availability (a constant 7.211 GW,
energy-correct for B11+B12 but shapeless, with B10 pumped generation
excluded entirely), so the model exports to France at FR demand peaks
where real France meets them with peak-shaped reservoir + pumped hydro.
These traces let FR reservoir(+pumped) hydro be wired through the
schema-v4 `energy_budget` machinery exactly as NO2 is. Builder:
`scripts/fetch-entsoe/build_fr.py`; validated by the extended
`validate.py` (exit 0). All sources accessed 2026-07-03 (API as §1).

**Built from cache vs fetched.** The 12 monthly A75 FR documents from the
2026-07-03 calibration fetch already carry every FR TimeSeries at native
resolution — nothing was re-fetched for generation. Exactly ONE new
document was fetched: A72 weekly reservoir filling for FR
(`reservoir_fr_2024.xml`, 7,933 bytes), which the platform DOES publish
for FR — same 53-week A03/P7D MWh structure as the Norwegian series,
French weeks Monday 00:00 Europe/Paris (= Sunday 23:00 UTC winter
anchor). The FR budget therefore gets the same evidence grade as NO2's
(observed storage trajectory), not a generation-window derivation.

**Files** (appended to `entsoe-2024.sha256`, now 36 entries; the
existing 31 verified byte-unchanged before and after — build_fr.py keeps
its own build report so `build_report_entsoe_2024.json` is untouched):
`fr_generation_2024.{parquet,csv}` — 17,568 half-hourly UTC rows x 13
columns (all 12 production types RTE publishes for FR 2024 +
`hydro_pumped_con`); `reservoir_fr_2024.{parquet,csv}` — 53 weekly rows,
`storage_mwh` + `inflow_proxy_mwh`; `build_report_fr_2024.json`. Native
resolution PT60M throughout (each series additionally has one PT15M
Period in the December document, like the FR load trace); every
TimeSeries declares curveType A03 (parsed as declared, per §2).

**FR 2024 annual energies per type** (GWh, transmission-metered; equal
to the independently built aggregation table for every complete non-B10
series — enforced by validate.py):

| PSR | Series | GWh 2024 |
|---|---|---|
| B14 | nuclear | 360,095.4 |
| B11 | hydro run-of-river | 45,893.0 |
| B19 | wind onshore | 41,910.4 |
| B16 | solar | 23,321.0 |
| **B12** | **hydro reservoir** | **17,438.6** |
| B04 | fossil gas | 17,228.2 |
| **B10** | **hydro pumped (gen)** | **6,929.9** |
| — | hydro pumped (pumping consumption) | 6,063.7 |
| B18 | wind offshore | 3,955.5 |
| B01 | biomass | 3,101.0 |
| B17 | waste | 1,847.7 |
| B06 | fossil oil | 1,393.6 |
| B05 | fossil hard coal | 603.0 |

**The budget headline: B12 + B10 = 24.37 TWh** of peak-shaped
dispatchable hydro. Against the scenario's flat model: the observed
B12+B10 output averages 2.77 GW but 4.72 GW over 17–21 UTC (max
9.88 GW), and 46.6 % of its annual energy lands in the two windows where
the A2 mismatches cluster (05–09 + 17–21 UTC). Total observed hydro
including RoR averages 10.48 GW over 17–21 UTC vs the model's constant
7.21 GW — and the model additionally lacks B10's 6.93 TWh entirely. The
`hydro_pumped_con` trace (night/midday-shaped, max 3.59 GW) is the
observed basis for the ruling's peak-shaped treatment of the wedge's
pumping component. B10 generation exceeding pumping consumption
(6.93 vs 6.06 TWh) is consistent with France's partly mixed
pumped-storage fleet (natural inflows into upper basins); the pair is
delivered as observed.

**B10 pair semantics — a correction of record.** RTE publishes FR B10 as
a mutually exclusive generation/consumption pair: of 17,568 slots, all
8,194 gen-missing slots have the consumption series actively pumping,
all 8,796 con-missing slots have generation reporting, and zero slots
are missing on both sides; gen gaps cluster at night/midday, con gaps at
the evening peak. The absent side is therefore 0 MW by construction and
is pair-filled with zero (build_fr.py rule 1; counted in the build
report). Consequence: the `aggregation_gen_2024` FR hydro_pumped figure
of **9,456 GWh — quoted as "9.46 TWh" in the Stage 5 review — is
inflated ~2.5 TWh** by that build's generic interpolation/day-offset
repairs, which invent generation during pumping windows (harmless for
its CF-anchor purpose, where B10 was out of scope and carried a residual
gap count). Any FR B10 energy number, including the budget, must come
from `fr_generation_2024`, not the aggregation table.

**Reservoir trajectory (A72).** 53 weeks; range 1.267 TWh (week of
2024-03-24, the late-winter minimum) to 3.179 TWh (week of 2024-10-13) —
the expected Alpine/Pyrenean cycle, mirroring NO2's late-April minimum /
autumn maximum. Whether RTE's A72 perimeter includes pumped upper basins
is not stated by the platform. The inflow proxy keeps the NO2 B12-only
convention with a stated caveat: FR pumped recharge (6.06 TWh) is
proportionally much larger than NO2's, so the FR proxy is a
seasonal-shape indication only — the budget evidence is the storage
trajectory plus the B12/B10 traces themselves.

**Gaps and repairs** (complete timestamp lists in
`build_report_fr_2024.json`): B10 pair-fills 8,194 (gen) + 8,796 (con)
slots, zero interpolation on the pair; fossil_hard_coal 62 missing slots
(1 interpolated, 61 day-offset); wind_offshore 44 (5 interpolated, 39
day-offset); every other series gap-free at source. Two single-month
auxiliary-consumption fragments (fossil_hard_coal_con 0.141 GWh,
wind_offshore_con 0.205 GWh, both December-only) are excluded and
recorded — not production types, 99.7 % absent. No ACKs.

**Licence.** A75 generation per type (Art. 16.1) and A72 reservoir
filling (16.1.d) are NOT on the CC-BY free-re-use list (§1): this is the
same GTC clause-3.1 internal-use posture as the calibration anchors —
fetch-and-build, pack git-ignored, only checksums committed, no
redistribution; attribution "Source: ENTSO-E Transparency Platform" on
anything derived.

**Non-goals honoured:** no scenario edits, no engine code, no
budget-window computation (the implementer wires `energy_budget` from
these traces), no zones other than FR.

## 11. ADDENDUM (2026-07-03) — FR non-GB cross-border flows (Stage 5 A2 remediation, observed wedge)

Added the same day, per docs/notes/stage-5-review.md ruling 1 (residual
anatomy): the 5-zone scenario closes the FR energy identity with a FLAT
+7.537 GW wedge dominated by FR's net exports to its non-GB neighbours;
the observed series replaces that flat component. This is the final
input-side option before a mechanism-(b) re-pin ruling. Builder:
`scripts/fetch-entsoe/build_fr_flows.py`; fetched by `fetch.py`'s new
`FR_BORDERS` A11 loop (120 monthly documents, per direction, fetched
2026-07-03, zero ACKs, zero 429s); validated by the extended
`validate.py` (exit 0). No wedge arithmetic here — the implementer
re-itemises.

**Border discovery (FR↔IT).** Probed 2026-07-03: the ONLY FR↔IT-* border
the platform serves is **FR↔IT-North** (`10Y1001A1001A73I`). The virtual
IT_North_FR zone (`10Y1001A1001A81J`) returns "no matching data"
acknowledgements in both directions for 2024, and the IT country
aggregate (`10YIT-GRTN-----B`) returns data identical to IT-North (the
same single border serialised twice) — IT-North alone is fetched. The
other four borders are the bidding zones BE, DE-LU, CH, ES.

**Files** (appended to `entsoe-2024.sha256`, now 39 entries; the
existing 36 verified byte-unchanged before and after — build_fr_flows.py
keeps its own build report): `fr_external_2024.{parquet,csv}` — 17,568
half-hourly UTC rows × 16 columns: per border `{b}_imp/{b}_exp/{b}_net`
(average MW, net + = import to FR) plus the headline
`fr_non_gb_net_export_mw` = Σexp − Σimp; `build_report_fr_flows_2024.json`.
Native resolutions: be/es PT15M, delu/ch/it_north PT60M.

**FR non-GB borders, 2024 annual** (TWh; net export = FR → neighbour):

| Border | FR imports | FR exports | FR net exports |
|---|---|---|---|
| FR↔DE-LU | 0.59 | 20.36 | **19.77** |
| FR↔IT-North | 0.09 | 15.01 | **14.92** |
| FR↔BE | 1.99 | 14.55 | **12.56** |
| FR↔CH | 0.83 | 13.23 | **12.39** |
| FR↔ES | 6.31 | 9.25 | **2.94** |
| **non-GB total** | 9.81 | 72.39 | **62.58** |

With the GB border from `flows_gb_entsoe_2024` (+19.88 TWh to GB), FR
total physical net exports 2024 = **82.47 TWh**.

**External cross-checks.** (1) energy-charts.info (Fraunhofer ISE,
independently assembled from the same ENTSO-E A11 physical flows;
queried 2026-07-03, `api.energy-charts.info/cbpf?country=fr`,
calendar-2024 UTC) reproduces EVERY border to 0.01 TWh: BE −12.55,
DE −19.77, IT −14.92, ES −2.94, CH −12.39, UK −19.88, total −82.46 TWh —
including the repaired IT-North Dec 31 (below). (2) RTE's 2024 annual
review ("La France a battu son record d'exports nets d'électricité en
2024", rte-france.com, accessed 2026-07-03) reports a record net export
balance of **89 TWh** (gross exports 101.3 TWh) with a positive balance
on every border: Allemagne-Belgique 27.2, Italie 22.3, Royaume-Uni 20.1,
Suisse 16.7, Espagne 2.8 TWh. Those are the COMMERCIAL (scheduled)
exchanges: the DC/radial borders, where physical = scheduled, match this
pack to 0.2 TWh (GB 19.88 vs 20.1; ES 2.94 vs 2.8), while the meshed AC
borders individually diverge (loop flows: physical DE-LU+BE 32.3 vs
commercial 27.2; IT 14.9 vs 22.3; CH 12.4 vs 16.7) and the totals differ
by the 6.5 TWh physical-vs-commercial wedge (82.5 vs 89). Order of
magnitude confirmed against the work order's 85-90 TWh expectation, with
the convention difference identified rather than hidden. validate.py
pins a 75-95 TWh band on the physical total (justification inline).

**The peak-collapse evidence.** The naive all-year hour-of-day cut shows
NO collapse — mean `fr_non_gb_net_export_mw` is 7,253 MW over 17-21 UTC
vs 7,125 MW all-hours (the 17-21 UTC window across the whole year is
dominated by the export-heavy seasons; DJF 17-21 UTC is 6,610 MW, still
mild). The collapse is DEMAND-conditioned, exactly as the residual
anatomy claims — FR non-GB net exports against FR actual load
(`load_fr_2024`):

| FR load condition | slots | mean fr_non_gb_net_export_mw | share of slots FR net-importing |
|---|---|---|---|
| all hours | 17,568 | 7,125 MW | 4.2 % |
| load ≥ p90 (62.5 GW) | 1,757 | 4,488 MW (−37 %) | 14 % |
| load ≥ p95 (67.0 GW) | 880 | 2,975 MW (−58 %) | 23 % |
| load ≥ p99 (75.6 GW) | 176 | **−528 MW (net importer)** | 58 % |
| top-50 load slots | 50 | −2,467 MW | — |

corr(headline, FR load) = −0.30; every p99 slot falls in January 2024.
At FR p99 load the GB border flips too (mean fr_net −595 MW, i.e. GB
exporting to FR, vs +2,263 MW all-hours). The flat +7.537 GW wedge is
therefore wrong by ~8 GW exactly where the A2 mismatches live: at the
highest FR demand, real France stops exporting to its non-GB neighbours
and becomes a net importer, instead of exporting a constant 7.5 GW.

**IT-North Dec-31 serialisation quirk — documented repair.** The two
FR↔IT-North December documents end 2024 with one PT15M Period per hour
covering only the first quarter-hour (timeInterval start HH:00, end
HH:15, single point; only the 23:00 Period spans its full hour) — the
platform artifact of the Italian market's move to 15-minute MTU on
2025-01-01. The border is PT60M on every other 2024 day and each
degenerate Period carries that hour's value; the strict PT15M rule
(both quarter-hours required) would leave 44 unrepairable slots per
direction on Dec 31. Repair (build_fr_flows.py docstring, applied to
it_north only): a one-quarter-hour PT15M Period is extended to the full
hour it heads — 23 Periods extended per direction, counted and
timestamped in the build report. The energy-charts agreement to
0.01 TWh on IT-North independently confirms the reading. A probe across
all 120 documents found zero degenerate periods on any other border
(be/es carry genuine full-coverage PT15M periods).

**Gaps and repairs** (complete record in
`build_report_fr_flows_2024.json`): eight of ten direction series are
gap-free at source with zero interpolation; flows_fr_it_north_imp/exp
have 2 missing 30-min slots each (both interpolated under the ≤ 2 h
rule) after the 23-Period extension above. Zero ACK direction-months
(every border reported flow in both directions every month), zero
unfilled slots. UTC end-to-end; clock changes a non-event.

**Licence.** A11 physical flows (Art. 12.1.g) ARE on the ENTSO-E CC-BY
4.0 free-re-use list (item 18 — §1). The named carve-outs (IFA, Nemo
Link; BritNed balancing items) concern GB-facing interconnector data
and do not touch FR's borders with BE/DE-LU/CH/IT-North/ES: these five
borders are clean CC-BY 4.0. Attribution on anything re-used or
derived: "Source: ENTSO-E Transparency Platform".

**Non-goals honoured:** no scenario edits, no engine code, no wedge
re-itemisation (the implementer's task), no other zones, no years
beyond 2024.
