# 04 — Staged Implementation Plan

Each stage is a self-contained work order: scope, explicit non-goals,
acceptance tests, demo artefact. **A stage is complete when its acceptance
tests pass — not before, not "mostly".** Coding sessions get this document's
relevant stage plus `02-architecture.md`, `03-domain-model.md`,
`06-conventions.md`.

Tolerances marked `TBD-DATA` must be filled in when the validation pack
(`05-validation.md`) is assembled — before Stage 1 begins.

---

## Stage 0 — Scaffold and domain model

**Scope:** Cargo workspace (`grid-core`, `grid-adequacy`, `grid-stability`,
`grid-cli`); newtype unit system; scenario TOML parsing with
`schema_version`; weather trace loading (one year); `fetch-data` CLI stub.

**Non-goals:** any dispatch logic.

**Acceptance tests:**
- Scenario file round-trips (parse → serialise → parse) losslessly.
- Unit arithmetic: `Power(2.0) * Duration::half_hour() == Energy(1.0)`;
  dimensionally invalid operations fail to compile (doc-tests).
- Loads a real 2024 half-hourly demand + wind CF trace, correct period count
  (17,568 for a leap year), UTC-clean across clock-change dates.

**Demo artefact:** CLI prints a scenario summary and trace statistics.

---

## Stage 1 — Single-zone merit-order dispatch (no storage)

**Scope:** chronological half-hourly dispatch: renewables as must-take,
merit-order stack by SRMC, unserved energy and curtailment accounting,
availability models (flat + nuclear profile).

**Non-goals:** storage, pricing outputs, multi-zone.

**Acceptance tests (the honesty gate — everything downstream inherits from
this):**
- 2024 actual fleet + 2024 weather + 2024 demand reproduces:
  - annual gas generation within **±5 %** of Elexon actuals (72.79 TWh, so
    ±3.6 TWh) — *conditional on the validation harness handling the three
    quantified correctable wedges: station load ≈ 5.9 TWh/yr (D3/demand
    convention), pumped-storage round-trip loss 0.60 TWh, coal-closure
    windowing 1.57 TWh. The remainder (station-level outage structure,
    embedded-estimate error ~1–2 TWh) supports ±5 %; ±2 % is not evidenced.
    Evidence: `docs/notes/2024-validation-pack-report.md` §7;*
  - net annual imports within **±1 %** (imports as exogenous trace in this
    stage, so this is a trace-ingestion check, not model skill: actual
    33.30 TWh net; known data defects ≤ 0.02 TWh ≈ 0.06 %. The modelled-
    imports tolerance is a Stage 5 question);
  - monthly generation mix correlation ≥ **0.99** (flattened 12×fuel
    absolute matrix, both sides in the D3 total-generation convention.
    Originally ≥ 0.95, set just outside the 0.973 mixed-convention
    ceiling; tightened 2026-07-02 after the first Stage 1 run: a
    zero-skill flat-output model scores 0.934, so 0.95 barely excluded
    zero skill, and the run achieved 0.997 —
    `docs/notes/stage-1-2024-run-report.md` §2/§4).
- Zero unserved energy for 2024 (as in reality).
- Determinism: identical results across runs and platforms (hash of output).

**Demo artefact:** modelled vs. actual 2024 monthly generation stack chart.

---

## Stage 2 — Pricing layer

**Scope:** SRMC model (fuel, efficiency, carbon price); system marginal price
per period; per-technology revenue and capture price; emissions accounting.

