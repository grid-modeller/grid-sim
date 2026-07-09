# D11 — Tier-2 SWEEP package: independent review and finding adjudication

**Independent reviewer (D11 sweep gate), 2026-07-05.** Gate review of
the uncommitted D11 sweep package (`grid-adequacy/src/sweep.rs`
+631 lines, `grid-adequacy/src/lib.rs` additive exports, NEW
`grid-adequacy/tests/acceptance_d11_sweep.rs`, `memory/project-state.md`
session entry) AND adjudication of the fired pre-registered finding
branch: the 60 GW multi-zone central estimate landed OUTSIDE the
Package B bracket on every axis, and the implementer withheld the run
report pending this ruling. Method: re-derive, do not trust — every
pinned number was reproduced by my own probe (a scratchpad crate
path-depending on the workspace; no tree file modified), the headline
capture/SMP arithmetic was recomputed from the raw dispatch series with
my own independent implementation of the documented conventions, the
flow rule's behaviour in every flagged period was reconstructed with a
verbatim reimplementation of the breakpoint walk, and all gates were
re-run on this machine.

## VERDICT: ACCEPT-WITH-CONDITIONS — the finding STANDS; framing ruled in §F

The package is correct, anchored, metric-parity-clean and honestly
reported. The above-bracket result is genuine model behaviour of the
committed engine under bounded, physical export machinery — NOT a
copper-plate or infinite-sink artifact (§B). The review found one real
ENGINE defect (a floating-point stall in the flow walk, §B.4) that is
pre-existing, present in the committed Stage 5 anchor behaviour, and
whose measured effect at 60 GW is immaterial and points AGAINST the
finding — it attenuates rather than inflates it. Four conditions (§H);
none touches the package's code.

## A. Headline re-derived — every pin reproduces exactly

My probe (separate process, serial execution) reproduced every claimed
value bit-for-bit:

| Quantity (60 GW, scarcity central) | claimed | re-derived |
|---|---|---|
| delivered capture | 0.6976839505365661 | 0.6976839505365661 |
| potential capture | 0.6816365528136847 | 0.6816365528136847 |
| curtailment TWh | 4.007462807827 | 4.007462807827 |
| gas TWh | 40.695234239837 | 40.695234239837 |
| net imports TWh | −6.456015207006 | −6.456015207006 |
| gas price-setting % | 64.247495446266 | 64.247495446266 |
| mean SMP £/MWh | 51.241226229505 | 51.241226229505 |
| unserved | 0 | 0 |

