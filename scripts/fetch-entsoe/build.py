#!/usr/bin/env python3
"""Build the ENTSO-E Stage 5 pack: raw XML -> processed UTC traces.

Deterministic (no network, no randomness, no wall-clock). Reads
data/packs/entsoe-2024/raw/, writes data/packs/entsoe-2024/processed/.

Conventions (docs/03, docs/06, and the fetch-2024 precedent):
- Output index: `utc_start`, strictly uniform 30-min UTC, 17,568 periods
  (2024 is a leap year), Parquet + CSV both.
- ENTSO-E quantities are AVERAGE MW over the market time unit. The
  platform's native resolution is a mixture not only across zones but
  ACROSS MONTHS WITHIN one series (observed: NO2 flows are PT15M early
  2024 and PT60M later; loads are fr/no2/dk1 PT60M, be/nl/delu PT15M,
  ie PT30M). Each document Period is therefore normalised to the pack's
  30-min grid individually, by an energy-preserving rule:
    PT15M -> PT30M: arithmetic mean of the two 15-min values (a half-hour
                    with either slot missing stays NaN — gaps propagate);
    PT30M: taken as-is;
    PT60M -> PT30M: repeat the hourly value into both half-hours.
  The per-series resolution mixture is recorded in
  build_report_entsoe_2024.json — do not assume it, read it.
- Curve semantics: the curveType is read from each TimeSeries and both
  conventions are handled explicitly — A03 (variable-sized blocks): a
  missing position repeats the previous value until the next stated
  position, to the end of the Period; A01 (fixed blocks): a missing
  position is a genuine gap (NaN). OBSERVED IN THE 2024 FETCH (review-
  verified 2026-07-03 across all 261 raw documents): EVERY TimeSeries —
  A11 flows AND A65/A75/A72 load/generation/reservoir — declares A03,
  and A03 hold-forward was actually exercised on load (e.g. FR Dec 31:
  27/96 points) and generation (125 periods). Do not assume A01 for any
  document type; trust the declared curveType only.
- "No matching data" acknowledgements: for a FLOW direction-month this
  means no flow was reported in that direction (the platform omits
  zero-only series) -> zeros, counted in the build report. For any other
  data type an ACK is an error (recorded; validate.py fails on it).
  (2026-07-03 fetch: no ACKs occurred at all.)
- Gap rule (evidence, not aspiration): internal gaps <= 2 h (4 slots on
  the 30-min grid) are linearly interpolated and counted; longer gaps are
  left NaN for validate.py to fail on. All gaps are in the build report.
- Flow sign convention: net = import - export, positive = import to GB
  (matches the NESO *_FLOW / Elexon INT* convention in the 2024 pack).

Usage: python build.py <repo-root>
"""

import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import pandas as pd

MONTHS = [f"2024-{m:02d}" for m in range(1, 13)]
BORDERS = ["fr", "be", "nl", "no2", "dk1", "ie"]
LOAD_ZONES = ["fr", "be", "nl", "delu", "no2", "dk1", "ie"]

YEAR_START = pd.Timestamp("2024-01-01T00:00:00Z")
YEAR_END = pd.Timestamp("2025-01-01T00:00:00Z")
GRID_30M = pd.date_range(YEAR_START, YEAR_END, freq="30min", inclusive="left")

RES_MIN = {"PT15M": 15, "PT30M": 30, "PT60M": 60}

# ENTSO-E production-source (PSR) types -> column stems, and the scenario
# technology ids they map onto where the mapping is clean. Lossy mappings
# are documented in docs/notes/entsoe-stage5-pack-report.md; this feeds D5
# evidence, not final fleet files.
PSR = {
    "B01": ("biomass", "biomass"),
    "B02": ("fossil_brown_coal", "coal"),  # lossy: lignite folded into coal
    "B03": ("fossil_coal_gas", ""),
    "B04": ("fossil_gas", "ccgt"),  # lossy: includes OCGT/CHP gas
    "B05": ("fossil_hard_coal", "coal"),
    "B06": ("fossil_oil", ""),
    "B07": ("fossil_oil_shale", ""),
    "B08": ("fossil_peat", ""),
    "B09": ("geothermal", ""),
    "B10": ("hydro_pumped", ""),  # storage, not a fleet technology
    "B11": ("hydro_ror", "hydro"),
    "B12": ("hydro_reservoir", "hydro"),  # lossy: GB 'hydro' has no split
    "B13": ("marine", ""),
    "B14": ("nuclear", "nuclear"),
    "B15": ("other_renewable", ""),
    "B16": ("solar", "solar"),
    "B17": ("waste", ""),
    "B18": ("wind_offshore", "offshore_wind"),
    "B19": ("wind_onshore", "onshore_wind"),
    "B20": ("other", ""),
}


