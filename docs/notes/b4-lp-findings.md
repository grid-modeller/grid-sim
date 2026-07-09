# B4 LP findings — perfect-foresight binding frequency on the Scotland–England arc (measured 2026-07-05)

**Measurement:** B4 boundary binding frequency (southward flow ≥ 99% of
the per-period observed 2024 DA limit) under the perfect-foresight
least-waste LP (`run_multi_lp_min_curtailment`,
`docs/notes/d12-mincurtailment-decision.md`), full-year 2024, three-zone
scenario `gb-2024-3zone.toml` **de-duplicated**: the dispatchable
pumped-hydro stores dropped from BOTH NSCO (Cruachan+Foyers, 740 MW) and
RGB (Dinorwig+Ffestiniog, 2,088 MW), since the exogenous
`pumped_storage_net` traces already carry every observed 2024 PS action
GB-wide. Pinned in `grid-adequacy/tests/acceptance_b4_lp.rs`.

## The numbers

| Dispatch | B4 binding frequency |
|---|---|
| Rule-based (committed, myopic) | **1.96%** (337/17,235) |
| Perfect-foresight LP — **band** | **[23.5%, 28.2%]** — floor 0.234639 (4,044/17,235), point 0.281578 (4,853/17,235) |
| Observed 2024 day-ahead schedule | **35.86%** (this convention) / 35.78% (committed convention) |

> **CORRECTION (2026-07-06, R7 stall fix — docs/08 R7):** the
> rule-based comparator row is 1.96 % (337/17,235) → **2.01 %
> (347/17,235)** on the fixed engine; the LP band and the observed
> rows are untouched, and the finding's ordering (rule ≪ LP band <
> observed-DA) is unchanged.

The band, not a point (Richard's ruling 2026-07-05): the MinCurtailment
objective has no link-flow term and both links are lossless, so B4 flow
is objective-degenerate where the LP is indifferent. The **point** is
the regression pin; the **floor** excludes the 809 binding periods in
which a downstream zone (SSCO or RGB) was itself curtailing — there the
spill could equally have sat north of B4, so the binding is a
solver-vertex artifact, not physics.

History: an earlier 31.75% was REJECTED (NSCO pumped-storage
double-count, +3.55pp); the interim 28.20% still carried the identical
defect class in RGB — removing it moved the point 0.281984 → 0.281578
(−0.04pp, immaterial but now treated).

**Convention note (stated once):** the LP test's mask drops B4's 42
zero-limit sentinel rows from the denominator as well as the numerator
(no real limit is posted to bind against); the committed
`acceptance_b4_3zone.rs` convention keeps them in the denominator.
Denominators 17,235 vs 17,277 — the same 6,181 observed binding periods
read 35.86% vs 35.78%. Both circulate; the difference is convention.

## Headline framing

> The perfect-foresight optimiser binds B4 in roughly **23–28% of
> periods** vs **~2%** under rule-based dispatch (**~12–14×**) — the
> choke is real and dispatch-limited, not geometry-limited. The
> rule-based flow convention, not the boundary, is what hides B4 in the
> committed model.

## Mandatory caveats (carried on every quote)

1. **Observed 35.86% is NOT a convergence target.** It is a day-ahead
   SCHEDULED position: the DA flow exceeds the posted limit in 32.9% of
   masked periods (max 6,974 MW against that period's 2,800 MW posted
   limit; the series maximum limit is 3,700 MW), so it measures
   constraint-managed scheduling, not physical dispatch. Perfect
   foresight is the optimistic bound on *physical* dispatch and should
   not be expected to reach it. The honest bracket is
   **rule-based 1.96% << LP band [23.5%, 28.2%] < observed-DA 35.86%**.
2. **Even the point likely overstates physical binding.** The three-zone
   data report's §3 onshore split (~+31% generation per unit of Scottish
   capacity north of B4) and §6 offshore-commissioning wedge (~19%) both
   bias modelled B4 binding UP, with no outturn cross-anchor to close
   them.
3. **HiGHS floating-point:** the degenerate-vertex selection is
   deterministic on one machine (ADR-5, serial simplex) but may differ
   across platforms/HiGHS builds; the pins carry ±0.01 tolerance for
   this, and the band — not any single vertex's statistic — is the
   quotable result.
4. **The 15.78 TWh DA net-flow magnitude is a wedge budget, not an
   outturn anchor** (three-zone data report, design-review item 4). Only
   DIRECTION (southward) + BINDING FREQUENCY are validated quantities on
   B4; no net-magnitude claim is made here.
5. **The floor is not proven tight.** It removes only the
   downstream-curtailment degeneracy class; costless-thermal-substitution
   vertices are not excluded (see the decision record), so true physical
   binding could sit below 23.5%. The floor is a bound on the identified
   artifact class, not a certified minimum.

## Self-validation (in-test, before the LP figure is trusted)

The same code path reproduces both known anchors: observed 35.86%
(±0.01, recomputed from the pack trace) and rule-based 1.96% (±0.005,
re-run on the de-duplicated scenario — the dropped stores are inert
under rule-based dispatch, so the committed digests are untouched). LP
feasibility is asserted (total unserved < 1e-6 TWh), and the qualitative
invariant floor > 10× rule-based is pinned.
