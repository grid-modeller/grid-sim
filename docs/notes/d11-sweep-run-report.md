# D11 tier-2 sweep — multi-zone wind-capacity run report (the finding record)

**Status:** measured record, 2026-07-05, reviewer-gated
(ACCEPT-WITH-CONDITIONS, finding ADJUDICATED —
`docs/notes/d11-sweep-review.md`; every number below reproduced
bit-for-bit by the reviewer's independent probe, the headline
capture/SMP arithmetic recomputed with an independent implementation).
Work order: `docs/notes/d11-priced-dispatch.md` rule 2 (the sweep runs
multi-zone) and rule 4 (the 60 GW central estimate pinned against the
Package B bracket). Every number in this note is pinned in
`grid-adequacy/tests/acceptance_d11_sweep.rs`. **Read §4 before
quoting anything — the review §F(i) frame is binding.**

> **CORRECTION (2026-07-06, R7 flow-walk stall fix — docs/08 R7,
> RESOLVED; adjudication `docs/notes/r7-fix-review.md`).** The §6
> defect is fixed; the movements land exactly as §B.4 of the sweep
> review bounded, AGAINST the finding, which stands unchanged on
> every axis. Re-measured on the fixed engine (all re-pinned in
> `acceptance_d11_sweep.rs` with old values recorded per pin):
> **§3 central estimate** — curtailment 4.007462807827 →
> **3.982736889304 TWh** (−0.0247, within the ≤ 0.025 bound);
> delivered capture 0.697684 → **0.697489** (−0.000195, within the
> ≤ 0.0002 bound); potential 0.681637 → 0.681545; gas 40.695 →
> 40.670 TWh; net imports −6.456 → **−6.463 TWh** (exports rise);
> gas price-setting 64.25 → 64.19 %; mean SMP £51.24 → £51.20/MWh.
> **§3 ladder sensitivity** — delivered 0.678412 → 0.678182;
> potential 0.662807 → 0.662679; curtailment likewise
> 3.982736889304 (still 12-dp-identical to the scarcity central —
> the degradation guarantee held through the fix); gas 40.030 →
> 40.023; net imports −5.601 → −5.626; price-setting 56.96 →
> 56.89 %; SMP £45.33 → £45.28.
> **§2 anchor** — GB net imports +35.94 → **+36.03 TWh** (+8.19 %)
> and gas 71.80 → **71.70 TWh** (−1.50 %); both A1 gates still PASS.
> **What did NOT move:** the single-zone reference record
> (779d7444…, capture 0.9413…), the Package B tier-1 pins, and the
> finding's position (outside the tier-1 envelope on every axis).

## 1. What was run, and the conventions

The Module 1 wind-capacity sweep on the multi-zone engine
(`grid_adequacy::wind_capacity_sweep_multi` over `run_multi`,
committed 5-zone scenario `scenarios/gb-2024-5zone.toml`), so GB
imports respond **endogenously** to the swept fleet — the tier-2 fix
of the tracked frozen-imports-under-sweep deviation.

- **Only the GB wind fleet scales** (onshore + offshore,
  proportionally from the committed 14.4/14.7 GW split, the Module 1
  convention; anchor = the exact committed 29.1 GW fleet at scale
  factor 1.0).
- **External fleets are NOT projected.** Every external zone (FR,
  CONT-NW, NO2, DK1, IE-SEM) keeps its committed 2024 fleet, demand,
  traces, budgets and (SAP/EUA) prices at every swept capacity. The
  sweep answers "60 GW of GB wind dropped into the **2024** European
  system"; the externals' response is purely operational
  (redispatch/displacement through the flow rule).
- **The central estimate runs the SCARCITY rule** — the
  d11-engine-review §G BINDING ruling: the scarcity rule is the
  configuration that passes the Stage 5 A-gates at the 2024 anchor;
  the priced ladder fails A1/A2a/A2b/A3/A4-BE there and its both-gas
  flow directions on 2024 prices are convention noise
  (d11-a2a-mismatch-characterisation.md §3/§5). The ladder appears in
  §3 as a **named sensitivity only**.