def local(tag: str) -> str:
    return tag.split("}")[-1]


def find1(el, name):
    for child in el.iter():
        if local(child.tag) == name:
            return child
    return None


def parse_doc(path: Path) -> tuple[bool, list[dict]]:
    """Parse one TP document. Returns (is_ack, timeseries list).

    Each timeseries dict: curve, psr (or None), in_dom, out_dom, unit,
    periods=[(start, end, resolution, {position: quantity})].
    """
    root = ET.parse(path).getroot()
    if "Acknowledgement" in local(root.tag):
        return True, []
    out = []
    for ts in root:
        if local(ts.tag) != "TimeSeries":
            continue
        curve = find1(ts, "curveType")
        psr_el = find1(ts, "psrType")
        in_dom = find1(ts, "in_Domain.mRID")
        if in_dom is None:
            in_dom = find1(ts, "inBiddingZone_Domain.mRID")
        out_dom = find1(ts, "out_Domain.mRID")
        if out_dom is None:
            out_dom = find1(ts, "outBiddingZone_Domain.mRID")
        unit = find1(ts, "quantity_Measure_Unit.name")
        periods = []
        for per in ts:
            if local(per.tag) != "Period":
                continue
            ti = [c for c in per if local(c.tag) == "timeInterval"][0]
            start = pd.Timestamp([c.text for c in ti if local(c.tag) == "start"][0])
            end = pd.Timestamp([c.text for c in ti if local(c.tag) == "end"][0])
            res = [c.text for c in per if local(c.tag) == "resolution"][0]
            pts = {}
            for pt in per:
                if local(pt.tag) != "Point":
                    continue
                pos = int([c.text for c in pt if local(c.tag) == "position"][0])
                qty = float([c.text for c in pt if local(c.tag) == "quantity"][0])
                pts[pos] = qty
            periods.append((start, end, res, pts))
        out.append(
            {
                "curve": curve.text if curve is not None else "A01",
                "psr": psr_el.text if psr_el is not None else None,
                "in_dom": in_dom.text if in_dom is not None else None,
                "out_dom": out_dom.text if out_dom is not None else None,
                "unit": unit.text if unit is not None else None,
                "periods": periods,
            }
        )
    return False, out


def expand_period(start, end, res, pts, curve) -> pd.Series:
    """Expand one Period's points onto its native uniform grid."""
    step = pd.Timedelta(minutes=RES_MIN[res])
    idx = pd.date_range(start, end, freq=step, inclusive="left")
    n = len(idx)
    vals = [float("nan")] * n
    if curve == "A03":
        # variable-sized blocks: value holds until the next stated position
        positions = sorted(pts)
        for i, p in enumerate(positions):
            stop = positions[i + 1] - 1 if i + 1 < len(positions) else n
            for k in range(p - 1, min(stop, n)):
                vals[k] = pts[p]
    else:  # A01 fixed blocks: absent position = genuine gap
        for p, q in pts.items():
            if 1 <= p <= n:
                vals[p - 1] = q
    return pd.Series(vals, index=idx, dtype="float64")


def period_to_30m(start, end, res, pts, curve) -> pd.Series:
    """Expand one Period natively, then normalise to the 30-min grid
    (energy-preserving rule in the module docstring)."""
    s = expand_period(start, end, res, pts, curve)
    if res == "PT30M":
        return s
    if res == "PT15M":
        out = s.resample("30min").mean()
        cnt = s.notna().astype("int64").resample("30min").sum()
        out[cnt < 2] = float("nan")
        return out
    if res == "PT60M":
        idx30 = pd.date_range(start, end, freq="30min", inclusive="left")
        return s.reindex(idx30, method="ffill", limit=1)
    raise RuntimeError(f"unexpected resolution {res}")


