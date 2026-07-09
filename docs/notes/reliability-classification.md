# Reliability classification — firm vs variable generation (adopted 2026-07-02)

Owner-requested first-class feature: the simulator carries the
reliable/unreliable generation distinction from Richard Lyon's
gb-grid-margin analysis (gridmargin.co.uk), implemented as *his published
cut*, verbatim — not an "improved" variant. Source methodology extracted
2026-07-02 from `[local path]` (file references
below are to that repo).

## The classification

**Binary, plus one deliberate exclusion. The criterion is correlated
failure under a synoptic weather event** — not dispatchability per se,
and not a capacity-credit number:

> "The reliability cut: firm, dispatchable, weather-independent
> generation Britain can call on (gas + nuclear + biomass + other firm
> fuels) vs the sources that fall away together in a synoptic calm
> (wind + solar + imports)." — `engine/grid_engine.py:211-216`

| Bucket | Members | Basis |
|---|---|---|
| **firm** (RELIABLE) | gas CCGT/OCGT, nuclear, biomass, hydro, coal, oil, "other" | dispatchable, weather-independent |
| **variable** (UNRELIABLE, "weather & imports") | wind (transmission + embedded), solar, **interconnector imports** | fall away together in a blocking high |
| **excluded** | pumped storage | pumping is demand; PS supply excluded from both buckets |

Imports are variable *deliberately* (`methodology.html:260-273`): wind is
correlated over ~1,000–2,000 km, so a winter blocking high becalms
Britain and its neighbours at once; in exactly those hours every
connected market is short and the interconnectors run flat or reverse —
"imports fail precisely when they are needed."

Biomass is firm on the same single test — "*can you call on it when the
wind dies?*" — with the subsidy/carbon case against it kept explicitly
in its own lane (`methodology.html:274-284`).

**No derating anywhere.** The classification is a pure bucket flag;
gb-grid-margin's notes explicitly forbid derated capacity figures in its
metrics (`NOTES.md §2`).

## The metric

**Firm share of demand, per half-hour period**: `firm_share =
firm_supply / demand`, **unclamped** (net-export periods legitimately
exceed 1.0 — matching the gb-grid-margin year-series convention,
`NOTES.md §12`). Presentation threshold: the source site arms
"UNRELIABLE" when firm share < 50 %.

The lay framing the outputs should honour (`methodology.html:285-293`):
"firmness, not this instant" — variable megawatts are real power when
flowing; the classification is a statement about what is *guaranteed
when the continent is also becalmed*.

## Simulator implementation

- Fleet entries: optional `reliability = "firm" | "variable"`, with the
  default **derived** (capacity-factor trace present ⇒ variable, absent
  ⇒ firm — which reproduces the roster above exactly for the modelled
  technology set). An explicit value overrides; overrides are legal (the
  classification is a contestable modelling assertion made visible in
  the scenario file) and always surfaced in outputs.
- Exogenous supply entries: **required explicit**
  `reliability = "firm" | "variable" | "excluded"` (reference scenario:
  net imports → variable, FUELHH "other" → firm, pumped-storage net →
  excluded).
- Outputs: per-period `firm_supply_gw`, `variable_supply_gw`,
  storage-discharge aggregate, `firm_share`; run summaries carry annual
  mean/min/P25 firm share and the count of periods below 0.5.
- Classification is pure accounting: it must not and does not change
  dispatch or pricing behaviour (asserted by test; physical pins
  unchanged).

## The one extension beyond the source analysis, flagged not fudged

gb-grid-margin has **no storage category** (batteries are not an Elexon
fuel code; PS is excluded). The simulator must say something once
storage dispatches (Stage 3+), so **storage discharge is reported as its
own fourth category — never silently folded into firm**. Whether
storage-backed supply counts as reliable (and under what duration
assumptions) is left visibly open for the owner to rule on; it is a
substantive question for the book, not a schema default.

## What this enables

gridmargin.co.uk computes the firm share for the *present* grid from
live data. The simulator computes the same metric for *any* scenario —
"here is the 2035 FES fleet's firm share, hour by hour, against 1985's
weather" — extending the site's central chart to counterfactual and
future fleets, with the same definition and therefore direct
comparability.
