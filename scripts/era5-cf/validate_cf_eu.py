#!/usr/bin/env python3
"""Validate the EU weather pack geometry and the derived EU CF/t2m traces.

Two responsibilities (exit non-zero on any failure):

A. EU PACK GEOMETRY — the committed validator that
   docs/notes/eu-pack-box-review.md note 3 obliges ("when Stage 5 derives
   EU CF traces, its validator should re-assert pack geometry the way
   validate_multiyear.py does for GB"). Over data/packs/era5-eu/:
   1. Exactly 480 monthly files (40 years x 12), none extra, none missing.
   2. Per file: rows = calendar hours x 13,189 cells; the grid is the
      full 121 x 109 quarter-degree lattice of the eu box (42-72N,
      11W-16E); no NaNs in u100/v100/ssrd/t2m; timestamps span exactly
      the calendar month, hourly.
   3. Per year: 8,760 hours (8,784 leap); 350,640 hours total.
   (Checksum verification against era5-eu-1985-2024.sha256 is a separate,
   pure-shasum concern: `cd data/packs && shasum -a 256 -c
   era5-eu-1985-2024.sha256`. This validator asserts the DATA geometry.)

B. DERIVED TRACES — data/packs/cf-eu/<country>/, the GB multi-year checks
   (validate_multiyear.py) applied to the EU families:
   1. Complete families 1985-2024: fr/be/nl/de/dk1 x {onshore, offshore,
      solar}, ie x {onshore, solar}, t2m for those six + no2.
   2. Per file: 17,520 periods (17,568 leap), strictly uniform 30-min UTC
      index, no duplicates, Jan 1 00:00Z .. Dec 31 23:30Z (UTC-clean
      through all clock changes by construction — asserted, not assumed);
      utc_start stored as timestamp[us, tz=UTC]; single float64 column
      (`cf` in [0, 1]; `t2m_c` in [-40, 45] C — beyond ERA5-plausible
      population-weighted extremes for these countries); no NaNs; CSV
      twin exists and matches the Parquet to 1e-9.
   3. Cross-year continuity per family (the engine's multi-file concat
      loader contract: last(Y) + 30 min == first(Y+1)).
   4. Calibration reproduction: for every series the derivation report
      (eu_cf_report.json) marks calibrated, the 2024 trace's annual mean
      CF must reproduce the anchor target gen_gwh / (capacity_mw x
      8,784 h) to 1e-6 relative — i.e. trace x paired capacity
      reproduces observed 2024 energy. The anchor is ENTSO-E A75/A68,
      EXCEPT NL onshore and NL solar (derive_cf_eu docstring deviation
      3a, the 2026-07-03 CBS recalibration): those reproduce the CBS
      national-statistics anchors read from data/packs/cbs-2024/
      (onshore net 17,657 GWh / 6,955 MW; solar 21,822 GWh /
      27,979.732 MWp DC). Applied factors in the report must equal
      derive_cf_eu.PINNED_FACTORS_EU (4 dp), and series the report
      marks uncalibrated must carry applied factor 1.0.

Deterministic, no network.
Usage:
    python validate_cf_eu.py <repo-root>
    python validate_cf_eu.py <repo-root> --traces-only   # skip the 51 GB
        pack scan (quick re-runs during trace work; a delivery run must
        be full)
"""

import argparse
import calendar
import json
import sys
from pathlib import Path

import pandas as pd
import pyarrow.parquet as pq

sys.path.insert(0, str(Path(__file__).resolve().parent))
import derive_cf_eu as eu  # noqa: E402  (families, pinned factors)

YEARS = range(1985, 2025)
VARS = ["u100", "v100", "ssrd", "t2m"]
N_CELLS = 13_189
N_LATS, N_LONS = 121, 109
T2M_RANGE_C = (-40.0, 45.0)


def expected_periods(year: int) -> int:
    return 17_568 if calendar.isleap(year) else 17_520


def month_hours(year: int, month: int) -> int:
    return calendar.monthrange(year, month)[1] * 24