def combine_pieces(
    pieces: list[pd.Series],
    resolutions: dict[str, int],
    units: set,
    report: dict,
    key: str,
    zero_fill_months: list[str] | None = None,
) -> pd.Series:
    """30-min pieces -> one series on the full-year grid, gap-filled per
    the <=2 h rule, stats recorded under report[key]."""
    s = pd.Series(float("nan"), index=GRID_30M, dtype="float64")
    for piece in pieces:
        piece = piece[(piece.index >= YEAR_START) & (piece.index < YEAR_END)]
        piece = piece[piece.notna()]
        s.loc[piece.index] = piece
    if zero_fill_months:
        for ym in zero_fill_months:
            m0 = pd.Timestamp(f"{ym}-01T00:00:00Z")
            m1 = (m0 + pd.Timedelta(days=32)).replace(day=1)
            block = s.loc[(s.index >= m0) & (s.index < m1)]
            s.loc[block.index[block.isna()]] = 0.0
    missing = int(s.isna().sum())
    filled = s.interpolate(method="linear", limit=4, limit_area="inside")
    report[key] = {
        "native_resolutions": dict(sorted(resolutions.items())),
        "unit": sorted(u for u in units if u),
        "zero_filled_ack_months": zero_fill_months or [],
        "missing_30m_slots": missing,
        "interpolated_slots": missing - int(filled.isna().sum()),
        "unfilled_slots": int(filled.isna().sum()),
        "first_gap_slots": [str(t) for t in s.index[s.isna()][:50]],
    }
    return filled


def assemble_files(
    files: list[Path], report: dict, key: str, ack_is_zero: bool
) -> pd.Series:
    """Monthly single-series documents -> one 30-min series."""
    pieces: list[pd.Series] = []
    resolutions: dict[str, int] = {}
    units: set = set()
    ack_months = []
    for f in files:
        is_ack, tss = parse_doc(f)
        if is_ack:
            ack_months.append(f.stem.rsplit("_", 1)[1])
            continue
        for ts in tss:
            units.add(ts["unit"])
            for start, end, res, pts in ts["periods"]:
                resolutions[res] = resolutions.get(res, 0) + 1
                pieces.append(period_to_30m(start, end, res, pts, ts["curve"]))
    if ack_months and not ack_is_zero:
        report[key + "_ack_error"] = {"ack_months_error": ack_months}
    if not pieces and not ack_months:
        raise RuntimeError(f"{key}: no data in any document")
    return combine_pieces(
        pieces,
        resolutions,
        units,
        report,
        key,
        zero_fill_months=ack_months if ack_is_zero else None,
    )


def write(df: pd.DataFrame, out_dir: Path, stem: str) -> None:
    df.to_csv(out_dir / f"{stem}.csv", date_format="%Y-%m-%dT%H:%M:%SZ")
    df.to_parquet(out_dir / f"{stem}.parquet")


# NESO GB-side per-link columns (2024 validation pack) per ENTSO-E border.
NESO_LINKS = {
    "fr": ["ifa_flow", "ifa2_flow", "eleclink_flow"],
    "be": ["nemo_flow"],
    "nl": ["britned_flow"],
    "no2": ["nsl_flow"],
    "dk1": ["viking_flow"],
    "ie": ["moyle_flow", "east_west_flow", "greenlink_flow"],
}


def fill_flows_from_neso(b, imp, exp, pack2024, report):
    """Documented, flagged repair for flow gaps longer than the 2 h
    interpolation limit (2024: only the GB<->IE(SEM) border — 80 slots of
    EirGrid publication outages): fill from the NESO GB-side per-link
    actuals in data/packs/2024, which measure the same physical quantity
    at the GB end of the same links and reconcile with ENTSO-E to
    corr >= 0.99 elsewhere. Every filled timestamp is recorded in the
    build report. This is a repair from an already-adopted primary
    source, not a silent substitution."""
    neso = (
        pd.read_parquet(pack2024 / "demand_2024.parquet")[NESO_LINKS[b]]
        .sum(axis=1)
        .astype("float64")
    )
    neso.index = GRID_30M
    mask = imp.isna() | exp.isna()
    imp, exp = imp.copy(), exp.copy()
    imp[mask] = neso[mask].clip(lower=0.0)
    exp[mask] = (-neso[mask]).clip(lower=0.0)
    for key in (f"flows_{b}_imp", f"flows_{b}_exp"):
        report[key]["filled_from_neso_slots"] = int(mask.sum())
        report[key]["filled_from_neso_times"] = [str(t) for t in imp.index[mask]]
        report[key]["unfilled_slots"] = 0
    return imp, exp


