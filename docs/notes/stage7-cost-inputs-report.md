# Stage 7 cost-inputs evidence note (data engineer, 2026-07-03)

**Status:** EVIDENCE NOTE — reviewer adjudicated ACCEPT-WITH-NOTES
(`docs/notes/stage7-cost-inputs-review.md`, 2026-07-03); **all twelve
conditions and notes x2–x4 are applied in this revision** (the two
blocking defects — the Ofgem real/semi-nominal WACC conflation and the
US-therm conversion factor — are corrected in §5 and §7). Serves
`docs/notes/d8-lcoe-methods.md` ("What this note does NOT do" list):
the TBD-DATA pins — WACC set, asset lives, capex/O&M with the
overnight-vs-IDC basis stated, price base year, storage cost splits,
holding costs, pathway fuel/carbon trajectories under the
opponent's-defaults discipline — plus the rule-9 emissions-factor gap.
Companion data file: `data/reference/costs-gb.toml` (DRAFT until the
supervisor pins into docs/04 citing the review; on adoption the schema
gets its docs/03 note and every later-published number its pinned
regression test — review condition 12). Nothing in this note edits
docs/04.

Every number below carries a named primary source and its licence.
Where a number could not be sourced openly, that is stated (§10) —
nothing was silently substituted (the NBP/ICE precedent,
`data/reference/prices-2024.toml`).

---

## 1. The capex basis (D8 rule-1/rule-4 obligation, adjudication note 1)

