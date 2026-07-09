#!/usr/bin/env python3
"""Derive external-zone (EU) per-country CF and temperature traces, 1985-2024.

Stage 5 work order (2026-07-03): per-country wind/solar capacity-factor
traces plus population-weighted temperature for GB's import counterparties,
derived from the committed EU weather pack (data/packs/era5-eu/, manifest
era5-eu-1985-2024.sha256, box 42-72N 11W-16E, u100/v100/ssrd/t2m,
Earthmover snapshot 39TK56WX185WZ1HP9WNG — single-source, no decode seam).

THE METHOD IS THE GB METHOD (scripts/era5-cf/derive_cf.py, Phase A/B,
reviewed and pinned) with per-country spatial weights and a different
calibration anchor. This script IMPORTS the GB power-curve, PV model,
interpolation and calibration functions — pinned code reuse, not
reimplementation. derive_cf.py itself is byte-unchanged; the GB derivation
path and all committed GB manifests are untouched.

Countries and series (dir data/packs/cf-eu/<country>/):

    fr, be, nl, de, dk1        {c}_{onshore,offshore,solar}_cf_<Y>.{parquet,csv}
    ie                         {c}_{onshore,solar}_cf_<Y>.{parquet,csv}
    fr..ie, no2                {c}_t2m_<Y>.{parquet,csv}

- CF traces: single float64 column `cf` in [0,1], `utc_start` half-hourly
  UTC index (17,520 periods; 17,568 leap) — the GB trace format exactly.
- Temperature traces: single float64 column `t2m_c` (degrees Celsius),
  same index — population-weighted 2 m temperature (approximate city/
  region weights), for temperature-driven neighbour demand modelling.
- `de` is the DE-LU bidding zone (Luxembourg included — it is inside the
  zone GB-relevant flows see; a Luxembourg temperature point is included).
- `dk1` is western Denmark only (Jutland + Funen, the Viking Link zone):
  weather points stay west of the Great Belt; Anholt OWF (Kattegat) is
  included because it grid-connects to Jutland (DK1); Zealand and
  Bornholm (DK2) are excluded.
- `ie` is the island of Ireland (all-island, matching the SEM market and
  ENTSO-E's IE(SEM) zone) — points cover both IE and NI. No offshore
  series: SEM offshore in 2024 is Arklow Bank (~25 MW), immaterial, and
  ENTSO-E lists no SEM B18 capacity.
- `no2` gets ONLY temperature: the Norwegian zone is hydro-driven from
  ENTSO-E data (docs/notes/entsoe-stage5-pack-report.md §6); NO2 wind is
  deliberately out of scope (D5 note).

DEVIATIONS FROM THE GB PIPELINE (complete list; everything else pinned):
1. Spatial weights: per-country approximate fleet-location weights
   (public-knowledge regional statistics and named offshore clusters —
   same honesty level as the GB UKWED-style weights; every cluster listed
   below with its assumed GW weight). 3x3-cell ERA5 box means, weights
   normalised, only relative sizes matter — identical mechanics.
2. Calibration anchor: ENTSO-E Transparency Platform 2024 actual
   generation per production type (A75, aggregation_gen_2024 built by
   scripts/fetch-entsoe/build_gen_agg.py) over A68 installed capacity —
   one multiplicative factor per technology per country so the 2024
   annual energy of trace x A68 capacity matches the observed 2024
   energy. GB used the NESO/Elexon pack instead. Licence: ENTSO-E
   generation/capacity are NOT on the CC-BY free-re-use list; this is
   the clause-3.1 internal-anchor use (fetch-and-build, never
   redistributed; attribution "Source: ENTSO-E Transparency Platform").
3. Calibration honesty policy (extends the GB [0.7, 1.3] band rule): a
   factor outside the band is treated as an ANCHOR data finding, not
   absorbed — the trace ships UNCALIBRATED (factor 1.0) and the computed
   factor + diagnosis are recorded in the report. 2024 evidence:
   NL solar (anchor 487 GWh vs ~28 GW — distributed generation missing
   from the platform), NL onshore (implied CF 0.126, ~half plausible —
   same disease), IE onshore (implied CF 0.508 — all-island generation
   over a stale 3.0 GW capacity). IE solar has NO anchor at all (A75
   series starts 2024-11-13, A68 lists no capacity) -> uncalibrated.
3a. CBS recalibration of NL onshore + NL solar (2026-07-03, the
   eu-cf-review ruling-1 escalation trigger adjudicated in
   docs/notes/stage-5-review.md addendum ruling 3): the two NL series
   are anchored to CBS (Statistics Netherlands) national statistics
   instead of ENTSO-E A75 — observed-statistics anchoring, not input
   tuning. Anchors read from data/packs/cbs-2024/processed/ (fetched
   and built by scripts/fetch-cbs/, CC BY 4.0, retrieved 2026-07-03,
   2024 status "NaderVoorlopig" = revised provisional):
   - NL onshore: net generation 17,657 GWh (StatLine 82610NED) over
     6,955 MW end-2024 capacity — the SAME number A68 carries, so the
     pairing rule is unchanged. Net (not gross 18,021 GWh) matches the
     A75 convention: for the trusted NL offshore series A75 15,203.9
     GWh sits 0.14% from CBS net 15,182 vs 2.0% from gross.
   - NL solar: generation 21,822 GWh (82610NED, all sectors: dwellings
     9,589 + economic activities 12,233, table 85005NED) over
     27,979.732 MWp DC panel capacity (85005NED) — numerically A68's
     27,980 MW, revealing A68's NL solar figure as the CBS DC panel
     capacity; the DC denominator is also the physically consistent
     one for the PV model (CF is referenced to STC panel rating).
     CBS's 24,772 MW (82610NED) is AC-side — NOT the paired capacity.
   Both factors land inside the honesty band; the ENTSO-E A75 values
   are echoed in the report to quantify the platform under-report
   (onshore 7,653.9/17,657 = 43%; solar 487.4/21,822 = 2% captured).
   End-of-year denominators overstate the within-year average fleet
   (solar grew ~14% in 2024); the factor absorbs that, per the GB
   Phase B semantics — the pairing rule below is what keeps it honest.
4. Temperature series are new (GB has none yet): instantaneous t2m,
   population-weighted, converted to Celsius, hourly -> half-hourly by
   the same linear interpolation as wind.
5. The cutout loader reads the EU pack layout and computes the 3x3 box
   means via a per-file pivot (13,189 cells vs GB's 2,009; arithmetic is
   the same mean over the same cells, in float64).

CALIBRATION SEMANTICS (the GB Phase B decision, restated for the EU):
the 2024 factors apply UNCHANGED to all years. A year-Y trace answers
"what would the A68-2024 fleet have produced in year Y's weather?" — NOT
"what did year Y's actual fleet produce?". Consequence, stated plainly:
the calibrated traces are energy-matching CFs for the A68-2024
capacities and must be paired with THOSE capacities (or deliberately
rescaled) in scenario work. Where A68 capacity is inconsistent with the
within-year fleet (FR offshore: commissioning year, implied CF 0.449),
trace x A68 capacity still reproduces observed 2024 energy by
construction — the pairing rule is what keeps it honest.

The derived factors must round (4 dp) to PINNED_FACTORS_EU below; an
empty dict means the first (pinning) run, which prints the values to pin.

Deterministic: no network, no randomness, no wall-clock. Outputs are a
pure function of the EU pack, the ENTSO-E anchor tables, and the
constants below. Attribution (ERA5): "Contains modified Copernicus
Climate Change Service information [2024]".

Usage:
    python derive_cf_eu.py <repo-root>                    # all years 1985-2024
    python derive_cf_eu.py <repo-root> --years 2024       # calibration year only
"""

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np
import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import derive_cf as gb  # noqa: E402  (pinned GB method — reuse, no changes)

