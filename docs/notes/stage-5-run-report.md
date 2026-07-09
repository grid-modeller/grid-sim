# Stage 5 — multi-zone + interconnectors: results

Committed record of the Stage 5 acceptance runs (2026-07-03), at the
final configuration: D5 five-zone set, w=1 FR release envelope, CBS
NL recalibration. Adversarial review + escalation adjudication:
`docs/notes/stage-5-review.md` (ACCEPT-WITH-NOTES; the A2 re-pin was
reviewer-adjudicated after both remediation options were measured —
never supervisor- or implementer-initiated). Suite at closure:
**370/0**, fmt/clippy clean, all prior-stage pins unmoved (dispatch
digest `779d7444…`; the direct `run_multi`-on-reference pin).

## 1. Acceptance gates at the final configuration

| Gate | Pin | Result |
|---|---|---|
| A1 gas annual | ±5 % | 71.80 TWh, **−1.36 % PASS** |
| A1 monthly mix corr | ≥ 0.99 | **PASS** |
| A1 GB net imports | ±10 % of 33.30 TWh | **+35.94 TWh, +7.9 % PASS** (drift §4) |
| A2a GB↔FR direction match | ≥ 88 % | **90.07 %** (15,823/17,568) |
| A2b GB→FR export recall | ≥ 70 % | **78.96 %** (1,036/1,312) |
| A3 NO2 hydro vs GB wind | \|r\| ≤ 0.15 | **−0.057 PASS** |
| A3 continental imports vs GB wind | r ≤ −0.25 | **−0.342 PASS** |
| A3 NSL flow (diagnostic, not gated) | ±0.15 of −0.399 | −0.269, in band |
| A4 BE annual net | ±1.5 TWh of +4.16 | **+2.82, −1.34 PASS** |
| A4 NL annual net | ±1.5 TWh of +1.59 | **+2.82, +1.23 PASS** |

Five-border table (modelled vs NESO, TWh net import to GB): FR +20.66
(+1.21), BE +2.82 (−1.34), NL +2.82 (+1.23), NO2 +9.63 (+0.01), DK1
+2.53 (−1.13), IE −2.52 (+2.66); total +35.94 vs +33.30. The worst
border is IE-SEM (+2.66 on a −5.18 observed export) — the crudest
zone (one aggregate island market, flat availabilities); recorded,
not gate-relevant (no IE gate was pinned).

> **CORRECTION (2026-07-06, R7 flow-walk stall fix — docs/08 R7,
> RESOLVED; adjudication `docs/notes/r7-fix-review.md`).** The pre-fix
> engine silently cap-truncated stalled flow walks (committed Stage 5
> behaviour, disclosed in the D11 sweep review §B.4). Re-measured on
> the fixed engine: **A1 gas 71.80 → 71.70 TWh (−1.36 % → −1.50 %)**
> and **A1 GB net imports +35.94 → +36.03 TWh (+7.9 % → +8.19 %)** —
> both still PASS their bands. The five-border table values move with
> the re-pinned 5-zone dispatch digests (old/new digests recorded in
> `grid-cli/tests/regression_5zone.rs`; the anchor A1 pair is
> re-pinned in `acceptance_d11_sweep.rs` with the old values
> recorded). **What did NOT move:** the A2 record is literally
> bit-unmoved — the exact A2a/A2b count pins (15,823 and 1,036) in
> the unmodified `acceptance_stage5_2024.rs` pass on the fixed
> engine, and the A3/A4 gates pass unchanged; the §2 re-pin history
> and publication rules are untouched.

## 2. THE A2 RE-PIN — the stage's central record (kill-criterion
## prominence; read before quoting ANY Stage 5 direction number)

The original pre-model pin — GB↔FR direction match ≥ 95 % — was
**re-pinned to a two-limb gate after a reviewer-adjudicated
escalation**, not relaxed quietly. The full sequence, in order:

1. **First measurement: 82.19 %** — below the 92.30 % "always import"
   base rate. Diagnosis: flat FR identity wedge + flat FR hydro
   availability overstate FR peak-hour scarcity; the model exported
   to France at French demand peaks.
2. **Remediation round i** (observed FR hydro budget, weekly grain
   mirroring NO2): **82.29 %** — and the honest headline is that the
   weekly grain later proved WORSE than the flat model it replaced
   (80.60 % in round ii): the greedy zero-foresight release
   front-loads each window (model [8.25, 8.22, 4.68, 1.83, 0.87,
   0.25, 0.26] TWh/day vs observed ~3.7 flat), so days 3–6 of every
   week reverted to the defect. **Weekly was REJECTED for FR** by the
   adjudication; this negative result is part of the record.
3. **Remediation round ii** (observed FR non-GB export series —
   demand-conditioned: at FR load p99, France flips to net importer;
   the flat wedge was ~8 GW wrong exactly at FR peaks): the
   mechanism-(a) mismatch categories fell to ZERO (FR-past-its-gas 0,
   FR unserved 0.000 TWh). Ceiling reached: **90.07 %** at the w=1
   envelope.
4. **The measured residual boundary**: 1,297 mismatches remain;
   **1,122 (86.5 %) are both-zones-gas-marginal periods**, 555 of
   them in 23–05 UTC (night/shoulder) — hours where real FR prices
   sit below GB's (nuclear-adjacent economics) but the deliberately
   unpriced scarcity ladder ranks a 12.8 GW gas fleet's fractional
   utilisation above a 31 GW one's. This is mechanism (b), the
   grid-adequacy `flow` module's named-at-design limitation. Measured
   expectation if that class is priced away: **97.4 %**.
5. **The re-pinned gate** (docs/04, history retained): A2a match
   ≥ 88 % AND A2b export recall ≥ 70 %. A2a alone is below the
   always-import base rate — stated openly; the pair strictly
   dominates it because always-import scores **0 %** on recall. The
   superseded ≥ 95 % stays in docs/04 as the priced-ladder target
   (Stage 7-adjacent).

**Publication rule:** never quote "90 % direction match" without the
two-limb structure and the base-rate sentence; never quote the
crossing of the old 95 % target as achieved; the priced-ladder
deferral is part of any quoted A2 number.

## 3. Circularity inventory (what is observation-anchored vs emergent)

Observation-anchored inputs (boundary conditions, disclosed):
- External-zone demand: observed 2024 load traces (all five zones).
- FR hydro: observed cumulative release ENVELOPE (w=1) — modelled
  release can never run ahead of observed cumulative; timing below
  the envelope is scarcity-driven. Ownership: the top-bin FR
  capacity-credit numbers reflect observed 2024 FR hydro
  availability — correct for a 2024 estimate, stated openly.
- FR non-GB exports, FR pumping consumption: observed traces.
- NO2 reservoir + pumped budgets: observed weekly A75/A72 energies.
- Identity wedges: per-zone flat closures of the 2024 annual balance,
  itemised in the scenario. **The wedge construction uses each zone's
  observed annual border net — so the A1 imports AGGREGATE is
  partially anchored by construction** (reviewer-measured leverage:
  ±1 GW FR wedge → ∓1.6–1.8 TWh GB imports). A1's information
  content is therefore the gas/mix gates plus the imports staying
  in-band as the wedges shrank (7.537 → 0.413 GW for FR); A4's is
  the per-border ALLOCATION, which is emergent.
- CF traces: weather physics calibrated one-factor-per-tech to annual
  national statistics (14/16 in-band post-CBS).

Emergent (the model's actual claims): GB dispatch (Stage 1 gates
re-passed with MODELLED imports), every half-hourly flow on every
link, the direction structure (A2), the correlation structure (A3),
the per-border energy allocation (A4), storage behaviour, Module 5.

## 4. Disclosures

- **A1 imports drift**: +32.80 TWh (−1.5 %) at round ii → **+35.94
  (+7.9 %)** at the final configuration — the observed-envelope FR
  (cheap and available when GB is tight) pulls more GB imports.
  Inside the ±10 % gate; direction and size disclosed. FR border
  +1.21 TWh vs NESO.
