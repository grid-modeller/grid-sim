#!/usr/bin/env python3
"""Build the B6 two-zone evidence pack (processed) from the raw fetch.

Inputs: data/packs/b6/raw/ (fetch.py — sources and licences there and in
docs/notes/b6-two-zone-data-report.md). Outputs (data/packs/b6/processed/),
Parquet + CSV per docs/06 conventions, UTC where time-indexed:

  b6_da_flows_limits.{parquet,csv}
      SCOTEX (= boundary B6) day-ahead limit and forecast flow, MW,
      half-hourly `utc_start` index over the raw file's full span
      (rolling ~3.5 years; 2023-01-01 onward at this build). Positive
      flow = north-to-south (Scotland exporting). SEMANTICS — NESO's
      own wording vs our interpretation, kept distinct (review
      condition 3, docs/notes/b6-two-zone-data-review.md §5): NESO
      documents the flow as "the forecast position after Day Ahead
      energy scheduling", a "power flow forecast ... based on the next
      day's wind forecast, generation dispatch and demand forecast ...
      modelled using power system software" — it never says
      "unconstrained". This package INTERPRETS it as the
      pre-constraint-action (unconstrained) boundary flow; the reading
      is supported by the data itself (flow exceeds the limit in 23.6%
      of 2024 periods, impossible for a constrained/settled series)
      and was ruled sound at review, but it is an interpretation, not
      NESO's language. The flow is NOT settled outturn.
      CLOCK-CHANGE HANDLING (verified empirically on 2024): the raw
      "Date (GMT/BST)" labels are local wall-clock on a fixed 48-row
      daily grid — the short (spring) day carries 2 phantom rows
      labelled 01:00/01:30 which do not exist in local time (DROPPED,
      counted in the report), and the long (autumn) day carries the
      repeated 01:00/01:30 hour as duplicated labels (disambiguated
      first-occurrence = BST, second = GMT via tz_localize). Gaps in
      the raw file (whole missing days) are left as missing rows —
      NEVER filled; per-year period counts are in the report.

  b4_da_flows_limits.{parquet,csv}   (--three-zone)
      B4 (SSEN->SPT) day-ahead limit + forecast flow, stitched from the
      NESO version rows SSE-SP + SSE-SP2 — same semantics/handling as
      the B6 series; see build_b4_series. Emitted only under
      --three-zone; the b6 outputs above are untouched.

  boundary_thermal_costs_daily.{parquet,csv}
      Long format (settlement_date, constraint_group, cost_gbp): daily
      outturn thermal-constraint cost per boundary group, FY 2021-22
      onward. SCOTEX = B6; SSE-SP (B4) and SSHARN (B7) are the
      neighbouring boundaries needed to interpret B6-only results.

  constraint_breakdown_daily.{parquet,csv}
      Daily GB-wide constraint cost and volume by category (thermal /
      voltage / inertia / largest-loss), FY 2023-24 + 2024-25 — the
      only openly published constraint VOLUME series (not per
      boundary).

  repd_end2024_by_country_tech.csv
      REPD April-2026 extract filtered to Development Status (short) ==
      Operational AND Operational date <= 2024-12-31, installed MW
      summed by Country x Technology Type. Convention limitations,
      stated: (i) sites operational during 2024 but decommissioned/
      repowered before the April-2026 extract are missed; (ii) REPD
      covers projects >= 150 kW only (rooftop solar and legacy hydro
      sit in the DESNZ regional table instead); (iii) Operational rows
      with NO Operational date are EXCLUDED by the date filter and
      counted in the report (this build: 26 rows, 586 MW GB — England
      solar 296 + England battery 206 + Scotland onshore 67 + minor;
      <0.5 pp on any share).

  b4_fleet_split_3zone.csv   (--three-zone)
      The Scottish REPD fleet re-partitioned at N=710k into N-Scotland
      (SSEN) and S-Scotland (SPT); see build_fleet_split_3zone.

  desnz_regional_capacity_mw2024.csv
      England / Scotland / Wales / Northern Ireland rows x technology
      from the DESNZ Regional Renewable Statistics MW2024 sheet
      (all-size installed capacity, end-2024).

  b6_report.json
      Validation summary: per-year period counts and gap analysis for
      the B6 series, clock-change row accounting, 2024 limit/flow
      statistics, calendar-year costs per boundary group, and the
      derived Scotland capacity shares.

  b4_report.json   (--three-zone)
      Validation summary for the B4 series (stitch provenance, per-year
      counts, gaps, 2024 stats) and the 3-zone fleet split (per-tech
      capacity either side of B4, forth_tay within-cluster note,
      Cruachan sensitivity). Separate file — the b6 report is untouched.

Deterministic: pure function of the raw files. No network.

Usage:
    python build.py <repo-root>                 # B6 two-zone pack (committed)
    python build.py <repo-root> --three-zone    # ADDITIVE: B4 series + 3-zone
                                                 # fleet split + b4_report.json
"""

