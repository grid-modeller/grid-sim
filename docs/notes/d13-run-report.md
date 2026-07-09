# D13 composed boundary-trade — 60 GW run report (the finding record)

**Status:** measured record, 2026-07-06, reviewer-gated
(ACCEPT-WITH-CONDITIONS, **Branch A adjudicated** — the 2026-07-06
addendum in `docs/notes/d13-composed-boundary-trade-review.md`, rulings
A–F; every load-bearing number below reproduced by the adjudicator's
independent probe: independent scaling, LP surgery, waste/mask/binding
arithmetic, and an independent net-trade recipe). Work order: the
ADOPTED design `docs/notes/d13-composed-boundary-trade.md` as re-scoped
by the package-1 adjudication (ruling C instrument set). Every number
in this note is pinned in `grid-adequacy/tests/acceptance_d13_60gw.rs`
(the 60 GW record + the anchor LP waste baseline) or
`grid-adequacy/tests/acceptance_d13_composed.rs` (the package-1 anchor
record). **Read §5 before quoting anything — the quotable sentences and
their mandatory caveat sets are binding, and EVERY quoted number
carries its basis label (*GB curtailment* or *system waste*, ruling
B).** No composed capture exists on any leg at any point (ruling D).

> **CORRECTION (2026-07-06, R7 flow-walk stall fix — docs/08 R7,
> RESOLVED; adjudication `docs/notes/r7-fix-review.md`).** The
> caveat-(c) stall is fixed; the RULE-BASED leg of this record moved
> (re-pinned with old values recorded per pin in
> `acceptance_d13_60gw.rs` / `acceptance_d13_composed.rs` /
> `regression_8zone.rs`). Re-measured on the fixed engine:
> **§2 anchor (rule-based)** — GB gas 75.019 → **74.960 TWh** (+4.49 %
> → +4.41 % vs the pre-fix committed anchor); net imports +42.428 →
> **+42.473 TWh** (+18.1 % → +18.2 %; +27.4 % → +27.5 % vs observed);
> curtailment 7.466 → **7.452 TWh**; unserved unmoved
> (1.355087867608 GWh); B4 binding 185 → **187**/17,277 (0.010708 →
> 0.010824); B6 662 → **671**/17,211 (0.038464 → 0.038987); the
> committed 3-zone comparators moved with their own re-pins (B4
> 0.019506 → 0.020085, i.e. 337 → 347/17,277; B6 0.033525 unmoved).
> **§3.1 60 GW (rule-based)** — GB curtailment ceiling 30.175 →
> **29.910 TWh**; net imports +11.869 → **+11.702 TWh**; gas 46.874 →
> **46.769 TWh**; gross exports/imports 23.859/35.727 →
> **24.048/35.750 TWh** (per-link gross energies and saturation
> counts move with the re-pinned table — old counts recorded at the
> pin); zonal split NSCO/SSCO 24.917/5.114 → **24.898/4.869 TWh**
> (RGB unmoved); B4/B6 southward 5.507/21.176 → **5.521/21.504 TWh**;
> B4/B6 binding 1,706/4,769 → **1,718/5,161**; all-zone unserved
> 207.926 → **207.684 GWh**; all-zone curtailment / system-waste
> analogue 35.520/36.929 → **35.253/36.666 TWh**.
> **§3.2/§4 derived arithmetic** — the conservative deconfounded
> exceedance +20.020 → **+20.044 TWh** (the copper-plate comparator
> moved to 3.982736889304 with its own re-pin; the LP side is
> unchanged); the §4 secondary sentence's perfect-foresight recovery
> ~0.70 → **~0.44 TWh** of 36.67 (36.666 − 36.224 — the
> absorption-limited reading strengthens). The §5 quote-block figures
> read +11.70 for +11.87, 29.91 for 30.17, 36.67 for 36.93.
> **What did NOT move:** the entire LP leg (every §3.2/§3.3 value,
> bit-identical — the fix touches only the rule-based walk), the
> 8(i)/8(ii) RED verdict shapes (robust under both the pre-registered
> pre-fix comparators and the post-fix record), the PS-inertness and
> conservation asserts, and both ruled quote frames (one-sided
> bounds; export survival OPEN).

