# fetch-cbs — CBS (Statistics Netherlands) 2024 NL anchor pack

Fetches and builds the CBS StatLine evidence that anchors the NL
onshore-wind and NL solar CF recalibration (docs/notes/
eu-cf-derivation-report.md, 2026-07-03 CBS recalibration addendum;
trigger adjudicated in docs/notes/stage-5-review.md addendum ruling 3;
mandate in docs/notes/eu-cf-review.md ruling 1 + defect D3).

## Sources, licence, status

- **82610NED** "Hernieuwbare elektriciteit; productie en vermogen":
  production (gross/net/normalised, mln kWh) and end-of-year installed
  electrical capacity (MW) per renewable source.
- **85005NED** "Zonnestroom; vermogen en vermogensklasse": solar PV
  panel capacity (kWp, DC), inverter capacity (kW, AC) and production
  per sector — documents the sector split (dwellings + economic
  activities) that sums to the 82610NED national solar total, and the
  DC/AC capacity conventions.
- API: CBS OData v1, `https://datasets.cbs.nl/odata/v1/CBS/<table>/`.
  No credentials. Retrieved 2026-07-03.
- **Licence: CC BY 4.0** — verified 2026-07-03 at
  https://www.cbs.nl/en-gb/about-us/website/copyright ("the content of
  this website is subject to Creative Commons Attribution (CC BY 4.0)").
  Attribution mandatory and carried: "Source: CBS (Statistics
  Netherlands), StatLine tables 82610NED and 85005NED".
- **2024 status: "NaderVoorlopig"** (revised provisional) — recorded on
  every processed row. CBS revises provisional figures in place; a
  re-fetch after a revision fails the committed manifest by design
  (anchor drift must be visible, never silent).

## Run order

Venv: `~/.local/share/grid-sim/entsoe-venv` (Python 3.13.11, pinned in
`requirements.txt` — same versions as fetch-entsoe; Parquet bytes and
the committed manifest are conditional on them).

```
python fetch.py <repo-root>   # network: CBS OData -> raw JSON (resumable)
python build.py <repo-root>   # raw -> processed anchors table (no network)
```

Outputs land in `data/packs/cbs-2024/` (git-ignored; only
`data/packs/cbs-2024.sha256` — the processed files, same convention as
the other manifests — is committed):

```
raw/        10 OData responses (observations + code lists + table
            properties + 2024 period-status rows, both tables)
processed/  cbs_2024_nl_anchors.{parquet,csv}   12 rows: table_id,
            series, measure_code, measure, value, unit, period, status
            cbs_build_report_2024.json          row count, status,
            licence, attribution, anchor values echoed
```

Build determinism: pure function of the raw JSON; double-run
byte-identical (verified 2026-07-03). Internal consistency asserted:
the 85005NED sector split sums exactly to the national solar total and
both tables agree on it (21,822 GWh).

## What consumes this

`scripts/era5-cf/derive_cf_eu.py` (`load_cbs_anchors`, docstring
deviation 3a) and `scripts/era5-cf/validate_cf_eu.py` (NL
anchor-reproduction checks). Anchor pairing (documented there and in
the derivation report): NL onshore = net generation 17,657 GWh over
6,955 MW (identical to A68); NL solar = 21,822 GWh over 27,979.732 MWp
DC panel capacity (numerically A68's 27,980 MW — A68's figure IS the
CBS DC panel capacity; the AC-side figures are recorded but not
paired).
