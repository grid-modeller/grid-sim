# 07 — Research Syllabus

The tool's functional requirements, expressed as the research it must
support. Two layers: the **core module sequence** (a learning path, each
module building machinery the next needs) and **extension questions**
(Q1–Q10). Each entry states its engine dependency so stages can be checked
against research readiness.

> **Reframing (2026-07-06, D15):** with the adoption of the book programme
> ("[book programme]"), this syllabus's operational role shifts
> from self-learning path to **the book's figure-production programme**:
> modules M1–M7 anchor Part I/IV chapters and the questions map to
> critique/prescription chapters. The per-chapter claim/figure register —
> and therefore what v1 must deliver — lives in the book workspace
> (`[book workspace]` and
> `03-simulator-v1-scope.md`). The module/question content below is
> unchanged and remains the engine-facing specification of each artefact.

## Core modules

### Module 1 — Merit order and marginal pricing
*Why does gas set the price ~97% of hours even at 40% renewable penetration?*
Sweep wind 10→60 GW; plot % hours gas-marginal; show zero-marginal-cost
capacity displaces mid-merit, not the peak, so it widens gas's price-setting
role. **Needs:** Stage 2. **Artefact:** the chart underpinning the
Rosenow-debate arguments.

### Module 2 — Residual demand
*What does the dispatchable fleet actually have to do?* Residual load
(demand − wind − solar): distributions, ramps, duration curves. Show added
renewables barely move the residual *peak* while gutting the *average* —
capacity without capacity credit. The pivotal concept: storage,
interconnectors, and stability all live in the residual load curve.
**Needs:** Stage 1 + residual utilities (Stage 4, or ad hoc earlier).

### Module 3 — Storage: "a few days" vs. 100 TWh
*What drives the requirement — daily cycling or inter-annual drought?*
(a) One benign year + 12 h batteries: works. (b) 1985–2024 continuous:
multi-week Dunkelflaute drawdowns, multi-year recharge — the Royal Society
number reproduced interactively. (c) Timescale decomposition
(diurnal/synoptic/seasonal/inter-annual) attributing TWh to each band; the
"few days" claim answers the diurnal band only. **Needs:** Stages 3–4.
**Artefact:** the decomposition chart — the single most persuasive output.

### Module 4 — The storage / overbuild / curtailment triangle
2D sweep (overbuild × storage) for zero unserved energy; iso-cost contours
using the Subsidy Clock cost stack; show where hydrogen round-trip
efficiency (~35–40%) bites. **Needs:** Stages 3–4 + cost stack (Stage 7 for
full costs; physical triangle earlier).

### Module 5 — Interconnectors
*When do imports arrive, what do they cost, are they there when needed?*
Correlate GB and continental residual loads under shared weather; price
imports off the exporter's scarcity; show capacity credit collapsing at
exactly the peak-residual hours (the anticyclone problem); NO (uncorrelated
hydro) vs. FR (correlated, cf. heatwave nuclear curtailment) asymmetry.
**Needs:** Stage 5.

### Module 6 — Stability and the inertia floor
Pull dispatched synchronous plant per hour from adequacy runs → system
inertia → loss-of-infeed events at minimum-inertia hours. Plot hours/year
below inertia thresholds vs. renewable share; cost the synchronous
condenser / must-run fleet needed — a line item Module 4 omitted, then add
it back. **Needs:** Stage 6.

### Module 7 — Whole-system cost synthesis
For published scenarios (FES pathways, CCC, Royal Society): delivered £/MWh
including storage, overbuild, curtailment, interconnection, stability
services. The Energy Trap thesis as a reproducible computation. **Needs:**
Stage 7. Terminal project.

## Extension questions

**Q1 — True capacity credit of wind.** ELCC: firm capacity needed with vs.
without the wind fleet; nameplate–ELCC gap. *Needs:* Stage 3 (bisection on
displaced firm capacity).

**Q2 — Diminishing returns to correlated capacity.** Marginal useful vs.
curtailed energy per added GW of wind — the 40th GW generates in the same
hours as the first. *Needs:* Stage 4 sweep + per-increment curtailment
attribution.

**Q3 — Cost of the last 10%.** Sweep a carbon constraint; £/tonne abated
hockey stick as constraint → 100%. *Needs:* emissions layer (Stage 2) +
cost stack (Stage 7).

**Q4 — One year or forty?** Every year 1985–2024 as an independent scenario;
distribution of storage/firm-capacity requirements; identify the design
year. Teaches why "modelled on 2019 data" is a tell. *Needs:* Stage 4 batch
mode.

**Q5 — Electrified heat.** Heat pumps convert temperature-correlated gas
load into temperature-correlated *electrical* load peaking in cold wind-lull
anticyclones. Storage requirement with gas heating vs. electrified.
*Needs:* demand heating overlay (schema v1, engine activation is its own
task) — the one genuinely new subsystem; where 2035–2050 numbers get
frightening.

**Q6 — Demand flexibility vs. storage.** DSR as pseudo-storage
(shift-duration + volume limits): clears the diurnal band, does nothing for
synoptic/seasonal. *Needs:* Stage 3 (`StorageKind::Dsr`) + Module 3
decomposition.

**Q7 — What is nuclear worth?** Identical adequacy targets at 0/10/20/30 GW
nuclear; storage/overbuild/curtailment re-optimise; storage requirement
collapses non-linearly. *Needs:* Stage 4 sweeps + nuclear availability
profiles.

