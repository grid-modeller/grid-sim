#!/usr/bin/env python3
"""Derive the GB population-weighted 2 m temperature trace, 1985-2024.

Q5 heating-overlay data package (docs/notes/d9-heating-overlay.md, data
requirement 1): a single half-hourly trace `t2m_pop` (degrees Celsius) at
the D9 rule-2 pinned path data/weather/gb_t2m_pop.{parquet,csv}, derived
from the committed GB ERA5 cutout (data/packs/era5/<year>/, manifests
era5-2024.sha256 + era5-1985-2023.sha256).

THE METHOD IS THE PINNED EU t2m METHOD APPLIED TO GB: this script IMPORTS
the pinned GB functions from derive_cf.py (load_point_means, weighted_cf,
half_hourly_index, to_half_hourly, hours_in_year) — pinned code reuse, not
reimplementation, exactly as derive_cf_eu.py does for the EU temperature
traces. derive_cf.py itself is byte-unchanged; the GB CF derivation path
and ALL committed GB CF manifests are untouched.

Method (identical mechanics to the EU `<c>_t2m_<Y>` traces):
- Population weights: approximate GB city/metro clusters (millions,
  ONS-level public knowledge — the same honesty level as the EU TEMP
  weights and the GB fleet weights; only relative sizes matter).
  Scotland ~8.0%, Wales ~3.6% of the sampled weight (GB actuals ~8.3%,
  ~4.7% — Welsh population is concentrated in the sampled south).
- Each point = mean over the 3x3-cell ERA5 box (gb.BOX_HALF_DEG).
- t2m is instantaneous, valid AT the hourly label (same time base as
  wind); Kelvin -> Celsius; hourly -> half-hourly by the pinned linear
  interpolation (gb.to_half_hourly).
- Years 1985-2024 concatenated into ONE file (the D9 heating overlay
  consumes the whole reference window; per-year splitting is a scenario-
  loader concern the overlay does not have).

Also emits the RULING-A GROUND-MODEL CROSS-CHECK (D9 data requirement 7,
review ruling A): fits the single annual harmonic on the full 40-year
T_pop, applies the Kusuda-Achenbach damped/lagged wave at the nominal
shallow-horizontal-loop depth with cited GB soil thermal diffusivity, and
compares amplitude/phase/winter-minimum against the measured GB 100 cm
soil temperature climatology (Busby 2015, transcribed below). Numbers ->
data/weather/gb_t2m_pop_report.json; adjudication ->
docs/notes/q5-heating-data-report.md.

Deterministic: no network, no randomness, no wall-clock. Outputs are a
pure function of the GB cutouts and the constants below. Attribution
(ERA5): "Contains modified Copernicus Climate Change Service information
[2024]".

Usage:
    python derive_t2m_gb.py <repo-root>                # full 1985-2024, writes
    python derive_t2m_gb.py <repo-root> --years 2024   # spot check, no write
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

# ---------------------------------------------------------------------------
# GB population weights — APPROXIMATE city/metro clusters (name, lat degN,
# lon degE, weight in millions; ONS mid-year population / built-up-area
# public knowledge, the EU TEMP honesty level). Weights are normalised;
# only relative sizes matter. Sampled total ~42M of ~66M GB — unsampled
# population is distributed regionally like the sampled clusters.
# ---------------------------------------------------------------------------
GB_TEMP = [
    ("london", 51.50, -0.12, 9.6),
    ("se_home_counties", 51.40, -0.75, 2.5),   # Reading/Guildford/Slough belt
    ("kent_medway", 51.35, 0.55, 1.7),
    ("sussex_coast", 50.85, -0.35, 1.4),       # Brighton/Worthing/Eastbourne
    ("southampton_portsmouth", 50.85, -1.20, 1.6),
    ("bristol_bath", 51.45, -2.55, 1.4),
    ("sw_devon_cornwall", 50.55, -3.85, 1.2),  # Exeter/Plymouth/Torbay
    ("cardiff_s_wales", 51.55, -3.35, 1.5),    # Cardiff/Newport/Swansea belt
    ("birmingham_wm", 52.50, -1.90, 3.5),      # West Midlands conurbation
    ("east_midlands", 52.80, -1.20, 2.5),      # Nottingham/Derby/Leicester
    ("cambridge_east", 52.20, 0.15, 1.3),
    ("norwich_east_anglia", 52.60, 1.20, 0.9),
    ("manchester", 53.50, -2.25, 2.9),
    ("liverpool_merseyside", 53.40, -2.95, 1.6),
    ("leeds_west_yorks", 53.80, -1.55, 2.4),
    ("sheffield_s_yorks", 53.40, -1.45, 1.5),
    ("newcastle_tyneside", 55.00, -1.60, 1.2),
    ("glasgow_clyde", 55.86, -4.25, 1.8),
    ("edinburgh", 55.95, -3.20, 1.0),
    ("aberdeen_ne", 57.15, -2.10, 0.5),
]

# ---------------------------------------------------------------------------
# Ground-model constants (D9 rule 4, review ruling A / edit 5) — all cited.
#
# Loop depth z: 1.0 m nominal — "100 cm depth, the average depth of a
# horizontal ground loop collector" (Busby 2015); D9 band 1.0-1.2 m
# (GSHPA 2014 recommend 0.8-1.5 m, VDI 4640 1.2-1.5 m, both as cited in
# Busby 2015/2016). 1.0 m also matches the depth of the measured
# cross-check series exactly.
#
# Soil thermal diffusivity alpha: Busby 2016 (Thermal conductivity and
# diffusivity estimations for shallow geothermal systems, QJEGH 49;
# NORA open manuscript), 60 accepted determinations at 56 GB Met Office
# stations from the amplitude/phase damping of the measured seasonal
# cycle: texture-class medians loam 0.7173, sand 0.9961, clay 1.0295
# (x1e-6 m2/s); full site range 0.3517-2.4691e-6. Centre = midpoint of
# the texture-class median band (the fleet-power-factor precedent:
# cited range, centre used, band stated).
# ---------------------------------------------------------------------------
Z_LOOP_M = 1.0
Z_BAND_M = (1.0, 1.2)
ALPHA_M2_S = 0.87e-6
ALPHA_BAND_M2_S = (0.7173e-6, 1.0295e-6)
OMEGA_S = 2.0 * math.pi / (365.2425 * 86400.0)  # annual angular frequency

# ---------------------------------------------------------------------------
# Measured GB 100 cm soil temperature climatology — Busby 2015 ("UK shallow
# ground temperatures for ground coupled heat exchangers", QJEGH 48(3-4),
# 248-260; NORA open manuscript nora.nerc.ac.uk/id/eprint/512282), Table 1:
# per-station mean annual / seasonal min / seasonal max at 100 cm depth,
# REDUCED TO SEA LEVEL (0.65 C per 100 m), fitted seasonal cycle 2000-2010,
# Met Office stations. One station transcribed per population cluster
# (nearest with 100 cm data). Values (mean_c, smin_c, smax_c).
# Measured 100 cm phase (Busby 2015, Results): seasonal minima occur
# 30 Jan - 22 Feb (England), 7-23 Feb (Scotland); maxima 31 Jul - 22 Aug
# (England), 7-23 Aug (Scotland).
# Known offsets to state, both from Busby 2015: soil annual mean is ~1 C
# ABOVE the air annual mean (12 paired comparisons, range 0.5-2.0, avg
# 0.9); sea-level reduction makes elevated stations (Bradford, Sheffield)
# read slightly warm vs in-situ.
# Acknowledgement: transcribed values (c) NERC/BGS, published by
# permission of the British Geological Survey.
# ---------------------------------------------------------------------------
BUSBY_100CM = {
    "london": ("st_james_park", 13.0, 7.4, 18.7),          # urban (UHI ~+2.2)
    "se_home_counties": ("wisley", 12.2, 7.0, 17.3),
    "kent_medway": ("wye", 12.0, 6.7, 17.3),
    "sussex_coast": ("eastbourne", 12.9, 7.8, 18.1),
    "southampton_portsmouth": ("hurn", 11.6, 6.5, 16.7),
    "bristol_bath": ("rodney_stoke", 12.1, 7.0, 17.2),
    "sw_devon_cornwall": ("dunkeswell", 12.8, 7.5, 18.2),
    "cardiff_s_wales": ("whitchurch", 12.1, 8.0, 16.2),
    "birmingham_wm": ("coleshill", 11.6, 6.3, 17.0),
    "east_midlands": ("church_lawford", 12.0, 6.1, 17.9),
    "cambridge_east": ("cambridge_botanic", 11.7, 6.2, 17.2),
    "norwich_east_anglia": ("westleton", 11.9, 5.2, 18.5),
    "manchester": ("myerscough", 11.9, 5.7, 18.0),
    "liverpool_merseyside": ("myerscough", 11.9, 5.7, 18.0),
    "leeds_west_yorks": ("bradford", 11.7, 6.4, 17.0),
    "sheffield_s_yorks": ("sheffield", 12.0, 6.4, 17.6),
    "newcastle_tyneside": ("durham", 11.5, 6.1, 16.9),
    "glasgow_clyde": ("glasgow_bishopton", 10.8, 5.8, 15.8),
    "edinburgh": ("edinburgh_gogarbank", 10.3, 5.5, 15.1),
    "aberdeen_ne": ("dyce", 9.7, 5.2, 14.2),
}
MEASURED_MIN_WINDOW = ("01-30", "02-22")  # England 100 cm minima (Busby 2015)

# District-geothermal premise check (D9 rule 4 / edit 7): the draft
# COP_const from data/reference/heating-cop.toml must exceed the heat
# pumps' maximum record COP. Draft value assembled in heating-cop.toml
# (ADEME/BRGM 2024 production-basis 20 kWh_th/kWh_e, converted to the
# delivered-heat basis with DECC 2015 bulk-network losses + margin).
COP_CONST_DRAFT = 15.0
# When2Heat parameterisation (Ruhnau et al. 2019, transcribed in
# heating-cop.toml) for the premise check only — the engine package
# re-implements these under its own tests.
W2H_ASHP = (6.08, -0.09, 0.0005)
W2H_GSHP = (10.29, -0.21, 0.0012)
W2H_CORRECTION = 0.85
W2H_GSHP_HX_OFFSET_K = 5.0
W2H_MIN_DT_K = 15.0
RADIATOR_T0_C, RADIATOR_SLOPE = 40.0, 1.0  # T_sink = 40 - 1.0*T_amb


def derive_year(era5_root: Path, year: int) -> pd.Series:
    """One year's population-weighted t2m (Celsius), half-hourly UTC —
    the pinned GB machinery end to end."""
    points = gb.load_point_means(
        era5_root / str(year), {"temp": GB_TEMP}, year
    )["temp"]
    series = {name: df["t2m"] - 273.15 for name, df in points.items()}
    hourly = gb.weighted_cf(series, GB_TEMP)
    index = gb.half_hourly_index(year)
    assert len(index) == gb.hours_in_year(year) * 2
    half = gb.to_half_hourly(hourly, index)
    if half.isna().any():
        sys.exit(f"{year}: NaNs after interpolation")
    return half


def fit_annual_harmonic(trace: pd.Series) -> dict:
    """Least-squares single annual harmonic T = m + A*cos(w*t - phi) on the
    full trace (closed form, deterministic). Returns mean, amplitude, and
    the calendar day of the fitted minimum."""
    t_sec = (trace.index - trace.index[0]).total_seconds().to_numpy()
    x = np.column_stack(
        [np.ones_like(t_sec), np.cos(OMEGA_S * t_sec), np.sin(OMEGA_S * t_sec)]
    )
    coef, *_ = np.linalg.lstsq(x, trace.to_numpy(), rcond=None)
    m, a, b = (float(c) for c in coef)
    amp = math.hypot(a, b)
    phi = math.atan2(b, a)
    # Minimum where cos(w*t - phi) = -1 -> w*t = phi + pi (mod 2*pi).
    t_min = ((phi + math.pi) % (2.0 * math.pi)) / OMEGA_S
    min_date = trace.index[0] + pd.Timedelta(seconds=t_min)
    return {
        "mean_c": m,
        "amplitude_c": amp,
        "cos_coef": a,
        "sin_coef": b,
        "surface_min_doy_date": f"{min_date.month:02d}-{min_date.day:02d}",
        "_phi": phi,
    }


def ka_parameters(z: float, alpha: float) -> tuple[float, float]:
    """Kusuda-Achenbach damping and lag (days) at depth z, diffusivity
    alpha: damping = exp(-z*sqrt(w/2a)), lag = z/sqrt(2aw)."""
    damping = math.exp(-z * math.sqrt(OMEGA_S / (2.0 * alpha)))
    lag_days = z / math.sqrt(2.0 * alpha * OMEGA_S) / 86400.0
    return damping, lag_days


def ground_wave(trace: pd.Series, fit: dict, damping: float, lag_days: float) -> pd.Series:
    """D9 rule-4 undisturbed ground temperature at loop depth."""
    t_sec = (trace.index - trace.index[0]).total_seconds().to_numpy()
    phase = OMEGA_S * (t_sec - lag_days * 86400.0) - fit["_phi"]
    return pd.Series(
        fit["mean_c"] + fit["amplitude_c"] * damping * np.cos(phase),
        index=trace.index,
    )


def cross_check(trace: pd.Series, fit: dict) -> dict:
    """Ruling-A cross-check: model amplitude/phase/winter-minimum at 1 m
    vs the Busby 2015 measured 100 cm climatology, population-weighted
    with the SAME cluster weights as the trace."""
    weights = {n: w for n, _la, _lo, w in GB_TEMP}
    total = sum(weights.values())
    meas_mean = sum(BUSBY_100CM[n][1] * w for n, w in weights.items()) / total
    meas_smin = sum(BUSBY_100CM[n][2] * w for n, w in weights.items()) / total
    meas_smax = sum(BUSBY_100CM[n][3] * w for n, w in weights.items()) / total
    meas_amp = (meas_smax - meas_smin) / 2.0

    damping, lag_days = ka_parameters(Z_LOOP_M, ALPHA_M2_S)
    bands = {
        f"alpha_{a:.4e}": ka_parameters(Z_LOOP_M, a) for a in ALPHA_BAND_M2_S
    }
    model_amp = fit["amplitude_c"] * damping
    model_mean = fit["mean_c"]
    model_min = model_mean - model_amp
    meas_min = meas_smin

    surface_min = pd.Timestamp(f"2001-{fit['surface_min_doy_date']}")
    model_min_date = surface_min + pd.Timedelta(days=lag_days)
    lo = pd.Timestamp(f"2001-{MEASURED_MIN_WINDOW[0]}")
    hi = pd.Timestamp(f"2001-{MEASURED_MIN_WINDOW[1]}")
    phase_in_window = bool(lo <= model_min_date <= hi)

    return {
        "kusuda_achenbach": {
            "z_m": Z_LOOP_M,
            "alpha_m2_s": ALPHA_M2_S,
            "alpha_band_m2_s": list(ALPHA_BAND_M2_S),
            "damping": round(damping, 4),
            "lag_days": round(lag_days, 2),
            "band_damping_lag": {
                k: [round(d, 4), round(l, 2)] for k, (d, l) in bands.items()
            },
        },
        "model_at_1m": {
            "mean_c": round(model_mean, 2),
            "amplitude_c": round(model_amp, 2),
            "winter_min_c": round(model_min, 2),
            "min_date": f"{model_min_date.month:02d}-{model_min_date.day:02d}",
        },
        "measured_100cm_busby2015": {
            "mean_c": round(meas_mean, 2),
            "amplitude_c": round(meas_amp, 2),
            "winter_min_c": round(meas_min, 2),
            "min_window": list(MEASURED_MIN_WINDOW),
            "basis": "sea-level-reduced, fitted seasonal cycle 2000-2010, "
                     "population-weighted over the trace's cluster weights",
        },
        "deviations": {
            "mean_c": round(model_mean - meas_mean, 2),
            "amplitude_c": round(model_amp - meas_amp, 2),
            "amplitude_pct": round(100.0 * (model_amp - meas_amp) / meas_amp, 1),
            "winter_min_c": round(model_min - meas_min, 2),
            "phase_min_date_in_measured_window": phase_in_window,
        },
    }


def district_premise(trace: pd.Series, fit: dict) -> dict:
    """Edit-7 premise check: COP_const (draft) vs the heat pumps' maximum
    record COP under the transcribed When2Heat parameterisation."""
    t_pop = trace.to_numpy()
    sink = RADIATOR_T0_C - RADIATOR_SLOPE * t_pop
    dt_ashp = np.maximum(sink - t_pop, W2H_MIN_DT_K)
    c0, c1, c2 = W2H_ASHP
    cop_ashp = W2H_CORRECTION * (c0 + c1 * dt_ashp + c2 * dt_ashp**2)

    damping, lag_days = ka_parameters(Z_LOOP_M, ALPHA_M2_S)
    t_ground = ground_wave(trace, fit, damping, lag_days).to_numpy()
    dt_gshp = np.maximum(sink - (t_ground - W2H_GSHP_HX_OFFSET_K), W2H_MIN_DT_K)
    g0, g1, g2 = W2H_GSHP
    cop_gshp = W2H_CORRECTION * (g0 + g1 * dt_gshp + g2 * dt_gshp**2)

    max_hp = float(max(cop_ashp.max(), cop_gshp.max()))
    return {
        "cop_const_draft": COP_CONST_DRAFT,
        "ashp_record_cop_max": round(float(cop_ashp.max()), 3),
        "ashp_record_cop_min": round(float(cop_ashp.min()), 3),
        "gshp_record_cop_max": round(float(cop_gshp.max()), 3),
        "gshp_record_cop_min": round(float(cop_gshp.min()), 3),
        "cop_const_exceeds_hp_max": bool(COP_CONST_DRAFT > max_hp),
        "margin_ratio": round(COP_CONST_DRAFT / max_hp, 2),
    }


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("repo_root", type=Path)
    ap.add_argument("--years", default="1985-2024", help="e.g. 1985-2024 or 2024")
    args = ap.parse_args()
    repo = args.repo_root
    years = gb.parse_cf_years(args.years)
    full = years == list(range(1985, 2025))

    era5_root = repo / "data" / "packs" / "era5"
    out_dir = repo / "data" / "weather"

    for year in years:
        n = len(sorted((era5_root / str(year)).glob(f"era5_gb_{year}-*.parquet")))
        if n != 12:
            sys.exit(f"{year}: GB cutout incomplete ({n}/12 monthly files)")

    parts = []
    annual_means = {}
    for year in years:
        s = derive_year(era5_root, year)
        parts.append(s)
        annual_means[str(year)] = round(float(s.mean()), 2)
        print(
            f"{year}: {len(s)} periods, annual mean {s.mean():.2f} C, "
            f"min {s.min():.2f}, max {s.max():.2f}"
        )
    trace = pd.concat(parts)
    trace.name = "t2m_pop"

    # Validation (derive-side; validate_t2m_gb.py re-asserts independently).
    expected = sum(gb.hours_in_year(y) * 2 for y in years)
    if len(trace) != expected:
        sys.exit(f"trace has {len(trace)} periods, expected {expected}")
    diffs = trace.index.to_series().diff().dropna().unique()
    if len(diffs) != 1 or diffs[0] != pd.Timedelta(minutes=30):
        sys.exit("index is not strictly uniform 30-min (incl. year boundaries)")
    if trace.isna().any():
        sys.exit("NaNs in final trace")
    if not (-25.0 < trace.min() and trace.max() < 40.0):
        sys.exit(f"implausible range: [{trace.min():.2f}, {trace.max():.2f}] C")
    if not (8.0 < trace.mean() < 12.0):
        sys.exit(f"implausible record mean: {trace.mean():.2f} C")
    print(
        f"validated: {len(trace)} periods, uniform 30-min UTC, no NaNs, "
        f"range [{trace.min():.2f}, {trace.max():.2f}] C, "
        f"record mean {trace.mean():.2f} C"
    )

    if not full:
        print("partial-year run: nothing written (full-sweep only)")
        return

    fit = fit_annual_harmonic(trace)
    report = {
        "trace": {
            "path": "data/weather/gb_t2m_pop.parquet",
            "column": "t2m_pop",
            "periods": len(trace),
            "years": "1985-2024",
            "record_mean_c": round(float(trace.mean()), 3),
            "record_min_c": round(float(trace.min()), 2),
            "record_max_c": round(float(trace.max()), 2),
            "annual_mean_c": annual_means,
        },
        "annual_harmonic_fit": {
            k: (round(v, 4) if isinstance(v, float) else v)
            for k, v in fit.items()
            if not k.startswith("_")
        },
        "ground_model_cross_check": cross_check(trace, fit),
        "district_cop_premise_check": district_premise(trace, fit),
        "attribution": "Contains modified Copernicus Climate Change Service "
                       "information [2024]",
    }

    out_dir.mkdir(parents=True, exist_ok=True)
    df = trace.to_frame().astype("float64")
    df.index.name = "utc_start"
    df.to_csv(out_dir / "gb_t2m_pop.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
    df.to_parquet(out_dir / "gb_t2m_pop.parquet")
    (out_dir / "gb_t2m_pop_report.json").write_text(
        json.dumps(report, indent=2, sort_keys=True, default=float) + "\n"
    )
    print(f"written: {out_dir}/gb_t2m_pop.parquet/.csv + gb_t2m_pop_report.json")
    print(json.dumps(report["ground_model_cross_check"], indent=2))
    print(json.dumps(report["district_cop_premise_check"], indent=2))


if __name__ == "__main__":
    main()
