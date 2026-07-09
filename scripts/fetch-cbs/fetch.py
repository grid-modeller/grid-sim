#!/usr/bin/env python3
"""Fetch the CBS (Statistics Netherlands) 2024 NL anchor evidence.

Purpose (docs/notes/eu-cf-review.md ruling 1 + defect D3; trigger
adjudicated in docs/notes/stage-5-review.md addendum ruling 3): ENTSO-E
A75 under-reports Dutch distribution-connected generation (onshore wind
7,653.9 GWh reported vs ~17.7 TWh real; solar 487.4 GWh vs ~21.8 TWh
real), so the NL onshore-wind and NL solar CF traces shipped
uncalibrated. The sanctioned recalibration path is observed-statistics
anchoring against CBS StatLine — this script fetches that anchor
evidence, raw and unmodified, from the CBS OData API.

Tables (both retrieved 2026-07-03; 2024 status "NaderVoorlopig" =
revised provisional — recorded per-row by build.py):

- 82610NED "Hernieuwbare elektriciteit; productie en vermogen":
  gross/net/normalised production (mln kWh) and installed end-of-year
  electrical capacity (MW) per renewable source. We take wind-onshore
  (E006637), wind-offshore (E006638, corroboration only) and solar
  (E006590, the single national all-sector total).
- 85005NED "Zonnestroom; vermogen en vermogensklasse":
  solar PV panel capacity (kWp, DC), inverter capacity (kW, AC) and
  production per sector. We take national (NL01) totals for
  all-sectors (E007161) plus the dwellings (E007037) / all-economic-
  activities (T001081) split that sums to it — documenting exactly
  what the 82610NED solar total contains.

Licence: CBS website content, including StatLine, is Creative Commons
Attribution 4.0 (CC BY 4.0) — verified 2026-07-03 at
https://www.cbs.nl/en-gb/about-us/website/copyright ("the content of
this website is subject to Creative Commons Attribution (CC BY 4.0)";
naming CBS as source is mandatory). Attribution carried on the pack
and in the derivation report: "Source: CBS (Statistics Netherlands),
StatLine tables 82610NED and 85005NED".

Deterministic fetch: fixed URLs (datasets.cbs.nl OData v1, the current
CBS OData endpoint), no randomness; resumable (one file per request,
atomic writes, existing files skipped — the entsoe fetch.py pattern).
No credentials required (open API). NOTE: CBS revises provisional
figures in place; a re-fetch after a revision will produce different
raw bytes and fail the committed manifest — that is the desired
behaviour (anchor drift must be visible, never silent).

Usage: python fetch.py <repo-root>
"""

import json
import sys
import time
from pathlib import Path

import requests

API = "https://datasets.cbs.nl/odata/v1/CBS"

# (filename stem, table, entity-set + query). Observations filters are
# fixed: year 2024, the named source/sector codes, national region.
QUERIES = [
    ("82610NED_properties", "82610NED", "Properties"),
    ("82610NED_perioden_2024", "82610NED", "PeriodenCodes?$filter=Identifier eq '2024JJ00'"),
    ("82610NED_measures", "82610NED", "MeasureCodes"),
    ("82610NED_brontechniek", "82610NED", "BronTechniekCodes"),
    (
        "82610NED_observations_2024",
        "82610NED",
        "Observations?$filter=Perioden eq '2024JJ00' and "
        "(BronTechniek eq 'E006637' or BronTechniek eq 'E006638' "
        "or BronTechniek eq 'E006590')",
    ),
    ("85005NED_properties", "85005NED", "Properties"),
    ("85005NED_perioden_2024", "85005NED", "PeriodenCodes?$filter=Identifier eq '2024JJ00'"),
    ("85005NED_measures", "85005NED", "MeasureCodes"),
    ("85005NED_sectors", "85005NED", "SectorEnVermogensklasseCodes"),
    (
        "85005NED_observations_2024",
        "85005NED",
        "Observations?$filter=Perioden eq '2024JJ00' and RegioS eq 'NL01' and "
        "(SectorEnVermogensklasse eq 'E007161' "
        "or SectorEnVermogensklasse eq 'E007037' "
        "or SectorEnVermogensklasse eq 'T001081')",
    ),
]


def fetch_one(out: Path, url: str) -> None:
    if out.exists():
        print(f"skip (exists): {out.name}")
        return
    r = requests.get(url, timeout=60)
    r.raise_for_status()
    doc = r.json()  # parse: fail loudly on non-JSON before writing
    tmp = out.with_suffix(".tmp")
    tmp.write_text(json.dumps(doc, indent=1, sort_keys=True) + "\n")
    tmp.rename(out)
    print(f"fetched: {out.name}")
    time.sleep(0.5)  # polite pacing; no documented cap, keep it light


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: fetch.py <repo-root>")
    raw = Path(sys.argv[1]) / "data" / "packs" / "cbs-2024" / "raw"
    raw.mkdir(parents=True, exist_ok=True)
    for stem, table, query in QUERIES:
        fetch_one(raw / f"{stem}.json", f"{API}/{table}/{query}")
    print("CBS raw fetch complete")


if __name__ == "__main__":
    main()
