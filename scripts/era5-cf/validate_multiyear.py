#!/usr/bin/env python3
"""Validate the Phase B per-year CF traces and the tiled demand traces.

Sibling of validate_cf.py (which validates the Phase A 2024 layout),
extended to the multi-year layouts consumed by Stage 3 part 2:

    data/packs/cf/gb_{onshore,offshore,solar}_cf_<YEAR>.{parquet,csv}
    data/packs/demand-tiled/demand_<YEAR>.{parquet,csv}   (1985-2024)

Checks (exit non-zero on any failure):
1. Per-year period counts: 17,520 (17,568 leap); strictly uniform 30-min
   UTC index; no duplicates; first period Jan 1 00:00Z, last Dec 31
   23:30Z. (GB clock-change UTC days therefore hold 48 periods each.)
2. Cross-year continuity: the engine's multi-file concat loader (docs/03
   migration note item 3) enforces a 30-min step across file boundaries —
   for every pair of CONSECUTIVE years present, last(Y) + 30 min must
   equal first(Y+1). Checked for the CF family (over the years present)
   and the demand-tiled family (full 1985-2024 chain).
3. Schema: CF files single float64 column `cf` in [0, 1], no NaNs;
   demand files columns [underlying_demand, nd] int64, no NaNs, positive;
   index `utc_start` stored as timestamp[us, tz=UTC] (grid-core loader
   requirement, ADR-3).
4. CSV twin exists and matches the Parquet (CF to 1e-9; demand exactly).
5. CF years come in complete technology triples (no partial years).
6. 2024 value identity: data/packs/cf/gb_*_cf_2024 must be value-identical
   to the pinned Phase A traces in data/packs/2024/processed/ (the pinned
   calibration re-derivation guarantees it; this check proves it).
7. Demand tiling identity: every tiled year's values equal the 2024
   pack's at the same (month, day, half-hour), all periods; 2024 equals
   the pack columns exactly.

Deterministic, no network.
Usage: python validate_multiyear.py <repo-root>
"""

import calendar
import re
import sys
from pathlib import Path

import pandas as pd
import pyarrow.parquet as pq

CF_TECHS = ("onshore", "offshore", "solar")
DEMAND_YEARS = range(1985, 2025)
DEMAND_COLUMNS = ["underlying_demand", "nd"]


def expected_periods(year: int) -> int:
    return 17_568 if calendar.isleap(year) else 17_520


def check_index(df: pd.DataFrame, year: int, label: str, failures: list) -> None:
    if len(df) != expected_periods(year):
        failures.append(
            f"{label}: {len(df)} periods, expected {expected_periods(year)}"
        )
    if df.index.duplicated().any():
        failures.append(f"{label}: duplicate periods")
    deltas = df.index.to_series().diff().dropna().unique()
    if len(deltas) != 1 or deltas[0] != pd.Timedelta(minutes=30):
        failures.append(f"{label}: index not uniform 30-min: {deltas}")
    if df.index[0] != pd.Timestamp(f"{year}-01-01 00:00", tz="UTC"):
        failures.append(f"{label}: starts {df.index[0]}, expected Jan 1 00:00Z")
    if df.index[-1] != pd.Timestamp(f"{year}-12-31 23:30", tz="UTC"):
        failures.append(f"{label}: ends {df.index[-1]}, expected Dec 31 23:30Z")


def check_utc_start_schema(path: Path, label: str, failures: list) -> None:
    schema = pq.read_schema(path)
    if str(schema.field("utc_start").type) != "timestamp[us, tz=UTC]":
        failures.append(
            f"{label}: utc_start is {schema.field('utc_start').type}, "
            "expected timestamp[us, tz=UTC]"
        )


def check_cross_year(bounds: dict, family: str, failures: list) -> None:
    """bounds: year -> (first_ts, last_ts). Consecutive years must chain
    with a 30-min step across the file boundary (concat-loader contract)."""
    for year in sorted(bounds):
        if year + 1 in bounds:
            gap = bounds[year + 1][0] - bounds[year][1]
            if gap != pd.Timedelta(minutes=30):
                failures.append(
                    f"{family}: {year}->{year + 1} boundary step is {gap}, "
                    "expected 30 min"
                )


