# D13 — Composed boundary-trade measurement: design adjudication

**Design adjudicator (D13 gate), 2026-07-05.** Adjudication of
`docs/notes/d13-composed-boundary-trade.md` (DRAFT) under the D11/D12
precedent: no code before the adjudicated work order. Method: verify,
don't trust — every load-bearing claim below was checked against the
committed record (scenario files, notes, `lp.rs`/`sweep.rs`/`flow.rs`
source, committed test pins), not against the draft's say-so. No code
exists yet, so no suite re-run is owed to this gate; the acceptance
tests are the ones this adjudication orders into existence.

## VERDICT: ADOPT-WITH-EDITS — nine ordered edits, three of them rulings that change the draft's proposals

The design is sound, honestly caveated, and correctly scoped as a
composition of committed data. The landing-point mapping is fully
supported by the committed record. The tractability arithmetic is
exact. Three of the draft's open forks are RULED here (not deferred):
the loss-as-waste term is IN (with conditions, and relocated to
package 1), the LP shadow-capture diagnostic is OUT (suppressed), and
the budget-conversion scope is ratified. One framing defect (the
two-leg band's "central" language) must be fixed before the note can
be quoted, because as drafted it can be quoted both ways.

## A. What was verified against ground truth

1. **Landing-point mapping — all ten links CONFIRMED.** The committed
   3-zone EXTERNAL INTERCONNECTORS convention
   (`scenarios/gb-2024-3zone.toml` header + SSCO/RGB `net_imports`
   blocks) enumerates exactly ten columns: `intirl` → S-Scotland
   (Moyle at Auchencrosh) and the other nine (`intelec`, `intew`,
   `intfr`, `intgrnl`, `intifa2`, `intned`, `intnem`, `intnsl`,
   `intvkl`) → RGB, NSL-at-Blyth named in both the RGB block comment
   and the d11-sweep-review addendum. The draft's table is one-for-one
   with the committed record; **no mapping is unsupported and no
   convention is invented**. Capacities/availabilities/losses match
   `gb-2024-5zone.toml` byte-for-byte (IFA 2.0/0.95/0.021 … Greenlink
   0.5/**0.0**/0.025). The Moyle→SSCO consequence (S-Scotland's only
   external outlet, 0.5 GW) and the NSL consequence (Norwegian
   exchange cannot relieve B6 directly) are stated correctly.
2. **Declaration order.** `flow.rs` rule 6 confirmed: borders dispatch
   sequentially in `[[links]]` first-appearance order, single pass.
   The proposed order (B4, B6, then the ten externals in committed
   5-zone order) is a disclosed departure from the rule-6 authoring
   guideline ("largest border first" — B6 at 4.1 GW and the FR border
   at 4.0 GW outrank B4's 1.8 GW): ACCEPTED — the guideline is a v1
   authoring convention, not a pin; the internal-cascade-first order
   is the committed 3-zone obligation-(4) cascade and the
   export-channel-faithful choice, and the draft charges the residual
   staleness to the dispatcher with the right sign (under-wheeling →
   overstates the constraint effect → conservative for the
   export-survives branch). One precision needed on the LP half of the
   claim — edit 7.
3. **Variable arithmetic — re-derived exactly.**
   `estimate_lp_variables` (`lp.rs:172`) counts one `gen` per
   non-trace fleet entry (budgeted entries as placeholders), 3/store,
   2/zone, 2 × `links.len()` when >1 zone. Recounted from the two
   committed files: NSCO 10, SSCO 7, RGB 14, FR 8, CONT-NW 7, NO2 3,
   DK1 2, IE-SEM 5, links 24 → **80/period; 80 × 17,568 = 1,405,440
   (56 % of `LP_VARIABLE_CAP` 2,500,000)**. Surgery −8 (two PS stores,
   two budget conversions) → **72 × 17,568 = 1,264,896 (51 %)**. All
   of the draft's numbers reproduce.
4. **Cost extrapolation — reproduces.** From the committed
   d12-lp-tractability clean points (0.35 M vars / 59.3 s / 1.11 GB;
   t ∝ n^1.41; ~30.6 KB/period ÷ ~20 vars/period ≈ 1.53 KB/var over a
   0.57 GB floor): 59.3 s × (1.2649/0.35)^1.41 ≈ 363 s ≈ **6.0 min**;
   0.57 + 1.2649 M × 1.53 KB ≈ **2.5 GB**. Far from the ~3.5 M-var
   abort. A ×2 miss on time/memory stays solvable; the variable count
   itself is exact, not an estimate. The rolling fallback's named
   hazards check out (`lp.rs:854–861` — rolling also rejects budgets).
5. **Cited pins and anchors — all committed.**
   `PIN_B4_CONSTRAINED_BINDING = 0.01950570122127684` and
   `PIN_B6_CONSTRAINED_BINDING = 0.03352507117541107`
   (acceptance_b4_3zone.rs); LP band point 0.2816 / floor 0.2346
   ±0.01 (acceptance_b4_lp.rs); anchor gas 71.797411 TWh / imports
   +35.935153 TWh and A1 gate bases 72.79/33.30 (d11-sweep-run-report
   §2); 60 GW central 0.6976839505365661 / 4.007462807827 /
   40.695234239837 / −6.456015207006; 986 cap-saturated periods;
   87.7 % IE export share; anchor wedge 0.894982…/0.941341…; the §F(i)
   five-caveat frame and the addendum's caveat-(e) text. All verbatim.
6. **De-dup precedent.** `acceptance_b4_lp.rs:257` drops
   `pumped_hydro` from **every** zone via `retain`, in memory, file
   untouched — the draft carries it verbatim. Correct.
7. **Budget rejection + observed traces.** `build_zone_data`
   (`lp.rs:1170`) rejects `energy_budget` with a structured
   `UnsupportedFeature`; FR (`window_periods = 1`) and NO2 (336) carry
   budgets reading `fr_generation_2024.parquet` /
   `no2_generation_2024.parquet` columns `hydro_reservoir` +
   `hydro_pumped` — the observed A75 traces are already the committed
   pack's, so the conversion needs **no new data**. The
   composition-not-new-data claim holds for the whole scenario: every
   number in rules 1–2 traces to one of the two committed files.
8. **The lossy-link degeneracy — confirmed, and it is a STRICT
   preference, not mere indifference.** From the LP balance
   (`lp.rs:582–624`): exports leave at full power, imports arrive
   × (1−loss), and MinCurtailment prices curtailment at 1 with no
   flow/loss term. Shipping x into a neighbour that is itself
   curtailing changes the objective by **−x·loss** — the LP strictly
   prefers burning the 1.6–2.8 % link loss to curtailing at home.
   The zero-coefficient pin-safety argument is **airtight**: the sole
   committed consumer of `MinCurtailment` is `acceptance_b4_lp.rs`
   (its own header: "the SOLE test on the MinCurtailment objective"),
   on the 3-zone scenario whose links both carry `loss = 0.0`; the
   lossy-link LP tests that exist (`lp_dispatch.rs:491` loss 0.05;
   `tractability_bench.rs:112` loss 0.02, `#[ignore]`d) run
   `run_multi_lp` = MinUnserved, which option (i) does not touch.
9. **Sweep gap.** `wind_capacity_sweep_multi` (`sweep.rs`) scales
   exactly one named zone (`apply_zone_wind_capacity`) and prices one
   zone's result — the zone-group scaling + GB-aggregate metrics
   helper is genuinely missing and genuinely additive.
10. **ADR touch-points.** ADR-7 is the multi-zone schema ADR and the
    committed 3-zone header itself files the separate-families rule as
    an "ADR-7 amendment note" — the draft's proposed v2-join amendment
    note (recorded, ADR not edited) is the correct CLAUDE.md posture.
    ADR-12 (B4/B6 as explicit links, B5 folded), ADR-9 (schema v7
    already carries per-zone pricing — a new scenario file is not a
    schema change, no docs/03 migration note), ADR-5/6/10 as stated.
    docs/08 row shape matches the committed table. Two-package split:
    RATIFIED (the anchor must adjudicate the composition before any
    60 GW number exists — the D11 discipline), subject to edit 3.

## B. Fork rulings (binding)

**(i) Loss-as-waste objective term — option (i) ADOPTED, with
conditions (edit 3).** It is the exact analogue of the committed
storage round-trip-loss term (d12-mincurtailment-decision term 3:
disposal costs exactly what curtailment costs; genuine use still nets
a gain): with the term, loss-disposal into a curtailing neighbour goes
from strictly preferred (−x·loss) to exactly indifferent (0), while
genuine wheeling that displaces thermal or serves load remains
strictly favoured (−x(1−loss)). The d12 note's deferred
"flow tie-break" precedent does NOT block it: that was an
arbitrary-weight cosmetic term that would have re-pinned every LP
surface; this is a physical waste term at weight 1 whose coefficient
is exactly zero on every committed MinCurtailment consumer (§A.8).
PRECISION, mandatory in the note: option (i) does not make gross trade
well-determined — it converts the strict bias into an indifference, so
the export-into-curtailing-neighbour class joins the ordinary
degeneracy classes and STAYS under the [floor, point] band discipline.
Option (ii) lapses.

**(ii) LP shadow capture — SUPPRESSED.** The draft proposed reporting
it as a labelled diagnostic; that proposal is REFUSED. Verified ground:
MinCurtailment carries no thermal-cost term, and the LP enforces no
merit order — `ThermalUnit.ladder` is used only to sort units and
reject duplicates (`lp.rs:1236–1246`); there is no constraint ordering
the rungs. The split of thermal dispatch across SRMC-bearing
technologies — precisely the quantity "SRMC of the most expensive
dispatched technology" reads — is therefore objective-degenerate: in a
period needing 1 GW of thermal the LP is indifferent between ccgt,
ocgt, coal or biomass carrying it, and the per-period price-setter is
a HiGHS vertex artifact. The D8/min-unserved precedent binds exactly
(the d12 decision's own words: reading an indifferent objective's
outputs "would measure the solver's pivoting"). A "never quoted alone"
label does not cure an under-determined quantity; it circulates it.
The capture axis carries the rule-based value only, with the
dispatcher caveat qualitative — the draft's own named fallback.
Curtailment/trade/binding keep the full two-leg treatment.

**(iii) Budget conversion vs cumulative-budget LP — conversion
RATIFIED as scoped** (in-memory, LP leg only, observed-as-history —
the identical posture to the committed PS de-dup and to FR's own
committed envelope language). The bias-direction claim is accepted as
argued: must-take external hydro cannot back off to absorb GB exports,
so the LP leg's export capacity is mildly understated — against the
export-survives branch, i.e. conservative for the finding under test;
carried in caveat (i) as a stated judgment, not a measurement. The
cumulative-budget LP constraint is a real engine package with its own
red-green cycle and does NOT block D13. Condition: edit 6's identity
asserts.

**(iv) Capture price basis (A) — ADOPTED.** Definitional
comparability verified: the same grid-core recipe
(`system_marginal_price` over `PricedSeries` built from dispatched
thermal) that the D11 review §C certified for tier-2, applied to the
union of the three GB zones' thermal dispatch, with all three zones
carrying the chain the §C review verified bit-identical to the
reference — the setter-set extension ("gas dispatched anywhere in GB
sets GB's SMP") is the same definitional move the review already
accepted for tier-2 ("gas dispatched for export sets GB's SMP"). The
one-bidding-zone institutional argument is factually sound for 2024
(constraint costs socialised; no zonal pricing). Basis (B) correctly
rejected for the headline (metric-parity break + smuggled market
design); the stated limit — the price stays copper-plate, so (A)
understates the zonal-pricing capture hit and composed capture is
upper-side on axis (j) — has the right direction. Condition: edit 9.

**(v) Two-leg band framing — SPLIT REQUIRED (edit 1).** The draft's
rule-4 sentence "each axis is quoted as the band [rule-based, LP] with
the rule-based value named the anchor-validated central" is correct
for the trade/capture/curtailment/gas axes (the caveat-(e) resolution
demands a like-for-like scarcity-rule comparator against 0.698, per
the d11-engine-review §G ruling that only the scarcity rule validates
at the anchor) — but applied to the B4/B6 **binding-frequency** axes
it contradicts the committed b4-lp-findings convention, under which
the rule-based figure is the measured myopic under-wheeler
(1.96 % vs [23.5 %, 28.2 %], "the rule-based flow convention, not the
boundary, is what hides B4") and the LP [floor, point] band is the
quotable measurement. As drafted, the note can be quoted both ways.

## C. Ordered edits (all mandatory; 1–3 are load-bearing)

1. **Rule 4, the framing split.** Restate the framing ruling as two
   explicit regimes: (a) capture / curtailment / net trade / gas — the
   [rule-based, LP] band, rule-based named the anchor-validated
   central and the like-for-like comparator for the caveat-(e)
   resolution sentence, LP the perfect-foresight optimistic bound, the
   band always quoted whole; (b) B4/B6 binding frequencies — the
   committed b4-lp-findings convention: the LP [floor, point] band is
   the quotable measurement, the rule-based figure is the disclosed
   myopic comparator and is NEVER named a central on this axis. State
   in one sentence why the split is not an inconsistency: the trade
   axes answer "what does the committed, anchor-validated dispatcher
   deliver", the binding axes answer "what does the boundary force at
   physical optimum" — different questions, each with its committed
   precedent (§G; b4-lp-findings).
2. **Rule 5, delete the shadow-capture diagnostic** (ruling B(ii)).
   Replace with: the LP leg reports no capture under any label; the
   capture axis carries the rule-based value with the dispatcher
   caveat qualitative. Record the suppression ground (thermal-split
   objective-degeneracy, no merit constraint in the LP) in one
   sentence so the decision is quotable.
3. **Rule 4 / package split: adopt option (i) and move it to the HEAD
   OF PACKAGE 1.** As drafted the loss term lands at the head of
   package 2, but package 1 already runs the composed-anchor LP legs —
   which would measure the anchor B4/B6 LP bands under the strict
   loss-disposal bias and then change objective mid-measurement.
   The term must be in place before ANY composed LP run. Conditions:
   (a) MinCurtailment only — the MinUnserved objective stays
   byte-for-byte unchanged; (b) the term is skipped when
   `loss == 0.0`, so the committed lossless family's objective is
   STRUCTURALLY byte-identical, not merely numerically equal;
   (c) mandatory unmoved-pins gate: `acceptance_b4_lp` re-run green
   (point/floor within the committed ±0.01) plus the full suite;
   (d) red-first unit test on a hand-computable lossy fixture: without
   the term the LP strictly prefers loss-disposal into a curtailing
   neighbour, with the term it does not (and genuine
   thermal-displacing wheeling still occurs); (e) the B(i) precision
   sentence — the disposal class becomes indifferent, not eliminated,
   and stays inside the degeneracy band.
4. **Rule 3, PS inertness: assert, don't assume.** The committed
   "harmless under rule-based" evidence is from the boundary family
   WITHOUT externals at the 2024 fleet. Require an in-test assertion
   of zero `pumped_hydro` cycling on the rule-based legs at BOTH
   points (anchor and 60 GW). If it holds, the harmless claim stands
   measured. If the stores wake at 60 GW, do NOT silently de-dup the
   rule-based leg (that would break like-for-like with the tier-2
   comparator, whose GB zone carried the identical double-count
   structure at 60 GW): disclose the active double-count as a carried
   tier-2 convention in caveat (i), and report its magnitude.
5. **Rule 10, add caveat (k): boundary capability frozen at 2024.**
   The B4/B6 capability traces (and the ETYS/HARETORIM reverse
   capacities) are the observed 2024 series while GB wind scales to
   60 GW; a real 60 GW system carries the planned reinforcements
   (B6 uprating, the eastern HVDC links), none modelled. Bias: DOWN on
   composed capture/exports, UP on composed curtailment — the
   counterweight to caveat (f), without which the caveat block reads
   as if every bias points the same way. (Branch B's "at their 2024
   capabilities" wording already implies it; the caveat block must
   carry it explicitly.)
6. **Rule 3, budget-conversion identity asserts.** The LP-leg
   conversion must assert in-test that the substituted FR/NO2
   must-take traces are per-period identical to the budget's own A75
   columns and that their annual sums reproduce the committed budget
   energies (FR 24.37 TWh; NO2 43.67 TWh), so the "observed operation
   as history" claim is mechanical, not editorial. (The FR pumping
   demand leg stays the committed `extra_profiles` trace, untouched —
   state this so the conversion is visibly one-sided.)
7. **Rule 1, LP order-invariance precision.** "Order matters only
   through the dispatcher's single-pass staleness, not the LP" is
   over-strong: the LP's optimum (objective value, non-degenerate
   quantities) is declaration-order-invariant, but degenerate-vertex
   flow statistics can move with variable/column order, which follows
   declaration order. Charge that to the existing degeneracy-band +
   ±0.01 conventions in one sentence rather than claiming outright
   LP order-invariance.
8. **Rule 8, add the anomaly branch.** A/B/C are not logically
   exhaustive once the LP leg carries different conventions from the
   rule-based leg (de-dup, hydro-as-history): an LP leg reading WORSE
   than rule-based on any axis is possible in principle and would
   indicate the conventions or a defect, not geometry. Pre-register
   the catch-all: any outcome outside A/B/C → stop, characterise,
   report before anything is quoted (the same clause the draft already
   applies to 8(ii) branch (c)).
9. **Rule 5, basis-(A) mechanical completeness.** State the aggregate
   recipe fully: per-technology thermal series summed across
   NSCO/SSCO/RGB (well-defined because the three chains are
   identical), GB-aggregate unserved = the three-zone sum (the second
   argument of `system_marginal_price`), delivered = summed per-zone
   pro-rata, potential = summed capacity × CF — and pin the helper
   against a hand-computable fixture (the draft's red-first clause
   already implies the fixture; the recipe must be explicit so the
   parity claim is checkable at review).

## D. Confirmations requiring no edit

- Acceptance tolerances 8(i) are defensible and discriminating: ±2 %
  of 71.797411 TWh ≈ ±1.44 TWh against a boundary-binding energy
  bound of ~1.5 TWh (337 × 1.8 GW × 0.5 h + 589 × 4.1 GW × 0.5 h),
  tighter than the outright A1 gates, with stop-and-diagnose on
  breach and no re-pinning. The 8(ii) expected-UP direction argument
  (export drain in £0 periods deepens the southward gradient; the
  removed exogenous import trace no longer pads southern supply) is
  correctly signed, and branches (a)–(c) prevent it becoming a
  self-confirming expectation.
- The inherited caveat block carries D11's (a)–(d) correctly, resolves
  (e) by measurement with the right precision (the bracket-escape
  direction stays copper-plate-consistent), and (f)'s
  northward-shift direction (still understates the constraint effect)
  matches the addendum. (g) matches b4-lp-findings caveats; (h)
  matches the 3-zone B5 posture; the b4-lp ±0.01 cross-platform
  convention is correctly housed in 8(iv).
- Rule 9's non-goals are the right fences (no priced ladder per §G, no
  B4-vs-B6 decomposition per obligation (2), no re-siting, no new
  data, not the Stage 7 sizing — the d12 methodology-divergence note
  carries).
- Determinism obligations (8(iv)) match ADR-5 and the committed LP
  conventions (`set_threads(1)`, parallel off, serial simplex).
- Proposed docs/08 D13 row and the ADR-7 amendment-note posture are
  correct as drafted.

With edits 1–9 applied, the design note becomes the work order;
package 1 returns to this gate per its own rule 8.

— design adjudicator (D13 gate), 2026-07-05

---

## ADDENDUM (2026-07-05) — Package 1 adjudication: the anchor reds and the instrument ruling

**Design adjudicator (D13 gate), 2026-07-05.** Package 1 delivered and
STOPPED THE LINE per rule 8: gates 8(i) and 8(ii)-B4 measured RED, the
verdict withheld, the measured state pinned in deviation shape (the D11
conversion precedent). This addendum is the ordered adjudication. All
rulings below are binding for package 2.

### Verification record (re-run, not trusted)

- **Tests re-run by me (release):** `acceptance_d13_composed` 10/10
  green (composition identities, both red-shape pins, both diagnosis
  tests, PS inertness, determinism + group-sweep reproduction, budget
  conversion identities, LP bands); `lp_loss_waste` 2/2 green;
  **`acceptance_b4_lp` green with the new loss term in the tree** — the
  edit-3(c) unmoved-pins gate holds; `regression_8zone` green (scenario
  sha256 + 8 zone digests + links digest). `cargo fmt --check` clean;
  `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **The 8(i) diagnosis has a committed basis I re-read**: the
  three-zone engine review's dispositive finding — the equal-depth
  single-pass rule strands **6.9046 TWh** in N-Scotland **with both
  links unbounded** (`PIN_REF_NSCO_COPPER`, reviewer-reproduced;
  "the cause is the equal-depth single-pass flow rule, not link
  capability"). The 3-zone family was validated on boundary-local
  gates, never on national trade axes. The implementer's in-test
  diagnosis (parent standalone gas further from the anchor than the
  composed run; parent curtailment ~7 TWh) asserts this mechanically
  and passed in my run.
- **The 8(ii) diagnosis is dispositive**: the decomposition test
  deletes all ten external links and reproduces the composed B4/B6
  rule-based binding **bit-identically** — under `flow.rs` rule 6
  (single pass, declaration order) B4 and B6 clear before any external
  border, so no external link can move them. Re-run green by me.
- **The loss-as-waste implementation** matches all four edit-3
  conditions: MinCurtailment-only, structurally skipped at
  `loss == 0.0`, red-first hand-computed fixtures (the disposal
  fixture's 2.4-vs-2.5 GWh arithmetic is the exact −x·loss bias I
  derived at adoption), the B(i) indifference precision carried in the
  module docs. The engine diff contains nothing else load-bearing.

### RULING A — Gate 8(i): the red stands; the tolerance derivation was mine and was wrong; no re-pin

**Owned correction.** My §D endorsement derived the ±2 % gas band from
boundary-binding energy (~1.5 TWh) alone. That bound was wrong: it
priced the *geometry* and missed the *dispatcher* — the committed
equal-depth stranding artefact, which lives at copper plate (6.90 TWh
with unbounded links) and therefore moves the composed GB trade axes
regardless of how rarely B4/B6 bind. The composed anchor inherits a
GB-side parent that was never anchored on national trade axes. The
derivation is corrected here, on the record.

**The ruling:** the red is accepted as a **genuine measured finding
about the instrument**, not a composition defect (identities all
green; the composition moves *toward* the anchor relative to its
parent). The tolerances are **NOT re-pinned** — the implementer was
right to refuse; widening a pre-registered band after a miss is
knob-turning, and no honest band exists anyway: composed net imports
fail the outright A1 ±10 % gate (+27.4 % vs observed), so the
composed rule-based leg is **not anchor-validated on national trade
axes, full stop**. What the composed anchor DOES validate, and the
record may say so: the composition itself (identities, conservation,
CF reconstruction), A1 gas in its own observed-basis terms (+3.06 %,
inside ±5 %), PS inertness, determinism, and the boundary-binding
instruments (ruling C). The deviation-shape pins stay as delivered;
the design note's 8(i) text is annotated MEASURED RED / ADJUDICATED,
never rewritten to pass.

**Supersession, explicit:** my adjudication edit 1 named the
rule-based leg "the anchor-validated central" on the trade axes on
the premise that it was "the only leg that validates at the anchor."
That premise is now **measured false on this scenario family**.
Edit 1 regime (a) is superseded by ruling C.

### RULING B — Gate 8(ii)-B4: diagnosis accepted; the declaration order STANDS; the expectation is re-registered

The unreachable-by-construction diagnosis is accepted (verified
bit-identical). The expected-UP registration was wrong on the
rule-based leg — I confirmed the export-drain mechanism's *economics*
at adoption without noticing that the adopted single-pass order makes
it *inexpressible in the walk*; owned alongside A.

**The declaration order is NOT revisited.** An externals-first order
would transmit export-pull to B6/B4 — but choosing an ordering because
it reaches a pre-registered expectation is exactly the tuning the
committed border-order ruling refused when it ratified B4-first over
the B6-first variant (which read "closer to observed" and was rejected
for precisely that reason). Single-pass order-precedence joins the
committed dispatcher-fidelity limits as a **disclosed property**
(caveat class (b′)/(c)); an order sensitivity may be run as a *named
sensitivity* someday, never silently switched.

**Re-registered expectation (binding for package 2):** on this family,
composed rule-based B4/B6 binding movement measures the
import-padding-removal surgery ONLY — B4 DOWN (0.0195 → 0.0107: the
removed mostly-negative `intirl` demand leaves SSCO less loaded) and
B6 UP (0.0335 → 0.0385: the removed nine-column supply leaves RGB
scarcer). Both accepted as measured. The instrument that actually
answers "does the export geometry load the boundaries" is the **LP
leg**, which has no order dependence — and it answered: composed-anchor
B4 point 0.281346 vs the committed copper-external 0.2816, **flat**.
That is a healthy, quotable package-1 result: at the 2024 fleet,
attaching the modelled external world does not move perfect-foresight
B4 binding.

### RULING C — The instrument question: option (i), re-scoped axes; the min-cost LP is a NAMED FUTURE RESOLVER, not a D13 blocker

The measured situation is accepted: on this family the rule-based leg
cannot express export-drain coupling (structural) and carries the
stranding artefact on trade axes (anchor red), while the MinCurtailment
LP's gas/trade aggregates are objective-degenerate — LP gas 160.2 TWh
is my B(ii) thermal-split degeneracy made concrete (gas IS the thermal
split), and under the loss-as-waste term lossy imports are strictly
dominated by free domestic thermal wherever headroom exists, so LP net
imports measure **loss-minimising autarky**. Both diagnoses are
correct; the implementer's decision to leave LP trade aggregates
unpinned was right (E.4). An objective with no economics cannot measure
trade volumes — that is not a bug in the loss term (without it the
same aggregates were vertex noise); it is the instrument's nature.

**Adopted: option (i), with the axes re-scoped as follows.**

Quotable composed axes for package 2 (60 GW + anchor):

1. **B4/B6 binding bands (LP)** — the sound instrument, both floor
   conventions, per ruling E.3.
2. **Minimum forced waste (LP)** — the well-determined optimum. The
   objective's optimal value is unique; its *components* (curtailment
   vs link-loss vs storage-loss) are mutually degenerate. Package 2
   pins the objective decomposition and quotes **total waste** as the
   primary min-waste quantity, with curtailment quoted as a band whose
   width is the degenerate loss channels (the d12 band discipline).
   This yields the strongest caveat-(e) statement available: if the
   composed LP minimum waste at 60 GW exceeds the copper-plate
   rule-based 4.01 TWh, the geometry NECESSARILY forces more waste
   than the tier-2 central reported, under ANY dispatch — one-sided,
   dispatch-independent, clean.
3. **Rule-based trade axes as ONE-SIDED disclosed bounds** — grounded
   on the committed under-wheeling direction (each hop halves the
   differential; surplus strands upstream): exports = FLOOR,
   curtailment = CEILING, net trade = most-pessimistic-for-exports.
   Quotable only with the anchor-red disclosure attached verbatim
   (+4.49 % gas / +18.1 %, +27.4 % imports). **Asymmetric evidential
   rule, mandatory:** a 60 GW rule-based net-export reading is
   evidence FOR export survival (a fortiori, through the artefact); a
   collapse reading is NOT evidence of collapse (artefact-confounded)
   — it leaves the question to the bracket.

**Caveat-(e) resolution language the re-scoped record CAN support:**
the curtailment axis (bracketed [LP min-waste, rule-based ceiling] —
a genuine two-sided resolution); the boundary-loading question
(binding bands with the geometry attached); export survival IF AND
ONLY IF the rule-based floor still shows it. **CANNOT support:** a
composed capture value, a composed net-trade central, or any "0.698
becomes X" sentence. Caveat (e) on the **capture axis remains OPEN**:
the run-report §4 text is amended to say the composed measurement
resolved the curtailment/boundary components and that the capture
component's resolver is an **economic-dispatch instrument** (the
D8-class min-cost LP with per-zone SRMC chains) — a real engine
package with its own design forks (cost coverage beyond the gas-only
recipe boundary, external price bases, £0-rung degeneracy), assigned
a NEW decision number for Richard, explicitly NOT commissioned inside
D13 and NOT blocking package 2. Option (ii) is rejected *as a D13
blocker* for cost and scope honesty, and preserved as the named
resolver — deferring the whole measurement behind it would trade a
partial, honest resolution now for an unscoped engine programme.

### RULING D — The anchor capture diagnostic: never quotable on this family

Composed-anchor rule-based capture 0.9585 with gas price-setting
93.28 % is the stranding artefact wearing a price: stranded surplus
forces gas dispatch, gas sets the SMP in 93 % of periods, and capture
is pushed ABOVE the single-zone reference 0.941 while the committed
tier-2 anchor read 0.895 — the wrong side of both committed
comparators, by mechanism. It measures the dispatcher, not a market.
**Diagnostic-only, permanently, on this family** — reported once in
the package report with the 93.28 % mechanism attached, never quoted,
never pinned as a capture record. With ruling C this closes the set:
the composed family currently has NO capture instrument.

### RULING E — The five implementer deviations

1. **Deviation-shape pins** (reds asserted in their measured shape;
   band re-entry = re-adjudication event): **ACCEPT** — the D11
   conversion precedent applied correctly; this is what stops silent
   rot in both directions.
2. **Per-zone-restricted srmc maps** (chain identical, listing
   restricted to each zone's SRMC-bearing fleet because the v7
   validator requires it; NSCO ccgt / SSCO none / RGB ccgt+ocgt):
   **ACCEPT** — a stated realisation of "identical in all three
   zones"; the composition test asserts recipe-identity against the
   committed GB chain and the group helper refuses conflicting traces.
3. **Two-floor convention** (floor_internal = committed-comparable;
   floor_full = externals included): **ACCEPT with one mandatory
   precision** — floor_full tests downstream curtailment without
   checking link saturation, so it OVER-excludes (a period where FR
   curtails behind saturated GB→FR links is not actually indifferent):
   floor_full is a deliberately loose lower bound on the artifact
   class, not a tight physics floor. Every quote names its floor
   convention; the committed-comparable sentence ("B4 flat,
   0.2813 vs 0.2816") uses floor_internal/point.
4. **LP trade aggregates unpinned (diagnostic eprintln only)**:
   **ACCEPT** — pinning them would dignify vertex artifacts; ruling C
   extends my B(ii) suppression to the LP gas/trade axes. The package
   report states the 160.2 / +9.6 diagnostics once, with mechanism,
   and they are never repeated elsewhere.
5. **Helpers built in package 1** (design placed them in package 2):
   **ACCEPT** — the anchor determinism gate needed the group helper at
   factor 1.0 (helper-vs-direct-run identity is asserted); the helpers
   are additive, red-first against hand-computed fixtures per edit 9
   (verified: the 18/23 and 19.5/23 capture fixtures are correct
   arithmetic), and the scope deviation was disclosed, not slipped.

### Package 2: BLOCKED confirmed — until these edits are applied, then commissioned re-scoped

The implementer's expectation is **confirmed**: package 2 does not run
until the coordinator applies the following and re-issues the work
order. It is **NOT** blocked behind any engine build.

Ordered edits (P2-1 … P2-6):

1. **Design note**: annotate rules 8(i)/8(ii) MEASURED RED /
   ADJUDICATED (link this addendum); supersede rule 4's regime (a)
   with ruling C's re-scoped axes; delete the composed capture
   headline from rule 5 (ruling D); carry the re-registered 8(ii)
   expectation (ruling B).
2. **Re-register the 60 GW branches** in the valid instruments: A/B/C
   restated over (LP min-waste vs 4.01, LP binding bands, rule-based
   export floor), the asymmetric evidential rule stated inside the
   branch definitions, the anomaly catch-all carried.
3. **Package 2 runs**: 60 GW rule-based leg (one-sided bounds +
   mandatory disclosures) and 60 GW LP leg (binding bands, both
   floors; pinned objective-decomposition / total-waste quantity).
4. **Caveat block additions**: (l) rule-based trade axes
   artefact-conditioned, anchor-red numbers verbatim; (m) the LP
   gas/trade/capture non-instrument statement (degeneracy +
   loss-term autarky mechanism); (n) the floor_full over-exclusion
   precision (E.3). Existing (a)–(k) carry.
5. **Run report / caveat-(e) text**: the CAN/CANNOT resolution
   language of ruling C verbatim; the capture axis stays open with
   the economic-dispatch resolver named.
6. **docs/08**: D13 row updated (package 1 adjudicated, reds ruled
   findings, package 2 re-scoped); a NEW decision row proposed for
   the min-cost-LP economic-dispatch instrument (number assigned by
   Richard), status NOT SCHEDULED.

The package-1 tree (scenario, pins, engine term, helpers, tests) is
accepted for commit as adjudicated here; the two reds are findings,
not defects, and their pins are the record.

— design adjudicator (D13 gate), addendum 2026-07-05

---

## ADDENDUM (2026-07-06) — Package 2 adjudication: the 60 GW record, the bracket convention, the branch verdict

**Design adjudicator (D13 gate), 2026-07-06.** Adjudication of the
delivered package 2 (`grid-adequacy/tests/acceptance_d13_60gw.rs`, the
only file touched). The anomaly catch-all's named shape fired and the
verdict was correctly withheld; it is ruled here. Rulings A–F, binding.

### A. Verification record (re-run and re-derived, not trusted)

- **Re-run by me (release):** `acceptance_d13_60gw` **10/10 green**;
  full suite **646 passed / 0 failed / 4 ignored** (matching the
  claim); `cargo fmt --check` and `cargo clippy --workspace
  --all-targets -- -D warnings` clean. Only the new test file is in
  the tree; every committed regression/acceptance file ran green, so
  committed pins are unmoved.
- **Independent probe (scratchpad crate, reviewer-typed statistics —
  my own scaling, my own LP surgery, my own waste/mask/binding
  arithmetic):**
  - LP minimum forced waste at 60 GW = **36.223998954 TWh**
    (curtailment 35.638937973 + storage 0.046226070 + link
    0.538834911) — matches the pin to 1e-9;
  - B4 LP point count **9,845/17,235**; B6 **6,612/17,042** — exact;
  - Rule-based net trade re-derived by an INDEPENDENT recipe (sum of
    GB-side link-end series over the ten external links, not
    `net_imports_energy`): **+11.868791001 TWh net imports** — sign
    and value confirmed; GB curtailment ceiling 30.174654042 confirmed.
- **Pin self-consistency hand-checked:** waste components sum to the
  total (1e-12); zonal stranding sums to the ceiling exactly
  (24.917 + 5.114 + 0.143 = 30.175; shares 82.6/16.9/0.5 %); gross
  imports − gross exports = the net-imports pin exactly
  (35.727 − 23.859 = 11.869); all band fractions match their pinned
  counts; anchor baseline components sum (12.196896137008); the
  deconfounded increment is 36.224 − 12.197 = **24.027**, i.e.
  **+20.020** above the 4.007 comparator.
- The module banner carries the asymmetric evidential rule verbatim,
  the caveat-(l) numbers verbatim, and ruling D's no-capture rule
  (the helper's capture fields are not read). Confirmed by reading.

### B. The bracket convention — RULED; the registration defect was mine

**Owned:** ruling C's registered bracket "curtailment ∈ [LP min-waste,
rule-based ceiling]" put an ALL-ZONE SYSTEM-WASTE quantity at one end
and a GB-ONLY CURTAILMENT quantity at the other. It was
basis-mismatched as registered, and the inversion (36.224 > 30.175)
is that mismatch surfacing — verified NOT an optimality anomaly: on
the like basis the LP optimum sits below the rule-based system waste
(36.224 < 36.929), and on the GB basis the vertex split sits below
the ceiling (26.218 < 30.175), both asserted in-file and re-run by
me. The registered bracket sentence is **retired**.

**The quotable convention (binding):** every quoted number carries its
basis label — *GB curtailment* or *system waste* — and the record
quotes THREE things, never a single bracket:

1. **PRIMARY (dispatch-independent, system basis):** composed minimum
   forced system waste at 60 GW = **36.22 TWh** (perfect-foresight
   floor, any dispatch), against the composed 2024-fleet baseline
   **12.20 TWh** — a wind-driven increment of **+24.03 TWh**, which
   exceeds the copper-plate tier-2 curtailment central (4.01 TWh) by
   **+20.02 TWh even on the conservative deconfounded basis** (the
   entire anchor baseline subtracted, including its GB component).
   The vs-4.01 comparison is always quoted in this deconfounded form
   (the raw 36.22-vs-4.01 comparison is basis-unfair and is not
   quoted as an exceedance).
2. **SECONDARY (like-basis system pair, a finding in its own right):**
   at 60 GW, perfect foresight recovers only **~0.70 TWh** of the
   composed rule-based dispatch's own system waste (36.224 vs
   36.929 TWh) — the waste is **absorption/geometry-limited, not
   dispatch-limited**. This INVERTS the current-fleet B4 headline
   ("dispatch-limited, not geometry-limited") at 60 GW, and is
   quotable with the conventions disclosure (the two legs differ by
   the PS de-dup and NO2-as-history, so ~0.70 is approximate, not a
   pinned dispatch premium).
3. **GB basis:** the one-sided rule-based ceiling **30.17 TWh** with
   caveat (l); the LP's GB-attributed 26.22 TWh is a degenerate
   vertex split, characterisation-only, never quoted as a floor.
   **No well-determined GB-basis floor exists in this record** — the
   record says so rather than manufacturing one.

### C. The NO2 conventions wedge — disclosed both ways; threatens no quoted number; invariant restated GB-side

The characterisation is verified and accepted: the wedge (LP 785.086
vs rule-based 207.926 GWh all-zone unserved = **0.577 TWh**) is
entirely NO2, wind-independent (bit-equal at anchor and 60 GW), and
is exactly the hydro-as-history judgment made visible — the ratified
conversion denies NO2 the within-week flexibility its committed
336-period budget gives the rule-based leg (FR shows no wedge because
`window_periods = 1` is already a trace). Disposition:

- **Caveat (i) is extended** with the measured cost ("the NO2
  hydro-as-history convention carries a measured 0.58 TWh unserved
  floor in NO2, wind-independent"), AND the run report carries it as
  a named **conventions-finding row** — both, as asked.
- **Threatened numbers: none, ruled explicitly.** The wedge is
  unserved, which the min-waste total excludes by construction; its
  waste-side analogue (denied NO2 absorption flexibility) points UP
  on composed waste — the direction already ruled conservative — is
  largely absorbed into the ANCHOR baseline by the deconfounded
  increment, and any residual on the increment is bounded by NSL's
  1.4 GW throughput in surplus hours (order ≤ a few TWh), two orders
  under the +32.2 and an order under the +20.0 margins. The finding
  stands with or without it.
- **The design-note invariant is restated GB-side** (owned: the
  all-zone LP ≤ rule-based guard was my package-1 anomaly-guard
  shape, and it passed at the anchor only on the contingent premise
  that rule-based NO2 unserved sat above the LP floor). The binding
  invariant is: GB zones carry zero unserved on both legs; the
  all-zone comparison is a characterised conventions wedge, pinned in
  its measured shape (as the file already implements).

### D. Branch verdict — BRANCH A FIRES; the anomaly resolves as a registration defect, not an instrument failure

With ruling B replacing the bracket convention, the anomaly
catch-all's named shape is fully characterised (accounting-basis
mismatch; both like-basis orderings hold) and **closes**. Branch A's
conditions are measured true on its own axes and the record carries
**Branch A — the geometry forces the waste**:

- the dispatch-independent exceedance is decisive (+20.02 TWh
  conservative; +32.22 raw with basis disclosure);
- the LP bands load hard and UP from the anchor beyond the ±0.01
  convention on both quoted ends — B4 [floor_internal 0.275, point
  0.571] vs anchor [0.238, 0.281] (the point-end doubles; the band
  also WIDENS — much of the new binding sits in downstream-curtailing
  periods, floor named on every quote); B6 [0.371, 0.388] vs anchor
  0.098 — **nearly degeneracy-free: B6 binds in physics, not vertex
  choice, in ≥ 37 % of masked periods at 60 GW**;
- export survival is **OPEN** under the asymmetric evidential rule:
  the artefact-conditioned rule-based floor reads +11.87 TWh net
  imports, which is NOT evidence of collapse; no composed instrument
  measures the trade level (the economic-dispatch LP remains the
  named resolver).

Caveat (e)'s **curtailment component resolves AGAINST the tier-2
level**; its capture and net-trade components remain OPEN.

### E. The quotable sentences, the D11 §4 amendment, and the run-report contents

**E.1 Curtailment/waste (the caveat-(e) curtailment resolution):**

> The tier-2 60 GW curtailment central (4.01 TWh, GB copper plate)
> does not survive composition with the measured B4/B6 boundaries and
> the modelled 2024 external system. The composed minimum forced
> SYSTEM waste at 60 GW is 36.22 TWh under ANY dispatch
> (perfect-foresight floor; composed 2024-fleet baseline 12.20 TWh;
> wind-driven increment +24.03 TWh — at least +20.0 TWh above the
> copper-plate central on the conservative deconfounded basis). On
> the GB-curtailment basis the composed rule-based ceiling is
> 30.17 TWh (one-sided, caveat (l)).

Mandatory caveats on every quote: basis labels (ruling B); (d) pinned
points only; (f) northward shift — understates the constraint effect;
(k) 2024 boundary capability — overstates it; (i) incl. the NO2
wedge; (l) verbatim for any rule-based number; (n) for any floor_full.
If the ±% -of-added-wind framing is wanted, the run report computes
and pins the added-potential denominator first — no unpinned ratio is
quoted.

**E.2 Boundary loading:**

> At 60 GW the perfect-foresight LP binds B4 in [0.275, 0.571] of
> masked periods (floor_internal, point; floor_full 0.060 is the
> caveat-(n) loose bound) against [0.238, 0.281] at the current
> fleet, and B6 in [0.371, 0.388] against 0.098 — the B6 band is
> nearly degeneracy-free, so the Anglo-Scottish boundary binds in
> physics in over a third of periods at 60 GW.

**E.3 Export survival:**

> OPEN. The composed rule-based floor reads +11.87 TWh net imports at
> 60 GW; under the pre-registered asymmetric evidential rule this is
> not evidence of collapse (the dispatcher is artefact-conditioned
> and structurally cannot express export-drain wheeling). No composed
> instrument measures the trade level; the economic-dispatch LP is
> the named resolver. The copper-plate −6.46 TWh net exports remains
> quotable only alongside this OPEN status.

**E.4 Capture:** unchanged (package-1 ruling D): no composed capture
instrument exists; 0.698 remains an upper-side estimate on the
capture axis of caveat (e).

**D11 run-report §4 amendment (append to caveat (e)):**

> PARTIALLY RESOLVED BY MEASUREMENT (D13, `d13-run-report.md`): on
> the curtailment axis the composed measurement resolves AGAINST the
> tier-2 level — minimum forced system waste 36.22 TWh at 60 GW
> (wind-driven increment +24.03 TWh over the composed anchor
> baseline; ≥ +20.0 TWh above the 4.01 TWh central on the
> conservative deconfounded basis); the 4.01 TWh copper-plate figure
> is quotable only alongside that record. The capture and net-trade
> components remain OPEN (no composed instrument; named resolver: the
> economic-dispatch LP, docs/08). Export survival: OPEN under the
> asymmetric evidential rule — the composed rule-based floor reads
> net imports, which is not evidence of collapse.

**Run-report contents (docs/notes/d13-run-report.md, written after
this ruling, before anything is quoted anywhere):**

1. What was run + every convention (composition citations, the shared
   scaling factor, LP surgeries, the loss-as-waste term, masks, both
   floor conventions, the basis-label rule).
2. The package-1 anchor record including both adjudicated reds and
   their diagnoses (quoted from the addendum, not re-litigated).
3. Full-precision 60 GW tables: rule-based one-sided bounds + zonal
   stranding split + gross flows + per-link saturation + rule-based
   B4/B6 binding; LP waste decompositions (anchor + 60 GW) with the
   degenerate-split labelling; LP bands (both boundaries, both
   floors, anchor + 60 GW).
4. The branch adjudication narrative: anomaly fired → basis mismatch
   (registration defect, adjudicator-owned) → ruling B convention →
   Branch A verdict; the like-basis ~0.70 TWh near-dispatch-
   independence finding with its conventions disclosure.
5. The quotable sentences E.1–E.4 verbatim, each with its mandatory
   caveat set.
6. The caveat block (a)–(n) as amended, plus the NO2
   conventions-finding row and the caveat-(m) one-time LP diagnostics
   (gas/net-imports vertex values, stated once with mechanism).
7. The D11 §4 amendment text above, and the supersession notes
   (retired bracket sentence; GB-side invariant restatement).
8. Reproducibility per docs/06: engine git hash, scenario sha256
   (23d51777…), pack manifest hashes, suite state 646/0/4,
   reproduce commands.
9. Onward work: the economic-dispatch LP (capture/net-trade
   resolver; docs/08 row, number assigned by Richard); the
   declaration-order named sensitivity; northward-shift re-siting;
   boundary-capability (caveat (k)) scenarios.

### F. VERDICT: ACCEPT-WITH-CONDITIONS — commit approved

The package is correct, honestly reported, and disciplined: the
anomaly was withheld exactly as pre-registered, both its
characterisation assertions verify, and every load-bearing number
reproduced in my independent probe. Conditions (none touches the
delivered code):

1. **Write `docs/notes/d13-run-report.md` per E** before any D13
   number is quoted anywhere (the D11 condition-2 discipline).
2. **Design-note amendments**: retire the registered bracket sentence
   (ruling B convention in its place, including the trailing "leaves
   the question to the bracket" of the asymmetric rule → "leaves the
   question OPEN"); restate the unserved invariant GB-side with the
   conventions-wedge row (ruling C); record the Branch A verdict and
   the OPEN statuses (ruling D).
3. **docs/08**: D13 row → measured/adjudicated; add the
   economic-dispatch-LP resolver row (number Richard's).
4. **Commit hygiene**: preserve the withheld→adjudicated trail
   (package-2 commit references this addendum; project-state session
   entry lands with it).

Deviations, adjudicated each: (1) bracket inversion — a registration
defect (adjudicator-owned), resolved by ruling B; the deviation-shape
pins stay. (2) all-zone unserved invariant failure — accepted as a
characterised conventions finding (NO2, wind-independent); GB-side
invariant holds; caveat (i) extended. (3) the anchor min-waste
decomposition pinned in this file — accepted (ruling C named the
instrument quotable at "60 GW + anchor"; package 1 did not pin it; no
committed pin is touched). (4) the composed-anchor rule-based run
re-executed diagnostics-only — accepted (needed by the wedge
characterisation; its pins remain in the package-1 file).

— design adjudicator (D13 gate), addendum 2026-07-06
