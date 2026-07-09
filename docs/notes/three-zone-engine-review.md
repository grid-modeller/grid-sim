# Three-zone Scottish-boundary SCENARIO + ENGINE package — adversarial review

Reviewer, 2026-07-04. Gating the uncommitted three-zone (N-Scotland /
S-Scotland / E+W, B4 + B6) scenario+test package before commit. The
package produced a MAJOR finding (the rule-based flow rule does NOT
validate the B4 gate: model binding 1.95% vs observed 35.8%; three-zone
B6 exit 10.26 TWh vs two-zone 15.79 / outturn 17). Job #1 was to decide
whether that finding is a GENUINE flow-rule property or an implementer
artefact. Everything below was run, not trusted.

## VERDICT: ACCEPT-WITH-NOTES

The finding is GENUINE and correctly framed. Engine source is byte-
untouched, checksums verify, fmt/clippy clean, every acceptance test ran
and passed under my hand. Three NOTES (all diagnostic-honesty / process,
none a correctness defect) must be cleared before commit/tag.

---

## Job #1 — the wheeling finding: GENUINE flow-rule limitation, NOT an artefact

**Ruling: genuine, structural property of `flow.rs`; cannot be fixed
within the rule-based model. The LP (Stage 7) is the only resolver. The
package's framing is correct and at the right prominence (the B6
precedent).**

Mechanism, verified against `grid-adequacy/src/flow.rs` and
`multizone.rs`:

- A surplus zone's scarcity signal is `−surplus` GW (`flow.rs` rule 1;
  `signal(r)` returns `r` for `r ≤ 0`). Two surplus zones equalise when
  their residuals MEET, i.e. at equal surplus DEPTH — not by draining the
  deeper zone (`equalising_flow`; unit test `two_surplus_zones_split_the_
  difference`, signals −6/−1 → −3.5). This is the documented negative-
  price analogue, a stated v1 modelling choice.
- Borders dispatch in a SINGLE sequential pass (`flow.rs` rule 6;
  `multizone.rs` step 2 loop over `borders`). B4 (N→S) equalises N/S
  depth, then B6 (S→E+W) reads the post-B4 S residual and equalises S/E+W
  depth. There is partial wheeling (B6 sees N's contribution to S), but
  no second sweep back to B4, so the cascade UNDER-wheels: each hop only
  halves the differential, stranding surplus upstream in N.

**Dispositive proof it is the RULE, not the boundary:** at the reference
fleet, the COPPER-PLATE run (both links set to 1000 GW, unbounded) still
strands **6.9046 TWh** of curtailment in N-Scotland
(`PIN_REF_NSCO_COPPER`, reproduced by me). Adding the finite 1.8 GW B4
gate raises N curtailment by only +0.042 TWh (to 6.9468). With infinite
link capacity the surplus is still stranded, so the cause is the equal-
depth single-pass flow rule, not link capability. The implementer did
NOT touch `flow.rs` (git-confirmed) — the behaviour falls straight out of
the committed rule. This is exactly what `scottish-group-boundary-design-
review.md` item 5 PREDICTED. Confirmed genuine.

**Phrasing nuance (NOTE 3):** the package says the rule "refuses to
wheel ... in a single pass." Strictly it DOES partially wheel (B6 reads
the post-B4 S position); it UNDER-wheels because equal-depth pairwise
equalisation in one sweep strands the residual. Substance is right;
recommend "under-wheels (equal-depth, single pass)" wherever the phrase
is quoted, so the paper does not overstate it as a total refusal.

**Framing check:** the package quotes DIRECTION (southward, validated) +
PINNED TOTALS under stated conventions only; forbids any "B4 effect
proper" % or B4-vs-B6 decomposition; carries the lower-bound duty (B5
folded into S-Scotland); names the Stage-7 LP as resolver; and pins the
1.95% / 10.26 TWh / binding-count diagnostics as the FINDING at full
prominence rather than burying them. This is the B6-audit precedent
applied verbatim. Correct.

## Border-order ruling — B4-FIRST is CORRECT

Ratified. Grounds:

1. It follows the physical cascade N→B4→S→B6→E+W (the upstream gate
   clears first and presents at the downstream exit).
2. It is NOT chosen to match the observed anchor. B6-first lifts model
   B4 binding to ~9% (closer to the observed 35.8%) but strands ~3 TWh in
   S instead. Selecting the ordering that pushes B4 binding toward the
   observed 35.8% would be precisely the tuning-to-the-B4-DA-series that
   obligation 1 forbids (B4 has no outturn cross-anchor). B4-first is the
   non-tuned choice.
3. The choice is immaterial to the headline: neither ordering can wheel
   under the single-pass equal-depth rule, so both are lower bounds and
   the package correctly refuses a clean decomposition either way.
   B4-first at least strands the surplus in N, where the physical
   throttle actually sits (northern wind behind the B4 wall), so it is
   the more faithful of two convention-bound options.

## Item 1 — direction-monotone numbers (reproduced) and the 3z-copper > 2z-B6 coherence

Reproduced exactly from `acceptance_b4_robustness.rs` (release, 40-year):

| baseline | GWh |
|---|---|
| single-zone | 23,872 |
| 2-zone copper | 26,480 |
| 3-zone copper | **35,968** |
| 2-zone B6 | 35,648 |
| 3-zone B6-only | 36,416 |
| 3-zone B4+B6 (headline) | **37,824** (+58.4% vs single, +6.1% vs 2z-B6) |