**Q8 — How fast can the system fail?** FES pathway year by year:
largest-survivable-loss vs. time; the year the grid can't ride through
losing its biggest infeed is a *date*. *Needs:* Stage 6 pathway runner.

**Q9 — LCOE vs. the socket.** Same fleet: per-tech LCOE vs. delivered
system £/MWh, gap decomposed (backup, balancing, curtailment, transmission,
stability). The most transferable teaching artefact. *Needs:* Stage 7.

**Q10 — Who pays when the wind blows?** Wind capture price vs. penetration;
cannibalisation; why merchant renewables need subsidy floors forever — the
Energy Trap in one chart. *Needs:* Stage 2 revenue accounting.

**Q11 — What is geothermal heat worth to the grid?** (added at Richard's
direction 2026-07-03; motivating context: the industry correspondent's programme to
promote geothermal at scale for high-quality heat.) For an identical
heat-decarbonisation quantum, serving heat geothermally instead of via
air-source heat pumps relieves the power system twice: the electrical
demand never arrives, and the demand it removes is the worst kind —
temperature-correlated, peaking in cold wind lulls when COP collapses.
Quantify the value as the wind, solar, gas, and nuclear generation and
the fleet build it relieves: per-technology generation deltas plus
equal-reliability avoided-capacity differencing (the D8 rule-3/rule-5
conventions), and, with Stage 7, the £ system-cost delta per unit of
geothermal heat. *Needs:* heating overlay (D9), Stage 3 solve machinery;
£ valuation needs Stage 7.

**Q12 — The electrification stress test (the Rosenow case).** (Added at
Richard's direction 2026-07-03; target: Jan Rosenow's forthcoming
paper claiming electrification is achievable and economic at
affordable storage costs — thesis laid out in his 2026-06 essay
"Europe's best defence…" and policy brief: efficiency +
electrification + clean power as one system; heat pumps and EVs as
"electrofficiency"; gas −70% by 2040.) Build the system he describes
— deep electrification of buildings heat, road transport, and
industry on a high-renewables fleet with storage at his claimed costs
— and run destructive tests: (i) weather-record stress (which of the
40 years, and which multi-year sequences, break adequacy at the
claimed storage, and what storage actually clears the record); (ii)
the storage-cost frontier (the £/kWh at which the D8 delivered system
cost crosses stated ruin thresholds, WACC-banded); (iii)
demand-assumption stress (COP covariance in cold years, behavioural
heating peaks, EV cold-weather efficiency); (iv) flexibility-delivery
stress (smart-charging/DSR participation below assumption); (v)
efficiency-delivery risk (claimed demand reductions not
materialising). *Needs:* D9 heating overlay; an EV/transport overlay
(NEW — the main scope addition, D10 design note); the process-heat
demand class (extension path pinned in D9); Stage 7 cost stack;
DSR activation (Q6); the behavioural-profile heating variant (named
D9 follow-on — load-bearing here, since the no-profile convention
understates peaks and therefore flatters the thesis under test).

**Q13 — The forward subsidy bill.** (Added at Richard's direction
2026-07-04.) Project future subsidy levels from grid shape and scheme
rules: a transfer-calculator layer applying the actual scheme
mechanics (CfD strike book with per-tranche reference-price classes
and negative-hour suspension rules, RO banding to 2037, Capacity
Market at stated clearing prices, constraint payments via the B6
model) to any scenario or pathway. Validated by backcasting: the
modelled 2024 subsidy bill must reproduce the Subsidy Clock's
measured 2024 record. Design intent: subsidy-aware bid floors let
negative prices EMERGE from scheme rules rather than being assumed —
the pricing extension this question funds. Contract granularity:
strike-banded tranches within technologies (pro-rata generation),
zone-split under B6 — not per-project simulation. *Needs:* Stage 7
pricing machinery; the Clock's contract register (import); B6 for
the constraint leg; D-note before any code.

**Q14 — Emergency-response corpus (game layer).** (Added at Richard's
direction 2026-07-04.) A cited knowledge base mapping blackout severity
tiers to the actual GB statutory emergency apparatus: the Electricity
Supply Emergency Code (ESEC) grades, rota load disconnection (lettered
demand blocks, the rolling ~3-hour schedule), protected-site / priority-
supply and shed order, NESO/ESO emergency instructions, and — as cited
*context only* — ONS excess-winter-mortality and Cold Weather Payment
triggers. Feeds the `grid-game` severity panel (see
`docs/superpowers/specs/2026-07-04-grid-game-design.md` §7). **Explicitly
excludes** any fabricated in-game death count. **Research responsibility:**
Richard produces and adversarially fact-checks the ~15-source corpus
(legislation, emergency powers, martial-law provisions, ESEC operational
codes, household-rationing mechanics, historical precedent); Claude
reviews the vetted material and encodes it as `emergency-response.toml`,
not the primary research. *Needs:* the vetted corpus before the data
asset is authored. Gates the Phase-1 MVP emergency panel.

## Readiness map

| After stage | Modules/questions unlocked |
|---|---|
| 1 | M2 (partial) |
| 2 | M1, Q10, Q3 (partial) |
| 3 | M3(a,b), Q1, Q6 (partial) |
| 4 | M3(c), M4 (physical), M2 full, Q2, Q4, Q7 |
| 5 | M5 |
| 6 | M6, Q8 |
| 7 | M7, Q3, Q9, M4 (costed) |
