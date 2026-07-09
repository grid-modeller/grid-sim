# B6 two-zone data package — adversarial review

Reviewer, 2026-07-04. Package under review (uncommitted):
`docs/notes/b6-two-zone-data-report.md`,
`scripts/era5-cf/derive_cf_gb2zone.py`, `scripts/fetch-b6/` (fetch.py,
build.py, README.md, requirements.txt),
`data/packs/cf-gb2-1985-2024.sha256` (481 entries),
`data/packs/b6.sha256` (21 entries), the `scripts/era5-cf/README.md`
append. Data gitignored under `data/packs/cf-gb2/` and `data/packs/b6/`.
Concurrent Q5 heating-mix files (grid-adequacy/grid-cli/
royal-society-37y-heated.toml) are out of scope and untouched by this
review. Everything below was recomputed or refetched by the reviewer;
nothing was taken on the implementer's word.

## Verdict: ACCEPT-WITH-NOTES — conditions 1–3 corrected in the note
## BEFORE commit; conditions 4–8 bind the scenario/engine packages.

The data, manifests, scripts and licence diligence are sound and
reproduce. One material defect: the package's own headline honesty item
(§3/§7.6 "direction conservative") is **backwards under the package's
own recommended convention** (§8.1) — corrected by condition 1 with
numbers supplied here.

## 1. Trace integrity — VERIFIED

- Both manifests verify in full (`shasum -c`): cf-gb2 481/481 OK,
  b6 21/21 OK (480 trace files + report; 12 raw + 9 processed).
- `derive_cf_gb2zone.py` genuinely IMPORTS `derive_cf.py` (module
  import, cluster constants and pipeline functions reused; no forked
  physics). `derive_cf.py` and all committed GB manifests byte-untouched
  (git diff empty; on-disk `gb_onshore_cf_2024.parquet` hash matches the
  committed manifest entry).
- Zone weight shares recomputed by the reviewer from the pinned cluster
  lists: offshore 3.2/15.3 = 0.209150, onshore 10.6/14.4 = 0.736111,
  solar 0.5/18.7 = 0.026738 — exact match; zone sets partition the
  pinned lists.
- Reconstruction identity independently reproduced for 4 years × 3
  techs (1990/2010/2021/2024): max per-period residual **2.4e-07**, max
  annual-energy residual **2.5e-08** relative — inside the claimed
  3.0e-07 / 1.1e-07 40-year maxima (report json maxima confirmed:
  2.98e-07 / 1.11e-07). Tolerance 1e-5 is evidence-based as stated.
- Determinism: reviewer re-ran `derive_cf_gb2zone.py --years 2024` —
  all 12 2024 outputs **byte-identical** (hash-diff empty). Reviewer
  re-ran `build.py` — all 9 processed b6 files byte-identical to the
  manifest. Env pins in requirements.txt match the venv exactly
  (Python 3.13.11, pandas 3.0.3, pyarrow 24.0.0, numpy 2.5.0).

## 2. The honesty flag — attribution SOUND, direction claim WRONG
## (condition 1)

The cross-check the note requested is done. DESNZ Regional Renewable
Statistics 2024 (OGL; Std-LFs + generation workbooks, fetched
2026-07-04):

- Observed 2024 standard load factors: **Scotland onshore 0.2684 >
  England 0.2490** (Wales 0.2535). The trace ordering is inverted
  (2024: sco 0.2917 < rgb 0.2956; 40y: 0.2865 < 0.3046) — the
  **weights artefact is real and confirmed**.
- BUT: under the note's own recommended §8.1 convention (split GB
  capacity by CLUSTER shares, onshore Scotland 73.6%), the model's
  Scottish share of GB onshore ENERGY is **73.4%** (2024) vs observed
  **69.8%** (DESNZ generation workbook 2024: Scotland share of GB
  onshore generation 0.6984, GB 32.0 TWh). The +3.6 pp capacity
  overweight DOMINATES the CF-ordering understatement: the delivered
  package **OVERSTATES Scottish onshore generation by ~+3.5 pp of
  share (~1.3 TWh at the reference 14.4 GW)** — it overstates B6
  export pressure and modelled curtailment, i.e. it is
  **anti-conservative for the Q2/Q10 bounding claims** (flatters the
  skeptical thesis). The note's "understates … conservative …
  flattering for nothing" holds only for the CF artefact in isolation,
  never for the delivered convention. §3, §7.6 and §8.1 must be
  corrected before commit.
