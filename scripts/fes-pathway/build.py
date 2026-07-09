#!/usr/bin/env python3
"""Build data/reference/fes-pathway.toml from the NESO FES 2025 data tables.

Deterministic: pure transformation of the pinned raw files (sha256 verified
before use), no network, no randomness, stdlib only. The committed TOML is
the artefact; this script is how it is regenerated and audited.

Inputs (data/packs/fes2025/raw/, fetched by fetch.py):
- fes2025_es1_v006.csv  ES1 electricity supply data table (capacities)
- fes2025_ed1_v006.csv  ED1 electricity demand summary (annual demand)

The technology mapping (NESO ES1 categories -> scenario-schema technology
ids and storage kinds) is defined ONCE here (FLEET_MAP / STORAGE_MAP) and
documented, choice by choice, in the generated TOML header. Change the
mapping here and the header text together or validate.py will disagree.

Usage: python build.py <repo-root>
"""

import csv
import hashlib
import sys
from pathlib import Path

PATHWAY = "Holistic Transition"
YEARS = list(range(2024, 2051))  # ES1 publishes annual 2024..2050
WAYPOINTS = (2024, 2030, 2040, 2050)  # for the split tables in the header

# Grid-connected tiers. "Non-Networked" (offshore wind direct-wired to
# electrolysis) is excluded from the fleet and documented in the header.
GRID = {"Transmission", "Distributed", "Distributed - Micro"}

SHA256 = {
    "fes2025_es1_v006.csv": (
        "7b7957443d37a09304fe2877bfa2a7a2fa71f8c00cb9f26308bd58391a4ff805"
    ),
    "fes2025_ed1_v006.csv": (
        "bd36b16b3f3d0cc5cc8e5118590777d18e3e501a72b325b3de617aa17d15bc24"
    ),
    # FLX1 is read by validate.py only (cross-checks).
    "fes2025_flx1_v006.csv": (
        "a5049beb77c3a58ba21f975177d309b207c667176a702387b269102fa4594462"
    ),
}

# scenario-schema technology id -> list of ES1 (Type, SubType) components.
# Rationale for every grouping is in the generated header (MAPPING section).
FLEET_MAP = [
    ("ccgt", [("Gas", "CCGT"), ("Gas", "Gas CCGT CHP"), ("CCS Gas", "CCS Gas")]),
    (
        "ocgt",
        [
            ("Gas", "OCGT"),
            ("Gas", "Gas Reciprocating Engines"),
            ("Gas", "Gas CHP"),
        ],
    ),
    ("oil", [("Other Thermal", "Diesel"), ("Other Thermal", "Fuel Oil")]),
    (
        "nuclear",
        [("Nuclear", "Nuclear - Large"), ("Nuclear", "Nuclear - Small")],
    ),
    (
        "biomass",
        [
            ("Biomass", "Biomass"),
            ("Biomass", "Biomass CHP"),
            ("CCS Biomass", "CCS Biomass"),
        ],
    ),
    ("waste", [("Waste", "Waste"), ("Waste", "Waste CHP")]),
    ("hydro", [("Hydro", "Hydro")]),
    ("marine", [("Other Renewable", "Marine")]),
    ("other", [("Other Renewable", "Geothermal CHP")]),
    ("onshore_wind", [("Onshore Wind", "Onshore Wind")]),
    ("offshore_wind", [("Offshore Wind", "Offshore Wind")]),
    ("solar", [("Solar PV", "Solar PV")]),
    ("interconnector", [("Interconnectors", "Interconnectors")]),
    (
        "hydrogen_turbine",
        [
            ("Hydrogen", "Hydrogen"),
            ("Hydrogen", "Hydrogen Peaking"),
            ("Hydrogen", "Hydrogen CHP"),
        ],
    ),
]

# StorageKind -> ES1 (Type, SubType) components (power from Capacity (MW),
# energy from Storage Capacity (GWh)).
STORAGE_MAP = [
    ("battery", [("Battery", "Battery")]),
    (
        "pumped_hydro",
        [
            ("Long Duration Energy Storage", "Pumped Hydro"),
            ("Long Duration Energy Storage", "Liquid Air"),
            ("Long Duration Energy Storage", "Compressed Air"),
        ],
    ),
]


