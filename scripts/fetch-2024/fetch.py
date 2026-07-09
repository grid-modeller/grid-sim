#!/usr/bin/env python3
"""Fetch the 2024 validation-pack raw data.

Provisional script — to be ported to `grid-cli fetch-data` in Stage 0.
Deterministic: fixed URLs, fixed date ranges, no randomness. Network access
is limited to the two sources below.

Sources (accessed 2026-07-02):
1. NESO Data Portal, "Historic Demand Data 2024" (CKAN resource
   f6d02c0f-957b-48cb-82ee-09003f2ba759), NESO Open Data Licence.
2. Elexon Insights API, dataset FUELHH (half-hourly generation by fuel
   type, including INT* interconnector flows and PS pumped storage).
   No API key required. Fetched in monthly chunks.

Usage: python fetch.py <repo-root>
"""

import json
import sys
from pathlib import Path

import requests

NESO_DEMAND_2024_URL = (
    "https://api.neso.energy/dataset/8f2fe0af-871c-488d-8bad-960426f24601/"
    "resource/f6d02c0f-957b-48cb-82ee-09003f2ba759/download/demanddata_2024.csv"
)

ELEXON_FUELHH_STREAM = "https://data.elexon.co.uk/bmrs/api/v1/datasets/FUELHH/stream"

# Monthly chunks covering calendar 2024 in settlement dates. FUELHH is keyed
# by (settlementDate, settlementPeriod); settlement dates are local-clock
# days, so this range covers all UTC periods of 2024 (the build step trims
# to the exact UTC year using the startTime field).
MONTH_ENDS = {
    1: 31, 2: 29, 3: 31, 4: 30, 5: 31, 6: 30,
    7: 31, 8: 31, 9: 30, 10: 31, 11: 30, 12: 31,
}


def fetch_neso(raw_dir: Path) -> None:
    out = raw_dir / "demanddata_2024.csv"
    print(f"fetching NESO demand -> {out}")
    r = requests.get(NESO_DEMAND_2024_URL, timeout=120)
    r.raise_for_status()
    out.write_bytes(r.content)
    print(f"  {len(r.content)} bytes")


def fetch_fuelhh(raw_dir: Path) -> None:
    # Include 2023-12-31 and 2025-01-01 so UTC-year trimming has full cover
    # around the settlement-day/UTC-day offset (GB local == UTC in winter,
    # so this is belt-and-braces only).
    ranges = [("2023-12-31", "2023-12-31")]
    ranges += [
        (f"2024-{m:02d}-01", f"2024-{m:02d}-{MONTH_ENDS[m]:02d}") for m in range(1, 13)
    ]
    ranges += [("2025-01-01", "2025-01-01")]
    for date_from, date_to in ranges:
        out = raw_dir / f"fuelhh_{date_from}_{date_to}.json"
        if out.exists():
            print(f"skip (exists): {out.name}")
            continue
        print(f"fetching FUELHH {date_from}..{date_to}")
        r = requests.get(
            ELEXON_FUELHH_STREAM,
            params={"settlementDateFrom": date_from, "settlementDateTo": date_to},
            timeout=300,
        )
        r.raise_for_status()
        data = r.json()
        if not isinstance(data, list) or not data:
            raise RuntimeError(f"unexpected FUELHH response for {date_from}: {data!r:.200}")
        out.write_text(json.dumps(data))
        print(f"  {len(data)} records")


def main() -> None:
    repo = Path(sys.argv[1])
    raw_dir = repo / "data" / "packs" / "2024" / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    fetch_neso(raw_dir)
    fetch_fuelhh(raw_dir)
    print("done")


if __name__ == "__main__":
    main()