- Measured fix available: splitting onshore by the observed DESNZ
  share (70.0%) with the same traces gives model Scottish energy share
  **69.7% ≈ observed 69.8%**, and the GB-energy wobble §8.1 warns
  about is negligible — GB onshore CF moves +0.05% (2024) / +0.22%
  (40-year means). The "exact GB reconstruction" argument therefore
  costs ~0.2% to give up for onshore.
- Offshore: cluster share 20.9% sits inside the REPD/DESNZ bracket
  (20.3–25.6%) — acceptable. The observed 2024 offshore ENERGY share
  is far lower (**14.7%**, generation workbook) — substantially the
  deliberate end-2024-fleet full-year convention (Moray West/NnG
  commissioning; model share 21.5%) plus DESNZ regional-assignment
  conventions (note 4). Up to ~3 TWh of 2024-specific Scottish
  offshore overstatement lands on the flow-validation anchors and must
  be quantified as a named wedge when tolerances are set (the note
  already defers tolerances — right call).
- Ruling on +3.6 pp: NOT acceptable as a silent default. The scenario
  package chooses once, explicitly, with these numbers in front of it:
  either DESNZ shares (recommended for onshore: matches observed zonal
  energy at ~0.2% GB cost) or cluster shares carrying the
  anti-conservative caveat verbatim on every Q2/Q10 output.

## 3. Fleet split — spot-verified against primaries (7 rows)

All recomputed from the raw files in the pack (independent code, not
build.py) or from station arithmetic:

| Row | Reviewer result | Match |
|---|---|---|
| Nuclear 20.2% | Torness ~1,190 MW of 5.9 GW ref = 0.2017; Hunterston B ceased 2022; rgb roster arithmetic sums to fleet | YES |
| CCGT 3.9% | Peterhead 1,180/30,000 = 3.93%; only Scottish transmission CCGT | YES |
| Biomass 0% Scotland | Ref roster (Drax/Lynemouth/MGT) all England; REPD Scotland "Biomass (dedicated)" 6.9% = embedded scale | YES |
| Hydro 88.8% | DESNZ MW2024 recomputed: 1,676/1,888 MW = 0.8881 | YES |
| Interconnectors | Raw NESO register: Moyle at Auchencrosh 275kV (Scotland; 475→500 MW import rows visible); **NSL at Blyth GSP (England)**; Viking Bicker Fen, EWIC Deeside, Greenlink Pembroke, rest southern England | YES |
| PS 26.2%/55.8% | 740/2,828 MW = 0.2617 (REPD confirms); 13.4/23.9 GWh | YES |
| Battery 14.6% | REPD 488/3,352 MW = 0.1455; share-only use of REPD is right given the Modo 4.7 GW basis gap | YES |
| On/offshore/solar | REPD onshore 0.7066, offshore 0.2030 (site list sums 2,980 MW: Seagreen 1,075/Moray East 950/Beatrice 588/…), solar 0.0087; DESNZ 0.6997/0.2561/0.0403 | YES |

REPD-withdrawal workaround (April-2026 extract, Operational-date
filter): **adequate** for REPD's corroborative role — the primary for
renewables is DESNZ MW2024, and 2025–26 decommissioning/repowering of
end-2024 sites is small. Two additions required (condition 5): (i) 26
Operational rows carry NO Operational date and are silently excluded
by the coerced filter — 610 MW GB (Scotland onshore 67 MW, England
battery 206 MW, England solar 296 MW), immaterial (<0.5 pp on any
share) but must be counted in the stated limitation; (ii) the §2
offshore parenthetical "NnG partial" is wrong — NnG contributes 0 MW
under the filter (the site list sums to the full 2,980 MW without it);
Energy Trends confirms NnG came online during 2024, reinforcing the
bracket point.

## 4. Demand split — VERIFIED; flat share adequate for v1

- Energy Trends special article (18 Dec 2025) PDF fetched and read:
  consumption shares England 81.2 / Scotland 9.8 / Wales 6.1 / NI 2.9
  (Chart 2B and text, verbatim), and the stability sentence is as
  quoted. GB-basis arithmetic 9.8/97.1 = 10.09% → 10.1% correct.
  The 17 TWh Scotland→England + 2.5 TWh→NI transfers are verbatim in
  the article.
