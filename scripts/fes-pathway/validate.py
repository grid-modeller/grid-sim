#!/usr/bin/env python3
"""Audit data/reference/fes-pathway.toml against its published sources.

Four independent checks, all failing loudly (exit 1):

1. REPORT ANCHORS — reproduce the published headline capacities of the
   FES 2025 report ("FES: NESO Pathways to Net Zero 2025", sha256 pinned
   in fetch.py) from the ES1 CSV: the derived sum rounded to the
   report's 1 dp must equal the printed value. Citations by table and
   printed page number.
2. FLX1 CROSS-CHECKS — ES1-derived storage/interconnector/dispatchable
   totals against the independently published FLX1 flexibility table.
3. TOML EQUALITY — every capacity_gw / power_gw / energy_gwh /
   demand_twh in the committed TOML equals the value recomputed from the
   pinned CSVs via build.py's mapping (catches hand-edits and drift
   between build.py and the committed artefact).
4. RECONCILIATION — every ES1 Holistic Transition capacity MW is either
   mapped into the TOML or is the documented non-networked offshore
   wind exclusion; totals reconcile to < 0.5 MW in every year.

Usage: python validate.py <repo-root>
"""

import csv
import sys
import tomllib
from pathlib import Path

# Reuse the mapping and loaders — the audit must test the same mapping
# the builder used, and the mapping is defined exactly once.
sys.path.insert(0, str(Path(__file__).parent))
from build import (  # noqa: E402
    FLEET_MAP,
    GRID,
    PATHWAY,
    STORAGE_MAP,
    YEARS,
    cap_gw,
    check_sha256,
    energy_gwh,
    load_ed1_demand_twh,
    load_es1,
)

FAILURES = []


def check(label: str, ok: bool, detail: str = "") -> None:
    print(f"{'PASS' if ok else 'FAIL'}  {label}" + (f"  [{detail}]" if detail else ""))
    if not ok:
        FAILURES.append(label)


