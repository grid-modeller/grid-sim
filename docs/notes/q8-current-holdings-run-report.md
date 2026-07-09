# Q8 current-holdings variant — run report (permanent record)

**Status:** committed record, 2026-07-03. Completes publication rule 2
of `docs/notes/stage-6-part2-run-report.md` (the 2019-default Q8 run's
missing counterpart). Package review:
`docs/notes/q8-variant-run-review.md` (ACCEPT-WITH-NOTES; every number
below reviewer-reproduced bit-identically; the mechanism story in §4
is the reviewer's probe-corrected version, not the implementer's
original). Holdings evidence: `docs/notes/q8-current-holdings.md` +
`data/reference/response-holdings-2025.toml` (commit 887d5e4).
Read §6 before quoting ANYTHING from this note.

## 1. Headline (quote in this form, rule 6.2)

Under FY2025 procured response volumes at contract delivery factors,
the modelled 1,800 MW infrequent-infeed standard is **met from 2024
and never lost** on the FES 2025 Holistic Transition pathway — 2024
band 2,433 / 2,701 MW (min φ=0.15 / mean φ=0.35), vs 1,373 / 1,573 MW
under 2019-era holdings. **The difference is a holdings input, not a
physics change.** All numbers are against the part-1/part-2
simulation contract (single-step loss, 120 s window, 48.8 Hz floor,
dynamic services only, no LFDD credit) — a modelled ride-through
capability, never an operational security standard.

The part-2 "recovery in 2035 (mean) / 2037 (min)" framing is
**superseded**: it was an artefact of the 2019-default holdings level.
Under current holdings there is no crossing to recover from.

## 2. The comparison (largest survivable loss, MW, min/mean band)

| Year | 2019-default | Current (df 1.0) | df 0.9 sensitivity |
|---|---|---|---|
| 2024 | 1,372.7 / 1,573.5 | **2,432.9 / 2,700.8** | 2,282.7 / 2,527.5 |
| 2030 | 1,450.8 / 1,627.2 | 2,485.4 / 2,742.3 | 2,346.8 / 2,574.5 |
| 2035 | 1,662.0 / 1,825.6 | 2,686.2 / 2,940.1 | 2,554.3 / 2,772.8 |
| 2040 | 2,009.9 / 2,191.2 | 3,082.3 / 3,322.8 | 2,932.1 / 3,150.0 |
| 2050 | 2,345.6 / 2,515.9 | 3,419.2 / 3,648.1 | 3,269.7 / 3,476.0 |

Crossings vs 1,800 MW: 2019-default first at-or-above 2037 (min) /
2035 (mean); current, df 0.9: **none — met from 2024, never lost**
(the pathway minimum IS the 2024 point). Full series: the run CSVs
under `runs/q8-current-holdings/` (regenerate from the committed spec
files; digests pinned).

## 3. What today's suite holds (context for §4)

FY2025 EAC mean cleared volumes: DC-L 1,178 / DM-L 416 / DR-L 461 MW
= 2,055 MW dynamic low-frequency response — **less** than the 2019
baseline's 2,336 MW held (1,896 MW effective at measured delivery
factors). The current suite is smaller and much faster: 1,594 MW at
full output within 1 s (DC-L + DM-L) vs the 2019 secondary tranche
reaching full output only at 30 s.

## 4. Why it improves: it is speed, in two forms (reviewer-corrected)

The gain decomposes sequentially (legs defined below; ~34–37 % :
~63–66 %, stable across 2024/2035/2050 and both φ conditions):

- **Leg A — envelope timing (~1/3):** the same MW ranked into the
  same speed classes, but with today's per-product delay/ramp
  envelopes instead of 2019's.
- **Leg B — composition (~2/3):** NOT held volume and NOT droop
  mechanics. Reviewer probes measured droop-saturation shape at
  ≤ 0.61 MW and the 2019 30 s sustain cliff at exactly zero
  (digest-identical with it removed). Leg B is delivery-factor
  accounting (+38/+89 MW at 2024 min/mean) plus **~+646 MW of held
  capacity relocating into the sub-second envelope rank** (fast-rank
  effective MW 472 → 1,178; slow rank 1,055 → 461) — a speed effect
  by composition. Do NOT attribute leg B to droop-saturation shape
  or the sustain cliff; that mechanism story is refuted.