- Flat 10.1% ruled adequate for v1: Scotland is ~10% of demand; its
  shape error is second-order against ~10 GW wind swings across B6,
  and the flow-validation gates would surface a material error. The
  three stated limits are honestly directional.
- P114 upgrade path: right recommendation; the BSC Open Data licence
  claim could NOT be machine-verified (Elexon blocks automated
  retrieval) — condition 7: verify the licence text at v2 fetch time
  before any assembly/redistribution.

## 5. B6 capability, flows, cross-anchor — VERIFIED; semantics wording
## conditioned

All 2024 statistics reproduced exactly from the processed pack
(independent recomputation): limit median 4,100 MW, IQR 2,700–5,500,
p95 6,350, p99 6,400; sentinels 53 (=0) / 116 (≥9,999); flow median
2,373 MW, p95 6,781, p99 8,960; negative share 8.22%; **net DA flow
22.627 TWh southward**; binding share **23.60%**; 2024 coverage
17,214/17,568 with 3 NaN rows; 2023 median 5,000. Nit (condition 6):
the note's "2025's 3,800 MW" recomputes as **3,850 MW**.

Costs reproduced exactly: calendar-2024 SCOTEX **£90.5m**, SSE-SP
**£366.8m**, SSHARN **£68.5m**, ESTEX £49.0m, SEIMP £4.7m, SWALEX
£0.04m; Scottish group **£525.8m**; six-boundary £579.5m of GB thermal
£1,482.5m (39.1%); thermal volume −11.02 TWh; voltage £201.6m/+4.45
TWh; inertia £43.1m.

**Flow semantics (load-bearing):** the NESO dataset/resource/field
documentation was refetched. It does NOT use the word "unconstrained";
it says the flows are "the forecast position after Day Ahead energy
scheduling", a "power flow forecast … based on the next day's wind
forecast, generation dispatch and demand forecast … modelled using
power system software". The package's pre-constraint-action reading is
**sound** — flows exceed limits in 23.6% of periods, impossible for a
constrained/settled series — but condition 3: the note, README and
build.py must cite NESO's actual wording and mark "unconstrained" as
the package's (well-supported) interpretation.

**ETYS 6.7 GW:** cited to a JS-rendered NESO page that cannot be
machine-verified; the observed limit envelope is consistent (p99
6,400 MW, no non-sentinel limit above it). Condition 4: before the
scenario package hard-codes 6.7 GW (sentinel replacement / upper
bound), pin the number to a fetchable artefact (ETYS appendix
workbook/PDF into the pack, or equivalent).

**Ruling on 22.6 vs 17 TWh: explained, not a red flag.** Reviewer
decomposition from the pack: clipping the DA flow at the DA limit
(non-sentinel periods) removes **3.51 TWh** → 19.12 TWh; the residual
~2.1 TWh vs the 17 TWh outturn is DA-forecast error + ledger
definitions + the 354 missing periods (~2% of the year). The two
anchors legitimately bracket the model; the validation design should
expect an irreducible wedge of order 2 TWh on gate (ii) from the
DA-vs-outturn basis alone.

## 6. The B4-dominance finding — link convention RULED
## (partially overrules §7.1 / §8.3–4; the engine work order cites this)

The B4 > B6 finding is verified (§5 above) and the surprise is real: a
two-zone model cannot represent intra-Scottish congestion. But the
note's "tune the effective link to the GROUP" recommendation is
rejected — it double-books B4/B5 congestion onto the border and breaks
every flow anchor. **The convention:**

(a) **The link is B6, not a group aggregate.** 2024 validation run:
    export capability = the observed half-hourly DA limit series.
    Sentinel rule: limit ≥ 9,999 → replace with the ETYS planning
    value (6.7 GW, subject to condition 4); limit = 0 → treat the
    period's capability as missing and exclude it from gate
    arithmetic unless corroborated as a real outage. Missing periods
    stay missing and are masked out of gates. Import capability =
    3.5 GW flat (superseded HARETORIM series, vintage stated).
    Non-2024 / scaled / 40-year runs: export 4.1 GW central (2024
    median), sensitivity brackets 2.7 / 5.5 GW (IQR) and 6.7 GW (ETYS
    upper bound); import 3.5 GW. No synthesised limit time-series for
    non-2024 years (outage-driven, no published basis) — agreed.

