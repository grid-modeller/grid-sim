# B6 / ETYS artefact addendum — pinning the 6.7 GW SCOTEX capability

Data engineer, 2026-07-06. Addendum to
`docs/notes/b6-two-zone-data-report.md` §5, closing the B6 data
follow-up: the B6 (SCOTEX) planning capability of **6.7 GW** was cited
to the NESO ETYS "Scottish boundaries" web page (retrieved 2026-07-04)
but had no fetchable, checksummed artefact in the data packs. This note
records the artefact, its provenance and licence, and exactly where the
number sits in it.

A separate note (not an edit to the b6 report) because that report is a
reviewed, revision-stamped document (`b6-two-zone-data-review.md`
ACCEPT-WITH-NOTES); appending post-review content would muddy its
review trail. A separate pack + manifest (not an append to `b6.sha256`)
because `b6.sha256` pins the 2026-07-04 retrieval snapshot as a unit —
mixing a 2026-07-06 retrieval into it would blur that provenance, and
`.gitignore` already tracks any `data/packs/*.sha256`.

## The artefacts (pack `data/packs/etys/`, manifest `data/packs/etys.sha256`)

Raw files are gitignored per the fetch-and-build law; only the manifest
is committed. All four retrieved 2026-07-06 over HTTPS from
neso.energy.

| File | What it is | sha256 (manifest-pinned) |
|---|---|---|
| `raw/etys_2025_publication.pdf` | ETYS 2025 main publication (55 pp; PDF title "Electricity Ten Year Statement ETYS 2025", created 2026-06-30). https://www.neso.energy/document/383876/download | `a3cdb8b6…0dee46` |
| `raw/etys_2025_boundary_chart_data.xlsx` | **The load-bearing artefact.** "ETYS 2025 Boundary Chart Data" workbook. https://www.neso.energy/document/383896/download | `5199c86c…2c56d7` |
| `raw/etys_b6_boundary_chart.html` | The per-boundary B6 interactive chart download linked from the Scottish-boundaries page (Plotly HTML with the embedded capability series, incl. 6,700). https://www.neso.energy/document/351906/download | `440eea78…c1b6bab` |
| `raw/scottish_boundaries_page.html` | Snapshot of the cited web page itself (the prose citation). https://www.neso.energy/publications/electricity-ten-year-statement-etys/electricity-transmission-network-requirements/scottish-boundaries | `4704de11…6366bf` |

Full hashes in `data/packs/etys.sha256` (verified with `shasum -c` at
assembly).

## Exactly where the 6.7 GW sits

1. **Workbook** (`etys_2025_boundary_chart_data.xlsx`, sheet
   "ETYS 2025 Chart Data", header row = Boundary/Scenario/Category +
   years 2025–2045): rows `B6 / {HT, EE, HE} / Capability` all carry
   **6,700 MW for year 2025** — the current-network capability, under
   all three FES25 backgrounds. (Later years embed planned
   reinforcements and rise steeply — 15,354 MW by 2030 HT, 27,629 MW by
   2041 — so 6.7 GW is *the current capability*, which is how the b6
   report uses it: the 2024-validation upper bound and the ETYS
   planning-value sentinel replacement.)
2. **Page snapshot** (`scottish_boundaries_page.html`), section
   "Boundary B6 – SP Transmission to NGET": "capability is limited to
   6.7 GW due to thermal constraint on the Harker-Moffat 400kV circuit"
   — verbatim the b6 report's citation.
3. **Corroboration in the same workbook**: B4 Capability 2025 =
   4,002 MW (report: "B4 … 4.0 GW"), B5 = 3,900 MW ("B5 3.9 GW") —
   both match the b6 report §5 context figures.

## Edition note (honesty item)

The current page and workbook are the **ETYS 2025 edition** (NESO's
documents page states publication 30 June 2026; the main PDF's creation
date agrees). The b6 report's citation was retrieved 2026-07-04 — i.e.
days *after* that publication, so the figure it cited is the same
edition pinned here; the 6.7 GW is identical either way. The b6
report's *overload* context ("ETYS 2024 year-round analysis sees
overloads above ~5.1/5.8 GW") referenced ETYS-2024-vintage analysis
text; that narrative is not re-pinned by this addendum — only the
capability number is.

Second honesty item: the main publication PDF carries a leftover
Microsoft MSIP authoring label ("Internal Only New") in its metadata.
It is the file NESO publicly serves from the ETYS documents page; the
label is an authoring artefact, recorded here so nobody mistakes it
for a distribution restriction we ignored.

## Licence

- The ETYS boundary datasets on the NESO Data Portal carry the **NESO
  Open Data Licence** (OGL-v3-based; attribution "Supported by National
  Energy SO Open Data") — the b6 report §1 diligence stands.
- The ETYS *documents* (publication PDF, chart-data workbook, page)
  are NESO publications fetched from neso.energy without a per-file
  licence statement. Nothing is redistributed — raw files are
  gitignored, only sha256 manifests are committed, and the 6.7 GW fact
  is quoted with attribution — so this sits inside the b6 report's
  existing "Adopt (cited numbers)" verdict. If a future artefact needs
  to *ship* ETYS-derived data, take it from the data-portal datasets
  under the NESO Open Data Licence, not from these documents.

## Reproduction

```
cd data/packs
curl -L -o etys/raw/etys_2025_publication.pdf        https://www.neso.energy/document/383876/download
curl -L -o etys/raw/etys_2025_boundary_chart_data.xlsx https://www.neso.energy/document/383896/download
curl -L -o etys/raw/etys_b6_boundary_chart.html      https://www.neso.energy/document/351906/download
curl -L -o etys/raw/scottish_boundaries_page.html    "https://www.neso.energy/publications/electricity-ten-year-statement-etys/electricity-transmission-network-requirements/scottish-boundaries"
shasum -c etys.sha256
```

NESO document links are edition-scoped but the page is live; a
re-fetch after the next ETYS edition will (correctly) fail the
manifest, exactly like the b6 pack's rolling day-ahead file.
