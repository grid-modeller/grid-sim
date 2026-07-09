# Review — EU external-zone CF derivation package (Stage 5 data deliverable)

**Date:** 2026-07-03 · **Reviewer:** review gate
**Scope:** the uncommitted package only — `scripts/era5-cf/derive_cf_eu.py`
+ `validate_cf_eu.py`; `scripts/fetch-entsoe/build_gen_agg.py` + the
`fetch.py` A75 extension + both READMEs; `data/packs/cf-eu-1985-2024.sha256`;
`data/packs/entsoe-2024.sha256` (28→31); `docs/notes/eu-cf-derivation-report.md`;
the project-state ledger entry. Rust changes (Stage 5 engine, in-flight)
explicitly excluded.

**Verdict: ACCEPT-WITH-NOTES.** Every delivered claim reproduced
independently; two prose defects (D1, D2) must be corrected in the commit
that lands this package; rulings (1)–(5) below are conditions on the
Stage 5 engine/scenario work order, not on this package.

## Checks run (all by the reviewer, none accepted on claim)

1. **GB path byte-unchanged — VERIFIED.** `git diff HEAD --
   scripts/era5-cf/derive_cf.py` empty; `derive_cf_eu.py` imports the
   pinned GB functions (power curve, PV model, `to_half_hourly`,
   `calibrate`, `weighted_cf`, honesty band) — reuse confirmed by reading
   both scripts. GB CF manifests untouched by the package (no GB pack
   files modified).
