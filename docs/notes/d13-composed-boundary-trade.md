# D13 — Composed boundary-trade measurement (3-zone GB × 5-zone externals) design

**Status:** ADOPTED 2026-07-05 — reviewer ADOPT-WITH-EDITS
(docs/notes/d13-composed-boundary-trade-review.md), all nine ordered
edits applied below. The adjudication ruled three of the draft's open
forks rather than deferring them: the loss-as-waste objective term is
IN (with four conditions, relocated to the head of package 1), the LP
shadow-capture diagnostic is SUPPRESSED (thermal-split
objective-degeneracy), and the LP-leg budget conversion is ratified as
scoped; it also split the two-leg framing (edit 1) so the note cannot
be quoted both ways. Supervisor draft 2026-07-05, pulled ahead of the
Stage 7 remainder by Richard's ruling 2026-07-05. D10 remains reserved
for the EV/transport overlay (Q12); D11 and D12 are taken; this is
**D13**.

**PACKAGE-1 ADJUDICATION (2026-07-05, review addendum — edits P2-1…
P2-6 applied below):** package 1 delivered and stopped the line per
rule 8 — gates 8(i) and 8(ii)-B4 measured **RED**, adjudicated as
**genuine measured findings about the instrument, not composition
defects**; the deviation-shape pins are the record; **no re-pin**.
Consequences ruled binding for package 2 and incorporated in place
below: the composed rule-based leg is **NOT anchor-validated on
national trade axes** (edit-1 regime (a)'s premise superseded by the
addendum's ruling C re-scoped instrument set: LP binding bands with
named floors, LP **minimum forced waste** with pinned objective
decomposition, rule-based trade as one-sided bounds under the
asymmetric evidential rule); the composed family currently has **NO
capture instrument** (ruling D — anchor capture 0.9585
diagnostic-only, never quotable; caveat (e)'s capture axis stays OPEN
with the min-cost economic-dispatch LP as its named resolver, a NEW
decision row for Richard, not commissioned in D13); the 8(ii)
expectation is re-registered as the import-padding-removal surgery
(ruling B; declaration order stands); caveats (l)/(m)/(n) added.
Package 2 is commissioned re-scoped once these edits are applied; it
is NOT blocked behind any engine build.

**PACKAGE-2 DELIVERED AND ADJUDICATED (2026-07-06, review addendum —
verdict ACCEPT-WITH-CONDITIONS, commit approved; ruled amendments
applied below): BRANCH A FIRES — the geometry forces the waste.**
The anomaly catch-all fired first in its named shape (LP min-waste
36.22 TWh above the rule-based GB curtailment ceiling 30.17 TWh),
the verdict was withheld as pre-registered, and the anomaly closed as
an adjudicator-owned **registration defect** (the [LP min-waste,
rule-based ceiling] bracket was basis-mismatched from birth —
RETIRED, replaced by the ruling-B basis-labelled convention in
rule 4: primary min system waste 36.22 / baseline 12.20 / wind-driven
increment +24.03, exceedance only in the deconfounded +20.02 form;
secondary like-basis ~0.70 TWh recovery — absorption/geometry-limited,
not dispatch-limited; GB basis one-sided-ceiling-only, no floor
exists). The LP bands load hard (B4 [0.275, 0.571] — point doubled;
B6 [0.371, 0.388] — nearly degeneracy-free); **export survival OPEN**
under the asymmetric rule (+11.87 TWh net imports *(R7 correction
2026-07-06: **+11.70 TWh** on the fixed engine; reading unchanged)* is
not evidence of
collapse); **caveat (e)'s curtailment component resolves AGAINST the
tier-2 level; capture and net-trade stay OPEN**. The unserved
invariant is restated GB-side with the NO2 hydro-as-history wedge
(0.577 TWh, wind-independent, threatens nothing — ruled explicitly)
as a named conventions finding + caveat (i) extension. **Run
report: `docs/notes/d13-run-report.md`** (addendum §E contents;
written before any D13 number is quoted anywhere).

## The problem: caveat (e) has a named resolver, and this is it

The D11 tier-2 sweep record (`docs/notes/d11-sweep-run-report.md` §3,
reviewer-adjudicated `d11-sweep-review.md` §F) measured the 60 GW
central estimate on a **copper-plated GB**: delivered capture
**0.6976839505365661**, curtailment **4.007462807827 TWh**, gas
40.695234239837 TWh, GB net exports **6.456015207006 TWh**. The
post-acceptance addendum (2026-07-05) ruled mandatory quoting caveat
(e): GB-internal transmission is unconstrained, nearly all
interconnector landing points sit **south of both B4 and B6**, so the
export channel implicitly wheels northern wind across the measured
binding constraints for free — bias UP on capture/exports, DOWN on
curtailment. The committed boundary evidence says the omission is
first-order: the perfect-foresight LP binds B4 in **[23.5 %, 28.2 %]**
of periods at the CURRENT ~29 GW fleet vs rule-based 1.96 %
(`docs/notes/b4-lp-findings.md`), and copper-plating GB understates
storage sizing by +38–49 % (B6 record, cited in the addendum).

The addendum names the resolver: **the composed measurement** — the
gb-2024-3zone.toml boundary family (NSCO/SSCO/RGB, B4+B6) joined to
the gb-2024-5zone.toml external set (FR, CONT-NW, NO2, DK1, IE-SEM).
This note is that measurement's adopted design — the work order for
the two packages below (adjudication:
`docs/notes/d13-composed-boundary-trade-review.md`).

**The question the measurement answers:** does the tier-2
export/capture finding at 60 GW survive when northern wind must cross
the measured B4/B6 constraints to reach the interconnector landing
points? Headline outputs as originally scoped: 60 GW
delivered/potential capture, curtailment, net trade, **and B4/B6
binding frequencies at the anchor and at 60 GW**, quotable against
(i) the tier-2 central (the caveat-(e) resolution) and (ii) the
committed B4 band. **⚠ Re-scoped by the package-1 adjudication
(ruling C — see rules 4/5/8):** the quotable outputs are the LP B4/B6
binding bands (both floors), the LP **minimum forced waste** vs the
copper-plate 4.01 TWh, and the rule-based trade axes as one-sided
bounds under the asymmetric evidential rule; **no composed capture is
quotable on this family** (ruling D — the capture axis of caveat (e)
stays open, resolver named in rule 5).

## Rule 1 — Zone set and the composition rule (no new data)

The composed scenario (proposed file: `scenarios/gb-2024-8zone.toml`)
is a **composition of committed data only**:

- **GB side**: NSCO / SSCO / RGB with fleet, demand splits, exogenous
  `other`/`pumped_storage_net` scales, storage, zonal CF traces
  (cf-gb2 `nsco_*`/`ssco_*`/`rgb_*`) and the B4 + B6 links (capability
  traces, reverse capacities, masks, sentinels) **value-identical to
  the committed `scenarios/gb-2024-3zone.toml`** — every number
  traceable to the three-zone data report and its six binding
  obligations, which carry over verbatim. Two stated departures, both
  the exact surgery the 5-zone file performed on the GB reference:
  1. the exogenous `net_imports` blocks are **removed** (SSCO's
     `intirl` column; RGB's nine-column block) — external trade is now
     modelled through `[[links]]`, so the observed traces must not be
     double-carried;
  2. each GB zone gains the committed GB `[zones.pricing]` block
     (reference `prices-2024.toml` + daily gas SAP + ccgt/ocgt
     efficiency keys), **identical in all three zones** — the chain the
     D11 review §C verified bit-identical to the single-zone reference
     chain. This is a metric prerequisite (rule 5), not new data, and
     schema v7 already supports it (no schema bump).
- **External side**: the five zones FR, CONT-NW, NO2, DK1, IE-SEM
  **byte-identical to the committed `scenarios/gb-2024-5zone.toml`
  entries** — demand, wedges, extra_profiles, calibrated fleets,
  energy budgets, v7 pricing blocks, everything. No re-derivation, no
  new fetches.
- **Links**: B4 and B6 byte-identical from the 3-zone file; the ten
  external links carried from the 5-zone file with only the GB
  endpoint renamed per the landing-point mapping (rule 2).
  **Declaration order** (flow.rs prose rule 6 — single pass, in-order):
  **B4, then B6, then the ten external links in the committed 5-zone
  order** (largest border first). Rationale: B4→B6 is the committed
  physical cascade (3-zone obligation 4), and clearing the internal
  cascade first lets wheeled-south northern surplus reach RGB *before*
  the export links clear — the export-channel-faithful order. The
  residual single-pass staleness (exports drain RGB after B6 has
  cleared, with no second sweep to pull more wind south) biases the
  rule-based leg toward **under-wheeling**, i.e. toward overstating
  the constraint effect — the same side as rule 4's dispatcher
  bracket, stated so it is charged to the dispatcher, not the
  geometry. On the LP legs the precision is narrower than outright
  order-invariance (review edit 7): the LP's **optimum** — objective
  value and every non-degenerate quantity — is
  declaration-order-invariant, but degenerate-vertex flow statistics
  can move with variable/column order, which follows declaration
  order; that sensitivity is charged to the existing [floor, point]
  degeneracy-band and ±0.01 cross-platform conventions (rules 4 and
  8(iv)), not claimed away.

**Engine mechanics, verified against the source:** `run_multi`
(`grid-adequacy/src/multizone.rs:230`) and the LP core
(`lp.rs::run_multi_lp_core`) are generic over arbitrary zone/link
sets — nothing in either hard-codes a zone count, and both families'
features (capability-traced links, per-direction capacities, lossy
availability-derated links, exogenous `scale`, `extra_profiles`,
energy budgets under rule-based) are individually exercised by
committed tests. **One honest gap**: `wind_capacity_sweep_multi`
(`sweep.rs:850`) scales exactly **one named zone's** wind and reads
one zone's result + pricing for its metrics. The composed measurement
scales GB wind across **three** zones and aggregates GB metrics across
them, so it needs a small **additive** measurement helper (a
zone-group scaling function applying one shared factor, plus a
GB-aggregate metrics function — rule 5/6). No dispatch-engine change;
the untested *combination* of features (capability-traced internal
links + lossy external links in one scenario) gets an explicit
composition test in package 1.

## Rule 2 — Landing-point mapping (the load-bearing geometry)

The committed record already assigns every built link a GB landing
zone: the 3-zone scenario's EXTERNAL INTERCONNECTORS convention
(design-review item 7) states *"Moyle lands at Auchencrosh 275 kV
(Ayrshire, SPT, south of B4) → S-Scotland gets the FUELHH `intirl`
column; every other built link lands in England/Wales → E+W (RGB) gets
the other nine columns"*, and the D11 sweep-review addendum names the
same list ("NSL at Blyth ⇒ RGB"). The composed mapping converts that
exogenous column convention into modelled link endpoints, one for one:

| Link | External zone | GB zone | Capacity (5-zone, committed) | Committed citation |
|---|---|---|---|---|
| IFA | FR | RGB | 2.0 GW, avail 0.95, loss 0.021 | 3-zone header (`intfr` → RGB) |
| IFA2 | FR | RGB | 1.0 GW, 0.95, 0.021 | 3-zone header (`intifa2` → RGB) |
| ElecLink | FR | RGB | 1.0 GW, 0.95, 0.021 | 3-zone header (`intelec` → RGB) |
| Nemo | CONT-NW | RGB | 1.0 GW, 0.95, 0.027 | 3-zone header (`intnem` → RGB) |
| BritNed | CONT-NW | RGB | 1.0 GW, 0.95, 0.027 | 3-zone header (`intned` → RGB) |
| NSL | NO2 | RGB | 1.4 GW, 0.95, 0.028 | 3-zone header + sweep-review addendum ("NSL at Blyth ⇒ RGB") |
| Viking | DK1 | RGB | 1.4 GW, 0.95, 0.016 | 3-zone header (`intvkl` → RGB) |
| Moyle | IE-SEM | **SSCO** | 0.5 GW, 0.95, 0.025 | 3-zone header (`intirl` → S-Scotland, Auchencrosh) |
| EWIC | IE-SEM | RGB | 0.5 GW, 0.95, 0.025 | 3-zone header (`intew` → RGB) |
| Greenlink | IE-SEM | RGB | 0.5 GW, **avail 0.0** (2024 commissioning), 0.025 | 3-zone header (`intgrnl` → RGB); availability carried from 5-zone |

**No link is ambiguous in the committed record** — the 3-zone
convention enumerates all ten. One near-boundary case is flagged so
the reviewer sees it was considered: **NSL at Blyth** (Northumberland)
is the only landing point close to a modelled boundary; it is south of
B6, the committed record assigns it to RGB, and that assignment is
carried. Consequence, stated: Norwegian exchange cannot relieve or
load B6 directly — northern wind must cross both boundaries to reach
NSL, which is exactly the geometry caveat (e) exists to measure.
Greenlink's `availability = 0.0` is carried byte-identical (the 2024
basis convention, rule 6): it contributes nothing at either point.

The Moyle placement is the one genuinely load-bearing composition
choice: it gives S-Scotland its only external outlet (0.5 GW), so a
sliver of Scottish surplus can exit without crossing B6. That is the
committed geography, not a convention invented here.

## Rule 3 — Pumped-storage double-count: the de-dup convention

The composed GB zones inherit the committed double-count: GB pumped
storage appears **twice below the surface** — the exogenous
`pumped_storage_net` traces (observed 2024 PS actions, split 0.2617
NSCO / 0.7383 RGB) *and* the dispatchable `pumped_hydro` stores
(Cruachan+Foyers 740 MW in NSCO; Dinorwig+Ffestiniog 2,088 MW in RGB).
Per the committed 3-zone warning blocks (audits 2026-07-04/05): this
is **harmless under the rule-based engine** (both stores stay inert —
zero cycling, SoC full all year — so the rule-based leg runs the
scenario as composed), but a perfect-foresight LP **wakes the stores**
and the same physical assets act twice (measured on B4: 31.75 %
binding with the NSCO store double-counted vs 28.16 % de-duplicated).

**Inertness is ASSERTED, not assumed (review edit 4).** The committed
"harmless under rule-based" evidence comes from the boundary family
*without* externals at the 2024 fleet; the composed scenario at 60 GW
is new territory. The acceptance tests therefore assert **zero
`pumped_hydro` cycling on the rule-based legs at BOTH points** (anchor
and 60 GW). If the assertion holds, the harmless claim stands
measured. If the stores wake at 60 GW, the rule-based leg is **NOT**
silently de-duplicated — that would break like-for-like with the
tier-2 comparator, whose GB zone carried the identical double-count
structure at 60 GW; instead the active double-count is disclosed as a
carried tier-2 convention in caveat (i), with its magnitude reported.

**Convention, carried verbatim from the `acceptance_b4_lp.rs`
precedent:** every LP run on the composed scenario first drops the
`pumped_hydro` store from **every** zone, in memory (the committed
scenario file stays byte-fixed; the surgery is test-local). The
de-duplication treats 2024's PS operation as history — conservative
for binding frequency, a stated modelling choice (Richard's ruling,
`d12-mincurtailment-decision.md`).

**LP-leg extension required for the externals (new here, same
posture):** the LP core rejects energy-budgeted fleet entries
(`lp.rs::build_zone_data` — structured `UnsupportedFeature`), and both
FR (reservoir+pumped, `window_periods = 1`) and NO2 (weekly budget)
carry them. Rather than build the deferred cumulative-window LP
machinery (a real engine package, out of scope — rule 9), the LP leg
converts both zones' budgeted hydro to **must-take exogenous traces at
their observed 2024 generation** (the same A75 columns the budgets
read), in memory. This is the identical "observed operation as
history" convention as the PS de-dup: it denies the LP counterfactual
external-hydro flexibility. Bias direction, stated as a judgment (per
the adjudication B(iii), carried in caveat (i)): external absorption
becomes less adaptive, so the LP leg's export capacity is mildly
**understated** — against the export-survives branch, i.e.
conservative for the finding this measurement tests. **RATIFIED by the
adjudication (B(iii))**; the cumulative-budget LP constraint (linear,
tractable) remains the named future engine package with its own
red-green cycle, and does not block D13.

**Identity asserts, mandatory (review edit 6):** the conversion is
mechanical, not editorial — the acceptance test asserts that the
substituted FR/NO2 must-take traces are **per-period identical** to
the budgets' own A75 columns (`hydro_reservoir` + `hydro_pumped` from
`fr_generation_2024.parquet` / `no2_generation_2024.parquet`) and that
their annual sums reproduce the committed budget energies (**FR
24.37 TWh; NO2 43.67 TWh** — the 5-zone header values). The FR
pumping **demand** leg stays the committed `extra_profiles` trace,
untouched — stated so the conversion is visibly one-sided
(generation-side only).

## Rule 4 — The dispatcher bracket: rule-based and LP, quoted as a band

