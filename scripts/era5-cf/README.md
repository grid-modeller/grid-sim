# era5-cf — ERA5-derived GB capacity-factor pipeline

**Provisional scripts** (to be ported to `grid-cli fetch-data` in Stage 0)
implementing decision D1 (`docs/08-risks-and-decisions.md`, resolved
2026-07-02: direct ERA5 derivation): fetch an ERA5 cutout for GB and derive
per-technology capacity-factor traces for the validation pack. Phase A
covers the 2024 validation year; Phase B extends the same extraction to
1985–2023. The 1985–2023 record was fetched complete on 2026-07-03 from
the Earthmover icechunk ERA5 store (see "Source switch" below); 2024
remains ARCO-sourced (Phase A, validated, byte-pinned). The 1985–2024 CF
derivation sweep follows.

Run order (Python 3.13.11, packages pinned in `requirements.txt`):

```
python fetch_era5.py   <repo-root>   # network: ARCO-ERA5 -> data/packs/era5/2024/
python derive_cf.py    <repo-root>   # cutout -> three CF traces + report JSON
python validate_cf.py  <repo-root>   # exit non-zero on any integrity failure
```

Phase B additions (multi-year, for the Stage 3 storage runs — see
"Phase B derivation" below):

```
python derive_cf.py          <repo-root> --cf-years 2019-2024  # per-year CF traces
python tile_demand.py        <repo-root>   # tiled demand, all years 1985-2024
python validate_multiyear.py <repo-root>   # exit non-zero on any integrity failure
```

**Environment pinning (added Phase B):** checksum reproducibility is
conditional on the exact versions in `requirements.txt` — the monthly
Parquet files are zstd-compressed by pyarrow and the bytes (hence the
committed `.sha256` manifests) vary across pyarrow/zstd versions even when
the data are identical. Rebuild the venv from `requirements.txt` before any
re-fetch expected to reproduce a manifest. The Phase A venv's versions were
not recorded at the time; the pinned environment was verified equivalent on
2026-07-02 by re-fetching `era5_gb_2024-12.parquet` from scratch and
confirming it byte-identical to the Phase A file (manifest entry OK). The
working venv lives at `~/.local/share/grid-sim/era5-venv` (outside the
repo and outside session tmp, so it survives restarts; the Phase B fetch
process depends on it — do not delete while the fetch is running).

All scripts are deterministic (no randomness, no wall-clock dependence);
`fetch_era5.py` is the only one that touches the network, is resumable
(per-month files, atomic writes, finished months skipped) and 2024 is
*final* ERA5, so re-fetches reproduce the cutout and its checksums.

## Source, licence, attribution

| Item | Value |
|---|---|
| Store A (2024 cutout) | `gs://gcp-public-data-arco-era5/ar/full_37-1h-0p25deg-chunk-1.zarr-v3` (Google ARCO-ERA5 mirror, anonymous read) |
| Store A coverage attrs at retrieval | `valid_time_start=1940-01-01`, `valid_time_stop=2025-12-31` (final ERA5), `valid_time_stop_era5t=2026-06-26` |
| Store A retrieval date | 2026-07-02 |
| Store B (1985–2023 cutouts) | `s3://earthmover-icechunk-era5/icechunkV2` (us-east-1, anonymous read), icechunk branch `main`, zarr group `single/temporal` |
| Store B snapshot ID (pinned) | `39TK56WX185WZ1HP9WNG` (main tip, snapshot dated 2026-06-15; pass `--snapshot` to re-read exactly this) |
| Store B retrieval date | 2026-07-03 |
| Store B coverage at retrieval | 1940-01-01 … 2025-12-31 hourly (753,888 steps); all of 1985–2023 final ERA5 (expver=1, QC status=0 verified in the adoption probe) |
| Variables | u100/v100 (100 m wind), ssrd, t2m (native 0.25°, hourly, float32; ARCO long names map to the same short names) |
| Cutout | GB box 49–61°N, 8°W–2°E (49×41 = 2,009 cells), hourly UTC, per calendar year (8,760 h; 8,784 leap) |
| Persisted as | `data/packs/era5/<year>/era5_gb_<year>-MM.parquet` (long format, zstd; git-ignored) — schema identical across both sources |
| Checksums | `data/packs/era5-2024.sha256` (Phase A, ARCO); `data/packs/era5-1985-2023.sha256` (written 2026-07-03 on fetch completion, 468 files, Earthmover) |
| Licence | ERA5: Copernicus licence (CC-BY 4.0 since 2025-07-02); redistribution of derived products permitted with attribution — identical obligations for both mirrors (both redistribute unmodified ERA5 values; the Copernicus attribution below is what our licence diligence keys on, see D1) |