2. **Manifest + determinism — VERIFIED.** `cf-eu-1985-2024.sha256` has
   1,921 entries (960 Parquet + 960 CSV + `eu_cf_report.json`); FULL
   `shasum -a 256 -c`: 1,921/1,921 OK, zero mismatches. Determinism:
   reviewer re-derived the complete 2024 layer (`derive_cf_eu.py
   --years 2024`) into a scratch root under the pinned era5-venv — all
   48 output files byte-identical to the committed manifest. Mechanism
   verified by reading: no network/wall-clock/randomness in
   derive/validate; pure function of the two packs + in-script constants;
   `PINNED_FACTORS_EU` drift guard confirmed live (rerun printed "pinned
   EU factors confirmed"). Partial runs do not clobber the full-sweep
   report (guard at derive_cf_eu.py:819).
3. **Validator — VERIFIED.** Full `validate_cf_eu.py` run by the
   reviewer: exit 0. It genuinely re-asserts the EU pack geometry
   (480 files, rows = hours × 13,189, full 121×109 lattice, no NaNs,
   per-month calendar spans, 350,640 hours — the eu-pack-box-review
   note 3 obligation, now committed code) and the trace contract
   (period counts 17,520/17,568; strict 30-min uniformity + tz=UTC +
   year-boundary asserts = UTC-cleanliness through clock changes;
   cross-year 30-min continuity; CSV/Parquet ≤1e-9; anchor-energy
   reproduction ≤1e-6 relative; uncalibrated ⇒ factor exactly 1.0).
4. **ENTSO-E extension — VERIFIED.** 96 A75 raw docs = 24 original
   (no/no2) + 72 new (fr/be/nl/delu/dk1/ie × 12). `fetch_one` is
   skip-if-exists (fetch.py:87-91), so the original fetch is untouched.
   Manifest 28→31: the 28 original lines are byte-identical to HEAD's
   (comm = 28/28); all 31 checksums verify locally. Licence wording in
   both READMEs and the note correctly states A75/A68 are NOT on the
   CC-BY free-re-use list, GTC clause-3.1 internal-anchor use,
   git-ignored, never redistributed, attribution carried.
5. **Calibration arithmetic — REPRODUCED for all 16 anchored series**
   (not just 3). Targets gen_gwh·10³/(cap_mw·8,784) recomputed from the
   anchor parquets match the report to 4 dp; factors match (out-of-band
   computed factors 1.5580/1.9716 exceed the naive target/raw ratio
   exactly as `gb.calibrate`'s clip-at-1.0 fixed point predicts — band
   verdicts identical either way); every calibrated 2024 trace mean
   reproduces its anchor to ≤1e-16 relative; every uncalibrated trace
   mean equals its raw CF; applied factors match `PINNED_FACTORS_EU`.
   DK1 onshore 0.7441 confirmed in-band. Honesty bookkeeping (which
   factors shipped applied vs 1.0) is exactly as tabled.
6. **Anchor-fault diagnoses — EVIDENCED, with one sourcing note.**
   - FR offshore: 3,955.469 GWh /(1,473 MW × 8,784 h) = CF 0.3057 vs raw
     0.3036 (+0.7 %) — reproduced. Real-fleet 1,473 MW = Saint-Nazaire
     480 + Fécamp 497 + Saint-Brieuc 496, named public nameplate figures
     (the GB public-knowledge honesty level). Diagnosis evidenced.
   - IE onshore: 13,367.86 GWh /(0.2850 × 8,784 h) = 5,340 MW —
     reproduced; ≈5.3 GW vs known ~5.9 GW all-island, within the stated
     ~10 %. Implied CF 0.5072 non-physical. Evidenced.
   - NL onshore/solar: implied CFs 0.1253 / 0.0020 are internally
     decisive (non-physical numerators); raw energies 19.7 / 25.0 TWh
     reproduced from trace × A68. The real-generation magnitudes
     (~15–16 / ~21–22 TWh) are asserted as "national statistics"
     WITHOUT a named source — see note D3.
   - IE solar: reviewer grepped the raw XML — no B16 in gen_ie_2024-10,
     first B16 period start `2024-11-13T17:00Z` in the November doc;
     `aggregation_gen_report_2024.json` carries the anchor-exclusion
     record (months_present 2024-11/12). Evidenced; the never-zero-fill
     rule (build_gen_agg.py:118-129) is the right call.
7. **Correlations — REPRODUCED.** From the shipped traces + GB pinned
   traces, A68 capacity weights: GB–IE 0.7082/0.7932, GB–FR
   0.4308/0.4784 (half-hourly/daily) — match the note's matrix to 4 dp.
   FR 40-y extremes (worst 2010 0.2131, best 2023 0.2631, mean 0.2336)
   reproduced. Monthly reconciliation r spot-checked for ie onshore
   (0.9373), fr offshore (0.6127), de solar (0.9928) — all reproduce.
8. **ssrd hour-ending — REPRODUCED.** Reviewer's scratch rerun printed
   the identical finding: clear day 2024-06-07 at 48°N 2°E, centroid −
   solar noon = +29.8 min ⇒ hour-ending.
9. **Scope/hygiene.** Data git-ignored (`data/**`), manifests committed;
   no edits to docs/02–05 or GB scripts; spatial-weight scrutiny: every
   offshore cluster set reconciles with its A68 capacity (BE 2.3 vs
   2,262 MW; NL 4.75 vs 4,739; DE 8.5 vs 8,456; DK1 1.53 vs 1,601),
   Anholt/DK1 and Great-Belt cut geometrically correct; the ledger
   entry is accurate (subject to D1).

## Defects (correct in the landing commit; none blocks the data)

- **D1 — count error:** "11 of 16 anchored series calibrate inside the
  band" (eu-cf-derivation-report.md §Calibration, and "11/16" in the
  project-state ledger entry) is wrong: the note's own table and
  `eu_cf_report.json` give **12 of 16** (reviewer count from the JSON:
  anchored 16, calibrated 12). One-word fixes in both places.
- **D2 — stale docstring:** build_gen_agg.py docstring says "the two new
  files are appended to the manifest"; three were (parquet, csv, report
  json) and the README correctly says three.
- **D3 — uncited magnitudes (note of record, fix-on-use):** the NL
  real-generation figures (~15–16 TWh onshore, ~21–22 TWh solar) that
  quantify the bias guidance carry no named source. The fault DIAGNOSIS
  stands without them (implied CFs are internally non-physical), but any
  recalibration or sensitivity bracket that uses these magnitudes
  (ruling 1) must first pin them to a named CBS series with licence
  status per docs/05.

## Rulings

