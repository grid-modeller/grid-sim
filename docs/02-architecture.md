# 02 — Architecture Decision Record

**Status: immutable.** Changes require an explicit, versioned amendment with
rationale. Every coding session must be given this document.

## ADR-1: One project, one Cargo workspace, separate crates

```
grid/
├── grid-core/        # scenario schema, domain types, units, weather/demand
│                     # loading, pricing & emissions layers, timescale
│                     # decomposition, analysis utilities
├── grid-adequacy/    # chronological dispatch, storage policies, solvers,
│                     # sweep runner
├── grid-stability/   # inertia aggregation, swing equation, loss-of-infeed
│                     # event simulation, response services
├── grid-cli/         # research CLI: run, sweep, solve, validate, plot
└── grid-wasm/        # (phase two) thin WASM bindings for the web UI
```

Rationale: the two engines share almost nothing computationally (different
state, timesteps, solvers) but share the fleet description and scenario
schema. The coupling — stability consumes adequacy dispatch output — is only
possible with a shared domain model.

## ADR-2: Two timescales, two engines

- `grid-adequacy`: half-hourly chronological merit-order dispatch over
  arbitrary horizons (up to the full 40-year weather record, ~700k
  timesteps). Target: full 40-year single-zone run in milliseconds–low
  seconds, enabling brute-force parameter sweeps.
- `grid-stability`: single-bus aggregate swing-equation event model. Inputs:
  fleet as dispatched at a chosen timestep (from adequacy output) → aggregate
  inertia H → trip a defined infeed → RoCoF, frequency nadir, LFDD threshold
  checks, with frequency-response services (dynamic containment, static)
  layered on. Explicitly *not* EMT or multi-bus.

## ADR-3: Time representation

- Internal time is **UTC, half-hourly settlement periods**, monotonic index.
- Local time / clock changes handled only at I/O edges (data ingest, display).
- No naive datetimes anywhere in library crates.

## ADR-4: Units via newtypes

`Power(GW)`, `Energy(GWh)`, `Price(£/MWh)`, `Inertia(GVA·s)`, `PerUnit`,
`Emissions(tCO2)` as newtype wrappers with only physically meaningful
arithmetic implemented (e.g. `Power × Duration = Energy`). GW/GWh confusion
is the classic energy-model failure mode; the type system eliminates it.

## ADR-5: Determinism and reproducibility

- Engine is a pure function: `(scenario, engine version) → results`.
  No hidden state, no wall-clock dependence, no unseeded randomness.
- Scenario schema carries a mandatory `schema_version` field.
- Every output artefact embeds the engine git hash and scenario hash.
- Any stochastic feature (if ever added) takes an explicit seed in the
  scenario file.

## ADR-6: Storage dispatch policy is pluggable

```rust
trait DispatchPolicy {
    fn dispatch(&self, state: &SystemState, horizon: &Horizon) -> DispatchDecision;
}
```

Implementations: `RuleBased` (greedy/heuristic, no foresight — the default,
higher and more defensible storage numbers) and `PerfectForesight` (LP over
the horizon via HiGHS through `good_lp`). Results for headline claims are
reported under both; the gap is a documented finding, not a bug.

## ADR-7: Multi-zone from day one in the schema, single-zone in v1 engine

`Scenario` holds `Vec<Zone>` plus a link (interconnector) matrix. The v1
engine may reject scenarios with >1 zone, but the schema never changes shape
when zones arrive (Stage 5). External zones planned: FR, NO, NL/BE/DE
aggregate — chosen so import availability and price *emerge* from the
exporter's own residual load rather than being assumed.

**Amendment (ratified by Richard 2026-07-03):** the external zone list
is FR, CONT-NW (BE+NL+DE-LU aggregate), NO2 (replacing the NO
aggregate), DK1, IE-SEM — the five zones adjudicated at Stage 5 design
from the imports-identity arithmetic (DK1, IE-SEM), the NSL scarcity
signal and the weather-pack NO4 gap (NO2). Evidence and reviewer
adjudication: `docs/notes/d5-zone-granularity.md`.

## ADR-8: Storage is a portfolio

`Vec<Storage>`, each with power (GW), energy (GWh), round-trip efficiency,
and dispatch order. Presets: battery, pumped hydro, hydrogen (η ≈ 0.35–0.40).
Demand-side response is modelled as a pseudo-storage entry with shift-duration
and volume limits.

## ADR-9: Pricing, emissions, and inertia live in grid-core

- Pricing: SRMC per technology (fuel + carbon), system marginal price per
  period, per-technology revenue accounting, annuitised capex for LCOE.
- Emissions: tCO2/MWh per technology, per-run totals.
- Stability metadata: inertia constant H and `synchronous: bool` per
  technology, so `grid-stability` consumes adequacy output directly.

## ADR-10: Numerical strategy

- Headline solvers: bisection (monotone 1-D problems, e.g. minimum storage
  for zero unserved energy) and brute-force sweeps parallelised with `rayon`
  (keep full response surfaces, don't just report optima).
- Multi-parameter search: `argmin` (Nelder-Mead first).
- LP mode: `good_lp` + HiGHS (same solver PyPSA uses).
- Arrays: `ndarray` where useful; plain `Vec` otherwise.

## ADR-11: Engine before UI

Phase one deliverable is the library + CLI producing CSV/Parquet and PNG
charts (via `plotters`) — the research instrument for the book. The web UI
(WASM, client-side, no server) is phase two with its own spec. The UI must
never acquire logic the engine lacks.

## ADR-12: Transmission constraints as a cost approximation

No network model. A single GB constraint-cost function keyed to Scottish
wind output approximates B6 boundary constraints, closing the most obvious
critic-visible gap. Fields exist in the schema; may be null in early stages.