def check_sha256(path: Path) -> None:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    expected = SHA256[path.name]
    if digest != expected:
        sys.exit(
            f"checksum mismatch for {path.name}:\n"
            f"  expected {expected}\n  got      {digest}\n"
            "NESO may have revised the table (they version in-place). Do NOT\n"
            "silently re-pin: diff the values, record the revision, then\n"
            "update SHA256 here, in fetch.py and in data/packs/fes2025.sha256."
        )


def load_es1(path: Path):
    check_sha256(path)
    with path.open() as f:
        return [r for r in csv.DictReader(f) if r["Pathway"] == PATHWAY]


def load_ed1_demand_twh(path: Path) -> dict[int, float]:
    check_sha256(path)
    with path.open() as f:
        for r in csv.DictReader(f):
            if (
                r["Pathway"] == PATHWAY
                and r["Data item"] == "GBFES System Demand: Total"
            ):
                return {y: round(float(r[str(y)]) / 1000.0, 3) for y in YEARS}
    sys.exit("ED1: 'GBFES System Demand: Total' row not found")


def cap_gw(rows, components, year, variable="Capacity (MW)") -> float:
    mw = sum(
        float(r[str(year)])
        for r in rows
        if r["Variable"] == variable
        and (r["Type"], r["SubType"]) in components
        and r["Connection"] in GRID
    )
    return round(mw / 1000.0, 4)


def energy_gwh(rows, components, year) -> float:
    gwh = sum(
        float(r[str(year)])
        for r in rows
        if r["Variable"] == "Storage Capacity (GWh)"
        and (r["Type"], r["SubType"]) in components
        and r["Connection"] in GRID
    )
    return round(gwh, 4)


def reconcile(rows) -> None:
    """Every ES1 capacity MW must be mapped or documented-excluded."""
    mapped = {c for _, comps in FLEET_MAP + STORAGE_MAP for c in comps}
    for r in rows:
        if r["Variable"] != "Capacity (MW)":
            continue
        key = (r["Type"], r["SubType"])
        if r["Connection"] == "Non-Networked":
            if key != ("Offshore Wind", "Offshore Wind"):
                sys.exit(f"unexpected Non-Networked capacity row: {key}")
            continue  # documented exclusion
        if key not in mapped:
            sys.exit(f"unmapped ES1 capacity row: {key} ({r['Connection']})")


def split_lines(rows, comps, indent="#     ") -> list[str]:
    """Per-component GW at the waypoint years, for the header."""
    out = []
    for c in comps:
        vals = "  ".join(
            f"{y}: {fmt(cap_gw(rows, [c], y))}" for y in WAYPOINTS
        )
        out.append(f"{indent}{c[0]} / {c[1]:<28} {vals}")
    return out