def fill_day_offset(s, report, key):
    """Documented, flagged repair for load gaps longer than the 2 h
    interpolation limit (2024: only IE-SEM — 74 slots, longest run 22 h):
    fill with the mean of the same half-hour one day earlier and one day
    later (preserves the diurnal shape; linear interpolation across most
    of a day would flatten the demand peaks). Iterated up to 3 times for
    gap clusters; every filled timestamp is recorded."""
    s = s.copy()
    orig_na = s.isna()
    for _ in range(3):
        na_idx = s.index[s.isna()]
        if not len(na_idx):
            break
        prev = s.shift(freq="1D").reindex(s.index)
        nxt = s.shift(freq="-1D").reindex(s.index)
        cand = pd.concat([prev, nxt], axis=1).mean(axis=1)
        s[na_idx] = cand[na_idx]
    filled = orig_na & s.notna()
    report[key]["day_offset_filled_slots"] = int(filled.sum())
    report[key]["day_offset_filled_times"] = [str(t) for t in s.index[filled]]
    report[key]["unfilled_slots"] = int(s.isna().sum())
    return s


def build_flows(raw: Path, out: Path, report: dict, pack2024: Path) -> None:
    cols = {}
    for b in BORDERS:
        imp = assemble_files(
            [raw / f"flows_{b}_imp_{ym}.xml" for ym in MONTHS],
            report,
            f"flows_{b}_imp",
            ack_is_zero=True,
        )
        exp = assemble_files(
            [raw / f"flows_{b}_exp_{ym}.xml" for ym in MONTHS],
            report,
            f"flows_{b}_exp",
            ack_is_zero=True,
        )
        if int(imp.isna().sum()) or int(exp.isna().sum()):
            imp, exp = fill_flows_from_neso(b, imp, exp, pack2024, report)
        cols[f"{b}_imp"] = imp
        cols[f"{b}_exp"] = exp
        cols[f"{b}_net"] = imp - exp
    df = pd.DataFrame(cols, index=GRID_30M)
    df.index.name = "utc_start"
    write(df, out, "flows_gb_entsoe_2024")


def build_load(raw: Path, out: Path, report: dict) -> None:
    for z in LOAD_ZONES:
        s = assemble_files(
            [raw / f"load_{z}_{ym}.xml" for ym in MONTHS],
            report,
            f"load_{z}",
            ack_is_zero=False,
        )
        if int(s.isna().sum()):
            s = fill_day_offset(s, report, f"load_{z}")
        df = pd.DataFrame({"load_mw": s}, index=GRID_30M)
        df.index.name = "utc_start"
        write(df, out, f"load_{z}_2024")


def build_capacity(raw: Path, out: Path, report: dict) -> None:
    rows = []
    for z in LOAD_ZONES:
        is_ack, tss = parse_doc(raw / f"capacity_{z}_2024.xml")
        if is_ack:
            report[f"capacity_{z}_ack_error"] = {"ack_months_error": ["capacity"]}
            continue
        for ts in tss:
            psr = ts["psr"]
            stem, tech = PSR.get(psr, (psr, ""))
            for _start, _end, _res, pts in ts["periods"]:
                rows.append(
                    {
                        "zone": z,
                        "psr_code": psr,
                        "psr_name": stem,
                        "technology": tech,
                        "capacity_mw": pts.get(1, float("nan")),
                    }
                )
    df = pd.DataFrame(rows).sort_values(["zone", "psr_code"]).set_index("zone")
    write(df, out, "capacity_2024")


def build_generation(raw: Path, out: Path, report: dict) -> None:
    """NO2 + NO actual generation per type. Generation TimeSeries carry
    inBiddingZone_Domain; consumption (pumping) carry outBiddingZone_Domain
    and get a `_con` suffix."""
    for z in ["no2", "no"]:
        pieces_by_col: dict[str, list] = {}
        res_by_col: dict[str, dict] = {}
        units: set = set()
        months_by_col: dict[str, set] = {}
        for ym in MONTHS:
            is_ack, tss = parse_doc(raw / f"gen_{z}_{ym}.xml")
            if is_ack:
                report[f"gen_{z}_{ym}_ack_error"] = {"ack_months_error": [ym]}
                continue
            for ts in tss:
                stem, _ = PSR.get(ts["psr"], (ts["psr"], ""))
                col = stem if ts["in_dom"] else f"{stem}_con"
                units.add(ts["unit"])
                months_by_col.setdefault(col, set()).add(ym)
                for start, end, res, pts in ts["periods"]:
                    r = res_by_col.setdefault(col, {})
                    r[res] = r.get(res, 0) + 1
                    pieces_by_col.setdefault(col, []).append(
                        period_to_30m(start, end, res, pts, ts["curve"])
                    )
        cols = {}
        for col, pieces in sorted(pieces_by_col.items()):
            key = f"gen_{z}_{col}"
            s = combine_pieces(pieces, res_by_col[col], units, report, key)
            # A month whose document simply omits this PSR series means
            # "nothing reported", not a gap (the platform drops empty
            # series) -> zeros, recorded (2024: NO-aggregate solar is
            # absent outside May-Nov; Norwegian winter solar ~ 0).
            absent = [ym for ym in MONTHS if ym not in months_by_col[col]]
            for ym in absent:
                m0 = pd.Timestamp(f"{ym}-01T00:00:00Z")
                m1 = (m0 + pd.Timedelta(days=32)).replace(day=1)
                blk = s.loc[(s.index >= m0) & (s.index < m1)]
                s.loc[blk.index[blk.isna()]] = 0.0
            if absent:
                report[key]["zero_filled_absent_months"] = absent
                report[key]["unfilled_slots"] = int(s.isna().sum())
            if z == "no" and int(s.isna().sum()):
                # The NO-aggregate file exists only for the D5 zone-
                # granularity note; residual sparse-reporting slots are
                # zero-filled and counted rather than failing the pack
                # (2024: 236 edge slots of the patchy solar series). The
                # NO2 evidence file gets NO such indulgence.
                n = int(s.isna().sum())
                s = s.fillna(0.0)
                report[key]["zero_filled_residual_slots"] = n
                report[key]["unfilled_slots"] = 0
            cols[col] = s
        df = pd.DataFrame(cols, index=GRID_30M)
        df.index.name = "utc_start"
        write(df, out, f"{z}_generation_2024")


