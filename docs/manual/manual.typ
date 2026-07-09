// grid-sim manual — plain-English manual, methods-paper source, and
// the canonical capabilities-and-limitations disclosure.
// Commissioned by Richard 2026-07-04. Readability is the gate:
// no project jargon in the body; technical terms defined at first
// use or exiled to the glossary.

#set document(title: "The Grid Simulator: A Manual")
#set page(margin: (x: 2.4cm, y: 2.6cm), numbering: "1")
#set text(font: "Equity A", size: 13pt)
#set par(justify: true, leading: 0.72em)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => [
  #pagebreak(weak: true)
  #block(above: 1.6em, below: 1.0em)[#it]
]

#align(center)[
  #text(22pt, weight: "bold")[The Grid Simulator]
  #v(0.3em)
  #text(14pt)[A Manual: what it does, how it works, \ and what it cannot tell you]
  #v(0.8em)
  #text(10pt)[Draft — Parts 1–7 for readability review. \ Version 0.2, July 2026.]
]

// Draft marker for chapters awaiting Richard's readability sign-off.
#let draftmark = block(
  inset: (left: 0.6em, y: 0.2em),
  stroke: (left: 2pt + rgb("#999999")),
  text(size: 10pt, style: "italic", fill: rgb("#555555"))[
    DRAFT — awaiting Richard's readability sign-off.
  ],
)

// Source-record line at the head of each component chapter.
#let record(..files) = block(
  text(size: 10pt, fill: rgb("#555555"))[
    *The committed record for this chapter:* #files.pos().map(f => raw(f)).join(", ").
  ],
)

#v(2em)

= The problem this simulator solves

Britain is rebuilding its electricity system around the weather. Over
the next two decades, most of the electricity used in this country is
intended to come from wind and sunshine, while the demands placed on
that electricity grow: heating that used to burn gas, and journeys
that used to burn petrol, are to run on power from the grid instead.

Whether this plan works — and what it costs — turns on a small number
of questions that sound simple and are not:

- How much backup does a weather-driven system need, and of what
  kind: power stations held in reserve, stored energy, or cables to
  neighbouring countries?
- How much energy storage is enough? Enough for a calm evening, a
  still fortnight, or a poor decade?
- What happens in the worst weather this country actually
  experiences — not an average winter, but the coldest, stillest
  weeks in living memory, when heating demand is highest at exactly
  the moment wind supply is lowest?
- Does the grid remain stable, second by second, when the heavy
  spinning machinery that has steadied it for a century is retired?
- And what does each version of the future system cost, per unit of
  electricity actually delivered to the people paying for it?

These questions are quantitative. They cannot be settled by
principle, preference, or slogan — only by arithmetic done carefully
on the right data. Yet much of the public debate rests on methods
with known blind spots. Studies that test a proposed system against a
single year of weather cannot see the rare events that set the true
requirement, because the worst week in forty years is, by
definition, unlikely to be in the year you picked. Annual averages
hide the hours that matter: a system can be in surplus for 8,700
hours of the year and still fail catastrophically in the other
sixty. And the cost measure most often quoted in public — the
lifetime cost of one power station divided by its own output —
says nothing about what it costs to make that station's output
_useful_: the backup, the storage, the spare capacity, and the wires
that turn intermittent generation into reliable supply.

This simulator exists to answer such questions by a more honest
method: *replay the actual weather.*

The idea is simple to state. First, describe an electricity system —
so much wind, so much solar, so much nuclear and gas, so much
storage of stated kinds, cables of stated sizes to named neighbours,
and a pattern of demand. The description is a short, readable file;
anyone can inspect it, and anyone can change it. Then let the
simulator march through forty years of recorded British weather —
every half-hour from 1985 to the end of 2024, roughly seven hundred
thousand steps — and at each step do what the real system would have
to do: generate what the weather allows, meet the demand that the
temperature implies, charge and discharge the stores, trade with the
neighbours over the cables, and keep the books. When something big
fails — the largest power station tripping off the grid in an
instant — a second, faster part of the simulator zooms in and asks
whether the system rides through the shock, second by second, or
collapses into blackout.

Everything that happens is recorded and reported: every hour of
unmet demand and when it would have occurred; how deeply the stores
were drawn down and how many years they took to refill; what prices
the market would have produced; how close the grid came to
instability; and what the whole arrangement costs per unit of
delivered electricity, under stated financial assumptions.

Three commitments distinguish the approach, and the rest of this
manual returns to them repeatedly:

+ *The weather is real, and all of it is used.* Not a typical year,
  not a designed stress case, but the actual record of four decades
  — including the wind droughts, the cold calm anticyclonic weeks,
  and the freak good years. If a proposed system fails, it fails on
  weather that demonstrably happened, on a stated date. The length
  of the record matters more than any other single choice: the
  worst events are rare, and a simulator that has not seen them
  will cheerfully approve a system they would destroy.

+ *Time runs in order.* The simulation is chronological: every
  half-hour follows the one before, so storage that was drained by
  last week's calm is still empty when this week's calm arrives.
  Methods that shuffle or average time cannot capture this, and
  storage questions are decided by exactly this.

+ *Anyone can check it.* The same inputs produce the same outputs,
  to the last digit, on any machine, every time. Every published
  figure is guarded by an automatic check that fails loudly if the
  simulator ever stops reproducing it. The input data is fetched
  from named public sources and verified against published
  fingerprints. Nothing depends on trust in the authors.

The simulator was built as the research instrument for a book, _The
Energy Trap_, and for a series of technical papers; it also serves
as a public teaching tool. It is not a lobbying artefact for any
technology. Its purpose is to make the arithmetic of the energy
transition inspectable — to replace "studies show" with "here is the
system, here is the weather, here is what happens; run it yourself."

What this simulator is _not_ is set out plainly in the next part.
It does not model the electricity market's fine structure, the
physical network within England and Wales, or the gas system. Its
answers are bounded by the assumptions this manual discloses, each
with the direction in which it biases results. The discipline
throughout the project is that every capability is stated next to
its limitation — and this manual is where both live.

= What the simulator is, and what it is not

The simulator answers questions about the *physics and economics of
supply meeting demand*. Give it an electricity system and it will
tell you whether that system keeps the lights on through forty years
of real weather, how hard its storage works, what its market prices
look like, whether it survives the sudden loss of its largest power
station, and what it costs per unit of electricity delivered. It
will answer these questions for systems that exist, systems that are
planned, and systems that are merely proposed — and it will answer
them identically every time, for anyone who runs it.

It is just as important to say what it is not.

It is not a model of the electricity *market* in its full
institutional detail. Real market prices emerge from auctions,
bilateral contracts, subsidy schemes, and the strategic behaviour of
traders. The simulator uses a deliberately simple rule — the price
in each half-hour is set by the running cost of the most expensive
plant needed — which captures the structure of pricing well enough
to study broad questions, and is disclosed as too simple for fine
ones. It produces no negative prices, though the real market now
sees them regularly.

It is not a model of the wires. Great Britain is treated as a
handful of connected regions — one solid block, or split at the real
transmission bottlenecks into two (Scotland and everywhere else) or
three (north Scotland, south Scotland, England-and-Wales), and
joined to five European neighbours — with each junction a cable of
limited size carrying the observed capability of the boundary it
represents. Within each region, power flows freely. The thousands
of actual circuits, transformers and substations, and everything
that can go wrong with them, are outside its scope, permanently and
by design.

It is not a model of the gas network, of fuel supply chains, of
cyber attack, of operator error, or of any human failure. The
challenges it tests are the ones set by weather, demand, and the
composition of the generating fleet.

The boundary sits where it does for a reason. Every capability
inside it can be tested against recorded reality — and has been.
Everything outside it would require assumptions that cannot be
checked, and untestable assumptions are how energy modelling earns
its poor reputation. The rule throughout: better a modest instrument
whose answers can be defended than an impressive one whose answers
must be believed.

= How it works: the big picture

The simulator runs on two clocks.

The *slow clock* ticks once every half-hour and runs for forty
years — about seven hundred thousand ticks. At each tick it performs
one round of a simple, strict routine. First, how much power do wind
and sun provide right now, given the recorded weather for this
half-hour of this actual year? Second, what does demand require —
including, if the scenario electrifies heating, the extra load
implied by the recorded temperature at this moment? Third, dispatch:
demand is served in a fixed order of preference — the take-it-or-
leave-it sources first (wind, solar, and the recorded flows the
simulator treats as given), then the cheapest plant, then upward
through the cost order until demand is met or nothing remains. Fourth, storage: surplus
charges the stores; shortfall draws them down. Fifth, the cables to
Europe carry power towards whichever side is under more strain. And
last, the books are written up: what ran, what was wasted, what was
short, what the price was, where every store stands.

Nothing is smoothed, shuffled, or averaged. If three calm weeks in
the winter of 2010 drained the stores, the stores *stay drained*
into the fourth week. This ordering of time is the whole point: it
is what single-year studies and annual averages cannot see, and it
is where weather-driven systems actually fail.

The *fast clock* answers a different question. Pick any moment from
the slow clock's forty years — usually the worst one — and ask: if
the largest single source on the system tripped off *right now*,
would the grid survive the next two minutes? The fast clock steps
through those two minutes at millisecond resolution, tracking the
grid's frequency as it falls, as the emergency response services
inject power, and as the system either recovers or crosses the
threshold where automatic disconnection of demand — a partial
blackout — begins. This part of the simulator was built against the
real event of 9 August 2019, and reproduces the measured depth of
that day's frequency fall to within six thousandths of a hertz.

Two further things underpin both clocks.

*The data is real and traceable.* The weather is the European
Centre's ERA5 reconstruction — the standard scientific record of
what the atmosphere actually did — turned into generation using the
recorded performance of the actual British fleet. Demand, prices,
fuel costs, and the behaviour of every interconnector are drawn from
the official published records of the bodies that operate the
system. Every input file carries a cryptographic fingerprint, and
the simulator stamps those fingerprints into every output, so any
result can be traced back to exactly the data that produced it.

