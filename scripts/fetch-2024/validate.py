#!/usr/bin/env python3
"""Validate the processed 2024 pack against the spec in docs/05-validation.md.

Checks (exit non-zero on failure):
1. Exactly 17,568 half-hourly periods per trace (2024 is a leap year).
2. No gaps, no duplicates; strictly monotonic UTC index at 30-min spacing.
3. UTC-clean across the 2024-03-31 (short) and 2024-10-27 (long) GB clock
   changes: both UTC days still hold exactly 48 periods, and the raw NESO
   settlement-day period counts are 46 and 50 respectively.
4. No NaNs (except documented INTGRNL pre-go-live handling, filled at
   build time); flag negative values in columns where they are anomalous.
   wind_cf_2024 additionally bounded to [0, 1].
5. Cross-check: NESO ND vs. Elexon transmission generation (FUELHH `ps`
   is already net of pumping — do not also subtract NESO pumping; see
   report §6.4) + net imports; report residual statistics.

Deterministic, no network. Usage: python validate.py <repo-root>
"""

import sys
from pathlib import Path

import pandas as pd

EXPECTED = 17_568
INT_COLS_MAY_BE_NEGATIVE = True  # net flows: + import, − export


def load(repo: Path, stem: str) -> pd.DataFrame:
    df = pd.read_parquet(repo / "data" / "packs" / "2024" / "processed" / f"{stem}.parquet")
    return df


def check_index(name: str, df: pd.DataFrame, failures: list) -> None:
    if len(df) != EXPECTED:
        failures.append(f"{name}: {len(df)} periods, expected {EXPECTED}")
    if df.index.duplicated().any():
        failures.append(f"{name}: duplicate periods")
    deltas = df.index.to_series().diff().dropna().unique()
    if len(deltas) != 1 or deltas[0] != pd.Timedelta(minutes=30):
        failures.append(f"{name}: index not uniform 30-min: {deltas}")
    for day in ("2024-03-31", "2024-10-27"):
        n = len(df.loc[day])
        if n != 48:
            failures.append(f"{name}: UTC day {day} has {n} periods, expected 48")


def check_clock_change_raw(repo: Path, failures: list) -> None:
    raw = pd.read_csv(repo / "data" / "packs" / "2024" / "raw" / "demanddata_2024.csv")
    counts = raw.groupby("SETTLEMENT_DATE")["SETTLEMENT_PERIOD"].count()
    for date, expected in (("31-MAR-2024", 46), ("27-OCT-2024", 50)):
        if counts.get(date) != expected:
            failures.append(
                f"raw NESO settlement day {date}: {counts.get(date)} periods, expected {expected}"
            )


def check_values(name: str, df: pd.DataFrame, failures: list) -> None:
    nan_cols = df.columns[df.isna().any()].tolist()
    if nan_cols:
        failures.append(f"{name}: NaNs in {nan_cols}")
    for col in df.columns:
        if col.startswith("int") and INT_COLS_MAY_BE_NEGATIVE:
            continue
        neg = int((df[col] < 0).sum())
        if neg:
            # Report, not fail: small negative station-load artefacts occur.
            print(f"  note: {name}.{col} has {neg} negative periods "
                  f"(min {df[col].min():.0f} MW)")


def cross_check(demand: pd.DataFrame, gen: pd.DataFrame) -> None:
    int_cols = [c for c in gen.columns if c.startswith("int")]
    tx_gen = gen.drop(columns=int_cols).sum(axis=1)
    net_imports = gen[int_cols].sum(axis=1)
    # Elexon `ps` within tx_gen is net (pumping negative), so pumping is
    # already accounted for; subtracting NESO pumping too would mix
    # metering conventions within one identity (report §6.4).
    supply = tx_gen + net_imports
    residual = supply - demand["nd"]
    print("\nCross-check: (Elexon tx gen incl. net PS + net imports) - NESO ND [MW]")
    print(residual.describe().round(1).to_string())
    print(f"  mean |residual| / mean ND: "
          f"{residual.abs().mean() / demand['nd'].mean() * 100:.2f}%")
    print(f"  annual residual energy: {residual.sum() * 0.5 / 1e6:.2f} TWh")


def main() -> None:
    repo = Path(sys.argv[1])
    failures: list = []
    demand = load(repo, "demand_2024")
    gen = load(repo, "generation_by_fuel_2024")
    wind_cf = load(repo, "wind_cf_2024")
    for name, df in (("demand", demand), ("generation", gen), ("wind_cf", wind_cf)):
        check_index(name, df, failures)
        check_values(name, df, failures)
    if (demand["underlying_demand"] != demand["nd"] + demand["embedded_wind_generation"]
            + demand["embedded_solar_generation"]).any():
        failures.append("demand: underlying_demand != nd + embedded wind + embedded solar")
    if float(wind_cf["wind_cf"].min()) < 0 or float(wind_cf["wind_cf"].max()) > 1:
        failures.append("wind_cf: values outside [0, 1]")
    check_clock_change_raw(repo, failures)
    cross_check(demand, gen)
    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    print("\nAll validation checks passed.")


if __name__ == "__main__":
    main()