**Acceptance tests (pinned 2026-07-02 from
`docs/notes/2024-price-pack-report.md`; gate 1 re-pinned same day after
the first Stage 2 run — history recorded here deliberately):**
- % of periods with gas (CCGT/OCGT) flagged price-setting by the model
  falls within the observable's own definition band **[89.4 %, 99.8 %]**,
  with the exact model value **regression-pinned (93.89 %)** and both
  framings reported (behavioural vs price-consistent ≈64 %).
  *Re-pin record: the original gate (±3 points of 99.4 %, the 5–95
  flexing proxy) was mis-pinned — the pack report §4 had warned ±3 does
  not cover the proxy's definition spread — and is unsatisfiable by
  dispatch arithmetic: the Stage 1 engine (correctly frozen) dispatches
  zero gas in 6.11 % of 2024 periods, capping the model's share at
  93.89 %. The causal model boundary is that CCGT minimum-stable
  generation is not modelled: observed CCGT reached zero in only 9 of
  17,568 periods. Reviewer-verified: the original gate is also jointly
  unsatisfiable with the capture-ratio gate (frontier capture ≥ 0.956 at
  96.4 % share vs ceiling 0.949). Like-for-like comparison pinned as a
  documented boundary metric, not gated: model 5–95 flexing statistic
  85.7 % vs observed 99.4 %.* *The "~97 %" claim is corrected from data
  AND model: on this model gas sets the price in ≈94 % of periods, not
  97–99 %; published claims must say which statistic and whose definition
  is meant (kill-criterion 4 record:
  `docs/notes/stage-2-2024-run-report.md`).*
- Wind capture price / baseload price ratio for 2024 (model prices, model
  wind) within **±0.05 of the observed 0.899** (MID price, D3 total
  wind). Evidence for the width: switching to modelled-wind weighting
  alone moves the ratio −0.023 (0.875), price-series choice +0.005,
  monthly spread ±0.07 — price-pack report §5. *Passed at 0.9413, 0.0077
  from the band edge; the like-for-like error vs the ERA5-weighted
  benchmark is +0.066 (model SMP has little within-day shape and no
  negative prices) — validation content is thin and the stage-2 run
  report says so.*
- Model marginal-price realism (promoted from reported to gated,
  2026-07-02, reviewer ruling — both carry genuine model content):
  median model-SMP / observed-MID within **[0.90, 1.10]** (measured
  1.0100); monthly model-vs-observed price correlation ≥ **0.85**
  (measured 0.9517).

**Demo artefact:** **Module 1 chart** — % hours gas-marginal vs. wind
capacity (10→60 GW sweep, manual loop acceptable pre-Stage 4).

---

## Stage 3 — Storage portfolio and rule-based dispatch

**Scope:** `Vec<Storage>` with dispatch order; `DispatchPolicy` trait;
`RuleBased` implementation (spec the rules explicitly in code docs: charge
priority on surplus, discharge priority on deficit, reserve behaviour);
bisection solver `min_storage_for_zero_unserved`; multi-year continuous runs
(SoC carries across years).

**Non-goals:** perfect-foresight LP (Stage 7).

**Acceptance tests:**
- Energy conservation: SoC trajectory consistent with flows × efficiency
  every period (property test).
- Bisection solver on a Royal-Society-style scenario (wind+solar+hydrogen,
  37+ weather years) produces a storage requirement of the published order
  of magnitude (tens of TWh); exact figure recorded as a regression test.
- Single benign year + 12 h battery: zero unserved (the "few days" claim's
  home turf, reproduced before being dismantled).

**Demo artefact:** 40-year hydrogen SoC trace showing multi-week drawdowns
and multi-year recharge — the anti-"few days" chart.

---

## Stage 4 — Sweep runner, rayon, timescale decomposition

**Scope:** parameter sweep infrastructure (`par_iter`, full response surfaces
persisted, not just optima); residual load utilities; timescale decomposition
(diurnal/synoptic/seasonal/inter-annual band attribution); per-year batch
mode (Q4).

**Acceptance tests:**
- Sweep results identical to serial execution (determinism under rayon).
- Decomposition bands sum to total storage requirement within numerical
  tolerance.
- Benchmark: full 40-year single-zone run < `N` ms (set in
  `06-conventions.md`); 10⁴-point sweep < 1 min on reference hardware.

**Demo artefacts:** **Module 3** decomposition chart; **Module 4**
storage × overbuild surface with iso-cost contours.

---

## Stage 5 — Multi-zone and interconnectors

