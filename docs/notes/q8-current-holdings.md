# Q8 current-holdings variant — evidence note: NESO response holdings, FY2025

Assembled 2026-07-03 (data engineer), for the current-holdings variant run
named by `stage-6-part2-run-report.md` §2 consequence 1 and §6 publication
rule 2 ("2019-era response holdings could not secure 1,800 MW" — never "GB
today cannot"; holdings are a spec input awaiting a current-holdings
variant run). Machine-readable record with per-number citations:
`data/reference/response-holdings-2025.toml`. Raw fetched snapshots:
`data/packs/response-holdings/raw/` (fetched, not committed); committed
checksum manifest: `data/packs/response-holdings.sha256`.
Reviewed 2026-07-03: ACCEPT-WITH-NOTES
(`docs/notes/q8-current-holdings-review.md`); the four review precision
fixes are incorporated below, each re-verified against the raw CSVs.

## 1. Sources and licences (checked first, per D1 discipline)

| Source | Licence | Redistribution |
|---|---|---|
| NESO Data Portal, dataset `eac-auction-results` (Results Summary FY2025 archive + current dump), retrieved 2026-07-03 | NESO Open Data Licence | Permitted with attribution: published outputs must carry **"Supported by National Energy SO Open Data"** |
| NESO Data Portal, dataset `static-firm-frequency-response-auction-results`, retrieved 2026-07-03 | NESO Open Data Licence | As above |
| NESO "Response Services — Service Terms" (2025-08-28), doc 367526, sha256 `6fb56f47…` | NESO publication (not portal data) | Facts/short quotes with attribution; PDF not redistributed (same treatment as the FRCR in the 2019 record) |
| Ofgem MFR derogation decision (effective 2025-04-01 → 2029-12-31) | Ofgem publication | Quoted with attribution, not redistributed |

No proprietary source was used or needed; everything is NESO primary.
URLs and sha256 checksums for every retrieved file are in the reference
TOML `[sources]` blocks.

## 2. What NESO procures today (verified against the live portal, not memory)

The response suite is Dynamic Containment (DC, post-fault), Dynamic
Moderation (DM, fast pre-fault) and Dynamic Regulation (DR, slow
pre-fault), each split low/high frequency (DCL/DCH etc.), procured in
daily EAC auctions per 4-hour EFA window. Service names unchanged since
2021; the procurement platform changed (EAC, Nov 2023) and now
co-optimises response with reserve products (Quick/Slow/Balancing Reserve
— BM-instructed reserve, not droop response, excluded). Two legacy
services persist: **Static FFR** (still auctioned daily; reform
consultation opened Nov 2025 — not ceased) and **MFR** (continues under
Ofgem derogation to 2029; no published per-period volumes).

## 3. Holdings, FY2025 (EFA days 2025-04-01 → 2026-03-31)

Cleared (procured) MW per 4-h EFA window, from the FY2025 EAC Results
Summary archive. Coverage: 2,190 windows per product (365 days × 6), no
gaps, no duplicates; `deliveryStart/End` are UTC (dataset convention), so
the EFA-day boundary moves 22:00↔23:00 UTC at the BST clock changes —
handled by aggregating on the windows themselves, not on calendar days.

| Service (low-freq) | Mean | Min | p5 | p95 | Max | Monthly-mean range | Jun 2026 mean |
|---|---|---|---|---|---|---|---|
| DC-L | **1,178** | 817 | 884 | 1,405 | 1,646 | 1,006 (Jan 26) – 1,292 (Apr 25) | 1,244 |
| DM-L | **416** | 44 | 295 | 530 | 533 | 276 (Apr 25) – 515 (Feb 26) | 500 |
| DR-L | **461** | 210 | 377 | 480 | 494 | 404 (Apr 25) – 480 (Mar 26) | 480 |
| **Total LF dynamic** | **2,055** | | | | | | 2,224 |

High-frequency counterparts (recorded, not consumed by a loss event):
DC-H 1,121 / DM-H 433 / DR-H 493 MW FY2025 means — DC volumes differ
low-vs-high by ~5 %.

Residuals: **SFFR 202 MW** mean (range 50–255, procured in every FY2025
window) — static/latched, which the engine rejects at parse by documented
design, so it is excluded **conservatively**. **MFR**: volume not
published anywhere on the portal (checked 2026-07-03) — excluded rather
than estimated, also conservative.

### Variability and the recommended convention

- DC-L is **inverse-seasonal** — more DC held when inertia is low:
  monthly means run from 1,290 MW (Apr–Jun 2025 mean) down to 1,009 MW
  (Jan–Feb 2026 mean); on meteorological seasons, JJA 2025 mean
  1,254 MW vs DJF 2025/26 mean 1,029 MW.
- DC-L also varies by EFA block: FY2025 per-block means span
  **1,106 MW (block 6, 19:00–23:00 local) to 1,225 MW (block 3,
  03:00–07:00 local)** — the same 1,106–1,225 range under either a UTC
  or a local-time block partition. (Individual windows span 817–1,646;
  the month×block interaction spans ~929–1,392.)
- DM-L **stepped up through FY2025** (monthly mean 276 → 515 MW); the
  annual mean (416) understates the going-forward holding (~500 MW).
