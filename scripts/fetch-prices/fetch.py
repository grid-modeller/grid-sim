#!/usr/bin/env python3
"""Fetch the 2024 price-series raw data (Stage 2 pricing-layer extension).

Provisional script — to be ported to `grid-cli fetch-data` alongside
`scripts/fetch-2024/`. Deterministic: fixed URLs, fixed date ranges, no
randomness. Network access is limited to the sources below.

Sources (all accessed 2026-07-02; licences in README.md):
1. Elexon Insights API, dataset MID (Market Index Data: half-hourly GB
   market price/volume per Market Index Data Provider). No API key.
2. Elexon Insights API, settlement system prices (DISEBSP: single
   imbalance price post-P305, per settlement date). No API key.
3. ONS "System Average Price (SAP) of gas" dataset, 9 Jan 2025 edition
   (daily GB OCM gas price, p/kWh; source National Gas Transmission;
   OGL v3.0).
4. GOV.UK / DESNZ-UK ETS Authority, "Report on the functioning of the UK
   carbon market for 2024" (Oct 2025; OGL v3.0) — PDF fetched for
   provenance; its Table 1 (25 UKA auction clearing prices) is
   transcribed into data/reference/prices-2024.toml.

Usage: python fetch.py <repo-root>
"""

import json
import sys
from datetime import date, timedelta
from pathlib import Path

import requests

ELEXON_MID_STREAM = "https://data.elexon.co.uk/bmrs/api/v1/datasets/MID/stream"
ELEXON_SYSTEM_PRICES = (
    "https://data.elexon.co.uk/bmrs/api/v1/balancing/settlement/system-prices"
)
ONS_SAP_XLSX = (
    "https://www.ons.gov.uk/file?uri=/economy/economicoutputandproductivity/"
    "output/datasets/systemaveragepricesapofgas/2024/"
    "systemaveragepriceofgasdataset090125.xlsx"
)
UK_CARBON_MARKET_2024_PDF = (
    "https://assets.publishing.service.gov.uk/media/68ee0df182670806f9d5e00f/"
    "report-on-the-functioning-of-the-UK-carbon-market-for-2024.pdf"
)

# UTC month windows covering calendar 2024, padded a day each side so the
# build step can trim to the exact UTC year (MID `from`/`to` are UTC).
MONTH_ENDS = {
    1: 31, 2: 29, 3: 31, 4: 30, 5: 31, 6: 30,
    7: 31, 8: 31, 9: 30, 10: 31, 11: 30, 12: 31,
}


def fetch_mid(raw_dir: Path) -> None:
    windows = [("2023-12-31T00:00Z", "2024-01-01T00:00Z")]
    for m in range(1, 13):
        end = (
            f"2025-01-01T00:00Z" if m == 12
            else f"2024-{m + 1:02d}-01T00:00Z"
        )
        windows.append((f"2024-{m:02d}-01T00:00Z", end))
    windows.append(("2025-01-01T00:00Z", "2025-01-02T00:00Z"))
    for w_from, w_to in windows:
        out = raw_dir / f"mid_{w_from[:10]}_{w_to[:10]}.json"
        if out.exists():
            print(f"skip (exists): {out.name}")
            continue
        print(f"fetching MID {w_from}..{w_to}")
        r = requests.get(
            ELEXON_MID_STREAM, params={"from": w_from, "to": w_to}, timeout=300
        )
        r.raise_for_status()
        data = r.json()
        if not isinstance(data, list) or not data:
            raise RuntimeError(f"unexpected MID response for {w_from}: {data!r:.200}")
        out.write_text(json.dumps(data))
        print(f"  {len(data)} records")


def fetch_system_prices(raw_dir: Path) -> None:
    # One request per settlement date (the endpoint has no range form).
    # Padded a day each side of 2024 for the settlement-day/UTC-day offset.
    d, end = date(2023, 12, 31), date(2025, 1, 1)
    month_records: dict[str, list] = {}
    while d <= end:
        month_key = f"{d.year}-{d.month:02d}"
        out = raw_dir / f"system_prices_{month_key}.json"
        if out.exists() and month_key not in month_records:
            print(f"skip (exists): {out.name}")
            # skip to first day of next month
            d = (d.replace(day=1) + timedelta(days=32)).replace(day=1)
            continue
        r = requests.get(f"{ELEXON_SYSTEM_PRICES}/{d.isoformat()}", timeout=120)
        r.raise_for_status()
        payload = r.json()
        recs = payload.get("data", [])
        if not recs:
            raise RuntimeError(f"no system-price records for {d}")
        month_records.setdefault(month_key, []).extend(recs)
        nxt = d + timedelta(days=1)
        if nxt.month != d.month or nxt > end:
            out.write_text(json.dumps(month_records.pop(month_key)))
            print(f"wrote {out.name}")
        d = nxt


def fetch_file(url: str, out: Path) -> None:
    if out.exists():
        print(f"skip (exists): {out.name}")
        return
    print(f"fetching {out.name}")
    r = requests.get(url, timeout=300)
    r.raise_for_status()
    out.write_bytes(r.content)
    print(f"  {len(r.content)} bytes")


def main() -> None:
    repo = Path(sys.argv[1])
    raw_dir = repo / "data" / "packs" / "2024" / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    fetch_mid(raw_dir)
    fetch_system_prices(raw_dir)
    fetch_file(ONS_SAP_XLSX, raw_dir / "ons_sap_of_gas_090125.xlsx")
    fetch_file(UK_CARBON_MARKET_2024_PDF, raw_dir / "uk-carbon-market-report-2024.pdf")
    print("done")


if __name__ == "__main__":
    main()