**(1) Uncalibrated-five fitness for Stage 5.** Differentiated, not
blanket:
- **FR offshore, IE onshore, IE solar: fit-to-consume as shipped**, with
  the pairing conditions of ruling (3) (FR offshore paired with the
  1,473 MW named-farm fleet, never A68's 1,003; IE onshore with a
  documented ~5.9 GW all-island figure, never A68's 3.0). FR offshore is
  ~1.5 GW in a nuclear-dominated zone (immaterial to FR scarcity); IE
  onshore's raw level is corroborated three ways (GB factor 1.04
  analogue, the 5.34 GW inversion, monthly r 0.9373); IE solar is
  ~1.5 GW, immaterial.
- **NL onshore + NL solar: NOT fit for unconditional consumption in the
  A4 acceptance run.** Paired with A68 capacities they overstate
  CONT-NW supply by roughly +4 and +3 TWh respectively — several times
  the ±1.5 TWh A4 per-border tolerance, and material to A1's ±3.3 TWh.
  Condition on the Stage 5 acceptance runs (the middle option, with an
  escalation trigger): the run report must carry a documented
  sensitivity bracket for NL onshore/solar (at minimum the stated-bias
  end points, ≈×0.78 onshore / ≈×0.85 solar, vs ×1.0), and A1/A4 pass
  verdicts are valid only if they hold across the bracket. If any gate
  verdict flips within the bracket, a national-statistics recalibration
  (CBS — separate source and licence check, per D3) becomes mandatory
  before the gate can be declared, not input tuning.

**(2) Spatial weights.** GB-precedent honesty level MET. Every point
(name, lat, lon, GW) is inline in derive_cf_eu.py for all six countries
× all techs + temperature, flagged APPROXIMATE at file, README, and note
level; offshore clusters reconcile with A68 capacities; the IE
true-fleet-vs-stale-A68 inconsistency is deliberate and documented both
ways.

**(3) Capacity-pairing rule.** Currently prose-only (derive docstring,
README, note §Calibration semantics, ledger) — nothing mechanical stops
a Stage 5 scenario builder pairing the IE trace with A68's stale 3.0 GW
or the FR offshore trace with 1,003 MW. CONDITION on the Stage 5
fleet-table/scenario deliverable: it must include a per-zone-per-tech
capacity-source table naming, for every CF trace consumed, the paired
capacity and its source, with **IE onshore explicitly not from A68** and
**FR offshore explicitly the 1,473 MW real fleet**; the Stage 5 review
checks this table against the pairing rule before the acceptance runs
are accepted. Recommended (non-blocking): a future rev of
`eu_cf_report.json` adds a machine-readable `pair_with_capacity_mw` /
`capacity_source` field per series so the check can become mechanical.

**(4) DK1 onshore 0.7441 band-edge keep: SOUND.** It is inside the
pinned band; the old-stock/low-hub-height explanation is physically
coherent (raw 0.3411 from a modern-fleet curve vs observed 0.2538);
monthly r 0.9852 shows the shape is right and only the level is scaled;
shipping it uncalibrated would knowingly overstate DK1 wind energy by
~34 % — strictly worse. The note already flags it as the weakest
calibrated factor; the Stage 5 run report should carry that flag
forward. No further condition.

**(5) Temperature population-weighting: ADEQUATE for the Stage 5/Q5
uses.** The traces feed per-zone temperature–demand regressions fitted
against observed ENTSO-E zone load — any static weighting bias is
absorbed into the fitted coefficients; limitation stated (note §Limitations
6). NO2's three-city set matches its actual population distribution and
its hydro-driven-zone role. Revisit only if a use appears that needs
absolute (not regression-calibrated) temperature exposure.

## Conditions of record (summary)

1. Fix D1 (11→12, note + ledger) and D2 in the landing commit.
2. Stage 5 run report: NL onshore/solar sensitivity bracket; A1/A4 valid
   only across it; CBS recalibration (with pinned citation + licence
   check, D3) mandatory if a verdict flips.
3. Stage 5 fleet-table deliverable: per-zone-per-tech capacity-source
   table enforcing the pairing rule (IE ≠ A68; FR offshore = 1,473 MW).
4. DK1 band-edge flag carried into the Stage 5 run report.
