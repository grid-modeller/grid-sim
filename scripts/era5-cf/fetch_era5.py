#!/usr/bin/env python3
"""Fetch box cutouts of ERA5 (ARCO-ERA5 mirror, or Earthmover icechunk).

D1 pipeline (docs/08-risks-and-decisions.md, resolved as direct ERA5
derivation). Phase A fetched calendar year 2024 from ARCO; Phase B
(launched 2026-07-02) extended the identical extraction to 1985-2023.
On 2026-07-03 the 1985-2023 record was re-sourced in full from the
Earthmover icechunk ERA5 store (--source earthmover), whose spatial
chunking makes the GB cutout ~2 orders of magnitude cheaper to read;
2024 stays ARCO-sourced (Phase A, validated, byte-pinned). See
scripts/era5-cf/README.md "Source switch" for the cross-source seam.

Source A, --source arco (default; accessed 2026-07-02, anonymous read):
    gs://gcp-public-data-arco-era5/ar/full_37-1h-0p25deg-chunk-1.zarr-v3
This is Google's Analysis-Ready Cloud-Optimized mirror of ERA5 (native
0.25 deg, hourly, float32). Store coverage attrs at retrieval:
    valid_time_start = 1940-01-01, valid_time_stop = 2025-12-31 (final ERA5)

Source B, --source earthmover (accessed 2026-07-03, anonymous read):
    s3://earthmover-icechunk-era5/icechunkV2 (us-east-1), branch `main`,
    zarr group `single/temporal`, vars u100/v100/ssrd/t2m already under
    their short names, float32, chunks (8736, 12, 12) = 364 days x
    12 x 12 cells. The time coordinate is `valid_time` (hours since
    1940-01-01, proleptic_gregorian), renamed to `time` on output.
    The icechunk snapshot ID actually read is printed at open (pinned
    in the README); pass --snapshot to re-read that exact snapshot.
    Reads iterate the store's 364-day time chunks once each (chunk-
    aligned; months are cut from an in-memory carry buffer, never by
    re-reading a chunk per month). The earthmover path OVERWRITES
    existing month files (no skip): it exists to replace ARCO-sourced
    files with a homogeneous Earthmover-sourced record.

All fetched years (1985-2024) are *final* ERA5 (not ERA5T), so a re-fetch
reproduces the cutouts bit-for-bit and the SHA-256 manifests
(data/packs/era5-2024.sha256, data/packs/era5-1985-2023.sha256) are
meaningful — conditional on the pinned environment (requirements.txt):
Parquet/zstd bytes vary across pyarrow versions; the earthmover manifest
is additionally conditional on the pinned icechunk snapshot ID.

Licence: ERA5 is Copernicus (CC-BY-4.0-equivalent) - redistribution of
derived products permitted with attribution: "Contains modified Copernicus
Climate Change Service information [2024]". See scripts/era5-cf/README.md.

Cutout box (--box, added 2026-07-03 for the NW-Europe Stage 5 fetch):
    default          GB box 49-61N, 8W-2E (0.25 deg grid: 49 x 41 =
                     2,009 cells) -> data/packs/era5/<year>/
                     era5_gb_<year>-MM.parquet — byte-identical behaviour
                     to the pre---box script (all committed manifests
                     remain valid).
    --box NAME,LATMIN,LATMAX,LONMIN,LONMAX
                     e.g. eu,42,72,-11,16 (NW-Europe import-counterparty
                     box: 121 x 109 = 13,189 cells). Longitudes in
                     degrees East, negative = west of Greenwich; all
                     four bounds must lie on the 0.25 deg grid. Output:
                     data/packs/era5-<NAME>/<year>/
                     era5_<NAME>_<year>-MM.parquet (own manifest; never
                     mixes with the GB layout).
Hourly UTC, 8,760 steps (8,784 leap years). Variables (short name on disk):
    100m_u_component_of_wind          -> u100  [m/s, instantaneous]
    100m_v_component_of_wind          -> v100  [m/s, instantaneous]
    surface_solar_radiation_downwards -> ssrd  [J/m2 ACCUMULATED over the
                                                hour ENDING at the label;
                                                verified - see README; the
                                                earthmover store states the
                                                same in an attr]
    2m_temperature                    -> t2m   [K, instantaneous]
(10m winds are not fetched: the derivation uses 100m only.)

The ARCO store is chunked (1 hour, whole globe) per variable, so a box
slice still transfers global chunks: 4 vars x 8,760 h = 35,040 chunk
reads per year, done concurrently via dask's threaded scheduler. The
earthmover store needs ~n_cells/144 spatial chunks x 4 vars per 364-day
chunk (~20 for GB, ~110 for the eu box).

Resumability: output is one Parquet per month, written atomically (temp
file + rename). ARCO path: months already on disk are skipped, so an
interrupted run resumes with the same command; years are fetched
newest-first. Earthmover path: years are fetched OLDEST-first and
existing files are overwritten (see Source B above); resume by re-running
with a narrowed --years range if interrupted.

Output format (long/tidy, deterministic ordering, identical across the
two sources): columns time (UTC), latitude, longitude (degrees East,
negative west), u100, v100, ssrd, t2m (float32 exactly as stored - no
rescaling; earthmover's float64 lat/lon coords are cast to float32 to
match the ARCO schema exactly).

Deterministic: no randomness; content depends only on the store (final
ERA5; earthmover additionally pinned by snapshot ID). This is the only
script in scripts/era5-cf/ that touches the network.

Usage:
    python fetch_era5.py <repo-root>                    # Phase A: 2024
    python fetch_era5.py <repo-root> --years 1985-2023  # Phase B (ARCO)
    python fetch_era5.py <repo-root> --years 1985-2023 --source earthmover
    python fetch_era5.py <repo-root> --years 1985-2023 --status
    python fetch_era5.py <repo-root> --years 1985-2024 \
        --source earthmover --snapshot 39TK56WX185WZ1HP9WNG \
        --box eu,42,72,-11,16                           # NW-Europe box
The --status mode is offline and read-only: it reports months complete vs
total, the recent fetch rate (from file mtimes), and a projected
completion time, so progress can be checked across sessions without
reading logs.
"""

