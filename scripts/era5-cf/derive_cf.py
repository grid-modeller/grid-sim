#!/usr/bin/env python3
"""Derive GB per-technology capacity-factor traces from ERA5 cutouts.

Phase A of the D1 pipeline: calendar 2024 only (the validation year), the
default invocation. Phase B (--cf-years) runs the SAME code over arbitrary
years — see PHASE B below. Input is the local cutout written by
fetch_era5.py (data/packs/era5/<year>/); this script touches no network
and contains no randomness — outputs are a pure function of the cutouts
and the constants below.

Phase A outputs (data/packs/2024/processed/), each a single float64 column
`cf` in [0, 1] on the pack's half-hourly UTC index (17,568 periods, leap
year), Parquet + CSV per docs/06-conventions.md:

    gb_onshore_cf_2024.{parquet,csv}
    gb_offshore_cf_2024.{parquet,csv}
    gb_solar_cf_2024.{parquet,csv}
    era5_cf_report_2024.json   (raw vs calibrated CFs, calibration factors,
                                correlation vs observed wind, monthly table,
                                ssrd-convention verification result)

METHOD (deliberately simple and explicit; every constant is defined and
justified below):

1. Spatial aggregation. The GB fleet is represented by a short list of
   APPROXIMATE cluster/region points with capacity weights (offshore: the
   major farm clusters; onshore: rough regional weights, Scotland-heavy;
   solar: south-of-England-heavy). For each point the ERA5 fields are
   averaged over a 3x3-cell box (+/-0.375 deg). Weights are normalised;
   only their relative sizes matter (absolute scale is set by
   calibration). See OFFSHORE_CLUSTERS / ONSHORE_REGIONS / SOLAR_REGIONS.

2. Wind. 100 m wind speed |(u100, v100)| per cluster -> fleet capacity
   factor via a multi-turbine-smoothed aggregate power curve, the logistic
   approximation

       cf_raw(v) = PMAX * LOSSES / (1 + exp(-(v - V0) / S)) * cutout(v)

   - V0 (speed at half of rated output) and S (smoothing width) mimic a
     fleet of turbines with mixed cut-ins/rated speeds (Staffell & Green
     2014 style smoothing of manufacturer curves).
   - PMAX < 1: the aggregate fleet never reaches nameplate simultaneously
     (wakes, availability).
   - LOSSES: explicit electrical/availability loss multiplier.
   - cutout(v): linear taper to zero between CUTOUT_LO and CUTOUT_HI m/s
     (storm shutdown, smoothed across the fleet).
   - Onshore hub height ~80 m vs the 100 m ERA5 level: power-law shear
     v_hub = v100 * (80/100)^ALPHA with ALPHA = 0.14. Offshore hubs are
     ~100 m+; no adjustment.

3. Solar. GHI-proportional PV model with temperature derate:
       G    = ssrd / 3600          (J/m2 accumulated over hour -> W/m2)
       Tcell = t2m[degC] + C_T * G (NOCT-style cell heating)
       cf_raw = PR * (G / G_STC) * (1 + GAMMA * (Tcell - 25))
       cf_raw = 0 where G < PV_G_MIN (inverter startup threshold; also
       removes ERA5 float32 night noise in ssrd)
   No tilt/tracking model (GHI on a horizontal plane); the systematic
   tilt-gain omission is absorbed by calibration and reported.

4. Time base. ERA5 instantaneous fields (wind, t2m) are valid AT the
   hourly label; ssrd is accumulated over the hour ENDING at the label
   (verified from the data itself — see verify_ssrd_convention()), so
   solar CF is placed at the interval centre (label - 30 min). Hourly ->
   half-hourly by linear interpolation in time onto the year's half-hourly
   index (17,520 periods; 17,568 leap); the trailing 23:30 period holds
   the 23:00 value (wind) and the leading 00:00 period back-fills from
   00:30 (solar; midwinter night, exactly zero). No NaNs.

5. Calibration (bias correction) — one multiplicative factor per
   technology, chosen so 2024 annual energy matches the observed pack
   (docs/notes/2024-validation-pack-report.md, D3 total-generation
   convention), holding the reference scenario's CONSTANT end-2024
   capacities:
   - Onshore: the NESO embedded-wind estimate pins the onshore CF level:
     16.97 TWh / 6.6 GW embedded => calibrated onshore annual CF
     = 0.2927 (embedded wind is effectively all onshore; the same trace
     serves the 14.4 GW onshore fleet).
   - Offshore: the remainder of observed total wind:
     (82.61 TWh - onshore 14.4 GW * 0.2927 * 8784 h) / 14.7 GW.
     This jointly reproduces both the 82.61 TWh total and the
     transmission/embedded split evidence.
   - Solar: 13.95 TWh / 18.7 GW => calibrated annual CF = 0.0849.
   The factor is applied, then values clipped to [0, 1]; if clipping
   bites, the factor is re-iterated (fixed point) so the clipped trace
   still hits the target annual energy exactly. Raw annual CFs and
   factors are reported — factors outside [0.7, 1.3] mean the physical
   model is off and must be flagged, not absorbed.
   NOTE the capacity-growth caveat: capacities grew during 2024 (Dogger
   Bank A, Moray West commissioning), so these calibrated CFs are
   energy-matching CFs for the fixed end-2024 fleet — exactly what the
   constant-capacity validation scenario needs, but biased low vs the
   true CF of capacity actually installed at each moment.

PHASE B (--cf-years, added 2026-07-02 for the Stage 3 multi-year storage
runs). Derives CF traces for arbitrary years from the per-year cutouts
(data/packs/era5/<year>/, written by fetch_era5.py --years), with the
METHOD above unchanged. Outputs one file family per year:

    data/packs/cf/gb_{onshore,offshore,solar}_cf_<YEAR>.{parquet,csv}

(2024 is regenerated into this layout too, so scenario trace lists are a
uniform per-year family; it is value-identical to the Phase A traces —
same raw derivation, same factors, same expression.)

Calibration under Phase B — supervisor decision, 2026-07-02: the pinned
2024 factors (offshore 0.8975, onshore 1.0395, solar 0.8837, 4 dp as
recorded in docs/notes/era5-cf-2024-report.md) apply UNCHANGED to every
year. They are not stored at full precision anywhere; each --cf-years run
re-derives them exactly from the 2024 cutout via the pinned calibrate()
and refuses to proceed if they no longer round to the pinned values
(cutout or method drift). Rationale: the scenario fleet is fixed (end-2024
capacities and layouts), and the calibration corrects the power-curve/
weighting model for THAT fleet; per-year outturn recalibration would also
be impossible — the historical fleet differed (and mostly did not exist).
Consequence, stated plainly: a year-Y trace answers "what would the
END-2024 fleet have produced in year Y's weather?" — exactly what a
multi-decade storage study needs — NOT "what did year Y's actual fleet
produce?".

Usage:
    python derive_cf.py <repo-root>                       # Phase A (2024)
    python derive_cf.py <repo-root> --cf-years 2019-2024  # Phase B
    python derive_cf.py <repo-root> --cf-years 1987,1990-1995
"""

