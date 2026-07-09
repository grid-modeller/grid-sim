# Q5/Q11 heating-engine package — reviewer adjudication

Reviewer, 2026-07-03. Subject: the UNCOMMITTED heating-engine package
(grid-core/src/heating.rs, the schema-v5 hunks of scenario.rs /
error.rs / trace.rs / units.rs / lib.rs, grid-adequacy inputs/sweep/
import-convention hunks, grid-cli run.rs heating outputs, the five
new test files, the v4 fixture, the migrated scenario files, and the
docs/03 v4→v5 migration note). The concurrent Stage 7 cost package
(costs.rs / costs_reference.rs / cost hunks in the shared files /
costs-gb.toml) is OUT OF SCOPE here and under its own review.

Acceptance contract: docs/notes/d9-heating-overlay.md (ADOPTED; rules
1–5, 6b outputs, the rule-2 migration paragraph), its adjudication
(rulings A–F), the reviewed data package (q5-heating-data-report.md +
data/reference/heating-cop.toml, 61ecfd9), docs/05, docs/06.

## VERDICT: ACCEPT-WITH-NOTES

Every hard law holds and every load-bearing number reproduces under
independent recomputation — including the pre-migration 5-zone digest
provenance, which I re-established from source rather than trusting
the test's comment. Gates: `cargo fmt --check` clean; `cargo clippy
--workspace --all-targets -D warnings` clean; full test suite green
(the four grid-cli/tests/cli.rs failures in the first pass were the
warned concurrent-suite scratch race — NotFound on shared temp paths
while the cost review's suite ran; all four pass in isolation, exit
0 for grid-core, grid-adequacy, grid-cli, grid-stability). No new
dependencies (no Cargo.toml touched). Five conditions, none requiring
redesign, to be actioned before/at landing.

## Independent reproductions (verify, don't trust)

**1. Digest protection (D9 rule 5 test 2 — the hard law).**
- `regression_2024.rs::pinned_2024_reference_result_digest` run by me
  on the MIGRATED v5 gb-2024-reference.toml: PASS — `779d7444…`
  unmoved.
- 5-zone provenance: I built the PRE-migration engine in a worktree at
  HEAD (61ecfd9) and ran the PRE-migration v4 gb-2024-5zone.toml
  against the live packs. All six per-zone digests match the
  regression_5zone.rs pins exactly (GB `c783b306…`, FR `91191dc8…`,
  CONT-NW `e5f37606…`, NO2 `fba1fb7c…`, DK1 `00065d89…`, IE-SEM
  `1956cd89…`) and the links digest matches `371aa257…`. The pinned
  values therefore genuinely predate the migration — proven, not
  claimed — and `regression_5zone.rs` passes on the migrated file.
  The provenance condition contemplated in the work order is
  DISCHARGED.
- The frozen fixture `grid-core/tests/fixtures/v4-gb-2024-reference.
  toml` is sha256-identical to `git show HEAD:scenarios/
  gb-2024-reference.toml` (`b72ef0d4…` both).

**2. Overlay physics (D9 rules 3–4), recomputed from the pinned trace
with an independent implementation (numpy, not the engine's code
path):**
- mean annual degree-hours 50,454.1235950804 °C·h; 2024 = 43,706.8
  (ratio 1.15438 — the data package's ×1.1544);
- k = 410.5e3 × 0.5 × (1 − 0.170) / 50,454.12 =
  **3.376483186333871 GW/K** — the claimed value to the last digit;
- annual-harmonic fit: mean 10.209 °C, amplitude 6.036 °C;
  Kusuda–Achenbach at z = 1.0 m, α = 8.7e-7: damping **0.713003**,
  lag **19.664 d** — the data package's 0.7130 / 19.66;
- implied SPFH2 (corrected, pre-derating, rule-3 weighting, GSHP on
  the wave − 5 K): ASHP **3.2215**, GSHP **3.8393**; to-median
  deratings **0.8226 / 0.7319** — the pinned 0.823 / 0.732;
- post-derating record max COP: ASHP 3.388, GSHP **4.611**; district
  premise 15.0 > 4.611 (3.25×), band bottom 12.0 > 4.611 (2.60×) —
  machine-checked at runtime in `compute_overlay` AND asserted in
  `district_lowest_limb_holds_with_the_premise_machine_checked`;
- DHW flat rate, ΔT floor 15 K, °C column convention documented at
  the single loading point (`trace.rs::load_temperature_trace_c`):
  all verified in source;
- horizon-subsetting bit-identity: the test
  (`horizon_subsetting_never_changes_the_overlay`) genuinely proves
  purity — it compares the 2024-only overlay per-period `==` against
  the 2024 slice of the full-record overlay, constants included, and
  the implementation computes k/wave from the trace's FULL record
  regardless of horizon;
- the edit-6 check is a REPRODUCTION, not a re-derivation: the pinned
  factors live in the committed reference file, the test recomputes
  the implied SPFs and asserts they REPRODUCE 3.221/3.838 and the
  file's factors within 5e-4 — drift in either direction fails.

**3. Characterisation pin (rule 5 test 4)** — reproduced by running
the suite myself (grid-adequacy/tests/heating.rs, PASS, 47.8 s):
baseline peak residual **92.23871490574456 GW** → heated
**113.4466987983204 GW** (+21.208 GW); at the committed 100 GW rating
the heated solve is **SolveInfeasible** (pinned as the first
finding); at 200 GW both endpoints: **23,872 → 40,224 GWh** (+16,352,
×1.685), with the 200 GW baseline asserted equal to the Stage 3
100 GW pin (the rating never bound the baseline).

**4. Ordering results.** District-lowest is gated on the
machine-checked premise (both in-test and at computation time — an
override below the heat-pump record max is a structured error naming
the premise; tested). ASHP-vs-GSHP: my independent recomputation gives
ASHP peak **47.72526272299193 GW** (pin-exact) and GSHP
**44.927558642468945 GW** (pin 44.927558642468775; Δ ≈ 2e-13,
summation-order float noise in my lstsq — same number). The
inversion-surfacing question: an inversion CANNOT silently pass — the
test carries a separate directional assert after the pins with the
full-prominence message; a re-pin that inverted the direction fails
that assert loudly and forces a visible edit to the recorded finding.

**5. Schema v5 discipline** — all verified by test and by reading the
hunks: v4-with-block fails with `SchemaVersion4Superseded` naming the
replacement fields and the version-line-only path; the fixture is
byte-frozen (above); share-sum `HeatingShareSum` names sum and
entries at 1e-9; `deny_unknown_fields` on spec and entries; duplicate
kinds rejected; override placement enforced (cop_const district-only,
curve/correction/derating heat-pump-only); v1/v2/v3 fixture tests
updated and green; docs/03 migration note follows the v3→v4
precedent.

**6. Outputs (rule 6b/ADR-5)** — heating.{csv,parquet} (both, always),
per-entry series keyed by kind, delivered heat, `[results.heating]`
echoing k, DHW rate, degree-hours, record window, ground damping/lag,
per-entry effective parameters with `overridden` flags, per-year
delivered-heat totals (the spread reported, never renormalised);
heating digest in summary and console; the t2m trace AND
heating-cop.toml enter `scenario_data_files` so their sha256s ride in
every output header. Sweep: heating is subtracted before
`annual_scale` and added back (`scale_demand`), unit-tested, with the
no-overlay path bit-identical to pre-v5. No special-casing in
residual/decomposition — heating is inside demand; attribution's
synthetic inputs carry `heating: None`.

**7. Conventions** — no panics in library paths (the two
`unwrap_or` fallbacks in the COP path are structurally infallible and
documented); no unsafe; determinism (no wall-clock/globals; constants
from the trace record). Newtypes: Temperature/Diffusivity/
HeatIntensity/Length::metres added with dimensional Mul impls — two
residual ADR-4 breaches in condition 2.

## Rulings requested

**The 200 GW store-power convention: LEGITIMATE, D8-rule-5-like —
with quoting duties.** (a) Both endpoints are solved by the same
stated solver at the same stated rating; (b) the raise is inert on
the baseline — asserted in-test equal to the Stage 3 pin, so the
convention changes nothing it did not have to change; (c) at 200 GW
the rating sits far above the heated peak residual (113.45 GW), so
energy binds at both endpoints — restoring the committed scenario's
own stated design ("the rating chosen so energy binds"), which the
overlay had broken; (d) the 100 GW infeasibility is pinned FIRST, at
full prominence, so the convention smuggles nothing — it is forced by
a finding that is itself the headline (electrified heat pushes the
peak residual through the committed discharge rating). Duty: any
quoted requirement or delta from this pin carries "at 200 GW store
power, both endpoints" next to the number, and the 100 GW
infeasibility finding travels with the headline wherever the ×1.69 is
quoted.

**The implementer's rating flag — what the record needs.** The RS
scenario headers' "energy-binding by design" claim is FALSE under
heating overlays at the committed 100 GW. project-state (and the Q5
analysis-runs work order when briefed) must carry: every heating run
on an RS-fleet scenario states its store rating next to every storage
number; SolveInfeasible at a committed rating is a reportable result,
never silently bumped past; rating raises are applied to both
endpoints of any differenced pair and stated.

**The reference-path-as-constant: ACCEPTED; correct the precedent
claim.** The claimed mirror of prices-2024.toml is FALSE — the prices
reference path is a scenario field (`PricingSpec.reference`,
scenario.rs), not a constant; the inertia precedent is different
again (values transcribed into code, file as drift-guarded evidence).
The constant path is nonetheless RIGHT on the adopted contract's own
terms: D9 rule 2's field list is normative and carries no COP path;
rule 4 orders "reference file, NOT hard-coded, NOT free scenario
text"; the trace (fetched-and-built data) IS in the scenario per
edit 3, while the COP file is a committed engine input inside the
engine git hash — determinism (`results = f(scenario, pack checksum,
engine hash)`) holds, and the file's sha256 is embedded in run
outputs. Per-entry overrides remain the scenario-side control and are
echoed. Correct the record wherever the prices-mirror claim appears;
do not add a scenario field.

## Conditions (numbered; action before/at landing)

1. **Reference-file schema string.** `heating-cop.toml` carries no
   `schema = "heating-cop-v1"` and the parser probes none — BOTH
   cited precedents carry one (inertia-constants.toml `schema =
   "inertia-constants-v1"`; prices-2024.toml `schema =
   "prices-reference-v1"`), and the concurrent cost package's docs/03
   "Committed reference files" registry declares the schema string
   mandatory. Add the string to the file (one line; a knowing edit to
   a reviewed file, recorded), probe it before the full parse, extend
   the drift-guard pins, register heating-cop.toml in the docs/03
   registry (coordinate with the cost package's landing of that
   section), and flip `[meta] status = "draft"` citing this review.
2. **ADR-4 breaches.** `GroundWave::at(hours_from_record_start: f64)`
   takes a time quantity as raw f64 across a public API (`Duration`
   exists), and `HeatingConstants.mean_annual_degree_hours_c_h: f64`
   is a dimensional quantity (Temperature × Duration) as a raw pub
   field — docs/06: raw f64 for physical quantities does not cross a
   public API boundary. Fix `at` to take `Duration`; type or
   encapsulate the degree-hours scalar (unit-named accessor or
   newtype). No silent ADR exception.
3. **Override COP positivity.** `validate_heating` checks override
   curve coefficients only for finiteness; a legal scenario override
   (e.g. `cop_curve = [1.0, -0.1, 0.0]`) yields a negative COP at
   large ΔT and hence NEGATIVE electrical demand, silently. The
   committed reference curves are positive-definite (verified: minima
   2.03/1.10 pre-factor), so only overrides are exposed — but the
   hole is real. `compute_overlay` must raise a structured error if
   any evaluated period's effective COP is ≤ 0 (or validate the
   quadratic's minimum over the floored ΔT domain).
4. **Multi-zone heating output path untested.** The
   `execute_multi` heating branch (per-zone `heating_<zone>.{csv,
   parquet}` + the `[results.heating` → `[results.heating_<zone>`
   section-rename string surgery in grid-cli/src/run.rs) is exercised
   by no test — "no untested code reaches trunk" (CLAUDE.md; docs/06).
   Add a multi-zone heated-run output test (e.g. the 5-zone scenario
   with a GB heating block) or defer the branch until the Q5 runs
   need it.
5. **TDD evidence at landing.** The package arrives as one uncommitted
   tree, so red-first cannot be evidenced by commit order
   (checklist 4). Land as a commit sequence with the acceptance tests
   preceding the implementation they gate (the Stage precedents), or
   record the red-run evidence explicitly in the landing record.

## Notes of record (non-blocking)

- **Shared-file coupling with the cost package**: heating depends on
  `Length` (declared in the cost hunk of units.rs) and both packages
  interleave in error.rs/lib.rs/docs/03. Whichever package lands
  second rebases; if the cost package were returned, `Length` +
  `metres` conversions must ride with heating.
- The cli.rs first-pass failures were the known concurrent-suite
  scratch race; all pass in isolation. Not a defect of this package,
  but the shared `std::env::temp_dir()` paths in the CLI tests remain
  race-prone under parallel suites — a standing hygiene item, not a
  condition.
- The dyadic-TWh proptest strategy note (energy_twh round-trip) is
  sound and honestly documented.
- Scope: clean. Scenario edits are exactly the ordered same-commit
  migration; docs/03 heating hunks are the ordered migration note;
  no unauthorised doc or pin edits found in the heating hunks.
