# D11 — Tier-2 priced-dispatch ENGINE package: independent review

**Independent reviewer (D11 engine gate), 2026-07-05.** Gate review of
the uncommitted D11 engine package (schema v7 + priced ladder +
Phase-0 characterisation; ~38 files changed, new
`grid-adequacy/tests/acceptance_d11_priced_ladder.rs`, new
`grid-core/tests/fixtures/v6-gb-2024-reference.toml`, new
`docs/notes/d11-a2a-mismatch-characterisation.md`). Method: re-derive,
do not trust — every load-bearing number below was recomputed with my
own probe (a scratchpad crate path-depending on the workspace; no file
in the tree was modified), the gates were re-run on this machine, and
the ladder arithmetic was hand-checked on constructed cases through
the public engine API.

## Verdict: ACCEPT-WITH-CONDITIONS

The engine deliverable is correct, conservative, and honestly
reported. The A2a regression is real engine behaviour of a correctly
built merit-order-coupling rule on price data whose only per-zone
difference is sub-noise — not an implementation bug (§C). The ≥95%
unreachability verdict reproduces from the committed packs
independently of the implementer's code path (§A). No committed gate,
pin or digest moved (§B). Four conditions, all documentation-level,
none touching engine code (§H).

## A. Phase-0 re-derived (my numbers, from the packs + my own scarcity run)

Probe: my own `run_multi` of the committed 5-zone scenario (scarcity),
joined per period against `fr_generation_2024.csv`,
`flows_gb_entsoe_2024.csv`, `generation_by_fuel_2024.csv` and the
UKA+CPS step series via `carbon_price_step_series`. Every
characterisation-note number reproduces:

| Quantity | note | re-derived |
|---|---|---|
| Scarcity A2a | 15,823 = 90.07 % | 15,823 = 90.07 % |
| Total mismatches / model-exp-obs-imp class | 1,745 / 1,297 | 1,745 / 1,297 |
| Model-labelled both-gas in class | 1,122 (86.5 %) | 1,122 (86.5 %) |
| GB observed gas > 1 GW in mismatches | 100 % | 1,745/1,745 |
| FR gas annual p05/p10/p20 floors | 0.43/0.46/0.49 GW | 0.435/0.460/0.493 GW |
| Really-non-gas shares (annual p05/p10/p20) | 0.2/0.9/6.1 % | 0.2/0.9/6.1 % |
| Monthly-p05 +0.1/+0.5/+1.0 GW shares | 14.4/39.3/51.1 % | 14.6/39.3/51.1 % |
| Nuclear headroom >0.5 / >1.0 GW | 5.2/3.8 % | 5.2/3.8 % |
| Nuclear at ceiling, class vs year | ~95 % vs 67.5 % | 94.8 % vs 67.5 % |
| FR gas in class p10/p50/p90 | 0.53/2.42/6.48 GW | 0.53/2.42/6.48 GW |
| Cross-tab (gg/ng/gn/nn) | 1,444/270/10/21 | 1,444/270/10/21 |
| Static ceiling at 51.1 % fixable | 93.8 % | 93.84 % |
| GB carbon span / dearer-share | £50.10–64.92; 44.5 % | £50.10–64.92 (mean 55.13); 44.5 % |
| GB dearer within both-gas class | 410/1,122 | 410/1,122 |
| §3 "~9,550 (5,332 GB-cheaper)" | ~9,550/5,332 | 9,548/5,330 (both-gas ∧ correctly-matched import periods) |

(The only wobble is 14.6 vs 14.4 % — percentile-convention rounding —
and ±2 on the §3 soft figures; nothing pinned is off.)