**Independent recomputation** (my own SMP construction from the raw GB
thermal series + SRMC traces under the documented Stage 2 conventions,
my own pro-rata delivered arithmetic, my own capture quotient — not the
engine's pricing functions): delivered 0.697683950537, potential
0.681636552814, mean SMP 51.241226229505, gas-setting 64.247495446266 %
— agreement to every printed digit. The metric arithmetic is what it
claims to be.

**Anchor self-validation confirmed**: the factor-1.0 sweep point equals
the committed 5-zone `run_multi` bit-for-bit (imports, gas, curtailment,
unserved — `==` on the newtypes, reproduced in my probe), and the A1
arithmetic checks: +35.935153 TWh = **+7.913 %** vs 33.30 (inside
±10 %); 71.797411 TWh = **−1.364 %** vs 72.79 (inside ±5 %).

**Ladder sensitivity confirmed**: delivered 0.6784115295781239,
potential 0.6628074159582596, gas 40.030291928817, imports
−5.601225528878; curtailment 4.007462807827 TWh — **identical to the
scarcity central to 12 dp**, corroborating the £0-surplus
graceful-degradation property exactly as claimed. Signal choice moves
capture −0.019: the finding is not signal-dependent.

## B. Physical plausibility of the above-bracket result — CLEARED, with one engine defect found and bounded

This was the critical question: is GB-as-net-exporter (−6.46 TWh at
60 GW) and capture-above-the-whole-band inflated by unbounded external
absorption? Measured answer: no.

1. **Exports are bounded and frequently bound.** Total GB export
   capability = 9.31 GW (9.8 GW nameplate ex-Greenlink × 0.95 —
   identical to the Package B export-convention cap; verified from the
   scenario's links). At 60 GW: gross exports 35.83 TWh (sending end),
   gross imports 29.38 TWh (GB end), net −6.46. **Zero periods exceed
   the 9.31 GW cap**; 986 periods sit at the full cap; 17,412 of 17,568
   periods carry some export (mostly the small IE links — GB exported
   to IE in 87.7 % of observed 2024 periods, so the shape is right).
2. **External absorption is thermal displacement, not a sink.** At
   60 GW vs anchor: FR gas 26.54 → 14.33 TWh, CONT-NW gas
   129.53 → 121.36, IE-SEM gas 15.07 → 12.64 — ~23 TWh of external
   gas displaced, all within the external fleets' own dispatch; no
   thermal series goes negative anywhere (verified per period), and
   external unserved FALLS (NO2 510.6 → 9.7 GWh; CONT-NW 2.0 → 1.0;
   DK1 unchanged 191.3 — its pre-existing wedge artifact). External
   zones dispatch at their 2024 demand basis throughout. The CONT-NW
   copper plate (D5) is not doing unphysical work: CONT-NW's own
   curtailment RISES 3.43 → 3.75 TWh as it absorbs GB surplus, and its
   ccgt still runs 121 TWh — displacement headroom nowhere exhausted.
   (Carried limitation, pre-existing: external ccgt has no must-run/
   CHP floor, a Stage 5 convention.)
3. **Curtailment 4.01 TWh is consistent with the caps.** Of 2,091 GB
   curtailment periods: **978 at the full 9.31 GW cap** (2.90 of the
   4.01 TWh), **852** with every unsaturated counterparty itself in
   surplus (the flow rule's stated negative-price-analogue
   equalisation), **25** exporter-bound (DK1's stack ceiling — it
   exported its whole surplus; legitimate rule-3 bound), and **236**
   in the stall class below. 1.73 TWh of exports (4.8 % of gross) land
   in zones curtailing that period — the documented surplus-depth
   equalisation shifting curtailment across borders, bounded by link
   caps, not free absorption.
4. **ENGINE DEFECT FOUND (pre-existing): the breakpoint walk can
   stall.** In 236 of the 2,091 curtailment periods (all on the
   Moyle/EWIC border), the flow stops mid-walk with link headroom left
   and the counterparty still on its stack — pairwise equalisation
   visibly violated. I reproduced the mechanism with a verbatim
   reimplementation of `equalising_flow` fed the exact period-10
   inputs: when a pass steps exactly `d_imp` onto the importer's
   segment boundary (here IE-SEM's coal/ccgt edge), rounding can land
   the recomputed residual epsilon ABOVE the edge, the next probe
   returns `dist_imp ≈ 1e-17`, and `q += d_exp.min(d_imp)` is absorbed
   below the ULP of `q` — zero progress; the 64-pass cap (documented as
   "generous headroom, not a tolerance") then binds and silently
   truncates the flow (my walk reproduces the engine's q = 0.516274126
   exactly, stalling at pass 1 with gap = +4.15 still open).
   **Materiality, measured**: at 60 GW the stall-attributable
   curtailment overhang is ≤ 0.025 TWh (headroom-capped) — corrected,
   curtailment 4.007 → ≥ 3.982 TWh and delivered capture
   0.697684 → ≥ 0.697581. **Direction: AGAINST the finding on both
   axes** (a stall-free walk exports more £0-priced energy: curtailment
   falls further below the tier-1 floor; capture falls by ≤ 0.0002).
   At the ANCHOR, all 23 curtailment periods (4.5 GWh total) carry the
   stall signature — this is committed, digest-pinned Stage 5
   behaviour, surfaced (not introduced) by this package, which only
   calls `run_multi`. Condition 1.

Conclusion: the above-bracket result is not a modelling artifact. The
mechanism is as claimed — endogenous imports withdraw when GB is long
and exports displace external thermal, keeping GB gas-marginal in
64.25 % of periods (vs 46.47 % frozen single-zone) with mean SMP
£51.24 (vs £37.14), which lifts capture; the export channel and the
storage-cycling differences pull curtailment below the tier-1 export
floor.

## C. Metric parity with the Q10/Q2 pinned definitions — EXACT

- **SRMC chain**: the 5-zone GB `[zones.pricing]` ccgt and ocgt SRMC
  traces are **bit-identical** (17,568/17,568 values) to the committed
  single-zone reference's `[pricing]` chain (`prices-2024.toml`
  reference + daily gas SAP + the same efficiency keys) — verified
  value-by-value in my probe. Same recipe, same reference, same trace.
- **SMP/capture/setter/mean recipes**: `multi_zone_point_metrics`
  reproduces `price_run`'s series construction line-for-line (same
  `PricedSeries` build over `result.thermal`, same
  `system_marginal_price`, `time_weighted_mean_price`,
  `capture_ratio`, `price_setting_share` grid-core functions) and the
  CLI sweep's `wind_capture_both_bases` arithmetic exactly (same wind
  identity, same `delivered_renewable_power` pro-rata). My independent
  recomputation (§A) is the numerical proof.
