# fetch-entsoe — ENTSO-E Stage 5 pack (2024)

**Provisional** Python scripts (fetch-2024 / fetch-prices pattern)
assembling the Stage 5 external-zone data: observed 2024 GB cross-border
physical flows, neighbour-zone actual load and installed capacity, and
Norwegian hydro evidence (NO2 generation per type, weekly reservoir
filling). Evidence summary: `docs/notes/entsoe-stage5-pack-report.md`.

Run order (Python 3.13.11, versions pinned in `requirements.txt`; venv at
`~/.local/share/grid-sim/entsoe-venv`, rebuilt from `requirements.txt` —
checksum reproducibility is conditional on those exact versions, per the
era5-cf precedent):

```
python fetch.py         <repo-root>   # network: ENTSO-E TP API -> raw XML
python build.py         <repo-root>   # raw XML -> UTC half-hourly traces
python build_fr.py      <repo-root>   # FR per-type traces + FR reservoir
                                      # (Stage 5 A2 remediation; added
                                      # 2026-07-03, see below)
python build_fr_flows.py <repo-root>  # FR non-GB cross-border flows
                                      # (Stage 5 A2 remediation, observed
                                      # wedge; added 2026-07-03, see below)
python validate.py      <repo-root>   # exit non-zero on any integrity failure
python analyze.py       <repo-root>   # Stage 5 evidence -> analysis_entsoe_2024.json
python build_gen_agg.py <repo-root>   # A75 per-zone 2024 energy aggregation
                                      # (EU CF calibration anchors; added
                                      # 2026-07-03, see below)
```

Outputs land in `data/packs/entsoe-2024/` (git-ignored; only
`data/packs/entsoe-2024.sha256` — processed files, same convention as the
other manifests — is committed). `fetch.py` is the only script that
touches the network; it is resumable (one file per request, atomic
writes, existing files skipped) and throttles to ~170 req/min with 60 s
backoff on HTTP 429 (documented platform cap: 400 req/min).

## Credentials

The API token lives at `~/.local/share/grid-sim/entsoe-token` (mode 600)
or `$ENTSOE_TOKEN`. It is never hardcoded, never committed, never
printed. Free registration at transparency.entsoe.eu; token issued per
account (Registered Data User, M2M interface per clause 3.3 of the Terms
of Use).

## Source, licence (accessed 2026-07-03)

