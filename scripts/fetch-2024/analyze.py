#!/usr/bin/env python3
"""Quantify 2024 discrepancies that inform Stage 1/2 validation tolerances
(decision D2, docs/08-risks-and-decisions.md; method per docs/05-validation.md).

Deterministic, no network, no randomness. Reads the processed pack, prints
all tables, and writes analysis_2024.json + monthly_generation_2024.csv
alongside the processed data.

Semantics established during assembly (see README):
- Elexon FUELHH `ps` is NET pumped storage (negative = pumping).
- Elexon INT* columns are net flows, + = import to GB; they reconcile with
  the NESO *_FLOW columns to corr >= 0.9996 and identical annual TWh.
- FUELHH has NO solar category: GB solar is almost entirely
  distribution-connected and appears only as the NESO embedded estimate.
- NESO ND excludes embedded generation, pumping demand and exports;
  identity: FUELHH total (PS net) + net imports - ND ~= station load.

Gas-marginal proxy (stated explicitly): a period is counted as
"gas plausibly marginal" when CCGT output is strictly between 5% and 95%
of its observed 2024 maximum — i.e. the CCGT fleet is flexing, neither at
its floor (gas likely extra-marginal) nor at its ceiling (scarcity /
something else setting the margin).

Usage: python analyze.py <repo-root>
"""

import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd

TWH = 0.5 / 1e6  # MW per half-hour -> TWh

INT_LABELS = {
    "intfr": "IFA (FR)", "intifa2": "IFA2 (FR)", "intelec": "ElecLink (FR)",
    "intned": "BritNed (NL)", "intnem": "Nemo (BE)", "intnsl": "NSL (NO)",
    "intvkl": "Viking (DK)", "intirl": "Moyle (NI/SEM)",
    "intew": "EWIC (IE/SEM)", "intgrnl": "Greenlink (IE/SEM)",
}


