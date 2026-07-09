#!/usr/bin/env python3
"""Derive Scotland / rest-of-GB zonal CF traces, 1985-2024 (B6 two-zone pack).

B6 two-zone work package (2026-07-04): split the GB per-technology CF
traces into a Scotland zone ("sco") and a rest-of-GB zone ("rgb",
England + Wales) for the intra-GB / B6-boundary study, from the SAME
committed GB cutouts (data/packs/era5/, manifests era5-2024.sha256 +
era5-1985-2023.sha256) with the SAME pinned derivation machinery.

THE METHOD IS THE GB METHOD (derive_cf.py, Phase A/B, reviewed and
pinned), applied to zone subsets of the SAME cluster lists. This script
IMPORTS the GB functions — pinned code reuse, not reimplementation;
derive_cf.py is byte-unchanged and every committed GB manifest stays
valid (the GB-total traces are not rewritten by this script).

ZONE ASSIGNMENT (the only new information; every cluster below is one of
the pinned GB clusters, assigned whole — weights and coordinates are
byte-identical to derive_cf.py):

  offshore  sco: moray_firth (1.9), forth_tay (1.3)          = 3.2 GW-w
            rgb: hornsea_1_2, dogger_bank_a, greater_wash,
                 east_anglia, thames, irish_sea              = 12.1 GW-w
            (Robin Rigg, ~0.17 GW, sits in Scottish waters but lives
            inside the irish_sea cluster point -> assigned rgb; a stated
            ~1.2%-of-fleet zone-assignment approximation.)
  onshore   sco: southern_uplands, central_belt, highlands,
                 argyll, ne_scotland                          = 10.6 GW-w
            rgb: wales, nw_england, ne_england, east_england,
                 sw_england                                   = 3.8 GW-w
  solar     sco: scotland (0.5)   rgb: the six E&W regions    = 18.2 GW-w

CALIBRATION CONVENTION (stated explicitly, the work-order requirement):
the pinned 2024 GB factors (offshore 0.8975, onshore 1.0395, solar
0.8837 — re-derived at full precision from the 2024 cutout and
drift-guarded exactly as Phase B does) apply UNCHANGED to both zones.
The GB calibration anchors are national (NESO/Elexon 2024 energies);
no independent zonal anchor of the same quality exists, and a per-zone
factor would break the reconstruction identity below. The traces are
zone-internal weighted means, so they are INDEPENDENT of how GB
capacity is later split between the zones; two split conventions are
verified/quantified here:

1. CLUSTER-WEIGHT shares (offshore 3.2/15.3 = 0.209150, onshore
   10.6/14.4 = 0.736111, solar 0.5/18.7 = 0.026738) — the derivation-
   correctness identity:

       w_sco * sco_trace + w_rgb * rgb_trace == gb_trace

   The weighted sum is linear and clipping cannot bite (max possible
   raw wind CF = PMAX*LOSSES = 0.855; x 1.0395 = 0.889 < 1; solar and
   offshore factors < 1), so the identity is exact up to arithmetic
   rounding — the cutout is float32, so the two aggregation orders
   differ at single precision (observed max 3.0e-07 over 40y) — and
   the script VERIFIES it per year per technology against the
   committed GB pack traces (data/packs/cf/gb_*_cf_<Y>.parquet).
   This check proves the zone split loses no information; it is NOT
   the recommended capacity split for onshore (see 2).

2. ADOPTED scenario split (supervisor decision 2026-07-04, review
   condition 1 — docs/notes/b6-two-zone-data-review.md §2): ONSHORE
   splits by the OBSERVED DESNZ end-2024 capacity share, Scotland
   0.6997 (DESNZ Regional Renewable Statistics MW2024: 10,281.06 /
   14,693.90 MW), because the cluster share (0.7361) overstates the
   observed Scottish share of GB onshore ENERGY (model 73.4% vs
   observed 69.8% in 2024, DESNZ generation workbook) — an
   anti-conservative bias for the B6 curtailment questions. Under the
   adopted split the model's 2024 Scottish onshore energy share is
   69.7% ~= observed 69.8%, at a quantified GB-energy cost (the
   weighted sum no longer reproduces gb_trace exactly): +0.05% (2024),
   +0.22% (40-year mean CFs), max +0.54% in any single year (1985) —
   reported per year as `adopted_split_gb_energy_rel` and sanity-
   bounded at 1% (2x the observed 40-year max; a breach means share
   or trace drift, not a tolerable wobble). OFFSHORE and SOLAR keep
   the cluster shares (offshore 0.2092 sits inside the observed
   20.3-25.6% bracket; solar's 2.7 vs 4.0% is immaterial at 0.75 GW).

Scenario-pairing rule: split the reference GB capacity by
ADOPTED_SPLIT_SHARES below. The onshore GB-energy deviation from the
validated single-zone runs is the stated, quantified convention cost;
carrying the cluster split instead requires the anti-conservative
caveat of the review, verbatim, on every Q2/Q10 output.

Outputs (data/packs/cf-gb2/): for zone z in {sco, rgb}, tech t in
{onshore, offshore, solar}, year Y in 1985-2024:

    {z}_{t}_cf_<Y>.{parquet,csv}     single float64 column `cf` in [0,1],
                                     `utc_start` half-hourly UTC index
                                     (17,520; 17,568 leap) — GB format
    gb2_cf_report.json               zone weights, factors, annual CFs,
                                     per-year reconstruction residuals

Deterministic: no network, no randomness, no wall-clock; outputs are a
pure function of the committed cutouts and the constants here + in
derive_cf.py. Attribution: "Contains modified Copernicus Climate Change
Service information [2024]".

Usage:
    python derive_cf_gb2zone.py <repo-root>                # 1985-2024
    python derive_cf_gb2zone.py <repo-root> --years 2024   # spot check
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import derive_cf as gb  # noqa: E402  (pinned GB method — reuse, no changes)

# Zone assignment: names must partition the pinned GB cluster lists.
ZONES = {
    "offshore": {
        "sco": {"moray_firth", "forth_tay"},
        "rgb": {
            "hornsea_1_2", "dogger_bank_a", "greater_wash",
            "east_anglia", "thames", "irish_sea",
        },
    },
    "onshore": {
        "sco": {
            "southern_uplands", "central_belt", "highlands",
            "argyll", "ne_scotland",
        },
        "rgb": {
            "wales", "nw_england", "ne_england", "east_england",
            "sw_england",
        },
    },
    "solar": {
        "sco": {"scotland"},
        "rgb": {
            "se_england", "sw_england", "east_england", "midlands",
            "n_england", "wales",
        },
    },
}

CLUSTERS = {
    "offshore": gb.OFFSHORE_CLUSTERS,
    "onshore": gb.ONSHORE_REGIONS,
    "solar": gb.SOLAR_REGIONS,
}

# The adopted scenario capacity split (Scotland share of GB), docstring
# convention 2. Onshore = DESNZ MW2024 observed share (0.6997, NOT the
# 0.7361 cluster share); offshore/solar = the cluster shares.
ADOPTED_SPLIT_SHARES = {
    "onshore": 0.6997,    # DESNZ MW2024: 10,281.06 / 14,693.90 MW
    "offshore": 0.209150,  # cluster share (inside the 20.3-25.6% bracket)
    "solar": 0.026738,     # cluster share (immaterial: 0.75 GW fleet)
}

# Sanity bound on the adopted-split GB-energy deviation (docstring
# convention 2): observed per-year range +0.05%..+0.54% over 1985-2024;
# 1% = ~2x the observed maximum. A breach means the shares or traces
# drifted — loud failure, never absorbed.
ADOPTED_DEV_TOL = 0.01

# Reconstruction tolerance — EVIDENCE-BASED, not aspirational: the cutout
# variables are float32 (ERA5 pack storage), and pandas keeps float32
# through the per-cluster power-curve/weighting arithmetic, so the GB
# aggregation order and the zone-then-combine order round differently at
# SINGLE precision. Observed max residual on 2024: 1.8e-07 (onshore);
# the tolerance sits ~50x above that and ~4 orders below any physical
# significance (CF quantum of interest ~1e-3).
RECON_TOL = 1e-5


def check_partition() -> dict:
    """Assert the zone sets exactly partition each pinned cluster list;
    return {(tech, zone): (subset_list, weight_share)}."""
    out = {}
    for tech, clusters in CLUSTERS.items():
        names = {n for n, *_ in clusters}
        sco, rgb = ZONES[tech]["sco"], ZONES[tech]["rgb"]
        if sco | rgb != names or sco & rgb:
            sys.exit(f"{tech}: zone sets do not partition the GB clusters")
        total = sum(w for *_, w in clusters)
        for z in ("sco", "rgb"):
            sub = [c for c in clusters if c[0] in ZONES[tech][z]]
            out[(tech, z)] = (sub, sum(w for *_, w in sub) / total)
    return out


def zone_raw(points: dict, tech: str, sub: list) -> pd.Series:
    """Raw hourly CF for one zone = the pinned GB per-tech pipeline run
    on the zone's cluster subset (identical mechanics, renormalised
    weights via gb.weighted_cf)."""
    if tech == "offshore":
        return gb.wind_hourly_cf(points, sub, gb.OFF_V0, gb.OFF_S,
                                 gb.OFF_PMAX, hub_factor=1.0)
    if tech == "onshore":
        return gb.wind_hourly_cf(points, sub, gb.ON_V0, gb.ON_S,
                                 gb.ON_PMAX,
                                 hub_factor=gb.ONSHORE_HUB_FACTOR)
    return gb.solar_hourly_cf(points, sub)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument("--years", default="1985-2024",
                    help="e.g. 1985-2024 or 2024")
    args = ap.parse_args()
    repo = args.repo_root
    years = gb.parse_cf_years(args.years)

    era5_root = repo / "data" / "packs" / "era5"
    gb_cf_dir = repo / "data" / "packs" / "cf"
    out_dir = repo / "data" / "packs" / "cf-gb2"

    for year in years:
        n = len(sorted((era5_root / str(year)).glob(f"era5_gb_{year}-*.parquet")))
        if n != 12:
            sys.exit(f"{year}: cutout incomplete ({n}/12 monthly files)")

    parts = check_partition()
    print("zone weight shares (of GB cluster weights):")
    for (tech, z), (_sub, share) in sorted(parts.items()):
        print(f"  {tech:9s} {z}: {share:.6f}")

    # Pinned factors, re-derived from the 2024 cutout with the Phase B
    # drift guard (gb.pinned_2024_factors exits on any drift).
    print("deriving 2024 raw traces (pinned-factor drift guard) ...")
    raw_2024_gb = gb.derive_raw(era5_root / "2024", 2024)
    factors = gb.pinned_2024_factors(raw_2024_gb)
    print("pinned factors confirmed: "
          + ", ".join(f"{n} {f:.4f}" for n, f in factors.items()))

    out_dir.mkdir(parents=True, exist_ok=True)
    report: dict = {
        "zone_weight_shares": {
            tech: {z: parts[(tech, z)][1] for z in ("sco", "rgb")}
            for tech in CLUSTERS
        },
        "calibration_factors_applied": {n: float(f) for n, f in factors.items()},
        "adopted_split_shares": dict(ADOPTED_SPLIT_SHARES),
        "annual_cf": {},
        "reconstruction_max_abs_diff": {},
        "reconstruction_annual_energy_rel": {},
        "adopted_split_gb_energy_rel": {},
    }

    for year in years:
        points = gb.load_point_means(
            era5_root / str(year),
            {tech: CLUSTERS[tech] for tech in CLUSTERS},
            year,
        )
        index = gb.half_hourly_index(year)
        for tech in ("onshore", "offshore", "solar"):
            zone_traces = {}
            for z in ("sco", "rgb"):
                sub, _share = parts[(tech, z)]
                pts = {n: points[tech][n] for n, *_ in sub}
                raw = gb.to_half_hourly(zone_raw(pts, tech, sub), index)
                if raw.isna().any():
                    sys.exit(f"{year} {z} {tech}: NaNs after interpolation")
                trace = (raw * factors[tech]).clip(0.0, 1.0)
                zone_traces[z] = trace
                gb.write(trace, out_dir, f"{z}_{tech}_cf_{year}")
                report["annual_cf"].setdefault(tech, {}).setdefault(z, {})[
                    str(year)
                ] = round(float(trace.mean()), 4)
            # Reconstruction identity vs the committed GB pack trace.
            w_sco = parts[(tech, "sco")][1]
            w_rgb = parts[(tech, "rgb")][1]
            recon = w_sco * zone_traces["sco"] + w_rgb * zone_traces["rgb"]
            gb_trace = pd.read_parquet(
                gb_cf_dir / f"gb_{tech}_cf_{year}.parquet"
            )["cf"]
            max_diff = float((recon.values - gb_trace.values).__abs__().max())
            rel_energy = float(
                recon.sum() / gb_trace.sum() - 1.0
            ) if gb_trace.sum() > 0 else 0.0
            report["reconstruction_max_abs_diff"].setdefault(tech, {})[
                str(year)
            ] = max_diff
            report["reconstruction_annual_energy_rel"].setdefault(tech, {})[
                str(year)
            ] = rel_energy
            if max_diff > RECON_TOL:
                sys.exit(
                    f"{year} {tech}: reconstruction residual {max_diff:.3e} "
                    f"exceeds {RECON_TOL} — zone split is not exact"
                )
            # Adopted-split deviation (docstring convention 2): the
            # quantified GB-energy cost of the DESNZ onshore share.
            w_a = ADOPTED_SPLIT_SHARES[tech]
            adopted = w_a * zone_traces["sco"] + (1 - w_a) * zone_traces["rgb"]
            dev = float(adopted.mean() / gb_trace.mean() - 1.0)
            report["adopted_split_gb_energy_rel"].setdefault(tech, {})[
                str(year)
            ] = dev
            if abs(dev) > ADOPTED_DEV_TOL:
                sys.exit(
                    f"{year} {tech}: adopted-split GB-energy deviation "
                    f"{dev:+.4%} exceeds {ADOPTED_DEV_TOL:.0%} — share or "
                    "trace drift"
                )
        print(f"{year}: written ({len(years) - years.index(year) - 1} to go)")

    # The report is only manifest-stable for the full sweep (EU precedent).
    if years == list(range(1985, 2025)):
        (out_dir / "gb2_cf_report.json").write_text(
            json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
        )
        print(f"report -> {out_dir / 'gb2_cf_report.json'}")
    else:
        print("partial-year run: gb2_cf_report.json NOT written (full-sweep only)")


if __name__ == "__main__":
    main()
