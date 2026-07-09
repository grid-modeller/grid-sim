#!/usr/bin/env python3
"""Build the FR per-type generation + reservoir traces (Stage 5 A2 red
remediation data package, 2026-07-03).

Why (docs/notes/stage-5-review.md ruling 1): the 5-zone scenario models
FR hydro with a flat availability, overstating FR peak scarcity; the fix
is observed FR per-type traces so FR reservoir(+pumped) hydro can be
wired through the schema-v4 energy_budget machinery exactly as NO2 is.
This script builds from the raw A75 FR XML already on disk (calibration
fetch, 2026-07-03) plus the one newly fetched A72 FR document; it fetches
nothing itself (fetch.py is the only network script).

Outputs (data/packs/entsoe-2024/processed/):
    fr_generation_2024.{parquet,csv}
        17,568-row UTC half-hourly frame, one column per FR PSR series
        (all 12 production types RTE publishes for 2024, average MW) plus
        `hydro_pumped_con` (pumping consumption, see pair rule below).
    reservoir_fr_2024.{parquet,csv}
        53 weekly rows: week_start_utc, storage_mwh, inflow_proxy_mwh —
        same conventions as reservoir_no2_2024.
    build_report_fr_2024.json
        per-series gap/repair record (same honesty as build.py's report).

Existing processed files are NOT rebuilt or touched — the committed
entsoe-2024.sha256 entries for them stay byte-valid (the build_gen_agg.py
precedent); the five new files are appended to the manifest in the same
change that adds this script.

Assembly rules (build.py's, reused by import, not reimplemented): each
Period normalised to the 30-min UTC grid (PT15M mean / PT60M repeat,
energy-preserving), curveType read per TimeSeries (everything observed in
the 2024 FR documents declares A03 — never assume A01), internal gaps
<= 2 h linearly interpolated and counted, longer gaps repaired only by a
documented rule below. All 12 months must be present for every kept
series (FR is evidence-grade: no absent-month zero-fill indulgence).

FR-SPECIFIC DATA SEMANTICS (evidence, established before any repair):

1. B10 pumped storage is published as a MUTUALLY EXCLUSIVE PAIR.
   RTE reports the B10 generation TimeSeries only while the fleet is net
   generating and the B10 consumption TimeSeries only while it is net
   pumping: of 17,568 slots, ALL 8,194 gen-missing slots have the con
   series actively reporting, all 8,796 con-missing slots have the gen
   series reporting (8,792 of them > 0 MW), and ZERO slots are missing on
   both sides. Gen gaps cluster at night/midday (pumping hours: 636
   missing slots at 02 UTC vs 14 at 18 UTC); con gaps cluster at the
   evening peak (generating hours). The absent side therefore means "the
   fleet is doing the other thing" = 0 MW, and the correct repair is
   PAIR-FILL WITH ZERO. Interpolating across these windows would invent
   generation while the fleet is pumping — that is precisely how the
   aggregation_gen_2024 FR hydro_pumped figure (9,456 GWh, built with the
   generic day-offset repair ladder for the CF-anchor use where B10 was
   out of scope) overstates the pair-rule energy (6,930 GWh) by ~2.5 TWh.
   Anything consuming FR B10 energy must use THIS trace, not the
   aggregation table.

2. Consumption series other than B10 are excluded. FR 2024 carries two
   single-month auxiliary-consumption fragments (fossil_hard_coal_con,
   one month, 0.1 GWh over ~60 reported slots; wind_offshore_con, one
   month, 0.2 GWh): not production types, 99.7 % absent, meaningless on a
   17,568 grid. Excluded and recorded in the report with their
   reported-slot energies (the build_gen_agg.py precedent).

3. Long gaps in fossil_hard_coal (62 slots) and wind_offshore (44 slots)
   generation take the documented day-offset repair (mean of the same
   half-hour one day earlier/later — build.fill_day_offset, the IE-SEM
   rule), every filled timestamp counted. Every other generation series
   is gap-free at source.

4. A72 weekly reservoir filling: one A03/P7D TimeSeries, MWh stored
   energy, 53 weeks anchored 2023-12-31T23:00Z — French weeks run Monday
   00:00 Europe/Paris (= Sunday 23:00 UTC at the winter anchor; the
   platform serialises a uniform 7-day UTC grid from it). Same
   inflow-proxy convention as reservoir_no2_2024: inflow(w) ~= ΔStorage +
   B12 reservoir-generation energy in week w; the two year-boundary weeks
   have no computable proxy. CAVEAT (stated, not hidden): the proxy
   ignores pumped recharge, which for FR is proportionally larger than
   for NO2 (6.06 TWh pumping vs 17.44 TWh B12 generation, vs NO2's
   negligible ratio), and whether RTE's A72 perimeter includes pumped
   upper basins is not stated by the platform. The proxy is a
   seasonal-shape indication only; the budget evidence is the storage
   trajectory + the B12/B10 generation traces themselves.

Licence (docs/notes/entsoe-stage5-pack-report.md §1): A75 generation per
type and A72 reservoir filling are NOT on the ENTSO-E CC-BY free-re-use
list; this use is the GTC clause-3.1 case — internal validation/scenario
evidence, fetched and built locally, never redistributed (pack
git-ignored; only checksums committed). Attribution on anything derived:
"Source: ENTSO-E Transparency Platform".

Deterministic: no network, no wall-clock, no randomness; pure function of
the raw XML under the pinned venv (requirements.txt).

Usage: python build_fr.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build import (  # noqa: E402  (pinned reuse of the pack's assembly rules)
    GRID_30M,
    MONTHS,
    PSR,
    YEAR_END,
    YEAR_START,
    combine_pieces,
    fill_day_offset,
    parse_doc,
    period_to_30m,
    write,
)


def overlay(pieces: list[pd.Series]) -> pd.Series:
    """30-min pieces -> full-year grid, NO gap repair (combine_pieces'
    overlay step only — used for the B10 pair, where the standard <= 2 h
    interpolation would invent generation during pumping windows)."""
    s = pd.Series(float("nan"), index=GRID_30M, dtype="float64")
    for piece in pieces:
        piece = piece[(piece.index >= YEAR_START) & (piece.index < YEAR_END)]
        piece = piece[piece.notna()]
        s.loc[piece.index] = piece
    return s


def build_fr_generation(raw: Path, out: Path, report: dict) -> pd.DataFrame:
    pieces_by_col: dict[str, list] = {}
    res_by_col: dict[str, dict] = {}
    units: set = set()
    months_by_col: dict[str, set] = {}
    for ym in MONTHS:
        is_ack, tss = parse_doc(raw / f"gen_fr_{ym}.xml")
        if is_ack:
            raise RuntimeError(f"gen_fr_{ym}: acknowledgement (no data)")
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

    # Rule 2: drop non-B10 consumption fragments, record their coverage.
    for col in sorted(c for c in pieces_by_col if c.endswith("_con")):
        if col == "hydro_pumped_con":
            continue
        frag = overlay(pieces_by_col.pop(col))
        report[f"gen_fr_{col}_excluded"] = {
            "reason": "single-month auxiliary-consumption fragment, "
            "not a production type (module docstring rule 2)",
            "months_present": sorted(months_by_col[col]),
            "reported_slots": int(frag.notna().sum()),
            "reported_energy_gwh": round(float(frag.sum() * 0.5 / 1e3), 3),
        }

    # FR is evidence-grade: every kept series must cover all 12 months.
    for col in pieces_by_col:
        if len(months_by_col[col]) != 12:
            raise RuntimeError(
                f"gen_fr_{col}: only months {sorted(months_by_col[col])}"
            )

    cols = {}
    # Rule 1: the B10 pair, overlay-only then pair-fill with zero.
    pair = {c: overlay(pieces_by_col.pop(c)) for c in ("hydro_pumped", "hydro_pumped_con")}
    both_missing = int((pair["hydro_pumped"].isna() & pair["hydro_pumped_con"].isna()).sum())
    if both_missing:
        raise RuntimeError(
            f"B10 pair: {both_missing} slots missing on BOTH sides — the "
            "mutually-exclusive-pair evidence no longer holds; re-examine "
            "before choosing a repair rule"
        )
    for col, other in (
        ("hydro_pumped", "hydro_pumped_con"),
        ("hydro_pumped_con", "hydro_pumped"),
    ):
        s = pair[col]
        fill = s.isna() & pair[other].notna()
        s = s.copy()
        s[fill] = 0.0
        report[f"gen_fr_{col}"] = {
            "native_resolutions": dict(sorted(res_by_col[col].items())),
            "unit": sorted(u for u in units if u),
            "missing_30m_slots": int(fill.sum()),
            "pair_zero_filled_slots": int(fill.sum()),
            "interpolated_slots": 0,
            "unfilled_slots": int(s.isna().sum()),
            "repair_rule": "B10 mutually-exclusive pair: absent while the "
            "counterpart series is reported = 0 MW (module docstring rule 1)",
        }
        cols[col] = s

    # Everything else: the standard build.py ladder (<= 2 h interpolation
    # via combine_pieces; day-offset for the two longer-gap series).
    for col, pieces in sorted(pieces_by_col.items()):
        key = f"gen_fr_{col}"
        s = combine_pieces(pieces, res_by_col[col], units, report, key)
        if int(s.isna().sum()):
            s = fill_day_offset(s, report, key)
        if int(s.isna().sum()):
            raise RuntimeError(f"{key}: unfilled gaps after documented repairs")
        cols[col] = s

    df = pd.DataFrame({c: cols[c] for c in sorted(cols)}, index=GRID_30M)
    df.index.name = "utc_start"
    write(df, out, "fr_generation_2024")
    return df


def build_fr_reservoir(raw: Path, out: Path, report: dict, gen: pd.DataFrame) -> None:
    """A72 weekly reservoir filling + inflow proxy (module docstring
    rule 4; mirrors build.build_reservoir for NO2/NO)."""
    is_ack, tss = parse_doc(raw / "reservoir_fr_2024.xml")
    if is_ack:
        raise RuntimeError("reservoir_fr: acknowledgement (no data)")
    rows = []
    unit = None
    for ts in tss:
        unit = ts["unit"]
        for start, _end, res, pts in ts["periods"]:
            if res != "P7D":
                raise RuntimeError(f"reservoir_fr: unexpected res {res}")
            for pos in sorted(pts):
                rows.append(
                    {
                        "week_start_utc": start + pd.Timedelta(days=7 * (pos - 1)),
                        "storage_mwh": pts[pos],
                    }
                )
    report["reservoir_fr"] = {
        "native_resolutions": {"P7D": len(rows)},
        "unit": [unit] if unit else [],
        "inflow_proxy_caveat": "B12-only convention (NO2 precedent); FR "
        "pumped recharge (6.06 TWh) is ignored and is proportionally "
        "larger than NO2's — seasonal-shape indication only (docstring rule 4)",
    }
    df = pd.DataFrame(rows).drop_duplicates("week_start_utc")
    df = df.sort_values("week_start_utc").set_index("week_start_utc")
    res_gen_mwh = gen["hydro_reservoir"] * 0.5
    nxt = list(df.index[1:]) + [None]
    inflow = []
    for wk, wk_next in zip(df.index, nxt):
        if wk_next is None or wk < YEAR_START or wk_next > YEAR_END:
            inflow.append(float("nan"))
            continue
        gen_wk = res_gen_mwh[(res_gen_mwh.index >= wk) & (res_gen_mwh.index < wk_next)]
        delta = df["storage_mwh"].loc[wk_next] - df["storage_mwh"].loc[wk]
        inflow.append(delta + float(gen_wk.sum()))
    df["inflow_proxy_mwh"] = inflow
    write(df, out, "reservoir_fr_2024")


def main() -> None:
    repo = Path(sys.argv[1])
    raw = repo / "data" / "packs" / "entsoe-2024" / "raw"
    out = repo / "data" / "packs" / "entsoe-2024" / "processed"
    report: dict = {}

    gen = build_fr_generation(raw, out, report)
    build_fr_reservoir(raw, out, report, gen)

    (out / "build_report_fr_2024.json").write_text(
        json.dumps(report, indent=2, sort_keys=True)
    )
    annual = (gen.sum() * 0.5 / 1e3).round(1)
    print("FR 2024 annual energies (GWh):")
    for col, v in annual.items():
        print(f"  {col:22s} {v:10.1f}")
    print("written: fr_generation_2024.* reservoir_fr_2024.* build_report_fr_2024.json")


if __name__ == "__main__":
    main()