N_CELLS_EU = 13_189
VARS = ["u100", "v100", "ssrd", "t2m"]
HOURS_2024 = 8_784

# Countries deriving CF; techs per country. `de` anchors on the DE-LU zone.
CF_COUNTRIES = {
    "fr": ("fr", ["onshore", "offshore", "solar"]),
    "be": ("be", ["onshore", "offshore", "solar"]),
    "nl": ("nl", ["onshore", "offshore", "solar"]),
    "de": ("delu", ["onshore", "offshore", "solar"]),
    "dk1": ("dk1", ["onshore", "offshore", "solar"]),
    "ie": ("ie", ["onshore", "solar"]),  # no offshore: ~25 MW Arklow only
}
TEMP_COUNTRIES = ["fr", "be", "nl", "de", "dk1", "ie", "no2"]
TECH_PSR = {"onshore": "B19", "offshore": "B18", "solar": "B16"}

# Pinned applied calibration factors (4 dp), written after the reviewed
# first 2024 calibration run of 2026-07-03 — the same drift-guard
# mechanism as derive_cf.PINNED_FACTORS_2024. 1.0 entries are the
# UNCALIBRATED sentinels of docstring deviation 3 (anchor unusable or
# absent), NOT physical calibrations. NL onshore/solar re-pinned
# 2026-07-03 to the CBS anchors of deviation 3a (previously 1.0).
PINNED_FACTORS_EU: dict[str, dict[str, float]] = {
    "fr": {"onshore": 1.0087, "offshore": 1.0, "solar": 1.1974},
    "be": {"onshore": 0.8342, "offshore": 1.0117, "solar": 1.0883},
    "nl": {"onshore": 0.8975, "offshore": 0.9597, "solar": 0.8735},
    "de": {"onshore": 0.9716, "offshore": 0.8195, "solar": 0.8750},
    "dk1": {"onshore": 0.7441, "offshore": 1.0749, "solar": 1.1015},
    "ie": {"onshore": 1.0, "solar": 1.0},
}

# ---------------------------------------------------------------------------
# Per-country fleet spatial weights — ALL APPROXIMATE, public-knowledge
# regional statistics (national statistics offices / TSO overviews /
# WindEurope-level knowledge; not a licensed site database), same honesty
# level as the GB UKWED-style weights. (name, lat degN, lon degE,
# weight_gw). Weights are normalised; only relative sizes matter (the
# absolute scale is set — or explicitly not set — by calibration).
# ---------------------------------------------------------------------------

