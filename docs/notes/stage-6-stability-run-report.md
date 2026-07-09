# Stage 6 (part 1) — swing-equation stability engine: results

Committed record of the Stage 6 part 1 acceptance runs (2026-07-03),
against the 9 August 2019 GB event reconstruction
(`scenarios/events/gb-2019-08-09.toml`; evidence pack
`docs/notes/` Aug-2019 series; measured record
`data/reference/neso-frequency-2019-08-09-event-window.csv`, NESO 1-s
data, nadir independently recomputed at **48.787 Hz, 15:53:49Z**).
Adversarial review: `docs/notes/stage-6-review.md`, ACCEPT-WITH-NOTES —
every number below was independently reproduced by the reviewer via both
the acceptance tests and the CLI. **`stage-6-validated` is NOT tagged**:
the Q8 pathway runner and the Module 6 chart are deferred to part 2
(reviewer ruling c); the tag waits for them.

## 1. Acceptance anchors T1–T4 — all PASS at both official inertia bounds

The official record self-disagrees on system inertia (~5 %): 210 vs
219.632 GVA·s. Every gate is therefore evaluated at both bounds; a pass
means both.

| Anchor | 210 GVA·s | 219.632 GVA·s | Gate (docs/04, pinned pre-model) |
|---|---|---|---|
| T1 nadir | 48.7928 Hz (t=72.91 s) | 48.7931 Hz (t=73.61 s) | (48.75, 48.80], stage-1 LFDD (931 MW, 0.3 s delay) modelled, stage 2 must not operate — asserted |
| T2 first arrest | 49.1706 Hz (t=13.76 s) | 49.1887 Hz (t=13.94 s) | 49.10 ± 0.10 — **top-of-band, see §2** |
| T3 RoCoF (pinned 2-s window from 0.51 s) | −0.1457 Hz/s | −0.1405 Hz/s | ±25 % of measured 0.144 → [0.108, 0.180]; lands 1.2 %/2.4 % from measured |
| T4 1,000 MW counterfactual | min 49.5396 Hz | min 49.5448 Hz | ≥ 49.5 Hz, no LFDD |

Measured nadir 48.787 Hz sits 6 mHz below the modelled value —
inside the T1 band, with the LFDD stage-1 precondition satisfied the
same way the real event's was.

Un-gated diagnostics (recorded, not gates): LFDD stage-1 trigger at
t = 72.61/73.31 s vs 75.9 s measured (~3 s early); steepest 1-s RoCoF
−0.1590/−0.1528 Hz/s; post-nadir managed recovery (dispatch actions,
demand reconnection) is out of scope by the event spec's documented
exclusion.

## 2. T2 is a top-of-band pass — mechanism and a standing constraint

**Recorded at kill-criterion prominence (the finding cuts against the
model, and it binds future work):**

At the upper inertia bound the first arrest lands at 49.1887 Hz —
**0.011 Hz from the 49.20 gate edge**. The reviewer ruled this
acceptable as-is (the ±0.10 band was pinned before the model existed,
from the irreducible input-ambiguity budget, and the model lands inside
it at both bounds — the test working as designed), with the mechanism
and constraint below mandatory in this record.

**Mechanism — the response envelope is fast mid-phase.** The frequency-
response model (held volumes × published delivery factors × Grid-Code
envelope timings × droop/latch shape) delivers response faster in the
10–40 s window than the real ESO deployment did. Three un-gated
diagnostics, all reviewer-verified from the model trace, say this
consistently:

1. Modelled first arrest at t ≈ 13.8–13.9 s vs ~25 s measured.
2. **Mid-event over-recovery to 49.63 Hz at t = 30 s** vs the measured
   ~49.2 Hz plateau (reviewer-sampled from `frequency_trace.csv`) —
   the model climbs where the real system held flat.
3. LFDD stage-1 trigger ~3 s early (§1).

This is a stated envelope convention, not a physics error: the gated
early phase (T3 RoCoF) and protection phase (T1 nadir) are insensitive
to it. The envelope shape matters only in the diagnostic mid-phase.

**Damping↔T2 coupling (reviewer ruling b).** The load-damping constant
(1.836 %/Hz of the 29 GW demand base = 532.5 MW/Hz) is derived from the
un-gated 49.2 Hz plateau balance ((1,481 − 1,055) MW / 0.8 Hz), sits
inside the literature span 1–2.5 %/Hz, and is **not** tuned against any
gate — T2 gates the first arrest (measured 49.083), not the plateau.
But the plateau value coincides numerically with T2's upper edge, and
higher damping pushes the arrest upward: the derivation is legitimate
and documented, and the coupling is why the edge is live.