*The arithmetic is exactly repeatable.* The same scenario and the
same data produce the same numbers, to the last digit, on any
machine, any day. This sounds like a small engineering virtue. It is
actually the foundation of everything: it is what allows a published
figure to be guarded by an automatic check, what allows a sceptical
reader to verify a claim rather than trust it, and what makes the
simulator's results *evidence* rather than testimony.

= The components

One chapter per component, each answering the same five questions:
What question does this part answer? How does it work? What was it
checked against? What does it leave out — and which way does that
push the answers? What can you safely conclude, and not conclude?

The fifth question matters most and is easiest to skip, so a word on
how to read these chapters. Nothing here asks to be believed. Every
number is guarded by an automatic check in the codebase that re-runs
the calculation and fails if the answer moves, and every chapter
names the written report — a file in the repository, produced when
the work was done and reviewed adversarially before it was accepted —
that records where the number came from. When a chapter says a result
was "checked against" something, the check itself is on the record
and can be re-run.

== Weather into power

#draftmark
#record("docs/notes/era5-cf-2024-report.md", "docs/notes/stage-3-storage-run-report.md")

=== The question this component answers

Given the weather that actually occurred over Britain — every hour
from 1985 to the end of 2024 — how much electricity would the wind
and solar fleet have produced, half-hour by half-hour?

=== How it works

The weather comes from ERA5, the European Centre for Medium-Range
Weather Forecasts' reconstruction of the atmosphere — the standard
scientific record of what the weather actually did, free to use and
redistribute under an open licence. The simulator's data pipeline
takes hourly wind speeds (at turbine height) and sunlight over a grid
of about two thousand cells covering Britain and its waters, and
turns them into fleet output in three steps.

First, *where the machines are*: wind speeds are sampled at the
locations of the real fleet — eight offshore clusters (Hornsea,
Dogger Bank, the Greater Wash, and so on) and ten onshore regions,
weighted by installed capacity, with solar weighted toward the south
of England where the panels actually are. Second, *what the machines
do with the weather*: wind speed becomes power through a fleet-level
power curve — the familiar S-shape where nothing happens below a few
metres per second, output climbs steeply through the middle speeds,
and turbines shut down in storms — and sunlight becomes solar output
with a correction for panels losing efficiency as they heat up.
Third, *calibration, done once and disclosed*: a single multiplying
factor per technology is chosen so that the simulated 2024 matches
the recorded 2024 annual totals. The factors are the honesty metric
of the whole exercise, and they are published in the pipeline report:
offshore wind 0.90, onshore wind 1.04, solar 0.88 — all comfortably
inside the report's pre-stated credibility band of 0.7 to 1.3. The
same factors are then held fixed for all forty years, because they
describe the fleet, not the weather.

Every derived file carries a cryptographic fingerprint, recorded in
committed manifest files, so a result can always be traced to exactly
the data that produced it.

=== What it was checked against

The recorded output of the real 2024 fleet, from the official
half-hourly generation records. The derived wind trace matches the
observed one with a correlation of 0.97 half-hour by half-hour, and
the month-by-month energy totals correlate at 0.99 for both wind and
solar — the worst wind month is out by 0.65 terawatt-hours in a
month of seven to ten. All of these figures, and the month-by-month
table behind them, are in the pipeline report
(`docs/notes/era5-cf-2024-report.md`), and the derivation is pinned
by checks that re-verify the data files' fingerprints.

=== What it leaves out — and which way that pushes the answers

- *Curtailment, breakdowns, and mid-year construction are not in the
  derived traces.* The real 2024 fleet was sometimes turned down
  deliberately, sometimes broken, and growing through the year; the
  derived traces show a constant end-of-2024 fleet running whenever
  the weather allows. The calibration factor absorbs the annual
  effect — which means *the 2024 pattern of turning wind down is
  frozen into the wind data*. For a future system that curtails more
  or less than 2024 did, the traces carry a mild 2024 flavour. This
  is the main source of the remaining disagreement with observation,
  and it lands on the modelled gas figures downstream.
- *The fleet locations are approximate* — publicly known cluster
  positions, not a site-by-site database. Fine for national totals;
  do not trust it for a single region's output.
- *The solar comparison is partly circular*: the "observed" solar
  record is itself the system operator's estimate, and the model has
  no panel-tilt geometry (tilted panels catch more light than the
  flat-surface sunlight measure used; the calibration factor absorbs
  this too).
- *The observed comparator has its own defect* (a constant
  fleet-size denominator early in 2024), so the 0.97 correlation
  slightly understates the model's real skill — a bias against the
  model, stated in the report.

=== What you can safely conclude

The shape and timing of British wind and solar output over forty
years — the calms, the storms, the seasonal swing, the awful
fortnights — is reproduced well enough at national scale to do
adequacy arithmetic on, and the whole derivation is reproducible
from named public sources. What you cannot do is read off any single
wind farm or region, treat the traces as free of 2024's operating
pattern, or use them for any year's *actual fleet* other than
end-2024: every historical year is answered as "the end-2024 fleet
in that year's weather", which is exactly what a planning question
wants and exactly what a historical reconstruction is not.

== Meeting demand: the dispatch engine and the merit order

#draftmark
#record("docs/notes/stage-1-2024-run-report.md", "docs/notes/stage-2-2024-run-report.md")

=== The question this component answers

Every half-hour, who generates what, and at what price? This is the
engine's core loop: given the weather-driven supply, the demand, and
the fleet, decide what runs, what is spare, what is short — and do it
seven hundred thousand times in a row without ever peeking ahead.

=== How it works

Demand is served in a fixed order of preference — the *merit order*.
Wind, solar and the other take-it-or-leave-it sources go first,
because they cost nothing to run once built. Then the dispatchable
plant, cheapest first — nuclear at the front of that queue — each
running up to what it can actually deliver that half-hour, until
demand is met. (One deliberate simplification, recorded in the
limitations register: nuclear is treated as the cheapest *flexible*
plant, not as a source that must always run — so in rare
deep-surplus half-hours the simulator backs nuclear down where the
real system would more likely have kept it running and curtailed
wind instead. Measured on 2024: 116 of 17,568 half-hours.) In the 2024 reference system the
last, most expensive rungs are the gas fleet — which makes gas *the
residual*: it fills whatever gap the weather leaves, so its
month-to-month pattern is a genuine prediction of the model rather
than something fed in.

Any surplus that nothing can absorb is *curtailment* — clean
generation deliberately thrown away. Any shortfall nothing can cover
is *unserved energy* — the lights going out, counted honestly rather
than hidden.

The price each half-hour is the running cost of the most expensive
plant needed — the standard textbook rule for how wholesale
electricity markets set prices. Half-hours in which the
take-it-or-leave-it sources cover everything are priced at zero,
because the marginal unit costs nothing to run.

=== What it was checked against

The whole of 2024, against the published records of the real
system, with the tolerances set before the model ran (the Stage 1
and Stage 2 run reports hold the full tables):

- Annual gas burn: modelled 73.45 TWh against 72.79 observed —
  within 1%, against a ±5% gate.
- The month-by-month generation mix: correlation 0.997, against a
  threshold that was *tightened* after the run (a naive model that
  holds every fuel flat at its annual average already scores 0.934,
  so the bar was raised to 0.99 to stay meaningfully above zero
  skill).
- The genuinely predictive content, stated as such in the report:
  the gas fleet's monthly shape (correlation 0.995, with nothing
  about it fed in), and the fact that the system stays feasible
  half-hour by half-hour with a realistic minimum margin (about
  0.2 GW at the tightest moment of the year) and zero unserved
  energy.
- Prices: the model's median price sits within 1% of the observed
  market's; monthly price correlation 0.95; gas sets the price in
  93.9% of modelled half-hours.

The circularity is inventoried rather than hidden: demand, imports,
and the wind/solar annual totals are fed in or calibrated, and the
report lists exactly which comparisons are therefore not evidence.

=== What it leaves out — and which way that pushes the answers

- *One wedge is load-bearing and always disclosed*: 3.35 TWh of
  real but unclassifiable 2024 generation (the records call it
  "other") is fed in as an observed trace. Without it, modelled gas
  reads +5.3% and the validation gate fails. Any claim that "the
  model reproduces 2024 gas burn" carries this caveat, by rule.
- *Gas plant in the model can switch off completely; real plant
  mostly cannot.* Real gas stations have a minimum stable level and
  stay on through windy nights. The model's gas fleet hits zero in
  1,074 half-hours of 2024; the real one did in 9. Consequence: the
  model somewhat overstates the number of zero-price half-hours and
  understates how often gas sets the price.
- *No negative prices.* The real market priced below zero in 495
  half-hours of 2024; the model's floor is zero. Together with the
  model price having almost no within-day shape, this means *the
  model overstates the average price wind earns*: measured
  like-for-like, the error is +0.066 on the wind capture ratio —
  larger than the validation band, stated in the Stage 2 report,
  and the reason revenue-sensitive conclusions are not allowed to
  lean on this component alone.
- The peaking gas fleet's dispatch is essentially unmodelled (it is
  tiny in energy terms); biomass and hydro month-shapes are
  calibrated, not predicted.

=== What you can safely conclude

The engine reproduces the 2024 system's behaviour where it claims
to — the gas residual, the monthly mix, feasibility at realistic
margins — and the validation's honest content is itemised, wedge by
wedge, in the run reports. Broad price structure (when prices are
high, when they collapse to zero, roughly how often gas is the
price-setter) is usable. Fine price structure — negative prices,
within-day shape, exact revenue for a wind farm — is disclosed as
beyond it. And one sentence to carry everywhere: on this model "gas
sets the price about 94% of the time" is defensible for 2024;
neither "about 97%" nor "about 99%" is, and the report spells out
which related statements are supportable under which definition.

== Storage, and the rules for using it

#draftmark
#record("docs/notes/d4-rule-based-dispatch.md", "grid-adequacy/src/policy.rs")

=== The question this component answers

When there is surplus, what gets stored; when there is shortfall,
what gets drawn down — and by what rule? Every storage-requirement
number in the programme is downstream of this choice, which is why
it was written out in prose and adversarially reviewed *before* the
code existed, and why the code's own documentation restates the
rules in full.

=== How it works

