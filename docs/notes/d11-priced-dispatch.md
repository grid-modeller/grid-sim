# D11 — Priced multi-zone dispatch (tier-2 imports) design

**Status:** ADOPTED 2026-07-04 — supervisor draft, reviewer
ADOPT-WITH-EDITS (docs/notes/d11-priced-dispatch-review.md), all
eight ordered edits applied below. The review corrected three
load-bearing errors: the acceptance metric was mislabelled (it is the
A2a direction-match, not export recall); per-zone SRMC is new schema +
data, not a free consumer of existing logic; and the claim that the
priced ladder resolves part of the B6 finding was INVALID — that
excess is store-headroom-blindness, resolved only by the LP. Fixes the
two tier-2 gaps named across this session's deviations. D10 is
reserved for the EV/transport overlay (Q12); this is D11.

## The problem: two gaps, one root

1. **Frozen imports under the capacity sweep** (Richard's Q10 finding;
   tracked deviation). The Module 1 wind-capacity sweep is single-zone
   and holds net imports at the 2024 observed trace at every swept
   capacity — physically wrong away from the anchor. Package B bracketed
   it (frozen / zero / export conventions); tier 2 is the endogenous
   fix.
2. **The flow rule is a scarcity proxy, not a price model**
   (grid-adequacy/src/flow.rs "what this rule deliberately is not"). It
   equalises a dimensionless scarcity signal, explicitly "wrong exactly
   where fuel/carbon prices diverge between zones." The Stage 5 A2 gate
   left a **≥95% GB↔FR direction-match (A2a) priced-ladder target,
   expectation 97.4%** as the acknowledged tier-2 acceptance bar. The
   B6 review named a related but DISTINCT gap (rule-based dispatch
   clears flows before storage, blind to headroom) — the LP resolves
   that one; the priced ladder does NOT (Edit 5, §C).

Root of both: flows respond to a scarcity *score*, and the high-wind
capacity sweep does not run on the multi-zone engine at all. Tier 2
closes both by (a) making the flow signal a genuine marginal price and
(b) running the sweep multi-zone.

## Rule 1 — The signal becomes a marginal price, not a score

Replace the scalar scarcity signal (flow.rs prose rule 1) with each
zone's **system marginal price** at its residual demand: the SRMC of
the marginal dispatched unit (the Stage 2 SRMC recipe evaluated per
zone from per-zone fuel/carbon inputs — Edit 2 schema work), with the
documented conventions carried (£0 must-take floor; unserved → the
Stage 2 convention-3 price (fleet SRMC ceiling), so the flow rule and
the pricing layer agree; unserved still outbids every dispatched rung
but at a pinned, consistent level, NOT a new VoLL proxy, and is never
monetised into adequacy). Flows then equalise *price*, capped by
capability — the textbook merit-order-coupling behaviour, myopic
(per-period, no foresight). This makes fuel/carbon-price asymmetry
between zones **visible** (the GB carbon floor premium over EU ETS
becomes a real term), which the scarcity score could not represent.

The scarcity signal is **retained as the tiebreak in the must-take
region**: when two zones both price at the £0 floor, the old
surplus-depth equalisation decides the flow (its negative-price
analogue) — so the price rule degrades gracefully to the current
behaviour exactly where prices carry no information. This is the
graceful-degradation property that keeps the Stage 5 validated
behaviour intact where it was already right.

**Signal well-posedness (the interior tiebreak).** The per-zone signal
is **lexicographic: (SRMC £/MWh primary, fractional-utilisation of the
marginal rung secondary)**. The secondary key is today's intra-rung
term, retained everywhere, not only at the £0 floor. This (a) preserves
`equalising_flow`'s positive-`rate` invariant (`flow.rs:241`) so the
exact breakpoint walk stays deterministic where SRMC is flat across a
band; (b) makes the rule degrade to **exactly today's behaviour whenever
per-zone SRMCs are equal** (equal carbon/fuel), not merely in the £0
surplus region; and (c) leaves the fix to operate through the SRMC
primary key **only where per-zone prices genuinely diverge** (the GB CPS
premium — the A2 case). The bang-bang flow that results when a non-zero
SRMC gap spans a flat band (equalisation runs to a rung edge or the cap)
is the intended merit-order-coupling behaviour and is stated here so it
is not an implementation accident.