**Coherence of 3z-copper (35,968) > 2z-B6 (35,648): COHERENT, correctly
attributed.** 3z-copper is not a physical copper plate — it is three
zones on infinite links dispatched by the equal-depth single-pass rule,
which strands surplus (as proved above). Splitting SCO into N+S adds a
second stranding hub, so the zone-count/dispatch-convention effect (already
+50.7% over single-zone at copper) dominates the boundary-capacity effect
(the finite B4+B6 add only a further +5.2%). It is the SAME equal-depth
artefact, and `acceptance_b4_robustness.rs` (doc-comment point 2) states
this explicitly. Coherent and honestly labelled.

**NOTE 1 (diagnostic honesty — must fix before commit).** The
`eprintln!` in `rs_requirement_under_the_three_zone_split...` strings all
six numbers with `<` separators:

> single 23,872 < 2z-copper 26,480 < 3z-copper 35,968 < 2z-B6 35,648 < ...

The junction `3z-copper 35,968 < 2z-B6 35,648` is arithmetically FALSE
(35,968 > 35,648). The 2-zone and 3-zone ladders INTERLEAVE; they are not
one monotone sequence. The test ASSERTIONS are correct (they assert only
the within-3-zone chain `2z-copper < 3z-copper ≤ 3z-B6-only ≤ 3z-full`
and `full > 2z-B6` — all true), so the test rightly passes. But the
printed narrative presents a false monotone chain that, if lifted into
the paper as-is, is an error. Fix the narrative to state the interleaving
(like-for-like: copper 26,480 < 35,968; B6-bounded 35,648 < 37,824; and
3z-copper already exceeds 2z-B6) rather than a single spliced `<` chain.

## Other verifications (all reproduced / confirmed)

2. **Scenario / GB conservation.** NSCO+SSCO fleet sums EXACTLY to the
   committed 2-zone SCO (CCGT 1.18, nuclear 1.19, hydro 1.68739, onshore
   10.07568, offshore 3.074505, solar 0.5). E+W (RGB) fleet is BYTE-
   IDENTICAL to `gb-2024-2zone.toml` RGB (ccgt 28.82 … solar 18.2). Moyle
   (intirl) → S-Scotland; other nine external columns → E+W; PS split
   0.2617/0/0.7383; demand 0.03333/0.06767/0.899. Links B4 1.8/4.0, B6
   4.1/3.5. All as specified. Confirmed.

3. **Q2/Q10 three-zone curtailment.** 60 GW Scottish total **31.666 TWh**
   (> two-zone 27.139) reproduced; reference-fleet Scottish 7.008 TWh
   (> two-zone 1.684). Direction-only quote duty carried. Confirmed.

4. **Cruachan both-ways.** N 7.0078 vs S 7.0024 TWh (Δ 5.34 GWh, −0.08%),
   immaterial. Reproduced. Confirmed.

5. **Pins unmoved / engine untouched.** `git diff` shows the ONLY tracked
   source change is `docs/03-domain-model.md`; `flow.rs`, `multizone.rs`,
   `inputs.rs`, `scenario.rs`, CLI src — all byte-untouched. Checksums:
   b4.sha256 4/4 OK, cf-gb3-1985-2024 481/481 OK, b6.sha256 21/21 OK,
   cf-gb2 481/481 OK (0 FAILED anywhere). Precedent regressions pass:
   regression_2024 (779d7444 reference), regression_2zone, regression_5zone,
   regression_delivered_2024 — all green. The b6 33,056 inversion pin and
   Q8/stability/heating pins live in tests over the byte-identical engine
   and byte-identical b6 packs, so cannot have moved by construction.
   Confirmed.

6. **Ratification items.**
   - **ADR-7 amendment placement — PARTIALLY correct (NOTE 2).** The
     docs/03 note correctly does NOT silently edit the ADR and explicitly
     defers the docs/02 text to supervisor/Richard — good. BUT the design
     review (Edit 7) directed the proposal into `memory/project-state.md`
     → docs/02, not into docs/03; and `memory/project-state.md` is NOT
     updated (git status). Editing docs/03 adds unratified prose to the
     domain-model doc with no schema bump (schema stays v6). Before
     commit: record the proposed ADR-7 amendment in
     `memory/project-state.md` (per convention and the design-review
     instruction), update project-state.md for session hygiene (stage
     status + this finding), and obtain sign-off for the docs/03 note and
     the eventual docs/02 edit.
   - **Border order B4-first — RATIFIED** (ruling above).

## Test-suite evidence (run by me)

- `acceptance_b4_3zone` — 6/6 pass. B4 model binding 0.0195 vs observed
  0.35776; B6 exit constrained 10.2582 TWh; anchors reproduce (B4 net
  15.7819 TWh / 35.8%; B6 22.627 TWh).
- `regression_3zone` — 2/2 pass. Per-zone + links digests pinned; B4/B6
  capability columns ship; B4 binding-period count 337, B6 577
  (337/17277 = 0.0195, 577/17211 = 0.0335 — consistent with the gate).
- `acceptance_b4_robustness` (release) — 2/2 pass. Direction chain and
  Cruachan sensitivity as above.
- `cargo fmt --check` clean; `cargo clippy --all-targets -D warnings`
  clean (no library panics; test-only `allow(panic)`).

## Conditions before commit / stage tag

1. Fix NOTE 1: correct the robustness `eprintln!` so it does not print a
   false `<` chain (state the 2-zone/3-zone ladder interleaving; 3z-copper
   35,968 > 2z-B6 35,648).
2. Clear NOTE 2: record the ADR-7 amendment in `memory/project-state.md`,
   update project-state.md, and get supervisor/Richard ratification for
   the docs/03 note and the docs/02 ADR text before the note stands.
3. NOTE 3 (recommended): change "refuses to wheel" to "under-wheels
   (equal-depth, single pass)" in quoted framing.

None of these blocks the finding, which is genuine, correctly framed, and
ready to carry into the paper once NOTE 1 is fixed.
