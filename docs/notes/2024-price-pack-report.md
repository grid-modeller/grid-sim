# 2024 Price Pack — Assembly Report and Stage 2 Tolerance Evidence

Assembled 2026-07-02 (data engineer) per the Stage 2 work package and
`docs/05-validation.md`. This note reports what was fetched, the licence
position, and the quantified evidence for the two Stage 2 `TBD-DATA`
tolerances in `docs/04-implementation-plan.md` (% periods gas-marginal;
wind capture/baseload ratio). It supplies evidence only; the tolerance
numbers and the pinned metric definitions are the supervisor's call.

Pack extension: `data/packs/2024/processed/{market_index_2024,
imbalance_prices_2024, gas_sap_daily_2024}.{csv,parquet}` +
`price_analysis_2024.json` (git-ignored; checksums appended to
`data/packs/2024.sha256`, full manifest re-verified OK, rebuild
byte-identical). Committed reference numbers:
`data/reference/prices-2024.toml` (gas monthly SAP, 25 UKA auctions, CPS,
efficiencies, emission factors — every number cited inline).
Scripts: `scripts/fetch-prices/` (fetch → build → validate → analyze;
README records URLs, access date 2026-07-02, licences, semantics).

## 1. Sources and licence position

| Series | Source | Licence | Status |
|---|---|---|---|
| Half-hourly market price (MID) | Elexon Insights API, dataset MID | Elexon BMRS open licence (redistribution incl. commercial, attribution) | Fetched |
| Half-hourly imbalance price | Elexon Insights API, settlement system prices (DISEBSP) | Same | Fetched |
| Daily GB gas price (SAP) | ONS "System Average Price (SAP) of gas" (data: National Gas Transmission) | OGL v3.0 | Fetched |
| UKA carbon price (25 auctions) | UK ETS Authority, "Functioning of the UK carbon market for 2024" (Oct 2025), Table 1 | OGL v3.0 | Transcribed to reference file; PDF kept in raw/ |
| CPS £18/tCO2, CCGT η 48.93 % (DUKES 5.10.C 2024), OCGT η 34.9 % HHV (DESNZ/GHD 2025), EF 0.18253 tCO2/MWh_th HHV (GHG CF 2024) | gov.uk (citations in reference file) | OGL v3.0 | Committed reference file |

**Rejected on licence grounds** (documented, not silently substituted):
NBP *day-ahead* assessments (ICIS/LSEG, incl. the Ofgem data portal
charts) and ICE UKA secondary-market settlements are proprietary. The
open substitutes are ONS SAP (a within-day OCM price, not day-ahead —
tracks NBP closely but is not the same assessment) and fortnightly
auction clearing prices (step-interpolated). Consequence quantified in
§4: daily-gas-price error is not the binding uncertainty; the SRMC
recipe lands the median price/SRMC ratio at 0.955.

## 2. Pack integrity

| Check | Result |
|---|---|
| Periods per trace | 17,568 exactly, all three traces |
| Gaps / duplicates | none after documented handling (below); uniform 30-min UTC index; clock-change UTC days 48 periods |
| NaNs | none |
| MID gap | **one genuine gap**: APXMIDP missing 2024-04-13 07:00 UTC (SP16). Convention: price = mean of adjacent periods (£4.275/MWh), volume 0, `filled=True` flag. 1/17,568 ≈ 0.006 % |
| N2EXMIDP (Nord Pool) | defunct in practice: zero volume in 17,489 of 17,524 published periods (after boundary dedupe); 44 missing rows filled (0,0). **Reference `mid_price` = volume-weighted across providers with volume > 0** — effectively the APX (EPEX) price; N2EX contributes in 35 periods only |
| Imbalance prices | complete; single-price (sell == buy) verified in all periods |
| Negative prices | real, kept: 495 MID periods < £0 (min −£61.09); imbalance min −£91.82 |
| Gas SAP | 366 daily values, no gaps; gas-day (05:00 local) mapped to UTC calendar day — ≤ 5 h approximation, immaterial at daily granularity |

## 3. Headline 2024 numbers

