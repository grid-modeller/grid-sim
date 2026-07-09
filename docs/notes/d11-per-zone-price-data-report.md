# D11 Per-Zone Price Data — Sourcing, Licences, and the 2024 Carbon Wedge

Assembled 2026-07-04 (data engineer) as the per-zone pricing DATA package
D11 requires before any tier-2 engine work (`docs/notes/d11-priced-dispatch.md`
rule 1 / "Per-zone price data (docs/05 discipline)" clause; review Edit 7,
§A). Companion draft reference: `data/reference/prices-eu-2024.toml`. Follows
the citation discipline of the committed GB pack (`data/reference/prices-2024.toml`;
assembly report `docs/notes/2024-price-pack-report.md`): every number cited
to a named primary, licence stated, proprietary series rejected not
substituted.

Reviewer verdict on this package: ACCEPT-WITH-NOTES
(`docs/notes/d11-per-zone-price-data-review.md`) — the carbon-wedge
inversion independently reproduced; four minor conditions actioned in this
revision (2026-07-04): (1) ESMA €65 promoted to primary because it is the
archived/checksummed source, EEA €64.8 demoted to cross-check, FX-archive
gap flagged; (2) wedge arithmetic reconciled to the unrounded 0.1652;
(3) the gas-conservatism claim re-worded (it flatters the ladder, not
penalises it); (4) §6 reconciled with the stage-5 97.4% derivation.

## 0. Headline

**The 2024 GB-vs-EU carbon wedge is ~nil: +£0.17/tCO2 (GB marginally
higher).** The UK ETS traded at a deep discount to the EU ETS through 2024
(UKA auction mean **£37.18** vs EUA average **£55.01**), and the GB **£18
Carbon Price Support closes almost exactly that discount** (£37.18 + £18 =
£55.18 ≈ £55.01). In SRMC terms the wedge is **£0.062/MWh_e** — negligible.
(Under the EEA cross-check EUA £54.85 the wedge is +£0.33 → £0.125/MWh_e;
still nil. The conclusion is robust to the source choice.)

This **inverts the D11 review §A working assumption** that per-zone carbon
asymmetry (the "GB CPS premium") is the load-bearing term for the A2a fix.
On the 2024 validation year it is not, because 2024 is a near-parity year by
coincidence of the UKA discount and the CPS. Reported plainly, not hidden.

## 1. Sources and licence position

| Input | Source | Licence | Status |
|---|---|---|---|
| EUA 2024 average (**€65/tCO2 spot — PRIMARY**) | ESMA, "EU Carbon Markets Report 2025" (22 Oct 2025) | ESMA/EU reusable, attribution | Cited; archived PDF in raw/, checksummed |
| EUA cross-check (€64.8 volume-weighted allowance avg) | EEA, "Use of auctioning revenues under the EU ETS" indicator | EEA re-use policy (free re-use, source acknowledged) | Cited; **web indicator, NOT archived** (flagged) |
| EUA H1-2024 auction range (€49.50-75.35); 2023 avg €83.60 | EU Commission DG CLIMA, "2024 Carbon Market Report" (19 Nov 2024, covers to 30 Jun 2024) | Commission Decision 2011/833/EU | Cited; archived PDF in raw/, checksummed |
| TTF 2024 average (€35/MWh) — cross-check only | EU Commission DG ENER, "Quarterly Report on European Gas Markets" Q4 2024 | Commission Decision 2011/833/EU | Cited; archived PDF in raw/, checksummed |
| FX EUR/GBP 2024 average (1.1815 €/£) | ONS, "Average Sterling exchange rate: Euro (XUMAERS)", THAP/MRET | OGL v3.0 | Cited (fetched scalar); **NOT archived** (flagged, low risk) |
| GB UKA (£37.18), CPS (£18), EF, η, GB SAP daily/monthly | prices-2024.toml (already committed) | OGL v3.0 | Reused |

