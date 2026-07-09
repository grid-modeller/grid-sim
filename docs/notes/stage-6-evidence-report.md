# Stage 6 Evidence Pack — 9 August 2019 Event and Inertia Constants

Assembled 2026-07-03 per `docs/05-validation.md` discipline (same pattern as
`docs/notes/2024-validation-pack-report.md` §7): quantify what the published
record can and cannot support **before** a tolerance is pinned. Evidence
only; the Stage 6 `TBD-DATA` number is the supervisor's call.

Reference files (committed, per-number citations inside):
- `data/reference/stability-2019-event.toml` — the event record
- `data/reference/inertia-constants.toml` — per-technology H

Primary sources (URLs, retrieval date 2026-07-03, sha256 in the TOMLs): NG
ESO *Technical Report on the events of 9 August 2019* (final, 6 Sep 2019) +
Appendices; Ofgem *9 August 2019 power outage report* (Jan 2020); E3C final
report (Jan 2020, OGL v3); NESO 1-second System Frequency data (NESO Open
Data Licence); NESO FRCR 2024.

## 1. The event in numbers (the acceptance anchors)

| Quantity | Value | Source |
|---|---|---|
| Pre-event demand / tx generation | 29 GW / ~32 GW available | ESO p.10, Table 4 p.24 |
| Pre-event system inertia | 210 GVA·s (Table 4); 219.632 GVA·s (Appendix M Q42) | ESO p.24; appendices Q42 |
| Largest secured infeed | 1,000 MW (interconnector) | ESO p.13; appendices App. H |
| Response held | 1,022 MW primary + 1,314 MW secondary (incl. 472 MW battery) | ESO pp.4, 25 |
| Losses (staged) | Hornsea 737; L. Barford ST1C 244 + GT1A 210 + GT1B 187 (=641); embedded vector-shift ~150 + RoCoF ~350; +200 net @49 Hz | ESO Table 2 p.14, p.22 |
| Total loss | 1,878 MW (ESO Table 2, infeed trips) / ≥1,990 MW (Ofgem 2.4.11) ≈ **1.9× the secured loss** | |
| Frequency | 50.0 → arrest 49.1 (t+25 s) → plateau 49.2 → 48.8 LFDD at 16:53:49.398 BST (t+75.9 s) → 50.0 by 16:57:15 | ESO timeline pp.13–14 |
| Measured nadir | **48.787 Hz** (NESO 1-s data, 16:53:49 BST; single sample < 48.8) | computed, this pack |
| Measured initial RoCoF | 0.144 Hz/s (2-s mean), 0.151 Hz/s (max 1-s) | computed, this pack |
| LFDD | Stage 1 (48.8 Hz): 931 MW DNO-reported (3.2 %) / 892 MW Ofgem net / only ~350 MW net seen at transmission; ~1.1 m customers; restored by 17:37 | ESO p.22; Ofgem 2.4.12–13 |

Sequence correction worth flagging: early accounts gave the Little Barford
steam turbine as ~640 MW. The final record is ST1C = 244 MW at t+1 s, with
Little Barford's *total* staged loss 641 MW (244+210+187, Ofgem 3.27).

## 2. What the record pins precisely

- The four transmission trips: magnitude to the MW, timing to the second
  (Hornsea to the ms). These are model *inputs*, not things Stage 6 predicts.
- System inertia per timestamp through the event (Appendix M Q42:
  219.6 → 212.4 GVA·s) — the exact input the engine's `Σ(H×MVA)` replaces.
- The frequency trajectory at 1-s / 1-mHz resolution (NESO open data, so the
  fixture trace itself is fetchable and redistributable with attribution).
- A bracket on the nadir *from protection physics*: LFDD stage 1 (48.80 Hz)
  operated; stage 2 (48.75 Hz) did not. The true nadir is inside
  **(48.75, 48.80]**; measured 48.787.
- The counterfactual: ESO's own simulation of a 1,000 MW loss under the same
  conditions stays ≥ 49.5 Hz, corroborated by the 1 July 2019 outturn (NEMO
  tripped 1,000 MW at 201 GVA·s; frequency held above 49.5).

## 3. Irreducible ambiguities (what bounds the tolerance)

1. **Embedded loss-of-mains volume.** ESO: 150 (vector shift) + 350 (RoCoF).
   Ofgem: 350–430 MW RoCoF, total LoM "up to 580MW", inferred from a 500 MW
   transformer-loading step — nobody measured it (DNOs do not monitor DG in
   real time). Ambiguity ≈ ±80–130 MW on the driving imbalance.
