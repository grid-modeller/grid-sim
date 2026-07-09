# Three-zone Scottish-boundary design — adversarial adjudication

Reviewer, 2026-07-04. Adjudicating the data-engineer's three-zone
recommendation (`docs/notes/scottish-group-boundary-scoping.md`,
committed) BEFORE any data or engine work — the D8/D9/D11 precedent
(contestable modelling design is adjudicated in prose first). Richard
ratified the three-zone direction 2026-07-04; this review gates the
DESIGN, not the direction. Everything below was cross-checked against
the committed B6 package (`b6-two-zone-data-report.md`, its two reviews,
`scenarios/gb-2024-2zone.toml`, `grid-adequacy/src/{multizone.rs,
flow.rs}`, `grid-core/src/scenario.rs`, ADR-7/12) and the beta-readiness
audit block in `memory/project-state.md`.

## VERDICT: ADOPT-WITH-EDITS

The three-zone N-Scotland / S-Scotland / E+W split with a B4 link and a
B6 link is the correct upgrade: it is the minimal geometry that puts the
binding upstream gate (B4) on a link that throttles the correct pool
(the ~7 GW northern surplus) while preserving B6 as the combined exit,
and the schema/engine genuinely support it with no new concept. The
cascade argument (item 1) is sound and the geography is load-bearing and
correctly placed (item 2). BUT three material design defects must be
corrected in the scoping note and carried as binding obligations into
the data + scenario + engine work orders before implementation, or the
model will repeat the exact over-attribution error the beta audit just
refuted on the B6 model:

1. the doubled circularity/identity wedges create an unfalsifiable
   tuning surface on the B4 link, which has NO annual-outturn
   cross-anchor (item 3 — highest risk);
2. the single-pass rule-based flow rule, run across TWO internal
   borders with S-Scotland as a hub, COMPOUNDS the dispatch-convention
   artefact that contaminated and inverted the B6 storage-sensitivity
   magnitude (item 5 — critical);
3. the note's "adequate representation" framing overclaims: with B5
   unmodelled and the single-pass hub-staleness live, three zones is a
   TIGHTER LOWER BOUND, not an adequate or complete one (items 1, 3, 5).

The numbered edits below are ordered with exact replacement text where
a wording change is required. The rulings on items 3, 5 and 7 are stated
explicitly, as requested.

---

## Item 1 — the cascade argument: SOUND, with two named failure modes

**The cascade is correct and the geography is load-bearing.** Verified
from the scoping's own arithmetic and the review-confirmed cost/flow
numbers: 7.00 of 13.15 GW of Scottish wind (53%, 94% of offshore) sits
north of B4; B4's observed median limit is 1.8 GW (35.8% binding), B6's
4.1 GW (23.6% binding). ~7 GW of northern wind behind a ~1.8 GW wall is
a real physical throttle that a single B6 link — which pools all
Scottish wind into one copper-plate zone reaching B6 freely — cannot
represent. B6 carries MORE energy (22.63 vs 15.78 TWh) precisely because
it aggregates B4's throughput plus southern-Scotland generation: B4 is
the tighter, more-often-binding UPSTREAM gate; B6 is the higher-
throughput EXIT. Both bind; the two-stage series cascade is real; no
single scalar link represents it. This is adjudicated CORRECT.

