# Stage 1 — 2024 validation run: results, wedges, and what was actually tested

Committed record of the Stage 1 honesty-gate run (2026-07-02), per review
condition: the numbers below were produced by the implementer and
independently re-measured by the reviewer (two fresh CLI runs, identical
digests; the counterfactual in §3 is the reviewer's own measurement).

Reproduce:

```
cargo run -p grid-cli -- run --scenario scenarios/gb-2024-reference.toml \
    --inputs scenarios/gb-2024-reference-inputs.toml --out runs/gb-2024-reference
cargo run -p grid-cli -- plot monthly-mix --run runs/gb-2024-reference \
    --actual data/packs/2024/processed/monthly_generation_2024.csv \
    --out runs/gb-2024-reference/monthly_mix_2024.png
```

Result digest (engine-behaviour-stable, pinned by regression test):
`779d7444577b0ef1d2201835fd36616c4eed8bfab0d58c3c82c19d0ac2541abd`
(the live pin, `grid-cli/tests/regression_2024.rs`. The original
schema-v1 digest `6f82c7b0…c088c5d` was SUPERSEDED at the schema-v2
migration — recorded here so an older transcript reconciles; it is
not the reproducible value on the current engine.)

## 1. Headline results vs docs/04 Stage 1 gates

| Gate | Tolerance | Achieved |
|---|---|---|
| Annual gas (CCGT+OCGT) | ±5 % of 72.79 TWh | **73.45 TWh, +0.91 %** |
| Net annual imports (exogenous) | ±1 % of 33.30 TWh | **33.299 TWh, −0.003 %** |
| Monthly mix correlation (flattened 12×8) | ≥ 0.95 (tightened to ≥ 0.99 post-run, §4) | **r = 0.9970** |
| Unserved energy | zero | **0 exactly** |
| Determinism | identical output hashes | **bit-identical across runs and rebuilds** |

Tightest system moment: thermal margin ≈ 0.23 GW at 2024-01-16 10:00Z.
Curtailment: 0.137 GWh over 2 periods. Other annual series (TWh, modelled
vs actual): wind 82.61 / 82.61 and solar 13.95 / 13.95 (exact **by
construction** — calibrated), nuclear 38.24 / 38.33, biomass 18.50 /
18.80, hydro 3.42 / 3.58, coal 1.46 / 1.57, OCGT 0.001 / 0.17 (known
deficiency; the gate is on the gas total).

## 2. What is fed in vs what is predicted (the circularity inventory)

Fed in as observed data or calibrated to 2024 outturn:

- Demand: underlying demand (ND + NESO embedded estimates) + constant
  0.667 GW station load.
- Exogenous must-take traces: net imports, pumped-storage net output,
  FUELHH `other` (§3) — all observed half-hourly series from the pack.
- Wind/solar CF traces: ERA5-derived *shapes*, annual totals calibrated to
  2024 actuals (factors 0.90 / 1.04 / 0.88).
- Availabilities: nuclear monthly profile derived from observed monthly
  output (its per-fuel monthly r = 0.999 is therefore **circular**);
  biomass and hydro flat factors and the coal Jan–Sep window derived from
  observed annual energies.

Genuinely predicted by the engine:

- **The gas fleet's monthly shape: per-fuel r = 0.995 — the real content
  of this gate.** Gas is the residual after weather-driven renewables and
  calibrated must-run/mid-merit plant; nothing about its month-to-month
  pattern is fed in.
- **Chronological feasibility**: zero unserved energy at a realistic
  ~0.2 GW minimum margin and near-zero curtailment, tested half-hour by
  half-hour — calibrated *annual* totals do not guarantee this.
- The gas annual *level* only weakly: the +0.66 TWh error is
  arithmetically the sum of the calibrated plants' clipping shortfalls
  (nuclear −0.09, biomass −0.30, hydro −0.16, coal −0.11 TWh) — an
  accounting residual of the calibration, **not** 0.91 % of model skill
  against a 5 % budget.

Not modelled at all: biomass/hydro monthly shape (per-fuel r ≈ 0, hidden
inside the flattened metric); OCGT dispatch (see §1).

Discrimination check (reviewer-measured): a zero-skill model holding every
fuel flat at its observed annual mean scores r = **0.934** on the
flattened metric. The original ≥ 0.95 threshold therefore barely excluded
zero skill; the achieved 0.997 is meaningfully above it, and the threshold
is now ≥ 0.99 (§4).

## 3. The four wedges, including the load-bearing one

Per docs/04, the ±5 % gas tolerance is conditional on handling the
quantified correctable wedges. As implemented (all declared with
provenance in `scenarios/gb-2024-reference-inputs.toml`, every derived
number pinned to the pack by a characterisation test):

1. **Station transformer load** (5.86 TWh/yr): demand-side constant
   +0.667 GW.
2. **Pumped-storage net output** (0.60 TWh round-trip loss): exogenous
   must-take trace — reproduces intra-day peak-shaving, keeps the loss
   off modelled gas.
3. **Coal closure 2024-09-30**: monthly availability window (Jan–Sep),
   reproducing Ratcliffe's 1.57 TWh under the documented
   coal-before-gas calibration ordering.
4. **FUELHH `other`** (3.35 TWh of real but unrepresentable generation —
   no fleet entry exists for it): exogenous must-take trace. **This wedge
   is load-bearing: with it removed, modelled gas is 76.65 TWh, +5.30 %,
   and the gate FAILS.** (Reviewer counterfactual measurement,
   2026-07-02.) It is judged defensible — the trace is observed
   generation, checksummed in the pack, declared and hashed in the
   run-inputs file — but any future claim that "the model reproduces 2024
   gas burn" must carry this caveat.

## 4. Post-run gate adjustments

- Monthly-mix threshold tightened in docs/04 from ≥ 0.95 to **≥ 0.99**
  (docs/04 anticipated this revisit; evidence: achieved 0.997, naive
  baseline 0.934).
- The reference run is pinned as a regression test (digest + gas total),
  per the CLAUDE.md published-number rule.

## 5. Known deviations and boundaries (in addition to docs/05's list)

- Merit order is a fixed documented Stage 1 stack (SRMC arrives in
  Stage 2); coal-before-gas is a calibration expedient, documented and
  test-pinned in `grid-adequacy/src/dispatch.rs`.
- The run-inputs file extends the ADR-5 determinism formula to
  `f(scenario, run-inputs, data pack, engine)` — addendum in docs/03,
  proposed ADR amendment recorded in `memory/project-state.md`.
- `grid-cli run` exits 0 when unserved energy is nonzero (reported in the
  summary); exit 1 is reserved for solver infeasibility (documented
  reading of docs/06).
- Chart hashes appear in the PNG footer caption but not in PNG metadata
  chunks (docs/06 asks for both).
- Cross-platform determinism asserted but only single-platform verified.
