# D5 — Continental zone granularity (resolved 2026-07-03, supervisor
# draft under Richard's standing delegation; reviewer ADOPT-WITH-EDITS,
# all six edits applied — see Review at the foot)

## The decision

**Five external zones** for the Stage 5 engine activation:

| Zone | Contents | GB links (modelled capacity) | Role |
|---|---|---|---|
| FR | France mainland | IFA + IFA2 + ElecLink (combined, per the platform's border) | Largest border; the docs/04 GB↔FR direction gate lives here |
| CONT-NW | BE + NL + DE-LU aggregated | Nemo (BE leg), BritNed (NL leg) | The continental bloc; DE-LU is the hinterland depth |
| NO2 | Norway bidding zone NO2 | NSL | Hydro-driven counterparty; the uncorrelated-resource test |
| DK1 | Denmark west | Viking | Wind-heavy, distinct behaviour (flow r ≈ 0 vs GB wind) |
| IE-SEM | Ireland + NI single market | EWIC + Moyle + Greenlink (combined) | Predominantly a GB-export sink (GB exports 87.7 % of periods) |

This resolves the docs/08 frame ("3 vs 4 zones: FR, NO mandatory;
NL/BE/DE aggregate vs split") as: **aggregate NL/BE/DE**, plus two
zones the original frame predates — DK1 (Viking commissioned Dec 2023)
and IE-SEM — without which the Stage 5 acceptance identity (modelled
imports vs 33.30 TWh net actual) cannot close: on NESO per-border
figures the four-border sum is +34.82 TWh vs +33.30 all-borders — a
+4.6 % error against a tolerance of the Stage 1 ±1 % order
(reviewer-verified arithmetic; NESO IE −5.18, DK1 +3.66 TWh — NESO
figures used because the identity is NESO-referenced, pack report
§9.3; the ENTSO-E per-border values stand as cross-check only).

**This decision proposes an ADR-7 zone-list amendment** (ADR-7 names
"FR, NO, NL/BE/DE"; this note delivers five zones and swaps NO for
NO2). Recorded in `memory/project-state.md` per CLAUDE.md — the ADR is
not edited. The docs/04 Stage 5 scope line is amended in the same
work-order pinning as the already-planned sign-test reformulation.

## Evidence (all from committed, reviewed packs; every number
## reviewer-checked against source)

1. **NO2, not NO-aggregate** (`docs/notes/entsoe-stage5-pack-report.md`
   §6–7): NO2 carries all of NSL, 40 % of Norwegian **reservoir-hydro**
   energy (36.6 % of all hydro), 44 % of reservoir storage, and has its
   own load trace (36.1 TWh). The GB-relevant scarcity signal is NO2's.
   This also dissolves the weather-pack NO4 coverage gap
   (`docs/notes/eu-pack-box-review.md`): the Norwegian zone is
   hydro-driven from ENTSO-E data (reservoir + generation-per-type +
   inflow proxy), not weather-derived.
   **NO2 wind (quantified, not hidden):** 4.52 TWh generated on
   1.45 GW in 2024 — 12.5 % of NO2 load and **47 % of the NSL net
   flow**. v1 carries no NO2 wind CF derivation; the energy is
   **absorbed in the NO2 energy balance at calibration** (netted off
   load or carried as a flat/derived trace — engine work-order
   condition), never silently dropped.
2. **NL/BE/DE aggregate, not split.** What carries the aggregation
   case: DE-LU has no GB border (Viking lands in DK1), so splitting DE
   out buys intra-continental network modelling that ADR-12 excludes;
   DE-LU is 70 % of the bloc's load (470.4 of 666.5 TWh) — the
   hinterland that actually sets BE/NL scarcity; and the stakes are
   capped at ~1 GW per link (Nemo, BritNed). The per-border
   correlations with GB wind are honestly different (BE −0.241/−0.321,
   NL −0.058/−0.026 half-hourly/daily) — the aggregation is a
   scope-control decision, not a "they behave identically" claim.
   **Validation consequence (structural, reviewer-ruled):** Nemo and
   BritNed terminate in the same zone, so a scarcity-driven flow rule
   gives both links the same sign of price differential at every
   timestep — while observed 2024 has *opposite* net signs on the two
   borders in ≥13.57 % of periods. Per-border flows therefore emerge
   as separate link series and validate against per-border **annual
   net energies** (NESO: BE +4.16, NL +1.59 TWh); per-border
   **direction rates cannot be separate acceptance gates** on these
   two borders. The direction gate stays GB↔FR-only per docs/04;
   bloc-level (Nemo+BritNed combined) direction is an optional
   diagnostic.
3. **FR separate from the bloc**: the docs/04 acceptance gate is
   GB↔FR-specific; FR's base rate (92.30 % import) and its
   nuclear-dominated fleet (heatwave-curtailment asymmetry, Module 5)
   make it behaviourally distinct.
4. **DK1 separate**: its flow shows r ≈ 0 vs GB wind (+0.002/+0.096)
   while the BE border shows −0.24 — folding Viking into CONT-NW would
   misstate the one border that behaves uncorrelated for reasons other
   than hydro (commissioning-year caveat recorded in the pack report).
5. **IE-SEM as one zone** matches both the market and the platform's
   border definition (EWIC + Moyle + Greenlink combined; per-asset
   series do not exist on the platform).

## Consequences for the Stage 5 data plan

- Weather-derived CF traces for FR, BE, NL, DE, DK1, IE (wind
  on/offshore + solar) plus temperature — derived **per-country** from
  the EU pack and aggregated to CONT-NW at scenario level, so the bloc
  decision can be revisited without re-derivation. Caveat (reviewer):
  the hedge covers wind/solar/temperature only — NO2 wind has no
  derivation, so revising THAT choice would need new derivation work.
- Calibration anchor: ENTSO-E 2024 actual generation per type per zone
  (internal use under TP GTC cl. 3.1), mirroring the GB approach.
- NO2 needs no CF derivation; its hydro dispatch model consumes the
  ENTSO-E reservoir/generation evidence (inflow proxy is
  weekly-budget-grade, pack report §6).
- Fleet tables per zone from the pack's capacity_2024, lossy PSR
  mappings carried as documented conventions; B11/B12 stay split for
  NO2. **IE fossil_oil 1.59 GW is material for a ~4.7 GW-average-load
  zone: map to a peaker technology or document a capacity-margin
  justification before the IE fleet table is built — do not silently
  drop** (reviewer ruling e). DE 'other' blocks: quantify at
  fleet-table time; immaterial provided calibration is to observed
  per-type annual energies.

## Adjudicated conditions on the Stage 5 work order (reviewer rulings)

(a) No per-border BE/NL direction gate may be pinned unless the
    observed BE↔NL direction-agreement rate is first computed from the
    pack and the gate set below the structural cap. Per-border
    validation on these borders is annual-energy only.
(b) NO2 hydro+load-only is defensible for the sign test as reformulated
    (resource-level), conditional on the NO2-wind energy-balance
    treatment above. NOTE: the engine-side sign-test limb "modelled NO2
    hydro generation uncorrelated with GB wind" is near-tautological
    (the seasonal-budget hydro driver is wind-independent by
    construction) — the observational r = −0.087 is what pins the
    real-world claim; the engine gate must therefore be framed as
    reproducing the OBSERVED flow/generation structure, not as
    discovering independence.
(c) CONT-NW internal copper plate is acceptable v1; link caps bound the
    error and calibration anchors zone energies. It is what makes (a)
    binding, and DE-LU's 70 % load share means the bloc scarcity signal
    is effectively German — both stated in the engine work order.
    Revisit trigger: BE/NL annual-energy tolerance misses.
(d) Per-country CF derivation keeps the bloc decision revisable;
    NO2-wind caveat as above.
(e) IE fossil_oil and NO2 wind as above; neither silently dropped.

## What this does NOT decide (engine work-order items)

Import/export dispatch coupling (relative scarcity + price per ADR
sketch), link availability modelling, and the docs/04 sign-test
amendment (resource-level reformulation per the pack review ruling —
supervisor drafts it in the Stage 5 work-order pinning, direction gate
threshold above the 92.3 % FR base rate).

## Review

Adjudicated 2026-07-03 (D4 precedent): **ADOPT-WITH-EDITS** — the
five-zone structure ruled the right reading of the evidence against
all stressed alternatives (NO-aggregate, BE/NL split, DK1 folded,
DE-LU omitted); six edits required and applied in this text:
(1) per-border direction-gate claim corrected to annual-energy
validation with the ≥13.57 % opposite-sign structural floor stated;
(2) correlation evidence re-aimed (per-border figures; aggregation
rested on scope control, not behavioural identity); (3) NO2 wind
quantified with the energy-balance condition; (4) NESO figures for the
NESO-referenced identity; (5) ADR-7 amendment proposed, not drifted;
(6) reservoir-hydro wording tightened. The reviewer's full check table
and arithmetic are in its adjudication (session record, 2026-07-03).
