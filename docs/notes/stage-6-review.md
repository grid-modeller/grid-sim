# Stage 6 review — stability engine (part 1), 2026-07-03

Adversarial review of the uncommitted Stage 6 package (reviewer agent).
Verdict: **ACCEPT-WITH-NOTES** — commit is authorised once the conditions
below are actioned; **do not tag `stage-6-validated` yet** (ruling c).
Every delivered number was independently reproduced; nothing was taken on
trust.

## Independently reproduced (commands and results)

Suite health (all run by the reviewer, not accepted from the implementer):

- `cargo fmt --check` — clean (exit 0).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — **298 passed, 0 failed** (includes all
  Stage 1–4 pins; `cargo test -p grid-cli --test regression_2024`
  re-passed explicitly: dispatch digest `779d7444…` unmoved).
- `cargo bench -p grid-stability` — mean event simulation **1.48 ms**
  (target < 10 ms, smoke < 20 ms).

Anchors, via `cargo test -p grid-stability --test acceptance_aug2019
report_measured_anchor_values -- --nocapture` AND independently via
`grid-cli stability event --event scenarios/events/gb-2019-08-09.toml`
(default 210 GVA·s and `--inertia-gva-s 219.632`) — both paths agree:

| Anchor | 210 GVA·s | 219.632 GVA·s | Gate | Verdict |
|---|---|---|---|---|
| T1 nadir | 48.7928 Hz (t=72.91 s) | 48.7931 Hz (t=73.61 s) | (48.75, 48.80], stage 1 only | PASS (stage-1 LFDD operated, stage 2 did not — asserted) |
| T2 first arrest | 49.1706 Hz (t=13.76 s) | 49.1887 Hz (t=13.94 s) | 49.10 ± 0.10 | PASS — top-of-band at the upper inertia bound (0.011 Hz margin), see ruling (a) |
| T3 RoCoF (pinned 2-s window from 0.51 s) | −0.1457 Hz/s | −0.1405 Hz/s | ±25 % of 0.144 [0.108, 0.180] | PASS (1.2 % / 2.4 % from measured) |
| T4 1,000 MW counterfactual | min 49.5396 Hz | min 49.5448 Hz | ≥ 49.5, no LFDD | PASS |

Diagnostics (un-gated, reproduced): LFDD stage-1 trigger t=72.61/73.31 s
(actioned 72.91/73.61) vs 75.9 s measured; steepest 1-s RoCoF
−0.1590/−0.1528.

Measured record re-derived from the committed fixture
(`data/reference/neso-frequency-2019-08-09-event-window.csv`, 540 rows):
minimum **48.787 Hz at 2019-08-09T15:53:49Z** — matches the evidence
report's nadir and the fixture's own header.

Module 6 first cut, via `grid-cli stability inertia`:

- `scenarios/gb-2024-reference.toml`: min **0.00 GVA·s at
  2024-04-06T11:30:00Z**; 2 zero-inertia periods; below 120 GVA·s:
  **15,020 periods = 7,510 h (85.49 % of 17,568)**; below 102: **13,335
  periods**. Matches the pins in
  `grid-stability/tests/inertia_sum.rs` and the CLI test.
- `scenarios/royal-society-37y-lean.toml`: **zero inertia in all 701,280
  periods**, `has_synchronous_provision = false`, FINDING line fires.