## 1. What was run, and the conventions

Both dispatch legs at the pinned 60 GW point on the composed scenario
`scenarios/gb-2024-8zone.toml` — the committed 3-zone GB boundary
family (NSCO/SSCO/RGB with the B4+B6 capability-traced links) joined to
the committed 5-zone external set (FR, CONT-NW, NO2, DK1, IE-SEM), a
composition of committed data only (design rules 1–3: every zone/link
value-checked against the two committed files in the package-1
composition-identity tests; landing-point mapping per the committed
3-zone EXTERNAL INTERCONNECTORS convention — Moyle → SSCO at
Auchencrosh, the other nine links → RGB; link declaration order B4, B6,
then the ten externals in committed 5-zone order, the disclosed
single-pass order-precedence property).

- **Scaling (design rule 6):** GB wind scales by ONE shared national
  factor 60 ÷ 29.1 applied to the onshore and offshore entries of all
  three GB zones, preserving the committed zonal splits and each zone's
  mix. Asserted bit-identical to the committed
  `wind_capacity_sweep_multi_group` helper (which also asserts parallel
  ≡ serial ≡ direct-run, the 60 GW determinism check). External fleets,
  demand and prices stay frozen at the 2024 basis (caveat (a)).
- **Rule-based leg:** `run_multi`, scarcity signal (the committed
  default). NOT anchor-validated on national trade axes (package-1
  ruling A — caveat (l) attaches verbatim to every trade number);
  quotable as ONE-SIDED bounds only: exports = FLOOR, curtailment =
  CEILING, under the asymmetric evidential rule (§5).
- **LP leg:** `run_multi_lp_min_curtailment` — the D12 MinCurtailment
  objective with the D13 **loss-as-waste term** (adopted at the head of
  package 1 under its four conditions: MinCurtailment-only, structurally
  skipped at `loss == 0.0`, unmoved-pins gate, red-first lossy
  fixtures). LP surgeries, in memory, the committed file byte-fixed:
  the `pumped_hydro` store dropped from EVERY zone (the
  `acceptance_b4_lp` PS de-dup precedent) and the FR/NO2 budgeted hydro
  converted to must-take exogenous traces at observed 2024 generation
  (hydro-as-history, ratified B(iii); per-period and budget-energy
  identity asserts committed in package 1). LP size at 60 GW: 72
  variables/period = 1,264,896 (51 % of `LP_VARIABLE_CAP`); solved
  whole-horizon, single-threaded deterministic HiGHS (ADR-5).
- **Masks and statistics:** rule-based B4/B6 binding on the committed
  gate-(iii) convention (observed flow+limit denominator: B4 17,277,
  B6 17,211; sentinel rows out of the numerator). LP binding on the
  committed b4-lp sentinel-dropped masks (B4 17,235; B6 17,042 —
  observed limit strictly inside (0.001, 9.0) GW), quoted as
  `[floor, point]` bands under TWO floor conventions, every quote
  naming its floor: **floor_internal** (committed-comparable — binding
  periods with an internal downstream zone curtailing excluded) and
  **floor_full** (externals included — caveat (n): it over-excludes;
  a deliberately loose lower bound, not a tight physics floor). LP
  binding statistics carry the b4-lp ±0.01 cross-platform convention.
- **The min-waste instrument (ruling C.2):** the weight-1 waste terms
  of the MinCurtailment objective — all-zone curtailment + storage
  round-trip loss ((1−η) × charged energy) + link loss (loss × sent,
  both directions) — reconstructed from the result series and pinned as
  a decomposition. The TOTAL is the well-determined optimum (pinned at
  1e-3 TWh); the components are mutually degenerate (the solved
  vertex's split, pinned ±0.02 TWh as characterisation — under the
  loss-as-waste term, relocating spill into any curtailing downstream
  zone is exactly objective-indifferent, so the GB/external curtailment
  split is vertex choice, not physics).
