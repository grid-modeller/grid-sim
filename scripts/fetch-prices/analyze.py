#!/usr/bin/env python3
"""Stage 2 tolerance evidence from the 2024 price pack (docs/05 philosophy:
quantify the irreducible discrepancies; the supervisor sets the numbers).

Produces data/packs/2024/processed/price_analysis_2024.json and prints a
summary. Deterministic, no network.

What is computed:

1. A 2024 half-hourly CCGT and OCGT SRMC series from the committed
   reference file (data/reference/prices-2024.toml):
     SRMC = gas_SAP_daily / eta + (EF_CO2 / eta) * (UKA_step + CPS)
   with UKA step-interpolated between the 25 fortnightly auctions
   (forward-fill; the pre-10-Jan stub carries the 10 Jan value). VOM
   excluded (sensitivity reported).

2. "% of periods consistent with gas setting the price" under explicit
   candidate definitions, against the MID reference price:
     A  |P - SRMC_ccgt| <= 20% of SRMC_ccgt
     B  |P - SRMC_ccgt| <= 15 GBP/MWh
     C  SRMC_ccgt - 10 <= P <= SRMC_ocgt + 10  (price in the gas band)
   plus sensitivities on A: CCGT eta 0.45 / 0.53, +3 GBP/MWh VOM.
   These are OUTTURN CONSISTENCY proxies, not the model's price-setting
   definition (the Stage 2 engine will know which unit is marginal); they
   bound what an outturn comparison can support.

3. 2024 capture/baseload price ratios (volume-weighted mean MID price over
   the technology's output, divided by the time-weighted mean MID price):
   total wind (tx + embedded, D3 convention), tx-only wind, embedded
   solar. Annual and monthly. Two reviewer-requested variants:
   (a) total wind against the IMBALANCE price instead of MID
   (sensitivity to price-series choice); (b) MID weighted by the
   MODELLED wind trace — capacity-weighted ERA5 CFs (14.4 GW onshore +
   14.7 GW offshore, the pack's gb_{on,off}shore_cf_2024) — measuring
   how much the ratio moves between observed and modelled weighting
   (Stage 2 validates with modelled wind, half-hourly r = 0.967).

Usage: python analyze.py <repo-root>
"""

import json
import sys
import tomllib
from pathlib import Path

import pandas as pd

CPS = 18.0  # GBP/tCO2, carbon price support (see reference TOML)


def load(repo: Path, stem: str) -> pd.DataFrame:
    return pd.read_parquet(
        repo / "data" / "packs" / "2024" / "processed" / f"{stem}.parquet"
    )


def uka_step_series(repo: Path, index: pd.DatetimeIndex) -> pd.Series:
    ref = tomllib.loads(
        (repo / "data" / "reference" / "prices-2024.toml").read_text()
    )
    auctions = pd.Series(
        {pd.Timestamp(a["date"], tz="UTC"): a["clearing_price"]
         for a in ref["carbon"]["uka"]["auctions"]}
    ).sort_index()
    s = auctions.reindex(index, method="ffill").bfill()  # bfill = pre-10-Jan stub
    return s


def srmc(gas: pd.Series, uka: pd.Series, eta: float, ef: float,
         vom: float = 0.0) -> pd.Series:
    return gas / eta + (ef / eta) * (uka + CPS) + vom


def pct(mask: pd.Series) -> float:
    return round(float(mask.mean()) * 100, 2)


def capture(price: pd.Series, volume: pd.Series) -> float:
    return float((price * volume).sum() / volume.sum())


