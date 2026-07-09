# Stage 7 Q9/bridges package review — rule-6a decomposition + LP-vs-rule-based gap report

**Reviewer verdict (2026-07-06, independent reviewer (stage-7 Q9 gate)):
ACCEPT-WITH-CONDITIONS** — four ordered conditions, none requiring
redesign; the load-bearing constructions are correct and were verified
by independent recomputation and by mutation.

**Scope reviewed (uncommitted):** `grid-adequacy/src/costs.rs` (Q9 +
`generation_asset_line` extraction), `grid-adequacy/src/solve.rs`
(`min_storage_multi`, `storage_gap_report`), `grid-adequacy/src/lib.rs`
re-exports, `grid-core/src/error.rs` (`SanityInvariantViolated`), NEW
`grid-adequacy/tests/q9_decomposition.rs` (12 tests) and
`grid-adequacy/tests/lp_gap_report.rs` (6 tests), NEW
`docs/notes/d4-policy-contract.md`. Full diff read (598 diff lines in
costs.rs; all hunks). No Cargo.toml change, no schema change, no
dependency added. The untracked `figures/` (rs-37y storage trace,
2026-07-04) predates this package — housekeeping, not scope creep.

## Gates (run by this reviewer)

- `cargo fmt --check`: clean. `cargo clippy --workspace --all-targets
  -- -D warnings`: clean.
- `cargo test --workspace`: **69 suites, 665 passed / 0 failed /
  4 ignored** (the 4 = tractability benchmarks, explicitly-run-only) —
  matches the implementer's claim exactly.
- Digests unmoved, asserted inside the suite:
  `pinned_2024_reference_result_digest` (779d7444…), 2-zone, 3-zone,
  migrated 5-zone, composed 8-zone, `pinned_2024_prices_digest` — all ok.
- `q9_decomposition.rs` 12/12 (including the pack-gated 2024-reference
  identity test — verified it HARD-FAILS loudly when the pack is
  absent, in my scratchpad copy, per the audit standing rule);
  `lp_gap_report.rs` 6/6; committed `cost_stack.rs` suite still green.

## A. Rule-6a fidelity — PASS on both flagged resolutions

Re-derived the algebra from the D8 note (rule 6a, rules 2/5/7) and the
docs/04 Stage 7 pins, then from the code.

**The construction telescopes exactly in real arithmetic.** Per band
point: L̄·G = Σᵢ(C_fixᵢ·rᵢ + C_varᵢ) (each per-tech plant-gate LCOE
weighted by its run energy gives exactly C_fixᵢ·rᵢ + C_varᵢ — checked
symbolically); + utilisation wedge Σᵢ C_fixᵢ(1−rᵢ) gives ΣC_fix + ΣC_var
= Cg (the stack's lines 1+2, same `generation_asset_line` arithmetic
and the same SRMC fold — the preconditions force every SRMC-bearing
thermal series into the costed set, so nothing leaks between Cg and the
per-tech sum); + denominator wedge = Cg/E; + missing-line wedge = (Cg +
Cm)/E = the rule-1 headline, since the stack has exactly the six rule-1
lines and lines 3–6 = Cm. No residual term exists in the construction.
The idle-tech edges (r=1 convention where no CF is published;
`plant_gate_lcoe = None` on a zero basis with the £ contribution still
exact) preserve the telescope and are stated in the module docs.

**(i) "Exact" at ≤ 1e-9 relative rather than bitwise — SOUND.** The
docs/04 pin binds two things simultaneously: the identity is "exact",
and component lines are "RECOMPUTED INDEPENDENTLY of the total — a
shared-code reconciliation is a tautology". In f64 those are jointly
satisfiable only the way this package does it: the four anchors are
independent folds (different association orders), so bitwise closure of
mean + Σwedges = headline is not achievable WITHOUT computing one wedge
as the remainder — which is exactly the tautology the pin forbids. The
pinned "exact" is honoured where it is meaningful: no residual term in
the published decomposition (three wedges, no plug), `gap = headline −
mean` asserted BITWISE against the library's own anchors, determinism
asserted bitwise, and closure at ≤ 1e-9 relative with the observed
residual at f64-dust level (my probe on the 2024 reference: closure
holds at 9 printed decimals at all three band points). Rule 2's
bit-exact Σ lines = total remains enforced on the stack itself
(committed `cost_stack_reconciles_with_independently_recomputed_lines`,
still green). The convention is stated in the module docs and the test
docstring. No different construction is demanded; a bitwise-closure
construction would be the weaker artefact.

