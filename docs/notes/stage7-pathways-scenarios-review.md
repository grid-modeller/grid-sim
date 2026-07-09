# Stage 7 published-pathways BUILD package — reviewer adjudication

**Independent reviewer (stage-7 pathways gate), 2026-07-06.**
Adversarial review of the FINAL Stage 7 build package (uncommitted):
NEW `grid-core/src/pathways_published.rs` + `grid-core/tests/
pathways_published.rs`, four scenarios (`fes2025-ee-2035/2050.toml`,
`ccc-cb7-bp-2035/2050.toml`),
`grid-adequacy/tests/acceptance_stage7_pathways.rs`; MODIFIED
`dispatch.rs`/`flow.rs`/`multizone.rs`/`lp.rs` (the merit-ladder
split), `engine.rs` (committed-test pin update), `grid-core`
`error.rs`/`lib.rs`, `docs/03` (schema registration). Reviewed against
docs/04 Stage 7 (pinned rules), the data review's binding conditions
5–8 (`stage7-pathways-data-review.md`), the Q9 review's run-report
obligations (`stage7-q9-review.md` conditions 1+3), D8
(`d8-lcoe-methods.md`), docs/02/05/06. Nothing taken on the
implementer's word: every gate re-run, the standout row re-derived
with independent code, the engine-change necessity claim re-derived by
mutation.

## VERDICT: ACCEPT-WITH-CONDITIONS

Every number I re-derived reproduces — the CCC BP 2050 row to the ulp
from my own Python dispatch reconstruction (CSV twins of the packs)
and the cost stack bit-exactly from my own CRF arithmetic; the FES EE
2035 row likewise (a second full independent row). The one engine
change (the two-ladder split) is verified NECESSARY by mutation: I
extended the frozen flow ladder in a scratchpad copy and the committed
2-zone digest moved (SCO dispatch digest b347095d… ≠ pinned 23d9eac5…)
— exactly as the implementer claimed to have measured. All committed
digests unmoved in the real tree. Four conditions (1–3 pre-commit,
4 at commit), plus the run-report obligations in §G, which the Stage 7
run report MUST satisfy before the stage tags.

## A. Gates (run by this reviewer)

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean; `cargo test --workspace`: **71 suites,
  691 passed / 0 failed / 4 ignored** (the 4 = tractability benches).
- Digests unmoved, all re-run this session:
  `pinned_2024_reference_result_digest` (779d7444…),
  `two_zone_dispatch_digests_are_pinned`,
  `three_zone_dispatch_digests_are_pinned`,
  `migrated_5zone_scenario_dispatch_digests_are_unmoved`,
  `composed_8zone_scenario_dispatch_digests_are_pinned`,
  `pinned_2024_prices_digest` — all ok.
- `cb7.sha256` re-verified 4/4 OK on disk; 2024 pack present; the
  acceptance file's `require_pack()` fails loudly on a missing pack
  (audit standing rule honoured — asserted, not self-skipping).
- No Cargo.toml change, no new dependency. Scenario `schema_version`
  unchanged at 7 (the new technology ids are open-set `TechId` — no
  scenario-schema change needed); the NEW reference schema
  `pathways-published-v1` is registered in docs/03 with its semantics
  and pinned tests (ADR schema discipline satisfied).

## B. Re-derivation — the headline rows REPRODUCE

**CCC BP 2050 (the standout finding), fully independent probe** (my
own Python dispatch reconstruction from the committed scenario + the
pack CSVs; policy re-implemented from the policy.rs prose — √η legs,
greedy charge-from-surplus-only, ascending dispatch_order, initial SoC
full):

| Quantity | My probe | Package pin |
|---|---|---|
| demand | 692,025.0000000029 GWh | identical |
| unserved | 5,251.239356108108 | 5,251.239356108109 (1 ulp) |
| curtailment | 59,041.07733918076 | identical |
| battery discharge | 2,102.2177553370066 | 2,102.2177553370093 (ulp) |
| pumped/LDES discharge | 3,564.2301580749117 | identical |
| LCD dispatch | 92,039.19437484704 | identical |
| nuclear dispatch | 60,324.31599526048 | identical |

