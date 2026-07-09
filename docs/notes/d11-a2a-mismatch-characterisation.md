# D11 — A2a mismatch characterisation and the priced-ladder verdict

Engine implementer, 2026-07-05. Phase 0 of the D11 tier-2 engine
package (docs/notes/d11-priced-dispatch.md, ADOPTED; the work-order
instruction carried from docs/notes/d11-per-zone-price-data-report.md
§6: re-characterise the A2a residual from **observed** FR marginal
technology before quoting any A2a number), plus the measured
priced-ladder outcome. Every quoted engine number is pinned in
`grid-adequacy/tests/acceptance_d11_priced_ladder.rs`; the Phase-0
shares derive from committed pack data + the committed scarcity-rule
run as described in §1.

## 0. Verdict, one paragraph

**The ≥ 95 % A2a priced-ladder target (expectation 97.4 %) is
UNREACHABLE on 2024 price data, and that is the pre-registered D11
rule-4 finding, not a defect.** The A2a residual is dominated by
genuinely-both-gas-marginal periods (§2), the 2024 GB-vs-EU carbon
wedge is ~nil (+£0.17/tCO2 — the D11 data package's carbon-parity
result), and the only per-zone price difference the committed,
licence-clean data can express in those periods is a **granularity
artifact** — a fortnightly GB UKA-step (+CPS) series against a flat
annual EUA mean — whose sign flips through the year. Measured on the
committed convention the priced ladder scores **A2a = 71.69 %**, a
*regression* from the scarcity rule's 90.07 %; under the most
ladder-favourable (flat-flat) carbon convention it scores **93.18 %**
— still below the target. A sub-noise wedge choice moves A2a by
~21.5 pp, so **the both-gas class is not price-identifiable on 2024
licence-clean data**. The ladder is built, tested, selectable
(`dispatch.flow_signal = "priced_ladder"`) and pinned; the scarcity
rule remains the default and the committed validated behaviour.

## 1. Method

- **The mismatch set.** The committed 5-zone scenario under the
  committed scarcity rule (the Stage 5 acceptance configuration,
  pins unmoved: A2a 15,823/17,568 = 90.07 %). Per period, modelled
  GB↔FR net flow (IFA+IFA2+ElecLink, home end) vs the observed
  ENTSO-E `fr_net`, 3-class direction under the 50 MW dead-band —
  the exact A2a arithmetic of `acceptance_stage5_2024.rs`.
  Total mismatches **1,745**; the model-export/obs-import class
  (the stage-5 §2.4 "A2a residual") **1,297**, of which **1,122
  (86.5 %) are model-labelled both-gas-marginal** — both reproduce
  the stage-5 record exactly.
- **Observed FR margin.** From `fr_generation_2024` (ENTSO-E
  generation-by-fuel, committed pack): per-period FR `fossil_gas`
  against a must-run floor (CHP/heat-led gas that runs regardless of
  the margin), and FR `nuclear` against its monthly availability
  ceiling (61.370 GW × the scenario's pinned monthly factors — if
  nuclear is at its ceiling it cannot be the flexing margin).
  Floors are reported as sensitivity bands, not a single tuned value.
- **Observed GB margin.** FUELHH `ccgt + ocgt` > 1 GW.
- **The carbon wedge, per period.** GB = the committed UKA auction
  step series (forward-filled) + £18 CPS (prices-2024.toml); EU = the
  committed flat £55.01/tCO2 EUA mean (prices-eu-2024.toml — the only
  licence-clean form; no open daily EUA series exists).

## 2. The measured shares

**GB was really gas-marginal in effectively every mismatch period**
(observed GB gas > 1 GW in 1,745/1,745 = 100 %).

**FR was really running gas above its must-run floor in most of the
residual** — the "FR genuinely non-gas-marginal while GB gas-marginal"
(ladder-fixable) share of the 1,297-period class is a MINORITY under
every classification, and its size is convention-sensitive:

| Classification of "FR really non-gas" | share of the 1,297 class |
|---|---|
| gas ≤ annual p05 floor (0.43 GW) | 0.2 % |
| gas ≤ annual p10 floor (0.46 GW) | 0.9 % |
| gas ≤ annual p20 floor (0.49 GW) | 6.1 % |
| gas ≤ monthly p05 floor + 0.1 GW | 14.4 % |
| gas ≤ monthly p05 floor + 0.5 GW | 39.3 % |
| gas ≤ monthly p05 floor + 1.0 GW | 51.1 % |
| nuclear headroom > 0.5 GW below monthly ceiling | 5.2 % |
| nuclear headroom > 1.0 GW | 3.8 % |

The two *physically anchored* observables agree on a small share: FR
gas sat above even generous seasonal must-run floors (monthly p05:
~2 GW in winter, ~0.4 GW in summer; FR gas in the mismatch class:
p10 0.53 / p50 2.42 / p90 6.48 GW), and FR nuclear was **at its
availability ceiling in ~95 % of the class** (vs 67.5 % of the whole
year) — so the flexing margin in these night/shoulder hours was gas
(or hydro priced against gas), not spare nuclear. The wide monthly-
floor-plus-margin readings (up to 51 %) are the honest upper
uncertainty from not knowing the true heat-led CHP floor; even taking
51 % at face value, the static ceiling is 90.07 % + 0.51 × 1,297 /
17,568 = **93.8 % < 95 %**. The 97.4 % expectation assumed the WHOLE
class was priceable away; the observed data says it is not.

Cross-tab against the model's own labels (all 1,745 mismatches, p10
floor): model-gas ∧ real-gas 1,444; model-nongas ∧ real-gas 270;
model-gas ∧ real-nongas 10; model-nongas ∧ real-nongas 21. The
model's FR state is broadly *right* about FR being gas-marginal in the
residual — the residual is not a mislabelling artifact.

## 3. The carbon term the ladder actually sees (the artifact)

Per period, GB effective carbon (UKA(t)+CPS) spans **£50.10–64.92**
(mean £55.13) against the flat EUA £55.01: GB is dearer in **44.5 %**
of periods and cheaper in 55.5 %, and within the 1,122 both-gas
residual class GB is dearer in only **410/1,122**. Because the ladder
is deliberately bang-bang on flat bands (D11 rule 1: a non-zero SRMC
gap runs to a rung edge or the cap), this sub-noise, sign-flipping
±£5/tCO2 wedge — an artifact of comparing a fortnightly auction series
to an annual mean, both committed exactly as the licence-clean data
allows — decides the direction of EVERY both-gas period, including the
~9,550 both-gas periods the scarcity rule already got right (5,332 of
which have GB-cheaper carbon and flip to wrong-way exports).

Static prediction from these shares: best case 92.5 %, sign-risk
realised 62.2 %. **Measured (dynamic, full engine): 71.69 %.**

## 4. The measured priced-ladder record (pinned)

All on the committed 5-zone scenario with `flow_signal` flipped to
`priced_ladder` in-memory (the committed file keeps the scarcity
default, so every v6-era digest and gate is byte-identical; pins in
`acceptance_d11_priced_ladder.rs`):

| Quantity | scarcity rule (committed) | priced ladder | band |
|---|---|---|---|
| A2a GB↔FR direction match | 90.07 % (15,823) | **71.69 % (12,595, pinned)** | ≥ 95 % target — **MISSED**; committed two-limb gate ≥ 88 % untouched |
| A2b export recall | 78.96 % | **63.87 %** | ≥ 70 % — regressed |
| A1 GB net imports | +35.94 TWh (+7.9 %) | **+25.70 TWh (−22.8 %)** | ±10 % — regressed |
| A1 GB gas | 71.80 TWh (−1.36 %) | **82.20 TWh (+12.9 %)** | ±5 % — regressed |
| A3 continental imports vs GB wind r | −0.342 | **−0.185** | ≤ −0.25 — regressed |
| A4 BE (Nemo) net | +2.82 TWh | **+0.79 TWh** | ±1.5 of +4.16 — regressed |
| A4 NL (BritNed) net | +2.82 TWh | +0.79 TWh | ±1.5 of +1.59 — passes |
| NO2 (NSL) net | +9.63 TWh | +9.49 TWh | (no gate) ~unchanged |
| **Sensitivity: A2a, flat-flat carbon** (GB £55.18 vs EUA £55.01) | — | **93.18 % (16,370, pinned)** | still < 95 % |

