# D4 — Rule-based storage dispatch policy (ADOPTED 2026-07-02, post-review)

The prose specification required by docs/04 Stage 3 and CLAUDE.md ("write
the rules in prose first, then code"). This is the most contestable
modelling choice in the tool (risk R2): storage-requirement numbers — the
book's central quantitative claims — are downstream of these rules. The
defences are ADR-6's (the policy is pluggable; results are reported under
both this policy and the Stage 7 perfect-foresight LP; the gap is a
published finding) plus this document: every rule stated, every
consequence owned. Reviewed and adopted with edits 2026-07-02; the
review's adversarial walk-throughs are part of the decision record.

## Design stance

**Greedy, chronological, zero foresight.** The policy sees only the
current period and the current store state. Enforced structurally:
`RuleBased` reads only the current `SystemState` (period inputs + store
state vector). The `Horizon` argument the shared ADR-6 trait requires
carries calendar metadata only, never trace data; the no-foresight
property is a documented invariant of what `SystemState` contains,
verified by the policy-boundary test — not a consequence of the trait
shape, which must also serve the `PerfectForesight` LP. Rationale:
(a) real operators do not know next month's weather, so no-foresight
storage requirements are the honest upper envelope; (b) any smarter
heuristic (price arbitrage, forecast horizons, reserve targets) imports
assumptions a critic can attack — those belong in the LP policy, where
they are explicit in the objective function; (c) ADR-6 chose this as the
default precisely because it yields *higher and more defensible* storage
numbers.

## The rules

> **ERRATUM (2026-07-06, comment-consistency sweep M2 —
> `docs/notes/comment-consistency-sweep.md`).** Rule 1 below, as
> adopted, lists "must-run plant at availability" in the must-take
> supply. That clause has NO engine representation and never did: in
> the engine, must-take is weather-driven renewables plus exogenous
> traces ONLY (`dispatch.rs` rule 1, `multizone.rs` step 1), and there
> is no must-run category. Nuclear — and any low-rung thermal — is the
> bottom rung of the merit-order stack (rule 3): it runs only against
> deficits and is backed down whenever residual demand is below its
> available ceiling. The ENGINE behaviour is canonical, not the prose:
> it is the validated instrument — the Stage 1 honesty gate passed
> against observed 2024 with it, and every committed digest and pinned
> number was measured with it. The prose is corrected here, dated,
> never silently.
>
> Corrected rule 1: **Must-take supply dispatches first:
> weather-driven renewables (CF × capacity) and exogenous traces.
> Compute `net = must_take − demand`. If `net = 0`, no storage
> action.** (Must-run-like behaviour arises only from merit position,
> and such plant runs only against deficits.)
>
> Divergence direction, owned: versus a must-run treatment the engine
> supplies LESS in deep-surplus periods, so it slightly UNDERSTATES
> curtailment and storage charging — anti-conservative for curtailment
> findings. Measured bound (2026-07-06): on `gb-2024-reference`,
> nuclear is backed down in 116/17,568 periods (max 5.15 GW) and total
> curtailment is 0.1367 GWh — pinned by
> `nuclear_backdown_periods_on_the_2024_reference`
> (`grid-adequacy/tests/acceptance_2024.rs`); on the 2-zone run, SCO
> back-down in 3,570 periods, max 1.05 GW. The bound grows with
> nuclear share (Q7-relevant, below).
>
> The same erratum corrects rule 3's Q7 parenthesis ("Q7's nuclear is
> must-run, so its surplus charges storage via rule 1/2" — marked
> below): under the engine, nuclear surplus NEVER charges storage; the
> plant backs down instead. So Q7 is NOT covered by the "unaffected"
> list as argued: its rule-based runs will understate storage charging
> (and nuclear's storage-displacement value) in deep-surplus periods,
> increasingly with nuclear share. Q7 work must own this limitation or
> use the LP policy. No committed Q7 result exists (checked
> 2026-07-06: Q7 appears only in the research syllabus and the papers
> plans — no scenario, run, pin, or quotable relies on the must-run
> treatment).

Each half-hour, in order:

1. **Must-take supply dispatches first**: weather-driven renewables (CF ×
   capacity), must-run plant at availability, exogenous traces. Compute
   `net = must_take − demand`. If `net = 0`, no storage action.
   *[Superseded — see the 2026-07-06 ERRATUM above for the corrected
   rule.]*
2. **Surplus (`net > 0`) → charge.** Stores charge in ascending
   `dispatch_order`, each absorbing
   `min(remaining surplus, power rating, headroom-limited intake)`.
   Surplus no store can absorb is **curtailment**.
   `dispatch_order` values must be unique within a zone; scenario
   validation rejects duplicates.
   - *Charging draws from surplus only — never from the thermal stack.*
     (One narrow, stated exception: DSR load repayment, below.) Charging
     from gas to store for later is an economic bet on future scarcity; a
     policy without prices or foresight cannot justify it, and allowing
     it would contaminate the storage-requirement question with an
     arbitrage model. It also matches the Royal Society's own methodology
     (electrolysis from surplus only), which matters for the replication.
     (Consequence, owned: the rule-based policy cannot pre-charge ahead
     of a forecast drought. The LP can. That gap is a designed finding.)
3. **Deficit (`net < 0`) → the non-storage dispatchable stack runs
   first** (Stage 1 merit order, at availability). Any deficit remaining
   after the full stack → **discharge** stores in ascending
   `dispatch_order`, each supplying
   `min(remaining deficit, power rating, SoC-limited output)`. Deficit no
   store can cover is **unserved energy**.
   - *Storage is the reliability backstop, not a price arbitrageur.* For
     adequacy this is in fact the feasibility-optimal discharge order —
     SoC is preserved for deficits that exceed the stack — so on the
     headline scenarios it moves rule-based *toward* the LP (good for
     kill criterion 3). In thermal-rich scenarios it means rule-based
     storage barely cycles — a known, documented behaviour (real
     batteries cycle daily on price, which this policy has no concept
     of). **Owned consequence for Q3 (docs/07):** under this policy,
     added storage can never displace gas, so storage's marginal effect
     on emissions is structurally zero in mixed fleets —
     carbon-constraint sweeps (Q3) must vary fleet composition or use
     the LP policy; they cannot be run under `RuleBased` by varying
     storage alone. Stage 3's own research questions (M3, M4, Q1, Q6,
     Q7) are unaffected: their scenarios have small or absent thermal
     stacks (Q7's nuclear is must-run, so its surplus charges storage
     via rule 1/2 *[WRONG — see the 2026-07-06 ERRATUM above: nuclear
     has no must-run representation; it backs down in surplus and its
     surplus never charges storage, so Q7 does not belong on this
     "unaffected" list]*). Stage 1's validated 2024 gas numbers are
     untouched by construction.
