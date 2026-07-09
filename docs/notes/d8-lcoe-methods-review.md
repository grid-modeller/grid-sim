# D8 adjudication — system-cost / LCOE methodology note (reviewer)

Reviewer adjudication, 2026-07-03. Subject: the uncommitted
`docs/notes/d8-lcoe-methods.md` (supervisor draft) plus its docs/08 D8
row. The ratified method basis (Richard, 2026-07-03: delivered
system-cost headline, additive decomposition, equal-reliability
comparisons, differencing-only marginal claims, WACC-prominent,
bridges-not-headlines, no Ueckerdt-style attribution headline) is not
relitigated here; this adjudication tests whether the draft implements
that basis soundly and completely enough that no unreviewed convention
choice is left to the implementer.

## VERDICT: ADOPT-WITH-EDITS

The draft is structurally sound: the ratified basis is faithfully and
completely transcribed (all seven elements present, verified against
the ratification record in memory/project-state.md "RICHARD'S
DECISIONS 2026-07-03"); the double-counting argument for
curtailment/overbuild is arithmetically correct; the D6 quarantine,
determinism/pin discipline, and the emissions-gap precondition are
right. But it has one first-order hole — the Q9 decomposition, a
docs/04 acceptance test, is asserted compatible with rules 5/7 without
any construction, which is exactly the unreviewed-convention failure
D8 exists to prevent — plus a rule-3/rule-5 internal contradiction,
a denominator ambiguity that will bite in code, and four smaller
unpinned conventions. Nine blocking edits. Apply all nine, then the
note is ADOPTED; the docs/08 D8 row updates to "Resolved" citing this
review.

---

## Blocking edits (exact text where a change is ordered)

### Edit 1 — Add Rule 6a: the Q9 gap decomposition is an identity,
### not an attribution (the first-order defect)

docs/04:300–301 makes "LCOE vs. delivered £/MWh gap fully decomposed
(Q9)" an acceptance test; docs/07:103–105 gives Q9's labels (backup,
balancing, curtailment, transmission, stability). As drafted, rule 5
("No formula-based attribution of shared system costs to individual
technologies is published as a finding") and rule 7 (no
integration-cost decompositions) can be read to forbid that test —
Q9's label list IS the shape of an Ueckerdt-style decomposition. The
draft's interactions section merely asserts "Q9 is rule 6's ...
decomposition, already an acceptance test" with zero methodology. The
implementer could not write the Q9 test red-first from this note
without inventing the single most contested convention in it. Insert
after rule 6:

> ## Rule 6a — The Q9 gap decomposition is an identity, not an
> ## attribution
>
> The docs/04 Stage 7 acceptance test "LCOE vs delivered £/MWh gap
> fully decomposed (Q9)" is implemented as a **system-level accounting
> identity**, exact under rule 2 — never as an attribution of shared
> costs to a single technology (which rules 5 and 7 forbid).
> Construction: the gap between the generation-weighted mean of
> per-tech plant-gate LCOEs (weighting basis stated) and rule 1's
> delivered system cost decomposes exactly into three wedges:
> 1. **Denominator wedge** — generation vs delivered-to-demand energy
>    (curtailment, storage round-trip losses, boundary flows per
>    rule 8);
> 2. **Missing-line wedge** — rule 1 lines absent from plant-gate
>    LCOE (storage, interconnection, stability services, constraint
>    costs);
> 3. **Utilisation wedge** — realised capacity factors vs the CF
>    assumptions inside the per-tech LCOE figures.
> The identity terms are the normative objects. docs/07 Q9's labels
> (backup, balancing, curtailment, transmission, stability) are
> presentational groupings of those terms; every Q9 artefact states
> the exact term-to-label mapping, and "transmission" in this model
> means constraint costs + interconnection only (no network model,
> ADR-12 — stated on the chart). Per-technology decompositions of the
> gap are published only as rule-5 scenario differences.