The default policy is deliberately simple: *greedy, chronological,
and blind to the future.* Each half-hour it sees only the current
situation and how full the stores are — never tomorrow's forecast.

+ Weather-driven and take-it-or-leave-it supply is tallied against
  demand.
+ Surplus charges the stores, fastest-cycling first (battery, then
  pumped hydro, then hydrogen). Whatever no store can absorb is
  curtailed. *Stores charge from surplus only, never from gas* —
  charging from gas would be an economic bet on future scarcity,
  which a policy without prices or foresight cannot honestly make.
+ Shortfall runs the dispatchable stack first; only what the stack
  cannot cover draws the stores down. Whatever nothing can cover is
  unserved energy.
+ There is no reserve-holding: a store discharges for today's small
  deficit even if tomorrow's is fatal. Greedy means greedy.

Why so austere? Because real operators do not know next month's
weather, so a no-foresight rule gives the honest *upper envelope* of
the storage requirement; and because every smarter heuristic imports
assumptions a critic can attack. The smart alternative exists — the
optimiser of chapter 4.7, which knows the whole future — and the
rule-versus-optimiser gap is published as a finding, never tuned
away.

Two engineering details carry weight. Storage losses are split
evenly between charging and discharging (the square-root convention
— stated because published studies use different conventions and
headline store sizes can shift 15–20% between them; the run reports
convert to delivered electricity before any comparison). And the
engine enforces physics *independently of the policy*: a policy that
tried to charge a store with energy that does not exist in a deficit
period is rejected by the engine itself, not trusted — a guard
locked in by regression tests (`multizone_deficit_charge_guard.rs`
and its single-zone twin) so that no future policy, however clever,
can leak energy.

=== What it was checked against

The policy's own conservation laws, as automatic property tests:
energy in the stores is exactly conserved period by period across
all forty years with no year-boundary resets; curtailment can only
occur when every store is genuinely full or at maximum charging
power; unserved energy only when every store is empty or at maximum
output. The behavioural check is Stage 3's: on a Royal Society-style
wind-plus-hydrogen system, this policy reproduces the published
storage requirement at their supply sizing, in comparable units
(next chapter).

=== What it leaves out — and which way that pushes the answers

- *No foresight and no reserve-holding* → storage requirements are
  biased *up*: this is the designed, defensible direction (a system
  sized under this rule is not sized optimistically), and the
  optimiser bounds the bias from below.
- *No price-driven cycling.* Real batteries cycle daily on price;
  under this policy, in a system with plenty of gas, stores barely
  move. Owned consequence, stated in the decision record: under this
  policy *added storage can never displace gas*, so any question
  about storage cutting emissions must vary the fleet or use the
  optimiser — asking it of this rule alone would return a
  structural zero, not a finding.
- Degradation, minimum-charge warranties, hydrogen geology, and
  ancillary-service commitments are out of scope for the tool and
  listed as such.

=== What you can safely conclude

Storage-requirement numbers from this policy are honest upper
envelopes under a stated, reviewed, deliberately conservative rule —
strong for "at least this much is needed" claims, and never quotable
as "the optimum". Any claim about storage economics or emissions
displacement needs the optimiser or a different fleet, by the
policy's own written admission.

== Forty years at once: multi-year adequacy and the drought

#draftmark
#record("docs/notes/stage-3-storage-run-report.md", "docs/notes/stage-4-decomposition-run-report.md")

=== The question this component answers

How much storage does a weather-driven Britain actually need — not
in a typical year, but across the full recorded weather of
1985–2024, with the stores carrying their level from year to year?
And where does the requirement come from: daily cycling, bad
fortnights, seasons, or bad decades?

=== How it works

The dispatch engine simply runs on, half-hour after half-hour, for
forty years, with store levels carried across every year boundary —
no resets, because resets are exactly how "a few days of storage"
errors happen. A search routine then finds the smallest store that
gets a given system through the whole record without unmet demand.
A separate decomposition splits the requirement by timescale —
within-day, one-to-fourteen days, seasonal, and slower — by
re-running the sizing on progressively smoothed versions of the
demand-minus-renewables series; the parts sum to the total exactly,
by construction.

=== What it was checked against

The benchmark is the Royal Society's 2023 large-scale storage study
— the most prominent published answer to the same question. At
their supply sizing and in comparable units, this engine reproduces
their storage requirement: our 36.9 TWh of delivered electricity
sits inside their 33–55 band, at 84% of their central value, with
every known difference (unit conventions, demand shape, weather
window) named in the Stage 3 report. The engine also reproduces
their feasibility cliff: at 1.15× average supply *no* store size
suffices (pinned as a first-class result, consistent with their
published threshold at 1.234×) and their
binding weather window: the store's longest spell below full runs
720.6 days, from December 2009 to November 2011, exactly the
2009–2011 window their report identifies. The internal
decomposition machinery had to reproduce the headline requirement
bit-for-bit before any of its attributions were trusted.

The headline numbers are locked by regression tests
(`acceptance_stage3_rs37y.rs`): on the lean Royal-Society-comparable
fleet the requirement is 58,432 GWh; overbuilding supply to 1.64×
and 1.92× shrinks it to 28,336 and 23,872 GWh — the 23,872 figure
is the programme's most-quoted single number and is discussed again
in the boundary chapter, where it acquires an important caveat.

=== What it leaves out — and which way that pushes the answers

- *Demand is 2024's pattern, tiled across the decades* — no growth,
  and no link between cold weather and demand until the heating
  overlay (chapter 4.9) adds one. Electrified heating is *the*
  storage amplifier, so this omission biases the requirement *down*,
  and it means the agreement with the Royal Society (whose demand
  was an electrified-heat 2050 profile) likely flatters us — stated
  in the report, and quantified rather than assumed once the
  heating overlay exists.
- Store charging/discharging power is fixed at a generous level, not
  optimised; the hydrogen store is a single aggregate with no
  geology.
- *The runs behind this chapter's numbers are single-zone*: they
  were made before the simulator could divide the map, so they let
  power flow freely within Britain. The simulator itself is no
  longer so limited — and the boundary chapter re-measures this
  chapter's flagship with the Scotland–England boundary in place:
  the 23,872 GWh requirement becomes 35,648 GWh (+49%). The
  single-zone figures are kept as the quoted record because they are
  the validated like-for-like reproduction of the Royal Society's
  own single-zone study; the boundary sensitivity travels with them
  by rule.

Two findings that cut against tidy expectations are part of the
record, at equal prominence: the within-day slice of the
requirement is a quarter (25.4%), not a sliver — so batteries and
demand-shifting can target at most that quarter, and the safe
headline is that *about three-quarters of the requirement lives at
timescales slower than the daily cycle* — days to seasons, beyond
any battery's daily rhythm; and the strictly multi-year slice
attributes *zero* — the store exists because of winters and takes
years to refill after bad ones, which is not the same as
decade-scale deficits. One ranking (bad-fortnights versus seasonal)
depends on an analysis window choice and is never quoted without
it, by standing rule.

=== What you can safely conclude

Single-year studies are quantifiably wrong: a system that sails
through a benign year with a 12-hour battery fails in 33 of the 40
recorded years (557 GWh of unmet demand, pinned in the Stage 3
report), the worst single year for storage (2021) still
underestimates the forty-year requirement by 24%, and which winter
binds depends on the fleet you choose — three different decades'
winters bind at three different supply sizings. "Modelled on one
year of data" is a tell, and this component is the instrument that
makes it one. The storage numbers themselves are quotable with
their conventions (delivered-electricity units, the stated demand
shape, single-zone) attached.

== Keeping the lights at 50 Hz: the stability engine

#draftmark
#record("docs/notes/stage-6-stability-run-report.md", "docs/notes/q8-current-holdings-run-report.md")

=== The question this component answers

Adequacy asks whether there is enough energy over hours and years.
Stability asks something faster: if the largest power station on
the grid tripped off *this instant*, would the system survive the
next two minutes? The grid's frequency — 50 cycles per second when
supply and demand balance — falls when a big source is lost, and if
it falls to 48.8 Hz, automatic protection begins disconnecting
whole blocks of demand: a partial blackout by design, to save the
rest.

=== How it works

The stability engine simulates those two minutes at millisecond
resolution. The physics is the standard "swing equation": the
heavy spinning machinery on the grid stores rotational energy
(inertia), and that stored energy is what buys time after a trip —
more inertia, slower fall. Against the fall, the engine deploys the
real emergency-response services the operator buys — so many
megawatts contracted to arrive within so many seconds — with their
published volumes, delivery factors and response-time envelopes,
plus the small self-correction from demand itself falling as
frequency falls. It integrates the balance forward and reports the
lowest frequency reached, how fast it fell, and whether automatic
disconnection fired.

The link to the rest of the simulator: any half-hour of any
adequacy run can be handed to the stability engine, which computes
the spinning inertia of exactly the fleet that was running that
half-hour and asks what the loss of the largest infeed would do.

=== What it was checked against

The one full-scale natural experiment available: 9 August 2019,
when two large sources tripped within seconds and about a million
customers were disconnected. Every input — the trip sequence, the
response volumes, the protection settings — was assembled from the
official record (with each number's source cited in the event
file), and the model was then required to reproduce the measured
event within bands set *before* it ran. It does, at both of the two
official (mutually disagreeing) inertia figures: modelled lowest
frequency 48.793 Hz against the measured 48.787 — six thousandths
of a hertz — with the initial rate of fall within 2.4% of measured,
and the counterfactual gate (a 1,000 MW loss must stay above
49.5 Hz) passed. All pinned in the Stage 6 run report and its
acceptance tests.

=== What it leaves out — and which way that pushes the answers

- *The middle of the event is stylised.* The model's contracted
  response arrives faster in the 10–40-second window than the real
  2019 deployment did (first arrest at about 14 seconds versus about 25 measured).
  The validated quantities — the initial fall rate and the lowest
  point — are insensitive to this; the mid-event trace shape is
  diagnostic only and never quoted as reconstruction, by standing
  rule. A companion rule forbids ever re-tuning inputs against the
  validation gates.
- *What happens after the lowest point* (the operator's managed
  recovery) is out of scope by the event specification.
