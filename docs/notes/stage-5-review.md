# Review — Stage 5 multi-zone engine package (uncommitted)

**Date:** 2026-07-03 · **Reviewer:** review gate
**Scope:** the uncommitted Stage 5 package — `grid-adequacy/src/{flow,multizone}.rs`,
`inputs.rs`/`dispatch.rs`/`lib.rs` edits, `grid-core` schema v4
(`scenario.rs`, `error.rs`, frozen `v3-gb-2024-reference.toml` fixture),
`scenarios/gb-2024-5zone.toml` (+ v4 bumps to all committed scenarios),
`grid-adequacy/tests/{acceptance_stage5_2024,multizone}.rs`,
`grid-cli/src/{run,plot}.rs`, `grid-cli/tests/stage5.rs`.
Everything at HEAD is regression surface only.

**Verdict: ACCEPT-WITH-NOTES (commit-grade). `stage-5-validated` may NOT
be tagged: the A2 acceptance gate is red by measurement.** The package
is fit for trunk as a mid-stage state (the CLAUDE.md red-green
convention has acceptance tests written and failing during a stage);
the stage closes only when A2 is green — see ruling 1.

## Independent verification (nothing accepted on claim)

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean.
- `cargo test --workspace --release --no-fail-fast`: **364 passed /
  1 failed**, the single red being exactly
  `a2_gb_fr_direction_matches_observed_at_least_95_percent`
  (implementer claimed 365 = 364+1 — confirmed; supervisor's
  independent re-run agrees).
- Pinned single-zone digest `779d7444…` reproduced
  (`regression_2024.rs`, green).
- Every headline number reproduced independently from a fresh
  `grid-cli run --scenario scenarios/gb-2024-5zone.toml` (links.csv +
  per-zone dispatch CSVs, analysed with pandas against the ENTSO-E and
  2024 packs):

| Claim | Reviewer-reproduced |
|---|---|
| A1 gas +2.85 % | 74.86 TWh vs 72.79 → **+2.85 %** ✓ |
| A1 GB net imports +32.80 TWh (−1.5 %) | **32.795 TWh, −1.52 %** ✓ |
| A1 mix corr ≥ 0.99 both NL-bracket ends | test loops both ends, green ✓ |
| A2 direction match 82.19 % | **82.19 %** (matches 14,440/17,568) ✓ |
| A2 mismatch class model-export/obs-import | **2,618** (test: 2 surplus-side + 2,616 deficit-side; total mismatches **3,128**, not the reported 3,129 — off-by-one, immaterial) |
| A3 NO2 hydro r | **−0.058** ✓ (gate ≤ 0.15) |
| A3 continental imports r | **−0.333** ✓ (gate ≤ −0.25) |
| A3 NSL diagnostic | **−0.269** ✓ (inside ±0.15 of −0.399) |
| A4 BE / NL | **+2.96 / +2.96 TWh** ✓ (errors −1.20/+1.37, both inside ±1.5; even split is architecture — D5 ruling a) |
| Five-border table | FR +16.89 (−2.56), NO2 +9.62 (+0.00), DK1 +2.53 (−1.13), IE −2.16 (+3.02) — ungated, must appear in the run report |
| FR unserved 0.784 TWh | **0.784 TWh over 374 periods** ✓ |
| Module 5 | NO2 top bins **+1.24/+1.22 GW** (flat); FR mid ~**2.33 → 0.83** top bin; DK1 top bin **−0.56 GW** (export) ✓ |
| 5-zone run time | 1.08 s user (claim 1.23 s — consistent; no docs/06 target applies) |
| Determinism | `five_zone_rerun_is_bit_identical` green; engines pure (no wall-clock/globals/randomness; BTreeMap keying) |

- **Wedge closures re-derived from the packs, exact:** FR +7.439 GW
  (supply 514.91 with pumped excluded as stated − load 429.70 − border
  19.88 = 65.33 TWh); CONT-NW **−10.116 GW** = 71.24 TWh itemised
  unrepresented generation (NL "other" CHP 36.96 dominates) + 17.61 TWh
  non-GB net imports; NO2 +1.057; DK1 −0.957; IE −0.890. FR nuclear
  monthly availabilities reproduce A75/(61.37 GW × month-hours) to 4 dp;
  CONT-NW coal/biomass/nuclear and IE coal flat factors reproduce.
