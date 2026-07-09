# Package A review — delivered-basis revenue/capture accounting

Reviewer verdict, 2026-07-03. Scope: the uncommitted working-tree
package (grid-adequacy/src/pricing.rs, grid-adequacy/src/lib.rs,
grid-cli/src/run.rs, grid-cli/src/sweep.rs,
grid-adequacy/tests/pricing_delivered.rs,
grid-cli/tests/regression_delivered_2024.rs, grid-cli/tests/cli.rs).
Work order: the ratified tracked-deviation fix (Richard, 2026-07-03) —
delivered-basis revenue/capture accounting ADDED alongside the
potential-basis convention, old columns retained, no pin moves.

## VERDICT: ACCEPT-WITH-NOTES

Conditions 1–2 are the implementer's, cheap, and should land with the
package. Condition 3 is the supervisor's record correction (ruling in
§4, apply verbatim). Nothing here requires rework of the delivered-basis
machinery itself, which is correct and was verified independently down
to full precision.

### Conditions

1. **Strengthen the CLI-level cross-basis lock (implementer).** No
   automated test currently fails if the CLI wiring silently substitutes
   the potential series for the delivered one (a no-op
   `total_wind_delivered`): the 2024 regression asserts only
   `delivered >= potential` (equality passes), the near-identity band
   `< 1e-4` passes, the ±0.0005 pin passes, and the sweep test's
   `delivered >= potential` passes. The library tests do lock the
   divergence, but the wiring does not. Fix either or both:
   (a) in `cli.rs::sweep_wind_capacity_writes_module1_table_and_chart`,
   assert `delivered > potential` STRICTLY at the 40 GW point
   (curtailment 0.8 TWh there — divergence is far above float noise);
   (b) tighten `PINNED_WIND_CAPTURE_RATIO_DELIVERED` to the measured
   0.9413419206 with ±1e-7 (the engine is bit-deterministic; the current
   ±0.0005 tolerance is ~400× the 1.2e-6 effect the pin exists to lock).
2. **Chart y-axis honesty (implementer).** The Module 1 chart's
   `y_desc` still reads "% of half-hourly periods", but the two new
   capture-ratio series are dimensionless ratios ×100, not period
   percentages. The legend labels are honest ("× 100", basis named);
   the axis description is now wrong for half the series. Fix the axis
   text (e.g. "% of periods / capture ratio × 100").
3. **Record corrections (supervisor)** — ruling text in §4 below; the
   tracked-deviation direction claim, the frozen-imports parenthetical,
   and Stage 2 run report §5 are all stale against the verified finding.

### Notes of record (non-blocking)

- `cli.rs` header check uses substring containment, so the
  `"wind_capture_ratio"` entry is subsumed by
  `"wind_capture_ratio_delivered"`; column *positions* are asserted
  nowhere for the sweep CSV (the comment claims positions are kept —
  true in the code, untested). Harmless; tidy opportunistically.
- A property test under model-shaped prices (SMP forced £0 exactly
  where curtailment > 0, random elsewhere; assert revenue identity +
  capture direction) would generalise the single worked divergence
  example. Nice-to-have.
- One blank line removed in cli.rs adjacent to the change — trivial,
  noted for the surgical-diff record.
- The Module 1 chart gains two curves (BOTH capture bases; previously
  the chart plotted gas share and curtailment % only — capture lived in
  the CSV). In scope per "both bases reported in the sweep artefacts",
  both labelled with basis; the regenerated Q10 figure will look
  different from the Stage 2 artefact. Expected, not drift.

## 1. Pins and claimed results — all independently verified

- **Stage 1 dispatch digest** `779d7444…` reproduced live by a fresh
  `grid-cli run` on the reference scenario, and
  `regression_2024.rs` (untouched by the diff) passes.
- **Stage 2 prices digest** `1d38ed75…` reproduced live;
  `regression_stage2_2024.rs` (untouched) passes. The run.rs diff
  touches summary.toml keys and stdout only; prices.csv writing code
  untouched.
- No pin file edited: `git status` shows only the package files + the
  supervisor's two record files modified; both regression files are
  pristine; the two new test files are additive.
- Suite: **382 passed / 0 failed** (workspace), `cargo fmt --check`
  clean, `cargo clippy --workspace --all-targets -- -D warnings` clean
  — all run by the reviewer.