**Circularity check — passes.** The mismatch set is necessarily
model-defined (it is the model's error set), but the classification
*within* it is observed-only (ENTSO-E FR generation, FUELHH GB gas);
the 1,122 both-gas figure is explicitly the model's own label and is
cross-tabbed against observation (1,444/270/10/21, reproduced) —
the note does not assume what it concludes. One caveat to carry: the
FR nuclear "availability ceiling" is the scenario's monthly-mean
calibration (A75 monthly GWh / capacity·hours), a soft ceiling — the
67.5 % whole-year at-ceiling base rate shows it. The unreachability
arithmetic does NOT rest on it: the static-ceiling bound uses the most
generous 51.1 % fixable reading from the independent gas-floor
observable, and 90.07 % + 0.511 × 1,297/17,568 = **93.84 % < 95 %**
stands. The 97.4 % expectation (15,823 + 1,297 = 17,120/17,568)
assumed the whole class priceable; the observed data refutes that.
**The ≥95 % target is unreachable on 2024 licence-clean prices — the
pre-registered rule-4 finding is confirmed independently.**

## B. Headline numbers re-derived; committed record untouched

My probe re-ran the priced ladder (in-memory `flow_signal` flip, as
the acceptance test) and the flat-flat sensitivity in a separate
process: **A2a priced 12,595 = 71.69 %; flat-flat 16,370 = 93.18 %**
— exact matches, which also demonstrates cross-process determinism.
The full suite re-run on this machine: **611 passed / 0 failed / 4
ignored** (the 4 are the ignored tractability benches), including
`acceptance_stage5_2024` (A1–A4 two-limb gates), `regression_2024`
(779d7444 digest), `regression_2zone/3zone/5zone` (digests unmoved —
the committed scenarios stay on the scarcity default), B4 (1.96 % +
LP band), B6 acceptance + robustness, heating/Q-series and stage5
capacity-credit tests. `acceptance_stage5_2024.rs` is not in the
diff; no committed gate was weakened or re-pinned. The ladder-run
regression record (A2b 838/1,312 = 63.87 %; A1 +25.70 TWh / gas
82.20 TWh; A3 −0.185; A4 BE +0.79; the six border TWh) is pinned in
`acceptance_d11_priced_ladder.rs` as FINDINGS of the ladder run only,
with the misses stated in-line — the D11 rule-4 discipline
("a regression is a failure/finding, not a re-pin") followed to the
letter.

## C. The ladder itself is correct; 71.69 % is honest behaviour

Read in full (`flow.rs` `PricedZoneCurve` / `equalising_flow_priced`,
`multizone.rs` wiring): the lexicographic signal is (rung SRMC
primary, the exact scarcity score secondary); the equal-primary
branch is byte-for-byte the scarcity walk's arithmetic (`rate`,
`d_cross` — positive-rate invariant preserved by the secondary); the
primary-gap branch is the stated bang-bang to breakpoint/cap; prices
are per-rung constants so "SRMC at the current residual as the border
sweep advances" is realised exactly by the breakpoint walk; the
q_max = cap ∧ (stack − r_exp) bound and the loss algebra are shared
with the scarcity path. Hand-checked through the public API
(constructed two-zone scenarios, my own expected values):

