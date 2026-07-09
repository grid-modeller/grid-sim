# 05 — Validation, Data, and Reproducibility

Validation is a first-class deliverable, not an afterthought. The 2024
validation pack must be assembled **before Stage 1 coding begins**, because
Stage 1's acceptance test is defined against it.

## Data sources

| Data | Source | Licence status | Delivery |
|---|---|---|---|
| Historic GB generation by fuel (half-hourly) | Elexon BMRS / NESO Data Portal | Open | Fetch |
| Historic GB demand (half-hourly) | NESO Data Portal | Open | Fetch |
| Historic interconnector flows | Elexon / NESO | Open | Fetch |
| Historic GB system inertia (outturn + market-provided, half-hourly, calendar 2024) | NESO Data Portal, dataset `8f3cd0ce-6636-469e-b582-55eadfeaa1d9` — resources `5bd6ec4d-a2df-4c94-9b27-fdf8cf04d7dd` (2023-24, supplies Jan–Mar) and `7a12d0bd-448d-42a9-b333-4a32761dbad4` (2024-25, supplies Apr–Dec) | NESO Open Data Licence — attribution **"Supported by National Energy SO Open Data"** required | Fetch |
| Weather-derived capacity factors (wind on/offshore, solar) 1985–2024 | ERA5, direct derivation (atlite-style). D1 resolved 2026-07-02: renewables.ninja is CC BY-NC (and MERRA-2/CM-SAF-based, not ERA5) — internal cross-check only, never shipped. See `docs/notes/d1-renewables-ninja-licence.md` | ERA5 is CC-BY 4.0 — redistribution incl. a hosted pack permitted, Copernicus attribution required | Fetch + build |
| Temperature traces per zone | ERA5 | As above | Fetch + build |
| Fleet capacities, availability | DUKES / NESO FES | Open | Committed reference file |
| Fuel and carbon prices | Published historic series | Open | Committed reference file |
| Continental demand/fleet (Stage 5) | ENTSO-E Transparency Platform | Open (registration) | Fetch |

**Decision (per conversation):** the tool **fetches and builds** its data
pack (`grid-cli fetch-data`) rather than shipping it. Consequences:
- Checksums of built data recorded in the repo; a run records the data-pack
  checksum alongside engine hash and scenario hash.
- ERA5-derived outputs (data packs, published charts using them) must carry
  the Copernicus attribution: "Contains modified Copernicus Climate Change
  Service information [Year]" plus the standard no-responsibility
  disclaimer (D1 licence note).
- **Manifest semantics (2026-07-03, on the `grid-cli fetch-data` port):**
  the invariant a pack manifest protects is **value identity** — row
  count, UTC index, bit-exact cell values. CSV bytes are additionally
  reproducible across builders; **Parquet bytes are writer-specific**
  (arrow/compression internals), so Parquet hashes pin a *builder
  generation*. Changing builders (Python→Rust, or a writer-library
  upgrade) is handled by proving value identity with the comparison
  harness (`fetch-data --compare-with`) and then re-pinning the Parquet
  hashes in one dedicated, recorded commit — never as an incidental
  rebuild. Pack files carry no embedded metadata block (the docs/06
  header rule applies to *run* outputs); pack files are identified by
  manifest checksum alone.
- **Bounded exception to "the tool fetches and builds its data pack":**
  the ERA5 capacity-factor pipeline (`scripts/era5-cf/`) remains Python
  by design and is not ported into `grid-cli fetch-data`. It consumes
  Zarr/icechunk stores through a pinned scientific stack
  (xarray/zarr/fsspec) with no mature Rust equivalent; reimplementation
  would be disproportionate and would risk the bit-reproducibility of
  already-validated CF traces. The exception is bounded: the pipeline is
  deterministic and version-pinned (requirements.txt; resumable fetch;
  checksummed cutouts and outputs; byte-identical re-derivation
  demonstrated), and its manifests are committed like any other pack
  input. Third parties rebuild the Elexon/NESO/ONS traces with
  `grid-cli fetch-data` alone and the ERA5 traces with the pinned Python
  pipeline. The remaining price-pack traces (Elexon MID, imbalance) are
  a later fetch-data increment.
- The WASM/web phase needs a hosted pre-built pack (tens of MB for 40 years
  half-hourly multi-tech) — a phase-two problem, noted here so the pack
  format is web-friendly (Parquet, per-trace files, lazily loadable).

## The 2024 validation pack

Contents:
- Actual 2024 fleet definition (reference scenario file).
- Actual 2024 half-hourly demand, generation by fuel, interconnector flows.
- Expected-output fixtures: annual and monthly aggregates with tolerances.

Filling in `TBD-DATA` tolerances: run the data assembly, quantify the
irreducible discrepancies (station-level outages we don't model, embedded
generation treatment, pumped storage round trips), and set tolerances just
outside them, with each tolerance justified in a comment. Tolerances are
evidence-based, not aspirational.

Suggested starting frame (to be confirmed against data):
- Annual gas burn: ±5%
- Net annual imports (Stage 5, modelled): ±15%
- Monthly generation mix correlation: ≥ 0.95
- % periods gas-marginal: ±3 points

## Reproducibility rules (restating ADR-5, operationally)

1. `results = f(scenario, data-pack checksum, engine git hash)` — nothing else.
2. Every output file header embeds all three identifiers.
3. Regression suite: every number published (book, Substack, LinkedIn) gets a
   pinned regression test — scenario file committed, expected output
   committed, CI fails if it drifts.
4. Schema migrations documented in `03-domain-model.md`; old scenario files
   must either parse or fail with a clear migration message — never silently
   reinterpret.

## Known model boundaries (stated, not hidden)

To be published alongside the tool — pre-empting critics by listing what the
model does not do:
- No intra-GB network model (constraint-cost approximation only, ADR-12).
- No EMT-level stability simulation (aggregate swing equation).
- Rule-based dispatch has no foresight (mitigated by dual-policy reporting).
- Embedded/behind-the-meter generation: total-generation convention (D3,
  resolved — `docs/notes/d3-embedded-convention.md`): underlying demand =
  ND + NESO half-hourly embedded estimates, embedded capacity modelled
  explicitly. Solar validation is consequently partly circular on the NESO
  estimate.
- Interconnector counterparties aggregated to 3–4 zones.
