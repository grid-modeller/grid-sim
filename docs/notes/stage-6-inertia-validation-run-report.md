# Stage 6 NESO enrichment (Task 7) — bottom-up vs NESO inertia correlation: results

Committed record of the `grid-cli stability validate-inertia` run against
the 2024 data pack (run 2026-07-07), correlating the bottom-up
`grid_stability::inertia_from_generation` estimate (computed directly
from the pack's `generation_by_fuel_2024` table) against the NESO System
Inertia outturn series (`inertia_outturn_2024`, built in Tasks 1–3 from
the pinned NESO System Inertia CSVs, 2023-24 + 2024-25 editions, covering
calendar 2024). **Supported by National Energy SO Open Data.**

## 1. Observed numbers

Full-year 2024 run, inner-joined on the shared `UtcInstant` half-hour
index (both tables cover the same 17,568-period year, so every period
matched — no gap periods were dropped):

| Quantity | Value |
|---|---|
| n (matched periods) | 17,568 |
| Pearson r | 0.9575443124106625 |
| OLS slope (`neso ≈ slope·ours + intercept`) | 1.5427278883728945 |
| OLS intercept, GVA·s | 53.333651518948145 |
| median ratio (`neso[i] / ours[i]`, nonzero `ours` only) | 2.2537730575740635 |

Reproduce with:

```
grid-cli stability validate-inertia --base-dir . --out <report.toml>
```

## 2. Interpretation

**Strong linear tracking (r ≈ 0.958):** the bottom-up transmission-only
sum moves with NESO's outturn estimate across the year — when
synchronous transmission-connected generation rises or falls, NESO's
own model estimate moves the same way. This is the headline finding:
independently computed from a completely different input (raw FUELHH
generation MW, not NESO's own dispatch/data assembly), the two track
each other closely.

**The intercept (~53 GVA·s) is the near-constant inertia the
transmission-only bottom-up method omits.** `inertia_from_generation`
sums only the six synchronous FUELHH fuel columns visible at
transmission level (see §3); it has no visibility into:

- embedded (distribution-connected) synchronous generation — smaller
  reciprocating/steam sets not metered by FUELHH,
- the rotating mass of synchronous demand-side plant,
- the two documented parity gaps (§3): pumped storage and oil.

None of these vary hugely period-to-period relative to the swings in the
dispatched synchronous fuels, so they show up as a roughly constant
offset rather than as noise — exactly what an OLS intercept captures.
The median ratio of ~2.25 says NESO's outturn typically runs more than
double the bottom-up sum, consistent with a large, close-to-constant
omitted component riding under a smaller variable one.

**NESO's outturn is NESO's own model estimate, not a measurement.**
There is no independent GB-wide "true" system inertia meter; NESO
publishes System Inertia as a modelled operational estimate from their
own methodology. This validation therefore checks *agreement between
two independent inertia-estimation methods*, not bottom-up-vs-ground-
truth — NESO's series is used here as an external reference series, not
as ground truth to be matched exactly.

## 3. Fuel mapping: contributing vs omitted

`generation_by_fuel_2024` columns are translated to `grid_core` tech ids
before `inertia_from_generation` runs (`translate_fuel_column`,
`grid-cli/src/stability.rs`):

- **Contributing (nonzero H, synchronous):** `biomass`, `ccgt`, `coal`,
  `npshyd → hydro`, `nuclear`, `ocgt` — six fuels, all already matching
  (or translated to) a `grid_core::inertia::technology_default`
  synchronous arm.
- **Included, zero contribution (non-synchronous by default):**
  `intelec`, `intew`, `intfr`, `intgrnl`, `intifa2`, `intirl`, `intned`,
  `intnem`, `intnsl`, `intvkl` (the ten FUELHH interconnector columns),
  `oil` (no `grid_core` arm at all — a known parity gap), `other`,
  `wind`.
- **Omitted outright:** `ps` (pumped storage). PS is synchronous only
  via `grid_core::inertia::storage_kind_default`, which
  `inertia_from_generation` does not reach (that function only consults
  `technology_default`, keyed by fuel name, not storage kind) — so
  passing the raw `ps` column through untranslated would silently
  contribute 0 while hiding a real method gap. It is dropped from the
  bottom-up input and documented here instead. This is the standing
  **asymmetry vs the engine's own `system_inertia`** (used elsewhere in
  Stage 6, e.g. the Module 6 sweep), which DOES count pumped storage via
  `storage_kind_default` when it is synchronised. `oil` and `ps` are both
  small, known parity gaps between this validation harness and the
  engine's dispatch-based inertia accounting — not silent omissions.

## 4. Provenance

- Inputs: `data/packs/2024/processed/generation_by_fuel_2024.parquet`
  (Elexon Insights FUELHH, BMRS open-data licence) and
  `data/packs/2024/processed/inertia_outturn_2024.parquet` (NESO Data
  Portal "System Inertia", 2023-24 + 2024-25 editions, NESO Open Data
  Licence — pinned in `grid-cli/src/fetchdata/mod.rs::sources`).
- Engine: `grid_stability::inertia_from_generation` +
  `grid_stability::correlate` (`grid-stability/src/reference.rs`), fed by
  `grid-cli stability validate-inertia`
  (`grid-cli/src/stability.rs::validate_inertia`).
- Smoke-tested: `grid-cli/tests/stability_validate_inertia_smoke.rs`
  (subcommand runs against the real pack, exit 0, report contains
  `pearson_r`). The exact-value pin (Task 8) lives in a separate
  regression test.

## 5. Engine-level correlation (Task 9): the honest test

§§1–4 above correlate a **method** — `inertia_from_generation` applied
directly to actual 2024 generation — against NESO's outturn. That is
not the engine itself: the engine's own inertia accounting
(`grid_stability::inertia`, used by `grid-cli stability inertia`) is
computed from a **scenario dispatch result**, not from actual
generation. This section correlates *that* — the full engine, cost-
optimised dispatch and all — against the same NESO outturn series, run
2026-07-07.

### 5.1 Observed numbers

`grid-cli stability inertia --scenario scenarios/gb-2024-reference.toml
--reference data/packs/2024/processed/inertia_outturn_2024.parquet`, the
canonical 2024 reference scenario (the same scenario
`regression_2024.rs` drives), inner-joined by UTC instant against the
engine's own `inertia_series` (each period `t`'s instant is
`result.timestamp_at(t)` — the exact instant the dispatch engine uses,
not an assumed positional alignment). Both series cover the same
full 2024 year, so every period matched:

| Quantity | Value |
|---|---|
| n (matched periods) | 17,568 |
| Pearson r | 0.9372226149035376 |
| OLS slope (`neso ≈ slope·engine + intercept`) | 1.3306682984048053 |
| OLS intercept, GVA·s | 70.03133259577058 |
| median ratio (`neso[i] / engine[i]`, nonzero `engine` only) | 2.2104643950262224 |

Reproduce with:

```
grid-cli stability inertia --scenario scenarios/gb-2024-reference.toml \
  --base-dir . --out <dir> \
  --reference data/packs/2024/processed/inertia_outturn_2024.parquet
```

### 5.2 Interpretation

**Lower than the method-level r (0.9575) — expected, and the honest
finding.** The gap (0.9575 → 0.9372) is the cost of scenario dispatch
diverging from actual 2024 unit commitment: `gb-2024-reference.toml`
runs the adequacy engine's own merit-order/storage dispatch against
2024 weather and demand, which does not reproduce NESO's/BMU operators'
actual minute-by-minute commitment decisions (start-up costs, must-run
constraints, balancing actions, and other operational detail the market
-only scenario dispatch does not model). Both methods still track
NESO's outturn strongly — the engine's cost-optimised dispatch is not
detached from reality, it is simply one further step removed from the
actual generation record than the bottom-up method in §§1–4, which is
computed directly from that record.

The slope/intercept/median-ratio pattern (slope > 1, ~70 GVA·s
intercept, median ratio ~2.2) mirrors the method-level finding in §2:
the engine's dispatch-based inertia accounting has the same structural
gap against NESO's outturn — omitted embedded/distribution-connected
synchronous plant and demand-side rotating mass that no bottom-up or
scenario-dispatch method sees.

### 5.3 Provenance

- Engine: `grid_stability::inertia_series` (fed by a `gb-2024-reference
  .toml` dispatch run) + `grid_stability::correlate`
  (`grid-stability/src/reference.rs`), joined and reported by
  `grid-cli stability inertia --reference`
  (`grid-cli/src/stability.rs::inertia`,
  `engine_vs_neso_fit`).
- The exact-value pin (Task 9, Pearson r only, ±0.01) lives in
  `grid-cli/tests/regression_inertia_validation_2024.rs::engine_inertia_tracks_neso_2024`.
