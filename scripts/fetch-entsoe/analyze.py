#!/usr/bin/env python3
"""Stage 5 evidence numbers from the ENTSO-E pack.

Produces analysis_entsoe_2024.json (machine-readable; summarised in
docs/notes/entsoe-stage5-pack-report.md) and prints the tables:

1. Per-border 2024 flows: annual GWh each direction (ENTSO-E), net TWh,
   and reconciliation against the NESO/Elexon per-link actuals from the
   2024 validation pack (whose total net is 33.30 TWh).
2. Flow-direction base rates: % of half-hourly periods each border is
   importing / exporting / idle, at a 0 MW and a 50 MW dead-band —
   evidence for pinning the Stage 5 GB<->FR direction-match TBD-DATA.
3. Direction agreement between ENTSO-E and NESO per border (sanity on
   using either source as the direction-match target).
4. Sign-test first cut: Pearson r of border net imports vs GB wind CF
   (observed fleet-wide trace from the 2024 pack), half-hourly and
   daily means; NO2 hydro generation vs GB wind CF; NO2 reservoir-hydro
   weekly generation vs weekly inflow proxy.
5. Neighbour-zone load: annual TWh (for external-zone demand sanity).
6. Installed capacity per zone/type (GW, mapped technology ids).

Deterministic; no network.

Usage: python analyze.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

BORDERS = ["fr", "be", "nl", "no2", "dk1", "ie"]
LOAD_ZONES = ["fr", "be", "nl", "delu", "no2", "dk1", "ie"]


def main() -> None:
    repo = Path(sys.argv[1])
    proc = repo / "data" / "packs" / "entsoe-2024" / "processed"
    pack2024 = repo / "data" / "packs" / "2024" / "processed"
    out: dict = {}

    flows = pd.read_parquet(proc / "flows_gb_entsoe_2024.parquet")
    demand = pd.read_parquet(pack2024 / "demand_2024.parquet")
    wind_cf = pd.read_parquet(pack2024 / "wind_cf_2024.parquet")["wind_cf"]

    neso = {
        "fr": demand[["ifa_flow", "ifa2_flow", "eleclink_flow"]].sum(axis=1),
        "be": demand["nemo_flow"],
        "nl": demand["britned_flow"],
        "no2": demand["nsl_flow"],
        "dk1": demand["viking_flow"],
        "ie": demand[["moyle_flow", "east_west_flow", "greenlink_flow"]].sum(axis=1),
    }

    print("== 1. per-border 2024 flows (ENTSO-E) vs NESO per-link ==")
    tbl = {}
    for b in BORDERS:
        imp_gwh = flows[f"{b}_imp"].sum() * 0.5 / 1e3
        exp_gwh = flows[f"{b}_exp"].sum() * 0.5 / 1e3
        net_twh = flows[f"{b}_net"].sum() * 0.5 / 1e6
        neso_twh = neso[b].sum() * 0.5 / 1e6
        tbl[b] = {
            "entsoe_import_gwh": round(float(imp_gwh), 1),
            "entsoe_export_gwh": round(float(exp_gwh), 1),
            "entsoe_net_twh": round(float(net_twh), 3),
            "neso_net_twh": round(float(neso_twh), 3),
            "wedge_twh": round(float(net_twh - neso_twh), 3),
        }
        print(
            f"  {b:5s} imp {imp_gwh:9.1f} GWh  exp {exp_gwh:8.1f} GWh  "
            f"net {net_twh:+7.3f} TWh  (NESO {neso_twh:+7.3f}, "
            f"wedge {net_twh - neso_twh:+.3f})"
        )
    total_e = sum(v["entsoe_net_twh"] for v in tbl.values())
    total_n = sum(v["neso_net_twh"] for v in tbl.values())
    tbl["total"] = {"entsoe_net_twh": round(total_e, 3), "neso_net_twh": round(total_n, 3)}
    print(f"  TOTAL ENTSO-E net {total_e:+.2f} TWh vs NESO {total_n:+.2f} TWh")
    out["flows_annual"] = tbl

    print("== 2. flow-direction base rates (% of 17,568 periods) ==")
    rates = {}
    for b in BORDERS:
        net = flows[f"{b}_net"]
        r = {
            "import_pct": round(float((net > 0).mean() * 100), 2),
            "export_pct": round(float((net < 0).mean() * 100), 2),
            "zero_pct": round(float((net == 0).mean() * 100), 2),
            "import_pct_deadband50": round(float((net > 50).mean() * 100), 2),
            "export_pct_deadband50": round(float((net < -50).mean() * 100), 2),
            "idle_pct_deadband50": round(float((net.abs() <= 50).mean() * 100), 2),
        }
        rates[b] = r
        print(
            f"  {b:5s} imp {r['import_pct']:6.2f}%  exp {r['export_pct']:6.2f}%  "
            f"zero {r['zero_pct']:5.2f}%   [50 MW band: imp {r['import_pct_deadband50']:6.2f}%"
            f" exp {r['export_pct_deadband50']:6.2f}% idle {r['idle_pct_deadband50']:5.2f}%]"
        )
    out["direction_base_rates"] = rates

    print("== 3. ENTSO-E vs NESO direction agreement ==")
    agree = {}
    for b in BORDERS:
        e = flows[f"{b}_net"]
        n = neso[b].astype("float64")
        n.index = e.index
        both_active = (e.abs() > 50) | (n.abs() > 50)
        match = ((e > 0) == (n > 0)) | (~both_active)
        agree[b] = round(float(match.mean() * 100), 2)
        print(f"  {b:5s} direction match {agree[b]:6.2f}% (50 MW activity threshold)")
    out["direction_agreement_pct"] = agree

    print("== 4. sign-test first cut: net imports vs GB wind CF ==")
    wind = wind_cf.copy()
    wind.index = flows.index
    corr = {}
    groups = {b: flows[f"{b}_net"] for b in BORDERS}
    groups["continental_fr_be_nl"] = (
        flows["fr_net"] + flows["be_net"] + flows["nl_net"]
    )
    for name, net in groups.items():
        hh = float(net.corr(wind))
        daily = float(net.resample("1D").mean().corr(wind.resample("1D").mean()))
        corr[name] = {"halfhourly_r": round(hh, 3), "daily_r": round(daily, 3)}
        print(f"  {name:22s} r(half-hourly) {hh:+.3f}   r(daily) {daily:+.3f}")
    gen2 = pd.read_parquet(proc / "no2_generation_2024.parquet")
    no2_hydro = gen2[["hydro_reservoir", "hydro_ror"]].sum(axis=1)
    hh = float(no2_hydro.corr(wind))
    daily = float(no2_hydro.resample("1D").mean().corr(wind.resample("1D").mean()))
    corr["no2_hydro_generation"] = {
        "halfhourly_r": round(hh, 3),
        "daily_r": round(daily, 3),
    }
    print(f"  {'no2_hydro_generation':22s} r(half-hourly) {hh:+.3f}   r(daily) {daily:+.3f}")
    out["wind_cf_correlations"] = corr

    print("== 4b. NO2 reservoir hydro: weekly generation vs inflow proxy ==")
    res2 = pd.read_parquet(proc / "reservoir_no2_2024.parquet")
    weekly_gen = []
    idx = list(res2.index)
    for i, wk in enumerate(idx[:-1]):
        g = gen2["hydro_reservoir"][(gen2.index >= wk) & (gen2.index < idx[i + 1])]
        weekly_gen.append(float(g.sum() * 0.5))
    wg = pd.Series(weekly_gen, index=idx[:-1])
    infl = res2["inflow_proxy_mwh"].iloc[:-1]
    r = float(wg.corr(infl))
    out["no2_weekly_gen_vs_inflow_r"] = round(r, 3)
    out["no2_reservoir"] = {
        "storage_min_gwh": round(float(res2["storage_mwh"].min() / 1e3), 1),
        "storage_max_gwh": round(float(res2["storage_mwh"].max() / 1e3), 1),
        "annual_inflow_proxy_twh": round(float(infl.sum() / 1e6), 2),
    }
    print(
        f"  r(weekly gen, inflow proxy) {r:+.3f}; storage range "
        f"{out['no2_reservoir']['storage_min_gwh']}-"
        f"{out['no2_reservoir']['storage_max_gwh']} GWh; "
        f"inflow proxy {out['no2_reservoir']['annual_inflow_proxy_twh']} TWh"
    )

    print("== 5. neighbour-zone load, annual TWh ==")
    loads = {}
    for z in LOAD_ZONES:
        s = pd.read_parquet(proc / f"load_{z}_2024.parquet")["load_mw"]
        loads[z] = round(float(s.sum() * 0.5 / 1e6), 2)
        print(f"  {z:5s} {loads[z]:7.2f} TWh")
    out["load_annual_twh"] = loads

    print("== 6. installed capacity 2024 (GW) ==")
    cap = pd.read_parquet(proc / "capacity_2024.parquet")
    cap_out = {}
    for z in LOAD_ZONES:
        zc = cap.loc[[z]]
        cap_out[z] = {
            row.psr_name: round(float(row.capacity_mw) / 1e3, 2)
            for row in zc.itertuples()
        }
        top = sorted(cap_out[z].items(), key=lambda kv: -kv[1])[:5]
        print(f"  {z:5s} " + ", ".join(f"{k} {v}" for k, v in top))
    out["capacity_gw"] = cap_out

    (proc / "analysis_entsoe_2024.json").write_text(
        json.dumps(out, indent=2, sort_keys=True)
    )
    print("written: analysis_entsoe_2024.json")


if __name__ == "__main__":
    main()
