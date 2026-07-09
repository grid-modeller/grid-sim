# D12 review — perfect-foresight LP dispatch + the policy-contract relaxation

**Reviewer adjudication, 2026-07-04.** Gating `docs/notes/d12-perfect-foresight-lp.md`
(supervisor draft) before any LP engine work (the D8/D9/D11 precedent). This is a
DESIGN adjudication: no LP code exists yet, so there are no acceptance tests to
run. The baseline I verified against is the committed engine
(policy.rs/flow.rs/solve.rs/dispatch.rs/multizone.rs) and the adjudicated
findings the note builds on.

## Verdict: ADOPT-WITH-EDITS

The spine is sound and I can certify three things the task asked me to rule on:
the rule-1 partition matches what the engine enforces today line-for-line; the
RuleBased digest CAN stay bit-identical through the refactor; and an LP with free
flow variables WILL wheel and genuinely resolve the three-zone finding. But four
load-bearing gaps must close before the work order issues — the LP objective vs
the bisection question (edit 1), the perfect-foresight-survival-under-a-finite-
window commitment (edit 2), the three-zone finding's mischaracterisation and the
compare-not-tune discipline (edit 3), and the missing red test for package 1
(edit 5). None is fatal; all are specifiable now.

---

## Ruling on item 1 — the policy-contract split and digest bit-identity

**The partition is CORRECT against the code.** I verified the engine's per-period
validation against the note's two tiers:

- **Genuine physical laws, enforced today for any policy** (correctly in the note's
  physical tier): non-negativity and no-simultaneous-charge/discharge
  (`dispatch.rs:328-333`, `multizone.rs:908-913`); per-store charge ≤ `max_charge`
  (SoC headroom + power rating) and discharge ≤ `max_discharge` (SoC + power)
  (`dispatch.rs:334-349`, `multizone.rs:914-929`); the SoC-bound snap in
  `StoreState::apply` (`policy.rs:155-190`).
