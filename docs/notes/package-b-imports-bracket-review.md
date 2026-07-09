# Package B review — import-convention bracketing sweep

Reviewer verdict, 2026-07-03. Scope: the uncommitted working-tree
package (grid-adequacy/src/import_convention.rs NEW,
grid-adequacy/src/lib.rs, grid-cli/src/sweep.rs, grid-cli/tests/cli.rs,
grid-adequacy/tests/import_convention.rs NEW,
grid-cli/tests/regression_imports_bracket_2024.rs NEW). Work order: the
ratified tier-1 fix of the frozen-imports-under-sweep deviation —
three import conventions as exogenous-trace transformations only,
Module 1 sweep gains the bracket, no engine/dispatch/pricing/schema
changes, no pin moves.

## VERDICT: ACCEPT-WITH-NOTES

Condition 1 is the implementer's, one line, and should land with the
package. Condition 2 is the supervisor's record update (ruling in §4,
apply verbatim). The bracketing machinery itself is correct and was
verified independently down to full precision at every pinned value.

### Conditions

1. **Interpretation guard in the CSV assumptions (implementer).**
   Assumption 5 defines the three conventions but does not state the
   single most important reading instruction: the conventions act only
   in £0-priced surplus periods, so they move curtailment and delivered
   energy, NOT price formation — potential capture is invariant by
   construction, the delivered-capture bracket is a convention WIDTH
   under a missing export-price channel (tier 2), and the gas columns
   are unchanged by construction. Without that line, the artefact (and
   the chart, where the frozen delivered curve sits HIGHEST) invites the
   exactly-wrong inference that import flexibility worsens capture.
   Append it to assumption 5 or add assumption 6. This is the
   public-facing object the deviation's binding quoting rule points at;
   the guard belongs in the artefact, not only in test docs.
2. **Record update (supervisor)** — the frozen-imports tracked-deviation
   entry's capture-direction claim is contradicted by the measured
   tier-1 bracket and must be reworded; the Q10 quoting rule needs the
   bracket-specific clauses. Ruling text in §4 below, apply verbatim.

### Notes of record (non-blocking)

- **The strongest result is invisible in the PNG.** The curtailment
  bracket (21.85 → 5.33 TWh at 60 GW) lives in the CSV only; the chart
  carries the delivered-capture bracket plus the frozen curtailment %
  curve. The six-curve readability argument is documented and
  reasonable, but at Q10 figure time the curtailment bracket deserves
  its own panel or curves — flag for the drafting session.
- Assumption 5 records `export_capacity = 9.309999999999999 GW` — float
  noise in a human-facing line. Cosmetic; the regression test pins the
  exact string, so tidy both together or not at all.
- The ordering test's doc comment says "the true invariant asserted
  here is zero ≥ export", but the assertion also pins frozen > zero at
  60 GW — a measured fact, not an invariant (it inverts at 40 GW).
  Fine as a 60 GW characterisation; tidy the comment opportunistically.
- `apply_import_convention` validates trace lengths but not trace
  starts; safe, because starts are preserved by the transform and
  dispatch re-validates alignment before any result exists. Noted only.
- The Module 1 sweep still emits CSV + PNG but no Parquet (docs/06 says
  "both, always" for tabular outputs). Pre-existing gap, predates
  Package B; recorded, not charged to this package.
- The no-links CLI test locates the `[[links]]` block by the
  interconnectors comment marker — fragile to cosmetic scenario edits,
  acceptable in a test.

## 1. Pins and claimed results — all independently verified

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean; `cargo test --workspace` **394 passed /
  0 failed** across 40 test binaries — all run by the reviewer.
- **No pin moves:** the diff touches only the six package files;
  regression_2024.rs, regression_stage2_2024.rs and
  regression_delivered_2024.rs are pristine. Fresh reference run by the
  reviewer reproduced the Stage 1 dispatch digest `779d7444…` and the
  Stage 2 prices digest `1d38ed75…` live.
- **Frozen bit-identity:** the reviewer's fresh release-build sweep
  (40/50/60 GW, `runs/reviewer-package-b/sweep-40-60/`) reproduced the
  frozen columns bit-for-bit against the Package A values (60 GW:
  potential 0.5347799945293277, delivered 0.6106059846371504,
  curtailment 21.845913344574633 TWh, gas 33.21 TWh, gas price-setting
  46.47 %, mean SMP £37.14/MWh).