**Attribution (required on the pack and anything published from it):**
"Contains modified Copernicus Climate Change Service information [2024].
Neither the European Commission nor ECMWF is responsible for any use that
may be made of the Copernicus information or data it contains."

## ssrd accumulation convention — verified

`surface_solar_radiation_downwards` in the ARCO `ar/` store is **J/m²
accumulated over the hour ENDING at the timestamp label** (divide by 3600
for the mean W/m² of the preceding hour). Verified from the data by
`derive_cf.py::verify_ssrd_convention()`: on the clearest June day the
irradiance-weighted centroid of label times sits ≈ +30 min after true
solar noon (Spencer equation-of-time), which is only consistent with
hour-ending accumulation; clear-sky June peak ≈ 879 W/m² hourly mean
(matches the feasibility probe). The derivation therefore places solar
values at the interval centre (label − 30 min) before interpolating.

## Method (full prose + every constant: `derive_cf.py` docstring)

- **Spatial weights** — APPROXIMATE, from public UKWED/Crown Estate-level
  knowledge, not a licensed dataset: 8 offshore clusters (Hornsea, Dogger
  Bank A, Greater Wash, East Anglia, Thames, Irish Sea, Moray Firth,
  Forth/Tay), 10 onshore regions (~73% Scotland), 7 solar regions
  (south-of-England-heavy). Each point = 3×3-cell ERA5 box mean; weights
  normalised (only relative sizes matter; absolute scale is calibrated).
- **Wind**: 100 m speed → logistic aggregate power curve (multi-turbine
  smoothed), explicit 0.90 loss multiplier, 25–30 m/s storm-cutout taper;
  onshore sheared to 80 m hub ((80/100)^0.14 ≈ 0.969).
- **Solar**: GHI-proportional PV with temperature derate (PR 0.85,
  γ −0.0037 /K, cell heating 0.03 K per W/m²); no tilt model (absorbed by
  calibration).
- **Half-hourly**: linear interpolation in time onto the pack's 17,568
  period UTC index; edge periods padded with the nearest value; no NaNs.
- **Calibration**: one multiplicative factor per technology so 2024 annual
  energy matches the observed pack (D3 total-generation convention):
  onshore pinned by the NESO embedded-wind estimate (16.97 TWh / 6.6 GW →
  annual CF 0.2927), offshore by the remainder of total wind 82.61 TWh,
  solar by 13.95 TWh / 18.7 GW → 0.0849. Factors and raw CFs are the
  honesty metric — recorded in `era5_cf_report_2024.json` and
  `docs/notes/era5-cf-2024-report.md`; factors outside 0.7–1.3 would mean
  the physical model is off and must be flagged, not absorbed.

## Outputs

`data/packs/2024/processed/gb_{onshore,offshore,solar}_cf_2024.{parquet,csv}`
— single float64 column `cf` in [0, 1], `utc_start` index
(timestamp[us, tz=UTC]), 17,568 periods — wired into
`scenarios/gb-2024-reference.toml`; checksums appended to
`data/packs/2024.sha256`. Plus `era5_cf_report_2024.json` (calibration and
validation numbers; summarised in `docs/notes/era5-cf-2024-report.md`).

## Fetch characteristics (observed 2026-07-02)

The store is chunked (1 hour, whole globe) per variable, so the GB slice
transfers global chunks: 4 vars × 8,784 h = 35,136 chunk reads, done with
16 concurrent dask threads. Observed: 6–18 chunk/s (throughput varied),
2.5–8 min per month, ~80 min wall total; persisted cutout 199 MB
(12 monthly Parquet files, 15–18 MB each). Resumability was exercised for
real: the first run was killed externally after 8 months; re-running
skipped them and completed Sep–Dec.