def fmt(v: float) -> str:
    # repr is the shortest round-trip representation (no %g 6-sig-fig
    # truncation); rounded values here never need exponent notation.
    return f"{v:.1f}" if v == int(v) else repr(v)


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: python build.py <repo-root>")
    root = Path(sys.argv[1])
    raw = root / "data" / "packs" / "fes2025" / "raw"
    es1 = load_es1(raw / "fes2025_es1_v006.csv")
    demand = load_ed1_demand_twh(raw / "fes2025_ed1_v006.csv")
    reconcile(es1)

    lines: list[str] = []
    add = lines.append

    def splits(tech_comps):
        lines.extend(split_lines(es1, tech_comps))

    add(HEADER_TOP)

    # -- MAPPING section, with computed per-component waypoint splits -----
    add("# TECHNOLOGY MAPPING (ES1 Type/SubType -> scenario technology id).")
    add("# Capacities are summed over Connection in {Transmission,")
    add("# Distributed, Distributed - Micro} — the grid-connected tiers.")
    add("# Per-component splits below are GW at 2024/2030/2040/2050 so the")
    add("# folds stay auditable without reopening the CSV.")
    add("#")
    add("# ccgt <- Gas/CCGT + Gas/Gas CCGT CHP + CCS Gas/CCS Gas.")
    add("#   All combined-cycle machine classes: synchronous steam+GT sets,")
    add("#   grid-core derived default H = 5.0 s. Gas CCS is a CCGT with")
    add("#   post-combustion capture — same rotating plant, so it keeps the")
    add("#   ccgt id; a separate open-set id (e.g. gas_ccs) would silently")
    add("#   pick up grid-core's unknown-id default (non-synchronous, no H),")
    add("#   which is physically wrong for this plant. CAVEAT for economic")
    add("#   use: the entry migrates from unabated to abated plant — by 2050")
    add("#   the whole 22.205 GW is CCS gas.")
    splits([("Gas", "CCGT"), ("Gas", "Gas CCGT CHP"), ("CCS Gas", "CCS Gas")])
    add("#")
    add("# ocgt <- Gas/OCGT + Gas/Gas Reciprocating Engines + Gas/Gas CHP.")
    add("#   The synchronous non-CCGT unabated gas bucket (peakers, engines,")
    add("#   small distributed CHP). NESO separates 'Gas CCGT CHP' from")
    add("#   'Gas CHP', so the latter is taken as engine/OCGT-class plant.")
    add("#   grid-core default H = 4.0 s (wide combustion-turbine band).")
    splits(
        [
            ("Gas", "OCGT"),
            ("Gas", "Gas Reciprocating Engines"),
            ("Gas", "Gas CHP"),
        ]
    )
    add("#")
    add("# oil <- Other Thermal/Diesel + Other Thermal/Fuel Oil.")
    add("#   'oil' is in the standard firm roster (scenario.rs), but note:")
    add("#   grid-core's inertia table has no 'oil' arm, so it gets the")
    add("#   unknown-id default (non-synchronous, no H) even though these")
    add("#   are synchronous machines. Small and shrinking (1.62 GW 2024 ->")
    add("#   0 by 2050); a scenario that cares must set inertia_h explicitly.")
    splits([("Other Thermal", "Diesel"), ("Other Thermal", "Fuel Oil")])
    add("#")
    add("# nuclear <- Nuclear - Large + Nuclear - Small (SMRs). One id: both")
    add("#   are synchronous steam plant (H default 4.5 s).")
    splits([("Nuclear", "Nuclear - Large"), ("Nuclear", "Nuclear - Small")])
    add("#")
    add("# biomass <- Biomass + Biomass CHP + CCS Biomass (BECCS). BECCS is")
    add("#   folded in because it is the same steam machine class with")
    add("#   capture added, and NESO's own headline groups 'Biomass/BECCS'")
    add("#   (report Table 3, p.45). Waste is NOT folded in — see waste.")
    splits(
        [
            ("Biomass", "Biomass"),
            ("Biomass", "Biomass CHP"),
            ("CCS Biomass", "CCS Biomass"),
        ]
    )
    add("#")
    add("# waste <- Waste + Waste CHP, kept as its own (open-set) id:")
    add("#   NESO's published biomass headline (Table 3: 4.3 GW today)")
    add("#   EXCLUDES waste, so folding it into biomass would break every")
    add("#   reconciliation against the report. Energy-from-waste is")
    add("#   steam-cycle synchronous plant, but as an unknown id grid-core")
    add("#   defaults it to non-synchronous — scenarios wanting its inertia")
    add("#   must set inertia_h (~4 s steam band) explicitly.")
    splits([("Waste", "Waste"), ("Waste", "Waste CHP")])
    add("#")
    add("# hydro <- Hydro (all tiers). Run-of-river/small hydro; synchronous,")
    add("#   H default 3.0 s. (Pumped storage is a StorageKind, not here.)")
    splits([("Hydro", "Hydro")])
    add("#")
    add("# marine <- Other Renewable/Marine (tidal stream + wave; the report")
    add("#   calls this 'Tidal', Table 27). Own open-set id: it is inverter-")
    add("#   coupled variable renewable, so grid-core's unknown-id default")
    add("#   (non-synchronous, no H) is the physically correct one, and")
    add("#   folding it into hydro would wrongly make it firm + synchronous.")
    splits([("Other Renewable", "Marine")])
    add("#")
    add("# other <- Other Renewable/Geothermal CHP. 11 MW from 2027; kept")
    add("#   only so the ES1 capacity total reconciles to the last MW.")
    splits([("Other Renewable", "Geothermal CHP")])
    add("#")
    add("# onshore_wind / offshore_wind / solar <- the like-named ES1 rows,")
    add("#   summed over tiers. 'Distributed - Micro' (rooftop solar 33.1 GW,")
    add("#   micro wind 1.0 GW by 2050) is INCLUDED: per decision D3 the")
    add("#   model uses the total-generation convention with embedded")
    add("#   capacity modelled explicitly.")
    add("#   offshore_wind EXCLUDES the 'Non-Networked' tier (direct-wired")
    add("#   to electrolysis, never grid-connected): 0 MW in 2024, 25.94 MW")
    add("#   in 2030, 371.77 MW in 2040, 442.91 MW in 2050. NESO's headline")
    add("#   offshore capacity (Table 24: 104.4 GW in 2050) INCLUDES it;")
    add("#   the grid-connected figure recorded here is 103.9305 GW.")
    splits(
        [
            ("Onshore Wind", "Onshore Wind"),
            ("Offshore Wind", "Offshore Wind"),
            ("Solar PV", "Solar PV"),
        ]
    )
    add("#")
    add("# interconnector <- Interconnectors (nameplate link capacity; all")
    add("#   HVDC, non-synchronous).")
    splits([("Interconnectors", "Interconnectors")])
    add("#")
    add("# hydrogen_turbine <- Hydrogen + Hydrogen Peaking + Hydrogen CHP.")
    add("#   Hydrogen-fuelled GENERATION, recorded as a clearly-named fleet")
    add("#   entry and deliberately NOT folded into the 'hydrogen'")
    add("#   StorageKind: the v1 modelling choice makes hydrogen storage")
    add("#   reconversion non-synchronous, and grid-core's guidance is that")
    add("#   a hydrogen-turbine scenario 'should model the turbines as a")
    add("#   fleet entry with an explicit H' (grid-core/src/inertia.rs).")
    add("#   As an unknown id it defaults to non-synchronous, no H;")
    add("#   scenarios asserting synchronous H2 turbines must set inertia_h")
    add("#   (combustion-turbine band ~4 s) explicitly. The charge side of")
    add("#   the hydrogen chain (grid-connected electrolysis: 0.77 GW 2030")
    add("#   -> 15.66 GW 2050, FLX1) and NESO's H2 storage volumes (whole-")
    add("#   system TWh of H2, report Table 38) are published elsewhere and")
    add("#   are NOT electricity-storage power/energy pairs, so no")
    add("#   'hydrogen' storage entry is emitted — see OMITTED below.")
    splits(
        [
            ("Hydrogen", "Hydrogen"),
            ("Hydrogen", "Hydrogen Peaking"),
            ("Hydrogen", "Hydrogen CHP"),
        ]
    )
    add("#")
    add("# STORAGE MAPPING (power = ES1 'Capacity (MW)', energy = ES1")
    add("# 'Storage Capacity (GWh)', same tier rule).")
    add("#")
    add("# battery <- Storage/Battery, all tiers (Distributed - Micro is")
    add("#   residential batteries; included per D3 as above).")
    add("#")
    add("# pumped_hydro <- LDES/Pumped Hydro + LDES/Liquid Air +")
    add("#   LDES/Compressed Air. StorageKind is a CLOSED set (battery |")
    add("#   pumped_hydro | hydrogen | dsr), so LAES/CAES cannot carry their")
    add("#   own kind. They are folded into pumped_hydro because (a) NESO")
    add("#   itself aggregates exactly these three as 'long duration")
    add("#   electricity storage' (report Table 29: 5.3 GW 2030, 16.5 GW")
    add("#   2050 = PH+LAES+CAES to the decimal), and (b) all three")
    add("#   discharge through turbines, so pumped_hydro's synchronous")
    add("#   H = 4.5 s default is the nearest physical class (it is an")
    add("#   approximation for LAES/CAES, and for VSD-driven charging).")
    add("#   Component splits (GW / GWh):")
    for comp in STORAGE_MAP[1][1]:
        vals = "  ".join(
            f"{y}: {fmt(cap_gw(es1, [comp], y))}/{fmt(energy_gwh(es1, [comp], y))}"
            for y in WAYPOINTS
        )
        add(f"#     {comp[1]:<16} {vals}")
    add(HEADER_REST)

    add("")
    add('schema = "fes-pathway-v1"')
    add(f'name = "{PATHWAY}"')
    add('fes_edition = "FES 2025"')

    for y in YEARS:
        add("")
        add("[[years]]")
        add(f"year = {y}")
        add(f"demand_twh = {fmt(demand[y])}")
        for tech, comps in FLEET_MAP:
            add("")
            add("[[years.fleet]]")
            add(f'technology = "{tech}"')
            add(f"capacity_gw = {fmt(cap_gw(es1, comps, y))}")
        for kind, comps in STORAGE_MAP:
            add("")
            add("[[years.storage]]")
            add(f'kind = "{kind}"')
            add(f"power_gw = {fmt(cap_gw(es1, comps, y))}")
            add(f"energy_gwh = {fmt(energy_gwh(es1, comps, y))}")

    out = root / "data" / "reference" / "fes-pathway.toml"
    out.write_text("\n".join(lines) + "\n")
    print(f"wrote {out} ({len(YEARS)} years)")