The committed B4 evidence fixes the expectation: the scarcity rule
**under-wheels** (B4 rule-based 1.96 % vs the LP band [23.5 %, 28.2 %]
— `b4-lp-findings.md`; the committed 3-zone pin is
`PIN_B4_CONSTRAINED_BINDING = 0.0195…` on the gate-(iii) denominator,
same statistic, different mask convention, both circulate). At 60 GW
the rule-based leg will therefore strand northern surplus behind B4/B6
and **overstate the constraint effect**; the perfect-foresight
MinCurtailment LP wheels surplus as far as the links physically allow
and is the **optimistic bound**. Neither is the truth; together they
bracket it. So the measurement runs **both legs at both points**
(anchor and 60 GW):

- **Rule-based leg** (`run_multi`, scarcity signal — the committed
  default): the **like-for-like comparator** against the tier-2 0.698,
  which was itself a scarcity-rule number (d11-engine-review §G).
- **LP leg** (`run_multi_lp_min_curtailment`,
  `d12-mincurtailment-decision.md` objective, with the rule-3
  surgeries): the perfect-foresight bound on the same quantities.
  Within this leg, B4/B6 binding statistics are themselves quoted as
  the **[floor, point] degeneracy band** (downstream-curtailment class
  excluded from the floor), exactly the committed convention.

**Framing ruling (review edit 1 — two explicit regimes, so the note
cannot be quoted both ways):**

