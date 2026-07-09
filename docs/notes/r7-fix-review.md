# R7 flow-walk stall fix — independent review (the R7 gate)

**Date:** 2026-07-06
**Reviewer:** independent reviewer (R7 gate)
**Package under review:** uncommitted working tree on 23676f1 —
`grid-adequacy/src/flow.rs` (both walks), re-pinned acceptance files
(`acceptance_d11_sweep`, `acceptance_d11_priced_ladder`,
`acceptance_b4_3zone`, `acceptance_b4_robustness`,
`acceptance_b6_2zone`, `acceptance_d13_60gw`, `acceptance_d13_composed`),
CLI digest re-pins (2/3/5/8-zone), `docs/08` R7 row,
`docs/notes/d13-run-report.md` §6 caveat-(c) pointer,
`docs/manual/manual.typ` R7 row, `memory/project-state.md`.
**Method:** re-derive, don't trust. Every load-bearing claim below was
re-derived with my own instruments (own harness, own old-engine build
from HEAD, own per-period census); the implementer's scratch evidence
was not reused.

## VERDICT: ACCEPT-WITH-CONDITIONS

The engine fix is correct, minimal, deterministic, tolerance-free, and
its attribution claim (only old-cap-truncated walks change) is proven
by construction and confirmed at 6M randomized cases with zero
counterexamples. Every committed finding I checked survives in
direction and band; the B6 storage-understatement pins are literally
unmoved. All defects found are documentation-level (stale pre-fix
literals and two denominator slips in comments); none blocks the
engine change. Conditions 1–7 below are ordered; 1–5 must land in the
same commit series as the fix.

---

## 1. The design, verified against the diff

- **Passes 0–63 bit-identical by construction.** Read line-by-line
  against `git show HEAD:grid-adequacy/src/flow.rs`: for `pass < 64`
  the probe positions are the old expressions (`r_exp + q`,
  `r_imp − delivered·q`), the loop body (gap test, `rate`, `d_cross`,
  the three step branches, `q += d_exp.min(d_imp)`) is character-for-
  character the old arithmetic, and the snap block is gated on
  `recovery`. A walk that broke within 64 passes takes identical
  branches and returns an identical `q`. The only behavioural change
  is that a walk which exhausted 64 passes (the old silent truncation)
  now continues into passes 64–95. The changed-bits class is therefore
  exactly the old silent-truncation class, by construction. Same
  verified for `equalising_flow_priced`; its recovery snap re-derives
  `d_exp`/`d_imp` with the same expressions both step branches use, so
  the snapped side is always the stepped side.
- **Recovery provably terminates.** Each recovery pass either breaks
  (gap closed, crossing, or cap) or steps and snaps the stepped side's
  probe strictly past its breakpoint (`next_up`/`next_down` — exact
  ULP operations, no tolerance). The next pass's probe on that side is
  past the breakpoint regardless of whether the `q` increment was
  absorbed below ULP. Breakpoints are finite (≤ 6 segments/curve on
  the frozen flow ladder), so ≤ ~12 recovery passes suffice; 32 are
  budgeted. Skipped slivers are below the representable resolution of
  `q` (the stall class is defined by exactly that), so no representable
  energy is created or lost. `ZoneCurve`, both probes, and `signal`
  are untouched (verified against HEAD). No other pass-capped walk
  exists in the workspace (grepped).
- **Determinism preserved:** pure arithmetic, no wall-clock, no
  globals, no randomness, no epsilon tolerance. Two full CLI runs of
  the 2-zone scenario on the post-fix engine are byte-identical
  (links.csv and summary.toml compared; digests equal).

## 2. Claim 1 (red-first minimal reproduction) — VERIFIED

Ran the exact reproduction (exporter `[(0,1000)]` r=100, importer
`[(3,0.01),(4,5.0)]` r=2.0, loss 0.015) through the VERBATIM old code
(lifted from HEAD into my harness, instrumented only with a cap flag)
and the verbatim new code:

- old scarcity walk: `q = 2.02030456852791884`, cap exhausted
  (silent truncation) — matches the claimed defective value;