- **The basis-label rule (ruling B, binding):** every quoted number is
  labelled *GB curtailment* or *system waste*; the registered
  single-bracket sentence is retired (§4, §7). Only the pinned anchor
  and 60 GW points are quotable (caveat (d), docs/05 rule 3).
- **No capture, no LP trade (rulings C/D):** no capture is measured on
  any leg at any point; the LP's gas/trade aggregates are
  non-instruments (caveat (m)) — stated once, with mechanism, in §3.4
  and never repeated.

## 2. The package-1 anchor record (adjudicated 2026-07-05 — quoted, not re-litigated)

The composed anchor (factor exactly 1.0) was measured and adjudicated
before any 60 GW number existed. Pins in `acceptance_d13_composed.rs`;
adjudication in the package-1 addendum. The record:

| Anchor quantity (rule-based) | measured | gate | verdict |
|---|---|---|---|
| GB gas (ccgt+ocgt) | 75.018859657887 TWh | ±2 % of 71.797411 | **RED (+4.49 %)** — adjudicated a genuine instrument finding |
| GB net imports | +42.427578713250 TWh | ±5 % of +35.935153; A1 ±10 % of 33.30 | **RED (+18.1 %; +27.4 % — outright A1 miss)** |
| GB curtailment | 7.466489326179 TWh | — | the carried 3-zone stranding artefact |
| GB unserved | 1.355087867608 GWh | — | SSCO walk staleness |
| B4 binding (gate-(iii)) | 185/17,277 = 0.010708 | expected UP from 0.019506 | **RED, branch (b)** — export-drain unreachable by construction |
| B6 binding (gate-(iii)) | 662/17,211 = 0.038464 | expected UP from 0.033525 | branch (a), accepted |

Diagnoses, ruled binding: the 8(i) red is the committed equal-depth
single-pass stranding artefact (the 3-zone parent standalone reads
GB gas 82.42 / curtailment 7.01 TWh; the composition moves TOWARD the
anchor), so **the composed rule-based leg is not anchor-validated on
national trade axes, full stop** (no re-pin; the deviation-shape pins
are the record). The 8(ii) red is structural: deleting all ten external
links reproduces the composed rule-based B4/B6 binding bit-identically
— under the single-pass walk B4/B6 clear before any external border, so
the anchor movement measures the import-padding-removal surgery only.
What the anchor DOES validate: the composition itself (identities,
conservation, CF reconstruction), A1 gas in observed-basis terms
(+3.06 %), PS inertness, determinism, and the boundary-binding
instruments.

**The healthy package-1 result (quotable, floor named):** composed
anchor LP B4 [floor_internal 0.238294, point 0.281346] vs the committed
copper-external [0.2346, 0.2816] — **FLAT**: at the 2024 fleet,
attaching the modelled external world does not move perfect-foresight
B4 binding. Anchor B6 LP [0.098052, 0.098052] (floor_full 0.017017,
caveat (n)).

**Diagnostic-only, permanently (ruling D, stated here once as the
durable record):** the composed-anchor rule-based capture read 0.9585
with gas price-setting 93.28 % — the stranding artefact wearing a
price (stranded surplus forces gas dispatch; gas sets the SMP in 93 %
of periods; the value sits on the wrong side of BOTH committed
comparators, 0.941 single-zone and 0.895 tier-2, by mechanism). Never
quoted, never pinned as a capture record; the composed family has NO
capture instrument.

## 3. The 60 GW record (full precision — the pinned values)

### 3.1 Rule-based leg — ONE-SIDED bounds (caveat (l) attaches verbatim to every row)

| Quantity (basis: GB, rule-based walk) | value | bound reading |
|---|---|---|
| GB curtailment | **30.174654042171 TWh** | **CEILING** |
| GB net imports | **+11.868791000907 TWh** | most-pessimistic-for-exports; NO net-export floor reading → export survival **OPEN** (§5, E.3) |
| GB gross exports (sending end) | 23.858666863965 TWh | — |
| GB gross imports (received) | 35.727457864872 TWh | — |
| GB gas (ccgt+ocgt) | 46.874253432776 TWh | artefact-conditioned (caveat (l)) |
| GB unserved | 0 GWh | the anchor's 1.355 GWh SSCO residue clears at 60 GW |