**(ii) Potential (pre-curtailment) weighting basis — CORRECT reading,
adequately disclosed.** Rule 6a's denominator wedge explicitly lists
curtailment among the generation-vs-delivered differences; curtailment
can only appear in the G→E gap if G includes it, which forces
pre-curtailment potential for weather-driven technologies. The dual
reading (post-curtailment G, curtailment in the utilisation wedge)
would double-count curtailment against the wedge-1 definition and make
wedge 3 measure something other than "realised CF vs the CF assumption
inside the LCOE figure". Rule 6a says "weighting basis stated";
`weighting_basis` is a mandatory struct field whose content is pinned
by test (`q9_states_its_weighting_basis_and_label_mapping` asserts
"potential"). Adequate. The docs/07 five-label mapping (backup,
balancing, curtailment, transmission, stability) is complete and the
ADR-12 "transmission = constraint + interconnection only" disclosure
travels — verified against docs/07 Q9.

## B. Anti-tautology — VERIFIED, including two mutation kills run myself

The test recomputes every anchor and wedge with a test-local CRF and
hard-coded reference literals (I checked each against
`data/reference/costs-gb.toml`: ccgt 1020+14.6/22,900/25y; onshore wind
1380+315.9/39,900/35y/LF 0.36; nuclear 5820+4.7/111,840/60y/16.1 £/MWh;
battery 262/135/12.9k/15y; Viking 1.7bn; DC-LF 3.31). The only library
cost call in the recomputation path is the object under test. The
overnight-no-IDC annuity basis matches `TechnologyCosts::annuity` (a
committed pkg-1 decision, not this package's).

Mutation kills, run in an isolated scratchpad copy:
1. **Utilisation ratio perturbed** (×1.001 at its definition):
   `q9_wedges_match_the_independent_recomputation` FAILS (mean LCOE
   £60.5618 vs recomputed £60.5358) — and, decisively,
   `q9_identity_closes_with_no_residual_term` still PASSES, because a
   consistent ratio error telescopes cleanly. The closure test alone
   would be a tautology exactly as the D8 adjudication warned; the
   independent-recomputation test is the load-bearing gate and it
   kills.
2. **LP/RB runners swapped** in `storage_gap_report`:
   `SanityInvariantViolated` ("LP 1.5 GWh exceeds rule-based 1 GWh
   beyond slack 0.2") — 4 of 6 tests fail, including the pinned
   wheeling gap. The structural invariant itself is what catches the
   swap.

## C. Re-derivation on the 2024 reference — PASS, with one measured hazard (→ condition 2)

Probe (scratchpad copy, packs symlinked), central WACC: E = 267,684.81
GWh delivered; G = 226,753.54 GWh; mean plant-gate LCOE 96.7840; wedges
+11.9611 (utilisation) / **−16.6280 (denominator)** / +1.6290
(missing-line); headline 93.7460; gap **−3.0380**. Closure exact at 9
decimals; gap = headline − mean bitwise as printed.

Hand spot-check of the missing-line wedge from the committed reference
with my own CRF arithmetic: battery 4.7 GW/6.6 GWh 2030-build →
£301.07 m (my CRF(0.075,15) ≈ 0.113287: power 139.50 m + energy
100.94 m + FOM 60.63 m); Viking 1.7 bn × CRF(0.075,40) ≈ 0.079401 →
£134.98 m; Cm = £436.05 m over E → **1.62897 £/MWh** vs the engine's
1.628973106 — match to my arithmetic precision. Stack lines printed
301,070,830.23 + 134,980,533.47 confirm.

A—B—C wheel pins confirmed green in the suite (RB in [1.4,1.6], LP in
[0.9,1.1], gap in [0.3,0.7] and strictly above the slack; gap =
RB − LP exact; full bisection traces embedded).

The hazard: on the 2024 reference ~41 TWh of demand service comes from
uncosted supply (hydro, coal, net imports, pumped-storage net, other),
so E > G and the denominator wedge — labelled 'curtailment'/'balancing'
in the docs/07 mapping — is NEGATIVE, dragging the whole gap negative.
The identity is exact over the costed stack and the coverage note names
this correctly, but a rendered chart would show a negative
"curtailment" wedge. See condition 2.

## D. The other resolutions

**(iii) Costed-coverage note — mechanism adequate, disclosure pins too
weak.** The coverage statement is always populated (notes[0], both
branches) and on the 2024 reference it is load-bearing (the sign flip
above). But the 2024 acceptance test asserts only
`n.contains("costed")` — a one-word probe on the single statement that
prevents the misreading. It does not need to become a separate
mandatory stamp field (recommended, not required — `notes` cannot be
empty by construction), but the pin must be strengthened and the
chart-adjacency obligation recorded. → Condition 2.

**(iv) docs/04 "PerfectForesight policy" superseded wording —
cross-reference-only is NOT sufficient at stage close.** The repo's own
precedent (Stage 5 D5 amendment; the A2 re-pin; the D11 measured-
outcome insertion) is a dated amendment line IN docs/04 at the pinned
text, superseded wording retained. The D4 note's "recorded here rather
than by editing docs/04" is fine for today (the package is not the
stage close), but the amendment line is owed at Stage 7 run-report
time. Wording in condition 1.

**(v) Caller-declared quarantine on the gap report — sound interim,
with a real forgetting vector.** Today the only machine-readable
quarantine lives in the costs reference, which the gap report (a GWh
artefact, no cost rows consumed) never touches — so there is nothing
for the artefact to read and caller declaration is the honest
mechanism. The hole: `&[]` yields `quotable: true` silently, and the
Stage 7 pathway pack WILL run gap reports on scenarios built from
reviewed data packs; when scenario-level quarantine metadata exists,
an artefact layer that forgot to thread it would publish. Mitigation
ruled in condition 3 (affirmative declaration + tracked item), not a
redesign.

## E. The D4 policy-contract note — ACCURATE against the code as it is

Every checkable claim verified: commits `24733eb`, `c5d0bb8`,
`f401b9a` exist with the stated content; `PolicyContract` flags,
trait-default = full rule-based contract, `RuleBased` explicit override
(policy.rs:378/440); tier-1 guards present and per-zone-period in both
`dispatch.rs` and `multizone.rs` (simultaneous charge/discharge,
negative-curtailment, negative-unserved, the deficit-charge guard, the
1e-6-relative conservation check — including the note's correct and
non-obvious point that conservation does NOT catch the two committed
leak classes); √η in `StoreState::apply`/`max_charge`/`max_discharge`
with the contract-boundary rationale; all five named characterisation
tests exist under the stated names and files; the 116/17,568
back-down pin is in `acceptance_2024.rs` (and passed this run);
`PreCharger`/`ChargeAlways` exist; the LP-is-not-a-DispatchPolicy
account matches `lp.rs` and the D12 note. The erratum consequence
("no must-run flag and needs none") is correctly drawn. The D12
review-artifact debt this note closes is closed.

## F. Stage-acceptance lines — both genuinely green; nothing weakened

- "LP storage requirement ≤ rule-based on every scenario (sanity
  invariant); gap reported per scenario" → `storage_gap_report` checks
  the invariant structurally on EVERY report it produces (violation =
  `SanityInvariantViolated`, an engine-defect error, never a finding);
  slack = sum of the two effective bisection tolerances is the correct
  like-for-like allowance for two smallest-known-feasible outputs;
  `lp_gap_report.rs` 6/6 with the strict wheel pinned above slack.
