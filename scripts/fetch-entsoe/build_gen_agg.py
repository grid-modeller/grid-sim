#!/usr/bin/env python3
"""Build the per-zone 2024 generation aggregation table (EU CF anchors).

Added 2026-07-03 for the Stage 5 external-zone CF derivation
(scripts/era5-cf/derive_cf_eu.py): the calibration needs 2024 actual
annual (and monthly) energy per production type per neighbour zone, which
the original pack fetched only for NO2/NO. fetch.py now also fetches
A75/A16 for fr/be/nl/delu/dk1/ie; this script parses those documents and
writes a compact aggregation table:

    data/packs/entsoe-2024/processed/aggregation_gen_2024.{parquet,csv}
        rows: (zone, series) — series is the PSR stem. Consumption
              TimeSeries (outBiddingZone pumping/auxiliary loads) are
              EXCLUDED: sparse/gap-ridden on the platform and irrelevant
              to the CF anchor (skips counted in the report)
        cols: psr_code, technology (clean scenario mapping or empty),
              gen_gwh (2024 annual), unfilled_slots (residual gaps;
              always 0 for wind/solar anchor series, enforced),
              gen_gwh_m01..m12 (monthly)
    data/packs/entsoe-2024/processed/aggregation_gen_report_2024.json
        per-series gap/repair record (same rules and honesty as build.py)

Deterministic, no network. Existing processed files are NOT rebuilt or
touched — the committed entsoe-2024.sha256 entries for them stay valid;
the three new files are appended to the manifest in the same change
that adds this script.

Assembly rules (build.py's, reused by import, not reimplemented):
- Each Period normalised to the 30-min UTC grid (PT15M mean / PT60M
  repeat), curveType read per TimeSeries (A03 hold-forward observed
  everywhere in 2024).
- A month whose document omits a PSR series entirely means "nothing
  reported" (the platform drops empty series) -> zeros, recorded.
- Internal gaps <= 2 h linearly interpolated (combine_pieces), longer
  gaps filled with the mean of the same half-hour one day earlier/later
  (fill_day_offset — the documented IE-SEM load repair rule; at annual-
  energy level the effect is negligible either way, and every filled
  timestamp is counted in the report). Residual NaN in a wind/solar
  anchor series (B16/B18/B19) fails the build; other series keep their
  gaps, sum over reported slots only, and carry the gap count.

Licence note (docs/notes/entsoe-stage5-pack-report.md §1): actual
generation per type (Art. 16.1) is NOT on the ENTSO-E CC-BY free-re-use
list. Use here is the clause-3.1 case — an internal calibration anchor,
fetched and built locally, never redistributed (pack is git-ignored;
only checksums committed). Attribution on anything derived/published:
"Source: ENTSO-E Transparency Platform".

Usage: python build_gen_agg.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build import (  # noqa: E402  (pinned reuse of the pack's assembly rules)
    GRID_30M,
    MONTHS,
    PSR,
    combine_pieces,
    fill_day_offset,
    parse_doc,
    period_to_30m,
    write,
)

AGG_ZONES = ["fr", "be", "nl", "delu", "dk1", "ie"]

# The CF-calibration anchor series: any residual gap in these fails the
# build (a gap would silently bias a calibration factor).
ANCHOR_PSR = {"B16", "B18", "B19"}  # solar, wind_offshore, wind_onshore


def assemble_zone(raw: Path, zone: str, report: dict) -> pd.DataFrame:
    """gen_<zone>_<ym>.xml (12 months) -> 30-min frame, one column per
    PSR series. Mirrors build.build_generation's assembly exactly."""
    pieces_by_col: dict[str, list] = {}
    res_by_col: dict[str, dict] = {}
    psr_by_col: dict[str, str] = {}
    units: set = set()
    months_by_col: dict[str, set] = {}
    for ym in MONTHS:
        is_ack, tss = parse_doc(raw / f"gen_{zone}_{ym}.xml")
        if is_ack:
            report[f"gen_{zone}_{ym}_ack_error"] = {"ack_months_error": [ym]}
            continue
        for ts in tss:
            if not ts["in_dom"]:
                # Consumption TimeSeries (outBiddingZone; pumping/auxiliary
                # loads). Sparse and gap-ridden on the platform (e.g. FR
                # fossil_hard_coal consumption has 1,139 unfillable slots)
                # and irrelevant to the CF calibration anchor -> excluded
                # from this table, counted here.
                key = f"gen_{zone}_consumption_series_skipped"
                report.setdefault(key, []).append(
                    f"{PSR.get(ts['psr'], (ts['psr'], ''))[0]} ({ts['psr']})"
                )
                continue
            stem, _ = PSR.get(ts["psr"], (ts["psr"], ""))
            col = stem
            psr_by_col[col] = ts["psr"]
            units.add(ts["unit"])
            months_by_col.setdefault(col, set()).add(ym)
            for start, end, res, pts in ts["periods"]:
                r = res_by_col.setdefault(col, {})
                r[res] = r.get(res, 0) + 1
                pieces_by_col.setdefault(col, []).append(
                    period_to_30m(start, end, res, pts, ts["curve"])
                )
    cols = {}
    residual_by_col: dict[str, int] = {}
    for col, pieces in sorted(pieces_by_col.items()):
        key = f"gen_{zone}_{col}"
        absent = [ym for ym in MONTHS if ym not in months_by_col[col]]
        if absent and psr_by_col[col] in ANCHOR_PSR:
            # An anchor series missing whole months is not "zero
            # generation" — it is a series the TSO only started (or
            # stopped) publishing mid-year (observed: IE-SEM solar B16
            # first appears 2024-11-13). Zero-filling it would silently
            # corrupt the annual calibration anchor -> the series is
            # EXCLUDED from the table and recorded; the CF derivation
            # must treat that zone/technology as having no anchor.
            report[key + "_anchor_excluded"] = {
                "reason": "anchor series absent in whole months",
                "months_present": sorted(months_by_col[col]),
            }
            continue
        s = combine_pieces(pieces, res_by_col[col], units, report, key)
        for ym in absent:
            m0 = pd.Timestamp(f"{ym}-01T00:00:00Z")
            m1 = (m0 + pd.Timedelta(days=32)).replace(day=1)
            blk = s.loc[(s.index >= m0) & (s.index < m1)]
            s.loc[blk.index[blk.isna()]] = 0.0
        if absent:
            report[key]["zero_filled_absent_months"] = absent
            report[key]["unfilled_slots"] = int(s.isna().sum())
        if int(s.isna().sum()):
            s = fill_day_offset(s, report, key)
            # A short hole can become interpolatable only once day-offset
            # has filled its surroundings (observed: 2 slots, IE wind,
            # 2024-03-03 04:00/04:30) -> re-apply the standard <=2 h
            # linear rule once, counted.
            before = int(s.isna().sum())
            s = s.interpolate(method="linear", limit=4, limit_area="inside")
            report[key]["post_day_offset_interpolated_slots"] = before - int(
                s.isna().sum()
            )
            report[key]["unfilled_slots"] = int(s.isna().sum())
        residual = int(s.isna().sum())
        if residual and psr_by_col[col] in ANCHOR_PSR:
            # The wind/solar anchor series MUST be complete: a gap here
            # would bias the calibration factor. Fail, do not absorb.
            raise RuntimeError(
                f"{key}: {residual} anchor slots unfilled after repairs"
            )
        # Non-anchor series (e.g. FR hydro_pumped: 1,324 slots of platform
        # publication gaps survive the repairs) stay in the table for the
        # reconciliation context, with energy summed over REPORTED slots
        # only and the residual gap count carried as a column — coverage
        # is visible, nothing is invented.
        residual_by_col[col] = residual
        cols[col] = s
    df = pd.DataFrame(cols, index=GRID_30M)
    df.attrs["psr_by_col"] = psr_by_col
    df.attrs["residual_by_col"] = residual_by_col
    return df


