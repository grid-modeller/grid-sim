# D9 — Heating overlay design (Q5): a technology portfolio

**Status:** ADOPTED 2026-07-03 — supervisor draft, adjudicated by the
reviewer ADOPT-WITH-EDITS (docs/notes/d9-heating-overlay-review.md),
all twelve ordered edits applied verbatim below. Priority set by
Richard 2026-07-03: "Make sure that you do the Q5 heat[ing] overlay …
I want to be able to do that analysis" — the overlay is promoted into
the overnight chain ahead of the wave-2 paper enablers. Extended
mid-adjudication with the geothermal-relief deliverable (rule 6b;
Richard's Q11/an industry correspondent directive).

**Design requirement (Richard, 2026-07-03, pinned in project-state):**
the overlay must be a **technology portfolio** from day one — per-entry
kind/share/COP-model with the COP's source temperature distinguished.
Purpose: quantify the **network value of geothermal heating** —
identical heat decarbonisation, vary the ASHP/GSHP/district mix,
measure the delta in peak residual demand and the 40-year storage
requirement. The single-`cop_curve` sketch in docs/03 is NOT to be
implemented. Cost side is Stage 7; physics first.

## Rule 1 — What the overlay is

A demand-side transformation, not a supply technology: it ADDS
electrified-heating demand to the electricity demand trace,
half-hourly, as a deterministic function of (a) a delivered-heat
quantum, (b) a portfolio of heating technologies, and (c) the
population-weighted GB air temperature trace. It changes nothing else:
no dispatch rules, no pricing conventions, no storage mechanics. All
existing scenarios without a heating block are BIT-IDENTICAL
(the Stage 3/5/6 precedent: old pins never move — though v5 is not
purely additive; see rule 2 on the removed sketch fields).

## Rule 2 — The portfolio schema (v5 bump)

```toml
[zones.demand.heating]            # per-zone; REPLACES the v1–v4 sketch block
delivered_heat_twh = 300.0        # record-mean annual quantum (rule 3), cited
electrified_share = 0.5           # fraction of the quantum electrified
dhw_fraction = 0.16               # illustrative; cited from ECUK (data package)
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.heating.entries]]
kind = "ashp"                     # ashp | gshp | district_geothermal
share = 0.70                      # of the electrified quantum; shares sum to 1
# optional per-entry COP-parameter overrides (rule 4); defaults live in
# data/reference/heating-cop.toml

[[zones.demand.heating.entries]]
kind = "gshp"
share = 0.20

[[zones.demand.heating.entries]]
kind = "district_geothermal"
share = 0.10
```

The block models the **buildings heat class**: space heating + DHW,
domestic + services — the ECUK quantum, and the field docs say so.
Industrial/process heat (higher temperature, flatter profile; deep
geothermal's high-quality-heat case) is a NAMED follow-on: a
sibling optional block (e.g. `[zones.demand.process_heat]`) with
its own entries and shape model — purely additive, no v5 field
reinterpreted, no overlay-pipeline rework. Per the Stage 0
strictness law it will cost a one-line `schema_version` bump like
every addition (v2→v3→v4 precedent); what this design guarantees is
that it costs nothing else.

`schema_version` bumps to v5 with the docs/03 migration note. v5
**replaces** the v1–v4 `[zones.demand.heating]` sketch
(`enabled` / `heat_demand_per_degree` / `cop_curve` — opaque
placeholders that no engine code ever read): those fields are
removed, and a v4 file carrying the old block fails with a
structured migration message naming the replacement. A v4 file
without the block migrates by changing only the version line. The
two live reference scenarios carry the old disabled block and are
edited in the same commit (version line + block removal); the old
block is engine-inert, so the dispatch digest `779d7444…` must be
re-verified unmoved on both — an explicit acceptance check, not an
assumption. The v4 reference scenario is frozen verbatim under
`grid-core/tests/fixtures/` (v1/v2/v3 precedent) so the migration
error path stays tested. Shares must satisfy `|Σ share − 1| ≤ 1e-9`
(structured error naming the sum and the entries); `share` and the
fractions are validated in [0, 1]; unknown kinds rejected
(`deny_unknown_fields` discipline); the heating block absent ⇒
engine byte-path untouched.

## Rule 3 — Heat demand shape (the most contestable choice, so pinned
## hard)

Half-hourly heat demand is **degree-hour proportional with a floor**:

- `heat_need(t) = max(T_base − T_pop(t), 0)` on the population-weighted
  GB air temperature `T_pop(t)` (2 m, ERA5, same pinned derivation
  machinery as the EU t2m traces), with `T_base = 15.5 °C` (the UK
  degree-day convention, cited in the data package).
- A **hot-water floor**: a stated fraction of the annual quantum is
  temperature-independent (DHW), spread flat. Fraction cited from
  ECUK in the data package.
- Space-heat electrical energy is scaled by a **single pinned
  intensity coefficient**, never per-year renormalisation:
  `k = electrified space-heat quantum ÷ mean annual degree-hours
  over the pinned reference window (1985–2024)`, computed once from
  the pinned `T_pop` trace, recorded in run outputs. Half-hourly
  heat is `heat(t) = k · heat_need(t) + DHW rate` (DHW rate = DHW
  fraction × electrified quantum, spread flat). `delivered_heat_twh`
  is therefore the **record-mean annual quantum**: cold years draw
  more heat than mild years — the inter-annual physics the 40-year
  storage question exists to measure, and the second half of the
  cold-year covariance (more heat AND worse COP in the same years),
  which this construction captures and per-year renormalisation
  would sever. `heat(t)` is a pure function of `T_pop(t)`: horizon
  subsetting never changes it (ADR-5 composability). Conservation
  is asserted over the reference window — mean annual delivered
  heat = quantum to a stated float tolerance — and per-year totals
  are a reported output whose spread is a finding.
- **No within-day behavioural profile in v1** (no morning/evening
  peaking beyond what temperature drives). This UNDERSTATES the
  heating peak — direction stated, prominent, in every artefact.
  The omission understates the PORTFOLIO DELTAS in the same
  direction: behavioural morning/evening peaking lands on cold,
  solar-free hours, so the missing profile scales down `heat(t)` at
  the binding residual peak and with it the ASHP−GSHP and
  ASHP−district peak deltas — the measured network value of
  geothermal is a **lower bound** under this convention, stated
  wherever the rule-6 gradient is quoted. Second owned limitation:
  the no-intercept degree-hour model overstates mild shoulder-hour
  heat (real systems switch off under solar gains and
  intermittency), so at fixed quantum it understates the cold-snap
  share — the same conservative direction. `T_base = 15.5 °C` is
  accepted as drafted (the UK degree-day convention, cited).
  Rationale: a behavioural profile is a second contestable convention
  stacked on the first; v1 isolates the temperature physics that
  differentiates the technologies, which is the question Richard is
  asking. A When2Heat-profile variant is the named follow-on if review
  or the paper demands it.

## Rule 4 — COP models: the source temperature is the point

Electrical demand per entry: `P_elec(t) = share × heat(t) / COP(t)`.

- **ASHP:** `COP(t) = f(T_sink(t) − T_source(t))` with
  `T_source = T_pop(t)` (air). Curve: the When2Heat (Ruhnau et al.
  2019) quadratic ASHP parameterisation, radiator convention with
  weather-compensated flow temperature (their heating-curve form),
  cross-checked against the GB RHPP field-trial seasonal performance
  factors, mechanics pinned: (i) the model-implied SPF per technology
  is computed with the model's own rule-3 heat weighting over the
  pinned record — not a manufacturer weighting; (ii) the comparison
  is at a **stated SPF system boundary** (SPFH2 vs SPFH4 — the RHPP
  band and the When2Heat curve must be brought to the same boundary,
  and the boundary is named next to every cross-check number);
  (iii) When2Heat's own field-calibration correction factor is
  transcribed and its status stated (retained or replaced), so the
  RHPP derating is never stacked on top of it — no double-derating;
  (iv) if the implied SPF falls outside the RHPP band, ONE
  multiplicative derating factor **per technology** (ASHP and GSHP
  determined independently) is applied to the COP curve and stated —
  the ERA5-CF one-factor-per-tech calibration precedent. The data
  package delivers all four items per technology; the cross-check
  counts as done only when they are all present.
- **GSHP:** same curve family with the GSHP parameterisation, but
  `T_source = T_ground(t)` modelled as the **damped, phase-lagged
  annual wave of `T_pop`**: `T_ground(t) = mean(T_pop) +
  A·damping·sin(ω(t − lag))` where A is the annual amplitude of a
  fitted sinusoid on `T_pop`. Damping and lag are the analytic conduction solution, not free
  parameters: `damping = exp(−z√(ω/2α))`, `lag = z/√(2αω)`
  (Kusuda–Achenbach form, cited), at a stated nominal loop depth `z`
  chosen as the **shallow horizontal loop (~1.0–1.2 m)** — the
  conservative case: the deepest winter source depression; boreholes
  are flatter and would flatter the geothermal-value finding — with
  cited GB soil thermal diffusivity `α` (cited range, centre used,
  band stated: the fleet-power-factor precedent). The single-harmonic
  fit on `T_pop` is justified by the same physics: damping depth
  scales with √period, so the ground extinguishes the diurnal and
  synoptic harmonics the fit discards. Two stated limitations: the
  model is UNDISTURBED ground temperature — a loaded loop runs colder
  (extraction depression), absorbed by the RHPP-band derating, which
  is field data and includes it; and population-weighted GB `T_pop`
  stands in for soil-surface forcing. Validation (data package): the
  fitted wave cross-checked against a cited GB measured shallow-soil
  temperature series (e.g. Met Office MIDAS 100 cm soil temperature,
  or BGS shallow ground temperature data), amplitude and phase within
  a stated tolerance. ERA5 soil temperature levels are ordered ONLY
  if that cross-check fails its tolerance (reviewer ruling A,
  d9-heating-overlay-review.md — Richard's stated fallback order,
  trigger now defined). The point this model exists to
  carry: the GSHP source barely feels the cold snap that crushes ASHP
  COP — the covariance of COP with system stress is the physics under
  the network-value question.
- **District/deep geothermal:** direct heat — the network sees only
  pump load: a constant effective COP (heat delivered ÷ electrical
  pump+auxiliary draw), cited from operating GB/EU scheme data in the
  data package, temperature-independent by construction.
  `COP_const` is defined as **heat delivered to buildings ÷ total
  electrical draw** (pumps + auxiliaries), network distribution
  losses inside the ratio — i.e. the cited operating-scheme figures
  must be on the delivered-heat basis, and the data package states
  the basis next to the number. Validation: `COP_const` must exceed
  the heat pumps' maximum record COP (the premise of the
  district-lowest ordering limb, checked, not assumed). No
  resistive backup line for any technology in v1; the COP curves
  already degrade smoothly, and a backup convention is a follow-on
  with the behavioural profile. Limitation stated.

Default COP parameters live in a cited, drift-guarded reference file
`data/reference/heating-cop.toml` (the `inertia-constants.toml`
precedent), NOT hard-coded and NOT free scenario text; optional
per-entry scenario overrides are legal and always emitted into run
outputs (the reliability/inertia overrides precedent).

## Rule 5 — The invariance property (the acceptance-test spine)

Across any two portfolios with the same `delivered_heat_twh` and
`electrified_share`, delivered heat is IDENTICAL by construction —
property test: the heat-side integral matches the quantum exactly for
every portfolio, every weather year. What varies is only electrical
demand shape and total. Acceptance tests, red-first:
1. Conservation: reference-window (1985–2024) mean annual delivered
   heat = quantum to a stated float tolerance, all mixes; per-year
   totals vary with weather — the inter-annual spread is a reported
   output, never normalised away. Share-sum validation per rule 2.
2. No-heating-block scenarios: dispatch digest bit-identical to the
   pinned reference (`779d7444…`) — including the two reference
   scenarios after their v5 migration edit (old inert block
   removed; rule 2).
3. Direction: the district-lowest limb is asserted red-first (a
   theorem given the rule-4 `COP_const` check). The ASHP-vs-GSHP
   peak ordering is **a measured finding, pinned from measurement**
   — parameter-contingent (two independent deratings, curve
   coefficients, portfolio-dependent binding hours), so it is
   never pre-committed as a theorem; the expected direction
   (all-ASHP peak ≥ all-GSHP) is recorded here as an expectation,
   and an inversion is a finding at full prominence (the
   Package A/B lesson, kill-criterion 4).
4. A pinned characterisation run: one stated three-way mix on the RS
   fleet, 40-year storage requirement + peak residual demand, both
   deltas vs the no-heating baseline.

## Rule 6 — The Q5 analysis runs (what Richard gets)

The deliverable analysis: hold heat decarbonisation identical, sweep
the portfolio mix (ASHP/GSHP/district shares on a simplex grid),
report (a) peak residual demand delta, (b) 40-year storage requirement
delta (bisection solve, Stage 3 machinery), (c) the timescale
decomposition of the added requirement (Stage 4 machinery — does
electrified heat load the seasonal band or the synoptic band?). The
"network value of geothermal" = the gradient of (a) and (b) along the
ASHP→GSHP and ASHP→district axes, quoted only with three named caveats plus the standing programme
set: (a) rule 3's no-behavioural-profile caveat, with the
lower-bound direction on the deltas; (b) 2024 non-heat
demand tiling under the overlay; (c) **climate-stationary heat
intensity** — one pinned `k` and DHW fraction across all 40 weather
years means a fixed building stock (no retrofit trend, no stock
growth, no warming-trend adjustment): the runs answer "today's/the
stated stock in year Y's weather", the Stage 3 fixed-fleet
convention applied to heat. The cold-year covariance itself is
captured by construction (rule 3) and is a finding, not a
caveat. Standing programme caveats (frozen-2024 curtailment in the
calibrated CF traces, frozen-imports convention) apply to the RS
fleet under the sweep as to every scaled-fleet result.

## Rule 6b — The geothermal-relief analysis (Richard, 2026-07-03)

Second deliverable, same runs plus differencing: quantify what a
geothermal share relieves, in the system's own terms.
- **Generation relieved**: per-technology generation deltas
  (wind/solar/gas/nuclear TWh and their dispatch shapes) between
  portfolio mixes at fixed fleet — read off the existing per-tech
  dispatch outputs of paired runs; the overlay never touches
  supply-side attribution.
- **Capacity relieved (avoided build)**: for a stated geothermal
  share, the capacity of a NAMED resource (wind+storage, nuclear,
  or gas) whose addition becomes unnecessary at the same adequacy
  standard — computed only as equal-reliability re-solved pairs
  (the D8 rule-3/rule-5 conventions: both endpoints solved to zero
  unserved on 1985–2024 by the same stated 1-D solver, both
  endpoints stated, difference quoted inclusive of balancing
  consequences). Storage-side relief runs on the Stage 3 bisection
  today; capacity-side relief REQUIRES the 1-D capacity solver (the
  ELCC-runner machinery, wave-2 paper-4 enabler) — a named
  dependency of this deliverable, not of the overlay engine work.
- Results are physical (GW, TWh) until Stage 7; £ valuation then
  follows D8 rules 3/5 unchanged.

**Overlay output series** (ADR-5 discipline — every convention
visible in outputs): per-period heating electrical demand, total
and per-entry; per-period delivered heat; the pinned constants
(`k`, DHW rate, damping, lag, derating factors, `COP_const`) and
any per-entry overrides echoed into run outputs (the
reliability/inertia precedent). Residual-load and decomposition
machinery see heating inside demand — no special-casing.

## Data requirements (data package, licence-checked first)

1. GB population-weighted t2m trace 1985–2024 from the GB ERA5 cutout
   (the machinery exists for EU zones; GB needs the same derivation —
   GB CF manifests must stay byte-untouched).
2. When2Heat COP parameterisations (Ruhnau et al. 2019, Sci Data —
   the paper is CC BY 4.0 open access; the companion OPSD
   when2heat data package carries its own licence terms — record
   BOTH, cite the parameter table directly from the paper, and
   transcribe their field-calibration correction factor with its
   retained/replaced status per rule 4).
3. RHPP field-trial SPF bands for ASHP/GSHP (UK, cited).
4. ECUK (or equivalent DESNZ) annual GB heat quantum: space + hot
   water, domestic + services, and the DHW fraction — additionally
   recording the DHW fraction's definitional basis (fraction of
   DELIVERED heat, domestic + services, matching the quantum's
   scope; a mismatched basis silently rescales the floor).
5. Soil damping/lag parameters at nominal loop depth (cited
   geotechnical source).
6. District/deep geothermal effective COP from operating scheme data
   (delivered-heat basis stated per rule 4).
7. GB measured shallow-soil temperature series for the ruling-A
   ground-model cross-check (Met Office MIDAS 100 cm soil
   temperature or BGS shallow ground temperature data; licence
   checked and cited).

## What this note does NOT do

No heating costs (Stage 7, per the ratified split); no hybrid/backup
systems, no behavioural demand profile, no building-stock model, no
industrial/process-heat class (named follow-on, extension path pinned
in rule 2), no half-hourly DHW draw profile (all named follow-ons);
no change to any existing trace, pin, or convention.