| Item | Value |
|---|---|
| API | `https://web-api.tp.entsoe.eu/api` (ENTSO-E Transparency Platform RESTful API) |
| Terms of Use | GTC approved by the ENTSO-E Market Committee 29 Mar 2023, applicable from 1 Nov 2023 (Legal Terms and Conditions article, TP help centre) |
| Re-use | Clause 3.1: TP data may be used "for any purpose whatsoever" subject to good faith, **mentioning the ENTSO-E Transparency Platform as the source of publication**, no implied endorsement, and no prejudice to Primary-Owner copyright |
| Open Data (CC-BY 4.0) | Clause 2.5 + "List of Data Available for Free Re-Use" (last modified 18 Oct 2023): listed items may be freely copied/redistributed/adapted for any purpose with attribution. Physical flows (Art. 12.1.g) are listed; **actual load (6.1.a), installed capacity (14.1.a), generation per type (16.1) and reservoir filling are NOT on the list** |
| Exceptions | All data provided by **IFA** ("Interconnexion France-Angleterre") and **Nemo Link** is excluded from the CC-BY list; BritNed is excluded only for balancing items #24, 26, 29, 30, 32–35 (physical flows explicitly opened 28 Oct 2019). The carve-outs concern GB-facing interconnector data; FR's non-GB borders are unaffected |
| Consequence | Fetch-and-build (this repo's architecture) is covered by clause 3.1 for everything fetched here. Raw/derived **redistribution** beyond aggregate statistics is only clean for CC-BY-listed items; the pack is git-ignored and never hosted, so nothing here relies on redistribution rights. Publication-grade per-link numbers for GB↔FR and GB↔BE use the Elexon/NESO 2024 pack (fully open licences); ENTSO-E is the neighbour-side cross-check |
| Attribution | "Source: ENTSO-E Transparency Platform" on anything re-used |

Full verdict with quotes: `docs/notes/entsoe-stage5-pack-report.md` §1.

## What is fetched (all 2024, all UTC)

| Data | documentType | Zones | Native resolution (2024) |
|---|---|---|---|
| Cross-border physical flows, per direction | A11 | GB borders: FR, BE, NL, NO2, DK1, IE(SEM); extended 2026-07-03 to FR non-GB borders: BE, DE-LU, CH, IT-North, ES (A2 remediation, see below) | fr/dk1/ie PT60M; be/nl/no2 PT15M; FR borders: be/es PT15M, delu/ch/it_north PT60M |
| Actual total load | A65/A16 | FR, BE, NL, DE-LU, NO2, DK1, IE(SEM) | fr/no2/dk1 PT60M; be/nl/delu PT15M; ie PT30M |
| Installed capacity per production type | A68/A33 | same 7 zones | P1Y |
| Actual generation per production type | A75/A16 | NO2 + NO aggregate; extended 2026-07-03 to fr/be/nl/delu/dk1/ie (EU CF calibration anchors, see below) | PT60M (fr/dk1/ie also PT60M; be/nl/delu PT15M; ie PT30M) |
| Weekly reservoir filling | A72/A16 | NO2 + NO aggregate; extended 2026-07-03 to FR (A2 remediation, see below) | P7D |

## Data semantics discovered during assembly (do not lose these)

- **A11 flows are per bidding-zone border, not per asset.** GB↔FR is
  IFA + IFA2 + ElecLink combined; GB↔IE(SEM) is EWIC + Moyle + Greenlink
  combined (SEM spans Ireland and Northern Ireland). The per-asset
  virtual zones GB(IFA)/GB(IFA2)/GB(ElecLink) return no A11 data. There
  is no GB↔DE-LU border (Viking lands in DK1). Per-link GB-side series
  remain the NESO/Elexon ones in `data/packs/2024/`.
- **The only FR↔IT-* border is FR↔IT-North** (10Y1001A1001A73I; probe
  2026-07-03): the virtual IT_North_FR zone (10Y1001A1001A81J) returns
  "no matching data" in both directions, and the IT country aggregate
  (10YIT-GRTN-----B) returns data identical to IT-North (the same single
  border serialised twice).
- **A11 `in_Domain` is the receiving zone**; each direction is a separate
  unsigned series. Net (+ = import to GB) = imp − exp.
- **Every 2024 TimeSeries declares curveType A03** (variable-sized
  blocks): a missing position repeats the previous value to the end of
  the Period — this holds for A65/A75/A72 too, not just A11
  (review-verified across all 261 documents; hold-forward was exercised
  on load and generation series). The parser reads the declared
  curveType per TimeSeries and would treat A01 as fixed blocks (missing
  position = genuine gap) if it ever appeared; never assume A01.
- **Quantities are average MW over the market time unit** (unit `MAW`);
  A72 reservoir filling is stored energy in MWh.
- Resampling to the pack's 30-min grid: PT15M → mean of the two slots
  (strict NaN propagation); PT60M → repeat into both half-hours; both
  energy-preserving. Native resolutions recorded per series in
  `build_report_entsoe_2024.json`.
- "No matching data" acknowledgements for a flow direction-month mean no
  flow was reported in that direction → zeros (counted in the build
  report); an ACK anywhere else fails validation.
- Internal gaps ≤ 2 h are linearly interpolated and counted; longer gaps
  are left NaN unless repaired by a documented rule (IE(SEM) bullet
  below); anything unrepaired fails validation.
- The API answers in UTC; the March/October clock changes require no
  special handling (unlike the NESO settlement-day sources).
- **Native resolutions change WITHIN a series**: NO2 flows are PT15M for
  9 months and PT60M for 4 documents; FR load has one PT15M document.
  Each document Period is normalised to 30-min individually; the exact
  mixture per series is in `build_report_entsoe_2024.json`.
- **FR↔IT-North Dec-31 degenerate Periods** (build_fr_flows.py rule): the
  December documents end 2024 with one PT15M Period per hour covering
  only the first quarter-hour (single point) — the artifact of Italy's
  move to 15-minute MTU on 2025-01-01. The border is PT60M on every
  other day; each such Period carries that hour's value and is extended
  to its full hour (counted and timestamped; it_north only — be/es carry
  genuine full-coverage PT15M periods, zero degenerate periods on any
  other border).
- **IE(SEM) series are gap-prone** (EirGrid publication outages): 718/592
  missing 30-min slots (flows/load), ~283 runs, mostly single slots,
  longest 14 h (flows) / 22 h (load). Repairs, all counted and
  timestamped in the build report: gaps <= 2 h linearly interpolated;
  longer flow gaps filled from the NESO GB-side per-link actuals
  (data/packs/2024 — same physical quantity, GB end); longer load gaps
  filled with the mean of the same half-hour one day earlier/later.
- **ENTSO-E border flows meter at the sending end**: annual net imports
  run 0.9-3 % above the NESO GB-side values per border (ENTSO-E total
  +34.34 TWh vs NESO +33.30 TWh) — consistent with HVDC losses landing
  between the two metering points. Reconciliation table in the evidence
  note; validate.py enforces a ±0.5 TWh per-border wedge ceiling.
- **A11 physical flows are not RTE's commercial exchanges**: FR total
  physical net exports 2024 are 82.46 TWh (this pack, incl. the GB
  border; energy-charts.info independently reproduces every border to
  0.01 TWh from the same ENTSO-E data) vs RTE's published 89 TWh, which
  is the COMMERCIAL (scheduled) balance — the 6.5 TWh loop-flow wedge
  sits on the meshed AC borders (DC/radial borders match RTE to
  0.2 TWh). Do not mix the two conventions in one energy identity.
- **A75 for NO zones carries generation only** — Statnett publishes no
  pumped-storage consumption TimeSeries (no `_con` columns result).
- **FR B10 pumped storage is a mutually exclusive TimeSeries PAIR** (do
  not lose this): RTE reports the generation series only while the fleet
  is net generating and the consumption series only while it is net
  pumping — in 2024 every gen-missing slot has the con series reporting,
  every con-missing slot has gen reporting, zero slots missing on both
  sides. The absent side is 0 MW by construction; the correct repair is
  pair-fill with zero (build_fr.py rule 1). Generic interpolation/
  day-offset repairs invent generation during pumping windows — the
  aggregation_gen_2024 FR hydro_pumped figure (9,456 GWh) overstates the
  pair-rule energy (6,930 GWh) by ~2.5 TWh for exactly this reason;
  anything consuming FR B10 ENERGY must use fr_generation_2024, the
  aggregation table's B10 row is coverage context only.
- **A72 FR weeks are French local weeks** (Monday 00:00 Europe/Paris =
  Sunday 23:00 UTC at the winter anchor); unit MWh stored energy, same
  53-week/A03/P7D structure as the Norwegian series.
- **A72 reservoir weeks are Norwegian local weeks** (Monday 00:00
  Europe/Oslo = Sunday 23:00 UTC); `week_start_utc` keeps the UTC
  instant. Unit is MWh stored energy.
- The NO-aggregate generation file exists for the D5 zone-granularity
  note only; its patchy solar series is zero-filled where absent
  (recorded). The NO2 file needed no such repair.

## A75 extension — EU CF calibration anchors (2026-07-03)

Added for the Stage 5 external-zone CF derivation
(`scripts/era5-cf/derive_cf_eu.py`): `fetch.py`'s A75 loop now also
fetches actual generation per production type for **fr, be, nl, delu,
dk1, ie** (72 more monthly documents, fetched 2026-07-03; the original
no2/no files are skipped by the resume logic and are untouched — all
existing processed outputs and their manifest entries are byte-unchanged).
`build_gen_agg.py` assembles them into

    data/packs/entsoe-2024/processed/aggregation_gen_2024.{parquet,csv}
    data/packs/entsoe-2024/processed/aggregation_gen_report_2024.json

annual + monthly GWh per zone per PSR series (generation TimeSeries only;
consumption series excluded — sparse and irrelevant to the anchor). The
three anchor PSRs (B16 solar, B18 offshore, B19 onshore) must be
gap-free after the documented repairs or the build fails; other series
keep residual gaps, sum over reported slots only, and carry an
`unfilled_slots` column. The three files are appended to
`entsoe-2024.sha256` (31 entries).

Data-quality findings recorded there (full diagnosis in
`docs/notes/eu-cf-derivation-report.md`):
- **IE-SEM solar (B16) starts 2024-11-13** — the platform has no
  full-year 2024 IE solar series; the anchor is excluded (an anchor
  series absent for whole months is never zero-filled).
- **NL onshore wind and NL solar are drastically under-reported**
  (7,654 GWh vs ~15 TWh real; 487 GWh vs ~21 TWh real) — distributed
  generation missing from the platform's NL feed.
- **FR offshore (1,003 MW) and IE onshore (3,000 MW) A68 capacities are
  stale** against their own A75 generation (implied CFs 0.449 / 0.507,
  non-physical).

Licence: A75 generation and A68 capacity are NOT on the CC-BY free
re-use list; this use is the clause-3.1 case (internal calibration
anchor, fetch-and-build, git-ignored, never redistributed). Attribution
on anything derived: "Source: ENTSO-E Transparency Platform".

## FR per-type traces — Stage 5 A2 remediation (2026-07-03)

Added per docs/notes/stage-5-review.md ruling 1 (the A2 red: the 5-zone
scenario's flat FR hydro availability overstates FR peak scarcity; the
fix is observed per-type traces so FR reservoir(+pumped) hydro can be
wired through schema-v4 `energy_budget` exactly as NO2 is).
`build_fr.py` builds, from the raw A75 FR XML already on disk (the
2026-07-03 calibration fetch — nothing re-fetched) plus ONE newly
fetched document (A72 FR weekly reservoir filling, which the platform
does publish for FR; fetch.py's A72 loop now includes fr):

    data/packs/entsoe-2024/processed/fr_generation_2024.{parquet,csv}
        17,568 half-hourly UTC rows x 13 columns: all 12 FR production
        types RTE publishes for 2024 + hydro_pumped_con (the pumping
        trace, for the wedge's peak-shaped pumping component)
    data/packs/entsoe-2024/processed/reservoir_fr_2024.{parquet,csv}
        53 weekly rows, storage_mwh + inflow_proxy_mwh (B12-only proxy
        convention as NO2; FR caveat in build_fr.py docstring rule 4)
    data/packs/entsoe-2024/processed/build_report_fr_2024.json

The five files are appended to `entsoe-2024.sha256` (36 entries; the
existing 31 verified byte-unchanged before and after). Existing
processed outputs are not rebuilt (build_gen_agg.py precedent).
Evidence addendum: docs/notes/entsoe-stage5-pack-report.md §10.

Licence: A75 generation and A72 reservoir filling are NOT on the CC-BY
free-re-use list; same GTC clause-3.1 internal-use posture as above
(pack git-ignored, only checksums committed, attribution "Source:
ENTSO-E Transparency Platform" on anything derived).

## FR non-GB flows — Stage 5 A2 remediation, observed wedge (2026-07-03)

Added per docs/notes/stage-5-review.md ruling 1, final observed-input
package: the 5-zone scenario's flat +7.537 GW FR identity wedge is
dominated by FR's non-GB net exports, which in reality collapse at FR
demand peaks — where the A2 mismatches live. `fetch.py`'s A11 loop now
also fetches FR's five non-GB borders (FR↔BE, FR↔DE-LU, FR↔CH,
FR↔IT-North, FR↔ES; border discovery documented at fetch.py
`FR_BORDERS`), per direction per month — 120 new documents, fetched
2026-07-03, zero ACKs; existing files skipped by the resume logic.
`build_fr_flows.py` assembles them into

    data/packs/entsoe-2024/processed/fr_external_2024.{parquet,csv}
        17,568 half-hourly UTC rows x 16 columns: per border
        {b}_imp/{b}_exp/{b}_net (average MW, net + = import to FR) plus
        fr_non_gb_net_export_mw = Σexp − Σimp — the observed series that
        replaces the scenario's flat non-GB wedge component
    data/packs/entsoe-2024/processed/build_report_fr_flows_2024.json

The three files are appended to `entsoe-2024.sha256` (39 entries; the
existing 36 verified byte-unchanged before and after). Existing
processed outputs are not rebuilt (build_gen_agg.py precedent).
Evidence addendum with the annual table, the peak-collapse statistics
and the external cross-checks (energy-charts per-border 0.01 TWh
agreement; RTE commercial 89 TWh vs physical 82.5 TWh):
docs/notes/entsoe-stage5-pack-report.md §11.

Licence: A11 physical flows (Art. 12.1.g) ARE on the CC-BY 4.0
free-re-use list (item 18); the IFA/Nemo Link carve-outs concern
GB-facing interconnector data and do not touch FR's non-GB borders —
these five borders are clean CC-BY with attribution "Source: ENTSO-E
Transparency Platform".