- "LCOE vs. delivered £/MWh gap fully decomposed (Q9)" →
  `q9_decomposition.rs` 12/12 including the 2024-reference identity;
  the sibling "Σ components = total" clause stays covered by the
  committed `cost_stack.rs` reconciliation (green).
- Nothing weakened: `min_storage_for_zero_unserved_lp` now delegates to
  `min_storage_multi` with `run_multi_lp` — same path, same options,
  same guards; the stack's per-asset arithmetic moved into
  `generation_asset_line` unchanged (committed cost_stack tests green);
  all committed digests unmoved; rule-based dispatch untouched.
- TDD: the package is uncommitted so commit order cannot yet evidence
  red-first; the mutation kills (two of them mine, independent)
  substitute for now — see condition 4.

## Conditions (ordered)

1. **docs/04 amendment line at Stage 7 run-report time** (precedent:
   Stage 5 / D11 in-place dated amendments). In the Stage 7 scope
   sentence, after "`PerfectForesight` policy via `good_lp` + HiGHS",
   insert: *(amended 2026-07-06: the adopted D12 design implements
   perfect foresight as a whole-horizon LP function — `run_multi_lp`
   and variants — deliberately NOT a per-period `DispatchPolicy`; the
   ADR-6 trait is per-period/no-lookahead, so the LP's physics are LP
   constraints and the D4 policy choices are absent from its
   objective; the policy-contract mechanism remains for future
   per-period dispatchers. See docs/notes/d4-policy-contract.md and
   docs/notes/d12-perfect-foresight-lp.md.)*
