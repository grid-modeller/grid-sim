# Stage 7 published-pathways data evidence note (data engineer, 2026-07-06)

**Status:** EVIDENCE NOTE — DRAFT pending review. Serves the docs/04
Stage 7 scope line "scenario pack for published pathways (FES, CCC,
Royal Society)": this package delivers the cited capacity/demand tables
for the FES and CCC legs (the Royal Society leg is already committed as
`scenarios/royal-society-37y*.toml`). Companion data file:
`data/reference/pathways-published.toml` (schema
`pathways-published-v1` — **no parser exists yet**; the scenario
package writes it and registers the schema in docs/03 per the
costs-reference-v1 precedent). New pack manifest:
`data/packs/cb7.sha256`. Nothing here edits docs/04, docs/08, engine
crates, or `scenarios/`.

Every number carries a named primary source, its licence, and its
citation (file, sheet/table, row). Where a published quantity could not
be mapped or verified to standard, it is named in §5/§7 — nothing was
silently substituted or silently included.

---

## 1. Sources and retrieval record

**FES 2025 (NESO).** No new fetch: the four data artefacts are the
already-pinned `data/packs/fes2025` pack (retrieved 2026-07-03; portal
v006, last_modified 2025-12-10). All five checksums re-verified
bit-identical against `data/packs/fes2025.sha256` this session
(2026-07-06, `shasum -c`: OK ×5). FES 2025 confirmed the most recent
edition (published 14 July 2025; no FES 2026 as of 2026-07-06 — checked
neso.energy publications page via search this session).

**CCC Seventh Carbon Budget (26 Feb 2025).** Fetched 2026-07-06 from
theccc.org.uk over HTTPS into `data/packs/cb7/raw/` (gitignored;
fetch-and-build law), manifest `data/packs/cb7.sha256`:

| File | sha256 |
|---|---|
| The-Seventh-Carbon-Budget-full-dataset.xlsx | `fe5fbe17d81065c99f3a534e8ecb2af01e379edf291c9429baad9d17d0f6ff72` |
| The-Seventh-Carbon-Budget-Charts-and-data-in-the-report-v2.xlsx | `c24c94d3bc0b49d43cd0da3db80caa09464872b1a61da4cf473cc24796d372ab` |
| The-Seventh-Carbon-Budget.pdf | `891fa9c010cd8020b6580cce05874c89072ab5ddfcc2b8b798162205b9bf91d3` |
| Seventh-Carbon-Budget-…-corrections-log.pdf | `77cc63f6d135a3d0eb16135109549eda7fb76f2b01dfc0d71cdc5101f6f939a5` |

The Sept-2025 **corrections log was read in full** (re-read
2026-07-06 after review): it contains **~20 corrections** (22 report-
location entries) spanning three report families — the CB7 advice
report, the devolved (Scotland/Wales/NI) budget reports, and the
methodology report. **One correction touches Table 7.5.1 (p.208)
itself**: the low-carbon dispatchable LEVELISED-COST rows (163–218 /
147–188 / 161–191 £/MWh should read 221–223 / 145–190 / 162–189) —
price variables this package does not carry. **The rows extracted here
(storage GW/GWh, demand, capacities) are unaffected** — the no-impact
claim is scoped to exactly that basis. The corrected report PDFs were
not reissued, but an **updated charts-and-data file was published with
the log — the "v2" workbook this package fetched is that corrected
reissue** (fetching v2 rather than the original was therefore right).
The March-2026 "Supplementary analysis of the Seventh Carbon Budget"
exists (sensitivity report); the Feb-2025 advice dataset remains the
pathway of record and is what this package pins. CB7 is used rather
than CB6 because it is both the current advice and licence-clean (§2).

## 2. Licence verdicts (D1 discipline, docs/08)