ONSHORE = {
    # France ~22 GW: north/northeast-heavy (Hauts-de-France + Grand Est
    # ~half the fleet), a Mediterranean pocket in Aude/Hérault.
    "fr": [
        ("hauts_de_france", 50.2, 2.6, 5.9),
        ("grand_est", 48.9, 4.6, 4.9),
        ("occitanie_aude", 43.3, 2.6, 1.7),
        ("centre_val_de_loire", 47.9, 1.8, 1.5),
        ("brittany", 48.2, -3.0, 1.3),
        ("pays_de_la_loire", 47.5, -1.2, 1.3),
        ("normandy", 49.5, 0.8, 1.0),
        ("nouvelle_aquitaine", 45.9, 0.2, 1.3),
        ("bourgogne_franche_comte", 47.4, 4.3, 1.0),
    ],
    # Belgium ~3 GW: Flanders (ports/canals) slightly ahead of Wallonia.
    "be": [
        ("flanders", 51.05, 4.0, 1.7),
        ("wallonia", 50.5, 4.8, 1.4),
    ],
    # Netherlands ~7 GW: Flevoland/IJsselmeer polders, Groningen/
    # Friesland, Zeeland/Zuid-Holland delta, Noord-Holland.
    "nl": [
        ("flevoland_ijsselmeer", 52.55, 5.55, 1.9),
        ("groningen_friesland", 53.3, 6.5, 1.9),
        ("zeeland_zuid_holland", 51.7, 4.1, 1.6),
        ("noord_holland", 52.75, 4.85, 1.0),
        ("inland_rest", 52.2, 5.9, 0.6),
    ],
    # Germany ~60 GW: strongly north-heavy (SH, Lower Saxony,
    # Brandenburg, Saxony-Anhalt, NRW, MV), thin in the south.
    "de": [
        ("schleswig_holstein", 54.4, 9.3, 8.5),
        ("niedersachsen_west", 52.9, 7.8, 6.5),
        ("niedersachsen_east", 52.8, 9.5, 6.0),
        ("brandenburg", 52.5, 13.2, 8.0),
        ("sachsen_anhalt", 51.9, 11.6, 5.4),
        ("nrw", 51.7, 7.3, 6.9),
        ("mecklenburg_vorpommern", 53.9, 12.6, 3.7),
        ("rheinland_pfalz_saar", 49.9, 7.6, 3.9),
        ("hessen_thueringen", 50.8, 9.6, 3.6),
        ("sued_bayern_bw", 48.9, 10.2, 2.5),
    ],
    # DK1 (Jutland + Funen only — the geographic cut): west-coast heavy.
    "dk1": [
        ("west_jutland", 56.1, 8.4, 1.6),
        ("north_jutland", 57.1, 9.9, 0.9),
        ("south_jutland", 55.1, 9.1, 1.0),
        ("east_jutland_funen", 55.7, 10.0, 0.6),
    ],
    # Island of Ireland ~5.9 GW true all-island distribution (SEM):
    # Atlantic-facing counties dominate; NI clusters in Tyrone/Antrim.
    "ie": [
        ("kerry_cork", 52.0, -9.3, 1.4),
        ("mayo_galway", 53.8, -9.1, 1.0),
        ("donegal", 54.9, -8.0, 0.6),
        ("clare_limerick_tipperary", 52.7, -8.7, 0.8),
        ("midlands_offaly", 53.2, -7.8, 0.7),
        ("ni_tyrone_west", 54.7, -7.2, 0.9),
        ("ni_antrim_east", 54.9, -6.2, 0.5),
        ("wexford_se", 52.4, -6.6, 0.3),
    ],
}

OFFSHORE = {
    # France ~1.5 GW physically end-2024 (A68 lists 1.0): the three
    # commissioned farms. Atlantic/Channel, not North Sea.
    "fr": [
        ("saint_nazaire", 47.15, -2.6, 0.48),
        ("fecamp", 49.9, 0.2, 0.50),
        ("saint_brieuc", 48.9, -2.5, 0.50),
    ],
    # Belgium ~2.3 GW: one compact zone off Zeebrugge (Thornton Bank /
    # Northwind / Norther / SeaMade cluster).
    "be": [
        ("belgian_north_sea", 51.6, 2.8, 2.3),
    ],
    # Netherlands ~4.7 GW: Borssele, Hollandse Kust (Zuid+Noord), Gemini
    # far north, plus the older IJmuiden-area farms.
    "nl": [
        ("borssele", 51.7, 3.05, 1.5),
        ("hollandse_kust", 52.35, 4.1, 2.3),
        ("gemini", 54.0, 5.95, 0.6),
        ("egmond_amalia_luchterduinen", 52.55, 4.35, 0.35),
    ],
    # Germany ~8.5 GW: German Bight cluster dominates; Baltic (Arkona/
    # Wikinger/Baltic 1+2) smaller.
    "de": [
        ("german_bight", 54.2, 6.6, 7.0),
        ("baltic_arkona", 54.7, 13.6, 1.5),
    ],
    # DK1 ~1.6 GW: Horns Rev 1-3, Vesterhav Syd/Nord, Anholt (Kattegat,
    # DK1-connected). DK2's Baltic farms excluded.
    "dk1": [
        ("horns_rev", 55.5, 7.9, 0.78),
        ("vesterhav_nissum", 56.5, 8.1, 0.35),
        ("anholt_kattegat", 56.6, 11.2, 0.40),
    ],
}

SOLAR = {
    # France ~18 GW A68: strongly south-heavy.
    "fr": [
        ("nouvelle_aquitaine", 44.7, -0.2, 4.0),
        ("occitanie", 43.7, 2.2, 3.2),
        ("paca", 43.6, 5.8, 2.3),
        ("auvergne_rhone_alpes", 45.5, 4.8, 1.6),
        ("grand_est", 48.6, 4.5, 1.6),
        ("pays_de_la_loire", 47.4, -1.0, 1.0),
        ("centre_ile_de_france", 48.2, 2.2, 1.5),
    ],
    # Belgium ~9 GW: rooftop-dominated, Flanders ~2/3.
    "be": [
        ("flanders", 51.05, 4.4, 6.0),
        ("wallonia", 50.45, 4.8, 2.8),
    ],
    # Netherlands ~28 GW A68: broadly distributed, Brabant/east-heavy.
    "nl": [
        ("noord_brabant_limburg", 51.5, 5.5, 8.0),
        ("zuid_holland_utrecht", 52.0, 4.8, 6.0),
        ("gelderland_overijssel", 52.3, 6.1, 6.0),
        ("groningen_drenthe_friesland", 53.0, 6.4, 5.0),
        ("noord_holland_flevoland", 52.6, 5.0, 3.0),
    ],
    # Germany ~77 GW A68: Bavaria + BW ~40%, east and NRW substantial.
    "de": [
        ("bayern", 48.8, 11.6, 19.5),
        ("baden_wuerttemberg", 48.6, 9.1, 9.0),
        ("nrw", 51.3, 7.4, 8.5),
        ("niedersachsen", 52.8, 9.3, 7.0),
        ("brandenburg_berlin", 52.4, 13.4, 7.5),
        ("sachsen_sachsen_anhalt", 51.4, 12.4, 8.0),
        ("rheinland_pfalz_hessen_saar", 49.9, 8.0, 7.5),
        ("mecklenburg_vorpommern", 53.7, 12.3, 4.0),
        ("schleswig_holstein_hamburg", 54.0, 9.9, 4.0),
    ],
    # DK1 ~2.7 GW: the big parks sit in southern/western Jutland.
    "dk1": [
        ("south_jutland", 55.0, 9.1, 1.5),
        ("mid_west_jutland", 56.1, 8.9, 0.7),
        ("funen", 55.3, 10.3, 0.5),
    ],
    # Island of Ireland ~1.5 GW end-2024 (mostly ROI southeast; NO
    # ENTSO-E anchor — see docstring deviation 3).
    "ie": [
        ("leinster_se", 52.6, -6.7, 0.8),
        ("munster_south", 51.9, -8.3, 0.5),
        ("midlands_east", 53.4, -6.9, 0.4),
        ("ni", 54.5, -6.4, 0.1),
    ],
}