Every regression is under the LADDER only. The committed Stage 5
gates, the Module 5 capacity-credit table, the B6 gates and robustness
numbers, the B4 pins (rule-based 1.96 %; LP band [0.2346, 0.2816]) and
the Q2/Q10 60 GW pins all run on the scarcity rule and are
**unchanged and green** (full suite 611/0/4, 2026-07-05).

The flat-flat sensitivity is the strongest single statement of the
finding: two equally-defensible conventions for the SAME committed
price levels — per-period-GB-vs-flat-EU (faithful to the committed
series) versus flat-vs-flat (granularity-consistent) — produce 71.69 %
and 93.18 %. A wedge of +£0.17/tCO2 (≈ £0.06/MWh on CCGT SRMC, an
order of magnitude below the price data's own uncertainty) moves the
headline by ~21.5 pp. Neither number validates a price model; the
number is a property of the carbon-series convention, which is why the
committed-convention 71.69 % is pinned as the finding rather than
either number being offered as the tier-2 A2a.

Why even the favourable convention misses: with a constant GB-dearer
wedge the ladder imports at the cap in essentially every both-gas
period, which fixes the 1,122-class but flips a smaller set of
correctly-matched export/idle periods to imports (confusion matrix in
the test output), landing at 93.18 % — the bang-bang behaviour cuts
both ways.

## 5. What stands, what changes

- **The engine deliverable stands**: the priced ladder is implemented
  per the ADOPTED design (lexicographic signal, £0 floor, run-scope
  convention-3 unserved ceiling, scarcity tiebreak retained
  everywhere, bang-bang stated), with the degradation guarantee
  (equal per-zone SRMCs ⇒ byte-identical to the scarcity rule) and
  determinism pinned by property tests, and the single-zone reference
  digest provably unreachable by the flow rule (zero borders).
- **The policy set is {scarcity-rule, priced-ladder,
  perfect-foresight-LP}** (ADR-6 generalisation, proposed amendment).
  The scarcity rule remains the default and the only signal any
  committed validated number uses. The ladder is selectable and its
  2024 record is this note.
- **The B6 ~nil-delta confirmation** (D11 rule 4): the B6
  dispatch-convention excess lives in £0-surplus periods where the
  ladder degrades to the scarcity rule BY CONSTRUCTION — pinned here
  as the equal-prices byte-identity property test (a GB-split
  scenario prices both zones identically, so the delta is exactly
  nil, not approximately nil). The LP remains the B6 resolver; the
  ladder bounds nothing about the +38–49 %.
- **For the sweep package (D11 rule 2, tier-2 frozen-imports fix)**:
  the ladder exists and is selectable as designed, but on 2024 price
  data its cross-zone directions in both-gas periods are convention
  noise (§3). Any tier-2 sweep quoting priced-ladder flows must carry
  this note's caveat; the Package B bracket remains the disclosed
  uncertainty band. Whether the sweep's central estimate should run
  the ladder or the scarcity rule is a supervisor/reviewer call this
  note deliberately does not make.
- **Year-specificity**: the wedge is a year-specific quantity and
  2024 is its null (data report §2). In a year with a real, signed
  GB-vs-EU carbon gap (2022–23, or post-linkage), the ladder's
  primary key carries signal and the A2a question is worth
  re-measuring. Nothing here says the priced ladder is wrong as a
  mechanism; it says 2024 cannot reward it.

## 6. Reproducibility

Engine numbers: `cargo test --release -p grid-adequacy --test
acceptance_d11_priced_ladder -- --nocapture` (pins: A2a priced
12,595; flat-flat 16,370; plus the gate-quantity report). Phase-0
shares: the scarcity-rule 5-zone run joined per period against
`fr_generation_2024.csv`, `flows_gb_entsoe_2024.csv`,
`generation_by_fuel_2024.csv` and the prices-2024.toml UKA step
series, classified as in §1 (dead-band and confusion arithmetic
identical to the Stage 5 acceptance test; the baseline confusion
reproduces the committed 15,823/1,297/1,122 record exactly, which is
the method's self-check).
