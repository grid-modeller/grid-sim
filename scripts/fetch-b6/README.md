# fetch-b6 — B6 two-zone evidence pack (Scotland / rest-of-GB)

Data package for the GB two-zone / B6-boundary study (post-Stage-5 work
package, ratified 2026-07-03; promoted VITAL 2026-07-04): the observed
B6 (SCOTEX) boundary series, per-boundary constraint costs, and the
fleet/demand-split evidence. Full source-by-source licence diligence,
the capacity split table, and the recommended scenario conventions:
`docs/notes/b6-two-zone-data-report.md`.

Run order (era5 venv, `~/.local/share/grid-sim/era5-venv`, Python
3.13.11, versions pinned in `requirements.txt`):

```
python fetch.py <repo-root>   # network -> data/packs/b6/raw/ (12 files)
python build.py <repo-root>   # raw -> data/packs/b6/processed/ + report
```

Manifest: `data/packs/b6.sha256` (raw + processed, 21 entries; paths
relative to `data/packs/`). Data is fetched-and-built, never committed.

## Sources, retrieval dates, licences (verified 2026-07-04)

| Source | Files | Licence |
|---|---|---|
| NESO Data Portal: Day Ahead Constraint Flows and Limits | half-hourly B6 (SCOTEX) day-ahead limit + unconstrained-flow forecast, rolling ~3.5y window (2023-01-01 onward at this fetch; earlier data available from the NESO OpenData team on request) | NESO Open Data Licence (OGL-v3-based; commercial re-use permitted; attribution "Supported by National Energy SO Open Data") |
| NESO Data Portal: Thermal Constraint Costs | daily outturn cost per boundary group (SCOTEX/B6, SSE-SP/B4, SSHARN/B7, ESTEX, SEIMP, SWALEX), FY 2021-22 – 2026-27 | NESO Open Data Licence |
| NESO Data Portal: Constraint Breakdown Costs and Volume | daily GB-wide constraint cost + volume by category, FY 2023-24, 2024-25 | NESO Open Data Licence |
| NESO Data Portal: Interconnector Register | connection sites (zone-assignment evidence) | NESO Open Data Licence |
| GOV.UK DESNZ: REPD quarterly extract (April 2026) | site-level renewables ≥150 kW; end-2024 fleet recovered by Operational-date filter | Open Government Licence v3.0 |
| GOV.UK DESNZ: Regional Renewable Statistics 2003–2024 | all-size installed capacity by country × technology (MW2024 sheet) | Open Government Licence v3.0 |

Attribution to carry on anything published from this pack:
"Supported by National Energy SO Open Data" (NESO datasets) and
"Contains public sector information licensed under the Open Government
Licence v3.0" (DESNZ datasets).

## Key semantics (documented in full in build.py)

- **B6 "flow" semantics — NESO's wording vs our interpretation, kept
  distinct (review condition 3):** NESO documents the flow as "the
  forecast position after Day Ahead energy scheduling", a "power flow
  forecast … based on the next day's wind forecast, generation
  dispatch and demand forecast … modelled using power system
  software" — never "unconstrained". This package INTERPRETS it as
  the pre-constraint-action (unconstrained) boundary flow, supported
  by the data (flow exceeds limit in 23.6% of 2024 periods —
  impossible for a constrained/settled series) and ruled sound at
  review; it is an interpretation, and NOT an observed metered flow
  series. Under it, the series anchors a copper-plate-then-constrain
  model's pre-constraint B6 flow.
- Positive flow = Scotland exporting (north→south).
- Clock changes: raw labels are local wall-clock on a fixed 48-row
  daily grid; the build drops the spring phantom rows (counted),
  disambiguates the autumn repeated hour (first=BST, second=GMT), and
  leaves raw gaps missing (never filled). Gap counts per year:
  `processed/b6_report.json`.
- The per-boundary cost set covers the six "significant" boundaries
  only — calendar-2024: £579.5m across the six vs £1,482.5m GB-wide
  thermal (constraint-breakdown dataset): the six-boundary set is ~39%
  of thermal constraint costs, stated so nobody totals it as if it
  were complete.

## Re-fetch behaviour

NESO refreshes the day-ahead file daily (rolling window) and re-dates
the interconnector-register filename; gov.uk re-issues REPD quarterly.
A re-fetch after any of those produces different raw bytes and fails
`b6.sha256` — deliberate: source drift must be visible, never silent.
The manifest pins the 2026-07-04 retrieval.
