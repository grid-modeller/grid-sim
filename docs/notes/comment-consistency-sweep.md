# Comment-consistency sweep — fix list

Date: 2026-07-06. Scope: grid-adequacy/src (lp, multizone, flow, sweep,
dispatch, policy, costs, inputs), grid-core/src (scenario, units,
pricing), grid-adequacy/tests acceptance module docs, and a lighter pass
over grid-stability/src and grid-cli/src. READ-ONLY sweep: nothing was
fixed; every item below is a defect for the fix package. Method: code
read against comments; behavioural claims verified against the code
path; the RTE-floor boundary claim verified empirically
(`cargo test -p grid-adequacy --test lp_guards` — 4 passed, including
`at_or_above_the_floor_is_accepted`).

**Summary: 4 major, 6 minor, 6 notes, plus a quotable-numbers audit
(3 numbers lacking a pin/citation pointer).**

---

## MAJOR (false guarantee / false constraint / policy-prose drift)

### M1. lp.rs module doc — the "Scope (package 2a)" section is now false
- **Where:** `grid-adequacy/src/lp.rs:86-93`.
- **Claims:** "Small hand-checkable scenarios only. NOT wired into the
  bisection (`solve.rs`) — that is 2b. No 40-year / full-GB scaling, no
  rolling-horizon window, no priced objective."
- **Code:** the same file exports `run_multi_lp_rolling` (lp.rs:856 —
  a rolling-horizon window), the file carries the 2b scaling guards
  (`LP_RTE_FLOOR` lp.rs:138, `LP_VARIABLE_CAP` lp.rs:154), and
  `solve.rs:270` (`min_storage_for_zero_unserved_lp`) wires
  `run_multi_lp` into the bisection as the feasibility oracle. A second
  labelled objective (`run_multi_lp_min_curtailment`, lp.rs:340) also
  exists. Only the energy-budget rejection and the absence of a
  priced/min-COST objective remain true.
- **Severity:** major — the module's normative scope statement asserts
  constraints the file itself violates.
- **Proposed text:** "# Scope. The 2a core (this formulation) plus the
  2b additions: the tractability guards (`LP_RTE_FLOOR`,
  `LP_VARIABLE_CAP`), the rolling-horizon window
  (`run_multi_lp_rolling`), and the bisection oracle wiring
  (`solve::min_storage_for_zero_unserved_lp`). Energy-budgeted fleet
  entries are still rejected with a structured error; no priced/min-cost
  objective exists (MinUnserved and MinCurtailment only)."

### M2. policy.rs rule-1 prose — "must-run plant at availability" has no engine representation (ADR-6 policy prose)
- **Where:** `grid-adequacy/src/policy.rs:36-38` (rule 1 of the
  normative D4 prose); source wording identical in
  `docs/notes/d4-rule-based-dispatch.md:35-36`.
- **Claim:** "Must-take supply dispatches first: weather-driven
  renewables (CF × capacity), **must-run plant at availability**,
  exogenous traces."