4. **No reserve holding.** A store discharges for today's small deficit
   even if tomorrow's is fatal, and charges toward full even if the store
   will overflow tomorrow. Greedy means greedy. A reserve-floor variant
   is explicitly designated the **kill-criterion-3 fallback sensitivity**:
   if the rule-based vs LP gap approaches an order of magnitude on a
   realistic scenario, the reserve-floor variant is the first named
   experiment — as a sensitivity, not a silent default change.

## Mechanics (normative details the code must document)

- **Efficiency split**: round-trip efficiency η splits symmetrically —
  `η_charge = η_discharge = √η`. SoC accounting: charging adds
  `power × Δt × √η`; discharging removes `power × Δt / √η`. Standard
  convention when only round-trip η is carried (ADR-8 v1). *Comparison
  convention, owned:* the Royal Society quotes store-side TWh under
  asymmetric per-leg efficiencies (electrolysis ≈ 0.7, reconversion
  ≈ 0.5); the symmetric split can shift store-side headline figures by
  up to ~15–20 %. The "published order of magnitude" acceptance band
  absorbs this; per-leg efficiencies are a schema-v2+ candidate if a
  published claim ever quotes store-side TWh.
- **Power ratings symmetric** for charge/discharge (ADR-8 v1).
- **`dispatch_order` is scenario data, policy obeys it** (uniqueness
  enforced, rule 2). Preset guidance (not enforced): shortest duration
  first — battery 1, pumped hydro 2, hydrogen 3 — so fast stores soak
  diurnal swings and the seasonal store fills on sustained surplus. The
  same ascending order is used for charge and discharge (one knob,
  explainable; asymmetric orderings are an LP-policy luxury).