- **All 12 characterisation pins reproduced to full precision:**

  | GW | convention | potential | delivered | curtailment TWh |
  |----|------------|-----------|-----------|-----------------|
  | 40 | frozen | 0.7128503657378394 | 0.7175046188216030 | 0.7747882502778707 |
  | 40 | zero   | 0.7128503657378394 | 0.7200840143439774 | 1.1755367901103146 |
  | 40 | export | 0.7128503657378394 | 0.7140435797728879 | 0.1984738767756059 |
  | 60 | frozen | 0.5347799945293277 | 0.6106059846371504 | 21.845913344574633 |
  | 60 | zero   | 0.5347799945293277 | 0.5952510429390278 | 17.797696822624250 |
  | 60 | export | 0.5347799945293277 | 0.5514484407085398 | 5.3280243997597205 |

  Unpinned 50 GW mid-point (reviewer's sweep, for the record — NOT
  quotable until pinned, docs/05 rule 3): delivered 0.6395 / 0.6356 /
  0.6110; curtailment 8.286 / 7.409 / 1.863 TWh.
- Export capability: 9.31 GW = 9.8 GW nameplate ex-Greenlink × 0.95,
  recorded in the CSV with its source; verified against the scenario's
  own `[[links]]` entries and locked by both a library test (either-
  endpoint sum, zero-availability link contributes nothing) and the
  assumptions-line regression test.
- Runtime: the 3-point, 3-convention sweep (9 dispatch+pricing runs)
  completes in ~1.1 s wall, release build. The 3× re-dispatch per point
  is milliseconds per annual run — no docs/06 performance target is
  approached, none threatened.

## 2. The no-export-price-channel claim — ruled, with one precision

**For ZERO-IN-SURPLUS and EXPORT-IN-SURPLUS the claim is a theorem**,
resting on the Package A theorem chain (surplus ⟹ no thermal, no
unserved ⟹ SMP convention 2 ⟹ £0):

- Non-mask periods carry bit-identical inputs → identical dispatch and
  SMP.
- Mask periods: under zero the post-import balance is s(t) > 0 (surplus
  branch, £0); under export it is s − min(cap, s) ≥ 0 — either surplus
  (£0) or exactly balanced, where no thermal dispatches, unserved is 0,
  and convention 2 again prices £0. The min() cap is exactly what makes
  this structural: exports never force thermal dispatch.
- Hence SMP series, wind revenue and mean SMP are identical between
  zero and export → potential capture identical (theorem); delivered
  capture differs only through the delivered-energy denominator.

