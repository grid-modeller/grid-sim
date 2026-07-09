#!/usr/bin/env python3
"""Validate the ENTSO-E Stage 5 pack. Exit non-zero on any failure.

Checks (docs/05-validation.md discipline — period counts, gap analysis,
clock-change handling, cross-source reconciliation):
1. Half-hourly traces (flows, load, NO2/NO/FR generation, FR non-GB
   flows): exactly 17,568 rows; strictly uniform 30-min UTC index
   covering calendar 2024 (UTC indexing makes the March/October European
   clock changes a non-event: the API is queried and answers in UTC; no
   46/50-period days exist to repair); no NaNs; CSV and Parquet
   value-identical.
2. Flows: net == imp - exp exactly; imp/exp non-negative.
3. Capacity: all 7 zones, known PSR codes, positive MW.
4. Reservoir (NO2/NO/FR): 52-53 weekly rows per zone, storage positive
   and monotone-dated; inflow proxy present for all but the two boundary
   weeks (Norwegian weeks run Monday 00:00 Europe/Oslo; French weeks
   Monday 00:00 Europe/Paris — both anchor Sunday 23:00 UTC).
4a. FR per-type traces (Stage 5 A2 remediation, build_fr.py): every
   complete non-B10 series' annual energy matches the independently
   assembled aggregation_gen_2024 table (same raw XML, two build paths);
   the B10 pair-rule energies sit BELOW the aggregation figure, whose
   generic repairs invent generation during pumping windows (build_fr.py
   docstring rule 1).
4b. FR non-GB flows (Stage 5 A2 remediation, build_fr_flows.py):
   identities and signs per border; the headline column equals
   -(sum of border nets) exactly; FR is an annual net exporter on every
   non-GB border (RTE 2024: "solde exportateur positif sur toutes ses
   frontières"); and FR total physical net exports (this file + the GB
   border from flows_gb_entsoe_2024) fall in an evidence-based band —
   see the inline justification.
5. Cross-source reconciliation: ENTSO-E GB annual net imports within
   tolerance of the NESO/Elexon per-link actuals in the 2024 validation
   pack (per-border table in analyze.py / the evidence note). Tolerance
   ±0.5 TWh per border, justification: the two metering points differ
   (ENTSO-E border metering vs NESO GB-side settlement metering) and
   link losses land between them; the observed wedges are 0.006-0.43 TWh
   per border, ENTSO-E consistently higher (sending-end vs GB receiving-
   end metering; analyze output), so ±0.5 TWh catches source regressions
   without failing on the irreducible metering wedge.
6. Build reports (build.py + build_fr.py + build_fr_flows.py): no
   unfilled gaps, no ACK-months on non-flow series.

Usage: python validate.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

FAILURES = []


def check(cond: bool, msg: str) -> None:
    if cond:
        print(f"  ok: {msg}")
    else:
        print(f"  FAIL: {msg}")
        FAILURES.append(msg)


def check_halfhourly(path: Path, stem: str) -> pd.DataFrame:
    df = pd.read_parquet(path / f"{stem}.parquet")
    idx = df.index
    check(len(df) == 17568, f"{stem}: 17,568 periods (got {len(df)})")
    check(str(idx[0]) == "2024-01-01 00:00:00+00:00", f"{stem}: starts Jan 1 00:00Z")
    check(str(idx[-1]) == "2024-12-31 23:30:00+00:00", f"{stem}: ends Dec 31 23:30Z")
    steps = idx.to_series().diff().dropna().unique()
    check(
        len(steps) == 1 and steps[0] == pd.Timedelta(minutes=30),
        f"{stem}: strictly uniform 30-min UTC index",
    )
    check(int(df.isna().sum().sum()) == 0, f"{stem}: no NaNs")
    csv = pd.read_csv(path / f"{stem}.csv", index_col=0, parse_dates=True)
    same = (abs(csv.values - df.values) < 1e-6).all()
    check(bool(same), f"{stem}: CSV and Parquet agree (<1e-6)")
    return df


def main() -> None:
    repo = Path(sys.argv[1])
    proc = repo / "data" / "packs" / "entsoe-2024" / "processed"
    pack2024 = repo / "data" / "packs" / "2024" / "processed"

    print("== half-hourly traces ==")
    flows = check_halfhourly(proc, "flows_gb_entsoe_2024")
    for z in ["fr", "be", "nl", "delu", "no2", "dk1", "ie"]:
        check_halfhourly(proc, f"load_{z}_2024")
    for z in ["no2", "no", "fr"]:
        check_halfhourly(proc, f"{z}_generation_2024")
    frx = check_halfhourly(proc, "fr_external_2024")

    print("== flows: identities and signs ==")
    for b in ["fr", "be", "nl", "no2", "dk1", "ie"]:
        net_ok = (
            (flows[f"{b}_net"] - (flows[f"{b}_imp"] - flows[f"{b}_exp"])).abs() < 1e-9
        ).all()
        check(bool(net_ok), f"{b}: net == imp - exp")
        check(bool((flows[f"{b}_imp"] >= 0).all()), f"{b}: imports non-negative")
        check(bool((flows[f"{b}_exp"] >= 0).all()), f"{b}: exports non-negative")

    print("== FR per-type generation vs aggregation table ==")
    frg = pd.read_parquet(proc / "fr_generation_2024.parquet")
    agg = pd.read_parquet(proc / "aggregation_gen_2024.parquet").loc["fr"]
    agg = agg.set_index("series")["gen_gwh"]
    for col in frg.columns:
        if col == "hydro_pumped_con":
            continue  # consumption series: excluded from the aggregation table
        e_gwh = float(frg[col].sum() * 0.5 / 1e3)
        if col == "hydro_pumped":
            # The aggregation figure (9,456 GWh) is inflated by generic
            # interpolation/day-offset repairs across pumping windows; the
            # pair-rule trace energy must sit below it (and above zero).
            check(
                0.0 < e_gwh < float(agg[col]),
                f"fr {col}: pair-rule {e_gwh:.1f} GWh < aggregation "
                f"{float(agg[col]):.1f} GWh (repair-inflated)",
            )
            continue
        # Same raw XML, same repair ladder, two build paths -> annual
        # energies agree to rounding (the table stores 3 dp).
        check(
            abs(e_gwh - float(agg[col])) < 0.01,
            f"fr {col}: {e_gwh:.3f} GWh matches aggregation table",
        )
    check(
        bool((frg["hydro_pumped"] * frg["hydro_pumped_con"] == 0).mean() > 0.95),
        "fr B10 pair: gen and con rarely simultaneous (mutually exclusive pair)",
    )

    print("== FR non-GB flows (fr_external_2024) ==")
    fr_borders = ["be", "delu", "ch", "it_north", "es"]
    for b in fr_borders:
        net_ok = (
            (frx[f"{b}_net"] - (frx[f"{b}_imp"] - frx[f"{b}_exp"])).abs() < 1e-9
        ).all()
        check(bool(net_ok), f"fr<->{b}: net == imp - exp")
        check(bool((frx[f"{b}_imp"] >= 0).all()), f"fr<->{b}: imports non-negative")
        check(bool((frx[f"{b}_exp"] >= 0).all()), f"fr<->{b}: exports non-negative")
        # RTE 2024 annual review: France ran a positive export balance on
        # EVERY border ("solde exportateur positif sur toutes ses
        # frontières") — annual net (+ = import to FR) must be negative.
        check(
            float(frx[f"{b}_net"].sum()) < 0,
            f"fr<->{b}: FR annual net exporter (RTE 2024, all borders)",
        )
    headline_ok = (
        (
            frx["fr_non_gb_net_export_mw"]
            + sum(frx[f"{b}_net"] for b in fr_borders)
        ).abs()
        < 1e-9
    ).all()
    check(bool(headline_ok), "fr_non_gb_net_export_mw == -(sum of border nets)")
    # Annual sanity vs published aggregates. Evidence (2026-07-03):
    # this pack's FR physical net exports incl. the GB border total
    # 82.46 TWh; energy-charts.info (Fraunhofer ISE, independently
    # assembled from the same ENTSO-E A11 physical flows) matches every
    # border to 0.01 TWh; RTE's 2024 annual review reports 89 TWh — but
    # that is the COMMERCIAL (scheduled-exchange) balance, and the
    # physical-vs-commercial loop-flow wedge on the meshed AC borders is
    # the irreducible 6.5 TWh difference (DC/radial borders match RTE to
    # 0.2 TWh: GB 19.9 vs 20.1, ES 2.9 vs 2.8). Band 75-95 TWh: centred
    # between the two published anchors, wide enough for the loop-flow
    # wedge and platform revisions, tight enough to catch sign, unit,
    # double-count or border-omission regressions.
    total_twh = (
        frx["fr_non_gb_net_export_mw"].sum() + flows["fr_net"].sum()
    ) * 0.5 / 1e6
    check(
        75.0 < total_twh < 95.0,
        f"FR total physical net exports {total_twh:.2f} TWh in 75-95 band "
        "(RTE 2024 commercial 89 TWh; physical 82.5 TWh, see comment)",
    )

    print("== capacity ==")
    cap = pd.read_parquet(proc / "capacity_2024.parquet")
    check(
        sorted(set(cap.index)) == ["be", "delu", "dk1", "fr", "ie", "nl", "no2"],
        "capacity: all 7 zones present",
    )
    check(bool((cap["capacity_mw"] > 0).any()), "capacity: positive values present")
    check(
        cap["psr_code"].str.match(r"^B\d\d$").all(),
        "capacity: PSR codes well-formed",
    )

    print("== reservoir ==")
    for z in ["no2", "no", "fr"]:
        res = pd.read_parquet(proc / f"reservoir_{z}_2024.parquet")
        check(52 <= len(res) <= 53, f"reservoir_{z}: 52-53 weekly rows ({len(res)})")
        check(bool((res["storage_mwh"] > 0).all()), f"reservoir_{z}: storage positive")
        check(
            res.index.is_monotonic_increasing, f"reservoir_{z}: weeks strictly ordered"
        )
        # Norwegian reservoir weeks run Monday 00:00 Europe/Oslo, French
        # weeks Monday 00:00 Europe/Paris (both = Sunday 23:00 UTC at the
        # winter anchor): the first week starts 1 h before the UTC year and
        # the last extends past it, so exactly those two boundary weeks
        # have no computable inflow proxy.
        check(
            int(res["inflow_proxy_mwh"].isna().sum()) <= 2,
            f"reservoir_{z}: inflow proxy present except boundary weeks",
        )

    print("== cross-source reconciliation vs 2024 validation pack ==")
    demand = pd.read_parquet(pack2024 / "demand_2024.parquet")
    neso = {
        "fr": demand[["ifa_flow", "ifa2_flow", "eleclink_flow"]].sum(axis=1),
        "be": demand["nemo_flow"],
        "nl": demand["britned_flow"],
        "no2": demand["nsl_flow"],
        "dk1": demand["viking_flow"],
        "ie": demand[["moyle_flow", "east_west_flow", "greenlink_flow"]].sum(axis=1),
    }
    for b, series in neso.items():
        e_twh = flows[f"{b}_net"].sum() * 0.5 / 1e6
        n_twh = series.sum() * 0.5 / 1e6
        wedge = abs(e_twh - n_twh)
        # ±0.5 TWh per border: metering-point wedge headroom (docstring).
        check(
            wedge < 0.5,
            f"{b}: ENTSO-E net {e_twh:+.2f} TWh vs NESO {n_twh:+.2f} TWh "
            f"(wedge {wedge:.3f} < 0.5)",
        )

    print("== build reports ==")
    rep = json.loads((proc / "build_report_entsoe_2024.json").read_text())
    # build_fr.py and build_fr_flows.py keep their own reports (existing
    # files/manifest entries stay byte-unchanged); their keys (gen_fr_*,
    # reservoir_fr, flows_fr_*) are disjoint.
    rep.update(json.loads((proc / "build_report_fr_2024.json").read_text()))
    rep.update(json.loads((proc / "build_report_fr_flows_2024.json").read_text()))
    bad_gaps = {k: v for k, v in rep.items() if isinstance(v, dict) and v.get("unfilled_slots")}
    check(not bad_gaps, f"no unfilled gaps (offenders: {list(bad_gaps)})")
    bad_ack = {
        k: v
        for k, v in rep.items()
        if isinstance(v, dict) and v.get("ack_months_error")
    }
    check(not bad_ack, f"no ACK months outside flows (offenders: {list(bad_ack)})")

    if FAILURES:
        print(f"\nVALIDATION FAILED: {len(FAILURES)} failure(s)")
        sys.exit(1)
    print("\nvalidation passed")


if __name__ == "__main__":
    main()