- old priced walk: same truncated value — the defect was in BOTH walks;
- new walks (both): `q = 2.03045685279187804 = 2.0/0.985` exactly,
  via the recovery regime, no exhaustion.

The sliver mechanism re-derived analytically: pass 1 steps exactly
`d_imp` onto the coal/ccgt edge; the recomputed residual lands
8.67e-18 above it; the next increment is absorbed below
ULP(q) ≈ 4.4e-16. The pinned tests in `flow.rs` assert the corrected
value at 1e-12 and the priced/scarcity byte-identity on the stall
case; both fail on the pre-fix arithmetic (demonstrated on the
verbatim old code), so red-first is functionally established.

## 3. Claim 2 (attribution) — VERIFIED at 6M cases, own harness

Own harness (scratchpad `r7-reviewer-harness`, own splitmix64 RNG, own
case construction: 1–4 segments/side, ceilings on mixed log scales
10^[−3,3] to provoke boundary-exact steps, losses 0–6% incl. 0,
residuals spanning surplus/stack/unserved, caps 10^[−2,2]).
Two seeds × 3M cases, scarcity AND priced walks on every case:

| walk | cases | old-stalls | terminating bit-identical | mismatches | new exhaustions | q regressions |
|---|---|---|---|---|---|---|
| scarcity, seed A | 3M | 52,943 | 2,947,057 | **0** | **0** | **0** |
| scarcity, seed B | 3M | 53,222 | 2,946,778 | **0** | **0** | **0** |
| priced, seed A | 3M | 58,076 | 2,941,924 | **0** | **0** | **0** |
| priced, seed B | 3M | 58,372 | 2,941,628 | **0** | **0** | **0** |

Every old-terminating walk bit-identical; every old-stalled walk
completes under recovery with `q_new ≥ q_old`. This independently
corroborates the implementer's 20M-case sweep.

## 4. Claim 3 (movement adjudication) — the movements are SOUND

**(a) The defect was real and the released flow is the correct flow.**
The old walk returned flows with link headroom left and the signal gap
open — violating the walk's own documented equalisation rule. Strongest
internal evidence: post-fix, the 2-zone 60 GW copper split lands at
near-exact equal surplus depth (SCO 10.934132203631213 vs RGB
10.934132203631211 TWh) — which is what the documented rule says
should happen and the pre-fix engine never achieved (13.77/10.42).

**Per-period census, independently re-derived.** I built the pre-fix
CLI from a HEAD worktree (it reproduces the committed 2-zone links
digest 905efb55… exactly — confirming both my build and the committed
record), ran both engines on `gb-2024-2zone.toml` against the same
data packs, and diffed per-period B6 flows: **2,795/17,568 periods
changed** (2,787 toward more southward export), mean |Δ| 0.443 GW,
max 1.914 GW, **net 0.618244 TWh** — matching the gate (ii) pin delta
(16.405946369726383 − 15.787702182212668 = 0.618244) to the last
digit. The implementer's census (2,795 / 0.44 / 1.91 / −0.618) is
confirmed exactly.

**Gates and bands (re-run, not accepted on trust):**
- Gate (i) 18.8088 → 19.8982 (+1.0894), TOWARD the 22.627 DA anchor,
  within ±4.5. PASS.
- Gate (ii) 15.7877 → 16.4059 (+0.6182), TOWARD the 17 TWh outturn,
  within ±2.5. PASS.
- Gate (iii) 0.23229 → 0.25019 vs observed 0.2360 ± 0.04: in band,
  but honesty note — it moved AWAY from the observed share (0.37pp →
  1.42pp). The docs/08 phrase "unmoved-or-improved" is generous here;
  the b6 banner (condition 1) must state this direction plainly.
- 5-zone tier-2 family: curtailment 4.007463 → 3.982737 (−0.024726,
  within the §B.4 ≤ 0.025 bound — but see condition 5: it is −0.0247,
  not the quoted −0.0245); delivered capture −0.000195 ≤ 0.0002 ✓.
  NOTE: §B.4's parenthetical point estimate ("capture ≥ 0.697581",
  echoed as "≥ 0.6976" in the sweep run report §6) is EXCEEDED by the
  measured 0.697489; the disclosed ≤ 0.0002 bound held. Condition 6.