2. **Coverage disclosure, two parts.** (a) Strengthen
   `q9_identity_closes_on_the_2024_reference` to pin the uncosted list
   BY NAME (hydro, coal, exogenous:net_imports,
   exogenous:pumped_storage_net, exogenous:other), not
   `contains("costed")`. (b) Record in the Stage 7 run report (and on
   any rendered Q9 chart) that the coverage note must sit adjacent to
   the denominator wedge, because on partially-costed scenarios the
   wedge labelled 'curtailment'/'balancing' can be NEGATIVE (measured:
   −16.63 £/MWh central on the 2024 reference, gap −3.04) — each
   published chart states whether its costed set covers all supply.
   Recommended, not required: promote the coverage statement from
   `notes[0]` to a dedicated mandatory field like `weighting_basis`.
3. **Quarantine declaration hygiene.** The Stage 7 run report states
   the declaration affirmatively for every gap report it quotes
   ("quarantined inputs declared: none — caller-declared; no
   machine-readable scenario-level quarantine exists yet"), and
   project-state carries a tracked item: wire machine-readable
   scenario-level quarantine into `storage_gap_report`'s artefact
   layer before pathway-pack gap reports are published against
   quarantine-touched scenario data.
4. **At commit:** structure the commits so the red-first ordering is
   visible (tests with/before implementation per house practice) and
   record the mutation-kill set in the commit body; this review's two
   independent kills (utilisation-ratio perturbation → killed by the
   independent recomputation and NOT by closure; runner swap → killed
   by the structural invariant) are the reviewer's evidence of record.

## Minor notes (non-blocking)

- `TechPlantGate::realised_cf` doc comment should state it is on the
  run's series convention (pre-curtailment potential for weather-driven
  technologies) — "realised CF" reads as delivered output to most
  readers (2024 onshore wind prints 0.2927 here, a potential CF).
- The q9 test docstring says the recomputation uses "parsed reference
  numbers"; it hard-codes the reference literals (stronger — say so).
- Theoretical-only drift vector, for the record: q9's per-asset
  variable fold would pick up an SRMC series keyed to a *renewable*
  tech, while the stack's line-2 fold iterates `result.thermal` only,
  and the SRMC-coverage precondition checks thermal only. Unreachable
  with current data (no renewable SRMC exists; renewables with nonzero
  VOM are refused) and the closure test would expose it; a one-line
  precondition extension would retire it.
- Wedge numbering: module docs order wedges utilisation/denominator/
  missing-line (1/2/3) while rule 6a orders denominator/missing-line/
  utilisation; the struct field docs carry the rule-6a numbers
  correctly, so no defect — just don't quote "wedge 1" without naming
  it.

— independent reviewer (stage-7 Q9 gate), 2026-07-06
