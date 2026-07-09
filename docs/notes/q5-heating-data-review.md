# Q5/Q11 heating data package — reviewer adjudication

Reviewer, 2026-07-03. Subject: the uncommitted Q5 heating data package —
`data/weather/gb_t2m_pop.{parquet,csv}` (fetched-not-committed) +
`data/packs/weather-gb-t2m-pop.sha256`, `scripts/era5-cf/derive_t2m_gb.py`
+ `validate_t2m_gb.py` + README append, `data/reference/heating-cop.toml`
(DRAFT), `docs/notes/q5-heating-data-report.md`. Contract:
`docs/notes/d9-heating-overlay.md` (ADOPTED — rules 2–4, data
requirements 1–7) and `docs/notes/d9-heating-overlay-review.md`
(ruling A, edit 6). Everything below was re-run or re-derived by the
reviewer; nothing was accepted on claim.

## VERDICT: ACCEPT-WITH-NOTES

The trace, its manifest, the validation machinery, and the source
transcriptions are excellent — every checksum, every table number, and
every derived quantity reproduced exactly, including an independent
raw-cutout reproduction that bypasses the pinned code path. Two
conditions are BLOCKING for engine consumption: (1) the D9 edit-6
cross-check is NOT complete — items (i) and (iv) were reassigned to the
engine package against the contract's explicit text, and the reviewer's
own computation shows the deratings WILL fire for both technologies, so
the absent numbers are load-bearing; (2) the two surfaced basis
decisions are adjudicated below and the quantum must be restated
(including a third basis element the package missed: weather
normalisation to the record mean, worth ~13% on the space-heat share).
`heating-cop.toml` therefore stays at `status = "draft"` until
conditions 1–2 land. The trace itself is accepted as delivered.

## Verification record (all reproduced by the reviewer)

**Trace integrity.**
- `shasum -c weather-gb-t2m-pop.sha256`: all three files OK.
- `validate_t2m_gb.py`: exit 0 — 701,280 periods, uniform 30-min UTC,
  no NaNs, range [−8.66, 33.61] °C, mean 10.21 °C, CSV agrees.
- GB CF path untouched: `git status` shows additions + README append
  only; `derive_cf.py` byte-unchanged; no manifest edits.
- Import claim genuine: `derive_year` is the EU `temp_hourly_c` path
  (`load_point_means` → K−273.15 → `weighted_cf` → `to_half_hourly`)
  character-for-character; no forked derivation logic.
- Independent raw-cutout reproduction (reviewer's own box-mean/weighting
  code, not the pinned functions): January 2024, 744 hourly points,
  max |diff| vs trace = 2.9e-06 °C. Spot re-run `--years 2024`
  reproduces annual mean 11.02 °C. Annual means track CET (1986
  coldest 8.70, 2022 warmest 11.26).
- Weights: NEW 20-cluster population set (no prior GB population
  weighting exists in the repo; EU TEMP is the precedent and the
  honesty level is fairly claimed). Total 42.0M, Scotland 7.9%, Wales
  3.6% — matches the script's own statement. The Busby cross-check uses
  the same weights (verified in code and re-derived).

**Cross-check (ruling A).** Harmonic fit reproduced exactly (mean
10.2091, amplitude 6.0363, surface min 26 Jan). Kusuda–Achenbach
reproduced exactly: damping 0.7130, lag 19.66 d at z=1.0 m,
α=0.87e-6 m²/s (band 0.6890/21.66 … 0.7327/18.08). Busby-weighted
measured climatology reproduced exactly (mean 12.08, amplitude 5.46,
winter min 6.63); deviations −1.87 / −1.15 (−21.1%) / −0.73 °C, model
min 14 Feb inside the measured 30 Jan–22 Feb window. Source checks:
Busby 2015/2016 PDFs fetched, sha256 match the toml records; all 20
transcribed station rows verified against Table 1 (Busby spells it
"Whitechurch"); England/Scotland 100 cm minima windows verified; the
+0.9 °C soil-above-air offset (12 comparisons, 0.5–2.0) verified;
texture-class diffusivity medians and site range verified; "100 cm =
average horizontal-loop depth" verified.