- Imports over the cables and the unclassifiable "other" generation
  carry no inertia in the model — a small understatement of 2024's
  true inertia, recorded.
- The famous companion finding needs its caveat attached wherever
  it travels: on 2024's fleet dispatched by market prices alone,
  85.5% of half-hours fall below the operator's inertia floor. That
  is *not* a claim that real Britain ran below its floor — the real
  operator pays for synchronous machines precisely to prevent it.
  It measures the gap between what the market would provide and
  what stability needs — the size of the problem someone has to pay
  to solve.

On today's response services (the Q8 record): under the volumes
actually procured for 2025, the modelled system rides through the
standard 1,800 MW loss from 2024 onward on the operator's own
future pathway — the modelled largest survivable loss in 2024 is
2,433–2,701 MW, against 1,373–1,573 under 2019-era services. The
mechanism, established by adversarial probe after the first
explanation was refuted: *the current services win on how early
their megawatts arrive, not on how many are held* — about one-third
faster per-product response, two-thirds the fleet's megawatts
moving into the fastest products. Caveats carried on every quote:
contractual rather than measured delivery factors (a stated
optimism; a 10% haircut sensitivity changes no headline), some
service classes conservatively excluded (understating the current
side), and post-2030 absolute values conditional on a demand-growth
channel — the "never lost" claim is robust to it; the 2050
absolutes are not.

=== What you can safely conclude

The engine reproduces the one measured full-scale event to within
six thousandths of a hertz on the quantity that matters, using only
cited public inputs — so "would this fleet survive its largest
loss?" is answerable, with the ride-through capability always
distinguished from an operational security standard. The zero-carbon
punchline is an output, not an assumption: an all-renewables fleet
with hydrogen turbines has zero spinning inertia in all 701,280
half-hours of the record *under the stated convention* that
hydrogen reconversion is non-rotating — a convention named on every
quote because a spinning-turbine variant would differ.

== Dividing the map: multiple zones and the Scotland–England boundary

#draftmark
#record("docs/notes/stage-5-run-report.md", "docs/notes/b6-two-zone-run-report.md", "docs/notes/b4-lp-findings.md", "docs/notes/d13-run-report.md")

=== The question this component answers

Britain's wind is disproportionately in Scotland; Britain's demand
is disproportionately in England; the wires between them have
limited capacity, and the cables to Europe mostly land in the
south. What do those geographical facts do to storage requirements,
curtailment, and exports — numbers the single-zone model computes
as if power flowed freely everywhere ("copper-plate", in the trade:
as if the country were one solid conductor)?

=== How it works

The engine can split the system into zones joined by
limited-capacity links. Three configurations matter:

- *Britain plus five European neighbours* (France; a
  Belgium–Netherlands–Germany bloc; southern Norway; western
  Denmark; the island of Ireland), each with its own fleet, demand
  and weather, joined by the real interconnectors. Flows respond to
  relative scarcity each half-hour.
- *Britain split internally* at the real transmission bottlenecks:
  first a two-zone split at the Scotland–England border (the
  boundary the industry calls B6), then — after measurement showed
  the harder constraint sits *within* Scotland — a three-zone split
  (north Scotland / south Scotland / England-and-Wales, with the
  internal boundary called B4), each link carrying the observed
  2024 half-hourly capability.
- *The composed system*: the three-zone Britain joined to the five
  neighbours — the first configuration in which northern wind must
  cross the measured internal constraints before it can reach the
  export cables.

=== What it was checked against

The European layer was validated against 2024 observation: Britain's
gas burn and monthly mix re-pass their Stage 1 gates with imports
now *modelled* rather than fed in; the model gets the direction of
flow on the French border right in 90% of half-hours (a two-part
gate whose full, unflattering history — including a first
measurement *below* the always-import base rate and the diagnosis
that fixed it — is the centrepiece of the Stage 5 report);
correlations and per-border annual energies pass their pre-set
bands; and the capacity-credit table reproduces the anticyclone
story, with Norway's hydro-backed link the only one whose delivery
holds flat when Britain is tightest. The Scottish boundary was
validated against the system operator's recorded boundary flows:
modelled flow lands between the unconstrained day-ahead figure
(22.6 TWh) and the constrained outturn (17 TWh), and the boundary
binds in 25.0% of periods against 23.6% observed. That share is a
post-fix number, and the direction is stated plainly: the R7
flow-walk repair (Part 7 register) moved it *away* from observation
— from 0.37 points below the observed share to 1.42 points above —
while staying within the pre-set ±4-point validation band.

=== What it leaves out — and which way that pushes the answers

This chapter's findings are mostly *about* what the simpler
configurations leave out, so the honest record is a sequence of
corrections — each one adversarially reviewed, several overturning
the authors' expectations:

+ *The copper-plate storage numbers are lower bounds.* The ratified
  expectation was that the internal boundary barely moves the
  flagship storage requirement. Measured, it does: 23,872 GWh
  single-zone becomes 35,648 at the two-zone 2024 boundary
  capability (+49%) and 37,824 under the three-zone split (+58%).
  The expectation was withdrawn on the record
  (`b6-two-zone-run-report.md`, correction block).
+ *But no clean "boundary effect" percentage is quotable.* An early
  attempt to state one was withdrawn when the audit showed it was
  contaminated by the simple dispatch rule's own artefacts: the
  rule-based flow logic settles internal flows in a single pass and
  cannot shuttle northern surplus through southern Scotland to
  England — even with *unlimited* wires it strands 6.9 TWh of wind
  in the north. Untangling geography from dispatch rule needed the
  optimiser.
+ *The optimiser's verdict at today's fleet:* the internal Scottish
  boundary (B4) genuinely binds in roughly 23–28% of half-hours
  under perfect dispatch, versus about 2% under the simple rule —
  the choke is real, and the simple rule was hiding it. The
  observed day-ahead figure (36%) is *not* a target the model
  should reach: it measures scheduling behaviour, not physics; the
  honest bracket is rule 2% ≪ optimiser 23–28% < scheduled 36%
  (`b4-lp-findings.md`, with five mandatory caveats).
+ *The composed measurement at 60 GW of wind* (the D13 record,
  2026-07-06) delivered the programme's most consequential
  correction. With the measured boundaries and the modelled 2024
  European system attached, the minimum possible waste — under
  *any* dispatch, however clever — is 36.2 TWh a year at 60 GW
  (system basis: all zones' curtailment plus storage and cable
  losses), of which +24.0 TWh is driven by the wind increase; on a
  conservative like-for-like basis that is at least +20.0 TWh more
  than the 4.0 TWh the no-internal-boundaries model reports at the
  same wind level. The optimistic curtailment story does not
  survive the geography. The same record settles a converse worry:
  at the 2024 fleet, attaching the external world leaves the
  boundary statistic essentially unmoved (composed B4 binding point
  0.2813 against the committed GB-in-isolation 0.2816) — the
  constraint is a property of the geometry, not of modelling
  Britain alone.
+ *And the earlier headline inverts at scale, plainly told:* at
  today's fleet, dispatch quality is the binding limit (the
  optimiser wheels far more than the rule). At 60 GW it no longer
  is — perfect foresight recovers only about 0.44 TWh of the simple
  rule's 36.67 TWh of system waste. The system becomes
  *absorption-limited*: there is nowhere for the surplus to go, and
  no cleverness of dispatch changes that. The boundaries bind in
  physics — the Scotland–England border in over a third of
  half-hours at 60 GW, on the nearly artefact-free measurement.

Standing biases, each with its direction (the full ruled list is in
the register, Part 7): boundary capability is frozen at 2024 — no
new wires are built as the fleet grows — which overstates
constraint effects; the planned northward shift of the fleet is
*not* represented, which understates them; the inputs locating wind
north of B4 push its measured binding *up*; Europe's fleets and
demand are frozen at 2024 in all high-wind runs, which flatters
British exports; and whether a 60 GW Britain is a net exporter is
formally OPEN — the composed model's simple-dispatch reading of net
imports is an artefact-conditioned floor, ruled inadmissible as
evidence of collapse, with the economic-dispatch optimiser named as
the resolver.

=== What you can safely conclude

Internal transmission is not a detail: every configuration measured
needs more storage than the copper-plate number, and the composed
minimum-waste figure is dispatch-proof — a floor no operator,
market, or algorithm could get under with 2024's wires. Quote the
single-zone flagship numbers only as lower bounds with the measured
+49–58% sensitivity attached; quote boundary-binding as bands with
their conventions; and treat the 60 GW export story as unresolved.
What you cannot conclude: any single "the boundary costs X%"
number, any composed capture-price figure (no instrument for it
exists yet, by ruling), or anything about the specific wires and
substations inside a zone — there is no network model, permanently
and by design.

== The optimiser: perfect-foresight dispatch

#draftmark
#record("docs/notes/d12-mincurtailment-decision.md", "docs/notes/d12-lp-tractability.md")

=== The question this component answers

The rule-based dispatcher of chapter 4.3 is deliberately simple and
blind to the future. So a critic can always ask: is your finding
real, or just your dispatcher being stupid? The optimiser exists to
answer exactly that. It is a mathematical optimisation (a linear
programme, solved by an off-the-shelf solver) that sees the *entire*
horizon at once — every future calm and storm — and chooses the
dispatch that minimises waste across all of it. No real operator
could ever do as well, because no real operator knows the future.

=== How it works

Every half-hour's decisions — what to store, discharge, send over
each link, and how much to waste — become variables in one huge
simultaneous problem, and the solver finds the plan that minimises
total waste: curtailment everywhere, plus storage round-trip losses,
plus cable losses (each counted so that hiding spillage inside a
store or a cable costs exactly what admitting it costs — a design
decision recorded after the cheaper alternative was caught gaming
the objective). Serving demand is enforced with overwhelming weight,
so the optimiser never trades blackouts for tidiness.

The two dispatchers *bracket the truth*. The rule-based policy is
pessimistic (no foresight); the optimiser is optimistic (perfect
foresight). Any real operator lies between. When both give the same
answer — as at 60 GW, where they agree within about 0.44 TWh of
waste — the
conclusion is dispatch-proof. When they diverge — as on
boundary-binding at today's fleet — the divergence is itself the
finding: it measures how much dispatch quality matters.