- **Pinned reference values reproduce**: single-zone reference capture
  potential 0.9413407336345198 / delivered 0.9413419206049041 — the
  committed regression_stage2/delivered pins, live in my probe.
- One definitional note, correctly handled by the implementer: in a
  multi-zone run GB gas dispatched FOR EXPORT also sets GB's SMP.
  That is not a parity break — it is the same convention ("most
  expensive dispatched SRMC-bearing technology") on the richer
  dispatch, i.e. the price channel tier 2 exists to add.
- The test's 0.535–0.611 band is the design's rule-4 envelope
  (potential floor to frozen-delivered top); the delivered-convention
  width proper is 0.551–0.611 — the test documents the distinction
  in-line. Quoting ruled in §F.

## D. The anchor wedge and the frozen-external convention — bias directions ruled

- **Anchor wedge −0.046 verified**: multi-zone anchor delivered capture
  0.894982731554173 vs single-zone reference 0.9413419206049041
  (Δ = −0.046359). The direction-of-bias argument is SOUND: the engine
  switch measurably LOWERS capture at the anchor, so it cannot be the
  mechanism that lifted the 60 GW value above the bracket. Two
  precisions carried: (a) the wedge is an anchor measurement, not a
  constant — it may not be subtracted from the 60 GW value; (b) its
  cause (modelled-vs-observed import timing, the A2 residual class) is
  plausible and consistent with the 90.07 % A2a record but was not
  independently decomposed here.
- **External-fleets-frozen bias at 60 GW: UP on capture, DOWN on
  curtailment — the load-bearing caveat.** The sweep answers "60 GW of
  GB wind dropped into the 2024 European system": neighbours keep
  their 2024 fleets, demand and (SAP/EUA) prices, so the export/
  withdrawal channel meets 2024-sized absorptive thermal. Any real
  60 GW world has neighbours who also decarbonised — more correlated
  renewables, less displaceable thermal, more coincident surplus. The
  0.698 is therefore a 2024-basis measurement whose known bias on the
  capture axis points the same way as the finding, and it must never
  be quoted without that condition (§F). The 2024 price basis is the
  same conditionality on the SMP level axis (mean SMP, gas share).

## E. Gates, determinism, hard rules

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean; full suite (release) **619 passed / 0 failed /
  4 ignored** — all re-run by me, matching the claim exactly. The four
  new acceptance tests and three new unit tests ran green inside it.
- **Committed pins unmoved**: the diff touches no committed test or
  scenario file; regression_2024 (779d7444), stage-2/delivered
  (0.9413…), regression_imports_bracket_2024 (all 12 Package B pins),
  Stage 5 A-gates, B4, B6, D11 ladder pins all passed in my run; the
  Package B 60 GW frozen row still stands as the tier-1 record.
- **Determinism**: parallel/serial bit-identity asserted at unit and
  acceptance scale; my separate-process probe reproduced every pin
  bit-for-bit (cross-process determinism). No wall-clock, globals or
  randomness in the diff; the shared-inputs convention (loaded once,
  scaling applied at dispatch) is sound — capacity scaling never
  touches trace loading.
- **No library panics**: the new library code returns structured
  `GridError`s on every rejection path (empty/non-finite capacities,
  unknown/windless zone, missing pricing); the only `unwrap_or`s are
  zero-energy defaults; unwraps live in `#[cfg(test)]` / the
  acceptance test (allowed).
- **Newtypes**: `Power`/`Energy`/`Price` across the new public API;
  the capture ratios are dimensionless `Option<f64>` with the
  NaN-free `None` convention (fork 4) — correct.
- **TDD**: single uncommitted tree — commit-order evidence not
  verifiable (the engine-review precedent). Artifact shape is
  consistent with the claimed red-first work order (the acceptance
  test is the work order's rule-4 assertion converted to the
  finding shape per the established rule-4 precedent); unit tests are
  discriminating (endogenous-response direction test, proportional
  scaling, error paths, bit-identity). Condition 4 (commit hygiene)
  carried from the engine review.
- **Scope**: matches the D11 rule-2 sweep work order; no schema
  change, no new dependencies, no CLI change, no doc edits beyond the
  required project-state session entry; the run report is correctly
  WITHHELD per the stop-and-report clause. Untracked `figures/`
  (rs-37y storage trace CSV/PNG, dated 2026-07-04) predates this
  package — not chargeable; disposition belongs to the supervisor.

## Fork adjudications (allegation 6)

1. **Pre-loaded `MultiZoneInputs` API** — ACCEPT (shared inputs
   bit-identical to per-point reload; scaling enters at dispatch).
2. **Library pins, no CLI artefact** — ACCEPT for the finding record;
   noted: quoting Q10 figures publicly later requires the artefact
   path (CSV/PNG/Parquet with embedded hashes, docs/06) — that is
   run-report/figure-time work, not this package's scope.
3. **GB SMP via the `[zones.pricing]` chain** — ACCEPT; definitional
   identity to the Q10 price series verified bit-for-bit (§C).
4. **All-£0 SMP ⇒ capture `None`** — ACCEPT (0/0 has no meaning;
   keeps points `PartialEq`-comparable for the determinism asserts).
5. **Finding-shaped test conversion with full-precision pins** —
   ACCEPT; the shape asserts (capture > band-top, curtailment <
   export floor) are exactly the anti-rot discipline the rule-4
   precedent requires: a silent drift back inside the band fails loud.

## F. RULING — the framing (binding for the run report, the deviation ledger, and the Q10/Q2 quoting rule)

**(i) The finding is sound and becomes the quotable tier-2 record**,
in this exact frame:

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
> all four): (a) external fleets, demand and prices are FROZEN at the
> 2024 basis — the number answers "60 GW of GB wind in the 2024
> European system", and this convention's known bias on capture points
> UP; (b) scarcity-rule dispatch fidelity, with the measured −0.046
> delivered-capture anchor wedge (multi-zone 0.895 vs single-zone
> 0.941; direction DOWN at the anchor; not a subtractable constant);
> (c) the flow-walk stall defect (§B.4), measured effect ≤ 0.025 TWh
> curtailment / ≤ 0.0002 capture, direction AGAINST the finding;
> (d) only the pinned anchor and 60 GW points are quotable (docs/05
> rule 3). The tier-1 delivered-capture width 0.551–0.611 remains
> quotable ONLY as the frozen-convention record with its §4(b)(iii)
> understatement caveat — never as the uncertainty band around 0.698.