# Population weights (approximate, millions — city/metro clusters chosen
# to cover the national population distribution; used ONLY as relative
# weights for the temperature series).
TEMP = {
    "fr": [
        ("paris_ile_de_france", 48.85, 2.35, 12.5),
        ("lyon", 45.75, 4.85, 2.3),
        ("marseille", 43.3, 5.4, 1.9),
        ("lille", 50.63, 3.07, 1.2),
        ("toulouse", 43.6, 1.45, 1.4),
        ("bordeaux", 44.85, -0.6, 1.0),
        ("nantes", 47.2, -1.55, 1.0),
        ("strasbourg", 48.6, 7.75, 0.8),
        ("rennes", 48.1, -1.7, 0.75),
    ],
    "be": [
        ("brussels", 50.85, 4.35, 3.3),
        ("antwerp_ghent", 51.15, 4.1, 2.0),
        ("liege_charleroi", 50.5, 5.0, 1.8),
    ],
    "nl": [
        ("randstad", 52.1, 4.6, 8.0),
        ("brabant", 51.6, 5.2, 2.6),
        ("east_gelderland_overijssel", 52.2, 6.0, 2.2),
        ("north_groningen", 53.1, 6.3, 1.7),
    ],
    "de": [
        ("rhine_ruhr", 51.4, 7.0, 11.0),
        ("berlin", 52.52, 13.4, 4.5),
        ("rhine_main_frankfurt", 50.1, 8.7, 4.0),
        ("stuttgart", 48.78, 9.18, 3.5),
        ("munich", 48.14, 11.58, 3.0),
        ("hamburg", 53.55, 10.0, 2.7),
        ("cologne_bonn", 50.94, 6.96, 2.5),
        ("leipzig_dresden", 51.2, 12.9, 2.2),
        ("nuremberg", 49.45, 11.08, 1.3),
        ("hannover", 52.37, 9.73, 1.2),
        ("luxembourg", 49.6, 6.1, 0.66),  # DE-LU zone includes LU
    ],
    "dk1": [
        ("aarhus", 56.15, 10.2, 0.9),
        ("aalborg", 57.05, 9.92, 0.6),
        ("odense_funen", 55.4, 10.39, 0.5),
        ("triangle_kolding", 55.5, 9.5, 0.7),
        ("esbjerg_west", 55.5, 8.45, 0.4),
        ("herning_mid", 56.14, 8.97, 0.4),
    ],
    "ie": [
        ("dublin", 53.35, -6.3, 2.1),
        ("belfast_ni", 54.6, -5.95, 1.0),
        ("cork", 51.9, -8.5, 0.6),
        ("galway_west", 53.27, -9.05, 0.5),
        ("midlands_rest", 53.0, -7.8, 1.2),
    ],
    # NO2 (southwest Norway: Rogaland/Agder — the NSL counterparty zone).
    "no2": [
        ("stavanger", 58.97, 5.73, 0.9),
        ("kristiansand_agder", 58.15, 8.0, 0.5),
        ("haugesund", 59.4, 5.3, 0.3),
    ],
}

CAL_BAND = gb.CALIBRATION_HONESTY_BAND  # (0.7, 1.3), the GB rule


def all_groups() -> dict:
    """(kind, country) -> cluster list, for the loader."""
    groups = {}
    for c, pts in ONSHORE.items():
        groups[("onshore", c)] = pts
    for c, pts in OFFSHORE.items():
        groups[("offshore", c)] = pts
    for c, pts in SOLAR.items():
        groups[("solar", c)] = pts
    for c, pts in TEMP.items():
        groups[("temp", c)] = pts
    return groups


def cell_key(lat: np.ndarray, lon: np.ndarray) -> np.ndarray:
    """0.25-degree grid cell -> unique int key (exact: coords are quarter
    degrees stored as float32)."""
    return (np.round(lat * 4).astype(np.int64) + 400) * 4096 + (
        np.round(lon * 4).astype(np.int64) + 2048
    )


def point_cell_keys(grid: pd.DataFrame, lat: float, lon: float) -> np.ndarray:
    """Keys of the cells in the 3x3 box (+/- 0.375 deg — gb.BOX_HALF_DEG)
    around a point, from the pack's actual grid."""
    m = (
        (grid["latitude"] >= lat - gb.BOX_HALF_DEG)
        & (grid["latitude"] <= lat + gb.BOX_HALF_DEG)
        & (grid["longitude"] >= lon - gb.BOX_HALF_DEG)
        & (grid["longitude"] <= lon + gb.BOX_HALF_DEG)
    )
    keys = cell_key(grid.loc[m, "latitude"].values, grid.loc[m, "longitude"].values)
    if len(keys) == 0:
        sys.exit(f"point ({lat}, {lon}): no cells in the EU box")
    return keys


