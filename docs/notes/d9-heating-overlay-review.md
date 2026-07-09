# D9 adjudication — heating-overlay design note (reviewer)

Reviewer adjudication, 2026-07-03. Subject: the uncommitted
`docs/notes/d9-heating-overlay.md` (supervisor draft) plus its docs/08
D9 row. Richard's ratified requirement (project-state "Q5
HEATING-OVERLAY DESIGN REQUIREMENT", 2026-07-03: technology portfolio
from day one; per-entry kind/share/COP-model; ASHP = air t2m; GSHP =
damped phase-lagged annual wave of t2m, ERA5 soil levels only if
review demands; district/deep geothermal = pump-load only; When2Heat
COP curves cross-checked vs GB RHPP field trials; cost side Stage 7)
is not relitigated. A same-day Richard-ratified SCOPE EXTENSION
(relayed mid-adjudication, 2026-07-03) is adjudicated in its own
section below: the overlay must also support quantifying the value of
geothermal heat at scale in terms of the wind/solar/gas/nuclear
generation and capacity it relieves. This adjudication tests whether the draft
implements that requirement soundly enough that no unreviewed
convention is left to the implementer. Faithfulness to the ratified
requirement: verified, element by element — the portfolio shape, the
source-temperature distinction, the fallback order, and the Stage 7
cost split are all correctly transcribed.

## VERDICT: ADOPT-WITH-EDITS

The draft's skeleton is right: demand-side transformation only,
portfolio schema, the COP source temperature as the load-bearing
physics, invariance-of-delivered-heat as the acceptance-test spine,
simplex mix sweep + timescale decomposition as the deliverable. But it
has one first-order physics defect — rule 3's per-year-exact quantum
normalisation erases the inter-annual heat band, corrupting exactly
the 40-year storage question the overlay exists to answer — plus a
false migration claim (the v5 bump is NOT additive: the v1–v4 sketch
`HeatingSpec` is live in the schema and in both reference scenarios),
a schema block in the wrong place with a missing trace reference, and
several unpinned conventions in the COP chain. The Richard-ratified
scope extension (the geothermal-relief analysis) is largely already
supported by the draft's architecture — the overlay is a pure
demand-side transformation, so supply-side attribution and
equal-reliability differencing survive it by construction (rulings
D–F) — but needs two edits to be load-bearing rather than lucky.
Twelve blocking edits (11–12 from the scope extension). Apply all
twelve, then the note is ADOPTED and the docs/08 D9 row updates to
"Resolved" citing this review.

---

## The ordered rulings (A–C requested explicitly; D–F from the
## scope extension)

### Ruling A — GSHP ground model: damped wave ADEQUATE; ERA5 soil
### levels NOT ordered

The damped phase-lagged annual sinusoid is not merely an acceptable
approximation — at GSHP loop depth it is the physics. The ground is a
low-pass filter with damping depth ∝ √period (Kusuda–Achenbach /
Carslaw & Jaeger conduction solution): at ~1 m the diurnal and
synoptic harmonics of surface temperature are extinguished and only
the annual harmonic survives materially, which is precisely why a
single-harmonic fit on `T_pop` is adequate — the higher harmonics the
fit discards are the ones the ground physically removes. ERA5 soil
levels would add resolution the loop never sees, at the cost of a new
data dependency and a level-to-loop-depth mapping convention of its
own. Fallback trigger stays defined: if the edit-5 validation
cross-check fails its stated tolerance, ERA5 soil levels are ordered
at that point — not before.

Conditions (edit 5): the analytic form and its two parameters must be
pinned from citation, not tuned — damping = exp(−z√(ω/2α)) and
lag = z/√(2αω) at a **stated nominal loop depth z** with **cited GB
soil thermal diffusivity α** (a cited range, centre used, band
stated — the inertia power-factor precedent). Loop depth choice must
be the conservative one: shallow horizontal loop (~1.0–1.2 m), which
gives the deepest winter source depression; boreholes are flatter and
would flatter the geothermal-value finding. The model gives
UNDISTURBED ground temperature; a loaded loop runs colder (extraction
depression) — stated limitation, absorbed by the RHPP-band derating
(edit 6), which is field data and therefore includes it. Evidence
that satisfies the reviewer: a cross-check of the fitted wave against
a cited GB measured shallow-soil-temperature series (e.g. Met Office
MIDAS 100 cm soil temperature climatology, or BGS shallow ground
temperature data — Busby), amplitude and phase within a stated
tolerance, delivered in the data package.

