# Stage 4 — timescale decomposition, sweeps, per-year batch: results

Committed record of the Stage 4 acceptance runs (2026-07-03). All
numbers measured on the complete 1985–2024 record (snapshot
`39TK56WX185WZ1HP9WNG`) against the Stage 3 pinned RS-lean scenario
(58,432 GWh total requirement — reproduced bit-identically by the
attribution machinery before any decomposition was trusted). Reviewer
independently re-derived the mathematics and re-ran every claim.

## 1. The decomposition (Module 3c) — kill-criterion 2 SURVIVED

Method: residual load split by successive differences of centred moving
averages **of the original series** at 24 h / 14 d / 365 d. Bands sum to
the residual exactly by construction (measured worst per-period error
2.8e-14 GW over 701,280 periods). Storage attribution: the Stage 3
bisection re-run on each successively smoothed series; differences
attribute the requirement per band; the sum telescopes to the total
exactly.

**Attribution of the 58,432 GWh requirement (RS-lean, 1.35× supply):**

| Band | GWh | Share |
|---|---|---|
| Diurnal (< 24 h) | 14,816 | **25.4 %** |
| Synoptic (1–14 d) | 18,224 | 31.2 % |
| Seasonal (14 d–1 y) | 25,392 | 43.5 % |
| Inter-annual (> 1 y) | **0** | **0.0 %** |

**Stability (the KC-2 detector):** perturbing the synoptic window
10 d ↔ 21 d moves the total, the diurnal band, the inter-annual band,
and the synoptic+seasonal aggregate by **exactly 0** (window-invariance
is a theorem of the construction — every level smooths the original
series, so a window touches only the two bands it bounds; reviewer
re-derived and re-measured). Only the adjacent synoptic↔seasonal pair
trades: 4.27 % of total at 10 d, 6.57 % at 21 d.

**Gate-history disclosure (recorded, not smoothed over):** the
implementer's pre-measurement placeholder gate (flat 5 % on any shift)
tripped at 6.57 % and was restructured around the construction's true
invariants (f64-dust gates on the four invariant quantities; 10 % on
the adjacent trade — a regression guard at 1.5× measured worst, not an
independently derived threshold). The reviewer ruled this legitimate —
stricter, not looser, on every argument-bearing quantity — after
re-deriving the invariance from the code.

**Standing publication rules:**
- The synoptic-vs-seasonal *ranking* flips with the window choice
  (at 21 d synoptic 37.8 % > seasonal 36.9 %): never quote the split
  without stating the window. Window-invariant, safely quotable:
  diurnal share, inter-annual share, synoptic+seasonal aggregate
  (43,616 GWh, 74.6 %), and the total.

## 2. Two findings AGAINST tidy expectations (kill-criterion 4)

1. **The diurnal band is a quarter, not a sliver (25.4 %).** Mechanism,
   verified arithmetically: cycling daily swings through a 40 %
   round-trip store costs 1/√η − √η = 0.949 GWh of store capacity per
   GWh of two-way daily traffic during the binding drawdown; 140 GW of
   solar drives large daily swings. Consequences: "a few days of
   storage" answers **at most this quarter** — the safe headline is
   "~75 % of the requirement is at synoptic-and-slower timescales" —
   and this quarter is precisely the slice batteries/DSR can target
   (quantifying Q6's ceiling in advance).
2. **The inter-annual band attributes ZERO.** At 1.35× supply the
   365-day-smoothed residual is never in deficit: "the requirement is
   inter-annual" is NOT supported. The correct statement of the Stage 3
   720-day episode: **seasonal-scale need, multi-year recovery** — the
   store exists because of winters and takes years to refill after bad
   ones, not because of decade-scale deficits.

## 3. Q4 — one year or forty?

All 40 weather years run as independent single-year scenarios
(initial-condition-sensitive by construction — store starts full each
1 Jan; flagged per row in `runs/q4-per-year/per_year.csv`):

- **Design year: 2021 (44,640 GWh) — not 2010** (third at 36,608,
  behind 1989's 36,736). Requirement tracks deficit *timing against
  demand shape*, not annual capacity factor alone (2010 is the worst
  CF year but not the worst storage year).
- Easiest: 1999 (7,020 GWh). Spread: **6.4×** across years.
- **Every single year underestimates the 40-year requirement** — even
  2021 by 24 %. "Modelled on [one year] of data" is a tell, now
  quantified.

## 4. Infrastructure delivered

Generic sweep runner (rayon, order-preserving — results bit-identical
to a permanently-maintained serial path, asserted); full response
surfaces persisted (CSV+Parquet, hash headers); residual-load
utilities (duration curves, ramps — Module 2 machinery); Module 4
storage×overbuild surface (8×8; feasibility frontier brackets the
pinned Stage 3 curve: infeasible at 1.15×, cliff neighbourhood at
1.26×, feasible from 50 TWh at 1.38×); benches/ smoke benchmarks.

## 5. Performance — one target missed, recorded

40-year single run: **30 ms** (target < 1 s: met 33×). 10⁴-point
single-year sweep: 2.3 s. **10⁴-point full-40-year sweep: 67–82 s vs
the docs/06 < 60 s target — MISSED** (the reviewer ruled the 40-year
reading is the honest one; the 2× CI smoke threshold passes). Cause:
memory-bandwidth-bound scaling (each run materialises ~60 MB of
per-period series; 8 threads is slower than 12, ruling out scheduler
effects). Remediation direction: a lean sweep-mode engine result that
skips materialising per-period series — open item in project-state;
alternatively a docs/06 amendment. Not silently drifted.

## 6. Boundaries

Attribution is defined for all-must-take, single-store scenarios (the
RS family); thermal-stack attribution is future work (structured error,
not silent misattribution). Single-year Q4 runs start full (stated).
Decomposition windows 24 h/14 d/365 d are the pinned defaults; all
quoted shares are from these windows.
