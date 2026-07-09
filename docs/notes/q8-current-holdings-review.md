# Q8 current-holdings data package — adversarial review

Reviewer, 2026-07-03. Package under review (uncommitted):
`data/reference/response-holdings-2025.toml`,
`docs/notes/q8-current-holdings.md`,
`data/packs/response-holdings.sha256` (manifest for fetched-not-committed
raw CSVs under `data/packs/response-holdings/raw/`).

## Verdict: ACCEPT-WITH-NOTES

The engine fragment, the volumes, the licence analysis, the parameter
derivations and the brief correction all verify independently. Two
evidence-note numbers do not reproduce and must be corrected before the
variant run quotes them (conditions 1–2); two wordings should be
tightened (conditions 3–4). Nothing conditions the engine fragment, the
headline FY2025 means, or the 2,336/1,896 MW comparison anchor.

## Conditions

1. **Note §3, DC-L EFA-block claim — not reproducible.** "varies by EFA
   block (window means 1,013–1,328 MW)". Reviewer recomputation from the
   pinned FY2025 CSV: per-EFA-block FY2025 means are **1,106–1,225 MW**
   (identical partition whether blocks are taken in UTC or Europe/London
   local); month×block means span 929–1,392. No stated grouping yields
   1,013–1,328. Correct the numbers or state the grouping that produces
   them.
2. **Note §3, "Quarterly snapshot (Apr–Jun 2026 means 1,162/510/480)" —
   mislabelled window.** 1,162 MW is the DC-L mean of the **entire**
   current dump (EFA days 2026-04-01 → 2026-07-04, i.e. including 1–4
   July); the true Apr–Jun DC-L mean is **1,147–1,148 MW** (510/480 are
   correct either way). Relabel or recompute. The conclusion drawn
   ("convention is not stale") survives either number.
3. **Note §3, "≈1,290 MW summer, ≈1,010 MW winter" — months unstated.**
   Reproduces only as Apr–Jun (1,290) vs Jan–Feb (1,010); JJA is 1,254
   and DJF 1,029. The inverse-seasonal direction is real under any
   reading; name the months.
4. **TOML droop caveat, "OVERSTATES DC delivery in the 0.015–0.45 Hz
   band" — band understated.** The engine's linear droop
   (`clamp(Δf/0.5, 0, 1)`, swing.rs) is ≥ the published two-segment
   curve over the whole 0–0.5 Hz range; the curves meet only at
   saturation. Direction (optimistic) and the saturated-nadir argument
   are unaffected; widen the stated band.

## Independently recomputed numbers (raw CSVs, reviewer's own code)

- **Manifest**: all three checksums verify (`shasum -c` OK). **Refetch
  test**: FY2025 archive re-downloaded from the NESO URL 2026-07-03 →
  **bit-identical sha256** `901fd1ad…`; Service Terms PDF re-downloaded →
  sha256 `6fb56f47…` matches.
- **FY2025 archive**: 62,864 data rows; DCL/DML/DRL/DCH/DMH/DRH each
  exactly **2,190 windows, zero duplicates**; windows contiguous over
  2025-03-31T22:00 → 2026-03-31T22:00 UTC (2,188 × 4 h, one 5 h and one
  3 h clock-change window — as the note describes).
- **Means (exact match to the package)**: DCL **1,178.2** (min 817, p5
  884, p95 1,405, max 1,646); DML **416.2** (44/295/530/533); DRL
  **460.8** (210/377/480/494); DCH 1,120.8, DMH 432.8, DRH 492.7.
  Totals 2,055 / p5-sum 1,556 / max-sum 2,673 ✓.
- **Monthly**: DCL low Jan-26 1,006, high 1,292 (Jun-25 1,291.9 vs
  Apr-25 1,291.6 — tie at rounding; TOML says Apr-25, immaterial); DML
  276 (Apr-25) → 515 (Feb-26) step-up ✓; DRL 404 → 480, pinned at cap ✓.
- **June 2026** (current dump, calendar-month): DCL 1,244 / DML 500 /
  DRL 480 = 2,224 ✓. Dump: 30,780 rows, span as stated ✓.
- **SFFR FY2025** (accepted volume summed per window): **2,190/2,190
  windows, mean 202.0, min 50, max 255** ✓; 209,613 data rows ✓.