**FES 2025: CLEAN.** NESO Open Data Licence, recorded on every Data
Portal dataset used. Re-verified 2026-07-06 at
https://www.neso.energy/data-portal/neso-open-licence — grant clause:
*"copy, publish, distribute and transmit the Information; adapt the
Information; exploit the Information commercially and
non-commercially"*; required attribution wording: **"Supported by
National Energy SO Open Data"**; NESO states CC BY 4.0 compatibility.
Redistribution of the derived tables is permitted with that attribution
carried on published outputs.

**CCC CB7: CLEAN — and this SUPERSEDES the prior negative finding.**
`docs/notes/stage7-cost-inputs-report.md` §10.4 (2026-07-03) recorded
"no explicit open licence found on theccc.org.uk; quote-with-attribution
only". Re-checked 2026-07-06: the CCC's copyright page
(https://www.theccc.org.uk/copyright-terms-conditions/) states verbatim:
*"The material on this website is licensed under the Open Government
Licence v3.0 except where otherwise stated"*, with *"Material (other
than logos and photography) may be reproduced free of charge in any
format or medium, provided it is reproduced accurately."* Overriding
statements checked: **both xlsx workbooks were searched sheet-by-sheet
for copyright/licence strings — none present**, so the site-wide OGL
v3.0 governs the datasets. The report PDF carries its own notice
(printed p.3):
*"© Climate Change Committee copyright 2025. The text of this document
… may be reproduced free of charge in any format or medium provided
that it is reproduced accurately and not in a misleading context. The
material must be acknowledged as Climate Change Committee copyright and
the document title specified."* Either way, redistribution of derived
numbers with attribution is permitted. Attribution carried in the TOML:
"Climate Change Committee, The Seventh Carbon Budget (2025); CCC and
AFRY analysis" (the AFRY co-credit is the figure-source line the CCC
itself prints under Figure 7.5.3/Table 7.5.1).

Supervisor note: the docs/08 D1 row and the cost-inputs note's §10.4
are unchanged by me (not my files to edit); the CCC licence register
should be updated to cite the copyright-terms page when this package is
adjudicated.

## 3. FES scenario choice (flagged judgment call)