Schema v3 migration (ruling d evidence): `grid-cli validate --scenario
grid-core/tests/fixtures/v2-gb-2024-reference.toml` → **exit 2** with the
full migration message (names `inertia_h`/`synchronous`, the derived
defaults, MVA = GW/0.9, the one-line "set schema_version = 3 and change
nothing else" migration, and points at docs/03). The frozen v2 fixture is
byte-identical to the HEAD reference scenario plus a 4-line frozen-header
(diffed). All five migrated v3 scenario TOMLs parse and run.

Analytic gate: `analytic.rs` pins ≤ 1 µHz vs the closed form
`f₀·√(1 − P·t/E)` over 60 s at 10 ms (passes; pin has slack against a
first-order regression, per its own justification).

## Rulings

**(a) T2 top-of-band pass — ACCEPTABLE AS-IS, with a mandatory documented
limitation; no pre-tag investigation required.** Grounds: the ±0.10 band
was pinned before the model existed, from the irreducible input-ambiguity
budget (evidence report §5), and the model lands inside it at both
official inertia bounds — that is the test working as designed. The
top-of-band position is causally coherent with three un-gated
diagnostics, all reviewer-verified from the model trace: (i) modelled
first arrest at t≈13.8–13.9 s vs ~25 s measured; (ii) mid-event
over-recovery to **49.63 Hz at t=30 s** vs the measured 49.2 plateau
(reviewer-sampled from `frequency_trace.csv`); (iii) LFDD trigger ~3 s
early. All three say the same thing: the response envelope delivers
faster mid-phase than the real ESO deployment timeline — a stated
envelope convention, not a physics error, and one that the gated early
phase (T3) and protection phase (T1) are insensitive to.
Conditions: the stage run report must document the top-of-band pass, the
mechanism, and diagnostics (i)–(iii) at KC-4-style prominence; and note
that the 49.20 edge is a live constraint — any future re-derivation that
strengthens mid-phase response (damping, delivery factors, ramps) risks
crossing it and must not be "fixed" by retuning against the gate.

**(b) No input tuning against gated anchors — VERIFIED.** Full chain
audited: losses = published trip sequence (magnitudes/times, ESO Table 2
+ timeline); response holdings × published delivery factors (1,022×0.89,
1,314×0.88; the battery/conventional split and the battery-favourable
shortfall attribution are documented inline as choices); envelope timings
from Grid Code/ESO timeline (delay 2/ramp 8, secondary 10/20, sustain 30
per CC.6.3.7; rundown 10 s is a stated un-gated convention); LFDD stage 1
= observed 931 MW, delay 0.3 s = centre of the published 0.2–0.5 s;
inertia = both official bounds; H constants = cited literature file,
drift-guarded by `grid-core/tests/inertia_defaults.rs`; event spec
cross-checked against the reference record by
`event_spec_matches_the_committed_reference_record`. Damping 1.836 %/Hz:
arithmetic re-verified ((1,481 − 1,055) MW / 0.8 Hz = 532.5 MW/Hz =
1.836 % of 29 GW), anchored to the 49.2 plateau **which is not a gated
anchor** (T2 gates the first arrest, measured 49.083; the 49.2→48.8
descent is declared diagnostic), and sits inside the literature span
1–2.5 %/Hz. Note for the run report: the plateau value coincides
numerically with T2's upper edge, and higher damping does push the
arrest upward — the derivation is legitimate and documented, but this
coupling should be stated (it is part of why ruling (a)'s edge is live).

**(c) The tag question — COMMIT NOW; DO NOT TAG `stage-6-validated`
until part 2 lands.** The hard rule ("a stage is complete only when its
acceptance tests pass") states a necessary condition, not a sufficient
one. All docs/04 Stage 6 acceptance tests are green — but the docs/04
section is the work order, and its scope line includes the Q8 pathway
runner (delivered as an explicit exit-2 stub) and its demo artefacts
include the Module 6 chart (hours/year below threshold **vs. renewable
share** — only single-scenario first-cut counts exist) and the Q8
largest-survivable-loss chart (not started). Every prior tag (Stages
1–4) shipped its demo artefacts and run report before tagging, and the
Stage 3 precedent is exactly this shape: part 1 committed after review,
tag withheld until the full work order closed. Tag when part 2 (pathway
runner + Module 6 chart) passes review.

**(d) Schema v3 migration discipline — PASS, one pending doc.** Probe-
before-parse extended (v1 and v2 each get their own structured migration
error); v2 refusal verified end-to-end (exit 2, message actionable,
names the one-line migration); v2 fixture frozen verbatim and pinned by
`v2_scenario_fails_with_a_migration_message_naming_what_was_added`;
optional fields ⇒ trivial migration; derived defaults drift-guarded
against the cited evidence file; MVA = GW/0.9 as a single conversion
point (`Power::apparent`) with a `compile_fail` doc-test blocking
`InertiaConstant × Power`; overrides surfaced in outputs (same pattern
as `reliability`); proptest round-trip extended to the new fields.
PENDING (commit-blocking condition 1): `docs/03-domain-model.md` carries
the v1→v2 migration note but **no v2→v3 note** — the error message and
the scenario module docs both point readers at docs/03 for it. Apply the
implementer's proposed note (handoff ledger) in the same commit.

**(e) Module 6 caveat prominence — VERIFIED in test and artefact.** The
unconstrained-dispatch caveat (no must-run, no min-stable generation, no
NESO stability actions; real GB held ~110–350 GVA·s because NESO pays
for it; "do not quote as 'GB was below its floor 85 % of 2024'") is
pinned three ways: prose block in `inertia_sum.rs` above the pinned
constants, the `UNCONSTRAINED` block written into every `report.toml`
(asserted by the CLI test, present in the committed
`runs/stage6-inertia-2024` and `-rs-lean` artefacts), and the CLI
console line. The RS zero-inertia finding is pinned at mechanism level
(`all_variable_fleet_has_zero_synchronous_inertia`) plus
`has_synchronous_provision` and the FINDING line; the reviewer
reproduced the full 701,280-period zero series. The 120/102 GVA·s floors
are cited (`inertia-constants.toml` FRCR 2024 p.10). The run report must
carry the caveat at KC-4 prominence and quote the 701,280 figure against
the mechanism test + artefact digest.

## Standard duties

- **No library panics**: grep of grid-stability src + grid-core/inertia.rs
  clean; indexing paths bounds-safe (interpolate handles both ends;
  non-empty trace guaranteed by timestep ≤ duration validation).
- **Newtypes**: no raw f64 for physical quantities across public APIs;
  new units (InertiaConstant, ApparentPower, Frequency, Rocof, Damping)
  with physically meaningful arithmetic only; conversions at single
  defined points.
- **Determinism**: fixed-step integrator, no adaptivity, no wall-clock
  (CLI `now_utc` only), BTreeMap ordering, bit-identical repeat runs
  (CLI determinism test; digests reproduced here).
- **Dependencies**: no new external crates — Cargo.lock gains only
  workspace edges (grid-stability → grid-core/grid-adequacy/serde/toml,
  justified inline in Cargo.toml).
- **Outputs**: docs/06 metadata block on CSV/Parquet/report; PNG footer
  carries engine hash + spec hash (metadata-chunk gap is the
  pre-existing tracked deviation, unchanged).
- **Data deliverables**: NESO CSV fixture carries source URL, retrieval
  date, both sha256s (matching the reference TOML), NESO Open Data
  Licence attribution, and the transformation description. Event spec
  carries per-number citations and is drift-guarded by test.
- **TDD**: single-tree delivery per the recorded Stage 0 decision
  (gate = acceptance tests + review, not commit archaeology); test
  quality consistent with test-first (structured-error assertions,
  designed-red gates, mechanism-level property tests).
- **Scope**: grid-adequacy/test and scenario changes are mechanical v3
  migration; no docs/ edits by the implementer (correctly left to the
  supervisor); no unrequested features beyond the permitted stub.

## Conditions (all actionable by the supervisor, none require code)

1. **docs/03 v2→v3 migration note** in the same commit as the package
   (the committed error message references it; docs/05 rule 4).
2. **Stage 6 run report** must document, at KC-4 prominence: (i) the T2
   top-of-band pass + mechanism + the three corroborating un-gated
   diagnostics (arrest timing 13.8 vs ~25 s; mid-phase over-recovery
   49.63 vs 49.2 plateau at t=30 s; LFDD ~3 s early) — the over-recovery
   is currently documented NOWHERE in the tree (the handoff's
   "documented" claim is wrong; the event TOML covers only the post-nadir
   slower-recovery exclusion); (ii) the damping↔T2 coupling note from
   ruling (b); (iii) the Module 6 caveat and the do-not-quote framing;
   (iv) the no-retuning warning at the 49.20 edge.
3. **docs/06 amendment proposal** (subcommand list gains `stability`) —
   record in project-state per CLAUDE.md, do not edit the ADR/conventions
   silently. Note `stability` is also absent from the docs/06 exit-code
   sentence's subcommand list.
4. **Tag discipline**: commit as Stage 6 part 1; `stage-6-validated`
   only after the Q8 pathway runner + Module 6 chart pass review
   (ruling c).
5. Regenerate `runs/` artefacts post-commit for clean hashes (existing
   precedent; current artefacts embed a dirty-tree engine hash).

Reviewer reproduction environment: macOS, workspace at
`[local path]`, uncommitted tree as delivered
(dispatch digest pin re-run green before and during review).