import argparse
import calendar
import json
import math
import sys
from pathlib import Path

import numpy as np
import pandas as pd

HOURS_2024 = 8_784  # leap year

# Phase B guard: the calibration factors derived from the 2024 cutout must
# round (4 dp) to these pinned values (docs/notes/era5-cf-2024-report.md).
PINNED_FACTORS_2024 = {"offshore": 0.8975, "onshore": 1.0395, "solar": 0.8837}

# ---------------------------------------------------------------------------
# Fleet spatial weights — ALL APPROXIMATE (order-of-magnitude public
# knowledge: RenewableUK UKWED / Crown Estate round information; not a
# licensed dataset). (name, lat degN, lon degE, weight_gw). Weights are
# normalised before use.
# ---------------------------------------------------------------------------

OFFSHORE_CLUSTERS = [
    # East-coast North Sea
    ("hornsea_1_2", 53.9, 1.8, 2.5),          # Hornsea One + Two
    ("dogger_bank_a", 55.1, 1.9, 0.8),        # partial commissioning 2024
    ("greater_wash", 53.4, 0.8, 3.0),         # Triton Knoll, Race Bank,
    #                                           Dudgeon, Sheringham Shoal,
    #                                           Lincs, Humber Gateway, ...
    ("east_anglia", 52.1, 2.0, 1.6),          # EA ONE, Galloper, Greater
    #                                           Gabbard. EA ONE lies ~2.5E,
    #                                           clamped to the cutout's 2E
    #                                           edge (flagged approximation)
    ("thames", 51.6, 1.2, 1.3),               # London Array, Thanet,
    #                                           Gunfleet Sands, Kentish Flats
    # West coast
    ("irish_sea", 53.8, -3.6, 2.9),           # Walney x3, West of Duddon
    #                                           Sands, Burbo Bank, Gwynt y
    #                                           Mor, Ormonde, Robin Rigg, ...
    # Scotland
    ("moray_firth", 58.2, -2.9, 1.9),         # Moray East, Beatrice, Moray
    #                                           West (partial 2024)
    ("forth_tay", 56.6, -2.0, 1.3),           # Seagreen, Neart na Gaoithe
    #                                           (partial), Hywind, Kincardine
]  # sums ~15.3 GW vs 14.7 GW UKWED end-2024 - only relative weights matter