**COP chain (edit 6).** When2Heat paper fetched, sha256 matches;
ASHP 6.08/−0.09/0.0005, GSHP 10.29/−0.21/0.0012, WSHP 9.97/−0.20/0.0012
verified against the paper; eq. 6 heating curves (40−1.0·T, 30−0.5·T),
50 °C DHW sink, 15 K ΔT floor, 0.85 Günther correction, 5 K GSHP brine
subtraction — all verified verbatim. RHPP report fetched, sha256
matches; Table 3-2 verified in full (all medians, IQRs, means, CIs, N;
SPFH2 2.65/2.81, SPFH4 2.44/2.71; SH-only and DHW-only comment values
too); glycol 4–7% over-read and <4% missing-data caveats verified;
Table 4-1 85% gas-boiler efficiency verified. Record COP ranges
reproduced exactly (ASHP 2.180–4.116, GSHP 2.607–6.298). The
correction-factor/derating structure makes stacking visible: one
published factor with a status field + one per-technology derating key
with a documented sentinel.

**District COP.** ADEME/BRGM communiqué fetched, sha256 matches; the
sentence verified verbatim on p.3 — "1 kWh d'électricité consommé par
l'installation permet de produire environ 20 kWh de chaleur"
(production basis confirmed by "produire"). DECC 2015 fetched, sha256
matches; 6% bulk / 28% non-bulk average losses verified. Conversion
arithmetic verified (17.8–18.8); premise check verified independently:
15.0 / 6.298 = 2.38×, band bottom 1.91×. ET 7.1 workbook fetched,
sha256 matches (T_base 15.5 citation). ECUK arithmetic verified
(ktoe→TWh rows, sum 441.35, DHW fraction 0.1917).

**Licences.** Table verified against what is actually in the repo:
ERA5 attribution carried in the report JSON; W2H paper CC BY 4.0
(transcribed parameters only); OPSD recorded, not consumed; RHPP
transcribed numbers with citation under its acknowledgement terms (the
package correctly notes it is NOT OGL); ECUK/DECC/ET7.1 OGL v3; ADEME
one-sentence cited quotation; MIDAS not consumed. The Busby Table 1
transcription (20 of 106 stations, 3 values each) is the largest
verbatim extraction — cited, factual-data, research use: acceptable.
No redistribution problem found.

**Scope.** No engine code, no scenario edits, no doc edits beyond the
README append; data fetched-not-committed per .gitignore. The working
tree also carries the CONCURRENT stage7-cost-inputs files
(`data/reference/costs-gb.toml`, two stage7 notes) — outside this
package; they must not ride along in the Q5 commit.

## Defects and conditions (1–2 blocking for engine consumption)

1. **BLOCKING — edit-6 items (i) and (iv) not delivered; the report's
   compliance claim is false.** D9 rule 4 (adopted text): "The data
   package delivers all four items per technology; the cross-check
   counts as done only when they are all present." The report (§4)
   asserts "this package delivers all four edit-6 items" while
   reassigning the model-implied SPF (i) and the derating
   determination (iv) to the engine package, and ships
   `rhpp_derating = 1.0` sentinels. The items are computable from the
   delivered artefacts alone — the reviewer computed them: with rule-3
   heat weighting (degree-hour space shape + 19.2% flat DHW at the
   50 °C sink) over the pinned trace, implied SPFH2 ≈ **3.21 (ASHP)**
   and **3.80 (GSHP)** vs RHPP IQR bands 2.33–2.95 and 2.63–3.14 —
   **both outside, so the one-factor-per-technology deratings fire**,
   at magnitudes ≈0.83 (ASHP) and ≈0.74 (GSHP) to the medians. These
   are first-order for the ASHP-vs-GSHP ordering (the GSHP curve takes
   the larger haircut). Deliver items (i) and (iv) per technology
   (tuning target stated — median vs band edge; the glycol 4–7%
   over-read cuts the bands LOWER, strengthening that the derating
   fires), replace the sentinels, restate the record COP ranges and the
   district premise margins post-derating (the premise only gains
   margin), and correct §4.
