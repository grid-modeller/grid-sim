#!/usr/bin/env python3
"""Fetch the ENTSO-E Stage 5 raw data (2024).

Provisional script (fetch-2024/fetch-prices pattern). Deterministic: fixed
domains, fixed date ranges, no randomness; the only network access is the
ENTSO-E Transparency Platform RESTful API (web-api.tp.entsoe.eu).

Credentials: the API token is read from $ENTSOE_TOKEN or, failing that,
~/.local/share/grid-sim/entsoe-token. It is NEVER hardcoded, never written
into the repo, and never printed/logged (error messages are scrubbed).

What is fetched (all periods UTC; API periodStart/periodEnd are UTC):
- A11 cross-border physical flows, per GB border, per direction, per month.
  The platform separates flows by BIDDING-ZONE BORDER, not by asset: the
  GB<->FR series is IFA + IFA2 + ElecLink combined (per-asset virtual
  zones GB(IFA)/GB(IFA2)/GB(ElecLink) return no A11 data — probed
  2026-07-03); GB<->IE(SEM) is EWIC + Moyle + Greenlink combined. Per-link
  GB-side data lives in the 2024 validation pack (NESO/Elexon).
- A11 FR non-GB cross-border physical flows, per direction, per month
  (Stage 5 A2 remediation, observed FR wedge; added 2026-07-03, see
  FR_BORDERS below).
- A65/A16 actual total load per neighbour zone, per month.
- A68/A33 installed capacity per production type per zone, year document.
- A75/A16 actual generation per production type, NO2 and NO country
  aggregate, per month (Norwegian-hydro evidence for the Stage 5 sign
  test; the EU weather pack carries no hydro variables).
- A72/A16 weekly reservoir filling, NO2 and NO, one year document each.

Rate limiting: the platform documents a 400 req/min cap; we sleep 0.35 s
between requests (~170 req/min) and back off 60 s on HTTP 429.

Resumable: one file per request, atomic writes, existing files skipped.
"No matching data" acknowledgements are saved as-is (they are meaningful:
for a flow direction they mean no flow was reported in that direction;
build.py decides per data type how to interpret them).

Usage: python fetch.py <repo-root>
"""

import os
import sys
import time
from pathlib import Path

import requests

API = "https://web-api.tp.entsoe.eu/api"

# Bidding-zone EIC codes (ENTSO-E area codes, verified by probe 2026-07-03).
ZONES = {
    "gb": "10YGB----------A",
    "fr": "10YFR-RTE------C",
    "be": "10YBE----------2",
    "nl": "10YNL----------L",
    "delu": "10Y1001A1001A82H",  # DE-LU bidding zone (single zone since 2018)
    "no2": "10YNO-2--------T",  # NSL counterparty zone (Kvilldal landing)
    "no": "10YNO-0--------C",  # Norway country aggregate (D5 evidence)
    "dk1": "10YDK-1--------W",  # Viking counterparty zone (Jutland)
    "ie": "10Y1001A1001A59C",  # SEM (Ireland + Northern Ireland)
    "ch": "10YCH-SWISSGRIDZ",  # Switzerland (FR non-GB border)
    "es": "10YES-REE------0",  # Spain (FR non-GB border)
    "it_north": "10Y1001A1001A73I",  # IT-North bidding zone (FR border)
}

# GB borders carrying interconnectors in 2024 -> NESO per-link mapping.
BORDERS = ["fr", "be", "nl", "no2", "dk1", "ie"]

# FR non-GB borders (A2 remediation, 2026-07-03): the observed
# counterparties of FR's non-GB net-export series. Border discovery probe
# (2026-07-03, one month per direction per candidate): the ONLY FR<->IT-*
# border the platform serves is FR<->IT-North (10Y1001A1001A73I); the
# virtual IT_North_FR zone (10Y1001A1001A81J) returns "no matching data"
# acknowledgements in both directions, and the IT country aggregate
# (10YIT-GRTN-----B) returns data identical to IT-North (the same single
# border serialised twice) — so IT-North alone is fetched.
FR_BORDERS = ["be", "delu", "ch", "it_north", "es"]

LOAD_ZONES = ["fr", "be", "nl", "delu", "no2", "dk1", "ie"]

MONTHS = [f"2024-{m:02d}" for m in range(1, 13)]


def month_bounds(ym: str) -> tuple[str, str]:
    y, m = int(ym[:4]), int(ym[5:7])
    ny, nm = (y + 1, 1) if m == 12 else (y, m + 1)
    return f"{y}{m:02d}010000", f"{ny}{nm:02d}010000"


def token() -> str:
    tok = os.environ.get("ENTSOE_TOKEN", "")
    if not tok:
        tok = (
            Path.home()
            .joinpath(".local/share/grid-sim/entsoe-token")
            .read_text()
            .strip()
        )
    if not tok:
        raise RuntimeError("no ENTSO-E token (env ENTSOE_TOKEN or token file)")
    return tok