- **Wedge sensitivity (reviewer perturbation runs):** ±1 GW on the FR
  wedge moves GB net imports +1.59/−1.81 TWh; ±1 GW on CONT-NW moves it
  ∓0.65 TWh — A1's ±3.33 TWh margin tolerates ~1.5–2 GW of error on the
  largest wedges. But note the construction: wedges close each zone's
  observed annual identity, so A1's aggregate is substantially anchored
  by construction (Stage-1 FUELHH-`other` posture); the flow rule owns
  the residual −0.50 TWh and the per-border splits. The run report's
  circularity inventory must say this.

## Ruling 1 — the A2 red (the central ruling)

**Diagnosis verified.** Of 3,128 mismatches, 2,618 are
model-export-while-observed-import; 2,616 of those in GB pre-link
deficit. Hour-of-day (UTC): 876 (33.5 %) in 17–21 and 629 in 05–09 —
both clusters are FR demand peaks (CET evening/morning). FR modelled
unserved (0.784 TWh) clusters at the same hours (35.1 % of the energy
in 17–21 UTC); 136 mismatch periods have model-FR literally unserved
while real FR was exporting to GB. Mechanism (a) — flat +7.439 GW
wedge + flat 0.3518 hydro availability (7.2 GW) overstating FR peak
scarcity where real FR meets peaks with reservoir + pumped hydro
(17.44 + 9.46 TWh, peak-shaped, currently flat/excluded) — is
evidence-backed.

**Ruling: (ii) — remediable input-modelling deficiency; NOT
structurally unsatisfiable on this evidence; no re-pin.** Eliminating
the model-export/obs-import class alone bounds the achievable match at
(14,440+2,618)/17,568 = **97.1 % > 95 %**, so unsatisfiability is not
demonstrated. The engine already carries the machinery (schema-v4
`energy_budget`); the fix is a data/scenario package: FR half-hourly
per-type A75 traces → budgeted FR reservoir(+pumped) hydro mirroring
NO2, and peak-shaped treatment of the wedge's pumping component.
Replacing a flat availability with observed-resource modelling is
legitimate refinement; lowering the gate to meet a model that scores
**below the 92.30 % always-import base rate** is not — this model has
not earned a re-pin. Only if the remediated model still fails may a
re-pin be considered, and then the named boundary is mechanism (b)
(the common ladder's fuel/carbon-price blindness — already a stated
limitation in flow.rs prose).

**Tag: WAITS.** Stage 6 part-1 precedent (tag withheld until the work
order closes) and the CLAUDE.md hard rule (a stage is complete only
when its acceptance tests pass) both bind. Commit yes; tag no.

## Rulings 2–8

**(2) Flow rule — PASS.** flow.rs prose is normative and complete
(signal construction with surplus/stack/unserved regions, exact
piecewise-linear breakpoint walk — no iteration tolerance, the 64-pass
cap is headroom over ≤~13 breakpoints; sequential border dispatch with
the bias documented and "largest border first" honoured by the
scenario; links-before-storage with consequences stated; same-pair
pro-rata split per the adjudicated D5 design). Conservation property
test is real (proptest: per-zone identity + per-link
received = sent×(1−loss) + cap bound, green). Exporter stack-ceiling
(never exports into own unserved / out of storage) enforced by
`q_max = min(cap, ceiling − r_exp)` and unit-tested. Boundary
subtlety noted, harmless: direction is chosen by `signal()` but the
probes at an exact segment boundary can see a smaller gap → zero flow
recorded; no wrong-direction flow is possible. D4 rule-2/3 validation
re-enforced per zone with link net folded into must-take (an importer
may charge storage from post-trade surplus — stated).

**(3) Identity wedges — legitimate-disclosed (Stage-1 pattern).**
Every wedge itemised, cited, and reviewer-reproduced from the packs
(table above); the CONT-NW −10.116 GW is 80 % unrepresented generation
(dominated by NL decentralised CHP that the platform reports as
"other" on 1 MW of A68 capacity) and 20 % non-GB transit — disclosed,
with the winter-CHP-not-flat limitation stated. Load-bearing at the
~1.5–2 GW-error level for A1 (quantified above) but derived from
observed identities, not tuned to gates. External-zone unserved
(FR 0.784, NO2 0.539, DK1 0.193, CONT-NW 0.010 TWh) are flat-wedge/
calibrated-cap artefacts: acceptable-disclosed for A1/A3/A4, but
**gate-relevant for A2** (they are the failure mechanism) — must be
disclosed in the run report, not only in this review.

