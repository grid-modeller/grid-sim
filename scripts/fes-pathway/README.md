# fes-pathway — FES 2025 Holistic Transition reference table

Builds and audits `data/reference/fes-pathway.toml` (schema
`fes-pathway-v1`): GB installed capacity by technology by year,
2024–2050, for the NESO FES 2025 "Holistic Transition" pathway. Input to
the Stage 6 part 2 Q8 pathway runner ("largest survivable loss vs
year").

The TOML is a **committed reference file** (like
`data/reference/prices-2024.toml`); the raw NESO tables it is derived
from are fetched, never committed (`data/packs/fes2025/raw/`, checksums
pinned in `data/packs/fes2025.sha256`).

Stdlib-only (Python ≥ 3.11 for `tomllib` in validate.py); deterministic:
pinned URLs, sha256-verified inputs, pure transformation.

## Usage

```
python scripts/fes-pathway/fetch.py    .   # download + verify raw inputs
python scripts/fes-pathway/build.py    .   # regenerate the TOML
python scripts/fes-pathway/validate.py .   # audit (must pass before commit)
```

`validate.py` reproduces 29 published headline capacities of the FES
2025 report (cited by table and page), cross-checks ES1-derived sums
against the independent FLX1 table, re-derives every number in the
committed TOML, and reconciles the ES1 capacity total to < 0.5 MW per
year. The technology mapping is defined once, in `build.py`
(`FLEET_MAP` / `STORAGE_MAP`), and documented decision-by-decision in
the generated TOML header.

## Licence

All inputs: NESO Open Data Licence
(<https://www.neso.energy/data-portal/neso-open-licence>) — copy,
publish, adapt, redistribute permitted (CC BY 4.0 compatible).
Published outputs derived from this table must carry the attribution
**"Supported by National Energy SO Open Data"**.

## Revisions

NESO versions Data Portal resources in place (the 2025 tables are at
v006, portal last-modified 2025-12-10). A checksum failure in
`fetch.py`/`build.py` means NESO revised a table: diff the values,
record the revision, then re-pin in `fetch.py`, `build.py` and
`data/packs/fes2025.sha256` in one dedicated commit — never silently.