Cost stack, central WACC, my own CRF arithmetic from `costs-gb.toml`
literals (CRF = r/(1−(1+r)^−n), overnight capex central +
infrastructure, FOM+insurance+connection, nuclear 16.1 £/MWh adder ×
its dispatched energy, battery 262 £/kW + 135 £/kWh at CRF(0.075,15) +
12.9 £/kW-yr): **total = 69,788,536,072.3623 £ — diff 0.0 against the
pin**; headline 69.7885e9 / 686,773,760.6 MWh delivered =
**101.61794184875532 £/MWh — bit-equal**. Q9 anchors re-derived with
my own arithmetic: mean plant-gate LCOE 81.58485646344826 (pin …28),
utilisation wedge 24.536354889036808 (pin …850; r_i = realised CF /
the row's assumed CF, nuclear r=1 — the committed q9 convention),
denominator wedge −9.775548513799762 (pin …820), missing-line wedge =
battery line / E = 5.2723. The identity closes:
81.584856 + 24.536355 − 9.775549 + 5.272279 = 101.617942 = headline,
exactly as pinned. The wedge arithmetic in the implementer's claim
(81.58 + 24.54 − 9.78 + 5.27 = 101.62) is confirmed.

**FES EE 2035, second independent row**: unserved 1,716.9369856648711
(pin …705), curtailment 38,469.743557467846 (identical), battery
407.274, pumped 1,115.234, and all four thermal pins including
hydrogen_turbine 466.1813848422092 GWh — identical. The hydro
flat-0.2147 ceiling reproduces the committed 2024-reference convention
(scenarios/gb-2024-reference.toml:272).

**Fleet/demand spot-checks**: FES fold reassembly e1/e2 verified
against the parsed reference — 2035 ccgt 12.9138 + ccgt_ccs 7.183 =
20.0968 (the published fold), biomass 1.245 + beccs 4.17 = 5.415;
2050 4.39 + 26.645 = 31.035, 1.0433 + 4.17 = 5.2133. All four
`annual_scale` values reproduce from the measured trace total
(261.8258865 TWh) to the last digit; run demand lands on the published
pathway demand at < 1e-9 relative (probed and pinned). S1 arithmetic:
29.71 × 12.9138/19.9693 = 19.21294 → 19.2129 at 4 dp; halves
reassemble 29.71 exactly.

## C. The merit-ladder split (the one engine change) — VERIFIED, one test gap

(i) **Digests unmoved** — run myself, §A. The committed 2024 path
depends only on the six Stage 1 rungs' relative order, which the
extension preserves (asserted directly in the updated engine.rs test).

(ii) **The split was NECESSARY, not convenient — verified by
mutation.** The flow-rule scarcity signal is numerically index-based
(`signal = ladder index + fractional utilisation`; unserved region =
`LADDER_LEN + unserved`; inter-rung gaps are real distances in signal
space, so the equalising-flow crossing points move when rungs are
inserted even if no scenario names them). I set `FLOW_MERIT_ORDER` to
the 13-rung dispatch ladder in a scratchpad copy:
`two_zone_dispatch_digests_are_pinned` FAILS (zone SCO digest moved).
The frozen six-rung `FLOW_MERIT_ORDER` is the only way to extend the
single-zone ladder without a multi-zone re-pin. Confirmed.

(iii) **The relative-order invariant test**
(`flow_ladder_is_a_relative_order_subset_of_the_dispatch_ladder`) is
discriminating for what it guards: it kills any reorder of the shared
six in either ladder and any removal from MERIT_ORDER. It does NOT
(and cannot) kill an extension of the flow ladder — that mutation is
killed by the committed multi-zone digests, as I verified in (ii). The
two protections are jointly sufficient; recorded here so nobody later
mistakes the invariant test for the extension guard.

(iv) **Multi-zone rejection of an extended-only rung: mechanism
VERIFIED, test MISSING.** My probe (a 2-zone scenario naming
`hydrogen_turbine`, run through `run_multi`) is rejected with
`GridError::UnknownThermalTechnology` as the flow.rs docs promise; the
LP path shares the same `FLOW_MERIT_ORDER` lookup (lp.rs:1266). But
the package carries NO test for this documented behaviour — the only
rejection test is the committed single-zone "fusion" case. Documented,
load-bearing behaviour with no test violates the red-green rule
(CLAUDE.md; docs/06). **Condition 1.**

(v) **The engine.rs committed-test edit is a genuine pin update, not a
weakened gate.** `merit_order_is_the_documented_stage_1_stack` →
`merit_order_is_the_documented_stack`: still an exact full-array
equality pin (now 13 rungs), PLUS a new direct assertion of the
Stage 1 relative order. Strictly stronger than before given the
deliberate ladder change. The two NEW engine tests
(`stage7_pathway_thermal_rungs_are_dispatchable`,
`stage7_pathway_rungs_dispatch_in_documented_order`) pin the extended
rungs' dispatchability and order. Rung placements are documented in
prose at the definition site (docs/06 obligation), with the correct
observation that adequacy outcomes are ordering-invariant.

**Defect found in passing (condition 2):** the
`UnknownThermalTechnology` error message (grid-core/src/error.rs:369)
still enumerates "the Stage 1 merit order (nuclear, biomass, hydro,
coal, ccgt, ocgt)". For the single-zone path this is now WRONG (13
rungs are accepted); for the multi-zone path it is literally true but
omits the essential guidance (the technology IS valid single-zone; the
multi-zone ladder is frozen pending a signal-convention re-pin).