- DR-L is near-constant at its ~480 MW requirement cap.

**Recommended central convention: annual mean of EAC cleared volumes over
FY2025** (the most recent complete NESO financial year: full seasonal
cycle, stable archive resource, matches the FES annual-year framing of
the Q8 pathway). State it as "FY2025 mean procured volumes". Low edge for
sensitivity: the per-product p5s (884/295/377, sum 1,556 MW — per-product
tails, not a joint p5). Quarterly snapshot (EFA days 2026-04-01 →
2026-06-30: means 1,147/510/480; DC-L 1,148 on a calendar-UTC partition)
confirms the convention is not stale.

## 4. Engine fragment (pathway `[assumptions]`, RawPathwayResponse fields)

Per-number citations and the four stated modelling conventions (linear
droop vs the published knee curves; contract maxima as delays;
no sustain inside the 120 s window; delivery_factor 1.0) are in
`data/reference/response-holdings-2025.toml` — read them before quoting.

```toml
# Current NESO holdings, FY2025 mean cleared volumes (EAC auction results,
# NESO Open Data Licence). Full citations:
# data/reference/response-holdings-2025.toml.
[[assumptions.responses]]
name = "dynamic_containment_lf"    # DCL, post-fault
mw = 1178                          # FY2025 mean cleared
droop_full_deviation_hz = 0.5      # saturation +/-0.5 Hz (Service Terms)
delay_s = 0.5                      # TiMAX
ramp_s = 0.5                       # TdMAX 1 s - TiMAX

[[assumptions.responses]]
name = "dynamic_moderation_lf"     # DML, pre-fault fast
mw = 416                           # FY2025 mean cleared (Jun 2026: 500)
droop_full_deviation_hz = 0.2      # saturation +/-0.2 Hz
delay_s = 0.5
ramp_s = 0.5

[[assumptions.responses]]
name = "dynamic_regulation_lf"     # DRL, pre-fault slow
mw = 461                           # FY2025 mean cleared
droop_full_deviation_hz = 0.2      # saturation +/-0.2 Hz
delay_s = 2.0                      # TiMAX standalone
ramp_s = 8.0                       # TdMAX 10 s - TiMAX
```

(`delivery_factor` omitted = parser default 1.0; `sustain_s` omitted =
indefinite within the 120 s window — DC/DM/DR contracted durations are
15/30/60 min.)

## 5. What the 2019-vs-current comparison will mean

The Stage 6 part 2 Q8 run used the 2019-default holdings: **2,336 MW
held** (1,022 MW primary + 1,314 MW secondary), **1,896 MW effective**
after measured delivery factors, slowest tranche full at 30 s. (The work
package brief said "931 MW-class 2019 holdings" — 931 MW is the LFDD
stage-1 block, not the holdings; the run's holdings are the 2,336 MW
trio, drift-guarded against `scenarios/events/gb-2019-08-09.toml`.)

Current LF holdings are **2,055 MW mean — smaller headline MW, materially
faster**: 1,594 MW (DC-L + DM-L) full within 1 s vs 472 MW sub-second in
2019; DR-L full by 10 s vs the 2019 secondary full by 30 s. The variant
therefore tests speed as much as volume. Interpretation caveats, binding:

1. **Contract-vs-measured asymmetry.** 2019 uses measured delivery
   factors (0.67–1.0); current services carry the contractual 1.0 (NESO
   publishes no EAC performance factors). The comparison is
   holdings-as-contracted vs holdings-as-delivered. Quantify by a
   delivery-factor sensitivity (e.g. 0.9 uniform), not by inventing a
   central factor.
2. **Linear-droop optimism.** The engine's single-parameter droop sits
   at or above the published DC knee curve over the **whole 0–0.5 Hz
   range** — the two meet only at saturation (e.g. linear gives 40 % at
   0.2 Hz vs the real curve's 5 %). Saturated at the 48.8 Hz floor, so
   it shapes the arrest trajectory, not the nadir balance — same family
   as the part-1 §2 "envelope fast mid-phase" caveat. The no-retuning
   rule is untouched (no 2019 constant changed).
3. **Conservative exclusions.** ~202 MW SFFR (static, engine-rejected by
   design) and unquantified MFR are real low-frequency response not
   credited to the current side.
4. Publication rules of both stage-6 reports continue to bind; the
   variant's numbers are a band with the φ condition named, and outputs
   using these volumes must carry "Supported by National Energy SO Open
   Data".

## 6. Validation summary (fetched data)

- FY2025 archive: 62,864 rows; response products 2,190 windows each
  (365 × 6), zero gaps/duplicates; window timestamps UTC; clock-change
  weeks verified by window count (EFA-day boundary shifts in UTC, no
  missing/extra windows).
- Current dump (2026-07-03 snapshot): 30,780 rows, EFA days 2026-04-01 →
  2026-07-04; live resource — the sha256 pins the snapshot, expect drift
  on refetch.
- SFFR results: 209,613 data rows from 2023-04-01; FY2025 slice covers
  all 2,190 windows.
- Checksums: `data/packs/response-holdings.sha256` (includes the
  Service Terms PDF, kept for provenance at
  `data/packs/response-holdings/raw/response-service-terms.pdf` —
  prices-2024 precedent).
