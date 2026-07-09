# Stage 7 published-pathways DATA package — reviewer adjudication

**Independent reviewer (stage-7 pathways data gate), 2026-07-06.**
Adversarial review of the uncommitted package:
`data/reference/pathways-published.toml` (DRAFT, schema
`pathways-published-v1`, no parser yet), `data/packs/cb7.sha256`, and
`docs/notes/stage7-pathways-data-report.md`. Raw sources re-opened on
disk (`data/packs/cb7/raw/`, `data/packs/fes2025/raw/`) and re-derived
with independent code; licence pages re-fetched live this session.
Reviewed against docs/05 (data discipline, D1), the costs-gb.toml /
prices-2024.toml citation and quarantine precedents, fes-pathway-v1
conventions, and the stage7-cost-inputs adjudication. Nothing was taken
on the data engineer's word.

Scope check: the package is exactly the three files above (all
untracked; raw files gitignored under `data/**` with the manifest
re-included by `!data/packs/*.sha256` — verified with
`git check-ignore`). The concurrently modified tree files (Cargo.toml,
grid-adequacy/*, memory/*, scenarios/gb-2024-3zone.toml, B4-LP tests)
belong to the B4 LP workstream and are untouched by this package. No
docs/04, docs/08, docs/03, engine, or scenarios/ edits. No code, so
TDD/clippy/fmt gates do not attach; the pinned-regression-test
obligation attaches when any number is first quoted (docs/05 rule 3).

## VERDICT: ACCEPT-WITH-CONDITIONS

Every number I re-derived from the raw sources reproduces —
FES bit-identically, CCC exactly at the file's stated precision — and
both licence verdicts verify from the primary pages, including the
CCC OGL v3.0 supersession, which I rule SOUND (§2). Two material
defects, both in the evidence note's prose rather than the numbers:
(1) the CCC demand basis EXCLUDES surplus-driven electrolysis
(29 TWh 2035 / 89 TWh 2050, report p.208) — an exclusion NOT carried
with magnitude, against the package's own law, and load-bearing for
any FES-vs-CCC demand comparison; (2) the corrections-log paragraph
misdescribes the log (it is ~20 corrections, not six, and one of them
DOES touch Table 7.5.1 — the levelised-cost rows, not the extracted
rows, so the package's conclusion survives but its stated basis is
wrong). Conditions 1–5 discharge before commit/pin; 6–9 bind the
scenario package.

---

## 1. Checksums, file discipline — VERIFIED

- `shasum -c data/packs/cb7.sha256`: 4/4 OK. `fes2025.sha256`: 5/5 OK
  (no new FES fetch, as claimed). Manifest hashes match the
  `[sources.*]` sha256 entries in the TOML one-for-one.
- Raw files gitignored (`data/**`), manifests tracked
  (`!data/packs/*.sha256`), reference TOML tracked
  (`!data/reference/**`) — the fetch-and-build law is honoured;
  nothing is fetched at build time (no parser exists at all yet).
- `pathways-published.toml` parses clean under tomllib. All four
  demand-component sums re-verified at load (FES 450.076 / 784.736;
  CCC 443.541 / 692.025 — see §3/§4 for basis).
- Field shapes verified against the committed `fes-pathway.toml`:
  `technology`/`capacity_gw` and `kind`/`power_gw`/`energy_gwh` match
  fes-pathway-v1 mechanically, as claimed. All 14 technology ids and
  both storage kinds are the fes-pathway-v1 set; `TechId` is an open
  set (grid-core/src/scenario.rs:203) so `hydrogen_turbine` and
  `marine` are legal; `mappable = false` booleans on the CCC
  aggregates are machine-visible, satisfying the costs-gb.toml
  quarantine-precedent in spirit (ruling: format acceptable —
  condition 7 makes the parser enforce it).

## 2. Licences — VERIFIED; the CCC supersession is SOUND

**FES 2025.** https://www.neso.energy/data-portal/neso-open-licence
re-fetched this session: grant clause verbatim ("copy, publish,
distribute and transmit the Information; adapt the Information;
exploit the Information commercially and non-commercially"); required
attribution string verbatim ("Supported by National Energy SO Open
Data") — matches the TOML `attribution` field. The CC BY 4.0
compatibility sentence is present on the page ("These terms are
compatible with the Creative Commons Attribution License 4.0…"),
though it sits in OGL-boilerplate text — the claim is supported;
nothing turns on it. CLEAN.

**CCC CB7.** https://www.theccc.org.uk/copyright-terms-conditions/
re-fetched this session: *"The material on this website is licensed
under the Open Government Licence v3.0 except where otherwise
stated"* and *"Material (other than logos and photography) may be
reproduced free of charge in any format or medium, provided it is
reproduced accurately"* — both verbatim as quoted. Both workbooks
swept sheet-by-sheet by my own code for
licence/copyright/rights/OGL strings: **zero hits in either** — no
overriding statement, so the site-wide OGL governs the datasets
(hosted on theccc.org.uk/wp-content). The report PDF's own notice
re-extracted from the raw file: *"© Climate Change Committee
copyright 2025. The text of this document … may be reproduced free of
charge in any format or medium provided that it is reproduced
accurately and not in a misleading context. The material must be
acknowledged as Climate Change Committee copyright and the document
title specified."* — verbatim (it sits on printed p.3, not p.2:
condition 3). The "CCC and AFRY analysis" line is the CCC's own
figure-source credit, not an identification of third-party copyright
in the sense of the site licence's carve-out; the workbooks carry no
third-party rights statement. CLEAN.

**Supersession ruling: SOUND.** The prior negative finding
(stage7-cost-inputs-report §10.4, 2026-07-03: "no explicit open
licence found on theccc.org.uk … their FoI page says check copyright
before reproducing") was a *not-found-this-session* result from a
narrower look — the earlier session reached the FoI page, not the
copyright-terms page. It was never a positive finding of a
restrictive licence, so there is no conflict of evidence to resolve:
the copyright-terms page exists, states OGL v3.0, and I have verified
it independently of the data engineer. The supersession stands;
docs/08 D1 register text in §8 below.

## 3. FES numbers — re-derived from the raw CSVs, ALL REPRODUCE

Independent extraction (my own code, not the package's recipe) from
the pinned ES1/ED1/FLX1 CSVs, Electric Engagement, 2035 and 2050:

- **All 28 fleet capacities** (14 ids × 2 years) reproduce to the
  stated 4 dp, including every fold component given in comments:
  ccgt 2035 = 12,687.788 + 226.040 + 7,183.0 MW = 20.0968 GW; ccgt
  2050 = 4,390 + 0 + 26,645 = 31.035 GW (the CCS-gas fold 7.183 →
  26.645 GW confirmed, and EE's 4.39 GW residual unabated CCGT in
  2050 confirmed); ocgt 7.0555/1.9568; oil 0.2425/0.0615; nuclear
  5.51/21.56; biomass 5.415/5.2133 (BECCS 4.17 both years); waste
  3.7013/2.6455; hydro 2.0105/2.1509; marine 1.6844/4.2644; other
  0.011; onshore 38.5758/50.7156; offshore 67.5195/96.3654; solar
  62.058/100.9131 (non-networked 0.0128/0.0556 excluded — the ONLY
  non-networked EE rows in ES1, confirming e4 and "no non-networked
  offshore tier"); interconnector 20.6/24.4; hydrogen_turbine
  0.9958/27.5201. **Zero unmapped grid-connected ES1 rows** —
  confirmed by exhaustive enumeration of the Type/SubType pairs.
- **Storage**: battery 32.5973/45.9488 and 40.4449/60.257 (ES1
  Storage Capacity (GWh) tier sums re-derived); pumped_hydro fold
  8.5781/89.3982 and 16.5781/223.3982 with all six PH/LAES/CAES
  components matching.
- **FLX1 independent cross-check**: bit-identical at both years —
  battery 32.59725/45.94879, 40.44492/60.25699; LDES components;
  interconnectors 20.6/24.4; hydrogen generation 0.99582/27.52013;
  CCS gas 7.183/26.645; unabated gas 19.96936 (= 12.6878 + 0.2260 +
  7.0555 mapped) / 6.3468; unabated biomass 1.245/1.04327. The FLX1
  storage total row is labelled "Total (excluding vehicle-to-grid)"
  — confirming e5's V2G statement verbatim; DSR appears only as
  peak-impact GW lines, confirming the not-transcribable ruling.
- **ED1 demand**: totals 450,076 / 784,736 GWh; peak 82.042/143.625
  (row "GBFES Peak Customer Demand: Total Consumption plus Losses",
  [Peak]); all six level-2 components and all six electrification
  markers match at 3 dp; "Annual [Fiscal]" label confirmed
  (fiscal-basis flag correct). Precision nit (condition 4): the
  level-2 components sum to the total *at the file's 3 dp precision*;
  at raw GWh precision they sum to 450,076.063 and 784,735.439 vs
  published totals 450,076 / 784,736 (residual ≤ 0.6 GWh, ~1e-6) —
  "sum exactly" should say "exact at carried precision".
- **Report-PDF spot-checks** (per-pathway tables, re-extracted):
  offshore 47.8 (2030) / 96.4 (2050) ✓; nuclear 4.1/21.6 ✓; solar
  46.8/101.0 (headline includes non-networked; 100.9131 + 0.0556 =
  100.9687 → 101.0 ✓); tidal 4.3 ✓; battery 25.2/40.4 ✓; LDES
  3.8/16.6 ✓; interconnectors 12.5/24.4 ✓; low-carbon dispatchable
  0.0/54.2 (26.645 + 27.5201 = 54.1651 ✓); unabated gas 35.3/6.3
  (4.39 + 1.9568 = 6.3468 ✓). One citation defect: the phrase
  "consumers lead the way through electrified demand", presented as a
  quote from "report p.31", does not appear anywhere in the report
  PDF; the actual text (pdf p.26) is "Net zero is achieved in
  Electric Engagement mainly through electrified demand. Consumers
  are highly engaged in the transition…" — condition 3. The
  substance (EE is FES 2025's high-electrification pathway) is
  confirmed by that same passage.

## 4. CCC numbers — re-derived from the raw workbooks, ALL REPRODUCE

- **Sheet "7.5.3"** (charts workbook v2), rows 13–23 exactly as
  cited: nuclear 6.09/10.98; BECCS 1.29/1.29; offshore 70/125;
  onshore 28.89/37.41; solar 70.01/106.4; low-carbon dispatchable
  8.49/38.28; unabated gas 29.71/0; other generation 7.57/5.5;
  interconnection 20.938/27.938; grid storage 26.75/42.03; smart
  demand flexibility 22.0/32.55. Battery + medium = grid storage
  exact at both years (26.749999…, 42.03).
- **Sheet "7.5.4"** CCC-milestone rows: battery 21.04/35.11; medium-
  duration 5.71/6.92 — exact.
- **Full dataset**: Economy-wide "Energy: gross demand electricity",
  Balanced Pathway = 443.5405884900234 / 692.0252594617826 TWh —
  bit-identical to the cited raws. Sector-level: the nine carried
  sectors are exactly the nonzero ones (Electricity supply, F-gases,
  Land use, Waste all zero at both years); their sum reproduces the
  economy-wide total to full float precision at both years. All nine
  values match at 3 dp (incl. aviation 0.0 in 2035).
- **Table 7.5.1** (report PDF, re-extracted): battery "21 / 54" and
  "35 / 139"; medium-duration (excl. hydrogen) "6 / 312" and
  "7 / 433"; demand 444/692; offshore 70/125; onshore 29/37; solar
  70/106; LCD 8/38; battery GW 21/35 — all match the workbook and the
  TOML's rounded-integer carry. The table is on printed **p.208**,
  not p.207 as cited (the corrections log itself cites "Table 7.5.1
  (p. 208)") — condition 3. No storage-GWh variable exists anywhere
  in the full dataset (variable-definitions sweep) — the
  "no machine-readable GWh series" quarantine claim holds, and the
  `energy_precision` stamps are the right treatment.
- **Figure 7.5.3 notes** verified verbatim on printed p.213 (bucket
  definitions for other/LCD/smart-flexibility as quoted).
- **Coverage arithmetic**: 2035 mapped 195.928 of 242.988 GW =
  80.63% → "80.6%" ✓; 2050 mapped 307.728 of 352.798 GW = 87.23% →
  "87.2%" ✓ (non-storage, excl. smart flexibility, as stated).
- **The three CCC split decisions are genuinely deferred**: the
  unabated-gas, LCD, and other-generation buckets appear ONLY under
  `[[aggregates]]` with `mappable = false` and suggested (not
  applied) treatments; no split was silently consumed into the fleet
  entries — verified by summing the five mapped ids against sheet
  7.5.3 (no aggregate leakage). The BECCS partial-coverage-trap
  ruling (kept aggregate rather than folded, because unabated biomass
  sits inside other_generation) is correct and consistent.

## 5. The two material defects

**(A) CCC electrolysis demand is EXCLUDED from the carried demand —
not flagged, no magnitude.** Report p.208, immediately under
Table 7.5.1: *"The production of hydrogen from surplus generation
accounts for an **additional** 29 TWh of electricity use in 2035 and
89 TWh in 2050"* — additional to the 444/692 TWh gross demand this
package carries. Figure 7.5.3 note (6): *"Generation includes surplus
electricity used for electrolytic hydrogen production"* — so the CCC
fleet is sized to serve demand PLUS electrolysis while the carried
`demand_twh` excludes it. Two consequences the package must state:
(i) the FES and CCC demand bases are asymmetric — FES 2050 = 784.736
TWh INCLUDING 81.91 TWh electrolysis (e6), CCC 2050 = 692.025 TWh
EXCLUDING ~89 TWh electrolysis; on a like-for-like
gross-incl-electrolysis basis the pathways are ~781 vs ~785 TWh —
nearly identical, where the headline comparison suggests a 93 TWh
gap; (ii) a scenario built from 692 TWh demand against the CCC fleet
under-loads that fleet by ~13% and will misstate curtailment/
adequacy. This violates the package's own law (every exclusion
carried with magnitude) and the TOML's "inclusive of all system-wide
demands" gloss overstates the definition (the dataset's actual
wording is "inclusive of all system-wide uses of each fuel type from
the sector's perspective" — and electrolysis is evidently accounted
elsewhere). Condition 1.

**(B) The corrections-log paragraph misdescribes the log.** The note
claims "read in full: six corrections … plus an offshore-wind £/kW
typo — none touches the electricity supply capacity or demand
figures." The log (re-read in full by me) contains ~20 entries
spanning the CB7 report, the devolved-budget reports, and the
methodology report — and one entry DOES touch Table 7.5.1 (p.208):
the low-carbon dispatchable **levelised-cost** rows (221–223 /
145–190 / 162–189 replacing 163–218 / 147–188 / 161–191). The
extracted rows (storage GW/GWh, demand, capacities) are untouched, so
the `[sources.cb7_corrections]` claim as scoped ("NO correction
touches the electricity supply capacity/demand figures used here")
survives — but the note's stated basis for it is wrong, and a
correction inside the very table this package quotes must be named,
not missed. The log's note (2) also explains the workbook's "v2"
(an updated charts-and-data file was published WITH the corrections
log — the package fetched the updated one, which is right). Condition 2.

## 6. Judgment calls — rulings

- **Electric Engagement pick: SOUND.** The work order's
  "system-transformation-style high-electrification pathway" is
  internally contradictory in FES-2021–23 naming (System
  Transformation was the hydrogen-led pathway); reading intent as
  high-electrification is the only coherent resolution, and EE is
  FES 2025's high-electrification case (confirmed from the report).
  Holistic Transition is already committed at full annual resolution
  (`data/reference/fes-pathway.toml`, verified on disk) — duplicating
  it adds nothing. **Pre-registered**: if a Holistic Transition
  comparison is later wanted, it is a mechanical lift from
  fes-pathway.toml (HT 2035: 433.422 TWh; HT 2050: 705.223 TWh,
  ccgt 22.205 GW all-CCS, hydrogen_turbine 26.082 GW, zero unabated
  gas) and would add (a) an FES central-vs-stress demand bracket
  (705 vs 785 TWh in 2050) and (b) the LCD composition contrast
  (HT reaches zero unabated gas by 2050; EE retains 6.35 GW) — no new
  evidence, no new licence work.
- **Electrolysis-as-firm-demand (FES e6): SOUND as flagged** — but
  incomplete until the CCC side carries its counterpart (defect A).
- **Rounded-integer storage stamps: SOUND.** No machine-readable GWh
  series exists (verified); `energy_precision` is the honest carry;
  the ~55–63 h medium-duration "planning volume, not plant spec"
  caveat is correct and must travel with any storage-sensitive CB7
  result.
- **UK-vs-GB scope: the flag is adequate for THIS file** (a
  transcription layer must not invent a derate), but NOT adequate to
  build on: the scenario package must declare the convention
  (UK-as-GB approximation named, or a stated uniform ~2.9–3% NI
  derate) BEFORE any CCC scenario is assembled — condition 6, and the
  CCC's own corrections log incidentally confirms NI CCS is shipped
  to GB, i.e. the UK figures genuinely embed NI quantities.
- **CCC peak demand: named gap, correctly quarantined** (no published
  system-peak series; flexibility-at-peak is not a peak). Any peak
  the scenario package constructs is a declared convention, not CCC
  data.

## 7. Conditions

**Pre-commit (discharge in the note + TOML before the supervisor pins
this package):**

1. **(Material)** Carry the CCC electrolysis exclusion with
   magnitude: add an exclusion/aggregate entry (29 TWh 2035 / 89 TWh
   2050, cite report p.208 under Table 7.5.1 and Fig 7.5.3 note 6)
   and a demand-basis note stating: FES demand INCLUDES electrolysis,
   CCC demand EXCLUDES it; like-for-like 2050 comparison is ~781 vs
   ~785 TWh; a CCC scenario using 692 TWh against the published fleet
   under-loads it. Correct the "inclusive of all system-wide demands"
   gloss to the dataset's actual definition wording.
2. **(Material)** Rewrite the corrections-log paragraph: the log
   contains ~20 corrections across three report families; one touches
   Table 7.5.1 (LCD levelised-cost rows — price variables this
   package does not carry); the storage/demand/capacity rows
   extracted here are unaffected; the v2 charts workbook is the
   corrected reissue published with the log (which is why fetching v2
   was right).
3. **(Citations)** Table 7.5.1 is printed p.208 (two TOML sites +
   note §4/§5); the CCC reproduction notice is printed p.3; replace
   the non-existent FES quote "consumers lead the way through
   electrified demand (report p.31)" with the real sentence ("Net
   zero is achieved in Electric Engagement mainly through electrified
   demand…") or cite its actual source. Nit: ES1's label is
   "Nuclear - Small", not "Nuclear - Small (SMR)" — keep the SMR
   gloss outside the quoted label.
4. **(Wording)** State the FES level-2 demand reconciliation as exact
   at carried (3 dp) precision; at raw GWh precision the residual is
   ≤ 0.6 GWh (~1e-6) against the published totals.

**Binding on the scenario package (record in its work order):**

5. The pathways-published-v1 parser (registered in docs/03 per the
   costs-reference-v1 precedent) must refuse to consume
   `mappable = false` rows without a declared, reviewed split rule,
   and must propagate `energy_precision` stamps onto any artefact
   quoting CB7 storage energy.
6. Declare the UK-as-GB convention (named approximation or stated NI
   derate) before assembling any CCC scenario; carry it on every
   CCC-derived output.
7. Any FES-vs-CCC demand or adequacy comparison carries the
   electrolysis-basis wedge (condition 1) explicitly; CCC scenarios
   state the constructed-peak convention (no CCC peak exists) and the
   fiscal-vs-calendar year wrinkle stays stated.
8. The three CCC split decisions (unabated gas CCGT:OCGT, LCD
   gas-CCS:hydrogen, other-generation apportionment) are reviewed
   decisions of that package — the suggested treatments here are
   suggestions, not adoptions. Per docs/05 rule 3, every number later
   published from this file acquires a pinned regression test.

## 8. docs/08 D1 register update (for the supervisor to apply)

Suggested text for the CCC licence entry: *"CCC (theccc.org.uk)
publications and datasets: Open Government Licence v3.0 per the
site-wide statement at theccc.org.uk/copyright-terms-conditions/
('licensed under the Open Government Licence v3.0 except where
otherwise stated'), re-verified independently by the data engineer and
the reviewer 2026-07-06; the CB7 workbooks carry no overriding
statement (string-swept by both); the report PDF carries CCC's own
permissive reproduction notice (printed p.3). This SUPERSEDES the
stage7-cost-inputs-report §10.4 'no explicit open licence found'
finding, which had reached only the FoI page. Redistribution of
derived numbers is permitted with attribution 'Climate Change
Committee, The Seventh Carbon Budget (2025)'; third-party-copyright
carve-out noted (no third-party rights statement in the CB7 data
artefacts used)."*

## Notes of record (non-blocking)

- x1. NESO's CC BY 4.0 compatibility sentence sits in OGL-derived
  boilerplate on the licence page; the operative grant is NESO's own
  clause, which is verified verbatim. Nothing depends on the CC BY
  framing.
- x2. The "portal last_modified 2025-12-10" and "no FES 2026"
  statements were not independently re-verified (portal metadata;
  immaterial — the pack is pinned by checksum and re-verified on
  disk).
- x3. BECCS report spot-check (0.6/4.2, Table 34) not independently
  re-extracted; the BECCS capacity itself is verified from ES1 and
  FLX1 (4.17 GW both years), which is what the TOML carries.
- x4. The single-file one-register-per-package layout and the
  DRAFT-until-reviewed header follow the costs-gb.toml precedent
  correctly.
