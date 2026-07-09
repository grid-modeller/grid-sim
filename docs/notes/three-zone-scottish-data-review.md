# Three-zone Scottish-boundary data package — adversarial review

Reviewer, 2026-07-04. Package under review (uncommitted): the `--three-zone`
extension to `scripts/fetch-b6/build.py`, `scripts/era5-cf/derive_cf_gb3zone.py`,
`docs/notes/three-zone-scottish-data-report.md`, and the two new manifests
`data/packs/b4.sha256` + `data/packs/cf-gb3-1985-2024.sha256` (built data
gitignored under `data/packs/b6/processed/` and `data/packs/cf-gb2/`). Judged
against the SIX BINDING OBLIGATIONS of the adjudication banner
(`scottish-group-boundary-scoping.md`), the rulings
(`scottish-group-boundary-design-review.md` items 3/5/6), the committed B6
pack + its data review (including the direction-inversion that review caught),
and `docs/05-validation.md`. Everything below was recomputed or re-run by the
reviewer; nothing taken on the implementer's word.

## VERDICT: ACCEPT-WITH-NOTES

The data reproduces byte-for-byte, all four manifests verify, both builders are
deterministic, the committed manifests are untouched, and the anti-tuning guard
(obligation 1, highest risk) HELD. One direction-label over-reach — the exact
failure class the B6 data review caught — must be corrected in the note before
commit (condition 1). It does not breach the model's lower-bound duty, so it is
a wording fix, not a structural defect. Conditions 2-3 are minor.

## Independent recomputation of the B4 stats (from the pinned raw file)

Recomputed directly from `neso_day_ahead_constraint_flows_limits.csv`, replicating
the builder's stitch/clock/sentinel handling:

| Metric | Report | Reviewer (independent) | Match |
|---|---|---|---|
| SSE-SP span | 2023-01-01 → 2024-04-20 23:30 | identical | YES |
| SSE-SP2 span | 2024-04-21 → present | identical | YES |
| Shared local-time labels (overlap) | zero | **0** | YES |
| 2024 periods | 17,280 (288 missing of 17,568) | 17,280 | YES |
| Net DA flow southward | 15.78 TWh | **15.782 TWh** | YES |
| Binding freq (flow ≥ 99% limit, non-sentinel) | 35.8% | **0.3578** | YES |
| Median limit | 1,800 MW | 1,800 MW | YES |
| Limit quantiles 1/5/25/50/75/95/99 | 1300/1500/1650/1800/2750/3100/3500 | identical | YES |
| Flow quantiles | −7598/91/1044/1782/2548/4402/5342 | identical | YES |
| Negative (import) share | 3.5% | 3.46% | YES |
| Limit == 0 sentinels | 42 | 42 | YES |
| Limit ≥ 9999 sentinels | **0** (never posted unconstrained) | **0** (0 anywhere in B4, all years) | YES |
| 2024 stitch provenance | SSE-SP 5,088 / SSE-SP2 12,194 | identical | YES |
| Exact dupes dropped / spring phantom | (48 / 4 across span) | 48 / 4 | YES |

The B4 series reproduces exactly. Zero-overlap stitch confirmed (0 shared labels;
clean version handover at 2024-04-21). The stitch/clock-change/dedupe/sentinel
code in `build_b4_series` is byte-for-byte identical to `build_b6_series` (design-
review item 4 requirement) — verified by reading both functions; they differ only
in the group filter (`SSE-SP`/`SSE-SP2` vs `SCOTEX`). B4 carries **no** ≥9999
no-constraint sentinel, unlike B6's 116 — confirmed independently.

## The anti-tuning guard (obligation 1, highest risk): HELD

- **CF partition** pinned from REPD operational northings, NOT tuned to the B4 DA
  series: forth_tay F_N = 1251.3/1258.3 = 0.9944; solar 0.694; onshore cluster
  split by cluster latitude; adopted onshore N-share 4006.7/9826.6 = 0.4077. None
  of these reference the 15.78 TWh / 35.8% figures. Verified.
- **N/S demand split** (33% N-Scotland ≈ 3.33% GB) is genuinely evidence-pinned,
  independent of B4: SSEN-North serves 740k/2.74m = 27.0% of Scottish customers
  (citable public DNO figure), uplifted to ~⅓ for higher northern per-customer
  consumption (off-gas-grid electric heating). The ⅓ figure was pre-ratified in
  the scoping (§1/§3) *before* this run. Crucially, the demand split is not even
  implemented in this data package (it is documented in report §4 for the scenario
  package) — so it structurally *cannot* have been tuned to B4 here.