2. **BLOCKING — quantum basis (reviewer adjudication of §5; see
   rulings below).** Restate `delivered_heat_twh` and the DHW fraction
   on the ruled basis: delivered heat, GB, record-mean. The package
   also missed a third basis element: ECUK 2024 is ACTUAL consumption
   in a warm year — 2024 degree-hours are 43,707 vs record mean 50,454
   on the package's own trace (−13.4%), so feeding the 2024 actual in
   as D9's record-mean quantum understates it by the same order as the
   fuel→heat swing the package did flag.
3. **Amplitude tolerance justification does not reproduce.** The claim
   "irreducible air-for-soil-surface bias ~1.0–1.3 °C at 1 m
   (Wallingford quantification)" is not derivable from the cited
   numbers: (13.3−12.1)/2 × 0.713 = **0.43 °C** at 1 m. The honest
   derivation is from the measured data itself: measured 1 m amplitude
   5.46 implies soil-surface forcing amplitude ≈ 5.46/0.713 = 7.66 °C
   vs air 6.04 — a ~1.6 °C surface-forcing shortfall the air-for-soil
   convention cannot reproduce, giving ~1.15 °C at 1 m. Re-derive the
   justification on that basis (tolerance 1.5 then covers
   re-derivation jitter) or tighten. docs/05: tolerances are
   evidence-based, not aspirational.
4. **Phase tolerance justification mismatch.** "±7 d covers the α-band
   lag spread (18.1–21.7 d)" — that spread is ±1.8 d and supports ±2,
   not ±7. The z-band (1.0–1.2 m) adds up to +6.3 d of lag (α_lo,
   z=1.2 → 25.99 d), which justifies extending the LATE edge only.
   Restate as an asymmetric window (measured window −2 d early / +7 d
   late, α- and z-band cited) or tighten to ±2 d. Observed (14 Feb)
   passes either way — nothing here is tuned-to-pass.
5. **District citation transcription slip.** The communiqué attributes
   1.69 TWh (2022) and 59 networks to FRANCE, with ÎdF concentrating
   the majority — not "54 installations … 1.69 TWh" as ÎdF. Correct
   the parenthetical (or cite the ÎdF-specific figure from the body of
   the communiqué if it exists elsewhere in the document).