## D. Conventions — adjudication (task C, each ruled)

1. **CCC electrolysis-excluded demand basis, wedge travelling —
   SOUND.** Running the CCC's own published basis rather than adding
   29/89 TWh as firm load is right: surplus-driven electrolysis is by
   definition interruptible; firm treatment would manufacture unserved
   the pathway never implies. The a-fortiori logic holds: added load
   can only increase unserved, so the nonzero unserved findings
   survive the under-loading; curtailment overstatement (up to
   29/89 TWh is intended feedstock, not waste) is named with magnitude
   in both CCC headers and the acceptance docstring. Machine
   visibility: the wedge lives at BOTH sites in the reference, the
   parser enforces they stay in step (tested on a fixture), and the
   acceptance test asserts the magnitudes. The remaining surface is
   the run report: every published FES-vs-CCC demand/adequacy/
   curtailment comparison must carry the wedge (§G.2) — the pins
   themselves are not comparisons.
2. **UK-as-GB unadjusted — SOUND.** Demand and fleet both embed NI, so
   the supply/demand ratio is preserved and no derate is invented (the
   data review's own instruction that a transcription layer must not
   invent one extends naturally here); direction stated (absolute
   quantities ~3% overstated, adequacy ratios ≈ unaffected);
   declaration machine-checked on the scenario description (data
   review condition 6 DISCHARGED).
3. **S1 gas split by FES EE class ratio — SOUND, one wording defect.**
   The adequacy-invariance claim is VERIFIED: my probe lumps all firm
   capacity and reproduces unserved/curtailment to the ulp, so the
   CCGT:OCGT ratio cannot move adequacy. But the header's "it moves
   only the SRMC/cost attribution between the ccgt and ocgt lines"
   UNDERSTATES: the two classes carry different efficiencies, so the
   split moves the line-2 TOTAL (and the headline), not merely its
   attribution — a different declared ratio would change the £
   number. The rule itself is a properly labelled, pinned,
   contemporaneous-source construction; only the claim about its
   effect must be exact. **Condition 3.**
4. **S2 low_carbon_dispatchable as one uncosted entry — SOUND.** The
   strawman-avoidance argument is correct and important: excluding
   38.28 GW (2050) of the pathway's main firm backstop would turn the
   adequacy run into a straw man; carrying it at published GW under
   the open-set id with NO invented component split respects the data
   review's mappable=false law (the parser makes silent consumption
   impossible — the type system carries aggregates as
   `ExcludedAggregate`, never fleet). The cost-stack effect is real
   and honestly stamped: in CCC 2050 the LCD serves 92.04 TWh — the
   single largest energy source — at zero cost in the numerator while
   its energy sits in the denominator; the uncosted set is pinned BY
   NAME per scenario (`costed_coverage_is_pinned_by_name_per_scenario`),
   the coverage statement co-emits, the headline is a stated
   partial-coverage figure, and the whole stack is non-quotable today
   anyway (battery quarantine). Run-report obligation: the LCD GW/TWh
   magnitude adjacent to any CCC £/MWh quote (§G.3), as the header
   itself promises.
5. **Autarkic interconnection — SOUND.** Adequacy-ADVERSE, named on
   every unserved quote; the D11/D13 record genuinely supports
   refusing firm-at-nameplate imports (correlated continental
   scarcity); the published GW is carried on an inert link and
   machine-checked (capacity = published, availability = 0.0). The
   uncosted link capex is a named cost limitation.
6. **Flat availability 1.0 except hydro — SOUND.** Direction correct
   (favourable → unserved findings a fortiori), named; hydro keeps the
   committed 2024 calibrated energy limitation (a physical constraint,
   not an outage — verified identical to the reference scenario); the
   deliberate non-application of the 2024 AGR profile to a
   new-build/SMR fleet is well reasoned and assigns the question to Q7.
7. **Marine/geothermal exclusions — SOUND.** No CF trace exists;
   treating cyclic tidal as firm would be an invented favourable
   convention; adequacy-ADVERSE, small, magnitudes named, and
   machine-checked (present in the reference, asserted ABSENT from the
   fleet).