**Scope:** activate `Vec<Zone>` + link matrix in the engine; **five
external zones per D5** (*amended 2026-07-03 from "FR, NO, NL/BE/DE";
ADR-7 amendment proposed in project-state; adjudicated decision:
`docs/notes/d5-zone-granularity.md`*): FR, CONT-NW (BE+NL+DE-LU
aggregate; internal copper plate is a stated v1 convention — DE-LU is
70 % of the bloc's load, so the bloc scarcity signal is effectively
German), NO2 (hydro-driven from the ENTSO-E evidence, no weather CF;
NO2 wind 4.52 TWh absorbed in the zone energy balance at calibration,
never silently dropped), DK1, IE-SEM (IE fossil_oil 1.59 GW mapped to
a peaker technology or explicitly justified, D5 ruling e). External
zones carry own demand, fleet, weather traces (per-country CF traces
aggregated to CONT-NW at scenario level); imports/exports emerge from
relative scarcity and price; link availability.

**Acceptance tests (tolerances pinned 2026-07-03, pre-model, from
`docs/notes/entsoe-stage5-pack-report.md` + the D5 adjudication; each
gate names its causal boundary per the Stage 2 lesson — if a gate
proves structurally unsatisfiable, the resolution is a reviewer-ruled
re-pin with the boundary named, never input tuning):**
- **A1 (GB re-validation):** the Stage 1 2024 gates re-pass with
  modelled (not exogenous) imports — gas annual within the Stage 1
  band, monthly mix corr ≥ 0.99 — and modelled GB net imports land
  within **±10 % of 33.30 TWh** (±3.3 TWh; boundary: scarcity-rule
  fidelity + CONT-NW copper plate; the cross-source metering wedge is
  +1.04 TWh systematic, so tighter than ±3 % is not
  evidence-supportable pre-model).
- **A2 (direction, GB↔FR only per D5 ruling a; RE-PINNED 2026-07-03
  after the adjudicated escalation — original pin superseded, retained
  below per the Stage 2 amendment pattern):** two limbs, both
  required, at the w=1 FR release-envelope convention (cumulative
  modelled FR hydro release never runs ahead of cumulative observed;
  release timing below the envelope stays scarcity-driven; the GB↔FR
  flow stays emergent; the gate content is flow direction, not hydro
  seasonality — ownership language at the scenario definition site):
  - **A2a:** modelled GB↔FR flow direction matches observed in
    **≥ 88 %** of 2024 periods (50 MW dead-band; exact pin 90.07 % =
    15,823/17,568);
  - **A2b:** the model recalls **≥ 70 %** of observed GB→FR export
    periods (exact pin 78.96 % = 1,036/1,312).
  A2a alone sits BELOW the 92.30 % "always import" base rate — stated
  openly; the limb pair strictly dominates the base-rate predictor,
  which scores 0 % on A2b. Boundary (measured, reviewer-adjudicated):
  86.5 % of residual mismatches are both-zones-gas-marginal periods,
  night/shoulder-concentrated (23–05 UTC), where the unpriced common
  ladder cannot see the FR–GB fuel/carbon price asymmetry — the
  grid-adequacy `flow` module's named limitation (mechanism b).
  Remediation history (both rounds measured before any re-pin,
  docs/notes/stage-5-review.md adjudication): the observed FR hydro
  budget and observed FR non-GB export series eliminated the
  mechanism-(a) mismatch categories entirely (FR unserved 0.000 TWh);
  the weekly budget grain was REJECTED for FR — greedy front-loading
  scored below the flat model it replaced (recorded in the run
  report). **The superseded ≥ 95 % pin remains the acceptance target
  for the priced-ladder flow rule (Stage 7-adjacent, ADR-9 SRMC
  machinery); measured expectation once the both-gas class is priced:
  97.4 %.** *Measured outcome (D11 engine package, 2026-07-05): the
  target was attempted and is UNREACHABLE on 2024 prices — the
  pre-registered D11 rule-4 finding, not a defect. 86.5 % of the
  residual class is both-gas-marginal in a carbon-parity year (static
  ceiling 93.84 %); the ladder measures 71.69 % on the committed
  carbon convention / 93.18 % flat-flat, both pinned as ladder-only
  findings with the committed scarcity-rule gates unchanged. See
  docs/notes/d11-a2a-mismatch-characterisation.md and
  docs/notes/d11-engine-review.md.* No per-border direction gates on BE/NL (structural
  opposite-sign floor ≥ 13.57 % under the aggregate zone); bloc-level
  direction is a reported diagnostic.
