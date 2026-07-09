#!/usr/bin/env python3
"""Build the processed CBS 2024 NL anchor table from the raw OData JSON.

Raw -> processed, no network. Extracts the NL 2024 values the EU CF
recalibration consumes (derive_cf_eu.py / validate_cf_eu.py) into one
small table, every row carrying its StatLine table id, measure code and
CBS period status ("NaderVoorlopig" = revised provisional for 2024 —
provisional national statistics still beat a known-biased A75 anchor,
but the status is recorded, never hidden).

Outputs (data/packs/cbs-2024/processed/, both formats per docs/06):
    cbs_2024_nl_anchors.parquet / .csv
        columns: table_id, series, measure_code, measure, value, unit,
                 period, status
    cbs_build_report_2024.json
        row count, per-table source files, the anchor values echoed.

Units: CBS "mln kWh" == GWh (used as-is); capacity MW as-is; 85005NED
panel capacity kWp and inverter capacity kW are converted to MW
(value / 1e3) with the unit recorded as MW_dc / MW_ac.

Deterministic: pure function of the raw JSON; rows written in the fixed
order below. Licence: CC BY 4.0, "Source: CBS (Statistics Netherlands)".

Usage: python build.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

# (raw stem, dimension field, dimension code, series label,
#  measure code, measure label, unit, scale)
ROWS = [
    # 82610NED — production (mln kWh == GWh) + end-of-year capacity (MW).
    ("82610NED_observations_2024", "BronTechniek", "E006637", "wind_onshore",
     "M002264_1", "gross_generation", "GWh", 1.0),
    ("82610NED_observations_2024", "BronTechniek", "E006637", "wind_onshore",
     "M002417_1", "net_generation", "GWh", 1.0),
    ("82610NED_observations_2024", "BronTechniek", "E006637", "wind_onshore",
     "M002163", "capacity_end_year", "MW", 1.0),
    ("82610NED_observations_2024", "BronTechniek", "E006638", "wind_offshore",
     "M002417_1", "net_generation", "GWh", 1.0),
    ("82610NED_observations_2024", "BronTechniek", "E006638", "wind_offshore",
     "M002163", "capacity_end_year", "MW", 1.0),
    ("82610NED_observations_2024", "BronTechniek", "E006590", "solar",
     "M002264_1", "generation", "GWh", 1.0),
    ("82610NED_observations_2024", "BronTechniek", "E006590", "solar",
     "M002163", "capacity_end_year", "MW", 1.0),
    # 85005NED — solar capacity conventions + the sector split that sums
    # to the 82610NED solar total (kWp/kW -> MW).
    ("85005NED_observations_2024", "SectorEnVermogensklasse", "E007161",
     "solar_all_sectors", "M002461", "panel_capacity_end_year", "MW_dc", 1e-3),
    ("85005NED_observations_2024", "SectorEnVermogensklasse", "E007161",
     "solar_all_sectors", "M008184", "inverter_capacity_end_year", "MW_ac", 1e-3),
    ("85005NED_observations_2024", "SectorEnVermogensklasse", "E007161",
     "solar_all_sectors", "M007785", "generation", "GWh", 1.0),
    ("85005NED_observations_2024", "SectorEnVermogensklasse", "E007037",
     "solar_dwellings", "M007785", "generation", "GWh", 1.0),
    ("85005NED_observations_2024", "SectorEnVermogensklasse", "T001081",
     "solar_economic_activities", "M007785", "generation", "GWh", 1.0),
]


def load_values(raw: Path, stem: str) -> list[dict]:
    return json.loads((raw / f"{stem}.json").read_text())["value"]


def period_status(raw: Path, stem: str) -> str:
    rows = load_values(raw, stem)
    if len(rows) != 1 or rows[0]["Identifier"] != "2024JJ00":
        sys.exit(f"{stem}: expected exactly the 2024JJ00 period row")
    return rows[0]["Status"]


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: build.py <repo-root>")
    pack = Path(sys.argv[1]) / "data" / "packs" / "cbs-2024"
    raw, proc = pack / "raw", pack / "processed"
    proc.mkdir(parents=True, exist_ok=True)

    status = {
        "82610NED": period_status(raw, "82610NED_perioden_2024"),
        "85005NED": period_status(raw, "85005NED_perioden_2024"),
    }
    obs = {
        stem: load_values(raw, stem)
        for stem in ("82610NED_observations_2024", "85005NED_observations_2024")
    }

    out = []
    for stem, dim, code, series, mcode, measure, unit, scale in ROWS:
        table = stem.split("_")[0]
        hits = [
            o for o in obs[stem]
            if o[dim] == code and o["Measure"] == mcode
            and o["Perioden"] == "2024JJ00"
        ]
        if len(hits) != 1 or hits[0]["Value"] is None:
            sys.exit(f"{table} {series} {mcode}: expected exactly one value")
        out.append({
            "table_id": table,
            "series": series,
            "measure_code": mcode,
            "measure": measure,
            "value": float(hits[0]["Value"]) * scale,
            "unit": unit,
            "period": "2024",
            "status": status[table],
        })

    df = pd.DataFrame(out)
    # Internal consistency: the 85005NED sector split must sum to the
    # national solar total, and both tables must agree on that total.
    split = df[df.series.isin(["solar_dwellings", "solar_economic_activities"])]
    total_85005 = float(
        df[(df.series == "solar_all_sectors") & (df.measure == "generation")]
        ["value"].iloc[0]
    )
    total_82610 = float(
        df[(df.series == "solar") & (df.measure == "generation")]["value"].iloc[0]
    )
    if abs(float(split["value"].sum()) - total_85005) > 1e-9:
        sys.exit("85005NED sector split does not sum to the national total")
    if abs(total_85005 - total_82610) > 1e-9:
        sys.exit("82610NED and 85005NED disagree on national solar generation")

    df.to_csv(proc / "cbs_2024_nl_anchors.csv", index=False)
    df.to_parquet(proc / "cbs_2024_nl_anchors.parquet", index=False)
    report = {
        "rows": len(df),
        "period_status": status,
        "retrieved": "2026-07-03",
        "licence": "CC BY 4.0 (https://www.cbs.nl/en-gb/about-us/website/copyright)",
        "attribution": "Source: CBS (Statistics Netherlands), StatLine 82610NED + 85005NED",
        "anchors": {
            r["series"] + ":" + r["measure"]: r["value"] for r in out
        },
    }
    (proc / "cbs_build_report_2024.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n"
    )
    print(df.to_string(index=False))
    print(f"\nprocessed -> {proc} ({len(df)} rows; status {status})")


if __name__ == "__main__":
    main()
