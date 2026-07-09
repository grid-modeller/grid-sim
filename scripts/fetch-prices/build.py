#!/usr/bin/env python3
"""Build the processed 2024 price traces from raw fetched data.

Provisional script — to be ported to `grid-cli fetch-data`. Deterministic:
pure transformation of the raw files, no network, no randomness.

Outputs (data/packs/2024/processed/), indexed by `utc_start` (UTC start of
each half-hourly settlement period, calendar 2024, 17,568 periods,
per ADR-3), CSV + Parquet:

- market_index_2024      Elexon MID (Market Index Data). Columns:
    apx_price, apx_volume    APXMIDP (EPEX Spot) price £/MWh, volume MWh
    n2ex_price, n2ex_volume  N2EXMIDP (Nord Pool) — DEFUNCT in practice:
                             zero volume in 17,489 of 17,524 published
                             2024 periods (nonzero in only 35; counts
                             after boundary dedupe)
    mid_price                REFERENCE PRICE: volume-weighted mean over
                             providers with volume > 0 (in practice the
                             APX price; N2EX contributes in 35 periods)
    filled                   bool; True where a price gap was filled (see
                             conventions below)
- imbalance_prices_2024  Elexon settlement system prices (DISEBSP).
    system_price             single imbalance price £/MWh (post-P305,
                             systemSellPrice == systemBuyPrice verified)
    niv                      net imbalance volume MWh
- gas_sap_daily_2024     ONS/National Gas daily System Average Price of
                             gas, upsampled: each half-hour carries its
                             UTC day's SAP. Columns: sap_gbp_per_mwh_hhv
                             (converted from p/kWh × 10; SAP is traded on
                             a gross-CV / HHV basis like all GB gas).

Conventions (documented, deterministic):
1. MID monthly chunks overlap at boundaries with byte-identical rows —
   dedupe on (utc_start, dataProvider). Verified identical before drop.
2. One genuine APX MID gap in 2024: 2024-04-13 07:00 UTC (SP16).
   Convention: mid_price/apx_price = mean of the two adjacent periods'
   APX prices; apx_volume = 0; filled = True. N2EX rows missing for 44
   periods are filled price 0, volume 0, matching N2EX's own published
   zero rows (N2EX price is unusable as a price series either way).
3. ONS SAP is a gas-day price (05:00–05:00 local); we map it to the UTC
   calendar day, a ≤ 5-hour approximation documented in the README.

Usage: python build.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

EXPECTED_PERIODS = 17_568  # 2024 is a leap year: 366 * 48


def utc_index_2024() -> pd.DatetimeIndex:
    return pd.date_range(
        "2024-01-01 00:00", "2024-12-31 23:30", freq="30min", tz="UTC"
    )


def load_mid(raw_dir: Path) -> pd.DataFrame:
    recs: list = []
    for f in sorted(raw_dir.glob("mid_*.json")):
        recs.extend(json.loads(f.read_text()))
    df = pd.DataFrame.from_records(recs)
    df["utc_start"] = pd.to_datetime(df["startTime"], utc=True)
    df = df[(df.utc_start >= "2024-01-01") & (df.utc_start < "2025-01-01")]
    # Chunk-boundary duplicates must be identical rows (convention 1).
    dup_mask = df.duplicated(subset=["utc_start", "dataProvider"], keep=False)
    dups = df[dup_mask]
    if dups.duplicated(
        subset=["utc_start", "dataProvider", "price", "volume"]
    ).sum() * 2 != len(dups):
        raise RuntimeError("non-identical duplicate MID rows — investigate")
    df = df.drop_duplicates(subset=["utc_start", "dataProvider"])
    return df


def build_market_index(raw_dir: Path) -> pd.DataFrame:
    df = load_mid(raw_dir)
    wide = df.pivot(index="utc_start", columns="dataProvider",
                    values=["price", "volume"])
    wide.columns = [f"{'apx' if p == 'APXMIDP' else 'n2ex'}_{v}"
                    for v, p in wide.columns]
    wide = wide.reindex(utc_index_2024()).rename_axis("utc_start")

    # Convention 2: N2EX missing rows -> price 0, volume 0 (matches its
    # own published zero rows).
    n2ex_missing = int(wide["n2ex_price"].isna().sum())
    wide["n2ex_price"] = wide["n2ex_price"].fillna(0.0)
    wide["n2ex_volume"] = wide["n2ex_volume"].fillna(0.0)

    # Convention 2: the single APX gap -> mean of adjacent APX prices.
    gap = wide.index[wide["apx_price"].isna()]
    wide["filled"] = False
    for t in gap:
        i = wide.index.get_loc(t)
        neighbours = wide["apx_price"].iloc[[i - 1, i + 1]]
        if neighbours.isna().any():
            raise RuntimeError(f"cannot fill APX gap at {t}: neighbour missing")
        wide.loc[t, "apx_price"] = neighbours.mean()
        wide.loc[t, "apx_volume"] = 0.0
        wide.loc[t, "filled"] = True
    print(f"market_index: APX gaps filled={len(gap)} "
          f"({[str(t) for t in gap]}), N2EX rows filled={n2ex_missing}")

    # Reference price: volume-weighted over providers with volume > 0.
    vol = wide["apx_volume"] + wide["n2ex_volume"]
    weighted = (wide["apx_price"] * wide["apx_volume"]
                + wide["n2ex_price"] * wide["n2ex_volume"])
    wide["mid_price"] = (weighted / vol).where(vol > 0, wide["apx_price"])
    cols = ["apx_price", "apx_volume", "n2ex_price", "n2ex_volume",
            "mid_price", "filled"]
    return wide[cols]


def build_imbalance(raw_dir: Path) -> pd.DataFrame:
    recs: list = []
    for f in sorted(raw_dir.glob("system_prices_*.json")):
        recs.extend(json.loads(f.read_text()))
    df = pd.DataFrame.from_records(recs)
    df["utc_start"] = pd.to_datetime(df["startTime"], utc=True)
    df = df[(df.utc_start >= "2024-01-01") & (df.utc_start < "2025-01-01")]
    if df["utc_start"].duplicated().any():
        raise RuntimeError("duplicate system-price periods")
    if not (df["systemSellPrice"] == df["systemBuyPrice"]).all():
        raise RuntimeError("sell != buy price — single-price assumption broken")
    out = df.set_index("utc_start").sort_index()
    out = out.rename(columns={"systemSellPrice": "system_price",
                              "netImbalanceVolume": "niv"})
    return out[["system_price", "niv"]].reindex(utc_index_2024()).rename_axis(
        "utc_start")


def build_gas_daily(raw_dir: Path) -> pd.DataFrame:
    sap = pd.read_excel(raw_dir / "ons_sap_of_gas_090125.xlsx",
                        sheet_name="Table 1 Daily SAP of Gas", skiprows=5)
    sap.columns = ["date", "sap_actual_day", "sap_7day_avg"]
    sap["date"] = pd.to_datetime(sap["date"])
    sap = sap[(sap.date >= "2024-01-01") & (sap.date < "2025-01-01")]
    if len(sap) != 366:
        raise RuntimeError(f"expected 366 daily SAP rows, got {len(sap)}")
    if sap["sap_actual_day"].isna().any():
        raise RuntimeError("NaN in daily SAP")
    # p/kWh -> £/MWh (×10); SAP is on a gross-CV (HHV) basis. Rounded to
    # 4 d.p. (source has 4 d.p. in p/kWh) to keep the CSV free of binary
    # float noise.
    daily = (sap.set_index("date")["sap_actual_day"].astype(float) * 10.0).round(4)
    idx = utc_index_2024()
    # Each half-hour carries its UTC calendar day's SAP (convention 3).
    out = daily.reindex(idx.tz_localize(None).normalize()).to_numpy()
    return pd.DataFrame({"sap_gbp_per_mwh_hhv": out}, index=idx).rename_axis(
        "utc_start")


def write(df: pd.DataFrame, out_dir: Path, stem: str) -> None:
    df.to_csv(out_dir / f"{stem}.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
    df.to_parquet(out_dir / f"{stem}.parquet")


def main() -> None:
    repo = Path(sys.argv[1])
    raw_dir = repo / "data" / "packs" / "2024" / "raw"
    out_dir = repo / "data" / "packs" / "2024" / "processed"
    out_dir.mkdir(parents=True, exist_ok=True)

    for stem, df in (
        ("market_index_2024", build_market_index(raw_dir)),
        ("imbalance_prices_2024", build_imbalance(raw_dir)),
        ("gas_sap_daily_2024", build_gas_daily(raw_dir)),
    ):
        assert len(df) == EXPECTED_PERIODS, (stem, len(df))
        write(df, out_dir, stem)
        print(f"built {stem}: {len(df)} periods")


if __name__ == "__main__":
    main()