ONSHORE_REGIONS = [
    ("southern_uplands", 55.3, -3.7, 4.0),    # Dumfries & Galloway/Borders
    ("central_belt", 55.7, -4.3, 2.5),        # Lanarkshire (Whitelee etc.)
    ("highlands", 57.5, -4.2, 2.5),
    ("argyll", 55.6, -5.4, 0.8),
    ("ne_scotland", 57.3, -2.5, 0.8),         # Aberdeenshire
    ("wales", 52.4, -3.6, 1.3),
    ("nw_england", 54.3, -2.9, 0.9),          # Cumbria/Lancashire
    ("ne_england", 54.5, -1.8, 0.8),          # Durham/Yorkshire
    ("east_england", 52.9, 0.0, 0.5),         # Lincolnshire/East Anglia
    ("sw_england", 50.6, -4.5, 0.3),          # Cornwall/Devon
]  # sums 14.4 GW; Scotland-heavy (~73%) per UKWED regional pattern

SOLAR_REGIONS = [
    ("se_england", 51.2, -0.5, 4.5),
    ("sw_england", 50.9, -3.0, 4.0),
    ("east_england", 52.2, 0.5, 3.5),
    ("midlands", 52.6, -1.5, 3.0),
    ("n_england", 53.8, -1.5, 1.7),
    ("wales", 52.0, -3.8, 1.5),
    ("scotland", 56.0, -3.5, 0.5),
]  # sums 18.7 GW; south-of-England-heavy per DUKES regional pattern

BOX_HALF_DEG = 0.375  # 3x3 ERA5 cells around each point

# ---------------------------------------------------------------------------
# Wind power-curve constants (aggregate logistic curve, see docstring)
# ---------------------------------------------------------------------------
OFF_V0, OFF_S, OFF_PMAX = 9.5, 1.9, 0.95   # offshore: rated ~12-13 m/s ->
#                                            half power ~9.5; wide fleet
#                                            smoothing over the North Sea
ON_V0, ON_S, ON_PMAX = 8.5, 1.7, 0.95      # onshore: lower specific power,
#                                            half power ~8.5 at hub height
WIND_LOSSES = 0.90                          # explicit electrical +
#                                            availability + wake losses
#                                            (~10%, renewables.ninja-style)
CUTOUT_LO, CUTOUT_HI = 25.0, 30.0           # storm-shutdown taper, m/s
SHEAR_ALPHA = 0.14                          # power-law shear exponent
ONSHORE_HUB_FACTOR = (80.0 / 100.0) ** SHEAR_ALPHA  # ~0.969: 100m -> 80m hub