def main() -> None:
    repo = Path(sys.argv[1])
    raw = repo / "data" / "packs" / "entsoe-2024" / "raw"
    out = repo / "data" / "packs" / "entsoe-2024" / "processed"
    report: dict = {}

    rows = []
    for zone in AGG_ZONES:
        df = assemble_zone(raw, zone, report)
        psr_by_col = df.attrs["psr_by_col"]
        monthly = df.resample("MS").sum() * 0.5 / 1e3  # avg-MW slots -> GWh
        annual = df.sum() * 0.5 / 1e3  # pandas sum skips NaN (reported slots)
        for col in df.columns:
            psr = psr_by_col[col]
            row = {
                "zone": zone,
                "series": col,
                "psr_code": psr,
                "technology": PSR.get(psr, (psr, ""))[1],
                "gen_gwh": round(float(annual[col]), 3),
                "unfilled_slots": df.attrs["residual_by_col"][col],
            }
            for m in range(1, 13):
                row[f"gen_gwh_m{m:02d}"] = round(float(monthly[col].iloc[m - 1]), 3)
            rows.append(row)
        wind_solar = {
            c: round(float(annual[c]), 1)
            for c in ("wind_onshore", "wind_offshore", "solar")
            if c in annual.index
        }
        print(f"{zone}: {len(df.columns)} series; wind/solar GWh {wind_solar}")

    table = pd.DataFrame(rows).sort_values(["zone", "psr_code", "series"])
    table = table.set_index("zone")
    write(table, out, "aggregation_gen_2024")
    (out / "aggregation_gen_report_2024.json").write_text(
        json.dumps(report, indent=2, sort_keys=True)
    )
    print("written: aggregation_gen_2024.{parquet,csv} + report json")


if __name__ == "__main__":
    main()
