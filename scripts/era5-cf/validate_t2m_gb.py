#!/usr/bin/env python3
"""Independent validation of the GB population-weighted t2m trace.

Reads ONLY the outputs of derive_t2m_gb.py (data/weather/gb_t2m_pop.*)
and re-asserts, exiting non-zero on any failure:

1. Parquet: single float64 column `t2m_pop`, index `utc_start`
   timestamp UTC.
2. Period count: 701,280 (30 x 17,520 + 10 x 17,568, 1985-2024).
3. Index strictly uniform 30-min UTC across the whole span — every year
   boundary and every DST clock change continuous (UTC has none, which is
   the assertion).
4. Span: 1985-01-01 00:00Z .. 2024-12-31 23:30Z.
5. No NaNs; plausible range (-25, 40) C; record mean in (8, 12) C.
6. CSV/Parquet value agreement (CSV is the human copy, docs/06).
7. Per-calendar-year period counts (17,520; 17,568 leap).
"""

import sys
from pathlib import Path

import numpy as np
import pandas as pd


def fail(msg: str) -> None:
    print(f"FAIL: {msg}")
    sys.exit(1)


def main() -> None:
    repo = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    d = repo / "data" / "weather"

    df = pd.read_parquet(d / "gb_t2m_pop.parquet")
    if list(df.columns) != ["t2m_pop"]:
        fail(f"columns {list(df.columns)}, expected ['t2m_pop']")
    if str(df["t2m_pop"].dtype) != "float64":
        fail(f"dtype {df['t2m_pop'].dtype}, expected float64")
    if df.index.name != "utc_start":
        fail(f"index name {df.index.name}, expected utc_start")
    if str(df.index.tz) != "UTC":
        fail(f"index tz {df.index.tz}, expected UTC")

    if len(df) != 701_280:
        fail(f"{len(df)} periods, expected 701,280")
    diffs = df.index.to_series().diff().dropna().unique()
    if len(diffs) != 1 or diffs[0] != pd.Timedelta(minutes=30):
        fail("index not strictly uniform 30-min over the full span")
    if df.index[0] != pd.Timestamp("1985-01-01 00:00", tz="UTC"):
        fail(f"starts {df.index[0]}, expected 1985-01-01 00:00Z")
    if df.index[-1] != pd.Timestamp("2024-12-31 23:30", tz="UTC"):
        fail(f"ends {df.index[-1]}, expected 2024-12-31 23:30Z")

    s = df["t2m_pop"]
    if s.isna().any():
        fail("NaNs present")
    if not (-25.0 < s.min() and s.max() < 40.0):
        fail(f"range [{s.min():.2f}, {s.max():.2f}] C implausible")
    if not (8.0 < s.mean() < 12.0):
        fail(f"record mean {s.mean():.2f} C implausible")

    counts = s.groupby(s.index.year).size()
    for year, n in counts.items():
        want = 17_568 if pd.Timestamp(f"{year}-12-31").is_leap_year else 17_520
        if n != want:
            fail(f"{year}: {n} periods, expected {want}")

    csv = pd.read_csv(d / "gb_t2m_pop.csv", index_col=0, parse_dates=True)
    if len(csv) != len(df):
        fail("CSV/Parquet length mismatch")
    if not np.allclose(csv["t2m_pop"].to_numpy(), s.to_numpy(), atol=1e-9):
        fail("CSV/Parquet value mismatch")

    print(
        f"OK: 701,280 periods 1985-2024, uniform 30-min UTC, no NaNs, "
        f"range [{s.min():.2f}, {s.max():.2f}] C, mean {s.mean():.2f} C, "
        f"CSV agrees"
    )


if __name__ == "__main__":
    main()
