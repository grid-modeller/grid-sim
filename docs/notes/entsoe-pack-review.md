# Review — ENTSO-E Stage 5 data package (pre-commit)

**Date:** 2026-07-03 · **Reviewer:** review gate
**Scope:** uncommitted `scripts/fetch-entsoe/` (fetch.py, build.py,
validate.py, analyze.py, README.md, requirements.txt),
`data/packs/entsoe-2024.sha256`, and
`docs/notes/entsoe-stage5-pack-report.md`. Stage 6 part-2 files
explicitly excluded (separate review).

**Verdict: ACCEPT-WITH-NOTES.** One documentation defect (curve-type
semantics, defect 1) must be corrected in the same commit; the rest are
notes of record. No data defect found: every headline number in the
evidence note was independently reproduced, the pack rebuilds
byte-identically from cached raw XML, and the licence citations were
verified against the primary documents.

## Checks run (all by the reviewer, none accepted on claim)

1. **Manifest — PASS.** `data/packs/entsoe-2024.sha256`: exactly 28
   entries, processed files only. Full (not spot) verification:
   `cd data/packs && shasum -a 256 -c entsoe-2024.sha256` → 28/28 OK.
2. **validate.py — PASS, exit 0**, run by the reviewer in the pinned
   venv (`~/.local/share/grid-sim/entsoe-venv`, Python 3.13.11;
   `pip freeze` matches requirements.txt exactly: requests==2.34.2,
   pandas==3.0.3, pyarrow==24.0.0). All 17,568-row/uniform-UTC/no-NaN/
   CSV≡Parquet checks pass; per-border wedge ceiling ±0.5 TWh enforced
   and passed (max observed 0.432, FR).
3. **Byte-identical rebuild — VERIFIED.** Reviewer re-ran build.py and
   analyze.py against the cached raw XML in a scratch tree (symlinked
   raw/ + NESO pack; repo copy untouched): all 28 outputs, including
   `build_report_entsoe_2024.json` and `analysis_entsoe_2024.json`,
   hash-identical to the committed manifest. Determinism claim is
   demonstrated, not asserted. Raw XML: 261 documents, 46 MB, retained
   in the git-ignored pack; zero Acknowledgement documents (report's
   "zero ACKs" claim confirmed by full parse of all 261 files).
4. **Reconciliation — REPRODUCED** (independent arithmetic on the
   parquet files, not analyze.py): FR +19.882, BE +4.301, NL +1.676,
   NO2 +9.915, DK1 +3.742, IE −5.174; total **+34.342 TWh** vs NESO
   **+33.301 TWh** (recomputed from `data/packs/2024` per-link sums).
   Wedge one-signed and proportional to gross imports (FR ≈2.0 %,
   NO2 ≈2.9 % of gross), ≈0 on the IE border where GB is the sender —
   internally consistent with sending-end vs GB-end metering + HVDC
   losses. The explanation is adequately evidenced by this pattern
   (direction-dependence is the discriminating observation), not
   hand-waved; correctly framed as "do not mix sources in one energy
   identity", and the Stage 5 imports tolerance keeps NESO as
   reference.
5. **Base rates and sign test — REPRODUCED.** FR import share 92.30 %
   (export 7.70, exact-zero 0.00). NO2 (NSL) net-import r vs GB wind CF
   −0.399 half-hourly / −0.458 daily; continental (FR+BE+NL) −0.352 /
   −0.430; NO2 hydro generation (reservoir+RoR) −0.087 / −0.088.
   Lowest-wind-decile means reproduced (FR 2,896 vs 2,263 MW;
   IE −749 vs −588). §6/§7 numbers reproduced: NO2 reservoir gen
   42.56 TWh, RoR 7.09, capacity reservoir 9.82 GW / RoR 1.41 /
   pumped 0.52 / wind 1.45; storage range 12.48–29.26 TWh, NO2 share
   of NO storage 43.7 %, inflow proxy 46.2 TWh; loads FR 429.7 /
   DE-LU 470.4 / NL 115.0 / BE 81.0 / NO2 36.1 / DK1 22.6 /
   IE 40.9 TWh; capacity table 79 rows.
6. **Gap handling — AUDITED** against `build_report_entsoe_2024.json`:
   IE flows 718 missing → 638 interpolated (≤2 h rule) + 80 NESO-filled
   (timestamps recorded); IE load 592 → 518 + 74 day-offset; DK1 flows
   2 interpolated; fr/be/nl/no2 flows and fr/be/nl/delu/no2/dk1 loads
   zero gaps; NO-aggregate solar 6,092 zero-filled slots = 5,808
   (absent months) + 284 (residual), recorded; NO2 generation needed no
   repair. All counts match the evidence note.