import argparse
import time
from dataclasses import dataclass
from pathlib import Path

STORE = "gs://gcp-public-data-arco-era5/ar/full_37-1h-0p25deg-chunk-1.zarr-v3"

VARS = {
    "100m_u_component_of_wind": "u100",
    "100m_v_component_of_wind": "v100",
    "surface_solar_radiation_downwards": "ssrd",
    "2m_temperature": "t2m",
}

GRID = 0.25  # both stores: native ERA5 0.25 deg grid


@dataclass(frozen=True)
class Box:
    """A lat/lon cutout box on the 0.25 deg grid. Longitudes are degrees
    East in [-180, 180) (negative = west of Greenwich); the store's
    0..359.75 axis handling lives in box_cut()."""

    name: str
    lat_min: float
    lat_max: float
    lon_min: float
    lon_max: float

    @property
    def n_cells(self) -> int:
        n_lat = round((self.lat_max - self.lat_min) / GRID) + 1
        n_lon = round((self.lon_max - self.lon_min) / GRID) + 1
        return n_lat * n_lon


# GB box: 49-61N, 8W-2E = 49 x 41 = 2,009 cells (the default; all
# committed GB manifests were produced with exactly this box).
GB_BOX = Box("gb", 49.0, 61.0, -8.0, 2.0)

N_WORKERS = 16  # concurrent chunk reads

# Earthmover icechunk ERA5 store (Source B; see module docstring).
EM_BUCKET = "earthmover-icechunk-era5"
EM_PREFIX = "icechunkV2"
EM_REGION = "us-east-1"
EM_GROUP = "single/temporal"
EM_TIME_CHUNK = 8736  # store time-chunk length: 364 days of hours,
#                       aligned to 1940-01-01 00:00


def parse_years(spec: str) -> list[int]:
    """'2024' -> [2024]; '1985-2023' -> [2023, 2022, ..., 1985] (newest
    first: recent years are the useful ones soonest)."""
    if "-" in spec:
        a, b = (int(p) for p in spec.split("-", 1))
        if b < a:
            raise SystemExit(f"bad --years range: {spec}")
        return list(range(b, a - 1, -1))
    return [int(spec)]


def parse_box(spec: str) -> Box:
    """'eu,42,72,-11,16' -> Box('eu', 42, 72, -11, 16), validated: name
    alphanumeric (it becomes a directory/file component), bounds on the
    0.25 deg grid, min < max, lat in [-90, 90], lon in [-180, 180)."""
    parts = spec.split(",")
    if len(parts) != 5:
        raise SystemExit(
            f"bad --box (want NAME,LATMIN,LATMAX,LONMIN,LONMAX): {spec}"
        )
    name = parts[0]
    if not name.isalnum() or name == "gb":
        raise SystemExit(
            f"bad --box name {name!r}: alphanumeric, and 'gb' is reserved "
            "for the default layout"
        )
    try:
        lat_min, lat_max, lon_min, lon_max = (float(p) for p in parts[1:])
    except ValueError:
        raise SystemExit(f"bad --box bounds (not numbers): {spec}") from None
    for v in (lat_min, lat_max, lon_min, lon_max):
        if (v / GRID) != round(v / GRID):
            raise SystemExit(f"--box bound {v} not on the {GRID} deg grid")
    if not (
        -90 <= lat_min < lat_max <= 90 and -180 <= lon_min < lon_max < 180
    ):
        raise SystemExit(f"--box bounds out of order/range: {spec}")
    return Box(name, lat_min, lat_max, lon_min, lon_max)


