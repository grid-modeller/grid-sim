# Stage 7 — run report: the costed published-pathway scenarios (stage-close document)

Committed record of the Stage 7 pathway runs (2026-07-06, engine commit
`24a2c6d`). The four scenarios (FES 2025 Electric Engagement and CCC
CB7 Balanced Pathway, each at 2035 and 2050) were built by the
implementer and adversarially adjudicated
(`docs/notes/stage7-pathways-scenarios-review.md`,
ACCEPT-WITH-CONDITIONS; pre-commit conditions 1–3 applied before
`24a2c6d`). **This report's binding contents are the review's §G
obligations 1–8; each is discharged explicitly below** (§G.n markers).
Data provenance: `data/reference/pathways-published.toml`
(pathways-published-v1; data evidence
`docs/notes/stage7-pathways-data-report.md`, adjudication
`docs/notes/stage7-pathways-data-review.md` — its conditions 5–8 were
discharged by the build package, review §E).

Reproduce (the pinned record IS the reproduction — full-precision,
pack-gated, loud-fail):

```
cargo test -p grid-adequacy --test acceptance_stage7_pathways   # 11 tests
cargo test -p grid-core --test pathways_published               # 12 tests
```

(`grid-cli run --scenario scenarios/<name>.toml` reproduces the
per-period series; the acceptance tests pin every headline quoted
here bit-exactly.)

## 0. The framing rule (stated once, governs every claim below)

Every unserved-energy and cost claim in this report is **CONDITIONAL on
the declared conventions**: the observed 2024 weather year and 2024
demand shape (no multi-year record), no electrolysis flexibility (each
source's own published demand basis), **autarky** (interconnection
excluded from dispatch at 20.6–27.9 GW published), flat 1.0
availability (no outage model; hydro keeps the 2024 calibrated energy
limitation), no D9/D10 electrification reprofiling, and 2024
fuel/carbon actuals. The correct sentence is always *"the pathway
fleet shows X under these conventions"* — never "the pathway fails".
Bias directions are named in §2; the a-fortiori structure (the
favourable biases dominate the demand side, so nonzero unserved
survives them) is the review's §F ruling, not this report's invention.

## 1. Headline rows (§G.1) — unserved adjacent to every figure; comparison REFUSED

Every £/MWh figure below is quoted with its unserved energy adjacent
(D8 rule 3(b)). All four cost stacks are **stamped NON-QUOTABLE**
(§4) — the rows are the committed pinned record, not publishable
headlines.

> **STATUS CHANGE (2026-07-06, the battery quarantine lift —
> adjudication `docs/notes/quarantine-lift-review.md`):** condition
> 3.i was discharged against the NREL primary (NREL/TP-6A40-93281;
> the committed numbers CONFIRMED, no pin moved) and the quarantine
> lifted as the parser-enforced reviewed act. The four stacks'
> consumed-quarantine set is now **affirmatively EMPTY** and the
> stacks are **QUOTABLE** (pinned:
> `quarantine_declaration_is_affirmatively_empty_and_stacks_are_quotable`).
> The 3.ii duration-attribution caveat and the 3.iii 2018-vintage
> staleness stamp REMAIN and travel on every artefact; the D8
> rule-3(c) comparison REFUSAL and every convention caveat in this
> report are untouched by the lift.

| Scenario | Demand basis (TWh) | **Unserved (TWh / % of demand)** | Curtailment (TWh) | Storage cycling batt / LDES (TWh dischg) | Stack total, central WACC (£bn/yr) | £/MWh delivered (4.5 / 7.5 / 10 % real) | Q9 gap, central (£/MWh) |
|---|---|---|---|---|---|---|---|
| FES EE 2035 | 450.076 | **1.717 / 0.38 %** | 38.47 | 0.41 / 1.12 | 46.53 | 84.90 / 103.78 / 121.04 | +20.22 |
| FES EE 2050 | 784.736 | **0.870 / 0.11 %** | 21.61 | 0.18 / 1.09 | 69.65 | 71.32 / 88.85 / 104.79 | +7.21 |
| CCC BP 2035 | 443.541 | **0.020 / 0.005 %** | 33.63 | 0.07 / 0.42 | 46.39 | 85.45 / 104.59 / 122.09 | +17.00 |
| CCC BP 2050 | 692.025 | **5.251 / 0.76 %** | 59.04 | 2.10 / 3.56 | 69.79 | 82.20 / 101.62 / 119.35 | +20.03 |