- **Code:** must-take is renewables + exogenous only
  (`dispatch.rs:288-296`, `multizone.rs:971-981`); there is no must-run
  category. Nuclear/biomass/hydro sit in the dispatchable stack and do
  NOT run in surplus periods (dispatch.rs rule 2, "no thermal plant
  runs" — dispatch.rs:17, code path dispatch.rs:388-410). The
  `SystemState::must_take` field doc in the SAME file
  (policy.rs:250-252) correctly omits must-run plant — the module is
  internally inconsistent.
- **Severity:** major — this is the hard-rule policy prose (docs/06:
  "documented in the code in full prose"); drift here is quotable.
- **Caution for the fixer:** the D4 decision note carries the same
  wording; the prose fix should either carry a clarifying parenthesis or
  be routed as a D4-note erratum — never a silent divergence from the
  adopted record.
- **Proposed text:** "1. **Must-take supply dispatches first**:
  weather-driven renewables (CF × capacity) and exogenous traces. (D4's
  'must-run plant' has no separate engine category: must-run behaviour
  arises from merit position, and such plant runs only against
  deficits.) Compute `net = must_take − demand` …"

### M3. sweep.rs — "bit-identical" demand-rescale guarantee is unsound on the heating path
- **Where:** `grid-adequacy/src/sweep.rs:24-30` (module doc, "Input
  reuse") and `sweep.rs:645-654` (`scale_demand` doc, "reproduced
  bit-identically").
- **Claim:** loading demand unscaled and applying the scenario's scaling
  in memory is "bit-identical to loading with the scaling applied".
- **Code:** the loader computes `p × scale + extra + h`
  (`inputs.rs:220-227`); the sweep computes
  `((p + h) − h) × scale + extra + h` (`sweep.rs:666-673`, where the
  neutral load already folded `h` in). `(p + h) − h` is not bit-equal to
  `p` in IEEE arithmetic in general, so the guarantee is unproven for
  any scenario with a schema-v5 heating overlay. The no-overlay path
  (×1.0, +0.0) does preserve bits. The in-file test
  (`scale_demand_scales_the_base_but_never_the_heating_overlay`,
  sweep.rs:2105) passes only for its particular values; it does not
  establish the general claim.
- **Severity:** major — asserts a bit-level determinism guarantee the
  arithmetic does not provide; anything comparing sweep-point output
  against a direct-run digest on a heating scenario relies on it.
- **Proposed text:** "…bit-identical to loading with the scaling
  applied for scenarios WITHOUT a heating overlay (both compute
  `base × scale + extra`). With an overlay the rescale round-trips
  `(neutral − h)` and may differ from a direct load by float rounding
  (≤ 1 ULP per period); no digest may be assumed shared across the two
  paths for heating scenarios." (Or: make the loader export the
  pre-heating base so the sweep never subtracts.)

### M4. RTE-floor boundary — "at or below the floor is rejected" is false at the boundary (verified empirically)
- **Where:** `grid-adequacy/src/lp.rs:192-193` (`check_rte_floor` doc:
  "Reject a store whose round-trip efficiency is **at or below**
  `LP_RTE_FLOOR`") and `grid-adequacy/tests/lp_guards.rs:7-8` (module
  doc: "sits at or below the safe floor … is rejected").
- **Code:** `lp.rs:200` rejects strictly below (`eta < LP_RTE_FLOOR`);
  η == 1e-3 is accepted. Proven by `lp_guards.rs:193`
  (`at_or_above_the_floor_is_accepted` — "η exactly at the floor is
  accepted") and by test run 2026-07-06. The `LP_RTE_FLOOR` constant doc
  (lp.rs:131-133) states it correctly ("< LP_RTE_FLOOR is rejected …
  η ≥ 1e-3 is the accepted region") — the two summary lines contradict
  the constant doc, the code, and the test in their own file.
- **Severity:** major by rubric (asserts a false constraint), boundary-
  only in impact.
- **Proposed text (both sites):** "Reject a store whose round-trip
  efficiency is strictly below [`LP_RTE_FLOOR`] (η ≥ 1e-3 is the
  accepted region)."

---

## MINOR (stale detail / wrong cross-reference / omitted term)

### m1. PerfectForesight described as a future `DispatchPolicy` implementation — contradicted by the adopted D12 design
- **Where:** `grid-adequacy/src/policy.rs:342-346` (trait doc:
  "Implementations: [`RuleBased`] … and — Stage 7 — `PerfectForesight`")
  and `grid-adequacy/src/lib.rs:6` ("pluggable per ADR-6: `RuleBased`
  and — Stage 7 — `PerfectForesight`").
- **Actual:** lp.rs:15-18 (the adopted D12 design): the LP "is
  deliberately NOT a [`crate::DispatchPolicy`]" — it is a whole-horizon
  function (`run_multi_lp`), and no PerfectForesight policy exists.
- **Proposed text:** "Implementations: [`RuleBased`] (the default). The
  perfect-foresight LP is deliberately NOT a policy — it is the
  whole-horizon function [`crate::run_multi_lp`] (D12); results for
  headline claims are reported under both, the gap a documented
  finding."

### m2. `SystemState::must_take` doc omits the folded link net position
- **Where:** `grid-adequacy/src/policy.rs:250-252`.
- **Claim:** "weather-driven renewables at CF × capacity plus exogenous
  traces (rule 1)".
- **Actual:** under `run_multi` the zone's net link position is folded
  in before the policy sees it (`multizone.rs:1014`,
  `must_take = self.must_take + self.link_net`).
- **Proposed text:** "…plus exogenous traces (rule 1) — and, in a
  multi-zone run, the zone's net link position (imports positive;
  links clear before storage, multizone.rs step 3)."

### m3. Demand formula stated three ways, each incomplete
- **Where:** `grid-adequacy/src/inputs.rs:22-23` (omits
  `extra_profiles`), `grid-core/src/scenario.rs:983-985` (`DemandSpec`
  struct doc, omits `extra_profiles`), `grid-core/src/scenario.rs:999`
  (`extra_profiles` field doc, omits `heating(t)`).
- **Actual:** `inputs.rs:189-227`:
  `demand(t) = (base(t) + Σ extras(t)) × annual_scale + extra_demand_gw + heating(t)`.
- **Proposed text:** use that one canonical formula at all three sites.

### m4. Priced-ladder secondary misdescribed as "fractional utilisation of the marginal rung"
- **Where:** `grid-core/src/scenario.rs:22` (module doc) and
  `scenario.rs:1663-1664` (`FlowSignal::PricedLadder` doc).
- **Actual:** the secondary is the FULL scarcity score — ladder index +
  fractional utilisation, with the −surplus and 6+unserved regions —
  retained everywhere (`flow.rs` prose 1b; `PricedZoneCurve::signal`,
  flow.rs:394-406).
- **Proposed text:** "the D11 lexicographic signal: (per-zone marginal
  SRMC primary, the Stage 5 scarcity score secondary)".

### m5. grid-cli main.rs — three stale statements
- **Where:** `grid-cli/src/main.rs:22` ("1 model infeasibility (later
  stages)"), `main.rs:4-5` (subcommand list omits `stability`),
  `main.rs:18-20` ("a docs/06 subcommand-list addition, noted for the
  supervisor since docs are not edited from here").
- **Actual:** exit 1 is implemented (`main.rs:110-113`,
  `solve::Failure::exit_code`); docs/06 lines 34-37 now list
  `stability` (ratified 2026-07-03), so the "noted for the supervisor"
  caveat is resolved.
- **Proposed text:** drop "(later stages)"; add `stability` to the
  subcommand list; replace the caveat with "(docs/06 subcommand list
  updated and ratified 2026-07-03)".

### m6. multi_year.rs — a captured-output skip described as "loud"
- **Where:** `grid-adequacy/tests/multi_year.rs:456-457` ("Skip
  (loudly) when any referenced trace file is absent") around the
  `eprintln!` + `return` at multi_year.rs:461-471.
- **Actual:** cargo captures test stderr and shows it only on failure or
  under `--nocapture`; in a normal `cargo test` run this skip is
  invisible — the test passes green with nothing validated. The section
  header (multi_year.rs:436-441) honestly delegates loud failure to
  `acceptance_stage3_rs37y.rs` (which does fail loudly,
  acceptance_stage3_rs37y.rs:86), so the design is sound; the word is
  wrong. This is the acceptance_b4_lp silent-skip class, one notch
  milder because the doc points at the loud twin.
- **Proposed text:** "Skip (with a captured note — visible only under
  `--nocapture`; the Stage 3 part 2 acceptance twin fails loudly
  instead) when any referenced trace file is absent."

---

## NOTES (imprecise but harmless)

### n1. "perfect_foresight is Stage 7" phrasing now under-describes reality
- **Where:** `grid-adequacy/src/dispatch.rs:87-88` and `:134-136`
  (run doc), `dispatch.rs:152` and `multizone.rs:237` (error strings
  "(Stage 7; the engine implements rule_based)"),
  `multizone.rs:224-225` (run_multi doc).
- **Actual:** still true for the `DispatchPolicyKind` routing (the enum
  value is rejected), and Stage 7 IS "Cost synthesis and LP dispatch
  mode" (docs/04:296) — but the perfect-foresight LP now exists and runs
  via `run_multi_lp`, so "is Stage 7" reads as "does not exist yet".
- **Proposed text:** "…(`perfect_foresight` is not routed through this
  field; the perfect-foresight LP runs via
  [`crate::run_multi_lp`] — D12/Stage 7)".

### n2. `DispatchPolicyKind::PerfectForesight` variant doc
- **Where:** `grid-core/src/scenario.rs:1698-1700`.
- **Actual:** the variant is declared but rejected by every engine
  entry point; the LP is invoked directly.
- **Proposed text:** "LP over the horizon (HiGHS via `good_lp`, Stage
  7). Declared but not yet routed: engines reject it; the LP runs via
  `grid_adequacy::run_multi_lp`."

