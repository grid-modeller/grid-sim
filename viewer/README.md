# grid-sim run viewer

A single-file, offline HTML viewer for grid-sim engine run outputs. Open
`viewer/index.html` in any modern browser (double-click; it works from
`file://` with no server, no network, no dependencies) and drop a run's
output files onto it.

This is the prototype for the **phase-2 sandbox's view layer** ("grid game").
It exists to work out the visual grammar — dispatch stack, price strip,
storage strip, provenance header — against real engine outputs before any
interactive sandbox is built.

## Guardrails

This viewer **reads engine outputs only and computes nothing the engine
didn't** (ADR-11: the UI never acquires logic the engine lacks). It performs
no dispatch, pricing, residual-load, or storage arithmetic. The single
transformation it applies is *display aggregation* — daily means of the
engine-emitted half-hourly values when zoomed out — and the on-screen status
line always states which mode is showing. Derived views (the deficit/surplus
strip, the storage state-of-charge strip) render **only if the engine emitted
the corresponding columns**; if the columns are absent, the strip is omitted
rather than reconstructed. The engine output schema is the UI contract:
columns are discovered dynamically from the CSV header, so new engine columns
appear without viewer changes.

## Usage

1. Generate or locate a run directory, e.g.
   `cargo run -p grid-cli -- run --scenario scenarios/gb-2024-reference.toml --out runs/ref`
   (schema v2 CLI; no `--inputs` flag)
2. Open `viewer/index.html` in a browser.
3. Drag these files from the run directory onto the drop zone (or click it
   to browse; multi-select works):
   - `dispatch.csv` — required; the main stacked chart, reliability strip,
     SoC strip
   - `summary.toml` — optional; headline numbers and hashes in the header
   - `prices.csv` — optional; adds the SMP price strip
   - `monthly_mix.csv` — optional; adds the monthly-energy bar panel
4. Navigate: **scroll** to zoom (year → single day), **drag** to pan,
   **double-click** or "Reset zoom" to return to the full run. Hover for a
   crosshair tooltip with the engine-emitted values at that period.

Files are parsed locally in the browser; nothing is uploaded anywhere.

## What it shows

- **Dispatch stack** — generation GW by technology (fixed semantic palette:
  nuclear dark violet, gas grey/brown, wind teal/green, solar yellow,
  imports blue, pumped storage pink, unserved bright red on top of the
  stack plus red top-edge ticks so sub-daily events can't hide in a daily
  mean). Negative values (imports, pumping) stack below the axis;
  curtailment is hatched below the axis. Demand is the black line.
- **Deficit/surplus strip** — rendered only if the engine emits a
  `residual`/`surplus`/`deficit` column in `dispatch.csv`; omitted otherwise
  (the viewer will not derive it).
- **Price strip** — SMP line from `prices.csv`; zero-price periods shaded and
  labelled "no price-setter (must-take only)". A thin band along the strip's
  base encodes the engine-emitted `price_setter` column (CCGT in the gas
  grey-brown, OCGT darker brown; setter names and colours are discovered from
  the data). In daily mode the band shows the day's setter mix as stacked
  fractions of the day's half-hours (display aggregation); the tooltip gives
  the exact percentages, or the single setter when zoomed to half-hours.
- **Reliability strip** — the engine-emitted `firm_share` ratio (unclamped),
  rendered per the owner's gridmargin convention: an OKLab traffic-light
  background ramp (green `#1b6e45` at firm share ≥ 0.65, amber `#e6a019`
  midway, red `#d61231` at ≤ 0.40) and a marked alarm threshold at 0.5.
  In daily mode the strip shows each day's **minimum** firm share — the worst
  half-hour wins (keepWorstHigh) — and says so in its title. The tooltip adds
  the engine's aggregate supply buckets (firm / variable / storage GW).
- **Storage SoC strip** — one line per `*_soc_gwh` column
  (`pumped_hydro_soc_gwh`, `battery_soc_gwh` in schema v2).
- **Monthly mix panel** — `monthly_mix.csv` is the engine's per-calendar-month
  energy ledger (GWh) used for validation (see `grid-cli plot monthly-mix`).
  It renders as a static stacked-bar panel (demand as a dash per month), not
  linked to the time zoom. The current schema emits modelled values only; if
  the file gains actual-vs-modelled column pairs the panel should become
  grouped bars (not yet implemented).
- **Provenance header** — engine git hash, scenario/inputs sha256, creation
  timestamp, schema version (from the CSV `#` metadata header and
  `summary.toml`), plus headline results. Mismatched engine hashes across
  loaded files raise a visible warning.

## Aggregate columns (excluded from stacking)

Schema v2 `dispatch.csv` emits AGGREGATE columns alongside the
per-technology ones: `firm_supply_gw`, `variable_supply_gw`,
`storage_discharge_gw` (sums of per-technology columns) and `firm_share`
(a dimensionless ratio). Stacking these on top of the technologies would
double-count supply, so the viewer excludes them from the dispatch stack
while still parsing them (they feed the reliability strip and the tooltip's
bucket breakdown). Exclusion is by documented naming convention
(`firm_supply|variable_supply|storage_discharge|total_supply|total_generation`
with a `_gw`/`_gwh` suffix; anything ending `_share`/`_ratio` is a ratio and
never stacked), **plus** a numeric sanity rule: any remaining column whose
values equal the sum of all the other stacked columns (sampled rows, 0.1%
tolerance) is treated as a total and excluded too — so a future `total_gw`
cannot silently double the stack. If the engine adds a new aggregate under a
different name, add it to `AGGREGATE_RE` in `index.html`.

## Storage SoC columns

Schema v2 emits `pumped_hydro_soc_gwh` and `battery_soc_gwh` in
`dispatch.csv`; the SoC strip picks up any `*_soc_gwh` column
automatically (the rendering path was originally built against a synthetic
column before the engine emitted real ones). If future column names differ,
the discovery pattern (`/_soc_gwh$/`) is the one line to update.

## Notes and limitations

- One run at a time; dropping a new `dispatch.csv` replaces the current one.
  Run comparison is out of scope for this prototype.
- Timestamps are displayed in UTC, as emitted.
- Rendering is canvas-based; at full-year zoom the chart shows labelled
  daily means (17,568 half-hours would be sub-pixel), switching to raw
  half-hours below roughly a two-month window.
- `parquet` outputs are not read; use the CSV twins.
- The lenient TOML parser is display-only (flat `key = value` under
  sections); it does not validate the file.
- `window.__viewer` is a debug hook (`state`, `setRange(t0,t1)`,
  `loadText(text, filename)`) used by the automated end-to-end check; it is
  not part of the UI contract.
