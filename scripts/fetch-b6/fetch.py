#!/usr/bin/env python3
"""Fetch the B6 two-zone evidence pack (raw files, unmodified).

B6 two-zone work package (2026-07-04): the observed-boundary and
fleet-split evidence for the Scotland / rest-of-GB split — see
docs/notes/b6-two-zone-data-report.md for the full source-by-source
licence diligence. Sources and licences (all verified 2026-07-04):

1. NESO Data Portal (licence: NESO Open Data Licence — OGL-v3-based,
   worldwide/royalty-free/perpetual, commercial re-use permitted,
   attribution "Supported by National Energy SO Open Data";
   https://www.neso.energy/data-portal/neso-open-licence; each dataset
   below individually carries that licence per its CKAN metadata):
   - Day Ahead Constraint Flows and Limits (SCOTEX = B6): half-hourly
     day-ahead boundary limit and forecast (pre-action) flow, MW;
     file retains past 3 years + current year (earlier on request).
   - Thermal Constraint Costs: daily outturn thermal-constraint cost
     per boundary group (SCOTEX, SSE-SP, SSHARN, ESTEX, SEIMP, SWALEX),
     financial years 2021-22 onward (19-20/20-21 exist only as XLSX
     maps and are not fetched).
   - Constraint Breakdown Costs and Volume: daily GB-wide constraint
     cost and volume by category (thermal/voltage/inertia/largest
     loss) — the only open constraint VOLUME series.
   - Interconnector Register: connection sites (zone assignment
     evidence: which interconnectors land in Scotland).

2. GOV.UK / DESNZ (licence: Open Government Licence v3.0, stated on
   each publication page):
   - REPD quarterly extract (April 2026 edition; the January 2025
     asset was withdrawn from gov.uk — end-2024 fleet is recovered by
     filtering on the Operational date column, see build.py).
   - Regional Renewable Statistics, installed capacity 2003-2024
     (all-size capacity incl. sub-150kW rooftop solar and legacy
     hydro that REPD misses), by country and technology.

Deterministic fetch: fixed URLs, no randomness; resumable (one file
per request, atomic writes, existing files skipped — the fetch-cbs
pattern). No credentials required. NOTE: NESO refreshes the day-ahead
file daily (rolling window) and the interconnector register file name
carries its publication date — re-fetches after new publications
produce different raw bytes and fail the committed manifest; that is
the desired behaviour (source drift must be visible, never silent).

Usage: python fetch.py <repo-root>
"""

import sys
from pathlib import Path

import requests

NESO = "https://api.neso.energy/dataset"
GOVUK = "https://assets.publishing.service.gov.uk/media"

FILES = [
    # --- NESO: B6 day-ahead flows and limits (retrieved 2026-07-04) ---
    (
        "neso_day_ahead_constraint_flows_limits.csv",
        f"{NESO}/cf3cbc92-2d5d-4c2b-bd29-e11a21070b26/resource/"
        "38a18ec1-9e40-465d-93fb-301e80fd1352/download/"
        "day-ahead-constraints-limits-and-flow-output-v1.5.csv",
    ),
    # --- NESO: thermal constraint costs per boundary, FY 2021-22 .. 2026-27 ---
    (
        "neso_thermal_constraint_costs_2021_2022.csv",
        f"{NESO}/f0055054-c55c-4068-a01c-61da4334e58f/resource/"
        "4357dd3b-5c7a-4caa-8d1a-8cf848521143/download/"
        "outturn-system-costs-2021-2022.csv",
    ),
    (
        "neso_thermal_constraint_costs_2022_2023.csv",
        f"{NESO}/f0055054-c55c-4068-a01c-61da4334e58f/resource/"
        "476b8d39-5eda-425c-9756-73ddfd36dc4d/download/"
        "outturn-system-costs-2022-2023.csv",
    ),
    (
        "neso_thermal_constraint_costs_2023_2024.csv",
        f"{NESO}/f0055054-c55c-4068-a01c-61da4334e58f/resource/"
        "75c9c564-af38-4421-a461-a612a6921212/download/"
        "outturn-system-costs-2023-2024.csv",
    ),
    (
        "neso_thermal_constraint_costs_2024_2025.csv",
        f"{NESO}/f0055054-c55c-4068-a01c-61da4334e58f/resource/"
        "27df75fd-6233-466c-beed-3e6e2261e6b1/download/"
        "outturn-system-costs-2024-2025.csv",
    ),
    (
        "neso_thermal_constraint_costs_2025_2026.csv",
        f"{NESO}/f0055054-c55c-4068-a01c-61da4334e58f/resource/"
        "407f3e9d-5c38-4a64-98f5-923dc4d64fb2/download/"
        "outturn-system-costs-2025-2026.csv",
    ),
    (
        "neso_thermal_constraint_costs_2026_2027.csv",
        f"{NESO}/f0055054-c55c-4068-a01c-61da4334e58f/resource/"
        "c730b788-4328-43dc-9f84-27fd3adeda59/download/"
        "outturn-system-costs-2026-2027.csv",
    ),
    # --- NESO: constraint breakdown (daily category cost + volume) ---
    (
        "neso_constraint_breakdown_2023_2024.csv",
        f"{NESO}/fb56b46e-cef3-4eb8-9294-0ca19769b7eb/resource/"
        "24d067d8-1328-452a-9720-21cb691e491e/download/"
        "constraint-breakdown-2023-2024.csv",
    ),
    (
        "neso_constraint_breakdown_2024_2025.csv",
        f"{NESO}/fb56b46e-cef3-4eb8-9294-0ca19769b7eb/resource/"
        "748557ef-2bb3-41c0-8181-5f1a148c1ff4/download/"
        "constraint-breakdown-2024-2025.csv",
    ),
    # --- NESO: interconnector register (landing-point evidence) ---
    (
        "neso_interconnector_register.csv",
        f"{NESO}/a7cca714-9dbb-42b1-99c8-4bc7211605a8/resource/"
        "64f7908f-f787-4977-93e1-5342a5f1357f/download/"
        "interconnector-register-03-july-2026.csv",
    ),
    # --- GOV.UK: REPD quarterly extract, April 2026 (Q1 2026) ---
    (
        "repd_q1_2026.csv",
        f"{GOVUK}/69fc56908cc72d2f863ea58d/REPD_publication_Q1_2026.csv",
    ),
    # --- GOV.UK: DESNZ regional renewable capacity 2003-2024 ---
    (
        "desnz_regional_capacity_2003_2024.xlsx",
        f"{GOVUK}/68da76f3c487360cc70c9e9e/"
        "Regional_spreadsheets__2003-2024__-_installed_capacity__MW_.xlsx",
    ),
]


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: python fetch.py <repo-root>")
    raw = Path(sys.argv[1]) / "data" / "packs" / "b6" / "raw"
    raw.mkdir(parents=True, exist_ok=True)
    for name, url in FILES:
        dest = raw / name
        if dest.exists():
            print(f"skip (exists): {name}")
            continue
        r = requests.get(url, timeout=120)
        r.raise_for_status()
        tmp = dest.with_suffix(dest.suffix + ".tmp")
        tmp.write_bytes(r.content)
        tmp.rename(dest)
        print(f"fetched: {name} ({len(r.content):,} bytes)")
    print(f"done -> {raw}")


if __name__ == "__main__":
    main()