6. **Nits.** (a) Add the "© NERC/BGS, published by permission of BGS"
   acknowledgement line to the `BUSBY_100CM` block in
   `derive_t2m_gb.py` (it is in the toml but the script is the
   transcription's home). (b) The report's claim that the DHW fraction
   is "robust to the basis choice" holds for fuel→heat but NOT for the
   weather normalisation in condition 2 (0.192 → ~0.17); restate.
   (c) The Q5 commit must exclude the concurrent stage7 files.

## Rulings

**Tolerances (item 2 of the brief).** Winter-minimum [−1.5, +0.5] °C
asymmetric: **APPROVED as designed** — it guards the decisive quantity,
binds the anti-conservative (warm) side tighter, and catches exactly
the failure mode the amplitude deviation could hide. Phase and
amplitude tolerances: magnitudes acceptable, justifications defective
(conditions 3–4). Not tuned-to-pass in effect — the observed values
pass the RAW measured window and sit under the honest ~1.15 °C
structural bias — but the written justifications must be made honest
before the numbers are pinned as the ruling-A fallback trigger.

**Amplitude direction (−21%).** Worked through: a smaller model
amplitude, IN ISOLATION, makes the modelled winter dip shallower —
modelled winter source WARMER, GSHP winter COP overstated:
**anti-conservative**. The package's "conservative where it matters" is
true only as a NET statement: the −1.87 °C mean bias dominates, so the
modelled winter minimum lands 0.73 °C COLDER than measured —
understating GSHP winter COP and the ASHP→GSHP gradient, the D9
lower-bound direction. The report states this correctly (§2, "nearly
cancel"). RULING: direction argument VERIFIED as stated, with the
proviso that it holds only while the mean bias persists — which is
precisely what the asymmetric winter-min guard enforces on any
re-derivation. Summer ground runs ~3 °C cold in the model; irrelevant
to heating load (DHW is flat), noted for completeness.

**District COP (item 4).** ADEQUATE AS A DRAFT PIN. The primary
sentence is verified verbatim; the production→delivered conversion is
cited and arithmetically correct; the extra margin (15.0 below the
converted 17.8–18.8) is in the conservative direction (more district
pump load → understates the geothermal advantage); the premise check is
machine-verified with 1.91× margin at the band bottom; leverage is low
(5.3–8.3% of delivered heat across the band). Conditions: keep
`status = "draft"` and echo the band into run outputs; fix the
transcription slip (condition 5); and before any PUBLISHED Q11/Q5
number rests on the district limb specifically, either land a
delivered-basis operating figure from a named scheme or quote the
[12.0, 18.8] band alongside the headline. The engine is NOT blocked on
a better primary.

**Basis decision (a) UK→GB.** RULED: scale by population share —
GB = UK × **0.972** (ONS mid-year estimates, NI ≈ 2.8% of UK; cite the
estimate year). Population is the right metric here because the demand
trace itself is population-weighted — internally consistent; households
or gas-share alternatives move the answer by well under the package's
other uncertainties. Applied to the quantum only; the DHW fraction is
dimensionless and unaffected by THIS scaling.

**Basis decision (b) fuel input vs delivered heat.** RULED: DELIVERED
(useful) HEAT. D9's `delivered_heat_twh` is heat; heat pumps replace
heat; `P_elec = heat/COP` is dimensionally heat-based — feeding fuel
input in overstates every electrified-heating result by ~1/0.85.
Conversion: per-fuel efficiencies over the ECUK U2 heat-class fuel
split (gas/oil/solid ≈ 0.85, RHPP Table 4-1 verified, or SAP cited;
direct electric 1.0; heat sold stated) — NOT a flat 0.85 across a
class that includes electricity. PLUS the record-mean normalisation
(condition 2): normalise the space-heat component by
mean/2024 degree-hours = 50,454/43,707 = ×1.154 from the pinned trace
(or use ECUK's temperature-corrected series, cited); DHW not
normalised. Note the elegant consistency: with this convention, rule
3's k equals the 2024-OBSERVED intensity (quantum_space/DH_2024 —
the normalisation cancels), which is exactly the fixed-stock
stationarity D9 rule 6(c) states. Illustrative chain at flat 0.85:
441.35 → space 356.75×1.154 + DHW 84.61 = 496.3 UK record-mean fuel →
×0.972 = 482.4 GB → ×0.85 ≈ **410 TWh delivered heat, GB, record-mean**
(per-fuel restatement will land it slightly higher, order 405–420;
the exact number is the data engineer's to restate with the fuel
table). DHW fraction on the same basis ≈ **0.17** (from 0.192). Every
artefact quoting the quantum states: basis = delivered heat; per-fuel
conversion from ECUK 2024 final energy, cited; GB = UK×0.972 (ONS);
weather-normalised to the 1985–2024 record mean by the stated method.

## Checklist disposition

Acceptance machinery re-run (manifest + validator, exit 0) ✓; ADR
compliance — determinism (no wall-clock/randomness/network in the
derive path), pinned code reuse, GB CF pins untouched ✓; no schema or
engine change, so no version bump due ✓; conventions — no library code
touched, so cargo fmt/clippy/test not applicable (verified nothing in
the workspace crates changed) ✓; data deliverables per docs/05 —
provenance + licences + checksums delivered and independently verified
(7/7 source hashes match), tolerance justifications defective per
conditions 3–4 ✓/✗; TDD posture — validator exists and is independent
of the deriver, the data-package analogue of red-first ✓; scope —
clean, with the stage7 ride-along hazard noted ✓.
