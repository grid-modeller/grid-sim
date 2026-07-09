# B6 two-zone — permanent record and publication rules

**Status:** committed record, 2026-07-04 (engine 209d78b). Package
review: `docs/notes/b6-two-zone-engine-review.md` (every number below
reproduced by the reviewer, twice for the headline finding; the
framing in §4 is its §1d ruling). Data: `b6-two-zone-data-report.md`
+ its review. Read §4 before quoting anything.

## 1. What was built

Great Britain split into two zones — Scotland (SCO) and rest-of-GB
(RGB) — joined by one link representing the B6 transmission boundary,
on the existing Stage 5 zone+link machinery (the flow rule itself
unchanged). Scenario `scenarios/gb-2024-2zone.toml`; schema v6 adds
directional link capability and an optional half-hourly capability
trace. Purpose (Richard-ratified, promoted to vital 2026-07-04):
bound the copper-plate bias with our own instrument.

## 2. Validation (against NESO observed boundary flows)

The link ruling (data review §6) is implemented verbatim: the link is
**B6 alone**, validated against NESO's day-ahead boundary-flow series.
Three gates, all pinned:
- Pre-constraint (copper-plate) B6 flow: **18.809 TWh** southward,
  r = 0.744 vs the 22.627 TWh day-ahead anchor — within the
  decomposed wedge budget (demand-basis, flow-rule, offshore
  commissioning, DA-forecast wedges named).
- Constrained export: **15.788 TWh** vs the 17 TWh Energy Trends
  outturn, carrying the ~2 TWh irreducible day-ahead-vs-outturn wedge.
- Binding frequency: **23.23%** of periods at ≥99% of limit vs the
  observed 23.60%.
The model lands between the two anchors (22.6 unconstrained /
17 outturn), as the review required.

> **CORRECTION (2026-07-06, R7 flow-walk stall fix — docs/08 R7,
> RESOLVED; adjudication `docs/notes/r7-fix-review.md`).** The pre-fix
> engine silently cap-truncated stalled boundary walks in
> 2,795/17,568 periods of the §2 validation run (reviewer-confirmed
> per-period census). Re-measured on the fixed engine:
> **§2 gates** — copper-plate 18.809 → **19.898 TWh** (r 0.744 →
> 0.740), still within the ±4.5 wedge budget and TOWARD the 22.627
> anchor; constrained 15.788 → **16.406 TWh**, TOWARD the 17 TWh
> outturn; binding 23.23 % → **25.02 %** — this one moved **AWAY from
> the observed 23.60 %** (0.37 pp below → 1.42 pp above) while staying
> within the ±4 pp band.
> **§4 rule 3** — 60 GW SCO curtailment 27.14 → **27.03 TWh**
> constrained / 13.77 → **10.93 TWh** copper-plate (the copper split
> now lands at near-exact equal surplus depth, as the flow rule
> prescribes); the system-net legs move +13.37 / −6.72 / **+6.65 →
> +16.09 / −7.24 / +8.85 TWh**.
> **§4 rule 4** — the two binding shares are now 0.2323 → **0.2502**
> (gate-(iii) DA-flow-mask) and 0.2330 → **0.2510**
> (capability-observed, 3,998 → 4,306 periods); still never conflated.
> **What did NOT move:** the §3 storage table and its controls
> (23,872 / 26,480 / 35,648 GWh, the placement spread and the
> decomposition controls) are unmoved — verified by the unmodified
> exact pins of `acceptance_b6_robustness.rs` passing on the fixed
> engine. Old values are recorded per pin in
> `acceptance_b6_2zone.rs` / `regression_2zone.rs`.

## 3. The finding (RS-fleet storage requirement)

The ratified expectation — copper-plate is conservative for adequacy,
the flagship storage numbers barely move under a split — is
**contradicted as measured**. On the Royal Society fleet:

| configuration | 40-y requirement | vs single-zone |
|---|---|---|
| single-zone (pinned) | 23,872 GWh | — |
| two-zone, unlimited boundary | 26,480 GWh | +10.9% |
| two-zone, B6 at 2024 capability | 35,648 GWh | +49.3% |