- **A3 (sign test, resource-level reformulation per the ENTSO-E pack
  review):** modelled NO2 hydro generation vs GB wind CF **|r| ≤
  0.15** (observed −0.087; NOTE the engine-side limb is
  near-tautological — the seasonal-budget hydro driver is
  wind-independent by construction — so the gate is framed as
  *reproducing the observed structure*, and the load-bearing limb is
  the continental one); modelled continental (FR + CONT-NW) imports vs
  GB wind CF **r ≤ −0.25** half-hourly (observed −0.352) — the
  anticyclone result. Diagnostic (reported, not gated): NSL modelled
  flow vs GB wind within ±0.15 of the observed −0.399.
- **A4 (per-border energy, D5 ruling a):** BE and NL per-border annual
  net energies within **±1.5 TWh** each of the NESO actuals (BE +4.16,
  NL +1.59 TWh); all five borders' modelled-vs-observed table
  published in the run report. Revisit trigger for the CONT-NW copper
  plate: misses here (D5 ruling c).

**Demo artefact:** **Module 5** — interconnector capacity credit vs. GB
residual demand percentile.

---

## Stage 6 — Stability engine

**Scope:** `grid-stability`: aggregate inertia from dispatched synchronous
plant at any adequacy timestep; swing-equation loss-of-infeed simulation;
RoCoF and nadir vs. era-dependent limits carried as scenario inputs
(*corrected 2026-07-03 from the published record: LFDD stage 1 is
**48.8 Hz** — 49.2 Hz is the SQSS abnormal-loss floor, a different
standard; RoCoF relay limits are era-dependent: 0.125 Hz/s in 2019,
1 Hz/s post-ALoMCP, 0.5 Hz/s NESO design*); LFDD as staged demand
blocks (E3C stage table in `data/reference/stability-2019-event.toml`);
response services (dynamic containment, static) with volumes and delays;
minimum-inertia hour finder; pathway runner (fleet as function of year)
for Q8.