- **(a) Capture / curtailment / net trade / gas —
  ⚠ SUPERSEDED BY MEASUREMENT (package-1 adjudication, ruling A/C;
  edit P2-1).** The original regime named the rule-based value "the
  anchor-validated central" on the premise that the rule-based leg
  validates at the anchor. That premise is **measured false on this
  scenario family**: the composed anchor failed gate 8(i) (rule 8
  annotation below — composed net imports miss the outright A1 ±10 %
  gate at +27.4 % vs observed; the committed equal-depth single-pass
  stranding artefact, not the composition, is the diagnosed cause),
  so the composed rule-based leg is **NOT anchor-validated on
  national trade axes**. The quotable instrument set is re-scoped by
  ruling C:
  1. **B4/B6 binding bands (LP)** — the sound instrument, quoted
     under BOTH floor conventions (floor_internal =
     committed-comparable, excluding downstream GB-internal
     curtailment; floor_full = externals included), and **every quote
     names its floor convention**. floor_full over-excludes (it tests
     downstream curtailment without checking link saturation) and is
     a deliberately loose lower bound on the artifact class, not a
     tight physics floor — caveat (n).
  2. **LP MINIMUM FORCED WASTE** — the well-determined optimum. The
     objective's optimal **value** is unique; its *components*
     (curtailment vs link-loss vs storage-loss) are mutually
     degenerate. Package 2 **pins the objective decomposition** and
     quotes **total waste** as the primary min-waste quantity, with
     curtailment quoted as a band whose width is the degenerate loss
     channels (the d12 band discipline).

     **⚠ The originally registered exceedance comparison ("if the
     composed LP minimum waste at 60 GW exceeds the copper-plate
     rule-based 4.01 TWh…") and its bracket sentence are RETIRED
     (package-2 adjudication, 2026-07-06, ruling B): they put an
     ALL-ZONE SYSTEM-WASTE quantity against a GB-ONLY CURTAILMENT
     quantity — basis-mismatched from birth, an adjudicator-owned
     registration defect.** In their place, the ruled basis-labelled
     convention (binding): every quoted number carries its basis
     label — *GB curtailment* or *system waste* — and the record
     quotes THREE things, never a single bracket:
     - **PRIMARY (dispatch-independent, system basis):** composed
       minimum forced system waste at 60 GW = **36.22 TWh**
       (perfect-foresight floor, any dispatch), against the composed
       2024-fleet baseline **12.20 TWh** — a wind-driven increment of
       **+24.03 TWh**, which exceeds the copper-plate tier-2
       curtailment central (4.01 TWh) by **+20.02 TWh even on the
       conservative deconfounded basis** (the entire anchor baseline
       subtracted, including its GB component). The vs-4.01
       comparison is ALWAYS quoted in this deconfounded form (the
       raw 36.22-vs-4.01 comparison is basis-unfair and is not
       quoted as an exceedance).
     - **SECONDARY (like-basis system pair, a finding in its own
       right):** at 60 GW, perfect foresight recovers only
       **~0.70 TWh** of the composed rule-based dispatch's own
       system waste (36.224 vs 36.929 TWh) — the waste is
       **absorption/geometry-limited, not dispatch-limited**. This
       INVERTS the current-fleet B4 headline ("dispatch-limited, not
       geometry-limited") at 60 GW, and is quotable with the
       conventions disclosure (the two legs differ by the PS de-dup
       and NO2-as-history, so ~0.70 is approximate, not a pinned
       dispatch premium).
     - **GB basis:** the one-sided rule-based ceiling **30.17 TWh**
       with caveat (l); the LP's GB-attributed 26.22 TWh is a
       degenerate vertex split, characterisation-only, never quoted
       as a floor. **No well-determined GB-basis floor exists in
       this record** — the record says so rather than manufacturing
       one.
  3. **Rule-based trade axes as ONE-SIDED disclosed bounds** —
     grounded on the committed under-wheeling direction (each hop
     halves the differential; surplus strands upstream): exports =
     **FLOOR**, curtailment = **CEILING**, net trade =
     most-pessimistic-for-exports. Quotable only with the anchor-red
     disclosure attached verbatim (+4.49 % gas / +18.1 %, +27.4 %
     imports — caveat (l)) *(R7 correction 2026-07-06: on the fixed
     engine the travel-verbatim values are +4.41 % gas / +18.2 % /
     +27.5 % — see caveat (l))*. **Asymmetric evidential rule, mandatory
     and verbatim:** a 60 GW rule-based net-export reading is
     evidence FOR export survival (a fortiori, through the artefact);
     a collapse reading is NOT evidence of collapse
     (artefact-confounded) — it leaves the question OPEN (wording
     amended from "to the bracket" per the package-2 adjudication,
     ruling B: no two-sided bracket exists on this axis).

  The LP's own gas/trade aggregates are **non-instruments** (ruling
  C: LP gas 160.2 TWh is the thermal-split degeneracy made concrete —
  gas IS the thermal split; and under the loss-as-waste term lossy
  imports are strictly dominated by free domestic thermal wherever
  headroom exists, so LP net imports measure **loss-minimising
  autarky**). They stay unpinned, reported once as diagnostics with
  mechanism, never repeated — caveat (m). There is currently **no
  composed capture instrument at all** (rule 5, ruling D).
- **(b) B4/B6 binding frequencies:** the committed b4-lp-findings
  convention — the **LP [floor, point] band is the quotable
  measurement**; the rule-based figure is the **disclosed myopic
  comparator** ("the rule-based flow convention, not the boundary, is
  what hides B4") and is **never named a central on this axis**.

The split is not an inconsistency: the (surviving) trade-axis bounds
answer "what does the committed dispatcher deliver, through its
disclosed artefact", the binding and min-waste axes answer "what does
the boundary force at physical optimum" — different questions, each
with its committed precedent (§G; b4-lp-findings; ruling C).
Full-year whole-horizon LP, not rolling: the horizon is the
measurement window (the recorded d12-mincurtailment methodology
divergence carries — this is a congestion/trade measurement, not
multi-year storage sizing).

**One new LP degeneracy class, disclosed up front — and its
treatment RULED (adjudication B(i), review edit 3):** the composed
scenario is the first MinCurtailment run with **lossy links** (the B4
measurement's internal links are lossless). The objective has no
link-flow or loss term, so shipping surplus into a curtailing
neighbour costs the objective **strictly less** than curtailing at
home (the objective changes by −x·loss; the 1.6–2.8 % link loss
vanishes from the accounting) — a loss-disposal channel that inflates
gross trade and shaves measured curtailment. **ADOPTED: the
loss-as-waste term** — link-loss energy joins the MinCurtailment waste
terms at weight 1, the exact analogue of the committed storage
round-trip-loss term (d12-mincurtailment term 3: disposal costs
exactly what curtailment costs; genuine use still nets a gain). It
lands at the **head of package 1**, before ANY composed LP run
(package 1 runs the composed-anchor LP legs, and the objective must
not change mid-measurement). Four conditions, mandatory:

- (a) **MinCurtailment only** — the MinUnserved oracle objective
  stays byte-for-byte unchanged;
- (b) the term is **skipped when `loss == 0.0`**, so the committed
  lossless family's objective is STRUCTURALLY byte-identical, not
  merely numerically equal;
- (c) **unmoved-pins gate**: `acceptance_b4_lp` re-run green (point
  and floor within the committed ±0.01) plus the full suite;
- (d) **red-first unit test on a hand-computable lossy fixture**:
  without the term the LP strictly prefers loss-disposal into a
  curtailing neighbour; with the term it does not, and genuine
  thermal-displacing wheeling still occurs.

**Precision (B(i), mandatory):** the term does not make gross trade
well-determined — it converts the strict −x·loss preference into an
exact **indifference**, so the export-into-curtailing-neighbour class
joins the ordinary degeneracy classes and STAYS under the
[floor, point] band discipline. The no-engine-change alternative
(quantified-band-only) lapses.

## Rule 5 — The capture price basis (the genuine fork, proposed not dodged)

Delivered capture needs a GB SMP series; tier-2 built it from the
single GB zone's `[zones.pricing]` chain over that zone's thermal
dispatch (`multi_zone_point_metrics`, D11 review §C: bit-identical
recipe to the pinned Q10/Q2 definitions). With three GB zones there is
a real fork:

- **(A) GB-aggregate SMP — ADOPTED (adjudication B(iv)).** One GB
  price series: the same grid-core recipe ("SRMC of the most expensive
  dispatched SRMC-bearing technology") applied to the **union of the
  three GB zones' thermal dispatch**. The aggregate recipe, stated
  mechanically and in full (review edit 9) so the parity claim is
  checkable at review:
  - per-technology thermal series **summed across NSCO/SSCO/RGB**
    (well-defined because the three zones carry the *identical*
    committed SRMC chain — realised in package 1 as
    per-zone-restricted srmc maps, chain identical, listing
    restricted to each zone's SRMC-bearing fleet because the v7
    validator requires it: NSCO ccgt / SSCO none / RGB ccgt+ocgt; an
    accepted, disclosed deviation, addendum E.2), feeding the
    existing single-zone `PricedSeries` construction;
  - **GB-aggregate unserved = the three-zone sum** — the second
    argument of `system_marginal_price`, so the unserved→ceiling
    convention fires on the aggregate exactly as it did on the single
    GB zone;
  - delivered wind = the per-zone pro-rata
    `delivered_renewable_power`, **summed**; potential = capacity ×
    CF, summed; capture = the committed quotient recipes
    (`capture_ratio`, `time_weighted_mean_price`,
    `price_setting_share`) on the aggregate series;
  - the helper is **pinned against a hand-computable fixture**
    (red-first) before it touches any composed result.

  **Comparability argument:** (i) it is the same definitional
  form the D11 review §C accepted for tier-2 vs tier-1 — same
  grid-core functions, same chain, richer dispatch (tier-2's own
  definitional note: gas dispatched for export sets GB's SMP; here,
  gas dispatched anywhere in GB sets GB's SMP); (ii) it is
  institutionally faithful — GB **is** one bidding zone in 2024, and
  constraint costs are socialised, so a single GB price is what the
  market actually pays. **Limits, stated:** the constraint enters
  capture only through the *physical* channel (delivered volumes,
  altered gas dispatch, altered trade) — the price itself remains
  copper-plate. Under real zonal pricing, trapped northern wind would
  earn a separated (lower) zonal price, so basis (A) **understates the
  capture hit** — the composed capture stays an upper-side estimate on
  the price-basis axis, and the note says so in the caveat block
  (rule 10).
- **(B) Per-zone SMPs with demand-weighted GB aggregation —
  REJECTED for the headline.** It is *not* definitionally comparable
  to 0.698 (a different setter set per zone plus a new free weighting
  convention — exactly the metric-parity break the D11 review treated
  as load-bearing), and it smuggles a zonal-pricing market design into
  a measurement of physics. Named instead as a **future sensitivity**
  (the REMA-shaped question: what would zonal pricing pay northern
  wind on this dispatch), not run here.

**The LP leg produces no prices, and reports NO capture under any
label (adjudication B(ii), review edit 2 — the draft's shadow-capture
diagnostic is SUPPRESSED).** Ground, recorded so the decision is
quotable: MinCurtailment carries no thermal-cost term and the LP
enforces no merit order — `ThermalUnit.ladder` is used only to sort
units and reject duplicates (`lp.rs:1236–1246`), with no constraint
ordering the rungs — so the split of thermal dispatch across
SRMC-bearing technologies, precisely the quantity "SRMC of the most
expensive dispatched technology" reads, is objective-degenerate: the
per-period price-setter would be a HiGHS vertex artifact, and reading
it would measure the solver's pivoting (the d12 decision's own
words). A "never quoted alone" label does not cure an
under-determined quantity; it circulates it.

**⚠ COMPOSED CAPTURE HEADLINE DELETED (package-1 adjudication,
ruling D; edit P2-1).** The original close of this rule ("the capture
axis carries the rule-based value only") is superseded by
measurement: the composed-anchor rule-based capture read **0.9585**
with gas price-setting **93.28 %** — the stranding artefact wearing a
price. Stranded surplus forces gas dispatch, gas sets the SMP in 93 %
of periods, and capture is pushed ABOVE the single-zone reference
0.941 while the committed tier-2 anchor read 0.895 — the wrong side
of **both** committed comparators, by mechanism. It measures the
dispatcher, not a market. That anchor value is
**diagnostic-only, permanently, on this family**: reported once in
the package report with the 93.28 % mechanism attached, **never
quoted, never pinned** as a capture record. With the shadow-capture
suppression above, this closes the set: **the composed family
currently has NO capture instrument** — no composed capture is
quoted on any leg, at any point. Caveat (e)'s **capture axis remains
OPEN** (rule 10); its named resolver is an **economic-dispatch
instrument** — the D8-class min-cost LP with per-zone SRMC chains, a
real engine package with its own design forks (cost coverage beyond
the gas-only recipe boundary, external price bases, £0-rung
degeneracy), assigned a **NEW decision number by Richard**,
explicitly NOT commissioned inside D13 and NOT blocking package 2.
Basis (A) above remains the adopted price-basis *definition* (its
helper and fixtures stand, reviewer-verified — addendum E.5) for
whenever a capture instrument exists on this family;
curtailment/trade/binding keep the re-scoped treatment of rule 4.

## Rule 6 — Sweep convention: proportional GB scaling, and the bias it carries

GB wind scales by **one shared national factor** f = target ÷
29.1 GW (the committed 14.4 onshore + 14.7 offshore fleet) applied to
the onshore and offshore entries of **all three GB zones**, preserving
the committed zonal splits (NSCO 4.107855+2.887883, SSCO
5.967825+0.186622, RGB 4.32432+11.625495 — the REPD-northing /
cluster-trace partition of the 3-zone data report) and each zone's
onshore/offshore mix. This is the Module 1 / tier-2 national-trace
convention carried: at every factor the capacity-weighted sum of the
zonal fleets equals the tier-2 national fleet, and the zonal CF traces
reconstruct the committed national trace within the pinned
reconstruction identity (3-zone header: max residual 2.4e-07 for the
nsco/ssco split of sco) — asserted in-test at composition, not
assumed. External fleets, demand and prices stay frozen at the 2024
basis (tier-2 convention, caveat (a) carried verbatim).

**The northward-shift caveat carries verbatim, with its direction
stated plainly:** proportional scaling holds the 2024 *geography* of
the fleet fixed. A real 60 GW fleet would be **more northern** than
the 2024 split (ScotWind and the northern offshore pipeline), putting
*more* wind behind B4/B6 than this convention does. The composed
measurement therefore still **understates the constraint effect** —
composed capture/exports remain upper-side estimates on this axis even
after caveat (e) is resolved. No fleet re-siting is attempted
(rule 9); the caveat is carried on every composed quote (rule 10).

## Rule 7 — LP tractability: the arithmetic, against the cap

`estimate_lp_variables` (`lp.rs:172`) counts, per period: one `gen`
per non-weather-driven fleet entry (budgeted entries included as
placeholders), 3 per store, 2 per zone, 2 per live link. For the
composed scenario over the 2024 horizon (17,568 half-hourly periods):

| Zone | dispatchable gens | stores ×3 | zone ±2 | per-period |
|---|---|---|---|---|
| NSCO | ccgt, hydro = 2 | 2 stores → 6 | 2 | 10 |
| SSCO | nuclear, hydro = 2 | 1 store → 3 | 2 | 7 |
| RGB | ccgt, ocgt, nuclear, biomass, coal, hydro = 6 | 2 stores → 6 | 2 | 14 |
| FR | nuclear, hydro(budget), biomass, coal, ccgt, ocgt = 6 | 0 | 2 | 8 |
| CONT-NW | nuclear, biomass, hydro, coal, ccgt = 5 | 0 | 2 | 7 |
| NO2 | hydro(budget) = 1 | 0 | 2 | 3 |
| DK1 | 0 (all traces) | 0 | 2 | 2 |
| IE-SEM | ccgt, coal, ocgt = 3 | 0 | 2 | 5 |
| links | 12 links × 2 | | | 24 |
| **total** | | | | **80** |

**80 × 17,568 = 1,405,440 variables — 56 % of `LP_VARIABLE_CAP`
(2,500,000). Under the cap; no fallback needed.** The LP leg as
actually run (rule 3 surgery: drop 2 pumped-hydro stores −6, convert
FR/NO2 budgeted hydro to traces −2) is **72 × 17,568 = 1,264,896
(51 % of cap)**.

Expected cost, from the d12-lp-tractability benchmarks (1-year 3-zone
probe: 0.35 M vars, 59.3 s, 1.11 GB; time ≈ O(n^1.41), memory ≈
linear at ~1.5 KB/variable over a ~0.57 GB floor): 1.26 M vars →
**~6 min and ~2.5 GB per solve** — between the clean 1- and 5-year
simplex points, far from the ~3.5 M-variable abort zone. The
measurement is **two single points (anchor + 60 GW), not a
bisection**, so the whole LP leg is ~15 min of wall time. The bench
note's warning that real fleets cost more per period than the probe is
partly absorbed by estimating in variables rather than periods; even a
×2 miss stays comfortably solvable. **Named fallback** (should the
estimate be badly wrong in practice): the d12-lp-tractability
binding-window/rolling precedent — noting that `run_multi_lp_rolling`
also rejects energy budgets (surgery order matters), that the composed
stores are small (hours-scale batteries, ≤13.4 GWh PS) so window
truncation error is bounded by store energy, and that a windowed
binding-frequency statistic must carry a stated window-seam
convention. The fallback is named, not designed: the arithmetic says
it is not needed.

The rule-based legs are cheap (`run_multi` full-year on 5 zones ran a
whole sweep; 8 zones is the same order — seconds per point).

## Rule 8 — Pre-registered acceptance criteria (before any 60 GW number is trusted)

Package 1 must pass all of these; a red stops the line (the D11
withhold-and-report discipline).

**(i) Anchor self-validation, composed vs the committed 5-zone A1
record.** Bit-identity is **NOT expected**: the composition changes
zonal geometry (GB's internal constraints now act, the observed
exogenous import trace is replaced by endogenous flows landing at
SSCO/RGB), so GB-aggregate quantities move. What IS expected, and why:
the internal boundaries bind rarely under rule-based dispatch (B4
1.95 %, B6 3.35 % of periods — committed 3-zone pins), and in those
periods they mostly relocate surplus/curtailment rather than destroy
supply, so the GB-aggregate movement must be small. Proposed
tolerances: **GB-aggregate gas within ±2 % of the committed 5-zone
anchor 71.797411 TWh; GB-aggregate net imports within ±5 % of
+35.935153 TWh** (both tier-2 run-report §2 values), AND the Stage 5
A1 gates pass outright in their own terms (gas ±5 % of 72.79 TWh;
imports ±10 % of 33.30 TWh). Justification for the widths: the energy
at stake in boundary-binding periods is bounded by (binding hours ×
boundary capability) ≈ 1 TWh-order, ≪ 2 % of GB gas; imports get the
looser band because the export channel now competes with wheeled-south
surplus at the RGB hub, a genuinely new interaction. Outside the
band → stop, diagnose, report; the tolerance is a pre-registered
expectation, not a re-pinnable knob.

> **⚠ MEASURED RED / ADJUDICATED (package-1, 2026-07-05 — review
> addendum ruling A; edit P2-1). The pre-registered text above is
> kept verbatim, never rewritten to pass.** The composed anchor
> missed the bands (gas +4.49 % vs the committed 5-zone anchor;
> imports +18.1 %) and **failed the outright A1 ±10 % imports gate
> (+27.4 % vs observed)**; gas passed A1 in its own observed-basis
> terms (+3.06 %, inside ±5 %). The red is adjudicated a **genuine
> measured finding about the instrument, not a composition defect**
> (identities all green; the composition moves *toward* the anchor
> relative to its standalone parent): the diagnosed cause is the
> committed equal-depth single-pass stranding artefact — 6.9046 TWh
> stranded in N-Scotland **with both links unbounded**
> (`PIN_REF_NSCO_COPPER`; "the cause is the equal-depth single-pass
> flow rule, not link capability") — and the 3-zone family was
> validated on boundary-local gates, never on national trade axes.
> **The tolerances are NOT re-pinned** (no honest re-pin exists:
> widening a pre-registered band after a miss is knob-turning, and
> the outright A1 gate fails anyway), so the composed rule-based leg
> is **NOT anchor-validated on national trade axes, full stop** —
> the rule-4 regime (a) premise is superseded (ruling C). What the
> composed anchor DOES validate, and the record may say so: the
> composition itself (identities, conservation, CF reconstruction),
> A1 gas in observed-basis terms, PS inertness, determinism, and the
> boundary-binding instruments. The deviation-shape pins stand as
> delivered; band re-entry is a re-adjudication event (addendum
> E.1).

**(ii) Composed rule-based B4/B6 binding at anchor vs the committed
3-zone pins.** Committed: B4 0.01950570122127684 (gate-(iii)
denominator; 1.96 % on the LP-mask convention — both stated), B6
3.35 %. **Expected direction with externals attached: UP, modestly,
same order of magnitude.** Why: the endogenous export channel drains
RGB surplus in exactly the £0-surplus periods where B4/B6 bind (GB
exported to IE in 87.7 % of observed periods; 986 cap-saturated export
periods at 60 GW in the tier-2 record), deepening the north→south
scarcity gradient that drives boundary flow; and the removed exogenous
import trace no longer pads southern supply in windy periods.
Pre-registered branches: (a) small increase — expected, proceed;
(b) decrease — composition defect suspected, stop and diagnose;
(c) order-of-magnitude jump — either a defect or a genuinely strong
export-pull mechanism: stop, characterise, report before any 60 GW
run. The composed-anchor **LP** B4 band vs the committed [23.5 %,
28.2 %] is reported under the same mask convention with the same
expected direction (UP: the LP gains export destinations for wheeled
surplus).

> **⚠ MEASURED RED (B4) / ADJUDICATED (package-1, 2026-07-05 —
> review addendum ruling B; edit P2-1). The pre-registered text
> above is kept verbatim.** The expected-UP registration was
> **unreachable by construction on the rule-based leg**: the
> diagnosis (verified bit-identical by the adjudicator — deleting
> all ten external links reproduces the composed B4/B6 rule-based
> binding exactly) shows that under `flow.rs` rule 6 (single pass,
> declaration order) B4 and B6 clear before any external border, so
> no external link can move them — the export-drain mechanism's
> economics are real but *inexpressible in the walk*. **The
> declaration order STANDS** (choosing an ordering because it
> reaches a pre-registered expectation is exactly the tuning the
> committed border-order ruling refused; single-pass
> order-precedence joins the committed dispatcher-fidelity limits as
> a disclosed property, caveat class (b′)/(c); an order sensitivity
> may someday run as a *named sensitivity*, never a silent switch).
> **Re-registered expectation, binding for package 2:** on this
> family, composed rule-based B4/B6 binding movement measures the
> **import-padding-removal surgery ONLY** — B4 DOWN
> (0.0195 → 0.0107: the removed mostly-negative `intirl` demand
> leaves SSCO less loaded) and B6 UP (0.0335 → 0.0385: the removed
> nine-column supply leaves RGB scarcer), both accepted as measured.
> The instrument that actually answers "does the export geometry
> load the boundaries" is the **LP leg**, which has no order
> dependence — and it answered at the anchor: composed B4 point
> **0.281346** vs the committed copper-external 0.2816 — **FLAT**, a
> healthy, quotable package-1 result: at the 2024 fleet, attaching
> the modelled external world does not move perfect-foresight B4
> binding.

**(iii) Conservation, per zone.** Composition identities asserted
mechanically in-test: the three GB zones' fleet/demand/storage sums
equal the committed GB totals (onshore 14.4, offshore 14.7, solar
18.7, ccgt 30.0, demand shares 0.03333+0.06767+0.899 = 1.0, PS scales
0.2617+0.7383 = 1.0, etc.); per-period zone energy balance and
link-flow ≤ capability via the existing property tests; the zonal CF
reconstruction identity (rule 6).

**(iv) Determinism.** Rerun bit-identity and parallel ≡ serial on the
rule-based leg; LP determinism per ADR-5 (single-threaded HiGHS) with
the b4-lp ±0.01 cross-platform tolerance convention on binding
statistics.

**(v) Pins.** New digests pinned for the composed scenario (per-zone
dispatch digests, links digest, scenario sha256); **all committed pins
unmoved** — the 3-zone, 5-zone, 2-zone, single-zone reference, B4/B6,
Package B, D11 sweep and B4-LP pins are all untouched files, and the
suite is re-run green as the check.

**Pre-registered outcome branches at 60 GW — RE-REGISTERED in the
valid instruments (package-1 adjudication, edit P2-2; the original
A/B/C registration was stated over instruments — a composed capture
value, a rule-based trade central, an LP trade reading — that the
package-1 rulings withdrew, so it is superseded PRE-RUN, before any
60 GW measurement existed).** The branches are now defined over the
ruling-C set: (LP minimum forced waste vs the copper-plate 4.01 TWh,
LP B4/B6 binding bands, the rule-based export FLOOR), with the
asymmetric evidential rule inside the definitions:

- **Branch A — the geometry forces the waste:** composed LP minimum
  forced waste at 60 GW **materially exceeds 4.01 TWh** (with the LP
  binding bands high — the geometry attached). Because the LP is the
  optimum, this is dispatch-independent and one-sided: the geometry
  NECESSARILY forces more waste than the tier-2 central reported,
  under ANY dispatch. Caveat (e)'s curtailment component resolves
  AGAINST the tier-2 level. Export survival is then decided only per
  the asymmetric rule: FOR survival if the rule-based floor still
  shows net exports; otherwise OPEN (a rule-based collapse is NOT
  evidence of collapse — artefact-confounded).
- **Branch B — the finding survives the geometry:** composed LP
  minimum forced waste at 60 GW is **at or near 4.01 TWh** (geometry
  forces no material extra waste), AND the rule-based export floor
  **still shows net exports** — evidence FOR export survival **a
  fortiori** (it survives even through the under-wheeling stranding
  artefact). Finding against the constraint-kills-exports hypothesis:
  B4/B6 at their 2024 capabilities do not bind enough of the 60 GW
  surplus to close the export channel.
- **Branch C — bounded split (the a-priori EXPECTED branch, given
  B4's 1.96 % vs [23.5 %, 28.2 %] structure):** LP minimum forced
  waste stays at or near 4.01 TWh while the rule-based leg collapses
  (exports gone / curtailment ceiling far above the LP floor). Per
  the asymmetric evidential rule, **the collapse reading is NOT
  evidence of collapse** — it is the artefact-confounded pessimistic
  bound. ~~The record is then the bracket quoted whole: curtailment ∈
  [LP min-waste, rule-based ceiling]~~ **(bracket sentence RETIRED —
  basis-mismatched from birth, an adjudicator-owned registration
  defect; package-2 ruling B: the basis-labelled convention of rule 4
  replaces it)**; export survival OPEN pending the economic-dispatch
  resolver (rule 5); the boundary-loading answer carried by the LP
  binding bands.
- **Anomaly branch (review edit 8 — the catch-all, carried through
  the re-registration):** the three branches are not logically
  exhaustive (the LP leg carries different conventions: PS de-dup,
  external hydro-as-history, loss-as-waste). Any outcome outside
  A/B/C — including an LP min-waste reading above the rule-based
  curtailment ceiling — **→ stop, characterise, report before
  anything is quoted**.

Whichever branch fires, the numbers are pinned at full precision, the
branch is named in the run report, and the framing goes to the
reviewer before anything is quoted (the D11 stop-and-report clause).

> **⚠ MEASURED AND ADJUDICATED — BRANCH A FIRES (package-2
> adjudication, 2026-07-06 addendum, rulings B–D; verdict
> ACCEPT-WITH-CONDITIONS).** The anomaly catch-all fired first in
> exactly its named shape (LP min-waste 36.224 TWh above the
> rule-based GB curtailment ceiling 30.175 TWh), the verdict was
> withheld as pre-registered, and the anomaly **closes as a
> registration defect, not an instrument failure**: the retired
> bracket was basis-mismatched (all-zone system waste vs GB-only
> curtailment; adjudicator-owned), and both like-basis orderings hold
> (system: LP 36.224 < rule-based 36.929; GB: LP vertex split
> 26.218 < ceiling 30.175 — asserted in-file, reviewer re-run). With
> ruling B in place, **Branch A — the geometry forces the waste** is
> measured true on its own axes:
>
> - the dispatch-independent waste exceedance is **decisive**:
>   wind-driven increment +24.03 TWh over the composed anchor
>   baseline 12.20 TWh — **+20.02 TWh above the 4.01 TWh copper-plate
>   central on the conservative deconfounded basis** (+32.22 raw,
>   quoted only with the basis disclosure);
> - the LP bands **load hard and UP** from the anchor, beyond the
>   ±0.01 convention on both quoted ends: **B4 [floor_internal 0.275,
>   point 0.571]** vs anchor [0.238, 0.281] — the point-end doubles,
>   and the band also WIDENS (much of the new binding sits in
>   downstream-curtailing periods; the floor is named on every
>   quote); **B6 [0.371, 0.388]** vs anchor 0.098 — **nearly
>   degeneracy-free: B6 binds in physics, not vertex choice, in
>   ≥ 37 % of masked periods at 60 GW**;
> - **export survival is OPEN** under the asymmetric evidential rule:
>   the artefact-conditioned rule-based floor reads **+11.87 TWh net
>   imports** *(R7 correction 2026-07-06: **+11.70 TWh** on the fixed
>   engine; reading unchanged)*, which is NOT evidence of collapse; no composed
>   instrument measures the trade level (the economic-dispatch LP
>   remains the named resolver).
>
> **Caveat (e)'s curtailment component resolves AGAINST the tier-2
> level; its capture and net-trade components remain OPEN.**
>
> **Invariant restated GB-SIDE (ruling C; adjudicator-owned guard
> shape corrected):** the all-zone LP ≤ rule-based unserved guard
> passed at the anchor only on the contingent premise that rule-based
> NO2 unserved sat above the LP floor. The **binding invariant is: GB
> zones carry zero unserved on both legs**; the all-zone comparison
> is a **characterised conventions wedge**, pinned in its measured
> shape — LP 785.086 vs rule-based 207.926 GWh all-zone unserved =
> **0.577 TWh**, entirely NO2, wind-independent (bit-equal at anchor
> and 60 GW): the ratified hydro-as-history conversion denies NO2 the
> within-week flexibility its committed 336-period budget gives the
> rule-based leg (FR shows no wedge because `window_periods = 1` is
> already a trace). Carried as a **named conventions finding** in the
> run report and as the caveat (i) extension (rule 10).
> **Threatened numbers: none, ruled explicitly** — the wedge is
> unserved, which the min-waste total excludes by construction; its
> waste-side analogue points UP on composed waste (the direction
> already ruled conservative), is largely absorbed into the anchor
> baseline by the deconfounded increment, and any residual is bounded
> by NSL's 1.4 GW throughput in surplus hours (order ≤ a few TWh) —
> two orders under the +32.2 and an order under the +20.0 margins.
> The finding stands with or without it.

## Rule 9 — What this does NOT do

- **No priced-ladder leg.** The d11-engine-review §G ruling stands:
  on 2024 prices the ladder's both-gas directions are convention
  noise and it fails the anchor gates. A priced-ladder composed run is
  a **named future sensitivity** for a year with a signed carbon
  wedge, nothing more.
- **No correlated-external-wind.** External zones stay 2024-frozen;
  the shared-weather sensitivity is the separate priority-2 item
  (d11-sweep-run-report §10.2), not this measurement.
- **No fleet re-siting.** The northward-shift bias is carried as a
  caveat (rule 6), not modelled.
- **No new data.** Composition of committed, checksummed inputs only;
  if any number cannot be traced to a committed file, it does not go
  in the scenario.
- **No B4-vs-B6 decomposition.** Obligation (2) of the 3-zone record
  carries: per-boundary binding frequencies and pinned totals under
  stated conventions, never a "B4 effect proper" %.
- **Not the Stage 7 storage sizing.** Full-year MinCurtailment ≠
  multi-year sizing; the d12-mincurtailment methodology-divergence
  note carries. The D12 step-3 remainders are unaffected.
- **No zonal-pricing market claim.** Basis (A) is a measurement
  convention; the zonal-price capture question is named future work
  (rule 5).

## Rule 10 — Quoting rules: what this resolves, what it inherits

**Resolution of D11 caveat (e) — PARTIAL, per the re-scoped
instrument set (package-1 adjudication ruling C; edit P2-5), with
the statuses now MEASURED (package-2 adjudication, 2026-07-06,
rulings B/D).** What the composed record supports, as measured:

- the **curtailment axis** — **RESOLVED AGAINST the tier-2 level**,
  quoted only under the ruling-B basis-labelled convention (rule 4:
  primary min system waste 36.22 TWh / composed baseline 12.20 /
  wind-driven increment +24.03, the vs-4.01 exceedance ALWAYS in the
  deconfounded +20.02 form; secondary like-basis ~0.70 TWh recovery
  finding — absorption/geometry-limited, not dispatch-limited; GB
  basis one-sided ceiling 30.17 TWh only — **no GB-basis floor
  exists**, the original "bracketed [LP min-waste, rule-based
  ceiling]" sentence is RETIRED as basis-mismatched, ruling B);
- the **boundary-loading question** — the LP binding bands with the
  geometry attached (B4 [0.275, 0.571]; B6 [0.371, 0.388], nearly
  degeneracy-free; every quote names its floor);
- **export survival** — governed by the asymmetric evidential rule;
  **measured OPEN**: the artefact-conditioned rule-based floor read
  +11.87 TWh net imports at 60 GW *(R7 correction 2026-07-06:
  **+11.70 TWh** on the fixed engine; reading unchanged)*, which is
  not evidence of
  collapse; the copper-plate −6.46 TWh net exports remains quotable
  only alongside this OPEN status.

What it **CANNOT** support: a composed capture value, a composed
net-trade central, or any **"0.698 becomes X" sentence**. Caveat (e)
on the **capture and net-trade axes remains OPEN**: the D11
run-report §4 text is amended (the package-2 addendum §E gives the
verbatim amendment) to say the composed measurement resolved the
curtailment/boundary components AGAINST the tier-2 level and that
the capture/net-trade resolver is an **economic-dispatch instrument**
(the D8-class min-cost LP with per-zone SRMC chains) — a real engine
package with its own design forks (cost coverage beyond the gas-only
recipe boundary, external price bases, £0-rung degeneracy), assigned
a NEW decision number by Richard, explicitly NOT commissioned inside
D13. The tier-2 0.698 / 4.01 TWh / −6.46 TWh *(R7 correction
2026-07-06: 0.6975 / 3.98 TWh / −6.46 on the fixed engine)* remain
quotable only
**alongside** the composed record's resolved components and OPEN
statuses (the copper-plate numbers become the disclosed
geometry-free comparators, exactly as the Package B bracket became
the disclosed convention band under tier 2). The bracket-escape
*direction* finding (tier-2 outside tier-1) was already ruled
copper-plate-consistent on both sides and is untouched.

**The composed record's own caveat block inherits:**

- **(a) frozen 2024 externals** — verbatim, bias UP on
  capture/exports;
- **(b′) dispatcher fidelity** — now carried as the two-leg band
  itself rather than a one-sided caveat; the −0.046 anchor capture
  wedge (multi-zone 0.895 vs single-zone 0.941, run-report §7) still
  conditions the level and is still not a subtractable constant;
- **(c) the R7 flow-walk stall** — carries to the rule-based leg
  (every border, internal and external, runs `equalising_flow`; more
  borders may mean more stall periods). Package 1 counts
  stall-signature periods at the composed anchor as a disclosed
  diagnostic with the ≤-bound convention of the committed record;
- **(d) pinned points only** — anchor and 60 GW (docs/05 rule 3);
- **(f) northward shift not represented** — verbatim from rule 6:
  the composed measurement still understates the constraint effect;
- **(g) B4 input biases** — DA-only/no-outturn-anchor, the §3 onshore
  split (~+31 % northern generation per unit) and §6
  offshore-commissioning wedge (~19 %), all pushing modelled B4
  binding UP (b4-lp-findings caveats, carried on every quote);
- **(h) B5 folded into the SSCO copper plate** — the 3-zone family's
  stated lower-bound posture on constraint severity carries;
- **(i) the LP-leg conventions** — PS de-dup + external-hydro-as-
  history (rule 3, both stated judgments) and the [floor, point]
  degeneracy band, which now also houses the loss-disposal class
  (rendered indifferent, not eliminated, by the adopted loss-as-waste
  term — rule 4). **Extension (package-2 ruling C): the NO2
  hydro-as-history convention carries a measured 0.58 TWh unserved
  floor in NO2, wind-independent** (bit-equal at anchor and 60 GW;
  the conversion denies NO2 its committed within-week budget
  flexibility; FR shows no wedge, its `window_periods = 1` budget is
  already a trace) — carried here AND as a named conventions-finding
  row in the run report; it threatens no quoted number (ruled
  explicitly, rule 8 verdict block). If the rule-8 inertness
  assertion finds the
  rule-based stores awake at 60 GW, the disclosed tier-2 double-count
  convention and its magnitude live here too (review edit 4);
- **(j) the single-GB-price capture basis** — basis (A) understates
  the zonal-pricing capture hit; it would make any future composed
  capture upper-side on this axis (rule 5 — note that after ruling D
  the family currently has NO capture instrument, so (j) conditions
  the price-basis *definition*, not a quoted number);
- **(k) boundary capability frozen at 2024 (review edit 5).** The
  B4/B6 capability traces (and the ETYS/HARETORIM reverse capacities)
  are the observed 2024 series while GB wind scales to 60 GW; a real
  60 GW system carries the planned reinforcements (B6 uprating, the
  eastern HVDC links), none modelled. Bias: **DOWN on composed
  capture/exports, UP on composed curtailment** — the counterweight
  to caveat (f), without which the caveat block would read as if
  every bias points the same way. (Branch B's "at their 2024
  capabilities" wording already implies it; it is carried explicitly
  here.);
- **(l) rule-based trade axes are artefact-conditioned (package-1
  ruling A; edit P2-4)** — the composed rule-based leg is NOT
  anchor-validated on national trade axes; every rule-based trade
  quote carries the anchor-red numbers verbatim (**+4.49 % gas /
  +18.1 % imports vs the committed 5-zone anchor; +27.4 % imports vs
  observed — the outright A1 miss**) *(R7 correction 2026-07-06: on
  the fixed engine the travel-verbatim values are **+4.41 % gas /
  +18.2 % / +27.5 %** — the acceptance_d13_composed comparator
  disclosure; the anchor-red verdict and the artefact-conditioning
  are unchanged)* and the one-sided-bounds reading of rule 4
  (exports = floor, curtailment = ceiling), under the asymmetric
  evidential rule;
- **(m) the LP is not a gas/trade/capture instrument (package-1
  ruling C; edit P2-4)** — LP gas is the thermal-split
  objective-degeneracy made concrete (the 160.2 TWh anchor diagnostic
  is vertex allocation, not a measurement), and under the
  loss-as-waste term lossy imports are strictly dominated by free
  domestic thermal wherever headroom exists, so LP net imports
  measure **loss-minimising autarky**; these aggregates stay
  unpinned, reported once as diagnostics with mechanism, never
  repeated — and the LP reports no capture (rule 5). The LP's valid
  outputs are the binding bands and the minimum-forced-waste optimum;
- **(n) floor_full over-excludes (package-1 ruling E.3; edit
  P2-4)** — the externals-included floor convention tests downstream
  curtailment without checking link saturation (a period where FR
  curtails behind saturated GB→FR links is not actually indifferent),
  so floor_full is a **deliberately loose lower bound on the artifact
  class, not a tight physics floor**. Every binding-band quote names
  its floor convention; the committed-comparable sentence ("B4 flat,
  0.2813 vs 0.2816") uses floor_internal/point.

## Package split (ratified — adjudication §A.10) and docs/08 row

**Two packages, the D8/D9/D11 rhythm — justified because the anchor
gates must adjudicate the composition before any 60 GW number
exists:**

1. **Scenario + pins package — DELIVERED AND ADJUDICATED (2026-07-05
   addendum; tree accepted for commit — the two reds are findings,
   not defects, and their deviation-shape pins are the record).** As
   ordered: opened with the **loss-as-waste term at its head**
   (review edit 3; all four conditions verified by the adjudicator —
   MinCurtailment only / skipped at `loss == 0.0` /
   `acceptance_b4_lp` + full-suite unmoved-pins gate green with the
   term in the tree / red-first lossy fixtures). Then
   `scenarios/gb-2024-8zone.toml` (composition per rules 1–3), the
   composition-identity and budget-conversion identity asserts, the
   anchor rule-based run against criteria 8(i)–(v) with the
   PS-inertness assertion, the composed-anchor B4/B6 binding
   measurements (rule-based + LP legs, both floors), new digests
   pinned (`regression_8zone`), committed pins verified unmoved.
   Gates 8(i)/8(ii)-B4 measured RED and adjudicated (rule 8
   annotations); the helpers were built here rather than in
   package 2 (disclosed deviation, accepted — addendum E.5).
2. **Measurement package — DELIVERED AND ADJUDICATED (2026-07-06
   addendum; ACCEPT-WITH-CONDITIONS, commit approved;
   `grid-adequacy/tests/acceptance_d13_60gw.rs`, the only file
   touched; suite 646/0/4, committed pins unmoved).** As
   commissioned (edit P2-3): the **60 GW rule-based leg** quoted as
   one-sided bounds with the mandatory disclosures (caveat (l)
   verbatim; asymmetric evidential rule); the **60 GW LP leg**
   quoting the B4/B6 binding bands under BOTH floor conventions
   (every quote naming its floor) and the **pinned
   objective-decomposition / total-waste quantity** (minimum forced
   waste primary; curtailment as the degenerate-channel band); the
   re-registered branch adjudication (rule 8 — the anomaly catch-all
   fired in its named shape, was withheld, and closed as the ruling-B
   registration defect; **Branch A fires**); NO composed capture on
   any leg (ruling D); and the run report
   **`docs/notes/d13-run-report.md`** per the addendum §E contents
   (conventions, both packages' records, full-precision tables, the
   ruling-B basis-labelled framing, the quotable sentences E.1–E.4
   with their caveat sets, caveat block (a)–(n) as amended + the NO2
   conventions-finding row, the D11 §4 amendment, hashes per
   docs/06) — written before any D13 number is quoted anywhere
   (addendum condition 1).

One package would save a gate but would let a 60 GW number exist
before the composition is validated — the exact failure mode the D11
anchor discipline exists to prevent. Two packages — RATIFIED by the
adjudication (§A.10), subject to its edit 3 (the loss term at the
head of package 1, applied above).

**Proposed docs/08 row (D13):**

> | D13 | Composed boundary-trade measurement: the 3-zone GB boundary
> family (B4/B6) joined to the 5-zone external set — the named
> resolver for D11 sweep quoting caveat (e) (does the 60 GW
> export/capture finding survive the measured Scotland–England
> constraints?) | Before the composed scenario/measurement packages;
> pulled ahead of the Stage 7 remainder (Richard, 2026-07-05) |
> **Resolved 2026-07-05: reviewer ADOPT-WITH-EDITS, all nine ordered
> edits applied** — loss-as-waste term adopted at the head of
> package 1 (four conditions incl. the unmoved-pins gate); LP shadow
> capture suppressed (thermal-split objective-degeneracy); two-regime
> band framing (trade axes: rule-based central; binding axes: LP
> [floor, point] per b4-lp-findings); PS inertness asserted at both
> points; budget conversion ratified with identity asserts; caveat (k)
> boundary capability frozen at 2024. **Package 1 delivered and
> adjudicated 2026-07-05 (review addendum): gates 8(i)/8(ii)-B4
> measured RED, ruled genuine findings about the instrument (the
> committed equal-depth single-pass stranding artefact; export-drain
> unreachable by construction under rule-6 order) — no re-pin, the
> deviation-shape pins are the record; composed rule-based leg NOT
> anchor-validated on national trade axes; package 2 COMMISSIONED
> RE-SCOPED (LP binding bands both floors, LP minimum forced waste
> with pinned objective decomposition, rule-based trade as one-sided
> bounds under the asymmetric evidential rule); capture axis of
> caveat (e) OPEN — resolver: the min-cost economic-dispatch LP (new
> decision row below, not scheduled); anchor LP result quotable: B4
> flat, 0.2813 vs 0.2816 (floor_internal/point).** Design
> `docs/notes/d13-composed-boundary-trade.md`; adjudication + addendum
> `docs/notes/d13-composed-boundary-trade-review.md` |

**Proposed NEW docs/08 decision row (number assigned by Richard —
edit P2-6; the addendum's ruling C resolver):**

> | D-NN (Richard to assign) | Min-cost economic-dispatch LP: the
> composed family's capture/trade instrument — an LP objective with
> real economics (per-zone SRMC chains), the named resolver for the
> OPEN capture axis of D11 caveat (e) after the D13 package-1 ruling
> that MinCurtailment cannot measure gas/trade/capture (thermal-split
> degeneracy; loss-term autarky) and the composed rule-based leg is
> not anchor-validated on trade axes. A real engine package with its
> own design forks: cost coverage beyond the gas-only recipe
> boundary, external price bases, £0-rung degeneracy | Before any
> composed capture number is quoted | **NOT SCHEDULED** — named
> resolver only (D13 package-1 adjudication, ruling C); explicitly
> not commissioned inside D13 and not blocking D13 package 2 |

## ADR touch-points (proposed amendments recorded here per CLAUDE.md; the ADR is not edited)

- **ADR-7 + 3-zone obligation (6) / design-review item 7** ("GB-internal
  and continental multizone stay SEPARATE scenario families in v1"):
  this note proposes the **v2 join** as a NEW scenario family
  (`gb-2024-8zone.toml`), leaving both committed families and all
  their pins untouched. Proposed amendment note: "v2 (D13): the
  composed family joins the two; the separate families remain the
  validated references."
- **ADR-12** (transmission as a cost approximation): unchanged — the
  boundary family models B4/B6 as explicit links (the committed
  Stage-5-era extension), not a network model; B5 stays folded.
- **ADR-6 / ADR-10**: both dispatch policies reported (the band IS the
  dual-policy discipline); the LP stays the D12 MinCurtailment
  objective. The one engine touch is the ADOPTED loss-as-waste term
  (rule 4, adjudication B(i)) under its four conditions — the
  committed-pin-safety argument (skip at `loss == 0.0`; structural
  byte-identity for the lossless family) is part of the work order,
  gated by the `acceptance_b4_lp` + full-suite re-run.
- **ADR-9 / schema**: no schema change — v7 per-zone pricing already
  carries everything rule 5 needs; the GB-aggregate metrics are
  additive library composition of existing grid-core functions. No
  docs/03 migration note required.
- **ADR-5**: determinism obligations as in rule 8(iv); no wall-clock,
  no globals, no randomness anywhere in the new code.
- **docs/05 known-boundaries list**: when next touched, the "No
  intra-GB network model" line gains a cross-reference to the
  boundary-family scenarios (a doc nicety, not a condition).

## Implementation shape (the work order)

Package 1 — **DONE and adjudicated** (executed as ordered: FIRST the
loss-as-waste term — red-first lossy fixture, the term, the
unmoved-pins gate, all four rule-4 conditions verified — then the
composed scenario, the failing-first acceptance test — composition +
budget-conversion identities, PS-inertness assertion, anchor gates
8(i)–(v) — the run, and the return to the reviewer gate per rule 8,
which fired: two reds, adjudicated in the addendum; the group-scaling
and aggregate-metrics helpers were built here for the determinism
gate, an accepted disclosed deviation, addendum E.5). Package 2 —
**DONE and adjudicated (2026-07-06)**: the re-scoped runs (edit
P2-3) executed — the 60 GW rule-based leg (one-sided bounds +
mandatory disclosures) and the 60 GW LP leg (binding bands under
both named floors; the pinned objective-decomposition / total-waste
quantity); the anomaly catch-all fired in its named shape and the
verdict was withheld exactly as pre-registered (the
withhold-the-report-until-ruled clause held), then closed as the
ruling-B registration defect; **Branch A adjudicated** (rule 8
verdict block). Remaining deliverable before any D13 number is
quoted anywhere: **`docs/notes/d13-run-report.md`** per addendum §E
(condition 1), plus the docs/08 row updates (condition 3) and the
withheld→adjudicated commit trail (condition 4).