**(4) NO2 budget model — PASS, near-tautology owned.** Weekly
(336-period) release matches the pack's evidence grade (A72
weekly-budget-grade inflow proxy, §6); carry-forward and greedy
drawdown implemented as documented and unit-tested (cap, carry,
exhaustion-visible-to-flow-rule). Budget energies verified against the
pack (42.56 + 1.11 TWh; RoR 7.09 and wind 4.52 TWh as observed
must-take traces — the D5 "derived trace" option, condition met, never
dropped). The A3 NO2 limb is framed as reproducing observed structure
in docs/04, the test, and the module docs — consistently owned. NSL
annual +9.62 TWh lands on the NESO actual almost exactly; note for the
run report that budget-from-observed-generation makes the annual NSL
energy semi-constructed.

**(5) Schema v4 migration — PASS.** `SchemaVersion3Superseded` names
all four additions (link `name`, `loss`; fleet `energy_budget`;
demand `extra_profiles`) + the one-line migration; frozen v3 fixture is
byte-identical to HEAD's reference scenario and refused with the
message (tested); all five committed scenarios bumped and parsing
(suite green); validation covers zone-id uniqueness, endpoint
existence (multi-zone only — single-zone external ids stay legal),
loss ∈ [0,1), availability ∈ [0,1], budget coherence.
**Defect D1: docs/03-domain-model.md carries no v3→v4 migration note
yet** — required by docs/03's own header and the v1→v2/v2→v3
precedents; must land in the same commit.

**(6) eu-cf-review binding conditions — MET.** NL bracket hard-coded
into the A1 and A4 tests, both ends genuinely run (loop + OnceLock per
bracket, read and verified; both green). Capacity-source table present
in the scenario header with IE onshore explicitly NOT A68 (5.9 GW) and
FR offshore explicitly the 1,473 MW named-farm fleet; the +1.4 TWh
IE-anchor consequence disclosed. DK1 band-edge flag carried at the
fleet entry. (Bracket table + DK1 flag must ALSO reach the stage run
report at tag time — eu-cf-review conditions 2/4.)

**(7) Single-zone inertness — PASS with a wording correction.** The
pinned digest test is green and `single_zone_run_multi_is_bit_identical_to_run`
+ inert-links assertions close the proof (run_multi ≡ run on one zone,
run ≡ pin on data). The energy_budget guard in the single-zone path
(`UnsupportedFeature`, dispatch.rs:155-168) exists and is tested.
**But "run-path byte-untouched" is not literally true**: dispatch.rs
gained the guard, inputs.rs a pure refactor (`load_zone_inputs`),
run.rs a multi-zone early-return + all-zones data-file hashing —
behaviour-identical on single-zone (digest proves it), byte-identical
it is not. Claims of record must say "digest-identical".