## Source switch — 1985–2023 re-sourced from Earthmover (2026-07-03)

The Phase B ARCO fetch (launched 2026-07-02, newest-first) was superseded
mid-run: the whole 1985–2023 record was re-fetched from the Earthmover
icechunk ERA5 store and every ARCO-sourced 1985–2023 month file on disk
was overwritten, giving a homogeneous single-source record for those 39
years. 2024 was deliberately left ARCO-sourced (Phase A: validated,
byte-pinned by `era5-2024.sha256`, and the calibration anchor).

**Why.** The ARCO `ar/` store is chunked (1 hour × whole globe), so a GB
cutout transfers 35,040+ global chunks per year — measured ~14–16
months/h (the full 39 years would have taken ~28 h). The Earthmover store
is chunked (8736 h × 12 × 12 cells), so the GB box reads ~80 objects per
364-day chunk — measured 6–16 s per chunk; the full 468-month fetch
completed in ~50 min wall (~7.7 GB, 468 files), dominated by local
Parquet/zstd writing, not transfer.

**How.** `fetch_era5.py --source earthmover --snapshot 39TK56WX185WZ1HP9WNG`
— years ascending, existing files overwritten (no skip: the point was to
replace ARCO-sourced files), each 364-day store chunk read exactly once
with months cut from a carry buffer. Log:
`data/packs/era5/fetch-earthmover-1985-2023.log` (includes the snapshot
ID and a per-file sha256). Repeated writes of the same month were
byte-identical across runs (30 duplicate writes observed during
crash-resume, zero mismatches).

**Cross-source decode-lineage seam.** The two mirrors decode the same
ERA5 GRIB archive but with different lineages, so values differ at the
GRIB 16-bit packing quantum. Measured on three months fetched from both
sources (max |diff| / max |value| per variable-month; saved ARCO copies
vs Earthmover):

| month | u100 | v100 | ssrd | t2m |
|---|---|---|---|---|
| 2023-06 | 3.7e-05 | 3.1e-05 | 9.8e-06 | 3.2e-06 |
| 2022-01 | 2.2e-05 | 1.9e-05 | 2.7e-05 | 2.9e-06 |
| 2019-09 | 2.2e-05 | 2.8e-05 | 1.2e-05 | 3.2e-06 |

(t2m max abs diff is exactly 2⁻¹⁰ K — the packing quantum.) Physically
negligible, but it means **the record has a value seam at the 2023/2024
boundary** (Earthmover ≤ 2023, ARCO 2024) of ~1e-5 scale-relative
(max|diff|/max|value| — the honest metric: *pointwise* relative diffs
reach ~4e-2 but only at wind-speed zero crossings, where the denominator
is near zero and absolute diffs are ≤ 7e-4 m/s) — far below
weather variability and the CF model's fidelity, stated here so nobody
hunts for it later. It also means CF traces for 2019–2023 derived from
the earlier ARCO cutouts (`cf-partial.sha256`) do NOT reproduce against
the new cutouts at the byte level: the full-range derivation sweep below
re-derives them.

**ssrd convention re-verified on the Earthmover data:** the store's own
attr states "accumulations are over 1 hour ending at `valid_time`", and
the Phase A empirical check reproduces on the 2023-06 cutout
(irradiance-weighted label centroid +29 min after true solar noon at the
clearest cell/day; clear-day peak hourly mean 907 W/m²).

- Manifest: `data/packs/era5-1985-2023.sha256` (relative paths from
  `data/packs/`, same convention as `era5-2024.sha256`), written and
  verified 2026-07-03 after the fetch completed — 468 files. Conditional
  on requirements.txt versions AND the pinned snapshot ID.
- Progress check (offline) still works:
  `<era5-venv>/bin/python scripts/era5-cf/fetch_era5.py <repo-root> --years 1985-2023 --status`
- Licence/attribution: unchanged (Copernicus, attribution required; see
  above). Earthmover publishes the store as an open ERA5 mirror (ERA5
  itself is CC-BY 4.0); values are unmodified ERA5, so the Copernicus
  terms govern.