**For FROZEN the claim is data-dependent, not structural**: a mask
period where observed net exports exceed the swept surplus flips to
deficit under frozen → gas dispatches → SMP > 0. The regression module
docs state exactly this ("no such period exists in the 2024 data at
40–60 GW") — honest, and the third regression test asserts the
potential-capture equality that would break the day it stops holding.
Empirically confirmed: all three potential columns identical to the
last digit at 40, 50 and 60 GW in the reviewer's sweep.

**The 40 GW frozen-vs-zero curtailment inversion is arithmetically
confirmed by the pins themselves**: with stores inert and no
deficit-flips, zero − frozen curtailment = −Σ_mask(imports). At 40 GW
that is +0.40 TWh ⟹ the observed trace nets ~0.40 TWh of EXPORT over
the deepest-wind mask — zeroing removes real relief, as claimed. At
60 GW it is −4.05 TWh ⟹ ~4.05 TWh of net observed IMPORTS over the
much wider mask — precisely the "retains foreign must-take supply"
arm of the deviation.

Consequence, ruled correct: the bracket acts on curtailment and
delivered energy only. The delivered-capture ordering at 60 GW (frozen
0.6106 ≥ zero 0.5953 ≥ export 0.5514) is the arithmetic of adding
£0-earning delivered MWh to the denominator, not a statement about
markets — and it inverts again at 40 GW (zero 0.7201 > frozen 0.7175),
so the only structural capture ordering is zero ≥ export.

## 3. Definitions ruling (item 2)

All five SOUND and adequately prose-documented (D4 precedent met — the
conventions live in full prose at the code site):

- **Strict s(t) > 0 mask** — matches dispatch's `net > 0` exactly
  (verified in dispatch.rs; renewable output is capacity × cf with no
  availability derating, which the mask mirrors). The boundary choice
  is argued in the prose (zero-in-surplus would zero a nonzero import
  at s = 0) and pinned by the fixture's t3.
- **Pre-storage mask by construction** — sound: a trace-level
  convention applied before dispatch; storage sees the transformed
  trace. Stated out loud in the prose, as required.
- **SET to 0, not clamp** (zero-in-surplus) — defensible and ruled the
  better choice: the convention is "no exchange in surplus", a genuine
  bracketing endpoint. Removing REAL observed export relief is the
  point, not a bug — it is what makes zero bracket frozen from the
  other side at moderate capacities (the 40 GW inversion is the
  demonstration). The consequence is that the curtailment bracket must
  be quoted as min/max over the three conventions, not [export,
  frozen] — folded into the §4 quoting rule.
- **Aggregate-on-first** — sound and the only defensible reading:
  dispatch sees the aggregate; any split would be invented physics.
  Documented as a choice; no multi-import scenario exists in the repo;
  tested anyway.
- **Either-endpoint link capability** — right for the current single-
  zone bracket: GB interconnectors are declared with one bidirectional
  capacity, so Σ capacity × availability over links touching the zone
  is the natural export capability; Greenlink (availability 0)
  correctly contributes nothing; 9.8 × 0.95 = 9.31 GW verified. If an
  asymmetric-direction link ever appears the convention needs
  revisiting — the CLI `--export-capacity-gw` override already covers
  that day, and the no-silent-default rule is enforced and tested at
  both the library (`Ok(None)`) and CLI (hard error naming the flag)
  levels.

## 4. Ruling for the record (item 4) — supervisor applies verbatim

The measured tier-1 bracket contradicts the tracked-deviation entry's
capture-direction claim as written, while VINDICATING its curtailment
claim. Ruled: the bracket is a strong one-sided correction on
CURTAILMENT (frozen overstates it, up to 4× at 60 GW), but only a
WIDTH on delivered capture — and because all three conventions act
only in £0-priced surplus periods, none of them carries the export-
price channel that is the real-world mechanism behind the entry's
direction claim. The claim about REALITY survives; the claim as a
description of what the tier-1 bracket would measure does not.

(a) **Tracked deviation (memory/project-state.md)** — replace the
frozen-imports-under-sweep entry's body from "Physically wrong away
from the anchor…" through "…Package B, this session." with:

> Physically wrong away from the 2024 (~30 GW) anchor, in BOTH
> directions. Tier-1 bracket DELIVERED (Package B, 2026-07-03,
> reviewed ACCEPT-WITH-NOTES,
> docs/notes/package-b-imports-bracket-review.md): the Module 1 sweep
> carries three conventions (frozen / zero-in-surplus /
> export-in-surplus; export cap 9.31 GW from the scenario's own links),
> exogenous-trace transformations only, frozen default bit-identical,
> 12 values pinned at 40/60 GW. Measured: CURTAILMENT is the strong
> one-sided result — frozen overstates it at high wind (60 GW: 21.85
> frozen / 17.80 zero / 5.33 export TWh); at 40 GW frozen-vs-zero
> INVERTS (0.77 vs 1.18 TWh — the observed trace already exports in
> the deepest-wind periods), so the curtailment bracket is min/max
> over the three conventions, not a fixed ordering. CAPTURE direction
> corrected from this entry's original prediction: the conventions act
> only in £0-priced surplus periods, so they cannot move price
> formation — potential-basis capture is IDENTICAL across conventions
> by construction, and the DELIVERED-capture bracket runs OPPOSITE the
> predicted direction (60 GW: frozen 0.611 ≥ zero 0.595 ≥ export
> 0.551 — export relief adds £0-earning delivered MWh). The
> original "frozen overstates cannibalisation" claim survives only via
> the EXPORT-PRICE channel (real exports in surplus periods would lift
> prices above the £0 floor), which tier 1 lacks by design — so ALL
> tier-1 variants likely UNDERSTATE real high-wind capture, and the
> bias-FOR-the-skeptical-thesis direction stands for reality, not for
> the measured bracket. GAS SHARE is un-bracketed by tier 1
> (degenerate by construction: the conventions touch only no-thermal
> surplus periods), and the low-wind arm (real imports would exceed
> the frozen trace) is untouched by tier 1 entirely.
> BINDING RULE (Q10/Q2, supersedes the pre-bracket rule): quoting per
> the Package B review §4(b); tier 2 (pricing on the multi-zone
> engine, endogenous flows) remains required before high-wind capture
> or gas-share numbers are quoted without the missing-price-channel
> caveat.

(b) **Q10/Q2 quoting rule (binding):**

> (i) High-wind CURTAILMENT: quote as the convention-labelled min/max
> bracket over the three conventions (60 GW: 5.3–21.8 TWh, export
> lower end, frozen upper; 40 GW: 0.20–1.18 TWh, export lower, ZERO
> upper). The frozen number alone may be quoted only as an explicit
> stated upper bound at ≥ 50 GW; at moderate capacities frozen is NOT
> the conservative end (the 40 GW inversion) and must not be presented
> as such.
> (ii) POTENTIAL-basis capture: its invariance across the tier-1
> conventions is BY CONSTRUCTION (they act only in £0 surplus
> periods) and must never be quoted as evidence that import behaviour
> does not affect capture.
> (iii) DELIVERED capture: quote the tier-1 range as a convention
> WIDTH (60 GW: 0.551–0.611), never as a correction direction, and
> always with the caveat that all three variants lack an export-price
> channel, so the entire bracket likely UNDERSTATES real high-wind
> capture; the frozen-highest ordering must never be glossed as
> "import flexibility worsens capture".
> (iv) GAS share / gas burn: tier 1 provides no bracket (identical by
> construction); the pre-bracket caveat with direction stated
> continues to bind, at low wind especially.
> (v) Only the pinned 40 and 60 GW bracket values are quotable today;
> any other point (the 50 GW mid-point included) gets its pinned
> regression test first (docs/05 rule 3).
> (vi) The Package A basis rules (§4(d)–(e) of that review: delivered
> basis is the market-comparable headline, basis label always) compose
> with, and are not displaced by, the above.

## 5. Test quality (item 5) and checklist verdicts

1. Acceptance tests: PASS — full suite 394/0 run by the reviewer; all
   12 pins and both Stage 1/2 digests reproduced live and
   independently.
2. ADR compliance: PASS — no schema change (no version bump due);
   pure trace transformation, no wall-clock/globals/randomness
   (temp_dir in tests only); `Power`/`PerUnit` newtypes across the new
   public API; transform lives in grid-adequacy where `RunInputs`
   lives, CLI wiring in grid-cli.
3. Conventions: PASS with condition 1 — no library panics
   (`GridError::InvalidRunInputs` on missing/misaligned traces;
   `Trace::from_parts` error propagated with `?`); clippy/fmt clean; no
   new dependencies; CSV columns appended after the existing ones
   (append-only stability held, asserted in cli.rs); hashes embedded
   unchanged; assumption line carries definitions + 9.31 GW + source;
   chart bracket curves labelled with convention and basis, frozen
   potential curve retained, footer carries the export-cap note.
   Runtime impact of the 3× re-dispatch: negligible (§1).
4. TDD evidence: PASS per the recorded single-tree precedent (gate is
   test quality + review). Quality is high: the proptest asserts the
   exact target (−min(cap, s)) and the only-surplus-touched invariant
   over random traces including negative imports; the boundary period
   (s = 0) is pinned strict in the fixture; the set-vs-clamp choice is
   pinned (t4, −1 → 0); the multi-series aggregate rule is tested; the
   ordering test asserts the TRUE (inverted) delivered ordering with
   the mechanism named in its failure message, and the
   potential-equality assert is the tripwire for the day frozen's
   data-dependence bites; pins at ±1e-7/±1e-6 are tight enough to
   catch any convention swap (columns differ at the 3rd decimal);
   no-links is tested at both library and CLI levels.
5. Data deliverables: N/A — no new data, no licence surface; the one
   new physical constant (9.31 GW) is derived from the scenario's own
   committed links and recorded with provenance in the artefact.
6. Scope: PASS — matches the ratified package intent exactly; the
   diff touches no engine/dispatch/pricing/schema file (verified:
   lib.rs module registration + sweep.rs + cli.rs + three new files);
   no pin moves; no doc edits inside the package diff.

## 6. Reviewer reproduction artefacts

- `runs/reviewer-package-b/run-2024/` — fresh reference run (digests
  779d7444…, 1d38ed75… reproduced).
- `runs/reviewer-package-b/sweep-40-60/` — 40/50/60 GW bracket sweep,
  release build, ~1.1 s (table in §1).
(runs/ is gitignored; regenerate with `grid-cli run` /
`grid-cli sweep wind-capacity --min-gw 40 --max-gw 60 --step-gw 10` at
the package's commit.)