**NO-RETUNING RULE (standing):** the 49.20 edge is a live constraint.
Any future re-derivation that strengthens mid-phase response — damping,
delivery factors, ramp/envelope timings — risks crossing it. If that
happens, the resolution is to revisit the physical derivation or the
band's evidence base, **never to retune inputs against the gate**. All
calibrated inputs must continue to trace to un-gated observables or
cited sources (the full no-tuning audit is in the review, ruling b).

## 3. Module 6 first cut — inertia of dispatched fleets

**Recorded at kill-criterion-4 prominence (findings with a mandatory
caveat):**

- **2024 reference fleet, market-only dispatch: 15,020 of 17,568
  periods (85.49 %, 7,510 h) fall below the 120 GVA·s operational
  floor; 13,335 below 102 GVA·s** (floors cited: NESO FRCR 2024 p.10).
  Minimum **0.00 GVA·s at 2024-04-06T11:30:00Z** (2 zero-inertia
  periods).
- **CAVEAT — do not quote as "GB was below its floor 85 % of 2024".**
  The model dispatches on merit order alone: no must-run, no
  min-stable generation, no NESO stability actions. Real GB held
  ~110–350 GVA·s *because NESO pays for synchronous provision*. The
  finding is the **size of the gap between market-only dispatch and
  what stability requires** — the cost of which is exactly the Stage 7
  question — not a claim about operated GB. The caveat is pinned three
  ways: prose block above the pinned constants in
  `grid-stability/tests/inertia_sum.rs`, the `UNCONSTRAINED` block in
  every `report.toml` (CLI-test-asserted), and the CLI console line.
- **RS-lean fleet (wind + solar + H2): zero synchronous inertia in all
  701,280 periods of the 40-year record**;
  `has_synchronous_provision = false`, FINDING line fires. This is an
  *output*, not an assumption: hydrogen reconversion defaults
  non-synchronous by documented schema-v3 choice (docs/03 migration
  note item 5) — a hydrogen-turbine variant would model turbines as a
  fleet entry with explicit H. Pinned at mechanism level
  (`all_variable_fleet_has_zero_synchronous_inertia`); the reviewer
  reproduced the full zero series.
- Exogenous traces (imports, FUELHH "other") carry no inertia — the
  2024 sum is understated by roughly 1–3 GVA·s. Recorded, not material
  to the findings above.

## 4. Model record

- **Integrator**: Heun (RK2), 10 ms fixed step, kinetic-energy-exact
  swing with f0/f correction. Analytic gate: ≤ 1 µHz vs the closed form
  `f₀·√(1 − P·t/E)` over 60 s. Performance 1.48 ms/event (target
  < 10 ms). Deterministic: fixed step, no adaptivity, BTreeMap
  ordering, bit-identical repeat runs (tested).
- **Event spec** (`scenarios/events/gb-2019-08-09.toml`): published
  trip sequence (Little Barford initial trip 244 MW — the 641 MW
  folklore figure is the station total; total loss ≥ 1,990 MW = 1.9×
  secured), response holdings × published delivery factors
  (1,022 × 0.89, 1,314 × 0.88), Grid-Code envelope timings (CC.6.3.7),
  LFDD staged blocks per the E3C table. Per-number citations inline;
  drift-guarded by `event_spec_matches_the_committed_reference_record`.
- **Inertia constants**: `data/reference/inertia-constants.toml`
  (literature values, cited — no NESO per-tech H publication exists),
  transcribed into `grid_core::inertia`, drift-guarded.
- **Schema v3**: `inertia_h`/`synchronous` on fleet entries, derived
  defaults, overrides surfaced, MVA = GW/0.9 at the single conversion
  point; v2 frozen fixture + structured migration error. Full note:
  docs/03.
- Suite at delivery: 298/0 workspace tests; fmt/clippy clean; all
  Stage 1–4 pins unmoved; dispatch digest `779d7444…` unchanged.
  Reviewer reproduced independently.

## 5. Publication rules (standing, per docs/05 discipline)

1. T1–T4 numbers are quotable with the inertia bound stated (both
   bounds differ in the third decimal; quote the pair or the range).
2. The 85.49 % figure is **never** quotable without the §3 caveat
   sentence (market-only dispatch, no NESO actions).
3. The RS zero-inertia finding is quotable, but always with the
   hydrogen-reconversion convention named (non-synchronous by choice;
   turbine variant would differ).
4. Mid-phase trace shape (arrest timing, plateau, recovery) is
   diagnostic, not validated — do not quote modelled mid-phase values
   as event reconstruction.
5. The no-retuning rule (§2) binds all future Stage 6+ work.