- **Wedge reconciliation (review condition)**: the committed round-0
  scenario carried FR `extra_demand_gw = +7.439`; the round-i report
  quoted +7.537 — the difference is the two observed pumping legs
  moving from wedge to model (pumping demand 6.06 TWh; budget gen).
  Final FR wedge: **+0.413 GW** (+3.63 TWh = net cross-source
  reconciliation between A65 load perimeter, A75 transmission
  metering and border metering ends; composition in the TOML).
- **CONT-NW wedge** recomputed under its own documented rule at the
  CBS point: −10.116 → **−10.704 GW** (NL trace energies fell to the
  CBS-anchored 17.66/21.82 TWh). Still the largest disclosed block
  (71 TWh unrepresented CHP/'other' + transit); the D5 ruling-c
  revisit trigger (BE/NL tolerance misses) did NOT fire at the
  calibrated point.
- **NL CBS recalibration record**: ENTSO-E A75 captured 43.3 % of
  CBS onshore and 2.2 % of CBS solar generation (distributed
  generation under-reporting); factors now 0.8975/0.8735, in-band;
  14/16 calibrated. Two findings of record: the old bias-bracket end
  (×0.78) was built on a wrong uncited magnitude (the true CBS net is
  17.66 TWh, not ~15.5); and the derivation note's NL–DK1 correlation
  cell was a transcription error (true .525/.596), caught in the
  same pass. 2024 CBS rows are "NaderVoorlopig" (revised
  provisional) — recorded on every row.
- **DK1 onshore factor 0.7441** remains a band-edge keep (physical
  old-stock/low-hub explanation; shape r 0.985) — carried wherever
  DK1 numbers appear.

## 5. Module 5 — interconnector capacity credit (embargo lifted on
## this cut, with caveats)

Mean net import by GB residual-demand percentile (20 equal-count
bins), final configuration:

| Link (zone) | Mid-bins | Top bin (p95+) |
|---|---|---|
| FR (IFA+IFA2+ElecLink) | 2.6–2.8 GW | **2.70 GW** |
| CONT-NW (Nemo+BritNed) | ~1.6 | **1.11 GW** |
| NO2 (NSL) | 1.22–1.25 | **1.22 GW — flat across ALL bins** |
| DK1 (Viking) | ~0.3 | **−0.57 GW (exports at GB's tightest)** |
| IE (EWIC+Moyle+Greenlink) | ~−0.4 | +0.17 GW |

The anticyclone story, measured: NO2's hydro-backed credit is flat —
the only counterparty whose availability does not degrade as GB
tightens; the continental bloc's credit falls at the top bins; DK1
flips to export (GB scarcity coincides with continental scarcity
pulling through Jutland). **Caveats bound to the artefact:** (i) the
mechanism-(b) direction residual lives at night/shoulder (23–05
UTC) — top-2-bin contamination measured at 35/1,757 = 2.0 %, so the
tight-evening story is essentially clean; (ii) FR's top-bin credit
reflects observed 2024 FR hydro availability (envelope convention,
§3) — a 2024 estimate, not a general law; (iii) DK1's flip carries
the commissioning-year caveat (Viking live Dec 2023). Regenerate
artefacts post-commit for clean hashes.

## 6. Publication rules (standing)

1. A2 numbers only with the §2 two-limb + base-rate framing.
2. Per-border publication-grade GB flow numbers cite the Elexon/NESO
   pack (licence posture: ENTSO-E is the neighbour-side cross-check).
3. Module 5 numbers carry §5's three caveats; NO2-flat is quotable as
   the headline with the hydro-resource-level framing from A3.
4. Nothing from the IE-SEM zone beyond the disclosed error band.
5. The circularity inventory (§3) accompanies any claim that Stage 5
   "re-validates 2024 with modelled imports".
6. Prior-stage rules (Stages 1–4, 6) continue to bind.
