# D12 — Perfect-foresight LP dispatch + the policy-contract relaxation

**Status:** ADOPTED 2026-07-04 — supervisor draft, reviewer
ADOPT-WITH-EDITS (docs/notes/d12-perfect-foresight-lp-review.md; the
contract split verified line-by-line against the engine). Four gating
edits applied below: √η is a policy convention not a physical law
(rule 1); the LP is the bisection FEASIBILITY ORACLE, not a standalone
dispatch, so it measures the same quantity (rule 2); the finite window
must span the binding multi-year recharge — short windows / typical-day
aggregation ruled out (rule 3); the flow rule UNDER-wheels not "cannot
wheel", and the LP's B4 binding is COMPARED to observed, never tuned to
it (rule 4); package 1's red test = a pre-charging policy accepted where
`dispatch.rs:353` errors today (implementation). Remaining minor edits:
the review file. This is the Stage-7 optimiser design and the
D4-relaxation the tracked items flagged. D10 = EV overlay (reserved),
D11 = priced myopic ladder; D12 is the full-foresight top rung.

## Why now: one limitation, three findings

Three separately-discovered results this week all trace to the SAME
cause — the rule-based dispatch's myopia:
- **The B6 storage magnitude** (beta audit refuted the "+33–35%
  placement-stable" claim): the equal-depth flow clears before storage,
  blind to headroom, and even inverts at some placements.
- **The tier-2 import A2a residual** (D11): the myopic flow can't price
  the non-gas merit order across a border in one pass.
- **The three-zone B4 wheeling** (just measured): the single-pass
  equal-depth rule cannot route northern surplus through S-Scotland to
  England — dispositive proof, unbounded links still strand 6.90 TWh in
  N-Scotland, so it is the RULE, not the boundary (model B4 binding
  1.95% vs observed 35.8%).

All three are quotable today only as DIRECTION + lower-bound totals.
The **perfect-foresight LP resolves all three at once** — it wheels
multi-hop, charges ahead of droughts, and prices on the true merit
order. That is why the LP is the single highest-leverage remaining
engine build: it converts three "direction only" caveats into
quotable magnitudes.

## Rule 1 — The policy-contract relaxation (the D4 tracked item)

Today the engine enforces the D4 rule-based invariants (surplus-only
charging, discharge-after-stack, the equal-depth single-pass flow) on
ALL dispatch — so a foresight policy cannot dispatch differently. Split
the engine's checks into two tiers:

- **Physical invariants (engine-enforced, EVERY policy):** energy
  conservation per period; per-technology capacity limits; charge ≤
  `max_charge` / discharge ≤ `max_discharge`; link capability limits
  (per-direction, per-period per schema v6); storage SoC bounds
  (`policy.rs:155-190`); non-negativity and no simultaneous
  charge/discharge (`dispatch.rs:328-333`, `multizone.rs:908-913`); no
  unserved masked. These are laws; no policy may violate them, and they
  are the acceptance-test spine.
- **Rule-based POLICY CHOICES (move to a per-policy contract):**
  surplus-only charging (`total_charge ≤ surplus`, `dispatch.rs:353`,
  `multizone.rs:933`), zero-discharge-in-surplus (`dispatch.rs:372`),
  discharge ≤ post-stack deficit (`dispatch.rs:402`, `multizone.rs:981`),
  single-pass equal-depth flow, no wheeling, no foresight — AND the
  symmetric √η per-leg split (a D4-owned CONVENTION, the ~15–20%
  round-trip shift, NOT a physical invariant; the review corrected my
  first draft, which mis-classified it as a law). These are the
  RuleBased policy's *choices*; the LP must be free to make different
  ones (charge into a forecast deficit — rejected today at
  `dispatch.rs:353` — wheel N→S→E+W, hold the store against a future
  drought).

Each policy declares a **contract**: the invariants it guarantees
(always the physical tier) plus its policy-specific behaviour. The
engine validates the physical tier for all; the policy tier is the
policy's own business. RuleBased's contract is exactly today's
behaviour — so **the RuleBased dispatch digest `779d7444…` must stay
BIT-IDENTICAL**, which holds ONLY if the refactor is
**validation-relocation-only** and freezes the value-producing
arithmetic byte-for-byte (the review's ruling-1 condition — the
relocated checks are a no-op on any valid RuleBased run, but the
√η arithmetic and every dispatch value must not move).

## Rule 2 — The LP formulation

A perfect-foresight linear program over the full horizon (or a
rolling window if 40-year × multi-zone is intractable — stated below):
- **Decision variables:** per-period per-zone generation dispatch,
  per-period per-link directional flow (enabling wheeling — the flow is
  a free variable subject only to capability, NOT the equal-depth
  rule), per-period per-store charge/discharge.
- **Objective (review ruling-2 correction — this is the load-bearing
  fix):** the bisection is a SIZING problem (vary capacity, dispatch
  fixed); a standalone "minimise unserved then curtailment" LP is a
  DISPATCH problem at fixed capacity — a DIFFERENT question, and
  `LP ≤ rule-based` would be ill-defined. So the LP is used as the
  **feasibility ORACLE INSIDE the existing bisection**: at each trial
  store size the LP answers "does a feasible zero-unserved dispatch
  exist?" (minimise unserved; feasible ⇔ min unserved = 0), and the
  bisection sizes the store exactly as today but with the LP as the
  inner feasibility test instead of the rule-based dispatch. Under this
  framing the LP measures the SAME quantity the rule-based bisection
  does (minimum store for zero unserved), so `LP requirement ≤
  rule-based requirement` is provable, not asserted. For priced/cost
  scenarios (thermal fleets) the objective is min total system cost
  (D8) at fixed capacity — a distinct, clearly-labelled use.
- **Constraints:** the physical-invariant tier of rule 1, as linear
  constraints. Storage dynamics (SoC(t+1) = SoC(t) + √η·charge −
  discharge/√η) are linear. Link flows respect the v6 directional/
  per-period capability. This is a linear program — no unit commitment,
  no min-stable-generation binaries (that stays a documented
  limitation, consistent with the UNCONSTRAINED caveat).
- **Solver:** `good_lp` + HiGHS (ADR-10), as pinned.

## Rule 3 — Horizon and tractability

A 40-year half-hourly multi-zone LP is ~700k periods × zones × links —
potentially large. Design decision, to be pinned by the implementer
with evidence: (a) full-horizon LP if HiGHS handles it in acceptable
time; (b) else a rolling-horizon LP with a window LONG ENOUGH to span
the binding multi-year recharge (the RS drought episodes run to
720+ days — review ruling-3: a window that cannot see a full
drawdown-recharge cycle is NOT perfect foresight and would understate
the requirement, so SHORT windows and representative-period /
typical-day aggregation are RULED OUT; the window must be ≥ the
longest below-full episode, with overlap, and the window-sensitivity
reported toward the window→∞ ideal). Either way the LP result is
DETERMINISTIC (same scenario+data → same solution; HiGHS deterministic
mode).

## Rule 4 — What it resolves, and how each finding is re-measured

Under the LP, re-run and re-report (each becomes a quotable magnitude,
replacing today's direction-only lower bound):
- **Three-zone B4 wheeling:** the rule-based flow **under-wheels**
  (partial — single-pass equal-depth, review correction: NOT "cannot
  wheel"); the LP's free simultaneous flow variables let it wheel
  N→S→E+W in one period, so the single-pass limit disappears. Its B4
  binding + storage requirement become the quotable numbers — but they
  are COMPARED to the observed ~35.8%, never TUNED to match it
  (review ruling-4: matching the anchor by construction is tuning; the
  LP's binding is whatever the optimum gives, and the gap to observed
  is itself a finding).
- **B6 magnitude:** the LP separates the boundary effect from the
  dispatch convention — the clean "+X% from the boundary" the rule-based
  model could not give (the beta-audit correction named the LP as the
  resolver).
- **Tier-2 A2a:** the LP on the priced multi-zone system — the A2a
  direction match under optimal (not myopic) flow.
The **LP-vs-RuleBased gap is a REPORTED FINDING** (ADR-6 dual-policy
discipline), not tuned away. Sanity invariant (docs/04 Stage 7): LP
storage requirement ≤ RuleBased on every scenario — provable under the
rule-2 bisection-oracle framing, but NON-STRICT (equality where the
rule-based dispatch is already optimal) and CONDITIONAL on identical
fleet and the same √η convention on both sides (review ruling-4).

## Rule 5 — What this does NOT do

Not a unit-commitment / min-stable-generation model (LP relaxation;
the UNCONSTRAINED caveat stands). Not a market-institution model (no
bidding/day-ahead-intraday split; the LP is a central-planner optimum,
the theoretical floor cost, explicitly labelled as such — real markets
sit above it, and that gap is itself a finding). Not a stochastic /
imperfect-foresight model (perfect foresight is the OTHER bound to the
rule-based myopia; reality sits between — both bracket the truth, the
ADR-6 framing). Not negative prices (Q13 territory).

## ADR touch-points (proposed, recorded per CLAUDE.md)

- **ADR-6:** the policy set is {RuleBased, PricedLadder (D11),
  PerfectForesightLP}; the LP-vs-rule gap and the LP-vs-priced gap are
  reported findings. The policy-contract (rule 1) is the mechanism.
- **ADR-10:** confirmed — good_lp + HiGHS, the only Stage-7 optimiser.
- **flow.rs:** the LP does NOT use the flow rule — it treats flow as a
  free LP variable. flow.rs stays the RuleBased policy's flow; the LP is
  a parallel dispatch path. (So flow.rs is untouched; the LP is new
  code behind the policy contract.)
- **docs/04 Stage 7:** the LP acceptance tests (hand-checkable optimum;
  LP ≤ rule-based; the three re-measurements) are the pinning targets.

## Implementation sequence (post-adjudication)

1. **The policy-contract refactor** (rule 1): separate the physical
   from the rule-based checks, RuleBased digest bit-identical
   (validation-relocation-only). Own package, own review — the D4
   relaxation, must land green before the LP. RED-FIRST TEST (review
   ruling-5 — a pure refactor cannot go red-first, so the note must
   supply the failing case): define a minimal non-rule-based test
   policy that PRE-CHARGES the store from the stack in a surplus-free
   period — this is REJECTED today at `dispatch.rs:353` (surplus-only
   charging); the refactor's success condition is that this policy is
   ACCEPTED (the physical tier passes) while RuleBased is unchanged.
   That test is red before the relaxation, green after.
2. **The LP dispatch** (rules 2–3): good_lp/HiGHS, the hand-checkable
   acceptance test first (red), then the small-scenario optimum, then
   scale to the horizon with the tractability decision pinned.
3. **The three re-measurements** (rule 4): three-zone, B6, tier-2 under
   the LP, each a run-report update converting direction→magnitude.
Estimated three packages + reviews — the largest remaining engine
build, and the one that unlocks the most quotable results.
