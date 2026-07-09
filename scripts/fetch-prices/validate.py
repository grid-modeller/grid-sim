#!/usr/bin/env python3
"""Validate the processed 2024 price traces (docs/05-validation.md rules).

Checks (exit non-zero on failure):
1. Exactly 17,568 half-hourly periods per trace (2024 is a leap year).
2. No gaps, no duplicates; strictly uniform 30-min UTC index.
3. UTC-clean across the 2024-03-31 / 2024-10-27 GB clock changes (48
   periods per UTC day).
4. No NaNs. Exactly one filled MID period (2024-04-13 07:00Z, the
   documented APX gap); imbalance and gas traces gap-free at source.
5. Value sanity: volumes non-negative; gas SAP positive and within the
   2024 plausibility band (10–60 £/MWh); prices may legitimately be
   negative (2024 min MID ≈ −£62/MWh).
6. Cross-trace: mid_price vs system_price correlation reported (they are
   different quantities; correlation is a sanity indicator, not a gate).

Deterministic, no network. Usage: python validate.py <repo-root>
"""

import sys
from pathlib import Path

import pandas as pd

EXPECTED = 17_568
FILLED_PERIOD = pd.Timestamp("2024-04-13 07:00:00", tz="UTC")


def load(repo: Path, stem: str) -> pd.DataFrame:
    return pd.read_parquet(
        repo / "data" / "packs" / "2024" / "processed" / f"{stem}.parquet"
    )


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


def main() -> None:
    repo = Path(sys.argv[1])
    failures: list = []

    mid = load(repo, "market_index_2024")
    imb = load(repo, "imbalance_prices_2024")
    gas = load(repo, "gas_sap_daily_2024")

    for name, df in (("market_index", mid), ("imbalance_prices", imb),
                     ("gas_sap_daily", gas)):
        check_index(name, df, failures)
        nan_cols = df.columns[df.isna().any()].tolist()
        if nan_cols:
            failures.append(f"{name}: NaNs in {nan_cols}")

    filled = mid.index[mid["filled"]]
    if list(filled) != [FILLED_PERIOD]:
        failures.append(f"market_index: filled periods {list(filled)}, "
                        f"expected exactly [{FILLED_PERIOD}]")
    for col in ("apx_volume", "n2ex_volume"):
        if (mid[col] < 0).any():
            failures.append(f"market_index: negative {col}")
    if not gas["sap_gbp_per_mwh_hhv"].between(10, 60).all():
        failures.append("gas_sap_daily: outside 10–60 £/MWh plausibility band")

    print(f"mid_price:     mean {mid['mid_price'].mean():.2f}, "
          f"min {mid['mid_price'].min():.2f}, max {mid['mid_price'].max():.2f} £/MWh")
    print(f"system_price:  mean {imb['system_price'].mean():.2f}, "
          f"min {imb['system_price'].min():.2f}, max {imb['system_price'].max():.2f} £/MWh")
    print(f"gas SAP daily: mean {gas['sap_gbp_per_mwh_hhv'].mean():.2f}, "
          f"min {gas['sap_gbp_per_mwh_hhv'].min():.2f}, "
          f"max {gas['sap_gbp_per_mwh_hhv'].max():.2f} £/MWh (HHV)")
    print(f"corr(mid_price, system_price) = "
          f"{mid['mid_price'].corr(imb['system_price']):.4f}")

    if failures:
        print("\nFAILURES:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    print("\nAll price-trace validation checks passed.")


if __name__ == "__main__":
    main()