### Edit 2 — Rule 3 needs the fixed-fleet escape hatch, and it is
### mandatory, not hypothetical

docs/04:294–295 puts "scenario pack for published pathways (FES, CCC,
Royal Society)" in Stage 7 scope. A fixed-fleet FES pathway year
cannot be re-solved to zero unserved without changing the fleet being
costed, so rule 3's default silently fails on an in-scope scenario
class, and rule 1's headline for a single unreliable scenario would
publish a cheap-looking £/MWh with the shortfall invisible. The "any
other standard must be stated" clause does not cover this: it
regulates comparisons, not single-scenario headlines, and gives no
construction. Append to rule 3:

> **Fixed-fleet scenarios (the FES/CCC pathway pack, docs/04 Stage 7
> scope):** these cannot be re-solved to the standard without changing
> the fleet being costed. Convention: (a) every cost artefact stamps
> the scenario's unserved energy and the adequacy standard it was
> solved to — zero-unserved artefacts stamp that fact too; (b) a
> fixed-fleet scenario with unserved energy above the stated standard
> is published only with its unserved TWh adjacent to the £/MWh
> figure, and is excluded from rule-3 comparisons; (c) for
> comparisons, a **reliability make-good** variant is constructed —
> the minimum addition of a stated resource by a stated solver (Stage
> 3 bisection precedent) that reaches the standard — and the make-good
> appears as a named rule-1 line. Equal-reliability comparisons use
> the make-good variants.

### Edit 3 — Rule 5 contradicts rule 3 as written, and must
### explicitly license sweep-derived marginal curves

"a scenario pair differing only in X" is unconstructible under
rule 3: re-solving both endpoints to equal reliability changes
storage/overbuild too. And without an explicit statement, rule 5 can
be read to forbid Q2/Q3 marginal curves read off Stage 4 sweep
surfaces. Replace rule 5's first two sentences with:

> "The cost of X" for any technology or asset class means EXACTLY:
> the difference in total system cost between a scenario pair that
> differ in X, with all other free resources re-solved to the rule-3
> standard by the same stated procedure at both endpoints, both
> endpoints stated. The difference is the system cost of X *inclusive
> of its balancing consequences* — that inclusiveness is the claim's
> content, not a confound. A sweep is a chain of such pairs: marginal
> curves read off Stage 4 sweep surfaces (Q2, Q3) are rule-5
> compliant, with the sweep axis and the re-solve procedure stated.

(Q3 additionally inherits the D4 policy conditioning the draft already
names: under RuleBased, storage never displaces gas —
docs/notes/d4-rule-based-dispatch.md rule 3.)

### Edit 4 — Pin the denominator exactly; "delivered" is now two
### different objects in this codebase

Package A introduced delivered-**by-technology** series
(post-curtailment generation; `total_wind_delivered` etc.). Rule 1's
denominator is delivered-**to-demand**. These differ by storage
losses, boundary flows, and unattributed spill; an implementer summing
per-tech delivered series as the headline denominator would be wrong
by construction, and nothing in the draft stops them. Also unstated:
exports and external-zone demand under multi-zone runs. Append to
rule 1's first paragraph:

> Denominator pinned exactly: **delivered-to-demand energy = GB
> underlying demand served = GB demand − GB unserved energy** over the
> horizon (D3 convention). It is NOT Σ per-technology delivered
> generation (the Package A "delivered-by-technology" series — a
> different object; the two differ by storage losses, boundary flows,
> and unattributed spill). Exports are not GB-demand service and do
> not enter the denominator (their settlement is rule 8); external-
> zone demand never enters it — Stage 7 costs the GB system, with the
> boundary settled per rule 8. Code carries the two "delivered"
> objects under distinct names.

### Edit 5 — Rule 8 import costing: precondition mis-stated, export
### side missing, understatement direction unnamed