- Metric definitions match the pinned single-zone Q10/Q2 sweep
  exactly (verified bit-for-bit by the review §C: the GB
  `[zones.pricing]` SRMC chain is value-identical to the reference
  `[pricing]` chain; SMP/capture/setter/mean recipes are the same
  grid-core functions). Determinism: parallel ≡ serial ≡ rerun,
  asserted at unit and acceptance scale; reproduced cross-process by
  the reviewer.

## 2. Anchor self-validation (passed BEFORE any swept number was trusted)

The sweep's anchor point (factor exactly 1.0) equals the committed
5-zone `run_multi` **bit-for-bit** (`==` on the newtypes: imports,
gas, curtailment, unserved — the acceptance_b4_lp discipline), and
reproduces the committed Stage 5 A1 record through the sweep path:

| Anchor quantity | measured | A1 gate | result |
|---|---|---|---|
| GB net imports | +35.935152502942 TWh | ±10 % of 33.30 TWh | **+7.91 % PASS** |
| GB gas (ccgt+ocgt) | 71.797411264632 TWh | ±5 % of 72.79 TWh | **−1.36 % PASS** |
| Bit-identity with committed run | equal on all compared series | — | **PASS** |

## 3. Results at 60 GW (full precision — the pinned values)

**Central estimate (scarcity rule, endogenous imports):**

| Quantity | value |
|---|---|
| Delivered capture | **0.6976839505365661** |
| Potential capture | 0.6816365528136847 |
| Curtailment | **4.007462807827 TWh** |
| Gas (ccgt+ocgt) | **40.695234239837 TWh** |
| GB net imports | **−6.456015207006 TWh** (net exporter) |
| Gas price-setting share | 64.247495446266 % |
| Mean SMP | £51.241226229505/MWh |
| Unserved | 0 GWh |

**Priced-ladder sensitivity (same 60 GW point, in-memory
`flow_signal` flip — the established precedent; committed scenario
stays on the scarcity default):**

| Quantity | value |
|---|---|
| Delivered capture | 0.6784115295781239 |
| Potential capture | 0.6628074159582596 |
| Curtailment | 4.007462807827 TWh |
| Gas (ccgt+ocgt) | 40.030291928817 TWh |
| GB net imports | −5.601225528878 TWh |
| Gas price-setting share | 56.955828779599 % |
| Mean SMP | £45.331356717144/MWh |

The ladder numbers are a **dispatch-convention sensitivity, not a
second central estimate** (characterisation-note §3/§5 caveat: on
2024 prices the ladder's both-gas flow directions are decided by a
sub-noise, sign-flipping carbon wedge, and the ladder fails the
Stage 5 A-gates at the anchor). Two readings from the row: (i) the
signal choice moves delivered capture by only −0.019, so the §4
finding is not signal-dependent; (ii) the ladder's curtailment is
**identical to the scarcity central to 12 dp** — curtailment lives in
£0-surplus periods where the ladder degrades to the scarcity rule by
construction (the graceful-degradation property, asserted in-test).

## 4. Against the Package B bracket — OUTSIDE ON EVERY AXIS (the finding)

The design's rule-4 expectation was "the 0.535–0.611
delivered-capture bracket becomes a central value with the bracket as
the band". **The measurement refused that framing.** The tier-2
central lands outside the tier-1 envelope on every axis:

| Axis | tier-2 central | tier-1 (Package B, 60 GW pins) | position |
|---|---|---|---|
| Delivered capture | 0.6977 | 0.5514 (export) – 0.6106 (frozen); design envelope 0.535–0.611 | **ABOVE the whole band** |
| Curtailment | 4.01 TWh | 5.33 (export) – 21.85 (frozen) TWh | **BELOW the bracket floor** |
| Gas | 40.70 TWh | frozen 33.21 TWh (tier 1 had NO gas bracket — degenerate by construction) | above, un-bracketed |
| Net imports | −6.46 TWh | frozen +33.30 (observed trace held) | sign flip — the endogeneity itself |

