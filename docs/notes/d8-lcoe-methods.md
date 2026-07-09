# D8 — System-cost and LCOE methodology (Stage 7)

**Status:** ADOPTED 2026-07-03 — supervisor draft under Richard's
standing delegation, adjudicated by the reviewer ADOPT-WITH-EDITS
(docs/notes/d8-lcoe-methods-review.md), all nine ordered edits applied
verbatim below. Written BEFORE any Stage 7 cost code (the Stage 7
opening condition in the 2026-07-03 handoff, funded by Richard the
same day). Method basis decided 2026-07-03.

**Purpose:** pin the cost-accounting rules the way D4 pinned the
dispatch rules — the most contestable choices written in prose first,
so the code implements a reviewed convention rather than an accident
of implementation. Every rule below names what it permits and what it
forbids in published claims.

## Rule 1 — The headline metric is delivered system cost

The headline cost of a scenario is **total annualised system cost
divided by delivered energy** (£/MWh delivered, real terms, pinned
price base year). Delivered energy = energy actually served to demand
(the D3 underlying-demand convention), NOT generation and NOT
potential output. The Package A ruling generalises here: metrics
carried on potential output smuggle a convention into the headline;
the delivered basis is the market- and consumer-relevant quantity.

Denominator pinned exactly: **delivered-to-demand energy = GB
underlying demand served = GB demand − GB unserved energy** over the
horizon (D3 convention). It is NOT Σ per-technology delivered
generation (the Package A "delivered-by-technology" series — a
different object; the two differ by storage losses, boundary flows,
and unattributed spill). Exports are not GB-demand service and do
not enter the denominator (their settlement is rule 8); external-
zone demand never enters it — Stage 7 costs the GB system, with the
boundary settled per rule 8. Code carries the two "delivered"
objects under distinct names.

Component scope of "total annualised system cost" (the pinned list —
additions require a D8 amendment):
1. Generation capex, annualised (rule 4) + fixed O&M, per technology.
2. Variable O&M + fuel + carbon (the Stage 2 SRMC chain:
   HHV-consistent, `data/reference/prices-2024.toml` precedent).
3. Storage capex + O&M, annualised (power and energy components
   priced separately — a hydrogen store's £/kWh and a battery's are
   different objects; per-leg efficiency per the D4 √η convention).
4. Interconnection: capex of modelled links + a stated treatment of
   import/export settlement (rule 8).
5. Stability services: the Stage 6 response-holdings machinery priced
   at cited holding costs (Q8 linkage).
6. Constraint costs per ADR-12 (function form = D6, still open;
   until D6 resolves, constraint costs are reported as a separately
   labelled line from the B6 approximation, never silently pooled).
7. Unserved energy is NOT priced into the headline (no VoLL monetising
   of blackouts into a £/MWh average) — rule 3 handles reliability by
   construction instead.

Curtailment and overbuild carry NO separate cost line: their cost IS
the capex/O&M of the capacity that produced them, which the headline
already contains. Reporting them as separate £ lines double-counts —
they appear as reported *quantities* (TWh, ratios) alongside costs.
This ruling interprets docs/04 Stage 7's scope list ("… storage
capex, overbuild, curtailment, interconnection …") as satisfied by
(i) the capex/O&M embodiment in lines 1/3, (ii) the reported
quantities, and (iii) named terms in the rule-6a identity — not as
separate £ lines. docs/04 is not edited; the interpretation is
recorded here and binds the "Σ components = total" acceptance test
to the rule-1 line list.

## Rule 2 — Additive decomposition, exactly

Σ(component lines) = total, bit-exact under the engine's determinism,
pinned as an acceptance test (the Stage 4 precedent: attribution
machinery must reproduce the total before being trusted). Any
component that cannot be made additive (interactions, shared assets)
is reported inside a named line with its allocation convention stated,
never spread across lines by formula.

## Rule 3 — Equal-reliability comparisons only

Cost comparisons between scenarios are published ONLY at equal
reliability: each compared scenario must first be solved to the same
adequacy standard on the same weather record (default: zero unserved
energy over the full 1985–2024 record, the Stage 3 convention; any
other standard must be stated with the number). Comparing a reliable
system's cost with an unreliable one's is the classic LCOE-argument
failure and is forbidden outright — there is no caveat wording that
rescues it.