| Quantity | Value |
|---|---|
| MID price, time-weighted annual mean ("baseload price") | **£71.38/MWh** |
| MID price, volume-weighted annual mean | £73.55/MWh |
| MID price range | −£61.09 to +£605.17/MWh |
| Imbalance price annual mean | £71.17/MWh (corr with MID 0.78) |
| Gas SAP annual mean | £28.67/MWh_th (HHV); monthly means £21.49 (Feb) → £38.28 (Dec) |
| UKA auction clearing prices | £32.10–£46.92; mean £37.18 (matches independently reported 2024 average) |
| **Computed CCGT SRMC** (η 0.4893 HHV, EF 0.18253, UKA step + CPS £18, no VOM) | **mean £79.16/MWh, range £57.29–£104.91** (monthly means £62.77 Feb → £97.96 Dec) |
| Computed OCGT SRMC (η 0.349 HHV) | mean £110.98/MWh, range £80.32–£147.09 |
| Wind (tx + embedded, D3) capture/baseload ratio | **0.899** (monthly 0.824–0.964) |
| Wind (tx-only) capture/baseload ratio | 0.905 |
| Solar (embedded) capture/baseload ratio | 0.896 (monthly 0.853–1.058; >1 in winter months) |

Machine-readable, incl. all monthly values:
`data/packs/2024/processed/price_analysis_2024.json`.

## 4. Gas-marginal: what the price data can and cannot support

The Stage 2 acceptance test reads "% of periods with gas marginal for
2024 within TBD-DATA points of observed (~97 % claim)". Two families of
observed-side proxy now exist, and they measure different things:

**(a) Outturn flexing proxy** (validation-pack report §5): CCGT output
strictly between x % and (100−x) % of its 2024 max — 99.8 / **99.4** /
89.4 % for the 3–97 / 5–95 / 10–90 bands. Reproduced here: 99.38 %
(5–95).

**(b) Price-level consistency** (new): observed MID price within a band
of the computed CCGT SRMC:

| Definition | % of 2024 periods |
|---|---|
| A: \|P − SRMC_CCGT\| ≤ 20 % | **64.1** |
| B: \|P − SRMC_CCGT\| ≤ £15/MWh | 62.6 |
| C: SRMC_CCGT − £10 ≤ P ≤ SRMC_OCGT + £10 | 65.2 |
| D: P within SRMC of any gas unit with η ∈ [0.40, 0.60] | 64.6 |
| A with η = 0.45 / 0.53 | 62.0 / 57.8 |
| A with +£3/MWh VOM | 64.5 |

Level diagnostics: the SRMC recipe is **well-centred** — median P/SRMC
= 0.955 — but the distribution has fat tails (p5 = 0.11, p95 = 1.30;
23.5 % of periods below 0.8, 12.4 % above 1.2). Correlation of price
with SRMC: 0.36 half-hourly, 0.50 daily, **0.85 monthly** — the daily
gas price drives the price *level* across the year, but within-day
shape (renewable output, scarcity, negative-price events) dominates
half-hour to half-hour.

Cross-tab: the two proxy families are nearly independent — among
flexing periods, 64.4 % are within ±20 % of SRMC (vs 64.1 % overall).
The flexing proxy is too broad to discriminate price-setting; the
price-level proxy conflates "gas marginal" with "price equals
fleet-average-efficiency SRMC".

**Implication (evidence, not a decision).** No outturn statistic
observable from this pack supports validating "~97 % gas-marginal
within ±3 points" as a *price-level* claim: the observable spread
across reasonable definitions is 57.8–99.8 %. The sharp quantity is the
*model's own* marginal-technology flag (Stage 2 computes which unit
sets SMP). Two defensible test designs:

1. **Marginal-technology test against the flexing proxy**: compare the
   model's % gas-marginal to the 5–95 flexing proxy (99.4 %), with a
   tolerance covering the proxy-definition spread (±3 points does *not*
   cover it; the 5–95 vs 10–90 gap alone is 10 points).
2. **Price-level test against MID**: pin the SRMC recipe (this note's:
   SAP daily, DUKES fleet η, EF 0.18253, UKA step + CPS), and test
   distributional statistics that the data shows are stable: median
   P/SRMC 0.955 (a ±0.05-ish band is plausible), % within ±20 % ≈ 64 %,
   monthly correlation ≈ 0.85. These are reproducible but they test the
   SRMC recipe, not the "~97 %" claim.