- **Ruling on the 27%→33% uplift:** honest, not a hidden knob. It is stated as an
  explicit 27–33% bracket, anchored at the 27% customer-count floor, and the
  conservative direction is correctly identified (higher N demand → smaller N
  surplus → *understates* B4 export). The uplift magnitude to exactly 33% is a
  judgment rather than a derived number; the flagged P114 `_P`/`_N` v2 upgrade is
  the right pin for the shape. Soft observation only — the guard holds.

The 33% + 67% split sums to Scotland's committed 10.1% of GB (3.333 + 6.767 = 10.10).

## Direction / bias claims (scrutinised like the B6 review)

**(a) onshore adopted-split deviation −1.70% "= conservative, understates B4/B6
export" — HALF-BACKWARDS (condition 1).** The scenario allocates onshore CAPACITY
by REPD-northing (0.408 north) and TRACES by cluster (0.311 north).
`adopted_split_sco_energy_rel` onshore = −1.70% (2024; range −2.88%…−1.36% over
40y) reproduces (mechanism: nsco onshore CF 0.2563 < ssco 0.3077, so shifting
capacity north lowers *total* Scottish onshore energy). That metric is a **total-
Scottish-energy / B6-EXIT** quantity, and for B6 the negative sign is genuinely
conservative — a real improvement over the B6 pack, whose analogous split was
+3.5pp anti-conservative. **But the note extends the "conservative" label to "B4
export," and for the B4 gate that is the wrong sign.** Concentrating onshore
capacity north (0.408 vs cluster 0.311) *raises* northern generation per unit
Scottish capacity: 0.4078·0.2563 = 0.1045 vs 0.3113·0.2563 = 0.0798, i.e. +31%
more generation behind the B4 wall → *more* N→S flow → the adopted split
**overstates** B4 binding relative to the cluster geometry. This is exactly the
direction-conflation the B6 data review flagged (its condition 1: "conservative"
held only for the CF artefact in isolation). It does NOT breach the lower-bound
duty, because (i) the B4 magnitude is quoted only as a DA-anchored direction +
binding-frequency + wedge budget, never as a CF-derived validated magnitude, and
(ii) the headline curtailment/storage is restricted to direction + pinned totals
(obligation 2). But the sentence "hence understates B4/B6 export" must be split:
conservative for the B6 exit / total Scottish energy; anti-conservative for the
B4 gate (more capacity north of B4 than the cluster geometry implies).
Fix in report §3.

**(b) B5 folded into S-Scotland copper-plate "→ under-states → lower bound" —
CORRECT.** Folding B5 (3.9 GW, within SPT) into the copper-plate assumes no
constraint between the B4 exit and B6 entry → lets more energy move freely within
S-Scotland than reality → understates constraint → lower bound. Matches design-
review item 1 Failure Mode A verbatim. Verified correct.

**(c) ~19% offshore-commissioning wedge "overstates modelled B4 binding" —
CORRECT.** End-2024 constant-capacity convention holds Moray West/Seagreen/etc at
full capacity all year though they commissioned through 2024; 94% of Scottish
offshore is north of B4, so the ~3 TWh overstatement lands on the northern pool →
overstates northern generation → overstates B4 flow. 3/15.78 = 0.19. Sign and
magnitude correct; magnitude honestly quoted only within the wedge.

## forth_tay within-cluster split (obligation 5): CORRECT

North forth_tay = Seagreen 1075 + EOWDC 96.8 + Kincardine 49.5 + Hywind 30 =
1251.3 MW; South = Levenmouth 7 MW → F_N = 0.994. Total S-Scotland offshore fleet
= 181.0 = Levenmouth 7 + Robin Rigg East 84 + Robin Rigg West 90 (174) — arithmetic
confirmed from the site list. Robin Rigg is CF-assigned to `irish_sea` → `rgb`
(documented ~1.2%-of-fleet approximation carried from B6), which is why the CF
offshore N-share (0.994) exceeds the fleet N-share (0.939) — reconciled and
reported. NnG (450 MW) correctly EXCLUDED: full CoD July 2025, absent from the
Operational ≤2024-12-31 REPD filter (it never enters the fleet), carried as a
forward wedge — consistent with the B6 pack's exclusion convention. Correct.

## Cruachan both-ways (obligation 3): SUFFICIENT, default-N defensible

Cruachan Y=728,674 = 18,674 north of the 710k line (~18.7k; the design review's
~17k estimate was superseded by the actual REPD northing — more precise, still
north). Pinned N (740 MW: Cruachan 440 + Foyers 300 all north) with the sensitivity
(Cruachan 440→S, Foyers 300 N) reported both ways in `b4_report.json`. Given the
~18.7k-north position and SSEN connection, default-N is defensible on both northing
and electrical grounds; reporting both ways discharges obligation 3. The engine
package must still run the swap (design-review Edit 3) — flagged, out of data scope.