**Zonal stranding split of the ceiling (where the stranding sits):**

| Zone | curtailment | share of GB ceiling |
|---|---|---|
| NSCO | 24.917339725736 TWh | 82.6 % |
| SSCO | 5.114250090157 TWh | 16.9 % |
| RGB | 0.143064226278 TWh | 0.5 % |

Net southward boundary transfers: B4 5.506629009292 TWh, B6
21.176151704704 TWh. All-zone rule-based curtailment (system
accounting, for §4): 35.520189138549 TWh; rule-based **system-waste
analogue** (same accounting as the LP instrument: all-zone curtailment
+ storage loss 0 + link loss 1.408729344915): **36.928918483464 TWh**.

**Per-link record (gross energies TWh; saturation = sending-end flow ≥
99 % of capacity × availability, periods of 17,568; counts pinned):**

| Link | GB gross export | GB gross import | export-saturated | import-saturated |
|---|---|---|---|---|
| IFA | 3.678762 | 7.977697 | 2,286 | 5,487 |
| IFA2 | 1.839381 | 3.988848 | 2,286 | 5,487 |
| ElecLink | 1.839381 | 3.988848 | 2,286 | 5,487 |
| Nemo | 4.090380 | 2.780520 | 7,066 | 4,922 |
| BritNed | 4.090380 | 2.780520 | 7,066 | 4,922 |
| NSL | 1.623238 | 8.479868 | 1,815 | 12,625 |
| Viking | 2.406201 | 4.869116 | 1,161 | 4,370 |
| Moyle | 1.460580 | 0.057369 | 5,405 | 131 |
| EWIC | 2.830363 | 0.804672 | 10,768 | 2,592 |
| Greenlink | 0 | 0 | inert (availability 0.0, 2024 commissioning — asserted) |

**Rule-based B4/B6 binding at 60 GW (gate-(iii) convention — the
disclosed myopic comparator, never a central on this axis):**

| Boundary | 60 GW | composed anchor | committed 3-zone |
|---|---|---|---|
| B4 | 1,706/17,277 = 0.098743994907 | 0.010708 | 0.019506 |
| B6 | 4,769/17,211 = 0.277090232991 | 0.038464 | 0.033525 |

PS inertness asserted green at 60 GW (review edit 4, both points now
held measured); per-zone conservation asserted; determinism per §1.

### 3.2 LP minimum forced waste (system basis) — the headline instrument

**TOTAL waste is the well-determined optimum; every component row is
the solved vertex's DEGENERATE split (characterisation, ±0.02 TWh
pins), never quotable alone.**

| Waste term (system basis, TWh) | anchor (29.1 GW) | 60 GW |
|---|---|---|
| **Total minimum forced waste** | **12.196896137008** | **36.223998953964** |
| Curtailment, all zones (degenerate) | 11.758631592405 | 35.638937973451 |
| — GB (degenerate vertex split) | 2.619408148203 | 26.217733834871 |
| — external (degenerate vertex split) | 9.139223444203 | 9.421204138580 |
| Storage round-trip loss | 0.010582720888 | 0.046226069891 |
| Link loss | 0.427681823715 | 0.538834910621 |
| Unserved (excluded from waste; §4 wedge) | 785.086485280863 GWh | 785.086485280863 GWh |

Vertex curtailment split at 60 GW, for the record (all degenerate):
NSCO 17.430491894724, SSCO 8.358788201457, RGB 0.428453738690, FR 0,
CONT-NW 3.438346212891, NO2 4.281645762987, DK1 1.342396472158, IE-SEM
0.358815690544 TWh.

Derived, pin-backed quantities (ruling B convention): wind-driven
increment 36.223998953964 − 12.196896137008 = **+24.027102816956 TWh**;
conservative deconfounded exceedance over the copper-plate tier-2
central 24.027102816956 − 4.007462807827 = **+20.019640009129 TWh**.
The raw 36.22-vs-4.01 comparison is basis-unfair and is **not quoted as
an exceedance** (the composed baseline contains external curtailment
and losses the copper-plate number never counted).