def fetch_one(out: Path, params: dict, tok: str) -> None:
    """One API request -> one file. Atomic, skip-if-exists, 429 backoff."""
    if out.exists():
        print(f"skip (exists): {out.name}")
        return
    p = dict(params)
    p["securityToken"] = tok
    for attempt in range(6):
        try:
            r = requests.get(API, params=p, timeout=120)
        except requests.RequestException as e:
            # scrub any URL (it embeds the token) from the message
            print(f"  transient error ({type(e).__name__}), retrying")
            time.sleep(30)
            continue
        if r.status_code == 429:
            print("  429 throttled, backing off 60 s")
            time.sleep(60)
            continue
        if r.status_code == 200:
            tmp = out.with_suffix(out.suffix + ".tmp")
            tmp.write_bytes(r.content)
            tmp.rename(out)
            print(f"fetched: {out.name} ({len(r.content)} bytes)")
            time.sleep(0.35)
            return
        raise RuntimeError(f"HTTP {r.status_code} for {out.name}: {r.text[:200]}")
    raise RuntimeError(f"gave up after retries: {out.name}")


def main() -> None:
    repo = Path(sys.argv[1])
    raw = repo / "data" / "packs" / "entsoe-2024" / "raw"
    raw.mkdir(parents=True, exist_ok=True)
    tok = token()

    # A11 physical flows: in_Domain is the RECEIVING zone.
    for b in BORDERS:
        for ym in MONTHS:
            start, end = month_bounds(ym)
            fetch_one(
                raw / f"flows_{b}_imp_{ym}.xml",
                {
                    "documentType": "A11",
                    "in_Domain": ZONES["gb"],
                    "out_Domain": ZONES[b],
                    "periodStart": start,
                    "periodEnd": end,
                },
                tok,
            )
            fetch_one(
                raw / f"flows_{b}_exp_{ym}.xml",
                {
                    "documentType": "A11",
                    "in_Domain": ZONES[b],
                    "out_Domain": ZONES["gb"],
                    "periodStart": start,
                    "periodEnd": end,
                },
                tok,
            )

    # A11 FR non-GB physical flows (Stage 5 A2 remediation work order,
    # 2026-07-03): imp = into FR, exp = out of FR — the observed series
    # behind the 5-zone scenario's flat FR identity wedge. Same machinery;
    # existing files are skipped, so earlier fetches are untouched.
    for b in FR_BORDERS:
        for ym in MONTHS:
            start, end = month_bounds(ym)
            fetch_one(
                raw / f"flows_fr_{b}_imp_{ym}.xml",
                {
                    "documentType": "A11",
                    "in_Domain": ZONES["fr"],
                    "out_Domain": ZONES[b],
                    "periodStart": start,
                    "periodEnd": end,
                },
                tok,
            )
            fetch_one(
                raw / f"flows_fr_{b}_exp_{ym}.xml",
                {
                    "documentType": "A11",
                    "in_Domain": ZONES[b],
                    "out_Domain": ZONES["fr"],
                    "periodStart": start,
                    "periodEnd": end,
                },
                tok,
            )

    # A65 actual total load.
    for z in LOAD_ZONES:
        for ym in MONTHS:
            start, end = month_bounds(ym)
            fetch_one(
                raw / f"load_{z}_{ym}.xml",
                {
                    "documentType": "A65",
                    "processType": "A16",
                    "outBiddingZone_Domain": ZONES[z],
                    "periodStart": start,
                    "periodEnd": end,
                },
                tok,
            )

    # A68 installed capacity per production type (year document).
    for z in LOAD_ZONES:
        fetch_one(
            raw / f"capacity_{z}_2024.xml",
            {
                "documentType": "A68",
                "processType": "A33",
                "in_Domain": ZONES[z],
                "periodStart": "202401010000",
                "periodEnd": "202412312300",
            },
            tok,
        )

    # A75 actual generation per type. NO2 + NO aggregate (Norwegian-hydro
    # evidence, original 2026-07-03 fetch); extended 2026-07-03 (same day,
    # EU CF derivation work order) to the six external-zone calibration
    # countries — the anchor for the per-country ERA5 CF calibration
    # (scripts/era5-cf/derive_cf_eu.py). Existing files are skipped, so
    # the original fetch is untouched.
    for z in ["no2", "no", "fr", "be", "nl", "delu", "dk1", "ie"]:
        for ym in MONTHS:
            start, end = month_bounds(ym)
            fetch_one(
                raw / f"gen_{z}_{ym}.xml",
                {
                    "documentType": "A75",
                    "processType": "A16",
                    "in_Domain": ZONES[z],
                    "periodStart": start,
                    "periodEnd": end,
                },
                tok,
            )

    # A72 weekly reservoir filling, NO2 + NO (single year documents);
    # extended 2026-07-03 (Stage 5 A2 remediation work order) to FR — the
    # FR reservoir-hydro budget evidence, same weekly conventions as the
    # Norwegian series. Existing files are skipped (resume logic), so the
    # original fetch is untouched.
    for z in ["no2", "no", "fr"]:
        fetch_one(
            raw / f"reservoir_{z}_2024.xml",
            {
                "documentType": "A72",
                "processType": "A16",
                "in_Domain": ZONES[z],
                "periodStart": "202401010000",
                "periodEnd": "202501010000",
            },
            tok,
        )

    print("done")


if __name__ == "__main__":
    main()