import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd


def build_b6_series(raw: Path, out: Path, report: dict) -> None:
    df = pd.read_csv(raw / "neso_day_ahead_constraint_flows_limits.csv")
    df.columns = ["group", "local_dt", "limit_mw", "flow_mw"]
    b6 = df[df.group == "SCOTEX"].copy()
    # Raw data-quality dedupe: the file occasionally repeats a whole day
    # verbatim (observed at this build: 2025-08-12, 48 value-identical
    # rows). Exact duplicates are dropped and counted; a duplicate label
    # with DIFFERENT values on a non-clock-change day would survive to
    # the duplicate-UTC check below and fail the build loudly.
    n_before = len(b6)
    b6 = b6.drop_duplicates(subset=["local_dt", "limit_mw", "flow_mw"])
    n_exact_dupes = n_before - len(b6)
    ts = pd.to_datetime(b6.local_dt, format="mixed")

    # Local wall-clock -> UTC. Nonexistent labels (the spring phantom
    # rows) -> NaT and are dropped; ambiguous labels (the autumn repeat)
    # are disambiguated first=BST (DST), second=GMT, which tz_localize
    # does with an explicit boolean mask built from duplication order.
    dup_rank = ts.groupby(ts).cumcount()
    ambiguous = dup_rank.values == 0  # first occurrence = still on BST
    localized = ts.dt.tz_localize(
        "Europe/London", ambiguous=ambiguous, nonexistent="NaT"
    )
    n_phantom = int(localized.isna().sum())
    b6["utc_start"] = localized.dt.tz_convert("UTC")
    b6 = b6.dropna(subset=["utc_start"]).sort_values("utc_start")
    dup_utc = int(b6.utc_start.duplicated().sum())
    if dup_utc:
        sys.exit(f"B6 series: {dup_utc} duplicate UTC periods after conversion")
    b6 = b6.set_index("utc_start")[["limit_mw", "flow_mw"]].astype("float64")

    b6.to_csv(out / "b6_da_flows_limits.csv",
              date_format="%Y-%m-%dT%H:%M:%SZ")
    b6.to_parquet(out / "b6_da_flows_limits.parquet")

    # Validation summary: per-year counts, gaps, 2024 statistics.
    per_year = {}
    for y, sub in b6.groupby(b6.index.year):
        y = int(y)
        # expected periods within the file's actual span for that year
        start = max(pd.Timestamp(f"{y}-01-01", tz="UTC"), b6.index.min())
        end = min(pd.Timestamp(f"{y + 1}-01-01", tz="UTC"),
                  b6.index.max() + pd.Timedelta(minutes=30))
        expected = int((end - start) / pd.Timedelta(minutes=30))
        per_year[str(y)] = {
            "periods": int(len(sub)),
            "expected_in_span": expected,
            "missing": expected - int(len(sub)),
            "limit_nan": int(sub.limit_mw.isna().sum()),
            "flow_nan": int(sub.flow_mw.isna().sum()),
        }
    y24 = b6[(b6.index >= "2024-01-01") & (b6.index < "2025-01-01")]
    l24 = y24.limit_mw.dropna()
    f24 = y24.flow_mw.dropna()
    both = y24.dropna()
    binding = (both.flow_mw >= 0.99 * both.limit_mw) & (both.limit_mw > 0) & (
        both.limit_mw < 9999
    )
    report["b6_da_flows_limits"] = {
        "span_utc": [str(b6.index.min()), str(b6.index.max())],
        "rows": int(len(b6)),
        "clock_changes": {
            "spring_phantom_rows_dropped": n_phantom,
            "autumn_ambiguous_disambiguated": "first=BST, second=GMT",
        },
        "exact_duplicate_raw_rows_dropped": n_exact_dupes,
        "per_year": per_year,
        "stats_2024": {
            "limit_mw_quantiles_1_5_25_50_75_95_99": [
                float(v) for v in np.percentile(l24, [1, 5, 25, 50, 75, 95, 99])
            ],
            "limit_sentinels": {
                "eq_0": int((l24 == 0).sum()),
                "ge_9999": int((l24 >= 9999).sum()),
            },
            "flow_mw_quantiles_1_5_25_50_75_95_99": [
                float(v) for v in np.percentile(f24, [1, 5, 25, 50, 75, 95, 99])
            ],
            "flow_negative_share": float((f24 < 0).mean()),
            "net_da_flow_twh": float((f24 * 0.5).sum() / 1e6),
            "share_periods_flow_ge_99pct_limit": float(binding.mean()),
        },
    }
    print(f"b6_da_flows_limits: {len(b6):,} rows "
          f"{b6.index.min()} -> {b6.index.max()}; "
          f"{n_phantom} spring phantom rows dropped")