### Ruling B — The fixed annual quantum: DEFECT, blocking (the most
### important item in this adjudication)

Rule 3 as drafted ("allocated to periods proportional to heat_need(t)
so the ANNUAL delivered-heat quantum is met EXACTLY") together with
acceptance test 1 ("annual delivered heat = quantum, all years")
forces per-year renormalisation: every weather year delivers exactly
300 TWh regardless of how cold it was. Consequences, all fatal to the
stated purpose:

1. Cold years do not draw more heat than mild years — the
   inter-annual band of electrified heat demand is zeroed **by
   construction**, and rule 6's deliverable (c), the timescale
   decomposition of the added requirement, would report an
   inter-annual contribution that is an artefact of the
   normalisation, not a finding. The Stage 4 lesson applies: the
   attribution machinery must not have its answer built in.
2. Half the cold-year covariance is severed. The physics under the
   40-year storage question is that a cold year needs MORE heat AND
   delivers it at WORSE COP; per-year renormalisation keeps only the
   COP half.
3. It is not horizon-composable: the same calendar year yields
   different heat(t) in a 1-year and a 40-year run (different
   normalisation constants) — an ADR-5 determinism smell.

Fix (edit 1): one pinned intensity coefficient over the full
reference window, `heat(t)` a pure function of `T_pop(t)`.
`delivered_heat_twh` becomes the **record-mean** annual quantum;
per-year totals vary with weather and the spread is a reported
finding. Acceptance test 1 changes accordingly.

### Ruling C — The ordering test: split; the ASHP≥GSHP limb is NOT a
### theorem — demoted to a measured, pinned finding from the start

- **District-lowest limb**: a theorem GIVEN `COP_const` exceeds the
  heat pumps' COP everywhere on the record — with cited district
  effective COPs (order 10+ heat-out per unit pump electricity) vs
  heat-pump COPs of order 2–4, this holds with margin; edit 8 adds
  the explicit check on the cited value so the theorem's premise is
  machine-verified, and then the limb may be asserted red-first.
- **ASHP ≥ GSHP limb**: parameter-contingent, not a theorem. It
  depends on (i) the two When2Heat quadratics (the GSHP curve sits
  lower at equal lift — brine circuit losses), (ii) two
  independently determined RHPP derating factors (edit 6) that can
  move the curves toward each other, and (iii) which hour binds the
  40-year peak for each portfolio (the binding hour can differ by
  portfolio). At the coldest system hours the lift difference
  dominates and the ordering is near-certain empirically — but
  "near-certain empirically" is exactly what the Package A/B lesson
  says to measure and pin, not pre-commit. The draft's
  verify-before-pin already concedes this; edit 8 makes the demotion
  explicit: the test's content is the MEASURED ordering, pinned; the
  expected direction lives in D9 as an expectation.

**Test 2 achievability: VERIFIED.** The dispatch digest `779d7444…`
is over dispatch outputs, not scenario text, and survived the v2→v3
and v3→v4 bumps unmoved (stage-6 and stage-5 review records; the
version-line edit to the reference scenarios does not perturb it, and
the v-older reference file is frozen as a fixture each time). BUT the
draft's premise "v4 files migrate by adding nothing" is false — see
edit 4 — because both live reference scenarios carry the OLD
`[zones.demand.heating]` block (`enabled = false`), which v5 removes.
That block is engine-inert today (no heating computation exists), so
the digest must and will survive its removal; edit 4 makes the
re-verification explicit rather than assumed.


### Scope-extension rulings (Richard, 2026-07-03: the geothermal-
### relief analysis — generation and capacity relieved)

**Ruling D — (a) per-technology generation deltas: SUPPORTED as
drafted, with one output addition (edit 11).** The overlay is a pure
demand-side transformation applied before dispatch (rule 1); the
merit-order stack, the per-technology dispatch series, and their
attribution are untouched by construction — per-tech generation
deltas between two portfolio mixes are read directly off the existing
per-tech outputs of the two runs. Verified against the draft: nothing
in rules 1–4 touches supply-side accounting. The one gap: the note
never lists the overlay's OWN output series, and without a reported
per-period heating electrical demand (total and per-entry) the relief
attribution cannot separate heating-driven dispatch changes from
noise, and the ADR-5 output-artefact discipline (every convention
visible in outputs) is unmet. Edit 11 orders the output list.

**Ruling E — (b) equal-reliability fleet differencing: COMPATIBLE;
one named machinery gap, outside D9, no overlay rework.** The
avoided-build question ("the wind+storage / nuclear / gas additions a
geothermal share makes unnecessary at the same adequacy standard") is
exactly a D8 rule-5 re-solved pair: two portfolio mixes, each
re-solved to the rule-3 standard (zero unserved, 1985–2024) by a
stated 1-D solver, difference quoted with both endpoints stated. The
overlay is compatible because it is deterministic in the scenario and
orthogonal to the solve axis — the solver varies fleet or storage
while the heating block stays fixed. The storage-side relief (avoided
TWh) runs TODAY on the Stage 3 bisection
(`min_storage_for_zero_unserved` — the only solver mode in the
schema, verified). The capacity-side relief (avoided wind/nuclear/gas
GW) needs a 1-D CAPACITY bisection — the ELCC-runner machinery
already queued as the wave-2 paper-4 enabler; monotone in capacity,
so the ADR-10 bisection strategy covers it. That dependency is NAMED
in edit 11, not hidden; no D9 construct prejudices it. Until Stage 7,
results are physical (GW, TWh); £ valuation lands under D8 rules 3/5
with no convention conflict — D8 rule 5's "inclusive of balancing
consequences" is precisely the framing Richard's relief question
wants.

**Ruling F — (c) industrial/process heat: STATED FOLLOW-ON, schema
not prejudiced (edit 12) — but no version-bump exemption exists.**
The v5 quantum is the BUILDINGS heat class (space + DHW, domestic +
services — the ECUK scope), and edit 12 names it as such in the
schema's field documentation. A process-heat class (higher
temperature, flatter profile, servable by deep geothermal but not by
building-class heat pumps) extends as a SIBLING optional block with
its own entries — purely additive, no v5 field reinterpreted, no
engine rework: the overlay pipeline (quantum → shape → per-entry
electrical draw) is class-generic by construction once edit 12's
naming lands. On "should not need a v6 bump": the Stage 0
reviewer-ratified strictness law says ANY schema addition requires a
`schema_version` bump — that law is not waivable by this note, and
the v2→v3→v4 precedent shows such bumps are deliberate one-line
migrations, not redesigns. The correct promise, and the one edit 12
makes, is: adding the class costs one version line and zero reshaping
— not zero bump.

---

## Blocking edits (exact text where a change is ordered)

### Edit 1 — Rule 3: replace per-year renormalisation with a pinned
### intensity coefficient (Ruling B)

Replace rule 3's third bullet ("Space-heat energy is allocated …
conservation acceptance test).") with:

> - Space-heat electrical energy is scaled by a **single pinned
>   intensity coefficient**, never per-year renormalisation:
>   `k = electrified space-heat quantum ÷ mean annual degree-hours
>   over the pinned reference window (1985–2024)`, computed once from
>   the pinned `T_pop` trace, recorded in run outputs. Half-hourly
>   heat is `heat(t) = k · heat_need(t) + DHW rate` (DHW rate = DHW
>   fraction × electrified quantum, spread flat). `delivered_heat_twh`
>   is therefore the **record-mean annual quantum**: cold years draw
>   more heat than mild years — the inter-annual physics the 40-year
>   storage question exists to measure, and the second half of the
>   cold-year covariance (more heat AND worse COP in the same years),
>   which this construction captures and per-year renormalisation
>   would sever. `heat(t)` is a pure function of `T_pop(t)`: horizon
>   subsetting never changes it (ADR-5 composability). Conservation
>   is asserted over the reference window — mean annual delivered
>   heat = quantum to a stated float tolerance — and per-year totals
>   are a reported output whose spread is a finding.

### Edit 2 — Rule 3: state the DELTA direction, not only the level
### direction, and own the shoulder-hour limitation

The draft's direction claim ("UNDERSTATES the heating peak") is
correct but is a LEVEL claim; the question asked is about portfolio
DELTAS. The omission biases the deltas too, in a statable direction —
which is why the profile may stay out of v1. Append to rule 3's
fourth bullet, after "direction stated, prominent, in every
artefact.":