def comps(*pairs):
    return list(pairs)


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: python validate.py <repo-root>")
    root = Path(sys.argv[1])
    raw = root / "data" / "packs" / "fes2025" / "raw"
    es1 = load_es1(raw / "fes2025_es1_v006.csv")
    demand = load_ed1_demand_twh(raw / "fes2025_ed1_v006.csv")

    def gw(components, year, grid_only=True):
        v = cap_gw(es1, components, year)
        if not grid_only:  # add the non-networked tier (offshore wind)
            v += (
                sum(
                    float(r[str(year)])
                    for r in es1
                    if r["Variable"] == "Capacity (MW)"
                    and (r["Type"], r["SubType"]) in components
                    and r["Connection"] == "Non-Networked"
                )
                / 1000.0
            )
        return v

    unabated_gas = comps(
        ("Gas", "CCGT"),
        ("Gas", "Gas CCGT CHP"),
        ("Gas", "OCGT"),
        ("Gas", "Gas Reciprocating Engines"),
        ("Gas", "Gas CHP"),
    )
    ldes = STORAGE_MAP[1][1]
    h2 = dict(FLEET_MAP)["hydrogen_turbine"]
    lcd = comps(("CCS Gas", "CCS Gas")) + h2  # "low carbon dispatchable"

    # ---- 1. report anchors ---------------------------------------------
    # (citation, published GW, components, year, includes non-networked)
    anchors = [
        ("Table 3 p.45 2024 offshore wind", 15.5, comps(("Offshore Wind", "Offshore Wind")), 2024, True),
        ("Table 3 p.45 2024 onshore wind", 14.6, comps(("Onshore Wind", "Onshore Wind")), 2024, True),
        ("Table 3 p.45 2024 solar", 18.8, comps(("Solar PV", "Solar PV")), 2024, True),
        ("Table 3 p.45 2024 nuclear", 6.1, dict(FLEET_MAP)["nuclear"], 2024, True),
        ("Table 3 p.45 2024 biomass/BECCS", 4.3, dict(FLEET_MAP)["biomass"], 2024, True),
        ("Table 3 p.45 2024 unabated gas", 39.3, unabated_gas, 2024, True),
        ("Table 3 p.45 2024 batteries", 6.8, comps(("Battery", "Battery")), 2024, True),
        ("Table 3 p.45 2024 LDES", 2.8, ldes, 2024, True),
        # NESO's headline offshore capacity includes the non-networked tier
        # (104.4 only reconciles with it; see the TOML header).
        ("Table 24 p.130 offshore wind 2030", 46.5, comps(("Offshore Wind", "Offshore Wind")), 2030, False),
        ("Table 24 p.130 offshore wind 2050", 104.4, comps(("Offshore Wind", "Offshore Wind")), 2050, False),
        ("Table 25 p.132 onshore wind 2030", 29.8, comps(("Onshore Wind", "Onshore Wind")), 2030, True),
        ("Table 25 p.132 onshore wind 2050", 47.5, comps(("Onshore Wind", "Onshore Wind")), 2050, True),
        ("Table 26 p.134 solar 2030", 46.7, comps(("Solar PV", "Solar PV")), 2030, True),
        ("Table 26 p.134 solar 2050", 97.0, comps(("Solar PV", "Solar PV")), 2050, True),
        ("Table 27 p.136 tidal (marine) 2050", 1.7, comps(("Other Renewable", "Marine")), 2050, True),
        ("Table 28 p.138 batteries 2030", 23.2, comps(("Battery", "Battery")), 2030, True),
        ("Table 28 p.138 batteries 2050", 39.3, comps(("Battery", "Battery")), 2050, True),
        ("Table 29 p.140 LDES 2030", 5.3, ldes, 2030, True),
        ("Table 29 p.140 LDES 2050", 16.5, ldes, 2050, True),
        ("Table 30 p.142 interconnectors 2030", 11.7, comps(("Interconnectors", "Interconnectors")), 2030, True),
        ("Table 30 p.142 interconnectors 2050", 21.8, comps(("Interconnectors", "Interconnectors")), 2050, True),
        ("Table 31 p.144 nuclear 2030", 2.9, dict(FLEET_MAP)["nuclear"], 2030, True),
        ("Table 31 p.144 nuclear 2050", 14.2, dict(FLEET_MAP)["nuclear"], 2050, True),
        ("Table 32 p.146 low-carbon dispatchable 2030", 1.0, lcd, 2030, True),
        ("Table 32 p.146 low-carbon dispatchable 2050", 48.3, lcd, 2050, True),
        ("Table 33 p.148 unabated gas 2030", 31.2, unabated_gas, 2030, True),
        ("Table 33 p.148 unabated gas 2050", 0.0, unabated_gas, 2050, True),
        ("Table 34 p.150 BECCS 2030", 0.6, comps(("CCS Biomass", "CCS Biomass")), 2030, True),
        ("Table 34 p.150 BECCS 2050", 2.7, comps(("CCS Biomass", "CCS Biomass")), 2050, True),
    ]
    print("== 1. FES 2025 report anchors (published vs ES1-derived) ==")
    for label, published, components, year, grid_only in anchors:
        derived = gw(components, year, grid_only)
        check(label, round(derived, 1) == published, f"pub {published} vs {derived:.4f}")

    # ---- 2. FLX1 cross-checks ------------------------------------------
    print("== 2. FLX1 cross-checks (independent NESO table) ==")
    flx_path = raw / "fes2025_flx1_v006.csv"
    check_sha256(flx_path)
    with flx_path.open() as f:
        flx = [r for r in csv.DictReader(f) if r["Pathway"] == PATHWAY]

    def flx_val(item, unit, detail, year):
        for r in flx:
            if (r["Data item"], r["Unit"], r["Detail"]) == (item, unit, detail):
                return float(r[str(year)])
        raise KeyError((item, unit, detail))

    flx_checks = [
        ("battery GW", "Electricity storage connection capacity", "GW", "Battery", comps(("Battery", "Battery")), cap_gw),
        ("battery GWh", "Electricity storage energy storage potential", "GWh", "Battery", comps(("Battery", "Battery")), energy_gwh),
        ("pumped hydro GW", "Electricity storage connection capacity", "GW", "Pumped Hydro", comps(("Long Duration Energy Storage", "Pumped Hydro")), cap_gw),
        ("pumped hydro GWh", "Electricity storage energy storage potential", "GWh", "Pumped Hydro", comps(("Long Duration Energy Storage", "Pumped Hydro")), energy_gwh),
        ("liquid air GW", "Electricity storage connection capacity", "GW", "Liquid Air", comps(("Long Duration Energy Storage", "Liquid Air")), cap_gw),
        ("compressed air GW", "Electricity storage connection capacity", "GW", "Compressed Air", comps(("Long Duration Energy Storage", "Compressed Air")), cap_gw),
        ("interconnectors GW", "Interconnectors", "GW", "Capacity", comps(("Interconnectors", "Interconnectors")), cap_gw),
        ("hydrogen generation GW", "Dispatchable electricity supply capacity", "GW", "Hydrogen generation", h2, cap_gw),
        ("unabated gas GW", "Dispatchable electricity supply capacity", "GW", "Gas generation", unabated_gas, cap_gw),
    ]
    for label, item, unit, detail, components, fn in flx_checks:
        for year in (2030, 2050):
            es1_v = fn(es1, components, year)
            flx_v = flx_val(item, unit, detail, year)
            check(f"FLX1 {label} {year}", abs(es1_v - flx_v) < 0.002, f"ES1 {es1_v} vs FLX1 {flx_v}")

    # ---- 3. committed TOML equality ------------------------------------
    print("== 3. committed TOML equals recomputation ==")
    toml_path = root / "data" / "reference" / "fes-pathway.toml"
    doc = tomllib.load(toml_path.open("rb"))
    ok = (
        doc["schema"] == "fes-pathway-v1"
        and doc["name"] == PATHWAY
        and doc["fes_edition"] == "FES 2025"
        and [y["year"] for y in doc["years"]] == YEARS
    )
    check("TOML preamble and year coverage", ok)
    mismatches = 0
    for yblock in doc["years"]:
        y = yblock["year"]
        if yblock["demand_twh"] != demand[y]:
            mismatches += 1
        fleet = {f["technology"]: f["capacity_gw"] for f in yblock["fleet"]}
        if set(fleet) != {t for t, _ in FLEET_MAP}:
            mismatches += 1
        for tech, components in FLEET_MAP:
            if fleet.get(tech) != cap_gw(es1, components, y):
                mismatches += 1
        store = {s["kind"]: s for s in yblock["storage"]}
        for kind, components in STORAGE_MAP:
            if store.get(kind, {}).get("power_gw") != cap_gw(es1, components, y):
                mismatches += 1
            if store.get(kind, {}).get("energy_gwh") != energy_gwh(es1, components, y):
                mismatches += 1
    check("every TOML number equals its recomputed value", mismatches == 0, f"{mismatches} mismatches")

    # ---- 4. reconciliation ---------------------------------------------
    print("== 4. ES1 capacity reconciliation ==")
    all_comps = [c for _, cs in FLEET_MAP + STORAGE_MAP for c in cs]
    worst = 0.0
    for y in YEARS:
        mapped_mw = cap_gw(es1, all_comps, y) * 1000.0
        total_mw = sum(
            float(r[str(y)])
            for r in es1
            if r["Variable"] == "Capacity (MW)" and r["Connection"] in GRID
        )
        worst = max(worst, abs(mapped_mw - total_mw))
    check("mapped == ES1 grid-connected total, every year", worst < 0.5, f"worst |diff| {worst:.3f} MW")
    nn = [
        (r["Type"], r["SubType"])
        for r in es1
        if r["Variable"] == "Capacity (MW)" and r["Connection"] == "Non-Networked"
    ]
    check("only documented exclusion is non-networked offshore wind", nn == [("Offshore Wind", "Offshore Wind")], str(nn))

    print()
    if FAILURES:
        sys.exit(f"{len(FAILURES)} check(s) FAILED: {FAILURES}")
    print(f"all checks passed ({PATHWAY}, FES 2025, years {YEARS[0]}-{YEARS[-1]})")


if __name__ == "__main__":
    main()
