# B6 two-zone scenario/engine package — adversarial review

> **CORRECTION BANNER (2026-07-04, beta-readiness audit — supersedes
> the §1d framing below).** This review's ruling that "the
> boundary-attributable delta is stable at ~+33–35% over placements"
> is WITHDRAWN. It is contradicted by the engine's own pinned
> constants: `PIN_B6_ES003_GWH` (33,056) < `PIN_COPPER_ES003_GWH`
> (33,632) — at the 3% placement the boundary *lowers* the
> requirement by 1.7%, impossible under optimal dispatch. The
> same-placement boundary-vs-copper delta is NOT stable; it ranges
> −1.7% (3% placement) to +34.6% (demand-share). The +38.5% lower
> bound is contaminated by the copper-plate rule-based-flow artefact,
> not a clean boundary effect. ONLY the raw total-delta DIRECTION
> (single-zone 23,872 < every two-zone/B6 configuration) is safely
> quotable; no single "boundary effect proper" percentage is, until
> the LP (Stage 7) separates boundary from dispatch convention.
> Records corrected: b6 run report §3/§4, Q4 paper entry,
> project-state, and the acceptance_b6_robustness.rs doc-comment.
> SEPARATELY (Richard, 2026-07-04): the two-zone link models the B6
> boundary alone, but the real Scotland→England restriction is the
> B4+B5+B6 group (B4 bound ~4x harder in 2024) — the model is a LOWER
> BOUND on the restriction; the Scottish-group upgrade is scoped in
> docs/notes/scottish-group-boundary-scoping.md.

Reviewer, 2026-07-04. Package under review (the entire uncommitted
tree): schema v6 (`grid-core`: scenario/error/trace; docs/03 migration
note; v5 fixture freeze; 7 scenario-file version bumps), the sparse
capability loader + `build_link_capability` (`grid-adequacy/src/
inputs.rs`), directional/per-period link capability in the multi-zone
engine (`multizone.rs`; `flow.rs` untouched), the B6 output columns +
assumption lines (`grid-cli/src/run.rs`), `scenarios/gb-2024-2zone.toml`,
and three new test files (`acceptance_b6_2zone.rs`,
`acceptance_b6_robustness.rs`, `regression_2zone.rs`). Everything below
was recomputed, re-run or independently re-derived by the reviewer;
nothing was taken on the implementer's word. Reviewer mechanism
harness: an out-of-tree crate against the package's own APIs; all
controls use the package's bisection conventions verbatim.

## Verdict: ACCEPT-WITH-NOTES — conditions 1–4 corrected BEFORE
## commit; 5–9 are notes of record / follow-ups.

The engineering is sound and everything numerical reproduces exactly.
The material defects are in the **attribution prose of the headline
robustness finding** (item 1 below): the mechanism statement shipping
in `acceptance_b6_robustness.rs` is measurably wrong in two of its
three claims, and the +49.3 % headline is convention-conditioned in
ways the module docs do not state. The finding itself — large, real,
contradicting the ratified expectation — SURVIVES the interrogation;
its framing does not.

