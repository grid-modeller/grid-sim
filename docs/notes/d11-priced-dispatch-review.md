# D11 — Priced multi-zone dispatch: reviewer adjudication

**Reviewer, 2026-07-04.** Adjudicates the supervisor draft
`docs/notes/d11-priced-dispatch.md` (committed 98578d9) and its docs/08
D11 row. D8/D9 precedent: contestable engine design is pinned in prose
and reviewer-adjudicated before any engine work.

## Verdict: ADOPT-WITH-EDITS

The core direction is sound and worth pinning: make the flow signal a
genuine marginal price, retain scarcity as the must-take tiebreak, run
the sweep multi-zone, report the three-policy ladder. That much I adopt.
But the draft carries three load-bearing errors — a **mislabelled
acceptance metric**, a **false "no new pricing logic" premise**, and an
**invalid claim that the priced ladder isolates the B6 dispatch-convention
component** — plus three unpinned conventions an implementer would have to
resolve blind (the within-band tiebreak, the unserved price level, and the
full set of flow-derived gates that must be re-asserted). Edits 1–8 below
are ordered; the exact replacement text is given where I order a change.
Nothing merges until they are applied and I re-check the note.

The three rulings the task demands are in §A (well-posedness + graceful
degradation), §B (the B6-pin interaction), and §C (the
dispatch-convention-isolation claim) before the edit list.

---

## A. RULING — well-posedness and graceful degradation (task item 1)

**Graceful degradation holds ONLY in the both-zones-£0 region, and is
well-posed there.** In the pure surplus region the note retains the
surplus-depth scarcity tiebreak, so the flow is identical to today
(`flow.rs` surplus branch, `signal` line 158 `if r <= 0.0 { return r; }`);
digests for surplus-only periods are unmoved by construction. Where one
zone is floored and one is not, the rule *intentionally* diverges — that
is the fix, and the draft discloses it. No silent-divergence defect there.

**The silent gap is the interior flat-SRMC region, and the draft does not
close it.** The current scarcity signal is *strictly* increasing within a
merit rung — the fractional-utilisation term, `index + rem/ceiling`
(`flow.rs:164`; `up_probe` slope `1.0/ceiling`, `flow.rs:182`). A pure
per-zone SRMC is a **step function**: flat across a whole technology band.
Consequences the draft must confront:

1. `equalising_flow` computes `rate = slope_exp + slope_imp*delivered`
   and then `d_cross = gap/rate` (`flow.rs:241-242`), with the code
   commenting "both slopes positive". Under a flat SRMC band the relevant
   slope is **zero**. Two same-rung zones with **equal** per-zone carbon
   have equal SRMC → `gap <= 0` → the loop breaks at `flow.rs:236` → **no
   flow**. The scarcity rule's intra-rung trade (energy flows toward the
   proportionally more-stressed gas fleet) is **silently deleted**. That
   intra-rung trade is exactly the both-gas-marginal regime that carries
   1,122 of the 1,297 A2 residual mismatches (stage-5 report §2.4), so the
   change is not cosmetic — it is the mechanism the note is trying to fix.
2. The A2 fix therefore **depends entirely on per-zone carbon differing**
   (GB CPS vs EU ETS) to create a non-zero `gap` between GB and FR gas
   SRMC. With a single global carbon price the two gas SRMCs are equal,
   `gap = 0`, and the direction match does **not** improve — the note
   fails its own 97.4% bar. This is not optional polish; it is load-bearing
   (see Edit 2 / §D, the pricing-data defect).
3. Even with a non-zero carbon gap, within each zone's own gas band the
   slope is zero, so the equalisation runs bang-bang to a rung edge or the
   cap rather than smoothly. That is a defensible merit-order-coupling
   behaviour, but it is a **material, unstated choice** and it is what
   determines the A2 number. Left unpinned, the implementer picks it.

**Ruling: not well-posed as written.** The note must specify the signal as
**lexicographic — (per-zone SRMC primary, fractional-utilisation
secondary)** — so that (a) it degrades to *exactly today's* behaviour when
per-zone SRMCs are equal (not just at the £0 floor), and (b)
`equalising_flow`'s positive-`rate` invariant is preserved by the
secondary key. Edit 3 carries the replacement text.

