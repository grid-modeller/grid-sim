# fetch-2024 — 2024 validation-pack assembly scripts

**Provisional.** These Python scripts assemble the 2024 validation pack
defined in `docs/05-validation.md`. They will be ported to
`grid-cli fetch-data` in Stage 0; nothing here is a long-term interface.

Run order (Python ≥ 3.11, needs `requests pandas pyarrow`):

```
python fetch.py    <repo-root>   # network: downloads raw data
python build.py    <repo-root>   # raw -> processed UTC half-hourly traces
python validate.py <repo-root>   # exit non-zero on any integrity failure
python analyze.py  <repo-root>   # D2 discrepancy numbers -> analysis_2024.json
```

Outputs land in `data/packs/2024/` (git-ignored; only the checksum manifest
`data/packs/2024.sha256` is committed). All scripts are deterministic: no
randomness, no wall-clock dependence; `fetch.py` is the only script that
touches the network.

## Sources (all accessed 2026-07-02)

| Data | URL | Licence |
|---|---|---|
| GB demand 2024 (half-hourly): ND, TSD, embedded wind/solar estimates + capacities, pump-storage pumping, interconnector flows | https://api.neso.energy/dataset/8f2fe0af-871c-488d-8bad-960426f24601/resource/f6d02c0f-957b-48cb-82ee-09003f2ba759/download/demanddata_2024.csv (dataset: NESO Data Portal "Historic Demand Data") | NESO Open Data Licence (use/adapt/redistribute with attribution) |
| Generation by fuel type 2024 (half-hourly), incl. INT* interconnector net flows and PS pumped storage (net) | https://data.elexon.co.uk/bmrs/api/v1/datasets/FUELHH/stream?settlementDateFrom=…&settlementDateTo=… (Elexon Insights API, dataset FUELHH; no API key; fetched in monthly chunks) | Elexon BMRS open-data licence: copy/publish/distribute incl. commercially, with attribution (https://www.elexon.co.uk/bsc/data/balancing-mechanism-reporting-agent/copyright-licence-bmrs-data/) |

Fleet capacities for `scenarios/gb-2024-reference.toml` are cited inline in
that file (World Nuclear Association UK profile / DUKES series, RenewableUK
UKWED, Modo Energy buildout report, NESO embedded capacity columns, station
data, and observed FUELHH maxima as cross-checks).

**D1 note:** D1 is resolved as **direct ERA5 derivation**; that pipeline is
not built yet and no ERA5 data is fetched by these scripts. The only CF
trace here is the provisional observed one below.

## Built columns and derived traces

- **`demand_2024.*` column `underlying_demand`** = `nd` +
  `embedded_wind_generation` + `embedded_solar_generation` (MW). This is
  the modelled-demand series under the D3 total-generation convention
  (`docs/notes/d3-embedded-convention.md`): ND grossed up by the NESO
  half-hourly embedded estimates. 2024 total: 261.83 TWh.
- **`wind_cf_2024.*`** (single column `wind_cf`): PROVISIONAL observed
  fleet-wide wind capacity factor = (FUELHH transmission wind + NESO
  embedded-wind estimate) / 29,100 MW, where 29.1 GW is the CONSTANT
  end-2024 GB wind capacity (14.7 GW offshore + 14.4 GW onshore, UKWED).
  Caveats: (a) this is an outturn CF for Stage 0 trace-loading tests and
  rough work, NOT the ERA5-derived weather trace D1 calls for; (b) the
  constant denominator biases early-2024 values low (capacity grew during
  the year); (c) it includes curtailment and outages; (d) it does not
  split onshore/offshore. 2024: mean 0.323, range 0.016–0.775, no
  clamping required.

## Data semantics discovered during assembly (do not lose these)

- **Elexon FUELHH `ps` is NET pumped storage**: negative values are pumping
  (9,543 negative periods in 2024). Gross 2024 volumes: 1.88 TWh
  generation, 2.48 TWh pumping (implied round trip 0.758).
- **FUELHH has no solar category.** GB solar is effectively all
  distribution-connected; the only half-hourly solar series is NESO's
  `EMBEDDED_SOLAR_GENERATION` estimate.
- **INT\* columns are net flows, + = import to GB.** They reconcile with the
  NESO `*_FLOW` columns to corr ≥ 0.9996 and identical annual TWh
  (mapping: intfr=IFA, intifa2=IFA2, intelec=ElecLink, intned=BritNed,
  intnem=Nemo, intnsl=NSL, intvkl=Viking, intirl=Moyle, intew=EWIC,
  intgrnl=Greenlink).
- **NESO `ND` excludes embedded generation, pumping demand and exports.**
  Closing identity: FUELHH total (PS net) + net imports − ND ≈ station
  transformer load (~0.67 GW mean, 2024).
- **UTC conversion of NESO settlement periods**: settlement dates are
  Europe/London clock days (46 periods on 2024-03-31, 50 on 2024-10-27);
  utc_start = local midnight → UTC + (period−1)×30 min. Elexon rows carry
  an explicit UTC `startTime`, used directly.
- **INTGRNL (Greenlink)** only reports from its late-2024 go-live; earlier
  periods are zero-filled at build time (real zero flow, not a gap).
- Elexon revises: dedupe on (utc_start, fuelType) keeping latest
  `publishTime`.

Full discrepancy analysis: `docs/notes/2024-validation-pack-report.md`.
