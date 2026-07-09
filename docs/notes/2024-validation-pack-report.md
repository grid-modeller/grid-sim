# 2024 Validation Pack — Assembly Report and D2 Evidence

Assembled 2026-07-02 per `docs/05-validation.md`. This note reports the
quantified, irreducible data discrepancies that bound Stage 1/2 validation
tolerances (decision D2, `docs/08-risks-and-decisions.md`). It supplies
evidence only; tolerance numbers are set by the supervisor.

Pack: `data/packs/2024/` (git-ignored; checksums in `data/packs/2024.sha256`).
Scripts: `scripts/fetch-2024/` (fetch → build → validate → analyze; the
README there records URLs, access date 2026-07-02, licences and data
semantics). Machine-readable numbers:
`data/packs/2024/processed/analysis_2024.json`.

## 1. Pack integrity

| Check | Result |
|---|---|
| Periods per trace (leap year) | 17,568 exactly, both traces |
| Gaps / duplicates | none; strictly uniform 30-min UTC index |
| Clock changes | UTC days 2024-03-31 / 2024-10-27 have 48 periods; raw NESO settlement days have 46 / 50 as required |
| NaNs | none after documented INTGRNL zero-fill (pre-go-live = real zero flow) |
| NESO vs Elexon interconnector flows | corr ≥ 0.9996 per link and identical annual TWh, excluding Greenlink (r = 0.396 on a degenerate near-zero commissioning series; annual −0.000 vs −0.001 TWh) — the two sources reconcile |

Sources: NESO Data Portal "Historic Demand Data 2024" (NESO Open Data
Licence — reuse/redistribution with attribution); Elexon Insights API
dataset FUELHH (Elexon BMRS open-data licence — copy/publish/distribute
incl. commercially, with attribution). Both are compatible with
fetch-and-build *and* with a hosted pre-built pack; neither is the D1
weather source, which remains open (no ERA5/renewables.ninja data fetched).

## 2. Headline 2024 actuals

### Annual generation, transmission-metered (Elexon FUELHH, TWh)

| Fuel | TWh |
|---|---|
| Gas CCGT | 72.62 |
| Gas OCGT | 0.17 |
| **Gas total** | **72.79** |
| Wind (transmission) | 65.64 |
| Nuclear | 38.33 |
| Biomass | 18.80 |
| Hydro (NPSHYD) | 3.58 |
| Other | 3.35 |
| Coal (to closure 2024-09-30) | 1.57 |
| Pumped storage (net) | −0.60 |
| Oil | 0.00 |
| Solar (transmission) | 0.00 — no FUELHH category; all solar is embedded |

### Demand and imports (TWh)

| Quantity | TWh |
|---|---|
| ND (National Demand) | 230.90 |
| TSD (Transmission System Demand) | 248.00 |
| Net imports, total | 33.30 |
| IFA (FR) | +10.11 |
| NSL (NO) | +9.62 |
| ElecLink (FR) | +5.19 |
| Nemo (BE) | +4.16 |
| IFA2 (FR) | +4.15 |
| Viking (DK) | +3.66 |
| BritNed (NL) | +1.58 |
| EWIC (IE/SEM) | −2.69 |
| Moyle (NI/SEM) | −2.50 |
| Greenlink (IE/SEM) | −0.00 (commissioning; commercial ops Jan 2025) |

### Embedded-generation wedge (NESO estimates)

| Quantity | Value |
|---|---|
| Embedded wind | 16.97 TWh (capacity 6.6 GW end-2024) |
| Embedded solar | 13.95 TWh (capacity 18.7 GW end-2024) |
| **Total embedded** | **30.92 TWh** — 13.4% of ND-equivalent supply |
| Half-hourly embedded share of total supply | mean 11.3%, p95 28.6%, max 65.6% |
| Total wind (tx + embedded) | 82.61 TWh |

This is the wedge a transmission-level model cannot see directly: ND is
*net* of embedded output, so the model must either (a) use ND and exclude
embedded capacity from the fleet, or (b) gross demand up by the NESO
estimates and model total wind/solar. The pack carries the NESO half-hourly
estimates so either convention is reproducible (D3).

### Pumped storage (a no-storage Stage 1 model cannot capture this)

| Quantity | Value |
|---|---|
| Gross generation (FUELHH `ps` > 0) | 1.88 TWh |
| Gross pumping (FUELHH `ps` < 0) | 2.48 TWh |
| Implied round-trip efficiency | 0.758 |
| Net (round-trip loss) | 0.60 TWh |
| NESO `PUMP_STORAGE_PUMPING` | 1.72 TWh (lower than Elexon — different metering scope; both retained in the pack) |

Note: FUELHH `ps` is **net** (negative = pumping) — 9,543 negative periods.

## 3. Cross-check residual (NESO demand vs Elexon supply)

Identity: FUELHH total (PS net) + net imports − ND ≈ station transformer
load.

| Statistic | MW |
|---|---|
| mean | +667 |
| median | +670 |
| std | 751 |
| p01 / p99 | +306 / +1,195 |
| annual | +5.86 TWh (≈ 2.5% of ND) |

