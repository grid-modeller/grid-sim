# ERA5 capacity-factor pipeline — Phase A report (2024 validation year)

Built 2026-07-02 per D1 (direct ERA5 derivation, `docs/notes/
d1-renewables-ninja-licence.md`) and the D3 total-generation convention.
Scripts: `scripts/era5-cf/` (fetch → derive → validate; README there holds
source URL, retrieval date, licence/attribution and fetch statistics).
Machine-readable numbers: `data/packs/2024/processed/era5_cf_report_2024.json`.
Deliverables (git-ignored; checksums appended to `data/packs/2024.sha256`,
cutout manifest `data/packs/era5-2024.sha256`):

    data/packs/2024/processed/gb_onshore_cf_2024.{parquet,csv}
    data/packs/2024/processed/gb_offshore_cf_2024.{parquet,csv}
    data/packs/2024/processed/gb_solar_cf_2024.{parquet,csv}

single float64 column `cf` in [0, 1], 17,568-period half-hourly UTC index —
wired into `scenarios/gb-2024-reference.toml` (placeholders removed;
`grid-cli validate --summary` parses and reports all three traces).

## Source and cutout

Google ARCO-ERA5 mirror
(`gs://gcp-public-data-arco-era5/ar/full_37-1h-0p25deg-chunk-1.zarr-v3`,
anonymous), store attrs at retrieval: `valid_time_start=1940-01-01`,
`valid_time_stop=2025-12-31` (final ERA5; 2024 re-fetches are
reproducible), `valid_time_stop_era5t=2026-06-26`. Cutout: 49–61°N, 8°W–2°E
(2,009 cells), hourly 2024 (8,784 steps), `u100/v100/ssrd/t2m`; persisted
as 12 monthly Parquet files, 199 MB, no NaNs. Attribution carried in the
pipeline README: "Contains modified Copernicus Climate Change Service
information [2024]".

## ssrd convention finding

`ssrd` in the ARCO `ar/` store is **J/m² accumulated over the hour ENDING
at the timestamp label**. Verified empirically (not assumed): on the
clearest June day (2024-06-02, cell 52.5°N 1°W) the irradiance-weighted
centroid of label times sits **+28.6 min** after true solar noon (Spencer
equation of time); hour-ending accumulation predicts +30, hour-starting
−30. Clear-sky peak after /3600: 936 W/m² (June max over the GB box;
probe saw 879 W/m² for one cell on Jun 15) — magnitudes consistent with
clear-sky GHI. Solar values are therefore placed at interval centres
(label − 30 min) before half-hourly interpolation.

## Method (full constants in `derive_cf.py`; prose docstring)

