# Independent audit — multi-node Scotland/England arc (2026-07-05)

**Commissioned by Richard, 2026-07-05**, on the statement: "I don't trust any of
the work that was done on the multi-node Scotland/England modelling. I don't
think we've complied with the safety protocols and external verification."

**Method.** Six independent audit agents (protocol/git compliance, code
correctness, test validity, docs/comments accuracy, phase objectives, live
build+mutation verification) ran over the full arc — commits `0916f52`
(stage-5 engine, 2026-07-03) through `b6`/`b4`/D12 steps 1–2b to the current
uncommitted working tree — followed by adversarial refutation of every
blocker/major finding (20 verification passes). All claims in
`memory/*.md` and commit messages were treated as allegations and re-derived
from code, git, and execution. 26 agents, ~1.84M tokens, run
`wf_596e31af-ed4`.

## Verdict

**The numbers are real; the verification trail is not — for the D12 LP arc.**

- The committed stage-5 / B6 / B4 work complied with the protocols: review
  artifacts exist, dated and scoped; the A2 re-pin followed a pre-declared
  escape clause; TDD co-commit holds for every library commit in the arc
  (scripted check, zero exceptions); quotable numbers are pinned. The 4→6→5
  stage resequencing was examined and REFUTED as a violation (ledger decision
  record `0ec0529` predates the work; tag-gate held at every transition).
- The D12 LP arc (steps 1–2b committed, step 3 uncommitted) broke the
  external-verification protocol: **no review artifact exists in the repo for
  any D12 code commit, nor for the step-3 review that rejected 31.75%, nor for
  the follow-up audit.** Those reviews demonstrably ran (probe-crate build
  artifacts survive in `target/`), but their reports live only in dead session
  transcripts. Given the b6 gate's measured false certification (below),
  "reviewer ACCEPT" without an artifact is not acceptable evidence.
- The uncommitted 28.20% B4 measurement is **not commit-ready**: two live
  defects (RGB double-count, Q3b degeneracy) mean the number may move again,
  and the mandated decision record + findings note are still missing.

## Independently confirmed (92 claims re-derived; highlights)

- **28.20% reproduces exactly**: LP B4 binding on the de-duplicated scenario
  = 0.281984 (4,860/17,235), re-derived by three separate agents including a
  standalone probe. The rejected 31.75% also reproduces on the
  non-de-duplicated scenario (0.317493) — the +3.55pp NSCO pumped-storage
  double-count mechanism is exactly as alleged.
- Observed anchor 35.86% (0.358631, LP-test convention; 0.357759 committed
  convention) and rule-based 1.96% (0.019553, 337 periods) both reproduce
  from pack arithmetic.
- `cargo test --workspace` = **581 passed / 0 failed / 4 ignored** (the 4 are
  the tractability benches — no acceptance criterion hidden); fmt and
  `clippy -D warnings` clean; all four rule-based digests (779d7444 +
  2/3/5-zone) unmoved by the working tree; determinism verified (repeat runs
  bit-identical; HiGHS single-threaded; no wall-clock/randomness).
- **All three mutations killed**: each deficit-charge guard caught by its own
  dedicated test independently; zeroing the MinCurtailment round-trip-loss
  term caught by the physical-invariant guard via the acceptance test. The
  "RED-if-broken" claims are true.
- The deficit-charge hole the guard fixes was real (contract-relaxed policy
  could conjure SoC in an unbackable deficit); RuleBased is immune, so no
  committed number was ever wrong.
- B4 data pack manifest verifies 4/4; the 3-zone scenario reconciles exactly
  against committed GB totals and NESO boundary values; the §3 (+31%/unit
  onshore) and §6 (~19% offshore wedge) bias claims are documented with
  method.

## Findings that survived adversarial verification

### Must fix before the working tree is committed

1. **[major] RGB pumped-storage double-count — untreated and undocumented.**
   The RGB zone carries the identical defect class that got 31.75% rejected:
   exogenous `pumped_storage_net` (scale 0.7383) AND a dispatchable 2.088 GW
   Dinorwig/Ffestiniog store (`gb-2024-3zone.toml:351-356`, `:424-429`). The
   LP wakes it. The new warning comment and the in-memory de-dup cover NSCO
   only. No documented argument that it cannot move the pinned 28.20%.
   → Resolve (de-dup or prove immaterial) and re-measure before pinning.
2. **[major] Q3b flow degeneracy, now quantified.** The MinCurtailment
   objective has no link-flow term and both links have loss = 0.0, so B4 flow
   is objective-degenerate in slack periods. Probe result: **810 of the 4,860
   binding periods (4.70pp of 28.20pp) occur while downstream zones are
   themselves curtailing** — solver-vertex artifact, not physics. The
   physics-determined floor is **23.5%** (still ~12× rule-based; the
   qualitative finding survives). → Add a flow tiebreak term, or quote the
   band [≈23.5%, ≈28.2%].