### n3. sweep.rs input-reuse comment narrower than it reads
- **Where:** `grid-adequacy/src/sweep.rs:614-615` ("Rebuild the demand
  trace only when a dimension changed the demand knobs").
- **Actual:** the code tests `annual_scale` only (sweep.rs:618) —
  correct today because `extra_demand_gw` is not a sweepable dimension,
  but the comment implies a general knob check.
- **Proposed text:** "Rebuild the demand trace only when `annual_scale`
  changed (the only sweepable demand knob; `extra_demand_gw` has no
  Dimension variant)."

### n4. multizone.rs "single-zone dispatch rules, unchanged" — consequence sentence is fine; no defect found
- Verified: exports served by stack only, storage sees post-trade
  position (`multizone.rs:1014` fold, flow.rs export bound). Recorded
  here as checked-clean because it is the most quoted step-3 claim.

### n5. Verified-clean list (for the manual's benefit)
- flow.rs prose rules 1-6 vs `ZoneCurve`/`PricedZoneCurve`/both walks:
  consistent (signal regions, boundary probe conventions, loss
  handling, borders grouping, first-appearance order).
- lp.rs formulation section vs constraint construction: consistent
  (balance, `√η` dynamics matching `StoreState::apply` exactly,
  variable order, determinism settings; determinism/oracle threshold
  1e-9 GWh matches `solve.rs::is_feasible`).
- dispatch.rs/multizone.rs two-tier validation comments vs code:
  consistent, including the "no-op under RuleBased / digest unmoved"
  claims.
- scenario.rs `validate()` doc checklist vs code: consistent (loss
  [0,1), availability [0,1], v6 capability physicality, v7
  priced-ladder pricing-on-every-zone, heating rules).
- units.rs conversion-point docs vs impls: consistent.
- acceptance pack-gating: `require_packs` asserts loudly in
  acceptance_b4_lp, b4_3zone, b6/b4_robustness, d11_priced_ladder,
  d11_sweep, d13_composed, d13_60gw, acceptance_2024 family — the
  claims "FAILS LOUDLY, no silent skip" are true at every site checked;
  the sole quiet skip in the tree is multi_year.rs (m6), whose doc
  delegates correctly.
- acceptance_b4_lp.rs mask/denominator claims (17,235 = 17,277 − 42
  sentinels; both conventions stated) match the in-test assertions.
- policy.rs:421 digest `779d7444…` — pinned at
  `grid-cli/tests/regression_2024.rs:50`.
- policy-boundary "no-future invariant … asserted by the
  policy-boundary test" — the test exists (`tests/storage.rs`, module
  doc bullet + implementation).
