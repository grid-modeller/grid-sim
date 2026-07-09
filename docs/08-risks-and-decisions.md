# 08 — Risks, Open Decisions, Kill Criteria

Working document. Resolve open decisions before the stage that needs them.

## Open decisions

| # | Decision | Needed by | Notes |
|---|---|---|---|
| D1 | renewables.ninja licence: fetch-and-build vs. direct ERA5 derivation | Before Stage 0 data work | **Resolved 2026-07-02: direct ERA5 derivation** (ERA5 is CC-BY 4.0; ninja is CC BY-NC and MERRA-2/CM-SAF-based — internal cross-check only). `docs/notes/d1-renewables-ninja-licence.md` **CCC licence addendum 2026-07-06:** CCC (theccc.org.uk) publications and datasets: Open Government Licence v3.0 per the site-wide statement at theccc.org.uk/copyright-terms-conditions/ ("licensed under the Open Government Licence v3.0 except where otherwise stated"), re-verified independently by the data engineer and the reviewer 2026-07-06; the CB7 workbooks carry no overriding statement (string-swept by both); the report PDF carries CCC's own permissive reproduction notice (printed p.3). This SUPERSEDES the stage7-cost-inputs-report §10.4 "no explicit open licence found" finding, which had reached only the FoI page. Redistribution of derived numbers is permitted with attribution "Climate Change Committee, The Seventh Carbon Budget (2025)"; third-party-copyright carve-out noted (no third-party rights statement in the CB7 data artefacts used). `docs/notes/stage7-pathways-data-review.md` §8 |
| D2 | Stage 1 validation tolerances (`TBD-DATA`) | Before Stage 1 | **Resolved 2026-07-02:** gas ±5 %, exogenous imports ±1 %, monthly mix corr ≥ 0.95 — set in `docs/04` Stage 1 with justifications; evidence in `docs/notes/2024-validation-pack-report.md` |
| D3 | Embedded generation treatment | Stage 1 | **Resolved 2026-07-02: total-generation convention** (underlying demand = ND + NESO embedded estimates; embedded capacity modelled explicitly). `docs/notes/d3-embedded-convention.md` |
| D4 | Rule-based storage dispatch rules (exact policy) | Stage 3 | **Resolved 2026-07-02: greedy zero-foresight policy** (surplus-only charging, discharge after the full stack, no reserve; reserve-floor named as the kill-criterion-3 fallback). Prose spec reviewed and adopted: `docs/notes/d4-rule-based-dispatch.md` |
| D5 | Continental zone granularity (3 vs. 4 zones) | Stage 5 | **Resolved 2026-07-03: five external zones** — FR, CONT-NW (BE+NL+DE-LU aggregate), NO2 (not NO-aggregate; hydro-driven from ENTSO-E), DK1, IE-SEM (DK1/IE-SEM forced by the imports identity, reviewer-verified). Per-border BE/NL direction gates ruled structurally unpassable under the aggregate (≥13.6 % opposite-sign floor) — annual-energy validation there, direction gate GB↔FR only. Proposes an ADR-7 zone-list amendment (recorded in project-state). Reviewed ADOPT-WITH-EDITS: `docs/notes/d5-zone-granularity.md` |
| D6 | Constraint-cost function form (ADR-12) | Stage 7 *(deferred past the Stage 7 tag, annotated 2026-07-06)* | Calibrate against published constraint payment data already tracked for the Subsidy Clock. **Status at the Stage 7 close: DEFERRED, not silently dropped** — Stage 7 tagged with constraint costs carried as a named zero in the cost stack (the stage7 run report's uncosted/named-gaps discipline); the D13 boundary family now provides the measured binding records a calibrated function would consume. Becomes live with the first scenario that quotes a constraint-cost line |
| D7 | Web UI framework and hosting | Phase two | Out of scope for engine spec |
| D8 | System-cost / LCOE methodology (headline metric, decomposition, comparison rules) | Stage 7 opening, before any cost code | **Resolved 2026-07-03: reviewer ADOPT-WITH-EDITS, all nine edits applied** — delivered system cost headline (denominator = delivered-to-demand, pinned), additive decomposition, equal-reliability comparisons with the fixed-fleet make-good convention, re-solved-pair differencing for marginal claims, Q9 as a three-wedge accounting identity (rule 6a), uniform-WACC prominent, bridges caveat-carried. `docs/notes/d8-lcoe-methods.md`; adjudication `docs/notes/d8-lcoe-methods-review.md` |
| D9 | Heating overlay design (Q5/Q11): technology portfolio, COP source temperatures, heat-demand shape, geothermal-relief analysis | Before any heating schema/engine work; promoted to the overnight chain by Richard 2026-07-03 | **Resolved 2026-07-03: reviewer ADOPT-WITH-EDITS, all twelve edits applied** — pinned-intensity heat demand (cold years draw more heat; per-year renormalisation rejected as an inter-annual-band zeroing defect), Kusuda–Achenbach ground model at conservative loop depth with MIDAS/BGS cross-check trigger, per-tech RHPP derating mechanics, per-zone v5 block REPLACING the live v1–v4 sketch, district-lowest ordering machine-checked / ASHP-vs-GSHP demoted to measured finding, rule 6b geothermal-relief deliverable (Q11). `docs/notes/d9-heating-overlay.md`; adjudication `docs/notes/d9-heating-overlay-review.md` |
| D10 | EV / transport demand overlay (Q12) | Before Q12 engine work; **promoted into v1 by D15** (needed to run FES EE demand at 2035+, not only Q12) | **Resolved 2026-07-06: design ADOPTED (reviewer ADOPT-WITH-EDITS, all seven ordered edits applied)** — road-EV demand overlay: per-zone `[zones.demand.ev]` segment portfolio (schema v8, additive), record-mean quantum with pinned-normalisation temperature derating on the shared `T_pop` trace (correlation with D9 heating stated as physics), dumb/smart charging bracket (smart = min-max water-fill on the pre-dispatch residual-load signal with machine-checked containment preconditions; bracket direction a theorem via that construction), V2G-lite emitted as ADR-8 `dsr` pseudo-storage (engine activation semantics: Q6 dependency, named). FES-consistency carve-out rule pins the anti-double-count composition MECHANISM for both D9/D10 reprofiling of pathway scenarios; the D9 component convention is named open until the scenario package. Chain: data package (ev-fleet-v1) → engine (v8) → FES-reprofiled scenarios + pinned runs. *(Schema bump RE-TARGETED to **v9**, 2026-07-06: the D16 engine package landed first and took v8 — banner on the D10 note; docs/03 v7→v8 migration note.)* `docs/notes/d10-ev-overlay.md`; adjudication `docs/notes/d10-ev-overlay-review.md` |
| D13 | Composed boundary-trade measurement: the 3-zone GB boundary family (B4/B6) joined to the 5-zone external set — the named resolver for D11 sweep quoting caveat (e) (does the 60 GW export/capture finding survive the measured Scotland–England constraints?) | Before the composed scenario/measurement packages; pulled ahead of the Stage 7 remainder (Richard, 2026-07-05) | **Resolved 2026-07-05: reviewer ADOPT-WITH-EDITS, all nine ordered edits applied** — loss-as-waste term adopted at the head of package 1 (four conditions incl. the unmoved-pins gate); LP shadow capture suppressed (thermal-split objective-degeneracy); two-regime band framing (trade axes: rule-based central; binding axes: LP [floor, point] per b4-lp-findings); PS inertness asserted at both points; budget conversion ratified with identity asserts; caveat (k) boundary capability frozen at 2024. **Package 1 delivered and adjudicated 2026-07-05 (review addendum): gates 8(i)/8(ii)-B4 measured RED, ruled genuine findings about the instrument (the committed equal-depth single-pass stranding artefact; export-drain unreachable by construction under rule-6 order) — no re-pin, the deviation-shape pins are the record; composed rule-based leg NOT anchor-validated on national trade axes; package 2 COMMISSIONED RE-SCOPED (LP binding bands both floors, LP minimum forced waste with pinned objective decomposition, rule-based trade as one-sided bounds under the asymmetric evidential rule); capture axis of caveat (e) OPEN — resolver: the min-cost economic-dispatch LP (D14 below, not scheduled); anchor LP result quotable: B4 flat, 0.2813 vs 0.2816 (floor_internal/point).** **Package 2 delivered + adjudicated 2026-07-06: BRANCH A FIRES.** At 60 GW: LP minimum system waste 36.22 TWh (baseline 12.20; wind-driven increment +24.03; exceedance vs the copper-plate 4.01 quoted ONLY in deconfounded form, +20.02) — caveat (e)'s curtailment component resolved AGAINST the tier-2 level; B4 LP band [0.275, 0.571] (point doubled), B6 [0.371, 0.388] nearly degeneracy-free; SECONDARY finding: perfect foresight recovers only ~0.70 TWh of rule-based system waste at 60 GW — absorption-limited, not dispatch-limited (the B4-era headline inverts at scale); export survival OPEN under the asymmetric rule (+11.87 TWh net imports on the artefact-conditioned floor is not evidence of collapse); NO2 hydro-as-history conventions finding (0.577 TWh, threatens nothing). *(R7 correction 2026-07-06 — the rule-based-leg values in this row moved with the walk-stall fix: copper-plate comparator 4.01→3.98 TWh, deconfounded exceedance +20.02→+20.04, PF recovery ~0.70→~0.44 TWh of 36.67, RB net imports +11.87→+11.70; the LP leg and both verdicts unmoved. Current values per the d13-run-report R7 banner.)* Quote only per `docs/notes/d13-run-report.md`. Design `docs/notes/d13-composed-boundary-trade.md`; adjudications `docs/notes/d13-composed-boundary-trade-review.md` |
| D16 | Geothermal source temperature (depth gradient): the ground-heat continuum — a D9 heating-overlay follow-on giving the ground-source pump a source that warms with depth (geothermal gradient), unifying shallow-GSHP → warm-aquifer → direct-use as one continuum | Before the geothermal-continuum analysis; motivated by the industry correspondent's 2026-07-06 critique and book ch. 27 | **Resolved 2026-07-06: data + engine + continuum packages DELIVERED in one implementer pass (supervisor-ordered collapse of the note's chain); reviewer ACCEPT-WITH-CONDITIONS (`docs/notes/d16-geothermal-engine-review.md`), both doc-only conditions applied same day.** Design `docs/notes/d16-geothermal-source-temperature.md`; run report `docs/notes/d16-geothermal-engine-run-report.md`. Landed: schema **v8** (optional gshp `resource_depth_m`, version-line migration, frozen v7 fixture; D10 re-targets to v9), **heating-cop-v2** (`[geothermal]`: 25 °C/km centre [an industry correspondent, conservative], BGS band [26, 35] — Busby 2014 / Busby & Terrington 2017, cited + checksummed; every v1 value untouched), depth-re-anchored Kusuda–Achenbach wave with the gradient measured from the loop-depth datum (reviewer-ratified resolution of the note's `G·z` vs bit-identity tension), per-component direct-use handoff capped at district `cop_const` (per-period cap-then-switch; the 1250–1500 m step travels with quotes). **Safety spine held: the three committed D9 pins bit-identical** (92.238…/113.446… GW, 23,872/40,224 GWh). Continuum PINNED (`grid-adequacy/tests/geothermal_depth.rs`): all-GSHP peak delta +22.16 GW (1 m) → +11.75 (1250 m) → +3.61 GW = the district endpoint exactly (≥2000 m); storage delta +17,376 → +2,000 GWh. Physical only (£ = Q11 Stage-7; cooling = stated uncounted benefit). **OWED: the rule-4 calibration anchor** (real installation at known depth — the company (on file)/the correspondent or United Downs/Southampton/Eastgate; `#[ignore]`d test names the debt) and the ch. 27 continuum figure. On the Q11 path |
| D14 | Min-cost economic-dispatch LP: the composed family's capture/trade instrument — an LP objective with real economics (per-zone SRMC chains), the named resolver for the OPEN capture axis of D11 caveat (e) after the D13 package-1 ruling that MinCurtailment cannot measure gas/trade/capture (thermal-split degeneracy; loss-term autarky) and the composed rule-based leg is not anchor-validated on trade axes. A real engine package with its own design forks: cost coverage beyond the gas-only recipe boundary, external price bases, £0-rung degeneracy | Before any composed capture number is quoted | **NOT SCHEDULED** — named resolver only (D13 package-1 adjudication, ruling C); explicitly not commissioned inside D13 and not blocking D13 package 2. Number assigned under standing delegation, 2026-07-05; supervisor recommendation: defer past beta |
| D11 | Priced multi-zone dispatch (tier-2 imports): price-based flow signal + multi-zone sweep | Before the tier-2 engine work | **Resolved 2026-07-04: reviewer ADOPT-WITH-EDITS, all eight edits applied** — per-zone SRMC (lexicographic: SRMC primary, utilisation tiebreak) replaces the scarcity score in the flow rule (scarcity retained as the must-take tiebreak); needs a per-zone `[pricing]` schema bump + cited EU-ETS/continental-gas data (NOT a free consumer); the capacity sweep runs multi-zone; three-policy ladder {scarcity, priced, LP} all reported; A2a direction-match ≥95% (expectation 97.4%) the acceptance bar; the priced ladder resolves the A2 direction residual, NOT the B6 storage component (that survives to the LP). `docs/notes/d11-priced-dispatch.md`; adjudication `docs/notes/d11-priced-dispatch-review.md`. **Measured outcome 2026-07-05 (engine package): A2a ≥95% UNREACHABLE on 2024 prices — the pre-registered rule-4 finding (86.5% of the residual is both-gas-marginal in a carbon-parity year; static ceiling 93.84%; ladder measures 71.69% committed convention / 93.18% flat-flat, pinned as ladder-only findings; committed gates unchanged). Sweep central estimate runs the scarcity rule, ladder as named sensitivity (reviewer ruling, `docs/notes/d11-engine-review.md`; characterisation `docs/notes/d11-a2a-mismatch-characterisation.md`)** |
| D12 | Perfect-foresight LP dispatch + the D4 policy-contract relaxation (Stage 7) | Before the LP engine work | **Resolved 2026-07-04: reviewer ADOPT-WITH-EDITS, four gating edits applied** (contract split verified line-by-line vs the engine). The LP resolves ONE limitation behind three findings (B6 magnitude, tier-2 A2a, three-zone B4 wheeling). Policy-contract splits physical invariants from rule-based CHOICES (√η is a choice not a law; RuleBased digest 779d7444 bit-identical via validation-relocation-only); the LP is the bisection FEASIBILITY ORACLE (measures the same min-store-for-zero-unserved quantity → LP ≤ rule-based provable); finite window must span the binding recharge (short windows/typical-day ruled out); LP under-wheels→wheels, B4 binding COMPARED to observed never tuned; good_lp+HiGHS (ADR-10); flow.rs untouched (parallel path). Sequence = contract refactor (red test: pre-charging policy accepted where dispatch.rs:353 errors) → LP dispatch → three re-measurements. `docs/notes/d12-perfect-foresight-lp.md`; adjudication `docs/notes/d12-perfect-foresight-lp-review.md` |
| D15 | Book-driven v1 scope: "[book programme]" governs the simulator's initial release | Governance sync after the 2026-07-06 book-programme adoption | **Ruled 2026-07-06 (Richard's direct ruling); RATIFIED 2026-07-06 (Richard read and ratified the book document set; the project-state V1/BETA SCOPE supersession is now EFFECTIVE):** the book thesis drives v1 scope — the definition is the claims register in the book workspace (`[book workspace]`, workspace state aa0f6cd..07b32bd at sync time; the D-B2/D-B8 rulings postdate the adoption commit). Carried forward: old beta items 1–5 (Stage 7 remainder, banked multi-zone record, heating overlay, the manual). Promoted into v1: Q13 subsidy transfer layer (backcast gate vs the Subsidy Clock 2024 record; D-note before code), D10 EV overlay, D9 behavioural-heating variant, Q1 ELCC runner, Q7 nuclear availability profiles, Q6 DSR activation, Q3 carbon sweep, "the Plan" scenario pack (Stage 7 pathway pack + overlays); Q14 emergency-response corpus — conditional on book D-B7 (open). Demoted below the book line: D14 (unchanged, not scheduled), D12 step-3 remainders, R7 (queued hygiene), literal EU-27 Q12. **ADR-11 AMENDMENT PROPOSAL DECLINED (Richard's ruling, 2026-07-06, same day):** ADR-11 STANDS UNAMENDED — the sandbox (WASM web UI) remains phase two, decoupled from the book critical path and REMOVED from v1 (the book proceeds without gating on it; the sandbox brief carries forward as the phase-2 spec; it may launch alongside the book if phase 2 lands in time, but no manuscript content may depend on it — book charter D-B6 re-ruling). D7 remains accurate as written. The [separate programme] fold-in (book D-B8) adds NO engine scope by design (one-way firewall; recorded in the v1 scope doc). Engine-facing consequences: D10 and Q13 D-notes become the next design work after the Stage 7 remainder; docs/04 amendments arrive per promoted item with its D-note/work order (Phase 3), not in this sync. Review: `docs/notes/book-programme-governance-sync-review.md` (ACCEPT-WITH-CONDITIONS, all five applied) |

## Risks

**R1 — Scope creep from UI ambition.** The interactive sandbox will pull
effort from the engine. Mitigation: ADR-11 hard split; the CLI is the
research tool for the book; no UI work in phase one.

**R2 — The dispatch heuristic as attack surface.** Storage numbers are
downstream of the policy choice. Mitigation: ADR-6 dual-policy reporting;
D4 documented in prose; the rule-based > LP gap published as a finding.

**R3 — Data licensing blocks the shareable-scenario mechanic.** If weather
traces can't be redistributed, third parties must run fetch-data themselves.
Mitigation: resolve D1 early; design the pack format for independent
rebuild + checksum verification either way.

**R4 — Single-author model credibility.** Mitigation: open source,
validation pack public, opponent's (NESO) defaults, regression-pinned
published numbers, stated model boundaries (`05-validation.md`).

**R5 — WASM data payload.** 40 years × half-hourly × multi-tech = tens of
MB. Mitigation: phase-two problem; Parquet per-trace lazy loading noted in
`05-validation.md`.

**R6 — Time sink on Stage 5 (multi-zone).** The only structural expansion.
Mitigation: schema supports it from v1 (ADR-7); Modules 1–4 and most
extension questions don't need it; it can slip without blocking the book's
core results.

**R7 — RESOLVED 2026-07-06: flow-walk stall in `equalising_flow`**
(`grid-adequacy/src/flow.rs`; found 2026-07-05 by the D11 sweep
reviewer, pre-existing — present in the committed Stage 5 anchor
behaviour, 23/23 anchor curtailment periods). A boundary-exact
equalisation step can leave a sub-ULP residual (`dist_imp ≈ 1e-17`)
whose increment is absorbed below the ULP of the accumulated flow; the
64-pass cap then binds silently and truncates the flow. Measured
effect at the 60 GW tier-2 point: ≤ 0.025 TWh curtailment /
≤ 0.0002 delivered capture, direction AGAINST the D11 above-bracket
finding (attenuates it). Disclosed in
`docs/notes/d11-sweep-review.md` §B.4 and the D11 sweep run report.
**FIXED 2026-07-06 (engine package, red-first):** the walk gains a
recovery regime — passes 0–63 are bit-identical to the old walk (a
20M-case randomized equivalence sweep proved every old-terminating
walk unchanged), and only walks the old code silently cap-truncated
continue past 64 passes, each stepped breakpoint snapped monotonically
past its sub-ULP sliver (provably terminating; the skipped slivers are
below the representable resolution of the flow). Both walks fixed
(`equalising_flow`, `equalising_flow_priced`); minimal reproduction
pinned in `flow.rs` tests. Pin movements (old values recorded per pin
in the test files): the 5-zone tier-2 family moved exactly within the
review's §B.4 bounds (60 GW curtailment 4.0075 → 3.9827, −0.0247 ≤
0.025 TWh; delivered capture −0.000195 ≤ 0.0002); the
GB-internal-boundary families moved more because the stall was
pervasive there (2-zone gate (i) copper B6 +1.09 TWh TOWARD the DA
anchor, gate (ii) +0.62 TWh TOWARD the 17 TWh outturn — 2,795 of
17,568 periods of the committed 2-zone validation run were
stall-truncated; composed 60 GW all-zone curtailment −0.267 TWh,
consistent with the D13 package-1 anchor diagnostic that found the
signature on 9,473/9,473 GB-curtailment periods). All validation
gates (Stage 5 A1–A4, B6 gates (i)–(iii), A2a/A2b records) pass their
bands unmoved-or-improved; the single-zone digest 779d7444… and the
whole LP path are bit-unmoved. 2/3/5/8-zone digests re-pinned with
the record.

## Kill criteria

Specified up front so the tool visibly could have come out the other way —
the discipline that makes a thesis-supporting tool credible.

1. **Validation failure.** If Stage 1 cannot meet its (evidence-based)
   tolerances after the data discrepancies are properly accounted for, do
   not publish adequacy results; diagnose or abandon.
2. **Decomposition failure.** If the Module 3 timescale decomposition does
   not cleanly separate (bands don't approximately sum, or attribution is
   unstable to filter choice), do not publish the "few days vs. 100 TWh"
   argument on the back of it.
3. **Policy-gap instability.** If rule-based vs. perfect-foresight storage
   results differ by more than an order of magnitude on realistic scenarios,
   the dispatch modelling is not robust enough to publish either number.
4. **Contradiction handling.** If a result comes out *against* the working
   thesis (e.g. flexibility clears more than the diurnal band, or nuclear's
   storage-collapse effect is weak), it gets published with the same
   prominence as confirming results. Suppressed contrary results are the one
   thing that would destroy all three tools' credibility at once.

## Explicit non-goals (restated)

- No intra-GB power-flow/network model.
- No EMT simulation.
- No half-hourly market microstructure (bid/offer, BM gaming).
- No demand forecasting — historical traces and stated overlays only.
- No web UI in phase one.