=== What it was checked against

Before any optimiser figure was trusted, the same code path had to
reproduce two known reference values: the observed day-ahead
boundary statistic recomputed from the data pack, and the
rule-based figure
re-run on the identical scenario (`acceptance_b4_lp.rs`). Its
feasibility claims are asserted (unmet demand below a millionth of
a terawatt-hour), and a provable invariant — the optimiser can
never need *more* storage than the rule — is pinned as a test.

=== What it leaves out — and which way that pushes the answers

- *Where the optimiser is indifferent, its answer is arbitrary.* If
  spilling a gigawatt-hour north or south of a boundary costs the
  objective the same, the solver simply picks one of the tied
  answers — always the same one on the same machine, but its
  choice, not physics. That is why every
  optimiser boundary statistic is quoted as a *band*: the point
  (everything the solver reported) down to a floor (with the
  identified arbitrary class removed) — and the floor itself is a
  bound on the *identified* artefact class, not a certified
  minimum, by its own decision record.
- *It has no prices.* The waste-minimising objective knows nothing
  of costs, so its gas, trade and revenue figures are meaningless
  accidents of that same arbitrary choice — at one point it
  "burned" 160 TWh of gas on paper simply because no cost term told
  it which of two equally free options to prefer. These aggregates are ruled non-quotable, permanently;
  a costed optimiser (an economic-dispatch objective) is the named,
  currently unscheduled successor.
- *It is a central planner*: no market institutions, no unit
  commitment, no reserve. That is the point — it is the optimistic
  bound, which is also why observed scheduling can bind a boundary
  more often than it does.
- *It cannot span the full forty years in one solve.* Measured on
  this machine: one year solves in a minute; five years in ten
  minutes; ten years crashes the solver outright (about 24 minutes, 5 GB,
  then an internal failure), with forty extrapolating to hours and
  about 22 GB. The tractability note records the measurements and a
  reviewed rolling-window design for multi-year optimiser sizing;
  the machinery is deliberately *not built* until that design is
  approved. Current optimiser results are whole-2024,
  single-year measurements, which is exact for the questions asked
  of them.

=== What you can safely conclude

Use the optimiser for what it is: the too-good-to-be-true bound.
"Even a perfect operator could not avoid this waste" — that is its
sentence, and at 60 GW it is the load-bearing sentence of the
boundary chapter. Never quote its gas, trade or price numbers;
never quote a single point where the record gives a band; and treat
"the optimiser said X" as meaningful only next to "and the rule
said Y", because the pair is the instrument.

== Priced dispatch across borders: a null result, honestly told

#draftmark
#record("docs/notes/d11-priced-dispatch.md", "docs/notes/d11-a2a-mismatch-characterisation.md", "docs/notes/d11-sweep-run-report.md")

=== The question this component answers

The multi-zone engine's cross-border flows follow relative
*scarcity* — which side needs the power more — not relative *price*.
That is knowingly wrong where prices diverge between countries: on
2024 carbon rules Britain taxes carbon on top of its own emissions
price, so British gas power should often be dearer than French, and
flows should notice. Would replacing the scarcity signal with a
genuine price signal fix the residual errors in flow direction?

=== How it works

The priced version was designed, adversarially reviewed, and built:
each zone's signal becomes the running cost of its marginal plant
(price first, scarcity retained as the tie-break where prices are
equal — so the validated behaviour is preserved wherever prices
carry no information). The design pre-registered its own success
criterion: the France-border direction match should rise from 90%
toward an expected 97.4%, and *a miss would be published as a
finding, not patched*.

=== What it was checked against

The same observed 2024 border-direction record as Stage 5 — and it
missed, for a reason that is itself a finding about 2024. The
carbon-price gap the design leaned on turned out to be nil that
year: Britain's effective carbon price came to £55.18 per tonne
against Europe's £55.01 — a wedge of 17 pence, because the UK's
lower emissions price plus its £18 top-up tax almost exactly equals
the EU's price. 2024 is a carbon-parity year. Of the remaining
direction errors, 86.5% occur in half-hours where gas is on the
margin *on both sides at once* — where the price signal has almost
nothing to say and the outcome hangs on sub-noise conventions (the
measured match is 71.7% under the committed price series and 93.2%
under an equally defensible flat one; a static ceiling of 93.8%
bounds what any such fix could reach). The pre-registered target was
unreachable on 2024 prices; the miss is pinned exactly as the
design's rules required (`docs/08`, D11 row).

=== What it leaves out — and which way that pushes the answers

The priced ladder also fails the established validation gates on
the 2024 reference year, so by reviewer ruling it is a *named sensitivity*,
never the headline: every central estimate continues to run the
validated scarcity rule. What the tier-2 work delivered instead is
the machinery for high-wind sweeps with *responsive* imports — and
a genuine finding: at 60 GW of wind, letting Europe respond flips
Britain to a net exporter, with curtailment (4.0 TWh) *below* and
wind's earnings *above* the entire bracket the frozen-import
convention had produced. That finding carries five mandatory
caveats, of which the sharpest — the sweep's Britain had no
internal boundaries — has since been partially resolved *against*
its optimistic curtailment level by the composed measurement of the
boundary chapter; its capture and trade components remain open.

=== What you can safely conclude

On 2024 prices, a price-based flow signal cannot beat the scarcity
rule — not because the idea is wrong but because 2024 gave it
nothing to work with, and the honest ceiling was measured rather
than asserted. The null is load-bearing: it is why the programme's
quoted flow numbers still rest on the scarcity rule, and why a year
with a real carbon wedge (2022–23, or post-linkage) is the named
condition under which the ladder would earn another test. And the
episode is the trust argument of Part 6 in miniature: the target
was pre-registered, the miss was published at full prominence, and
nothing was tuned until the number cooperated.

== Electrified heating

#draftmark
#record("docs/notes/q5-heating-mix-run-report.md", "docs/notes/d9-heating-overlay.md")

=== The question this component answers

Britain intends to move much of its building heat from gas boilers
onto the electricity grid. Heat is the grid's worst possible
customer: demand rises exactly when it is cold, and the cold, still
anticyclonic week is also when wind output collapses and heat pumps
lose efficiency. What does electrified heating do to peak demand
and to the storage requirement — and does the answer depend on
*which* heating technology carries the load?

=== How it works

An overlay converts outdoor temperature (from the same forty-year
weather record) into heating demand: 410.5 TWh a year of delivered
building heat (the reviewed GB figure), an electrified share, and a
technology portfolio — air-source heat pumps, ground-source heat
pumps, and district heating from geothermal-class sources. Each
technology's electricity draw is its share of the heat divided by
its efficiency (its "coefficient of performance"), and the
efficiencies are *temperature-dependent*: air-source efficiency
falls with the outdoor air temperature, ground-source with the much
steadier ground temperature, so the model captures the vicious
correlation — more heat needed and less efficiency delivered in the
same cold hours. Cold years genuinely draw more heat (intensity is
pinned, not renormalised per year — a reviewed decision, because
renormalising would erase exactly the bad-year signal the tool
exists to see).

=== What it was checked against

The heat-pump efficiency curves come from the published When2Heat
parameterisations and were then forced to face the field: the
model's implied seasonal performance was checked against the GB
field-trial (RHPP) measured bands, at a stated system boundary, and
where it came out too optimistic a single derating factor per
technology was applied and disclosed — air-source ×0.823,
ground-source ×0.732 (`q5-heating-data-report.md`). The overlay's
energy accounting closes exactly, and the sweep's starting point
reproduces the committed no-heating baseline bit-for-bit before any
heated number is trusted.

=== What it leaves out — and which way that pushes the answers

- *No behavioural profile*: real households batch their heating
  into morning and evening; the overlay spreads it with the
  temperature. This *understates* the peaks, and therefore
  understates every technology difference in the same direction —
  so all the deltas below are lower bounds, by standing rule.
- Heat intensity is climate-stationary (no future efficiency gains
  or building retrofit), non-heating demand stays 2024-shaped, and
  the storage figures are computed at 200 GW of store power because
  the committed 100 GW rating is *infeasible* under heating — a
  pinned finding, reported rather than silently bumped.

=== What you can safely conclude

Electrifying half of Britain's building heat with air-source heat
pumps nearly doubles the forty-year storage requirement — 23,872 to
43,488 GWh (×1.82) — and adds 23 GW to the peak; ground-source
improves efficiency but not the correlation with cold calms (×1.73,
nearly the same peak); district geothermal heat, which draws almost
nothing from the grid at the peak, holds the increase to ×1.08. All
pinned in the Q5 run report across a 66-point portfolio sweep. The
added requirement loads the *seasonal* band hardest — precisely the
storage class with no cheap solution — and the benefit of moving
load off-grid is front-loaded: the steepest part of the gradient is
at Britain's actual starting point of near-zero district share.
What you may not do: quote a single averaged gradient (the curve
has a knee; both limbs or nothing), net the curtailment effects
into one number (heating also *absorbs* surplus wind; both sides
are quoted in physical units), or read any of it as a costed
verdict — the money question belongs to the cost stack, next.

== What it costs

#draftmark
#record("docs/notes/d8-lcoe-methods.md")

_The accounting rules below are adopted and binding (decision D8,
adversarially reviewed 2026-07-03). The first costed record — the
four published-pathway scenarios — now exists, pinned and reviewed
(`docs/notes/stage7-run-report.md`); its figures appear below under
their mandatory conventions._

=== The question this component answers

What does each version of the future system cost, per unit of
electricity actually delivered to the people paying for it — with
the backup, storage, spare capacity and wires *included*, rather
than quoted as someone else's problem?

=== How it works — the adopted rules

The method was pinned in prose before any cost code was written,
the same discipline as the storage rules. The rules that will
govern every published figure:

+ *The headline is delivered system cost*: total annualised system
  cost divided by energy actually delivered to demand — not
  generation, not potential output. The commonly quoted plant-gate
  measure (LCOE — a station's lifetime cost divided by its own
  output) appears only as a labelled bridge to other people's
  numbers, never as a headline.