**Fixed-fleet scenarios (the FES/CCC pathway pack, docs/04 Stage 7
scope):** these cannot be re-solved to the standard without changing
the fleet being costed. Convention: (a) every cost artefact stamps
the scenario's unserved energy and the adequacy standard it was
solved to — zero-unserved artefacts stamp that fact too; (b) a
fixed-fleet scenario with unserved energy above the stated standard
is published only with its unserved TWh adjacent to the £/MWh
figure, and is excluded from rule-3 comparisons; (c) for
comparisons, a **reliability make-good** variant is constructed —
the minimum addition of a stated resource by a stated solver (Stage
3 bisection precedent) that reaches the standard — and the make-good
appears as a named rule-1 line. Equal-reliability comparisons use
the make-good variants.

## Rule 4 — Annualisation and WACC sensitivity, prominent

Capital recovery factor per technology: CRF = r(1+r)^n / ((1+r)^n − 1)
with cited asset lives, real WACC r. **Every headline cost is quoted
at three WACCs** (low/central/high — the exact set is a TBD-DATA pin
from cited GB evidence, not this note's job), because capital-intensive
portfolios re-rank with r and hiding that in a single central number
is the second classic failure. A single-WACC quote of any Stage 7
number is a publication-rule violation.

The three-WACC set is applied **uniformly across technologies** by
default — per-technology WACCs embed risk-premium assumptions that
re-rank portfolios by fiat. A per-tech-WACC variant is a labelled
sensitivity requiring cited per-tech rates from the evidence
package, never the headline.

## Rule 5 — Marginal claims by scenario differencing only

"The cost of X" for any technology or asset class means EXACTLY:
the difference in total system cost between a scenario pair that
differ in X, with all other free resources re-solved to the rule-3
standard by the same stated procedure at both endpoints, both
endpoints stated. The difference is the system cost of X *inclusive
of its balancing consequences* — that inclusiveness is the claim's
content, not a confound. A sweep is a chain of such pairs: marginal
curves read off Stage 4 sweep surfaces (Q2, Q3) are rule-5
compliant, with the sweep axis and the re-solve procedure stated.
(Q3 additionally inherits the D4 policy conditioning named in the
interactions section: under RuleBased, storage never displaces gas.)
No formula-based attribution of shared system costs to individual
technologies is published as a finding. This is
the Stage 4 lesson (attribution is window-sensitive) and the Package A
lesson (convention choice can invert a sign) applied to money.

## Rule 6 — Literature bridges are bridges, not headlines

Computed and reported for comparability, each labelled as a bridge:
- **Per-tech LCOE** (standard plant-gate £/MWh) — reported because
  every reader expects it; always adjacent to the system-cost number
  that supersedes it (Q9's decomposition of the gap between them).
- **Hirth value factors** — our delivered-basis capture ratios are
  exactly this (Package A alignment); reported per technology.
- **LFSCOE** (Idel-style levelized full system cost) — computed as a
  robustness comparable where cheap to do so.
None of these leads a paper, an abstract, or a chart title. The
headline is always rule 1's delivered system cost.

Bridge caveat, carried from the Stage 2 record: model SMP has almost
no within-day shape and no negative prices, and the Stage 2 capture
gate passed on thin validation content (stage-2 run report §2 —
Stage 7 capture-price economics must not lean on that pass alone).
Value-factor and capture-price bridges carry this caveat until the
priced ladder re-validates capture.

## Rule 6a — The Q9 gap decomposition is an identity, not an attribution

The docs/04 Stage 7 acceptance test "LCOE vs delivered £/MWh gap
fully decomposed (Q9)" is implemented as a **system-level accounting
identity**, exact under rule 2 — never as an attribution of shared
costs to a single technology (which rules 5 and 7 forbid).
Construction: the gap between the generation-weighted mean of
per-tech plant-gate LCOEs (weighting basis stated) and rule 1's
delivered system cost decomposes exactly into three wedges:
1. **Denominator wedge** — generation vs delivered-to-demand energy
   (curtailment, storage round-trip losses, boundary flows per
   rule 8);
2. **Missing-line wedge** — rule 1 lines absent from plant-gate
   LCOE (storage, interconnection, stability services, constraint
   costs);
3. **Utilisation wedge** — realised capacity factors vs the CF
   assumptions inside the per-tech LCOE figures.
The identity terms are the normative objects. docs/07 Q9's labels
(backup, balancing, curtailment, transmission, stability) are
presentational groupings of those terms; every Q9 artefact states
the exact term-to-label mapping, and "transmission" in this model
means constraint costs + interconnection only (no network model,
ADR-12 — stated on the chart). Per-technology decompositions of the
gap are published only as rule-5 scenario differences.

## Rule 7 — No Ueckerdt-style integration-cost attribution headline

We do not publish "integration costs of wind = £X/MWh" decompositions.
Rationale: the attribution of system-level interaction costs to a
single technology is convention-dominated (multiple defensible
conventions, materially different answers — the same class of problem
as the potential/delivered inversion, at whole-system scale). The
information those decompositions try to carry is delivered instead by
rule 5 scenario differences, which have an operational meaning.

## Rule 8 — Boundary conventions (stated, not defaulted)

- **Imports/exports settlement:** modelled import energy is costed at
  the modelled exporter-zone SRMC **once multi-zone SRMC exists (the
  Stage 7 priced ladder — pricing is single-zone-only today, a
  recorded code fact)**, else at a cited reference price; exports are
  credited under the same convention imports are costed, symmetrically
  and stated (a no-credit variant is a legitimate conservative
  sensitivity, labelled); the model's £0 must-take floor and missing
  negative/scarcity prices mean modelled exporter SRMC systematically
  UNDERSTATES real import cost in surplus periods — direction stated
  wherever import costs are quoted; the convention used is stamped
  into every cost artefact. The frozen-imports deviation and its
  bracket (Package B) propagate: high-wind cost sweeps inherit the
  import-convention bracket obligation.
- **Pathway years:** the cost of pathway year Y is the annualised
  snapshot — every asset alive in Y carries its rule-4 annuity
  (assets added during the pathway included from their build year)
  plus year-Y operating costs. No discounted pathway-total (NPV)
  headline; a pathway NPV may appear only as a labelled derived
  figure with its discount rate stated.
- **Existing fleet:** sunk capex is NOT annualised into pathway costs
  for already-built assets unless the scenario explicitly models
  rebuild/life-extension — the scenario file must say which; both
  "system rebuild cost" (greenfield) and "forward cost" (sunk excluded)
  are legitimate framings and every artefact names which one it is.
- **Price base and currency:** single pinned base year (TBD-DATA),
  GDP-deflator series cited; nominal figures never mixed in.

## Rule 9 — Determinism, pins, and data discipline (unchanged law)

Cost inputs live in a versioned, cited reference TOML
(`prices-2024.toml` precedent: per-number citations, licence-clean
sources only); every published cost number gets a pinned regression
test before it is quoted anywhere; cost artefacts embed the full hash
set per ADR-5. The emissions-factor gap (biomass/coal/"other" — a
tracked deviation) must be closed by the cost-inputs evidence package
before any emissions-priced cost line ships.

## Interactions with open items (named so the reviewer can check)

- **D6 (constraint-cost function form):** open; rule 1.6 quarantines
  it into a labelled line so Stage 7 need not block on it.
- **D4 policy contract:** the LP (`PerfectForesight`) policy requires
  the engine-level D4 rule-2/3 checks relaxed to a policy contract
  (tracked item) — Stage 7 design work, not a cost-methods question,
  but the rule-based-vs-LP cost gap it produces is a REPORTED FINDING
  (ADR-6), not an error to be tuned away.
- **Q3 limitation:** under RuleBased, storage never displaces gas —
  cost results conditioned on policy must say which policy.
- **Q8/Q9:** rule 1.5 prices the holdings Q8 varies; Q9 is rule 6's
  LCOE-vs-system-cost gap decomposition, already an acceptance test
  in docs/04 Stage 7.

## What this note does NOT do

It pins no numbers. The TBD-DATA pins (WACC set, asset lives, capex/
O&M sources — stating the overnight-vs-IDC basis, price base year,
storage cost splits, holding costs, and pathway fuel/carbon price
trajectories under the opponent's-defaults discipline) are
the next Stage 7 package: a data-engineer evidence note with cited GB
sources, reviewer-gated, then written into docs/04 Stage 7 as numeric
tolerances/values — the D2 pattern. Two further reviewer notes of
record bind that package and the implementation: the zero-unserved
default is STRICTER than the GB reliability standard (LOLE 3 h/yr) —
name the difference when comparing with official cost claims; and the
rule-2 reconciliation acceptance test must recompute component lines
independently of the total, or it is a tautology.