8. **Greenfield framing + named uncosted/unpriced sets — SOUND.**
   D8-rule-8 framing named (`CostFraming::Greenfield` — a pathway
   fleet is a future build); the costed set is exactly the
   technologies with an honest reference row; costing abated/novel
   plant at unabated rates is rightly refused; pumped/LDES stores
   uncosted with magnitude named; SRMC = 2024 actuals is a stated
   convention ("this fleet under 2024 weather AND 2024 prices").

## E. Data-review conditions 5–8 — all four DISCHARGED

- **5 (parser enforces mappable=false + stamps):** discharged.
  Aggregates parse into `ExcludedAggregate` (never fleet), mappable =
  true is a structured parse error, fleet-name collision is a parse
  error, all tested on fixtures; `energy_precision` parsed and
  asserted present at the consumption site. Registered in docs/03 per
  the costs-reference-v1 precedent, with pinned regression tests on
  the committed file. Residual (run report): the stamp must travel
  onto any artefact quoting CB7 storage energy (§G.5).
- **6 (UK-as-GB declared):** discharged, machine-checked (§D.2).
- **7 (wedge on comparisons; constructed peak; fiscal wrinkle):**
  discharged at package level — wedge machine-visible + asserted, the
  constructed peak declared as a convention (CCC header, ~80.5 GW
  2035), fiscal-vs-calendar stated in the FES header. Comparison
  surfaces beyond this package: §G.2.