## B. RULING — the B6-pin interaction (task item 2)

**The single-zone digest claim is structurally TRUE.** `multizone.rs:293`
sets `links_live = scenario.zones.len() > 1`; with one zone the borders
are inert and `equalising_flow` is never entered (module docs line 55:
"there are no dispatchable borders, every link series is zero"). The
"direct `run_multi`-on-reference pin" (stage-5 report line 11) is itself a
single-zone scenario, so it too cannot move. Changing the flow signal
cannot touch 779d7444. Adopt the claim; sharpen the *reason* to "a
single-zone scenario has zero borders, so the flow rule is unreachable"
(Edit 6).

**But the note names only the 5-zone/2-zone dispatch digests, and that is
badly incomplete.** Every published Stage 5 and B6 number is
flow-rule-derived and WILL move under the priced ladder:

- Stage 5 **validation gates** A1 (imports ±10%), A2a/A2b, A3 (both sign
  tests), A4 (BE/NL per-border energies), and the **Module 5 capacity-credit
  table** (incl. the embargo-lifted NO2-flat headline). These are not
  digests to be re-pinned — they are *realism gates against observed 2024*
  that the priced ladder must **still pass**. A ladder that fixes A2 but
  pushes A4 NL or A3 out of band is a failure, not a re-pin.
- The **B6 gates (i/ii/iii)** just pinned at 209d78b — copper-plate net
  18.809 TWh / r 0.7443, constrained net 15.788 TWh, binding share 0.2323
  (b6 review §2) — are computed by the flow rule and will move; they must
  be re-measured and shown to still validate.
- The **B6 robustness numbers** (26,480 copper / 35,648 B6; b6 review §1a)
  and the **Q2/Q10 60 GW measurements** (SCO 27.139/13.768, binding 0.4674;
  b6 review §4) are flow-derived and must be re-measured with old values
  recorded.
- The **b6 §1d finding-of-record framing** names "rule-based dispatch,
  upper-bias … the LP comparison (Stage 7) is the named resolver." The
  priced ladder inserts an intermediate rung; the §1d "DISPATCH CONVENTION"
  clause needs updating — but see §C, because the update is the *opposite*
  of what the draft claims.

**Ruling: the pin discipline is right in kind (Stage 5 A2 re-pin
precedent: old values recorded, diff explained) but wrong in scope.** The
note must split into (a) internal digests → re-pinned; (b) validation
gates (A1/A2/A3/A4, Module 5, B6 i/ii/iii) → re-asserted and STILL
PASSING, a gating requirement not a re-pin. Edit 6.

## C. RULING — the dispatch-convention-isolation claim is INVALID (task item 3)

This is the draft's central analytical error. Rule 3 claims the priced
ladder "bounds how much of the B6 '+38–49%' upper-bias is
dispatch-convention (resolved here) versus genuine boundary constraint
(survives to the LP)," and rule 4 operationalises it as the delta between
the +10.9% copper-plate and a priced-ladder copper-plate.

**The B6 review already decomposed the +10.9%, and it is NOT
price-blindness.** b6 review §1b isolates the driver via single-bus
controls (pooled 23,808 ≈ nil; two-store single-bus 24,112 = +1.0%; two-zone
copper 26,480 = +10.9%) and attributes the excess to *"flows clear before
storage by surplus-depth equalisation, blind to store headroom"* — the
smoking gun being **3,618 TWh/40y of Scottish curtailment with the link
slack and the RGB store holding both power and energy headroom.** That is a
**store-headroom-blindness in the pre-storage clearing ORDER**, not a
price-blindness.

**The priced ladder does not touch that mechanism.** D11 replaces the
*signal* but explicitly does **not** change the pre-storage clearing order
(`flow.rs` header: "before any storage action … link trades clear ahead of
residual balancing"; rule 5 of the draft keeps this — that is Stage-7/LP
territory). And the B6 robustness finding lives entirely in **surplus /
curtailment periods**, where both zones sit at the **£0 must-take floor** →
by the draft's OWN graceful-degradation rule the priced ladder **reduces to
the scarcity tiebreak** there. So a priced-ladder copper-plate run of the
RS robustness scenario will show a delta of **~nil** — not because the
dispatch-convention component is small, but because the priced ladder is
*identical to the scarcity rule in the region where that component lives.*