## Phase B derivation — per-year CF traces + tiled demand (2026-07-02)

Prepared for Stage 3 part 2 (the multi-decade storage runs), running ahead
of the fetch: years are derived as their cutouts complete. Two supervisor
design decisions (2026-07-02) govern it:

**Decision 1 — the pinned 2024 calibration factors apply unchanged to all
years** (offshore 0.8975, onshore 1.0395, solar 0.8837). The scenario
fleet is fixed (end-2024 capacities and layouts); the calibration corrects
the power-curve/weighting model for THAT fleet. Per-year outturn
recalibration would in any case be impossible — the historical fleet
differed (and mostly did not exist). Consequence, stated plainly: a
year-Y trace answers **"what would the END-2024 fleet have produced in
year Y's weather?"** — exactly what multi-decade storage studies need —
NOT "what did year Y's actual fleet produce?". Mechanically, `derive_cf.py
--cf-years` re-derives the factors at full precision from the 2024 cutout
on every run (no hidden full-precision constants) and refuses to proceed
if they stop rounding to the pinned 4-dp values (cutout/method drift
guard). The derivation METHOD is byte-for-byte the Phase A method; only
the year is a parameter.

**Decision 2 — demand for non-2024 years tiles the 2024 profile by
calendar date** (`tile_demand.py`): for weather year Y,
`demand(Y, month, day, half-hour) = demand(2024, same month/day/half-hour)`.
Feb 29: non-leap years omit it (17,520 periods); leap years use 2024's
Feb 29 (17,568). Known limitations, stated plainly: **day-of-week
misalignment** (a 2024 Saturday profile may land on a Tuesday) and **no
demand growth** — standard practice in fixed-demand storage studies (the
Royal Society large-scale-storage study did the same); the scenario's
`annual_scale` handles level scaling. 2024's own file carries the REAL
2024 demand, so scenario trace lists are one uniform per-year family.

Layouts (git-ignored data; committed manifests only):

```
data/packs/cf/gb_{onshore,offshore,solar}_cf_<YEAR>.{parquet,csv}
    single float64 column `cf` in [0,1], utc_start timestamp[us, tz=UTC],
    17,520 periods (17,568 leap). 2024 is regenerated into this layout
    and verified VALUE-IDENTICAL to the Phase A traces in
    data/packs/2024/processed/ (validate_multiyear.py check 6) — one
    uniform path family for scenario files.
data/packs/demand-tiled/demand_<YEAR>.{parquet,csv}      1985-2024
    columns [underlying_demand, nd] (int64 MW, D3 convention), same index
    conventions.
```

`validate_multiyear.py` enforces: per-year period counts, strictly uniform
30-min UTC indexes, year boundaries (Jan 1 00:00Z … Dec 31 23:30Z) and the
**cross-year 30-min continuity the engine's multi-file concat loader
requires** (docs/03 migration note item 3), CF in [0,1], no NaNs, CSV/
Parquet agreement, complete technology triples per year, the 2024
value-identity check, and the full tiling identity for every demand year.

Manifests (same relative-path convention as the others; environment
pinning above applies):

- `data/packs/cf-1985-2024.sha256` — complete (40 years x 3
  technologies x 2 formats, 240 entries), written 2026-07-03 after the
  full-range sweep below; supersedes the interim `cf-partial.sha256`
  (removed in the same commit). An incomplete (in-progress) cutout year
  is never derived or checksummed.
- `data/packs/demand-tiled.sha256` — complete (tiling needs no ERA5 data).

Full-range sweep — executed 2026-07-03 once the Earthmover fetch
completed (re-derives everything, incl. 2019–2023 from the new source;
already-derived years re-derive deterministically):

```
V=~/.local/share/grid-sim/era5-venv/bin/python
$V scripts/era5-cf/derive_cf.py . --cf-years 1985-2024   # skips nothing; re-derives all
$V scripts/era5-cf/validate_multiyear.py .
cd data/packs && shasum -a 256 cf/* > cf-1985-2024.sha256 && git rm -q cf-partial.sha256 && git add cf-1985-2024.sha256   # commit both together
```

