# B4 LP step-3 package — independent review (2026-07-05)

**Reviewer:** independent reviewer (step-3 gate).
**Mandate:** item 6 of the required sequence in
`docs/notes/multizone-independent-audit-2026-07-05.md` — the review gate
for the uncommitted B4 Scotland–England LP measurement package, with a
repo artifact (this file), per the standing rule that a review without a
repo artifact does not count.

**Scope reviewed (the uncommitted working tree):**
`grid-adequacy/src/lp.rs`, `grid-adequacy/src/lib.rs`,
`grid-adequacy/tests/lp_rolling.rs`,
`grid-adequacy/tests/acceptance_b4_lp.rs` (new),
`scenarios/gb-2024-3zone.toml` (comment-only),
`docs/notes/d12-mincurtailment-decision.md` (new),
`docs/notes/b4-lp-findings.md` (new).

**Method.** Every implementer claim was treated as an allegation. All
numbers were re-derived with a standalone probe (my own mask/binding
code, written fresh, compiled as an example in a scratchpad COPY of the
tree — the real tree was never modified). Mutations were run in the copy.
Gates (fmt, clippy `-D warnings`, full workspace suite, the four
rule-based digest regressions, the acceptance test itself) were run by me
on the real tree.

## Verdict: ACCEPT-WITH-CONDITIONS

Two conditions, both cheap; neither requires re-measurement. Details at
the end.

## 1. Independent re-derivation (probe, own code — all claims reproduce)

| Quantity | Claimed | My probe |
|---|---|---|
| Mask (sentinels dropped) | 17,235 | 17,235 |
| Observed DA binding | 6,181/17,235 = 0.358631 | 6,181/17,235 = 0.358631 |
| Rule-based (de-duplicated scenario) | 337/17,235 = 0.019553 | 337/17,235 = 0.019553 |
| LP point, NSCO+RGB de-dup | 4,853/17,235 = 0.281578 | 4,853/17,235 = 0.281578 |
| LP floor (no downstream curtailment) | 4,044/17,235 = 0.234639 | 4,044/17,235 = 0.234639 |
| Excluded binding periods | 809 | 809 (4,853 − 4,044) |
| LP point, NSCO-only de-dup | 0.281984 | 4,860/17,235 = 0.281984 |
| LP feasibility | unserved < 1e-6 | 0.0 GWh (both runs) |
| DA exceedance fact (caveat 1) | 32.9%, max 6,974 MW | 5,673/17,235 = 32.92%, max flow 6.974 GW |

The acceptance test run on the real tree (release) prints exactly the
pinned values and passes in ~13 s. My probe is a separately-compiled
binary with independently written mask/binding logic; counts are
bit-identical across the two binaries — strong determinism evidence on
this machine (ADR-5 machinery unchanged; HiGHS serial).

**De-dup verified structurally:** `gb-2024-3zone.toml` carries
`pumped_hydro` stores only in NSCO (0.74 GW, line 237) and RGB (2.088 GW,
line 429); SSCO has battery only. The test's
`retain(|s| s.kind != StorageKind::PumpedHydro)` runs over EVERY zone,
removes exactly those two, keeps the batteries, and cannot touch the
exogenous `pumped_storage_net` entries (lines 175, 353 — they are
`exogenous_supply` traces, not storage), so the observed 2024 PS actions
remain in the model exactly once. The scenario file itself is unchanged
apart from warning comments (its digest tests still pass — see §5).

## 2. Mutation results (all in the scratchpad copy)

1. **Panic-fix red-green (audit item 4/6):** with the new cross-zone
   validation block deleted from `run_multi_lp_rolling`, the new test
   `rolling_rejects_misaligned_non_first_zone_trace_without_panicking`
   panics with exactly the alleged defect — `panicked at lp.rs:1018:44:
   range end index 6 out of range for slice of length 5` (line 998 in the
   pre-package file). With the fix restored it passes, returning
   structured `GridError::InvalidRunInputs`. RED confirmed, GREEN
   confirmed.