def load_point_means_eu(eu_dir: Path, year: int, groups: dict) -> dict:
    """Mean u100/v100/ssrd/t2m over each point's 3x3 box, hourly for
    `year`, for ALL groups in one pass over the 12 monthly files.

    Validates the cutout as it reads (the derive-side geometry assertion;
    validate_cf_eu.py re-asserts independently): 12 monthly files, rows =
    hours x 13,189 cells, 121 x 109 grid, no NaNs, year hour count.
    Returns {(kind, country): {point_name: DataFrame(time x VARS)}}.
    """
    files = sorted(eu_dir.glob(f"era5_eu_{year}-*.parquet"))
    if len(files) != 12:
        sys.exit(
            f"EU cutout incomplete: {len(files)}/12 monthly files in {eu_dir}"
        )
    first = pd.read_parquet(files[0], columns=["latitude", "longitude"])
    grid = first.drop_duplicates().reset_index(drop=True)
    if len(grid) != N_CELLS_EU:
        sys.exit(f"EU grid has {len(grid)} cells, expected {N_CELLS_EU}")
    point_keys = {
        gkey: {name: point_cell_keys(grid, lat, lon) for name, lat, lon, _w in pts}
        for gkey, pts in groups.items()
    }
    needed = np.unique(
        np.concatenate([k for d in point_keys.values() for k in d.values()])
    )

    monthly: dict = {g: {n: [] for n in d} for g, d in point_keys.items()}
    n_hours = 0
    for f in files:
        df = pd.read_parquet(f)
        if df[VARS].isna().any().any():
            sys.exit(f"{f.name}: NaNs in cutout")
        hours = df["time"].nunique()
        if len(df) != hours * N_CELLS_EU:
            sys.exit(
                f"{f.name}: expected {hours}x{N_CELLS_EU} rows, got {len(df)}"
            )
        n_hours += hours
        keys = cell_key(df["latitude"].values, df["longitude"].values)
        sub = df.loc[np.isin(keys, needed)]
        sub_keys = cell_key(sub["latitude"].values, sub["longitude"].values)
        # One (time x cell) float64 table per variable; a point's box mean
        # is then the row-mean over its cell columns — the same arithmetic
        # as gb.load_point_means' groupby, in float64.
        pivots = {}
        base = sub.assign(_key=sub_keys)
        for v in VARS:
            pivots[v] = (
                base.pivot(index="time", columns="_key", values=v)
                .astype("float64")
            )
        for gkey, d in point_keys.items():
            for name, keys_pt in d.items():
                monthly[gkey][name].append(
                    pd.DataFrame(
                        {v: pivots[v][list(keys_pt)].mean(axis=1) for v in VARS}
                    )
                )
    if n_hours != gb.hours_in_year(year):
        sys.exit(f"cutout has {n_hours} hours, expected {gb.hours_in_year(year)}")
    print(
        f"EU cutout {year} OK: {n_hours} hours x {N_CELLS_EU:,} cells x 4 vars,"
        " no NaNs"
    )
    return {
        g: {n: pd.concat(parts).sort_index() for n, parts in d.items()}
        for g, d in monthly.items()
    }


def temp_hourly_c(points: dict, clusters: list) -> pd.Series:
    """Population-weighted 2 m temperature, Celsius, hourly (instantaneous
    field, valid AT the label — same time base as wind)."""
    series = {name: df["t2m"] - 273.15 for name, df in points.items()}
    return gb.weighted_cf(series, clusters)


def derive_raw_country(points: dict, year: int) -> dict:
    """One year's raw traces for every country/series, from the loaded
    point means. Wind/solar/interpolation are the pinned GB functions."""
    index = gb.half_hourly_index(year)
    assert len(index) == gb.hours_in_year(year) * 2
    out: dict = {}
    for c, (_zone, techs) in CF_COUNTRIES.items():
        out[c] = {}
        if "onshore" in techs:
            out[c]["onshore"] = gb.to_half_hourly(
                gb.wind_hourly_cf(
                    points[("onshore", c)], ONSHORE[c],
                    gb.ON_V0, gb.ON_S, gb.ON_PMAX,
                    hub_factor=gb.ONSHORE_HUB_FACTOR,
                ),
                index,
            )
        if "offshore" in techs:
            out[c]["offshore"] = gb.to_half_hourly(
                gb.wind_hourly_cf(
                    points[("offshore", c)], OFFSHORE[c],
                    gb.OFF_V0, gb.OFF_S, gb.OFF_PMAX, hub_factor=1.0,
                ),
                index,
            )
        if "solar" in techs:
            out[c]["solar"] = gb.to_half_hourly(
                gb.solar_hourly_cf(points[("solar", c)], SOLAR[c]), index
            )
    for c in TEMP_COUNTRIES:
        out.setdefault(c, {})["t2m"] = gb.to_half_hourly(
            temp_hourly_c(points[("temp", c)], TEMP[c]), index
        )
    for c, d in out.items():
        for name, s in d.items():
            if s.isna().any():
                sys.exit(f"{year} {c} {name}: NaNs after interpolation")
    return out


def load_anchors(repo: Path) -> tuple[pd.DataFrame, pd.DataFrame]:
    proc = repo / "data" / "packs" / "entsoe-2024" / "processed"
    cap = pd.read_parquet(proc / "capacity_2024.parquet").reset_index()
    agg = pd.read_parquet(proc / "aggregation_gen_2024.parquet").reset_index()
    return cap, agg