**(ii) Replacement for the design's "resolved, with the bracket as the
error bar" language** (apply to the tracked-deviation entry and
anywhere rule 2's wording is quoted):

> Frozen-imports deviation: RESOLVED BY MEASUREMENT — and the
> measurement refused the planned framing. The tier-2 endogenous
> central at 60 GW falls OUTSIDE the tier-1 bracket on every axis, in
> the pre-registered §4(b)(iii) direction (missing export/withdrawal
> price channel). The tier-1 bracket is therefore a one-sided
> (understating) bound on high-wind delivered capture and a
> frozen-convention upper envelope on curtailment — NOT the error bar
> around the tier-2 central. Uncertainty on the central is carried as
> the named caveats (2024-frozen externals: capture-bias UP;
> scarcity-rule fidelity, anchor wedge −0.046: DOWN at anchor;
> walk-stall ≤ 0.03 TWh: against the finding), not as the tier-1 band.

**(iii) The run report** (docs/notes/d11-sweep-run-report.md, to be
written now) MUST contain: the conventions (GB-only scaling, external
2024 basis, scarcity central per §G with the ladder as named
sensitivity); the anchor self-validation table (bit-identity statement
+ A1 numbers +7.91 %/−1.36 %); the full-precision 60 GW central and
ladder-sensitivity tables; the Package B comparison with the
outside-on-every-axis statement and the (i) frame verbatim; the
mechanism evidence (gas price-setting 64.25 % vs 46.47 %, mean SMP
£51.24 vs £37.14, gross exports 35.8 / imports 29.4 TWh, 986
cap-saturated export periods, curtailment split 978 at-cap / 852
counterparty-surplus / 25 exporter-bound / 236 stall); the anchor
wedge −0.046 with its direction argument; the §B.4 stall disclosure
with its measured bound and tracking pointer; the mandatory caveat
block (i)(a)–(d); and the note that the ladder numbers are a
dispatch-convention sensitivity, not a second central
(characterisation §3/§5 caveat). Engine/scenario/data-pack hashes per
docs/06.

**(iv)** Not applicable in its REJECT form: §B found no inflating
artifact. The one defect found (the stall) attenuates the finding and
is disclosed under (i)(c)/(iii) rather than leading the note.

## H. Conditions