**Failure mode A — B5 folded into the S-Scotland copper-plate (which
way does it bias?).** B5 (3.9 GW, Denny-Lambhill, within SPT, between B4
and B6) is TIGHTER than B6 (6.7 GW) and comparable to B4, and has no
open anchor. Folding it into the S-Scotland copper-plate assumes NO
constraint between the B4 exit and the B6 entry, so the model lets more
energy flow freely within S-Scotland than reality → it UNDERSTATES the
S-Scotland-internal constraint → the three-zone result REMAINS a lower
bound (same direction as the B6 model's ruling). Direction of bias:
toward under-stating constraint. This is acceptable AS a stated lower-
bound simplification, but it means the note's "adequate" language is
wrong (see Edit 1).

**Failure mode B — single-pass hub-staleness.** With N-S-EW as a linear
chain, S-Scotland is a hub touched by BOTH borders. The flow rule is a
single sequential pass (flow.rs rule 6): whichever border dispatches
second moves S-Scotland after the first equalised against it, leaving
the first border marginally over-dispatched. The scoping does not
mention border order at all — an omission (see Edit 4). This bounds what
the model may quote to direction + pinned totals, never a clean
B4-vs-B6 decomposition (item 5 ruling).

---

## Item 2 — zone-boundary placement (N=710k proxy): ADOPT WITH SENSITIVITY

The horizontal N=710k northing proxy for the B4 (Tealing-Westfield) line
is defensible for the AGGREGATE wind split: it is named-station
validated (Seagreen 749k / Peterhead 846k / Moray north; Whitelee /
Clyde / Torness / Crystal Rig south) and stable across 700/710/720k
(onshore N-of-B4 moves only 4,156→3,900 MW, ~250 MW). ADOPTED as the v1
placement.

**Large-unit placement — checked, one headline-relevant flag.** Torness
(nuclear 1.19 GW, N≈674k) sits 36k south of the line → S-Scotland:
unambiguous, electrically SPT. Peterhead (CCGT 1.18 GW, N≈846k) →
N-Scotland: unambiguous, electrically SSEN. Neither swings on the proxy.
**Pumped storage (0.74 GW, Cruachan+Foyers) → N-Scotland is the flagged
case:** Foyers (Loch Ness) is unambiguously north, but Cruachan (Loch
Awe, N≈727k, only ~17k north of the line) sits in exactly the Argyll
fringe the scoping itself concedes the horizontal line approximates
poorly (the SSEN↔SPT interface dips around Argyll). This matters because
PS placement feeds the HEADLINE storage-sensitivity finding (northern
recharge is the boundary-limited quantity). Cruachan is SSEN-connected,
so N-Scotland is defensible on BOTH northing and electrical grounds — but
because it is headline-load-bearing, its placement must be stated
explicitly and a sensitivity swapping Cruachan (440 MW PS) N↔S must be
run in the engine package (see Edit 3). The proxy stands; the caveat is
mandatory.

---

## Item 3 — CIRCULARITY / IDENTITY WEDGES (highest risk): RULING

**The ruling.** A three-zone GB carries strictly MORE identity wedges
than the two-zone model, and — decisively — the new wedges land on the
ONE link (B4) that has NO annual-outturn cross-anchor. This is the exact
structural condition that produced the B6 over-claim (an unseparated
term the validation could not falsify). The design is ADOPTED only with
the following binding guards; without them the B4 link is an
unfalsifiable tuning surface.

**The new wedges, named.** The two-zone model already carried: the flat
10.1% demand split, the offshore-commissioning wedge (~3 TWh), the
CF-ordering artefact, and the ~2 TWh DA-vs-outturn wedge on B6. Three
zones ADD, on top:

- (w1) **N-Scotland demand split (~3% of GB).** There is no clean
  published N-of-B4 half-hourly demand series. Energy Trends gives
  Scotland 9.8%, not an N/S-of-B4 split; DESNZ subnational is by local
  authority and must be assigned across the 710k line; the only
  half-hourly source (Elexon P114 GSP-group `_P`=SSE/north,
  `_N`=SP/south) is explicitly NOT fetched in v1. So the N-Scotland
  demand LEVEL is a rough exogenous estimate and its SHAPE is absent.
- (w2) **N-Scotland generation-unrepresented / CF sub-cluster term.**
  The N-Scotland residual = N-demand − N-generation feeds the B4 flow,
  and the ~3 TWh offshore-commissioning overstatement lands
  DISPROPORTIONATELY on B4 (94% of Scottish offshore is north of B4).
  Against B4's 15.78 TWh DA anchor that is a ~19% wedge — with NO
  outturn cross-anchor to catch it (unlike B6's 17 TWh Energy Trends
  anchor).

**Can they be pinned cleanly? Partly — and only under these guards:**

(a) **FORBID tuning the N/S demand split OR the CF sub-cluster partition
    to reproduce the B4 DA series (15.78 TWh / 35.8% binding).** These
    two knobs must be pinned to a STATED exogenous basis BEFORE the run
    (demand: the named DESNZ-subnational / P114 basis; CF: the cluster
    re-assignment reconciled to the REPD-northing capacity split — see
    item 6), and the resulting B4 net-flow MISS reported as a wedge, not
    closed by adjustment. This is the direct analogue of the B6 ruling's
    "the link capability must never be tuned to reproduce the group
    cost." Any retuning of w1/w2 to hit B4 is the unfalsifiable-tuning
    trap and is out of bounds.

(b) For the B4 flow MAGNITUDE, w1 (demand) is second-order: N-Scotland
    is heavily export-dependent (~1 GW mean demand vs ~7 GW wind
    swings), so a ±1% GB demand error (~0.35 GW) is small against the
    generation term. This is defensible and must be STATED as the reason
    the crude demand split is tolerable for the flow gate — not left
    implicit.

(c) w2 (the offshore wedge) is NOT second-order and has no cross-anchor.
    The B4 net-flow tolerance must be set WIDE enough to absorb the
    ~3 TWh commissioning wedge plus the demand-split uncertainty, and
    reported as a decomposed wedge budget (the B6 gate-(i) precedent),
    never as a tight validated magnitude.

**Consequence for what B4 may claim:** direction + binding-frequency,
yes; net-flow magnitude only within the stated wedge, with "DA-only, no
outturn cross-anchor" attached. See item 4.

---

## Item 4 — validation anchors: two gates WELL-POSED, B4 acceptance ruled

Two three-part gates (correlation + net + binding-frequency), one per
link, are well-posed AND non-double-counting: SCOTEX physically IS the
S-Scotland→England (SPT→NGET) boundary, so B6 still anchors the S-EW
link correctly; the 17 TWh Energy Trends outturn (all-Scotland net
export) still equals the B6 EXIT flow in the three-zone geometry (all
Scottish export exits via B6), so it anchors B6-exit with the existing
~2 TWh wedge and does NOT double-count against B4. Clean.

**B4 acceptance criterion (ruled).** B4 is an INTERNAL flow with only
the stitched `SSE-SP`+`SSE-SP2` DA series — no annual-outturn cross-
anchor. Its gate is therefore:

- correlation floor (tripwire, first-pass-calibrated, B6 precedent) and
  binding-frequency vs 35.8% ± band: these are the LOAD-BEARING B4
  gates and are honest to quote as validated;
- net southward flow vs 15.78 TWh carried as a WEDGE BUDGET wide enough
  to absorb the ~3 TWh offshore-commissioning wedge (item 3c) + the
  demand-split term — NOT a tight magnitude gate;
- the stitch (`SSE-SP`→`SSE-SP2` mid-year version change) MUST replicate
  the B6 builder's clock-change / repeated-hour / sentinel handling
  byte-for-byte (the scoping commits to this in §7 — hold it).

**Is a DA-only-validated internal link honest enough to quote?** YES,
for DIRECTION and binding behaviour ("B4 binds ~36% of periods; northern
wind is double-gated"); NO for a precise validated net-flow magnitude.
Every B4 quote carries "DA-only anchor, no independent annual outturn;
net flow consistent within a ~3-4 TWh commissioning+demand wedge." This
caveat is mandatory and must be written into the scenario header and the
run report exactly as the B6 lower-bound duty was.

---

## Item 5 — DISPATCH-CONVENTION TRAP (critical): RULING

**The ruling. The three-zone model COMPOUNDS the artefact, and may quote
ONLY direction + pinned totals under stated conventions. No "B4 effect
proper" percentage, and no clean B4-vs-B6 decomposition, may be quoted —
this is a hard prohibition, the exact error the beta audit just refuted
on the B6 model.**

**Why it compounds (mechanism, verified against flow.rs + the B6
audit).** The rule-based flow rule (flow.rs rules 1-3, multizone.rs
step 2 before step 3) clears flows BEFORE storage, blind to store
headroom, and in the negative-price region trades two surplus zones
toward equal surplus DEPTH in GW. The B6 audit measured the consequence:
net B6 flow ran 20-53 TWh/yr NORTHWARD at copper-plate (RGB's
absolutely-deeper surplus shipped into Scotland whose store charges at
only 10.1 GW), and this contaminated the storage-sensitivity magnitude
so badly it INVERTED (B6 33,056 < copper 33,632 at the 3% placement —
impossible under optimal dispatch). A three-zone model runs the SAME
rule across TWO borders with S-Scotland as a hub. The identical
equal-depth mechanism can now push RGB surplus north across B6 into
S-Scotland AND then further north across B4 into N-Scotland — English
wind charging Scottish stores through two constraint boundaries the
wrong way. The artefact does not merely repeat; it chains. Additionally
the single-pass hub-staleness (item 1B) means one of the two borders is
always marginally stale, so the per-border split of the total effect is
convention-dependent and not physical.

**What the three-zone model MAY quote:**
- the raw total-delta DIRECTION (Scottish curtailment UP, storage
  requirement UP, Q10 Scottish capture DOWN, vs the B6-only and copper-
  plate baselines) — the only thing that survived the B6 audit;
- PINNED TOTALS under fully stated conventions (fleet shares, boundary
  capabilities, store placement, "rule-based dispatch, upper-bias" +
  "B4/B5 lower bound" dual duty);
- the B4 BINDING FREQUENCY and at-cap curtailment classification (the
  observed-anchored quantities).

**What it MAY NOT quote:**
- any "B4 effect proper" or "B4-attributable %" — FORBIDDEN;
- any clean B4-vs-B6 or boundary-vs-dispatch decomposition percentage;
- any storage-sensitivity magnitude presented as a boundary effect
  rather than a convention-conditioned total.

The LP (Stage 7, ADR-10) is the named resolver that separates boundary
from dispatch convention; until then the three-zone result is framed as
direction + pinned totals, per the §1d B6 framing precedent applied
verbatim.

---

## Item 6 — scope / cost: schema+engine claim HOLDS; two under-scoped items

**Schema v6 + engine support 3 zones + 2 links with no engine change —
CONFIRMED.** `Scenario` holds `Vec<ZoneSpec>` + `Vec<LinkSpec>`;
`scenario.validate()` is zone-count-generic; `multizone.rs` builds
`borders` by grouping links by unordered zone pair in first-appearance
order and iterates zones/borders generically (the only index into a
specific zone is `zones[0]` for the horizon anchor — generic). `flow.rs`
is untouched and untouchable by this work: capability was always its
parameter. Two borders (B4: N→S; B6: S→EW) are two distinct zone pairs,
dispatched sequentially. No new engine concept. The scope claim is TRUE.

**Genuinely new work, correctly scoped:** the stitched
`b4_da_flows_limits` builder; the 3-zone scenario; per-boundary A1/A2-
style gates + robustness. Fine.

**UNDER-SCOPED item 6a — the CF sub-cluster split is NOT a free
re-partition.** The scoping §2(iii) claims "the pinned cluster members
carry coordinates, so this is a re-partition of the existing cluster
lists, not new ERA5 fetching." Inspecting `derive_cf_gb2zone.py`: the
`sco` traces are built from NAMED REGIONAL clusters (offshore
`moray_firth`+`forth_tay`; onshore `highlands`/`central_belt`/
`southern_uplands`), assigned WHOLE. The N/S-of-B4 split therefore
re-assigns whole clusters (highlands→N; central_belt+southern_uplands→S)
— clean for onshore — BUT the offshore `forth_tay` cluster STRADDLES B4
(Seagreen ~749k north; Firth of Forth south), so N-of-B4 offshore is a
WITHIN-cluster split that may require descending below the pinned cluster
granularity (back to per-point ERA5), which the scoping's "not new
fetching" claim understates. The data package must verify `forth_tay` is
point-resolved before asserting a re-partition; if it is a pre-averaged
trace, this is real ERA5 work, not a re-slice.

**UNDER-SCOPED item 6b — fleet-capacity split vs CF-trace split
consistency.** The fleet N/S split (§1 REPD-northing, 41% onshore north)
and the CF-trace N/S split (cluster re-assignment) are TWO independent
partitions of the same fleet, and nothing in the scoping reconciles
them. This is the identical failure class the B6 package was CORRECTED
for at review (cluster share 73.6% vs DESNZ 70.0% onshore → +3.5 pp
anti-conservative energy overstatement). The data package MUST reconcile
the CF cluster-weight N/S shares to the REPD-northing capacity shares
(as B6 reconciled onshore to DESNZ), report the residual GB-energy cost,
and carry the anti-conservative caveat if it keeps the cluster split.
Flag both 6a and 6b as data-package pre-commit conditions.

---

## Item 7 — ADR-7 zone-count interaction: RULING

**The ruling. "GB single-zone → three GB-internal zones" is a CLEAN
ADR-7 amendment, but it must state that GB-INTERNAL multizone and
CONTINENTAL-EXTERNAL multizone are SEPARATE SCENARIO FAMILIES in v1.**

The schema treats all zones uniformly (`Vec<ZoneSpec>`), so the
internal/external distinction is a MODELLING convention, not a schema
concept — the amendment is clean at the schema level. On the combinatorics:
a combined scenario would be 3 GB-internal + 5 external = 8 zones with a
sparse link matrix; the engine is O(zones × periods × borders) and 8
zones is computationally trivial. So there is NO combinatorial or
computational barrier. The reason to keep them SEPARATE families in v1 is
methodological, not performance:

- the two studies have different purposes (GB-internal constraint vs
  import-emergence) and different convention regimes: the 2-zone/3-zone
  GB scenario treats external interconnectors as EXOGENOUS observed 2024
  net-import traces (gb-2024-2zone.toml, explicit), whereas the 5-zone
  continental scenario MODELS the import response. Mixing them conflates
  two regimes in one run;
- combining them stacks every external zone's imports-identity wedge on
  top of the DOUBLED internal wedges (item 3), multiplying the
  unfalsifiable surface;
- Moyle lands at Auchencrosh (Ayrshire, SPT/south-of-B4) → in the
  three-zone geometry Moyle attaches to S-Scotland, not N-Scotland — a
  routing the combined scenario would have to get right, another degree
  of freedom best deferred.

So the ADR-7 amendment (proposed in `memory/project-state.md`, to be
written into docs/02 as an amendment note, NOT a silent ADR edit) should
read: GB may be split into internal zones (N-Scotland / S-Scotland /
E+W) for the boundary study; GB-internal and continental-external
multizone are separate scenario families in v1; unification awaits the
LP (Stage 7) replacing the single-pass flow rule (which is what makes
the hub-staleness and northward-shuffle artefacts, items 1B/5, quotable
as clean effects). This is combinatorially and computationally sane; it
is the CONVENTION mixing, not the zone count, that forces separation.

---

## NUMBERED EDITS (ordered; exact replacement text where specified)

**Edit 1 (MANDATORY) — remove the "adequate representation" overclaim.**
The note calls option (b) "the adequate (whole-Scottish-group)
restriction" (header), "the adequate representation" (§3 RECOMMENDATION,
§3 Bottom line) and titles §3 "Physical justification for adequacy."
With B5 unmodelled (folded into S-Scotland) and the single-pass hub-
staleness live, three zones is a TIGHTER LOWER BOUND, not adequate.
Replace, in §3 "Bottom line," the sentence:
> "(b) is the adequate representation; (a) is a bracket, not a fix."
with:
> "(b) is a materially tighter lower bound than (a): it moves the
> dominant B4 term from structurally invisible to modelled, but with B5
> unmodelled (folded into S-Scotland) and the single-pass flow rule's
> hub-staleness live, it remains a LOWER BOUND on the Scottish
> constraint phenomenon, not an adequate or complete representation. It
> inherits the B6 model's lower-bound duty; it does not discharge it."
And retitle the §3 subhead "Physical justification for adequacy" to
"Physical justification (why (b) is the tighter lower bound)."

**Edit 2 (MANDATORY) — add the unfalsifiable-tuning guard to §4.**
Append to §4 (validation anchors) a new paragraph, exact text:
> "GUARD (item 3 ruling, design review): the N/S-of-B4 demand split and
> the CF sub-cluster partition are pinned to their stated exogenous
> bases BEFORE any run and are NEVER retuned to reproduce the B4 DA
> series (15.78 TWh / 35.8% binding). The B4 net-flow miss is reported
> as a wedge budget (absorbing the ~3 TWh offshore-commissioning wedge
> that lands disproportionately on B4, plus the demand-split term), not
> closed by adjustment — the direct analogue of the B6 ruling's 'the
> link capability must never be tuned to reproduce the group cost.' The
> B4 gate quotes correlation + binding frequency as validated and net
> flow only within the stated wedge, tagged 'DA-only, no outturn
> cross-anchor.'"

**Edit 3 (MANDATORY) — Cruachan/PS placement flag in §1 and §5.**
In §1 (Conventional plant placement) add: "Cruachan (Loch Awe, N≈727k,
~17k north of the 710k line) sits in the Argyll fringe where the
horizontal proxy is weakest; it is SSEN-connected so N-Scotland is
defensible on both grounds, but because pumped-storage placement feeds
the headline storage-sensitivity finding, a sensitivity swapping
Cruachan (440 MW) N↔S is required in the engine package." Cross-
reference this from §5 (storage sensitivity).

