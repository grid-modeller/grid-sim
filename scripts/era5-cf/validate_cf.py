#!/usr/bin/env python3
"""Validate the three ERA5-derived CF traces against the pack conventions.

Sibling of scripts/fetch-2024/validate.py, applying the same index/NaN/
range discipline to the Phase A deliverables (docs/05-validation.md):

    data/packs/2024/processed/gb_onshore_cf_2024.parquet
    data/packs/2024/processed/gb_offshore_cf_2024.parquet
    data/packs/2024/processed/gb_solar_cf_2024.parquet

Checks (exit non-zero on any failure):
1. Exactly 17,568 half-hourly periods (2024 is a leap year).
2. No gaps, no duplicates; strictly uniform 30-min UTC index; the
   2024-03-31 / 2024-10-27 GB clock-change UTC days hold 48 periods each.
3. Single value column `cf`, float64; index column `utc_start` stored as
   timestamp[us, tz=UTC] (what grid-core's trace loader requires, ADR-3).
4. No NaNs; all values in [0, 1].
5. CSV twin exists and matches the Parquet to 1e-9.
6. Solar: zero at night — every 2024 period with sun below the horizon
   nationwide (proxy: 22:00-02:00 UTC in Dec/Jan) must be exactly 0.

Deterministic, no network. Usage: python validate_cf.py <repo-root>
"""

import sys
from pathlib import Path

import pandas as pd
import pyarrow.parquet as pq

EXPECTED = 17_568
STEMS = ("gb_onshore_cf_2024", "gb_offshore_cf_2024", "gb_solar_cf_2024")


def check(stem: str, processed: Path, failures: list) -> None:
    path = processed / f"{stem}.parquet"
    if not path.exists():
        failures.append(f"{stem}: missing (run derive_cf.py)")
        return
    schema = pq.read_schema(path)
    if str(schema.field("utc_start").type) != "timestamp[us, tz=UTC]":
        failures.append(
            f"{stem}: utc_start is {schema.field('utc_start').type}, "
            "expected timestamp[us, tz=UTC]"
        )
    if str(schema.field("cf").type) != "double":
        failures.append(f"{stem}: cf is {schema.field('cf').type}, expected double")

    df = pd.read_parquet(path)
    if list(df.columns) != ["cf"]:
        failures.append(f"{stem}: columns {list(df.columns)}, expected ['cf']")
    if len(df) != EXPECTED:
        failures.append(f"{stem}: {len(df)} periods, expected {EXPECTED}")
    if df.index.duplicated().any():
        failures.append(f"{stem}: duplicate periods")
    deltas = df.index.to_series().diff().dropna().unique()
    if len(deltas) != 1 or deltas[0] != pd.Timedelta(minutes=30):
        failures.append(f"{stem}: index not uniform 30-min: {deltas}")
    for day in ("2024-03-31", "2024-10-27"):
        if len(df.loc[day]) != 48:
            failures.append(f"{stem}: UTC day {day} has {len(df.loc[day])} periods")
    if df["cf"].isna().any():
        failures.append(f"{stem}: NaNs")
    lo, hi = float(df["cf"].min()), float(df["cf"].max())
    if lo < 0.0 or hi > 1.0:
        failures.append(f"{stem}: values outside [0, 1]: {lo}..{hi}")

    csv = pd.read_csv(
        processed / f"{stem}.csv", index_col="utc_start", parse_dates=True
    )
    if (csv["cf"] - df["cf"]).abs().max() > 1e-9:
        failures.append(f"{stem}: CSV and Parquet disagree")

    if stem == "gb_solar_cf_2024":
        night = df.loc[
            df.index.month.isin((1, 12)) & ((df.index.hour >= 22) | (df.index.hour < 2)),
            "cf",
        ]
        if (night != 0.0).any():
            failures.append(f"{stem}: nonzero at midwinter night")

    print(f"{stem}: {len(df)} periods, mean {df['cf'].mean():.4f}, "
          f"range {lo:.4f}..{hi:.4f}")


def main() -> None:
    repo = Path(sys.argv[1])
    processed = repo / "data" / "packs" / "2024" / "processed"
    failures: list = []
    for stem in STEMS:
        check(stem, processed, failures)
    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    print("\nAll CF-trace validation checks passed.")


if __name__ == "__main__":
    main()
