#!/usr/bin/env python3
"""Build the FR non-GB cross-border flow trace (Stage 5 A2 remediation,
final observed-input package, 2026-07-03).

Why (docs/notes/stage-5-review.md ruling 1, residual anatomy): the 5-zone
scenario closes the FR energy identity with a FLAT +7.537 GW wedge whose
dominant component is FR's net exports to its non-GB neighbours. In
reality those exports collapse at FR demand peaks — exactly where the A2
direction mismatches live. This script builds the OBSERVED series so the
implementer can replace the flat wedge component; it does no wedge
arithmetic itself (work-order non-goal).

Input: the 120 monthly A11 documents fetched 2026-07-03 (fetch.py
FR_BORDERS loop — FR<->BE, FR<->DE-LU, FR<->CH, FR<->IT-North, FR<->ES;
the border discovery probe is documented at fetch.py FR_BORDERS: IT-North
is the only FR<->IT-* border the platform serves). This script fetches
nothing (fetch.py is the only network script).

Output (data/packs/entsoe-2024/processed/):
    fr_external_2024.{parquet,csv}
        17,568-row UTC half-hourly frame: per border b in
        {be, delu, ch, it_north, es} the columns {b}_imp / {b}_exp /
        {b}_net (average MW; net = imp - exp, positive = import to FR),
        plus the headline column
            fr_non_gb_net_export_mw = sum(exp) - sum(imp) = -sum(net)
        — the observed series that replaces the scenario's flat non-GB
        wedge component.
    build_report_fr_flows_2024.json
        per-direction gap/repair record (build.py's honesty rules).

Existing processed files are NOT rebuilt or touched — the committed
entsoe-2024.sha256 entries for them stay byte-valid (the build_gen_agg.py
/ build_fr.py precedent); the three new files are appended to the
manifest in the same change that adds this script.

Assembly rules (build.py's, reused by import, not reimplemented): each
Period normalised to the 30-min UTC grid (PT15M mean / PT60M repeat,
energy-preserving), curveType read per TimeSeries, "no matching data"
acknowledgements for a flow direction-month = zeros (counted), internal
gaps <= 2 h linearly interpolated and counted. Native resolutions
observed (recorded per direction in the build report): be/es PT15M,
delu/ch/it_north PT60M.

IT-NORTH DEC-31 SERIALISATION QUIRK (evidence, established before the
repair): the two FR<->IT-North December documents end 2024 with ONE
PT15M Period per hour covering only the first quarter-hour (timeInterval
start HH:00, end HH:15, a single point; only the 23:00 Period spans its
full hour) — the platform artifact of the Italian market's move to
15-minute MTU on 2025-01-01. The border is PT60M on every other 2024 day
(and the value in each degenerate Period is that hour's value: the
Periods sit on an exact hourly cadence and the 23:00 Period holds one
value across its whole hour in the import document). The strict PT15M
rule (both quarter-hours required) would discard them, leaving 44
unrepairable slots per direction on Dec 31. Documented repair, applied
to it_north ONLY (be/es carry genuine full-coverage PT15M periods —
probe 2026-07-03 found zero degenerate periods on any other border):
a PT15M Period of exactly one quarter-hour is extended to the full hour
it heads, i.e. treated as the PT60M value it is. Every extended Period
is counted and timestamped in the build report. Any OTHER unfilled gap
fails the build — there is no NESO-style GB-side fallback for these
borders, so a future long gap must be diagnosed, not auto-repaired.

Licence (docs/notes/entsoe-stage5-pack-report.md §1, §11): A11 physical
flows (Art. 12.1.g) ARE on the ENTSO-E CC-BY 4.0 free-re-use list
(item 18), and the IFA/Nemo Link carve-outs concern GB-facing
interconnector data, not FR's non-GB borders — these five borders are
clean CC-BY. Attribution: "Source: ENTSO-E Transparency Platform".

Deterministic: no network, no wall-clock, no randomness; pure function of
the raw XML under the pinned venv (requirements.txt).

Usage: python build_fr_flows.py <repo-root>
"""

import json
import sys
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build import (  # noqa: E402  (pinned reuse of the pack's assembly rules)
    GRID_30M,
    MONTHS,
    combine_pieces,
    parse_doc,
    period_to_30m,
    write,
)
from fetch import FR_BORDERS  # noqa: E402  (single source for the border set)