2. **The 200 MW at 49 Hz** is *back-inferred by the ESO from this same
   frequency trace* ("modelled as a 200MW loss", ESO p.22). Using it as an
   input makes the 49.2 → 48.8 segment partially circular — the same class
   of caveat as the NESO embedded-solar estimate in D3.
3. **Official inertia disagrees with itself by ~5 %**: 210 GVA·s (Table 4)
   vs 219.632 GVA·s (Appendix M Q42) — and it is a model estimate, not a
   measurement ("The ESO does not currently measure system inertia
   directly", Q42).
4. **Response delivery**: of validated holdings, 89 % of primary and 88 % of
   secondary delivered to contract — ±100 MW-scale uncertainty in delivered
   response.
5. **Load damping** is not published for the day; the literature span
   (1–2.5 %/Hz ⇒ ~290–725 MW/Hz at 29 GW) is a first-order free parameter.
6. **LFDD accounting is threefold** (931 DNO / 892 net / ~350 at
   transmission, the gap being ~550 MW of coincident embedded loss that "the
   reasons for ... need to be better understood", Ofgem 2.4.13) — recovery
   after the nadir is materially less well determined than the descent.
7. **Trace granularity**: 1-s sampling with the frequency falling
   ~0.03 Hz/s at the trigger ⇒ the sampled nadir is within ~0.03 Hz of the
   instantaneous extremum; LFDD relay + breaker action (~200 ms to recross
   48.8, Ofgem 2.4.14) sets the depth below 48.8.

## 4. What a single-bus swing model can and cannot capture

Can: aggregate RoCoF from Σ(H×MVA); the arrest at 49.1 (inertia + primary
response vs the 1,481 MW cumulative loss); staged trips as timed exogenous
inputs; response services with ramp/delay envelopes (primary: full by 10 s,
sustain 30 s; secondary: by 30 s, sustain 30 min — ESO appendices); LFDD as
staged demand blocks; the 1,000 MW counterfactual.

Cannot (and should not pretend to): *why* Hornsea deloaded (converter
control instability — an EMT phenomenon, out of scope per ADR-2); vector
shift at the fault (input, not physics); regional distribution effects; the
hidden ~550 MW embedded loss coincident with LFDD; frequency measurement
spatial spread during the transient.

Decisive structural point: **the nadir of this event is an LFDD
interception, not a free swing minimum.** Frequency was falling ~0.03 Hz/s
when stage 1 tripped; the depth below 48.80 (13 mHz measured) is set almost
entirely by the LFDD action delay, not by inertia or response. Any model
that (a) breaches 48.8 and (b) implements stage-1 LFDD with a 0.2–0.5 s
action delay lands at ~48.78–48.80 automatically. A nadir-only tolerance is
therefore a weak test — it mostly validates the LFDD implementation. The
physics-discriminating anchors are the *first arrest* and the *initial
RoCoF*.

## 5. Recommended tolerance frame (evidence-based, for the supervisor to pin)

Given the published inputs (trips as timed injections; inertia 210–219.6
GVA·s; response holdings with published delivery factors; LoM losses at ESO
central values):

- **T1 — nadir (the docs/04 `TBD-DATA`): within the LFDD stage-1 band,
  48.75 < f_min ≤ 48.80 Hz** (measured 48.787). Equivalent to ±~0.04 Hz,
  but stated as the band because that is what the record actually pins:
  stage 1 tripped, stage 2 did not. Tighter is unsupportable (relay-delay
  dominated); looser crosses a protection boundary and is physically wrong.
- **T2 — first arrest: 49.10 ± 0.10 Hz** (measured minimum 49.083). This is
  the real physics test. Justification: near the arrest the system
  stiffness is ≈ primary-response slope (~1,022 MW/0.5 Hz ≈ 2.0 GW/Hz) plus
  damping (0.3–0.7 GW/Hz) ⇒ ~2.3–2.7 GW/Hz; the ±80–130 MW LoM-input
  ambiguity (§3.1) alone maps to ~0.03–0.06 Hz (130 MW / 2.3 GW/Hz =
  0.057 Hz). Adding the ±~100 MW delivered-response uncertainty (§3.4)
  linearly gives a combined worst case of ~230 MW ⇒ ~0.10 Hz at the softer
  2.3 GW/Hz stiffness bound. ±0.10 Hz equals that combined worst case —
  the D2 pattern (set just outside the irreducible input ambiguity).
- **T3 — initial RoCoF: within ±25 % of 0.145 Hz/s measured over a pinned
  2-s window from the first trip** (i.e. 0.11–0.18 Hz/s). Spread budget:
  window definition (0.144 vs 0.151 vs 0.065 over 2 s/1 s/10 s — pin the
  window), ±5 % official-inertia self-disagreement, sub-second loss
  staging, damping. Sanity: f₀·ΔP/2E gives 0.129 (1,131 MW) to 0.169
  (1,481 MW) Hz/s at 219.6 GVA·s — the measurement sits inside.
- **T4 — counterfactual (binary): 1,000 MW loss under identical conditions
  never drops below 49.5 Hz.** Anchored to ESO's published simulation and
  the 1 July 2019 outturn. This is the "security standard was adequate for
  the secured loss" pin and costs nothing.
- **Diagnostic only, no tolerance: time from fault to LFDD (75.9 s) and the
  recovery path.** The descent 49.2 → 48.8 depends on the trace-circular
  200 MW @49 Hz input (§3.2) and secondary-ramp detail; the recovery
  depends on the threefold LFDD accounting (§3.6). Report, don't gate.

Not recommended: ±0.05 Hz on nadir (fits relay noise, implies false
precision); any tolerance on absolute time-below-49 Hz (damping-dominated);
validating against the ESO simulation trace instead of the measured trace
(model-vs-model).

## 6. Inertia constants — summary of `inertia-constants.toml`

| Technology | H central (s) | Range (s) | Sync | Primary evidence |
|---|---|---|---|---|
| CCGT | 5.0 | 4.0–6.0 | yes | FG19 Table 1; ERCOT18 Table 1; KRA22 GB 3–10 s |
| OCGT | 4.0 | 2.5–6.0 | yes | ERCOT18 CT 1–12.5 s (low confidence) |
| Nuclear | 4.5 | 3.5–7.0 | yes | ERCOT18 3.8–4.34; FG19 4 s; Kundur 4-pole 4–10 |
| Coal / biomass steam | 4.0 | 2.9–5.0 | yes | ERCOT18 2.9–4.5; FG19 3.3–4 |
| Hydro | 3.0 | 2.0–4.0 | yes | Kundur/FG19; KRA22: GB small hydro higher |
| Pumped storage | 4.5 | 2.5–5.0 | yes | KRA22/Stability Pathfinder ⇒ ~4.8 s derived |
| Wind / solar / battery / interconnector | 0 | — | no | inverter/HVDC-coupled (ERCOT18) |

Caveats carried in the file: H is per **MVA** while docs/03 capacities are
GW — the MW/MVA (power-factor ~0.9) convention must be decided at Stage 6
design time; no NESO per-technology H publication exists (per-unit GB
values are not public — KRA22's reconstruction is the closest evidence);
pumped storage contributes inertia while pumping; synthetic inertia is a
response service, never an H. The firm/variable ↔ synchronous mapping and
its failure cases (pumped storage, batteries, imports) are documented in
the TOML against `docs/notes/reliability-classification.md`.

## 7. Licence and source-availability report (D1 discipline)

- NESO 1-s frequency data: **NESO Open Data Licence** — reuse and
  redistribution with attribution; safe for a fetched fixture trace and for
  published charts.
- E3C reports: **OGL v3.0** — freely quotable/redistributable.
- Ofgem and NG ESO reports: public documents, quoted with attribution and
  page references; PDFs not committed to the repo (sha256 recorded for
  verification).
- ERCOT/arXiv sources for H constants: public; citation only.
- Nothing paywalled was relied on. The IET journal version of FG19 is
  paywalled; the arXiv postprint (same Table 1) is used and labelled.

## 8. Flags for the supervisor (not actioned — outside this task's remit)

1. `docs/04-implementation-plan.md` Stage 6 says "limits (1 Hz/s, **49.2 Hz
   LFDD**)". Per the primary record: LFDD stage 1 is **48.8 Hz**; 49.2 Hz is
   the SQSS infrequent-infeed-loss floor (FRCR 2024 impact level L1). The
   RoCoF number is also date-dependent: 0.125 Hz/s relay limit at the 2019
   event; 1 Hz/s + 500 ms relays post-ALoMCP (deadline 31 Aug 2022); NESO
   *designs* to 0.5 Hz/s at the 120 GVA·s (→102) inertia floor (FRCR 2024
   p.10). Scenario-era limits should be a scenario input, not a constant.
2. The H-per-MVA vs capacity-per-GW convention (§6) needs a decision before
   the `Σ(H×MVA)` acceptance test is written.
3. If the Stage 6 fixture wants the raw 1-s trace committed rather than
   fetched, the NESO Open Data Licence permits it (attribution required);
   the event-window extract is ~500 rows.