- **Initial SoC: full, by default** (scenario-overridable when the
  schema gains the field), **with a guard**: if a bisection run's
  minimum SoC falls within the first weather year, the result is flagged
  initial-condition-sensitive and re-run with a one-year burn-in (the
  requirement measured excluding year 1's initial transient); both
  figures reported. This matters concretely: the weather record starts
  1985-01, a plausible design-drought winter, and detection without
  correction would leave the headline number quietly low.
- **SoC carries across year boundaries** (docs/04 Stage 3: multi-year
  continuous). No annual reset — resets are exactly how "a few days of
  storage" errors happen.
- **Determinism**: the decision is a pure function of
  (period inputs, store state vector), where store state = SoC plus,
  for DSR, its volume-used-today and outstanding-repayment bookkeeping —
  all explicit, serialisable state (ADR-5-clean). No randomness, no
  wall-clock, no hidden memory.

## DSR pseudo-storage (v1, PROVISIONAL — binding only when Q6 work begins)

docs/04 Stage 3 acceptance tests do not cover DSR; this section becomes
binding when Q6 work starts and may be refined then. v1 semantics:

- A store with η = 1, `energy = power × shift_duration`, representing
  deferrable load. *Discharge* (load reduction) is capped per **UTC
  calendar day** by `daily_volume_limit`.
- **Repayment** (recovering the deferred load): at the first subsequent
  opportunity, from surplus *or from thermal headroom* — the one stated
  exception to rule 2's surplus-only charging, justified because
  repayment is deferred load that must be served, not an arbitrage
  purchase. Repayment is capped by (surplus + thermal headroom) in the
  period; it never triggers another store's discharge and never causes
  unserved energy.
- **Deadline**: repayment is due within `shift_duration` of the
  deferral. If no opportunity arrives in time, repayment happens at the
  first subsequent opportunity regardless, and the deadline violation is
  counted and reported as a run output — a visible model-fidelity
  metric, never a silent constraint break. When a deadline would
  otherwise be violated, DSR repayment takes priority over other stores'
  charging in that period; otherwise ascending `dispatch_order` applies.
- Explicitly a coarse model: real DSR is behavioural and price-driven;
  v1 exists so Q6 can show DSR clears the diurnal band and nothing else.

## What this policy is NOT claimed to model

Price-driven cycling, ancillary-service commitments, degradation,
minimum-SoC warranties, forecast-based pre-positioning, pumped-storage
head effects, hydrogen storage geology. The Stage 7 LP policy addresses
the first and the pre-positioning; the rest are out of scope for the
tool (docs/05 model boundaries).

## Kill-criterion-3 posture (why the greedy/LP gap should stay bounded)

On the headline RS-style scenarios (wind+solar+storage, no thermal
stack), greedy charge-on-surplus / discharge-on-need is close to
feasibility-optimal: with no stack and no prices, the LP's remaining
advantages are cross-store allocation (the shortest-duration-first
cascade loses little) and reserve positioning (irrelevant when discharge
is need-driven) — tens-of-percent mechanisms, not 10×. The
order-of-magnitude risk concentrates in mixed thermal scenarios where
the LP pre-charges from gas; those are not the headline
storage-requirement scenarios, and Stage 7's `LP ≤ rule-based` invariant
plus per-scenario gap reporting will surface any surprise. Fallback:
rule 4's reserve-floor sensitivity.

## Acceptance hooks (Stage 3 tests this prose binds)

- SoC conservation property test: every period,
  `ΔSoC = charge×√η×Δt − discharge×Δt/√η` exactly; SoC ∈ [0, capacity].
- Charge/curtailment and discharge/unserved exclusivity: curtailment > 0
  only when every store is full or power-saturated; unserved > 0 only
  when every store is empty, power-saturated, or volume-capped (DSR).
- Duplicate `dispatch_order` within a zone → scenario validation error
  (test).
- Single benign year + 12 h battery → zero unserved (the "few days"
  claim's home turf, reproduced before being dismantled).
- Royal-Society-style scenario (wind+solar+hydrogen, 37+ years) →
  storage requirement of the published order (tens of TWh),
  regression-pinned, with the initial-SoC guard applied.
- Policy-boundary test: `SystemState` passed to `RuleBased` contains no
  data from periods beyond the current one (documented invariant,
  asserted in test — not a trait-shape guarantee).
- DSR hooks (deferred until Q6 work): daily discharge ≤
  `daily_volume_limit` per UTC day; deadline violations counted and
  reported.