- **Spatial weights — APPROXIMATE, flagged as such.** Public-knowledge
  (UKWED/Crown Estate-level) cluster/region points, 3×3-cell box means,
  weights normalised:
  - *Offshore* (8 clusters, GW weights): Hornsea 2.5, Dogger Bank A 0.8,
    Greater Wash 3.0, East Anglia 1.6 (EA ONE lies ~2.5°E, clamped to the
    cutout's 2°E edge), Thames 1.3, Irish Sea 2.9, Moray Firth 1.9,
    Forth/Tay 1.3 (sums 15.3 vs 14.7 GW UKWED — only relative weights
    matter).
  - *Onshore* (10 regions, 14.4 GW, ~73 % Scotland): Southern Uplands 4.0,
    Central Belt 2.5, Highlands 2.5, Argyll 0.8, NE Scotland 0.8,
    Wales 1.3, NW England 0.9, NE England 0.8, E England 0.5,
    SW England 0.3.
  - *Solar* (7 regions, 18.7 GW, south-heavy): SE England 4.5,
    SW England 4.0, E England 3.5, Midlands 3.0, N England 1.7, Wales 1.5,
    Scotland 0.5.
- **Wind**: |(u100,v100)| → logistic aggregate power curve
  `PMAX·LOSSES/(1+exp(−(v−V0)/S))` with 25–30 m/s cut-out taper; offshore
  V0=9.5, S=1.9; onshore V0=8.5, S=1.7, sheared 100→80 m hub
  ((80/100)^0.14≈0.969); PMAX=0.95, LOSSES=0.90 (both explicit).
- **Solar**: GHI-proportional with temperature derate (PR 0.85,
  γ=−0.0037/K, cell heating 0.03 K/(W/m²), 1 W/m² inverter floor — also
  zeroes ERA5 float32 night noise, observed ≤0.25 J/m²). No tilt model.
- **Half-hourly**: linear interpolation onto the pack index; edges padded
  with nearest value; no NaNs.

## Calibration (the honesty metric)

One multiplicative factor per technology matches 2024 annual energy to the
observed pack. Onshore is pinned by the NESO embedded-wind evidence
(16.97 TWh / 6.6 GW → annual CF 0.2927; embedded wind is effectively all
onshore); offshore takes the remainder of total wind 82.61 TWh
(→ 0.3530); solar 13.95 TWh / 18.7 GW (→ 0.0849). This jointly reproduces
the total AND the transmission/embedded split. No clipping at 1.0 occurred.

| Tech | Raw annual CF | Calibrated (target) | Factor | In 0.7–1.3 band |
|---|---|---|---|---|
| Offshore wind | 0.3933 | 0.3530 | **0.8975** | yes |
| Onshore wind | 0.2816 | 0.2927 | **1.0395** | yes |
| Solar PV | 0.0961 | 0.0849 | **0.8837** | yes |

Factor readings: offshore <1 is expected — the raw curve has no
curtailment (GB curtails offshore-heavy wind), and 2024 capacity grew
through the year (Dogger Bank A, Moray West commissioning) while the
denominator is the constant end-2024 fleet, so the energy-matching CF is
biased low. Solar 0.88 combines the same constant-capacity bias with the
missing tilt model (tilt raises real yield vs horizontal GHI; NESO's
estimate is itself modelled). Onshore 1.04 says the physical level was
already right to 4 %. These calibrated traces are energy-matching CFs for
the FIXED end-2024 fleet — exactly what the constant-capacity reference
scenario needs.

## Validation vs observation

- **Half-hourly total-wind CF vs the pack's observed `wind_cf_2024`**
  (capacity-weighted 14.4/14.7 blend): **r = 0.9666** (raw, uncalibrated:
  0.9699). The observed trace includes curtailment, outages and
  within-year capacity growth that the derived trace deliberately
  excludes; ~0.97 is in the expected band for an ERA5 point-cluster model.
- **Monthly energy correlation**: wind r = 0.9860, solar r = 0.9953.

| Month | Wind derived (TWh) | Wind observed | Solar derived | Solar observed |
|---|---|---|---|---|
| 1 | 9.00 | 9.04 | 0.40 | 0.45 |
| 2 | 8.36 | 8.22 | 0.47 | 0.53 |
| 3 | 7.95 | 7.95 | 1.03 | 1.04 |
| 4 | 8.23 | 8.04 | 1.57 | 1.50 |
| 5 | 4.04 | 4.03 | 2.00 | 1.83 |
| 6 | 5.06 | 5.03 | 2.26 | 2.13 |
| 7 | 4.22 | 4.41 | 1.97 | 1.93 |
| 8 | 7.44 | 6.79 | 1.77 | 1.82 |
| 9 | 5.54 | 5.45 | 1.14 | 1.21 |
| 10 | 7.20 | 7.08 | 0.72 | 0.87 |
| 11 | 5.92 | 6.55 | 0.38 | 0.41 |
| 12 | 9.65 | 10.03 | 0.23 | 0.24 |

Worst wind months: August (+0.65 TWh, derived high) and November
(−0.63 TWh, derived low) — consistent with no-curtailment vs curtailed
reality and outage structure we do not model. Observed monthlies are
"wind_incl_embedded" and "solar_embedded" from
`monthly_generation_2024.csv` (D3 convention on both sides).

## Known limitations (Phase A boundaries, stated)

1. **2024 only.** Phase B runs the same code over 1985–2024; the
   calibration factors derived here are then held fixed (they are fleet
   properties, not weather properties) — to be revisited at Phase B.
2. Spatial weights are approximate points, not a site database; EA ONE
   clamped to the 2°E cutout edge. Adequate for a GB-aggregate model;
   revisit only if zonal splits (ADR-12) ever need weather.
3. No curtailment, no outage structure, no within-year capacity growth in
   the derived traces (all present in the observed trace) — this is the
   main wedge in r and the monthly residuals, and it lands on modelled
   gas via the residual in Stage 1.
4. Solar validation is partly circular on the NESO embedded estimate
   (D3 cost, already documented) and has no tilt model.
5. Onshore/offshore split of the observed total relies on the NESO
   embedded estimate + end-2024 capacities; the true split is not
   independently observable from this pack.
6. The observed `wind_cf_2024` comparator itself uses a constant end-2024
   denominator (biased low early-2024), so r against it slightly
   understates model skill.
