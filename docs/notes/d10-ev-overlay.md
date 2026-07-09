# D10 — EV/transport demand overlay design: road-EV segments with a charging-profile bracket

> **SCHEMA BUMP RE-TARGETED 2026-07-06 (D16 coordination):** this note
> reserves **schema v8** throughout its body. The D16 geothermal
> depth-continuum engine package landed first and took v8 (docs/03
> v7 → v8 migration note, 2026-07-06; the D16 note's own rule:
> "coordinate the bump; do not collide"). **The D10 engine package
> therefore takes v9** — read every `v8` below as `v9`. The body text
> is deliberately unedited (it is the adjudicated record); this banner
> supersedes it on the version number alone. Recorded in the docs/08
> D10 row and the D16 engine run report
> (docs/notes/d16-geothermal-engine-run-report.md).

**Status:** ADOPTED 2026-07-06 — reviewer ADOPT-WITH-EDITS
(docs/notes/d10-ev-overlay-review.md), all seven ordered edits applied
below. Supervisor draft,
2026-07-06, the named next design work under D15 (docs/08: D10 was
promoted into v1 because it is needed to run FES Electric Engagement
demand honestly at 2035+, not only for Q12). Follows the D9/D11/D13
gate pattern: docs only, no schema or engine code until adjudicated.
Architectural template: the ADOPTED heating overlay
(docs/notes/d9-heating-overlay.md + its review) — pinned-intensity,
per-year weather-driven, per-zone schema block, red-first acceptance
tests. Designed within the ADR (ADR-8 governs the V2G-lite limb: DSR
is pseudo-storage); no amendment proposed.

## Rule 1 — What the overlay is, the questions it serves, and the
## claim ceiling

A demand-side transformation, the D9 sibling: it ADDS road-EV charging
demand to the electricity demand trace, half-hourly, as a
deterministic function of (a) an annual charging-energy quantum,
(b) a portfolio of EV segments each with a charging profile, and
(c) the population-weighted GB air temperature trace (the SAME pinned
`gb_t2m_pop.parquet` the heating overlay reads — no new weather data).
It changes nothing else: no dispatch rules, no pricing conventions, no
storage mechanics (the V2G-lite limb EMITS a storage entry into the
existing ADR-8 portfolio; it does not touch storage dispatch — rule 5).
All existing scenarios without an EV block are BIT-IDENTICAL (the
Stage 3/5/6 precedent: old pins never move; unlike v5 this bump IS
purely additive — no EV sketch block exists in v1–v7, verified against
`grid-core/src/scenario.rs` `DemandSpec`).