# ---------------------------------------------------------------------------
# Solar PV constants
# ---------------------------------------------------------------------------
G_STC = 1000.0     # W/m2, standard test conditions
PV_PR = 0.85       # performance ratio: inverter, wiring, soiling, mismatch
PV_GAMMA = -0.0037  # /K, crystalline-Si power temperature coefficient
PV_C_T = 0.03      # K per W/m2 cell heating ~ (NOCT-20)/800 with NOCT~44C
PV_G_MIN = 1.0     # W/m2: below this, output is 0 (inverter startup
#                    threshold; also zeroes ERA5's float32 night noise in
#                    ssrd, observed up to 0.25 J/m2 at night)

# ---------------------------------------------------------------------------
# Calibration targets (docs/notes/2024-validation-pack-report.md, D3)
# ---------------------------------------------------------------------------
TOTAL_WIND_TWH = 82.61        # transmission 65.64 + NESO embedded 16.97
EMBEDDED_WIND_TWH = 16.97
EMBEDDED_WIND_GW = 6.6        # NESO embedded wind capacity, end-2024
ONSHORE_GW = 14.4             # scenarios/gb-2024-reference.toml (UKWED)
OFFSHORE_GW = 14.7
SOLAR_TWH = 13.95             # NESO embedded-solar estimate
SOLAR_GW = 18.7

CALIBRATION_HONESTY_BAND = (0.7, 1.3)


def hours_in_year(year: int) -> int:
    return 8_784 if calendar.isleap(year) else 8_760


def half_hourly_index(year: int) -> pd.DatetimeIndex:
    return pd.date_range(
        f"{year}-01-01 00:00", f"{year}-12-31 23:30", freq="30min", tz="UTC"
    )


def load_point_means(era5_dir: Path, points: dict, year: int) -> dict:
    """Mean u100/v100/ssrd/t2m over each point's 3x3 box, hourly for `year`.

    `points` maps a group label to a cluster list; returns
    {group: {name: DataFrame(time x [u100, v100, ssrd, t2m])}}.
    Also validates the cutout: 12 months, 8,760/8,784 hours, 2,009 cells,
    no NaNs.
    """
    files = sorted(era5_dir.glob(f"era5_gb_{year}-*.parquet"))
    if len(files) != 12:
        sys.exit(
            f"cutout incomplete: {len(files)}/12 monthly files in {era5_dir} "
            "- run fetch_era5.py first (it resumes; finished months are kept)"
        )
    monthly = {g: {n: [] for n, *_ in pts} for g, pts in points.items()}
    n_hours = 0
    for f in files:
        df = pd.read_parquet(f)
        if df[["u100", "v100", "ssrd", "t2m"]].isna().any().any():
            sys.exit(f"{f.name}: NaNs in cutout")
        hours = df["time"].nunique()
        if len(df) != hours * 2009:
            sys.exit(f"{f.name}: expected {hours}x2009 rows, got {len(df)}")
        n_hours += hours
        for group, pts in points.items():
            for name, lat, lon, _gw in pts:
                m = (
                    (df["latitude"] >= lat - BOX_HALF_DEG)
                    & (df["latitude"] <= lat + BOX_HALF_DEG)
                    & (df["longitude"] >= lon - BOX_HALF_DEG)
                    & (df["longitude"] <= lon + BOX_HALF_DEG)
                )
                sub = df.loc[m].groupby("time")[["u100", "v100", "ssrd", "t2m"]].mean()
                monthly[group][name].append(sub)
    if n_hours != hours_in_year(year):
        sys.exit(f"cutout has {n_hours} hours, expected {hours_in_year(year)}")
    print(f"cutout {year} OK: {n_hours} hours x 2,009 cells x 4 vars, no NaNs")
    return {
        g: {n: pd.concat(parts).sort_index() for n, parts in d.items()}
        for g, d in monthly.items()
    }