Reviewer control decomposition (all pinned): pooled traces on one bus
23,808 GWh (trace substitution nil); split store on one bus 24,112
(+1.0%). So the copper-plate +10.9% is **dispatch-convention-
dominated** — the rule-based flow clears before storage, blind to
store headroom — not a spatial effect. Mechanism of the boundary
term: drought depth is GB-wide (2010 worst in both zones); it is the
**inter-drought recharge** that the boundary throttles — Scottish
wind surplus cannot get south fast enough to refill a national store
between droughts.

> **CORRECTION (2026-07-04, beta-readiness audit).** An earlier version
> of this note and the engine review claimed the "boundary effect
> proper is ~+33–35%, stable across store placements." That is
> WITHDRAWN — it is contradicted by this file's own pinned data: at the
> 3% energy-optimum placement, B6 (33,056 GWh) sits BELOW copper-plate
> at the same placement (33,632 GWh) — a −1.7% "boundary effect,"
> physically impossible under optimal dispatch and a symptom of the
> same rule-based flow artefact. The boundary-vs-copper delta at the
> *same* placement is not stable: it ranges from −1.7% (3% placement)
> to +34.6% (demand-share). The +38.5% lower bound of the headline is
> therefore contaminated by the copper-plate flow artefact, not a clean
> boundary effect. **What survives, safely quotable:** the raw
> total-delta DIRECTION — the single-zone requirement (23,872 GWh) is a
> lower bound, and every two-zone/B6 configuration measured needs more
> storage — plus the pinned total values with all three conditions
> below. The clean separation of "boundary" from "dispatch convention"
> awaits the LP (Stage 7); until then, no single "boundary effect
> proper" percentage is quotable.

## 4. Publication rules (binding)

1. The single-zone flagship numbers (23,872 GWh etc.) are henceforth
   quoted as **copper-plate lower bounds with respect to internal
   network constraints**, with the measured +38–49% two-zone/B6
   sensitivity attached. The "barely moves" caveat is withdrawn (the
   record correction is in project-state and the Q4 paper entry).
2. Every two-zone/B6 storage number carries three conditions: (i) the
   frozen-2024-boundary-capability stress convention (reality builds
   transmission alongside the fleet — this does not); (ii) the store
   placement (+38.5% to +106% spread — the total is strongly
   placement-dependent, and NO single "boundary effect proper"
   percentage is quotable, per the §3 correction); (iii) "rule-based
   dispatch, upper-bias" — the LP (perfect-foresight, Stage 7) is the
   named resolver of the dispatch-convention component, which is
   entangled with the boundary term and not yet separable.
3. Curtailment/capture (Q2/Q10): model Scottish curtailment is a
   **lower bound** on the real Scottish constraint phenomenon (B6-only
   slice; the £526m Scottish boundary-group cost is context, never a
   tuning target). Pinned 60 GW-wind bracket: SCO curtailment 27.14
   vs 13.77 TWh copper-plate — but quote the **system net** with it
   (SCO +13.37 / RGB −6.72 / net +6.65 TWh); the SCO delta alone
   overstates the system effect by ignoring the RGB counter-movement.
4. Two binding-share statistics exist and must not be conflated: the
   gate-(iii) DA-flow-mask 0.2323, and the summary's
   capability-observed 0.2330 (different denominators; labelled in the
   outputs).

## 5. Determinism, pins, open items

2-zone digests pinned (SCO / RGB / links); all prior pins unmoved
(779d7444 single-zone, the 5-zone set, the RS 23,872 pin). Suite
536/0. Open follow-ups: ETYS 6.7 GW is cited to a JS-rendered NESO
page, not yet a fetchable pinned artefact (negligible sensitivity —
116/17,568 periods — but a data-package follow-up owed); a
pre-existing duplicate-fleet-TechId validation hazard (an engine
follow-up, not this package's defect). The group-effective tighter-
link Q2/Q10 sensitivity and the RS store-placement alternatives are
named, not run.