The reviewer's §F(i) ruling, **binding and quoted verbatim** (this
frame supersedes the Package B §4(b) quoting rule for high-wind
capture/curtailment/gas; the Package A basis-label rules continue to
compose):

> The Package B tier-1 bracket is a measured ONE-SIDED bound, not an
> error bar. All three tier-1 conventions act only in £0-priced
> surplus periods and lack the export/withdrawal price channel; the
> tier-2 measurement confirms the Package B review §4(b)(iii) caveat
> in both direction and magnitude. The tier-2 central estimate at
> 60 GW (multi-zone, endogenous imports, scarcity rule per the
> d11-engine-review §G ruling) is: delivered capture **0.698**
> (potential 0.682), curtailment **4.01 TWh**, gas **40.70 TWh**, GB
> net exports **6.46 TWh**, gas price-setting 64.2 %, mean SMP
> £51.24/MWh — outside the tier-1 envelope on every axis (delivered
> 0.698 > 0.611 frozen top; curtailment 4.01 < 5.33 export floor; gas
> above the un-bracketed frozen 33.21). QUOTING CONDITIONS (mandatory,
> all five): (a) external fleets, demand and prices are FROZEN at the
> 2024 basis — the number answers "60 GW of GB wind in the 2024
> European system", and this convention's known bias on capture points
> UP; (b) scarcity-rule dispatch fidelity, with the measured −0.046
> delivered-capture anchor wedge (multi-zone 0.895 vs single-zone
> 0.941; direction DOWN at the anchor; not a subtractable constant);
> (c) the flow-walk stall defect (§B.4), measured effect ≤ 0.025 TWh
> curtailment / ≤ 0.0002 capture, direction AGAINST the finding;
> (d) only the pinned anchor and 60 GW points are quotable (docs/05
> rule 3); (e) GB-internal transmission is UNCONSTRAINED (the GB zone is
> an internal copper plate: the B4/B6 Scotland–England boundary family —
> measured binding [23.5 %, 28.2 %] of periods under the LP at current
> capacity, and +38–49 % on copper-plated storage sizing — is not
> composed with the interconnected scenario). Nearly all interconnector
> landing points sit south of both boundaries, so the export channel
> implicitly wheels northern wind across the measured binding
> constraints for free. Documented bias: UP on delivered capture and
> exports, DOWN on curtailment — the same side as (a). Resolver, named:
> the composed 3-zone-GB + external-zones measurement
> (gb-2024-3zone.toml boundary family joined to the 5-zone external
> set); until that is run and pinned, 0.698 is an upper-side estimate
> on this axis as well as on (a). The tier-1 delivered-capture width
> 0.551–0.611 remains quotable ONLY as the frozen-convention record
> with its §4(b)(iii) understatement caveat — never as the uncertainty
> band around 0.698.

Caveat (e) mechanism (3-zone scenario convention note): all external
links except Moyle land in England/Wales (IFA, IFA2, ElecLink, BritNed,
Nemo, NSL at Blyth, Viking, EWIC, Greenlink) — south of both
boundaries. Like-for-like precision (reviewer addendum, do not
over-correct): the tier-1 Package B bracket is ALSO a GB copper plate,
so the bracket-escape DIRECTION (tier-2 outside the tier-1 envelope) is
a like-for-like comparison and survives caveat (e); what (e) conditions
is the ABSOLUTE LEVEL of the quotable central (0.698 / 4.01 TWh /
−6.46 TWh) as a statement about a real 60 GW system.