## Rule 2 — The sweep runs multi-zone

The wind-capacity sweep (and the heating-mix and per-year sweeps, if
cheap) run on `run_multi`, so imports respond endogenously to the
swept fleet. The Package B import-convention bracket is **retained as
the single-zone lower/upper bounds** — tier 2 becomes the measured
central estimate between them, and the bracket becomes the disclosed
uncertainty band, not the headline. Frozen-imports deviation moves
from "bracketed" to "resolved, with the bracket as the error bar."

## Rule 3 — Relationship to the Stage 7 LP (ADR-6)

Three dispatch policies now sit on a ladder of increasing fidelity and
cost, all reported (ADR-6's dual-policy discipline generalised):
- **Scarcity rule** (current) — myopic, price-blind. Retained as the
  must-take-region tiebreak and the cheapest sweep mode.
- **Priced ladder** (this note) — myopic, price-aware. The new default
  for priced/market questions (Q10, Q2, the B6 curtailment bracket).
- **Perfect-foresight LP** (Stage 7, ADR-10) — optimal, expensive. The
  resolver of the remaining myopia gap; the priced-ladder-vs-LP gap is
  a **reported finding**, exactly as the rule-vs-LP gap is.
The priced ladder resolves the **A2 direction residual** — the
night/shoulder both-gas-marginal periods where per-zone prices genuinely
diverge (the GB CPS premium; stage-5 §2.4). It does **not** resolve the
B6 storage-sizing dispatch-convention component: that excess (b6 review
§1b) is store-headroom-blindness in the pre-storage clearing order, and
it lives in surplus / £0 periods where — by this note's own
graceful-degradation rule — the priced ladder is identical to the
scarcity rule. The B6 §1d resolver remains the **LP** (Stage 7); the
priced ladder is not a bound on the B6 +38–49%.

## Rule 4 — Validation and pins

- **A2a GB↔FR direction match** (Stage 5): the ≥95% priced-ladder
  target with expectation 97.4% is the acceptance bar. Miss → the
  priced ladder is not adequate and the finding is named (not
  re-pinned).
- **Determinism**: the single-zone reference digest (779d7444) stays
  UNMOVED **because a single-zone scenario has zero borders and the
  flow rule is unreachable** (`multizone.rs:293`), not merely because
  single-zone runs are "byte-untouched". The 5-zone and 2-zone
  **dispatch digests** WILL move; they are re-pinned with old values
  recorded and the diff explained (Stage 5 A2 re-pin precedent).
- **Validation gates must still PASS (not re-pinned).** The priced
  ladder changes every per-border flow, so re-assert and re-measure:
  Stage 5 A1 (imports ±10%), A2a/A2b, A3 (both sign tests), A4 (BE/NL
  per-border energies), the Module 5 capacity-credit table (incl.
  NO2-flat); and the B6 gates (i/ii/iii), the B6 robustness numbers
  (26,480/35,648) and the Q2/Q10 60 GW measurements. The ladder must
  fix A2a to the 97.4% expectation **without regressing any of these
  below their bands** — a regression is a failure/finding, not a
  re-pin.
- **The frozen-imports sweep**: the tier-2 central estimate at 60 GW
  wind is pinned and quoted against the Package B bracket
  (0.535–0.611 delivered-capture bracket becomes a central value with
  the bracket as the band).
- **B6 dispatch-convention component**: re-run the RS robustness
  finding under the priced ladder and record the copper-plate delta.
  The **expectation is ~nil** (the finding lives in the £0 surplus
  region where the priced ladder degrades to the scarcity rule); a
  measured ~nil delta is itself the pinned confirmation that the B6
  excess is store-headroom-blindness, not price-blindness, and that it
  survives to the LP. Do NOT quote this as bounding the +38–49%.

## Rule 5 — What this does NOT do

Not the LP (Stage 7, ADR-10 — perfect foresight is a different policy,
not this note). Not a market-coupling institutional model (no
day-ahead/intraday split, no bidding, no strategic behaviour). Not a
wheeling/transit model (D5 stands — CONT-NW internal copper plate). No
negative *prices* yet — the £0 floor stays until the Q13 subsidy-aware
bid floors land (which is where genuine negative prices emerge); the
priced ladder's must-take tiebreak is the current negative-price
analogue, unchanged. No fuel-price *forecast* — zone SRMCs use the
committed price references, not projected trajectories (that is Stage 7
/ Q13 territory).

**Per-zone price data (docs/05 discipline).** The per-zone SRMCs require
cited, licence-clean, checksummed inputs the current pack lacks: an EU ETS
carbon price (vs the committed UK UKA+CPS), and continental/FR gas price
references. These carry provenance, licence status, checksums and
tolerance/convention justifications per docs/05 before any A2 number is
quoted, exactly as prices-2024.toml did. Absent this data the flow rule
falls back to equal per-zone carbon → no A2 improvement.

## ADR touch-points (proposed amendments, recorded here per CLAUDE.md)

- **flow.rs normative prose**: rule 1 becomes price-based with the
  scarcity signal as the must-take tiebreak; "not a price model" in the
  deliberately-is-not list is struck (it becomes a price model, myopic).
- **ADR-6**: the policy set becomes {scarcity-rule, priced-ladder,
  perfect-foresight-LP}, all reported; the priced-ladder-vs-LP gap
  joins the rule-vs-LP gap as a reported finding.
- **ADR-9** (pricing lives in grid-core): per-zone SRMC is **new input
  plumbing over the existing SRMC recipe**, not a free consumer. It
  requires (i) a per-zone `[pricing]` extension to the scenario schema —
  per-zone fuel-price and carbon inputs (or a per-zone carbon adder over a
  shared base) — a **schema_version bump with a docs/03 migration note**;
  (ii) a per-zone marginal-SRMC evaluation the flow rule calls at each
  zone's current residual as the border sweep advances; and (iii) the
  cited, licence-clean, checksummed price data above. The grid-core
  SRMC *recipe* is reused unchanged; the per-zone application is new.
- **Schema + docs/03**: the per-zone `[pricing]` extension is a
  schema_version bump with a docs/03 migration note.
- **docs/04**: the Stage 5 (and Stage 7) work-order text gains the priced
  ladder as the priced-question default and records the A2a ≥95%
  priced-ladder target being attempted; the superseded-target line in
  docs/04 is cross-referenced, not silently changed.
- **ADR-7**: external zones (FR, CONT-NW, NO2, DK1, IE-SEM) now require
  per-zone pricing inputs to be dispatchable under the priced ladder —
  noted so the zone list and the pricing-data package stay in step.
- **b6 review §1d**: its "DISPATCH CONVENTION" clause gains "the priced
  ladder does not resolve this component; the LP remains the resolver"
  when next touched.
- **ADR-10 boundary intact**: the priced ladder is myopic/per-period; the
  LP (perfect foresight, `good_lp`+HiGHS) remains the only Stage 7
  optimiser — no edit needed beyond confirming it.

## Implementation shape (for the work order, post-adjudication)

One engine package: the priced flow signal in flow.rs + multizone
pricing wiring (the Stage 2 SRMC chain per zone), red-first against the
A2 target; then the multi-zone sweep mode. Sequenced after this note is
adjudicated. Estimated one engine package + one sweep package, the
D8/D9 two-package rhythm.