Full-precision values: the pins in
`grid-adequacy/tests/acceptance_stage7_pathways.rs` (`*_pins_are_exact`).

**THE FINDING (the pre-registered shape — reporting rule honoured, no
fleet tuned):** every published pathway fleet shows nonzero unserved
energy under the 2024 weather year on the declared conventions. The
standouts are CCC BP 2050 (5.25 TWh — despite its demand basis
under-loading the fleet by ~13 %, §2) and FES EE 2035 (1.72 TWh).

**Like-for-like cost comparison: REFUSED (D8 rule 3(c)).** The four
scenarios have unequal, nonzero unserved energy (1.72 / 0.87 / 0.02 /
5.25 TWh): their £/MWh figures price DIFFERENT reliability levels and
must not be ranked against each other or against any zero-unserved
system cost. Cost comparisons wait for reliability make-good variants
(fleets augmented to a common adequacy standard) — future work, not
this package. The reliability stamp on every stack carries the honest
standard string: *"not solved to a standard — published-pathway fleet
as-is under 2024 weather (unserved energy is the finding)"*.

## 2. Conventions and bias directions (§G.2) — travel on every quote

**The FES-vs-CCC demand-basis wedge (every comparison surface).** FES
`demand_twh` INCLUDES grid-connected electrolysis as firm load
(7.360 TWh 2035 / 81.910 TWh 2050); CCC `demand_twh` EXCLUDES
surplus-driven electrolysis (29 TWh 2035 / 89 TWh 2050, report p.208).
Never compare the headline demands without the wedge: like-for-like on
a gross-incl-electrolysis basis, **2050 is ~781 TWh (CCC) vs ~785 TWh
(FES EE) — nearly identical**, where the headline 692-vs-785 suggests
a 93 TWh gap; **2035 inverts** (~472.5 vs 450.1 TWh). The same wedge
governs adequacy and curtailment comparisons: the CCC runs are
under-loaded (~6.5 % 2035, ~13 % 2050 — adequacy-FAVOURABLE, unserved
a fortiori on the CCC's own basis) and **CCC curtailment is overstated
as waste by up to 29 / 89 TWh** — that much of any measured surplus is
CCC-intended electrolysis feedstock, not waste (so CCC 2050's 59.0 TWh
curtailment is honestly "≥ ~0 TWh of genuine waste net of the
89 TWh feedstock wedge, ≤ 59.0 TWh" — the wedge brackets it). The FES
side carries the opposite flag: electrolysis as FIRM load overstates
demand rigidity (e6 flexibility-overstatement, adequacy-ADVERSE).

**UK-as-GB (every CCC-derived number carries this stamp).** All CCC
figures are UNITED KINGDOM scope run as GB, unadjusted: Northern
Ireland (~3 % of UK demand, Irish synchronous area) is not separable
from any CB7 table, and demand and fleet embed NI together, so the
supply/demand ratio is preserved and no derate was invented. Direction:
absolute CCC quantities (TWh, GW, £) overstated ~3 %; adequacy ratios
approximately unaffected. (Data review condition 6, discharged and
machine-checked on the scenario descriptions.)

**Bias directions on every unserved/curtailment quote:**

| Convention | Direction | Magnitude handle |
|---|---|---|
| Autarky (no interconnection in dispatch) | adequacy-ADVERSE (unserved includes energy imports might have served) | 20.6 / 24.4 / 20.938 / 27.938 GW published, carried inert |
| No outage model (flat 1.0 availability) | adequacy-FAVOURABLE — unserved a fortiori | whole dispatchable fleet except hydro (2024 calibrated 0.2147) |
| Unlimited-fuel hydrogen turbines / LCD (no H₂ fuel-chain or supply constraint) | adequacy-FAVOURABLE | FES 0.996/27.52 GW; CCC LCD 8.49/38.28 GW |
| No electrification reprofiling (2024 shape scaled) | adequacy-FAVOURABLE (understates winter-evening heat-pump/EV peakiness) | constructed peaks 81.7 / 142.4 / 80.5 / 125.6 GW (the CCC publishes NO peak — a named quarantine gap; the constructed peak is this package's convention, not CCC data) |
| FES marine/geothermal excluded (no CF trace; firm treatment would be invented-favourable) | adequacy-ADVERSE, small | 1.695 GW (2035) / 4.275 GW (2050) |
| CCC smart demand flexibility excluded (peak-GW DSR, not a storage pair — e5 ruling) | adequacy-ADVERSE | 22.0 / 32.55 GW |
| Single weather year (2024), fiscal-vs-calendar label wrinkle | direction unknown; 2024 is not a stress year | multi-year record variants are future work |

The autarky and DSR exclusions are the two biases that argue AGAINST
the finding; both are named on every unserved quote, and neither
converts 5.25 TWh (CCC 2050) into a zero without assuming firm imports
at nameplate — the convention the D11/D13 record rejects (correlated
continental scarcity).

## 3. Partial-coverage adjacency (§G.3) — every £/MWh with its uncosted set

The costed set is exactly the technologies with an honest
costs-reference row (ccgt, ocgt, nuclear, biomass, onshore/offshore
wind, solar). Every pathway £/MWh is a **partial-coverage figure**;
the named uncosted set with magnitudes, pinned by name per scenario
(`costed_coverage_is_pinned_by_name_per_scenario`):

- **FES EE (both years):** ccgt_ccs 7.183 / 26.645 GW (dispatched
  24.78 / 87.91 TWh — unpriced fuel chain), hydrogen_turbine
  0.996 / 27.520 GW (0.47 / 18.22 TWh), waste 3.70 / 2.65 GW, hydro
  2.01 / 2.15 GW, oil 0.24 / 0.06 GW, beccs 4.17 GW.
- **CCC BP (both years):** low_carbon_dispatchable 8.49 / 38.28 GW —
  dispatched 28.01 TWh (2035) and **92.04 TWh (2050): the single
  largest energy source in that run, at zero cost in the numerator
  while its energy sits in the denominator** — other_generation
  7.57 / 5.5 GW, beccs 1.29 GW. **This LCD magnitude must sit adjacent
  to any CCC £/MWh quote.**
- **All four:** the pumped_hydro/LDES stores are uncosted (no
  reference row for the fold): 8.578 GW/89.40 GWh and
  16.578 GW/223.40 GWh (FES), 5.71 GW/312 GWh and 6.92 GW/433 GWh
  (CCC); interconnection capex uncosted (autarky, no cited
  future-link primary); stability services not modelled (lines 4 and
  5 are structural zeros); constraint costs a named zero pending D6.

**The denominator wedge is NEGATIVE on all four scenarios** — central
−6.00 (FES 2035), −15.78 (FES 2050), −9.04 (CCC 2035), −9.78
(CCC 2050) £/MWh — exactly the Q9 review's condition-2b hazard:
uncosted supply serves demand inside the delivered denominator E while
its energy sits outside the weighting basis G. **That wedge must never
be rendered without its co-emitted coverage statement** (the machinery
enforces this: `Q9Decomposition::denominator_wedge()` is the only
accessor and returns the statement with the wedge). The Q9 identity
(mean plant-gate LCOE + utilisation + denominator + missing-line =
headline) closes at ≤ 1e-9 on all four instruments
(`cost_stack_reconciles_on_every_pathway_scenario`), and Σ six lines =
total re-folds exactly.

Reading aid (reviewer note x4): the large positive utilisation wedges
(+21.6 / +19.6 / +22.2 / +24.5 £/MWh central) are substantially the
**realised-vs-assumed capacity-factor gap under 2024 weather** (e.g.
ERA5-calibrated realised offshore CF 0.353 vs the reference's assumed
0.48), not pure fleet idling — the wedge expresses both through the
same rᵢ construction.

## 4. Q9 review conditions 1 + 3 — discharged here (§G.4)

**Condition 1 (docs/04 amendment):** applied 2026-07-06, in the tree —
docs/04 Stage 7 scope now reads: *"`PerfectForesight` policy via
`good_lp` + HiGHS (amended 2026-07-06: the adopted D12 design
implements perfect foresight as a whole-horizon LP function —
`run_multi_lp` and variants — deliberately NOT a per-period
`DispatchPolicy`; the ADR-6 trait is per-period/no-lookahead, so the
LP's physics are LP constraints and the D4 policy choices are absent
from its objective; the policy-contract mechanism remains for future
per-period dispatchers.)"*.

**Condition 3 (affirmative quarantine declaration, never a silent
empty):** for ALL FOUR pathway cost stacks and Q9 decompositions —
**consumed quarantined rows: `storage.battery_li_ion` — NON-QUOTABLE;
the publish path refuses** (`ensure_publishable()` returns
`NonQuotableResult` naming the battery row). Asserted affirmatively
per scenario (`quarantine_flags_propagate_affirmatively_and_publication_is_refused`:
`consumed_quarantined_rows == ["storage.battery_li_ion"]` exactly,
`quotable == false`). The battery staleness stamp travels on every
artefact. No LP gap report attaches to these instruments (§7), so the
gap-report empty-or-not declaration is N/A here; any future pathway
gap report owes it, gated by the tracked item in §6.

> **STATUS CHANGE (2026-07-06):** the battery quarantine was lifted
> (see the §1 banner; adjudication
> `docs/notes/quarantine-lift-review.md`). The affirmative declaration
> above is superseded in the tree by its successor —
> `consumed_quarantined_rows == []` exactly, `quotable == true`, the
> publish path accepts — pinned in the renamed test
> `quarantine_declaration_is_affirmatively_empty_and_stacks_are_quotable`.
> The affirmative-never-silent discipline itself is unchanged; the
> staleness stamp now travels unconditionally (lift or no lift).

## 5. CB7 storage rounding stamps (§G.5)

Every storage-sensitive CCC quote carries: the CB7 storage **energy**
figures are the report Table 7.5.1 **rounded integers as published**
(battery 54 / 139 GWh; medium-duration excl. hydrogen 312 / 433 GWh) —
no machine-readable GWh series exists in the CB7 corpus; and the
medium-duration volume (~55–63 h at published GW) is the CCC's
**storable-energy planning volume, not a single-plant spec**, carried
as the pumped_hydro fold. The stamps are machine-carried
(`energy_precision` in pathways-published-v1) and asserted at the
consumption site. The §1 storage-cycling column (CCC: 0.07/0.42 and
2.10/3.56 TWh discharged) is therefore quoted under this rounding
caveat.

## 6. Tracked item (§G.6) — cited

`memory/project-state.md` carries the tracked item (applied
2026-07-06, in the tree): **scenario-level machine-readable quarantine
wiring** — today the only machine-readable quarantine lives in the
costs reference; gap reports and scenario-consuming artefacts rely on
caller-declared quarantine (honest interim, real forgetting vector).
It must be wired before any pathway gap report is published against
quarantine-touched data (Q9 review condition 3 / pathways review
§G.6).

## 7. Acceptance-index mapping (§G.7) — all three docs/04 Stage 7 lines GREEN

1. **"LP mode on a small hand-checkable scenario matches manual
   optimum"** —
   `lp_dispatch.rs::lp_storage_feasibility_matches_the_hand_computed_minimum`,
   `::lp_soc_convention_matches_rule_based_when_dispatch_is_forced`,
   `::lp_wheels_north_through_middle_to_south_for_zero_unserved`,
   `lp_solve.rs::lp_bisection_recovers_the_single_zone_requirement`.
2. **"LP storage requirement ≤ rule-based on every scenario; gap
   reported per scenario"** —
   `lp_solve.rs::lp_requirement_is_at_most_rule_based_requirement`,
   `::lp_needs_strictly_less_storage_than_rule_based_when_wheeling_helps`,
   `lp_gap_report.rs::gap_report_pins_the_strict_wheeling_gap`,
   `::gap_report_holds_at_equality_where_rule_based_is_optimal`,
   `::gap_invariant_violation_is_a_structured_error`.
3. **"Cost stack reconciles: Σ components = total; Q9 fully
   decomposed"** — `cost_stack.rs` (independent recomputation),
   `q9_decomposition.rs` (identity, synthetic + 2024 reference), PLUS
   the four pathway instruments:
   `acceptance_stage7_pathways.rs::cost_stack_reconciles_on_every_pathway_scenario`
   and the four `*_pins_are_exact`.

**Why the rule-vs-LP gap report does not attach to the pathway
instruments (auditable at stage close):** acceptance line 2 is a
property of the storage-SIZING solvers — "LP storage requirement ≤
rule-based" is a statement about `min_storage_for_zero_unserved` and
its LP twin measuring a solved requirement on the same designation.
The pathway scenarios are **fixed published fleets: their storage is
transcribed DATA (FES FLX1 / CB7 Table 7.5.1), not a solved
requirement** — there is no storage-requirement quantity on these
instruments for the invariant to bind, and no gap to report. The
acceptance line is discharged on the committed solver suites (above);
an LP-vs-rule-based UNSERVED comparison on the fixed fleets would be a
different, unregistered instrument (and would owe the §6 quarantine
wiring plus the D12 quoting caveats before publication).

Additionally the multi-zone engines REJECT the pathway-only merit
rungs by construction (frozen six-rung flow ladder; the scarcity
signal is numerically index-based, so extending it is a
signal-convention re-pin) — tested at
`multizone.rs::extended_dispatch_rungs_are_rejected_by_both_multi_zone_engines`
(run_multi AND the shared LP lookup, mutation-verified
discriminating; review condition 1).

## 8. Nuclear bracket rule (§G.8)

The rule-4/condition-4 obligation — any quoted number with nuclear
content quotes BOTH variants (GHD component build-up AND the Sizewell
C observed project total) — is carried in machine metadata on all four
stacks (`bracket_rules` asserted non-empty, naming nuclear) and is
currently **satisfied trivially: nothing is quotable** (every stack is
battery-quarantined, §4). The obligation becomes live the moment the
NREL re-verification lifts the battery quarantine; the metadata will
still be carrying it.

> **STATUS CHANGE (2026-07-06):** the lift happened (§1/§4 banners;
> adjudication `docs/notes/quarantine-lift-review.md`) — the bracket
> obligation is now **LIVE**: the stacks are quotable, so any quoted
> number with nuclear content owes BOTH variants. The machine
> metadata was already carrying the rule (asserted non-empty on all
> four stacks), exactly as this section anticipated.

## 9. Reproducibility block (docs/06)

- **Engine:** commit `24a2c6d` ("stage7: costed pathway scenarios…"),
  workspace clean of package files at commit; no new dependencies; no
  scenario-schema change (v7; the new technology ids are open-set
  `TechId`); new reference schema `pathways-published-v1` registered
  in docs/03 with pinned tests.
- **Scenario checksums (sha256, as committed):**
  - `scenarios/fes2025-ee-2035.toml` `99e53c1886af6fbd77eb1511360879dc1119b585e477507182b5ed37d2641a71`
  - `scenarios/fes2025-ee-2050.toml` `a87263aa3793b87a38d5e21115a612dfaee6d329c6605f2190829ccd8f22d6eb`
  - `scenarios/ccc-cb7-bp-2035.toml` `4dccd5f0eb1ff77e387d709562142a1d9d3dd1571e5c40fc0f83e0f56479febf`
  - `scenarios/ccc-cb7-bp-2050.toml` `806cd2b778ebc665022d8f86a4759861f0405c3746b042aebec04455d0a4e6f1`
  - `data/reference/pathways-published.toml` `8ab05f4002d371e8995293228f7e979c788c9ee703ba39339929180ecb24f5c5`
  - `data/reference/costs-gb.toml` `28a1a67b0491581f816d3cfd669217423aceffc3ff6f88392eac906238c62647`
    *(as consumed by the pinned runs at commit `24a2c6d`; the file was
    since revised by the battery quarantine lift — comment/flag
    changes only, every numeric value byte-identical (CONFIRMED not
    corrected) — current sha `a411e6a5bff8a1b1…`; see the §1/§4
    STATUS CHANGE banners and `docs/notes/quarantine-lift-review.md`.
    To reproduce the runs byte-exactly, use the file at `24a2c6d`.)*
- **Data packs (fetched-and-built, manifests committed):**
  `data/packs/2024.sha256` (2024 demand/CF/price traces; measured
  2024 underlying-demand trace total 261.8258865 TWh is the
  annual_scale denominator), `data/packs/fes2025.sha256` (5/5 OK),
  `data/packs/cb7.sha256` (4/4 OK, re-verified at review);
  `data/reference/prices-2024.toml` (D8 rule 1.2 SRMC chain).
- **Suite:** at adjudication 71 suites, **691 passed / 0 failed /
  4 ignored** (the 4 = tractability benches; reviewer's own run);
  review conditions 1–3 then added one test (multizone 27 → 28),
  re-verified green with fmt/clippy clean and digest spot-checks
  (779d7444…, 2-zone) before commit. All committed digests unmoved
  throughout (779d7444…, 2/3/5/8-zone, prices — reviewer-run, §A of
  the review).
- **Independent reproduction (review §B):** the CCC BP 2050 row was
  re-derived by the reviewer with an independent Python dispatch
  reconstruction from the pack CSVs — demand/curtailment/LCD/nuclear
  identical, unserved and battery discharge to 1 ulp; the cost-stack
  total re-derived from independent CRF arithmetic with **diff 0.0**
  and the headline bit-equal; FES EE 2035 reproduced as a second full
  row; all Q9 anchors re-derived and the identity closed. Determinism:
  rerun bit-equality asserted
  (`pathway_runs_and_decompositions_are_deterministic`); rule-based
  only (no HiGHS), so the pins are digest-grade with no cross-machine
  solver caveat.

## 10. Named gaps (built what the data supports; each named, none silently filled)

1. **CCC peak demand** — none published (CB7 quarantine 3); the
   constructed peak (2024 shape × scale) is a convention of this
   package.
2. **Pumped/LDES store capex** — uncosted (no reference row for the
   fold); magnitudes in §3.
3. **Hydrogen / CCS / LCD fuel chains** — unpriced and uncosted
   (no cited primary in costs-reference-v1); dispatched energies in §3
   so the missing £ is boundable.
4. **No electrification reprofiling** (D9 heating / D10 EV overlays
   not applied) — direction stated in §2; D10 is promoted into v1
   scope by the book programme and is the named fix.
5. **Single weather year** — the 1985–2024 record variants (the RS
   precedent) are the natural extension; no multi-year claim is made.
6. **Multi-zone pathway variants** — post-beta; require the frozen
   flow-ladder signal-convention re-pin (§7).
7. **Reliability make-good variants** — required before any
   like-for-like cost comparison (§1 refusal).
8. **Scenario-level quarantine wiring** — tracked (§6).

— implementer, Stage 7 stage-close, 2026-07-06