def load_cbs_anchors(repo: Path) -> dict:
    """The CBS national-statistics anchors of docstring deviation 3a:
    {(country, tech): (pair_capacity_mw, gen_gwh_2024, status)}. Read
    from the fetched-and-built CBS pack so a CBS revision surfaces as a
    factor drift against PINNED_FACTORS_EU, never silently."""
    proc = repo / "data" / "packs" / "cbs-2024" / "processed"
    df = pd.read_csv(proc / "cbs_2024_nl_anchors.csv")

    def one(series: str, measure: str) -> tuple[float, str]:
        row = df[(df["series"] == series) & (df["measure"] == measure)]
        if len(row) != 1:
            sys.exit(f"CBS anchor {series}/{measure}: expected exactly one row")
        return float(row["value"].iloc[0]), str(row["status"].iloc[0])

    on_gen, on_st = one("wind_onshore", "net_generation")
    on_cap, _ = one("wind_onshore", "capacity_end_year")
    pv_gen, pv_st = one("solar", "generation")
    pv_cap, _ = one("solar_all_sectors", "panel_capacity_end_year")  # MW_dc
    return {
        ("nl", "onshore"): (on_cap, on_gen, on_st),
        ("nl", "solar"): (pv_cap, pv_gen, pv_st),
    }


CBS_SOURCE = (
    "CBS (Statistics Netherlands) StatLine 82610NED + 85005NED, "
    "retrieved 2026-07-03, CC BY 4.0"
)


def anchor_for(cap: pd.DataFrame, agg: pd.DataFrame, zone: str, psr: str):
    """(capacity_mw, gen_gwh, monthly_gwh list) or None if either half of
    the anchor is missing for this zone/technology."""
    c = cap[(cap["zone"] == zone) & (cap["psr_code"] == psr)]
    g = agg[(agg["zone"] == zone) & (agg["psr_code"] == psr)]
    if len(c) != 1 or len(g) != 1 or not np.isfinite(c["capacity_mw"].iloc[0]):
        return None
    monthly = [float(g[f"gen_gwh_m{m:02d}"].iloc[0]) for m in range(1, 13)]
    return float(c["capacity_mw"].iloc[0]), float(g["gen_gwh"].iloc[0]), monthly


def calibrate_2024(
    raw_2024: dict, cap: pd.DataFrame, agg: pd.DataFrame, cbs: dict
) -> tuple:
    """Per-country/tech factors from the 2024 raw traces and the ENTSO-E
    anchors — except the series in `cbs` (NL onshore/solar), which anchor
    to CBS national statistics per docstring deviation 3a — with the
    honesty policy of docstring deviation 3. Returns
    (factors {c: {tech: applied}}, calibration report dict)."""
    factors: dict = {}
    report: dict = {}
    for c, (zone, techs) in CF_COUNTRIES.items():
        factors[c] = {}
        report[c] = {}
        for tech in techs:
            raw = raw_2024[c][tech]
            raw_cf = float(raw.mean())
            if (c, tech) in cbs:
                cap_mw, gen_gwh, status = cbs[(c, tech)]
                target_cf = gen_gwh * 1e3 / (cap_mw * HOURS_2024)
                factor, _cal = gb.calibrate(raw, target_cf)
                factor = float(factor)
                in_band = CAL_BAND[0] <= factor <= CAL_BAND[1]
                if not in_band:
                    # The recalibration mandate is conditional on the CBS
                    # factor being honest — never force it (eu-cf-review
                    # ruling 1: report, do not tune).
                    sys.exit(
                        f"{c} {tech}: CBS-anchored factor {factor:.4f} is "
                        f"OUTSIDE the honesty band {CAL_BAND} — stopping"
                    )
                factors[c][tech] = factor
                ent = anchor_for(cap, agg, zone, TECH_PSR[tech])
                report[c][tech] = {
                    "calibrated": True,
                    "computed_factor": round(factor, 4),
                    "applied_factor": round(factor, 4),
                    "raw_annual_cf_2024": round(raw_cf, 4),
                    "target_annual_cf_2024": round(target_cf, 4),
                    "anchor_source": f"{CBS_SOURCE}, status {status}",
                    "anchor_capacity_mw_cbs": cap_mw,
                    "anchor_gen_gwh_2024_cbs": gen_gwh,
                    "pair_with_capacity_mw": cap_mw,
                    "entsoe_a75_gen_gwh_2024_superseded": (
                        None if ent is None else ent[1]
                    ),
                    "in_honesty_band_0.7_1.3": True,
                }
                print(
                    f"{c} {tech}: raw CF {raw_cf:.4f} -> CBS target "
                    f"{target_cf:.4f}, factor {factor:.4f} (CBS anchor)"
                )
                continue
            anchor = anchor_for(cap, agg, zone, TECH_PSR[tech])
            if anchor is None:
                factors[c][tech] = 1.0
                report[c][tech] = {
                    "calibrated": False,
                    "applied_factor": 1.0,
                    "raw_annual_cf_2024": round(raw_cf, 4),
                    "diagnosis": (
                        "no ENTSO-E anchor (capacity and/or full-year "
                        "generation series missing for this zone/tech)"
                    ),
                }
                print(f"{c} {tech}: NO ANCHOR — uncalibrated (factor 1.0)")
                continue
            cap_mw, gen_gwh, _ = anchor
            target_cf = gen_gwh * 1e3 / (cap_mw * HOURS_2024)
            factor, _cal = gb.calibrate(raw, target_cf)
            factor = float(factor)
            in_band = CAL_BAND[0] <= factor <= CAL_BAND[1]
            applied = factor if in_band else 1.0
            factors[c][tech] = applied
            report[c][tech] = {
                "calibrated": in_band,
                "computed_factor": round(factor, 4),
                "applied_factor": round(applied, 4),
                "raw_annual_cf_2024": round(raw_cf, 4),
                "target_annual_cf_2024": round(target_cf, 4),
                "anchor_capacity_mw_a68": cap_mw,
                "anchor_gen_gwh_2024_a75": gen_gwh,
                "in_honesty_band_0.7_1.3": in_band,
            }
            if not in_band:
                report[c][tech]["diagnosis"] = (
                    "factor outside honesty band -> anchor data finding, "
                    "NOT absorbed; trace shipped uncalibrated (see the "
                    "derivation report note for the per-case diagnosis)"
                )
            flag = "" if in_band else "  ** OUT OF BAND — shipped UNCALIBRATED **"
            print(
                f"{c} {tech}: raw CF {raw_cf:.4f} -> target {target_cf:.4f}, "
                f"factor {factor:.4f}{flag}"
            )
    return factors, report


