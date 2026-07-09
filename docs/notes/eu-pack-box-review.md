# Review — NW-Europe ERA5 pack + `fetch_era5.py --box` (deferred formal review)

**Date:** 2026-07-03 (Stage 5 opening) · **Reviewer:** review gate
**Scope:** the review deferred at the NW-Europe banking entry
(`memory/project-state.md`, 2026-07-03): the `--box` extension to
`scripts/era5-cf/fetch_era5.py`, the EU weather pack
(`data/packs/era5-eu/`, manifest `era5-eu-1985-2024.sha256`), and its
provenance/licence/determinism discipline — before any Stage 5 work
consumes the pack.

**Verdict: ACCEPT-WITH-NOTES.** No defect blocks Stage 5 consumption.
Notes 1–3 below are conditions of record, not fix-work.

## Checks run (all by the reviewer, none accepted on claim)

1. **`--box` GB-path byte-unchanged claim — VERIFIED from the diff.**
   `--box` landed in commit 16219b9 (`git diff 16219b9^ 16219b9 --
   scripts/era5-cf/fetch_era5.py`). The old constants
   (`LAT = slice(61, 49)`, `LON_WEST = slice(352, 359.75)`,
   `LON_EAST = slice(0, 2)`, `N_CELLS = 49*41`) map exactly onto
   `GB_BOX = Box("gb", 49.0, 61.0, -8.0, 2.0)` through `box_cut()`
   (Greenwich-spanning branch: west `[352, 359.75]` + east `[0, 2]`,
   west re-labelled −360 — identical slices, identical concat order)
   and `Box.n_cells` (49×41 = 2,009). `era5_dir_for()`/`month_path()`
   reproduce the legacy `data/packs/era5/<year>/era5_gb_*.parquet`
   layout for the default box. The only behavioural changes on the GB
   earthmover path are a bounded S3 retry (re-reads the same pinned
   snapshot — no data change) and an empty-carry-buffer guard (old code
   would have crashed; no written bytes differ). `parse_box()` reserves
   the name `gb` and forces 0.25°-grid alignment, so a named box can
   never collide with the GB layout.
2. **GB manifests re-verify locally — PASS.** `shasum -a 256 -c` on
   `era5-2024.sha256` (12 files) and `era5-1985-2023.sha256`
   (468 files): zero mismatches.
3. **EU manifest coverage — PASS.** `era5-eu-1985-2024.sha256`:
   exactly 480 entries = 40 years × 12 months, paths
   `era5-eu/<year>/era5_eu_<year>-MM.parquet`, no gaps, no extras,
   480 distinct hashes.
4. **EU pack checksums — FULL verification, PASS.** All 480 local files
   (49 GiB) verify against the committed manifest (not a spot check).
5. **Write-time hashes vs manifest — PASS.** The fetch log
   (`data/packs/era5-eu/fetch-earthmover-1985-2024.log`, git-ignored
   alongside the data) carries a per-file sha256 for all 480 files; all
   480 match the committed manifest. The log's first line records
   `snapshot_id=39TK56WX185WZ1HP9WNG`; one transient `IcechunkError`
   with retry is visible (chunk 52), matching the README's account.
6. **GB-overlap cross-check — independently REPRODUCED.** Reviewer
   loaded the EU 2024-01 and 2024-06 cutouts, cut the GB box
   (13,189 cells asserted in the EU frames; 1:1 merge with the ARCO GB
   cutouts on time/lat/lon, full row-count match), and computed the
   README's metric (max|diff|/max|value| per variable-month):
   - 2024-01: u100 1.66e-5, v100 1.63e-5, ssrd 2.75e-5, t2m 2.85e-6
   - 2024-06: u100 2.66e-5, v100 2.41e-5, ssrd 9.57e-6, t2m 3.22e-6
   - t2m max abs diff 2024-06 = 0.000976562 K = 2⁻¹⁰ K exactly (the
     GRIB packing quantum), as recorded.
   This confirms the recorded "1e-5–3e-5 per variable" / "≤2.9e-5"
   claim (reviewer max 2.75e-5; the 0.15e-5 delta vs the recorded
   bound is within choice-of-denominator noise) and is consistent with
   the GB pack's own ARCO-vs-Earthmover seam table (≤3.7e-5, same
   decode-lineage quantum). The Earthmover 2024 layer agrees with ARCO
   at decode-lineage level; the EU pack is single-source with no
   internal seam.