Sweep record: 55 s wall; pinned-factor guard passed (offshore 0.8975,
onshore 1.0395, solar 0.8837); 2024 outputs byte-identical to Phase A
(all six files); `validate_multiyear.py` exit 0 over the full 40-year
chain (cross-year continuity 1985→2024; demand-tiled 40/40). Wind
extremes across the record (fleet-weighted): worst 2010 (onshore
0.2318 / offshore 0.2999), second-worst 2021 (0.2478 / 0.3136) — the
two known GB wind-drought years, as expected; best 1986
(0.3460 / 0.3929; 1990 is the onshore maximum at 0.3491).

## NW-Europe box — banked for Stage 5 (2026-07-03)

Owner-approved early fetch of the import-counterparty weather (Stage 5
continental zones): box **42–72°N, 11°W–16°E** (121 x 109 = 13,189
cells — Ireland, France complete, Benelux, Germany, Denmark, Norway to
Nordkapp), vars u100/v100/ssrd/t2m, years **1985–2024**, all from the
Earthmover store at the same pinned snapshot `39TK56WX185WZ1HP9WNG` —
including 2024: this box has no Phase A legacy, so the whole record is
single-source (no decode-lineage seam anywhere in it).

- Fetched 2026-07-03 with `fetch_era5.py --years 1985-2024 --source
  earthmover --snapshot 39TK56WX185WZ1HP9WNG --box eu,42,72,-11,16`
  (the `--box NAME,LATMIN,LATMAX,LONMIN,LONMAX` parameter was added for
  this fetch; without it every path/geometry default is the GB box,
  unchanged — all committed GB manifests remain valid).
- Layout: `data/packs/era5-eu/<year>/era5_eu_<year>-MM.parquet` (schema
  identical to the GB cutouts), 480 files, 51 GB, 43 min wall (one
  transient S3 read failure, retried). Log incl. per-file sha256:
  `data/packs/era5-eu/fetch-earthmover-1985-2024.log`.
- Validation: per-month inline at write (hour count, 13,189 cells, no
  NaNs); independent post-fetch pass over all 480 files (row counts vs
  calendar hours, uniform schema); 350,640 total hours (30 x 8,760 +
  10 x 8,784). Cross-source spot check on the GB overlap of 2024 months
  01 and 06 vs the ARCO GB cutout: max relative diff 1e-5–3e-5 per
  variable (t2m max abs diff = 2^-10 K, the GRIB packing quantum) — the
  Earthmover 2024 layer is final ERA5, agreeing with ARCO at decode-
  lineage level.
- Manifest: `data/packs/era5-eu-1985-2024.sha256` (480 entries, same
  relative-path convention; verified equal to the write-time hashes in
  the fetch log). Environment + snapshot pinning above applies.
- **NOT yet consumed by any pipeline** — banked data. Deliberately no
  CF derivation for Europe: Stage 5 design decides technologies and
  spatial weighting first.
- Licence/attribution: unchanged (Copernicus, attribution required).

## EU external-zone CF derivation — Stage 5 (2026-07-03)

`derive_cf_eu.py` + `validate_cf_eu.py` derive per-country wind/solar CF
and population-weighted temperature traces for GB's import
counterparties from the banked EU pack (`data/packs/era5-eu/`, manifest
`era5-eu-1985-2024.sha256`), 1985–2024. **The GB derivation path is
byte-unchanged**: the EU script imports the pinned GB functions
(power curve, PV model, interpolation, calibration) from `derive_cf.py`
— reuse, not reimplementation; all committed GB manifests remain valid.

- Countries/series: fr, be, nl, de (= DE-LU zone), dk1 (Jutland/Funen
  only; Anholt OWF included, Zealand/Bornholm excluded) × {onshore,
  offshore, solar}; ie (all-island SEM, no offshore — Arklow ~25 MW) ×
  {onshore, solar}; t2m for those six + no2 (temperature ONLY — the
  Norwegian zone is hydro-driven from ENTSO-E data; NO2 wind
  deliberately out of scope, D5 note).