+ *The parts must sum to the whole, exactly*, as an automatic
  acceptance test — and the reconciliation must recompute the parts
  independently, or it proves nothing.
+ *Equal-reliability comparisons only.* Comparing a reliable
  system's cost with an unreliable one's is the classic failure of
  cost arguments and is forbidden outright; fleets that miss the
  standard get a stated "make-good" addition before comparison.
+ *Every headline at three financing rates*, because
  capital-intensive portfolios re-rank as the cost of capital
  moves; a single-rate quote is a publication-rule violation.
+ *"The cost of X" means scenario differencing*: the difference in
  total system cost between two systems differing in X, both
  re-solved to the same reliability. No formula ever attributes
  shared system costs to a single technology — multiple defensible
  conventions give materially different answers, so none is
  published as a finding.
+ *Boundary conventions stated, not defaulted*: import pricing,
  sunk-versus-rebuild treatment of the existing fleet, and the
  price base year are stamped into every artefact.

Two standing disclosures already on the record: the zero-unserved
reliability standard used here is *stricter* than the official GB
standard, and must be named when comparing with official cost
claims; and modelled import costs understate real import costs in
surplus periods because the model has no negative or scarcity
prices — direction stated wherever import costs appear.

=== What it was checked against

The cost input data (capital costs, lifetimes, fuel and carbon
prices, financing rates) was assembled with a citation per number
and adversarially reviewed before any cost code consumed it
(`docs/notes/stage7-cost-inputs-report.md`); the
parts-must-sum-to-the-whole rule runs as an automatic acceptance
test whose reconciliation recomputes the parts independently; and
the Stage 7 acceptance runs have now been made — every figure in
the pathway record is pinned exactly
(`acceptance_stage7_pathways.rs`, with the battery rows' data
quarantine lifted only as a reviewed act against the primary
source, and their staleness caveats still travelling).

=== What it leaves out — and which way that pushes the answers

The pathway record's five conventions travel on every quote, each
with its direction: the single observed 2024 weather year and 2024
demand shape (2024 was not a stress year; direction unknown);
*autarky* — interconnection excluded from dispatch entirely, which
counts against adequacy (some unserved energy imports might have
served); no outage model — flat availability, which flatters
adequacy, so the nonzero-unserved finding survives it *a fortiori*;
no electrolysis flexibility (each source's own published demand
basis — and the FES-vs-CCC demand bases differ materially, so
headline demands are never compared without the stated wedge); and
no electrification reprofiling of the demand shape, which
understates winter-evening peakiness and again flatters adequacy.
Two entries carried from D8 remain: transmission constraint costs
enter as a labelled approximation keyed to Scottish wind output
(there is no network model, permanently); and any revenue-based
bridge inherits the pricing chapter's disclosed weaknesses (no
negative prices, thin within-day shape).

=== What you can safely conclude

The costed record covers the four published pathway fleets — the
system operator's and the Climate Change Committee's own planning
scenarios — run as-is under the 2024 weather year. Each figure is
quoted with its unserved energy adjacent, as the rules require, at
the three financing rates (4.5 / 7.5 / 10% real):

- *FES Electric Engagement 2035*: £84.90 / 103.78 / 121.04 per MWh
  delivered, with *1.72 TWh unserved* (0.38% of demand).
- *FES Electric Engagement 2050*: £71.32 / 88.85 / 104.79, with
  *0.87 TWh unserved* (0.11%).
- *CCC Balanced Pathway 2035*: £85.45 / 104.59 / 122.09, with
  *0.02 TWh unserved* (0.005%).
- *CCC Balanced Pathway 2050*: £82.20 / 101.62 / 119.35, with
  *5.25 TWh unserved* (0.76%).

The finding — pre-registered in shape, no fleet tuned to pass — is
that every published pathway fleet shows nonzero unserved energy
under the 2024 weather year on the record's declared conventions;
the honest sentence is always "the pathway fleet shows X under
these conventions", never "the pathway fails". And the
comparison-refusal rule bites immediately: because the four fleets
miss reliability by *different* amounts, their £/MWh figures price
different reliability levels, and ranking them against each other —
or against any zero-unserved system — is refused outright (D8 rule
3(c)). Cost *comparisons* wait for the reliability make-good
variants (fleets augmented to a common adequacy standard), which
remain genuinely future work.

= Using it

Everything is driven from one command-line program, `grid-cli`, and
every operation follows the same shape: *point it at a scenario,
name an output folder, and read what appears there.*

== Describing a system: the scenario file

A scenario is a single text file saying what is built. It is meant
to be read, and edited, by a person. An extract:

```toml
[[zones.fleet]]
name = "offshore_wind"
capacity_gw = 14.7

[[zones.storage]]
name = "hydrogen"
power_gw = 100.0
energy_gwh = 100000.0
```

The scenarios that ship with the simulator are the reference points:
`gb-2024-reference.toml` is Britain as it stood in 2024, validated
against that year's records; the `royal-society-37y` family are the
wind-plus-solar-plus-hydrogen systems used in the storage studies.
The normal working method is to copy one and change what you want
changed: more wind, less nuclear, a bigger store, electrified
heating. Every quantity has a comment saying what it is and where
its value came from.

== Running

```sh
grid-cli run --scenario scenarios/gb-2024-reference.toml --out runs/my-run
```

That is the fundamental operation: one system, forty years (or one),
every half-hour accounted for. It takes seconds. The other commands
follow the same pattern — `sweep` repeats a run many times while
varying something (wind capacity, the heating mix) and tabulates the
results; `solve` searches for the smallest store that gets a system
through the record without unmet demand; `stability` runs the
fast-clock frequency simulation; `plot` draws charts from a
completed run.

== What comes out, and how to read it

The output folder from a run contains a handful of files, in two
kinds: *things to read* and *things to analyse*.

*Read first: `summary.toml`.* A short text file — open it in any
editor — with the headline results. From the 2024 reference run:

```toml
[results]
demand_twh = 267.7        # what the country asked for
unserved_twh = 0          # demand that could not be met: none
curtailment_twh = 0.0001  # wind/solar wasted: essentially none

[results.energy_twh]
ccgt = 73.4               # gas provided 73.4 TWh of the year
offshore_wind = 45.6
nuclear = 38.2
net_imports = 33.3
onshore_wind = 37.0
```

Ten lines tell you the shape of the year: what each source
contributed, whether the system ever failed, and how much clean
generation was thrown away. The summary also records, under
`[metadata]`, the fingerprints of every input — the part you can
ignore day to day, and the part that makes the run citable.

*Look second: the viewer.* The repository ships an interactive
viewer — `viewer/index.html` — that opens in any browser with no
installation and no network connection. Drag a run's files onto it
and you get the whole run as a picture: the generation stack with
demand drawn over it, the price, the firm-supply reliability strip
with its traffic-light colouring, and the storage levels — zoomable
from the full forty years down to a single day, with exact values
under the cursor. It displays only what the engine computed, never
its own arithmetic, so what you see is always the run itself.
Nothing is uploaded anywhere; the files never leave your machine.

For print and papers, `plot` renders static charts — the monthly
generation mix, and for storage scenarios the store's level across
the whole forty years, which is the single most informative picture
the simulator makes: the seasonal breathing, the deep multi-year
droughts, and how close the system came to empty, all in one image.

*Analyse third: `dispatch.csv`.* The full half-hourly ledger — one
row per half-hour, one column per source, plus demand, storage
levels, curtailment, unmet demand, and (for priced runs) the price.
This opens directly in Excel, R, Python, or anything else that reads
a spreadsheet; a forty-year run is about seven hundred thousand
rows, which is large for Excel but trivial for the rest. The same
table ships in a second format (Parquet) that analysis tools read
faster; the contents are identical.

Sweeps produce the same pattern one level up: a table with one *row
per system tried* — sixty-six heating mixes, eleven wind capacities
— plus a chart of the headline curve, with the assumptions written
into the file's header lines.

== The habit to build

Every output folder is self-describing: the summary states the
headline, the header lines state the assumptions and caveats, and
the metadata states exactly which inputs produced it. So the habit
is simply: *read the summary, look at the chart, and only then open
the big table if the question needs it.* Nothing requires special
software, and nothing requires remembering what a run meant — the
folder says.

= Why you can trust the numbers

A model that cannot be checked is an opinion with decimal places. The
whole design of this simulator is arranged so that its numbers can be
checked — by its authors continuously, and by a hostile stranger who
downloads it. Three mechanisms do the work.

== Tripwires on every published number

Every figure this project puts in print is protected by a small
automatic test that re-runs the exact calculation and fails, loudly,
if the answer ever changes. The storage requirement of 23,872
gigawatt-hours; the 2019 frequency low of 48.79 hertz; the wind
capture ratio of 0.94 — each has such a tripwire, and all of them run
every time any part of the code is touched. This is not a courtesy.
It is what lets the authors rebuild half the engine overnight and
still state, with certainty rather than hope, that a number quoted in
a paper last week is the number the code produces today. When you see
a figure in one of these papers, a machine somewhere is standing
guard over it.

== Fingerprints on every input

The simulator's answers depend on its input data, so the input data
is pinned too. Each data file — a weather trace, a price series, a
cost table — carries a cryptographic fingerprint, and every result
the simulator writes records the fingerprints of the exact files that
produced it. Change a single number in an input, and the fingerprint
changes, and the mismatch is visible. This is what makes a result
*traceable*: not "we used the 2024 weather data" but "we used
precisely this file, and here is the proof."

== An adversary in the loop

Every substantial piece of work in this project is checked by a
second, independent pass whose explicit job is to *break it* — to
reproduce the numbers from scratch, hunt for the error, and refuse to
approve until the work survives. This is not a rubber stamp. It is
adversarial by design, and it earns its place by catching real
mistakes before they reach print.

A few examples, all from a single week of development, will show what
that means better than any assurance:

- A calculation of wind revenue was found to have its *bias pointing
  the wrong way* — the authors believed one convention understated a
  key quantity; the checker proved it overstated it, and reversed the
  published direction of the claim.

- An account of *why* the grid's modern emergency-response services
  outperform their older counterparts was shown, by direct
  experiment, to be wrong: the improvement was not the mechanism the
  first analysis proposed. The correct mechanism — speed of response
  — was established and recorded.