def build_costs(raw: Path, out: Path, report: dict) -> None:
    frames = []
    for f in sorted(raw.glob("neso_thermal_constraint_costs_*.csv")):
        d = pd.read_csv(f)
        d.columns = ["settlement_date", "constraint_group", "cost_gbp"]
        # Some FY files parse the cost column as strings (thousands
        # separators absent but pandas' string dtype kicks in on mixed
        # inference) — normalise to float64 explicitly.
        d["cost_gbp"] = pd.to_numeric(
            d.cost_gbp.astype(str).str.replace(",", ""), errors="raise"
        ).astype("float64")
        frames.append(d)
    df = pd.concat(frames, ignore_index=True)
    df["settlement_date"] = pd.to_datetime(df.settlement_date)
    df = df.sort_values(["settlement_date", "constraint_group"])
    df.to_csv(out / "boundary_thermal_costs_daily.csv", index=False,
              date_format="%Y-%m-%d")
    df.to_parquet(out / "boundary_thermal_costs_daily.parquet", index=False)

    by_year = (
        df.assign(year=df.settlement_date.dt.year)
        .groupby(["year", "constraint_group"]).cost_gbp.sum()
        .unstack().round(0)
    )
    report["boundary_thermal_costs"] = {
        "span": [str(df.settlement_date.min().date()),
                 str(df.settlement_date.max().date())],
        "rows": int(len(df)),
        "calendar_year_totals_gbp": json.loads(by_year.to_json()),
    }
    print("boundary_thermal_costs_daily:", len(df), "rows; calendar-2024 "
          "SCOTEX GBP m:",
          round(by_year.loc[2024, "SCOTEX"] / 1e6, 1))


def build_breakdown(raw: Path, out: Path, report: dict) -> None:
    frames = []
    for f in sorted(raw.glob("neso_constraint_breakdown_*.csv")):
        d = pd.read_csv(f)
        d = d.rename(columns={d.columns[0]: "settlement_date"})
        frames.append(d)
    df = pd.concat(frames, ignore_index=True)
    df["settlement_date"] = pd.to_datetime(df.settlement_date)
    df = df.sort_values("settlement_date")
    df.to_csv(out / "constraint_breakdown_daily.csv", index=False,
              date_format="%Y-%m-%d")
    df.to_parquet(out / "constraint_breakdown_daily.parquet", index=False)
    report["constraint_breakdown"] = {
        "span": [str(df.settlement_date.min().date()),
                 str(df.settlement_date.max().date())],
        "rows": int(len(df)),
        "columns": [c for c in df.columns if c != "settlement_date"],
    }
    print("constraint_breakdown_daily:", len(df), "rows")