- Layout: `data/packs/cf-eu/<country>/<country>_{tech}_cf_<YEAR>.{parquet,csv}`
  (single float64 `cf` column, GB trace format) and
  `<country>_t2m_<YEAR>.{parquet,csv}` (float64 `t2m_c`, Celsius);
  half-hourly UTC, 17,520/17,568 periods. Report:
  `data/packs/cf-eu/eu_cf_report.json`.
- Spatial weights: APPROXIMATE per-country fleet-location weights
  (public-knowledge regional statistics / named offshore clusters),
  documented point-by-point in `derive_cf_eu.py` — the GB honesty level.
- Calibration: one factor per technology per country, anchored to
  ENTSO-E 2024 actual generation (A75) over A68 capacity
  (`aggregation_gen_2024` in the entsoe pack, built by
  `scripts/fetch-entsoe/build_gen_agg.py`). Factors outside the GB
  honesty band [0.7, 1.3] are ANCHOR findings — trace shipped
  uncalibrated (factor 1.0), diagnosis recorded (2024: fr offshore,
  ie onshore, ie solar, nl onshore, nl solar). Pinned factors and the
  drift guard: `derive_cf_eu.PINNED_FACTORS_EU`. The 2024 factors apply
  unchanged to all years (the Phase B fixed-fleet decision): a year-Y
  trace answers "what would the A68-2024 fleet have produced in year
  Y's weather?", and calibrated traces must be paired with the A68-2024
  capacities in scenario work.
- ssrd hour-ending convention re-verified empirically on the EU pack
  (cell 48N 2E, June 2024; centroid +30 min after true solar noon).
- Validator: `validate_cf_eu.py` re-asserts the EU pack geometry
  (480 files, 13,189 cells, full 121×109 lattice, no NaNs, 350,640
  hours — the committed check that `docs/notes/eu-pack-box-review.md`
  note 3 obliges) AND the derived traces (period counts, uniform
  30-min UTC through all clock changes, cross-year continuity,
  ranges, CSV/Parquet agreement, anchor-energy reproduction).
- Manifest: `data/packs/cf-eu-1985-2024.sha256` (same relative-path
  convention; environment pinning above applies — same era5-venv).
- Evidence note: `docs/notes/eu-cf-derivation-report.md` (method,
  per-country weights, calibration table + honesty verdicts, 2024
  reconciliation, 40-year statistics, GB cross-correlations,
  limitations).