- A dataset describing Scotland's wind fleet was found to overstate
  Scottish generation, in the direction that would have flattered the
  paper's own argument. It was corrected before it was used.

- Most consequentially, a long-standing *expectation* of the
  authors — that the transmission bottleneck between Scotland and
  England barely affects the national storage requirement — was put
  to the test and found false: the requirement rises substantially
  once the boundary is represented. The expectation was withdrawn,
  and the finding it became is now one of the more important results
  in the programme.

The pattern in those examples is the point. The process does not
merely confirm what the authors expected; it repeatedly overturns it,
and the overturning is recorded rather than buried. An instrument
that reliably catches its own makers' errors — including the
comfortable ones — is the only kind whose numbers deserve to be
trusted. That is the standard this simulator is held to, and the
register of limitations that follows is written in the same spirit:
everything it gets wrong, stated plainly, by the people who built it.

= The register of limitations

#draftmark

Every simplification the simulator makes, in one place, each with
the *direction* in which it biases results and the committed record
in which it was ruled. This register is *append-only*: rows are
added or amended with a dated note, never silently removed —
including limitations that have since been resolved, which stay
here with their resolution named. It is the section every paper
cites. A row here is not an apology; it is the price list of the
model's honesty, and most rows were written by the adversarial
review process of Part 6, several against the authors' own
expectations.

Reading the table: "up" and "down" name the direction the
limitation pushes the affected results; "none stated" means the
review ruled no direction claim supportable. File paths are the
committed decision and run records under `docs/notes/` (or the
architecture record `docs/02-architecture.md`).

#let reg(..rows) = table(
  columns: (1fr, 0.62fr, 0.55fr),
  align: left + top,
  inset: 6pt,
  stroke: 0.5pt + rgb("#bbbbbb"),
  table.header([*The limitation*], [*Direction of bias*], [*Where ruled*]),
  ..rows.pos(),
)

#set text(size: 10.5pt)

== Weather and input data

#reg(
  [Derived wind/solar traces contain no curtailment, no outages, no
  mid-year construction; 2024's operating pattern (including its
  curtailment) is frozen into the calibration factors.],
  [The main residual vs observation; lands on modelled gas. Traces
  carry a 2024 flavour into every scenario.],
  [`era5-cf-2024-report.md` §6],
  [Fleet locations are approximate public cluster points, not a site
  database.],
  [None stated; national aggregate only — regional readings
  unreliable.],
  [`era5-cf-2024-report.md` §6],
  [Solar validation is partly circular on the operator's embedded
  estimate; no panel-tilt model.],
  [Tilt omission understates raw yield; absorbed by the disclosed
  0.88 calibration factor.],
  [`era5-cf-2024-report.md` §6],
  [Calibration factors are fleet properties held fixed 1985–2024:
  every year is "the end-2024 fleet in that year's weather".],
  [By design; not a historical reconstruction of any year but 2024.],
  [`era5-cf-2024-report.md` §6],
)

== Dispatch and prices (single-zone)

#reg(
  [The 3.35 TWh "other"-generation wedge is fed in as an observed
  trace and is load-bearing: without it the 2024 gas gate fails
  (+5.3%).],
  [Any "reproduces 2024 gas burn" claim carries this caveat.],
  [`stage-1-2024-run-report.md` §3],
  [No minimum-stable generation: model gas reaches zero output in
  1,074 half-hours of 2024 vs 9 observed.],
  [Up on zero-price periods; down on the gas price-setting share
  (93.9% modelled vs about 99% behavioural).],
  [`stage-2-2024-run-report.md` §1],
  [No negative prices (495 real 2024 half-hours were negative) and
  almost no within-day price shape.],
  [Up on wind's capture ratio: +0.066 like-for-like error, larger
  than the gate band. Revenue-sensitive results must not lean on
  the Stage 2 pass alone.],
  [`stage-2-2024-run-report.md` §2],
  [Revenue accounting on potential vs delivered output: revenues
  identical (a theorem of the dispatch rules), but the potential
  basis dilutes the capture denominator.],
  [Potential basis overstates cannibalisation. NB the original
  claim had the direction backwards; corrected on review.],
  [`stage-2-2024-run-report.md` §5;
  `package-a-delivered-basis-review.md` §4],
  [Peaking-gas (OCGT) dispatch essentially unmodelled; biomass and
  hydro month-shapes calibrated, not predicted.],
  [Immaterial to the gas-total gate; stated.],
  [`stage-1-2024-run-report.md` §1–2],
)

== Storage and its dispatch rules

#reg(
  [Greedy, zero-foresight policy: no reserve holding, charging from
  surplus only, no price-driven cycling.],
  [Up on storage requirements — the designed honest upper envelope;
  the optimiser bounds it from below.],
  [`d4-rule-based-dispatch.md`],
  [Under the rule-based policy, added storage can never displace
  gas: its marginal emissions effect is structurally zero in mixed
  fleets.],
  [Emissions questions must vary the fleet or use the optimiser;
  asking the rule alone returns a structural zero.],
  [`d4-rule-based-dispatch.md` rule 3],
  [Round-trip efficiency split evenly between charge and discharge
  (square-root convention); store-side headline sizes can shift
  15–20% between published conventions.],
  [Convention, not bias; convert to delivered electricity before
  any cross-study comparison.],
  [`d4-rule-based-dispatch.md`;
  `stage-3-storage-run-report.md` §3],
  [No must-run category: nuclear (and any low-rung thermal) is the
  bottom of the merit order and backs down in deep surplus rather
  than running through it — the dispatch-rules prose said otherwise
  until a dated erratum corrected it.],
  [Down on curtailment and on storage charging in deep-surplus
  periods — anti-conservative for curtailment findings. Bounded
  small at the 2024 fleet (116 half-hours, 0.14 GWh curtailment);
  grows with nuclear share, so nuclear-value (Q7) runs must own it
  or use the optimiser.],
  [`comment-consistency-sweep.md` M2;
  `d4-rule-based-dispatch.md` erratum 2026-07-06],
)

== Multi-year adequacy

#reg(
  [Demand is the 2024 pattern tiled across forty years: no growth,
  no weather–demand correlation (until the heating overlay adds
  one).],
  [Down on storage requirements vs electrified-heat futures; the
  Royal Society agreement is likely flattered by it.],
  [`stage-3-storage-run-report.md` §2, §5],
  [Store charge/discharge power fixed (100 GW), un-optimised;
  hydrogen store is one aggregate with no geology.],
  [None stated; disclosed convention.],
  [`stage-3-storage-run-report.md` §5],
  [The fortnights-vs-seasons split of the storage requirement
  depends on the analysis window.],
  [Ranking flips with the window; never quoted without it. Safe:
  the within-day quarter, the inter-annual zero, and the combined
  75% slower than the daily cycle.],
  [`stage-4-decomposition-run-report.md` §1],
  [Single-year comparison runs start with full stores.],
  [Down: every single year underestimates the forty-year
  requirement (best case −24%).],
  [`stage-4-decomposition-run-report.md` §3],
)

== Stability

#reg(
  [The modelled emergency response arrives faster mid-event than
  the real 2019 deployment (arrest about 14 s vs 25 s); mid-event trace
  shape is diagnostic, not validated.],
  [None on the gated quantities (nadir, initial fall rate); never
  quote mid-phase shape as reconstruction.],
  [`stage-6-stability-run-report.md` §2, rule 4],
  [Standing no-retuning rule: calibrated inputs trace to un-gated
  observables; any future strengthening of mid-phase response that
  crosses the 49.20 Hz gate edge forces a re-derivation, never a
  retune.],
  [Guards against tuning-to-pass; binds all future stability
  work.],
  [`stage-6-stability-run-report.md` §2],
  [The 85.5%-below-the-inertia-floor figure is market-only
  dispatch: no must-run, no operator stability actions.],
  [Not a claim about operated GB; it measures the gap the operator
  pays to close.],
  [`stage-6-stability-run-report.md` §3],
  [Imports and "other" generation carry no inertia.],
  [Down 1–3 GVA·s on the 2024 inertia sum; immaterial to the
  findings.],
  [`stage-6-stability-run-report.md` §3],
  [Current response services carry contractual delivery factors
  (1.0) vs 2019's measured 0.67–1.0.],
  [Up on the current side; the 10% haircut sensitivity (about 6% effect)
  changes no headline and is always available.],
  [`q8-current-holdings-run-report.md` §5],
  [About 202 MW of static response and all unpublished-volume response
  excluded from the current side.],
  [Down on current-holdings capability — conservative.],
  [`q8-current-holdings-run-report.md` §5],
  [Linear droop sits at or above the published response knee
  curve.],
  [Up, second-order (≤0.61 MW at the boundary case).],
  [`q8-current-holdings-run-report.md` §5],
  [Post-2030 survivable-loss absolutes ride on a demand-growth →
  damping channel with a 2019 damping constant.],
  [The "standard met from 2024, never lost" claim is robust to it;
  2050 absolutes are not quotable without the caveat.],
  [`q8-current-holdings-run-report.md` §5],
)

== Zones, boundaries, and Europe