- Composed/D13: GB curtailment ceiling 30.1747 → 29.9099 (−0.2647);
  all-zone 35.5202 → 35.2529 (−0.2673, the docs/08 "−0.267" ✓).
- 3-zone B4 binding 337 → 347 ✓ (re-run in regression_3zone).

**(b) No committed finding changes direction or leaves its band:**
- **B6 storage understatement (+38.5 %/+49.3 %):**
  `acceptance_b6_robustness.rs` was NOT re-pinned, its EXACT pins
  (23,872 / 26,480 / 35,648 / 33,056 / 33,632 / 49,152 GWh, < 1e-6)
  all pass on the post-fix engine (ran; 2/2, 18.7 s — not among the
  ignored). The finding survives numerically verbatim, not merely in
  direction. Same for `acceptance_b4_robustness` (Cruachan N/S
  re-pinned 6.9747/6.9742 — immateriality intact).
- **D11 above-bracket finding:** capture 0.6977 → 0.6975, still far
  above the 0.611 bracket top; movement AGAINST the finding exactly as
  the sweep review predicted. Stands.
- **D13 8(i)/8(ii) RED verdicts:** shape-robust under BOTH the
  pre-registered and post-fix comparators (gas +4.41 %/+4.55 % vs
  ±2 %; imports +18.2 %/+17.9 % vs ±5 %; A1 outright +27.5 % vs
  ±10 %; B4 187 vs 347 still a DECREASE — branch (b) RED holds; B6
  671 vs 577 still a modest rise — branch (a) holds).
- **B4 pre-emption finding:** 347/17,277 ≈ 2.0 % vs observed 35.8 % —
  unchanged in substance.

**(c) The untouched records:** single-zone digest 779d7444… — re-run,
unmoved (regression_2024, 6/6). Stage 5 acceptance — re-run, 15/15;
the A2a/A2b EXACT count pins (15,823 and 1,036, unmodified file) pass
on the post-fix engine, i.e. the A2 record is literally bit-unmoved.
The LP path: no LP test was re-pinned and all pass — consistent with
the fix touching only the rule-based walk.

## 5. Claim 5 (gates) — PASS

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean.
- Full suite: **694 passed / 0 failed / 4 ignored** (the 4 are the
  `tractability_bench` timing probes — pre-existing, correctly
  ignored). Matches the claimed 694/0/4.
- No panics in the fix (no unwrap/expect/panic; probe functions total).
- Determinism: two-run byte-identity on the 2-zone multi-zone run;
  fix is tolerance-free (ULP snaps only).
- `manual.typ` compiles (typst); its R7 row edit is accurate.
- docs/08 R7 row: accurate except the −0.0245 slip (condition 5).
- d13-run-report caveat-(c) edit: accurate but insufficient as the
  quoting correction (condition 1).
- Scope: matches the R7 work order; ADR (docs/02) untouched; schema
  unchanged (v7, no domain-model edit owed); no new dependencies.
  Housekeeping note, outside this package: an untracked `figures/`
  directory sits in the tree.
- TDD: red-first established functionally (§2). Commit-order evidence
  must be preserved at commit time (condition 7).

## 6. Defects

1. **Denominator slips in `acceptance_d13_composed.rs` re-pin
   comments** (~lines 207–211): "committed 3-zone 0.020133" is
   347/17,235 (the LP sentinel mask) — the pin block's own declared
   gate-(iii) denominator gives 347/17,277 = 0.020085; "committed
   0.033629" is 577/17,158 (the 2-ZONE capability-observed
   denominator) — the committed 3-zone B6 value is unchanged at
   577/17,211 = 0.033525. This is precisely the binding-share
   conflation the b6 run report's publication rule 4 bans. Comment-
   only (no assertion affected), but these files are quoting records.
