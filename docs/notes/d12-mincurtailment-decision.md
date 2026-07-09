# D12 — `LpObjective::MinCurtailment` as the congestion-measurement objective (recorded 2026-07-05)

**Decision:** the B4 boundary re-measurement (D12 rule 4, step 3) is read
off a third LP objective, `LpObjective::MinCurtailment`
(`run_multi_lp_min_curtailment`, `grid-adequacy/src/lp.rs`) — a
least-waste economic dispatch at fixed capacity — and its binding
statistics are quoted only as a **band [floor, point]** (Richard's ruling
2026-07-05). The D12 design note (`d12-perfect-foresight-lp.md`) defines
only the min-unserved feasibility oracle and the priced min-cost
objective; this note records the third objective it did not anticipate.

## Why the min-unserved oracle cannot measure congestion

On an adequate fleet the minimised unserved is ~0 over a huge optimal
face: any dispatch that serves all load is optimal, so curtailment and
link flows are **under-determined** — the oracle is indifferent to
whether surplus is wheeled south or spilled north, and HiGHS returns an
arbitrary vertex. Reading boundary flows off it would measure the
solver's pivoting, not the boundary (a known 2b reviewer follow-up). The
oracle stays exactly what rule 2 made it: the bisection's feasibility
test for storage SIZING. Congestion needs an objective that is *not*
indifferent to waste.

## The objective's terms

Minimise, over all zones and periods (energy units, ×dt):

1. **Unserved × 1e6** (`MIN_CURTAILMENT_UNSERVED_WEIGHT`): feasibility
   strongly dominates, so the LP never trades served load for reduced
   curtailment. On an adequate fleet max-delivery serves both goals, so
   unserved is pinned at its feasible minimum (~0).
2. **Curtailment × 1**: the waste being minimised. The LP wheels surplus
   as far as the links physically allow; what remains spilled is what
   the boundary forces.
3. **Storage round-trip loss as waste**: each store's charge is charged
   at rate `1 − η`. Disposing of surplus by fake cycling (charge and
   burn the round-trip loss) then costs *exactly* what curtailing it
   costs, so the LP cannot hide spillage inside a store to game term 2 —
   the earlier failure mode, now also caught by the physical
   no-simultaneous-charge/discharge invariant — while genuinely storing
   surplus against a later deficit still nets a gain.
4. **Cycling tie-break** (`CYCLING_PENALTY`, 1e-6 on throughput): removes
   degenerate simultaneous charge/discharge vertices, unchanged from the
   oracle.

The min-unserved oracle's objective is byte-for-byte unchanged by the
addition (guarded by the 19 LP tests and the pinned digests).

## Known limitation: no link-flow term → degeneracy → the band

The objective carries **no term on link flow**, and both internal links
have `loss = 0.0`. So in periods where the objective is indifferent —
shifting spill (or costless thermal backing) across a link changes
nothing — the flow is a HiGHS solver-vertex artifact: deterministic
(ADR-5, serial simplex), but not model-determined. One class is
quantified in the acceptance test: **binding periods in which a
downstream zone (SSCO or RGB) is itself curtailing**, where the spill
could equally have been left upstream and B4 need not have bound.
Binding statistics are therefore quoted as a band:

- **point** = all binding periods (the regression pin);
- **floor** = binding periods with no downstream curtailment (the
  physics-determined lower edge).

Both are pinned in `grid-adequacy/tests/acceptance_b4_lp.rs`
(`PIN_B4_LP_BINDING_POINT`, `PIN_B4_LP_BINDING_FLOOR`). The alternative
— a tiny flow tie-break term in the objective — was considered and
deferred: it would re-pin every LP surface for a cosmetic gain, and the
band states the uncertainty honestly rather than hiding it inside
another arbitrary weight.

## Methodology: full-year, not the rolling binding-window

The design note's step-3 wording (rule 3) prescribes binding-window
ROLLING slices for the LP re-measurements. The B4 binding-frequency
measurement instead runs a **single full-year (2024) whole-horizon LP**.
That is adequate FOR THIS QUESTION and is hereby recorded as a
divergence: the B4 statistic is a single-year congestion frequency on
the observed 2024 DA limit series — the horizon is the whole
measurement window, so perfect foresight over it is exact, and no
multi-year drought recharge is involved. Rule 3's rolling-window
machinery guards *multi-year storage sizing*, a different question; any
step-3 storage-magnitude re-measurement must still use it.

## Assumptions a critic could attack

- **Central-planner optimum:** no market institutions, no unit
  commitment, no reserve holding — the LP is the optimistic bound on
  physical dispatch (design note rule 5), which is exactly why observed
  day-ahead scheduling can bind more often than it does.
- **The floor's degeneracy class is not exhaustive:** it removes
  downstream-curtailment vertices only; other indifference classes
  (costless thermal substitution patterns) could in principle exist, so
  the floor is a documented lower edge, not a proof of tightness.
- **Curtailment weighted equally everywhere:** a GWh spilled in NSCO
  costs the objective the same as one spilled in RGB; only the
  round-trip-loss term breaks ties between zones.
- **The de-duplication call** (dropping the dispatchable pumped-hydro
  stores from NSCO and RGB because the exogenous `pumped_storage_net`
  traces already carry the observed PS actions) treats 2024's PS
  operation as history, denying the LP counterfactual PS flexibility —
  conservative for the binding frequency, but a modelling choice
  (Richard's, mirroring the NSCO Option-A treatment).
- **Input biases:** the three-zone data report's §3 onshore split
  (~+31%/unit north of B4) and §6 offshore-commissioning wedge (~19%)
  both push modelled B4 binding UP — carried on every quote (see
  `b4-lp-findings.md`).
