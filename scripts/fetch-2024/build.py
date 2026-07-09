#!/usr/bin/env python3
"""Build the processed 2024 validation-pack traces from raw fetched data.

Provisional script — to be ported to `grid-cli fetch-data` in Stage 0.
Deterministic: pure transformation of the raw files, no network, no
randomness.

Outputs (data/packs/2024/processed/), all indexed by `utc_start` — the UTC
start of each half-hourly settlement period, calendar year 2024
(17,568 periods, leap year), per ADR-3:

- demand_2024.{csv,parquet}       NESO demand columns (MW), plus the built
                                  column `underlying_demand` = nd +
                                  embedded_wind_generation +
                                  embedded_solar_generation (D3
                                  total-generation convention,
                                  docs/notes/d3-embedded-convention.md)
- generation_by_fuel_2024.{csv,parquet}  Elexon FUELHH, one column per
                                  fuel type (MW); INT* columns are
                                  interconnector net flows (+ = import)
- wind_cf_2024.{csv,parquet}      single column `wind_cf`: PROVISIONAL
                                  observed fleet-wide wind capacity factor,
                                  (FUELHH wind + NESO embedded-wind
                                  estimate) / 29,100 MW constant end-2024
                                  capacity. For Stage 0 trace-loading tests
                                  only — NOT the ERA5-derived trace (D1);
                                  early-2024 values biased low by capacity
                                  growth during the year.

UTC conversion for the NESO file: settlement dates are Europe/London
clock days, period 1 starting 00:00 local; periods count 46/50 on the
short/long clock-change days. utc_start = local midnight (converted to
UTC) + (period-1) * 30 min. The Elexon file carries an explicit UTC
`startTime` which is used directly.

Usage: python build.py <repo-root>
"""

import json
import sys
from datetime import timedelta
from pathlib import Path
from zoneinfo import ZoneInfo

import pandas as pd

LONDON = ZoneInfo("Europe/London")

EXPECTED_PERIODS = 17_568  # 2024 is a leap year: 366 * 48


def utc_index_2024() -> pd.DatetimeIndex:
    return pd.date_range("2024-01-01 00:00", "2024-12-31 23:30", freq="30min", tz="UTC")


def build_demand(raw_dir: Path) -> pd.DataFrame:
    df = pd.read_csv(raw_dir / "demanddata_2024.csv")
    local_midnight = pd.to_datetime(
        df["SETTLEMENT_DATE"], format="%d-%b-%Y"
    ).dt.tz_localize(LONDON)
    df["utc_start"] = local_midnight.dt.tz_convert("UTC") + (
        df["SETTLEMENT_PERIOD"] - 1
    ).apply(lambda p: timedelta(minutes=30 * p))
    df = df.drop(columns=["SETTLEMENT_DATE", "SETTLEMENT_PERIOD"])
    df.columns = [c.lower() for c in df.columns]
    # D3 (total-generation convention): underlying demand = ND grossed up
    # by the NESO embedded-generation estimates.
    df["underlying_demand"] = (
        df["nd"] + df["embedded_wind_generation"] + df["embedded_solar_generation"]
    )
    return df.set_index("utc_start").sort_index()


def build_generation(raw_dir: Path) -> pd.DataFrame:
    records = []
    for f in sorted(raw_dir.glob("fuelhh_*.json")):
        records.extend(json.loads(f.read_text()))
    df = pd.DataFrame.from_records(records)
    df["utc_start"] = pd.to_datetime(df["startTime"], utc=True)
    df["publishTime"] = pd.to_datetime(df["publishTime"], utc=True)
    # Keep the latest publication per (period, fuel) — Elexon revises.
    df = (
        df.sort_values("publishTime")
        .drop_duplicates(subset=["utc_start", "fuelType"], keep="last")
    )
    wide = df.pivot(index="utc_start", columns="fuelType", values="generation")
    wide = wide.loc[(wide.index >= "2024-01-01") & (wide.index < "2025-01-01")]
    # INTGRNL (Greenlink) only reported from its 2024 go-live; absent
    # periods are genuinely zero flow, not gaps.
    if "INTGRNL" in wide.columns:
        wide["INTGRNL"] = wide["INTGRNL"].fillna(0)
    wide.columns = [c.lower() for c in wide.columns]
    return wide.sort_index()


WIND_CAPACITY_MW = 29_100  # constant end-2024: 14.7 GW offshore (UKWED)
#                            + 14.4 GW GB onshore (UKWED minus NI)


def build_wind_cf(demand: pd.DataFrame, gen: pd.DataFrame) -> pd.DataFrame:
    """Provisional observed fleet-wide wind capacity factor (see docstring).

    Uses a CONSTANT end-2024 capacity denominator, so values early in the
    year are biased low by within-year capacity growth. Clamped to [0, 1]
    only if the raw ratio strays outside; any clamping is reported.
    """
    cf = (gen["wind"] + demand["embedded_wind_generation"]) / WIND_CAPACITY_MW
    below, above = int((cf < 0).sum()), int((cf > 1).sum())
    if below or above:
        print(f"wind_cf: clamped {below} periods <0 and {above} periods >1 "
              f"(raw range {cf.min():.4f}..{cf.max():.4f})")
    else:
        print(f"wind_cf: no clamping needed (range {cf.min():.4f}..{cf.max():.4f})")
    return cf.clip(0.0, 1.0).rename("wind_cf").to_frame()


def write(df: pd.DataFrame, out_dir: Path, stem: str) -> None:
    df.to_csv(out_dir / f"{stem}.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
    try:
        df.to_parquet(out_dir / f"{stem}.parquet")
    except ImportError:
        print(f"pyarrow unavailable; skipped {stem}.parquet")


def main() -> None:
    repo = Path(sys.argv[1])
    raw_dir = repo / "data" / "packs" / "2024" / "raw"
    out_dir = repo / "data" / "packs" / "2024" / "processed"
    out_dir.mkdir(parents=True, exist_ok=True)

    demand = build_demand(raw_dir)
    gen = build_generation(raw_dir)

    expected = utc_index_2024()
    assert len(expected) == EXPECTED_PERIODS
    for name, df in (("demand", demand), ("generation", gen)):
        missing = expected.difference(df.index)
        extra = df.index.difference(expected)
        print(f"{name}: {len(df)} periods, missing={len(missing)}, extra={len(extra)}")

    write(demand, out_dir, "demand_2024")
    write(gen, out_dir, "generation_by_fuel_2024")
    write(build_wind_cf(demand, gen), out_dir, "wind_cf_2024")
    print("built:", ", ".join(p.name for p in sorted(out_dir.iterdir())))


if __name__ == "__main__":
    main()