HEADER_TOP = """\
# FES 2025 "Holistic Transition" pathway reference — GB installed capacity
# by technology by year, 2024-2050, for the Stage 6 part 2 Q8 pathway
# runner (docs/04: largest survivable loss vs year). Assembled 2026-07-03
# (data engineer). Every number carries its citation; the committed file
# is regenerated deterministically by scripts/fes-pathway/build.py and
# audited by scripts/fes-pathway/validate.py.
#
# PATHWAY CHOICE: Holistic Transition is FES 2025's central net-zero
# pathway — the only pathway achieving the 2030 NDC (report p.31), and
# the basis of NESO's Clean Power 2030 advice (report p.43). The other
# published pathways are Electric Engagement, Hydrogen Evolution, the
# Ten Year Forecast and the Falling Behind counterfactual.
#
# SOURCES (all retrieved 2026-07-03)
# Primary (machine-readable, the published FES data tables on the NESO
# Data Portal — identical values to the Data Workbook sheets):
# [ES1] "FES: Pathways to Net Zero – Electricity Supply Data Table (ES1)",
#   2025 edition v006 (portal last_modified 2025-12-10). All capacities.
#   https://api.neso.energy/dataset/549b0667-b533-4748-95bd-f6e13933a47d/resource/6c78a777-b885-4bb6-bc35-8100f9e137a2/download/fes2025_es1_v006.csv
#   sha256 7b7957443d37a09304fe2877bfa2a7a2fa71f8c00cb9f26308bd58391a4ff805
# [ED1] "FES: Electricity Demand Summary Data Table (ED1)", 2025 v006.
#   The demand_twh series. Definitions: ED2 PDF on the same dataset page.
#   https://api.neso.energy/dataset/2c15c755-d8fe-4229-9169-3b6dd7c88fec/resource/300c07b9-baeb-4411-bc40-987cbb4aec0b/download/fes2025_ed1_v006.csv
#   sha256 bd36b16b3f3d0cc5cc8e5118590777d18e3e501a72b325b3de617aa17d15bc24
# Cross-checks and provenance (not read by build.py):
# [FLX1] "FES: Flexibility Data Table (FLX1)", 2025 v006 — independent
#   storage/interconnector/dispatchable totals, checked by validate.py.
#   https://api.neso.energy/dataset/2e3275e2-dd6a-4c2e-8cfb-eeb9b4320dcb/resource/299bc6c8-7608-4946-ab0a-5bc22129c897/download/fes2025_flx1_v006.csv
#   sha256 a5049beb77c3a58ba21f975177d309b207c667176a702387b269102fa4594462
# [WB] "FES: Data Workbook 2025" (xlsx). Its ES1 sheet was verified
#   value-identical to [ES1] during assembly (2026-07-03, row-level
#   comparison); kept for provenance, not read by these scripts.
#   https://www.neso.energy/document/364551/download
#   sha256 f11b9a2c08084d4c4596d67d9e498f46c8739d942ad7dd383580365e10405593
# [RPT] "FES: NESO Pathways to Net Zero 2025" (report PDF, July 2025) —
#   the spot-check citations below.
#   https://www.neso.energy/document/364541/download
#   sha256 184c745d74cbf406f3adf5c4b1d73796cfa61d48453414fb30ba4ce3a7172a34
#
# LICENCE: NESO Open Data Licence
# (https://www.neso.energy/data-portal/neso-open-licence) — the licence
# recorded on every Data Portal dataset above. It grants a worldwide,
# royalty-free, perpetual licence to copy, publish, distribute, adapt and
# exploit the information commercially and non-commercially, with
# attribution; NESO states it is CC BY 4.0 compatible. Redistributing
# this derived table is therefore permitted. Required attribution
# (carry it on published outputs derived from this file):
#   "Supported by National Energy SO Open Data"
#
# GRANULARITY: NESO publishes ES1 annually, 2024-2050 inclusive (27
# values per row); every published year is recorded, nothing is
# interpolated. 2024 is the pathway's base year ("Today" in report
# Table 3). ED1 demand is labelled "Annual [Fiscal]" (fiscal-year
# accounting; see the ED2 definitions PDF); ES1 capacity columns carry
# plain year labels. ED1 also publishes 2023; there is no matching 2023
# fleet in ES1, so this file starts at 2024.
#
# UNITS & ROUNDING: capacity_gw = sum of ES1 MW / 1000 rounded to 4 dp
# (0.1 MW); energy_gwh rounded to 4 dp; demand_twh = ED1 GWh / 1000
# rounded to 3 dp. Python banker's rounding, applied only at output.
#"""

