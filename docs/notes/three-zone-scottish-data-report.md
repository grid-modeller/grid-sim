# Three-zone Scottish-boundary data package — N-Scotland / S-Scotland / E+W evidence note

Data engineer, 2026-07-04, for the three-zone (B4 + B6) Scottish-boundary
package (design ADOPTED-WITH-EDITS:
`docs/notes/scottish-group-boundary-design-review.md`; scoping
`docs/notes/scottish-group-boundary-scoping.md`, ratified by Richard
2026-07-04). This is the **data** side only — it EXTENDS the committed B6
two-zone pack (`docs/notes/b6-two-zone-data-report.md`); the scenario +
engine package follows separately. **No engine code, no scenario file, no
schema, no `memory/`, no `docs/04` was touched.** All committed GB / cf-gb2
/ b6 manifests are **byte-untouched** (verified: b6.sha256 21/21 OK,
cf-gb2-1985-2024.sha256 481/481 OK after this build); every artefact here is
**additive**.

The six binding obligations of the adjudication banner are honoured
throughout; the two most load-bearing — (item 3) FORBID tuning the N/S
demand split or CF partition to the B4 DA series, and (item 5) quote
DIRECTION + PINNED TOTALS only, never a "B4 effect proper" % — are carried
into every number below as caveats.

## 0. Licence diligence (checked first, per project law / D1)

No new data source is required or fetched. Every figure derives from
sources already licence-cleared in the committed B6 pack (report §1):

| Source | Used for | Licence | Verdict |
|---|---|---|---|
| NESO Day Ahead Constraint Flows and Limits (in pack) | B4 series (`SSE-SP`+`SSE-SP2`) | NESO Open Data Licence | Adopt (already adopted) |
| DESNZ REPD Q1-2026 (in pack) | N/S-of-B4 fleet split (site northings) | OGL v3.0 | Adopt (already adopted) |
| ERA5 GB cutouts (in pack) | N/S-Scotland CF sub-traces | Copernicus / CC-BY 4.0 | Adopt (already adopted) |
| SSEN-North vs SP DNO customer counts (public, cited) | N/S demand share | Company statements (factual counts) | Cited only, not packed |

Attribution carried unchanged: "Supported by National Energy SO Open Data"
(NESO); OGL v3.0 (DESNZ REPD); "Contains modified Copernicus Climate Change
Service information [2024]" (ERA5 traces). Nothing proprietary; no source
substituted silently.

## 1. B4 flow/limit series (deliverable 1)

`scripts/fetch-b6/build.py --three-zone` → `build_b4_series` emits
`data/packs/b6/processed/b4_da_flows_limits.{parquet,csv}` and
`b4_report.json` (separate report — the committed `b6_report.json` is
untouched). The series stitches the two NESO version rows **`SSE-SP`**
(2023-01-01 → 2024-04-20 23:30) and **`SSE-SP2`** (2024-04-21 → present),
which NESO versions mid-year with **zero overlap** (verified: 0 shared
labels). Clock-change, sentinel and exact-duplicate handling are
byte-for-byte the B6 builder's (design-review item 4 requirement): spring
phantom rows dropped, autumn repeat disambiguated first=BST/second=GMT, a
verbatim-duplicated day deduped, gaps left missing never filled.

**2024 validation — reproduces the scoping figures exactly:**

| Metric | This build | Scoping target |
|---|---|---|
| Periods (2024) | **17,280** (288 missing of 17,568; 3 NaN rows) | 17,280 |
| Net DA flow, southward | **15.78 TWh** | 15.78 TWh |
| Binding frequency (flow ≥ 99% limit, non-sentinel) | **35.8%** | 35.8% |
| Median limit | **1,800 MW** | 1.8 GW |