### 3.3 LP B4/B6 binding bands (every quantity names its floor; ±0.01)

| Boundary / statistic | anchor | 60 GW |
|---|---|---|
| B4 point | 4,849/17,235 = 0.281346 | 9,845/17,235 = **0.571221351900** |
| B4 floor_internal | 4,107/17,235 = 0.238294 | 4,744/17,235 = **0.275253843922** |
| B4 floor_full (caveat (n) loose bound) | 905/17,235 = 0.052509 | 1,034/17,235 = 0.059994197853 |
| B6 point | 1,671/17,042 = 0.098052 | 6,612/17,042 = **0.387982631147** |
| B6 floor_internal | 1,671/17,042 = 0.098052 | 6,321/17,042 = **0.370907170520** |
| B6 floor_full (caveat (n) loose bound) | 290/17,042 = 0.017017 | 1,167/17,042 = 0.068477878183 |

The committed-comparable B4 band [floor_internal, point] roughly
doubles at the point end and WIDENS — much of the new binding sits in
downstream-curtailing periods. The B6 band [0.371, 0.388] is **nearly
degeneracy-free: B6 binds in physics, not vertex choice, in over a
third of masked periods at 60 GW** (adjudication ruling D).

### 3.4 Caveat-(m) one-time LP diagnostics (stated once, with mechanism — never repeated, never pinned)

The MinCurtailment LP is not a gas/trade instrument. Its GB vertex
aggregates, recorded here once as the durable diagnostic statement:
anchor — gas 160.2 TWh (+123 % vs the committed anchor: the
thermal-split objective-degeneracy made concrete, HiGHS parks thermal
service on ccgt because no cost term orders the rungs), net imports
+9.6 TWh (loss-minimising autarky: under the loss-as-waste term lossy
imports are strictly dominated by free domestic thermal wherever
headroom exists); 60 GW — gas 102.73 TWh, net imports +4.67 TWh (same
two mechanisms). These are HiGHS vertex artifacts, not measurements.

## 4. Branch adjudication — how the anomaly fired and how Branch A was ruled

Chronology, per the pre-registration (nothing re-framed after the
fact):

1. **The anomaly catch-all's one NAMED shape fired on first
   measurement:** the LP min-waste reading (36.224 TWh, system basis)
   sits ABOVE the rule-based GB curtailment ceiling (30.175 TWh, GB
   basis), so the registered bracket "curtailment ∈ [LP min-waste,
   rule-based ceiling]" INVERTS as registered. The implementer stopped,
   characterised, pinned both facts in their measured shape, and
   withheld the verdict (the D11 discipline).
2. **The adjudication ruled the inversion a REGISTRATION defect,
   adjudicator-owned, not an instrument failure** (2026-07-06 addendum,
   ruling B): the registered bracket put an all-zone SYSTEM-WASTE
   quantity at one end and a GB-ONLY CURTAILMENT quantity at the other.
   Verified not an optimality anomaly: on the like basis the LP optimum
   sits below the rule-based system waste (36.223998953964 <
   36.928918483464), and on the GB basis the vertex split sits below
   the ceiling (26.218 < 30.175) — both asserted in-file and re-run by
   the adjudicator. The bracket sentence is **retired**; the ruling-B
   basis-label convention (§5 sentences) replaces it.
3. **With the anomaly characterised and closed, Branch A fires — the
   geometry forces the waste:** the dispatch-independent exceedance is
   decisive (+20.02 TWh conservative deconfounded; +32.22 TWh raw with
   basis disclosure); the LP bands load hard and UP from the anchor
   beyond the ±0.01 convention on both quoted ends; export survival is
   OPEN under the asymmetric evidential rule (the artefact-conditioned
   rule-based floor reads +11.87 TWh net imports — not evidence of
   collapse). **Caveat (e)'s curtailment component resolves AGAINST the
   tier-2 level; its capture and net-trade components remain OPEN.**