## Schema and engine checks

- `RawPathwayResponse` (grid-stability/src/pathway.rs:459):
  `name, mw, delivery_factor (default 1.0), droop_full_deviation_hz,
  delay_s, ramp_s, sustain_s?, rundown_s?`, `deny_unknown_fields`. The
  note §4 fragment uses exactly the required subset; **reviewer parsed
  the fragment verbatim through `PathwaySpec::from_toml_str` → PARSE
  OK**. `delivery_factor` omission → code default 1.0 confirmed;
  `sustain_s` omission is legal and justified (Tsus 15/30/60 min ≫
  120 s window — Tsus values confirmed in the pinned Service Terms).
- **Service Terms parameters** (verified in the refetched, checksum-
  matching PDF): TiMAX 0.5/0.5/2 s (DR 0.5 s only when stacked —
  standalone reading is the conservative one, correctly used), TdMAX
  1/1/10 s, ramp upper bound 0.5/0.5/8 s = TdMAX−TiMAX ✓; deadband
  ±0.015 Hz; DC knee ±0.2 Hz @ 5 %, saturation ±0.5 Hz; DM knee
  ±0.1 Hz @ 5 %, saturation ±0.2 Hz; DR linear, saturation ±0.2 Hz —
  all as documented.
- **Brief correction verified in code and spec**: the 931 MW in the
  brief is the LFDD stage-1 block
  (`scenarios/events/gb-2019-08-09.toml`); `default_2019_responses()`
  holds 472 + 550 + 1,314 = **2,336 MW**, effective 472×1.0 +
  550×0.670909 + 1,314×0.802892 = **1,896.0 MW**, secondary full at
  10+20 = **30 s**. 1,022 = 472+550 ✓; 1,594 = DCL+DML full at 1 s ✓;
  DR-L full at 10 s ✓. ("472 MW sub-second" — battery_ffr is full at
  exactly 1.0 s; the looseness flatters the 2019 side, conservative for
  the claim being made.)

## Licences

Sound and complete. Portal datasets under the NESO Open Data Licence
with the required attribution string recorded in both files, matching
the fes-pathway/part-2-report precedent; Service Terms treated exactly
as the FRCR in `data/reference/stability-2019-event.toml` (sha256
pinned, facts quoted with attribution, PDF not redistributed); Ofgem
decision quoted with attribution. No proprietary source.

## Conventions and caveats ruling

- Annual-mean-of-cleared-volumes is a defensible central convention;
  the DM-L step-up (annual 416 vs going-forward ~500) is flagged in
  both files — publication-grade.
- Delivery-factor 1.0 with a prescribed 0.9-uniform sensitivity is the
  right treatment: the engine caps delivery_factor at 1.0, NESO
  publishes no EAC performance factors, and the contract-vs-measured
  asymmetry is stated with its direction. KC-prominence standard met.
- Exclusions (SFFR 202 MW static — engine rejects latched services at
  parse by design, verified in pathway.rs; MFR unquantified; reserve
  products BM-instructed) are all conservative in the stated direction.

## Observations (no action required)

- The Service Terms PDF is checksum-pinned in the TOML but kept neither
  in `raw/` nor in the manifest — consistent with the FRCR precedent it
  cites, though fes2025 and prices-2024 keep their PDFs in `raw/`.
  Reviewer refetch matched today; NESO document URLs drift — consider
  adding the PDF to `raw/` + manifest.
- TOML `[excluded.reserve_products]` lists NSR/PSR; the FY2025 archive
  contains only NBR/NQR/PBR/PQR. Harmless.
- Window means weight the two clock-change windows (5 h/3 h) equally
  with 4 h windows; effect < 0.1 MW.

## Gates

- `cargo test --workspace`: **395 passed / 0 failed** (reviewer-run).
- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets
  -- -D warnings` clean.
- Scope: exactly the three files; no code, scenario, or `memory/`
  edits. (Two unrelated untracked files — `data/reference/costs-gb.toml`,
  `docs/notes/stage7-cost-inputs-report.md` — appeared in the worktree
  during review; Stage 7 package, not this one, not reviewed here.)
- TDD: data/docs-only package; no library code, no tests required;
  suite green.