def main() -> None:
    repo = Path(sys.argv[1])
    ref = tomllib.loads(
        (repo / "data" / "reference" / "prices-2024.toml").read_text()
    )
    ef = ref["emission_factor"]["natural_gas"]["co2_tonnes_per_mwh_th_hhv"]
    eta_ccgt = ref["efficiency"]["ccgt"]["hhv"]
    eta_lo, eta_hi = ref["efficiency"]["ccgt"]["hhv_sensitivity"]
    eta_ocgt = ref["efficiency"]["ocgt"]["hhv"]
    vom = ref["vom"]["ccgt_typical_gbp_per_mwh"]

    mid = load(repo, "market_index_2024")
    gas = load(repo, "gas_sap_daily_2024")["sap_gbp_per_mwh_hhv"]
    gen = load(repo, "generation_by_fuel_2024")
    demand = load(repo, "demand_2024")
    imb = load(repo, "imbalance_prices_2024")["system_price"]
    # Modelled wind (MW): capacity-weighted ERA5 CF traces, end-2024
    # capacities per the reference scenario (14.4 GW onshore GB,
    # 14.7 GW offshore, UKWED — see scripts/fetch-2024/README.md).
    wind_modelled = (
        14_400 * load(repo, "gb_onshore_cf_2024")["cf"]
        + 14_700 * load(repo, "gb_offshore_cf_2024")["cf"]
    )

    p = mid["mid_price"]
    uka = uka_step_series(repo, p.index)

    ccgt = srmc(gas, uka, eta_ccgt, ef)
    ocgt = srmc(gas, uka, eta_ocgt, ef)

    defs = {
        "A_within_20pct_of_ccgt_srmc": (p - ccgt).abs() <= 0.20 * ccgt,
        "B_within_15gbp_of_ccgt_srmc": (p - ccgt).abs() <= 15.0,
        "C_between_ccgt_minus10_and_ocgt_plus10": (p >= ccgt - 10.0)
        & (p <= ocgt + 10.0),
        "A_sens_eta_0.45": (p - srmc(gas, uka, eta_lo, ef)).abs()
        <= 0.20 * srmc(gas, uka, eta_lo, ef),
        "A_sens_eta_0.53": (p - srmc(gas, uka, eta_hi, ef)).abs()
        <= 0.20 * srmc(gas, uka, eta_hi, ef),
        "A_sens_vom_plus3": (p - srmc(gas, uka, eta_ccgt, ef, vom)).abs()
        <= 0.20 * srmc(gas, uka, eta_ccgt, ef, vom),
    }
    # D: price consistent with SOME gas unit of plausible efficiency —
    # equivalent to SRMC(eta=0.60) <= P <= SRMC(eta=0.40).
    defs["D_implied_gas_eta_0.40_to_0.60"] = (
        p >= srmc(gas, uka, 0.60, ef)) & (p <= srmc(gas, uka, 0.40, ef))
    gas_marginal = {k: pct(v) for k, v in defs.items()}

    # Level diagnostics: where does the observed price sit relative to the
    # computed CCGT SRMC? (Evidence for whether a level-based definition
    # can ever reach the ~97% claim.)
    ratio = p / ccgt
    level_diag = {
        "ratio_price_over_ccgt_srmc_percentiles": {
            f"p{q}": round(float(ratio.quantile(q / 100)), 3)
            for q in (5, 25, 50, 75, 95)
        },
        "pct_below_0.8_srmc": pct(ratio < 0.8),
        "pct_above_1.2_srmc": pct(ratio > 1.2),
        "corr_halfhourly_price_vs_ccgt_srmc": round(float(p.corr(ccgt)), 4),
        "corr_daily_means": round(float(
            p.resample("1D").mean().corr(ccgt.resample("1D").mean())), 4),
        "corr_monthly_means": round(float(
            p.resample("1ME").mean().corr(ccgt.resample("1ME").mean())), 4),
    }

    # Cross-tab against the pack's outturn flexing proxy (report §5):
    # CCGT output strictly between 5% and 95% of its observed 2024 max —
    # are the price-inconsistent periods the non-flexing ones?
    ccgt_max = float(gen["ccgt"].max())
    flexing = (gen["ccgt"] > 0.05 * ccgt_max) & (gen["ccgt"] < 0.95 * ccgt_max)
    within = defs["A_within_20pct_of_ccgt_srmc"]
    crosstab = {
        "pct_flexing_5_95": pct(flexing),
        "pct_within20_given_flexing": pct(within[flexing]),
        "pct_within20_given_not_flexing": pct(within[~flexing]),
        "median_price_over_srmc_given_flexing": round(
            float((p / ccgt)[flexing].median()), 3),
        "median_price_over_srmc_given_not_flexing": round(
            float((p / ccgt)[~flexing].median()), 3),
    }

    # Capture prices (D3 total-generation convention for total wind).
    wind_total = gen["wind"] + demand["embedded_wind_generation"]
    wind_tx = gen["wind"]
    solar = demand["embedded_solar_generation"]
    baseload = float(p.mean())

    def ratios(volume: pd.Series, price: pd.Series = p) -> dict:
        annual = capture(price, volume) / float(price.mean())
        monthly = {
            f"2024-{m:02d}": round(
                capture(price[price.index.month == m],
                        volume[volume.index.month == m])
                / float(price[price.index.month == m].mean()), 4)
            for m in range(1, 13)
        }
        return {"annual": round(annual, 4), "monthly": monthly}

    out = {
        "generated_by": "scripts/fetch-prices/analyze.py",
        "mid_price": {
            "annual_time_weighted_mean_gbp_mwh": round(baseload, 2),
            "annual_volume_weighted_mean_gbp_mwh": round(
                capture(p, mid["apx_volume"] + mid["n2ex_volume"]), 2),
            "min": round(float(p.min()), 2),
            "max": round(float(p.max()), 2),
            "negative_periods": int((p < 0).sum()),
        },
        "srmc_2024_gbp_mwh": {
            "inputs": {
                "ef_tco2_per_mwh_th_hhv": ef,
                "eta_ccgt_hhv": eta_ccgt,
                "eta_ocgt_hhv": eta_ocgt,
                "cps_gbp_per_tco2": CPS,
                "uka_mean_step_series": round(float(uka.mean()), 2),
                "gas_sap_daily_mean": round(float(gas.mean()), 2),
                "vom": 0.0,
            },
            "ccgt": {
                "mean": round(float(ccgt.mean()), 2),
                "min": round(float(ccgt.min()), 2),
                "max": round(float(ccgt.max()), 2),
                "monthly_mean": {
                    f"2024-{m:02d}": round(float(ccgt[ccgt.index.month == m].mean()), 2)
                    for m in range(1, 13)
                },
            },
            "ocgt": {
                "mean": round(float(ocgt.mean()), 2),
                "min": round(float(ocgt.min()), 2),
                "max": round(float(ocgt.max()), 2),
            },
        },
        "pct_periods_gas_marginal_by_definition": gas_marginal,
        "price_vs_srmc_level_diagnostics": level_diag,
        "crosstab_flexing_proxy_vs_price_consistency": crosstab,
        "capture_baseload_ratios": {
            "wind_total_tx_plus_embedded_D3": ratios(wind_total),
            "wind_tx_only": ratios(wind_tx),
            "solar_embedded": ratios(solar),
            "wind_total_vs_imbalance_price": ratios(wind_total, imb),
            "wind_modelled_era5_weighted_vs_mid": ratios(wind_modelled),
        },
    }

    dest = repo / "data" / "packs" / "2024" / "processed" / "price_analysis_2024.json"
    dest.write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))
    print(f"\nwritten: {dest}")


if __name__ == "__main__":
    main()