**All DESNZ Electricity Generation Costs figures are overnight-style
component costs**: pre-development, construction and infrastructure
costs are published as undiscounted £/kW (or £'000 per plant) together
with **separately published build durations and annual spend phasing**.
Interest during construction (IDC) is NOT embedded in the published
capex: DESNZ applies the hurdle rate as the discount rate across the
published phasing, so financing cost enters through discounting, not
as a cost line ("Financing costs: … applied as the discounting rate
and not as a separate cost component", Electricity Generation Costs
2025, "Presentation of costs"). **Consequence for the engine:** the
rule-4 CRF must be applied to capex *escalated over the source's build
phasing at the same WACC* (or the build-time effect must be reported
as a named adjustment) — using the raw overnight number with a CRF
silently under-costs long-build technologies (nuclear 8-year build,
offshore 3-year) relative to the source's own method. **Per review
condition 11, the per-technology central phasing arrays are
transcribed into the TOML** (`predev_phasing` / `build_phasing`,
fractions summing to 1), so the implementer can honour this without
inventing a convention. Source build times are also recorded per
technology below.

**The current edition is Electricity Generation Costs 2025, published
14 January 2026** (verified on gov.uk this session; the 2023 edition
is superseded). All 2025-edition figures are **real 2024 GBP**.
Licence: OGL v3.0.
- Report: https://www.gov.uk/government/publications/electricity-generation-costs-2025
- Data: "Annex A: Additional estimates and key assumptions 2025"
  (xlsx), retrieved 2026-07-03,
  https://assets.publishing.service.gov.uk/media/69d8efec96c86b7513170229/annex-a-additional-estimates-and-key-assumptions-2025.xlsx

The 2025 edition covers solar, onshore wind, fixed/floating offshore
wind, unabated gas (CCGT/OCGT/recips), hydrogen-to-power (CCHT/OCHT),
gas CCUS, tidal, geothermal. **It does not update nuclear or
biomass** (§3).

## 2. Per-technology capex, O&M, lives (central; low–high in the TOML)

All converted to **real 2024 GBP** using the ONS GDP deflator as
tabulated (2024 = 100) in the DESNZ Fossil Fuel Price Assumptions 2025
workbook (§6): 2012→2024 ×1.3873, 2014→2024 ×1.3410, 2020→2024
×1.1714, 2021→2024 ×1.1731, 2023→2024 ×1.0384, 2025→2024 ×0.9692.
Every conversion is shown in the TOML comments. (The full deflator
chain and all derived conversions were independently recomputed and
verified by the reviewer — spot-check record items 5–14.)

Capex below = pre-development + construction £/kW (overnight, §1
basis). Site infrastructure is a separate per-plant £ figure in the
sources and is carried separately in the TOML (it is material for
onshore wind: £316/kW). "FOM+" = fixed O&M + insurance + connection
and use-of-system, the source's three fixed annual lines.

| Tech | Capex £/kW (2024£) | FOM+ £/MW/yr | VOM £/MWh | Life (yrs) | Build (yrs) | Basis / vintage | Source (licence) |
|---|---|---|---|---|---|---|---|
| CCGT (H-class, 1,666 MW, 2030 comm.) | 1,020 (810–1,120) | 16,000 + 2,500 + 4,400 | 5 | 25 | 3 (+2 pre-dev) | overnight, 2024£; eff. 54 % HHV new-plant | EGC 2025 Annex A (OGL v3.0) |
| OCGT (299 MW, 2030 comm.) | 620 (510–720) | 11,800 + 1,500 + 3,100 | 5 | 25 | 2 (+2) | overnight, 2024£; eff. 35 % HHV | EGC 2025 Annex A (OGL) |
| Nuclear (PWR FOAK 3,300 MW) | 5,820 (5,110–7,700) | 97,760 + 13,410 + 670 | 6.7 + fuel ≈ 6.7 | 60 | 8 (+5) | overnight, **2014£ ×1.3410** — DESNZ still uses the 2016-report assumptions (2025 edition withholds SZC/HPC assumptions for commercial sensitivity) | BEIS Electricity Generation Costs 2016 (OGL); **both-variants-quoted rule, below** |
| Onshore wind (51.6 MW, 2030 comm.) | 1,380 (1,040–1,910) + infra 316/kW | 18,400 + 3,800 + 17,700 | 0 | 35 | 2 (+8 pre-dev) | overnight, 2024£; net LF 36 % | EGC 2025 Annex A / Arup 2024 (OGL) |
| Offshore wind fixed (1,297 MW, 2030 comm.) | 2,670 (2,210–3,340) | 46,000 + 8,900 + 86,700 | 0 | 30 | 3 (+7) | overnight, 2024£; net LF 48 %; peer-review-adjusted Arup | EGC 2025 Annex A (OGL) |
| Solar PV >5 MW (52 MW, 2030 comm.) | 530 (410–560) + infra 117/kW | 5,900 + 1,600 + 1,600 | 0 | 38 | 1 (+3) | overnight, 2024£; net LF 12 % | EGC 2025 Annex A / Arup 2024 (OGL) |
| Biomass dedicated (22.9 MW) | 3,910 (3,160–4,670) | 87,870 + 15,490 + 17,360 | 8.2 | 25 | 2 (+3) | overnight, **2021£ ×1.1731** (2023 edition; not updated 2025) | EGC 2023 Annex A (OGL) |
| Battery Li-ion — power leg | 262 £/kW (2030 build); 416 £/kW (2018 build) | 12.9 £/kW/yr (4 h config) | 0 | 15 | 1–2 | derived split, **2012£ ×1.3873** — see §4; **non-quotable pending NREL re-verification (review condition 3)** | BEIS/Mott MacDonald 2018 (OGL) |
| Battery Li-ion — energy leg | 135 £/kWh (2030 build); 403 £/kWh (2018) | — | — | 15 (5,000 cycles) | — | as above; RTE 85 %, DoD 80 % | as above |
| H2 electrolyser (PEM, 2030 comm.) | 518 £/kWe input (per-kW-H2: 562 £/kW_H2 HHV, 2020£) | 39.1 £/kW_H2/yr | 3.6 £/MWh_H2 (annex VOM incl. stack replacement; 3.1 2020£ ×1.1714 — review note x3) | 30 | 3 | overnight, **2020£ ×1.1714**; eff. 1.27 kWhe/kWh_H2 HHV | DESNZ Hydrogen Production Costs 2021 Annex (OGL) |
| H2 storage (salt cavern) | levelised-with-convention (review condition 5): 0.27 £/kg = 6.9 £/MWh_H2-throughput at 9 cycles/yr, **binding**; no open capex £/kWh (§10) | — | — | — | — | levelised only, 2023£ ×1.0384 | DESNZ Hydrogen T&S Cost Report, Dec 2023 (OGL) |
| H2 reconversion (OCHT 100 MW FOAK, 2030) | 850 (750–950) | 19,400 + 1,500 + 3,100 | 5 | 25 | 2 (+2) | overnight, 2024£; **eff. 25 % as published = default; mandatory labelled sensitivity at ~29.6 % HHV (35 % LHV × 0.846) per review condition 6** | EGC 2025 Annex A / Baringa (OGL) |
| H2 reconversion (CCHT 900 MW, 50 % blend FOAK) | 1,170 (1,060–1,380) | 14,500 + 2,500 + 4,400 | 5 | 25 | 3 (+2) | overnight, 2024£; eff. 51 % HHV | EGC 2025 Annex A (OGL) |

**Nuclear stale-source flag and the both-variants-quoted rule (review
condition 4):** DESNZ's only published nuclear cost assumption set
remains the 2016 report (£4,100/kW construction, 2014£). The observed
contemporary project cost is ~2× that: Sizewell C FID (gov.uk press
release, 22 July 2025, OGL) states a target construction cost of
"around £38 billion" (2024 prices) for 3.2 GW ≈ **£11,900/kW**,
described as ~20 % cheaper than Hinkley Point C. Both are carried as
labelled variants — `desnz_2016` (the opponent's default) and
`sizewell_c_observed` — and, per the review's ruling, **any published
nuclear-containing headline must quote the bracket (both variants)**,
with the basis caveat (overnight component set vs all-in project cost)
appearing wherever the bracket does. Naming one variant alone is not
sufficient. Kill-criterion-4 prominence discipline applies.

**Cross-checks (licence-checked):** NREL ATB — the ATB datasets are
published as open data (Creative Commons Attribution per the NREL/OEDI
data-catalog entries); however **nrel.gov domains did not resolve from
this build environment** (corroborated independently by the reviewer),
so ATB values could not be re-verified against the primary this
session (retrieval obstacle, §10; battery rows non-quotable until it
is cleared). Danish Energy Agency technology catalogues — **no licence
stated anywhere on ens.dk** (checked 2026-07-03): treated like
renewables.ninja under D1 — internal cross-check only, never shipped,
never load-bearing.

## 3. Asset lives (cited, same sources)

CCGT 25, OCGT 25 (EGC 2025 Annex A); nuclear 60 (EGC 2016); onshore
wind 35, offshore wind 30, solar PV 38 (EGC 2025 Annex A — note solar
lengthened vs older editions); biomass 25 (EGC 2023 Annex A); Li-ion
battery 15 years / 5,000 cycles (BEIS-MM 2018); electrolyser 30 years
with stack replacement costed in variable OPEX (HPC 2021); OCHT/CCHT
25 (EGC 2025). All are the source's "operating lifetime" — the rule-4
CRF n per technology.

## 4. Battery split derivation (stated, because it is derived)

BEIS/Mott MacDonald, "Storage cost and technical assumptions for BEIS"
(Aug 2018, OGL v3.0, **all costs 2012 prices**) publishes Li-ion capex
for a 50 MW/50 MWh (1 h) and a 200 MW/800 MWh (4 h) configuration.
The £/kW-vs-£/kWh split is obtained by differencing the two
configurations (3 kWh/kW apart):

- 2030-build, medium: 1 h = 286.6 £/kW; 4 h = 579.7 £/kW →
  energy = (579.7−286.6)/3 = **97.7 £/kWh**, power = **188.9 £/kW**
  (2012£) → ×1.3873 → **135.5 £/kWh + 262.1 £/kW** (2024£).
- 2018-build, medium: 1 h = 590.3; 4 h = 1,462.1 → 290.6 £/kWh +
  299.7 £/kW (2012£) → **403.2 £/kWh + 415.8 £/kW** (2024£).

**Review condition 3 ruling (applied):** the derivation is accepted as
the draft pin — the only OGL GB primary supporting a split — with
three binding caveats now carried in the TOML: (i) the rows are
**`quotable = false`** until the NREL 2025 storage-cost bracket is
re-verified against the primary from an unblocked network; (ii) the
differencing ignores the two configurations' differing infrastructure
costs (5.8 vs 34.1 £/kW medium, 2012£) and use-case archetypes (FM vs
PL-DA) — the split is a duration-attribution over CAPEX only;
(iii) the 2018-projection-vintage staleness stamp propagates to any
artefact quoting a battery-containing cost. Sanity check: the
2030-build 4 h total (≈201 £/kWh 2024£) sits inside the NREL "Cost
Projections for Utility-Scale Battery Storage: 2025 Update" mid
trajectory ($247/kWh mid by 2035, low $152) — secondary-sourced,
hence the quarantine.

## 5. WACC set (D8 rule 4 — three real rates, uniform)

Primary evidence, both ends of the financing spectrum:

1. **CEPA for DESNZ, "Hurdle rate estimates for electricity sector
   technologies"** (published alongside EGC 2025; OGL v3.0;
   https://assets.publishing.service.gov.uk/media/68cd1d818c44a661b4995d9f/cepa-desnz-hurdle-rates-electricity.pdf).
   Basis: **pre-tax real (CPI), vanilla, whole-life, 50 % gearing,
   estimated at 31 Dec 2024**. Lead-scenario scale: L-M 7.60 %
   (solar, onshore, nuclear-RAB, cap-and-floor interconnectors),
   M 8.90 % (offshore, unabated gas, Li-ion batteries under CM
   contract, biomass), M-H 10.10 % (LDES, electrolysers, maturing
   CCUS/H2P), H 11.40 % (floating offshore, wave/tidal).
2. **Ofgem RIIO-3 Final Determinations, Finance Annex** (4 Dec 2025;
   https://www.ofgem.gov.uk/sites/default/files/2025-12/RIIO-3-Final-Determinations-Finance-Annex.pdf),
   Table 16, electricity transmission at 55 % notional gearing:
   **WACC allowance (real, CPIH) 4.46–4.67 %** (NGET 4.46, SPT 4.58,
   SHET 4.67; gas sectors 4.28 % at 60 % gearing). The same table's
   **5.53–5.74 % figures are the SEMI-NOMINAL WACC** (real cost of
   equity + semi-nominal cost of debt — Ofgem's cash-return
   construction, Finance Annex ¶4.3), NOT a real rate; the allowed
   real cost of equity is 5.70 %. [Correction per review condition 1:
   an earlier draft of this note mis-quoted the semi-nominal ~5.6 %
   as CPIH-real.] The regulated-asset financing floor, in the real
   terms this package uses everywhere else, is **~4.5 %**.

**Proposed uniform set (real): low 4.5 %, central 7.5 %, high 10.0 %.**
- Low ≈ Ofgem RIIO-3 allowed **real** WACC for electricity
  transmission (unrounded anchor range 4.46–4.67 %, Table 16 "WACC
  allowance (real)") — "everything financed like a regulated network
  asset" (the RAB/cap-and-floor limit; nuclear RAB and LDES
  cap-and-floor are policy instruments that exist precisely to move
  assets toward this rate).
- Central ≈ CEPA L-M lead scenario (7.60 %) — established, contracted
  (CfD-class) generation.
- High ≈ CEPA M-H (10.10 %) — nascent / merchant-exposed.
Rounding to 4.5/7.5/10.0 is proposed for headline hygiene; the
unrounded anchors (4.46–4.67 / 7.60 / 10.10) are recorded in the TOML;
the engine treats the set as data.

Stated basis caveats (so no false equivalence ships): the CEPA rates
are pre-tax real-CPI vanilla; the Ofgem real WACC is CPIH-real vanilla
with a post-tax equity component; and **Ofgem's semi-nominal WACC
(5.53–5.74 % ET) is a different object from its real WACC
(4.46–4.67 %) — a ~110 bp wedge that dwarfs the CPI/CPIH wedge — so
the low anchor must be taken from the "WACC allowance (real)" row and
nowhere else.** The set is a *sensitivity spread over cited GB
financing evidence*, not a claimed like-for-like composite, and every
artefact quotes it as such.

**Per-technology rates (labelled sensitivity only, per D8 rule 4):**
the full CEPA lead-scenario table and the EGC 2025 Annex A per-tech
low/medium/high hurdle triplets are transcribed into the TOML under
`wacc.per_tech_sensitivity` (all 14 assignments reviewer-verified
against CEPA Table E.1). Never the headline.

## 6. Price base year and deflator

**Recommendation: real 2024 GBP.** It is simultaneously: the EGC 2025
price base, the FFPA 2025 price base, and the year of the committed
actuals pack (`prices-2024.toml`). One conversion step (×0.9692) is
needed for the traded-carbon values (2025£) — shown in the TOML.

**Deflator series: the ONS GDP deflator at market prices** — cited
operationally as **HM Treasury "GDP deflators at market prices, and
money GDP" (quarterly publication, OGL)**, and pinned numerically this
session from the DESNZ FFPA 2025 workbook's "UK GDP Deflator" sheet
(source: ONS; 2024 = 100), so our conversions are bit-identical to the
ones DESNZ itself used. The conversion table used here is committed in
the TOML `[deflator]` block.

## 7. Fuel and carbon: 2024 reconciliation + pathway trajectories

**Reconciliation with the 2024 pack:** FFPA 2025 records the 2024 gas
outturn as 84 p/therm. Using the UK statutory therm (29.3071 kWh,
gross CV — the GB/NBP trading unit), p/therm → £/MWh_HHV is
**× 0.341214** (= 10 / 29.3071): 84 p/therm = **£28.66/MWh (HHV)**,
against the pack's ONS daily-SAP mean of £28.67
(`prices-2024.toml` cross-check line) — **agreement within £0.01/MWh**
between two independent series (within-day SAP vs the FFPA outturn).
[Correction per review condition 2: an earlier draft used the US-therm
factor 0.34130 and overstated the reconciliation as exact; FFPA 2025
publishes no therm→kWh factor of its own, so the UK statutory therm is
the named basis.] The pack stays the 2024 actual, FFPA supplies the
forward path. For carbon, the pack's 2024 UKA auction mean (£37.18) is
an *actual*; the traded carbon values are *modelling values* (their
"market" 2025 value is £44, 2025£) — actuals and planning values are
different objects and both are stamped as what they are.

**Recommended trajectory sources (the opponent's defaults — these ARE
the government's published planning assumptions):**

1. **Gas (and oil/coal): DESNZ Fossil Fuel Price Assumptions 2025**,
   published 3 Feb 2026, OGL v3.0, data tables xlsx retrieved
   2026-07-03. Real 2024 p/therm, low/central/high to 2050. Central:
   94 (2025) → 71 (2030) → 69 (2035) → 66 (2040, flat to 2050);
   low flat-lines at 32, high at 108 from 2040. Conversion
   p/therm → £/MWh_HHV: ×0.341214 (UK statutory therm = 29.3071 kWh,
   gross CV — consistent with the Stage 2 HHV chain). Full arrays in
   the TOML.
2. **Carbon: DESNZ "Traded carbon values used for modelling purposes,
   2025"**, published 3 Feb 2026, OGL v3.0, real **2025** GBP
   (converted ×0.9692): central £/tCO2e 29 (2025) → 50 (2030) → 70
   (2035) → 136 (2040) → 197 (2045) → 235 (2050); low 22→167, high
   37→298 (2050). Explicitly "not forecasts" — scenario-based
   modelling values; stamped as such. Note the EGC 2025 report itself
   switched to these assumptions (they no longer converge to the
   appraisal carbon value — a change DESNZ flags as lowering gas LCOE
   vs the 2023 edition; the report's Table 4 quantifies it at
   ~£56/MWh for a 93 % LF CCGT).
   **CPS treatment (accepted, review condition 8):** Carbon Price
   Support is £18/tCO2 *nominal*, frozen (HMT; SN05927 citation in
   prices-2024.toml). Convention: hold £18 nominal and deflate in real
   terms along the pathway, labelled; the alternative (real-terms
   continuation) is the labelled conservative variant.
3. **Cross-checks:** NESO FES 2025 input assumptions (NESO Open Data
   Licence; pack already fetched — `data/packs/fes2025`) for pathway
   fleet/demand consistency; CCC Seventh Carbon Budget datasets as an
   analytical cross-check only — **no explicit open licence found on
   theccc.org.uk this session** (their FoI page says check copyright
   before reproducing): quote-with-attribution, do not redistribute,
   licence question flagged (§10). Nothing from either is load-bearing
   in the TOML (review note x5).

## 8. Stability-service holding costs (rule 1.5, Q8 linkage)

Computed this session from the **already-pinned FY2025 EAC pack**
(`data/packs/response-holdings/raw/eac-results-summary-fy2025.csv`,
sha256 `901fd1ad…`, NESO Open Data Licence, retrieved 2026-07-03 —
same snapshot the Q8 work uses; no new fetch). Reviewer-recomputed
bit-identical (review condition 10: accepted). 2,190 four-hour EFA
windows per product, no gaps. Clearing prices £/MW/h:

| Product (EAC) | vol-wt mean | mean | p5 | p50 | p95 | min | max |
|---|---|---|---|---|---|---|---|
| DCL (containment, LF) | **3.31** | 3.23 | 0.89 | 2.54 | 7.29 | −0.15 | 34.25 |
| DCH (containment, HF) | 2.06 | 1.96 | 0.10 | 1.36 | 5.75 | −2.74 | 19.50 |
| DML (moderation, LF) | **6.05** | 6.21 | 3.08 | 5.42 | 11.38 | −4.68 | 25.00 |
| DMH (moderation, HF) | 1.24 | 1.57 | −2.03 | 0.74 | 7.82 | −15.22 | 18.00 |
| DRL (regulation, LF) | **14.19** | 14.15 | 7.35 | 13.36 | 23.34 | −2.68 | 62.85 |
| DRH (regulation, HF) | −3.42 | −3.25 | −10.05 | −4.66 | 8.17 | −16.93 | 20.33 |

Central: FY2025 **volume-weighted means** per product; range =
per-product p5–p95. Negative DRH prices are genuine (providers pay for
the charging opportunity) — a pooled "response price" would hide this;
keep per-product. Names match `response-holdings-2025.toml` services
(`dynamic_containment_lf` ↔ DCL etc.) so the cost file joins the
holdings file mechanically. Per instruction, this note does NOT touch
`docs/notes/q8-current-holdings.md`; format coordination is via the
shared product keys only. Annual cost sanity check: FY2025 mean LF
holdings (1,178/416/461 MW) × vw means × 8,760 h ≈ £34m + £22m + £57m
≈ **£113m/yr** for the low-frequency dynamic suite — right order
against NESO's published response-procurement spend.

## 9. Interconnector capex (rule 1.4)

No single open £/GW-km source exists; converter-station fixed costs
make a naive per-GW-km intensity misleading for short links.
Recommendation: **per-project cited capex for the modelled link
classes** (D5 zones), with intensity derived only as a reported
diagnostic. **Per review condition 9, only Viking Link is usable; the
other three rows carry machine-readable `verified = false` /
`quotable = false` flags in the TOML** so a committed reference file
cannot let the engine consume them by accident:

- **Viking Link (GB–DK1)**: £1.7bn, 1.4 GW, 765 km — National Grid
  press release, 29 Dec 2023 (company publication; facts quoted with
  attribution, not redistributed). → £1.21bn/GW; 1.59 £m/GW·km.
  The clean, citable anchor (`verified = true`).
- **North Sea Link (GB–NO2)**: 1.4 GW, 720 km, ~€2bn estimated —
  secondary-sourced this session; quarantined until pinned to a
  National Grid primary.
- **NeuConnect (GB–DE, CONT-NW class)**: 1.4 GW, ~725 km — public
  figures conflict (£1.4bn project cost vs £2.4bn reported financing
  package); Ofgem FPA cost tables are redacted. Quarantined; gap
  (§10).
- **Greenlink (GB–IE-SEM)**: 0.5 GW, ~190 km — cost not verified to a
  primary this session; quarantined; gap.
- CEPA assigns cap-and-floor interconnectors a 7.60 % hurdle rate
  (§5, reviewer-verified) — consistent with costing them at the
  low/central WACC.

## 10. What could NOT be sourced openly (named, per project law)

1. **Battery capex split, current-year GB primary.** Only open GB
   source with a derivable £/kW-vs-£/kWh split is BEIS/MM 2018 (2012
   prices). Current market benchmarks (BNEF survey, Modo Energy GB
   indices) are proprietary — rejected, not substituted. NREL ATB is
   openly licensed but **nrel.gov/openei.org did not resolve from this
   environment** (reviewer-corroborated) — values quoted here came via
   secondary snippets of the NREL 2025 storage-cost update; the
   battery rows are `quotable = false` until re-verified against the
   primary (review condition 3).
2. **Hydrogen salt-cavern storage capex (£/kWh_H2).** DESNZ publishes
   levelised £/kg only (T&S Cost Report 2023). Review condition 5
   ruling: carry the levelised number with the 9-cycles/yr convention
   **binding** — it must not be applied to a store cycling materially
   below that rate; **a seasonal (~1 cycle/yr) store re-opens this as
   a named blocker** and more evidence must be ordered then (a derived
   capex was rejected as an unsourced construction; DEA remains
   cross-check-only per D1).
3. **DEA technology catalogue licence** — no licence statement on
   ens.dk (checked 2026-07-03): internal cross-check only (D1
   precedent).
4. **CCC dataset licence** — no explicit open licence found on
   theccc.org.uk; quote-with-attribution only until clarified.
   > **SUPERSEDED (2026-07-06):** the CCC's site-wide copyright-terms
   > page states OGL v3.0, verified independently by the data engineer
   > and the reviewer (the original finding had reached only the FoI
   > page — a not-found-this-session result, not a conflicting one).
   > See `docs/notes/stage7-pathways-data-review.md` §8 and the
   > docs/08 D1 CCC licence addendum (2026-07-06). The original
   > finding above is kept verbatim as the dated record.
5. **NeuConnect / Greenlink / NSL project costs** — Ofgem FPA figures
   redacted for commercial confidentiality; press totals conflict
   (NeuConnect) or are unverified (Greenlink). Viking Link is the one
   clean anchor. Rows quarantined machine-readably (§9).
6. **Nuclear current-build capex as an assumption set** — Sizewell C's
   £38bn is a single press-released total (OGL), not a component
   breakdown; the only open component-level nuclear assumption set
   remains 2016-vintage. Both carried, labelled, and quoted as a
   bracket on any published headline (§2).
7. **MFR holding volumes/prices** — unchanged from the Q8 record: not
   published by NESO; excluded rather than estimated.
8. **OCHT efficiency basis** — the Annex A 25 % figure vs the report
   footnote's ~35 % LHV is unresolved in the open sources retrieved;
   per review condition 6 the Baringa H2P primary must be checked
   (one fetch) before any OCHT-containing number is *published*.
   Until then: default 25 % (the government's published number,
   opponent's-defaults discipline; the conservative/higher-cost end,
   direction stated), mandatory labelled sensitivity at ~29.6 % HHV
   (= 35 % LHV × 0.846, the H2 LHV/HHV ratio), and the
   annex-vs-footnote discrepancy stamped on every OCHT-consuming
   artefact.

## 11. Rule-9 emissions-factor gap (biomass / coal / "other")

From UK Government GHG Conversion Factors 2024, condensed set v1.1
(OGL v3.0; the same source and CO2-only convention as
`prices-2024.toml`), gross-CV basis. **Accepted by review condition 7
with the residual-reporting requirement made mandatory.**
- **Coal (electricity generation): 0.31530 kgCO2/kWh_th** (CO2-only,
  for UKA/CPS pricing; 0.31699 kgCO2e total).
- **Biomass (wood pellets): 0 kgCO2/kWh for carbon pricing** —
  biogenic CO2 is "outside of scopes" in the GHG CF framework and
  UK-ETS zero-rated; non-CO2 combustion factor 0.01132 kgCO2e/kWh
  (CH4+N2O) recorded for emissions *accounting*, not pricing.
- **"Other" fuel category:** no defensible single factor exists (the
  BMRS OTHER bucket mixes biomass CHP, waste, batteries, misc).
  Convention (now mandatory): carbon-price at zero, and **every
  emissions-priced artefact reports the unpriced "other" residual as
  a named quantity** — TWh, and a bounded tCO2 range where a bounding
  factor is stated.
With these, the emissions-priced cost lines' precondition (D8 rule 9)
is discharged for coal and biomass and explicitly conventioned for
"other".

## 12. Retrieval record

All gov.uk artefacts retrieved 2026-07-03 over HTTPS from
assets.publishing.service.gov.uk (OGL v3.0 unless stated); the Ofgem
RIIO-3 Finance Annex retrieved from ofgem.gov.uk (Table 16 re-verified
2026-07-03 for the condition-1 correction); EAC statistics computed
from the committed-manifest FY2025 snapshot (sha256 in
`data/packs/response-holdings.sha256`). The downloaded source files
were used for extraction only and are not committed (fetch-and-build
law); the TOML records every URL so the pack builder can re-fetch and
checksum them if the supervisor wants raw-file provenance pinned like
the UKA PDF precedent (review note x6: pack-builder task, not a
defect).
