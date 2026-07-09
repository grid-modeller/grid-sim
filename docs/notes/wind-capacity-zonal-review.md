# Review — `sweep wind-capacity-zonal` package (branch `sweep-zonal-cli`)

Reviewer gate, 2026-07-07. Branch: `sweep-zonal-cli` (3 commits, base
`6ec7a44`), worktree `.claude/worktrees/agent-a1b8e1fbdf5c3466d`. Work
order: `docs/notes/wind-capacity-zonal-work-order.md`. Context: package
implemented under a disputed authorization, since RATIFIED by Richard
(2026-07-07) conditional on this gate. Judged on merits.

## Verdict: ACCEPT-WITH-CONDITIONS

Conditions (both procedural; no code defects found):

1. **Full workspace gate must complete green before merge.** My runs
   were targeted: the 3 new tests (`wind_capacity_zonal.rs`, all green,
   2.66 s), the decisive pin check (`regression_3zone`, both tests
   green — the scenario edit moves no dispatch digest), `cargo fmt
   --check` (clean), `cargo clippy -p grid-cli --all-targets -D
   warnings` (clean). The supervisor's in-flight full-suite + fmt +
   workspace-clippy run is the remaining gate; merge only on its green.
2. **Merge ledger entry supersedes the branch's `memory/project-state.md`
   note** (commit `a7a6a51`), per the supervisor's instruction. Its
   "full workspace tests green" claim is the item condition 1 verifies.

## What was verified (verify-don't-trust record)

**Acceptance tests (ran myself, worktree):**
- `wind_capacity_zonal.rs` — 3/3 pass: step-0 priced-gate test, NSCO
  single-zone sweep (CSV+PNG, exact column header, monotone-or-nearly
  potential-basis decline, per-row delivered>=potential direction lock,
  pinned 40 GW row), NSCO+SSCO group sweep.
- `regression_3zone` — 2/2 pass: the pre-existing pinned per-zone
  dispatch digests are UNMOVED, confirming the scenario header's claim
  that adding `[zones.pricing]` with `flow_signal = "scarcity"` leaves
  dispatch byte-identical.
- No committed file on master references the old scenario sha256
  (`9fd490a8…`), so the scenario edit strands no committed digest or
  run-report citation.

**Scenario pricing numbers (`scenarios/gb-2024-3zone.toml`):**
- Recipes are named-not-transcribed as claimed: `efficiency = "ccgt"` /
  `"ocgt"` name the `[efficiency.*]` keys of
  `data/reference/prices-2024.toml` (0.4893 / 0.349 HHV confirmed
  there); the gas trace path/column matches `gb-2024-reference.toml`
  `[pricing]` exactly. Citation comments mirror the reference file's
  per-line style.
- Recipe coverage matches the fleets: NSCO has CCGT (Peterhead) + hydro
  only → ccgt recipe only, correct; RGB has CCGT + OCGT → both recipes;
  SSCO has nuclear + hydro, no gas → block present (the schema-v7
  every-zone gate) with NO recipe, pricing at the £0 must-take floor —
  documented in the file as the cannibalisation signal, and guarded by
  the step-0 test (`pricing("SSCO").srmc.is_empty()`).

**CLI code quality:**
- `grid-cli/src/sweep.rs` — `wind_capacity_zonal()` wraps
  `wind_capacity_sweep_multi` / `_multi_group` unchanged; error style is
  `Result<_, String>` throughout, matching the sibling `wind_capacity()`
  exactly (same range validation, same trace-load-once convention, same
  provenance header machinery via `crate::run::{sha256_file,
  scenario_data_files}` and `env!("GRID_ENGINE_GIT_HASH")`). No panics,
  no unwraps outside tests, no wall-clock in the artefact header.
- `grid-cli/src/run.rs:145-161` — `scenario_data_files` now also hashes
  per-zone pricing inputs and link capability traces. Correct ADR-5 gap
  fix: these files ARE read by `load_multi_zone_inputs`. Additive;
  BTreeMap keying dedups the shared `prices-2024.toml`. No test pins an
  exact data-file list, so nothing moves.
- Scope: diff touches ONLY the 5 stated files. NO grid-core or
  grid-adequacy source change; no Cargo.toml change (no new deps).

**Assumption blocks (work-order outputs spec):** all three are emitted
by the code and I inspected the actual artefact produced by the test
run:
1. The 3-zone honesty conventions — checked word-for-word against the
   scenario header (obligation (2), lines 26–32, and the B4
   anti-conservative direction-sign paragraph, lines 55–60): verbatim
   (one line-break reflow only).
2. The £0-SMP-floor guard, verbatim as specified.
3. Capture-ratio definitions — consistent with the library's
   `MultiZoneWindPoint` docs (potential = pooled-curtailment
   convention; delivered = pro-rata post-curtailment). The "delivered
   sits at or above potential" claim follows from curtailment occurring
   only in £0-SMP periods (identical revenue, smaller energy
   denominator) and is additionally asserted per-row in the test.
   Blank-cell-never-NaN honoured (`ratio_cell`).

**Pinned values:** measured-then-pinned per house style (commit body
records the measurement date and the full trajectory; pin tolerance
relative 1e-9 with a re-pin-requires-record failure message). The
delivered-basis NON-monotonicity is honestly characterised, not hidden:
the ledger's 0.393 → 0.383 → 0.429 matches the artefact
(0.39280… → 0.38283… → 0.42892…), the test comment explains the
mechanism (pro-rata removal concentrates surviving energy in priced
periods at extreme curtailment: 80.8 of ~119 TWh potential curtailed at
40 GW), and the test deliberately asserts the direction lock instead of
monotonicity on that basis. The potential-basis collapse
0.2356 → 0.1338 → 0.1010 is the monotone published-convention signal.

**TDD evidence:** commit `e023476` carries the step-0 gate test with
the TOML and records RED (priced-ladder refusal, exit 2) before GREEN;
commit `89eb9e0` records both CLI tests failing on the unrecognised
subcommand before implementation. Commit order sound.

**ADR/conventions:** no schema change (schema v7 `[zones.pricing]` is
pre-existing), so no docs/03 bump required; units newtypes at the CLI
boundary (`Power::gigawatts`, `Energy`, `Price`); determinism preserved
(parallel==serial covered by existing library tests per work order;
artefacts embed engine hash + scenario sha + per-file data shas).

## Non-blocking observations

- The work order sketched `# assumption:` lines; the code emits
  `# assumption 1 (...)`/`2`/`3` numbered blocks. Content matches the
  spec's three required items in full — cosmetic deviation only.
- `x_max` in `render_zonal_chart` defaults to 1.0 on an empty points
  slice; unreachable in practice (the range validation guarantees at
  least one capacity point). No action needed.