def logistic_power_curve(ws: pd.Series, v0: float, s: float, pmax: float) -> pd.Series:
    cf = pmax * WIND_LOSSES / (1.0 + np.exp(-(ws - v0) / s))
    taper = ((CUTOUT_HI - ws) / (CUTOUT_HI - CUTOUT_LO)).clip(0.0, 1.0)
    return cf * taper


def weighted_cf(point_cfs: dict, clusters: list) -> pd.Series:
    weights = {n: gw for n, _lat, _lon, gw in clusters}
    total = sum(weights.values())
    out = sum(point_cfs[n] * (w / total) for n, w in weights.items())
    return out


def wind_hourly_cf(points: dict, clusters: list, v0, s, pmax, hub_factor) -> pd.Series:
    cfs = {}
    for name, df in points.items():
        ws = np.hypot(df["u100"], df["v100"]) * hub_factor
        cfs[name] = logistic_power_curve(ws, v0, s, pmax)
    return weighted_cf(cfs, clusters)


def solar_hourly_cf(points: dict, clusters: list) -> pd.Series:
    cfs = {}
    for name, df in points.items():
        g = df["ssrd"] / 3600.0  # J/m2 over hour ending at label -> W/m2
        t_cell = (df["t2m"] - 273.15) + PV_C_T * g
        cf = PV_PR * (g / G_STC) * (1.0 + PV_GAMMA * (t_cell - 25.0))
        cf[g < PV_G_MIN] = 0.0  # inverter threshold / night noise floor
        cfs[name] = cf.clip(lower=0.0)
    cf = weighted_cf(cfs, clusters)
    # ssrd is a mean over the hour ENDING at the label: place it at the
    # interval centre (label - 30 min) before interpolating.
    cf.index = cf.index - pd.Timedelta(minutes=30)
    return cf


def to_half_hourly(hourly: pd.Series, index: pd.DatetimeIndex) -> pd.Series:
    """Linear interpolation in time onto the pack index; edges padded with
    the nearest value (wind: trailing 23:30 holds 23:00; solar: leading
    00:00 takes the 00:30 value - midwinter night, exactly zero)."""
    hourly = hourly.copy()
    hourly.index = pd.DatetimeIndex(hourly.index, tz="UTC")
    union = index.union(hourly.index)
    s = hourly.reindex(union).interpolate(method="time", limit_area="inside")
    return s.reindex(index).ffill().bfill()


def derive_raw(era5_dir: Path, year: int) -> dict:
    """Cutout for `year` -> the three raw half-hourly CF traces (the
    Phase A pipeline steps 1-4, verbatim; calibration is separate)."""
    points = load_point_means(
        era5_dir,
        {
            "offshore": OFFSHORE_CLUSTERS,
            "onshore": ONSHORE_REGIONS,
            "solar": SOLAR_REGIONS,
        },
        year,
    )
    index = half_hourly_index(year)
    assert len(index) == hours_in_year(year) * 2
    raw = {
        "offshore": to_half_hourly(
            wind_hourly_cf(points["offshore"], OFFSHORE_CLUSTERS,
                           OFF_V0, OFF_S, OFF_PMAX, hub_factor=1.0),
            index,
        ),
        "onshore": to_half_hourly(
            wind_hourly_cf(points["onshore"], ONSHORE_REGIONS,
                           ON_V0, ON_S, ON_PMAX, hub_factor=ONSHORE_HUB_FACTOR),
            index,
        ),
        "solar": to_half_hourly(
            solar_hourly_cf(points["solar"], SOLAR_REGIONS), index
        ),
    }
    for name, s in raw.items():
        if s.isna().any():
            sys.exit(f"{year} {name}: NaNs after interpolation")
    return raw