HEADER_REST = """\
#
# OMITTED (published by NESO, not representable in this schema):
# - StorageKind hydrogen: no electrical power/energy pair is published —
#   see the hydrogen_turbine note above.
# - StorageKind dsr: NESO publishes DSR as negative peak-impact
#   decompositions (FLX1 2030 HT: residential appliance DSR -3.0 GW, I&C
#   DSR -1.7 GW, smart charging -2.74 GW, V2G -1.19 GW at peak, ...) and
#   an ES1 'Demand Reduction' energy line (0.0587 TWh in 2030), not a
#   storage-like power/energy pair. Deriving one would be a modelling
#   choice, not transcription — left to the Q6 DSR work.
# - Non-capacity ES1 variables (Generation/Consumption/Curtailment/
#   Emissions/Import/Export/Netflows TWh, DACCS demand) — out of scope.
#
# VALIDATION (spot-checks reproduced by scripts/fes-pathway/validate.py;
# published values from [RPT], derived values are the mapped sums above
# rounded to the report's 1 dp):
#   [RPT] citation                            published   derived
#   Table 3  p.45  2024 offshore wind         15.5        15.4527
#   Table 3  p.45  2024 onshore wind          14.6        14.5539
#   Table 3  p.45  2024 solar                 18.8        18.7885
#   Table 3  p.45  2024 nuclear                6.1         6.076
#   Table 3  p.45  2024 biomass/BECCS          4.3         4.2562
#   Table 3  p.45  2024 unabated gas          39.3        39.3121
#   Table 3  p.45  2024 batteries              6.8         6.7779
#   Table 3  p.45  2024 LDES                   2.8         2.7523
#   Table 24 p.130 offshore wind 2030/2050    46.5/104.4  46.4893/104.3734 (incl. non-networked)
#   Table 25 p.132 onshore wind 2030/2050     29.8/47.5   29.8173/47.4528
#   Table 26 p.134 solar 2030/2050            46.7/97.0   46.6529/97.0387
#   Table 27 p.136 tidal (marine) 2050         1.7         1.6644
#   Table 28 p.138 batteries 2030/2050        23.2/39.3   23.1693/39.3474
#   Table 29 p.140 LDES 2030/2050              5.3/16.5    5.3021/16.4981
#   Table 30 p.142 interconnectors 2030/2050  11.7/21.8   11.7/21.8
#   Table 31 p.144 nuclear 2030/2050           2.9/14.2    2.9/14.2
#   Table 32 p.146 low-carbon disp. 2030/2050  1.0/48.3    1.0047/48.2869 (CCS gas + hydrogen_turbine)
#   Table 33 p.148 unabated gas 2030/2050     31.2/0.0    31.1804/0.0
#   Table 34 p.150 BECCS 2030/2050             0.6/2.7     0.6/2.662
# validate.py additionally cross-checks ES1 sums against the independent
# FLX1 table (battery, pumped-hydro/LAES/CAES power & energy,
# interconnectors, hydrogen generation, unabated gas) and reconciles the
# grand total: every ES1 Holistic Transition capacity MW is either mapped
# into an entry below or listed as a documented exclusion (non-networked
# offshore wind only).
#
# REGENERATION: python scripts/fes-pathway/fetch.py <repo-root>
#               python scripts/fes-pathway/build.py <repo-root>
#               python scripts/fes-pathway/validate.py <repo-root>"""


if __name__ == "__main__":
    main()