2. **Stale pre-fix literals inside the re-pinned test files** —
   headers, in-test comments, and assert-message strings now
   contradict the same files' pins:
   - `acceptance_d13_composed.rs` module header (75.019 / +4.49 % /
     42.428 / +18.1 % / +27.4 %);
   - `acceptance_d13_60gw.rs` header ("+4.49 % gas / +18.1 %
     imports"; "ceiling 30.175") and the §8 doc comments (~1502–1522:
     4.007, +32.217, +20.02, +11.87, 30.175/30.17);
   - `acceptance_b6_2zone.rs` in-test comments at the gate asserts
     ("measured 18.809 (−3.82)", "measured 15.788 (−1.21)");
   - `acceptance_b4_3zone.rs` assert-message literals ("two-zone
     18.809"; "must exceed the two-zone 27.14 TWh" — the constant it
     tests against was re-pinned to 27.025);
   - `acceptance_d11_sweep.rs` module header (0.6977; "+35.94 TWh,
     71.80 TWh").
3. **Arithmetic slip −0.0245** (should be −0.0247) in the docs/08 R7
   row and the `acceptance_d11_sweep.rs` re-pin comment. The ≤ 0.025
   bound holds either way.
4. **Quoting records still carry pre-fix numbers** with no correction
   apparatus (ruled in §7; condition 1).

## 7. RULING — the quoting records (claim 4)

**Form: in-file correction banners, the b6 §3 precedent — NOT a single
addendum note.** Reasons: the precedent exists and its form is proven
in this repo; each report is quoted independently, so the correction
must be visible without following a cross-reference; the moved values
are few and enumerable per file. Each banner: dated 2026-07-06, cites
docs/08 R7, lists old → new per quoted value, and states what did NOT
move. File-by-file (verified against the re-pinned tests):

1. **`docs/notes/b6-two-zone-run-report.md`** — banner covering:
   §2 gates (18.809 → 19.898 TWh, r 0.744 → 0.740; 15.788 → 16.406;
   binding 23.23 % → 25.02 % — state plainly that this one moved AWAY
   from the observed 23.60 % while staying in band); §4 rule 3
   (27.14 → 27.03 constrained / 13.77 → 10.93 copper; system-net legs
   +13.37/−6.72/+6.65 → +16.09/−7.24/+8.85 TWh); §4 rule 4
   (0.2323 → 0.2502; 0.2330 → 0.2510, 3,998 → 4,306 periods).
   State explicitly: the §3 storage table (23,872/26,480/35,648 and
   the controls) is UNMOVED — verified by the unmodified exact pins.
2. **`docs/notes/stage-5-run-report.md`** — banner: A1 record
   71.80 → 71.70 TWh (−1.36 % → −1.50 %), +35.94 → +36.03 TWh
   (+7.9 % → +8.19 %), both still PASS; the five-border table values
   move with the re-pinned 5-zone digests (point at
   `regression_5zone.rs` for the old/new record); A2a/A2b/A3/A4:
   A2a 15,823 and A2b 1,036 verified bit-unmoved (exact pins,
   unmodified file, passing).
3. **`docs/notes/d11-sweep-run-report.md`** — banner: §3 tables
   (curtailment 4.007463 → 3.982737; delivered 0.697684 → 0.697489;
   potential 0.681637 → 0.681545; gas 40.695 → 40.670; net imports
   −6.456 → −6.463; price-setting 64.25 → 64.19 %; SMP 51.24 → 51.20;
   ladder column likewise, from the re-pinned tests); §5 headline
   quote block; anchor A1 +35.94/71.80 → +36.03/71.70; §6 marked
   RESOLVED with the condition-6 honesty note; §8 open item 3 marked
   delivered. Finding unchanged (above the bracket on every axis).
4. **`docs/notes/d13-run-report.md`** — the caveat-(c) pointer already
   added is kept but is NOT sufficient; banner covering: §2 anchor
   table (75.019 → 74.960; 42.428 → 42.473; 7.466 → 7.452; 185 → 187;
   662 → 671); §3 60 GW tables (30.175 → 29.910; 46.874 → 46.769;
   11.869 → 11.702; NSCO/SSCO split 24.917/5.114 → 24.898/4.869;
   B4/B6 south 5.507/21.176 → 5.521/21.504; gross trade; saturation
   counts; 1,706/4,769 → 1,718/5,161; 207.926 → 207.684;
   35.520/36.929 → 35.253/36.666); §4 derived arithmetic
   (+20.020 → +20.044 vs the corrected comparator); the §6/§7 quote
   blocks (+11.87 → +11.70; 30.17 → 29.91; 36.93 → 36.67). State: the
   LP leg and both RED verdict shapes unchanged.
5. **`docs/notes/b4-lp-findings.md`** — one-line correction: the
   rule-based comparator row 1.96 % (337/17,235) → 2.01 % (347/17,235);
   the LP band itself is untouched.

**Review notes stay untouched** (`d11-sweep-review.md`,
`d13-composed-boundary-trade-review.md`,
`multizone-independent-audit-2026-07-05.md`, the engine reviews):
they are dated review records of the pre-fix engine; the audit trail
runs through docs/08 R7 and the per-pin old-value records. Editing
other reviewers' signed records is the wrong instrument.

**The 8(i)/8(ii) comparator question — RULED: leave them, disclose.**
`GAS_5ZONE_ANCHOR_TWH = 71.797411264632` /
`IMPORTS_5ZONE_ANCHOR_TWH = 35.935152502942` (and
`B4_RB_BINDING_3ZONE = 0.019506`) stay at the pre-fix record: they are
pre-registered comparators ("NOT re-pinnable knobs"), and re-basing
them after seeing the measurement is exactly what rule 8 exists to
prevent. The mixed-engine wedge is 0.13–0.25 % — an order below the
band widths — and both verdicts are robust under either basis (§4b).
CONDITION: a note at each comparator block recording that the
committed record moved with R7 (71.7008 / 36.0259; B4 0.020085), that
the registered comparison is intentionally against the pre-fix record,
and that the verdicts hold under both.

## 8. Conditions (ordered; 1–5 in the same commit series as the fix)

1. Apply the §7 correction banners to the five quoting records,
   exactly as enumerated (including the gate-(iii) away-from-anchor
   statement and the "what did not move" statements).
2. Fix the two denominator-slip comment values in
   `acceptance_d13_composed.rs` (0.020133 → 0.020085 i.e. 347/17,277;
   0.033629 → 0.033525 i.e. 577/17,211 unmoved).
3. Sweep the stale pre-fix literals in the re-pinned test files
   (defect 2 list) — update them or mark them "(pre-fix)" explicitly,
   following the good pattern already used in `regression_2zone.rs`.
4. Add the 8(i)/8(ii) comparator disclosure notes (§7 ruling).
5. Correct −0.0245 → −0.0247 in the docs/08 R7 row and the
   `acceptance_d11_sweep.rs` re-pin comment.
6. In the d11-sweep-run-report §6 resolution note (condition 1),
   record honestly that the measured capture movement (0.697489)
   exceeded §B.4's parenthetical point estimate (≥ 0.697581 /
   "≥ 0.6976") while remaining within the disclosed ≤ 0.0002 bound.