- **Rule-based CHOICES the engine currently enforces on ALL policies** (correctly
  named in the note's policy tier, and the reason the relaxation is needed): total
  charge ≤ surplus — surplus-only charging, D4 rule 2 (`dispatch.rs:353-362`,
  `multizone.rs:933-943`); zero discharge during surplus (`dispatch.rs:372-380`,
  `multizone.rs:945-954`); total discharge ≤ post-stack deficit — discharge-after-
  stack, D4 rule 3 (`dispatch.rs:402-411`, `multizone.rs:981-991`). A foresight
  policy that pre-charges from the gas stack (`total_charge > surplus`) is rejected
  by `dispatch.rs:353` TODAY — so the note's premise is accurate, not asserted.

The single-pass equal-depth flow (`flow.rs` prose rules 2-6, `equalising_flow`) is
also correctly a policy choice, not a law.

**One genuine mis-classification (edit 4):** the note lists "per-leg √η efficiency"
under physical invariants (draft line 42) and bakes the symmetric split into the LP
dynamics (rule 2). The symmetric √η split is a **modelling CONVENTION**, not a
physical law — D4 mechanics owns it explicitly (the ~15-20 % store-side shift vs the
Royal Society's asymmetric per-leg convention). The invariant is round-trip η plus
`0 ≤ SoC ≤ capacity`; the symmetric split is a convention both policies share for
comparability. **One omission (edit 10):** the fixed merit-order thermal dispatch
(`dispatch.rs` docs, "not SRMC") is itself a rule-based choice the LP replaces via
its objective; the note's enumeration of choices omits it (moot for pure-RS
adequacy, load-bearing for mixed-fleet cost runs).

**Digest 779d7444 CAN stay bit-identical — with a stated precondition.** The digest
is produced by the value-generating arithmetic (RuleBased::dispatch, the thermal
stack loop, the curtailment/unserved accounting). The checks the refactor relocates
are a NO-OP on any valid RuleBased run — they only ever produce errors, which never
fire on the reference scenario. So bit-identity holds **iff the refactor is
validation-relocation-only** and leaves every value-producing line in
`RuleBased::dispatch`, the `dispatch.rs`/`multizone.rs` stack dispatch, and the
accounting untouched. `acceptance_stage5_2024.rs:564` already proves `run_multi` on
the reference equals `run` field-for-field, so the single pinned digest covers both
paths. The note must state the guarantee rests on freezing the arithmetic and moving
only check *location* — otherwise an implementer "tidying" the accounting silently
moves the pin. That is edit 6 (naming the seam) plus a one-line freeze statement.

## Ruling on item 2 — does the LP measure the same thing the bisection measures?

**As written, NO — and this is the most substantive gap.** The bisection
(`solve.rs::min_storage_for_zero_unserved`) is a **sizing** problem: it varies one
store's energy capacity and finds the smallest capacity at which a FIXED dispatch
gives zero unserved. The note's rule-2 objective ("minimise unserved then
curtailment") at a FIXED capacity is a **dispatch** problem: it finds the minimum
achievable unserved GIVEN the storage. These are different questions, and
"LP storage requirement ≤ rule-based" is only well-defined under the sizing framing.

The fix is clean and must be committed in the note (edit 1): the LP is the
**feasibility oracle inside the same bisection** — for each candidate capacity C,
solve the LP for min-unserved, test `unserved = 0`, bisect on C. Then "LP storage
requirement" = smallest C at which the LP reaches zero unserved, directly comparable
to RuleBased. Under this framing the sanity invariant is not just true but provable:
at every C the LP achieves `unserved_LP(C) ≤ unserved_rule(C)` (RuleBased is a
feasible dispatch the LP may replicate), so the LP hits zero at a capacity no larger
than RuleBased. The lexicographic "then curtailment" secondary term is cosmetic for
the sizing question (it only makes the reported dispatch unique) — worth saying so.

The two objectives the note offers (min-cost vs min-unserved-then-curtailment) are
NOT interchangeable and serve different questions: min-cost (with unserved at a
penalty) is the Q10/Q9 priced/cost run; lexicographic-adequacy wrapped in bisection
is the storage-requirement run. The note gestures at this but must nail which
objective backs which claim.

## Ruling on item 4 — does the LP actually wheel, and is the invariant always true?

**Wheeling: YES.** In an LP every link flow is a simultaneous free variable bounded
by capability, with per-zone per-period conservation. If N has surplus and E/W have
deficit reachable only via S, the LP sets N→S and S→E/W in the SAME period (S nets
to a pure transit) whenever that cuts unserved. There is no single-pass ordering
limitation — the equal-depth cascade's under-wheeling (confirmed below) is exactly
what disappears. Multi-hop losses compound and the LP prices them correctly; if the
alternative is penalised unserved, wheeling wins. So the LP genuinely converts the
three-zone finding from direction-only to a quotable B4 binding. **But** the note
must not pre-commit to reproducing the OBSERVED 35.8 % — see edit 3.

**The sanity invariant `LP ≤ rule-based` is robust, non-strict, with two stated
preconditions.** It holds per-designated-store under the bisection parameterisation
(the LP may replicate RuleBased's use of the other fixed stores and then do at least
as well on the designated one). It is non-strict — on pure-RS no-stack adequacy
D4's greedy discharge is already feasibility-optimal, so LP = rule-based there
(equality, not a bug). The two preconditions the note must record: identical fleet/
traces, and the **same √η convention** (edit 4) — a different efficiency split
breaks the comparison.

---

## Numbered edits (ordered)

1. **[Item 2 — load-bearing] Commit the LP to the sizing framing for the storage-
   requirement comparison.** Rule 2 must state: for the storage-requirement question
   the LP is the feasibility oracle inside the existing bisection (replacing `run` in
   `solve.rs`'s `evaluate` closure), or equivalently is reformulated to minimise the
   designated store's energy subject to `unserved = 0`. State that
   min-unserved-then-curtailment at fixed capacity is a DISPATCH objective and does
   not by itself yield a storage requirement, and that the min-cost objective is a
   separate run for the Q9/Q10 cost questions. Without this the rule-4 sanity
   invariant is undefined.

2. **[Item 3 — load-bearing] The note must commit on horizon, not defer wholesale.**
   Add to rule 3: for the seasonal storage-requirement question the perfect-foresight
   claim REQUIRES a window at least as long as the longest drawdown-to-refill cycle
   (multi-year for hydrogen — the 1985 design drought, SoC carrying across years is
   the whole point). A short rolling window degrades to myopic behaviour at the
   seams and does NOT bound the perfect-foresight ideal for this question — so it is
   inadmissible there, not merely "a documented approximation." Representative-period
   / time-slice aggregation is likewise inadmissible (it breaks the multi-year
   chronological drought, contradicting D4 "no annual reset") and must be ruled out
   explicitly. A finite window is admissible only with reported window-sensitivity
   showing the requirement has converged (window→∞ reached in practice). Full-horizon
   remains the target; the implementer pins tractability WITHIN these constraints,
   not around them.

3. **[Item 4 — consistency with the adjudicated three-zone review] Correct the
   finding's characterisation and impose compare-not-tune.** The draft says the rule
   "cannot route" / "cannot wheel" (lines 20-21, 96). The adjudicated review
   (`docs/notes/three-zone-engine-review.md`, Job #1) rules the opposite: the flow
   rule DOES partially wheel and **UNDER-wheels** (equal-depth, single pass). Replace
   the draft's line 20-21 clause with, verbatim:
   > the single-pass equal-depth flow rule **under-wheels** — it wheels partially
   > (B6 reads N's post-B4 contribution to S) but its equal-depth pairwise
   > equalisation cannot make the second sweep back to B4, so northern surplus is
   > left stranded (6.90 TWh in N-Scotland; model B4 binding 1.95 % vs observed
   > 35.8 %).

   And in rule 4, replace "reproduce the observed ~35.8 %?" with:
   > The LP's B4 binding is the honest model output under free-flow wheeling; it is
   > COMPARED against the observed 35.8 %, never tuned to it. The three-zone review
   > warns that matching 35.8 % by construction is "tuning-to-the-B4-DA-series"; any
   > residual LP-to-observed gap is itself a reported finding (LP relaxation, no unit
   > commitment, within-zone copper plate), not a miss.

4. **[Item 1 — mis-classification] Reclassify the √η split as a shared convention.**
   Strike "per-leg √η efficiency" from the physical-invariant list (draft line 42);
   the invariant is round-trip η plus `0 ≤ SoC ≤ capacity`. State in rule 2 that the
   symmetric split `η_charge = η_discharge = √η` is a modelling convention (D4
   mechanics; ~15-20 % store-side shift owned there) that the LP adopts DELIBERATELY,
   identical to RuleBased, so the LP-vs-rule comparison is clean. Record it as a
   precondition of the rule-4 invariant.

5. **[Item 7 — TDD, load-bearing for the next gate] Specify package 1's red test.**
   Package 1 (the contract refactor) leaves RuleBased bit-identical, so it has NO red
   test as described — a pure refactor cannot go red-first. The note must specify the
   red-first test that proves the RELAXATION: a minimal non-rule-based test policy
   that legitimately violates a rule-based CHOICE (e.g. pre-charges from the thermal
   stack, so `total_charge > surplus`) while respecting the physical tier, asserted
   to be ACCEPTED by the engine where today it errors at `dispatch.rs:353` /
   `multizone.rs:933`. That failing-then-passing test IS package 1's red-green
   evidence; without it the next gate has no TDD basis and I will reject it.

6. **[Item 6/1 — the seam] Name the refactor seam precisely; it is NOT the
   DispatchPolicy trait.** State that the `DispatchPolicy` trait stays per-period,
   storage-only, with the no-future invariant (`policy.rs:236-259` already says the
   LP "receives foresight elsewhere; the shared trait shape does not provide it") —
   the LP cannot and must not be shoehorned into it. The "policy contract" of rule 1
   is a NEW shared physical-tier validator extracted from the inline checks in
   `dispatch.rs`/`multizone.rs`; the rule-based-specific checks (surplus-only,
   discharge-after-stack) stay on the rule-based path only. Add the one-line freeze:
   the RuleBased value-producing arithmetic is untouched; only check LOCATION moves
   (this is what protects digest 779d7444).

7. **[Item 1 — LP relaxation integrity] No-simultaneity must not become an LP
   constraint.** "No simultaneous charge and discharge" is enforced structurally in
   RuleBased (`dispatch.rs:331`) but a no-simultaneity binary would make the LP a
   MILP, breaking rule 5's LP-relaxation commitment. State that in the LP it emerges
   from optimality (`√η·√η < 1` makes simultaneous charge+discharge strictly
   wasteful, never optimal) and is therefore NOT added as an explicit constraint.

8. **[Item 6 — output conventions] "flow.rs untouched" is right but understates the
   shared surface.** `equalising_flow` is genuinely not called by the LP — clean.
   But the LP must reproduce `multizone.rs`'s RESULT-ASSEMBLY conventions (the loss
   split, both-ends `home_end`/`away_end` recording, the NESO metering sign, the
   link-as-exogenous-series folding) so LP and rule-based outputs are
   diff-comparable and feed the same validation gates. Add a sentence: the LP shares
   the multizone result-assembly conventions (not the flow rule), and a test must
   pin that an LP run and a rule-based run of the same scenario emit structurally
   identical `MultiZoneRunResult` shapes.

9. **[Item 5 — the quotable framing] Soften "both bracket the truth" to a
   proposition.** The bracket is a modelling proposition, not a theorem: reality is
   no better than perfect foresight (true), but "no worse than zero-foresight greedy"
   is an assumption, not a guarantee — real operations carry reserve margins and
   forced outages that could push real requirements ABOVE the greedy rule-based.
   Replace the rule-5 clause with, verbatim:
   > perfect foresight is the OTHER bound to the rule-based myopia. Under the stated
   > assumption that real dispatch is no better than perfect foresight and no worse
   > than zero-foresight greedy, reality sits between the two — the ADR-6 framing.
   > The assumption is a modelling proposition (D4's "honest upper envelope"
   > argument), owned as such, not a proven bracket.

10. **[Item 1 — completeness] Add the fixed merit order to the enumerated
    rule-based choices** (rule 1 policy tier): the fixed thermal merit order
    (`dispatch.rs` docs, "not SRMC") is a rule-based choice the LP replaces via its
    objective. Moot for pure-RS adequacy; load-bearing for mixed-fleet cost runs.

## Consistency checks that PASSED (no edit)

- **D11 three-policy ladder (ADR-6).** D12's {RuleBased, PricedLadder, LP} is
  consistent with D11 rule 3 ({scarcity, priced-ladder, LP}); D12 correctly inherits
  D11's ruling that the LP — not the priced ladder — is the B6 headroom-blindness
  resolver, and does not re-claim the priced ladder's A2a territory. No contradiction.
- **ADR-10.** good_lp + HiGHS is the pinned Stage 7 optimiser; D12 confirms without
  amendment. Deterministic-mode requirement correctly carried.
- **Stage 7 work order (docs/04).** The D4-relaxation is a pre-approved tracked item
  ("D4 relaxation... digest 779d7444… unmoved"); the LP-≤-rule invariant and the
  hand-checkable-optimum test are already the pinned Stage 7 acceptance tests. D12 is
  in scope, not scope creep.
- **Scope cuts (item 5).** LP relaxation (no UC/min-stable-gen), central-planner-not-
  market, no negative prices (Q13) — all sound and consistent with the standing
  UNCONSTRAINED caveat and the fixed-merit-order honesty gate.

## Bottom line

ADOPT-WITH-EDITS. Apply edits 1-3 and 5 before the work order issues (they change
what gets built and how it is tested); edits 4, 6-10 are precision/consistency fixes
that prevent a later gate rejection. With those applied the note is a sound basis for
the three-package sequence.
