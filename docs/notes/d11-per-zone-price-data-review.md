# D11 per-zone price DATA package — reviewer adjudication

**Reviewer, 2026-07-04.** Adversarial review of the uncommitted D11
per-zone pricing DATA package: `data/reference/prices-eu-2024.toml` (DRAFT),
`docs/notes/d11-per-zone-price-data-report.md`, and three git-ignored
provenance PDFs under `data/packs/2024/raw/`. Gates the D11 Edit 7 data
deliverable before any tier-2 engine work.

## Verdict: ACCEPT-WITH-NOTES

The load-bearing finding — **the 2024 GB-vs-EU carbon wedge is ~nil** — is
CORRECT, independently reproduced, and robust to the source chosen. Citations
are not fabricated: I extracted the text of all three PDFs and each cited
figure is present verbatim. Checksums match. Licences are correct. Scope is
clean (two data/docs files only; no engine/schema/memory/docs-04/D11-note
edits; PDFs git-ignored, not redistributed). Four notes below are all minor
and none blocks the engine work order; they should be cleared before the file
loses its DRAFT header or any number is quoted in a paper.

## The carbon wedge — reproduced (the load-bearing inversion)

Confirmed against the committed GB pack (`prices-2024.toml`): UKA 2024 mean
**£37.18** (mean of the 25 auction clearing prices, line-item verified) and
CPS **£18** (frozen). GB effective carbon = **£55.18/tCO2**.

EUA €64.8 → GBP at ONS XUMAERS 1.1815 €/£ (gbp_per_eur = 0.846382):
64.8 × 0.846382 = **£54.85/tCO2**. Wedge = 55.18 − 54.85 = **+£0.33/tCO2**
(GB +0.6%). In CCGT SRMC terms (EF 0.18253, η 0.4893):
0.3345 × 0.18253 / 0.4893 = **£0.12/MWh_e**.

Robustness to the EUA source: the ESMA PDF (archived, checksummed) states
"Spot price (EUR/tCO2) **65**" for 2024. Using €65: 65 × 0.846382 = £55.01;
wedge = **+£0.17/tCO2**. The ~nil conclusion holds under either source. The
"2024 is a coincidental carbon near-parity year" framing (UKA ~£18 below EUA,
CPS £18 closes it; the wedge is year-specific and signed differently in
2022-23) is sound and appropriately caveated. **The inversion of the D11
review §A working assumption is CORRECT.**

## Citation verification (all three PDFs, text-extracted)

- DG ENER Q4-2024 gas report: *"The annual average gas wholesale price was
  35€/MWh, a decrease of 16% ..."* — confirms the €35/MWh TTF level.
- ESMA Carbon Markets Report 2025: *"Prices and volatility 2024 2023 Spot
  price (EUR/tCO2) 65 83 ..."* — confirms €65 2024 spot (€83 in 2023).
- EU Commission 2024 Carbon Market Report (H1): *"The average price in 2023
  was EUR 83.60 ... In the first half of 2024, the price varied between EUR
  49.50 (23 February) and EUR 75.35 (3 June)."* — confirms the H1 range and
  2023 average verbatim.

Checksums: dg-ener 356b8ee1…, esma 763dccfc…, eu-carbon-h1 fc17ff61… — all
match the values recorded inline in the TOML.

## FX and gas basis

- FX 1.1815 €/£ (ONS XUMAERS 2024 annual): consistent with the ~€1.18 2024
  sterling average; OGL v3.0. Accepted (see Note 1 on provenance form).
- Gas basis (GB-SAP fallback): the €35/MWh TTF level is verified from the DG
  ENER PDF; £29.62 vs SAP £28.67 = +3.33%; gas-SRMC gap +£1.95/MWh_e all
  reproduce. Licence and level justifications are sound. The
  "conservative for the mechanism" justification is imprecise — see Note 3.

## Item-4 ruling (A2a conclusion + engine-work-order instruction)

**The reasoning is SOUND.** (a) +£0.12/MWh_e is far below the CCGT SRMC
spread (mean ~£79, p5-p95 ~£9-103) and cannot flip a flow. (b) With the
GB-gas fallback, per-zone gas SRMCs are equal by construction, so in
genuinely both-gas-marginal periods gap≈0 and, by the D11 §A lexicographic
rule, the priced ladder degrades to the scarcity rule — zero A2a movement
there. (c) The only surviving lever is the non-gas merit order: where FR is
nuclear/hydro-marginal (~£0) and GB is gas-marginal (£60-100), the ladder
sees a large, correctly-signed gap and pulls FR→GB. All three steps are
correct and consistent with D11 §A.