1. **Record the flow-walk stall defect** (engine, pre-existing —
   `flow.rs::equalising_flow`: sub-ULP increment after a
   boundary-exact step; 64-pass cap binds silently; present in the
   committed anchor behaviour, 23/23 anchor curtailment periods).
   Track it in docs/08 / the issue ledger + project-state with the
   measured 60 GW bound and direction. The FIX is a separate engine
   package (it will move the 5-zone dispatch digests — Stage 5 re-pin
   discipline applies); it is NOT chargeable to, and does not block,
   this package.
2. **Write the run report per §F(iii)** before any tier-2 number is
   quoted anywhere.
3. **Supervisor record updates**: tracked-deviation wording per
   §F(ii); Q10/Q2 quoting rule per §F(i) (supersedes the Package B
   §4(b) rule for high-wind capture/curtailment/gas; the Package A
   basis-label rules continue to compose).
4. **Commit hygiene** (carried from the engine review): structure the
   commit(s) so the red-first → finding-conversion record survives,
   and land the project-state entry with the package.

Non-blocking notes: (i) `figures/` untracked leftovers (2026-07-04
rs-37y trace) need a supervisor disposition; (ii) the acceptance
test's band constants document the 0.535 potential-vs-0.551 delivered
distinction correctly — carry that distinction into any figure; (iii)
the Module 1 CLI sweep remains single-zone — fine until Q10 figure
time, when the tier-2 artefact path (with hashes) must exist.

— independent reviewer (D11 sweep gate), 2026-07-05

---

## ADDENDUM (2026-07-05) — post-acceptance ruling: caveat (e), GB internal copper plate

Raised by Richard after acceptance; ruled here as a binding extension
of §F. Evidence verified against the committed record before ruling:

- `gb-2024-5zone.toml` models GB as a **single internal node** (one
  zone, national CF traces, no internal links). The tier-2 sweep
  therefore ignores the Scotland–England transmission constraints
  entirely.
- The committed B6 finding (b6-two-zone-run-report, reviewer-accepted):
  copper-plating GB understates the storage requirement by **+38–49 %**
  (35,648 GWh = +49.3 % at 2024 B6 capability; +38.5 % lower bound).
- The committed B4-LP band (f2bc9a5): the perfect-foresight optimiser
  binds B4 in **[23.5 %, 28.2 %] of periods at CURRENT (~29 GW) wind**,
  ~12–14× the rule-based 1.96 % — the constraint is first-order today,
  a fortiori at 60 GW (and the sweep scales the national CF trace, so
  the plausible northward shift of a 60 GW fleet is not even
  represented).
- The 3-zone scenario's own landing-point convention: every external
  link except Moyle lands in England/Wales (IFA, IFA2, ElecLink,
  BritNed, Nemo, NSL at Blyth, Viking, EWIC, Greenlink) — **south of
  both boundaries**. The tier-2 export channel therefore requires
  wheeling northern wind across the measured binding constraints,
  which the copper plate grants for free.

**RULING: YES — §F(i) gains mandatory caveat (e).** Bias direction:
copper-plating GB overstates the deliverability of (predominantly
northern) wind both to southern demand and to the southern
interconnector landing points, so it understates behind-constraint
curtailment and overstates exports and delivered capture — **UP on
the capture/export axes, the same side as caveat (a)**. Given the
committed B4/B6 magnitudes, this is material and cannot be left
implicit. One precision, stated so the finding is not over-corrected:
the tier-1 Package B bracket is ALSO a GB copper plate, so the
**bracket-escape direction** (tier-2 outside the tier-1 envelope) is a
like-for-like comparison and is NOT undermined by this caveat; what
(e) conditions is the **absolute level** of the quotable central
(0.698 / 4.01 TWh / −6.46 TWh) as a statement about a real 60 GW
system.

**Caveat (e), exact text (append to the §F(i) quoting conditions,
which now read "all five"):**

> (e) GB-internal transmission is UNCONSTRAINED (the GB zone is an
> internal copper plate: the B4/B6 Scotland–England boundary family —
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
> on this axis as well as on (a).

**The run report's §4 (mandatory-caveat block) must add**: caveat (e)
verbatim; one mechanism sentence naming the landing-point geography
(all external links except Moyle land in England/Wales — 3-zone
scenario convention note); the like-for-like precision above (the
bracket-escape finding is copper-plate-consistent on both sides; the
caveat conditions the central's level, not the finding's direction);
and the composed 3-zone+externals run as the named resolver in the
onward-work list. The §F(ii) deviation-ledger wording gains "(e)
GB copper plate: UP" in its caveat parenthesis.

— independent reviewer (D11 sweep gate), addendum 2026-07-05