7. **Parser semantics — AUDITED against raw XML.** Reviewer re-expanded
   documents independently: FR flows (PT60M, A03) hourly-repeat and
   hold-at-missing-position verified value-for-value; BE flows
   (PT15M, A03, sparse) reconstructed with an independent A03-hold +
   pair-mean implementation — max |mine − built| = 0.0 over the month;
   IE PT30M taken as-is. The FR Dec-31 PT15M period (27/96 points,
   69 held) was cross-examined: the PT60M December series ends Dec 31
   00:00, Dec 31 exists only in that A03-compressed PT15M envelope, and
   the hold reconstruction yields a plausible diurnal curve (61.7 GW
   night trough → 71.5 GW midday) — correct under declared semantics.
   UTC clock-change slots (31 Mar, 27 Oct) present exactly once each.
   **But see defect 1: the platform emits curveType A03 for ALL
   document types, not just flows.**
8. **Licence — PRIMARY-SOURCE VERIFIED.** The help-centre article is
   Cloudflare-challenged, but the reviewer retrieved the "List of Data
   Available for Free Re-Use" (attachment 40921869379729, direct) and
   the GTC PDF (attachment 40921869376401 via the Wayback Machine,
   snapshot 2026-04-28). Verified verbatim: GTC approved 29 Mar 2023 /
   applicable 1 Nov 2023; clause 3.1 ("for any purpose whatsoever",
   good-faith, source-mention, no-prejudice-to-Primary-Owner-copyright
   wording exactly as quoted); clause 2.5 Open Data wording exactly as
   quoted; list last modified 18 Oct 2023; physical flows = item 18
   (Art. 12.1.g) listed; actual load 6.1.a NOT listed (only forecast
   items 1–4, 6.1.b–e); installed capacity 14.1.a, generation 16.1,
   reservoir filling absent from the list; exclusions verbatim — IFA
   ("Interconnexion France-Angleterre") and Nemo Link excluded
   entirely, BritNed excluded only for balancing items #24, 26, 29,
   30, 32–35, plus MD/TR and UA balancing items. §1 of the evidence
   note is accurate.
9. **Token hygiene — CLEAN.** The token value (read from
   `~/.local/share/grid-sim/entsoe-token`) appears nowhere in the repo
   (full grep including the git-ignored raw XML and processed pack);
   only the path is referenced. fetch.py scrubs URLs from error output
   and never logs the token.
10. **Scope — CLEAN.** Only the three named deliverables are new; no
    edits to docs/04, docs/05, or any ADR; per-link GB flows being
    unavailable from ENTSO-E (border-level only) is documented and is
    corroborated by the reconciliation (border series ≈ NESO per-link
    sums + losses). No Rust code touched → cargo gates not applicable
    to this package.

## Rulings on decisions 1–5 (evidence note §1)

1. **Fetch-and-build locally, per-user token — ACCEPT.** Squarely
   within clause 3.1 (verified) and clause 3.3 (M2M interface offered
   for exactly this; fetch.py throttles to ~170 of the 400 req/min
   cap).
2. **Git-ignored pack + committed sha256 manifest — ACCEPT.** No
   Transparency Platform data is redistributed; filenames and hashes
   are not TP data. Satisfies both clause 3.1 and the docs/05
   fetch-and-build/manifest convention. `.gitignore` verified to
   exclude the pack and admit only `data/packs/*.sha256`.
3. **Publishing derived aggregate statistics — ACCEPT, with
   condition C2** (attribution string must actually accompany every
   published number derived from this pack, and non-CC-BY items are
   published only as derived aggregates, never as trace files). The
   posture of keeping publication-grade GB↔FR / GB↔BE per-link numbers
   on Elexon/NESO is the right belt-and-braces.
4. **Phase-2 hosted-pack flag — ACCEPT as a deferral.** The flagged
   exclusions match the verified list; correctly not a Stage 5
   decision.
5. **IFA2/ElecLink treated as inside the IFA carve-out — ACCEPT the
   conservative reading.** The list names "Interconnexion
   France-Angleterre"; the platform serves one combined GB↔FR A11
   series whose per-asset primary owners are not separable (per-asset
   virtual zones return no A11 data), so no narrower reading is
   operationally available. Nothing in the architecture depends on
   resolving it: the pack is never redistributed and per-link published
   numbers cite Elexon/NESO.

## Ruling on the docs/04 sign-test reformulation

