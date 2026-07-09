# B6 two-zone data package — Scotland / rest-of-GB evidence note

Data engineer, 2026-07-04, for the post-Stage-5 GB two-zone / B6 work
package (memory/project-state.md, ratified 2026-07-03; promoted VITAL by
Richard 2026-07-04). REVISED 2026-07-04 after adversarial review
(docs/notes/b6-two-zone-data-review.md, ACCEPT-WITH-NOTES): pre-commit
conditions 1-3, 5, 6 are incorporated — the onshore split convention is
CORRECTED (the delivered cluster-share recommendation was
anti-conservative; §3), the DA-flow semantics are labelled as
interpretation (§6), and the review's §6 link-convention ruling
supersedes this note's original §8.3-4 (transcribed verbatim in §8). This note delivers the evidence side: the zonal
fleet split, zonal CF traces, the demand-split evidence, B6 capability
numbers, and the observed-boundary validation anchors. The scenario and
engine work follow separately; **no scenario file and no engine code is
touched by this package** (ADR-7 zone+link schema is what the scenario
package will use; ADR-12's constraint-cost approximation is what the
two-zone model supersedes for this study).

Deliverables and machinery:

| Item | Where |
|---|---|
| Zonal CF traces 1985–2024 (sco / rgb × onshore/offshore/solar) | `data/packs/cf-gb2/` (480 files + `gb2_cf_report.json`), manifest `data/packs/cf-gb2-1985-2024.sha256` (481 entries, verified) |
| Derivation script (pinned GB method, GB path byte-unchanged) | `scripts/era5-cf/derive_cf_gb2zone.py` |
| B6 boundary series, constraint costs, fleet-split evidence | `data/packs/b6/` (12 raw + 9 processed files incl. `b6_report.json`), manifest `data/packs/b6.sha256` (21 entries, verified) |
| Fetch/build scripts | `scripts/fetch-b6/` (fetch.py, build.py, README, requirements) |

## 1. Licence diligence (checked FIRST, per project law / decision D1 posture)

Every source below was licence-checked before fetching; nothing
proprietary is used and no source was substituted silently.