**Provenance note (review condition 1).** The headline number now rests on
ESMA €65, which HAS an archived, checksummed PDF in `raw/` — the docs/05
"checksum every delivered datum" requirement. The EEA €64.8 (which would
give +£0.33) is retained as a cross-check but is a web indicator with no
checksummed snapshot; it is not relied on. The FX scalar (ONS XUMAERS,
1.1815) is likewise read live from the ONS series API and not archived —
lower risk (a single stable OGL scalar, updateDate 2025-02-13) but flagged
here and in the TOML `[fx]` block. If a fully-checksummed FX provenance is
wanted before the DRAFT header drops, fetch + checksum the XUMAERS CSV into
`raw/`; the value would not change.

**Rejected on licence grounds** (the NBP/ICE precedent, replayed):
- **EEX EU ETS primary-auction data as a redistributable series.** EEX's own
  market-data terms: "Any systematic republication or dissemination of
  substantial amount of Data is only permitted with the express permission
  of EEX AG." That is the same reject signal as ICE/NBP. The task briefing
  suggested "EEX primary auction data is public" — it is *viewable*, but its
  redistribution terms fail docs/05, so it is rejected as a shippable
  series, exactly as instructed ("reject proprietary rather than substitute
  silently"). Only report-level averages (ESMA/EEA/Commission) are openly
  licensed.
- **ICE / EEX TTF futures** (continental gas daily/HH series): proprietary.
- Consequence: as with the GB pack (which fell back to ONS SAP because NBP
  day-ahead assessments are proprietary), there is **no licence-clean
  continental daily gas series and no licence-clean daily EUA series**. Both
  carbon and gas are available openly only as report-level *levels*. This is
  sufficient for a per-zone SRMC *level* (the D11 flow rule needs a price,
  not a forecast), with the within-year range as the disclosed uncertainty.

**Provenance PDFs** (fetched, git-ignored per docs/05, checksummed):
`data/packs/2024/raw/{eu-carbon-market-report-2024-h1,
esma-carbon-markets-report-2025, dg-ener-quarterly-gas-q4-2024}.pdf` —
sha256s recorded inline in `prices-eu-2024.toml`. No new *series* was
fetched, so no new pack manifest is required (the GB SAP series, reused for
gas, is already in `data/packs/2024.sha256`).

## 2. The carbon wedge (the load-bearing term) — quantified

| Term | Value |
|---|---|
| UK ETS UKA auction mean 2024 | £37.18/tCO2 (prices-2024.toml, 25 auctions) |
| GB Carbon Price Support 2024 | £18.00/tCO2 (frozen since 2016) |
| **GB effective carbon on gas** | **£55.18/tCO2** |
| EU ETS EUA average 2024 (ESMA €65 spot) | €65/tCO2 = **£55.01/tCO2** (× 0.84638) |
| **Wedge (GB − EU)**, unrounded 0.1652 | **+£0.17/tCO2** (GB +0.3%) |
| Wedge in CCGT SRMC (EF 0.18253, η 0.4893), from unrounded 0.1652 | **+£0.062/MWh_e** |

Arithmetic (review condition 2): GB 55.18 − EU 55.0148 = **0.1652/tCO2**,
shown rounded as +£0.17. SRMC = 0.1652 × 0.18253 / 0.4893 = **£0.0616/MWh_e**,
shown as £0.062. The TOML `[carbon.wedge]` block carries the same unrounded
inputs; shown values and result now agree.

FX convention: ONS XUMAERS 2024 annual = 1.1815 €/£, i.e. €1 = £0.84638.
Basis caveat: GB uses the auction clearing mean; ESMA's €65 is a
secondary-market SPOT average — a small (<€1) spot-vs-auction basis
difference that does not change the ~nil conclusion (the EEA auction-weighted
cross-check €64.8 gives +£0.33, also nil). The within-year EUA range
(€51-75 ≈ £43-64) is wider than this wedge, but symmetric about the mean and
shared by both GB (UKA moves too) — it does not create a systematic
GB-vs-EU premium.

**Why 2024 is a near-parity year.** The CPS was designed (2013) to top the
then-EU-ETS price up to a floor; since Brexit the UKA is a separate,
oversupplied market that in 2024 sat ~£18 below the EUA — so CPS + UKA
landed back on the EUA almost exactly. In other years the wedge is real and
signed: 2022-23 (UKA ≈ EUA, both high) the CPS made GB **more expensive**
(+£18-ish/tCO2); if UKA-EUA linkage proceeds the CPS again opens a GB
premium. **The wedge is a year-specific quantity; 2024 happens to be its
null.** Any A2a claim built on it is a claim about 2024 specifically.

## 3. Gas basis decision

**GB-gas fallback: external zones use the committed GB daily SAP series.**
Three reasons, in order of force:
1. **Licence.** No licence-clean continental daily/HH gas series exists
   (ICE/EEX TTF proprietary). Only DG ENER quarterly/annual TTF *averages*
   are open — too coarse to drive a per-period SRMC.
2. **Level.** 2024 TTF annual €35/MWh ≈ **£29.62/MWh_th** (HHV/GCV) vs GB
   SAP **£28.67** — within **3.3%**, the same gas level to within the SRMC
   recipe's own noise (P/SRMC p5=0.11, p95=1.30; price-pack report §4).
3. **Best-case gas treatment for the ladder (review condition 3, corrected).**
   Real TTF sat *slightly above* SAP in 2024, so a TTF basis would give
   continental gas SRMC a touch **higher** than GB (+£1.95/MWh_e) — a
   systematic **wrong-way (GB→FR) term** against GB's dominant *observed*
   import pattern (FR→GB). A wrong-way term would **degrade** the ladder's
   A2a, so zeroing it via the GB-gas fallback marginally **flatters** the
   ladder, it does not penalise it. This is therefore the ladder's *best
   case* on gas — and the finding stands even under it: the gas/carbon lever
   is **still empty**. So the treatment is **conservative for the null**
   (the wedge is empty even with the most ladder-favourable gas basis) and
   **optimistic for the ladder** (it removes a term that would hurt A2a).
   The carbon term (also ~nil, §2) then carries the whole per-zone
   asymmetry — which in 2024 is ~nil.

Calorific basis: TTF and NBP/GB-SAP both quote per-MWh on a **GCV (gross)**
basis — comparable without adjustment (assumption stated in the TOML).

## 4. Per-zone assignment

Carbon = EUA for all five external zones; gas = GB-SAP fallback for all.
"Wedge bites" = where the zone is plausibly gas-marginal so the (~nil in
2024) fuel/carbon SRMC actually enters the flow rule.

| Zone | Carbon | Gas | Gas-marginal exposure | Where the wedge bites |
|---|---|---|---|---|
| FR | EUA | GB-SAP | **low** | nuclear+hydro dominated; gas a minority of hours. A2a fix (if any) comes from FR's *non-gas* margin pricing below GB gas — not the wedge |
| CONT-NW | EUA | GB-SAP | **high** | BE+NL+DE-LU; most gas-exposed. Boundary: DE coal/lignite-marginal periods priced with the gas-only recipe (no coal EF/η in pack) — stated approximation |
| NO2 | EUA | GB-SAP | **nil** | ~100% hydro; EU ETS via EEA. Price = hydro water-value, which a fuel-SRMC recipe does NOT capture. Consistent with the Module 5 "NO2-flat" result. Wedge never binds |
| DK1 | EUA | GB-SAP | **medium** | wind + gas/biomass; gas-marginal in low-wind hours |
| IE-SEM | EUA | GB-SAP | **high** | all-island SEM, gas-heavy. Caveat: ROI in EU ETS; NI in UK ETS but a small SEM share — treated as EU ETS (dominant), stated approximation |

**Carbon confirmations:** Norway (NO2) participates in the EU ETS via the
EEA agreement — EUA, correct. Ireland (IE-SEM) — ROI is an EU Member State
in the EU ETS; the NI slice of SEM is in the UK ETS but immaterial. All five
zones therefore take the EUA carbon price, with the NI wrinkle noted.

## 5. What could not be sourced openly (and the fallback taken)

| Wanted | Open source? | Fallback |
|---|---|---|
| Daily/HH EUA series | No (EEX/ICE proprietary) | Flat 2024 EUA mean per zone; within-year range as disclosed uncertainty (mirrors the GB UKA fortnightly-step limit) |
| Daily/HH continental (TTF) gas series | No (ICE/EEX proprietary) | GB-SAP fallback (§3) — licence-clean, ~3% off in level, best-case for the ladder |
| Zone-specific coal/lignite SRMC (CONT-NW) | Out of pack scope | Gas-only recipe applied uniformly — stated boundary, not substituted |
| Hydro water-value (NO2, FR reservoirs) | Not a fuel price | Not modelled by the SRMC recipe — stated; NO2 wedge never binds anyway |

## 6. Estimate: can the carbon wedge move A2a toward 97.4%? — and a caution on the 97.4% derivation

**No — not the carbon wedge, and not on 2024 data.** At +£0.062/MWh_e
against a CCGT SRMC mean of ~£79/MWh (p5-p95 ~£9-£103; price-pack §3), the
wedge is far below the noise and cannot flip a flow direction. With the
GB-gas fallback, per-zone gas SRMCs are additionally **equal by
construction**, so wherever both zones are *modelled* gas-marginal the
priced ladder degrades to today's scarcity rule (D11 §A) — zero A2a movement
from price there.

**Caution on the 97.4% derivation (review condition 4).** The stage-5 §2.4
figure — "1,122 of 1,297 A2a mismatches are both-gas-marginal" — is the
**model's own scarcity-tiebreak label**, i.e. which unit the current engine
flags as marginal in each zone. It is **not** the physical FR margin.
Stage-5 §2.4 itself notes that real FR prices sit **below** GB's in those
hours on nuclear-adjacent economics — i.e. FR is very likely on a **non-gas**
(nuclear/hydro-adjacent) margin in many periods the model has labelled
"both-gas-marginal". So the 97.4% expectation and the priced-ladder
mechanism are **in tension until the mismatch set is re-derived from FR's
REAL marginal technology**, not the model's label.

**Instruction the engine work order must carry (coherent form).**
1. Re-characterise the ~1,297 A2a mismatch periods using **observed FR
   marginal technology from ENTSO-E generation-by-fuel** (the entsoe-2024
   pack already holds `fr_generation_2024.*`), NOT the model's
   scarcity-tiebreak label.
2. The ladder-fixable share is the periods where FR is **really non-gas**
   (nuclear/hydro pricing ~£0-low) while GB is gas-marginal (~£60-100): there
   the ladder sees a large, correctly-signed gap and pulls FR→GB. The
   genuinely-both-gas-marginal share is **not** fixable (~nil wedge under
   §2/§3).
3. **Do not budget the carbon or gas wedge to carry A2a** — both are empty
   for 2024 (§2, §3). If the re-derivation shows the mismatch set is
   dominated by FR-really-non-gas / GB-gas periods, 97.4% is plausible via
   the *merit-order* lever the ladder captures directly. If it is dominated
   by genuinely-both-gas-marginal periods, 97.4% is **not reachable on 2024
   price data**, and that is the finding to name (D11 rule 4: "Miss → the
   priced ladder is not adequate and the finding is named, not re-pinned") —
   with the honest cause being 2024's carbon near-parity, not a modelling
   deficiency.

The data package cannot settle which case holds; it establishes that the
**price-asymmetry lever the 97.4% target was premised on is empty for 2024**,
so the target's fate rests on the FR-real-non-gas share — a quantity the
engine measures from the ENTSO-E pack, and which the work order must compute
before quoting any A2a number.

## 7. Files
- Draft reference: `data/reference/prices-eu-2024.toml` (DRAFT header).
- This report: `docs/notes/d11-per-zone-price-data-report.md`.
- Provenance PDFs (git-ignored, checksummed inline):
  `data/packs/2024/raw/{eu-carbon-market-report-2024-h1,
  esma-carbon-markets-report-2025, dg-ener-quarterly-gas-q4-2024}.pdf`.
- Reused (committed): `data/reference/prices-2024.toml`,
  `data/packs/2024/processed/gas_sap_daily_2024.{csv,parquet}`.
