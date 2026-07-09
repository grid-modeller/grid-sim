# Data provenance and licensing

The engine's data packs are **fetched and built, never committed** — this
repository carries only `data/packs/*.sha256` checksum manifests and the small
cited reference files under `data/reference/`. Anyone can rebuild the packs
from the primary sources with the committed fetch scripts (`scripts/`,
`grid-cli fetch-data`); the manifests pin what a correct build produces.
Authoritative source-by-source detail lives in `docs/05-validation.md`.

## Licence

The **derived data packs published by this project** (and the committed
reference files in `data/reference/`, except where a row below says
otherwise) are licensed under **Creative Commons Attribution 4.0
International (CC-BY-4.0)** — see `data/LICENSE`. The engine code is
licensed separately (MIT OR Apache-2.0; see the repository root).

Redistribution of a derived pack is only offered where every upstream source
permits it. Sources are kept physically separable (one pack per source
family, one manifest per pack) so no pack inherits a restriction from an
input it does not use.

## Sources, attribution strings, redistribution status

| Pack(s) | Source | Upstream licence | Redistributable in our packs? | Required attribution |
|---|---|---|---|---|
| `cf`, `cf-gb2`, `cf-gb3`, `era5`, `weather-gb-t2m-pop` (weather-derived capacity factors and temperature traces, 1985–2024) | ERA5, Copernicus Climate Change Service (C3S) | CC-BY 4.0 | Yes | "Contains modified Copernicus Climate Change Service information [year]. Neither the European Commission nor ECMWF is responsible for any use that may be made of the Copernicus information or data it contains." |
| `2024` (demand, generation by fuel, interconnector flows), `demand-tiled`, `response-holdings`, inertia outturn | NESO Data Portal | NESO Open Data Licence (OGL-compatible) | Yes | "Supported by National Energy SO Open Data" |
| `2024` price traces, FUELHH | Elexon BMRS / Insights | Elexon open terms, attribution | Yes | "Contains BMRS data © Elexon Limited copyright and database right [year]" |
| `fes2025`, `cb7`, `etys`, `costs-evidence` | NESO FES 2025 / CCC CB7 / NESO ETYS / cited public reports | Open (per-file citations in the pack build scripts and `data/reference/`) | Yes | Per-source citation as recorded in the manifest/build script |
| `entsoe-2024`, `cf-eu`, `era5-eu` (continental zones) | ENTSO-E Transparency Platform (load/generation/capacity); ERA5 for `era5-eu` | ENTSO-E TP terms: **use permitted, raw redistribution not permitted** | **No — fetch yourself** with a free ENTSO-E API token and the committed `scripts/fetch-entsoe` pipeline | n/a (not redistributed) |

Notes:

- GB↔FR / GB↔BE interconnector flows are sourced from Elexon/NESO, never
  ENTSO-E (ENTSO-E excludes IFA and Nemo Link even from its open
  physical-flows list).
- renewables.ninja is CC BY-NC and was used only as an internal
  cross-check during validation; nothing derived from it is committed or
  shipped (see `docs/notes/d1-renewables-ninja-licence.md`).
- Scenarios under `scenarios/` embed small *derived aggregates* (zone
  capacities, cost recipes) with per-value citations in the file; the two
  continental scenarios cite ENTSO-E-derived aggregates, which the ENTSO-E
  terms permit for any purpose with attribution — reproduction of their
  underlying traces is bring-your-own-token.