## Reconstruction + determinism (obligation 6): VERIFIED

- **Reconstruction identity** independently reproduced (w_nsco·nsco + w_ssco·ssco
  vs committed sco, 2024): onshore max |resid| 1.54e-07, offshore 1.42e-07, solar
  5.6e-17 — all far inside tol 1e-5. Report's stored 40y maxima (onshore 2.38e-07,
  offshore 1.79e-07, solar 2.98e-08) confirmed from the JSON. Tolerance basis
  (float32 cutout arithmetic, ~50× headroom) matches the cf-gb2 precedent.
- **Determinism.** Re-ran `build.py --three-zone`: all four b4 artefacts
  byte-identical. Re-ran `derive_cf_gb3zone.py --years 2024`: all 6 nsco/ssco 2024
  traces byte-identical. Env pins match (Python 3.13.11, pandas 3.0.3, pyarrow
  24.0.0, numpy 2.5.0).
- **Manifests.** `b4.sha256` 4/4 OK; `cf-gb3-1985-2024.sha256` 481/481 OK.
  Additive proof: committed `b6.sha256` 21/21 OK and `cf-gb2-1985-2024.sha256`
  481/481 OK after both re-runs. `derive_cf.py`, `derive_cf_gb2zone.py`,
  `b6.sha256`, `cf-gb2-1985-2024.sha256` all git-unmodified.
- **No forked logic.** `derive_cf_gb3zone.py` imports the pinned `derive_cf`
  (OFFSHORE_CLUSTERS, wind/solar pipelines, derive_raw, pinned_2024_factors,
  load_point_means, write); no physics re-implemented. The `build.py` diff is
  purely additive (docstring + `build_b4_series` + `build_fleet_split_3zone` + a
  `--three-zone` branch); the committed b6 functions are untouched.

## Licences (obligation 7): CLEAR

REPD (OGL v3.0), NESO DA constraint flows/limits (NESO Open Data Licence) — both
already cleared in the B6 review. SSEN/SP DNO customer counts are factual public
company figures, cited-not-packed. Attribution strings carried unchanged. The
unsourceable items are correctly flagged NOT substituted: B5 flow/limit/cost
(folded, stated); ETYS B4/B5/B6 capabilities pinned to a JS-rendered page
(condition-4 debt, now covers B4/B5); per-boundary curtailed volume (BOALF, v2);
half-hourly N/S demand shape (P114 `_P`/`_N`, v2). Honest.

## Obligations checklist

1. Anti-tuning — HELD (see above). 2. Direction + pinned totals only — HONORED:
report §6 states the prohibition and quotes no "B4 effect proper" % or B4-vs-B6
decomposition (grep confirms the only two mentions are the prohibition itself);
Edit-1 "adequate representation" overclaim does not appear (lower-bound framing
throughout). 3. Cruachan both-ways — DELIVERED. 4. Border-clearing order — a
scenario obligation, correctly out of data scope (flagged to the scenario package).
5. forth_tay straddle + REPD-vs-CF reconciliation — DELIVERED. 6. GB-internal vs
continental separate families — an ADR/scenario obligation, N/A to data. Nothing
was quietly dropped on the data side.

## Scope

Matches the work order: B4 series, 3-zone fleet split, N/S CF sub-traces, anchors/
wedges, Cruachan sensitivity. No engine code, no scenario file, no schema, no
`memory/`, no `docs/04` touched. (An untracked `figures/` directory is present in
the tree; not part of this package and not reviewed — flag to confirm it is not
an unauthorised addition riding along with the commit.)

## Conditions

1. **(Pre-commit)** Report §3: split the "−1.70% = conservative, understates
   B4/B6 export" claim. It is conservative for the **B6 exit / total Scottish
   onshore energy** (correct sign), but the REPD-northing capacity concentration
   (0.408 north vs cluster 0.311) places MORE generation behind B4 than the
   cluster geometry, so for the **B4 gate** the direction is anti-conservative
   (overstates B4 binding). State both. Note that the lower-bound duty is not
   breached only because B4 magnitude is quoted as a DA-anchored wedge, not a
   CF-derived number. (Same failure class as B6 data-review condition 1.)
2. **(Pre-commit, minor)** Report §4: mark the 27%→33% demand uplift magnitude as
   a stated judgment (not a derived figure) pending the P114 `_P`/`_N` shape pin,
   so it is not read as evidence-derived to the percentage point.
3. **(Confirm)** Confirm the untracked `figures/` directory is intentional / not
   part of this commit.

Report path: docs/notes/three-zone-scottish-data-review.md