## 0. Gates

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -D warnings` clean; `cargo test --workspace --release`
  **535 passed / 0 failed** (includes the Stage 1–5 pins, heating and
  cost suites). Targeted re-runs: `regression_2024` (779d7444… digest
  unmoved), `regression_5zone` (all six zone digests + links digest
  unmoved), `regression_2zone`, `acceptance_stage3_rs37y` (23,872 pin
  stands), `acceptance_b6_2zone` (6/6), `acceptance_b6_robustness`
  (1/1), heating/heating_mix/heating_mix_sweep — all green.
- Manifests re-verified: `b6.sha256` 21/21 OK, `cf-gb2-1985-2024.sha256`
  481/481 OK.
- No dependency changes (workspace Cargo.toml untouched). `flow.rs`
  byte-untouched (empty diff). No `memory/` or docs/04 edits — scope
  clean; every changed tracked file traces to the work order.

## 1. THE ROBUSTNESS FINDING — reproduced; mechanism adjudicated;
## framing RULED (conditions 1–3)

### 1a. Reproduction — VERIFIED

All three numbers reproduce exactly, twice over (the acceptance test
re-run, and the reviewer's independent harness re-implementing the
scenario construction and bisection):

| Configuration | GWh | vs 23,872 |
|---|---|---|
| Single-zone RS pin (suite re-run) | 23,872 | — |
| Two-zone copper-plate, demand-share store | 26,480 | +10.9 % |
| Two-zone B6 4.1/3.5 GW, demand-share store | 35,648 | +49.3 % |

`run_multi` on the committed single-zone RS scenario at 23,872 GWh
gives exactly zero unserved — the single- and multi-zone dispatch
paths are consistent, and the robustness test's hand bisection is
convention-comparable with the pin (the pinned 23,872 is the NAIVE
all-periods requirement; no burn-in exclusion was in play — verified
from `acceptance_stage3_rs37y.rs`).

### 1b. Mechanism — INTERROGATED with controls; the module docs'
### attribution is WRONG on two of three claims

Reviewer controls (all at the package's own conventions; pooled
traces = share-weighted zonal traces, DESNZ onshore split):

| Control | GWh | Meaning |
|---|---|---|
| Single bus, pooled zonal traces, one 100 GW store | **23,808** | trace substitution ≈ NIL (−0.3 %; consistent with the +0.22 % energy wobble) |
| Single bus, same traces, store split 10.1/89.9 GW **and** GWh | **24,112** | the store split per se ≈ +1.0 % |
| Two zones, copper-plate link, demand-share store | 26,480 | +10.9 % |
| Two zones, B6 4.1/3.5 | 35,648 | +49.3 % |

So the claim in the test module docs — *"the split alone (zonal
traces + demand-share store placement) moves the number by ~11 %"* —
is **false**. Traces contribute ~nothing; the split store on a single
bus contributes ~+1 %. The copper-plate two-zone +10.9 % is dominated
by the **rule-based flow convention**: flows clear before storage by
surplus-depth equalisation, blind to store headroom (flow.rs rule 1/3,
documented). Measured evidence at the pinned sizes:

- at the copper pin, **3,618 TWh/40 y of Scottish curtailment occurs
  with the link slack and the RGB store holding both power and energy
  headroom** — the rule declined to wheel surplus to the big southern
  store;
- net annual B6 flow at copper-plate runs **20–53 TWh/yr NORTHWARD**
  (the equal-depth rule ships RGB's absolutely-deeper surplus into the
  zone whose store can charge at only 10.1 GW);
- smoking gun: at the 0.03 energy placement the B6-CONSTRAINED system
  needs **less** storage than copper-plate (33,056 vs 33,632 GWh) —
  physically impossible under optimal dispatch; the module docs' claim
  *"Direction is structural: a finite boundary can only increase the
  requirement relative to copper-plate"* is **false** for this engine
  (the assert holds at the tested placement; the comment overclaims).

The B6 leg (+34.6 % over copper) is, by contrast, substantially
genuine boundary physics at the stated stress convention: ~2,719
TWh/40 y of Scottish curtailment occurs with the export at ≥99 % of
the 4.1 GW cap; southward export binds in ~28–35 % of periods every
year; and the boundary-attributable delta is **stable at ~+33–35 %
across store placements** (B6 vs copper at the same placement:
demand-share +34.6 %; placement-optimum 33,056 vs 24,928 = +32.6 %).
The drought-depth premise of the ratified expectation is half right —
worst year 2010 in both zones, no unserved anywhere at the pinned
sizes — it is the inter-drought RECHARGE that is boundary-limited.

### 1c. Store placement — the named alternatives are NOT runnable as
### named; the energy placement moves the number materially

- *"Alternatives (wind-share placement, all-south) are named, not
  run"* (module docs) is misleading: as splits of power AND energy
  both are **infeasible at any store energy** (bisection cliff,
  reviewer-run). The 100 GW rating must track zonal peak demand (RGB
  needs ≈ its ~90 GW peak minus 3.5→4.1 GW import; SCO its ~10 GW
  peak minus 3.5). Only the ENERGY placement is free.
- Energy-placement sweep at fixed demand-share power (B6 4.1/3.5):
  33,056 GWh at 3 % Scottish energy (the measured optimum), 33,728 at
  5 %, **35,648 at the 10.1 % demand-share convention**, 37,696 at
  15 %, 40,064 at 20 %, 49,152 at the 34.8 % wind share. The
  demand-share headline sits **+7.8 % above the placement optimum**;
  the spread is material and must be quoted.
- Onshore-split convention sensitivity (cluster 0.7361 instead of the
  adopted DESNZ 0.6997): copper 26,592 / B6 36,512 — ~+2.4 % on the
  headline; second-order, consistent with the data package's adopted
  convention.

### 1d. Framing RULING (the supervisor applies this verbatim)

"+49 % at 2024 boundary capability" with only the stress convention
stated is **not** the honest headline. The finding of record is quoted
in this form:

> Under the two-zone split with B6 frozen at the ruling's 2024 central
> capabilities (export 4.1 / import 3.5 GW), the RS-fleet 40-year
> storage requirement rises from the single-zone 23,872 GWh to
> 35,648 GWh (+49.3 %) at the demand-share store placement. Three
> conditions travel with the number. (1) STRESS CONVENTION: end-2024
> zonal capacity shares and 2024 boundary capability projected onto a
> 520 GW fleet — a statement about today's network under that fleet,
> never a projection of a credible future network. (2) PLACEMENT: the
> hydrogen store's energy placement moves the requirement from
> 33,056 GWh (+38.5 %, measured optimum near a 3 % Scottish energy
> share) through 35,648 (+49.3 %, demand share) to 49,152 (+106 %,
> wind share); power placement is not free (the rating must track
> zonal peak demand). The boundary-attributable effect proper is
> stable at ~+33–35 % over placements (B6 vs copper-plate at the same
> placement). (3) DISPATCH CONVENTION: the two-zone copper-plate
> baseline sits +4 to +11 % above the single-zone pin depending on
> placement, and controls show this is dominated by the rule-based
> flow convention (flows clear before storage by surplus-depth
> equalisation, blind to store headroom), not by the zonal split —
> the same fleet, traces and two-store split on a single bus need
> 24,112 GWh (+1.0 %). Every quote therefore carries "rule-based
> dispatch, upper-bias" alongside the existing "B6-only slice, lower
> bound on network effects" duty; the two biases run in opposite
> directions, and the LP comparison (Stage 7) is the named resolver.
> The ratified expectation "flagship storage numbers barely move" is
> CONTRADICTED as measured, at full prominence: drought depth is
> GB-wide, but inter-drought recharge is boundary-limited.

### 1e. Record correction — ORDERED (Package A/B precedent: corrected
### entry, direction stated). Verbatim draft:

> CORRECTION (2026-07-04, B6 two-zone engine review): the standing
> caveat attached to the Stage 3/4 flagship storage numbers (the
> 23,872 GWh RS requirement and its derivatives), the Q4 paper
> framing, and the manual's limitations register — "copper-plate is
> expected conservative for adequacy; drought periods are
> network-unconstrained, so the flagship storage numbers should barely
> move under a two-zone split" — is WITHDRAWN as measured. Direction:
> the copper-plate convention UNDERSTATES the storage requirement once
> the B6 boundary is represented: +38 % to +49 % at 2024 boundary
> capability under the RS fleet (placement-dependent; framing of
> record in docs/notes/b6-two-zone-engine-review.md §1d). The premise
> was half right: drought depth is GB-wide (worst year 2010 in both
> zones); it is the inter-drought RECHARGE that is boundary-limited.
> The single-zone flagship numbers remain the pinned copper-plate
> results and are henceforth quoted as "copper-plate LOWER BOUND on
> the storage requirement with respect to internal network constraints
> (measured +38–49 % two-zone/B6 sensitivity at 2024 boundary
> capability, rule-based dispatch, demand-share placement)". The
> Q2/Q10 curtailment/capture direction statements are unaffected (they
> were already quoted as copper-plate-flattered bounds); Q4/M3
> adequacy claims and the manual's limitations register inherit this
> correction verbatim.

## 2. Validation gates — reproduced; wedge budgets ADJUDICATED LEGITIMATE

- Anchors independently recomputed from the pack (reviewer Python,
  not the test's arithmetic): flow mask **17,211** rows (17,214 − 3
  NaN), net DA flow **22.627189 TWh**, binding share
  **0.236011852884783** — exact matches to the test constants and
  b6_report.json. The binding convention (53 zero-limit rows in the
  denominator, never the numerator) matches the reviewed pack
  convention on both the observed and model sides.
- Gate (i): copper-plate net **18.809 TWh**, r = **0.7443** —
  reproduced. The ±4.5 TWh budget is a named-wedge ENVELOPE calibrated
  on the first pass, which is exactly what the link ruling's deferral
  sanctioned ("tolerances pinned only after first runs quantify the
  wedges"). The demand-basis wedge (−3.7 TWh) is evidence-based in its
  components — both bases (10.1 % consumption / 8.7 % metered) are in
  the reviewed data report §4; (0.101−0.087) × 261.8 TWh = 3.66 TWh is
  first-order transfer arithmetic, an upper-ish bound since it bites
  only in Scottish-surplus periods. Not reverse-engineered, but not an
  independent prediction either: the honest description, which the
  test carries ("first-run quantification, per the ruling's
  deferral"), is a decomposed envelope around a −3.82 measured miss.
  The r ≥ 0.70 floor is a tripwire under the 0.7443 first pass, stated
  as such. The exact-value pins (1e-6/1e-9) carry the real regression
  duty. ACCEPTED.
- Gate (ii): constrained net **15.788 TWh** vs 17 ± 2.5 — reproduced;
  the model lands between the anchors (22.6 unconstrained / 17
  constrained), the reviewed expected position; full-year total with
  masked periods dispatched at the 4.1 fill matches the ledger basis
  of the anchor. Gate (iii): **0.2323** vs 0.2360 ± 0.04 — reproduced;
  numerator/denominator conventions consistent with the observed
  anchor's.

## 3. Schema v6 — VERIFIED

- **Additive claim holds structurally, not by luck**: the capability/
  binding columns are emitted only when `link.capability.is_some()`,
  which requires v6 detail (`reverse_capacity_gw` or
  `capability_trace`); the 5-zone file sets neither (diff = header +
  version line only); the assumption lines ride OUTSIDE the digested
  links data section and are empty for non-detailed links. Pre-v6
  byte-identity re-verified by the pinned digests (regression_2024
  779d7444…, regression_5zone all-six + links digest, both re-run).
- **Sentinel handling lives in the scenario**, never a loader default:
  `sentinel_high_mw`/`upper_bound_gw`/`masked_fill_gw` are required
  fields of the declared block; the sparse loader itself treats zero
  as a value ("sentinel semantics belong to the scenario's declared
  spec" — trace.rs) and the ruling's handling (≥9,999 → 6.7 observed;
  0/NaN/missing → masked, filled 4.1, dispatch every period) is
  implemented in `build_link_capability` and tested
  (`capability_assembly_applies_the_ruling_sentinel_handling`, horizon
  bounds, off-grid and negative rejection).
- **Sparse loader error paths**: missing file, missing column, wrong
  index type (naive tz refused per ADR-3), nulls in index,
  non-strictly-increasing index, empty file — all structured errors;
  gaps/None preserved; tested including a live read of the B6 pack.
- **Migration discipline**: v5 fixture frozen
  (`grid-core/tests/fixtures/v5-gb-2024-reference.toml`, verified to
  be the pre-migration file at schema_version 5);
  `SchemaVersion5Superseded` names what v6 added and the one-line
  migration; docs/03 note complete with the ruling cross-reference;
  all seven scenario migrations are version-line(+header) only —
  **no heating or cost semantics touched in any collateral file**
  (every collateral diff read: literal `5` → `6` plus header notes).
- Semantic validation (finite/non-negative reverse capability, finite
  positive sentinel, finite non-negative bounds/scale) implemented and
  property-tested; unknown fields rejected; v6 fields round-trip.

## 4. Q2/Q10 measurements — reproduced; one quote-arithmetic ruling

- Reference fleet: SCO curtailment **1.684 TWh** constrained vs
  **0.0486** copper-plate (RGB 0 vs 0.00007) — reproduced. 60 GW
  point: SCO **27.139 vs 13.768**, RGB 3.696 vs 10.421, binding share
  **0.4674**, net southward 22.886 TWh — reproduced. The 60 GW run
  uses the ruling's scaled-run convention verbatim (capability trace
  dropped, flat 4.1/3.5).
- Quote-duty lines verified ON the artefact (regression test asserts
  the verbatim ruling-(c) text in links.csv; re-run green), and the
  gate-(iii) binding statistics ship in summary.toml.
- **"B6-attributable" subtraction ruling (condition 7)**: the
  constrained-minus-copper SCO delta (+13.371 TWh at 60 GW) must never
  be quoted alone — RGB moves **−6.725 TWh** in the same comparison
  (blocked northward surplus-shuffling), so the system-net B6 effect
  is **+6.65 TWh**. Any B6-attributable quote carries both, or quotes
  the system net.

## 5. Conventions — PASS (one newtype defect, condition 4)

- Pins unmoved (all suites re-run, §0). No new dependencies. No
  panics/unwraps in library code (clippy deny-set clean; test files
  carry the sanctioned allows). Determinism: no clocks/randomness in
  the new paths; digests pinned. Outputs embed the standard hash
  metadata (pre-existing block; assumption lines added outside the
  digest).
- **Defect**: `LinkCapabilityTraceSpec::sentinel_high_mw` is a raw
  `f64` MW quantity on a grid-core public API — the ADR-4/docs-06
  newtype rule says `Power` (its sibling fields are `Power`).
  Condition 4.
- ETYS condition (data-review condition 4) correctly carried as
  **OUTSTANDING on the record** in the scenario header (with the
  honest sensitivity statement: 116 sentinel periods, p99 6,400
  consistency) — not silently absorbed. Stays open.
- TDD: commit-order evidence is unverifiable on an uncommitted tree.
  The structure is consistent with designed-red practice (dated
  first-pass pins; unit tests covering every new code path); the
  supervisor should preserve test-first ordering if the commit is
  split. Note 9.

## 6. Scope — PASS

Every modified tracked file traces to the work order (schema bump
collateral, the scenario, tests, engine, CLI outputs, docs/03).
No unauthorised doc edits; memory/ untouched; the ADR untouched.

## Conditions

1. **(Pre-commit)** `acceptance_b6_robustness.rs` module docs: correct
   the mechanism attribution per §1b — (a) the "~11 % split effect" is
   dispatch-convention-dominated (quote the controls: pooled 23,808;
   two-store single-bus 24,112); (b) the "direction is structural"
   comment is false for this engine (B6 33,056 < copper 33,632 at the
   3 % placement) — keep the assert, re-scope the comment to the
   tested placement; (c) the named placement alternatives are
   infeasible as power+energy splits (power must track zonal peak);
   only energy placement is free, and its measured spread (33,056–
   49,152; demand-share 35,648) is stated.
2. **(Pre-commit)** The finding of record is framed per §1d verbatim
   (module docs now; run-report note and papers-plan caveat when
   written).
3. **(Pre-commit)** The record correction of §1e enters
   memory/project-state.md (and propagates to the papers plan and the
   manual's limitations register when those are next touched).
4. **(Pre-commit)** `sentinel_high_mw` becomes `Power`
   (megawatts) or carries a documented field-level exemption.
5. **(Note)** `forward_binding_share_of_observed` (denominator 17,158
   observed periods; 0.23301) is not the gate-(iii) statistic
   (denominator 17,211 mask rows; 0.23229) — never quote them
   interchangeably; label at next artefact touch.
6. **(Note, pre-existing, follow-up)** Duplicate fleet TechIds within
   a zone are not rejected by `Scenario::validate` and silently
   corrupt programmatic input assembly (keyed maps) — found by the
   reviewer's harness, not introduced by this package. File it.
7. **(Binding on quotes)** The B6-attributable subtraction rule of §4.
8. **(Standing)** ETYS 6.7 GW fetchable-artefact condition remains
   open (correctly flagged).
9. **(Note)** TDD commit-order evidence: see §5.

## Reviewer-reproduced numbers (appendix)

Gates: 18.808794819925264 TWh / r 0.7443336620570405;
15.787702182212668 TWh; binding 0.23229330079600255. Anchors from the
pack: 17,211 mask; 22.627189 TWh; 0.236011852884783. Robustness:
26,480 / 35,648 (twice, independently). Controls: 23,808 (pooled),
24,112 (two-store single bus), copper energy-placement 24,928–36,512
(min es 0.045), B6 energy-placement 33,056–49,152 (min es 0.03);
cluster-onshore variant 26,592 / 36,512. Q2/Q10: 1.684/0.0486;
27.139/13.768; RGB 3.696/10.421; 0.4674; 22.886. Curtailment
classification at pins (TWh/40 y): B6 — SCO 9,751 [at-cap 2,719 |
slack+RGB-headroom 2,155 | slack+RGB-saturated 4,878]; copper — SCO
11,286 [0 | 3,618 | 7,668]; net copper B6-flow 20–53 TWh/yr northward.