**Edit 4 (MANDATORY) — border order + single-pass staleness in §3.**
The note is silent on border order; flow.rs rule 6 makes it load-
bearing (S-Scotland is a hub). Add to §3 (RECOMMENDATION) a sentence:
> "The scenario must state and justify the `[[links]]` border order (the
> cascade is physically N→B4→S→B6→E+W; whichever border dispatches
> second leaves the first marginally over-dispatched — flow.rs rule 6,
> single pass). The run report bounds this staleness and NEVER quotes a
> clean B4-vs-B6 decomposition (design-review item 5)."

**Edit 5 (MANDATORY) — dispatch-convention prohibition, verbatim into
§5.** Replace the §5 storage-sensitivity bullet's closing "magnitude
requires the three-zone run to quantify" with:
> "magnitude is convention-conditioned and NOT separately quotable: the
> rule-based flow rule runs across two borders with S-Scotland as a hub
> and COMPOUNDS the equal-depth northward-shuffle artefact that inverted
> the B6 storage-sensitivity magnitude at some placements (beta audit).
> The three-zone model quotes DIRECTION (up) and PINNED TOTALS under
> stated conventions only — no 'B4 effect proper' %, no B4-vs-B6 or
> boundary-vs-dispatch decomposition. The LP (Stage 7) is the named
> resolver. Framing follows the §1d B6 precedent verbatim."