def calibration_targets() -> dict:
    """2024 target annual CFs (docstring section 5). The 8,784-h year and
    the observed 2024 energies are intrinsic to the targets — they define
    the end-2024 fleet's energy-matching CF levels."""
    onshore_target_cf = EMBEDDED_WIND_TWH * 1e6 / (EMBEDDED_WIND_GW * 1e3 * HOURS_2024)
    onshore_twh = onshore_target_cf * ONSHORE_GW * HOURS_2024 / 1e3
    offshore_target_cf = (TOTAL_WIND_TWH - onshore_twh) * 1e6 / (
        OFFSHORE_GW * 1e3 * HOURS_2024
    )
    solar_target_cf = SOLAR_TWH * 1e6 / (SOLAR_GW * 1e3 * HOURS_2024)
    return {
        "offshore": offshore_target_cf,
        "onshore": onshore_target_cf,
        "solar": solar_target_cf,
    }


def calibrate(cf_raw: pd.Series, target_annual_cf: float) -> tuple[float, pd.Series]:
    """One multiplicative factor so the clipped trace's mean hits the
    target exactly (fixed-point re-iteration if clipping at 1.0 bites)."""
    factor = target_annual_cf / cf_raw.mean()
    for _ in range(50):
        clipped = (cf_raw * factor).clip(0.0, 1.0)
        err = target_annual_cf / clipped.mean()
        if abs(err - 1.0) < 1e-12:
            break
        factor *= err
    return factor, (cf_raw * factor).clip(0.0, 1.0)


def verify_ssrd_convention(era5_dir: Path) -> dict:
    """Verify from the data that ssrd is accumulated over the hour ENDING
    at the label (J/m2). Discriminator: on a clear midsummer day the
    irradiance-weighted centroid of label times sits ~+30 min after true
    solar noon under the hour-ending convention (interval centres are
    label - 30 min), vs ~-30 min under hour-starting. Also checks the
    clear-sky peak magnitude (~879 W/m2 hourly mean seen by the probe)."""
    june = pd.read_parquet(era5_dir / "era5_gb_2024-06.parquet")
    lat, lon = 52.5, -1.0
    cell = june[(june["latitude"] == lat) & (june["longitude"] == lon)].set_index("time")
    daily = cell["ssrd"].resample("1D").sum()
    clear_day = daily.idxmax().date()
    day = cell.loc[str(clear_day), "ssrd"]
    hours = day.index.hour + day.index.minute / 60.0
    centroid = float((day * hours).sum() / day.sum())
    # True solar noon (UTC h) = 12 - lon/15 - EoT/60, Spencer's equation:
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
    convention = (
        "hour-ending" if offset_min > 0 else "hour-starting"
    )
    peak_wm2 = float(june["ssrd"].max() / 3600.0)
    result = {
        "clear_day": str(clear_day),
        "cell": [lat, lon],
        "centroid_minus_solar_noon_min": round(offset_min, 1),
        "inferred_convention": convention,
        "june_peak_hourly_mean_wm2": round(peak_wm2, 1),
    }
    print(
        f"ssrd convention check: clear day {clear_day}, centroid - solar "
        f"noon = {offset_min:+.1f} min => {convention} accumulation; June "
        f"peak {peak_wm2:.0f} W/m2 (probe saw ~879, clear-sky plausible)"
    )
    if convention != "hour-ending":
        sys.exit("ssrd convention check FAILED: expected hour-ending")
    return result