def build_repd(raw: Path, out: Path, report: dict) -> None:
    df = pd.read_csv(raw / "repd_q1_2026.csv", encoding="latin1",
                     low_memory=False)
    df["cap_mw"] = pd.to_numeric(df["Installed Capacity (MWelec)"],
                                 errors="coerce")
    df["op_date"] = pd.to_datetime(df["Operational"], format="%d/%m/%Y",
                                   errors="coerce")
    df["country"] = df["Country"].str.strip()
    op_status = df[df["Development Status (short)"] == "Operational"]
    op = op_status[op_status.op_date <= "2024-12-31"]
    undated = op_status[op_status.op_date.isna()]
    t = (op.groupby(["Technology Type", "country"]).cap_mw.sum()
         .unstack(fill_value=0.0).round(3))
    t.to_csv(out / "repd_end2024_by_country_tech.csv")
    gb = t.reindex(columns=["England", "Scotland", "Wales"]).fillna(0)
    shares = (gb["Scotland"] / gb.sum(axis=1)).round(4)
    report["repd_end2024"] = {
        "rows_used": int(len(op)),
        "undated_operational_rows_excluded": {
            "rows": int(len(undated)),
            "gb_mw": round(float(
                undated[undated.country.isin(
                    ["England", "Scotland", "Wales"])].cap_mw.sum()), 1),
            "uk_mw": round(float(undated.cap_mw.sum()), 1),
        },
        "scotland_share_of_gb": {
            k: float(v)
            for k, v in shares.items()
            if k in ("Wind Onshore", "Wind Offshore", "Solar Photovoltaics",
                     "Battery", "Large Hydro", "Small Hydro",
                     "Pumped Storage Hydroelectricity", "Biomass (dedicated)")
        },
    }
    print("repd_end2024_by_country_tech: Scotland shares",
          report["repd_end2024"]["scotland_share_of_gb"])


def build_desnz(raw: Path, out: Path, report: dict) -> None:
    df = pd.read_excel(raw / "desnz_regional_capacity_2003_2024.xlsx",
                       sheet_name="MW2024", header=6)
    df = df.rename(columns={df.columns[0]: "region"})
    rows = df[df.region.isin(["England", "Scotland", "Wales",
                              "Northern Ireland", "Other Sites [note 5]"])]
    rows.to_csv(out / "desnz_regional_capacity_mw2024.csv", index=False)
    num = rows.set_index("region").apply(pd.to_numeric, errors="coerce")
    gb_rows = ["England", "Scotland", "Wales"]
    shares = {}
    for col in ("Onshore Wind", "Offshore Wind [note 4]", "Solar PV", "Hydro"):
        gb_total = num.loc[gb_rows, col].sum()
        shares[col] = round(float(num.loc["Scotland", col] / gb_total), 4)
    report["desnz_mw2024"] = {
        "scotland_share_of_gb": shares,
        "gb_totals_mw": {
            col: round(float(num.loc[gb_rows, col].sum()), 1)
            for col in ("Onshore Wind", "Offshore Wind [note 4]",
                        "Solar PV", "Hydro")
        },
    }
    print("desnz_regional_capacity_mw2024: Scotland shares", shares)