def assemble_direction(raw: Path, b: str, d: str, report: dict) -> pd.Series:
    """Monthly A11 documents for one border direction -> 30-min series.

    Mirrors build.assemble_files (ack_is_zero=True for flow months) plus
    the it_north degenerate-Period rule (module docstring)."""
    key = f"flows_fr_{b}_{d}"
    pieces: list[pd.Series] = []
    resolutions: dict[str, int] = {}
    units: set = set()
    ack_months: list[str] = []
    extended: list[str] = []
    for ym in MONTHS:
        is_ack, tss = parse_doc(raw / f"flows_fr_{b}_{d}_{ym}.xml")
        if is_ack:
            ack_months.append(ym)
            continue
        for ts in tss:
            units.add(ts["unit"])
            for start, end, res, pts in ts["periods"]:
                if (
                    b == "it_north"
                    and res == "PT15M"
                    and end - start == pd.Timedelta(minutes=15)
                ):
                    # Dec-31 quirk: a one-quarter-hour Period carries the
                    # hourly value (module docstring); extend to the hour.
                    end = start + pd.Timedelta(hours=1)
                    res = "PT60M"
                    extended.append(str(start))
                resolutions[res] = resolutions.get(res, 0) + 1
                pieces.append(period_to_30m(start, end, res, pts, ts["curve"]))
    s = combine_pieces(
        pieces,
        resolutions,
        units,
        report,
        key,
        zero_fill_months=ack_months if ack_months else None,
    )
    if extended:
        report[key]["degenerate_hour_periods_extended"] = len(extended)
        report[key]["degenerate_hour_period_starts"] = extended
    if int(s.isna().sum()):
        raise RuntimeError(
            f"{key}: {int(s.isna().sum())} unfilled slots after the <= 2 h "
            "rule — no documented repair exists for FR non-GB borders "
            "beyond the it_north Dec-31 rule (module docstring); diagnose "
            "before repairing"
        )
    return s


def main() -> None:
    repo = Path(sys.argv[1])
    raw = repo / "data" / "packs" / "entsoe-2024" / "raw"
    out = repo / "data" / "packs" / "entsoe-2024" / "processed"
    report: dict = {}

    cols: dict[str, pd.Series] = {}
    for b in FR_BORDERS:
        imp = assemble_direction(raw, b, "imp", report)
        exp = assemble_direction(raw, b, "exp", report)
        cols[f"{b}_imp"] = imp
        cols[f"{b}_exp"] = exp
        cols[f"{b}_net"] = imp - exp

    # Headline: FR's net exports to its non-GB neighbours (positive when
    # FR is a net exporter to them) — the flat-wedge replacement series.
    cols["fr_non_gb_net_export_mw"] = sum(
        cols[f"{b}_exp"] for b in FR_BORDERS
    ) - sum(cols[f"{b}_imp"] for b in FR_BORDERS)

    df = pd.DataFrame(cols, index=GRID_30M)
    df.index.name = "utc_start"
    write(df, out, "fr_external_2024")

    (out / "build_report_fr_flows_2024.json").write_text(
        json.dumps(report, indent=2, sort_keys=True)
    )

    print("FR non-GB borders, 2024 annual (TWh, + = import to FR):")
    for b in FR_BORDERS:
        imp_twh = df[f"{b}_imp"].sum() * 0.5 / 1e6
        exp_twh = df[f"{b}_exp"].sum() * 0.5 / 1e6
        net_twh = df[f"{b}_net"].sum() * 0.5 / 1e6
        print(f"  {b:9s} imp {imp_twh:6.2f}  exp {exp_twh:6.2f}  net {net_twh:+7.2f}")
    hl = df["fr_non_gb_net_export_mw"]
    print(f"  non-GB net export total: {hl.sum() * 0.5 / 1e6:+.2f} TWh")
    peak = hl[(hl.index.hour >= 17) & (hl.index.hour < 21)]
    print(f"  mean MW all-hours {hl.mean():,.0f}; 17-21 UTC {peak.mean():,.0f}")
    print("written: fr_external_2024.* build_report_fr_flows_2024.json")


if __name__ == "__main__":
    main()