3. **[major] `acceptance_b4_lp.rs` silently self-skips when packs are absent**
   (`:136-139` early-returns green) while its own doc claims it fails loudly
   like the siblings (which use `require_packs()` asserts). It is the SOLE
   test on the 28.20% pin and on the entire MinCurtailment objective: on a
   packless checkout the LP loss-term mutation survives the whole 581-test
   suite. → Convert to hard-fail; fix the doc.
4. **[major] Mandated step-3 artifacts still missing**: MinCurtailment
   decision record (zero hits in docs/), B4-LP findings note with the
   DA-magnitude + stacked-bias caveats (28.20 appears nowhere in docs/), and
   the methodology ruling (full-year vs binding-window) recorded only in
   memory/. The design note defines only min-unserved and min-cost
   objectives — MinCurtailment is a third, undocumented objective.
5. **[major] Re-review with a repo artifact.** The step-3 work must pass an
   independent review whose report lands in docs/notes/ before commit/tag.

### Committed-code defects (fix-forward)

6. **[major] Reachable panic in a library crate** (violates hard rule):
   `run_multi_lp_rolling` slices per-zone traces before validating lengths
   (`lp.rs:998`, committed in 925bb7c) — out-of-bounds panic on a misaligned
   non-first-zone trace, reproduced empirically; the same input to
   `run_multi_lp` returns structured `InvalidRunInputs`. No test covers it.
7. **[major] Step 3 is 1-of-3 delivered**: B4 binding attempted
   (uncommitted); the LP storage-magnitude re-measurement, B6 boundary
   decomposition, and tier-2 A2a re-measurement are not started. The
   withdrawn B6 "+33–35%" claim explicitly awaits an LP run that hasn't
   happened.
8. **[major] lp.rs doc overclaims**: MinCurtailment doc says "read boundary
   flows from this objective" with no link-flow term and no mention of the
   known degeneracy anywhere in code or test comments.

### Process record (no code action; governs future gates)

9. **[major] No review artifact for any D12 code commit** (24733eb, 81557b4,
   925bb7c, b5e406a), the step-3 rejection, or the follow-up audit — all
   review outcomes are commit-message/memory assertions;
   project-state:230 admits the report lives "in this session's transcript".
   Breaks the arc's own documented pattern (stage-5, b6, b4 all have review
   files).
10. **[major] The b6 review gate certified a false claim** ("+33–35%
    placement-stable") contradicted by its own package's pinned constants
    (PIN_B6_ES003 33,056 < PIN_COPPER_ES003 33,632), caught only by the later
    beta audit; the withdrawal banner is committed (485f2f3) but the wrong
    claim is permanent in 209d78b's message. "Reviewer ACCEPT" alone is not
    trustworthy in this repo.
11. **[major] Step-1 (24733eb) shipped the physical-tier hole to trunk**;
    found only by the later audit; fix still uncommitted.
12. **[minor] 925bb7c committed mid-review** with the reviewer's B1 hold
    open (protective commit during an agent-collision incident; the
    discriminating SoC-carry test followed 6 minutes later in b5e406a).
13. **[minor] Pinned-number rule was breached** for earlier published numbers
    (2019 nadir 48.787 Hz etc.), self-detected and remediated (732f0ca,
    2191e15) before this audit.
14. **[minor→note, refuted as violation] Stage order 4→6→5**: deviated from
    docs/04's listing under a documented external blocker (ENTSO-E
    credentials), with the sequencing call in the ledger before the work and
    the tag-gate honoured at every transition.

### Also noted

- The observed 35.86% anchor is a day-ahead **scheduled** position (DA flow
  exceeds the posted limit in 32.9% of masked periods, max 6,974 MW vs limit
  3,700 MW) — confirming the "not a convergence target" framing; the correct
  bracket is rule-based 1.96% < LP ≈23.5–28.2% < observed-DA 35.86%.
- The LP-test convention drops 42 sentinel-limit rows the committed
  convention keeps — why both 35.86% and 35.78% circulate as "the" anchor;
  should be stated once, in the findings note.
- The deficit-charge guard itself is sound, correctly per-zone (imports fold
  into `must_take` before the guard), provably a no-op for RuleBased, and
  mutation-verified — separable and commit-ready on its own merits.

## Required sequence to close step 3

1. Resolve the RGB double-count (Richard's modelling call, as with NSCO);
   re-run the measurement.
2. Add the flow tiebreak or adopt the [23.5%, 28.2%] band; re-pin.
3. Make `acceptance_b4_lp.rs` fail loudly without packs.
4. Fix the rolling-LP panic + add the misaligned-trace test (fix-forward on
   committed code).
5. Write the MinCurtailment decision record + B4-LP findings note in docs/.
6. Independent re-review producing a docs/notes artifact; then commit.
7. Going forward: **a review without a repo artifact does not count** as a
   gate. Transcript-only reviews are the root cause of this audit.