**The secondary like-basis finding (quotable with its conventions
disclosure):** at 60 GW, perfect foresight recovers only
**~0.70 TWh** of the composed rule-based dispatch's own system waste
(36.224 vs 36.929 TWh) — the waste is **absorption/geometry-limited,
not dispatch-limited**. This INVERTS the current-fleet B4 headline
("the choke is dispatch-limited, not geometry-limited",
b4-lp-findings) at 60 GW. Disclosure: the two legs differ by the PS
de-dup and NO2-as-history conventions, so ~0.70 is approximate, not a
pinned dispatch premium.

**The NO2 conventions finding (disclosed both ways — caveat (i)
extension AND this named row):** the all-zone unserved comparison
between the legs is a measured conventions wedge, not a feasibility
result: LP 785.086485280863 vs rule-based 207.925547466578 GWh at
60 GW = **0.577 TWh**, entirely in NO2 (LP 592.793 vs rule-based
15.632 GWh; DK1 191.309 and CONT-NW 0.985 GWh bit-identical on both
legs; FR zero on both), and wind-independent (bit-equal at anchor and
60 GW, per zone). Mechanism: the ratified hydro-as-history conversion
denies NO2 the within-week flexibility its committed 336-period budget
gives the rule-based leg; FR shows no wedge because
`window_periods = 1` is already a trace. Ruled to threaten NO quoted
number: the wedge is unserved, which the min-waste total excludes by
construction; its waste-side analogue points UP on composed waste (the
direction already ruled conservative), is largely absorbed into the
anchor baseline by the deconfounded increment, and any residual is
bounded by NSL's 1.4 GW throughput in surplus hours — two orders under
the +32.2 and an order under the +20.0 margins. The binding invariant
is restated GB-side: GB zones carry zero unserved on both legs
(asserted); the all-zone comparison is pinned as the characterised
wedge.

## 5. QUOTE ONLY THESE (the adjudicated sentences, verbatim, each with its mandatory caveat set)

**E.1 — Curtailment/waste (the caveat-(e) curtailment resolution):**

> The tier-2 60 GW curtailment central (3.98 TWh (pre-R7: 4.01), GB
> copper plate)
> does not survive composition with the measured B4/B6 boundaries and
> the modelled 2024 external system. The composed minimum forced
> SYSTEM waste at 60 GW is 36.22 TWh under ANY dispatch
> (perfect-foresight floor; composed 2024-fleet baseline 12.20 TWh;
> wind-driven increment +24.03 TWh — at least +20.0 TWh above the
> copper-plate central on the conservative deconfounded basis). On
> the GB-curtailment basis the composed rule-based ceiling is
> 30.17 TWh (one-sided, caveat (l)).

Mandatory caveats on every E.1 quote: basis labels (ruling B); (d)
pinned points only; (f) northward shift — understates the constraint
effect; (k) 2024 boundary capability — overstates it; (i) including
the NO2 wedge; (l) verbatim for any rule-based number; (n) for any
floor_full. **No GB-basis floor exists in this record**: the LP's
GB-attributed 26.22 TWh is a degenerate vertex split,
characterisation-only, never quoted as a floor — the record says so
rather than manufacturing one. No ±%-of-added-wind framing is quoted:
the added-potential denominator is unpinned, and no unpinned ratio is
quoted (compute and pin the denominator first if that framing is ever
wanted).

**E.2 — Boundary loading:**

> At 60 GW the perfect-foresight LP binds B4 in [0.275, 0.571] of
> masked periods (floor_internal, point; floor_full 0.060 is the
> caveat-(n) loose bound) against [0.238, 0.281] at the current
> fleet, and B6 in [0.371, 0.388] against 0.098 — the B6 band is
> nearly degeneracy-free, so the Anglo-Scottish boundary binds in
> physics in over a third of periods at 60 GW.

Mandatory caveats: floor named on every number; (g) B4 input biases
(DA-only, §3 onshore split, §6 offshore wedge — UP); (h) B5 folded;
(k) counterweight; the b4-lp ±0.01 cross-platform convention.

**E.3 — Export survival:**