def era5_dir_for(repo_root: Path, box: Box) -> Path:
    """GB keeps the legacy data/packs/era5/ layout; any other box gets
    its own sibling tree data/packs/era5-<name>/."""
    leaf = "era5" if box.name == "gb" else f"era5-{box.name}"
    return repo_root / "data" / "packs" / leaf


def month_path(era5_dir: Path, box: Box, year: int, month: int) -> Path:
    return era5_dir / str(year) / f"era5_{box.name}_{year}-{month:02d}.parquet"


def box_cut(sub, box: Box):
    """Spatial cutout: lat descending (store convention); the store
    longitude axis is 0..359.75, so a box spanning the Greenwich meridian
    is read as west [360+lon_min, 359.75] + east [0, lon_max] and the
    west part re-labelled negative, so output longitude runs
    lon_min..lon_max ascending."""
    import xarray as xr

    lat = slice(box.lat_max, box.lat_min)
    if box.lon_min < 0 <= box.lon_max:
        west = sub.sel(
            latitude=lat, longitude=slice(360.0 + box.lon_min, 360.0 - GRID)
        )
        east = sub.sel(latitude=lat, longitude=slice(0.0, box.lon_max))
        west = west.assign_coords(longitude=west.longitude - 360.0)
        return xr.concat([west, east], dim="longitude")
    if box.lon_max < 0:  # entirely west of Greenwich
        out = sub.sel(
            latitude=lat,
            longitude=slice(360.0 + box.lon_min, 360.0 + box.lon_max),
        )
        return out.assign_coords(longitude=out.longitude - 360.0)
    return sub.sel(latitude=lat, longitude=slice(box.lon_min, box.lon_max))


def month_slice(ds, box: Box, year: int, month: int):
    import numpy as np

    t0 = f"{year}-{month:02d}-01"
    t1 = f"{year}-{month + 1:02d}-01" if month < 12 else f"{year + 1}-01-01"
    sub = ds[list(VARS)].sel(time=slice(t0, t1))
    # slice(t0, t1) includes t1 00:00 (the next month); drop it.
    sub = sub.sel(time=sub.time < np.datetime64(t1))
    return box_cut(sub, box)


def status(era5_dir: Path, box: Box, years: list[int]) -> None:
    """Progress report from the filesystem only (no network, no log
    parsing). Rate = recent months per hour, from the mtime span of the
    12 most recently written files (robust to pauses/resumes further
    back); projection assumes that rate holds."""
    done: list[Path] = []
    per_year: list[str] = []
    for year in sorted(years):
        months = [
            m
            for m in range(1, 13)
            if month_path(era5_dir, box, year, m).exists()
        ]
        done += [month_path(era5_dir, box, year, m) for m in months]
        per_year.append(f"  {year}: {len(months):2d}/12")
    total = 12 * len(years)
    print(f"months complete: {len(done)}/{total} across years "
          f"{min(years)}-{max(years)}")
    for line in per_year:
        if not line.endswith(" 0/12"):
            print(line)
    if len(done) >= 2:
        mtimes = sorted(p.stat().st_mtime for p in done)[-12:]
        span = mtimes[-1] - mtimes[0]
        if span > 0:
            rate = (len(mtimes) - 1) / span * 3600.0  # months/hour
            remaining = total - len(done)
            eta = time.time() + remaining / rate * 3600.0
            print(f"recent rate: {rate:.1f} months/h "
                  f"(last {len(mtimes)} files)")
            print(f"remaining: {remaining} months; projected completion "
                  f"{time.strftime('%Y-%m-%d %H:%M %Z', time.localtime(eta))}"
                  f" at that rate")
        age = time.time() - mtimes[-1]
        print(f"last file written {age / 60:.0f} min ago")