2024 limit quantiles (1/5/25/50/75/95/99): 1300/1500/1650/**1800**/2750/3100/3500 MW.
2024 flow quantiles: −7598/91/1044/**1782**/2548/4402/5342 MW; negative (import) share **3.5%**
(vs B6's 8.2% — B4 flows south far more persistently). Limit sentinels: 42
zero-limit periods (outage states), **zero** ≥9999 no-constraint sentinels
(B4 is never posted as unconstrained, unlike B6's 116). Stitch provenance
(2024): 5,088 periods from `SSE-SP`, 12,194 from `SSE-SP2`.

**Validation-anchor caveat (design-review item 4, MANDATORY on every B4
quote):** B4 is an INTERNAL Scottish flow with only the DA series — **no
annual-outturn cross-anchor** (unlike B6's 17 TWh Energy Trends anchor).
Load-bearing, honest-to-quote-as-validated: **direction** (southward) and
**binding frequency** (35.8% ± band). The net-flow magnitude (15.78 TWh) is
carried as a **wedge budget**, NOT a tight validated number: "DA-only, no
outturn cross-anchor."

## 2. Three-way fleet split (deliverable 2)

`build_fleet_split_3zone` re-partitions the committed Scottish REPD fleet
(Country==Scotland, Operational, op_date ≤ 2024-12-31 — the identical filter
to the committed B6 `build_repd`; 574 sites, **zero missing northings**) at
**N=710k** (the SSEN-T↔SPT / Tealing–Westfield B4 line) into N-Scotland
(≥710k) and S-Scotland (710k→border). Output
`data/packs/b6/processed/b4_fleet_split_3zone.csv`.

Applying the REPD-northing within-Scotland N-shares to the committed
scenario SCO capacities (so bands sum to the validated SCO zone; conventional
plant placed by station):

| Technology | **N-Scotland** (GW) | **S-Scotland** (GW) | **E+W** (GW) | N-share (REPD northing) |
|---|---|---|---|---|
| Onshore wind | **4.11** | 5.97 | 4.32 | 0.408 |
| Offshore wind | **2.88** | 0.19 | 11.63 | 0.939 |
| Solar PV | 0.52 | 0.23 | 17.95 | 0.694 (immaterial) |
| Nuclear | 0.00 | **1.19** (Torness) | 4.71 | by station |
| CCGT | **1.18** (Peterhead) | 0.00 | 28.82 | by station |
| Hydro | 1.62 | 0.07 | 0.21 | 0.958 |
| Pumped storage | **0.74** (Cruachan+Foyers, pinned) | 0.00 | 2.06 | by station |
| Battery | 0.33 | 0.35 | 4.02 | 0.488 |

**Headline: 6.99 GW of 13.15 GW Scottish wind (53.2%, incl. 94% of offshore)
sits north of B4** — behind a median-1.8 GW wall. Raw REPD MW (before scaling
to the scenario): onshore N 4006.7 / S 5819.9; offshore N 2799.3 / S 181.0;
PS N 740.0 / S 0.0.

## 3. Zonal CF sub-traces (deliverable 3)

`scripts/era5-cf/derive_cf_gb3zone.py` splits the committed `sco` zone into
`nsco` (N-Scotland) + `ssco` (S-Scotland) using the **pinned GB derivation
path** (imports `derive_cf.py`; `derive_cf.py` and `derive_cf_gb2zone.py`
byte-unchanged). The **E+W trace = the committed `rgb` trace, byte-unchanged**
(reused, not re-derived). 480 new `nsco_*`/`ssco_*` traces (1985–2024, 3 tech,
Parquet+CSV) + `gb3_cf_report.json` written into `data/packs/cf-gb2/`
(additive); manifest `data/packs/cf-gb3-1985-2024.sha256` (481 entries,
verified).

**Cluster partition within `sco`** (onshore splits cleanly by cluster
latitude across B4; offshore/solar need a within-cluster split):

- onshore `nsco` = highlands + ne_scotland (3.3 GW-w); `ssco` = southern_uplands
  + central_belt + argyll (7.3 GW-w).
- offshore `nsco` = moray_firth + forth_tay·F_N; `ssco` = forth_tay·F_S.
- solar `nsco`/`ssco` = the single scotland point, weight-split 0.694/0.306.

**forth_tay within-cluster split (design-review items 5, 6a/6b — the
straddling cluster).** A pinned CF cluster is a **single pre-averaged 3×3-box
ERA5 point** (verified: `derive_cf.OFFSHORE_CLUSTERS` lists `forth_tay` as one
tuple), so there is **no sub-cluster CF granularity** to descend into without
new per-cell ERA5 work — the scoping's "not new fetching" claim is honoured by
resolving the straddle as a **capacity-weight split with a shared CF shape**
(the review's item-6a resolution, reported not silently claimed as
point-resolved). MW either side, from REPD operational northings:

- **North: 1,251 MW** — Seagreen 1,075 (N749k) + Aberdeen Bay/EOWDC 96.8 +
  Kincardine 49.5 + Hywind 30.
- **South: 7 MW** — Levenmouth (N697k, Firth of Forth).
- → **F_N = 0.994, F_S = 0.006** (pinned, from northings — NOT tuned to B4).

**NnG (Neart na Gaoithe, 450 MW, outer Firth of Forth, south of B4) is
EXCLUDED** — full commercial operation **July 2025** (offshorewind.biz,
2025-07-25), so it is not in the end-2024 validation fleet, exactly as the
committed B6 pack excludes it (report §2 note-a). It is carried as a
**forward wedge**: when included, S-Scotland offshore rises from ~0.18 GW to
~0.46 GW (a *conservative* correction — see §6 wedge).

**Reconciliation, REPD-northing FLEET vs CF-cluster split (design-review
item 6b — the anti-conservative failure class):**

- **Offshore consistent.** REPD offshore N-share 0.939; CF-cluster N-share
  0.994. The 174 MW difference is **Robin Rigg** (Scottish waters, south, but
  in the `irish_sea` CF cluster → `rgb`, a documented ~1.2%-of-fleet
  approximation carried from B6). 181 MW S-Scotland-fleet offshore = 7
  (Levenmouth, CF→ssco) + 174 (Robin Rigg, CF→rgb). Reconciled.
- **Onshore residual quantified.** REPD-northing onshore N-share **0.408** vs
  CF-cluster N-share **0.311** (highlands+ne_scotland only 31% of sco onshore
  cluster weight; REPD counts Perthshire/Angus sites — Griffin etc. — north
  that the coarse cluster boxes place south). Following the B6 DESNZ-vs-cluster
  precedent, the scenario allocates CAPACITY by REPD-northing (0.408) and the
  TRACES by cluster; the resulting within-Scotland GB-energy deviation is
  reported as `adopted_split_sco_energy_rel`: **onshore −1.70% (2024), range
  −2.88%…−1.36% over 40y**; offshore +0.003%, solar 0.000% (both negligible).
  **Sign is NEGATIVE, and its meaning is DIRECTION-DEPENDENT** (the
  distinction the review flagged — the same conflation class the B6 data
  review caught): the −1.70% is a *total-Scottish-energy / B6-EXIT* metric,
  and for the B6 gate the negative sign is genuinely conservative (a real
  improvement over B6's +3.5 pp over-weight — understates B6 export, lower
  bound preserved). **But for the B4 gate it is ANTI-conservative:**
  allocating onshore capacity 0.408 north (vs the cluster's 0.311) raises
  northern generation per unit Scottish capacity by ~+31%, so more
  generation sits behind the B4 wall and the split **overstates** B4
  binding. This does not breach the lower-bound duty (B4 magnitude is
  quoted only as a DA-anchored wedge; the headline is direction + pinned
  totals, never a "B4 effect proper"), but the per-gate signs are opposite
  and must be stated as such. (Mechanism: `nsco` onshore CF 0.249 < `ssco`
  0.303, so shifting capacity share north reduces total sco onshore energy
  — hence conservative for the B6 total — while concentrating capacity
  north raises the generation the B4 gate must clear.)

**Reconstruction identity vs the committed `sco` traces** (transitively vs
GB): `w_nsco·nsco + w_ssco·ssco == sco` verified per year per tech — **max
residual 2.4e-07 over 40y** (offshore 1.8e-07, onshore 2.4e-07, solar 3.0e-08),
tolerance 1e-5 (float32-cutout arithmetic, ~50× headroom, cf-gb2 precedent).
The split loses no information.

**40-year mean sub-zone CFs:** onshore nsco 0.249 / ssco 0.303; offshore nsco
0.354 / ssco 0.358 (near-identical — offshore N/S level is immaterial); solar
nsco = ssco 0.078 (single point). Worst wind year 2010 in both sub-zones
(nsco onshore 0.204) — GB droughts carry through, consistent with the
robustness premise.

## 4. N/S-Scotland demand split (deliverable 4)

**PINNED PRE-RUN from cited evidence, NOT tuned to the B4 DA series
(design-review item 3, HARD).** No open half-hourly N-of-B4 demand series
exists, so a **flat level-only share** is used, aligned to the B4 electrical
boundary (which IS the SSEN-T↔SPT interface):

- **N-Scotland (SSEN area) = 33% of Scottish demand ≈ 3.33% of GB;
  S-Scotland (SP area) = 67% ≈ 6.77% of GB** (summing to Scotland's committed
  10.1% of GB, `b6` report §4).
- **Basis:** SSEN-North distribution serves **740k** of Scotland's ~2.74m
  electricity customers (**27%** by customer count; SSEN / SP Energy Networks
  public figures) — this 27% is the one DERIVED figure. The step from 27% to
  the adopted **33% is a STATED JUDGMENT, not a derived number** (an uplift
  for the north's higher per-customer consumption — off-gas-grid electric
  heating, Highlands & Islands — pending the P114 `_P`/`_N` half-hourly
  shape pin that would replace it in v2). Stated bracket **27% (derived,
  customer count) – 33% (judgment, energy)**; the adopted 33% matches the
  ratified scoping's "SSEN ≈ ⅓ of Scottish demand, ~3% of GB" (§1/§3).

**Why the crude split is tolerable (design-review item 3b, stated not
implicit):** N-Scotland is heavily export-dependent — mean demand ~1 GW vs
~7 GW northern wind swings — so a ±few-% GB demand error (~0.35 GW) is
second-order against the generation term for the B4 flow. **Bias direction:**
under-stating N demand would over-state B4 export (anti-conservative); pinning
to ⅓ rather than the 27% customer-count floor is the more conservative choice.

**v2 upgrade flagged (item 3(w1)):** Elexon **P114 GSP-group** half-hourly
(`_P` = North Scotland/SSE ≈ north-of-B4; `_N` = South Scotland/SP) under the
BSC Open Data Licence would give the true N/S demand **shape** (absent here),
not just the level. Not fetched in v1 (assembly cost disproportionate to a
robustness study); it is the pinned check on the flat share.

## 5. Cruachan sensitivity (deliverable 5)

Cruachan pumped storage (**440 MW / 7.1 GWh**, Loch Awe, **Y=728,674 —
~18.7k north of the 710k line**) sits on the Argyll fringe where the
horizontal proxy is weakest; SSEN-connected, so N-Scotland is defensible on
both northing and electrical grounds — but PS placement feeds the headline
storage-sensitivity finding, so it is reported **both ways** for the scenario
package to carry (design-review item 3/Edit 3):

| Configuration | N-Scotland PS | S-Scotland PS |
|---|---|---|
| **Cruachan in N (PINNED v1)** | **740 MW** (Cruachan 440 + Foyers 300) | 0 MW |
| Cruachan in S (sensitivity) | 300 MW (Foyers only) | 440 MW (Cruachan) |

Foyers (300 MW / 6.3 GWh, Y=820,941) is unambiguously north. Cruachan energy
7.1 GWh; Foyers 6.3 GWh (station data, b6 report §2).

## 6. Anchors, wedges, and what could not be sourced (deliverable 6)

**Validation anchors (design-review items 3/4/5):**
- **B4 link:** DA-only — direction + binding frequency (35.8%) are the
  load-bearing validated gates; net 15.78 TWh is a wedge budget with
  "DA-only, no outturn cross-anchor." **No B4 outturn cross-anchor exists.**
- **B6 link:** SCOTEX, **unchanged** from the committed model (net 22.63 TWh,
  23.6% binding; 17 TWh Energy Trends outturn anchors the B6 EXIT only, with
  the ~2 TWh DA-vs-outturn wedge). Not touched by this package.
- **Prohibition carried (item 5):** the three-zone model may quote DIRECTION
  + PINNED TOTALS under stated conventions only — **no "B4 effect proper" %,
  no B4-vs-B6 decomposition** (single-pass hub-staleness; the Stage-7 LP is
  the resolver). This note quotes no such decomposition.

**Offshore-commissioning wedge on B4 (~19%, REPORTED not tuned out — item
3(w2)):** 94% of Scottish offshore is north of B4, and the end-2024 fleet
convention (constant end-2024 capacities over the full year) overstates 2024
northern offshore ENERGY because Moray West / Seagreen / NnG commissioned
through 2024–25. The B6 pack sized this at ~3 TWh of Scottish-offshore
overstatement; against B4's 15.78 TWh DA anchor that is a **~19% wedge**, with
**no outturn cross-anchor to catch it**. The B4 net-flow tolerance must absorb
it (plus the demand-split term) as a decomposed budget; it is NOT closed by
adjustment. Direction: **overstates** modelled B4 binding (anti-conservative
for the magnitude — hence magnitude is quoted only within the wedge).

**B5 disposition (deliverable 6):** B5 (3.9 GW, Denny North–Lambhill, within
SPT, between B4 and B6) has **no separate NESO named series** (cost dataset
has B4/B6 not B5; DA dataset has no clean B5 row). **Folded into the
S-Scotland copper-plate.** **Bias direction: toward UNDER-stating constraint**
— folding B5 in assumes no constraint between the B4 exit and the B6 entry, so
the model lets more energy flow freely within S-Scotland than reality → the
three-zone result **remains a LOWER BOUND** (design-review item 1 failure-mode
A; the "adequate" framing was struck by Edit 1). It is bracketed above by B4
and below by B6 and has no open anchor to model it independently.

**Could not be sourced openly (unchanged from scoping §6, restated):** B5
flow/limit/cost (folded, above); ETYS B4/B5/B6 capabilities pinned to a
fetchable artefact (JS-rendered page — condition 4 open, now covers B4/B5);
per-boundary curtailed-wind volume (BOALF assembly, v2); half-hourly N/S-of-B4
demand SHAPE (P114 `_P`/`_N`, v2); the precise B4 line (N=710k proxy, stable
±10k, named-station validated).

## 7. Deliverables and reproduction

| Artefact | Path | Manifest |
|---|---|---|
| B4 series (Parquet+CSV) | `data/packs/b6/processed/b4_da_flows_limits.*` | `data/packs/b4.sha256` |
| B4 + 3-zone fleet report | `data/packs/b6/processed/b4_report.json` | `data/packs/b4.sha256` |
| 3-zone fleet split | `data/packs/b6/processed/b4_fleet_split_3zone.csv` | `data/packs/b4.sha256` |
| N/S-Scotland CF traces 1985–2024 | `data/packs/cf-gb2/{nsco,ssco}_*_cf_*.{parquet,csv}` | `data/packs/cf-gb3-1985-2024.sha256` |
| 3-zone CF report | `data/packs/cf-gb2/gb3_cf_report.json` | `data/packs/cf-gb3-1985-2024.sha256` |
| B4 series builder | `scripts/fetch-b6/build.py --three-zone` | — |
| N/S CF derivation | `scripts/era5-cf/derive_cf_gb3zone.py` | — |

```
V=~/.local/share/grid-sim/era5-venv/bin/python
$V scripts/fetch-b6/build.py . --three-zone      # b4 series + 3-zone fleet split
$V scripts/era5-cf/derive_cf_gb3zone.py .         # 40-year nsco/ssco sweep (~1 min)
cd data/packs && shasum -c b4.sha256 && shasum -c cf-gb3-1985-2024.sha256
# additive proof (must still pass):
shasum -c b6.sha256 && shasum -c cf-gb2-1985-2024.sha256
```

Retrieval dates: all NESO/DESNZ/ERA5 inputs 2026-07-04 (unchanged — no new
fetch; the B4 series is a re-slice of the DA file already pinned by
`b6.sha256`). E+W CF = committed `rgb` traces (`cf-gb2-1985-2024.sha256`,
unchanged).