> The omission understates the PORTFOLIO DELTAS in the same
> direction: behavioural morning/evening peaking lands on cold,
> solar-free hours, so the missing profile scales down `heat(t)` at
> the binding residual peak and with it the ASHP−GSHP and
> ASHP−district peak deltas — the measured network value of
> geothermal is a **lower bound** under this convention, stated
> wherever the rule-6 gradient is quoted. Second owned limitation:
> the no-intercept degree-hour model overstates mild shoulder-hour
> heat (real systems switch off under solar gains and
> intermittency), so at fixed quantum it understates the cold-snap
> share — the same conservative direction. `T_base = 15.5 °C` is
> accepted as drafted (the UK degree-day convention, cited).

### Edit 3 — Rule 2: the block is per-zone, and it must carry its
### temperature trace

Top-level `[heating]` contradicts the schema's shape: the demand
model is per-zone (docs/03 `DemandModel`; `DemandSpec.heating` in
`grid-core/src/scenario.rs`), and in a five-zone scenario (ADR-7) a
top-level block is ambiguous about which zone's demand it transforms.
The draft's sketch also carries **no trace reference** — a scenario
must be self-contained (docs/03; data is fetched-and-built, so the
path must be in the file). Replace the rule-2 sketch with:

```toml
[zones.demand.heating]            # per-zone; REPLACES the v1–v4 sketch block
delivered_heat_twh = 300.0        # record-mean annual quantum (rule 3), cited
electrified_share = 0.5           # fraction of the quantum electrified
dhw_fraction = 0.16               # illustrative; cited from ECUK (data package)
temperature_trace = { path = "data/weather/gb_t2m_pop.parquet", column = "t2m_pop" }

[[zones.demand.heating.entries]]
kind = "ashp"                     # ashp | gshp | district_geothermal
share = 0.70                      # of the electrified quantum; shares sum to 1
# optional per-entry COP-parameter overrides (edit 6); defaults live in
# data/reference/heating-cop.toml

[[zones.demand.heating.entries]]
kind = "gshp"
share = 0.20

[[zones.demand.heating.entries]]
kind = "district_geothermal"
share = 0.10
```

### Edit 4 — Rules 1 and 2: the v5 bump is NOT additive — the live
### sketch block must be explicitly removed, and the note must say so

The v1–v4 schema already parses `[zones.demand.heating]`
(`HeatingSpec`: `enabled`, `temperature_trace`,
`heat_demand_per_degree`, `cop_curve` as `Option<String>` —
`grid-core/src/scenario.rs:709–725`), and BOTH live reference
scenarios carry it (`scenarios/gb-2024-reference.toml:119`,
`scenarios/gb-2024-5zone.toml:137`, `enabled = false`). "v4 files
migrate by adding nothing" is therefore false, and rule 1's
"additive schema work" claim is inaccurate. Replace rule 2's
validation paragraph's migration sentences (from "schema_version
bumps to v5" to the end of the paragraph) with:

> `schema_version` bumps to v5 with the docs/03 migration note. v5
> **replaces** the v1–v4 `[zones.demand.heating]` sketch
> (`enabled` / `heat_demand_per_degree` / `cop_curve` — opaque
> placeholders that no engine code ever read): those fields are
> removed, and a v4 file carrying the old block fails with a
> structured migration message naming the replacement. A v4 file
> without the block migrates by changing only the version line. The
> two live reference scenarios carry the old disabled block and are
> edited in the same commit (version line + block removal); the old
> block is engine-inert, so the dispatch digest `779d7444…` must be
> re-verified unmoved on both — an explicit acceptance check, not an
> assumption. The v4 reference scenario is frozen verbatim under
> `grid-core/tests/fixtures/` (v1/v2/v3 precedent) so the migration
> error path stays tested. Shares must satisfy `|Σ share − 1| ≤ 1e-9`
> (structured error naming the sum and the entries); `share` and the
> fractions are validated in [0, 1]; unknown kinds rejected
> (`deny_unknown_fields` discipline); the heating block absent ⇒
> engine byte-path untouched.

