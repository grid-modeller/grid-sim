# 03 — Domain Model and Scenario Schema

**Status: versioned.** Any change to the schema requires a `schema_version`
bump and a migration note in this file.

## Scenario file (TOML)

A scenario is the complete, self-contained description of a run. Shareable,
diffable, hashable. Parsing is strict (Stage 0 decision, reviewer-ratified):
unknown fields are rejected — any schema addition requires a
`schema_version` bump — and `schema_version` is probed before the full
parse, so files from another schema version fail with a clear migration
message rather than field-level errors (05-validation rule 4).

**Stage 1 addendum — run-inputs file (2026-07-02). SUPERSEDED by schema
v2 (2026-07-02, Stage 3).** Stage 1 introduced a companion *run-inputs*
TOML carrying demand-column selection and adjustment, exogenous must-take
supply traces, per-technology availability calibration and (from Stage 2)
the pricing inputs. Schema v2 folds all of it into the scenario file: the
run-inputs mechanism, its parser and `grid-cli run --inputs` no longer
exist. "A scenario is the complete, self-contained description of a run"
holds again without qualification; the ADR-5 determinism formula returns
to `results = f(scenario hash, data-pack checksums, engine git hash)`,
and the proposed ADR-5 amendment recorded in `memory/project-state.md`
is dissolved. The v1 reference pair is frozen verbatim under
`grid-core/tests/fixtures/` so the migration error path stays tested.

### Migration note: schema v1 → v2 (2026-07-02, Stage 3)

`schema_version = 2`. v1 files are refused with a structured migration
message naming what moved (05-validation rule 4). Changes:

1. **Run-inputs folded in** (supersedes the Stage 1 addendum): run-inputs
   `[demand]` `column`/`extra_demand_gw` → `[zones.demand]` (defaults:
   `underlying_demand` per D3; 0); `[[exogenous_supply]]` →
   `[[zones.exogenous_supply]]`; `[availability.<tech>]` →
   `availability = { flat = … }` or `{ monthly = [ … ] }` on the
   `[[zones.fleet]]` entry (rejected on weather-driven entries);
   `[pricing]` → a top-level `[pricing]` block. All shapes otherwise
   unchanged.
2. **Stage 3 storage fields** on `[[zones.storage]]`: `initial_soc`
   (fraction of `energy_gwh`; default **full** per D4) and the DSR-only
   `shift_duration` (hours) / `daily_volume_limit` (GWh) — schema shape
   only; the engine rejects `kind = "dsr"` until the Q6 work (D4).
3. **Multi-file traces**: `base_profile`, `capacity_factor_trace` and
   exogenous `path` accept a single Parquet path or a list of consecutive
   (typically per-year) files, concatenated in order with continuity
   validated — multi-year horizons, docs/04 Stage 3. CF traces read the
   pinned `cf` column (Stage 1 convention, unchanged).
4. **Semantic validation** (`Scenario::validate`, run by the engine and
   `grid-cli validate`): unique storage `dispatch_order` per zone (D4
   rule 2), physical storage parameter ranges, DSR-only fields restricted
   to DSR stores, availability factors in [0, 1].
5. **Reliability classification** (gb-grid-margin methodology,
   implemented as published — binary, correlated-failure criterion, no
   derating; `docs/notes/reliability-classification.md`):
   `[[zones.fleet]]` entries accept an optional
   `reliability = "firm" | "variable"`; the default is **derived**
   (capacity_factor_trace present ⇒ variable, absent ⇒ firm), which
   reproduces the published roster for the standard technology set.
   Explicit overrides are legal (the classification is a contestable
   modelling assertion made visible) but are always emitted into run
   outputs (`[results.reliability.overrides]`) so they cannot hide.
   `[[zones.exogenous_supply]]` entries **require** an explicit
   `reliability = "firm" | "variable" | "excluded"` — no safe default
   exists for hand-written series; the reference values are imports →
   `variable` (blocking highs becalm GB and its neighbours together),
   FUELHH "other" → `firm`, pumped-storage net → `excluded` (pumping is
   demand; PS supply sits in neither bucket).
6. **Storage discharge is its own fourth output category, never folded
   into firm**: the published analysis has no storage representation, so
   the simulator reports it separately (`storage_discharge_gw`;
   `"storage"` in the classification map) and the question whether
   storage-backed supply counts as reliable stays visibly open. Headline
   outputs: per-period `firm_supply_gw`, `variable_supply_gw` and the
   **unclamped** `firm_share` (firm/demand; net-export periods
   legitimately exceed 1.0), with annual mean/min/P25 and the count of
   periods below the 0.5 alarm threshold in `summary.toml`.

Known modelling tension, recorded: the migrated 2024 reference scenario
carries both the exogenous observed pumped-storage trace (Stage 1
wedge 2) and active pumped-hydro/battery stores. Under D4's initial-full
default the stores are provably inert on that run (zero headroom to
charge, no post-stack deficit to discharge into), so there is no double
counting — see the scenario's storage-section comment and the pinned
test `active_storage_does_not_act_on_the_2024_reference_run`.