The "~97 %" number, if it is to be published, needs its definition
pinned first (which statistic, which band) — the claim sits inside the
proxy spread but is not identified by it.

## 5. Wind capture ratio: evidence for the second TBD-DATA

Observed 2024 (MID reference price, D3 total-wind convention):
**0.899** annual; tx-only 0.905 (convention moves it 0.006). Monthly
range 0.824–0.964 — month-to-month movement (±0.07 around the annual
value) is far larger than the convention wedge.

Sensitivity of the observed-side number (both computed, reviewer
request):
- **Price-series choice**: against the imbalance price instead of MID,
  the total-wind ratio is **0.904** (vs 0.899) — a +0.005 move; the
  choice of price series is not the binding sensitivity. MID remains
  the reference (day-ahead-adjacent, traded volume ~2.1 GWh/period).
- **Observed vs modelled wind weighting**: weighting MID by the
  *modelled* wind trace (capacity-weighted ERA5 CFs, 14.4 GW onshore +
  14.7 GW offshore — the trace Stage 2 will actually dispatch,
  half-hourly r = 0.967 vs observed) gives **0.875** (vs 0.899
  observed-weighted; monthly 0.782–0.964). The weighting alone moves
  the ratio by **−0.023** — this is the floor on any acceptance
  tolerance for a model driven by the ERA5 trace, before any
  price-model error is added.
- The 2024 negative-price periods (495) and price spikes land
  asymmetrically on windy periods — this is the phenomenon the ratio
  measures, not noise.
- Solar: 0.896 annual, computed for free; note winter monthly values
  > 1 (solar output concentrated in daylight peaks).

A model reproducing 0.899 requires reproducing the *joint* distribution
of wind output and price. Stage 1 evidence (run report §2): wind trace
is calibrated ERA5 (half-hourly r = 0.967 vs observed), so the volume
side is largely given; the error budget is dominated by the modelled
price side (SRMC level + which unit is marginal per period). Given the
fat price tails in §4, a tolerance of the order of a few hundredths on
the ratio (rather than a few thousandths) is what the evidence
supports; monthly ratios (0.824–0.964) give 12 additional pinnable
points if a tighter joint test is wanted.

## 6. Data-quality issues (complete list)

1. One APX MID gap (2024-04-13 07:00Z), filled and flagged (§2).
2. N2EX MIDP defunct — do not use `n2ex_price` as a price series (§2).
3. ONS SAP is within-day OCM, not day-ahead NBP; gas-day/UTC-day offset
   accepted (§1, §2).
4. UKA is fortnightly-stepped, not daily; between auctions the secondary
   market moved within roughly the neighbouring-auction range in 2024
   (£32–47 overall). Carbon-term error from stepping: at η 0.4893 a
   £5/tCO2 step error moves CCGT SRMC by ~£1.9/MWh (~2.4 %).
5. DUKES CCGT η is a fleet annual average; the marginal unit's
   efficiency varies per period (sensitivity band carried: η 0.45–0.53
   moves the mean SRMC ±~£7/MWh).
6. VOM excluded from the SRMC recipe (typical ~£2–5/MWh; +£3 sensitivity
   changes the gas-marginal statistic by +0.35 points only).
7. MID records carry no publishTime — revisions, if any, are invisible
   (unlike FUELHH).

## 7. What Stage 2 should consume

- Reference price: `market_index_2024.mid_price` (documented in the
  scripts README; imbalance price is the secondary series).
- SRMC inputs: `data/reference/prices-2024.toml` + daily SAP trace.
  The reference file states the recipe and the HHV-basis rule (do not
  mix gross/net CV bases).
- Emissions accounting: use `co2e_tonnes_per_mwh_th_hhv` (0.18290);
  carbon *cost* uses the CO2-only factor (0.18253) — ETS/CPS charge CO2.
- Candidate schema_version 2 fields (per project-state next-actions):
  fuel price, efficiency, EF, carbon price per thermal fleet entry.
