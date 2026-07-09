# Stage 2 — 2024 pricing run: results, the corrected gas-marginal claim, and what the pass is worth

Committed record of the Stage 2 pricing run (2026-07-02). Per
kill-criterion 4 (`docs/08-risks-and-decisions.md`), the headline item
here is a result **against** the working framing, published with the same
prominence as the passes. All numbers below were produced by the
implementer and independently re-measured by the reviewer (fresh CLI
runs, bit-identical digests; the frontier and zero-period counts are the
reviewer's own measurements).

Prices digest (pinned by regression test):
`1d38ed7513340bfc2323e710883a4d67822ac95fc6a436b652329671f809538d`
Stage 1 dispatch digest unchanged (`6f82c7b0…`) — pricing is computed
after dispatch and structurally cannot perturb it.

## 1. THE HEADLINE, AGAINST THE FRAMING: gas sets the price in ≈94 % of
periods on this model — not "~97 %", and not 99 %

- Model gas price-setting share, 2024: **93.89 %** (16,494 / 17,568
  periods; regression-pinned).
- The remaining 6.11 % (1,074 periods) are must-take-only periods: wind +
  solar + calibrated must-run plant + observed exogenous supply cover
  demand entirely, no gas is dispatched, and the model prices them at £0.
  They are windy periods (mean wind 17.9 GW vs 9.4 GW overall — the 88th
  percentile).
- **Causal model boundary:** CCGT minimum-stable generation is not
  modelled. Observed CCGT output reached zero in only **9 of 17,568**
  periods in 2024; the model's CCGT reaches zero in 1,074. Like-for-like
  (same 5–95 flexing definition both sides): **model 85.7 % vs observed
  99.4 %** (regression-pinned). This is a documented Stage 1 dispatch
  limitation, quantified — not observational fuzz.
- The original acceptance gate (±3 points of 99.4 %) was mis-pinned
  against the price-pack report's own warning and was reviewer-verified
  to be **jointly unsatisfiable** with the capture-ratio gate under the
  frozen Stage 1 dispatch (pushing the share to 96.4 % forces the capture
  ratio to ≥ 0.956, above its 0.949 ceiling — the zero-price periods are
  precisely the windy ones). The re-pin and its full history are recorded
  in docs/04 Stage 2.
- **For any published claim:** "gas sets the GB electricity price ~97 %
  of the time" is not supportable unqualified. Defensible statements,
  each with its definition: gas was *behaviourally* marginal (flexing) in
  ≈99 % of 2024 periods; the market price was *gas-cost-consistent* in
  ≈64 % (57.8–65.2 % across definitions); on this model gas *sets the
  price* in ≈94 % of periods, falling to ≈46 % at 60 GW wind (§3).

## 2. Gate results

| Gate (docs/04 Stage 2, as re-pinned) | Measured | Verdict |
|---|---|---|
| Gas price-setting share within observable band [89.4, 99.8] % | **93.89 %** | PASS (exact value pinned) |
| Wind capture ratio within 0.899 ± 0.05 | **0.9413** | PASS — but see caveat below |
| Median model-SMP / observed-MID within [0.90, 1.10] | **1.0100** | PASS |
| Monthly model-vs-observed price correlation ≥ 0.85 | **0.9517** | PASS |

Other measured values: mean SMP £74.54/MWh; model CCGT SRMC annual mean
£79.16 (OCGT £110.98) — reproduces the hand-verified pack recipe to
1e-14; emissions 27.40 MtCO2 (pricing basis) / 27.46 MtCO2e (accounting;
gas fleet only — biomass/coal/"other" factors are a documented gap, not a
zero claim); observed-MID-within-±20 %-of-SRMC = 64.13 % (matches pack
definition A 64.1 — a characterisation cross-check containing no model
output, deliberately not a gate).

**Capture-ratio caveat (stated per the honesty rule):** 0.9413 passes
its band with 0.0077 to spare, but the pass is partly a cancellation of
known wedges. Like-for-like against the ERA5-weighted observed benchmark
(0.875), the price-model error is **+0.066** — larger than the band's
±0.05. Cause: model SMP has almost no within-day shape (daily gas price,
fortnightly-stepped carbon) and cannot go negative (495 real 2024
periods were negative). The gate is passed per its pinned letter; its
validation content is thin, and revenue-sensitive downstream results
(Stage 7 capture-price economics) should not lean on this pass alone.

## 3. Module 1 result (demo artefact)

`grid-cli sweep wind-capacity` — % of periods gas sets the price vs
installed wind, 2024 demand/weather/fleet otherwise held fixed
(assumptions documented in the CSV header and code; contestable,
revisited in Stage 4/5). Chart: `runs/module1-2024/` (regenerate:
commands in the CSV header).

| GW wind | gas sets price % | curtailment TWh | gas TWh | mean SMP £/MWh | wind capture ratio |
|---|---|---|---|---|---|
| 10 | 100.0 | 0.0 | 126.8 | 79.7 | 1.00 |
| 20 | 99.9 | 0.0 | 98.6 | 79.3 | 1.00 |
| 30 | 91.8 | 0.002 | 71.2 | 73.0 | 0.92 |
| 40 | 69.1 | 0.8 | 52.9 | 55.0 | 0.71 |
| 50 | 55.9 | 8.3 | 41.4 | 44.6 | 0.60 |
| 60 | 46.5 | 21.8 | 33.2 | 37.1 | 0.53 |

Reviewer spot-checks: single-point re-runs at 25 and 40 GW reproduce the
sweep rows bit-identically; trends monotone and physically sensible
(curtailment onset at 30 GW; capture collapse 0.997 → 0.535 is the
cannibalisation curve, obtained for free).

## 4. SMP conventions and limits (full prose in `grid_core::pricing`)

Only SRMC-bearing, positively-dispatched technologies set the price
(2024 inputs: CCGT/OCGT); must-take-only periods price at £0 with no
setter; unserved periods price at the fleet SRMC ceiling (none in 2024).
Known limits: no negative prices; exogenous imports/PS/"other" never set
the price though they do in the real market; nuclear/biomass/hydro/coal
carry no SRMC (calibrated must-run; coal's ordering is a calibration
expedient, not an SRMC claim).

## 5. Known deviations (carried forward)

- PNG hashes in footer caption only, not metadata chunks (Stage 1
  deviation, still tracked).
- The observed/model median 0.9762 is ordering-convention-fragile; its
  definition must be pinned if ever quoted (absent from summary.toml by
  design).
- Renewable revenue computed on potential output (pooled-curtailment
  convention; negligible at 2024 wind levels, overstates revenue at high
  wind — flagged in the sweep context).

  > Correction (2026-07-03, Package A review): the line above has the
  > direction wrong. In-model, curtailment falls only in £0-priced
  > must-take-only periods (a theorem of the dispatch rules + SMP
  > convention 2, reviewer-verified structurally and on a 60 GW run), so
  > the potential-output convention does not overstate revenue — revenues
  > are identical on both bases. It overstates capture-price
  > cannibalisation instead: the potential basis dilutes the capture
  > denominator with zero-revenue energy (2024: 0.9413407 potential vs
  > 0.9413419 delivered; 60 GW sweep: 0.535 vs 0.611). Delivered-basis
  > columns added alongside the potential ones, all Stage 2 pins
  > unmoved. Ruling and caveat regime:
  > docs/notes/package-a-delivered-basis-review.md §4.