**Ruling: invalid, and it double-counts.** The priced ladder resolves the
**A2 direction residual** — the night/shoulder both-gas-marginal periods
where real FR prices sit below GB's and prices are **non-zero and
divergent** (stage-5 §2.4). It does **not** resolve, bound, or isolate the
B6 storage-sizing dispatch-convention component, which is a £0-region
store-headroom-blindness that survives unchanged to the LP. The note must
delete the B6-isolation claim and re-home the priced ladder's contribution
to A2. The B6 §1d "resolver" remains the LP, not the priced ladder. Edit 5.

---

## Ordered edits

### Edit 1 — correct the mislabelled acceptance metric (task item 5)

The 95%/97.4% figure is the **GB↔FR direction-match (A2a)** target, not
export recall. Stage-5 report §2: the original pre-model pin was "GB↔FR
direction match ≥ 95%"; §2.4: the 1,297 *direction* mismatches, "measured
expectation if that class is priced away: 97.4%"; §2.5: "the superseded
≥ 95% stays in docs/04 as the priced-ladder target." Export recall (A2b)
is a *different* metric, pinned ≥ 70% and already passing at 78.96% — if
the ladder gated only on it, the bar would be trivially met.

- Note lines 20–24, replace "≥95% export-recall priced-ladder target,
  expectation 97.4%" with: **"≥95% GB↔FR direction-match (A2a)
  priced-ladder target, expectation 97.4%".**
- Note lines 81–83, replace "A2 export recall (Stage 5): the ≥95% target"
  with: **"A2a GB↔FR direction match (Stage 5): the ≥95% priced-ladder
  target".**
- docs/08 D11 row: replace "A2 export-recall ≥95% (expectation 97.4%)"
  with **"A2a direction-match ≥95% (expectation 97.4%)".**

### Edit 2 — per-zone SRMC is new pricing input + data, NOT "a new consumer" (task items 1, 6)