### Migration note: schema v2 → v3 (2026-07-03, Stage 6)

`schema_version = 3`. v2 files are refused with a structured migration
message (05-validation rule 4). One addition: **stability metadata on
`[[zones.fleet]]` entries** — optional `inertia_h` (the inertia constant
H in seconds, machine-MVA base) and optional `synchronous` (bool). Both
fields are optional, so a v2 file migrates by setting
`schema_version = 3` and changing nothing else. Details:

1. **Derived defaults**: when the fields are absent, per-technology
   defaults apply, transcribed in `grid_core::inertia` from the
   per-number-cited evidence file
   `data/reference/inertia-constants.toml` and drift-guarded by
   `grid-core/tests/inertia_defaults.rs`. Synchronous with H: ccgt 5.0,
   ocgt 4.0, nuclear 4.5, coal 4.0, biomass 4.0, hydro 3.0 s.
   Non-synchronous, no H: onshore/offshore wind, solar, interconnector
   — and **any technology id the model does not know** (the honest
   default; scenarios claiming otherwise must say so explicitly).
2. **Overrides are surfaced**: explicit `inertia_h`/`synchronous` are
   legal (the classification is a contestable modelling assertion made
   visible) but always emitted into run outputs, exactly like the
   `reliability` field.
3. **Semantic validation** (`Scenario::validate`): `inertia_h` must be
   positive and finite; `inertia_h` on an effectively non-synchronous
   entry is rejected (decoupled plant contributes no inertia — set
   `synchronous = true` to claim a synchronous coupling); an effectively
   synchronous entry with no explicit H and no derived default is
   rejected.