The work order recommended "the system-transformation-style
high-electrification pathway". These two labels point at different
things: in FES 2021–2023, *System Transformation* was the
**hydrogen-led** pathway and *Consumer Transformation* the
high-electrification one. In FES 2025 the pathway set is Holistic
Transition / Electric Engagement / Hydrogen Evolution (+ Ten Year
Forecast, Falling Behind), and the high-electrification pathway is
**Electric Engagement** ("Net zero is achieved in Electric Engagement
mainly through electrified demand. Consumers are highly engaged in the
transition …" — report PDF p.26, pathways overview). I read the intent
as *high-electrification* and picked
**Electric Engagement**, because:

1. It is FES 2025's high-electrification case — the demand-side stress
   pathway (peak 143.6 GW and 784.7 TWh in 2050; the largest
   heat-pump/EV electrification of the three net-zero pathways).
2. **Holistic Transition is already committed at full annual
   resolution** in `data/reference/fes-pathway.toml` (adopted, Q8
   consumer) — re-extracting it would duplicate an existing asset;
   its 2035/2050 snapshots are already available there.
3. It complements the rest of the pack: CCC's Balanced Pathway is
   itself electrification-led, and the Royal Society scenarios are the
   renewables+storage-only extreme. EE gives the scenario package the
   FES counterpart on the same axis.

If the supervisor wants Holistic Transition snapshots in this file
instead/as well, they are a mechanical lift from `fes-pathway.toml`
(same mapping, same sources) — no new evidence needed.

## 4. Extraction method and validation

**FES (GB).** Extracted from the pinned ES1/ED1 CSVs with the identical
mapping rules as `fes-pathway.toml` (grid-connected tiers Transmission
+ Distributed + Distributed - Micro; Non-Networked excluded and
reported; the ccgt/ocgt/oil/nuclear/biomass/waste/marine/
hydrogen_turbine folds and the battery / pumped_hydro(+LAES/CAES)
storage folds as documented there). Extraction script run this session
(scratchpad; recipe = the mapping table in the TOML header — the
committed `scripts/fes-pathway/` tooling regenerates the HT file and
was not modified). Rounding: GW/GWh 4 dp, TWh 3 dp, at output only.
**Zero unmapped ES1 rows** for Electric Engagement.

Validation, all at 2035 AND 2050:

- **Independent FLX1 cross-check (bit-identical):** battery power+energy
  (32.5973/45.9488; 40.4449/60.2570), LDES components summing to the
  pumped_hydro fold (8.5781/89.3982; 16.5781/223.3982), interconnectors
  (20.6; 24.4), hydrogen generation (0.9958; 27.5201), CCS gas (7.1830;
  26.6450), unabated gas (19.9694 = mapped unabated ccgt-part + ocgt;
  6.3468), unabated biomass (1.2450; 1.0433).
- **Published-table spot-checks (report PDF, per-pathway columns, all
  match at the report's 1 dp):** offshore 47.8 (2030) / 96.4 (2050);
  onshore 29.0/50.7; solar 46.8/101.0 (report headline includes
  non-networked: 100.9131 + 0.0556 = 100.9687 → 101.0); tidal 4.3
  (2050); nuclear 4.1/21.6; battery 25.2/40.4; LDES 3.8/16.6;
  interconnectors 12.5/24.4; low-carbon dispatchable 0.0/54.2
  (= CCS gas 26.645 + hydrogen 27.5201 = 54.1651); unabated gas
  35.3/6.3; BECCS 0.6/4.2 (Tables 24–34).
- **Demand reconciliation:** ED1 level-2 components sum to the System
  Demand Total exactly at the carried 3 dp precision (450.076 and
  784.736 TWh); at raw GWh precision the residuals against the
  published totals are +0.063 GWh (2035) and −0.561 GWh (2050),
  ≤ 0.6 GWh (~1e-6 relative).

**CCC (UK).** Capacity mix transcribed from the charts workbook sheet
"7.5.3" (the data behind Figure 7.5.3), battery/medium-duration GW from
sheet "7.5.4" (CCC-milestone rows), storage GWh from report Table 7.5.1
(printed p.208 — rounded integers, the only published energy figures;
the log-corrected rows of that table are its levelised-cost lines, not
these), demand
from the full dataset ("Energy: gross demand electricity", Balanced
Pathway, UK). Validation:

- Battery + medium-duration = "Grid storage" bucket exactly at both
  years (21.04+5.71 = 26.75; 35.11+6.92 = 42.03).
- Sector-level demand components sum exactly to the economy-wide total
  at both years (443.541; 692.025 TWh).
- Table 7.5.1's rounded values match the workbook data everywhere they
  overlap (demand 444/692; offshore 70/125; onshore 29/37; solar
  70/106; LCD 8/38; battery 21/35 GW).

## 5. Mapping decisions and EXCLUSIONS (with magnitudes)

**FES Electric Engagement** — full mapping; the honest caveats are
folds, not omissions:

| # | Item | 2035 | 2050 | Treatment |
|---|---|---|---|---|
| e1 | CCS gas inside `ccgt` | 7.183 GW | 26.645 GW | folded (same machine class, fes-pathway precedent); economic caveat: abated plant, different SRMC/carbon position |
| e2 | BECCS inside `biomass` | 4.170 GW | 4.170 GW | folded (NESO's own headline grouping) |
| e3 | Hydrogen turbines | 0.996 GW | 27.520 GW | open-set `hydrogen_turbine` id — engine v1 has no H2 fuel-chain cost/supply; non-synchronous inertia default unless `inertia_h` set |
| e4 | Non-networked solar | 0.013 GW | 0.056 GW | excluded (never grid-connected); EE has no non-networked offshore tier |
| e5 | DSR / V2G | — | — | not transcribed: no published power/energy pair (peak-impact decompositions only); FLX1 storage totals exclude V2G and so does this file |
| e6 | Grid-connected electrolysis demand | 7.360 TWh | 81.910 TWh | INCLUDED inside industrial demand as firm load — flexibility-overstatement flag for the scenario package |

Also note: EE retains **4.39 GW unabated CCGT + 1.96 GW unabated
OCGT-class in 2050** (report Table 33: 6.3 GW) — unlike Holistic
Transition, which reaches zero unabated gas.

**CCC Balanced Pathway** — five unambiguous fleet mappings (nuclear,
offshore_wind, onshore_wind, solar, interconnector) + two storage
entries; everything else is a published bucket the engine cannot take
without a split decision, carried in the TOML as `[[aggregates]]` with
**`mappable = false` (load-bearing — a scenario builder must not
consume these without a declared, reviewed split rule)**:

| Aggregate | 2035 | 2050 | Why unmappable |
|---|---|---|---|
| unabated_gas | 29.71 GW | 0 GW | CCGT/OCGT split not published |
| low_carbon_dispatchable | 8.49 GW | 38.28 GW | "gas CCS and hydrogen" (Fig 7.5.3 note 3), split not published — the largest unmapped bucket in 2050 |
| other_generation | 7.57 GW | 5.5 GW | "unabated biomass, energy from waste, hydro, and CHP", split not published |
| ccs_biomass (BECCS) | 1.29 GW | 1.29 GW | mappable to `biomass` in principle, but the id would then exclude unabated biomass (inside other_generation) — partial-coverage trap, kept aggregate |
| smart_demand_flexibility | 22.0 GW | 32.55 GW | peak-GW DSR quantity, not a storage pair (same ruling as e5) |

Plus one **demand-side exclusion (c1, condition-1 of the review)**: the
CCC `demand_twh` **excludes surplus-driven electrolysis** — *"The
production of hydrogen from surplus generation accounts for an
additional 29 TWh of electricity use in 2035 and 89 TWh in 2050"*
(report p.208, immediately under Table 7.5.1; Figure 7.5.3 note 6:
"Generation includes surplus electricity used for electrolytic hydrogen
production"). Carried machine-visibly in the TOML as
`surplus_electrolysis_excluded_twh` on each CCC year block and in
`[pathways.ccc_cb7_balanced.exclusions]`.

Coverage statement the scenario package must carry (non-storage,
excluding the demand-side smart-flexibility row): at 2050 the CCC
mappable fleet covers 307.73 GW of 352.80 GW published capacity
(87.2%); the gap is low-carbon dispatchable 38.28 + other generation
5.5 + BECCS 1.29 GW. At 2035, mappable 195.93 GW of 242.99 GW (80.6%);
unabated gas 29.71 GW is the biggest single gap.

**Storage duration classes:** FES publishes none explicitly (the
battery/LDES energy figures imply ~1.4 h battery and ~10–13 h LDES);
CCC publishes battery vs "medium-duration (excl. hydrogen)" — its
312/433 GWh medium-duration volumes (~55–63 h at published GW) are
storable-energy planning volumes, not single-plant specs; flagged in
the TOML.

## 6. Demand-construction notes

- **FES ED1 is fiscal-year** ("Annual [Fiscal]"); ES1 capacities carry
  plain year labels; CCC years are calendar. The 2035/2050 snapshots mix
  these bases as published — a sub-1% framing wrinkle, stated not fixed.
- **FES demand = GBFES System Demand Total**: includes transmission +
  distribution losses and grid-connected electrolysis; GB scope. The
  level-2 decomposition (residential / commercial / industrial /
  direct-transmission / losses) is carried in full, plus six
  electrification markers (heat pumps, resistive heat, residential and
  commercial EVs, electrolysis, data centres) — subsets, not additive.
- **CCC demand = "Energy: gross demand electricity"** — dataset
  definition verbatim: *"Gross energy demands are inclusive of all
  system-wide uses of each fuel type from the sector's perspective,
  regardless of whether the fuel is used at primary or final stage"*;
  **UK scope, not GB** — Northern Ireland (~3% of UK demand, on the
  Irish synchronous area) is not separable from any CB7 table. The
  scenario package must either state the UK-as-GB approximation or
  derate; flagged in the TOML header.
- **Demand-basis wedge (load-bearing for any FES-vs-CCC comparison):**
  FES `demand_twh` INCLUDES grid-connected electrolysis (7.360 TWh
  2035 / 81.910 TWh 2050, exclusion e6); CCC `demand_twh` EXCLUDES
  surplus electrolysis (29 / 89 TWh, exclusion c1). Like-for-like on a
  gross-incl-electrolysis basis, 2050 is **~781 TWh (CCC) vs ~785 TWh
  (FES EE)** — nearly identical, where the headline 692-vs-785
  comparison suggests a 93 TWh gap; 2035 inverts (~472.5 vs 450.1 TWh).
  **Warning:** a scenario running 692 TWh against the published CCC
  fleet under-loads that fleet by ~13% (89/692) and will misstate
  curtailment and adequacy — the CCC fleet is sized to serve demand
  PLUS electrolysis (Fig 7.5.3 note 6).
- The CCC sector decomposition (9 sectors, sums exact) gives the
  electrification components: surface transport 100.97 → 162.84 TWh,
  residential buildings 137.88 → 234.71 TWh, engineered removals
  (DACCS etc.) 0.63 → 14.90 TWh across 2035 → 2050.
- Neither pathway's demand is a half-hourly trace: profile construction
  (D3 underlying-demand convention, D9 heating overlay for the
  heat-pump share) is the scenario package's work, not transcription.

## 7. Quarantine list (named, per project law)

1. **CCC storage energy (GWh) precision** — published only as rounded
   integers in Table 7.5.1 (54/139 battery; 312/433 medium-duration);
   no machine-readable GWh series exists in either workbook. Usable,
   carried with `energy_precision` stamps; any storage-sensitive result
   quoting CB7 must carry the rounding caveat.
2. **CCC unabated-gas CCGT/OCGT split, low-carbon-dispatchable split,
   other-generation split** — not published anywhere in the CB7
   corpus retrieved; `mappable = false`, split rules are the scenario
   package's reviewed decision (suggested treatments recorded).
3. **CCC peak demand** — not found in the CB7 dataset or the
   electricity-supply chapter figures retrieved this session (the CCC
   publishes flexibility-at-peak, not a system peak-demand series).
   FES peak is carried; the CCC scenario has no cited peak. Named gap.
4. **DSR/V2G (both pathways)** — excluded, not derived (e5 / smart
   demand flexibility): deriving a `dsr` StorageKind entry would be a
   modelling choice, not transcription (same ruling as
   fes-pathway.toml; the Q6/D10 work owns it).
5. **CB7 methodology-report per-technology assumptions** (load factors,
   AFRY modelling inputs) — not extracted this session; if the scenario
   package needs CCC-consistent capacity factors it must order that
   extraction (the engine's own ERA5 CFs are the default per docs/05).

Nothing else was excluded: every ES1 Electric Engagement capacity row
is either mapped or listed (e4), and every Figure 7.5.3 bucket is
either mapped or carried as a flagged aggregate.

## 8. Files delivered

- `data/reference/pathways-published.toml` — the reference table
  (parses clean under Python tomllib; all four demand-component sums
  re-verified exact at load). Single-file layout follows the
  costs-gb.toml one-register-per-package precedent; fleet/storage field
  shapes follow fes-pathway-v1 so the scenario package can reuse those
  conventions.
- `data/packs/cb7.sha256` — manifest for the fetched CB7 raw files
  (files themselves gitignored per fetch-and-build law).
- `data/packs/cb7/raw/` — the four fetched CB7 artefacts (local only).
- This note.

Not delivered (out of scope, next package): scenario TOMLs, the
pathways-published-v1 parser + docs/03 registry entry, pinned
regression tests, demand-profile construction, CCC split rules.