def main() -> None:
    repo = Path(sys.argv[1])
    proc = repo / "data" / "packs" / "2024" / "processed"
    d = pd.read_parquet(proc / "demand_2024.parquet")
    g = pd.read_parquet(proc / "generation_by_fuel_2024.parquet")
    out: dict = {}

    int_cols = [c for c in g.columns if c.startswith("int")]
    fuel_cols = [c for c in g.columns if not c.startswith("int")]

    # ---- 1. Annual generation by fuel (TWh, transmission-metered) ----
    annual = (g[fuel_cols].sum() * TWH).round(3)
    ps_gen = float(g["ps"].clip(lower=0).sum() * TWH)
    ps_pump = float(-g["ps"].clip(upper=0).sum() * TWH)
    out["annual_generation_twh"] = {
        "gas_ccgt": annual["ccgt"], "gas_ocgt": annual["ocgt"],
        "gas_total": round(annual["ccgt"] + annual["ocgt"], 3),
        "wind_tx": annual["wind"], "nuclear": annual["nuclear"],
        "biomass": annual["biomass"], "hydro_npshyd": annual["npshyd"],
        "pumped_storage_net": annual["ps"],
        "pumped_storage_gross_generation": round(ps_gen, 3),
        "pumped_storage_gross_pumping": round(ps_pump, 3),
        "coal": annual["coal"], "oil": annual["oil"], "other": annual["other"],
        "solar_tx": 0.0,  # no FUELHH solar category — all embedded
    }
    print("Annual transmission-metered generation (TWh):")
    print((g[fuel_cols].sum() * TWH).round(2).sort_values(ascending=False).to_string())

    # ---- 2. Demand and imports ----
    imports = (g[int_cols].sum() * TWH).round(3)
    out["annual_demand_twh"] = {
        "nd": round(float(d["nd"].sum() * TWH), 3),
        "tsd": round(float(d["tsd"].sum() * TWH), 3),
    }
    out["annual_net_imports_twh"] = {
        INT_LABELS[c]: imports[c] for c in int_cols
    } | {"total": round(float(imports.sum()), 3)}
    print("\nAnnual demand (TWh):", out["annual_demand_twh"])
    print("Net imports per interconnector (TWh, + = import):")
    for k, v in out["annual_net_imports_twh"].items():
        print(f"  {k:20s} {v:7.2f}")

    # ---- 3. Embedded-generation wedge ----
    emb_wind = float(d["embedded_wind_generation"].sum() * TWH)
    emb_solar = float(d["embedded_solar_generation"].sum() * TWH)
    tx_wind = float(annual["wind"])
    total_supply = g[fuel_cols].sum(axis=1) + g[int_cols].sum(axis=1) \
        + d["embedded_wind_generation"] + d["embedded_solar_generation"]
    emb_share = (d["embedded_wind_generation"] + d["embedded_solar_generation"]) / total_supply
    out["embedded_wedge"] = {
        "embedded_wind_twh": round(emb_wind, 3),
        "embedded_solar_twh": round(emb_solar, 3),
        "tx_wind_twh": round(tx_wind, 3),
        "embedded_share_of_supply_mean": round(float(emb_share.mean()), 4),
        "embedded_share_of_supply_p95": round(float(emb_share.quantile(0.95)), 4),
        "embedded_share_of_supply_max": round(float(emb_share.max()), 4),
        "embedded_wind_capacity_mw_end": float(d["embedded_wind_capacity"].iloc[-1]),
        "embedded_solar_capacity_mw_end": float(d["embedded_solar_capacity"].iloc[-1]),
    }
    print("\nEmbedded wedge:", json.dumps(out["embedded_wedge"], indent=2))

    # ---- 4. Pumped storage (NESO vs Elexon views) ----
    out["pumped_storage"] = {
        "elexon_gross_generation_twh": round(ps_gen, 3),
        "elexon_gross_pumping_twh": round(ps_pump, 3),
        "implied_round_trip_efficiency": round(ps_gen / ps_pump, 3),
        "neso_pump_storage_pumping_twh": round(float(d["pump_storage_pumping"].sum() * TWH), 3),
    }
    print("\nPumped storage:", json.dumps(out["pumped_storage"], indent=2))

    # ---- 5. Monthly generation-by-fuel matrix + correlation ceiling ----
    gm = g[fuel_cols].resample("MS").sum() * 0.5 / 1e3  # GWh
    gm["wind_incl_embedded"] = gm["wind"] + d["embedded_wind_generation"].resample("MS").sum() * 0.5 / 1e3
    gm["solar_embedded"] = d["embedded_solar_generation"].resample("MS").sum() * 0.5 / 1e3
    gm.round(1).to_csv(proc / "monthly_generation_2024.csv")
    print("\nMonthly generation matrix (GWh) written to monthly_generation_2024.csv")
    print(gm.round(0).to_string())

    # Correlation ceiling: how much does the embedded treatment alone move
    # the "monthly generation mix correlation" metric? Compare tx-metered
    # actuals vs the same data with embedded wind folded into wind and
    # embedded solar added as a fuel — the two candidate model conventions.
    main_fuels = ["ccgt", "ocgt", "nuclear", "biomass", "npshyd", "wind"]
    a = gm[main_fuels].copy()
    b = a.copy()
    b["wind"] = gm["wind_incl_embedded"]
    a["solar"] = 0.0
    b["solar"] = gm["solar_embedded"]
    flat_r = float(np.corrcoef(a.values.ravel(), b.values.ravel())[0, 1])
    per_fuel_r = {f: (1.0 if f in ("ccgt", "ocgt", "nuclear", "biomass", "npshyd")
                      else round(float(a[f].corr(b[f])), 4) if a[f].std() > 0 else None)
                  for f in a.columns}
    out["monthly_mix_correlation"] = {
        "tx_vs_embedded_convention_flattened_r": round(flat_r, 4),
        "wind_tx_vs_wind_incl_embedded_monthly_r": round(float(gm["wind"].corr(gm["wind_incl_embedded"])), 4),
        "note": "r between 12xfuel matrices under the two embedded conventions; "
                "a model can hit at most ~this if validated against the other convention",
        "per_fuel": per_fuel_r,
    }
    print("\nMonthly mix correlation ceiling:", json.dumps(out["monthly_mix_correlation"], indent=2))

    # ---- 6. Gas-marginal proxy ----
    ccgt_max = float(g["ccgt"].max())
    flexing = (g["ccgt"] > 0.05 * ccgt_max) & (g["ccgt"] < 0.95 * ccgt_max)
    out["gas_marginal_proxy"] = {
        "ccgt_observed_max_mw": ccgt_max,
        "proxy": "CCGT strictly between 5% and 95% of observed 2024 max",
        "pct_periods": round(float(flexing.mean() * 100), 2),
        "pct_periods_3pct_97pct_band": round(float(((g["ccgt"] > 0.03 * ccgt_max) & (g["ccgt"] < 0.97 * ccgt_max)).mean() * 100), 2),
        "pct_periods_10pct_90pct_band": round(float(((g["ccgt"] > 0.10 * ccgt_max) & (g["ccgt"] < 0.90 * ccgt_max)).mean() * 100), 2),
    }
    print("\nGas-marginal proxy:", json.dumps(out["gas_marginal_proxy"], indent=2))

    # ---- 7. Data quality: residual and glitch periods ----
    residual = g[fuel_cols].sum(axis=1) + g[int_cols].sum(axis=1) - d["nd"]
    glitch = residual[residual.abs() > 5000]
    out["cross_check_residual_mw"] = {
        "identity": "FUELHH total (PS net) + net imports - ND ~= station load",
        "mean": round(float(residual.mean()), 1),
        "median": round(float(residual.median()), 1),
        "std": round(float(residual.std()), 1),
        "p01": round(float(residual.quantile(0.01)), 1),
        "p99": round(float(residual.quantile(0.99)), 1),
        "annual_twh": round(float(residual.sum() * TWH), 3),
        "abs_gt_5gw_periods": int(len(glitch)),
        "abs_gt_5gw_timestamps": [str(t) for t in glitch.index],
    }
    print("\nCross-check residual:", json.dumps(
        {k: v for k, v in out["cross_check_residual_mw"].items()
         if k != "abs_gt_5gw_timestamps"}, indent=2))
    print("glitch periods:", out["cross_check_residual_mw"]["abs_gt_5gw_timestamps"])

    (proc / "analysis_2024.json").write_text(json.dumps(out, indent=2))
    print("\nwritten:", proc / "analysis_2024.json")


if __name__ == "__main__":
    main()