| Source | Licence | Verdict |
|---|---|---|
| NESO Data Portal datasets: Day Ahead Constraint Flows and Limits; Thermal Constraint Costs; Constraint Breakdown Costs and Volume; Interconnector Register | **NESO Open Data Licence** (per-dataset, confirmed in each dataset's CKAN metadata, `license_title = "NESO Open Data Licence"`). OGL-v3-based: worldwide, royalty-free, perpetual, non-exclusive; commercial re-use and redistribution permitted; attribution "Supported by National Energy SO Open Data" required. NESO may revise the licence without notice (monitor). https://www.neso.energy/data-portal/neso-open-licence, retrieved 2026-07-04 | **Adopt** |
| DESNZ REPD quarterly extract (April 2026 edition) | Open Government Licence v3.0 (stated on the gov.uk publication page) | **Adopt** |
| DESNZ Regional Renewable Statistics 2003–2024 (installed capacity workbook) | OGL v3.0 | **Adopt** |
| DESNZ Energy Trends special article, "Electricity generation and supply in Scotland, Wales, Northern Ireland, and England, 2020 to 2024" (18 Dec 2025) | OGL v3.0 (gov.uk publication) | **Adopt** (cited numbers only; not fetched into a pack) |
| NESO ETYS (Electricity Ten Year Statement) web pages / ETYS boundary datasets | NESO publication; the ETYS data-portal datasets carry the NESO Open Data Licence | **Adopt** (cited numbers) |
| ERA5 (existing GB cutouts) | Copernicus / CC-BY 4.0; attribution "Contains modified Copernicus Climate Change Service information [2024]" carried | **Adopt** (already-committed pack, D1) |
| Elexon P114 settlement data (GSP-group half-hourly demand — §5 upgrade path) | **BSC Open Data Licence / BSC Public Data Licence** since Elexon's P114 licence change (previously restrictive); requires a free Elexon Portal account to download | **Open — recommended upgrade path, not fetched in v1** |
| Modo Energy, UKWED/RenewableUK | Proprietary / restricted | **Not used** for any new number (the reference scenario's existing 4.7 GW battery citation to Modo is carried, not re-derived; UKWED remains only the honesty label on the pinned GB cluster weights) |

## 2. Zonal fleet split, end-2024 (every capacity cited)

Zone definition: **sco** = Scotland; **rgb** = England + Wales.
Reference GB capacities are the validated `scenarios/gb-2024-reference.toml`
values; the split evidence is independent of them, so both the share and
the implied GW are given. Primary for renewables is the DESNZ Regional
Renewable Statistics MW2024 sheet (all-size, end-2024, OGL; extract in
`data/packs/b6/processed/desnz_regional_capacity_mw2024.csv`); REPD
(≥150 kW site-level, filtered to Operational ≤ 2024-12-31; extract in
`repd_end2024_by_country_tech.csv`) corroborates the large-site
technologies. REPD filter limitation (review condition 5): 26
Operational-status rows carry NO Operational date and are excluded by
the date filter — 586 MW GB (610 MW incl. NI): England solar 296 MW,
England battery 206 MW, Scotland onshore 67 MW, minor remainder;
immaterial (<0.5 pp on any share) but counted, here and in
`b6_report.json`.

| Technology | GB ref (GW) | Scotland share | sco / rgb (GW, share × GB ref) | Evidence |
|---|---|---|---|---|
| Onshore wind | 14.4 | **70.0%** | 10.08 / 4.32 | DESNZ MW2024: Scotland 10,281 MW, England 3,111, Wales 1,302 (GB 14,694 MW — independently close to the 14.4 GW reference). REPD ≥150 kW: 70.7%. Trace cluster share: 73.6% (§3 mismatch discussion) |
| Offshore wind | 14.7 | **20.3–25.6%** (see note a) | 3.07 / 11.63 at 20.9% (trace share) | REPD end-2024 operational: Scotland 2,980 MW of 14,679 = 20.3% (Seagreen 1,075, Moray East 950, Beatrice 588, Robin Rigg 174, Hywind 30, Kincardine 50, and smaller sites — NnG contributes 0 MW under the Operational-date filter, its REPD Operational date being post-2024; Energy Trends lists NnG among the farms that came online during 2024, reinforcing the bracket). DESNZ MW2024: Scotland 4,077 of 15,916 = 25.6% (counts Moray West 882 and other 2024-commissioning capacity in full). Trace cluster share: 20.9% |
| Solar PV | 18.7 | **4.0%** | 0.75 / 17.95 | DESNZ MW2024 (all-size, incl. rooftop): Scotland 722 MW, England 15,633(+21), Wales 1,533 → 4.03% of GB 17,909 MW. REPD (ground-mount ≥150 kW only): 0.9% — the gap is rooftop, DESNZ is the correct basis. Trace cluster share: 2.7% |
| Nuclear | 5.9 | **20.2%** | 1.19 / 4.71 | Civil site list: Scotland = Torness only (~1,190 MW net, 2 AGRs; Hunterston B ceased generation 2022). rgb: Heysham 1+2, Hartlepool, Sizewell B |
| CCGT | 30.0 | **3.9%** | 1.18 / 28.82 | Scotland's only transmission CCGT is Peterhead, 1,180 MW (SSE Thermal; T-4 capacity-market listing). All other CCGT England/Wales |
| OCGT | 1.0 | **~0%** | 0 / 1.0 | No transmission-connected OCGT of note in Scotland (BM-visible OCGT fleet is England/Wales) |
| Biomass (transmission) | 3.5 | **0%** | 0 / 3.5 | The reference 3.5 GW is Drax (2.6) + Lynemouth (0.42) + MGT Teesside (0.30) + minor units — all England. (REPD "Biomass dedicated" Scotland 238 MW ≈ 6.9% is embedded-scale plant, e.g. Markinch, Stevens Croft — not part of the reference roster) |
| Hydro (non-PS) | 1.9 | **88.8%** | 1.69 / 0.21 | DESNZ MW2024 Hydro: Scotland 1,676 MW, Wales 168, England 43 (GB 1,888 MW — matches the 1.9 GW reference). REPD large+small hydro corroborates (88.9% / 90.2%) |
| Pumped storage | 2.8 GW / 24 GWh | **26.2% power / ~55.8% energy** | 0.74 GW, 13.4 GWh / 2.09 GW, 10.5 GWh | Station data (reference scenario citations): Scotland = Cruachan 440 MW/7.1 GWh + Foyers 300 MW/6.3 GWh; rgb (Wales) = Dinorwig 1,728 MW/9.1 GWh + Ffestiniog 360 MW/1.4 GWh. REPD PS country split confirms 740/2,088 MW |
| Battery | 4.7 GW / 6.6 GWh | **14.6%** | 0.68 / 4.02 | REPD end-2024 operational battery: Scotland 488 MW of 3,353 MW GB = 14.55%. Basis caveat: REPD's GB battery total (3.35 GW) undercounts the Modo-cited 4.7 GW operational fleet; the REPD **share** is the best open zonal evidence and is applied to the reference 4.7 GW |
| Interconnectors | 10 links | **Moyle only** | 0.5 GW / all others | NESO Interconnector Register (connection sites): Moyle lands at Auchencrosh 275 kV (Ayrshire) → **Scotland**. All other built links land in England/Wales: NSL at Blyth (England, NOT Scotland), Viking at Bicker Fen, EWIC at Deeside (Wales), Greenlink at Pembroke (Wales), IFA/IFA2/ElecLink/BritNed/Nemo in southern England. Consequence: in the two-zone model, 9.3 of 9.8 GW interconnection attaches to rgb; Scotland's only external link is Moyle 0.5 GW (0.45 GW import TEC until Aug 2022 uprate — now 0.5/0.5) |

Note (a) — the offshore bracket: REPD counts only capacity with an
Operational date ≤ 2024-12-31 (Moray West's 2024-commissioned share is
partially captured), DESNZ MW2024 counts end-of-year installed capacity
including full 2024 commissioning. The calibrated traces represent the
2024 energy-matching fleet (constant end-2024 denominators), whose
effective Scottish share sits at the lower end; the trace cluster share
(20.9%) is consistent with that. Recommendation in §8.

## 3. Zonal CF traces (1985–2024) — derivation, convention, residual

`scripts/era5-cf/derive_cf_gb2zone.py` (imports `derive_cf.py`
functions; **the GB derivation path is byte-unchanged and all committed
GB manifests are untouched** — verified: only new files under
`data/packs/cf-gb2/` were written). The pinned GB clusters are assigned
to zones whole; weights/coordinates are byte-identical to the GB lists.
The pinned 2024 GB calibration factors (offshore 0.8975, onshore
1.0395, solar 0.8837; re-derived at full precision with the Phase B
drift guard — confirmed) apply unchanged to both zones.

**Calibration convention, stated explicitly:** the GB anchors are
national (NESO/Elexon 2024 energies); no zonal anchor of comparable
quality exists (the onshore/offshore split of observed GB wind is
itself inferred, not metered — era5-cf-2024-report limitation 5). The
traces are zone-internal weighted means, independent of how GB capacity
is later split, and two split conventions are verified/quantified:

1. **Derivation-correctness identity (cluster-weight shares):**

       w_sco · sco_trace + w_rgb · rgb_trace = gb_trace

   with w_sco = 0.209150 (offshore), 0.736111 (onshore), 0.026738
   (solar). Verified per year per technology against the committed
   `data/packs/cf/` traces: **max per-period residual 3.0e-07**
   (float32 cutout arithmetic — the two aggregation orders round
   differently at single precision; evidence-based tolerance 1e-5,
   ~50× above the observed residual), **max annual-energy residual
   1.1e-07 relative**. Clipping at 1.0 never bites (max possible wind
   CF 0.855 × 1.0395 = 0.889), so the identity is exact up to
   rounding. This proves the zone split loses no information; it is
   NOT the adopted onshore capacity split (see 2).

2. **Adopted scenario split (supervisor decision 2026-07-04 on review
   condition 1):** onshore splits by the OBSERVED DESNZ end-2024
   capacity share, **Scotland 0.6997** (MW2024: 10,281.06/14,693.90);
   offshore and solar keep the cluster shares (0.2092 — inside the
   20.3–25.6% observed bracket; 0.0267 — immaterial at a 0.75 GW
   fleet). Why: under the cluster share (73.6%) the model's Scottish
   share of GB onshore ENERGY is **73.4% (2024) vs observed 69.8%**
   (DESNZ generation workbook; reviewer-measured) — the delivered
   package OVERSTATED Scottish onshore generation by ~+3.5 pp of share
   (~1.3 TWh at the reference 14.4 GW), anti-conservative for the
   Q2/Q10 bounding claims. Under the adopted split the model gives
   **69.7% ≈ observed 69.8%**, at a quantified GB-energy cost (the
   weighted zone sum no longer reproduces the GB trace exactly):
   **+0.05% (2024), +0.22% (40-year mean CFs), max +0.54% in any
   single year (1985)** — reported per year in `gb2_cf_report.json`
   (`adopted_split_gb_energy_rel`) and sanity-bounded at 1% in the
   derivation script (2× the observed 40-year max; a breach means
   share or trace drift, not a tolerable wobble).

Zone annual mean CFs over the 40-year record (from
`gb2_cf_report.json`): onshore sco 0.2865 / rgb 0.3046; offshore sco
0.3538 / rgb 0.3484; solar sco 0.0780 / rgb 0.0904. Worst zonal wind
year: 2010 for both zones (sco onshore 0.2301) — the GB drought years
carry through to both zones, consistent with the drought periods being
network-unconstrained (the robustness-purpose premise).

**Stated limitation — the onshore CF ordering (direction history
corrected at review).** The model's rgb onshore mean CF (0.305)
slightly EXCEEDS sco (0.287), whereas observed 2024 standard load
factors put Scotland above England (0.2684 vs 0.2490,
reviewer-verified from DESNZ) — the spatial-weights artefact is real
(five rgb points sit on windy coastal/upland 3×3 ERA5 boxes; the large
Scottish weights sit on inland boxes) and inherited from the pinned GB
weights, unfixable here without touching the GB derivation. **As
delivered, this note claimed the artefact made the package
conservative; that was WRONG under its own recommended cluster-share
split** — the +3.6 pp capacity overweight dominated the CF-ordering
understatement, and the package overstated Scottish onshore energy
(anti-conservative for Q2/Q10). Corrected by adopting the DESNZ split
(convention 2 above), which matches the observed zonal energy split;
the residual caveats that REMAIN are: the CF-ordering artefact itself
(zone traces' relative levels, now second-order), the offshore
2024-commissioning wedge (observed 2024 offshore ENERGY share 14.7% vs
model ~21.5% — substantially the deliberate end-2024-fleet full-year
convention plus DESNZ regional-assignment conventions; up to ~3 TWh of
2024-specific Scottish offshore overstatement lands on the
flow-validation anchors and must be quantified as a named wedge when
tolerances are set), and the deferred tolerances (§8).

## 4. Zonal demand split

Primary evidence (DESNZ Energy Trends special article, 18 Dec 2025,
OGL): 2024 UK electricity consumption shares — Scotland 9.8%, England
81.2%, Wales 6.1%, NI 2.9%. On a GB basis: **Scotland = 9.8 / 97.1 =
10.1% of GB consumption**. The shares "did not vary much from 2023 and
have been relatively consistent across the reported data series" (ibid.)
— which is what makes a flat share defensible for v1. Corroboration:
DESNZ subnational consumption statistics 2024 (OGL): Scotland 21.7 TWh
metered consumption of GB 249.2 TWh = 8.7% at the meter; the gap to
10.1% is losses/definitional (the Energy Trends shares are the
consumption basis consistent with its own generation/transfer ledger).

**Recommendation:** v1 splits the D3 underlying-demand trace by a flat
factor — Scotland 10.1%, rest-of-GB 89.9% — applied per period.
Stated limits of the flat convention: (i) it ignores Scotland's
slightly peakier winter shape (higher electric-heating share in the
north of Scotland); (ii) losses are ascribed pro-rata although northern
networks run higher per-unit losses; (iii) station-transformer load
(the +0.667 GW wedge) splits pro-rata with no site evidence. All three
are second-order against a 10% share.

**Half-hourly zonal demand DOES exist openly** (the upgrade path):
Elexon P114 settlement data at GSP-group level (`_N` = South Scotland/
SP, `_P` = North Scotland/SSE) is half-hourly from July 2014 and now
sits under the BSC Open Data / Public Data Licences (free Elexon Portal
account required; per-settlement-day flat files, ~heavy assembly).
Recommended as a v2 refinement and as the validation check on the flat
share (it would also give the observed Scottish demand SHAPE, not just
the level). Not fetched in v1: assembly cost is disproportionate to the
robustness-study purpose.

## 5. B6 boundary capability

Nameplate/planning vs operational, kept distinct as the sources do:

- **Planning capability (ETYS):** B6 boundary capability **6.7 GW**,
  limited by thermal constraint on the Harker–Moffat 400 kV circuit
  (NESO ETYS "Scottish boundaries" page, retrieved 2026-07-04). ETYS
  2024 year-round analysis sees overloads above ~5.1 GW (summer) /
  ~5.8 GW (winter) on the critical Eccles–Torness / Eccles–Stella West
  corridor. Context: B4 (SSEN→SPT) 4.0 GW, B5 (north–south SPT) 3.9 GW.
- **Operational day-ahead limits (observed, the pack's series):**
  calendar-2024 SCOTEX export limit — median **4,100 MW**, IQR
  2,700–5,500, p95 6,350, p99 6,400 MW; 53 zero-limit periods (outage
  states) and 116 periods at the 10,000 MW no-constraint sentinel
  (0.3% and 0.7% respectively — treat both as sentinels, not data).
  2023 median was 5,000 MW and 2025's 3,850 MW (same sentinel-inclusive
  basis as the 2024 median; review condition 6): the operational limit
  is outage-driven and materially BELOW the ETYS planning capability
  most of the time (median ≈ 61% of 6.7 GW).
- **Import (England→Scotland) direction:** the only published limit
  series found is the superseded Year Ahead Constraint Limits dataset
  (NESO Open Data Licence), whose "B6 import – HARETORIM" column for
  2021-22 runs 2,150–3,500 MW (median 3,500). No 2024 import-limit
  series is openly published; import flows are rare (8.2% of 2024 DA
  periods show negative flow).

**Modelling convention recommendation:** the two-zone link should take
the **observed half-hourly DA limit series as the export capability for
the 2024 validation year** (it is openly published, half-hourly, and is
what actually bound the system), with sentinels (0 kept as real outage
state only if corroborated; ≥9,999 replaced by the ETYS 6.7 GW cap) —
the scenario package decides the sentinel rule and must state it. For
scaled/future scenarios and the 40-year runs, a **single value per
direction**: export 4.1 GW (the 2024 median operational limit) with
2.7/5.5 GW (IQR) sensitivity brackets and 6.7 GW (ETYS planning) as the
upper bound case; import 3.5 GW. A time-series capability for non-2024
years is NOT recommended: the limit is outage-scheduling, not weather,
and no published basis exists to synthesise it.

## 6. Observed B6 flows and constraint outturn (the validation anchors)

- **B6 flow series** (`data/packs/b6/processed/b6_da_flows_limits.*`):
  half-hourly SCOTEX limit + flow, 2023-01-01 → present (NESO retains
  ~3.5 years in the public file; earlier data available from the NESO
  OpenData team on request — a named gap, not fetched). **Semantics —
  NESO's wording vs this package's interpretation, kept distinct
  (review condition 3).** NESO documents the flow as "the forecast
  position after Day Ahead energy scheduling", a "power flow forecast
  … based on the next day's wind forecast, generation dispatch and
  demand forecast … modelled using power system software" — it never
  uses the word "unconstrained". This package INTERPRETS the series as
  the pre-constraint-action (unconstrained) boundary flow; the reading
  is supported by the data itself — flow exceeds the limit in 23.6% of
  2024 periods, impossible for a constrained/settled series — and was
  ruled sound at review, but it is an interpretation, not NESO's
  language, and it is NOT settled outturn. Under that interpretation
  it is the right anchor for a copper-plate-then-constrain model:
  compare the model's pre-constraint B6 flow to it (correlation,
  distribution), and the model's binding frequency to the observed
  **23.6%**. 2024 flow stats: median 2,373 MW southward, p95 6,781,
  p99 8,960 (flow > limit = anticipated constraint action); negative
  (import) 8.2% of periods; net day-ahead flow energy **22.6 TWh
  southward**.
- **Annual outturn cross-anchor:** Energy Trends (Dec 2025, OGL):
  Scotland transferred **17 TWh to England** in 2024 (+2.5 TWh to NI
  via Moyle). **Reviewer decomposition of the 22.6 vs 17 TWh wedge
  (independently re-verified on this pack):** clipping the DA flow at
  the DA limit on non-sentinel periods removes **3.51 TWh → 19.12
  TWh** (the anticipated-constraint-action component); the residual
  **~2.1 TWh** is DA-forecast error + ledger definitions + the 354
  missing 2024 periods (~2% of the year). The two anchors legitimately
  bracket the model — modelled UNCONSTRAINED B6 export ≈ 22–23 TWh,
  modelled CONSTRAINED export ≈ 17 TWh — and the scenario package's
  gate (ii) must carry an **irreducible wedge of order 2 TWh** from
  the DA-vs-outturn basis alone.
- **Constraint costs per boundary**
  (`boundary_thermal_costs_daily.*`, daily, FY2021-22 → 2026-27):
  calendar-2024 thermal constraint costs — **SCOTEX (B6) £90.5m,
  SSE-SP (B4) £366.8m, SSHARN (B7) £68.5m**, ESTEX £49.0m, SEIMP £4.7m,
  SWALEX £0.04m. The Q2/Q10 comparison target for "Scottish wind
  curtailment cost" is the Scottish-boundary group (B4+B6+B7 = £525.8m
  calendar 2024), NOT B6 alone — see §7 surprise 1.
- **Constraint volumes** (`constraint_breakdown_daily.*`): calendar-2024
  GB-wide thermal constraint volume **−11.0 TWh** (turn-down), cost
  £1,482.5m; voltage £201.6m/+4.5 TWh; inertia £43.1m. No per-boundary
  or per-fuel volume is openly published as a dataset. Per-unit wind
  curtailment volumes ARE computable from Elexon BOALF bid-offer
  acceptance data (BSC open data; substantial assembly) — named as the
  v2 path if Q2/Q10 needs curtailed TWh rather than cost.

## 7. Gaps, surprises, and honesty items

1. **B4 > B6 in 2024 costs** — the headline surprise: the intra-Scotland
   B4 (SSE-SP, £367m) cost 4× the Anglo-Scottish B6 (£90.5m) in
   calendar 2024. A two-zone model with a single B6 link CANNOT
   represent intra-Scotland constraints; it will attribute the whole
   Scottish constraint phenomenon to one boundary. The comparison
   target must therefore be the Scottish boundary GROUP (£526m), and
   the model's B6-only geometry stated as aggregating B4/B5/B6 into one
   effective link. **This item's original recommendation ("tune the
   effective link to the group") was REJECTED at review** — it would
   double-book B4/B5 congestion onto the border and break every flow
   anchor. The binding convention is the review's §6 ruling, transcribed
   verbatim in §8 below: the link is B6, the group cost (£525.8m) is
   context only, and model curtailment outputs are quoted as a LOWER
   BOUND on the Scottish constraint phenomenon.
2. **The six-boundary cost set is ~39% of GB thermal costs** (£579.5m of
   £1,482.5m calendar 2024): NESO's per-boundary dataset covers only
   "significant" named boundaries. Do not total it as if complete.
3. **DA flow/limit series starts 2023-01-01** in the public file;
   earlier history is on-request only. The validation window for the
   B6 anchor is therefore 2023–2025, and 2024 is fully inside it
   (17,214 of 17,568 periods present; 354 missing — 5 whole missing
   days + 3 partial around 2024-05-21/23; 3 NaN rows; gaps left
   missing, never filled).
4. **Data-quality warts in the NESO DA file, all handled and counted in
   `b6_report.json`:** local wall-clock labels on a fixed 48-row grid
   (spring phantom rows dropped: 4 across the file; autumn repeated
   hour disambiguated first=BST); one whole day (2025-08-12) duplicated
   verbatim (48 value-identical rows deduped); limit sentinels 0 and
   10,000.
5. **Offshore split is a bracket (20.3–25.6%)**, not a point — the
   2024-commissioning fleet (Moray West et al.) straddles the year-end.
   The trace share (20.9%) is consistent with the energy-matching
   convention of the calibrated traces.
6. **Onshore split correction (review condition 1)**: as delivered,
   this package's cluster-share convention OVERSTATED Scottish onshore
   energy share (model 73.4% vs observed 69.8%, 2024) —
   anti-conservative for Q2/Q10, and the note's original
   "conservative" direction claim was wrong (it held only for the
   CF-ordering artefact in isolation). Corrected by adopting the DESNZ
   70.0% onshore split (§3 convention 2): model 69.7% ≈ observed
   69.8%, GB-energy cost +0.05% (2024) / +0.22% (40y). The CF-ordering
   artefact itself (rgb trace CF slightly above sco, against the
   observed 0.2684 > 0.2490 load-factor ordering) remains a stated,
   now second-order, weights limitation.
7. **REPD battery undercount**: REPD GB battery 3.35 GW vs the
   scenario's Modo-cited 4.7 GW; only the share (14.6% Scotland) is
   used. If a better open zonal battery register appears, revisit.
8. **Robin Rigg** (~174 MW, Scottish waters) is inside the `irish_sea`
   cluster and therefore contributes to the rgb trace; REPD counts it
   as Scotland. A ~1.2%-of-fleet zone-assignment approximation, stated.
9. **No engine/scenario files were touched.** GB manifests
   byte-untouched (no `data/packs/cf/` or `data/packs/2024/` file was
   rewritten; `derive_cf.py` unmodified). New manifests:
   `cf-gb2-1985-2024.sha256`, `b6.sha256`.

## 8. Recommended conventions for the scenario package (summary)

1. **Fleet split (REVISED per review condition 1/2; supervisor
   decision 2026-07-04):** capacities from §2. Split the reference GB
   GW by the ADOPTED shares of §3 convention 2 — **onshore 70.0%
   Scotland (DESNZ 0.6997), offshore 20.9% (cluster share, inside the
   observed bracket), solar 2.7% (cluster share)**. Measured basis:
   DESNZ split reproduces the observed 2024 zonal onshore energy
   (69.7% vs 69.8%) at a GB-energy cost of +0.05% (2024) / +0.22%
   (40y) / max +0.54% (single year) — quantified per year in
   `gb2_cf_report.json`. If the scenario package nevertheless keeps
   the cluster shares (73.6% onshore), the review's anti-conservative
   caveat travels verbatim with every Q2/Q10 output: the model then
   overstates Scottish onshore energy share by ~+3.5 pp (~1.3 TWh),
   overstating B6 export pressure and modelled curtailment.
2. **Demand:** flat 10.1% / 89.9% of the D3 underlying-demand trace;
   the station-load wedge splits pro-rata. P114 GSP-group data is the
   v2 refinement and the check.
3./4. **B6 link convention and validation gates — the review §6
   ruling BINDS here and supersedes this note's original §8.3–4 (which
   had recommended tuning to the boundary group and gating against the
   group cost). Transcribed verbatim from
   docs/notes/b6-two-zone-data-review.md §6:**

   > (a) **The link is B6, not a group aggregate.** 2024 validation run:
   >     export capability = the observed half-hourly DA limit series.
   >     Sentinel rule: limit ≥ 9,999 → replace with the ETYS planning
   >     value (6.7 GW, subject to condition 4); limit = 0 → treat the
   >     period's capability as missing and exclude it from gate
   >     arithmetic unless corroborated as a real outage. Missing periods
   >     stay missing and are masked out of gates. Import capability =
   >     3.5 GW flat (superseded HARETORIM series, vintage stated).
   >     Non-2024 / scaled / 40-year runs: export 4.1 GW central (2024
   >     median), sensitivity brackets 2.7 / 5.5 GW (IQR) and 6.7 GW (ETYS
   >     upper bound); import 3.5 GW. No synthesised limit time-series for
   >     non-2024 years (outage-driven, no published basis) — agreed.
   >
   > (b) **Validation gates are B6-specific and require configuration (a):**
   >     (i) modelled pre-constraint boundary flow vs the DA flow series —
   >     correlation + net ≈ 22.6 TWh computed over the same 17,214-period
   >     mask; (ii) modelled constrained export vs 17 TWh (Energy Trends),
   >     carrying the ~2 TWh DA-vs-outturn wedge named above; (iii) binding
   >     frequency vs 23.6%. Tolerances pinned only after first runs
   >     quantify the wedges (incl. the §2 zonal-energy wedges) — the
   >     note's deferral stands.
   >
   > (c) **Costs and curtailment: B6-only is the like-for-like anchor
   >     (£90.5m); the Scottish group (£525.8m) is reported alongside as
   >     the full size of the phenomenon the model structurally cannot
   >     see — never as the model's target.** Model constraint/curtailment
   >     outputs are quoted as a LOWER BOUND on the Scottish constraint
   >     phenomenon, with the B4/B5 invisibility stated. The link
   >     capability must never be tuned to reproduce the group cost in the
   >     validation configuration. A group-effective tighter-link variant
   >     for the Q2/Q10 bounding study is permitted only as a separately
   >     labelled sensitivity that claims NO flow-gate validity.
   >
   > (d) **Schema fact for the work order:** the current `[[links]]` schema
   >     (gb-2024-5zone.toml) carries a single symmetric `capacity_gw` +
   >     availability + loss. Convention (a) needs per-direction and
   >     time-series capability → a `schema_version` bump + docs/03
   >     migration note in the scenario/engine package.
5. **Attribution to carry:** "Supported by National Energy SO Open
   Data" (NESO), OGL v3 statement (DESNZ), Copernicus attribution
   (traces).

## 9. Reproduction

```
V=~/.local/share/grid-sim/era5-venv/bin/python
$V scripts/era5-cf/derive_cf_gb2zone.py .        # 40-year zonal sweep (~1 min)
$V scripts/fetch-b6/fetch.py .                   # network; 12 raw files
$V scripts/fetch-b6/build.py .                   # processed + b6_report.json
cd data/packs && shasum -c cf-gb2-1985-2024.sha256 && shasum -c b6.sha256
```

Retrieval dates: all NESO/gov.uk fetches 2026-07-04. NESO's day-ahead
file is a rolling window refreshed daily — `b6.sha256` pins this
retrieval; re-fetches later will (correctly) fail the manifest.