def check_pinned(factors: dict) -> None:
    if not PINNED_FACTORS_EU:
        print("\nPINNED_FACTORS_EU is empty — first (pinning) run. Pin these:")
        print(json.dumps(
            {c: {t: round(f, 4) for t, f in d.items()} for c, d in factors.items()},
            indent=2,
        ))
        return
    for c, d in factors.items():
        for tech, f in d.items():
            pinned = PINNED_FACTORS_EU.get(c, {}).get(tech)
            if pinned is None or round(f, 4) != pinned:
                sys.exit(
                    f"{c} {tech}: applied factor {f:.6f} does not round to "
                    f"pinned {pinned} — cutout, anchor or method drift"
                )
    print("pinned EU factors confirmed (all countries/techs)")


def write_temp(trace: pd.Series, out_dir: Path, stem: str) -> None:
    """Same file format as gb.write, column `t2m_c` (Celsius)."""
    df = trace.rename("t2m_c").to_frame().astype("float64")
    df.index.name = "utc_start"
    df.to_csv(out_dir / f"{stem}.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
    df.to_parquet(out_dir / f"{stem}.parquet")


def verify_ssrd_convention_eu(eu_dir: Path) -> dict:
    """The GB Phase A empirical ssrd-convention check, re-run on the EU
    pack (gb.verify_ssrd_convention hardcodes the GB file/cell, so the
    probe is adapted here: cell 48.0N 2.0E, June 2024): irradiance-
    weighted label centroid must sit ~+30 min after true solar noon
    (hour-ending accumulation)."""
    june = pd.read_parquet(eu_dir / "2024" / "era5_eu_2024-06.parquet")
    lat, lon = 48.0, 2.0
    cell = june[(june["latitude"] == lat) & (june["longitude"] == lon)].set_index(
        "time"
    )
    daily = cell["ssrd"].resample("1D").sum()
    clear_day = daily.idxmax().date()
    day = cell.loc[str(clear_day), "ssrd"]
    hours = day.index.hour + day.index.minute / 60.0
    centroid = float((day * hours).sum() / day.sum())
    doy = pd.Timestamp(clear_day).dayofyear
    b = 2.0 * math.pi * (doy - 1) / 366.0
    eot_min = 229.18 * (
        0.000075
        + 0.001868 * math.cos(b)
        - 0.032077 * math.sin(b)
        - 0.014615 * math.cos(2 * b)
        - 0.040849 * math.sin(2 * b)
    )
    solar_noon = 12.0 - lon / 15.0 - eot_min / 60.0
    offset_min = (centroid - solar_noon) * 60.0
    convention = "hour-ending" if offset_min > 0 else "hour-starting"
    print(
        f"EU ssrd convention check: clear day {clear_day} at (48N, 2E), "
        f"centroid - solar noon = {offset_min:+.1f} min => {convention}"
    )
    if convention != "hour-ending":
        sys.exit("EU ssrd convention check FAILED: expected hour-ending")
    return {
        "clear_day": str(clear_day),
        "cell": [lat, lon],
        "centroid_minus_solar_noon_min": round(offset_min, 1),
        "inferred_convention": convention,
    }


def total_wind(traces: dict, c: str, cap: pd.DataFrame) -> pd.Series:
    """Capacity-weighted total-wind CF for a country (A68 weights; IE has
    onshore only)."""
    zone = CF_COUNTRIES[c][0]
    on_cap = cap[(cap["zone"] == zone) & (cap["psr_code"] == "B19")][
        "capacity_mw"
    ].iloc[0]
    if "offshore" not in traces[c]:
        return traces[c]["onshore"]
    off_cap = cap[(cap["zone"] == zone) & (cap["psr_code"] == "B18")][
        "capacity_mw"
    ].iloc[0]
    return (traces[c]["onshore"] * on_cap + traces[c]["offshore"] * off_cap) / (
        on_cap + off_cap
    )


def gb_total_wind(repo: Path, year: int) -> pd.Series:
    cf_dir = repo / "data" / "packs" / "cf"
    on = pd.read_parquet(cf_dir / f"gb_onshore_cf_{year}.parquet")["cf"]
    off = pd.read_parquet(cf_dir / f"gb_offshore_cf_{year}.parquet")["cf"]
    return (on * gb.ONSHORE_GW + off * gb.OFFSHORE_GW) / (
        gb.ONSHORE_GW + gb.OFFSHORE_GW
    )


def reconcile_2024(
    calibrated_2024: dict, cap: pd.DataFrame, agg: pd.DataFrame
) -> dict:
    """Monthly derived vs ENTSO-E observed energy per country/tech (GWh),
    2024 — the GB monthly-table precedent. Reported for uncalibrated
    series too (it shows the anchor mismatch scale rather than model
    skill there)."""
    recon: dict = {}
    for c, (zone, techs) in CF_COUNTRIES.items():
        recon[c] = {}
        for tech in techs:
            anchor = anchor_for(cap, agg, zone, TECH_PSR[tech])
            if anchor is None:
                continue
            cap_mw, gen_gwh, monthly_obs = anchor
            s = calibrated_2024[c][tech]
            derived_m = (s.groupby(s.index.month).sum() * 0.5 * cap_mw / 1e3)
            r = float(np.corrcoef(derived_m.values, np.array(monthly_obs))[0, 1])
            recon[c][tech] = {
                "annual_derived_gwh": round(float(derived_m.sum()), 1),
                "annual_entsoe_gwh": round(gen_gwh, 1),
                "monthly_r": round(r, 4),
                "monthly_derived_gwh": [round(float(v), 1) for v in derived_m],
                "monthly_entsoe_gwh": monthly_obs,
            }
    return recon


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument("--years", default="1985-2024", help="e.g. 1985-2024 or 2024")
    args = ap.parse_args()
    repo = args.repo_root
    years = gb.parse_cf_years(args.years)

    eu_dir = repo / "data" / "packs" / "era5-eu"
    out_root = repo / "data" / "packs" / "cf-eu"
    cap, agg = load_anchors(repo)
    cbs = load_cbs_anchors(repo)
    groups = all_groups()

    for year in years:
        n = len(sorted((eu_dir / str(year)).glob(f"era5_eu_{year}-*.parquet")))
        if n != 12:
            sys.exit(f"{year}: EU cutout incomplete ({n}/12 files)")

    ssrd_finding = verify_ssrd_convention_eu(eu_dir)

    print("deriving 2024 raw traces (calibration anchor year) ...")
    points_2024 = load_point_means_eu(eu_dir / "2024", 2024, groups)
    raw_2024 = derive_raw_country(points_2024, 2024)
    factors, cal_report = calibrate_2024(raw_2024, cap, agg, cbs)
    check_pinned(factors)

    report: dict = {
        "ssrd_convention_eu": ssrd_finding,
        "calibration": cal_report,
        "annual_cf": {},
        "t2m_annual_mean_c": {},
    }
    wind_by_country: dict = {c: [] for c in CF_COUNTRIES}
    wind_gb: list = []

    for year in years:
        points = points_2024 if year == 2024 else load_point_means_eu(
            eu_dir / str(year), year, groups
        )
        raw = raw_2024 if year == 2024 else derive_raw_country(points, year)
        calibrated: dict = {}
        for c, (_zone, techs) in CF_COUNTRIES.items():
            calibrated[c] = {}
            out_dir = out_root / c
            out_dir.mkdir(parents=True, exist_ok=True)
            for tech in techs:
                trace = (raw[c][tech] * factors[c][tech]).clip(0.0, 1.0)
                calibrated[c][tech] = trace
                gb.write(trace, out_dir, f"{c}_{tech}_cf_{year}")
                report["annual_cf"].setdefault(c, {}).setdefault(tech, {})[
                    str(year)
                ] = round(float(trace.mean()), 4)
            wind_by_country[c].append(total_wind(calibrated, c, cap))
        for c in TEMP_COUNTRIES:
            out_dir = out_root / c
            out_dir.mkdir(parents=True, exist_ok=True)
            t = raw[c]["t2m"]
            write_temp(t, out_dir, f"{c}_t2m_{year}")
            report["t2m_annual_mean_c"].setdefault(c, {})[str(year)] = round(
                float(t.mean()), 2
            )
        wind_gb.append(gb_total_wind(repo, year))
        if year == 2024:
            report["reconciliation_2024"] = reconcile_2024(calibrated, cap, agg)
        print(f"{year}: written ({len(years) - years.index(year) - 1} to go)")

    # Cross-country wind correlations (GB + all CF countries), half-hourly
    # and daily-mean, over all derived years — the Module 5 anticyclone
    # evidence.
    series = {"gb": pd.concat(wind_gb)}
    for c in CF_COUNTRIES:
        series[c] = pd.concat(wind_by_country[c])
    names = list(series)
    mat = np.corrcoef([series[n].values for n in names])
    daily = {n: s.resample("1D").mean() for n, s in series.items()}
    mat_d = np.corrcoef([daily[n].values for n in names])
    report["wind_correlation"] = {
        "series": names,
        "halfhourly": [[round(float(v), 4) for v in row] for row in mat],
        "daily_mean": [[round(float(v), 4) for v in row] for row in mat_d],
    }

    # Per-country annual total-wind extremes (A68-capacity-weighted).
    extremes: dict = {}
    for c in CF_COUNTRIES:
        annual = {
            y: float(s.mean()) for y, s in zip(years, wind_by_country[c])
        }
        worst = min(annual, key=annual.get)
        best = max(annual, key=annual.get)
        extremes[c] = {
            "worst_year": worst,
            "worst_cf": round(annual[worst], 4),
            "best_year": best,
            "best_cf": round(annual[best], 4),
            "mean_cf": round(float(np.mean(list(annual.values()))), 4),
        }
    report["wind_extremes_total"] = extremes

    # The pack report (calibration + 40-year statistics + correlations) is
    # only meaningful — and only manifest-stable — for the full 1985-2024
    # sweep. A partial run (e.g. --years 2024 as a determinism check) must
    # not clobber it with a shorter-span report.
    if years == list(range(1985, 2025)):
        (out_root / "eu_cf_report.json").write_text(
            json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
        )
        print(f"report -> {out_root / 'eu_cf_report.json'}")
    else:
        print("partial-year run: eu_cf_report.json NOT written (full-sweep only)")


if __name__ == "__main__":
    main()