Questions served, in order:
1. **FES/pathway demand fidelity NOW.** The committed Stage 7 pathway
   scenarios (`scenarios/fes2025-ee-2035.toml`, `fes2025-ee-2050.toml`)
   tile the 2024 demand SHAPE scaled to the pathway total — a named
   adequacy-FAVOURABLE gap on every unserved quote ("NO
   electrification reprofiling … understates the winter-evening
   peakiness of heat-pump/EV load", scenario header convention 1).
   D10 + D9 close that gap: the pathway's EV and heat-pump components
   get their own physics-shaped profiles instead of riding the 2024
   shape.
2. **Q12 LATER** (the Rosenow electrification stress test): the
   overlay's parameters — quantum, segment shares, smart participation,
   the derating curve — are exactly Q12's stress axes (iii)
   demand-assumption stress (EV cold-weather efficiency) and (iv)
   flexibility-delivery stress (smart participation below assumption).
   Q12's full analysis is out of scope here (rule 9).

Claim ceiling: this is a **half-hourly national-aggregate charging
model with declared conventions, not a transport-sector model and not
a reproduction of NESO's own EV modelling**. No individual-vehicle
state, no charger network, no trip chains. The dumb/smart profile pair
is a BRACKET whose direction is stated, never a forecast of realised
charging behaviour; every result quotes which profile it ran and the
bracket caveat. Coverage is ROAD EVs only — the demand class matching
the FES ED1 EV components (rule 2); rail/aviation/shipping
electrification stays wherever the base tiling puts it, stated.

## Rule 2 — The double-count hazard (read first) and the carve-out
## composition rule

**FES Electric Engagement carries electrified transport INSIDE its
demand totals.** The ED1 System Demand Total the committed scenarios
scale to (450.076 TWh at 2035, 784.736 TWh at 2050) already contains
the EV components, published as level-3/4 rows and carried in the
pinned reference (`data/reference/pathways-published.toml`,
pathways-published-v1, evidence
docs/notes/stage7-pathways-data-report.md):

| Component (ED1) | 2035 | 2050 |
|---|---|---|
| Residential EVs | 41.488 TWh | 54.370 TWh |
| Commercial EVs | 31.404 TWh | 77.378 TWh |
| **Road-EV total** | **72.892 TWh** | **131.748 TWh** |

Applying the overlay ON TOP of a scenario scaled to the System Demand
Total would double-count that energy. The heating overlay faces the
same hazard (the FES total also contains the heat-pump rows) and the
Stage 7 data report already names profile construction as "the
scenario package's work, not transcription". The composition
MECHANISM is pinned here for both overlays; the D9-side component
convention is a named open item (see the electrical-terms bullet
below):

- **Carve out of the target, not the shape.** The pathway scenario's
  `annual_scale` is retargeted so the scaled 2024 base carries
  `(System Demand Total − Σ overlay component quanta)`; each overlay
  then adds its component with its own physics shape. The scaled base
  shape itself is untouched — uniform rescale, no surgery on the 2024
  trace.
- **The carved quanta are electrical energies (reviewer edit 2).**
  For D10 the quantum IS electrical (`annual_energy_twh` = the ED1
  road-EV component, exact). For D9 the scenario input is heat-side
  (`delivered_heat_twh`); its electrical energy is derived
  (`heat/COP`, portfolio-dependent), and the pinned ED1 extraction
  carries only RESIDENTIAL heat markers (18.223/58.505 heat-pump,
  3.478/14.643 resistive TWh) — no commercial-heat row. The D9 FES
  carve-out therefore needs its own convention (which ED1 rows
  constitute the carved component; the heat-to-electrical inversion
  computed ONCE from pinned inputs on the record mean, never per
  year), adjudicated at the scenario package with the reviewer
  gate — NAMED OPEN, not silently pinned here. What IS pinned here
  for both overlays is the composition MECHANISM: subtract the
  overlays' electrical quanta from the retarget, add each back with
  its physics shape, one subtraction of the sum
  (order-independent).
- **Acceptance test (red-first, the FES-consistency test):** scaled
  base energy + overlay annual quanta == the published System Demand
  Total, at **1e-9 relative** (fork-6 ruling: the committed
  pathway-pin precedent, stage7-pathways-scenarios-review §B — the
  retarget is a single f64 division of exact TOML decimals, so 1e-9
  has orders of margin while catching any wrong-row or wrong-basis
  error), asserted per pathway scenario in the pinned acceptance
  suite (the `acceptance_stage7_pathways.rs` precedent: every number
  asserted equal to the parsed reference). The overlay quantum in an
  FES-reprofiled scenario MUST equal the ED1 road-EV component —
  EXACT equality of the parsed reference value, no tolerance — cited
  to the reference file, not free scenario text.
  The identity is asserted on QUANTA. A single-weather-year
  scenario's REALISED overlay energy is `quantum × mean(m(T̄_d))`
  over that year, not the quantum: under 2024 weather (mild — 2024
  degree-hours are ~13% below the record mean) the realised annual
  total lands BELOW the published System Demand Total by the weather
  modulation of both overlays. Direction stated on every
  FES-reprofiled quote and on any scaled-peak-vs-published-peak
  comparison; the record-mean identity is the design claim, the
  per-year deviation is physics.
- **The second-order layer, owned:** the 2024 base SHAPE contains
  2024's actual EV charging (order 5 TWh — magnitude to be pinned
  from ECUK/DUKES in the data package, cited). Under carve-out-of-
  the-target that embedded load is uniformly rescaled with all other
  2024 demand and treated as generic demand shape — a stated framing
  wrinkle (≈1–2 % of the trace), the same "2024 non-heat demand tiling
  under the overlay" caveat class D9 rule 6 carries. Stated, not
  fixed, direction second-order.
- One further basis note travels with every FES-consistency quote: the
  ED1 totals are fiscal-year labelled (the stated stage7-data wrinkle)
  and NESO's components are its own modelled quantities — the
  tolerance test asserts arithmetic consistency with the published
  total, not agreement with NESO's unpublished half-hourly profile.