**(8) Module 5 — numbers reproduced; publication rule imposed.**
Binning (20 equal-count bins of GB residual demand = demand −
weather-driven potential) is pinned in the code docstring and sound.
NO2 flat ~1.22 GW at the top bins, FR ~2.33 → 0.83 GW collapse, DK1
flip to −0.56 GW all reproduce. **The FR line is contaminated by the
A2 defect** (top residual bins are GB-tight winter evenings, exactly
where model-FR's scarcity is overstated), biasing FR capacity credit
LOW at the top bins. Publication rule: the Module 5 FR series is not
quotable until the FR hydro remediation lands and the chart is
re-cut; NO2's line carries the near-tautology caveat; DK1 the transit
limitation. The artefact is otherwise publication-shaped.

## Standard duties

- **No library panics** in the new modules outside `#[cfg(test)]`;
  structured errors throughout (NaN ceilings, out-of-order ladders,
  zone/input mismatches, budget-window coverage).
- **Newtypes**: raw f64 confined to `pub(crate)` flow internals;
  public API is Power/Energy/PerUnit.
- **No new dependencies** (Cargo.lock untouched) — verified.
- **Outputs embed hashes**: multi-zone summary.toml carries engine
  hash, scenario sha, per-file data-pack shas (now including EU/ENTSO-E
  inputs), per-zone result digests, links digest; links.csv/parquet
  both written (docs/06).
- **TDD**: single-package delivery per the Stage 0 audit-trail
  decision; acceptance tests match the docs/04 pins exactly; A2
  written honest-red rather than weakened — commendable.
- **Scope**: clean; edits to committed tests/scenarios are the
  mechanical v4 bump; no unauthorised doc edits (docs untouched — see
  D1); project-state update falls to the landing commit.

## Conditions

On the landing commit (blocking):
1. **D1**: write the v3→v4 migration note in docs/03-domain-model.md
   (mirror the v2→v3 note; content already correct in scenario.rs
   module docs and the error message).
2. Project-state ledger entry recording: A2 RED at 82.19 % with the
   ruling above; FR-hydro remediation package queued as the next Stage
   5 work item; tag withheld. Correct the mismatch total to 3,128 and
   the "byte-untouched" wording to "digest-identical" wherever claimed.

At stage close / tag (blocking the tag, not the commit):
3. A2 green after the FR observed-resource remediation package — or,
   only if remediation demonstrably fails, a reviewer-ruled re-pin
   naming mechanism (b) as the boundary.
4. Stage 5 run report (docs/notes/) at KC prominence: NL-bracket
   verdict table, DK1 band-edge flag, the circularity inventory
   (wedges anchor annual identities; NO2 budget = observed generation;
   availability factors calibrated to A75), external-zone unserved
   disclosure, the five-border table, Module 5 caveats incl. the FR
   publication hold.

Non-blocking notes:
5. Optional strengthening: pin run_multi-on-the-reference against the
   779d7444 digest directly (currently proven by composition).
6. Characterisation gap (implementer self-reported): external-zone
   availability factors are not re-derived by any test. Reviewer
   hand-verified FR nuclear (monthly, 4 dp), FR/CONT-NW/IE flat
   factors, and all five wedge closures — records in this note. Rule:
   follow-up (a derivation-check script or test alongside the FR
   remediation package), not a commit condition.

---

# Addendum — A2 escalation adjudication (2026-07-03, second review pass)

Ruling 1's escalation condition was reached: two remediation rounds
measured, A2 still red. The reviewer independently reproduced the whole
record before ruling (nothing accepted on claim).

## Reviewer verification of the remediation record

- Suite on the remediation tree: **368 passed / 1 failed** (the A2 gate,
  at the shipped weekly grain); fmt/clippy clean; digest pin
  `779d7444…` unmoved; the four new characterisation tests (direct
  run_multi-on-reference pin — prior note 5; availability-factor,
  budget-schedule and FR-trace re-derivations — prior note 6) all green.
- Pack additions verified: all 39 `entsoe-2024.sha256` checksums OK,
  the original 31 byte-unchanged; §10/§11 addenda read (B10
  pair-semantics correction of record noted — the aggregation table's
  9.46 TWh B10 figure quoted in this review's first pass is inflated
  ~2.5 TWh by generic gap repairs; the clean trace's 6.93 TWh governs).
- **Weekly grain (shipped): 80.60 %** reproduced — WORSE than the
  82.19 % flat model it replaced; the greedy week-start flush is a
  D4-policy artefact (model releases 8.25/8.22 TWh on window days 1–2
  vs observed ~3.7/day), and it contaminates Module 5 (398 of 1,756
  top-2-bin periods in the mismatch class) and leaves 0.079 TWh FR
  unserved + 162 FR-past-gas mismatches.
- **w=1 cumulative-observed envelope: 90.07 %** reproduced; mismatch
  class 1,297; **mechanism (a) categories ZERO** (FR-past-gas 0, FR
  unserved 0.000 TWh); both-zones-gas-marginal tie-breaks 1,122/1,297
  (86.5 %), night/shoulder-concentrated (646 of the class in 23–05
  UTC); bound if that class were eliminated **97.4 %**; export recall
  **79.0 %** (constant always-import predictor: 0 %). FR border net
  20.66 TWh vs NESO 19.45 (weekly: 17.69). A3 at w=1: −0.058 / −0.335
  (NSL −0.270) — continental limb IMPROVES. A1 mix corr at w=1: 0.9968.
- **Decisive bracket fact (reviewer-measured, both ends run):** at w=1
  the A4-BE verdict FLIPS inside the NL sensitivity bracket — Nemo
  error −1.44 TWh (shipped) / **−1.67 TWh (bias-corrected end, outside
  ±1.5)**; A1 imports +7.4 % / +6.2 % (inside ±10 %), gas −1.12 % /
  −0.62 %. The eu-cf-review ruling-1 trigger is therefore live: a CBS
  national-statistics recalibration of NL onshore/solar is MANDATORY
  before A1/A4 can be declared at the w=1 configuration.

## Ruling 1 (final) — grain and re-pin

**FR budget grain: w=1 (cumulative-observed envelope).** Weekly is
REJECTED for FR: it is measurably wrong-shaped (scores below the flat
model it replaced) because every grain coarser than the period lets the
greedy zero-foresight policy flush the window's water early; w=1 is the
only shape-correct option under D4, and it is not finer than the
evidence grain (A75 is PT60M). NO2 stays weekly — its gates pass and no
measurement demands refinement; the budget grain is a per-zone
disclosed choice, refined only where a gate red requires it.

**Near-tautology at w=1 — acceptable under the NO2 framing, with
stronger ownership language (mandatory text):** the FR envelope is an
observation-anchored boundary condition — cumulative modelled release
can never run AHEAD of cumulative observed release, but WHEN to release
below the envelope remains scarcity-driven, and the GB↔FR flow itself
stays emergent. The A2 gate content is the emergent flow direction, not
FR hydro seasonality (reproduced by construction). This text goes at
the energy_budget block, in the docs/04 amendment, and the run report.

**A2 re-pin (the protocol clause is exercised; boundary named):** the
gate is structurally unsatisfiable under the pinned model boundary —
the unpriced common merit ladder cannot rank two gas-marginal zones by
price (GB carbon-price floor / fuel-price asymmetry; flow.rs's named
limitation), and the measured residual is 86.5 % exactly that class.
New gate, two limbs, both required:

- **A2a: GB↔FR direction match ≥ 88 %** under the 50 MW dead-band
  (measured 90.07 % at w=1 — exact value pinned as a regression
  alongside the gate);
- **A2b: export recall ≥ 70 %** — the share of observed GB-export
  periods the model signs correctly (measured 79.0 %). This limb
  carries the information content: the 92.30 % always-import base rate
  scores 0 % on it, so the pair strictly dominates the constant
  predictor even though raw match sits below the base rate — the
  docs/04 text must state this explicitly rather than hide it.

The original ≥ 95 % is retained in docs/04 as the PRICED-LADDER target
(superseded-not-deleted, Stage 2 pattern), with the 97.4 % bound pinned
as the expectation when that work lands. **The priced ladder (per-zone
SRMC scarcity) is NOT required for Stage 5 completion** — it is a
different model class, natural to Stage 7 (ADR-9 SRMC machinery, cost
synthesis); mechanism (b) is the documented Stage 5 boundary.

## Ruling 2 — docs/04 amendment discipline

Amend A2 in place: keep the original pin and its re-pin clause visibly
as history; add the two-limb gate with exact measured values, the w=1
convention and its ownership sentence, the boundary naming with the
measured anatomy (1,297-period class; 1,122 both-gas-marginal;
night/shoulder concentration), the base-rate statement, and the
priced-ladder deferral with its ≥ 95 % / 97.4 %-bound expectation.
Reviewer checks the text before tag.

## Ruling 3 — tag

`stage-5-validated` MAY tag on the re-pinned gate + run report, without
waiting for the priced ladder — CONDITIONAL, in order:

1. **CBS NL recalibration package first** (mandatory, adjudicated
   trigger — the A4-BE flip inside the bracket at w=1): pinned CBS
   citations + licence check per docs/05, traces recalibrated,
   bracket collapsed to the calibrated point.
2. A1/A4 re-verified at the calibrated point under w=1. If A4-BE then
   lands outside ±1.5 TWh, the D5 ruling-c revisit triggers and the tag
   WAITS for its adjudication (bloc split vs an A4 re-pin to bloc-total
   with the even-split convention named — not pre-judged here).
3. Suite green: A2 test rewritten to the two-limb gate; exact-value
   regression pins re-measured at the final (w=1 + CBS) configuration.
4. Run report at KC prominence: both rounds' record INCLUDING that
   weekly scored below the flat model; the residual anatomy as an exact
   cross-tab (both-gas × hour), not a conflated sentence; the base-rate
   discussion; envelope ownership; A1 imports drift (+7.4 %/+6.2 %)
   disclosed; five-border table; NL-CBS record; DK1 flag; circularity
   inventory extended with the FR envelope; the §11 wedge-figure
   reconciliation (+7.537 GW is the B10-corrected like-for-like
   recomputation of the committed +7.439 GW — one line).

## Ruling 4 — Module 5 FR embargo

**Lift with caveat, once re-cut at the final w=1 + CBS configuration.**
Measured basis: at w=1 only 35 of 1,756 top-2-bin periods sit in the
mismatch class (2.0 %) — the residual defect lives at night/shoulder,
orthogonal to the tight evening bins where the capacity-credit story
lives; FR top-bin 2.70 GW. The weekly re-cut (1.66 GW) is NOT
publication-grade (398/1,756 top-bin contamination — flush artefact).
Caveats to carry on the artefact: the mechanism-(b) direction residual
with its hours named, and the envelope note (top-bin FR credit reflects
observed 2024 FR hydro availability — the correct input for a 2024
capacity-credit estimate, stated openly).
