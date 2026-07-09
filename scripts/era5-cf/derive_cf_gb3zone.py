#!/usr/bin/env python3
"""Split the committed Scotland ("sco") zone into N-Scotland + S-Scotland.

Three-zone Scottish-boundary work package (2026-07-04, design ADOPTED-WITH-
EDITS: docs/notes/scottish-group-boundary-design-review.md). The committed
B6 two-zone pack (data/packs/cf-gb2/, scripts/era5-cf/derive_cf_gb2zone.py)
splits GB into sco (Scotland) + rgb (England+Wales). This script splits the
sco zone AGAIN, at the B4 line (N=710k OSGB northing, the SSEN-T <-> SPT
interface, Tealing-Westfield), into:

  nsco  = N-Scotland  (north of B4, SSEN)
  ssco  = S-Scotland  (B4 -> B6, SPT)

The E+W zone (ew) = the committed rgb trace, byte-UNCHANGED (reused from
cf-gb2, NOT re-derived here). The three-zone geometry is therefore
{nsco, ssco, rgb} joined by a B4 link (nsco->ssco) and a B6 link
(ssco->rgb).

THE METHOD IS THE PINNED GB METHOD (derive_cf.py), applied to sub-subsets
of the SAME pinned GB cluster lists. This script IMPORTS the GB functions;
derive_cf.py AND derive_cf_gb2zone.py are byte-unchanged, and every
committed GB / cf-gb2 manifest stays valid (this script writes ONLY new
nsco_* / ssco_* files + gb3_cf_report.json; it rewrites nothing committed).

ZONE ASSIGNMENT WITHIN sco (the only new information). Onshore clusters
split cleanly by cluster-point latitude across the B4 line; the two Scottish
OFFSHORE clusters do NOT: moray_firth is wholly north, but forth_tay
STRADDLES B4 (Seagreen ~N749k north of the line; the Firth-of-Forth members
south). Because a pinned CF cluster is a SINGLE pre-averaged 3x3-box ERA5
point (verified: derive_cf.OFFSHORE_CLUSTERS lists forth_tay as one
(name,lat,lon,gw) tuple), there is NO sub-cluster CF granularity to descend
into without new per-cell ERA5 work. The within-cluster split is therefore
a CAPACITY-WEIGHT split with a SHARED CF shape (design-review item 6a
resolution: reported, not silently claimed as point-resolved):

  offshore  nsco: moray_firth (1.9)  + forth_tay (1.3 * F_N)
            ssco: forth_tay (1.3 * F_S)          [trace == forth_tay]
  onshore   nsco: highlands (2.5), ne_scotland (0.8)          = 3.3 GW-w
            ssco: southern_uplands (4.0), central_belt (2.5),
                  argyll (0.8)                                 = 7.3 GW-w
  solar     nsco: scotland (0.5 * S_N)   ssco: scotland (0.5 * S_S)
            [trace == the single scotland solar point; split is weight-only]

forth_tay within-cluster split (F_N / F_S) — PINNED from REPD operational
northings, NOT tuned to the B4 DA series (design-review item 3 guard). Of
forth_tay's operational (<=2024-12-31) members, only Levenmouth (7 MW,
N696860 south) sits south of B4; Seagreen (1075), Kincardine (49.5),
Hywind (30) and Aberdeen Bay/EOWDC (96.8) are north. NnG (450 MW, Firth of
Forth, south) reached FULL commercial operation July 2025 and contributes
0 MW to the end-2024 validation fleet -> excluded, exactly as the committed
B6 pack excludes it (b6 report §2 note-a). Hence F_S = 7 / ~1258 = 0.006
(forth_tay 99.4% north). The NnG placement is carried as a documented
forward wedge (S-Scotland offshore -> ~0.46 GW when NnG is included), NOT
pinned into the 2024 fleet. This reconciles to the REPD-northing offshore
FLEET split (design-review item 6b): 94% of Scottish offshore is north of
B4; the CF-cluster split places all but ~7 MW north, the ~174 MW residual
being Robin Rigg (Scottish waters, but in the irish_sea CF cluster -> rgb).

solar N/S split (S_N / S_S) — PINNED from REPD operational solar northings
(Scotland N-of-B4 solar 58.1 MW of 83.8 MW = 0.694). Immaterial (0.5 GW
Scottish solar); the scotland solar point (56.0N ~N700k) sits on the line
so both sub-zones share its CF shape; the split is weight-only.

CALIBRATION: the pinned 2024 GB factors (offshore 0.8975, onshore 1.0395,
solar 0.8837, re-derived at full precision with the Phase B drift guard)
apply UNCHANGED to all zones, exactly as cf-gb2 does.

RECONSTRUCTION IDENTITY (verified per year per tech against the COMMITTED
cf-gb2 sco traces, not just GB): by construction the sub-zone weights
partition the sco cluster weights, so

    w_nsco * nsco_trace + w_ssco * ssco_trace == sco_trace  (committed)

is exact up to float32-cutout arithmetic rounding. Transitively with the
committed cf-gb2 identity (w_sco*sco + w_rgb*rgb == gb) the three-zone split
reconstructs the GB total. The script asserts this and reports the residual.

ADOPTED-SPLIT DEVIATION (design-review item 6b reconciliation cost): the
scenario allocates zonal CAPACITY by the REPD-northing shares (onshore
0.408 north, offshore 0.939 north), which differ from the cluster-weight
shares (onshore 0.311, offshore ~0.994). Re-combining the sub-zone traces
by the REPD-northing capacity shares no longer reproduces the committed sco
trace exactly; that within-Scotland GB-energy deviation is reported per year
as `adopted_split_sco_energy_rel` (it is small because both Scottish
sub-zone traces sit at similar CF levels) and its SIGN is the anti-
conservative check the B6 package established.

Outputs (data/packs/cf-gb2/, additive): for z in {nsco, ssco}, tech in
{onshore, offshore, solar}, year 1985-2024:

    {z}_{t}_cf_<Y>.{parquet,csv}   float64 `cf` in [0,1], utc_start index
    gb3_cf_report.json             sub-zone weights, forth_tay/solar splits,
                                   annual CFs, reconstruction + adopted-split
                                   residuals vs the committed sco traces

Deterministic; no network, no randomness, no wall-clock. Attribution:
"Contains modified Copernicus Climate Change Service information [2024]".

Usage:
    python derive_cf_gb3zone.py <repo-root>                # 1985-2024
    python derive_cf_gb3zone.py <repo-root> --years 2024   # spot check
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np  # noqa: F401  (kept for parity / potential diagnostics)
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import derive_cf as gb  # noqa: E402  (pinned GB method — reuse, no changes)

# forth_tay within-cluster split (docstring): south = Levenmouth 7 MW of
# ~1258 MW operational forth_tay members -> F_S pinned 0.006. NnG (450 MW,
# operational July 2025) excluded from the end-2024 fleet.
FORTH_TAY_F_S = 0.006
FORTH_TAY_F_N = 1.0 - FORTH_TAY_F_S

# scotland solar N/S split (docstring): REPD operational solar northings,
# Scotland N-of-B4 = 58.1 MW of 83.8 MW = 0.694. Weight-only; immaterial.
SOLAR_S_N = 0.694
SOLAR_S_S = 1.0 - SOLAR_S_N

# The ADOPTED scenario capacity split within Scotland (N share of the sco
# fleet), from REPD operational <=2024-12-31 northings at N=710k. Used ONLY
# to quantify the reconciliation cost (adopted_split_sco_energy_rel); the
# TRACES are cluster-based. Onshore 4006.7/9826.6; offshore CF-cluster north
# share (all but Levenmouth) ~0.994; solar 0.694.
ADOPTED_N_SHARE = {"onshore": 0.4078, "offshore": 0.994, "solar": 0.694}

# Sub-zone cluster membership within sco (each name is a pinned GB cluster).
# offshore forth_tay and solar scotland are WEIGHT-SPLIT across both zones.
SUBZONES = {
    "offshore": {
        "nsco": {"moray_firth": 1.0, "forth_tay": FORTH_TAY_F_N},
        "ssco": {"forth_tay": FORTH_TAY_F_S},
    },
    "onshore": {
        "nsco": {"highlands": 1.0, "ne_scotland": 1.0},
        "ssco": {"southern_uplands": 1.0, "central_belt": 1.0, "argyll": 1.0},
    },
    "solar": {
        "nsco": {"scotland": SOLAR_S_N},
        "ssco": {"scotland": SOLAR_S_S},
    },
}

# The pinned sco cluster set (must equal cf-gb2's sco assignment — the union
# of the two sub-zones' clusters).
SCO_CLUSTERS = {
    "offshore": {"moray_firth", "forth_tay"},
    "onshore": {"southern_uplands", "central_belt", "highlands",
                "argyll", "ne_scotland"},
    "solar": {"scotland"},
}

CLUSTERS = {
    "offshore": gb.OFFSHORE_CLUSTERS,
    "onshore": gb.ONSHORE_REGIONS,
    "solar": gb.SOLAR_REGIONS,
}

# Reconstruction tolerance — EVIDENCE-BASED, identical basis to cf-gb2's
# RECON_TOL: float32 cutout arithmetic rounds the GB aggregation order and
# the sub-zone-then-combine order differently at single precision (cf-gb2
# observed max 3.0e-07 over 40y). 1e-5 sits ~50x above that and ~4 orders
# below any physical significance (CF quantum of interest ~1e-3).
RECON_TOL = 1e-5


def cluster_tuple(tech: str, name: str) -> tuple:
    """The pinned (name, lat, lon, gw) tuple for one GB cluster."""
    for c in CLUSTERS[tech]:
        if c[0] == name:
            return c
    sys.exit(f"{tech}: cluster {name} not in the pinned GB list")


def point_cf(points: dict, tech: str, name: str) -> pd.Series:
    """Raw hourly CF for ONE cluster point (single-member weighted_cf == the
    point's own CF), using the pinned per-tech pipeline."""
    tup = cluster_tuple(tech, name)
    sub = [tup]
    pts = {name: points[tech][name]}
    if tech == "offshore":
        return gb.wind_hourly_cf(pts, sub, gb.OFF_V0, gb.OFF_S,
                                 gb.OFF_PMAX, hub_factor=1.0)
    if tech == "onshore":
        return gb.wind_hourly_cf(pts, sub, gb.ON_V0, gb.ON_S, gb.ON_PMAX,
                                 hub_factor=gb.ONSHORE_HUB_FACTOR)
    return gb.solar_hourly_cf(pts, sub)


def subzone_weight_of_sco(tech: str, zone: str) -> float:
    """Weight of a sub-zone as a fraction of the sco cluster-weight total,
    honouring the forth_tay / solar within-cluster weight splits."""
    total = sum(gw for name, *_ , gw in CLUSTERS[tech]
                if name in SCO_CLUSTERS[tech])
    w = 0.0
    for name, frac in SUBZONES[tech][zone].items():
        w += cluster_tuple(tech, name)[3] * frac
    return w / total


def check_partition() -> None:
    """Assert the two sub-zones' clusters union to the committed sco set and
    that the weight-split fractions of any shared cluster sum to 1."""
    for tech in CLUSTERS:
        names = set()
        fracs: dict = {}
        for zone in ("nsco", "ssco"):
            for name, frac in SUBZONES[tech][zone].items():
                names.add(name)
                fracs[name] = fracs.get(name, 0.0) + frac
        if names != SCO_CLUSTERS[tech]:
            sys.exit(f"{tech}: sub-zones do not cover the committed sco set")
        for name, tot in fracs.items():
            if abs(tot - 1.0) > 1e-12:
                sys.exit(f"{tech} {name}: split fractions sum to {tot}, not 1")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument("--years", default="1985-2024")
    args = ap.parse_args()
    repo = args.repo_root
    years = gb.parse_cf_years(args.years)

    era5_root = repo / "data" / "packs" / "era5"
    gb2_cf_dir = repo / "data" / "packs" / "cf-gb2"     # committed sco traces
    out_dir = repo / "data" / "packs" / "cf-gb2"        # additive: nsco_/ssco_

    for year in years:
        n = len(sorted((era5_root / str(year)).glob(
            f"era5_gb_{year}-*.parquet")))
        if n != 12:
            sys.exit(f"{year}: cutout incomplete ({n}/12 monthly files)")

    check_partition()
    print("sub-zone weight shares (of the committed sco cluster weight):")
    weights = {}
    for tech in CLUSTERS:
        for zone in ("nsco", "ssco"):
            weights[(tech, zone)] = subzone_weight_of_sco(tech, zone)
            print(f"  {tech:9s} {zone}: {weights[(tech, zone)]:.6f}")

    print("deriving 2024 raw traces (pinned-factor drift guard) ...")
    raw_2024_gb = gb.derive_raw(era5_root / "2024", 2024)
    factors = gb.pinned_2024_factors(raw_2024_gb)
    print("pinned factors confirmed: "
          + ", ".join(f"{n} {f:.4f}" for n, f in factors.items()))

    out_dir.mkdir(parents=True, exist_ok=True)
    report: dict = {
        "b4_line_northing": 710000,
        "subzone_weight_shares_of_sco": {
            tech: {z: weights[(tech, z)] for z in ("nsco", "ssco")}
            for tech in CLUSTERS
        },
        "forth_tay_within_cluster_split": {
            "f_north": FORTH_TAY_F_N, "f_south": FORTH_TAY_F_S,
            "basis": "REPD operational <=2024-12-31 northings; south="
                     "Levenmouth 7 MW; NnG 450 MW excluded (full CoD 2025-07)",
        },
        "solar_within_cluster_split": {"s_north": SOLAR_S_N,
                                       "s_south": SOLAR_S_S},
        "calibration_factors_applied": {n: float(f) for n, f in factors.items()},
        "adopted_capacity_n_share_repd_northing": dict(ADOPTED_N_SHARE),
        "annual_cf": {},
        "reconstruction_max_abs_diff_vs_committed_sco": {},
        "reconstruction_annual_energy_rel_vs_committed_sco": {},
        "adopted_split_sco_energy_rel": {},
    }

    for year in years:
        points = gb.load_point_means(
            era5_root / str(year),
            {tech: CLUSTERS[tech] for tech in CLUSTERS},
            year,
        )
        index = gb.half_hourly_index(year)
        for tech in ("onshore", "offshore", "solar"):
            # Per-cluster raw point CFs for the sco clusters.
            pcf = {name: point_cf(points, tech, name)
                   for name in SCO_CLUSTERS[tech]}
            zone_traces = {}
            for zone in ("nsco", "ssco"):
                members = SUBZONES[tech][zone]
                wsum = sum(cluster_tuple(tech, n)[3] * f
                           for n, f in members.items())
                raw = sum(pcf[n] * (cluster_tuple(tech, n)[3] * f / wsum)
                          for n, f in members.items())
                raw = gb.to_half_hourly(raw, index)
                if raw.isna().any():
                    sys.exit(f"{year} {zone} {tech}: NaNs after interpolation")
                trace = (raw * factors[tech]).clip(0.0, 1.0)
                zone_traces[zone] = trace
                gb.write(trace, out_dir, f"{zone}_{tech}_cf_{year}")
                report["annual_cf"].setdefault(tech, {}).setdefault(
                    zone, {})[str(year)] = round(float(trace.mean()), 4)

            # Reconstruction vs the COMMITTED cf-gb2 sco trace.
            w_n = weights[(tech, "nsco")]
            w_s = weights[(tech, "ssco")]
            recon = w_n * zone_traces["nsco"] + w_s * zone_traces["ssco"]
            sco = pd.read_parquet(
                gb2_cf_dir / f"sco_{tech}_cf_{year}.parquet")["cf"]
            max_diff = float((recon.values - sco.values).__abs__().max())
            rel = float(recon.sum() / sco.sum() - 1.0) if sco.sum() > 0 else 0.0
            report["reconstruction_max_abs_diff_vs_committed_sco"].setdefault(
                tech, {})[str(year)] = max_diff
            report[
                "reconstruction_annual_energy_rel_vs_committed_sco"
            ].setdefault(tech, {})[str(year)] = rel
            if max_diff > RECON_TOL:
                sys.exit(f"{year} {tech}: reconstruction residual "
                         f"{max_diff:.3e} exceeds {RECON_TOL} vs committed sco")

            # Adopted-split (REPD-northing capacity) deviation vs sco: how far
            # the capacity-weighted recombination drifts from the cluster
            # recombination (the item-6b reconciliation cost).
            a_n = ADOPTED_N_SHARE[tech]
            adopted = a_n * zone_traces["nsco"] + (1 - a_n) * zone_traces["ssco"]
            dev = float(adopted.mean() / sco.mean() - 1.0) if sco.mean() > 0 \
                else 0.0
            report["adopted_split_sco_energy_rel"].setdefault(tech, {})[
                str(year)] = dev
        print(f"{year}: written ({len(years) - years.index(year) - 1} to go)")

    if years == list(range(1985, 2025)):
        (out_dir / "gb3_cf_report.json").write_text(
            json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
        )
        print(f"report -> {out_dir / 'gb3_cf_report.json'}")
    else:
        print("partial-year run: gb3_cf_report.json NOT written (full-sweep only)")


if __name__ == "__main__":
    main()