(b) **Validation gates are B6-specific and require configuration (a):**
    (i) modelled pre-constraint boundary flow vs the DA flow series —
    correlation + net ≈ 22.6 TWh computed over the same 17,214-period
    mask; (ii) modelled constrained export vs 17 TWh (Energy Trends),
    carrying the ~2 TWh DA-vs-outturn wedge named above; (iii) binding
    frequency vs 23.6%. Tolerances pinned only after first runs
    quantify the wedges (incl. the §2 zonal-energy wedges) — the
    note's deferral stands.

(c) **Costs and curtailment: B6-only is the like-for-like anchor
    (£90.5m); the Scottish group (£525.8m) is reported alongside as
    the full size of the phenomenon the model structurally cannot
    see — never as the model's target.** Model constraint/curtailment
    outputs are quoted as a LOWER BOUND on the Scottish constraint
    phenomenon, with the B4/B5 invisibility stated. The link
    capability must never be tuned to reproduce the group cost in the
    validation configuration. A group-effective tighter-link variant
    for the Q2/Q10 bounding study is permitted only as a separately
    labelled sensitivity that claims NO flow-gate validity.

(d) **Schema fact for the work order:** the current `[[links]]` schema
    (gb-2024-5zone.toml) carries a single symmetric `capacity_gw` +
    availability + loss. Convention (a) needs per-direction and
    time-series capability → a `schema_version` bump + docs/03
    migration note in the scenario/engine package.

## 7. Licences — VERIFIED

All four NESO datasets' CKAN metadata refetched: `license_title =
"NESO Open Data Licence"` on each (day-ahead flows/limits, thermal
constraint costs, constraint breakdown, interconnector register).
DESNZ Energy Trends + Regional Renewable Statistics + REPD are gov.uk
OGL v3.0 publications. Nothing non-open is redistributed: data is
gitignored; committed manifests carry hashes only (and the underlying
licences permit redistribution with attribution anyway). Modo: only
the pre-existing 4.7 GW reference-scenario citation is carried; no
Modo data fetched or re-derived. Attribution strings correct in note
§8.5 and the README. P114: see condition 7.

## 8. Scope and conventions — PASS

Tracked-file changes: the era5-cf README append only (diff read; claims
match the verified numbers). New files: 2 manifests + note + scripts.
No engine code, no scenario files, no memory/ or docs/04 edits. GB
derivation path byte-unchanged. Q5 files present in the tree belong to
the concurrent implementer and were not reviewed here; cargo gates are
not applicable to this package (no Rust) and were not run against the
mixed tree. No panics/determinism concerns (scripts are pure functions
of pinned inputs; loud failures on partition/duplicate-UTC/residual
breaches — the reconstruction check is the characterisation test, per
data-package precedent). Retrieval-date pinning of the rolling NESO
file is the correct manifest posture.

## Conditions

1. **(Pre-commit)** Correct §3, §7.6, §8.1: under the recommended
   cluster-share split the package OVERSTATES Scottish onshore energy
   share (+3.5 pp vs observed 2024; model 73.4% vs observed 69.8%) —
   anti-conservative for Q2/Q10. The "conservative" direction holds
   only for the CF-ordering artefact in isolation. Include the
   measured alternative: DESNZ 70.0% split → 69.7% vs observed 69.8%,
   GB-energy cost +0.05% (2024) / +0.22% (40y).
2. **(Pre-commit)** §8.1 rewritten to present the split choice with
   these numbers; if cluster shares are kept, the anti-conservative
   caveat travels verbatim with every Q2/Q10 output.
3. **(Pre-commit)** Flow-semantics wording: quote NESO's actual
   language ("forecast position after Day Ahead energy scheduling");
   mark "unconstrained" as interpretation supported by flow>limit in
   23.6% of periods. Note §6 + fetch-b6 README + build.py docstring.
4. **(Scenario package)** Pin ETYS 6.7 GW (and the 5.1/5.8 GW overload
   context) to a fetchable artefact before hard-coding.
5. **(Pre-commit)** REPD limitation additions: 26 undated Operational
   rows (610 MW GB) excluded; "NnG partial" corrected to zero
   contribution under the filter.
6. **(Pre-commit, nit)** 2025 limit median 3,800 → 3,850 MW.
7. **(v2 gate)** Verify the Elexon P114 BSC Open Data licence text
   before any P114 fetch/assembly.
8. **(Engine/scenario work order)** Implement the §6 link convention
   above, including the schema_version bump for per-direction/
   time-series link capability; validation gates (i)–(iii) B6-only;
   group cost as context, never target.
