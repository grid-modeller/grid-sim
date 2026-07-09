# Q5 heating-overlay data package — evidence note

Data engineer, 2026-07-03. Serves the ADOPTED design note
`docs/notes/d9-heating-overlay.md` (data requirements 1–7, rules 3–4) and
its adjudication `docs/notes/d9-heating-overlay-review.md` (ruling A,
edit 6). Reference file delivered: `data/reference/heating-cop.toml`
(**draft, reviewer-gated**). All retrievals 2026-07-03; every source's
sha256 is recorded in the reference file or below.

**Revised 2026-07-03** actioning the review conditions
(`docs/notes/q5-heating-data-review.md`, verdict ACCEPT-WITH-NOTES):
condition 1 (edit-6 items i/iv now delivered, §4 compliance claim
corrected), condition 2 (quantum restated on the ruled basis, §5),
conditions 3–4 (tolerance justifications re-derived, §2), condition 5
(ADEME attribution corrected, §6), condition 6 nits (BGS acknowledgement
line in the script; DHW robustness claim restated, §5). Every changed
number was re-verified against its primary or recomputed from the pinned
trace before writing; source sha256s re-checked on refetch (ADEME, RHPP
both match the recorded hashes).

## 1. GB population-weighted t2m trace (data requirement 1)

- **Files**: `data/weather/gb_t2m_pop.{parquet,csv}` (fetched-and-built,
  git-ignored) + `gb_t2m_pop_report.json`. Column `t2m_pop` (float64,
  °C), index `utc_start` — the D9 rule-2 pinned path/column.
- **Manifest**: `data/packs/weather-gb-t2m-pop.sha256` (verify with
  `cd data/packs && shasum -c weather-gb-t2m-pop.sha256`). Parquet bytes
  pin the builder generation (docs/05 manifest semantics; same era5-venv
  environment pinning as every ERA5 pack).
- **Method**: `scripts/era5-cf/derive_t2m_gb.py` — IMPORTS the pinned GB
  machinery from `derive_cf.py` (`load_point_means`, `weighted_cf`,
  `half_hourly_index`, `to_half_hourly`), exactly as the EU t2m path
  does. `derive_cf.py` is byte-unchanged; **all committed GB CF
  manifests are untouched** (verified: `git status` shows additions
  only). Source: the committed GB ERA5 cutout (`era5-2024.sha256`,
  `era5-1985-2023.sha256`), which carries t2m. Population weights: 20
  approximate GB city/metro clusters (documented point-by-point in the
  script; the EU TEMP honesty level).
- **Validation** (`validate_t2m_gb.py`, independent, exit 0):
  701,280 periods (30 × 17,520 + 10 × 17,568), strictly uniform 30-min
  **UTC** index 1985-01-01 00:00Z … 2024-12-31 23:30Z — continuous
  through every year boundary and every BST clock change (UTC has none:
  that is the assertion); no NaNs; no gaps; range −8.66 … +33.61 °C;
  record mean 10.21 °C; CSV/Parquet agree. Annual means track the CET
  record: coldest 1986 (8.70 °C), warmest 2022 (11.26 °C); 2010 = 9.03,
  2024 = 11.02.
- **Attribution**: "Contains modified Copernicus Climate Change Service
  information [2024]" (ERA5, CC-BY 4.0 — carried in the report JSON).

## 2. Ground-model cross-check (ruling A — the fallback trigger)

Fitted single annual harmonic on the full 40-year `t2m_pop` (closed-form
least squares): mean 10.21 °C, amplitude 6.04 °C, surface minimum 26 Jan.
Kusuda–Achenbach at z = 1.0 m, α = 0.87×10⁻⁶ m²/s (centre; band
0.7173–1.0295×10⁻⁶, Busby 2016 texture-class medians): damping 0.7130,
lag 19.66 days (band: 0.689/21.7 d … 0.733/18.1 d).

Measured reference: Busby (2015) 100 cm soil-temperature climatology
(106 Met Office stations, fitted seasonal cycles 2000–2010, sea-level
reduced), one station transcribed per population cluster and weighted
with the trace's own weights (station table in `derive_t2m_gb.py`).