- multizone.rs "bit-identical to `crate::run` (pinned by test)" —
  `tests/multizone.rs:199`
  (`single_zone_run_multi_is_bit_identical_to_run`).
- LP determinism "pinned by test" — `tests/lp_dispatch.rs:453`,
  `tests/lp_rolling.rs:251`, `tests/lp_solve.rs:320`.

### n6. grid-stability / grid-cli spot-check
- grid-stability lib.rs / swing.rs / inertia.rs module docs: every
  numeric claim carries a citation (ESO appendices Q42, evidence-report
  §3.5, acceptance-test pins); no drift found at spot-check depth.
- grid-cli run.rs metadata claims (engine git hash, scenario hash,
  per-data-file SHA-256 in CSV header + Parquet footer) match the code
  (run.rs:73-101, 242-251, 381-397).

---

## Quotable numbers stated inline — pin/citation audit

Compliant (points at a pin or committed source):
- flow.rs:108 "+1.04 TWh on 2024 — pack report §3"
  (entsoe-stage5-pack-report.md §3/§157).
- policy.rs:421 digest `779d7444…` → regression_2024.rs:50.
- policy.rs:80-82 "~15-20 % store-side" → D4 note (cited).
- lp.rs:148-152 tractability numbers (~1.75 M / 3.25 GB solved; ~3.5 M
  aborted) → d12-lp-tractability.md (cited).
- acceptance_b4_lp.rs constants (0.3586 / 0.0195 / 0.2816 / 0.2346) are
  themselves the pins, with dated measurements.
- inputs.rs:25 "≈ 0.667 GW" — value lives in
  scenarios/gb-2024-reference.toml:136 and is guarded by the
  acceptance_2024 characterisation test, but the comment cites nothing.
  ADD the pointer: "(gb-2024-reference.toml `extra_demand_gw`, guarded
  by the acceptance_2024 calibration characterisation test)".

Lacking a pin or pointer (fix package should add one or soften):
1. `grid-adequacy/src/dispatch.rs:64` — "gas is the marginal fuel in
   ≈ 99 % of periods (validation pack report §5)": cites the report but
   no regression test pins the 99 % figure that underwrites the
   fixed-merit-order argument.
2. `grid-adequacy/src/dispatch.rs:77` — "understating coal by its full
   1.57 TWh": no inline citation; source is
   2024-validation-pack-report.md:45. Add "(validation pack report
   §2)".
3. `grid-core/src/pricing.rs:39-41` and `:370-371` — "negative in 495
   half-hours of 2024 (down to −£61/MWh)" / "the 2024 MID has 495
   negative periods": stated twice, no test pins 495 and no report
   section is cited (grep of acceptance_stage2_2024.rs and
   grid-core/tests/pricing.rs finds no pin). Add a citation to the
   price-pack report or pin it where the stage-2 realism stats are
   computed.

---

## Suggested fix order

M2 first (policy prose is the hard rule; needs the D4-note coordination
decision), then M1/M4 (pure doc rewrites), M3 (decide: soften the claim
or change the sweep loader), then the minors as one mechanical commit,
then the quotable-number pointers.
