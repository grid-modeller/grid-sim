#!/usr/bin/env python3
"""Tile the 2024 demand profile across 1985-2024 for multi-year storage runs.

Supervisor design decision (2026-07-02, Stage 3 part 2 preparation):
demand for non-2024 weather years is the 2024 profile tiled by CALENDAR
DATE — for weather year Y,

    demand(Y, month, day, half-hour) = demand(2024, month, day, half-hour)

- Feb 29: non-leap years simply omit it (17,520 periods); leap years use
  2024's Feb 29 (17,568 periods).
- 2024's own file is the REAL 2024 demand (same two columns), so scenario
  trace lists are one uniform per-year family.

Known limitations, stated plainly:
- Day-of-week misalignment: a 2024 Saturday profile may land on a Tuesday
  in year Y (weekday/weekend structure is not preserved).
- No demand growth: every year carries 2024's level; the scenario's
  `annual_scale` handles level scaling.
Both are standard practice in fixed-demand storage studies (the Royal
Society large-scale-storage study tiled a fixed demand profile the same
way). The point of the multi-year record is weather variability against a
FIXED fleet and demand.

Input:  data/packs/2024/processed/demand_2024.parquet (D3 pack; columns
        `underlying_demand` = ND + NESO embedded estimates, and `nd`,
        both int64 MW, 17,568 half-hourly UTC periods).
Output: data/packs/demand-tiled/demand_<YEAR>.{parquet,csv} for
        1985-2024, columns [underlying_demand, nd] (int64 MW),
        `utc_start` index (timestamp[us, tz=UTC]).

Deterministic, no network. Usage: python tile_demand.py <repo-root>
"""

import sys
from pathlib import Path

import pandas as pd

YEARS = range(1985, 2025)
COLUMNS = ["underlying_demand", "nd"]


def main() -> None:
    repo = Path(sys.argv[1])
    src_path = repo / "data" / "packs" / "2024" / "processed" / "demand_2024.parquet"
    out_dir = repo / "data" / "packs" / "demand-tiled"
    out_dir.mkdir(parents=True, exist_ok=True)

    src = pd.read_parquet(src_path)[COLUMNS]
    if len(src) != 17_568 or src.isna().any().any():
        sys.exit(f"{src_path}: expected 17,568 clean periods, got {len(src)}")
    # Lookup keyed by (month, day, half-hour) — the tiling rule.
    by_date = src.copy()
    by_date.index = src.index.strftime("%m-%dT%H:%M")

    for year in YEARS:
        index = pd.date_range(
            f"{year}-01-01 00:00", f"{year}-12-31 23:30", freq="30min", tz="UTC"
        )
        if year == 2024:
            df = src.copy()  # the real 2024 demand, uniform column family
        else:
            df = by_date.loc[index.strftime("%m-%dT%H:%M")].set_axis(index)
        df.index.name = "utc_start"
        stem = out_dir / f"demand_{year}"
        df.to_csv(f"{stem}.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
        df.to_parquet(f"{stem}.parquet")
        print(f"demand_{year}: {len(df)} periods "
              f"({'real 2024' if year == 2024 else 'tiled from 2024'})")

    print(f"written {len(YEARS)} year files to {out_dir}")


if __name__ == "__main__":
    main()