2. **Objective pin is live:** mutating `run_multi_lp_min_curtailment` to
   silently call the MinUnserved oracle is killed hard by the point pin —
   "LP B4 binding point 0.5534 moved from pinned 0.2816". (Incidentally
   confirming the degeneracy story: the indifferent oracle "binds" 55% of
   periods.)
3. **De-dup perturbation (NSCO-only, RGB store left in):** the acceptance
   test PASSES — point 0.281984, floor 0.234987, both inside ±0.01 of the
   pins. **The tolerance cannot distinguish the RGB treatment** (0.04pp
   effect vs a 1pp band). Assessment: this is acceptable and should be
   understood plainly — the pin's job is regression detection on the
   objective/engine/data (which it does, see mutation 2), NOT de-dup
   enforcement. The de-dup is enforced structurally inside the test
   itself (the retain loop over every zone); no code outside the test can
   reintroduce the double-count, and weakening the retain requires
   editing this reviewed test. The ±0.01 is justified for HiGHS
   cross-machine vertex variation at degenerate vertices, which is
   plausibly larger than 0.04pp; tightening it to catch the RGB delta
   would make the pin machine-fragile. No change required.
4. **Packless checkout (audit item 3):** with the data packs removed from
   the copy, the acceptance test FAILS loudly in `require_packs` with the
   build hint (verified empirically). The prior silent self-skip is gone;
   the module doc now truthfully describes the sibling `require_packs`
   pattern.

## 3. Gates (run by me on the real tree)

- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **582 passed / 0 failed / 4 ignored** —
  matches the claim; the 4 ignored are the `tractability_bench.rs` timing
  probes (`#[ignore]` with reasons), no acceptance criterion hidden.
- Four rule-based digest regressions (`regression_2024` incl. the pinned
  `779d7444…`, `regression_2zone`, `regression_3zone`,
  `regression_5zone`): all pass on the working tree — the MinUnserved
  objective construction is untouched under the new enum (verified in the
  diff: the added terms are gated behind `LpObjective::MinCurtailment`).
- No new dependencies (no Cargo.toml/lock change in the package). No
  library-crate panics on the new paths (the new validation returns
  structured errors; unwraps live only in test code under the standard
  test-lint allowances). No wall-clock, no randomness introduced.
- TDD evidence: commit-order evidence is unavailable for uncommitted
  work; red-green was demonstrated empirically by mutation (§2.1, §2.2),
  which is the stronger form for this gate.

## 4. The floor definition and residual degeneracy (question B)

The floor's exclusion — binding periods in which SSCO or RGB curtails
> 1e-6 GW — correctly targets the degeneracy class the audit quantified
(spill shifted across a lossless link with no objective consequence), and
excluding them is conservative in the right direction for a floor (a
period with both genuine over-limit northern surplus AND downstream
curtailment is dropped, understating the floor).

However the class is **not exhaustive**, and the package knows it: with
thermal generation costless in the objective, an NSCO thermal export
(Peterhead) displacing RGB CCGT across a lossless link is
objective-neutral, so B4 can "bind" at a solver vertex in periods with no
curtailment anywhere — such periods are counted IN the floor. The true
physics-forced binding could therefore sit below 23.5%. This is honestly
stated in the decision record ("the floor is a documented lower edge, not
a proof of tightness" and the explicit costless-thermal-substitution
bullet) and in the lp.rs docs ("costless thermal backing"). It is NOT
currently stated in the findings note's own "Mandatory caveats" block,
which is the block that claims to be carried on every quote —
**condition 1**.

## 5. Document check (every number verified)