The identity closes to a stable ~0.67 GW offset (station load + metering
scope), **except 20 periods** where Elexon generation data collapses
(residual to −30.7 GW): 2024-01-10, 01-19, 01-23, 02-01, 03-12, 06-04,
07-11, 08-29, 11-20, 12-05 (timestamps in `analysis_2024.json`). These are
FUELHH publication glitches, ~0.01% of periods, ≤ 0.02 TWh total — left
as-is in the pack and flagged, not repaired.

## 4. Monthly generation matrix and correlation ceiling

`data/packs/2024/processed/monthly_generation_2024.csv` holds the 12×fuel
GWh matrix (plus wind-incl-embedded and embedded-solar columns).

How much does the embedded-generation convention *alone* move the "monthly
generation mix correlation" metric? Comparing the 12×fuel matrix under the
two conventions (tx-only vs embedded folded in):

| Metric | r |
|---|---|
| Flattened 12×fuel matrices, tx vs embedded convention | **0.973** |
| Monthly wind, tx vs tx+embedded | 0.998 |

So ≥ 0.95 is achievable under either convention; ≥ 0.99 is only achievable
if the model and the validation target use the *same* embedded convention.
The metric definition (which fuels, shares vs absolute, convention) must be
pinned alongside the number.

## 5. Gas-marginal proxy (Stage 2)

Proxy, stated explicitly: a period counts as "gas plausibly marginal" when
CCGT output is strictly between x% and (100−x)% of its observed 2024
maximum (27,339 MW) — the CCGT fleet is flexing, neither floored nor
ceilinged.

| Band | % of 2024 periods |
|---|---|
| 3–97% | 99.8 |
| **5–95%** | **99.4** |
| 10–90% | 89.4 |

The "~97%" figure in `docs/04-implementation-plan.md` sits inside the
spread of reasonable proxy definitions: the statistic moves ~10 points
between the 5–95 and 10–90 bands. A ±3-point tolerance is only meaningful
once the marginal-fuel definition is pinned (the model will have an actual
price-setting definition, which is sharper than this outturn proxy).

## 6. Data-quality issues (complete list)

1. 20 FUELHH glitch periods (§3) — flagged, not repaired.
2. FUELHH `ps` is net; sign convention documented (README).
3. No FUELHH solar — the only half-hourly solar series is the NESO
   *estimate*; solar validation inherits NESO estimation error, which is
   unquantifiable from this pack alone.
4. NESO vs Elexon pumping volumes differ (1.72 vs 2.48 TWh) — metering
   scope; do not mix conventions within one identity.
5. Coal ran Jan–Sep only (Ratcliffe closure 2024-09-30); Greenlink
   commissioned during Q4 2024 (negligible flow). Schema v1 cannot express
   mid-year capacity change — noted in the reference scenario.
6. Elexon revises publications; pack keeps latest `publishTime` per period.
7. Small persistent negative values in NESO flow columns are exports
   (convention, not errors).

## 7. Implications for tolerances (evidence only — numbers are D2's call)

- **Annual gas burn.** Actual: 72.79 TWh. Irreducible data-side
  uncertainty on the *actual* is small (< 0.1 TWh glitches). But three
  systematic wedges land preferentially on modelled gas, because gas is
  the balancer: station load ≈ 5.9 TWh/yr (8.1% of gas burn) if ND is used
  without grossing up; PS round-trip loss 0.60 TWh (0.8%) in a no-storage
  model; coal 1.57 TWh (2.2%) if the closure isn't windowed. These are
  *correctable* modelling choices, not noise — the tolerance should assume
  they are handled, and cover what remains: station-level
  outage/availability structure (not quantifiable from FUELHH — it would
  need per-BMU data; carried unquantified) and embedded-estimate error of
  order 1–2 TWh (asserted, not computed — §6.3). The suggested ±5% of
  05-validation.md (±3.6 TWh) is consistent with that remainder; ±2% is
  not evidently achievable.
- **Net annual imports.** Stage 1 takes imports as an exogenous trace, so
  Stage 1 import error is ~0 by construction; the ±15% frame is a Stage 5
  question against 33.30 TWh net (per-link table in §2 for direction/sign
  tests).
- **Monthly mix correlation.** Embedded convention alone caps the flattened
  metric at ~0.973 if conventions are mixed; ≥ 0.95 has headroom, ≥ 0.99
  requires convention pinning (§4). Define the metric before the number.
- **% gas marginal.** Outturn proxy: 99.4% (5–95 band), 89.4% (10–90).
  Proxy-definition sensitivity (~10 points) dwarfs the suggested ±3-point
  tolerance; pin the definition first (§5).
- **Zero unserved energy**: GB recorded no demand-control events in 2024
  (public record; the pack contains no unserved-energy series to check
  directly) — the Stage 1 zero-unserved test is well-posed.
- **D3 (embedded convention)** is now the binding open decision for
  Stage 1 validation; the pack supports both conventions.
