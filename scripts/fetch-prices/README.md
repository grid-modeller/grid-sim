# fetch-prices — 2024 price-series pack extension (Stage 2 pricing layer)

**Provisional.** Python scripts extending the 2024 validation pack
(`scripts/fetch-2024/`) with the price series Stage 2's SRMC model and
acceptance tests need. Same conventions as the base pack: outputs are
git-ignored, only checksums are committed (`data/packs/2024.sha256`);
UTC half-hourly index, 17,568 periods; CSV + Parquet. To be ported to
`grid-cli fetch-data`.

Run order (Python ≥ 3.11, needs `requests pandas pyarrow openpyxl`;
built 2026-07-02 with pandas 3.0.3 / pyarrow 24.0.0 / requests 2.34.2 /
openpyxl 3.1.5 — Parquet bytes may differ under other pyarrow versions,
see the env-pinning open item in `memory/project-state.md`):

```
python fetch.py    <repo-root>   # network: downloads raw data
python build.py    <repo-root>   # raw -> processed UTC half-hourly traces
python validate.py <repo-root>   # exit non-zero on any integrity failure
python analyze.py  <repo-root>   # Stage 2 tolerance evidence -> price_analysis_2024.json
```

## Sources (all accessed 2026-07-02)

| Data | URL | Licence |
|---|---|---|
| Market Index Data 2024 (half-hourly GB market price/volume per provider) | https://data.elexon.co.uk/bmrs/api/v1/datasets/MID/stream?from=…&to=… (Elexon Insights API, dataset MID; no API key; monthly UTC chunks) | Elexon BMRS open-data licence: copy/publish/distribute incl. commercially, with attribution (https://www.elexon.co.uk/bsc/data/balancing-mechanism-reporting-agent/copyright-licence-bmrs-data/) |
| Imbalance (system) prices 2024 | https://data.elexon.co.uk/bmrs/api/v1/balancing/settlement/system-prices/{settlementDate} (DISEBSP; one request per settlement date — no range endpoint) | Same Elexon licence |

The Elexon licence is cited by name in case the URL stops responding (it
403s to non-browser agents): **Elexon "Copyright and licence for the
supply and use of BMRS Data"** (the BMRS open-data licence published on
elexon.co.uk), which grants a non-exclusive, royalty-free licence to
copy, publish and distribute BMRS data, including commercially, with
attribution to Elexon.
| Daily System Average Price (SAP) of gas | https://www.ons.gov.uk/economy/economicoutputandproductivity/output/datasets/systemaveragepricesapofgas — 9 Jan 2025 edition xlsx (data source: National Gas Transmission) | Open Government Licence v3.0 |
| UKA auction clearing prices 2024 (all 25 auctions) | https://www.gov.uk/government/publications/functioning-of-the-uk-carbon-market-2024 (UK ETS Authority report PDF, Table 1; fetched for provenance, transcribed to `data/reference/prices-2024.toml`) | Open Government Licence v3.0 |

Sources that were **considered and rejected** on licence grounds (do not
substitute silently — recorded per docs/05):
- NBP day-ahead price assessments (ICIS Heren, LSEG/Refinitiv, incl. as
  charted on the Ofgem data portal): proprietary, not redistributable.
  ONS SAP (open, daily) is used as the GB gas price instead; it is a
  within-day OCM price, not a day-ahead assessment (documented in the
  reference file and the report note).
- ICE UKA secondary-market/futures settlement prices: ICE data terms are
  restrictive. Fortnightly auction clearing prices from the OGL report
  are used instead, step-interpolated.

## Built traces (data/packs/2024/processed/)

- `market_index_2024.{csv,parquet}` — `apx_price`, `apx_volume`
  (APXMIDP = EPEX Spot GB), `n2ex_price`, `n2ex_volume` (N2EXMIDP =
  Nord Pool), `mid_price` (reference price), `filled` (bool).
- `imbalance_prices_2024.{csv,parquet}` — `system_price` (single
  imbalance price post-P305; sell == buy verified for all 17,568
  periods), `niv` (net imbalance volume, MWh).
- `gas_sap_daily_2024.{csv,parquet}` — `sap_gbp_per_mwh_hhv`: daily SAP
  upsampled to half-hourly (each period carries its UTC day's value).
- `price_analysis_2024.json` — Stage 2 tolerance evidence (see
  `docs/notes/2024-price-pack-report.md`).

## Data semantics discovered during assembly (do not lose these)

- **N2EXMIDP is defunct in practice**: zero volume in 17,489 of its
  17,524 published 2024 periods after boundary dedupe (non-zero in only
  35, max 328.7 MWh vs APX mean 2,100 MWh). The **reference `mid_price` is the volume-weighted
  mean over providers with volume > 0** — in practice the APX price;
  N2EX contributes only in those 35 periods. Do not use `n2ex_price`
  (0.00 when no trades) as a price series.
- **One genuine MID gap in 2024**: APXMIDP has no record for
  2024-04-13 07:00 UTC (SP16). Convention: price = mean of the two
  adjacent APX prices (4.275 = (1.71 + 6.84)/2), volume = 0,
  `filled = True`. Everything else is gap-free.
- MID monthly chunks overlap at boundaries with byte-identical rows;
  build dedupes on (utc_start, dataProvider) after verifying identity.
- MID records carry no `publishTime` (unlike FUELHH) — no revision
  dedupe is needed or possible.
- **System prices are single-price** (P305): systemSellPrice ==
  systemBuyPrice in every 2024 period; stored once as `system_price`.
  corr(mid_price, system_price) = 0.78 — related but distinct series;
  MID is the day-ahead-ish reference, the imbalance price is secondary.
- **ONS SAP is a gas-day (05:00–05:00 local) price mapped to the UTC
  calendar day** — a ≤ 5-hour timing approximation, immaterial at the
  daily granularity Stage 2 needs. SAP is on a gross-CV (HHV) basis;
  p/kWh × 10 = £/MWh.
- Negative electricity prices are real: 495 MID periods < £0 in 2024
  (min −£61.09/MWh); imbalance price min −£91.82/MWh. Do not clamp.

Committed reference numbers (gas monthly, UKA auctions, CPS,
efficiencies, emission factors, all with per-number citations):
`data/reference/prices-2024.toml`.

Full evidence write-up: `docs/notes/2024-price-pack-report.md`.