- 2024 reference (reviewer's fresh run): potential capture ratio
  **0.9413407336345198**, delivered **0.9413419206049041**
  (Δ = +1.187e-6); curtailment 0.13670684844 GWh over exactly 2 periods
  (2024-04-06 11:30Z, 12:00Z), both priced £0. Matches the
  implementer's claim to all quoted digits.
- Module 1, 60 GW (reviewer's fresh sweep, 50/55/60 GW): potential
  **0.5347799945**, delivered **0.6106059846**; curtailment 21.846 TWh,
  gas 33.21 TWh, gas price-setting 46.47 %, mean SMP £37.14/MWh — the
  50 and 60 GW rows also match the Stage 2 report's table. Claim
  confirmed.

## 2. The inversion finding is a THEOREM of the model, not a 2024 artefact

Ruled: **in-model, curtailment ⟹ SMP = £0 is structural.** The chain,
verified in the code:

1. `dispatch.rs` pushes nonzero curtailment ONLY in the `net > zero`
   (surplus) branch (line ~383). In that branch: thermal output stays at
   its initialisation `vec![Power::gigawatts(0.0); periods]` (only the
   deficit branch writes it), `unserved` is pushed zero, and any storage
   discharge is REJECTED as an invalid decision (D4 rule-3 check,
   reviewer-added Stage 3). Storage charging only reduces curtailment;
   it cannot move the period out of the surplus branch.
2. `grid_core::pricing::system_marginal_price`: convention 1 requires a
   *dispatched* SRMC-bearing technology (power > 0) — none exists in a
   surplus period (renewables and exogenous must-take carry no SRMC;
   nuclear/biomass/coal/hydro carry none either); convention 3 requires
   unserved > 0 — impossible in a surplus period; so convention 2 fires:
   SMP = £0, no setter.

There is **no reachable state** where curtailment coincides with a
nonzero price: not via storage charging, not via must-run interactions
(must-run is must-take, SRMC-less), not via exogenous must-take (never
price-setting by convention). Pricing is single-zone only today
(recorded fact), so no multi-zone loophole exists yet.

Empirical adversarial check on a high-curtailment run (reviewer-built
doubled-wind scenario, 60.6 GW): **5,673 curtailment periods, zero with
SMP ≠ 0**, gas output exactly 0 and unserved exactly 0 in every one, no
storage charging. Per-technology revenue identical to full precision
(offshore wind £2,061.274363348159 m on BOTH bases; onshore
£1,293.9960145906891 m on BOTH bases) while delivered energy is 11.7 %
and 14.2 % below potential — capture ratios 0.5915→0.6701 and
0.4568→0.5323.

Consequences, ruled correct as documented in the module prose:
- delivered revenue ≡ potential revenue (identity, in-model);
- delivered energy ≤ potential energy, strict whenever curtailment > 0
  and the tech generates in a curtailed period;
- therefore delivered capture price/ratio ≥ potential — the naive
  expectation inverts. The identity is convention-dependent and breaks
  under future negative prices / minimum-stable generation / scarcity
  pricing — correctly stated in the prose, and the reason
  `revenue_delivered` is carried as a separate checkable field.

## 3. Definitions ruling (item 2)

**Pro-rata by potential within the period, clamped at the renewable
pool: SOUND and adequately prose-documented** (D4 precedent met — the
rule lives in full prose at `delivered_renewable_power`, with the
ambiguities named). Specifically:
- Pro-rata is the only allocation consistent with the pooled series
  carrying no ordering information; any priority order would be an
  invented dispatch rule. Per-increment/marginal attribution correctly
  deferred to Q2.
- The clamp (`min(curtailment, renewable pool)`, residual exogenous
  spill unattributed) is the right conservative direction and is
  documented as an upper bound on renewable curtailment — delivered can
  only fall, never exceed potential. Zero-pool periods attribute
  nothing (no NaN — tested). Negative curtailment clamped (engine never
  produces it — defensive, documented).

## 4. Ruling for the record (item 4) — supervisor applies verbatim

The prior record has the DIRECTION of the pooled-curtailment bias
wrong. The potential basis does NOT overstate revenue in-model (revenue
is identical on both bases — theorem, §2); it dilutes the capture-price
denominator with £0-priced curtailed energy, so **potential-basis
CAPTURE numbers OVERSTATE cannibalisation** at high wind (60 GW: 0.535
potential vs 0.611 delivered). That is a bias FOR the skeptical thesis
— the dangerous direction for Q10/Q2 — not against it.

(a) **Tracked deviation (memory/project-state.md)** — replace the first
two lines of the pooled-curtailment entry with:

> Renewable revenue/capture carried on potential output
> (pooled-curtailment convention). Package A (2026-07-03) established
> in-model that curtailment occurs only in £0-priced surplus periods,
> so REVENUE is identical on both bases and the potential basis instead
> OVERSTATES capture-price cannibalisation at high wind (dilutes the
> denominator with £0 energy; 60 GW: capture 0.535 potential vs 0.611
> delivered) — a bias FOR the skeptical thesis, corrected from the
> earlier "overstates revenue / understates cannibalisation" record.
> Delivered-basis accounting added alongside (old columns retained, no
> pin moves).

(b) **Frozen-imports entry (same file)** — the parenthetical "(biases
FOR the skeptical thesis — the dangerous direction for Q10/Q2, unlike
pooled-curtailment which biases against)" must lose its last clause:
both deviations now bias FOR the skeptical thesis on capture numbers.
Replace with "(biases FOR the skeptical thesis — the dangerous
direction for Q10/Q2, as does the pooled-curtailment capture
convention per Package A)".