> **Caveat (e) status amendment (D13 package-2 adjudication,
> 2026-07-06):** PARTIALLY RESOLVED BY MEASUREMENT (D13,
> `d13-run-report.md`): on the curtailment axis the composed
> measurement resolves AGAINST the tier-2 level — minimum forced
> system waste 36.22 TWh at 60 GW (wind-driven increment +24.03 TWh
> over the composed anchor baseline; ≥ +20.0 TWh above the 4.01 TWh
> central on the conservative deconfounded basis); the 4.01 TWh
> copper-plate figure is quotable only alongside that record. The
> capture and net-trade components remain OPEN (no composed
> instrument; named resolver: the economic-dispatch LP, docs/08).
> Export survival: OPEN under the asymmetric evidential rule — the
> composed rule-based floor reads net imports, which is not evidence
> of collapse.

(The design's 0.535–0.611 envelope spans the convention-invariant
potential floor to the frozen delivered top; the delivered-convention
width proper is 0.551–0.611. Carry the distinction into any figure.)

## 5. Mechanism evidence (reviewer-measured, §B of the review)

The above-bracket result is genuine model behaviour under **bounded,
physical** export machinery on the EXTERNAL side — not an
infinite-sink artifact (GB-internal copper-plating is a separate,
disclosed limitation: quoting condition (e) in §4):

- **Prices**: gas price-setting 64.25 % of periods at 60 GW (vs
  46.47 % single-zone frozen); mean SMP £51.24/MWh (vs £37.14).
  Endogenous imports withdraw when GB is long and exports keep GB
  gas-marginal in windy periods — the export/withdrawal price channel
  tier 1 lacked by construction, lifting capture.
- **Flows are bounded and frequently bound**: gross exports 35.83 TWh
  (sending end) / gross imports 29.38 TWh (GB end), net −6.46; zero
  periods exceed the 9.31 GW export capability (identical to the
  Package B export-convention cap); **986 periods sit at the full
  cap**.
- **External absorption is thermal displacement, not a sink**: ~23 TWh
  of external gas displaced at 60 GW (FR 26.54 → 14.33, CONT-NW
  129.53 → 121.36, IE-SEM 15.07 → 12.64 TWh); no thermal series goes
  negative; external unserved falls; CONT-NW's own curtailment RISES
  as it absorbs GB surplus — displacement headroom nowhere exhausted.
- **Curtailment split** (2,091 GB curtailment periods): **978 at the
  full 9.31 GW export cap** (2.90 of the 4.01 TWh), **852** with every
  unsaturated counterparty itself in surplus (the flow rule's stated
  £0-region equalisation), **25** exporter-bound (stack ceiling),
  **236** in the walk-stall class (§6).

## 6. Disclosed engine defect: the flow-walk stall (pre-existing; direction AGAINST the finding) — RESOLVED 2026-07-06

The review found (and reproduced mechanically) a pre-existing
floating-point stall in `flow.rs::equalising_flow`: a boundary-exact
step can leave the next increment below the ULP of the accumulated
flow, and the 64-pass cap then binds silently, truncating the flow
with link headroom left. It is **committed, digest-pinned Stage 5
behaviour** (all 23 anchor curtailment periods carry the signature),
surfaced — not introduced — by this package. Measured bound at 60 GW:
**≤ 0.025 TWh** curtailment overhang / **≤ 0.0002** delivered
capture; corrected values would be curtailment ≥ 3.982 TWh and
capture ≥ 0.6976 — i.e. **against** the finding on both axes (a
stall-free walk exports more £0-priced energy). Tracked in `docs/08`
(review condition 1); the fix is a separate engine package (it will
move the 5-zone dispatch digests — Stage 5 re-pin discipline).

> **RESOLVED (2026-07-06, the R7 engine package — docs/08 R7;
> movements in the correction banner at the top of this note).**
> Honesty note on the point estimates: the measured curtailment
> movement (−0.0247 TWh) held within the disclosed ≤ 0.025 bound, but
> the measured capture (0.697489) **exceeded** this section's
> parenthetical point estimate ("capture ≥ 0.697581" / "≥ 0.6976" —
> it fell 0.000195, below that floor) while remaining within the
> disclosed ≤ 0.0002 bound. The bounds held; the parenthetical point
> estimate did not.