def build_reservoir(raw: Path, out: Path, report: dict) -> None:
    """Weekly reservoir filling (A72) + inflow proxy. The proxy is stated,
    not measured: inflow(w) ~= storage(w+1) - storage(w) + reservoir-hydro
    generation energy in week w (B12 only; pumped-storage recharge B10 is
    ignored — small in NO2/NO relative to reservoir hydro, and stated in
    the evidence note). Last week has no successor -> NaN."""
    for z in ["no2", "no"]:
        is_ack, tss = parse_doc(raw / f"reservoir_{z}_2024.xml")
        if is_ack:
            raise RuntimeError(f"reservoir_{z}: acknowledgement (no data)")
        rows = []
        unit = None
        for ts in tss:
            unit = ts["unit"]
            for start, _end, res, pts in ts["periods"]:
                if res != "P7D":
                    raise RuntimeError(f"reservoir_{z}: unexpected res {res}")
                for pos in sorted(pts):
                    rows.append(
                        {
                            "week_start_utc": start + pd.Timedelta(days=7 * (pos - 1)),
                            "storage_mwh": pts[pos],
                        }
                    )
        report[f"reservoir_{z}"] = {
            "native_resolutions": {"P7D": len(rows)},
            "unit": [unit] if unit else [],
        }
        df = pd.DataFrame(rows).drop_duplicates("week_start_utc")
        df = df.sort_values("week_start_utc").set_index("week_start_utc")
        # inflow proxy from the built generation trace (30-min MW -> MWh)
        gen = pd.read_parquet(out / f"{z}_generation_2024.parquet")
        res_gen_mwh = gen["hydro_reservoir"] * 0.5
        nxt = list(df.index[1:]) + [None]
        inflow = []
        for wk, wk_next in zip(df.index, nxt):
            if wk_next is None or wk < YEAR_START or wk_next > YEAR_END:
                inflow.append(float("nan"))
                continue
            gen_wk = res_gen_mwh[
                (res_gen_mwh.index >= wk) & (res_gen_mwh.index < wk_next)
            ]
            delta = df["storage_mwh"].loc[wk_next] - df["storage_mwh"].loc[wk]
            inflow.append(delta + float(gen_wk.sum()))
        df["inflow_proxy_mwh"] = inflow
        write(df, out, f"reservoir_{z}_2024")


def main() -> None:
    repo = Path(sys.argv[1])
    raw = repo / "data" / "packs" / "entsoe-2024" / "raw"
    out = repo / "data" / "packs" / "entsoe-2024" / "processed"
    out.mkdir(parents=True, exist_ok=True)
    pack2024 = repo / "data" / "packs" / "2024" / "processed"
    report: dict = {}

    build_flows(raw, out, report, pack2024)
    build_load(raw, out, report)
    build_capacity(raw, out, report)
    build_generation(raw, out, report)
    build_reservoir(raw, out, report)

    (out / "build_report_entsoe_2024.json").write_text(
        json.dumps(report, indent=2, sort_keys=True)
    )
    print("build complete; report -> build_report_entsoe_2024.json")


if __name__ == "__main__":
    main()