(c) **Stage 2 run report §5** — do not rewrite the committed record;
APPEND a dated correction:

> Correction (2026-07-03, Package A review): the line above has the
> direction wrong. In-model, curtailment falls only in £0-priced
> must-take-only periods (a theorem of the dispatch rules + SMP
> convention 2, reviewer-verified structurally and on a 60 GW run), so
> the potential-output convention does not overstate revenue — revenues
> are identical on both bases. It overstates capture-price
> cannibalisation instead: the potential basis dilutes the capture
> denominator with zero-revenue energy (2024: 0.9413407 potential vs
> 0.9413419 delivered; 60 GW sweep: 0.535 vs 0.611). Delivered-basis
> columns added alongside the potential ones, all Stage 2 pins
> unmoved.

(d) **Q10 public wording rule (binding):** the market-relevant "capture
price" is the **delivered basis** — industry capture/achieved-price and
value-factor conventions (revenue per MWh *generated*) are metered,
post-curtailment quantities. Q10 must: (i) label every capture number
with its basis; (ii) quote the delivered basis as the market-comparable
headline; (iii) where the potential basis is quoted, state that it
overstates the capture decline at high wind and give the delivered
counterpart at the same point; (iv) state the two model caveats — the
delivered series is a modelled pro-rata allocation of pooled
curtailment (a stated convention, not an observation), and the real GB
market pays constrained-off wind through the balancing mechanism /
CfD arrangements the model lacks, so neither basis is a unit's actual
realised revenue at high curtailment; (v) the revenue-identity claim
may be quoted, but only WITH its convention dependence (no negative
prices, no min-stable generation — both documented model limits).

(e) **Module 1 published numbers (potential basis): REMAIN QUOTABLE.**
They are pinned, unchanged, and correct under their stated convention.
Caveat regime: basis label always; at ≥ 40 GW points, pair with the
delivered value or state the direction ("the potential basis overstates
the capture decline; delivered-basis at 60 GW is 0.611 vs 0.535"); and
the frozen-imports bracket rule (separate deviation) continues to bind
independently — Package A does not discharge it.

## 5. Checklist verdicts

1. Acceptance tests: PASS — package tests + full suite run by reviewer
   (382/0); pins re-run and reproduced live.
2. ADR compliance: PASS — pure functions of `RunResult` (ADR-5), no
   wall-clock/globals/randomness; `Power`/`Energy`/`Money`/`Price`
   newtypes across the new public API (the internal `kept` fraction is
   a dimensionless multiplier, acceptable); no schema change → no
   version bump due; crate boundaries respected (allocation in
   grid-adequacy where `RunResult` lives, arithmetic reused from
   grid-core pricing).
3. Conventions: PASS with condition 2 — no library panics
   (structured `InvalidPricing` on misalignment; indexing pre-validated),
   clippy/fmt clean, no new dependencies, outputs' hash embedding
   unchanged, CSV column appended last (order stable), chart series
   labelled with basis (axis text is condition 2).
4. TDD evidence: PASS per the recorded Stage 0 decision (single-tree
   delivery acceptable; gate is test quality + review). Test quality is
   high: the proptests assert the RIGHT general invariants (revenue
   INEQUALITY under arbitrary nonnegative prices — asserting the
   identity there would be false, since the identity is a consequence
   of model-shaped prices; the identity is pinned deterministically
   where it actually holds), the divergence test constrains the correct
   inverted direction with hand-worked numbers (£60 vs £90/MWh), the
   clamp is exercised by overshooting curtailment to 1.5× the pool.
   Weak spot is the CLI wiring lock — condition 1.
5. Data deliverables: N/A (no new data; no licence surface).
6. Scope: PASS — matches the ratified fix exactly; old keys, names,
   values and digests untouched (verified, not trusted); the chart
   addition is within "both bases reported in the sweep artefacts";
   the supervisor's papers-file-note/project-state edits are out of
   package scope and adjudicated in §4.

## 6. Reviewer reproduction artefacts

- `runs/reviewer-package-a/run-2024/` — fresh reference run (digests
  779d7444…, 1d38ed75… reproduced).
- `runs/reviewer-package-a/sweep/` — 50/55/60 GW sweep (table above).
- `runs/reviewer-package-a/run-wind60/` — doubled-wind (60.6 GW)
  theorem check: 5,673 curtailment periods, all SMP = £0, revenue
  identity to full precision.
(runs/ is gitignored; regenerate with the commands above at the
package's commit.)