7. Commit hygiene: land as a commit series that preserves the
   red-first evidence (repro tests visible before or with the fix),
   referencing R7; re-run fmt/clippy/suite at the final commit.

Conditions 2, 3 and 5 are comment/prose edits with no assertion
changes; nothing in this list touches the engine arithmetic, which is
accepted as delivered.

## 9. Verification inventory (re-derived, not trusted)

- Own harness, verbatim old (HEAD) + new walks, both variants: minimal
  repro; 2 × 3M-case sweeps × 2 walks (§3).
- Pre-fix CLI built from a HEAD worktree; reproduces the committed
  2-zone links digest 905efb55… exactly; post-fix CLI reproduces the
  re-pinned 46781178… twice (byte-identical runs).
- Per-period 2-zone census: 2,795 changed / 0.443 mean / 1.914 max /
  0.618244 TWh net — exact match to gate (ii)'s pin delta.
- Full suite 694/0/4; fmt, clippy clean; regression_2024 and
  acceptance_stage5_2024 re-run individually.
- All movement deltas recomputed from the per-pin old-value records
  (gate (i) +1.0894; gate (ii) +0.6182; SCO copper −2.8342; tier-2
  −0.024726 / −0.000195; ceilings −0.2647/−0.2673; 337→347; 185→187;
  662→671; A1 anchor 36.0259/71.7008 in their A1 bands).
- typst compile of the manual.

— independent reviewer (R7 gate), 2026-07-06