"where Stage 5 zones exist" is wrong as a trigger: the zones exist
NOW, but multi-zone SRMC does not — "pricing and sweep are single-zone
only today (multizone.rs has no SMP wiring …)" is a recorded code fact
(memory/project-state.md, frozen-imports entry), and the priced ladder
is the Stage 7-adjacent target (docs/04:215–218). As written the rule
directs the implementer to a machine that doesn't exist. The rule is
also silent on export credits (an unreviewed convention choice), and
omits the known direction of error. Replace the first bullet's opening
clause with:

> modelled import energy is costed at the modelled exporter-zone SRMC
> **once multi-zone SRMC exists (the Stage 7 priced ladder — pricing
> is single-zone-only today, a recorded code fact)**, else at a cited
> reference price; exports are credited under the same convention
> imports are costed, symmetrically and stated (a no-credit variant is
> a legitimate conservative sensitivity, labelled); the model's £0
> must-take floor and missing negative/scarcity prices mean modelled
> exporter SRMC systematically UNDERSTATES real import cost in surplus
> periods — direction stated wherever import costs are quoted;

### Edit 6 — Pathway-year cost aggregation is unpinned

For FES pathway runs (Stage 6 part 2 precedent, in Stage 7 scope),
nothing says whether "the cost of pathway year Y" is an annualised
snapshot or a discounted pathway total. Add a third bullet to rule 8:

> - **Pathway years:** the cost of pathway year Y is the annualised
>   snapshot — every asset alive in Y carries its rule-4 annuity
>   (assets added during the pathway included from their build year)
>   plus year-Y operating costs. No discounted pathway-total (NPV)
>   headline; a pathway NPV may appear only as a labelled derived
>   figure with its discount rate stated.

### Edit 7 — Uniform-vs-per-technology WACC is a methods choice, not
### a TBD-DATA number — it belongs in D8

Rule 4 defers "the exact set" to TBD-DATA, correctly — but whether the
set applies uniformly across technologies or per-tech re-ranks
portfolios by assumption and is precisely the kind of contestable
convention this note exists to pin. Append to rule 4:

> The three-WACC set is applied **uniformly across technologies** by
> default — per-technology WACCs embed risk-premium assumptions that
> re-rank portfolios by fiat. A per-tech-WACC variant is a labelled
> sensitivity requiring cited per-tech rates from the evidence
> package, never the headline.

### Edit 8 — The Stage 2 capture-validation caveat must propagate to
### the rule 6 bridges

The Stage 2 run report (§2) records: the capture gate passed on thin
validation content; model SMP has almost no within-day shape and no
negative prices; "revenue-sensitive downstream results (Stage 7
capture-price economics) should not lean on this pass alone." Rule 6
publishes exactly those objects (value factors = delivered-basis
capture ratios) with no carry-over of that caveat. Append to rule 6:

> Bridge caveat, carried from the Stage 2 record: model SMP has almost
> no within-day shape and no negative prices, and the Stage 2 capture
> gate passed on thin validation content (stage-2 run report §2 —
> Stage 7 capture-price economics must not lean on that pass alone).
> Value-factor and capture-price bridges carry this caveat until the
> priced ladder re-validates capture.

### Edit 9 — Name and record the docs/04 scope-wording reconciliation

docs/04:290–292 lists "overbuild, curtailment" as cost-stack members;
rule 1 gives them no £ line. The draft's double-counting argument is
CORRECT (curtailment's resource cost is the capex/O&M of the capacity
that produced it — lines 1/3; a separate £ line double-counts; a
lost-revenue "curtailment cost" is a transfer, not a resource cost),
but the note must say explicitly that it is interpreting docs/04, or
the divergence reads as unauthorised scope drift and the "Σ components
= total" test is ambiguous about its component list. Append to the
curtailment/overbuild paragraph:

> This ruling interprets docs/04 Stage 7's scope list ("… storage
> capex, overbuild, curtailment, interconnection …") as satisfied by
> (i) the capex/O&M embodiment in lines 1/3, (ii) the reported
> quantities, and (iii) named terms in the rule-6a identity — not as
> separate £ lines. docs/04 is not edited; the interpretation is
> recorded here and binds the "Σ components = total" acceptance test
> to the rule-1 line list.

---

## Non-blocking notes of record

1. **Deferral list additions (data-engineer package):** pathway
   fuel/carbon price trajectories (source per the opponent's-defaults
   principle, docs/01) and the capex basis definition (overnight vs
   IDC-inclusive — source-dependent) should be named on the TBD-DATA
   list. Nothing currently on the list belongs in D8 itself; with
   edit 7, nothing in D8 belongs on the list. Correctly scoped
   otherwise.
2. **Standard-strictness disclosure:** zero unserved over 1985–2024 is
   stricter than the GB reliability standard (LOLE 3 h/yr). When a
   Stage 7 number is compared with an official cost claim, the
   standard mismatch should be named. Fold into rule 3 wording at
   editorial discretion.
3. **LFSCOE:** "where cheap to do so" is acceptably optional, but if
   computed, the exact definitional convention used (Idel's) must be
   cited on the artefact — bridges obey the same definitional-honesty
   regime as headlines.
4. **ADR-6 both-policy reporting:** the interactions bullet ("must say
   which policy") is weaker than ADR-6 ("results for headline claims
   are reported under both"). Headline cost claims on
   storage-material scenarios report both policies with the gap;
   restate in the interactions section.
5. **Rule 2 test design:** "Σ = total, bit-exact" is only a real test
   if the reconciliation recomputes the component lines independently
   of the code path that writes the total — otherwise total := Σ makes
   it a tautology. Test-design note for the implementer, not a note
   defect.

## Verified against the record (not trusted)

- Ratified basis fully and faithfully transcribed; no relitigation, no
  additions beyond it except the pinning work D8 is for
  (memory/project-state.md, "RICHARD'S DECISIONS 2026-07-03" and
  handoff item 4).
- Stage 7 scope/acceptance list: docs/04-implementation-plan.md:288–306
  (scope 290–295; LP tests 297–299; cost-stack + Q9 test 300–301).
- Q9 labels and Module 7 wording: docs/07-research-syllabus.md:103–105,
  56–60.
- ADR-6 dual-policy finding, ADR-9 annuitised-capex placement, ADR-12
  constraint approximation: docs/02-architecture.md:60–71, 95–101,
  119–123. The draft's rule 1.6 D6 quarantine is consistent with the
  open D6 row (docs/08-risks-and-decisions.md:14).
- Priced-ladder status: the superseded A2 ≥95 % pin is the Stage 7
  priced-ladder target (docs/04:215–218); single-zone-only pricing is
  a recorded code fact (memory/project-state.md, frozen-imports entry)
  — basis of edit 5.
- Package A delivered-basis ruling and Q10 wording rule:
  docs/notes/package-a-delivered-basis-review.md §4(d)–(e) — basis of
  edit 4's two-objects distinction and consistent with rule 6's
  "capture ratios = Hirth value factors" claim (correct on the
  delivered basis).
- Stage 2 capture caveat text: docs/notes/stage-2-2024-run-report.md
  §2 ("should not lean on this pass alone") — basis of edit 8.
- Emissions-factor gap wording in rule 9 matches the tracked deviation
  (memory/project-state.md, tracked deviations; stage-2 report §2).
- D4 Q3 limitation as cited in the interactions section matches
  docs/notes/d4-rule-based-dispatch.md rule 3.
- Zero-unserved default matches the Stage 3 solve convention
  (`min_storage_for_zero_unserved`, docs/04:123–138) and Module 4's
  zero-unserved frame (docs/07:35–39).
- docs/08 D8 row (uncommitted diff): accurate as a status row; after
  the edits land it must flip to Resolved citing this review, per the
  D4/D5 row pattern.