Standalone (non-pathway) scenarios simply state their quantum and cite
it; the consistency test binds only scenarios that claim a published
pathway's demand.

## Rule 3 — Fleet/energy model: quantum, segments, and the
## bottom-up cross-check

The scenario-side control is the **annual charging-energy quantum**
(`annual_energy_twh`), split across segments — NOT fleet × km ×
kWh/km computed live in the engine. Rationale: the FES-consistency
constraint (rule 2) anchors the total to a published component; a
bottom-up product would not reproduce it (NESO assumes its own
efficiency improvement and demand trajectories), and a hidden
renormalisation to force agreement would be exactly the tuning rule 7
forbids. The bottom-up identity instead serves two stated jobs:

1. **Plausibility cross-check (data package, cited):** quantum ≈
   fleet × annual km per vehicle × kWh/km (at-the-meter). GB anchors,
   all OGL:
   - Vehicle stock: DfT vehicle licensing statistics, table VEH0101
     (41.7 M licensed vehicles at end-June 2024; cars ≈ 34 M —
     gov.uk "Vehicle licensing statistics: 2024", accessed
     2026-07-06). **Scope pin (reviewer edit 4):** VEH0101 carries
     both GB and UK bases; the release headline's basis is pinned by
     the data package FROM THE RELEASE ITSELF, and if a GB figure is
     needed from a UK row the conversion is a cited factor (the D9
     ONS-0.972 precedent), stated next to the number — never a
     silent relabel. The data package pins the exact car/van rows
     used.
   - Distance: DfT road traffic estimates, TRA0101 — car traffic
     256.1 bn vehicle-miles (≈ 412 bn vehicle-km) in 2024 (gov.uk
     "Road traffic estimates in Great Britain, 2024: headline
     figures", accessed 2026-07-06); cross-checked against NTS0901
     annual mileage per car (England; DfT National Travel Survey
     "Vehicle mileage and occupancy" tables, accessed 2026-07-06) —
     the two must agree to a stated band once fleet composition is
     accounted for, and disagreement is reported, not averaged away.
   - Intensity: kWh/km **at the meter** (charger + charging losses
     inside the figure — the overlay is grid-side demand, so the
     at-battery basis is WRONG by ~10–15 % and the basis is stated
     next to the number, the D9 delivered-heat-basis discipline).
     Source to be pinned by the data package from a named measured
     real-world source (candidates: the FES workbook's own
     stock/efficiency components if the pinned pack carries them —
     to be checked first, since FES-internal consistency is the
     cleanest; else a named published real-world-consumption study,
     licence-checked — the fork-7 ruling adopts exactly this order,
     with the at-the-meter basis MANDATORY and the conversion cited
     if the source reports at-battery). Order of magnitude on the
     anchors:
     412 bn km × ~0.17 kWh/km ≈ 70 TWh for a fully electrified 2024
     car fleet — consistent with the FES EE component table above,
     which is the point of the check.
2. **Q12's standalone axis:** when Q12 builds the Rosenow system
   (not a FES reproduction), the quantum is CONSTRUCTED from cited
   fleet/km/kWh-per-km and swept — same schema, no engine change.

**Segments** (the D9 portfolio template): the block carries
`[[zones.demand.ev.segments]]` entries, each `kind` / `share` /
`profile`. v1 kinds: `residential` (home-based cars — the ED1
"Residential EVs" component: overnight-dominant plug-in) and
`commercial` (vans/fleets/depot — the ED1 "Commercial EVs" component:
daytime/depot plug-in). Shares are of the quantum,
`|Σ share − 1| ≤ 1e-9` (the D9 share-sum validation, structured error
naming sum and entries); unknown kinds rejected
(`deny_unknown_fields`). The FES-reprofiled scenarios set the shares
from the two ED1 rows (2035: 0.5692/0.4308 of 72.892 TWh — the data
file's own numbers, no invention). HGV electrification has no
separate ED1 row in the pinned extraction; it rides inside
`commercial` in v1 — a stated fold, and a named follow-on kind if a
published split arrives (one `schema_version` bump, nothing else —
the D9 rule-2 extension-path promise, same wording, same law).

Driving demand within the year is **flat per day in v1** (quantum ÷
days, before rule 4's temperature modulation): no
weekday/weekend/holiday driving cycle. Direction stated: observed
car traffic is mildly seasonal and weekly-cyclic; flattening it
smooths the charging base but is second-order against the profile
choice (rule 5) and the derating (rule 4), both of which act at the
binding winter-evening hours. Accepted for v1 by the adjudication
(fork-3 ruling); a day-type variant is the named follow-on if review
or Q12 demands it.

## Rule 4 — Temperature derating: EV demand peaks when heating peaks

Cold weather raises EV consumption per km (battery chemistry + cabin
heating). The overlay applies a **pinned multiplier on the daily
charging energy**, `m(T̄_d)`, a function of the day's mean
population-weighted temperature from the SAME pinned `T_pop` trace as
D9:

- Curve: pinned from a **named measurement source** — the Geotab
  telematics analysis (4,200 EVs, 5.2 M trips; "New analysis by
  Geotab investigates the impact of temperature and speed on electric
  vehicle range" / "How temperature and speed impact EV range",
  geotab.com, accessed 2026-07-06): range ≈ rated at 10–31 °C,
  peaking ~115 % of rated near 21.5 °C, ~20 % loss around 0 °C,
  ~46–50 % loss by −15…−25 °C. The data package transcribes the curve
  (or a cited piecewise fit to it), records its licence status, and
  names a second corroborating source (candidates: the NAF/Norwegian
  winter test series, US DOE/AAA); if licence terms block
  transcription, the fallback is a cited academic real-world
  consumption-vs-temperature study — the trigger and fallback are
  defined now, the D9 ruling-A pattern. Consumption multiplier =
  1 / (range fraction), stated.
- **Normalisation is the D9 rule-3 law, not per-year:** the curve is
  rescaled by a single pinned constant `c` so that the record-mean of
  `m(T̄_d)` over the pinned reference window (1985–2024) equals 1,
  computed once from the pinned `T_pop` trace and recorded in run
  outputs. `annual_energy_twh` is therefore the **record-mean annual
  quantum**: cold years draw more charging energy than mild years —
  never per-year renormalisation (the D9 ruling-B lesson applies
  verbatim: per-year exactness would zero the inter-annual band and
  sever the covariance the 40-year questions exist to measure).
  `m(T̄_d)` is a pure function of the pinned trace: horizon subsetting
  never changes it (ADR-5 composability).
- **The correlation hazard, stated prominently:** the multiplier is
  LARGEST exactly when the D9 heating overlay is largest — cold
  anticyclonic wind lulls raise heat-pump demand, degrade heat-pump
  COP, and raise EV kWh/km in the SAME hours that wind output
  collapses. Omitting the derating would therefore be
  adequacy-FAVOURABLE precisely at the binding hours; including it is
  not optional. Every combined D9+D10 result states that the two
  overlays share the `T_pop` driver by construction — the
  covariance is a modelled physical effect, not an artefact.
- Limitation, owned: daily-mean modulation cannot move energy WITHIN
  the day (a cold morning commute's extra draw lands via the day's
  multiplier, not that half-hour); the within-day shape belongs to
  the charging profile (rule 5). Direction: smooths the derating's
  intraday incidence — mildly favourable at the evening peak, stated
  with the rule-5 profile caveats.

## Rule 5 — Charging profiles: dumb, smart, V2G-lite

Each segment's daily energy `E_d = share × quantum/day × m(T̄_d)` is
allocated to half-hours by the segment's `profile`. All three
profiles deliver IDENTICAL energy (the invariance spine, D9 rule 5):
what varies is shape only.

- **`dumb` (plug-on-arrival):** the day's energy is allocated by
  convolving a **fixed, cited plug-in arrival distribution** with a
  constant-power charge at the segment's nominal charger power until
  the day's energy is delivered — a deterministic fleet-aggregate
  convolution, no sampling (rule 7). Arrival distribution source:
  DfT "Electric Chargepoint Analysis 2017: Domestics" (OGL,
  gov.uk/data.gov.uk, accessed 2026-07-06 — the observed domestic
  plug-in time distribution with its evening-arrival cluster) for
  `residential`; for `commercial`, the data package must SEARCH
  FIRST and RECORD the search (fork-2 ruling): a cited public
  depot/commercial charging distribution if one exists
  licence-clean; the flat-working-hours convention is the recorded
  fallback, labelled as convention with its bias direction stated —
  an ordered search-and-record deliverable, not an optional look.
  Bias direction, stated on every artefact:
  dumb charging lands on the 17:00–20:00 winter evening — the
  adequacy-ADVERSE end of the bracket.
- **`smart` (surplus-following):** the same daily energy, allocated
  within a stated charging window to the half-hours of lowest
  **pre-dispatch residual load** — the signal question answered
  against the engine's own conventions: the signal is
  `R(t) = zone demand before EV (scaled base + extras +
  extra_demand_gw + heating overlay) − Σ must-take weather-driven
  generation (capacity × CF trace)`, per zone. It is NOT price
  (pricing is a post-dispatch layer, D8/D11 — a price-following
  profile would be circular: price depends on dispatch depends on
  demand depends on charging), and NOT storage-aware (storage state
  is a dispatch output). `R(t)` is a pure pre-dispatch function of
  the scenario, so determinism and ADR-5 composability hold; it
  ignores interconnector flows — stated. Allocation: the **level
  (water-fill) allocation** — the day's energy is placed within the
  window so as to MINIMISE the maximum of `R(t) + ev(t)`, subject to
  the segment's aggregate power cap per half-hour (fill the
  lowest-`R` half-hours up to a common level, capped; the level is
  the unique solution of the energy-balance equation —
  deterministic, no tie-breaking needed). Containment is a VALIDATED
  precondition, not an assumption: the smart window must contain the
  dumb allocation's support and the cap must be ≥ the dumb
  allocation's maximum aggregate power, checked at load time with a
  structured error naming the violation. Because the allocator is
  the feasible-set max-minimiser and the feasible set contains the
  dumb allocation, smart's maximum post-EV residual ≤ dumb's is a
  genuine theorem (rule 8.3). A greedy fill AT the cap is explicitly
  rejected (adjudicator's counterexample, edit 1): on a flat
  residual it produces `R + cap`, while the dumb convolution's
  staggered arrivals keep its aggregate power strictly below the
  cap — smart would exceed the dumb peak and the bracket would
  invert by construction artefact, not physics. Per-segment window
  convention: the window lives in `data/reference/ev-fleet.toml`
  (cited, or declared-as-convention and labelled so), with the
  containment check above mandatory (fork-1 ruling).
  Bias direction, stated: smart
  assumes PERFECT fleet-wide coordination against a perfect system
  signal — real tariffs track system conditions imperfectly and
  participation is partial — so smart is the adequacy-FAVOURABLE
  end of the bracket. Partial participation is expressed by
  splitting a segment into dumb and smart sub-segments with stated
  shares (no new mechanism), which is exactly Q12's stress axis
  (iv).
- **`v2g_lite` (DSR pseudo-storage per ADR-8):** smart charging PLUS
  a derived storage entry of the existing schema-v2 `kind = "dsr"`
  shape (`grid-core/src/scenario.rs`: DSR-only fields
  `shift_duration`, `daily_volume_limit` — schema shape committed;
  ADR-8: "Demand-side response is modelled as a pseudo-storage entry
  with shift-duration and volume limits"). The overlay DERIVES the
  storage-equivalent parameters from cited EV quantities and emits
  them into the zone's storage portfolio, all echoed to run outputs:
  - `power_gw` = participating fleet × per-charger export power ×
    minimum plugged-in fraction over the day — bounded above by the
    segment's charging power cap convention; participation ∈ [0, 1]
    validated.
  - `energy_gwh` (and `daily_volume_limit`) = participating fleet ×
    usable battery swing per vehicle (a stated fraction of a cited
    pack size — drivers do not offer full packs; bound stated,
    cited where a trial source exists).
  - `round_trip_efficiency` = charger in × charger out (order
    0.8–0.9; cited in the data package, basis stated).
  - `shift_duration` = the overnight window length convention.
  "Lite" means exactly this: an aggregate, bounded, storage-shaped
  stand-in — no per-vehicle state of charge, no mobility constraint
  beyond the plugged-in bound. **Named dependency (the D9
  ELCC-runner pattern):** the `dsr` StorageKind's ENGINE dispatch
  semantics are provisional until Q6 (documented in scenario.rs) —
  D10 defines the parameter derivation and validation now; runs
  quoting the v2g_lite limb wait for Q6's activation semantics, and
  that dependency binds the analysis package, not the overlay
  engine work (fork-4 ruling: parse/derivation/validation/echo land
  now; the gating is accepted as drafted).

**NOT modelled, stated:** distribution-network constraints (an EV
street cluster can bind an LV feeder long before GB adequacy — out
of scope by ADR-12's no-network-model law, named on every smart/V2G
quote as favourable to flexibility); charger availability/queuing
(every vehicle finds a charger); public rapid/en-route charging as a
separate shape (folded into the segment profiles); holiday-getaway
demand spikes; battery preconditioning as a separate load (inside
the rule-4 aggregate curve, which is measured on-road consumption);
battery degradation feedback from V2G cycling.

## Rule 6 — Schema: the `[zones.demand.ev]` block (v8 bump)

```toml
[zones.demand.ev]                 # per-zone, optional; absent ⇒ byte-path untouched
annual_energy_twh = 72.892        # record-mean annual quantum (rule 4), cited;
                                  # in FES-reprofiled scenarios == the ED1
                                  # road-EV component (rule 2), asserted
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.ev.segments]]
kind = "residential"              # residential | commercial
share = 0.5692                    # of the quantum; shares sum to 1
profile = "dumb"                  # dumb | smart | v2g_lite
# optional per-segment parameter overrides (charger power, window,
# v2g participation/swing); defaults live in data/reference/ev-fleet.toml

[[zones.demand.ev.segments]]
kind = "commercial"
share = 0.4308
profile = "smart"
```

- `schema_version` bumps v7 → v8 with the docs/03 migration note.
  The bump is **purely additive** (no EV sketch exists in v1–v7 —
  unlike D9's v5 there is nothing to remove): a v7 file migrates by
  changing only the version line. All committed scenarios are
  migrated that way in the v8 commit and their pinned dispatch
  digests re-verified unmoved — an explicit acceptance check, not an
  assumption (rule 8.4). The v7 reference scenario is frozen under
  `grid-core/tests/fixtures/` (v1…v6 precedent) so the migration
  error path stays tested.
- Defaults (charger powers, arrival distribution, derating curve,
  V2G swing bounds) live in a cited, drift-guarded reference file
  `data/reference/ev-fleet.toml` (`ev-fleet-v1` schema string; the
  `heating-cop.toml` precedent exactly: engine-constant path, no
  scenario field, pinned regression tests on every engine-facing
  value, per-segment scenario overrides legal and always echoed into
  run outputs).
- Demand identity extends to
  `demand(t) = (base(t) + Σ extras(t)) × annual_scale +
  extra_demand_gw + heating(t) + ev(t)`; like `heating(t)`, `ev(t)`
  carries its own quantum and is NOT subject to `annual_scale`
  (docs/03 demand-model section gains the term).
- **Per-zone convention for multi-zone scenarios:** the block is
  per-zone (the D9/edit-3 ruling — a top-level block is ambiguous
  about whose demand it transforms). The GB quantum is split across
  zones by the scenario author with cited shares; the DEFAULT
  convention, stated in the field docs, is the zone's demand share
  (the existing `annual_scale`-share convention of the committed
  2/3/5/8-zone scenarios), with the bias owned: vehicle stock is not
  proportional to electrical demand (rural/urban ownership skew).
  **Hardened per reviewer edit 6:** the demand-share default is
  quotable only at GB aggregate; any ZONE-RESOLVED EV claim MUST use
  the cited DfT per-region licensing split (VEH0105-class table,
  named data-package option), and the convention in force is stated
  next to any zonal claim.
  Each zone's block reads the SAME GB `T_pop` trace in v1 (no zonal
  temperature split exists for GB; stated, matching D9).

## Rule 7 — Determinism and no-tuning

- Pure function: `ev(t)` is fully determined by (scenario, pinned
  `T_pop` trace, ev-fleet reference file). No sampling anywhere — the
  dumb profile is a fleet-aggregate convolution of a FIXED cited
  arrival table, not a Monte Carlo draw (ADR-5: any future stochastic
  variant takes an explicit seed; none is proposed).
- The profiles are **conventions with declared bias directions,
  never fitted to outcomes**: the arrival distribution and derating
  curve are transcribed from their citations and then FROZEN; the
  two pinned normalisation constants (rule 4's `c`; the dumb
  profile's charge-duration arithmetic) are computed once from
  pinned inputs and echoed into outputs. Nothing is adjusted to make
  adequacy results, the dumb/smart gap, or the FES peak comparison
  look right — if the scaled-shape peak lands far from the published
  FES peak after reprofiling, that is a REPORTED FINDING about the
  conventions, not a residual to be tuned away (the Package A/B
  lesson).
- Output discipline (ADR-5, the D9 rule-6b list): per-period EV
  electrical demand, total and per-segment; the daily multiplier
  series' pinned constant `c`; the derived V2G storage parameters;
  every reference-file value in force and any per-segment overrides —
  all echoed into run outputs.

## Rule 8 — Pre-registered acceptance criteria (red-first)

1. **Conservation:** reference-window (1985–2024) mean annual EV
   energy = quantum to a stated float tolerance, for every
   segment/profile mix; per-year totals vary with weather and the
   inter-annual spread is a reported output, never normalised away.
   All profiles deliver identical energy on identical inputs (the
   invariance spine). Share-sum and bounds validation per rules 3/5.
2. **FES-consistency (rule 2):** for each FES-reprofiled scenario,
   retargeted base energy + D10 quantum (+ the D9 electrical
   carve-out quantum under its adjudicated convention, where applied)
   == the published System Demand Total at 1e-9 relative (fork-6
   ruling), asserted against the parsed `pathways-published.toml` —
   and the D10 quantum == the ED1 road-EV component as EXACT equality
   of the parsed reference value (no tolerance). The rule-2
   realised-vs-quantum statement travels with every quote of this
   test's scenarios.
3. **The dumb-vs-smart bracket direction:** on the same scenario,
   same quantum, the smart profile's maximum post-EV residual load
   ≤ the dumb profile's — asserted red-first as a THEOREM, valid by
   the rule-5 min-max construction (the level/water-fill allocator is
   the feasible-set max-minimiser and the machine-checked containment
   preconditions put the dumb allocation inside that feasible set;
   reviewer edit 1). The ADEQUACY
   ordering (unserved energy smart ≤ dumb) is the expected
   direction but is parameter- and storage-interaction-contingent —
   a measured finding, pinned from measurement, inversion reported
   at full prominence (the D9 ruling-C split, applied verbatim).
4. **Digest discipline:** scenarios without the EV block —
   including every committed scenario after its v8 version-line
   migration — keep their pinned dispatch digests bit-identical
   (`779d7444…` for the reference; the multi-zone and pathway pins
   likewise). Explicit re-verification in the v8 commit.
5. **A pinned characterisation run:** FES EE 2035, all-dumb vs
   all-smart, reporting peak residual demand, unserved energy, and
   the delta vs the committed unreprofiled scenario — every published
   number gets its pin before it is quoted (project law).

## Rule 9 — Non-goals

No Q12 full analysis (the stress-test campaign is its own work order
consuming this overlay); no literal EU-27 transport modelling (the
D15 demotion stands); no charging-infrastructure economics (charger
capex/queuing/utilisation — Stage 7 owns £ and even there only
generation-side stacks are in scope today); no distribution-network
modelling (ADR-12); no hydrogen or e-fuel transport chains; no
behavioural adoption modelling (fleet size is an input, never a
diffusion forecast); no day-type driving cycle and no HGV segment in
v1 (named follow-ons, rule 3); no change to any existing trace, pin,
or convention.

## Rule 10 — Implementation shape

- **Package split (the D9 chain, three packages after adjudication):**
  1. *Data package* (data-engineer): `data/reference/ev-fleet.toml`
     (`ev-fleet-v1`) — arrival distribution transcription (DfT
     chargepoint analysis, OGL), derating curve transcription with
     licence verdict + corroborating source, charger power and V2G
     swing bounds, the VEH0101/TRA0101/NTS0901 cross-check numbers
     with exact table rows, the ECUK/DUKES 2024-embedded-EV
     magnitude, kWh/km at-the-meter pin, all access-dated; evidence
     note + review.
  2. *Engine package* (rust-implementer): schema v8 + docs/03
     migration note; `grid_core::ev` module on the
     `grid_core::heating` loader pattern (reference-file parser with
     schema-string probe + strict fields + pinned regression tests;
     `TraceRefSpec` temperature trace; overlay computed in demand
     assembly beside `heating(t)`); the pre-dispatch residual signal
     (a pure function over already-loaded traces — no dispatch-side
     change); the V2G parameter derivation emitting the ADR-8 `dsr`
     entry; rule 8 tests 1/3/4 red-first.
  3. *Scenario/analysis package*: the FES-reprofiled scenario
     variants (retargeted `annual_scale` + D10 block, D9 block where
     the heating carve-out is taken in the same commit — ONE
     composition rule, rule 2), rule 8 tests 2/5, run report.
- **Reused committed machinery, named:** the D9 loader/validation/
  override/echo pattern end-to-end; the pinned `gb_t2m_pop.parquet`
  trace and its manifest (byte-untouched); `pathways-published.toml`
  and its parser for the consistency test; the stage7 acceptance-test
  pattern; the schema-bump/fixture-freeze migration machinery
  (v1→…→v7 precedent). Genuinely new code: the profile allocator
  (convolution + greedy fill) and the V2G derivation — small,
  well-bounded.
- **Estimated cycles:** comparable to but smaller than D9 — no COP
  physics, one temperature curve instead of three source models.
  Data package ~1 session; engine package ~1–2 sessions
  (allocator + tests dominate); scenario package ~1 session
  (the retile arithmetic is mechanical; its review is where the
  composition rule gets checked). Each package gates on the
  reviewer per the supervisor-mode law. The v2g_lite RUN deliverable
  additionally waits on Q6 (rule 5) — parameters and parsing do not.

## Rule 11 — Proposed docs/08 D10 row update (on adoption)

> | D10 | EV / transport demand overlay (Q12) | Resolved — design
> adopted (docs/notes/d10-ev-overlay.md + review) | Road-EV demand
> overlay: per-zone `[zones.demand.ev]` segment portfolio (schema
> v8, additive), record-mean quantum with pinned-normalisation
> temperature derating on the shared `T_pop` trace (correlation
> with D9 heating stated as physics), dumb/smart charging bracket
> (smart = min-max water-fill on the pre-dispatch residual-load
> signal with machine-checked containment preconditions; bracket
> direction a theorem via that construction), V2G-lite emitted as
> ADR-8 `dsr` pseudo-storage (engine activation semantics: Q6
> dependency, named). FES-consistency carve-out rule pins the
> anti-double-count composition MECHANISM for both D9/D10
> reprofiling of pathway scenarios; the D9 component convention is
> named open until the scenario package.
> Chain: data package (ev-fleet-v1) → engine (v8) → FES-reprofiled
> scenarios + pinned runs. |

## Data requirements (data package, licence-checked first)

1. DfT vehicle licensing statistics VEH0101 (+ per-region VEH0105-
   class table as the zonal-split option) — OGL; accessed 2026-07-06
   at gov.uk (Vehicle licensing statistics: 2024).
2. DfT road traffic estimates TRA0101, car/van vehicle-km — OGL;
   accessed 2026-07-06 (Road traffic estimates in Great Britain,
   2024: car traffic 256.1 bn vehicle-miles).
3. DfT National Travel Survey NTS0901 annual car mileage (England;
   scope caveat vs GB stated) — OGL; accessed 2026-07-06.
4. DfT "Electric Chargepoint Analysis 2017: Domestics" — OGL;
   accessed 2026-07-06 — the residential plug-in arrival
   distribution; its age (2017, early-adopter fleet) recorded as a
   stated limitation with direction discussed in the evidence note.
5. Temperature-derating curve: Geotab temperature/range analysis
   (4,200 EVs, 5.2 M trips; accessed 2026-07-06) — licence status to
   verify before transcription; corroborating source and the
   defined fallback per rule 4.
6. kWh/km at-the-meter pin: FES workbook components first (pinned
   pack, licence-clean), else a named measured real-world study —
   basis (at-meter vs at-battery) stated next to the number.
7. ECUK/DESNZ (or DUKES) road-transport electricity consumption,
   latest year — the rule-2 embedded-2024 magnitude.
8. FES ED1 road-EV components: already pinned and reviewed in
   `data/reference/pathways-published.toml` — no new fetch.

## What this note does NOT do

No engine or schema code (design-before-code, the D9 adjudication
posture); no docs/04 or docs/08 edit (proposed text only, rule 11);
no Q6 DSR semantics (named dependency); no new weather data; no ADR
amendment — ADR-8's pseudo-storage sentence is implemented, not
extended.