7. **Provenance / licence / determinism (docs/05) — PASS.**
   Source, snapshot ID, retrieval date, coverage, variables, box
   geometry, layout, and manifest conditionality (requirements.txt
   versions + snapshot ID) recorded in `scripts/era5-cf/README.md`
   ("NW-Europe box" section). ERA5 CC-BY 4.0 + Copernicus attribution
   carried (README "Source, licence, attribution"). Environment fully
   pinned (`requirements.txt`, incl. `icechunk==2.1.0`). Fetch is
   deterministic (no randomness/wall-clock in outputs; snapshot-pinned
   store; final ERA5), atomic per-month writes, resumable (overwrite
   semantics + narrowed `--years` documented). A third party can
   rebuild the pack from the recorded command
   (`fetch_era5.py --years 1985-2024 --source earthmover
   --snapshot 39TK56WX185WZ1HP9WNG --box eu,42,72,-11,16`) and check
   it against the committed manifest — the docs/05 requirement
   (results = f(scenario, data-pack checksum, engine hash), pack
   buildable and checksummable by a third party) is met under the
   documented bounded-Python exception.

## Notes (conditions of record)

1. **Commit traceability.** The `--box` code landed inside the Stage 3
   commit 16219b9, whose message says the Europe-box fetch is "not in
   this commit" and does not mention the `--box` code change; the
   banking commit bb8018b says "fetch_era5.py gains --box" but contains
   no fetch_era5.py change. The recorded review deferral covers the
   code, but the commit attribution is misleading for future
   archaeology — this note is the correction of record.
2. **Cross-check not re-runnable from committed code.** The GB-overlap
   comparison (and the earlier GB seam table) were ad hoc measurements
   recorded in prose. Acceptable under the existing precedent — both
   packs and manifests are committed/available, and this review
   reproduced the numbers independently — but a future source switch
   should commit its comparison script.
3. **No committed EU-pack validator.** The "independent post-fetch pass
   over all 480 files" was ad hoc (inline write-time validation exists
   in `write_month_earthmover`: hour count, 13,189 cells, no NaNs).
   When Stage 5 derives EU CF traces, its validator should re-assert
   pack geometry the way `validate_multiyear.py` does for GB.

## Fitness for Stage 5 (docs/04 Stage 5: external zones FR, NO, NL/BE/DE)

**Box 42–72°N, 11°W–16°E — what is IN:**
- Ireland: complete (westernmost ≈10.5°W).
- France: mainland complete (southernmost ≈42.33°N).
- Belgium, Netherlands: complete.
- Germany: complete (easternmost ≈15.04°E < 16°E).
- Denmark: complete incl. Bornholm (≈15.2°E).
- Norway: everything **west of 16°E** — NO1 (Oslo), NO2 (southwest;
  the NSL landing at Kvilldal and the big hydro reservoirs), NO5
  (Bergen), and most of NO3 incl. the Fosen wind cluster. Latitude is
  not the binding cut (Nordkapp 71.17°N < 72°N).

**What is OUT (known gaps, stated before engine design):**
- **Norway east of 16°E**: most of NO4 (Troms/Finnmark — Tromsø
  18.96°E, Finnmark wind e.g. Raggovidda ≈29.7°E) and northeastern
  slivers of NO3. Immaterial for GB imports (NSL connects to NO2;
  Norwegian wind capacity is concentrated in-box at Fosen), but a NO
  zone CF trace from this pack under-represents NO4 wind — state it in
  the Stage 5 zone design.
- Southern Corsica (<42°N): irrelevant (not synchronous with the FR
  mainland zone).
- **No hydro-capable variables.** The pack is u100/v100/ssrd/t2m —
  wind CF, solar CF, and temperature-driven demand only. The Stage 5
  acceptance test "Norwegian hydro exports uncorrelated with GB wind"
  CANNOT be served from this pack: Norwegian hydro needs
  inflow/reservoir/generation data (the ENTSO-E track, now unblocked).
  This is by design ("Stage 5 design decides technologies first") but
  must be explicit in the work plan.
- No 10 m winds (same as GB: derivation shears from 100 m) — fine if
  Stage 5 reuses the GB derivation method.

**Statement:** the pack is fit to serve as the weather substrate for
Stage 5's external-zone wind/solar CF and temperature-demand
derivation for FR, BE/NL/DE, DK, IE, and southern/central Norway. It
does not, and was never intended to, cover Norwegian hydro or NO4
wind; Stage 5's data plan must source hydro from ENTSO-E.