## 7. The anchor wedge (quoting condition (b), the direction argument)

The multi-zone capture axis is a NEW quantity (the committed 5-zone
scenario carries no Stage 2 `[pricing]` block; GB is priced through
its v7 `[zones.pricing]` chain, bit-identical to the reference
chain). At the ANCHOR it reads delivered **0.894982731554173** vs the
single-zone reference's 0.9413419206049041 — a **−0.046** wedge from
scarcity-rule dispatch fidelity (modelled-vs-observed import timing,
the A2 residual class; plausible cause, not independently decomposed).
The engine switch measurably LOWERS capture at the anchor, so it
cannot be the mechanism that lifted the 60 GW value above the
bracket — the wedge strengthens the finding's direction. It is an
anchor measurement, **not a subtractable constant**.

## 8. Reproducibility (docs/06)

- Engine: `e41e008` (the committed D11 engine; dispatch/pricing path
  byte-committed) + this uncommitted sweep package (additive:
  `grid-adequacy/src/sweep.rs`, `grid-adequacy/src/lib.rs`,
  `grid-adequacy/tests/acceptance_d11_sweep.rs`).
- Scenario: `scenarios/gb-2024-5zone.toml`, sha256
  `d19c6efd357bf800e92a3e39ae3988ee4d0a126a0db7e70e6add9d557911de6d`
  (schema v7; `dispatch.flow_signal` at the committed `scarcity`
  default — the ladder row is an in-memory flip).
- Data packs (fetched/derived, never committed; committed manifests
  pin the contents): `data/packs/2024.sha256` (manifest sha256
  `04e235857d90ebf9…`), `data/packs/entsoe-2024.sha256`
  (`11ab1426d9e155aa…`), `data/packs/cf-eu-1985-2024.sha256`
  (`79663ea49d251fe9…`); price reference
  `data/reference/prices-2024.toml` (`aebeca6953d89e00…`),
  drift-guarded in-test.
- Reproduce: `cargo test -p grid-adequacy --test acceptance_d11_sweep
  -- --nocapture` (anchor self-validation, both 60 GW tables, the
  band-shape assertions, serial-vs-parallel bit-identity). Suite at
  handover: 619 passed / 0 failed / 4 ignored; fmt/clippy clean.

## 9. Record pointers

- Review + binding framing: `docs/notes/d11-sweep-review.md` (§F).
- Engine-review §G ruling (scarcity central): `docs/notes/d11-engine-review.md`.
- Ladder caveat: `docs/notes/d11-a2a-mismatch-characterisation.md` §3/§5.
- Tier-1 bracket record: `docs/notes/package-b-imports-bracket-review.md`
  (its §4(b) quoting rule is superseded for high-wind
  capture/curtailment/gas by §4 above; Package A basis-label rules
  compose unchanged).
- Tracked-deviation rewording: review §F(ii), supervisor record
  (condition 3).

## 10. Onward work (named resolvers, in priority order)

1. **The composed 3-zone-GB + external-zones measurement** — the
   gb-2024-3zone.toml boundary family (B4/B6) joined to the 5-zone
   external set. Named resolver for quoting condition (e): the first
   measurement of high-wind capture/exports where northern wind must
   cross the measured binding constraints to reach the interconnector
   landing points. Until run and pinned, 0.698 is an upper-side
   estimate.
2. **Correlated-external-wind sensitivity** — external zones scale wind
   alongside GB (shared weather system) instead of the 2024-frozen
   basis. Resolver for the caveat (a) bias direction: tests whether the
   export channel survives when the neighbours are long at the same
   time. (Research-directive candidate, not yet scoped.)
3. **R7 walk-stall fix** (docs/08) — **DELIVERED 2026-07-06** (the R7
   engine package; 5-zone dispatch digests re-pinned with the record —
   see the correction banner above and docs/08 R7 RESOLVED).