- **8 (splits are this package's reviewed decisions):** discharged —
  S1/S2/S3 + other_generation each carries a declared rationale in the
  scenario headers, adjudicated in §D above, and is pinned by
  acceptance tests that recompute S1 from the reference itself and
  assert exact reassembly. Every published-number obligation (docs/05
  rule 3) is met by full-precision pack-gated pins.

## F. Honesty of the finding — PASS

- The publish path REFUSES today, affirmatively: the acceptance test
  asserts `consumed_quarantined_rows == ["storage.battery_li_ion"]`
  exactly (never a silent empty — the Q9 condition-3 shape, NOT-EMPTY
  by name), `quotable == false`, and `ensure_publishable()` returns
  `NonQuotableResult` naming the battery, on all four scenarios.
  Battery staleness stamps and the nuclear bracket rule travel on
  every pathway artefact (asserted).
- The reliability stamp carries the run's unserved energy and the
  honest standard string ("not solved to a standard — published-pathway
  fleet as-is under 2024 weather (unserved energy is the finding)") —
  the D8 rule-3(a) fixed-fleet convention.
- No overclaim anywhere in the package: no "fails"/"blackout" language
  in any scenario header, test doc, or module doc (swept); the
  acceptance docstring states the correct conditional shape ("how does
  this PUBLISHED fleet perform under the OBSERVED 2024 weather year",
  unserved is the finding, no fleet tuned, biases named with
  direction). The CCC 2050 5.25 TWh figure is everywhere conditional
  on the declared conventions (under-loaded demand basis — which is
  adequacy-FAVOURABLE, making the finding a fortiori on its own basis;
  no electrolysis flex; autarkic — adverse, the one bias that argues
  the other way and is named; 2024 weather, single year).
- The Q9 review's run-report obligations are correctly STILL OPEN —
  this package does not pretend to discharge them: docs/04 was not
  edited (condition 1 owed at run-report time, and only then), and the
  affirmative quarantine declaration is proven at test level with the
  run-report statement still to be written. One loose end found: the
  project-state tracked item required by Q9 condition 3 (wire
  machine-readable scenario-level quarantine before pathway-pack gap
  reports are published) is NOT in memory/project-state.md — §G.6.

## G. What the Stage 7 run report MUST contain (binding on the stage close)

1. **D8 rule-3 discipline on the four headlines.** The scenarios have
   unequal, nonzero unserved (1.72 / 0.87 / 0.02 / 5.25 TWh). Every
   £/MWh figure is quoted with its unserved TWh adjacent (rule 3(b));
   the four headline costs are NOT presented as a like-for-like
   comparison — cost comparisons wait for reliability make-good
   variants (rule 3(c)), or are explicitly refused in the report.
2. **The demand-basis wedge on every FES-vs-CCC surface** (like-for-like
   ~781 vs ~785 TWh at 2050; 2035 inverts), the UK-as-GB stamp on
   every CCC-derived number, and the autarky/no-outage/no-reprofiling
   bias directions on every unserved/curtailment quote.
3. **Partial-coverage adjacency:** every quoted pathway £/MWh carries
   its named uncosted set with magnitudes (CCC: LCD 8.49/38.28 GW —
   92.04 TWh dispatched in 2050 — other_generation, BECCS; FES:
   ccgt_ccs, hydrogen_turbine, waste, hydro, oil, beccs; pumped/LDES
   stores uncosted), and the coverage note sits adjacent to the
   denominator wedge on any rendered Q9 chart (the wedge is NEGATIVE
   on all four scenarios — measured −6.00 / −15.78 / −9.04 / −9.78
   central — exactly the Q9 review's condition-2b hazard).
4. **Q9 review conditions 1 + 3 discharged there:** the dated docs/04
   amendment line (perfect foresight as whole-horizon LP, wording in
   the Q9 review), and the affirmative quarantine declaration for
   every artefact quoted ("consumed quarantined rows: storage.battery_li_ion
   — non-quotable; publish path refuses" for the stacks; the
   empty-or-not statement for any gap report).
5. **CB7 storage rounding stamps** on any storage-sensitive CCC quote
   (rounded-integer GWh; planning volume, not plant spec).
6. **Add the missing project-state tracked item** (Q9 condition 3):
   scenario-level machine-readable quarantine wiring before pathway
   gap reports are published against quarantine-touched data.
7. **State why the rule-vs-LP gap report does not attach to the
   pathway instruments** (fixed published fleets; storage is data, not
   a solved requirement — acceptance line 2 is discharged on the
   committed solver suites), so the acceptance-index mapping is
   auditable at stage close.
8. **The nuclear bracket rule** (both variants) on any quoted number
   with nuclear content, per the docs/04 pin — currently satisfied
   trivially because nothing is quotable.

## Conditions (ordered)

1. **(Pre-commit, test gap)** Add the multi-zone rejection test: a
   multi-zone scenario naming an extended-only rung (e.g.
   `hydrogen_turbine`) is rejected with `UnknownThermalTechnology` —
   `run_multi` path at minimum; cover or explicitly note the shared
   LP-path lookup (lp.rs:1266). My probe verified the mechanism; the
   package must own the test (red-green: documented behaviour,
   currently untested).
2. **(Pre-commit, error accuracy)** Update the
   `UnknownThermalTechnology` message (error.rs:369): it enumerates
   the six-rung Stage 1 ladder as "the merit order", which is now
   false for the single-zone path and unhelpfully incomplete for the
   multi-zone rejection (should say: valid single-zone rungs vs the
   frozen multi-zone ladder pending re-pin). Keep it one structured
   error; just make the text true.
3. **(Pre-commit, wording)** Correct the S1 claim in
   `ccc-cb7-bp-2035.toml`: the CCGT:OCGT split moves the priced
   line-2 TOTAL (different efficiencies → different fuel+carbon per
   MWh), not "only the attribution between the lines". The
   adequacy-invariance half of the sentence is verified and stays.
4. **(At commit)** Red-first commit structure per house practice (the
   Q9 condition-4 precedent), the verification evidence in the commit
   body, and keep the UNRELATED uncommitted book-programme insert in
   `memory/project-state.md` out of this package's commit (separate
   workstream, supervisor's file).

## Notes of record (non-blocking)

- x1. The acceptance-line index in the test-file header names every
  committed test it relies on; I verified each named test exists and
  passed this run. All three docs/04 Stage 7 acceptance lines are
  green: line 1 (LP vs manual) and line 2 (LP ≤ rule-based + gap) on
  the committed suites, line 3 (Σ components = total + Q9) on the
  committed suites PLUS the four pathway instruments
  (`cost_stack_reconciles_on_every_pathway_scenario` and the pinned
  identity closure re-verified here).
- x2. Determinism: rerun bit-equality asserted
  (`pathway_runs_and_decompositions_are_deterministic`); the parser is
  pure and deterministic (asserted); no wall-clock/globals/randomness
  in any new code (read in full).
- x3. The pins are full-precision f64, pack-gated, rule-based-only (no
  HiGHS), so they are digest-grade with no cross-machine solver
  caveat, as claimed.
- x4. The realised-vs-assumed CF gap (ERA5 2024 realised offshore
  0.353 vs the reference's assumed 0.48) is a large part of every
  utilisation wedge — correctly expressed THROUGH the wedge, but worth
  a sentence in the run report so nobody reads the +24.5 £/MWh (CCC
  2050) as pure fleet idling.
- x5. `figures/` untracked predates this package (Q9 review, scope
  note) — still housekeeping.

— independent reviewer (stage-7 pathways gate), 2026-07-06