| Quantity (1 m depth) | Model | Measured (Busby 2015) | Deviation |
|---|---|---|---|
| Annual mean | 10.21 °C | 12.08 °C | −1.87 °C |
| Annual amplitude | 4.30 °C | 5.46 °C | −1.15 °C (−21.1%) |
| Winter minimum | 5.91 °C | 6.63 °C | −0.73 °C |
| Date of minimum | 14 Feb | 30 Jan – 22 Feb (England window) | inside |

**Verdict: PASSES plausibility — the ERA5-soil-levels fallback is NOT
fired.** The deviations are quantified and structural, not mysterious:

- **Phase** is the ground model's load-bearing claim (the winter lag is
  why GSHP barely feels the cold snap) and it lands inside the measured
  window with the analytic lag alone — nothing tuned.
- **Mean −1.87 °C** decomposes into known, cited offsets: soil annual
  mean sits ~+0.9 °C above air mean (Busby 2015, 12 paired comparisons,
  range 0.5–2.0); the measured values are sea-level-reduced (+~0.3 °C at
  the clusters' real elevations); St James's Park is an urban-green-space
  station (UHI ~+1.0 to +2.2) carrying London's 23% weight (+~0.3–0.5).
  Sum ≈ 1.5–1.8 °C of the 1.87.
- **Amplitude −21%** is the D9-stated limitation working as expected:
  population-weighted AIR temperature stands in for soil-surface
  forcing, and the soil-surface annual swing exceeds the air swing —
  quantified from the measured data itself under tolerance 2 below
  (Busby's Wallingford surface-vs-air example, 13.3 °C surface range vs
  our 12.1 °C air range, shows the same direction but does not by itself
  quantify the 1 m bias).
- **Net winter minimum −0.73 °C**: the mean and amplitude biases nearly
  cancel at the point that matters, leaving the modelled winter source
  slightly COLDER than measured — the conservative side for the
  geothermal-value finding (understates GSHP winter COP, hence
  understates the ASHP→GSHP gradient; the D9 lower-bound convention).

**Proposed tolerances** (docs/05: quantify the irreducible discrepancy,
set the tolerance just outside, justify). Justifications for 1–2
re-derived per review conditions 3–4:

1. **Phase** (asymmetric): fitted 1 m minimum date within the measured
   England window widened −2 days early / +7 days late, i.e.
   **28 Jan – 1 Mar**. Justification, both edges derived from the pinned
   parameter bands: at z = 1.0 m the α-band moves the lag by −1.6/+2.0 d
   around the 19.66 d centre (18.08 d at α = 1.0295e-6, 21.66 d at
   0.7173e-6) — supporting ±2 d, no more; the loop-depth band 1.0–1.2 m
   only ADDS lag (later minimum), up to 25.99 d at z = 1.2 m with α at
   the band bottom = +6.3 d vs centre — justifying the LATE edge only.
   The earlier ±7 d symmetric window claimed the α-band justified ±7; it
   supports ±2 (review condition 4). Observed: 14 Feb, passes inside the
   raw measured window — nothing tuned.
2. **Amplitude**: |model − measured| ≤ 1.5 °C. Justification (re-derived
   per review condition 3; the earlier Wallingford-based claim of
   "~1.0–1.3 °C at 1 m" does not reproduce — the cited Wallingford
   numbers give (13.3 − 12.1)/2 × 0.713 = 0.43 °C at 1 m): the honest
   quantification comes from the measured data itself. The measured 1 m
   amplitude 5.46 °C implies a soil-SURFACE forcing amplitude of
   5.46 / 0.713 = 7.66 °C, against the air-trace amplitude of 6.04 °C —
   a ~1.6 °C surface-forcing shortfall that the air-for-soil convention
   cannot reproduce by construction, i.e. ~1.16 °C of irreducible
   amplitude bias at 1 m. Tolerance 1.5 sits just outside, covering
   re-derivation jitter (new weights, new α within the band). Observed:
   1.15 °C.
3. **Winter minimum** (the decisive quantity): model − measured within
   [−1.5, +0.5] °C — asymmetric because a model winter source WARMER
   than measured by more than 0.5 °C would be anti-conservative and must
   flag even inside the amplitude tolerance. Observed: −0.73 °C.
   (Approved as designed by the review.)

If a re-derivation (new weights, new α) breaches any of the three, the
ruling-A fallback fires: order ERA5 soil-temperature levels.

## 3. COP parameterisation (data requirement 2; heating-cop.toml)

Transcribed from Ruhnau, Hirth & Praktiknjo (2019), Sci Data 6:189
(paper **CC BY 4.0**; companion OPSD when2heat data package **CC BY 4.0**
(2023-07-27 release), code MIT — both licences recorded per edit 10):

- ASHP `COP = 6.08 − 0.09·ΔT + 0.0005·ΔT²`; GSHP
  `COP = 10.29 − 0.21·ΔT + 0.0012·ΔT²` (WSHP 9.97/−0.20/0.0012
  transcribed in comments only); ΔT = T_sink − T_source, floored at
  15 K ("in line with the manufacturer data").
- Weather-compensated heating curve (radiator convention, D9 rule 4):
  `T_sink = 40 °C − 1.0·T_amb`; floor heating `30 − 0.5·T_amb`
  (transcribed, unused in v1); DHW sink 50 °C constant.
- **Field-calibration correction factor 0.85** ("set to 0.85,
  corresponding to field measurements from Günther et al.") — status
  per edit 6(iii): **RETAINED**. The RHPP cross-check tests the
  corrected curve; the RHPP derating (§4) is the single additional
  per-technology factor — no double-derating.
- Corrected-curve record ranges on the pinned trace (report JSON):
  ASHP COP 2.18–4.12, GSHP 2.61–6.30. Post-derating ranges in §4.

## 4. RHPP field-trial cross-check (data requirement 3; D9 edit 6, items i–iv)

Lowe et al. (RAPID-HPC/UCL) for BEIS, "Final report on analysis of heat
pump data from the RHPP scheme", March 2017, Table 3-2 (cropped B2:
292 ASHP, 92 GSHP; 700-site trial, 2-min data, Oct 2013–Mar 2015).
Licence: © RAPID-HPC 2017, open publication on gov.uk, reproduction
with acknowledgement (not OGL — numbers transcribed with citation).
**SPF boundary named per number (SEPEMO)**:

| SPF (boundary) | Technology | Median | IQR | Mean (95% CI) |
|---|---|---|---|---|
| SPFH2 (unit + source fan/pump) | ASHP | 2.65 | 2.33–2.95 | 2.64 (2.60–2.70) |
| SPFH2 | GSHP | 2.81 | 2.63–3.14 | 2.93 (2.80–3.06) |
| SPFH4 (whole system incl. backup + circulation) | ASHP | 2.44 | 2.15–2.67 | 2.41 (2.37–2.46) |
| SPFH4 | GSHP | 2.71 | 2.48–3.02 | 2.77 (2.66–2.89) |

Cross-check boundary (edit 6(ii)): **SPFH2** — the boundary the
When2Heat corrected curve approximates (EN 14511-style unit COP) and the
EU RES Directive measure. Band caveats carried: heat meters calibrated
for water not glycol ⇒ published SPFs **over-estimated by 4–7%**
(RAPID-HPC did not correct); missing heat-meter data <4% on the median.
Medians are robust to the outlier filter (Table 3-3: 2.63–2.65 ASHP,
2.81 GSHP across filters).

**Record correction (review condition 1).** The original version of this
note claimed "this package delivers all four edit-6 items" while
actually delivering only (ii) the SPFH2 boundary naming and (iii) the
correction-factor status, and reassigning (i) the model-implied SPF and
(iv) the derating determination to the engine package. That claim was
FALSE against D9 rule 4's adopted text ("The data package delivers all
four items per technology; the cross-check counts as done only when they
are all present"), and the reference file shipped `rhpp_derating = 1.0`
sentinels. Items (i) and (iv) are computable from the delivered
artefacts alone; they are now delivered below, independently recomputed
from the pinned trace (they also reproduce the review's own computation:
3.208/3.801 at the pre-revision DHW fraction 0.192).

**Item (i) — model-implied SPFH2 per technology**, rule-3 heat weighting
over the pinned 1985–2024 trace. Weights: space by
`heat_need(t) = max(15.5 − T_pop(t), 0)` (degree-hour shape); DHW flat
at the 50 °C sink, fraction 0.170 (the §5 restated basis). COPs: the
§3 corrected curves with the 15 K ΔT floor; GSHP source = the §2 ground
wave − 5 K. The quantum cancels, leaving

`1/SPF = (1−f)·Σ[hn(t)/COP_sp(t)]/Σhn(t) + f·mean[1/COP_dhw(t)]`, f = 0.170:

| Technology | Implied SPFH2 | RHPP IQR | Position |
|---|---|---|---|
| ASHP | **3.221** | 2.33–2.95 | OUTSIDE (above) |
| GSHP | **3.838** | 2.63–3.14 | OUTSIDE (above) |

Both fall outside their bands, so per edit 6(iv) the deratings FIRE for
both technologies. The glycol caveat strengthens this: the published
bands over-read by 4–7%, so the true field bands sit LOWER and the model
overshoot is larger still.

**Item (iv) — one-factor-per-technology deratings**, determined
independently per technology. Tuning target: **TO-MEDIAN** (stated
convention — the band medians are the central field estimate; tuning to
the band edge would leave the model at the optimistic extreme of the
field evidence):

- ASHP: 2.65 / 3.221 = **0.823**
- GSHP: 2.81 / 3.838 = **0.732**

Recorded in `heating-cop.toml` (`rhpp_derating`, sentinels replaced).
Sensitivity to the DHW fraction is third-decimal (at the pre-revision
f = 0.192: 0.826 / 0.739); the pinned factors use the §5 restated
f = 0.170, consistent with the demand model the engine will run.
Because the published medians themselves over-read by 4–7% (glycol),
to-median derating is mildly GENEROUS to heat pumps — direction stated.
The GSHP curve takes the larger haircut, which moves the ASHP-vs-GSHP
ordering — exactly why these factors are load-bearing and had to be
pinned in the data package.

**Post-derating restatements:**

- Record COP ranges on the pinned trace: ASHP 2.18–4.12 → **1.79–3.39**;
  GSHP 2.61–6.30 → **1.91–4.61**.
- District premise margins (edit 7): `cop_const` 15.0 vs post-derating
  GSHP max 4.61 → **3.25×** (pre-derating 2.38×); at the band bottom
  12.0 → **2.60×** (pre-derating 1.91×). The derating only gains the
  premise margin; the district-lowest limb holds everywhere in the band.

The engine package's acceptance step is now REPRODUCTION: recompute the
implied SPFs under its own tests and match these factors (drift guard),
not a fresh determination.

## 5. ECUK heat quantum + DHW fraction (data requirement 4)

ECUK 2025 End Use tables (DESNZ, published 25 Sep 2025, latest data year
**2024**, **OGL v3**), Table U2, units ktoe (1 toe = 11.63 MWh).
Workbook re-retrieved 2026-07-03 at its 20 Apr 2026 revision (sha256
`98de3e94…` in the reference file); the revisions touched U5/U6
(services oil) and U3 (domestic 2017–2019) only — the 2024 U2 rows below
reproduce the original transcription exactly (re-summed from the
workbook this revision):

| End use, 2024, UK | ktoe | TWh (fuel) |
|---|---|---|
| Domestic space heating | 20,838.1 | 242.35 |
| Domestic water heating | 5,871.2 | 68.28 |
| Services space heating | 9,836.4 | 114.40 |
| Services water heating | 1,403.9 | 16.33 |
| **Buildings heat class (space + water, dom + serv)** | **37,949.6** | **441.35** |

**Restated basis (review condition 2 + basis rulings a/b): the quantum
is DELIVERED (USEFUL) HEAT, GB, RECORD-MEAN.** The original note
surfaced two basis decisions (UK vs GB; fuel vs useful heat) and missed
a third — ECUK 2024 is actual consumption in a WARM year, and D9 rule 3
defines `delivered_heat_twh` as the record-mean quantum. Full chain,
every step cited:

**Step 1 — per-fuel useful heat (not flat 0.85).** U2 fuel split of the
heat class (ktoe): gas 27,745.9 (73.1%), oil 4,716.0 (12.4%), solid
170.9 (0.5%), electricity 2,289.9 (6.0%), heat sold 544.7 (1.4%),
bioenergy & waste 2,482.2 (6.5%). Efficiencies, each cited: gas 0.85,
oil 0.84, solid (coal) 0.60, direct electricity 1.00 — all RHPP
Table 4-1 (EST-sourced system efficiencies, re-verified against the PDF
this revision); heat sold 1.00 (ECUK "heat" is metered heat delivered at
the building boundary — basis stated); bioenergy & waste 0.725 = centre
of the Table 4-1 combustion-appliance band [0.60, 0.85] (Table 4-1 has
no biomass row; the band bounds are its coal and gas/LPG entries;
leverage ±4.0 TWh = ±1.0% on the final quantum, band stated in the
reference file). Direct-electric 1.00 ignores the small 2024 installed
heat-pump stock inside the electricity column (useful > fuel for that
sliver) — understates the quantum, conservative, small.

→ UK 2024 useful heat: **space 303.4 TWh, DHW 72.0 TWh** (total 375.4;
the flat-0.85 shortcut gives 375.1 — the per-fuel restatement lands
slightly higher because electricity and heat sold enter at 1.0).

**Step 2 — weather-normalise the space component to the record mean.**
On the pinned trace, annual degree-hours (T_base 15.5 °C, half-hourly)
average **50,454 °C·h** over 1985–2024 vs **43,707 °C·h** in 2024 — 2024
was **−13.4%** vs the record mean, so the 2024 actual understates the
record-mean quantum. Factor: 50,454 / 43,707 = **×1.1544** (method:
pinned-trace degree-hours, stated in preference to ECUK's
temperature-corrected series so the normalisation and the rule-3 shape
share one definition). DHW is not normalised (temperature-independent
floor). → space 303.4 × 1.1544 = **350.3 TWh**; UK record-mean useful =
350.3 + 72.0 = **422.3 TWh**.

**Step 3 — UK → GB by population share.** GB = UK × **0.972** (ONS
mid-year estimates, mid-2024, series UKPOP/NIPOP: UK 69,281,400, NI
1,927,900 = 2.78%). Population is the ruled metric — the demand trace
itself is population-weighted, so the scaling is internally consistent.

→ **`delivered_heat_twh` = 410.5 TWh — delivered heat, GB, record-mean**
(space 340.5 + DHW 70.0). Inside the review's expected 405–420 landing.

**DHW fraction = 72.0 / 422.3 = 0.170** (dimensionless — the GB scaling
cancels; definitional basis: fraction of DELIVERED heat, space + water,
domestic + services, record-mean — exactly the quantum's scope, so the
flat floor and the degree-hour share are internally consistent).
Robustness restated (review nit 6b): the fraction IS robust to the
fuel→useful conversion (0.1917 → 0.1914 — space and DHW are served by
the same appliance stock) but NOT to the weather normalisation, which
dilutes the flat DHW share: 0.192 → **0.170**. The old "robust to the
basis choice" claim held only for the first of the two moves.

**Rule-6(c) consistency** (review ruling b): with this convention,
rule 3's intensity `k = quantum_space/DH_mean = space_2024/DH_2024` —
the normalisation cancels, so k is the 2024-OBSERVED intensity of the
fixed building stock: exactly the climate-stationary convention D9
rule 6(c) states.

## 6. District/deep geothermal effective COP (data requirement 6)

Cited operating figure: ADEME IdF / Région IdF / BRGM joint communiqué
(21 Feb 2024, Géoscan launch, p.3): in the EXISTING Île-de-France
deep-geothermal district-heating networks, **1 kWh of electricity
consumed by the installation produces ~20 kWh of heat**. Context, same
page (attribution corrected per review condition 5, re-verified against
the communiqué text this revision — the sha256 matches the recorded
value): France has **59 deep-geothermal urban heat networks**, with
Île-de-France concentrating "le plus grand nombre d'équipements et la
majorité de la production de chaleur géothermale en France avec 1.69 TWh
produit en 2022" — the 1.69 TWh (2022) and the 59 networks are stated at
FRANCE scope with ÎdF holding the majority; the earlier "54
installations … 1.69 TWh" parenthetical attributed both to ÎdF and the
"54" appears nowhere in the document. The 20:1 operating figure itself
IS ÎdF-specific ("dans les réseaux de chaleur existants en
Île-de-France"). That is a PRODUCTION basis; D9 edit 7 requires
DELIVERED-heat basis. Conversion with DECC (2015) measured bulk-network
distribution losses (avg 6%, range 3–11%, OGL): 17.8–18.8 delivered.
Whether the ADEME denominator includes network circulation pumping is
not stated, so the pinned draft takes margin below the converted range:

- **`cop_const` = 15.0 (draft), band [12.0, 18.8], basis: heat delivered
  to buildings ÷ total electrical draw** — basis stated next to the
  number, per edit 7.
- **Premise check**: `cop_const` must exceed the heat pumps' maximum
  record COP. Pre-derating (machine-verified in
  `gb_t2m_pop_report.json`): 15.0 vs 6.30 (GSHP max, corrected curve,
  ΔT floor): 2.38×, band bottom 1.91×. Post-derating (§4, the
  engine-facing curves): 15.0 vs 4.61 → **3.25×**, band bottom
  **2.60×**. The district-lowest ordering limb's premise holds with
  margin everywhere in the band.
- Sensitivity: across the band the district electrical draw moves
  between 5.3% and 8.3% of delivered heat — small against heat-pump
  draws. A named operator figure on the delivered basis would upgrade
  the citation; flagged as the package's weakest single source, with low
  leverage. Per the review's district ruling: before any PUBLISHED
  Q11/Q5 number rests on the district limb specifically, either land a
  delivered-basis operating figure from a named scheme or quote the
  [12.0, 18.8] band alongside the headline; the band is echoed into run
  outputs.

## 7. Licences (D1 discipline)

| Source | Licence | Use |
|---|---|---|
| ERA5 (Earthmover/ARCO mirrors) | Copernicus CC-BY 4.0, attribution required | trace derivation; attribution carried |
| Ruhnau et al. 2019 (paper) | CC BY 4.0 | parameters transcribed |
| OPSD when2heat data package | CC BY 4.0 (code MIT) | recorded per edit 10; not consumed |
| RHPP final report (BEIS/RAPID-HPC) | © RAPID-HPC 2017; reproduction with acknowledgement | SPF bands + Table 4-1 efficiencies transcribed with citation |
| ECUK 2025, Energy Trends ET 7.1, DECC 2015 heat networks | OGL v3 | quantum, T_base 15.5 °C, network losses |
| ONS mid-year population estimates (UKPOP/NIPOP) | OGL v3 | UK→GB scaling ×0.972 |
| Busby 2015/2016 (NORA manuscripts) | © NERC/BGS, published by permission of BGS | cross-check climatology + α transcribed with citation |
| ADEME/BRGM Géoscan communiqué 2024 | public press release, cited | district COP (production basis) |
| Met Office MIDAS (underlying Busby) | MIDAS Open = OGL, CEDA registration | NOT consumed directly — Busby's published analysis used instead |

No proprietary source consumed; nothing redistributed beyond transcribed,
cited numbers.

## 8. Gaps and surprises

1. **ECUK basis decisions — RESOLVED** (review basis rulings a/b +
   condition 2, actioned in §5): delivered-heat basis with per-fuel
   efficiencies; GB = UK × 0.972; space component weather-normalised to
   the record mean (the third basis element the original package missed,
   worth +13.4% on the space share). Quantum 441.35 TWh (UK fuel, 2024
   actual) → 410.5 TWh (GB delivered heat, record-mean).
2. **District COP citation quality** — production-basis press figure
   converted with cited losses + margin; low sensitivity (§6).
3. **Ground amplitude −21%** — structural, direction understood, winter
   minimum conservative; fallback NOT fired (§2). If the engine review
   wants the amplitude bias removed rather than tolerated, ERA5 soil
   levels remain the ordered fallback.
4. **When2Heat↔SPFH2 boundary match is approximate** (EN 14511 unit COP
   + field correction ≈ SPFH2); named next to every cross-check number
   per edit 6(ii) — the engine cross-check inherits this naming.
5. RHPP bands themselves over-read by 4–7% (glycol caveat) — carried
   with the band; the §4 to-median deratings are therefore mildly
   generous to heat pumps; direction stated.
6. `T_base = 15.5 °C` citation delivered: DESNZ Energy Trends ET 7.1
   Notes ("base temperature for heating degree days is 15.5 degrees
   Celsius", Met Office data) — the D9 rule-3 convention is now cited.
7. **Edit-6 compliance failure in the original package** — items (i) and
   (iv) were wrongly reassigned to the engine package and §4 claimed
   otherwise; corrected in this revision (record kept in §4, deratings
   0.823/0.732 pinned in the reference file).