And in rule 1, replace "(the Stage 3/5/6 precedent for additive
schema work: old pins never move)" with "(the Stage 3/5/6 precedent:
old pins never move — though v5 is not purely additive; see rule 2
on the removed sketch fields)".

### Edit 5 — Rule 4 GSHP: pin the ground model's provenance,
### conservative loop depth, and validation cross-check (Ruling A)

Replace the GSHP bullet's text from "Damping and lag pinned" to
"(Richard's stated fallback order)." with:

> Damping and lag are the analytic conduction solution, not free
> parameters: `damping = exp(−z√(ω/2α))`, `lag = z/√(2αω)`
> (Kusuda–Achenbach form, cited), at a stated nominal loop depth `z`
> chosen as the **shallow horizontal loop (~1.0–1.2 m)** — the
> conservative case: the deepest winter source depression; boreholes
> are flatter and would flatter the geothermal-value finding — with
> cited GB soil thermal diffusivity `α` (cited range, centre used,
> band stated: the fleet-power-factor precedent). The single-harmonic
> fit on `T_pop` is justified by the same physics: damping depth
> scales with √period, so the ground extinguishes the diurnal and
> synoptic harmonics the fit discards. Two stated limitations: the
> model is UNDISTURBED ground temperature — a loaded loop runs colder
> (extraction depression), absorbed by the RHPP-band derating, which
> is field data and includes it; and population-weighted GB `T_pop`
> stands in for soil-surface forcing. Validation (data package): the
> fitted wave cross-checked against a cited GB measured shallow-soil
> temperature series (e.g. Met Office MIDAS 100 cm soil temperature,
> or BGS shallow ground temperature data), amplitude and phase within
> a stated tolerance. ERA5 soil temperature levels are ordered ONLY
> if that cross-check fails its tolerance (reviewer ruling A,
> d9-heating-overlay-review.md — Richard's stated fallback order,
> trigger now defined).

### Edit 6 — Rule 4 ASHP/GSHP: pin the cross-check mechanics —
### per-technology factor, correction-factor interaction, SPF
### boundary convention, and what the data package must deliver

Replace the ASHP bullet's text from "cross-checked against" to "the
ERA5-CF calibration precedent)." with:

> cross-checked against the GB RHPP field-trial seasonal performance
> factors, mechanics pinned: (i) the model-implied SPF per technology
> is computed with the model's own rule-3 heat weighting over the
> pinned record — not a manufacturer weighting; (ii) the comparison
> is at a **stated SPF system boundary** (SPFH2 vs SPFH4 — the RHPP
> band and the When2Heat curve must be brought to the same boundary,
> and the boundary is named next to every cross-check number);
> (iii) When2Heat's own field-calibration correction factor is
> transcribed and its status stated (retained or replaced), so the
> RHPP derating is never stacked on top of it — no double-derating;
> (iv) if the implied SPF falls outside the RHPP band, ONE
> multiplicative derating factor **per technology** (ASHP and GSHP
> determined independently) is applied to the COP curve and stated —
> the ERA5-CF one-factor-per-tech calibration precedent. The data
> package delivers all four items per technology; the cross-check
> counts as done only when they are all present.

Default COP parameters live in a cited, drift-guarded reference file
`data/reference/heating-cop.toml` (the `inertia-constants.toml`
precedent), NOT hard-coded and NOT free scenario text; optional
per-entry scenario overrides are legal and always emitted into run
outputs (the reliability/inertia overrides precedent). Add this as a
closing paragraph of rule 4.

### Edit 7 — Rule 4 district: define the constant's basis

The formula `P_elec = share × heat(t) / COP_const` is verified
correct as written — pump load scales with heat delivered and the
COP is what is constant, which is what "constant effective COP"
means; no defect. But the constant's basis is unpinned. After
"temperature-independent by construction.", insert:

> `COP_const` is defined as **heat delivered to buildings ÷ total
> electrical draw** (pumps + auxiliaries), network distribution
> losses inside the ratio — i.e. the cited operating-scheme figures
> must be on the delivered-heat basis, and the data package states
> the basis next to the number. Validation: `COP_const` must exceed
> the heat pumps' maximum record COP (the premise of the
> district-lowest ordering limb, checked, not assumed).

### Edit 8 — Rule 5: restate tests 1–3 per rulings B and C

Replace acceptance tests 1–3 with:

> 1. Conservation: reference-window (1985–2024) mean annual delivered
>    heat = quantum to a stated float tolerance, all mixes; per-year
>    totals vary with weather — the inter-annual spread is a reported
>    output, never normalised away. Share-sum validation per rule 2.
> 2. No-heating-block scenarios: dispatch digest bit-identical to the
>    pinned reference (`779d7444…`) — including the two reference
>    scenarios after their v5 migration edit (old inert block
>    removed; rule 2).
> 3. Direction: the district-lowest limb is asserted red-first (a
>    theorem given the edit-7 `COP_const` check). The ASHP-vs-GSHP
>    peak ordering is **a measured finding, pinned from measurement**
>    — parameter-contingent (two independent deratings, curve
>    coefficients, portfolio-dependent binding hours), so it is
>    never pre-committed as a theorem; the expected direction
>    (all-ASHP peak ≥ all-GSHP) is recorded here as an expectation,
>    and an inversion is a finding at full prominence (the
>    Package A/B lesson, kill-criterion 4).

### Edit 9 — Rule 6: the caveat set is incomplete — add the
### stationarity caveat and inherit the standing programme caveats

Replace "quoted only with the rule-3 no-behavioural-profile caveat
and the demand-shape caveat (2024 non-heat demand tiling under it,
the papers-programme standing caveat)." with:

> quoted only with three named caveats plus the standing programme
> set: (a) rule 3's no-behavioural-profile caveat, with the
> lower-bound direction on the deltas (edit 2); (b) 2024 non-heat
> demand tiling under the overlay; (c) **climate-stationary heat
> intensity** — one pinned `k` and DHW fraction across all 40 weather
> years means a fixed building stock (no retrofit trend, no stock
> growth, no warming-trend adjustment): the runs answer "today's/the
> stated stock in year Y's weather", the Stage 3 fixed-fleet
> convention applied to heat. The cold-year covariance itself is
> captured by construction (rule 3, edit 1) and is a finding, not a
> caveat. Standing programme caveats (frozen-2024 curtailment in the
> calibrated CF traces, frozen-imports convention) apply to the RS
> fleet under the sweep as to every scaled-fleet result.

### Edit 10 — Data requirements: two additions and a licence pin

Append to the data-requirements list:

> 7. GB measured shallow-soil temperature series for the ruling-A
>    ground-model cross-check (Met Office MIDAS 100 cm soil
>    temperature or BGS shallow ground temperature data; licence
>    checked and cited).

And amend item 2 to:

> 2. When2Heat COP parameterisations (Ruhnau et al. 2019, Sci Data —
>    the paper is CC BY 4.0 open access; the companion OPSD
>    when2heat data package carries its own licence terms — record
>    BOTH, cite the parameter table directly from the paper, and
>    transcribe their field-calibration correction factor with its
>    retained/replaced status per edit 6).

Item 4 (ECUK/DESNZ) additionally records the DHW fraction's
definitional basis (fraction of DELIVERED heat, domestic + services,
matching the quantum's scope — a mismatched basis silently rescales
the floor).

### Edit 11 — Rule 6: add the geothermal-relief deliverable (rule 6b)
### and the overlay's output series

Append to the note, after rule 6:

> ## Rule 6b — The geothermal-relief analysis (Richard, 2026-07-03)
>
> Second deliverable, same runs plus differencing: quantify what a
> geothermal share relieves, in the system's own terms.
> - **Generation relieved**: per-technology generation deltas
>   (wind/solar/gas/nuclear TWh and their dispatch shapes) between
>   portfolio mixes at fixed fleet — read off the existing per-tech
>   dispatch outputs of paired runs; the overlay never touches
>   supply-side attribution.
> - **Capacity relieved (avoided build)**: for a stated geothermal
>   share, the capacity of a NAMED resource (wind+storage, nuclear,
>   or gas) whose addition becomes unnecessary at the same adequacy
>   standard — computed only as equal-reliability re-solved pairs
>   (the D8 rule-3/rule-5 conventions: both endpoints solved to zero
>   unserved on 1985–2024 by the same stated 1-D solver, both
>   endpoints stated, difference quoted inclusive of balancing
>   consequences). Storage-side relief runs on the Stage 3 bisection
>   today; capacity-side relief REQUIRES the 1-D capacity solver (the
>   ELCC-runner machinery, wave-2 paper-4 enabler) — a named
>   dependency of this deliverable, not of the overlay engine work.
> - Results are physical (GW, TWh) until Stage 7; £ valuation then
>   follows D8 rules 3/5 unchanged.
>
> **Overlay output series** (ADR-5 discipline — every convention
> visible in outputs): per-period heating electrical demand, total
> and per-entry; per-period delivered heat; the pinned constants
> (`k`, DHW rate, damping, lag, derating factors, `COP_const`) and
> any per-entry overrides echoed into run outputs (the
> reliability/inertia precedent). Residual-load and decomposition
> machinery see heating inside demand — no special-casing.

### Edit 12 — Rules 1–2: name the demand class and the extension
### path (no schema prejudice)

In rule 2, after the sketch, add:

> The block models the **buildings heat class**: space heating + DHW,
> domestic + services — the ECUK quantum, and the field docs say so.
> Industrial/process heat (higher temperature, flatter profile; deep
> geothermal's high-quality-heat case) is a NAMED follow-on: a
> sibling optional block (e.g. `[zones.demand.process_heat]`) with
> its own entries and shape model — purely additive, no v5 field
> reinterpreted, no overlay-pipeline rework. Per the Stage 0
> strictness law it will cost a one-line `schema_version` bump like
> every addition (v2→v3→v4 precedent); what this design guarantees is
> that it costs nothing else.

And in "What this note does NOT do", after "no building-stock
model", insert: "no industrial/process-heat class (named follow-on,
extension path pinned in rule 2)".

---

## Notes of record (non-blocking)

1. **District formula challenge resolved, no defect**: `P_elec =
   share × heat / COP_const` scales pump load with heat delivered;
   prose and formula agree (edit 7 pins only the constant's basis).
2. **`electrified_share` × `delivered_heat_twh` parameterisation:
   ruled KEPT** over a single electrified-TWh field. The
   decarbonisation level is the first-class, citable, sweepable
   quantity in Richard's question ("identical heat decarbonisation,
   vary the mix"). Two clarifications the implementation must carry
   in doc comments: the engine consumes only the product (the
   electrified quantum — two scenarios with equal products are
   physically identical); and `dhw_fraction` applies uniformly
   WITHIN the electrified quantum (electrification assumed
   proportional across space heat and DHW — a stated assumption).
3. **`T_base = 15.5 °C` accepted** (UK degree-day convention, cited
   in the data package).
4. **Working tree**: this adjudication is design-before-code —
   correctly no engine or schema code accompanies D9. The modified
   `grid-cli/tests/stability.rs` / `grid-stability/tests/pathway.rs`
   in the tree belong to the in-flight Q8 package, not D9; they are
   outside this adjudication's scope and must not ride along in the
   D9 commit.
5. **Digest label**: "pinned Stage 1 reference (779d7444…)" matches
   the established usage in the package-A/B and stage-5/6 records;
   no change ordered.
6. TDD posture is correct as drafted: rule 5's tests are written
   red-first at package start and the pinned characterisation run
   (test 4) follows the every-published-number-gets-a-pin law.

## Chain instruction

Apply edits 1–12 to `docs/notes/d9-heating-overlay.md`, update the
docs/08 D9 row to Resolved citing this file (the row's summary should
also name the rule-6b relief deliverable), commit the pair
(note + review + docs/08 + project-state entry) BEFORE the Q5 data
package is briefed — edits 5, 6, 7, and 10 change the data package's
deliverable list; edit 1 changes the engine package's acceptance
test 1; edits 11–12 change the analysis-runs package (rule 6b) and
add the ELCC-runner dependency to the capacity-relief limb (that limb
waits for the wave-2 runner; everything else in rule 6/6b runs
without it).