**Acceptance tests (tolerances pinned 2026-07-03 from
`docs/notes/stage-6-evidence-report.md` — primary-source-verified,
every checksum refetched in review):**
- Analytic check: single-machine case matches closed-form swing solution.
- Reproduce the 9 Aug 2019 event given the published loss sequence,
  response holdings, and inertia (210–219.6 GVA·s — the official record
  self-disagrees ~5 %; both bounds tested), **with stage-1 LFDD
  explicitly modelled** (931 MW block at 48.8 Hz, 0.2–0.5 s action
  delay — precondition: without it the nadir band is unreachable and
  T1 validates nothing):
  - **T1 (nadir)**: within the protection band **48.75 < f_min ≤
    48.80 Hz** (measured 48.787; tighter fits relay noise, looser
    crosses a protection boundary — the band is what the record pins);
  - **T2 (first arrest)**: **49.10 ± 0.10 Hz** (measured 49.083) — the
    genuine swing-physics discriminator;
  - **T3 (initial RoCoF)**: **±25 % of 0.144 Hz/s** over the pinned 2-s
    window (convention in the reference TOML);
  - **T4 (binary)**: a 1,000 MW loss under identical conditions stays
    ≥ 49.5 Hz (ESO's own published simulation + 1 Jul 2019 outturn).
  - Fault-to-LFDD time and recovery trajectory: diagnostic only (input
    circularity documented in the evidence report).
- Inertia at each hour equals Σ(H × MVA) of dispatched synchronous plant
  (H values and the MVA/GW power-factor convention:
  `data/reference/inertia-constants.toml`; the reliability
  classification's firm/variable cut does NOT coincide with
  synchronous/non-synchronous — mapping documented there).

**Demo artefacts:** hours/year below inertia threshold vs. renewable share
(**Module 6**); largest-survivable-loss vs. year under a FES pathway (Q8) —
"the year the grid can no longer ride through" as a date.

---

## Stage 7 — Cost synthesis and LP dispatch mode

**Scope:** whole-system cost stack (subsidy-clock-aligned): generation,
storage capex, overbuild, curtailment, interconnection, stability services,
constraint costs (ADR-12 approximation); LCOE calculator;
`PerfectForesight` policy via `good_lp` + HiGHS *(amended 2026-07-06:
the adopted D12 design implements perfect foresight as a whole-horizon
LP function — `run_multi_lp` and variants — deliberately NOT a
per-period `DispatchPolicy`; the ADR-6 trait is per-period/no-lookahead,
so the LP's physics are LP constraints and the D4 policy choices are
absent from its objective; the policy-contract mechanism remains for
future per-period dispatchers. See docs/notes/d4-policy-contract.md and
docs/notes/d12-perfect-foresight-lp.md.)*; rule-based vs. LP gap
reporting; scenario pack for published pathways (FES, CCC, Royal Society).

**Acceptance tests:**
- LP mode on a small hand-checkable scenario matches manual optimum.
- LP storage requirement ≤ rule-based on every scenario (sanity invariant);
  gap reported per scenario.
- Cost stack reconciles: Σ components = total, LCOE vs. delivered £/MWh gap
  fully decomposed (Q9).

*Pinned at Stage 7 opening (2026-07-03, supervisor under delegation;
method = D8 as adopted, `docs/notes/d8-lcoe-methods.md` — every rule
there binds this stage):*
- *Cost inputs for the new Stage 7 lines (capex, O&M, lives, WACC,
  trajectories, holding costs) come ONLY from
  `data/reference/costs-gb.toml` (reviewed evidence:
  `docs/notes/stage7-cost-inputs-report.md` +
  `docs/notes/stage7-cost-inputs-review.md`); the 2024 fuel/carbon
  actuals and the efficiency chain stay in `prices-2024.toml` per D8
  rule 1.2. Machine-readable `quotable = false` quarantine flags are
  load-bearing and PROPAGATE: any result that consumed a quarantined
  row is stamped non-quotable in its metadata, and the artefact layer
  refuses to emit such a result as a publishable artefact (battery
  split until NREL re-verification; NSL/NeuConnect/Greenlink; OCHT
  numbers past the publication gate without the Baringa check).
  Nuclear headlines quote BOTH variants per the bracket rule.*
- *WACC set 4.5 / 7.5 / 10.0 % real, uniform across technologies
  (D8 rule 4); price base 2024 GBP, ONS GDP-deflator series per the
  evidence note; gas/carbon trajectories = FFPA 2025 + traded carbon
  values 2025, low/central/high arrays as committed.*
- *LP-vs-manual acceptance: bit-exact where the optimum is integral,
  else within the solver's stated tolerance, both endpoints printed.*
- *Σ components = total is asserted with component lines RECOMPUTED
  INDEPENDENTLY of the total (D8 adjudication note: a shared-code
  reconciliation is a tautology); exact under determinism.*
- *Q9 decomposition = the D8 rule-6a three-wedge identity, exact;
  every published Stage 7 number carries its D8 rule-3 reliability
  stamp and rule-4 WACC band.*
- *D4 relaxation: engine-level D4 rule-2/3 validation moves to a
  policy contract so `PerfectForesight` can dispatch differently from
  RuleBased (tracked item, Stage 3 part 1 review) — designed and
  reviewed BEFORE the LP lands; the rule-based path must stay
  bit-identical (digest 779d7444… unmoved).*

**Demo artefacts:** **Module 7 / Q9** — LCOE vs. delivered system cost
decomposition chart per published scenario; rule-based vs. perfect-foresight
storage table.

---

## Phase two (separately specced, not scheduled here)

`grid-wasm` + web UI: sandbox mode (live sliders, weather-year selector,
click-an-hour → stability stress test), study mode (sweeps → charts),
scenario sharing. The UI acquires no logic the engine lacks (ADR-11).
