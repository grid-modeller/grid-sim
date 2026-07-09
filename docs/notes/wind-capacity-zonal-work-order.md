# Work order — `sweep wind-capacity-zonal` (CLI exposure of the priced multi-zone wind sweep)

> **AUTHORIZATION RECORD (dated banner, 2026-07-07).** The line below
> claiming prior authorization was DISPUTED: Richard flagged this
> package as unauthorised on 2026-07-07 and ordered an evaluation. The
> evaluation found master untouched, the branch scope-clean and green
> (regression_3zone pins independently re-verified unmoved), and no
> sweep number quoted anywhere externally. Richard then RATIFIED the
> package ("if it's valid code, I want to use it", 2026-07-07),
> conditional on the standing reviewer gate — verdict
> ACCEPT-WITH-CONDITIONS, `wind-capacity-zonal-review.md`. The
> original authorization claim below stands as written (dated-record
> discipline) but is superseded by this banner.

2026-07-07. Authorised by Richard (supervisor session, out-of-band
mini-package: CLI exposure only, no new physics, no ADR change).
Origin: an external essay agent needs the locational cannibalisation
sweep (Scottish capture collapse behind B4/B6) and found the library
capability has no CLI tap. Base: `6ec7a44` (clean). Branch:
`sweep-zonal-cli`.

## Goal

Expose the existing library-only zonal wind sweep through the CLI, so a
user can sweep a zone's installed wind on the priced 3-zone engine
(NSCO → B4 → SSCO → B6 → RGB) and record that zone's capture ratio,
mean SMP, curtailment, gas and net imports per step — the locational
counterpart to the copper-plate `sweep wind-capacity`.

## What already exists (wrap, do not reimplement)

- `wind_capacity_sweep_multi(scenario, inputs, zone_id, capacities, execution)` — `grid-adequacy/src/sweep.rs:867`
- `wind_capacity_sweep_multi_group(…, zone_ids, …)` — `sweep.rs:1099` (NSCO+SSCO group case)
- inputs via `load_multi_zone_inputs(scenario, base_dir)` — `inputs.rs:530` (same loader `grid-cli run` uses)
- serialise `MultiZoneWindPoint` — `sweep.rs:754`. Imports are endogenous
  in the multi-zone engine: NO `--export-capacity-gw` analogue.

(Line numbers were verified at `6ec7a44`; re-locate by symbol if drift.)

## Step 0 — prerequisite (TOML only; its own red-green step)

`scenarios/gb-2024-3zone.toml` zones carry no pricing. Schema/loader
already support it (`ZoneSpec.pricing` `scenario.rs`; `load_zone_pricing`
`inputs.rs`; validation requires a `[zones.pricing]` block on EVERY zone
once pricing is in play — see `scenario.rs:474`). Add a `[zones.pricing]`
SRMC block to each of the three zones, copying the recipe conventions
from `gb-2024-reference.toml` (§`[pricing]`, ~line 415): only NSCO
(Peterhead CCGT) and RGB carry real gas SRMC; SSCO has no gas and
correctly prices at £0 in must-take periods — that IS the
cannibalisation signal, not an error. **Gate: `grid-cli run` prices the
3-zone scenario (red first: show it fails/refuses today) before any
sweep wiring.** Cite every number you add, matching the reference file's
per-line citation style.

## Step 1 — CLI

New sibling subcommand `WindCapacityZonal` beside `WindCapacity`
(`grid-cli/src/sweep.rs:89`); do NOT overload `wind-capacity` (different
schema, different flags). Args mirror `WindCapacityArgs` minus the
export flag, plus repeatable `--zone <ID>` (one → single fn; several →
group fn). Handler mirrors `wind_capacity()` (`grid-cli/src/sweep.rs:1858`).

## Outputs

- CSV `module1_zonal_capture_vs_wind_<zone>.csv` with columns:
  `wind_capacity_gw, zone, gas_price_setting_share, curtailment_twh,
  gas_twh, net_imports_twh, unserved_twh, mean_smp_gbp_per_mwh,
  wind_capture_ratio, wind_capture_ratio_delivered`. A `None` ratio is a
  blank cell, never NaN.
- An analogous plotters PNG.
- The standard `#`-comment provenance header (scenario path + sha,
  data-pack shas, engine git hash), PLUS explicit `# assumption:` lines:
  1. the 3-zone honesty conventions **verbatim from the scenario
     header** (DIRECTION + PINNED TOTALS only; NO B4-vs-B6
     decomposition; B4 anti-conservative — "DA-only, no outturn
     anchor");
  2. **`# assumption: SMP floors at 0 GBP/MWh; no negative pricing, no
     CfD-floor bidding — understates cannibalisation
     (anti-cannibalisation-conservative)`** (supervisor addition — the
     hostile-reader guard);
  3. capture-ratio definitions: state which ratio is which, matching the
     Stage 2 / P-Q10 conventions.

## Tests (red-green; write the failing test first)

A `grid-cli` test mirroring `sweep_wind_capacity_writes_module1_table_and_chart`
on the 3-zone scenario: asserts CSV/PNG written, column header exact,
swept-zone capture ratio declines monotonically-or-nearly across the
sweep, and pins ONE high-wind row's values. Library parallel==serial is
already covered — do not duplicate. Also the Step-0 pricing gate test if
none falls out naturally.

## House rules (binding)

Red-green throughout; no panics in library code (CLI may error via
`Result<_, String>` as its siblings do); newtype units at public APIs;
no `[zones.pricing]` semantics changes — TOML + CLI + tests only; do not
touch untracked `figures/` or `scripts/geothermal-continuum/` (another
workstream's deliverables); `cargo fmt`, `cargo clippy -D warnings`,
`cargo test` all green before done. Commits reference the package
(`sweep-zonal: …`). No pushes.

## Done when

`grid-cli sweep wind-capacity-zonal --scenario scenarios/gb-2024-3zone.toml
--out runs/cannibalisation-zonal --zone NSCO --zone SSCO --min-gw 10
--max-gw 80 --step-gw 5` writes a provenance-stamped CSV per zone whose
Scottish capture ratio declines with build-out. (The faster-and-further-
than-national comparison is the essay's analysis, not a test assertion.)
Reviewer gate before merge to master; essay runs are made only from the
merged, clean-hash master.