#reg(
  [No network model, permanently: at most three internal zones plus
  boundary links; constraint costs approximated by a function keyed
  to Scottish wind output.],
  [Down on constraint effects (everything inside a zone flows
  freely).],
  [`docs/02-architecture.md` ADR-12],
  [Single-zone ("copper-plate") storage headlines ignore internal
  boundaries.],
  [Down: measured +49% (two-zone) to +58% (three-zone). The 23,872
  GWh flagship is quoted as a lower bound with this sensitivity
  attached, by rule.],
  [`b6-two-zone-run-report.md` §3–4;
  `three-zone-engine-review.md`],
  [No single "boundary effect proper" percentage is quotable: the
  boundary term is entangled with the dispatch convention. An
  earlier +33–35% claim is withdrawn.],
  [None quotable; direction (more storage under a split) survives.],
  [`b6-two-zone-run-report.md` §3 correction],
  [The rule-based flow walk settles internal flows in a single pass:
  it strands northern surplus (6.9 TWh even with unlimited wires)
  and structurally cannot express export-drain across B4.],
  [Down on modelled boundary binding under the rule (2% vs 23–28%
  optimised); rule-based trade figures artefact-conditioned.],
  [`three-zone-engine-review.md`; `d13-run-report.md` §2],
  [Boundary capability frozen at 2024: no reinforcement is built as
  the fleet grows.],
  [Down on exports, up on composed curtailment — the counterweight
  to the northward-shift row.],
  [`d13-run-report.md` caveat (k)],
  [The planned northward shift of the wind fleet is not
  represented.],
  [Down: composed measurements understate the constraint effect.],
  [`d13-run-report.md` caveat (f)],
  [B4 inputs: day-ahead-only limit series; onshore capacity split
  (about +31% generation per unit north of B4); offshore commissioning
  wedge (about 19%).],
  [Up on modelled B4 binding, with no observed outturn available
  to close them.],
  [`d13-run-report.md` caveat (g); `b4-lp-findings.md` caveat 2],
  [The B5 boundary is folded into the south-Scotland zone.],
  [Down on constraint severity (lower-bound posture).],
  [`d13-run-report.md` caveat (h)],
  [The observed 35.86% day-ahead B4 binding is a *scheduled*
  position (the schedule exceeds the posted limit in a third of
  periods), not a physical-dispatch target.],
  [Never a convergence target; the honest bracket is rule 2% ≪
  optimiser 23–28% \< scheduled 36%.],
  [`b4-lp-findings.md` caveat 1],
  [The composed (8-zone) rule-based leg is not validated against
  the 2024 record on national trade axes: +4.4% gas / +18.2%
  imports vs the committed five-zone model, +27.5% vs observed
  (post-R7 values).],
  [Rule-based trade quotes are one-sided bounds only, under the
  asymmetric evidential rule; caveat carried verbatim.],
  [`d13-run-report.md` §2, caveat (l)],
  [Optimiser-leg conventions on the composed record: pumped storage
  treated as history (de-duplicated), Norwegian/French hydro as
  observed traces; the Norwegian conversion carries a measured 0.58
  TWh unserved wedge, wind-independent.],
  [Ruled to threaten no quoted number; disclosed both ways.],
  [`d13-run-report.md` §4, caveat (i)],
  [The looser of the two published lower bounds on
  boundary-binding (the record's "floor_full", which also excludes
  periods when an external zone was curtailing) deliberately
  over-excludes.],
  [A loose lower bound, never a tight physics floor; every binding
  quote names which bound it uses.],
  [`d13-run-report.md` caveat (n)],
  [External fleets, demand and prices frozen at 2024 in every
  high-wind sweep: "60 GW of GB wind in the *2024* European
  system".],
  [Up on GB capture and exports (neighbours never get long at the
  same time as GB).],
  [`d11-sweep-run-report.md` caveat (a); `d13-run-report.md`
  caveat (a)],
  [Scarcity-rule dispatch fidelity: −0.046 capture wedge at the
  2024 reference year (multi-zone 0.895 vs single-zone 0.941).],
  [Down at the reference year; strengthens the above-bracket
  direction; not
  a subtractable constant.],
  [`d11-sweep-run-report.md` caveat (b), §7],
  [R7 defect, resolved 2026-07-06: floating-point stall in the flow
  walk could truncate a flow with link headroom left. The fix moved
  the 5-zone family by ≤0.025 TWh of curtailment at 60 GW, but the
  GB-internal families moved at TWh scale (e.g. +0.62 TWh across B6
  at the two-zone anchor, +1.09 TWh unconstrained) — toward the
  observed flow anchors, every validation gate staying in-band (the
  binding-share gate moved away from observation but within band);
  fixed with the multi-zone pins re-pinned.],
  [Was against the export finding (attenuated it); pre-fix numbers
  carry the attenuation, post-fix pins do not.],
  [`docs/08-risks-and-decisions.md` R7 (RESOLVED);
  `d11-sweep-run-report.md` §6],
  [Only the pinned anchor and 60 GW points of any sweep are
  quotable.],
  [Guards against reading unpinned interpolations as results.],
  [`d11-sweep-run-report.md` caveat (d); `d13-run-report.md`
  caveat (d)],
  [Tier-2 sweep's GB had no internal boundaries (caveat (e)):
  interconnector landing points sit south of both, so exports
  implicitly wheeled northern wind for free.],
  [Up on capture/exports, down on curtailment. STATUS: curtailment
  component resolved *against* the tier-2 level by the composed
  measurement (4.0 → ≥ +20 TWh); capture and net-trade remain
  open (resolver: the economic-dispatch optimiser, unscheduled).],
  [`d11-sweep-run-report.md` §4 + amendment;
  `d13-run-report.md` §5, §7],
  [Ireland is the crudest external zone (+2.66 TWh error on a −5.18
  TWh observed export).],
  [Nothing quotable from that zone beyond the disclosed band.],
  [`stage-5-run-report.md` §1, rule 4],
  [Interconnector capacity credits: France's top-bin credit
  reflects observed 2024 French hydro; Denmark's export flip
  carries the commissioning-year caveat.],
  [2024 estimates, not general laws; caveats bound to the
  artefact.],
  [`stage-5-run-report.md` §5],
)

== The optimiser

#reg(
  [Where the waste-minimising objective is indifferent, flows are
  solver's choice, not physics; boundary statistics are quoted only
  as bands, and the floor removes only the *identified* arbitrary
  class.],
  [Point up-biased by degeneracy; true physical binding could sit
  below the floor.],
  [`d12-mincurtailment-decision.md`; `b4-lp-findings.md` caveats
  3, 5],
  [The optimiser has no prices: its gas, trade and capture
  aggregates are accidents of the solver's arbitrary choice among
  tied answers (e.g. 160 TWh of paper gas), permanently
  non-quotable.],
  [Not usable on those axes at all; economic-dispatch optimiser is
  the named, unscheduled successor (D14).],
  [`d13-run-report.md` §3.4, caveat (m); `docs/08` D14],
  [Central-planner optimum: no markets, no unit commitment, no
  reserve.],
  [The deliberate optimistic bound on physical dispatch.],
  [`d12-mincurtailment-decision.md`],
  [Full-horizon forty-year optimisation is intractable (solver
  aborts at ten years); multi-year optimiser sizing awaits the
  reviewed rolling-window build. Very large solves can abort the
  whole process — a size guard is a tracked follow-up.],
  [Current optimiser results are single-year (2024) measurements —
  exact for the questions asked of them.],
  [`d12-lp-tractability.md`],
)

== Priced dispatch (tier 2)

#reg(
  [The price-based flow signal cannot reach its pre-registered
  target on 2024 prices: a carbon-parity year (wedge +£0.17/tCO₂)
  leaves 86.5% of direction errors both-gas-marginal, where the
  answer is convention noise (71.7% vs 93.2% under two defensible
  carbon conventions; static ceiling 93.8%).],
  [The null result, published as pre-registered; the scarcity rule
  remains every central estimate.],
  [`docs/08` D11 row; `d11-a2a-mismatch-characterisation.md`],
  [The priced ladder fails the Stage 5 validation gates at the 2024
  anchor.],
  [Named sensitivity only, never a central.],
  [`d11-sweep-run-report.md` §1, §3],
)

== Heating

#reg(
  [Heat-pump efficiency curves derated to GB field-trial evidence:
  air-source ×0.823, ground-source ×0.732; derating-to-median is
  mildly generous to heat pumps.],
  [Up (slightly) on heat-pump performance; direction stated.],
  [`q5-heating-data-report.md`],
  [No behavioural heating profile (demand follows temperature, not
  household timing).],
  [Down on peaks and on every technology delta — all Q5 deltas are
  lower bounds.],
  [`q5-heating-mix-run-report.md` rule 6;
  `d9-heating-overlay.md`],
  [Heat intensity climate-stationary; non-heating demand stays
  2024-shaped; storage figures at 200 GW store power (the committed
  100 GW rating is infeasible under heating — a pinned finding,
  reported, never silently bumped).],
  [Conventions carried on every quoted heating number.],
  [`q5-heating-mix-run-report.md` rules 1, 6],
)

== Costs (Stage 7 complete; pathway cost records quotable)

#reg(
  [The zero-unserved reliability standard is stricter than the
  official GB standard (3 hours per year expected).],
  [Up on modelled system costs vs official-basis claims; the
  difference is named in any comparison.],
  [`d8-lcoe-methods.md` closing notes],
  [Import costs at modelled exporter running cost, with no negative
  or scarcity prices.],
  [Down on real import cost in surplus periods; direction stated
  wherever import costs are quoted.],
  [`d8-lcoe-methods.md` rule 8],
  [Constraint costs enter as a labelled approximation until the D6
  function form is resolved (no network model, ADR-12; deferred past
  the Stage 7 tag — carried as a named zero in the cost stack).],
  [Separately labelled line, never silently pooled.],
  [`d8-lcoe-methods.md` rule 1.6; `docs/08` D6],
  [Battery cost basis: the capacity/energy split rests on a 2018-era
  vintage source (the "staleness stamp"); the quarantine on the row
  was lifted 2026-07-06 after primary re-verification, but the stamp
  travels on every cost artefact regardless.],
  [Direction unquantified; the stamp is the disclosure.],
  [`quarantine-lift-review.md`; `costs-gb.toml` battery row],
  [Battery duration attribution: the four-hour system cost is
  attributed over the power/energy split by convention, not
  measurement.],
  [Direction unquantified; convention stated where the split is
  quoted.],
  [`stage7-cost-inputs-review.md` condition 3.ii],
)

#set text(size: 13pt)

One convention row closes the register, carried from the programme's
publication rules rather than from any single review: any figure
shared with the Subsidy Clock or quoted in real-terms money must
state which real-terms convention (price base year and deflator) it
uses — nominal and real figures are never mixed.

= The results we stand behind

_To be drafted: the current quotable numbers, each with its
conditions, in readable form._

= Glossary

_To be drafted: every internal term of art, defined once. If a word
appears in this manual's body without a definition, that is a
defect — report it._

= Technical appendices

_To be drafted, for referees: equations and algorithms; data
sources and licences; the validation record; file formats._