**Legitimate evidence-driven refinement, not gate-weakening.** The
observation that NSL *flows* are the most wind-anticorrelated series
(r ≈ −0.40/−0.46) while NO2 hydro *generation* is uncorrelated
(r ≈ −0.09) is pinned from observation, before any Stage 5 engine
exists — this is data contact, not model contact, and the gate as
written ("Norwegian hydro exports uncorrelated with GB wind") encodes
an empirically false expectation at the flow level that no correct
model could pass. The reformulation preserves the anticyclone-mechanism
intent (resource-level independence + continental correlation) and
*adds* a requirement (the model must reproduce the NSL flow
anticorrelation from scarcity pricing). Precedent: the Stage 2 gate
re-pin. **Condition C3:** the docs/04 amendment is the supervisor's to
make, must cite the evidence note as the contrary-finding record, and
must pin numeric thresholds (including the GB↔FR direction-match
number, which must sit meaningfully above the 92.3 % constant-predictor
base rate); until then the docs/04 Stage 5 sign test must not be quoted
as the gate.

## Ruling on the cross-source (NESO) fill

**Acceptable for a validation-grade pack as executed.** 80 of 17,568
slots (0.46 %) on one border, from the already-adopted primary source
measuring the same physical quantity at the GB end, every timestamp
recorded in the build report, rule documented in code prose
(build.py `fill_flows_from_neso`). Note N4: those slots are not
independent evidence — the IE ENTSO-E-vs-NESO direction-agreement
figure (98.12 %) and any future IE-border cross-source comparison
inherit 0.46 % of self-agreement; immaterial at this scale but must be
remembered if the IE border is ever used to *validate* NESO data.

## Defects and conditions

1. **[Fix in the same commit] Curve-type record is factually wrong.**
   build.py docstring (lines 22–26), README.md ("A11 uses curveType
   A03 ... A65/A75/A72 use A01"), and the evidence note §2 ("A03
   repeat-blocks for flows, A01 fixed-blocks for the rest") all claim
   the platform emits A01 for load/generation/reservoir. Reviewer
   parsed all 261 raw documents: **every TimeSeries of every type
   declares curveType A03** (flows 144, load 84, gen 188, capacity 79,
   reservoir 2; 7 load periods and 125 generation periods have
   omitted-position compression that was hold-filled, e.g. FR load
   Dec 31, NO biomass/"other" published as 1 point/month). The built
   data is CORRECT — parse_doc reads the declared curveType per
   TimeSeries and applies declared semantics — but the written
   convention record is exactly the wedge-class error this project
   documents against, and a future maintainer acting on the A01 claim
   (e.g. treating omitted load positions as gaps) would corrupt a
   rebuild. Correct the three places to state: curveType is read per
   TimeSeries; in the 2024 documents all series declare A03
   (hold-forward); the A01 branch exists for robustness and is
   currently unexercised.
2. **[Condition of record] Attribution at publication.** Any published
   number derived from this pack carries "Source: ENTSO-E Transparency
   Platform"; non-CC-BY items only ever as derived aggregates
   (decision 3 above).
3. **[Condition of record] docs/04 Stage 5 amendment** per the
   reformulation ruling above (supervisor action; thresholds numeric;
   contrary finding cited).
4. **[Note] IE NESO-filled slots** are not independent for
   cross-source claims (ruling above).
5. **[Note] Minor factual slips in the evidence note, non-blocking**
   (fix opportunistically or let this review be the correction of
   record): (a) §6 "one negative NO week in January" — actually 4
   negative NO proxy weeks (07 Jan, 28 Jan, 10 Mar, 14 Apr) and 3 for
   NO2; (b) §2/§9 "NO-aggregate solar absent/reported Apr–Nov" — the
   series first appears 6 May and is patchy in December (zero-filled
   months are Jan–Apr + 284 residual slots; the recorded 6,092 total
   is correct); (c) §4 "direction agreement (both >50 MW active)" —
   analyze.py's activity mask is *either*-source >50 MW
   (`both_active = (e.abs() > 50) | (n.abs() > 50)`), idle periods
   counted as agreeing.
6. **[Note] Raw XML is not checksummed anywhere.** The processed
   manifest is the value-identity anchor (docs/05) and the rebuild was
   proven byte-identical, but if the raw cache is ever lost, a
   re-fetch may legitimately differ (platform revisions) and the
   builder cannot distinguish revision from regression. A git-ignored
   raw-side sha256 sidecar (era5 fetch-log precedent) would close
   this; recommended next time the fetch runs.

## Reproduction commands

- Manifest: `cd data/packs && shasum -a 256 -c entsoe-2024.sha256`
- Validation: `~/.local/share/grid-sim/entsoe-venv/bin/python
  scripts/fetch-entsoe/validate.py <repo>` (exit 0)
- Rebuild: build.py + analyze.py against a scratch tree with the raw/
  dir and `data/packs/2024/processed` linked in; hash-compare all 28
  outputs to the manifest (28/28 identical, this review).
- Licence: list PDF direct from the zendesk attachment; GTC PDF via
  web.archive.org snapshot 20260428130408 of attachment
  40921869376401.