**The recommendation is the right instruction to carry into the engine work
order**, with one sharpening the package does not make explicit (Note 4):
the stage-5 "1,122 both-gas-marginal (86.5%)" label is the *model's*
scarcity-ladder tiebreak view, and stage-5 §2.4 itself notes that in these
night/shoulder hours *real FR prices sit below GB's (nuclear-adjacent
economics)*. So the re-characterisation must use FR's **real** marginal
technology (observed ENTSO-E generation-by-fuel), not the model's label; and
the 97.4% bound — which stage-5 derived by assuming the 1,122 class is
"priced away" — is in tension with the mechanism, because the price wedge
cannot touch the genuinely-both-gas subset.

**"If the mismatches are dominated by both-gas-marginal periods, 97.4% is
unreachable on 2024 data and that is the finding (carbon near-parity, not a
defect)" is a LEGITIMATE pre-registered outcome** under D11 rule 4 ("Miss →
the priced ladder is not adequate and the finding is named, not re-pinned").
The package's refinement — that the cause is a year-specific carbon null, not
a modelling deficiency — is a more honest naming than rule 4's default and is
consistent with it.

## Licences

EEA re-use policy, ESMA/EU reusable, Commission Decision 2011/833/EU (DG CLIMA
and DG ENER), ONS OGL v3.0 — all correctly citable/open. The EEX (systematic
redistribution barred by market-data terms) and ICE/EEX TTF futures
(proprietary) rejections correctly replay the NBP/ICE precedent from the
committed pack. Nothing in the TOML or report redistributes a proprietary
series — only report-level *levels* are used, as the committed pack did. The
provenance PDFs are git-ignored. Clean.

## Notes (conditions — all minor, none blocking)

1. **Provenance form of the load-bearing number.** The headline EUA €64.8
   rests on the EEA web indicator, for which there is NO checksummed snapshot
   in `raw/`, while its cross-checks (ESMA, Commission) do have archived PDFs.
   Per docs/05 the most load-bearing input should carry archived provenance.
   Fix: either archive the EEA indicator page (checksummed) like the others,
   OR promote the ESMA €65 (already archived and checksummed) to the primary
   and demote EEA to a cross-check. The conclusion is robust either way
   (ESMA €65 → wedge +£0.17/tCO2). The FX 1.1815 is likewise a single fetched
   value without an archived artifact — lower risk (standard OGL ONS series),
   but note it.

2. **Internal arithmetic inconsistency.** `wedge_srmc_gbp_per_mwh_e = 0.125`,
   but the displayed formula `0.33 * 0.18253 / 0.4893` evaluates to 0.123;
   0.125 requires the unrounded wedge (0.3345). Immaterial to the conclusion,
   but in a file whose discipline is per-number reproducibility, make the
   shown inputs and the shown result agree (state 0.12, or note the unrounded
   input).

3. **"Conservative for the mechanism" is imprecise / arguably inverted.**
   Zeroing the small continental-gas premium removes a *systematic wrong-way*
   (GB→FR) term against GB's dominant import pattern, which marginally
   FLATTERS the ladder's overall A2a direction match — it does not
   conservatively penalise it. The defensible reading is: "we give the ladder
   its most-favourable gas treatment and the gas/carbon lever is STILL empty"
   (conservative for the NULL finding, optimistic for the ladder). Re-word so
   it does not read as if zeroing a wrong-way term is a pessimistic choice.
   Magnitude is tiny; the GB-gas fallback decision itself is sound.

4. **Reconcile with the stage-5 97.4% derivation.** Report §6 does not
   confront that stage-5 §2.4 *derived* the 97.4% bound from "pricing away the
   1,122 both-gas-marginal class" — the very class the package shows the price
   wedge cannot touch where FR is really gas-marginal. State explicitly that
   the "both-gas-marginal" label is the model's scarcity-ladder tiebreak view
   (not the physical FR margin), that the achievable A2a must be re-derived
   from the FR-real-non-gas share, and that the 97.4% expectation is in tension
   with the mechanism until that re-characterisation is done. This keeps the
   instruction the supervisor carries internally coherent.

## Confirmed clean

- TOML parses (Python tomllib); DRAFT header present; schema
  "prices-reference-v1" matches the committed pack; per-number citations
  present; per-zone assignment table correct (all-EUA carbon, all-GB-SAP gas;
  FR/NO2 non-gas-exposed noted; CONT-NW coal/lignite boundary stated; IE-SEM
  EU-ETS with the NI-in-UK-ETS slice caveated; NO2 hydro water-value flagged
  as uncaptured).
- The file correctly defers the per-zone `[pricing]` scenario schema to the
  engine package (no docs/03 or schema edit here) — in scope.
- Scope: only `prices-eu-2024.toml` and the report are untracked; no engine,
  schema, memory, docs/04, or D11-note edits.

## Files
- Reviewed: `data/reference/prices-eu-2024.toml`,
  `docs/notes/d11-per-zone-price-data-report.md`, and
  `data/packs/2024/raw/{eu-carbon-market-report-2024-h1,
  esma-carbon-markets-report-2025,dg-ener-quarterly-gas-q4-2024}.pdf`.
- This review: `docs/notes/d11-per-zone-price-data-review.md`.