def check_pack(repo: Path, failures: list) -> None:
    root = repo / "data" / "packs" / "era5-eu"
    files = sorted(root.glob("*/era5_eu_*.parquet"))
    expected = {
        root / str(y) / f"era5_eu_{y}-{m:02d}.parquet"
        for y in YEARS
        for m in range(1, 13)
    }
    if set(files) != expected:
        missing = sorted(str(p) for p in expected - set(files))[:5]
        extra = sorted(str(p) for p in set(files) - expected)[:5]
        failures.append(
            f"pack file set wrong: {len(files)}/480; missing {missing}; "
            f"extra {extra}"
        )
        return
    total_hours = 0
    for y in YEARS:
        year_hours = 0
        for m in range(1, 13):
            f = root / str(y) / f"era5_eu_{y}-{m:02d}.parquet"
            df = pd.read_parquet(f)
            hours = month_hours(y, m)
            label = f.name
            if len(df) != hours * N_CELLS:
                failures.append(
                    f"{label}: {len(df)} rows, expected {hours}x{N_CELLS}"
                )
            if df["latitude"].nunique() != N_LATS or (
                df["longitude"].nunique() != N_LONS
            ):
                failures.append(
                    f"{label}: grid {df['latitude'].nunique()}x"
                    f"{df['longitude'].nunique()}, expected {N_LATS}x{N_LONS}"
                )
            if df[VARS].isna().any().any():
                failures.append(f"{label}: NaNs")
            t = df["time"]
            t0 = pd.Timestamp(f"{y}-{m:02d}-01 00:00")
            t1 = t0 + pd.Timedelta(hours=hours - 1)
            if t.min() != t0 or t.max() != t1 or t.nunique() != hours:
                failures.append(
                    f"{label}: time span {t.min()}..{t.max()} "
                    f"({t.nunique()} steps), expected {t0}..{t1} ({hours})"
                )
            year_hours += hours
        total_hours += year_hours
        print(f"pack {y}: 12 files, {year_hours} hours OK")
    if total_hours != 350_640:
        failures.append(f"pack total hours {total_hours}, expected 350,640")
    print(f"pack geometry: 480 files, {total_hours} hours, {N_CELLS} cells OK")


def trace_families() -> list[tuple[str, str, str]]:
    """(country, stem-tech, column) for every expected family."""
    fams = []
    for c, (_zone, techs) in eu.CF_COUNTRIES.items():
        for tech in techs:
            fams.append((c, f"{tech}_cf", "cf"))
    for c in eu.TEMP_COUNTRIES:
        fams.append((c, "t2m", "t2m_c"))
    return fams


def check_one(
    path: Path, column: str, year: int, failures: list
) -> tuple | None:
    label = path.stem
    if not path.exists():
        failures.append(f"{label}: missing")
        return None
    schema = pq.read_schema(path)
    if str(schema.field("utc_start").type) != "timestamp[us, tz=UTC]":
        failures.append(
            f"{label}: utc_start is {schema.field('utc_start').type}"
        )
    if str(schema.field(column).type) != "double":
        failures.append(f"{label}: {column} is {schema.field(column).type}")
    df = pd.read_parquet(path)
    if list(df.columns) != [column]:
        failures.append(f"{label}: columns {list(df.columns)}")
        return None
    if len(df) != expected_periods(year):
        failures.append(
            f"{label}: {len(df)} periods, expected {expected_periods(year)}"
        )
    if df.index.duplicated().any():
        failures.append(f"{label}: duplicate periods")
    deltas = df.index.to_series().diff().dropna().unique()
    if len(deltas) != 1 or deltas[0] != pd.Timedelta(minutes=30):
        failures.append(f"{label}: index not uniform 30-min")
    if df.index[0] != pd.Timestamp(f"{year}-01-01 00:00", tz="UTC"):
        failures.append(f"{label}: starts {df.index[0]}")
    if df.index[-1] != pd.Timestamp(f"{year}-12-31 23:30", tz="UTC"):
        failures.append(f"{label}: ends {df.index[-1]}")
    s = df[column]
    if s.isna().any():
        failures.append(f"{label}: NaNs")
    lo, hi = float(s.min()), float(s.max())
    if column == "cf" and (lo < 0.0 or hi > 1.0):
        failures.append(f"{label}: cf outside [0, 1]: {lo}..{hi}")
    if column == "t2m_c" and (lo < T2M_RANGE_C[0] or hi > T2M_RANGE_C[1]):
        failures.append(f"{label}: t2m_c outside {T2M_RANGE_C}: {lo}..{hi}")
    csv = pd.read_csv(
        path.with_suffix(".csv"), index_col="utc_start", parse_dates=True
    )
    if len(csv) != len(df) or (csv[column] - s).abs().max() > 1e-9:
        failures.append(f"{label}: CSV and Parquet disagree")
    return df.index[0], df.index[-1], float(s.mean())


