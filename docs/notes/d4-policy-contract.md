# D4 policy contract — the two-tier dispatch validation (Stage 7 documentation of record)

**Status:** documentation of committed behaviour, written Stage 7
(Q9/bridges package) to close the audit's documentation debt. It
documents the code AS IT IS after the D4 erratum — nothing here is a
proposal. Lineage: the step-1 policy-contract refactor (commit
`24733eb`, D12 rule 1, reviewer ACCEPT-WITH-NOTES), the deficit-charge
physical-tier guard the audit ordered (commit `c5d0bb8`,
mutation-verified), and the D4 erratum (commit `f401b9a` — must-take
NEVER included must-run plant; see below).

Authoritative code sites:
- `grid-adequacy/src/policy.rs` — `PolicyContract`, `DispatchPolicy`,
  `RuleBased`, and the D4 rules in prose (with the dated erratum
  banner).
- `grid-adequacy/src/dispatch.rs` (`run_with_policy`, the per-period
  validation block) — the single-zone enforcement.
- `grid-adequacy/src/multizone.rs` (`ZoneEngine::dispatch_period`) —
  the multi-zone twin; the tiers are enforced identically per
  zone-period.

## Why the split exists

The engine validates every storage-dispatch decision each period in
two tiers, and the two tiers exist for one reason: **so smarter
dispatchers can relax the rule-based policy's *choices* while the
physics stays enforced for everyone.** Before `24733eb`, D4 rules 2
and 3 (surplus-only charging, discharge-after-stack) were engine-level
checks: no policy could dispatch differently, which made the Stage 7
perfect-foresight LP impossible (a foresight dispatcher must be able
to pre-charge a store from the thermal stack ahead of a drought — a
decision D4 rule 2 forbids). The refactor moved those two checks into
a per-policy **contract** while leaving the physical laws
unconditional. The rule-based path is bit-identical across the
refactor: the 2024 reference digest `779d7444…` and the 2/3/5-zone
digests were pinned before and unmoved after (the refactor's
acceptance condition, verified at commit).

## Tier 1 — physical invariants (laws, enforced for EVERY policy)

Enforced unconditionally in `dispatch.rs` and `multizone.rs`
regardless of what a policy's contract declares. A violation is a
structured `GridError::InvalidDispatchDecision`, never silent
corruption:

1. **Per-store non-negativity** — no negative charge or discharge.
2. **No simultaneous charge and discharge** within one store-period.
3. **Ratings and state bounds** — `charge ≤ max_charge` (power rating
   and √η-adjusted headroom) and `discharge ≤ max_discharge` (power
   rating and √η-adjusted SoC); the SoC bounds inside
   `StoreState::apply` (dust-snapped at [0, capacity], error beyond
   rounding dust).
4. **Non-negative derived series** — curtailment may not go negative
   (energy conjured from nowhere: a policy charging beyond the surplus)
   and unserved may not go negative (masked load-shedding: discharge
   beyond the post-stack deficit).
5. **The deficit-charge guard (`c5d0bb8`)** — charging and unserved
   energy are **mutually exclusive within a period**. This closes the
   hole guards 1–4 and 6 do not: in a deficit period, a relaxed policy
   charging a store the stack cannot back raises the store's SoC on
   supply that never existed while the phantom charge inflates the
   *positive* `unserved` plug — both derived series stay non-negative
   and the conservation identity still balances, so only this dedicated
   guard catches it. Characterisation pins (proven RED without the
   guard):
   `a_relaxed_contract_may_not_conjure_energy_by_charging_in_a_deficit`
   (`tests/storage.rs`, single-zone) and
   `multizone_deficit_charge_with_no_supply_is_rejected`
   (`tests/multizone_deficit_charge_guard.rs`, multi-zone twin).
6. **Per-period energy conservation** — `must_take + stack_output +
   discharge + unserved == demand + charge + curtailment` (to
   1e-6-relative float tolerance). Because curtailment and unserved
   are the identity's residual plug variables, this is a guard on the
   accounting arithmetic (a mis-wired term, a float blow-up), NOT a
   backstop against conjured energy — guards 4 and 5 are that. The
   distinction is load-bearing: both committed leaks (negative
   curtailment; deficit-charge) passed conservation.

RuleBased never triggers any of these (its own rules are strictly
tighter), so every physical-tier check is a behavioural no-op on the
rule-based path — which is why the digests are unmoved by
construction.

## Tier 2 — policy choices (conventions, enforced only when the contract declares them)

`PolicyContract` (`policy.rs`) carries the rule-based policy's own D4
conventions. They are contestable modelling *choices*, not physics —
exactly the things a foresight dispatcher must be free to do
differently:

- **`charge_from_surplus_only`** (D4 rule 2): total charge ≤ the
  period's must-take surplus — stores never charge from the thermal
  stack. RuleBased: `true`. Declared `false`, a policy may draw
  charging power from the stack, and the stack then serves
  `deficit + charge` (the pre-charge-ahead-of-a-drought move; pinned
  by `a_relaxed_contract_may_precharge_a_store_from_the_stack`,
  `tests/storage.rs` — the refactor's red-first case, rejected before
  `24733eb`, accepted after).
- **`discharge_after_stack_only`** (D4 rule 3): no discharge in a
  surplus period, and total discharge ≤ the deficit left after the
  full thermal stack has run — storage is the reliability backstop,
  never dispatched before the stack. RuleBased: `true`. Declared
  `false`, a policy may discharge ahead of the stack or in surplus
  (accounted as extra curtailment).

`DispatchPolicy::contract()` defaults to the FULL rule-based contract
— the conservative choice: a policy that declares nothing is held to
today's rules on top of the always-on physical tier. `RuleBased`
overrides it explicitly (returning the same value) so the reference
behaviour is greppable and cannot drift with the trait default; that
explicit contract is what pins digest `779d7444…`
(`the_surplus_only_safety_net_survives_under_the_rule_based_contract`,
`tests/storage.rs`).

Not expressed in the contract (a deliberate boundary): the symmetric
√η per-leg efficiency split is ALSO a rule-based convention (D4, a
~15–20 % store-side shift versus asymmetric per-leg conventions), but
it lives in shared mechanics (`StoreState::apply` / `max_charge` /
`max_discharge`) that every policy's decisions are validated against.
Relaxing it is a schema/mechanics change (per-leg efficiencies in
ADR-8), not a contract flag.

## Who declares what (the policy set as committed)

| Dispatcher | Where | Contract |
|---|---|---|
| `RuleBased` (default) | `policy.rs` | both flags `true` (declared explicitly, not via the trait default) |
| Priced ladder (D11) | a `dispatch.flow_signal` variant on the multi-zone LINK path (`multizone.rs`), not a storage policy — storage still dispatches under `RuleBased` | rule-based contract (inherited: the ladder changes how flows are signalled, not the D4 storage choices) |
| Perfect-foresight LP (D12) | `lp.rs` — deliberately NOT a `DispatchPolicy` | not applicable — see below |

The adopted D12 design made the LP a **whole-horizon function**
(`run_multi_lp` and its rolling/objective variants), not a
`DispatchPolicy` implementation: the ADR-6 trait is per-period with a
no-lookahead `SystemState`, and perfect foresight needs the whole
horizon. The LP therefore never passes through the per-period
validation path at all — its physics (conservation, bounds,
non-negativity, no phantom charge) are LP **constraints**, asserted by
the LP test suite, and the D4 choices are simply absent from its
objective. docs/04's older "`PerfectForesight` policy via good_lp"
wording is superseded by this adopted design (recorded here rather
than by editing docs/04). The contract mechanism still carries its
weight for any future *per-period* smart dispatcher, and it is what
lets the LP-adjacent test policies (`PreCharger`, `ChargeAlways`)
exercise the physical tier with the choices relaxed.

## The D4 erratum cross-reference (f401b9a)

The D4 note's original rule 1 listed "must-run plant at availability"
inside must-take supply. That clause NEVER matched the engine:
must-take is weather-driven renewables + exogenous traces ONLY, and
nuclear (or any low-rung thermal) is merit-order bottom — backed down
whenever residual demand is below its available ceiling. The engine is
canonical (it is what every committed digest and the Stage 1
observed-2024 gate validated); the prose was corrected by dated
erratum banners in `policy.rs` and `d4-rule-based-dispatch.md`.
Consequence for this note: **the policy-choice tier contains no
must-run flag and needs none** — there is no must-run behaviour to
relax. Owned bias direction: versus a must-run treatment the engine
understates curtailment and storage charging in deep surplus (small at
the 2024 fleet — 116/17,568 back-down periods, pinned by
`nuclear_backdown_periods_on_the_2024_reference` in
`tests/acceptance_2024.rs`; grows with nuclear share).

## Consequences owned elsewhere (pointers, not restatements)

- Under the rule-based contract, storage never displaces gas (Q3
  conditioning — D4 note, D8 interactions section).
- The rule-based-vs-LP storage gap is a REPORTED FINDING (ADR-6);
  Stage 7's per-scenario surface is `storage_gap_report`
  (`solve.rs`), which asserts the LP ≤ rule-based sanity invariant
  structurally.
