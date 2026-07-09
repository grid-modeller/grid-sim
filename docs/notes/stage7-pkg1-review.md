# Stage 7 package 1 review — costs-reference parser + D8 cost stack

**Reviewer verdict (2026-07-04): ACCEPT-WITH-NOTES** — three conditions,
all actionable before commit; no defect requires redesign.

**Scope reviewed:** `grid-core/src/costs_reference.rs`, `grid-core/src/costs.rs`,
`grid-adequacy/src/costs.rs`, `grid-core/tests/costs_reference.rs`,
`grid-adequacy/tests/cost_stack.rs`, cost hunks in
`grid-core/src/{units,error,lib}.rs` and `grid-adequacy/src/lib.rs`, the
costs-gb.toml DRAFT lift, the docs/03 costs-reference-v1 note. The
concurrent Q5 heating package's hunks (heating.rs, scenario/trace,
heating tests, scenario TOMLs, heating hunks in shared files) were
scoped OUT per the work order; reviewed separately
(docs/notes/q5-heating-engine-review.md).

## Gates (run by the reviewer, isolated tree)

- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p grid-core --test costs_reference`: 26/26.
- `cargo test -p grid-adequacy --test cost_stack`: 15/15.
- `grid_core::costs` unit tests: 6/6.
- Full workspace: 46 suites, all ok, 0 failures (no scratch-dir race —
  suite had the tree to itself).

## Independent recomputations (reviewer's own arithmetic)

CRF(0.075, 25) = **0.089710671649444018** — matches the pinned
0.08971067164944402 exactly (last-digit display rounding only).

Annuities, central capex incl. site infrastructure × 1000 × CRF, £/MW/yr:

| Row | Reviewer recomputation | Package pin | Match |
|---|---|---|---|
| CCGT (1034.6 £/kW, 25 y, 4.5 %) | 69,772.418409 | 69,772.418409 | exact |
| CCGT (7.5 %) | 92,814.660889 | 92,814.660889 | exact |
| CCGT (10.0 %) | 113,979.887488 | 113,979.887488 | exact |
| Nuclear (5824.7 £/kW, 60 y, 7.5 %) | 442,627.210994 | 442,627.21 | exact |
| Offshore (2670 £/kW, 30 y, 7.5 %) | 226,072.199500 | 226,072.20 | exact |
| Battery power leg (262 £/kW, 15 y, 7.5 %) | 29,681.255899 £/MW/yr | — (recomputed in test) | exact |
| Battery energy leg (135 £/kWh, 15 y, 7.5 %) | 15.293777 £/kWh/yr | — (recomputed in test) | exact |

## Adversarial findings, per the brief

**1. Parser strictness.** Strict throughout (`deny_unknown_fields`,
schema probe first, structured `InvalidCostsReference` errors naming
table+field). All governance fields parsed and load-bearing:
quotable/verified, bracket_rule (mandatory on nuclear AND
nuclear_observed), publication_gate, binding_convention,
staleness_stamp, phasing sum ±0.025. Ten reviewer-constructed bad
TOMLs all rejected with correctly-named errors: battery
quotable=true; quarantined interconnector given a point capex;
nuclear without bracket_rule; project-total capex on a component
row; capex in both spellings; zero phasing fraction; hurdle rate
> 1; zero life_years; reversed holding percentiles; phasing sum
1.06. **Gap → condition 1:** the package's own test suite does NOT
cover the two most load-bearing rejections (battery quotable=true
lift; quarantined interconnector exposing a point capex) — the
guards exist and work (reviewer probes), but they are untested code
(CLAUDE.md: no untested code reaches trunk; the docs/04 pin makes
these flags load-bearing).

**2. Annualisation (D8 rule 4).** CRF formula exact (table above).
`WaccBand<T>` carries every money output of `CostStack` (all seven
fields incl. the constraint line's value and the headline);
operating lines are flat across the band but still banded —
verified by test and by reading. `grid_core::costs::annuity_per_mw`
and `TechnologyCosts::annuity` are single-rate building blocks
(necessarily — `WaccBand::try_map` calls them); no stack-level
output escapes the band. Overnight-basis limitation stamped in
module docs AND in every stack's `metadata.limitations` (tested).

**3. Cost stack (D8 rule 1).** Lines match the pinned list 1–6 with
rule-1.7 honoured: unserved energy appears ONLY in the denominator
subtraction and the rule-3 stamp — no VoLL anywhere. Gas
fuel+carbon: **same code path confirmed** — `cost_stack` consumes
`PricingInputs.srmc`, the identical `Trace<Price>` the Stage 2
builder produces via `grid_core::pricing::srmc_series`
(fuel/η + EF·carbon; VOM is NOT in the SRMC recipe, so the stack's
separate VOM adder is not a double count). Constraint costs: named
zero, `pending_d6: true`, never pooled (tested). Denominator:
`delivered_to_demand_energy = result.total_demand_energy() −
result.total_unserved()` — genuinely demand-series-based, distinct
name from Package A's `energy_delivered` per-tech series, no
conflation found. Leap-year accrual: `horizon_years` counts
periods/17,568 vs 17,520 per calendar year; the reconciliation test
runs on 48 periods of 2024 with YEARS = 48/17,568, so a 8,760-hour
approximation would fail at 1e-9 — exercised, not just claimed.

**4. Rule-2 reconciliation — RULING: genuinely non-tautological.**
The test recomputes all six lines from hard-coded reference numbers
with a test-local CRF, then asserts (a) total == bitwise sum of
reported lines and (b) every line == independent recomputation at
1e-9 relative, at all three WACCs. Reviewer mutation checks:
(A) dropping the stability line from the library's total → fails
assertion (a) (cost_stack.rs:242); (B) doubling the VOM adder
(wrong line, self-consistent total) → fails assertion (b)
(cost_stack.rs:199, "got £9412800, independently recomputed
£8306400"). Neither a shared-accumulator bug nor a
wrong-but-consistent line can pass. Meets the docs/04 pin and the
D8 adjudication note verbatim.

**5. Quarantine (propagate-then-refuse).** Stack level: both halves
tested — consuming the battery row is legal, stamps non-quotable,
lists `storage.battery_li_ion`, carries the staleness stamp, and
`ensure_publishable` refuses with structured `NonQuotableResult`;
a battery-free stack is publishable. OCHT publication gate:
structurally unreachable from the stack (the row lives in
`hydrogen_reconversion_ocht`, not the `technologies` map the spec
draws from; `publication_gates` in `cost_stack` is an empty vec by
construction) — the gate-refusal half is tested at the metadata
level, which is the honest available seam this package. Nuclear
bracket_rule surfaces in metadata without blocking (tested).
Quarantined interconnectors: no consumable figure exists in the
parsed struct (capex: Option is None), and the stack errors naming
the row (tested). Parser level: see condition 1.

**6. Owed actions (cost-inputs review condition 12).** DRAFT lift
verified against 1fe5df5: 12 insertions / 7 deletions, ALL in the
header comment block plus the schema string
(`costs-reference-v1-DRAFT` → `costs-reference-v1`); zero number
changes. docs/03 gains a "Committed reference files" registry
following the prices-reference-v1 precedent, with the
costs-reference-v1 semantics enumerated. Pinned regressions present
(WACC set, CCGT row, gas central 2030, battery flag, CCGT annuity
band).

**7. Conventions.** No panics in library code (test modules carry
the standard allow). No new dependencies (no Cargo.toml changes).
Units single-conversion-points verified: kW→MW ×10³ in
`annuity_per_mw`; GW↔kW/MW and GWh↔kWh factors each defined once in
the units.rs Mul impls; £/yr→£ only via `MoneyRate::over_years`.
No cost content leaked into the heating package's files (checked
sweep/attribution/import_convention/inputs/cli/inertia diffs).
Rule-3 reliability stamp (unserved + standard) on every stack,
tested. ADR-5 hash embedding is not in this package — the artefact
layer is later; noted below.

## Rulings requested by the work order

**Phasing tolerance ±0.025 — ACCEPTED, with a note.** Justified as a
parse-time sanity gate: the as-published shortfalls are 0.98
(onshore predev) and 0.99 (biomass predev); 2 dp rounding over ≤ 6
entries bounds legitimate error at 0.03, and 0.025 sits just above
the worst observed case while rejecting genuinely broken arrays
(my 1.06 probe rejected). BUT it is weaker than enumerating the two
as-published exceptions: my probe shows a silently edited biomass
predev summing to 1.02 parses fine, and no pinned regression covers
any phasing array except CCGT's — so the file header's "pinned
regression tests fail on silent edits" overstates for phasing.
Bounded risk: the arrays are currently inert (IDC escalation out of
scope). Non-blocking note 5: enumerate the exceptions or pin all
phasing arrays BEFORE the escalation package consumes them.

**Reconciliation independence — GENUINE** (finding 4 above; two
mutations, both caught, one per anti-tautology failure mode).

## Conditions (before this package commits)

1. **Add the two missing parser red tests**: (i) battery
   `quotable = true` is rejected at parse ("quarantine cannot lift
   silently"); (ii) a quarantined interconnector row carrying a
   point `capex_gbp_bn` is rejected at parse (review condition 9).
   Both guards exist and work (reviewer-verified by constructed
   TOMLs) but are untested — CLAUDE.md hard rule, docs/04 Stage 7
   pin. Suggested location: `grid-core/tests/costs_reference.rs`
   error-case section.
2. **TDD evidence at landing**: the package is uncommitted, so
   commit order cannot be audited now. Landing commits must show
   the acceptance tests with (or before) the implementation,
   referencing the stage per docs/06 — the reviewer will check the
   pushed history.
3. **Landing-order coupling (coordination)**: the heating package's
   `grid-core/src/heating.rs` consumes the `Length` newtype declared
   in THIS package's units.rs cost hunk (heating.rs:91,170,479). This
   package's units.rs hunk must land before or with the heating
   package; the two packages are not independently revertible at the
   units.rs file level.

## Non-blocking notes

4. The docs/03 "Committed reference files" registry this package
   created is generic and accommodates the heating review's ordered
   `heating-cop-v1` entry — the heating package should add its entry
   to this same section, not a parallel one.
5. Phasing-array pinning before the IDC escalation package (ruling
   above).
6. ADR-5 hash embedding (engine/scenario/data-pack) on cost
   artefacts is deferred with the artefact/CLI layer — required
   before any Stage 7 number is published; `ensure_publishable` is
   the natural seam.
7. Line 5 prices holdings at the central volume-weighted mean only;
   the parsed p5–p95 range is available for a labelled sensitivity
   when Q8 needs it.

Reviewer: Stage 7 gate, 2026-07-04. Verified, not trusted: all
gates, recomputations, mutations and probes above were run by the
reviewer on the working tree.