def check_traces(repo: Path, failures: list) -> dict:
    out_root = repo / "data" / "packs" / "cf-eu"
    means_2024: dict = {}
    for c, stem_tech, column in trace_families():
        bounds: dict = {}
        for year in YEARS:
            path = out_root / c / f"{c}_{stem_tech}_{year}.parquet"
            res = check_one(path, column, year, failures)
            if res is None:
                continue
            bounds[year] = (res[0], res[1])
            if year == 2024 and column == "cf":
                means_2024[(c, stem_tech.removesuffix("_cf"))] = res[2]
        for year in sorted(bounds):
            if year + 1 in bounds:
                gap = bounds[year + 1][0] - bounds[year][1]
                if gap != pd.Timedelta(minutes=30):
                    failures.append(
                        f"{c}_{stem_tech}: {year}->{year + 1} step {gap}"
                    )
        print(f"traces {c}_{stem_tech}: {len(bounds)}/40 years OK")
    return means_2024


def check_calibration(repo: Path, means_2024: dict, failures: list) -> None:
    report = json.loads(
        (repo / "data" / "packs" / "cf-eu" / "eu_cf_report.json").read_text()
    )["calibration"]
    proc = repo / "data" / "packs" / "entsoe-2024" / "processed"
    cap = pd.read_parquet(proc / "capacity_2024.parquet").reset_index()
    agg = pd.read_parquet(proc / "aggregation_gen_2024.parquet").reset_index()
    cbs = eu.load_cbs_anchors(repo)  # NL onshore/solar anchors (deviation 3a)
    for c, (zone, techs) in eu.CF_COUNTRIES.items():
        for tech in techs:
            entry = report[c][tech]
            pinned = eu.PINNED_FACTORS_EU[c][tech]
            if round(float(entry["applied_factor"]), 4) != pinned:
                failures.append(
                    f"{c} {tech}: report factor {entry['applied_factor']} != "
                    f"pinned {pinned}"
                )
            mean = means_2024.get((c, tech))
            if mean is None:
                continue
            if not entry["calibrated"]:
                if float(entry["applied_factor"]) != 1.0:
                    failures.append(
                        f"{c} {tech}: uncalibrated but factor != 1.0"
                    )
                continue
            if (c, tech) in cbs:
                cap_mw, gen_gwh, status = cbs[(c, tech)]
                target = gen_gwh * 1e3 / (cap_mw * 8_784)
                anchor_name = f"CBS anchor (status {status})"
                if float(entry.get("anchor_gen_gwh_2024_cbs", -1)) != gen_gwh:
                    failures.append(
                        f"{c} {tech}: report CBS anchor "
                        f"{entry.get('anchor_gen_gwh_2024_cbs')} != pack "
                        f"{gen_gwh}"
                    )
            else:
                row_c = cap[(cap["zone"] == zone) & (cap["psr_code"] == eu.TECH_PSR[tech])]
                row_g = agg[(agg["zone"] == zone) & (agg["psr_code"] == eu.TECH_PSR[tech])]
                target = (
                    float(row_g["gen_gwh"].iloc[0])
                    * 1e3
                    / (float(row_c["capacity_mw"].iloc[0]) * 8_784)
                )
                anchor_name = "ENTSO-E anchor"
            rel = abs(mean - target) / target
            if rel > 1e-6:
                failures.append(
                    f"{c} {tech}: 2024 mean CF {mean:.6f} does not reproduce "
                    f"anchor target {target:.6f} (rel {rel:.2e})"
                )
            else:
                print(
                    f"calibration {c} {tech}: 2024 energy reproduces the "
                    f"{anchor_name} (rel {rel:.1e})"
                )


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument(
        "--traces-only",
        action="store_true",
        help="skip the EU pack geometry scan (delivery runs must be full)",
    )
    args = ap.parse_args()
    failures: list = []

    if not args.traces_only:
        check_pack(args.repo_root, failures)
    means_2024 = check_traces(args.repo_root, failures)
    check_calibration(args.repo_root, means_2024, failures)

    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    print("\nAll EU pack + CF validation checks passed.")


if __name__ == "__main__":
    main()
