# Q5/Q11 heating-mix runs — permanent record and publication rules

**Status:** committed record, 2026-07-04. Engine at a6fdc34; artefacts
regenerated at that hash under `runs/q5-heating-mix/` (regenerate from
the committed scenario + `--store-power-gw 200`; all values pinned).
Package review: `docs/notes/q5-heating-mix-review.md` (every number
below reproduced bit-identically by the reviewer; rulings R1–R4 are
its rulings). Inputs: the adopted D9 design
(`docs/notes/d9-heating-overlay.md`), the reviewed heating data
(61ecfd9), the heating engine (cb1c8b2). Read §4 before quoting.

## 1. The experiment

RS-class fleet (royal-society-37y-heated.toml: 570 TWh/yr electrical
demand before heating, wind+solar+hydrogen), buildings heat quantum
410.5 TWh (GB record-mean delivered heat, reviewed convention),
electrified share 0.5, portfolio swept over the full
ASHP/GSHP/district simplex at 0.1 steps (66 points + baseline).
Storage requirement = bisection to zero unserved over 1985–2024 **at
200 GW store power, both endpoints** (the committed 100 GW rating is
power-bound infeasible under heating — a pinned finding, reported
never bumped). Dispatch metrics at the committed 100 TWh store.

## 2. Pinned findings

Corners (peak residual GW / 40-y storage requirement GWh / horizon
curtailment TWh):

| point | peak | requirement | curtailment |
|---|---|---|---|
| no-heating baseline | 92.24 | 23,872 | 18,712 |
| all-ASHP | 115.69 (+23.45) | 43,488 (×1.82) | 14,289 |
| all-GSHP | 114.40 (+22.16) | 41,248 (×1.73) | 14,569 |
| all-district | 95.85 (+3.61) | 25,872 (×1.08) | 17,963 |

The 0.70/0.20/0.10 point reproduces the engine characterisation pin
bit-identically (113.4466987983204 GW / 40,224 GWh) — the sweep is
tied to the pinned baseline.

Gradients along ASHP→district (per 10% of the electrified quantum):
peak −1.984 GW, linear to float precision. Storage: **piecewise, with
a real knee** (reviewer-verified at ~100× the bisection quantum) —
≈−2,780 GWh/10% for the first four tenths, ≈−1,030 GWh/10% beyond;
interior edge values pinned end to end (40,672 / 37,856 / 35,104 /
32,352 / 31,040 / 30,000 / 28,960 / 27,936 / 26,896 GWh).
ASHP→GSHP: peak −0.129 GW, storage −224 GWh per 10%.

Timescale decomposition of the added requirement (windows
24 h / 14 d / 365 d): the addition loads the **seasonal band
hardest** — ASHP−district deltas seasonal 9,528 > synoptic 6,280 >
diurnal 1,808 GWh. Inter-annual attribution is zero at every point.

Curtailment: electrified heat *reduces* curtailment (direct
absorption + storage cycling) — all-ASHP absorbs 4,423 TWh (horizon)
of otherwise-curtailed energy, all-district only 749 TWh. Energy
balance closes exactly (reviewer-verified).

## 3. What the findings mean (safe conclusions)

Serving decarbonised heat with air-source heat pumps converts heat
into the grid's worst demand — temperature-correlated, peaking in
cold anticyclonic wind lulls with COP collapsed — and the storage it
demands is seasonal-class, the kind with no economic solution at
scale. District geothermal keeps most of that off the grid entirely.
The network value of geothermal heat is therefore front-loaded: the
steep limb of the storage gradient is where GB actually stands
(near-zero district share). Ground-source improves efficiency but
not the peak correlation — it is not the transformative option here.
The £ valuation is Stage 7 (equal-reliability scenario differencing
per D8); nothing in this note is a £ claim.

## 4. Publication rules (binding)

1. Every storage number carries "at 200 GW store power, both
   endpoints"; the 100 GW infeasibility finding travels with the
   ×1.69/×1.82-class headlines. SolveInfeasible at a committed
   rating is a reportable result, never silently bumped.
2. The storage gradient is quoted piecewise (both limbs, or the
   curve); the −1,762 GWh/10% edge average never alone (R1). The
   marginal-value claim from GB's starting point uses the steep limb.
3. Curtailment: quote both sides in physical units side by side
   (relief of peak/storage AND foregone absorption); any netting
   into a single value is a Stage 7 statement only (R2). Energy
   columns are HORIZON totals (40 years), not annual — say so.
4. Inter-annual zero is a fleet-contingent finding ("on this
   overbuilt fleet"), not a general truth (R3). The −48 GWh
   all-district diurnal delta is resolution-scale: quote as ≈0,
   never as physical daily-cycling relief (R4).
5. Decomposition rankings never quoted without the windows
   (24 h / 14 d / 365 d) — the Stage 4 rule.
6. All deltas are LOWER BOUNDS on the technology differences (D9:
   no behavioural profile understates the peaks and the deltas in
   the same direction); the standing programme caveats (2024
   non-heat demand tiling, climate-stationary intensity,
   frozen-2024 curtailment in the CF traces, frozen-imports) apply.
7. Attribution on published artefacts: ERA5 (CC-BY) and "Supported
   by National Energy SO Open Data" as applicable; the capacity-
   relieved leg (equal-reliability avoided build) is NOT in this
   package — it awaits the ELCC runner, stated wherever rule-6b
   relief is discussed.