> OPEN. The composed rule-based floor reads +11.87 TWh net imports at
> 60 GW; under the pre-registered asymmetric evidential rule this is
> not evidence of collapse (the dispatcher is artefact-conditioned
> and structurally cannot express export-drain wheeling). No composed
> instrument measures the trade level; the economic-dispatch LP is
> the named resolver. The copper-plate −6.46 TWh net exports remains
> quotable only alongside this OPEN status.

**E.4 — Capture:** unchanged (package-1 ruling D): no composed capture
instrument exists; 0.698 remains an upper-side estimate on the capture
axis of caveat (e).

The secondary like-basis sentence (§4, "absorption/geometry-limited,
not dispatch-limited, at 60 GW — perfect foresight recovers only
~0.70 TWh of 36.93") is quotable WITH its conventions disclosure
(PS de-dup + NO2-as-history: approximate, not a pinned dispatch
premium).

## 6. The caveat block (a)–(n), as amended, carried on the composed record

- **(a)** frozen 2024 externals — bias UP on exports (verbatim from
  the tier-2 record);
- **(b′)** dispatcher fidelity — carried as the two-leg record itself;
  the −0.046 anchor capture wedge still conditions any future capture
  level and is not a subtractable constant;
- **(c)** the R7 flow-walk stall — carries to the rule-based leg; the
  package-1 stall diagnostic (9,473/9,473 GB-curtailment periods carry
  the ≤-bound signature at the anchor) is VACUOUS at composed scale and
  is not quoted as a stall measurement; **fixed in the R7 engine
  package 2026-07-06 (docs/08 R7 RESOLVED — the rule-based-leg pins of
  this record were re-pinned there, old values recorded per pin)**;
- **(d)** pinned points only — anchor and 60 GW;
- **(e)** of the D11 record — curtailment component RESOLVED AGAINST
  the tier-2 level by this measurement; capture and net-trade
  components OPEN (§7 amendment);
- **(f)** northward shift not represented — the composed measurement
  still UNDERSTATES the constraint effect;
- **(g)** B4 input biases (DA-only, onshore split ~+31 %/unit,
  offshore commissioning wedge ~19 %) — push modelled B4 binding UP;
- **(h)** B5 folded into the SSCO copper plate — lower-bound posture on
  constraint severity;
- **(i)** LP-leg conventions — PS de-dup + external hydro-as-history +
  the [floor, point] degeneracy band housing the loss-disposal class;
  **EXTENDED (2026-07-06): the NO2 hydro-as-history convention carries
  a measured 0.58 TWh unserved floor in NO2, wind-independent** (the §4
  conventions-finding row);
- **(j)** the single-GB-price basis (A) — conditions the price-basis
  definition only; no capture instrument exists;
- **(k)** boundary capability frozen at 2024 — no reinforcements
  modelled; bias DOWN on exports, UP on composed curtailment (the
  counterweight to (f));
- **(l)** rule-based trade axes artefact-conditioned — every rule-based
  trade quote carries **+4.49 % gas / +18.1 % imports vs the committed
  5-zone anchor; +27.4 % imports vs observed (the outright A1 miss)**
  *(R7 correction 2026-07-06: on the fixed engine the travel-verbatim
  values are **+4.41 % gas / +18.2 % / +27.5 %** — the anchor-red
  verdict and the artefact-conditioning are unchanged)* and the
  one-sided-bounds reading under the asymmetric evidential
  rule;
- **(m)** the LP is not a gas/trade/capture instrument — vertex
  diagnostics stated once in §3.4, never repeated;
- **(n)** floor_full over-excludes — a deliberately loose lower bound
  on the artifact class; every binding quote names its floor.

## 7. Record amendments carried by this report

**D11 run-report §4 amendment (append to quoting condition (e) of
`docs/notes/d11-sweep-run-report.md`):**

> PARTIALLY RESOLVED BY MEASUREMENT (D13, `d13-run-report.md`): on
> the curtailment axis the composed measurement resolves AGAINST the
> tier-2 level — minimum forced system waste 36.22 TWh at 60 GW
> (wind-driven increment +24.03 TWh over the composed anchor
> baseline; ≥ +20.0 TWh above the 3.98 TWh (pre-R7: 4.01) central on
> the conservative deconfounded basis); the 3.98 TWh (pre-R7: 4.01)
> copper-plate figure
> is quotable only alongside that record. The capture and net-trade
> components remain OPEN (no composed instrument; named resolver: the
> economic-dispatch LP, docs/08). Export survival: OPEN under the
> asymmetric evidential rule — the composed rule-based floor reads
> net imports, which is not evidence of collapse.

**Supersession notes (design-note amendments per addendum condition
2):** the registered bracket sentence "curtailment ∈ [LP min-waste,
rule-based ceiling]" is RETIRED (basis-mismatched as registered; ruling
B's three labelled quantities replace it), and the asymmetric rule's
trailing "leaves the question to the bracket" reads "leaves the
question OPEN"; the all-zone LP ≤ rule-based unserved invariant is
restated GB-side, with the all-zone comparison carried as the
characterised NO2 conventions wedge (§4).