1. both-gas, £1 wedge, big cap → q = 8 (importer's whole gas rung
   displaced = exporter's stack headroom) ✓
2. same, cap 1.5 → 1.5 ✓
3. equal prices, loss 0.1 → q = 6/1.9 (the scarcity equalisation),
   and **bit-equal** to the scarcity rule's flow ✓
4. wedge REVERSED against the stress gradient → direction flips,
   stops exactly at the exporter's stack ceiling (+2.0) — no sign
   error, no wrong residual point ✓
5. £0.01 wedge → full-band displacement (−5.0): the sub-noise
   mechanism behind the regression, confirmed as designed bang-bang ✓
6. unserved importer vs OCGT exporter → run-scope ceiling outbids,
   deficit fully served, never exports into own unserved ✓

The byte-identity property test is genuinely discriminating: it
sweeps a dense residual grid over four curve shapes (shared rungs,
ladder gaps, surplus, unserved) × three cap/loss pairs — a flipped
lexicographic order, a mis-evaluated residual or a broken secondary
would diverge on that grid. The £0-floor tiebreak engages (two-surplus
split 2.5 reproduces). **Conclusion: 71.69 % is the honest output of a
correctly built merit-order-coupling rule fed a sign-flipping,
sub-noise wedge; the ±21.5 pp swing between two equally defensible
carbon conventions (71.69 vs 93.18) is the finding, and pinning the
committed-convention number as the finding (not the flattering one)
is the right call.**

## D. Schema v7 — clean

Strict parse (`deny_unknown_fields`, frobnicate tests), lossless
round-trip, default-`scarcity` serialisation-skip, v6 fixture rejected
with a migration message naming every addition
(`SchemaVersion6Superseded`), docs/03 migration note present and
accurate. Nine scenarios: eight are version-line-only (verified in the
diff); `gb-2024-5zone.toml` adds the six cited `[zones.pricing]`
blocks — GB on the reference UKA+CPS step (no flat override), five
external zones on flat £55.01 = the committed
`prices-eu-2024.toml` `[carbon.eua]` value, guarded by the drift test
(which reads the committed reference, not a second transcription);
NO2/DK1 declare no priced technology with the water-value/identity
boundary stated in comments. Semantic validation (finite non-negative
flat carbon; srmc keys name dispatchable own-fleet entries;
priced_ladder requires pricing on every zone) is tested at both
scenario and loader level.

## E. The five design forks — adjudicated

1. **`dispatch.flow_signal` field, default scarcity, committed
   scenarios unmoved** — ACCEPT. The design assumed the ladder would
   replace the signal and the 5/2-zone digests would move; rule 4's
   own miss-clause ("the priced ladder is not adequate and the
   finding is named") makes the selectable-signal outcome the correct
   one. The deviation is surfaced (characterisation §5, migration
   note), and the digests' not moving is the proof the committed
   record is untouched.
2. **Optional `carbon_flat_gbp_per_tco2`** (flat level, not the
   design's alternative adder-over-shared-base) — ACCEPT. It is the
   faithful representation of the licence-clean data (flat EUA mean
   vs UKA step); the granularity asymmetry it creates is disclosed in
   the struct docs and IS the finding. (Note the design's adder
   alternative corresponds to the flat-flat sensitivity — both
   conventions are measured and pinned, so nothing is hidden.)
3. **Unserved ceiling at RUN scope** (per-period max over every
   zone's priced SRMCs) — ACCEPT, and required: a per-zone ceiling
   could rank an unserved zone's primary below a neighbour's dearest
   rung (wrong-way flow), and would violate the curve invariant
   unserved ≥ top rung under cross-zone comparison. It is the coherent
   multi-zone generalisation of Stage 2 convention 3, stated in the
   flow.rs prose, never monetised into adequacy.
4. **Monotone-price validation + IE-SEM oil-as-gas-OCGT** — ACCEPT.
   The ladder IS the engine's SRMC-proxy order; a price inversion
   would contradict the dispatch order the flow walk assumes, so
   refusing it (structured error) is right for v1. The IE oil-peaker
   rung priced at gas-OCGT is the data package's stated gas-only
   boundary (report §4/§5) and the only monotone-consistent choice
   short of new oil-price data. Carried constraint, noted: a future
   fleet whose real SRMC inverts the ladder (e.g. priced coal above
   gas) will be refused until the ladder/pricing model is revisited;
   CONT-NW's unpriced coal (£0 below priced ccgt) keeps monotonicity
   at the cost of understating CONT-NW's price when coal-marginal —
   stated in the scenario.
5. **A2a run flips `flow_signal` in-memory** — ACCEPT. B4-LP in-memory
   precedent; keeps every committed digest byte-identical, and the
   regression digests would catch a silent file flip.

## F. Hard rules

- **No library panics** on the new paths: all constructors/loaders
  return structured `GridError`s (NaN ceilings, non-finite/decreasing
  prices, ceiling-below-top-rung, missing pricing inputs, no-priced-
  technology-anywhere, non-gas fuel, unknown efficiency key, horizon
  misalignment); indexing is bounds-guaranteed by prior validation.
- **Newtypes**: `Trace<Price>` and `CarbonPrice` cross the public
  APIs; raw f64 stays inside `pub(crate)` flow internals.
- **Determinism**: no clock/globals/randomness in the new code; the
  5-zone rerun bit-identity test passes, and my probe reproduced the
  pinned counts exactly in a separate process.
- **fmt/clippy/tests**: `cargo fmt --check` clean;
  `cargo clippy --workspace --all-targets --release -D warnings`
  clean; suite 611/0/4 (re-run, not taken on trust). No new
  dependencies.
- **TDD**: not verifiable by commit order — the package is a single
  uncommitted tree. The artefact shape is consistent with the claimed
  red-first ≥95 % test converted per rule 4 (the test asserts
  `rate < 0.95` as the finding's shape plus the exact pin), and the
  property/mechanism tests exist at every layer. Condition 4 below.

## G. RULING — the tier-2 sweep central estimate runs the SCARCITY rule

The sweep package (D11 rule 2) should run its central estimate on the
**scarcity rule**, with the priced ladder as a named sensitivity, not
the headline. From the design's own rules:

- Rule 2's purpose is **endogeneity** (imports respond to the swept
  fleet via `run_multi`); that is achieved under either signal — the
  signal choice is orthogonal to the frozen-imports fix.
- Rule 4's discipline: gates must still pass; the ladder measurably
  fails A1 (+25.70 vs 33.30 ±10 %), GB gas (82.20 vs 72.79 ±5 %),
  A2a/A2b, A3 (−0.185 vs ≤ −0.25) and A4 BE at the 2024 anchor, while
  the scarcity rule passes all of them. A central estimate must be
  anchored to the configuration that validates at the anchor year.
- Phase 0 shows the ladder's primary key carries no signal in the
  dominant both-gas regime on 2024 data — its directions there are
  convention noise (±21.5 pp across two defensible carbon
  conventions). A capacity sweep would multiply unvalidated bang-bang
  directions into the curtailment/import central numbers.
- Rule 2 already names the disclosed band: the Package B
  frozen/zero/export bracket remains the uncertainty band around the
  scarcity-rule central estimate.

Recommended shape for the sweep work order: scarcity-rule multi-zone
central estimate quoted against the Package B bracket; the priced
ladder run at least at the 60 GW pin (the whole sweep if cheap) as a
reported sensitivity carrying the characterisation §3 caveat — the
three-policy-ladder discipline (rule 3) reported, not silently
dropped. Re-open the signal question only for a year with a real,
signed carbon wedge (2022–23 or post-linkage), where the ladder's
primary key carries information.

## H. Conditions (all documentation; none touch engine code)

1. **`data/reference/prices-eu-2024.toml` header**: still says
   "STATUS: DRAFT. Not yet consumed by the engine." It is now
   load-bearing (transcribed into `gb-2024-5zone.toml`, read by the
   drift-guard test). Update the header in the D11 commit.
2. **docs/04 (~line 214) and the docs/08 D11 row** still present the
   ≥95 % / 97.4 % target as pending. Per the design's own touch-point
   ("cross-referenced, not silently changed"), add the one-line
   cross-reference: target attempted 2026-07-05, measured 71.69 %
   (93.18 % flat-flat), unreachable on 2024 prices — see the
   characterisation note; scarcity remains the committed signal.
3. **Stale migration comments**: five version-line-only scenarios
   (`gb-2024-benign-battery`, the four `royal-society-37y*` files)
   still carry `# v5 -> v6: version line only` on the line now
   reading `schema_version = 7`. Correct to name v7.
4. **Commit hygiene**: structure the commit(s)/message so the
   red-first → rule-4-conversion record survives (the checklist's
   TDD-evidence requirement cannot be met retroactively otherwise),
   and update `memory/project-state.md` per session hygiene.

Non-blocking nits: (i) in
`multizone.rs::priced_ladder_follows_the_srmc_wedge…` the comment
"instead of the equalising 1.5 GW" is wrong (the scarcity
equalisation there is 4.5 GW, cap-bound at 2.0 like the ladder — the
first two limbs don't discriminate at that cap; the reversed-wedge
limb does; consider raising the cap); (ii) the characterisation could
add one clause noting the FR-nuclear "ceiling" is a monthly-mean
calibration (soft ceiling) — the verdict does not rest on it (§A).

— independent reviewer (D11 engine gate), 2026-07-05