- Licence/attribution: ERA5 unchanged (Copernicus attribution). The
  ENTSO-E anchors are clause-3.1 internal use ("Source: ENTSO-E
  Transparency Platform"); they are not redistributed.

Run order (same venv):

```
python derive_cf_eu.py   <repo-root>                 # full 1985-2024 sweep
python validate_cf_eu.py <repo-root>                 # full: pack + traces
cd data/packs && shasum -a 256 cf-eu/*/*.parquet cf-eu/*/*.csv \
    cf-eu/eu_cf_report.json > cf-eu-1985-2024.sha256
```

## GB population-weighted t2m trace — Q5 heating overlay (2026-07-03)

`derive_t2m_gb.py` + `validate_t2m_gb.py` derive the D9 heating-overlay
temperature trace (docs/notes/d9-heating-overlay.md, data requirement 1)
from the committed GB cutouts (`era5-2024.sha256`,
`era5-1985-2023.sha256`): population-weighted 2 m temperature, Celsius,
one file for the whole 1985–2024 window.

- **Pinned code reuse, GB path untouched**: imports `derive_cf.py`
  (`load_point_means`, `weighted_cf`, `half_hourly_index`,
  `to_half_hourly`) exactly as `derive_cf_eu.py` does for the EU t2m
  traces; `derive_cf.py` is byte-unchanged and all committed GB CF
  manifests remain valid.
- Layout: `data/weather/gb_t2m_pop.{parquet,csv}` — single float64
  column `t2m_pop`, `utc_start` index, 701,280 half-hourly UTC periods
  (D9 rule-2 pinned path/column) + `gb_t2m_pop_report.json` (annual
  means, annual-harmonic fit, the ruling-A Kusuda–Achenbach ground-model
  cross-check vs Busby 2015, the district COP premise check).
- Weights: 20 approximate GB city/metro population clusters, documented
  point-by-point in the script (the EU TEMP honesty level).
- Manifest: `data/packs/weather-gb-t2m-pop.sha256` (paths relative to
  `data/packs/`; environment pinning above applies — same era5-venv).
- Evidence note: `docs/notes/q5-heating-data-report.md`.
- Licence/attribution: ERA5 unchanged (Copernicus attribution).

Run order (same venv):

```
python derive_t2m_gb.py   <repo-root>    # full 1985-2024 sweep (writes)
python derive_t2m_gb.py   <repo-root> --years 2024   # spot check, no write
python validate_t2m_gb.py <repo-root>
cd data/packs && shasum -a 256 ../weather/gb_t2m_pop.parquet \
    ../weather/gb_t2m_pop.csv ../weather/gb_t2m_pop_report.json \
    > weather-gb-t2m-pop.sha256
```

## Scotland / rest-of-GB zonal CF traces — B6 two-zone package (2026-07-04)

`derive_cf_gb2zone.py` splits the GB per-technology CF traces into a
Scotland zone (`sco`) and a rest-of-GB zone (`rgb`, England + Wales)
for the intra-GB / B6-boundary study, 1985–2024, from the SAME
committed GB cutouts. **The GB derivation path is byte-unchanged**: the
script imports the pinned GB functions from `derive_cf.py` (the
`derive_cf_eu.py` pattern) and assigns the pinned GB clusters to zones
whole — weights and coordinates untouched; all committed GB manifests
remain valid and the GB-total traces are not rewritten.

- Zone assignment (documented cluster-by-cluster in the script):
  offshore sco = Moray Firth + Forth/Tay (weight share 0.2092); onshore
  sco = the five Scottish regions (0.7361); solar sco = the Scotland
  point (0.0267). Robin Rigg (~0.17 GW, Scottish waters) sits inside
  the `irish_sea` cluster → assigned rgb, a stated approximation.
- Calibration: the pinned 2024 GB factors apply unchanged to both zones
  (the GB anchors are national; no zonal anchor of the same quality
  exists). Verified per year per technology against the committed
  `data/packs/cf/` traces: **the cluster-weight-share combination of
  the two zone traces reconstructs the GB trace** — max per-period
  residual 3.0e-07 (float32 cutout arithmetic; the evidence-based
  tolerance is 1e-5), max annual-energy residual 1.1e-07 relative
  (the derivation-correctness identity). Scenario pairing rule
  (ADOPTED, supervisor decision 2026-07-04 on review condition 1 —
  docs/notes/b6-two-zone-data-review.md §2): ONSHORE splits by the
  observed DESNZ end-2024 share (Scotland 0.6997, NOT the 0.7361
  cluster share, which overstates the observed Scottish onshore
  energy share — anti-conservative for Q2/Q10); offshore/solar keep
  the cluster shares. GB-energy cost of the adopted onshore split:
  +0.05% (2024) / +0.22% (40y) / max +0.54% (1985), reported per year
  in `gb2_cf_report.json` and sanity-bounded at 1% in the script.
- Layout: `data/packs/cf-gb2/{sco,rgb}_{onshore,offshore,solar}_cf_<YEAR>.{parquet,csv}`
  (GB trace format exactly) + `gb2_cf_report.json` (zone weights,
  factors, annual CFs, per-year reconstruction residuals).
- Manifest: `data/packs/cf-gb2-1985-2024.sha256` (481 entries; same
  relative-path convention; environment pinning above applies — same
  era5-venv).
- Evidence note: `docs/notes/b6-two-zone-data-report.md`.
- Licence/attribution: ERA5 unchanged (Copernicus attribution).

Run order (same venv):

```
python derive_cf_gb2zone.py <repo-root>                 # full 1985-2024 sweep
python derive_cf_gb2zone.py <repo-root> --years 2024    # spot check, no report
cd data/packs && shasum -a 256 cf-gb2/*.parquet cf-gb2/*.csv \
    cf-gb2/gb2_cf_report.json > cf-gb2-1985-2024.sha256
```