Honest one-line summary: **the current suite wins on how early its
megawatts arrive, not on how many are held** — one third faster
envelopes per product, two thirds the fleet's MW moving into the
fastest products.

The decomposition instrument (the "2019-speed" diagnostic spec) is
**never quotable as a holdings scenario** — it exists only to define
leg A/leg B; its values are pinned for reproducibility (§7).

## 5. Sensitivities and robustness

- **Delivery factor (rule 6.5):** 2019 delivery factors were measured
  (0.67–1.0); current services carry contractual 1.0 — an asymmetry,
  always stated. The df 0.9 uniform sensitivity quantifies it
  (~6 % haircut, uniform along the pathway; every headline statement
  unchanged). Never invent a central measured factor.
- **Demand→damping channel (rule 6.4, asymmetric):** with demand
  pinned at 2024 (reviewer probe), the pathway minimum is
  2,359 / 2,623 MW — still above 1,800, so the **no-crossing claim is
  robust** to the channel. Later-year absolutes are NOT: 2050 min
  falls 3,419 → 2,452 MW with demand pinned. No post-2030 absolute is
  quotable without this caveat; part-2 §2 consequence 3 carries over
  verbatim (the rise along the pathway is wholly the demand-growth →
  damping channel on a 2019 damping constant).
- **Conservative exclusions (rule 6.6):** ~202 MW of SFFR (static,
  engine-rejected by design) and MFR (no published volumes) are
  excluded from the current side — current-holdings numbers are
  **understated** on this axis.
- **Linear-droop optimism (rule 6.7):** the engine's linear droop
  sits at or above the published DC knee curve over the whole
  0–0.5 Hz range (both legs inherit this); probe evidence shows
  saturation-width itself moves the boundary ≤ 0.61 MW here
  (second-order — the boundary case is saturated).

## 6. Publication rules (binding; supersets, never replaces, the
## part-1/part-2 rules)

1. Band always with the condition named (min φ=0.15 / mean φ=0.35 —
   the 2024-anchored market-behaviour convention, not a 2050 dispatch
   forecast); the market-only lower edge (zero inertia ⇒ zero
   survivable loss) accompanies any quoted band.
2. Headline only in the §1 form: FY2025 volumes at contract delivery
   factors; paired with "a holdings input, not a physics change";
   against the simulation contract, never an operational standard.
3. The part-2 crossing years (2035/2037) are superseded as a
   2019-holdings artefact — cite both notes together.
4. Demand→damping caveat per §5, asymmetric: no-crossing robust,
   post-2030 absolutes conditional.
5. Contract-vs-measured delivery asymmetry always stated; df 0.9 is
   its quantification.
6. SFFR/MFR conservative exclusions named where current holdings are
   characterised.
7. Linear-droop optimism caveat on both decomposition legs.
8. The speed/volume split only as the §4 sequential leg decomposition
   with legs defined and the corrected mechanism — never as exact
   attribution, never via droop-shape/sustain-cliff language.
9. The 2019-speed diagnostic is never a holdings scenario; repeat the
   marking wherever its numbers appear.
10. Every published artefact/chart derived from these runs carries
    "Supported by National Energy SO Open Data" — the CLI chart
    footer has no attribution hook, so the string is added manually
    at publication.
11. Volumes quoted as "FY2025 mean procured volumes"; the DM-L
    step-up (annual mean 416 vs going-forward ~500 MW) noted where
    DM is discussed.
12. Every number above is covered by a pinned regression (§7); any
    new point quoted from these specs gets its pin first.

## 7. Determinism and pins

Run digests (all reviewer-reproduced from the committed specs):
central `2c8d6997…`, df 0.9 `2da63edd…`, diagnostic `bd8f4684…`;
2019 baseline `fd410616…` untouched. Pinned regressions in
`grid-cli/tests/stability.rs`:
`q8_current_holdings_pathway_reproduces_the_pinned_headline_numbers`,
`q8_current_holdings_df090_sensitivity_reproduces_the_pinned_numbers`,
plus the diagnostic pin (landed with this report) and the three spec
drift-guards in `grid-stability/tests/pathway.rs`. Suite state at
commit: fmt/clippy clean, full workspace green, Stage 1/2 digests and
the part-2 FES pin unmoved, no-retuning guard green.