4. **MW→MVA convention**: H is quoted on the machine MVA base; scenario
   capacities are real GW. A single documented fleet power factor 0.9
   (`grid_core::inertia::DEFAULT_POWER_FACTOR`, centre of the evidence
   file's 0.85–0.95 band) is applied at the one conversion point
   `Power::apparent`, so the dispatch-keyed inertia sum is
   `E = Σ H_i × (dispatched GW_i / 0.9)` over synchronised plant. A
   per-entry power-factor override was rejected for v1: no public GB
   per-unit MVA register exists to populate it.
5. **Storage kinds carry no schema fields**; their mapping lives in
   `grid_core::inertia::storage_kind_default`: pumped storage
   synchronous, H = 4.5 s, contributing in **both** pumping and
   generating modes while synchronised; battery, DSR and hydrogen
   non-synchronous — hydrogen by documented v1 modelling choice (the
   reconversion technology is unspecified in the schema; fuel cells are
   inverter-coupled, hydrogen turbines would be synchronous), which
   makes the zero-inertia finding on all-variable fleets an *output*,
   not a smuggled assumption. A hydrogen-turbine scenario should model
   the turbines as a fleet entry with an explicit H.
6. **Exogenous supply traces carry no inertia** (hand-written series
   have no machine model; the resulting 2024 understatement is
   ~1–3 GVA·s, recorded in the Stage 6 run report).

The v2 reference scenario is frozen verbatim under
`grid-core/tests/fixtures/v2-gb-2024-reference.toml` so the migration
error path stays tested (v1 precedent).

### Migration note: schema v3 → v4 (2026-07-03, Stage 5)

`schema_version = 4`. v3 files are refused with a structured migration
message (05-validation rule 4). The multi-zone activation fields — all
optional or defaulted, so a v3 file migrates by setting
`schema_version = 4` and changing nothing else:

1. **`[[links]]`** gains optional `name` (per-link identity for outputs
   and per-border validation; absent → derived `<from>-<to>-<index>`)
   and `loss` (default 0.0, in [0, 1); the receiving end gets
   `sent × (1 − loss)` — the HVDC sending/receiving-end metering wedge,
   `docs/notes/entsoe-stage5-pack-report.md` §3).
2. **`[[zones.fleet]]`** gains optional `energy_budget` (`trace`,
   `columns`, `window_periods` default 336 = one week): the
   seasonal-budget model for budget-limited dispatchables (NO2
   reservoir hydro, D5) — the named MW columns are summed per
   consecutive window from horizon start, each window's energy is
   released as dispatch allowance at its first period, unused allowance
   carries forward, and the per-period ceiling is
   `min(capacity × availability, allowance/Δt)`. Dispatchable entries
   only.
3. **`[zones.demand]`** gains optional `extra_profiles` (a list of
   `{ path, column }`) summed onto the base profile before
   `annual_scale` — aggregate-zone demand (CONT-NW = DE-LU + BE + NL).
4. **Semantic validation**: unique zone ids; link endpoints distinct,
   `availability` in [0, 1], `loss` in [0, 1); multi-zone scenarios
   require every link endpoint declared (single-zone scenarios may keep
   external counterparty ids while links are inert — the GB reference
   pattern); `energy_budget` needs ≥ 1 column and `window_periods` ≥ 1.
5. **Engine**: single-zone scenarios run exactly as before (dispatch
   digest-pinned; a loud `UnsupportedFeature` error guards
   `energy_budget` on the single-zone path). Multi-zone scenarios run
   under `grid_adequacy::run_multi` — per-zone merit dispatch plus the
   scarcity-equalising flow rule, whose normative prose lives in the
   grid-adequacy `flow` module; link net positions appear as
   imports-flagged series labelled by link name.

The v3 reference scenario is frozen under
`grid-core/tests/fixtures/v3-gb-2024-reference.toml` so the migration
error path stays tested.

### Migration note: schema v4 → v5 (2026-07-03, Q5/Q11 heating overlay)

`schema_version = 5`. v4 files are refused with a structured migration
message (05-validation rule 4). One change, and it is **not purely
additive**: `[zones.demand.heating]` is REPLACED. The v1–v4 sketch
block (`enabled` / `heat_demand_per_degree` / `cop_curve` — opaque
placeholders that no engine code ever read) is removed; schema v5
carries the heating **technology portfolio** of the adopted design
note `docs/notes/d9-heating-overlay.md` (rule 2 — field names there
are normative):

1. **`[zones.demand.heating]`** (optional; absent ⇒ the engine
   byte-path is untouched and pre-v5 dispatch digests are
   bit-identical): `delivered_heat_twh` (the **record-mean** annual
   buildings-heat quantum, TWh — space + DHW, domestic + services, the
   ECUK scope), `electrified_share`, `dhw_fraction` (the
   temperature-independent hot-water floor's share of the electrified
   quantum), and `temperature_trace = { path, column }` (the pinned
   population-weighted °C air-temperature trace, whole calendar
   years — the intensity `k` and the GSHP ground wave are computed
   over its full record, so `heat(t)` is a pure function of `T_pop(t)`
   and horizon subsetting never changes it).
2. **`[[zones.demand.heating.entries]]`**: `kind = "ashp" | "gshp" |
   "district_geothermal"`, `share` (shares sum to 1 within 1e-9 —
   structured error naming the sum and entries), and optional
   per-entry COP overrides (`cop_curve`, `correction_factor`,
   `rhpp_derating` on heat-pump kinds; `cop_const` on district) —
   defaults live in the cited, drift-guarded
   `data/reference/heating-cop.toml`, and overrides are always echoed
   into run outputs (the reliability/inertia precedent). Kinds are
   unique per portfolio (per-entry output series are keyed by kind).
3. **Engine**: heating electrical demand ADDS to zone demand before
   dispatch (`demand(t) = base(t) × annual_scale + extra_demand_gw +
   heating(t)`; heating carries its own quantum and is not subject to
   `annual_scale`); nothing else changes. Outputs gain
   `heating.{csv,parquet}` (per-period delivered heat, total and
   per-entry electrical demand) and a `[results.heating]` summary
   block echoing the pinned constants (k, DHW rate, ground damping and
   lag, deratings, cop_const) plus per-year delivered-heat totals —
   the inter-annual spread is a reported finding, never renormalised.
   Residual-load and decomposition machinery see heating inside
   demand — no special-casing.

Migration: a v4 file WITHOUT a heating block migrates by changing only
the version line. A v4 file carrying the old (engine-inert) block also
deletes it — both live reference scenarios were migrated this way in
the v5 commit, and their dispatch digests were re-verified unmoved by
explicit pins (`grid-cli/tests/regression_2024.rs` — `779d7444…` — and
`grid-cli/tests/regression_5zone.rs`).

The v4 reference scenario is frozen under
`grid-core/tests/fixtures/v4-gb-2024-reference.toml` so the migration
error path stays tested.

### Migration note: schema v5 → v6 (2026-07-04, B6 two-zone package)

`schema_version = 6`. v5 files are refused with a structured migration
message (05-validation rule 4). The per-direction / time-series link
capability the B6 link-convention ruling requires
(`docs/notes/b6-two-zone-data-review.md` §6d), plus the exogenous
split multiplier — **all additive** (optional or defaulted), so a v5
file migrates by setting `schema_version = 6` and changing nothing
else. Both live reference scenarios and the RS scenario family were
migrated this way in the v6 commit; the pinned dispatch digests were
re-verified unmoved by the explicit pins
(`grid-cli/tests/regression_2024.rs` — `779d7444…` — and
`grid-cli/tests/regression_5zone.rs`, all six zone digests plus the
links digest).

1. **`[[links]]`** gains optional `reverse_capacity_gw` (GW): the
   capability of the reverse (`to → from`) direction. Absent ⇒ the
   link is symmetric at `capacity_gw` — the pre-v6 semantics,
   byte-for-byte. The B6 ruling needs export 4.1 / import 3.5 GW
   asymmetry.
2. **`[[links]]`** gains optional `[links.capability_trace]`
   (`path`, `column`, `sentinel_high_mw`, `upper_bound_gw`,
   `masked_fill_gw`): a **sparse** per-period forward (`from → to`)
   capability series in MW (the observed B6 day-ahead limit series —
   rows may be missing, values may be NaN; the only trace reference
   not required to cover the horizon), superseding `capacity_gw` on
   horizon periods. The ruling's sentinel handling is stated in the
   scenario, never a loader default: values ≥ `sentinel_high_mw` are
   "no constraint recorded" sentinels replaced by `upper_bound_gw`
   (ETYS 6.7 GW for B6; observed, stays in gates); zero values, NaN
   rows and missing rows are MASKED — excluded from validation-gate
   arithmetic and filled with `masked_fill_gw` (the 2024 median
   4.1 GW for B6) for dispatch, which must run every period.
   `availability` multiplies both directions.
3. **`[[zones.exogenous_supply]]`** gains optional `scale` (default
   1.0): a flat multiplier on the summed MW columns — how a national
   exogenous series is split across zones by a cited share (the
   2-zone scenario's pumped-storage-net 0.2617/0.7383 and FUELHH
   "other" 0.101/0.899 splits). Finite and non-negative; sign
   conventions stay in the trace.
4. **Semantic validation**: `reverse_capacity_gw` finite,
   non-negative; `sentinel_high_mw` finite, positive;
   `upper_bound_gw`/`masked_fill_gw` finite, non-negative; `scale`
   finite, non-negative.
5. **Engine**: the flow rule itself is unchanged (the capability is
   already a parameter of the equalisation); the multi-zone engine
   selects the directional capability per border per period, and
   links carrying v6 detail record a capability series
   (`LinkCapabilitySeries`) feeding the `<name>_fwd_cap_gw` /
   `<name>_binding` output columns and the summary binding
   statistics. Pre-v6 links keep their exact output shape, so pinned
   links digests never move. Outputs of capability-detailed links
   carry the ruling's convention/quote-duty assumption lines.

The v5 reference scenario is frozen under
`grid-core/tests/fixtures/v5-gb-2024-reference.toml` so the migration
error path stays tested.

### Migration note: schema v6 → v7 (2026-07-05, D11 priced multi-zone dispatch)

`schema_version = 7`. v6 files are refused with a structured migration
message (05-validation rule 4). The per-zone pricing inputs and the
flow-signal selector the adopted D11 design requires
(`docs/notes/d11-priced-dispatch.md`, ADR-9 touch-point) — **all
additive** (optional or defaulted), so a v6 file migrates by setting
`schema_version = 7` and changing nothing else. All committed scenario
files were migrated this way in the v7 commit; the pinned dispatch
digests were re-verified unmoved (all committed scenarios keep the
default `scarcity` flow signal, so their behaviour is byte-identical).

1. **`[zones.pricing]`** (optional per zone): the zone's own SRMC
   chain for the priced flow signal — `reference` (a
   prices-reference-v1 file supplying efficiencies, emission factors
   and, when the flat field is absent, the UKA+CPS carbon series),
   optional `carbon_flat_gbp_per_tco2` (a flat per-zone carbon level
   in £/tCO2 **replacing** the reference's UKA+CPS step series — the
   external-zone EUA basis: no licence-clean daily EUA series exists,
   so the committed convention is a flat 2024 annual mean per zone,
   `data/reference/prices-eu-2024.toml`), `[zones.pricing.fuel_price.*]`
   traces and `[zones.pricing.srmc.*]` recipes — the same shapes as
   the top-level `[pricing]` block; the Stage 2 SRMC *recipe* is
   reused unchanged, applied per zone. Technologies without a recipe
   price at the £0 must-take floor in the flow signal (grid-core
   pricing conventions 1–2).
2. **`[dispatch] flow_signal`** (optional; default `"scarcity"`):
   which flow-rule signal a multi-zone run equalises. `"scarcity"` is
   the Stage 5 scarcity score — every pre-v7 scenario behaves
   identically. `"priced_ladder"` is the D11 lexicographic signal
   (per-zone marginal SRMC primary, fractional utilisation of the
   marginal rung secondary; `grid-adequacy/src/flow.rs` normative
   prose) and requires `[zones.pricing]` on **every** zone (ADR-7
   touch-point: external zones need pricing inputs to be dispatchable
   under the ladder).
3. **Semantic validation**: `carbon_flat_gbp_per_tco2` finite,
   non-negative; every `srmc` key names a dispatchable (non-CF-trace)
   entry of that zone's own fleet whose `fuel` has a `fuel_price`
   entry; `flow_signal = "priced_ladder"` without a pricing block on
   every zone is a validation error.
4. **Engine**: the scarcity path is byte-untouched (the default);
   the priced ladder is selected per scenario. The single-zone
   reference digest (`779d7444…`) cannot move under EITHER signal —
   a single-zone scenario has zero borders, so the flow rule is
   unreachable (`multizone.rs` `links_live`).

The v6 reference scenario is frozen under
`grid-core/tests/fixtures/v6-gb-2024-reference.toml` so the migration
error path stays tested.

### Migration note: schema v7 → v8 (2026-07-06, D16 geothermal depth continuum)

`schema_version = 8`. v7 files are refused with a structured migration
message (05-validation rule 4). One optional field
(`docs/notes/d16-geothermal-source-temperature.md`), so a v7 file
migrates by setting `schema_version = 8` and changing nothing else.
All committed scenario files were migrated this way in the v8 commit;
none sets the new field, so every committed result is byte-identical.
(The adopted-but-unbuilt D10 EV overlay, which had reserved v8,
re-targets its bump to v9.)

1. **`[[zones.demand.heating.entries]]` `resource_depth_m`** (optional;
   `gshp` entries only — a structured validation error on `ashp` or
   `district_geothermal`): the geothermal resource depth in metres,
   positive and finite. Absent ⇒ the committed shallow-loop behaviour,
   byte for byte. Present ⇒ the GSHP source mean warms with depth,
   `T_source_mean(z) = T_surface_mean + gradient × (z − loop_depth_m)`
   — the gradient from `data/reference/heating-cop.toml` `[geothermal]`
   (heating-cop-v2, BGS-cited; 25 °C/km centre, 26–35 band), anchored
   at the committed shallow datum so `resource_depth_m = 1.0`
   reproduces the committed behaviour bit-identically (the D16 rule-4
   test-1 invariance). Kusuda–Achenbach damping/lag recompute at z;
   the brine offset is retained; when the offset source meets a
   component's sink the component passes through at the district
   `cop_const`, the heat-pump COP capped AT `cop_const` below it (the
   direct-use handoff — the district-lowest ordering can tie, never
   invert). The depth is echoed into run outputs
   (`resource_depth_m` on the entry's summary block), not an
   `overridden` entry — it is a scenario field, not a reference
   override.
2. **Acceptance tests**: `grid-adequacy/tests/geothermal_depth.rs`
   (invariance at the default, depth monotonicity, the direct-use
   limit meeting the district endpoint, determinism; the
   real-installation calibration anchor is an ignored placeholder —
   OWED pending validation data).

The v7 reference scenario is frozen under
`grid-core/tests/fixtures/v7-gb-2024-reference.toml` so the migration
error path stays tested.

### Scenario-family note: GB internal zones (2026-07-04, three-zone Scottish-group package)

**Proposed ADR-7 amendment (design-review item 7; three-zone direction
ratified by Richard 2026-07-04, `docs/notes/scottish-group-boundary-
design-review.md`). Recorded here as a scenario-family convention; the
architectural ADR-7 text in `docs/02` is left for the supervisor/Richard
to ratify into the ADR (no silent ADR edit).**

GB may be split into INTERNAL zones (N-Scotland / S-Scotland / E+W) for
the boundary study — a clean use of the existing `Vec<Zone>` + link
matrix, no schema change, `flow.rs` untouched (`scenarios/gb-2024-3zone.toml`,
B4 + B6 links). The internal/external distinction is a MODELLING
convention, not a schema concept (the schema treats all zones uniformly),
so this is clean at the schema level. Two rulings travel with it:

1. **GB-internal and continental-external multizone are SEPARATE
   scenario families in v1.** The reason is CONVENTION mixing, not zone
   count (8 zones = 3 internal + 5 external is computationally trivial):
   the 2-/3-zone GB scenarios treat external interconnectors as EXOGENOUS
   observed 2024 net-import traces, whereas the 5-zone continental
   scenario MODELS the import response — mixing them conflates two
   regimes and stacks every external zone's imports-identity wedge on top
   of the doubled internal wedges. (Moyle also lands at Auchencrosh,
   S-of-B4, so a combined run would have to route it to S-Scotland.)
   Unification awaits the Stage-7 LP replacing the single-pass flow rule.

2. **Quote duty under the single-pass rule-based flow.** With two
   internal borders sharing the S-Scotland hub, the flow rule's
   equal-depth artefact compounds and the border-order staleness is
   live, so a three-zone GB run quotes DIRECTION + PINNED TOTALS under
   stated conventions ONLY — no clean per-boundary ("B4 effect proper")
   decomposition. It remains a LOWER BOUND on the Scottish constraint
   phenomenon (B5 folded into S-Scotland). See the run report in
   `grid-adequacy/tests/acceptance_b4_3zone.rs` /
   `acceptance_b4_robustness.rs`.

Illustrative sketch (field names normative, values illustrative):

```toml
schema_version = 7   # v1–v6 superseded — see the migration notes above
name = "FES-2035-Holistic-Transition"
description = "NESO FES 2035 fleet vs. worst weather on record"

[horizon]
start = "1985-01-01T00:00:00Z"   # UTC, half-hourly periods
end   = "2024-12-31T23:30:00Z"
weather_years = "all"             # or [2010], or "worst_on_record"

[[zones]]
id = "GB"
[zones.demand]
base_profile = "data/demand/gb_halfhourly.parquet"
annual_scale = 1.0                # demand growth multiplier
[zones.demand.heating]            # heating technology portfolio (optional; D9 rule 2)
delivered_heat_twh = 410.5        # record-mean annual quantum, cited
electrified_share = 0.5
dhw_fraction = 0.170              # ECUK-derived, delivered-heat basis
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.heating.entries]]
kind = "ashp"                     # ashp | gshp | district_geothermal
share = 0.70                      # shares sum to 1; COP defaults from
                                  # data/reference/heating-cop.toml
[[zones.demand.heating.entries]]
kind = "gshp"
share = 0.20

[[zones.demand.heating.entries]]
kind = "district_geothermal"
share = 0.10

[[zones.fleet]]
technology = "ccgt"
capacity_gw = 30.0

[[zones.fleet]]
technology = "offshore_wind"
capacity_gw = 50.0
capacity_factor_trace = "data/weather/gb_offshore_cf.parquet"

[[zones.storage]]
kind = "battery"
power_gw = 10.0
energy_gwh = 40.0
round_trip_efficiency = 0.88
dispatch_order = 1

[[zones.storage]]
kind = "hydrogen"
power_gw = 20.0
energy_gwh = 60000.0
round_trip_efficiency = 0.38
dispatch_order = 2

[[links]]                          # interconnector matrix
from = "GB"
to = "FR"
capacity_gw = 4.0
availability = 0.95

[[links]]                          # schema v6: per-direction + trace capability
name = "B6"
from = "SCO"
to = "RGB"
capacity_gw = 4.1                  # forward (from → to); superseded per period by the trace
reverse_capacity_gw = 3.5          # reverse (to → from); absent = symmetric
availability = 1.0
[links.capability_trace]           # sparse observed MW series + stated sentinel rule
path = "data/packs/b6/processed/b6_da_flows_limits.parquet"
column = "limit_mw"
sentinel_high_mw = 9999.0
upper_bound_gw = 6.7
masked_fill_gw = 4.1

[dispatch]
policy = "rule_based"              # or "perfect_foresight"

[constraints]                      # ADR-12; nullable
b6_cost_model = "scottish_wind_keyed"

[solver]                           # optional solver mode
mode = "min_storage_for_zero_unserved"   # bisection target
```

## Core types (grid-core)

### Technology

```rust
struct Technology {
    id: TechId,                    // ccgt, ocgt, nuclear, offshore_wind,
                                   // onshore_wind, solar, biomass, hydro, ...
    capacity: Power,
    srmc: Option<SrmcModel>,       // fuel price, efficiency, carbon price →
                                   // £/MWh; None for must-run/zero-marginal
    emissions_factor: EmissionsRate,   // tCO2/MWh
    inertia_h: Option<InertiaConstant>,// GVA·s per GVA; None if non-sync
    synchronous: bool,
    availability: AvailabilityModel,   // flat, profile (nuclear refuelling),
                                       // or outage rate
    annuitised_capex: Option<Price>,   // for LCOE / whole-system cost (Q9)
    cf_trace: Option<TraceRef>,        // weather-driven techs only
}
```

### Storage

```rust
struct Storage {
    kind: StorageKind,             // battery | pumped_hydro | hydrogen | dsr
    power: Power,                  // symmetric charge/discharge in v1
    energy: Energy,
    round_trip_efficiency: PerUnit,
    dispatch_order: u8,
    // DSR only:
    shift_duration: Option<Duration>,
    daily_volume_limit: Option<Energy>,
}
```

### Zone and links

```rust
struct Zone {
    id: ZoneId,
    demand: DemandModel,           // base trace × scale + heating overlay
    fleet: Vec<Technology>,
    storage: Vec<Storage>,         // ordered
}

struct Link { from: ZoneId, to: ZoneId, capacity: Power, availability: PerUnit }
```

### Demand model

`demand(t) = base(t) × scale + heating(t)` where `heating(t)` is the
schema-v5 heating technology portfolio's electrical demand
(docs/notes/d9-heating-overlay.md rules 3–4, implemented in
`grid_core::heating`): degree-hour-proportional space heat with a flat
DHW floor, scaled by the single pinned intensity `k` over the trace's
full record, divided per portfolio entry by its COP —
ASHP on air temperature, GSHP on the damped, phase-lagged annual ground
wave, district geothermal at a constant effective COP. The overlay is
the mechanism for Q5 (electrified heat) and captures the critical
anti-correlation: cold anticyclonic wind lulls raise electrical demand
exactly when wind output collapses — and, per technology, the
covariance of COP with system stress (the network-value-of-geothermal
question, Q11).

## Committed reference files (versioned)

Alongside the scenario schema, the engine reads committed, per-number-
cited reference TOMLs under `data/reference/`. Each carries a mandatory
`schema` string, probed before the full parse; parsing is strict
(unknown fields rejected) with structured errors naming the offending
table and field. A change to a reference file's shape requires a schema
string bump and a note here.

- **`prices-reference-v1`** (`prices-2024.toml`, Stage 2): the 2024
  fuel/carbon *actuals* and fleet efficiencies behind the pinned SRMC
  recipe (HHV-consistent). Parser: `grid_core::prices_reference`.
- **`heating-cop-v2`** (`heating-cop.toml`, Q5/D9, adopted 2026-07-03;
  evidence `docs/notes/q5-heating-data-report.md`, engine-review
  adjudication `docs/notes/q5-heating-engine-review.md`; **v2 bump
  2026-07-06, D16**: the `[geothermal]` section added — UK geothermal
  gradient, 25 °C/km centre (an industry correspondent, conservative) with the BGS
  26–35 band (Busby 2014; Busby & Terrington 2017), the loop-depth
  datum statement and the direct-use handoff convention, all cited and
  checksummed in the file header; **every v1 value untouched** —
  `docs/notes/d16-geothermal-source-temperature.md`): the heating
  overlay's default COP parameterisation — When2Heat curves with the
  retained 0.85 correction, the one-per-technology RHPP to-median
  deratings (0.823 ASHP / 0.732 GSHP), weather-compensated sink
  parameters, the Kusuda–Achenbach ground-model z/α, the district
  effective COP with its delivered-heat basis and band, the RHPP SPFH2
  bands, and the reviewed heat quantum (410.5 TWh record-mean, DHW
  0.170). Parser: `grid_core::heating` (path is the documented engine
  constant `HEATING_COP_REFERENCE_PATH` — no scenario field, per D9
  rule 2/4; per-entry scenario overrides are the scenario-side control
  and are echoed into run outputs). Pinned regression tests:
  `grid-core/tests/heating.rs` (every engine-facing value plus the
  schema string — a silent edit to the committed file fails them).

### Reference schema note: costs-reference-v1 (2026-07-03, Stage 7)

`costs-gb.toml`, adopted 2026-07-03 (evidence
`docs/notes/stage7-cost-inputs-report.md`; reviewer adjudication
`docs/notes/stage7-cost-inputs-review.md`; pinned into docs/04
Stage 7). Parser: `grid_core::costs_reference` (the prices-reference-v1
strictness pattern). Contents: the GDP-deflator table, cited sources,
the uniform three-rate WACC set (4.5/7.5/10.0 % real) with unrounded
anchors and the labelled per-tech sensitivity, per-technology overnight
capex/O&M/lives with build and pre-development phasing arrays,
the nuclear bracket pair, the battery power/energy split, hydrogen legs
(electrolyser, cavern, OCHT), FFPA 2025 gas and traded-carbon
trajectories, FY2025 stability-service holding costs, interconnector
capex rows, and the rule-9 emission factors. Division of labour with
prices-reference-v1 is D8 rule 1.2: 2024 actuals and the SRMC
efficiency chain stay in `prices-2024.toml`; `costs-gb.toml` carries
forward trajectories and all Stage 7 cost-line inputs. Semantics the
parser enforces:

1. **Governance fields are load-bearing.** Machine-readable
   `quotable`/`verified` quarantine flags, the nuclear `bracket_rule`,
   the OCHT `publication_gate`, the battery `staleness_stamp` and the
   cavern `binding_convention` are parsed into the validated structs.
   Consuming a `quotable = false` row is legal but stamps the result's
   metadata non-quotable (consumed rows listed); the artefact/publish
   path refuses a flagged result with a structured
   `GridError::NonQuotableResult`. *(Amended 2026-07-06: the battery
   row's quarantine was LIFTED as the reviewed act condition 3.i
   demanded — the reference revised citing the NREL primary
   re-verification (NREL/TP-6A40-93281, `costs-evidence.sha256`;
   evidence 23676f1; adjudication
   `docs/notes/quarantine-lift-review.md`); the parser guard was
   removed with the revision and the successor test byte-pins the
   lifted flag WITH its citation, so a silent re-quarantine or
   citation drop still fails. The 3.iii staleness stamp travels
   unconditionally, lift or no lift. No schema-string bump: the shape
   is unchanged.)* Quarantined interconnector rows expose **no**
   consumable capex figure at all.
2. **Phasing arrays** (`predev_phasing`/`build_phasing`, review
   condition 11) must each sum to 1.0 ± 0.025 — the tolerance admits
   the as-published rounding (onshore pre-development 0.98, biomass
   0.99), nothing more. They are carried for the rule-4
   escalation-over-phasing (IDC), which is a documented not-yet in the
   annualiser.
3. **Capex brackets** are `[low, central, high]`, ordered; the two
   field spellings (`_gbp_` published-2024-GBP vs `_gbp2024_`
   deflator-converted) are mutually exclusive per row and the
   conversion provenance is carried (`capex_converted_to_gbp2024`).
4. **Trajectories** must have strictly ascending years, aligned
   scenario arrays, and pointwise low ≤ central ≤ high; the gas
   trajectory is converted at parse from p/therm to £/MWh-thermal HHV
   with the file's own UK-statutory-therm factor (0.341214, review
   condition 2).
5. **Holding-cost keys** match `response-holdings-2025.toml`
   `[[services]]` names for a mechanical join (low-frequency products
   consumable; high-frequency recorded only).

Pinned regression tests: `grid-core/tests/costs_reference.rs` (WACC
set, CCGT capex row, gas-central-2030 trajectory point, battery
quarantine flag, CCGT annuity at the three WACCs — a silent edit to
the committed file fails them).

### Reference schema note: pathways-published-v1 (2026-07-06, Stage 7)

`pathways-published.toml`, adopted 2026-07-06 (evidence
`docs/notes/stage7-pathways-data-report.md`; reviewer adjudication
`docs/notes/stage7-pathways-data-review.md` — its conditions 5–8 bind
the parser and the scenario package). Parser:
`grid_core::pathways_published` (the costs-reference-v1 strictness
pattern: schema probe first, unknown fields rejected, structured
errors naming table and field). Contents: cited/checksummed sources,
the two snapshot years (2035, 2050), and per pathway (FES 2025
Electric Engagement, GB; CCC CB7 Balanced Pathway, UK) the demand
basis with its decomposition, the unambiguous fleet mappings, storage
power/energy pairs, the exclusions register, and the CCC's unmappable
aggregates. Semantics the parser enforces:

1. **`mappable = false` is load-bearing (review condition 5).**
   Aggregates parse into their own `ExcludedAggregate` type — a named
   exclusion with magnitude, never a fleet entry; no API merges an
   aggregate into fleet capacity, so a scenario builder cannot
   silently consume one. `mappable = true` on an aggregate is a parse
   error, as is an aggregate name colliding with a fleet technology.
   Consuming an aggregate requires a declared, reviewed split rule in
   the consuming artefact (the Stage 7 pathway scenarios carry theirs
   in prose, pinned by their acceptance tests).
2. **`energy_precision` stamps travel.** CB7 storage GWh figures are
   published only as rounded integers (Table 7.5.1); the stamp is
   parsed onto the storage entry so consuming artefacts propagate the
   rounding caveat.
3. **The surplus-electrolysis exclusion is verified at both sites.**
   The per-year `surplus_electrolysis_excluded_twh` field and the
   pathway-level `exclusions.surplus_electrolysis_demand_twh`
   register entry must agree ("the two sites must stay in step" —
   review condition 1 as committed).
4. **Units at parse.** TWh/GW/GWh become `Energy`/`Power` newtypes;
   exclusion-register year-maps must state their unit through a
   `_gw`/`_twh` key suffix (an unsuffixed magnitude is an error, not
   a guess); year-map keys must cover exactly the snapshot years.
5. `geography` is carried verbatim (`"UK"` on the CCC pathway): the
   UK-as-GB convention is a scenario-package declaration (review
   condition 6), never a parser-side adjustment.

Pinned regression tests: `grid-core/tests/pathways_published.rs`
(FES/CCC demand and capacity pins, the aggregate set by name, the
storage rounding stamps, the surplus-electrolysis pair, plus the
enforcement fixtures — a silent edit to the committed file fails
them).

## Weather data model

- Format: Parquet, one column per trace, UTC half-hourly index, 1985–2024.
- Capacity factors per technology per zone (ERA5 / renewables.ninja derived).
- Temperature trace per zone (heating overlay).
- Data is **fetched and built**, not committed (licensing — see
  `05-validation.md`): a `grid-cli fetch-data` step produces the local pack
  with checksums recorded.

## Outputs (per run)

Per-period: dispatch per technology, system marginal price, storage
state-of-charge per store, unserved energy, curtailment, imports/exports per
link, emissions.

Per-run aggregates: total gas burn, unserved energy hours, total curtailment,
storage min/max SoC, per-technology revenue and capture price, total system
cost (£/MWh delivered), total emissions, constraint cost.

Stability outputs (per event): aggregate inertia H, RoCoF, frequency nadir,
LFDD threshold breaches, response service deployment timeline.

Analysis utilities in grid-core:

- **Residual load** series and duration curves (Module 2).
- **Timescale decomposition**: filter residual load into diurnal / synoptic /
  seasonal / inter-annual bands and attribute storage TWh to each — the
  single most persuasive artefact (Module 3).
- ELCC solver support (Q1), per-increment curtailment attribution (Q2).