def build_b4_series(raw: Path, out: Path, report: dict) -> None:
    """Stitch the B4 (SSEN->SPT) day-ahead limit+flow series from the two
    NESO version rows SSE-SP (2023-01-01 -> 2024-04-20) and SSE-SP2
    (2024-04-21 -> present), which NESO versions mid-year with ZERO overlap
    (verified this build). Semantics, clock-change handling, sentinel and
    dedupe treatment are BYTE-FOR-BYTE identical to build_b6_series (the
    review holds the B4 stitch to the B6 builder's handling) — B4 differs
    from B6 only in which constraint-group rows are selected. Positive flow
    = north-to-south (N-Scotland exporting to S-Scotland). This is the same
    pre-constraint-action DA-forecast interpretation as B6; it is NOT
    settled outturn, and — unlike B6 — B4 is an INTERNAL Scottish flow with
    NO annual-outturn cross-anchor (design-review item 4): its net-flow
    magnitude carries the 'DA-only' caveat and a wedge budget, never a tight
    validated magnitude."""
    df = pd.read_csv(raw / "neso_day_ahead_constraint_flows_limits.csv")
    df.columns = ["group", "local_dt", "limit_mw", "flow_mw"]
    b4 = df[df.group.isin(["SSE-SP", "SSE-SP2"])].copy()
    n_before = len(b4)
    b4 = b4.drop_duplicates(subset=["local_dt", "limit_mw", "flow_mw"])
    n_exact_dupes = n_before - len(b4)
    ts = pd.to_datetime(b4.local_dt, format="mixed")
    dup_rank = ts.groupby(ts).cumcount()
    ambiguous = dup_rank.values == 0  # first occurrence = still on BST
    localized = ts.dt.tz_localize(
        "Europe/London", ambiguous=ambiguous, nonexistent="NaT"
    )
    n_phantom = int(localized.isna().sum())
    b4["utc_start"] = localized.dt.tz_convert("UTC")
    b4 = b4.dropna(subset=["utc_start"]).sort_values("utc_start")
    dup_utc = int(b4.utc_start.duplicated().sum())
    if dup_utc:
        sys.exit(f"B4 series: {dup_utc} duplicate UTC periods after conversion "
                 "(SSE-SP / SSE-SP2 overlap — the stitch is not clean)")
    b4 = b4.set_index("utc_start")[["limit_mw", "flow_mw"]].astype("float64")

    b4.to_csv(out / "b4_da_flows_limits.csv",
              date_format="%Y-%m-%dT%H:%M:%SZ")
    b4.to_parquet(out / "b4_da_flows_limits.parquet")

    per_year = {}
    for y, sub in b4.groupby(b4.index.year):
        y = int(y)
        start = max(pd.Timestamp(f"{y}-01-01", tz="UTC"), b4.index.min())
        end = min(pd.Timestamp(f"{y + 1}-01-01", tz="UTC"),
                  b4.index.max() + pd.Timedelta(minutes=30))
        expected = int((end - start) / pd.Timedelta(minutes=30))
        per_year[str(y)] = {
            "periods": int(len(sub)),
            "expected_in_span": expected,
            "missing": expected - int(len(sub)),
            "limit_nan": int(sub.limit_mw.isna().sum()),
            "flow_nan": int(sub.flow_mw.isna().sum()),
        }
    y24 = b4[(b4.index >= "2024-01-01") & (b4.index < "2025-01-01")]
    l24 = y24.limit_mw.dropna()
    f24 = y24.flow_mw.dropna()
    both = y24.dropna()
    binding = (both.flow_mw >= 0.99 * both.limit_mw) & (both.limit_mw > 0) & (
        both.limit_mw < 9999
    )
    # Stitch provenance: which version row supplies each 2024 period.
    src = df[df.group.isin(["SSE-SP", "SSE-SP2"])].copy()
    src["dt"] = pd.to_datetime(src.local_dt, format="mixed")
    src24 = src[(src.dt >= "2024-01-01") & (src.dt < "2025-01-01")]
    report["b4_da_flows_limits"] = {
        "stitched_from": ["SSE-SP", "SSE-SP2"],
        "stitch_boundary_local": "SSE-SP end 2024-04-20 23:30; "
                                 "SSE-SP2 start 2024-04-21 00:00 (zero overlap)",
        "stitch_source_rows_2024": {
            "SSE-SP": int((src24.group == "SSE-SP").sum()),
            "SSE-SP2": int((src24.group == "SSE-SP2").sum()),
        },
        "span_utc": [str(b4.index.min()), str(b4.index.max())],
        "rows": int(len(b4)),
        "clock_changes": {
            "spring_phantom_rows_dropped": n_phantom,
            "autumn_ambiguous_disambiguated": "first=BST, second=GMT",
        },
        "exact_duplicate_raw_rows_dropped": n_exact_dupes,
        "per_year": per_year,
        "stats_2024": {
            "limit_mw_quantiles_1_5_25_50_75_95_99": [
                float(v) for v in np.percentile(l24, [1, 5, 25, 50, 75, 95, 99])
            ],
            "limit_sentinels": {
                "eq_0": int((l24 == 0).sum()),
                "ge_9999": int((l24 >= 9999).sum()),
            },
            "flow_mw_quantiles_1_5_25_50_75_95_99": [
                float(v) for v in np.percentile(f24, [1, 5, 25, 50, 75, 95, 99])
            ],
            "flow_negative_share": float((f24 < 0).mean()),
            "net_da_flow_twh": float((f24 * 0.5).sum() / 1e6),
            "share_periods_flow_ge_99pct_limit": float(binding.mean()),
        },
    }
    s24 = report["b4_da_flows_limits"]["stats_2024"]
    print(f"b4_da_flows_limits: {len(b4):,} rows "
          f"{b4.index.min()} -> {b4.index.max()}; 2024 "
          f"{per_year['2024']['periods']} periods, median limit "
          f"{np.percentile(l24, 50):.0f} MW, net {s24['net_da_flow_twh']:.2f} "
          f"TWh, binding {s24['share_periods_flow_ge_99pct_limit']:.1%}")