The draft's premise (line 35–36 "the Stage 2 pricing chain already computes
this per zone"; line 122–123 ADR-9 "no new pricing logic, a new consumer of
existing logic") is **false**. `pricing.rs:131` calls `single_zone(scenario)`;
`Scenario.pricing` is a single top-level `Option<PricingSpec>`
(`scenario.rs:218`) with one `fuel_price` map and one `srmc` recipe map —
**global, not per-zone**. There is no per-zone carbon or fuel today
(project-state confirms: "pricing and sweep are single-zone only … multizone.rs
has no SMP wiring"). Per §A, the carbon asymmetry is load-bearing for the A2
fix.

Replace the ADR-9 touch-point (note lines 122–124) with:

> - **ADR-9** (pricing lives in grid-core): per-zone SRMC is **new input
>   plumbing over the existing SRMC recipe**, not a free consumer. It
>   requires (i) a per-zone `[pricing]` extension to the scenario schema —
>   per-zone fuel-price and carbon inputs (or a per-zone carbon adder over a
>   shared base) — a **schema_version bump with a docs/03 migration note**;
>   (ii) a per-zone marginal-SRMC evaluation the flow rule calls at each
>   zone's current residual as the border sweep advances; and (iii) the
>   cited, licence-clean, checksummed price data of Edit 7. The grid-core
>   SRMC *recipe* is reused unchanged; the per-zone application is new.

And in rule 1 (note line 35–36) replace "the Stage 2 pricing chain already
computes this per zone" with **"the Stage 2 SRMC recipe evaluated per zone
from per-zone fuel/carbon inputs (Edit 2 schema work)".**

### Edit 3 — pin the within-band tiebreak (task items 1, 7)

Extend rule 1 (append after note line 50). Replacement text:

> **Signal well-posedness (the interior tiebreak).** The per-zone signal
> is **lexicographic: (SRMC £/MWh primary, fractional-utilisation of the
> marginal rung secondary)**. The secondary key is today's intra-rung
> term, retained everywhere, not only at the £0 floor. This (a) preserves
> `equalising_flow`'s positive-`rate` invariant (`flow.rs:241`) so the
> exact breakpoint walk stays deterministic where SRMC is flat across a
> band; (b) makes the rule degrade to **exactly today's behaviour whenever
> per-zone SRMCs are equal** (equal carbon/fuel), not merely in the £0
> surplus region; and (c) leaves the fix to operate through the SRMC
> primary key **only where per-zone prices genuinely diverge** (the GB CPS
> premium — the A2 case). The bang-bang flow that results when a non-zero
> SRMC gap spans a flat band (equalisation runs to a rung edge or the cap)
> is the intended merit-order-coupling behaviour and is stated here so it
> is not an implementation accident.

### Edit 4 — unserved price consistent with Stage 2 (task item 1)

The draft's "unserved → scarcity price = VoLL proxy" (note line 37–38)
contradicts the Stage 2 convention: unserved periods "price at the fleet
SRMC ceiling — grid-core convention 3" (`pricing.rs:296-297`, `:420-421`).
A VoLL proxy and a fleet-SRMC-ceiling price rank unserved on top equally
but produce different flow magnitudes (a VoLL proxy pulls imports far
harder). Replace "unserved → scarcity price = VoLL proxy, stated, NOT
monetised into adequacy" with:

> **unserved → the Stage 2 convention-3 price (fleet SRMC ceiling), so the
> flow rule and the pricing layer agree; unserved still outbids every
> dispatched rung but at a pinned, consistent level, NOT a new VoLL proxy,
> and is never monetised into adequacy.**

If a harder-than-SRMC-ceiling scarcity price is wanted specifically for the
flow rule, it is a separate, pinned convention with its own justification —
not folded in silently.

### Edit 5 — delete the invalid B6-isolation claim (task item 3, §C)

Rule 3 (note lines 73–77), replace the sentence beginning "The priced
ladder is the middle rung…" through "…survives to the LP" with:

> The priced ladder resolves the **A2 direction residual** — the
> night/shoulder both-gas-marginal periods where per-zone prices genuinely
> diverge (the GB CPS premium; stage-5 §2.4). It does **not** resolve the
> B6 storage-sizing dispatch-convention component: that excess (b6 review
> §1b) is store-headroom-blindness in the pre-storage clearing order, and
> it lives in surplus / £0 periods where — by this note's own
> graceful-degradation rule — the priced ladder is identical to the
> scarcity rule. The B6 §1d resolver remains the **LP** (Stage 7); the
> priced ladder is not a bound on the B6 +38–49%.

Rule 4's B6 bullet (note lines 94–98), replace with:

> - **B6 dispatch-convention component**: re-run the RS robustness finding
>   under the priced ladder and record the copper-plate delta. The
>   **expectation is ~nil** (the finding lives in the £0 surplus region
>   where the priced ladder degrades to the scarcity rule); a measured
>   ~nil delta is itself the pinned confirmation that the B6 excess is
>   store-headroom-blindness, not price-blindness, and that it survives to
>   the LP. Do NOT quote this as bounding the +38–49%.

Also add a line to the ADR touch-points: the b6 §1d "DISPATCH CONVENTION"
clause gains "the priced ladder does not resolve this component; the LP
remains the resolver" when next touched.

### Edit 6 — re-assert ALL flow-derived gates; split digests from validation (task items 2, 5)

Replace rule 4's "Determinism" bullet (note lines 84–89) and add a new
bullet:

> - **Determinism**: the single-zone reference digest (779d7444) stays
>   UNMOVED **because a single-zone scenario has zero borders and the flow
>   rule is unreachable** (`multizone.rs:293`), not merely because
>   single-zone runs are "byte-untouched". The 5-zone and 2-zone **dispatch
>   digests** WILL move; they are re-pinned with old values recorded and the
>   diff explained (Stage 5 A2 re-pin precedent).
> - **Validation gates must still PASS (not re-pinned).** The priced ladder
>   changes every per-border flow, so re-assert and re-measure: Stage 5 A1
>   (imports ±10%), A2a/A2b, A3 (both sign tests), A4 (BE/NL per-border
>   energies), the Module 5 capacity-credit table (incl. NO2-flat); and the
>   B6 gates (i/ii/iii), the B6 robustness numbers (26,480/35,648) and the
>   Q2/Q10 60 GW measurements. The ladder must fix A2a to the 97.4%
>   expectation **without regressing any of these below their bands** — a
>   regression is a failure/finding, not a re-pin.

### Edit 7 — name the per-zone pricing DATA deliverable (task items 4, 6; docs/05)

The A2 fix depends on GB-vs-continental carbon/fuel asymmetry (§A), which
needs new cited data. Add to rule 5 or a new "Data" clause:

> **Per-zone price data (docs/05 discipline).** The per-zone SRMCs require
> cited, licence-clean, checksummed inputs the current pack lacks: an EU ETS
> carbon price (vs the committed UK UKA+CPS), and continental/FR gas price
> references. These carry provenance, licence status, checksums and
> tolerance/convention justifications per docs/05 before any A2 number is
> quoted, exactly as prices-2024.toml did. Absent this data the flow rule
> falls back to equal per-zone carbon → no A2 improvement (§A).

### Edit 8 — complete the ADR / docs touch-points (task item 6)

Add to the ADR touch-points list:

> - **Schema + docs/03**: the per-zone `[pricing]` extension is a
>   schema_version bump with a docs/03 migration note (Edit 2).
> - **docs/04**: the Stage 5 (and Stage 7) work-order text gains the priced
>   ladder as the priced-question default and records the A2a ≥95%
>   priced-ladder target being attempted; the superseded-target line in
>   docs/04 is cross-referenced, not silently changed.
> - **ADR-7**: external zones (FR, CONT-NW, NO2, DK1, IE-SEM) now require
>   per-zone pricing inputs to be dispatchable under the priced ladder —
>   noted so the zone list and the pricing-data package stay in step.
> - **ADR-10 boundary intact**: the priced ladder is myopic/per-period; the
>   LP (perfect foresight, `good_lp`+HiGHS) remains the only Stage 7
>   optimiser — correctly stated; no edit needed beyond confirming it.

---

## What the draft gets right (adopt as-is once edited)

- Pricing the flow signal and retaining scarcity as the must-take tiebreak
  is the correct generalisation of ADR-6 dual-policy discipline; the
  three-policy ladder {scarcity, priced, LP} is coherent **once the priced
  ladder's role is scoped to the A2 direction residual** (Edit 5) rather
  than over-claimed against B6.
- Running the sweep multi-zone with the Package B bracket demoted to the
  disclosed single-zone uncertainty band (rule 2) is sound.
- Deferring genuine negative prices to Q13 is **consistent** with the
  syllabus: docs/07 Q13 (lines 150–159) is where "subsidy-aware bid floors
  let negative prices EMERGE from scheme rules." No gap results, because the
  priced ladder uses the surplus-depth tiebreak (not a price) to allocate
  curtailment in the £0 region — it never needs a negative price it cannot
  produce. Adopt.
- Scope cuts (no LP, no market-coupling institutions, no wheeling, no fuel
  forecast) are the right cuts.

## Completeness for the implementer

With Edits 2, 3, 4, 6, 7 applied, an implementer can write the acceptance
tests red-first: the lexicographic signal is pinned (Edit 3), the unserved
price is pinned (Edit 4), the per-zone pricing schema + data are named
(Edits 2, 7), and the full gate set is enumerated (Edit 6). Without them,
the within-band flow magnitude, the unserved price level, and the per-zone
carbon source are all unreviewed choices that would silently determine the
97.4% result — precisely the failure D-notes exist to prevent.

## Files
- Adjudicated: `docs/notes/d11-priced-dispatch.md` (edits 1–8), plus its
  docs/08 D11 row (edit 1).
- This review: `docs/notes/d11-priced-dispatch-review.md`.