def fetch(era5_dir: Path, box: Box, years: list[int]) -> None:
    import xarray as xr

    print(f"opening {STORE} (anonymous)", flush=True)
    ds = xr.open_zarr(STORE, storage_options=dict(token="anon"))
    for k in sorted(ds.attrs):
        if "valid_time" in k:
            print(f"  store attr {k} = {ds.attrs[k]}", flush=True)

    total_hours = 0
    for year in years:
        (era5_dir / str(year)).mkdir(parents=True, exist_ok=True)
        for month in range(1, 13):
            out = month_path(era5_dir, box, year, month)
            if out.exists():
                print(f"{out.name}: already on disk, skipped", flush=True)
                continue
            t0 = time.time()
            sub = month_slice(ds, box, year, month)
            n_hours = sub.sizes["time"]
            n_chunks = n_hours * len(VARS)
            loaded = sub.compute(scheduler="threads", num_workers=N_WORKERS)
            df = (
                loaded.rename(VARS)
                .to_dataframe()[list(VARS.values())]
                .reset_index()
                .sort_values(
                    ["time", "latitude", "longitude"], ignore_index=True
                )
            )
            tmp = out.with_suffix(".parquet.tmp")
            df.to_parquet(
                tmp, engine="pyarrow", compression="zstd", index=False
            )
            tmp.rename(out)
            dt = time.time() - t0
            total_hours += n_hours
            print(
                f"{out.name}: {n_hours} h x {df['latitude'].nunique()}x"
                f"{df['longitude'].nunique()} cells, {n_chunks} chunks in "
                f"{dt:.0f}s ({n_chunks / dt:.1f} chunk/s), "
                f"{out.stat().st_size / 1e6:.1f} MB",
                flush=True,
            )

    print(f"done: {total_hours} new hours fetched")


def write_month_earthmover(
    month_ds, box: Box, expected_hours: int, out: Path
) -> None:
    """Emit one month from the earthmover carry buffer: identical schema
    to the ARCO path (columns time/latitude/longitude/u100/v100/ssrd/t2m;
    lat/lon cast float64->float32 to match ARCO's coord dtype). Validates
    inline (hour count, cell count, no NaNs) and refuses to write a bad
    file. Overwrites: atomic tmp+rename with an earthmover-specific tmp
    name so it can never collide with a concurrent ARCO fetcher's
    .parquet.tmp for the same month."""
    import hashlib

    m = month_ds.rename(valid_time="time")
    df = (
        m.to_dataframe()[list(VARS.values())]
        .reset_index()
        .sort_values(["time", "latitude", "longitude"], ignore_index=True)
    )
    df["latitude"] = df["latitude"].astype("float32")
    df["longitude"] = df["longitude"].astype("float32")

    n_hours = df["time"].nunique()
    if n_hours != expected_hours or len(df) != expected_hours * box.n_cells:
        raise SystemExit(
            f"{out.name}: expected {expected_hours} h x {box.n_cells} cells, "
            f"got {n_hours} h, {len(df)} rows"
        )
    if df[list(VARS.values())].isna().any().any():
        raise SystemExit(f"{out.name}: NaNs in data variables")

    tmp = out.with_suffix(".parquet.em-tmp")
    df.to_parquet(tmp, engine="pyarrow", compression="zstd", index=False)
    tmp.rename(out)
    sha = hashlib.sha256(out.read_bytes()).hexdigest()
    print(
        f"{out.name}: {n_hours} h x {box.n_cells} cells, "
        f"{out.stat().st_size / 1e6:.1f} MB, sha256={sha}",
        flush=True,
    )