def build_fleet_split_3zone(raw: Path, out: Path, report: dict) -> None:
    """Re-partition the committed Scottish fleet (REPD Country==Scotland,
    Operational <=2024-12-31 — the same filter as build_repd) at the B4 line
    (N=710k OSGB northing) into N-Scotland (>=710k, SSEN) and S-Scotland
    (710k..border, SPT). Emits per-technology capacity either side, the
    forth_tay offshore within-cluster split, and the Cruachan pumped-storage
    N<->S sensitivity (design-review items 3, 5, 6). PINNED from site
    northings — cited evidence, NOT tuned to the B4 DA series (item 3
    guard)."""
    N = 710000
    df = pd.read_csv(raw / "repd_q1_2026.csv", encoding="latin1",
                     low_memory=False)
    df["cap_mw"] = pd.to_numeric(df["Installed Capacity (MWelec)"],
                                 errors="coerce")
    df["op_date"] = pd.to_datetime(df["Operational"], format="%d/%m/%Y",
                                   errors="coerce")
    df["Y"] = pd.to_numeric(df["Y-coordinate"], errors="coerce")
    df["country"] = df["Country"].str.strip()
    op = df[(df["Development Status (short)"] == "Operational")
            & (df.op_date <= "2024-12-31")]
    sco = op[op.country == "Scotland"].copy()
    n_missing_y = int(sco.Y.isna().sum())

    techs = ["Wind Onshore", "Wind Offshore", "Solar Photovoltaics", "Battery",
             "Large Hydro", "Small Hydro", "Pumped Storage Hydroelectricity",
             "Biomass (dedicated)"]
    rows = []
    split = {}
    for tech in techs:
        t = sco[sco["Technology Type"] == tech]
        n = round(float(t[t.Y >= N].cap_mw.sum()), 1)
        s = round(float(t[t.Y < N].cap_mw.sum()), 1)
        tot = round(n + s, 1)
        rows.append({"technology": tech, "n_scotland_mw": n,
                     "s_scotland_mw": s, "scotland_total_mw": tot,
                     "n_share": round(n / tot, 4) if tot else 0.0})
        split[tech] = {"n_mw": n, "s_mw": s,
                       "n_share": round(n / tot, 4) if tot else 0.0}
    pd.DataFrame(rows).to_csv(out / "b4_fleet_split_3zone.csv", index=False)

    # forth_tay offshore within-cluster split (design-review items 5, 6a/6b):
    # the CF cluster forth_tay straddles B4. Report its operational members
    # either side of 710k. NnG (450 MW, Firth of Forth, full CoD 2025-07)
    # excluded from the end-2024 fleet — carried as a forward wedge.
    off = sco[sco["Technology Type"] == "Wind Offshore"].copy()
    off_sites = [{"site": r["Site Name"], "y": round(float(r.Y), 0),
                  "cap_mw": round(float(r.cap_mw), 1),
                  "side": "N" if r.Y >= N else "S"}
                 for _, r in off.sort_values("Y", ascending=False).iterrows()]

    # Cruachan sensitivity (design-review items 3/5, Edit 3): Cruachan
    # (Argyll fringe, ~18.7k north of the line) placement feeds the storage
    # headline. Report N-Scotland vs S-Scotland pumped-storage both ways.
    ps = sco[sco["Technology Type"] == "Pumped Storage Hydroelectricity"]
    ps_sites = {r["Site Name"]: {"y": round(float(r.Y), 0),
                                 "cap_mw": round(float(r.cap_mw), 1)}
                for _, r in ps.iterrows()}

    report["fleet_split_3zone"] = {
        "b4_line_northing": N,
        "repd_filter": "Operational, op_date <= 2024-12-31, Country==Scotland",
        "scotland_operational_sites": int(len(sco)),
        "sites_missing_northing": n_missing_y,
        "per_technology_mw": split,
        "offshore_sites": off_sites,
        "forth_tay_within_cluster_note": (
            "Scottish-cluster offshore south of 710k = Levenmouth (7 MW). "
            "NnG 450 MW (Firth of Forth, south) excluded — full CoD 2025-07, "
            "not in the end-2024 fleet; carried as a forward wedge "
            "(S-Scotland offshore -> ~0.46 GW when included). Robin Rigg "
            "(174 MW, Scottish waters south) sits in the irish_sea CF "
            "cluster -> rgb, so it is NOT S-Scotland in the CF split."
        ),
        "cruachan_sensitivity": {
            "pinned_cruachan_in": "N-Scotland (Y=728674, ~18.7k north of 710k)",
            "sites": ps_sites,
            "cruachan_in_N": {
                "n_scotland_ps_mw": round(
                    sum(v["cap_mw"] for v in ps_sites.values()), 1),
                "s_scotland_ps_mw": 0.0},
            "cruachan_in_S": {
                "n_scotland_ps_mw": round(sum(
                    v["cap_mw"] for k, v in ps_sites.items()
                    if k != "Cruachan"), 1),
                "s_scotland_ps_mw": round(sum(
                    v["cap_mw"] for k, v in ps_sites.items()
                    if k == "Cruachan"), 1)},
        },
    }
    print("b4_fleet_split_3zone: onshore N/S "
          f"{split['Wind Onshore']['n_mw']}/{split['Wind Onshore']['s_mw']} MW "
          f"(N share {split['Wind Onshore']['n_share']}); offshore N/S "
          f"{split['Wind Offshore']['n_mw']}/{split['Wind Offshore']['s_mw']} MW")


def main() -> None:
    args = sys.argv[1:]
    three_zone = "--three-zone" in args
    args = [a for a in args if a != "--three-zone"]
    if len(args) != 1:
        sys.exit("usage: python build.py <repo-root> [--three-zone]")
    pack = Path(args[0]) / "data" / "packs" / "b6"
    raw, out = pack / "raw", pack / "processed"
    out.mkdir(parents=True, exist_ok=True)

    if three_zone:
        # ADDITIVE ONLY: build the B4 series + 3-zone fleet split into a
        # SEPARATE b4_report.json. Rewrites NO committed b6 output; the b6
        # manifest stays byte-valid.
        report: dict = {}
        build_b4_series(raw, out, report)
        build_fleet_split_3zone(raw, out, report)
        (out / "b4_report.json").write_text(
            json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
        )
        print(f"report -> {out / 'b4_report.json'}")
        return

    report = {}
    build_b6_series(raw, out, report)
    build_costs(raw, out, report)
    build_breakdown(raw, out, report)
    build_repd(raw, out, report)
    build_desnz(raw, out, report)
    (out / "b6_report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
    )
    print(f"report -> {out / 'b6_report.json'}")


if __name__ == "__main__":
    main()