**Edit 6 (MANDATORY, data-package pre-commit conditions) — add to §6.**
Add two items to §6 "What could not be sourced openly":
> "- CF sub-cluster point-resolution. `forth_tay` (offshore) straddles
>   B4 (Seagreen north / Firth of Forth south); the N/S split is a
>   within-cluster split, not the clean whole-cluster re-partition §2
>   implies. Verify `forth_tay` is point-resolved before claiming 'not
>   new ERA5 fetching.'
> - Fleet-vs-CF split consistency. The REPD-northing capacity split
>   (41% onshore north) and the CF cluster-re-assignment split are
>   independent partitions; they must be reconciled (the B6 DESNZ-vs-
>   cluster correction precedent) with the residual GB-energy cost
>   reported, or the anti-conservative caveat travels."

**Edit 7 (RECOMMENDED) — ADR-7 amendment wording.** When the ADR-7
amendment is written into docs/02 (as an amendment note, not a silent
edit), state per item 7: GB internal zones are a distinct scenario
family from the continental external-zone list in v1; unification awaits
the Stage 7 LP. The scoping's §7-equivalent should record this so the
scenario package does not attempt an 8-zone combined run.

---

## Rulings summary (as requested)

- **Item 3 (circularity):** ADOPT with binding guards. The B4 link's
  new wedges (N-Scotland demand split; offshore-commissioning term
  landing ~19% on B4) create an unfalsifiable tuning surface because B4
  has no outturn cross-anchor. FORBID retuning the demand split or CF
  partition to the B4 DA series; pin both to stated exogenous bases
  pre-run; report the B4 net-flow miss as a wedge budget. Demand-split
  error is second-order for the B4 flow magnitude (state why); the
  offshore wedge is not and must be absorbed in tolerance, not tuned out.

- **Item 5 (dispatch convention):** The three-zone model may quote
  DIRECTION + PINNED TOTALS under stated conventions ONLY. It may NOT
  quote any "B4 effect proper" %, any B4-vs-B6 decomposition, or any
  storage-sensitivity magnitude framed as a boundary effect. The
  single-pass rule across two hub-sharing borders COMPOUNDS the
  equal-depth northward-shuffle artefact that inverted the B6 magnitude;
  the LP (Stage 7) is the named resolver. Framing = §1d B6 precedent
  verbatim. Hard prohibition — this is the beta-audit error.

- **Item 7 (zone-count interaction):** Clean ADR-7 amendment. 8 zones
  (3 GB-internal + 5 external) is combinatorially and computationally
  sane, but GB-internal and continental-external multizone must be
  SEPARATE scenario families in v1 — the reason is convention mixing
  (exogenous vs modelled imports; stacked identity wedges; Moyle→
  S-Scotland routing), not zone count. Unify only when the Stage 7 LP
  replaces the single-pass flow rule.

Report path: `docs/notes/scottish-group-boundary-design-review.md`