`b4-lp-findings.md`: the table (1.96% = 337; band floor 0.234639 = 4,044,
point 0.281578 = 4,853; observed 35.86%/35.78%) reproduces exactly. The
809-period exclusion, the 31.75% rejection history (+3.55pp NSCO), and
0.281984 → 0.281578 (−0.04pp RGB) all reproduce. The convention note
(17,235 vs 17,277; 6,181 rows; 35.86% vs 35.78% = 6,181/17,277 =
0.357759) is arithmetically correct. **All four mandatory caveats are
present**: (1) observed-DA non-convergence framing with the 32.9%
exceedance fact — verified 32.92% from the pack; (2) §3 ~+31% and §6 ~19%
upward biases — both present in
`three-zone-scottish-data-report.md` (§3 "~+31%", §6 "~19% wedge") and
carried; (3) HiGHS cross-machine float caveat with the ±0.01 rationale;
(4) 15.78 TWh named a wedge budget, not an anchor — matching the data
report's framing. One wording nit: "(max 6,974 MW vs a 3,700 MW limit)" —
3,700 MW is the series' maximum posted limit; the limit in the actual
max-flow period was 2,800 MW (my probe), so the exceedance is starker
than the parenthetical suggests. Not gating.

`d12-mincurtailment-decision.md`: correctly records the third objective
the design note did not anticipate; the objective's four terms match the
code exactly (`MIN_CURTAILMENT_UNSERVED_WEIGHT = 1e6`; curtailment × 1;
round-trip loss at `1 − η` — code: `1.0 − sqrt_efficiency²`, and
√η² = η, so the doc is right; `CYCLING_PENALTY = 1e-6` unchanged). The
"19 LP tests" count is right (20 now, 19 pre-package). The **full-year
vs rolling methodology divergence is recorded** with the correct
justification (single-year congestion frequency; horizon = measurement
window; rule 3's rolling machinery reserved for multi-year storage
sizing), and the **band ruling is attributed (Richard, 2026-07-05)**.
The de-dup is recorded as Richard's modelling call mirroring the NSCO
Option-A treatment. The "assumptions a critic could attack" section is
frank and accurate.

## 6. Audit-closure table (required sequence, items 1–5)

| # | Audit requirement | Status |
|---|---|---|
| 1 | Resolve RGB double-count; re-measure | **CLOSED** — de-dup extended to every zone in-test; re-measured 0.281984 → 0.281578 (reproduced); recorded as Richard's call; scenario warnings now name RGB explicitly |
| 2 | Flow tiebreak OR the band; re-pin | **CLOSED** — band adopted per ruling; both edges pinned (`PIN_B4_LP_BINDING_POINT`/`_FLOOR`); tiebreak alternative considered and documented as deferred |
| 3 | acceptance_b4_lp fails loudly without packs | **CLOSED** — `require_packs` asserts verified empirically on a packless copy; doc corrected |
| 4 | Rolling-LP panic fix + misaligned-trace test | **CLOSED** — red-green demonstrated by mutation; structured error on the fixed path; fix-forward on committed code as mandated |
| 5 | Decision record + findings note in docs/ | **CLOSED** — both present, every number verified, all four mandatory caveats carried (subject to condition 1's addition) |

(Item 8 of the audit — lp.rs doc overclaims — is also closed by the new
enum/function docs, which carry the degeneracy caveat and the band
instruction. Item 6, this review, is this file; the commit is the
supervisor's.)

## 7. Conditions

1. **Findings note, one sentence:** add to the "Mandatory caveats" block
   of `docs/notes/b4-lp-findings.md` that the floor removes only the
   downstream-curtailment degeneracy class and is not proven tight —
   other indifference classes (costless thermal substitution across the
   lossless links) are not excluded, so true physics-forced binding could
   sit below 23.5% (this is already stated in the decision record; the
   quotable artifact must carry it too, since its caveat block claims to
   travel with every quote).
2. **Commit hygiene:** the untracked `figures/` directory
   (`rs-37y-storage-trace.*`, 22 MB) is NOT part of this package and must
   not be swept into the step-3 commit.

Non-gating observations: the "3,700 MW limit" parenthetical wording
(§5); the lp.rs module-header "Objective" section still describes only
the 2a oracle (the MinCurtailment documentation lives, thoroughly, on the
enum and function — acceptable); the point pin's ±0.01 cannot distinguish
the RGB de-dup, accepted with the structural-enforcement argument (§2.3).