def write(trace: pd.Series, out_dir: Path, stem: str) -> None:
    df = trace.rename("cf").to_frame().astype("float64")
    df.index.name = "utc_start"
    df.to_csv(out_dir / f"{stem}.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
    df.to_parquet(out_dir / f"{stem}.parquet")


def monthly_twh(cf: pd.Series, gw: float) -> pd.Series:
    return cf.groupby(cf.index.month).sum() * 0.5 * gw / 1e3


def pinned_2024_factors(raw_2024: dict) -> dict:
    """Exact (full-precision) calibration factors from the 2024 raw traces,
    guarded against drift: they must round (4 dp) to the pinned values in
    docs/notes/era5-cf-2024-report.md, else the 2024 cutout or the method
    has changed and Phase B must not proceed."""
    targets = calibration_targets()
    factors = {}
    for name, s in raw_2024.items():
        factor, _ = calibrate(s, targets[name])
        factor = float(factor)
        if round(factor, 4) != PINNED_FACTORS_2024[name]:
            sys.exit(
                f"{name}: derived 2024 factor {factor:.6f} does not round to "
                f"pinned {PINNED_FACTORS_2024[name]} — cutout or method drift"
            )
        factors[name] = factor
    return factors


def run_phase_a(repo: Path) -> None:
    """The pinned Phase A run: 2024, self-calibrating, validated against
    the observed pack. Unchanged from the reviewed 2026-07-02 derivation."""
    era5_dir = repo / "data" / "packs" / "era5" / "2024"
    out_dir = repo / "data" / "packs" / "2024" / "processed"

    ssrd_finding = verify_ssrd_convention(era5_dir)
    raw = derive_raw(era5_dir, 2024)
    targets = calibration_targets()

    report = {"ssrd_convention": ssrd_finding, "technologies": {}}
    calibrated = {}
    for name, s in raw.items():
        factor, cal = calibrate(s, targets[name])
        factor = float(factor)  # numpy scalar -> Python float (JSON)
        calibrated[name] = cal
        flagged = not (
            CALIBRATION_HONESTY_BAND[0] <= factor <= CALIBRATION_HONESTY_BAND[1]
        )
        report["technologies"][name] = {
            "raw_annual_cf": round(float(s.mean()), 4),
            "target_annual_cf": round(targets[name], 4),
            "calibration_factor": round(factor, 4),
            "outside_honesty_band_0.7_1.3": flagged,
            "clipped_periods_at_1": int((s * factor > 1.0).sum()),
        }
        print(
            f"{name}: raw annual CF {s.mean():.4f} -> target "
            f"{targets[name]:.4f}, factor {factor:.4f}"
            + ("  ** OUTSIDE HONESTY BAND — physical model off **" if flagged else "")
        )

    # Validation vs the pack's observed wind trace (D3 convention).
    obs = pd.read_parquet(out_dir / "wind_cf_2024.parquet")["wind_cf"]
    derived_total = (
        ONSHORE_GW * calibrated["onshore"] + OFFSHORE_GW * calibrated["offshore"]
    ) / (ONSHORE_GW + OFFSHORE_GW)
    r = float(np.corrcoef(derived_total.values, obs.values)[0, 1])
    r_raw = float(
        np.corrcoef(
            (ONSHORE_GW * raw["onshore"] + OFFSHORE_GW * raw["offshore"]).values,
            obs.values,
        )[0, 1]
    )
    print(f"half-hourly total-wind CF vs observed: r = {r:.4f} (raw: {r_raw:.4f})")
    report["wind_halfhourly_r_vs_observed"] = round(r, 4)

    # Monthly energy comparison (TWh) vs the observed monthly matrix.
    matrix = pd.read_csv(
        out_dir / "monthly_generation_2024.csv", index_col=0, parse_dates=True
    )
    monthly = pd.DataFrame(
        {
            "derived_wind_twh": monthly_twh(calibrated["onshore"], ONSHORE_GW)
            + monthly_twh(calibrated["offshore"], OFFSHORE_GW),
            "observed_wind_twh": (matrix["wind_incl_embedded"] / 1e3).set_axis(
                range(1, 13)
            ),
            "derived_solar_twh": monthly_twh(calibrated["solar"], SOLAR_GW),
            "observed_solar_twh": (matrix["solar_embedded"] / 1e3).set_axis(
                range(1, 13)
            ),
        }
    )
    monthly.index.name = "month"
    print(monthly.round(3).to_string())
    r_month_wind = float(
        np.corrcoef(monthly["derived_wind_twh"], monthly["observed_wind_twh"])[0, 1]
    )
    r_month_solar = float(
        np.corrcoef(monthly["derived_solar_twh"], monthly["observed_solar_twh"])[0, 1]
    )
    print(f"monthly energy r: wind {r_month_wind:.4f}, solar {r_month_solar:.4f}")
    report["monthly_twh"] = json.loads(monthly.round(4).to_json(orient="index"))
    report["monthly_r"] = {
        "wind": round(r_month_wind, 4),
        "solar": round(r_month_solar, 4),
    }

    for name, stem in (
        ("onshore", "gb_onshore_cf_2024"),
        ("offshore", "gb_offshore_cf_2024"),
        ("solar", "gb_solar_cf_2024"),
    ):
        write(calibrated[name], out_dir, stem)
    (out_dir / "era5_cf_report_2024.json").write_text(
        json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
    )
    print("written:", ", ".join(
        f"gb_{n}_cf_2024.parquet/.csv" for n in ("onshore", "offshore", "solar")
    ), "+ era5_cf_report_2024.json")


def parse_cf_years(spec: str) -> list[int]:
    """'2019-2024' -> [2019..2024]; '1987,1990-1995' -> union; sorted."""
    years: set[int] = set()
    for tok in spec.split(","):
        if "-" in tok:
            a, b = (int(p) for p in tok.split("-", 1))
            if b < a:
                sys.exit(f"bad --cf-years range: {tok}")
            years.update(range(a, b + 1))
        else:
            years.add(int(tok))
    return sorted(years)


def run_phase_b(repo: Path, years: list[int]) -> None:
    """Phase B: per-year CF traces with the pinned 2024 calibration (see
    docstring PHASE B). Refuses incomplete (in-progress) cutouts up front."""
    era5_root = repo / "data" / "packs" / "era5"
    out_dir = repo / "data" / "packs" / "cf"

    for year in years:
        n = len(sorted((era5_root / str(year)).glob(f"era5_gb_{year}-*.parquet")))
        if n != 12:
            sys.exit(
                f"{year}: cutout incomplete ({n}/12 monthly files) — "
                "in-progress years must not be derived; re-run when the "
                "fetch completes"
            )

    verify_ssrd_convention(era5_root / "2024")
    print("deriving 2024 raw traces (exact pinned calibration factors) ...")
    raw_2024 = derive_raw(era5_root / "2024", 2024)
    factors = pinned_2024_factors(raw_2024)
    print(
        "pinned factors confirmed: "
        + ", ".join(f"{n} {f:.4f}" for n, f in factors.items())
    )

    out_dir.mkdir(parents=True, exist_ok=True)
    for year in years:
        raw = raw_2024 if year == 2024 else derive_raw(era5_root / str(year), year)
        for name in ("onshore", "offshore", "solar"):
            trace = (raw[name] * factors[name]).clip(0.0, 1.0)
            write(trace, out_dir, f"gb_{name}_cf_{year}")
            print(
                f"{year} {name}: {len(trace)} periods, "
                f"annual mean CF {trace.mean():.4f}"
            )
    print(f"written {3 * len(years)} trace pairs to {out_dir}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument(
        "--cf-years",
        help="Phase B: derive per-year traces to data/packs/cf/ for these "
        "years (e.g. 2019-2024 or 1987,1990-1995) using the pinned 2024 "
        "calibration; omit for the Phase A 2024 run",
    )
    args = ap.parse_args()
    if args.cf_years:
        run_phase_b(args.repo_root, parse_cf_years(args.cf_years))
    else:
        run_phase_a(args.repo_root)


if __name__ == "__main__":
    main()