## 8. Reproducibility (docs/06)

- **Engine:** `36e775a` (the committed D13 package 1: loss-as-waste
  term + composed scenario + anchor pins) + the package-2 measurement
  file `grid-adequacy/tests/acceptance_d13_60gw.rs` (additive; commit
  approved by the 2026-07-06 addendum, to reference that addendum).
- **Scenario:** `scenarios/gb-2024-8zone.toml`, sha256
  `23d51777a935cfc92c6863520759e4be460cf1ae241cd4c24d801f25986981f9`
  (pinned with the per-zone dispatch digests and links digest in
  `grid-cli/tests/regression_8zone.rs`; schema v7).
- **Data packs** (fetched/derived, never committed; committed
  manifests pin the contents — manifest-file sha256s):
  `data/packs/2024.sha256` (`04e235857d90ebf9…`),
  `data/packs/cf-gb2-1985-2024.sha256` (`a680bf835a59a1e4…`),
  `data/packs/cf-gb3-1985-2024.sha256` (`51c28ac4fe72b0de…`),
  `data/packs/b4.sha256` (`004687ea65adb22c…`),
  `data/packs/b6.sha256` (`3f98129a2b89d607…`),
  `data/packs/entsoe-2024.sha256` (`11ab1426d9e155aa…`),
  `data/packs/cf-eu-1985-2024.sha256` (`79663ea49d251fe9…`); price
  reference `data/reference/prices-2024.toml` (`aebeca6953d89e00…`).
- **Reproduce:** `cargo test --release -p grid-adequacy --test
  acceptance_d13_60gw -- --nocapture` (both legs at 60 GW, the anchor
  LP baseline, the composed-anchor rule-based rerun for the wedge
  characterisation, all pins and shape assertions); the anchor record:
  `cargo test --release -p grid-adequacy --test acceptance_d13_composed
  -- --nocapture`. Composed LP solve ≈ 15–20 s per point (release,
  Apple Silicon; the d12 extrapolation predicted ~6 min — comfortably
  under; no HiGHS abort, 51 % of the variable cap). Suite at handover
  and at adjudication: **646 passed / 0 failed / 4 ignored**;
  fmt/clippy clean; all committed pins unmoved.

## 9. Onward work (named, not scheduled here)

1. **The min-cost economic-dispatch LP** — the composed family's
   capture/net-trade instrument and the named resolver for the OPEN
   axes of caveat (e) and for E.3's trade level (docs/08 row; decision
   number assigned by Richard; explicitly not commissioned inside D13).
2. **Declaration-order sensitivity** — single-pass order-precedence is
   a disclosed dispatcher property (package-1 ruling B); may run as a
   NAMED sensitivity someday, never a silent switch.
3. **Northward-shift re-siting** (caveat (f)) — a fleet-geography
   scenario package; until then every composed quote understates the
   constraint effect.
4. **Boundary-capability scenarios** (caveat (k)) — B6 uprating /
   eastern HVDC reinforcement variants against the frozen-2024
   capability basis.