def check_cf(repo: Path, failures: list) -> dict:
    cf_dir = repo / "data" / "packs" / "cf"
    years = sorted(
        {
            int(m.group(1))
            for p in cf_dir.glob("gb_*_cf_*.parquet")
            if (m := re.match(r"gb_(?:onshore|offshore|solar)_cf_(\d{4})$", p.stem))
        }
    )
    if not years:
        failures.append(f"no CF traces found in {cf_dir}")
        return {}
    means: dict = {}
    bounds: dict = {}
    for year in years:
        for tech in CF_TECHS:
            stem = f"gb_{tech}_cf_{year}"
            path = cf_dir / f"{stem}.parquet"
            if not path.exists():
                failures.append(f"{stem}: missing (partial year {year})")
                continue
            check_utc_start_schema(path, stem, failures)
            schema = pq.read_schema(path)
            if str(schema.field("cf").type) != "double":
                failures.append(
                    f"{stem}: cf is {schema.field('cf').type}, expected double"
                )
            df = pd.read_parquet(path)
            if list(df.columns) != ["cf"]:
                failures.append(f"{stem}: columns {list(df.columns)}, expected ['cf']")
            check_index(df, year, stem, failures)
            if df["cf"].isna().any():
                failures.append(f"{stem}: NaNs")
            lo, hi = float(df["cf"].min()), float(df["cf"].max())
            if lo < 0.0 or hi > 1.0:
                failures.append(f"{stem}: values outside [0, 1]: {lo}..{hi}")
            csv = pd.read_csv(
                cf_dir / f"{stem}.csv", index_col="utc_start", parse_dates=True
            )
            if len(csv) != len(df) or (csv["cf"] - df["cf"]).abs().max() > 1e-9:
                failures.append(f"{stem}: CSV and Parquet disagree")
            means.setdefault(year, {})[tech] = float(df["cf"].mean())
            bounds[year] = (df.index[0], df.index[-1])
    check_cross_year(bounds, "cf", failures)

    # 2024 value identity vs the pinned Phase A traces.
    if 2024 in years:
        phase_a = repo / "data" / "packs" / "2024" / "processed"
        for tech in CF_TECHS:
            new = pd.read_parquet(cf_dir / f"gb_{tech}_cf_2024.parquet")
            old = pd.read_parquet(phase_a / f"gb_{tech}_cf_2024.parquet")
            if not new.equals(old):
                failures.append(
                    f"gb_{tech}_cf_2024: NOT value-identical to Phase A "
                    f"({(new['cf'] - old['cf']).abs().max()} max abs diff)"
                )
        print("2024 CF traces: value-identical to Phase A (all three techs)")
    return means


def check_demand(repo: Path, failures: list) -> int:
    tiled_dir = repo / "data" / "packs" / "demand-tiled"
    src = pd.read_parquet(
        repo / "data" / "packs" / "2024" / "processed" / "demand_2024.parquet"
    )[DEMAND_COLUMNS]
    by_date = src.copy()
    by_date.index = src.index.strftime("%m-%dT%H:%M")
    bounds: dict = {}
    n_ok = 0
    for year in DEMAND_YEARS:
        stem = f"demand_{year}"
        path = tiled_dir / f"{stem}.parquet"
        if not path.exists():
            failures.append(f"{stem}: missing")
            continue
        check_utc_start_schema(path, stem, failures)
        df = pd.read_parquet(path)
        if list(df.columns) != DEMAND_COLUMNS:
            failures.append(
                f"{stem}: columns {list(df.columns)}, expected {DEMAND_COLUMNS}"
            )
        if [str(t) for t in df.dtypes] != ["int64", "int64"]:
            failures.append(f"{stem}: dtypes {list(df.dtypes)}, expected int64")
        check_index(df, year, stem, failures)
        if df.isna().any().any():
            failures.append(f"{stem}: NaNs")
        if (df <= 0).any().any():
            failures.append(f"{stem}: non-positive demand values")
        csv = pd.read_csv(
            tiled_dir / f"{stem}.csv", index_col="utc_start", parse_dates=True
        )
        if len(csv) != len(df) or (csv[DEMAND_COLUMNS] != df[DEMAND_COLUMNS]).any().any():
            failures.append(f"{stem}: CSV and Parquet disagree")
        # Tiling identity: every period equals 2024 at (month, day, hh:mm).
        expected = by_date.loc[df.index.strftime("%m-%dT%H:%M")].set_axis(df.index)
        if not df.equals(expected):
            failures.append(f"{stem}: values do not match 2024 tiling rule")
        bounds[year] = (df.index[0], df.index[-1])
        n_ok += 1
    check_cross_year(bounds, "demand-tiled", failures)
    return n_ok


def main() -> None:
    repo = Path(sys.argv[1])
    failures: list = []

    means = check_cf(repo, failures)
    if means:
        print(f"\nCF years present: {sorted(means)}")
        print("annual mean CF per year:")
        print(f"  {'year':>4}  {'onshore':>8}  {'offshore':>8}  {'solar':>7}")
        for year in sorted(means):
            m = means[year]
            print(
                f"  {year:>4}  {m.get('onshore', float('nan')):8.4f}  "
                f"{m.get('offshore', float('nan')):8.4f}  "
                f"{m.get('solar', float('nan')):7.4f}"
            )

    n_demand = check_demand(repo, failures)
    print(f"\ndemand-tiled: {n_demand}/{len(DEMAND_YEARS)} year files validated")

    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    print("\nAll multi-year validation checks passed.")


if __name__ == "__main__":
    main()