def fetch_earthmover(
    era5_dir: Path, box: Box, years: list[int], snapshot: str | None
) -> None:
    """Source B: read the box cutout from the earthmover icechunk store,
    one 364-day time chunk at a time (each chunk read exactly once), and
    cut month files from an in-memory carry buffer. Years ascending;
    existing files overwritten (this path replaces ARCO-sourced files)."""
    import icechunk
    import numpy as np
    import xarray as xr

    years = sorted(years)
    if years != list(range(years[0], years[-1] + 1)):
        raise SystemExit("--source earthmover needs a contiguous year range")

    storage = icechunk.s3_storage(
        bucket=EM_BUCKET, prefix=EM_PREFIX, region=EM_REGION, anonymous=True
    )
    repo = icechunk.Repository.open(storage)
    if snapshot:
        session = repo.readonly_session(snapshot_id=snapshot)
    else:
        session = repo.readonly_session(branch="main")
    print(
        f"opened s3://{EM_BUCKET}/{EM_PREFIX} group={EM_GROUP} "
        f"snapshot_id={session.snapshot_id} (pin via --snapshot to re-fetch)",
        flush=True,
    )
    ds = xr.open_zarr(session.store, group=EM_GROUP, consolidated=False)
    ds = ds.drop_vars("lsm", errors="ignore")[list(VARS.values())]

    tv = ds["valid_time"].values
    step = EM_TIME_CHUNK
    t_start = np.datetime64(f"{years[0]}-01-01").astype("datetime64[ns]")
    t_end = np.datetime64(f"{years[-1] + 1}-01-01").astype("datetime64[ns]")
    pos0 = int(np.searchsorted(tv, t_start))
    pos1 = int(np.searchsorted(tv, t_end))
    if tv[pos0] != t_start or tv[pos1 - 1] != t_end - np.timedelta64(1, "h"):
        raise SystemExit("store time axis does not cover the requested years")

    buf = None  # carry buffer: box hours read but not yet emitted
    cur_year, cur_month = years[0], 1
    total_hours = 0
    for k in range(pos0 // step, (pos1 - 1) // step + 1):
        lo, hi = max(k * step, pos0), min((k + 1) * step, pos1)
        c0 = time.time()
        # Transient S3 connect timeouts observed (~1 per 100 chunk reads);
        # bounded retry with backoff, then fail loudly.
        for attempt in range(5):
            try:
                loaded = box_cut(
                    ds.isel(valid_time=slice(lo, hi)), box
                ).compute(scheduler="threads", num_workers=N_WORKERS)
                break
            except Exception as exc:
                if attempt == 4:
                    raise
                wait = 15 * (attempt + 1)
                print(
                    f"chunk {k}: read failed "
                    f"({type(exc).__name__}); retry in {wait}s",
                    flush=True,
                )
                time.sleep(wait)
        print(
            f"chunk {k}: {hi - lo} h [{tv[lo]} .. {tv[hi - 1]}] "
            f"read in {time.time() - c0:.0f}s",
            flush=True,
        )
        buf = loaded if buf is None else xr.concat(
            [buf, loaded], dim="valid_time"
        )
        # Emit every month now fully covered by the buffer.
        while cur_year <= years[-1]:
            if cur_month == 1:
                (era5_dir / str(cur_year)).mkdir(parents=True, exist_ok=True)
            ny, nm = (
                (cur_year + 1, 1) if cur_month == 12
                else (cur_year, cur_month + 1)
            )
            m1 = np.datetime64(f"{ny}-{nm:02d}-01").astype("datetime64[ns]")
            # Buffer may be exactly empty when a month boundary
            # coincides with a chunk boundary.
            if (
                buf.sizes["valid_time"] == 0
                or buf["valid_time"].values[-1] < m1 - np.timedelta64(1, "h")
            ):
                break
            month_ds = buf.sel(valid_time=slice(None, m1))
            month_ds = month_ds.sel(
                valid_time=month_ds.valid_time < m1
            )
            m0 = np.datetime64(
                f"{cur_year}-{cur_month:02d}-01"
            ).astype("datetime64[ns]")
            expected = int((m1 - m0) / np.timedelta64(1, "h"))
            write_month_earthmover(
                month_ds,
                box,
                expected,
                month_path(era5_dir, box, cur_year, cur_month),
            )
            total_hours += expected
            buf = buf.sel(valid_time=slice(m1, None))
            if nm == 1:
                print(f"year {cur_year} complete", flush=True)
            cur_year, cur_month = ny, nm

    print(f"done: {total_hours} hours fetched from earthmover")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument("--years", default="2024",
                    help="year or inclusive range, e.g. 1985-2023")
    ap.add_argument("--status", action="store_true",
                    help="report progress (offline) instead of fetching")
    ap.add_argument("--source", choices=["arco", "earthmover"],
                    default="arco",
                    help="arco (default; skip-if-exists, newest-first) or "
                         "earthmover (overwrite, oldest-first)")
    ap.add_argument("--snapshot", default=None,
                    help="icechunk snapshot ID to pin (earthmover only; "
                         "default: latest on main, recorded in the log)")
    ap.add_argument("--box", default=None,
                    help="cutout box NAME,LATMIN,LATMAX,LONMIN,LONMAX "
                         "(deg East, negative=west, 0.25-grid), e.g. "
                         "eu,42,72,-11,16 -> data/packs/era5-<NAME>/; "
                         "default: the GB box and legacy layout")
    args = ap.parse_args()

    box = parse_box(args.box) if args.box else GB_BOX
    era5_dir = era5_dir_for(args.repo_root, box)
    years = parse_years(args.years)
    if args.status:
        status(era5_dir, box, years)
    elif args.source == "earthmover":
        fetch_earthmover(era5_dir, box, years, args.snapshot)
    else:
        fetch(era5_dir, box, years)


if __name__ == "__main__":
    main()
