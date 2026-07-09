# D10 adjudication — EV/transport demand overlay design note (reviewer)

Design adjudicator (D10 gate), 2026-07-06. Subject: the uncommitted
`docs/notes/d10-ev-overlay.md` (supervisor draft, DRAFT status) plus its
proposed docs/08 D10 row. This is the D9/D11/D13-precedent design gate:
docs only, no schema or engine code until adjudicated. Every load-bearing
claim below was verified against ground truth, not trusted: the committed
`data/reference/pathways-published.toml` (ED1 rows), the committed
`grid-core/src/scenario.rs` (v7 `DemandSpec`, the `dsr` StorageKind
fields and their Q6-provisional doc), the FES EE scenario headers, the
adopted D9 note + review, `data/reference/heating-cop.toml`, the Stage 7
scenarios review §B/§C, docs/02 ADR-8, and the docs/08 D10/D15 rows.

## VERDICT: ADOPT-WITH-EDITS

The skeleton is right and genuinely the D9 architecture, not a cargo
cult: demand-side transformation only; quantum-plus-portfolio schema;
record-mean pinned normalisation (Ruling B correctly transferred — see
§3); the shared `T_pop` covariance stated as physics; carve-out of the
target, not the shape; V2G-lite emitted as the existing ADR-8 `dsr`
pseudo-storage with the Q6 dependency named. The double-count rule is
arithmetically sound for D10's own quantum. But the draft has one
first-order construction defect — the smart allocator as specified
(greedy lowest-R fill at the power cap) does NOT make acceptance test
8.3 a theorem, and a flat-residual counterexample breaks it — plus an
over-claim on the joint D9+D10 carve-out (the D9 side is heat-basis and
under-determined by the pinned ED1 rows; "one rule pinned for both" is
not yet true as written), an unstated realised-vs-quantum wrinkle on
single-weather-year FES scenarios, and a GB/UK scope mislabel on
VEH0101. Seven ordered edits; the seven flagged forks are all ruled
below. Apply all seven edits, then the note is ADOPTED and the docs/08
D10 row updates per rule 11 (with the rule-11 text amended by edits 1–2).

---

## §1 The double-count rule — arithmetic VERIFIED for D10; joint claim
## with D9 OVER-STATED (edits 2, 3)

Verified against ground truth:
- ED1 components bit-match the committed reference: residential EVs
  41.488 / 54.370, commercial EVs 31.404 / 77.378, road-EV totals
  72.892 / 131.748 TWh (pathways-published.toml lines 181–182,
  278–279). Shares: 41.488/72.892 = 0.56917 → 0.5692 and
  31.404/72.892 = 0.43083 → 0.4308; the 4-dp pair sums to 1.0 well
  inside the 1e-9 share-sum law. Correct: these are the data file's
  own numbers.
- The committed demand identity is `demand(t) = (base(t) + Σ extras(t))
  × annual_scale + extra_demand_gw + heating(t)` (scenario.rs:985) and
  the draft's extension appends `+ ev(t)` outside `annual_scale`,
  exactly as `heating(t)` sits. Retargeting `annual_scale` to
  `(total − Σ overlay quanta)/trace_total` and adding the overlay
  quanta back composes exactly and is order-independent — the
  carve-out is a single subtraction of the sum, so D9-then-D10 and
  D10-then-D9 are the same arithmetic. The rule is well-defined and
  testable FOR QUANTA THAT ARE ELECTRICAL SCENARIO INPUTS.
- Defect (edit 2): D10's quantum is exactly that — `annual_energy_twh`
  is electrical and equals an ED1 component. D9's is NOT: the scenario
  input is `delivered_heat_twh` (heat-side); the electrical energy
  added to demand is `share × heat(t)/COP(t)` — a derived,
  portfolio-dependent output. And the pinned reference carries ONLY
  residential heat markers (residential_heat_pumps_twh 18.223/58.505,
  residential_resistive_heat_twh 3.478/14.643 — no commercial-heat
  row exists in the extraction). So rule 2's "pinned here for BOTH
  overlays so the FES reprofiling is done once, consistently" and test
  8.2's "(+ D9 quantum where applied)" are under-determined on the D9
  side as written. The D10 carve-out is adopted; the D9 FES carve-out
  needs its own adjudicated convention (which ED1 rows constitute the
  carved component; the heat-to-electrical inversion computed once
  from pinned inputs, never per-year).
- The 2024-embedded-EV layer (~5 TWh, magnitude to be pinned from
  ECUK/DUKES — data requirement 7): RULED ACCEPTABLE as "stated, not
  fixed". Under carve-out-of-the-target the total identity still
  holds; the embedded load is a SHAPE wrinkle of order 1–2% of the
  trace, the same caveat class as D9 rule 6's "2024 non-heat demand
  tiling". No quantified bias direction is required at that scale —
  but the magnitude pin is mandatory (already ordered by the draft)
  and the caveat travels on FES-reprofiled quotes.
- The fiscal-year basis note and the "arithmetic consistency, not
  NESO's profile" claim ceiling: correct and adequate (matches the
  stage7 data report's stated wrinkle).

## §2 The smart-signal definition — computable VERIFIED; the theorem
## claim as constructed is FALSE (edit 1, the most important item)

Computability/circularity: verified sound. `R(t) = demand-before-EV −
Σ must-take (capacity × CF trace)` is a pure function of scenario
inputs — base trace, extras, `annual_scale`, `extra_demand_gw`,
`heating(t)` (itself a pure pre-dispatch function of the pinned
`T_pop`), and the fleet's CF traces. Nothing dispatch-produced enters
it; the draft's refusal of price-following (post-dispatch layer,
genuinely circular) and storage-awareness (dispatch output) is
correct. Interconnector-blindness stated. Determinism and ADR-5
composability hold.

The theorem claim does not, as constructed. Rule 5 allocates by
"greedy lowest-`R` fill subject to the segment's aggregate power cap",
and rule 8.3 asserts red-first that smart's maximum post-EV residual
≤ dumb's, "valid by the rule-5 construction (smart's feasible set
contains the dumb allocation)". Feasible-set containment gives that
theorem only for an allocator that MINIMISES the maximum over the
feasible set. Greedy cap-chunk filling does not: counterexample — a
flat `R` over the window, cap = fleet aggregate power. Greedy dumps
`cap` into the first half-hours: peak = `R + cap`. The dumb
convolution staggers arrivals, so its instantaneous aggregate power is
strictly below `cap`: dumb peak = `R + max dumb power < R + cap`.
Smart WORSE than dumb; test 8.3 fails; the bracket direction inverts
by construction artefact, not physics. Edit 1 replaces the allocator
with the level/water-fill (min-max) allocation, which restores the
theorem genuinely and stays deterministic.

Fork 1 (containment convention) is ruled in edit 1: containment is
MANDATORY and MACHINE-CHECKED, never assumed.

## §3 Temperature derating — Ruling-B transfer VALID; Geotab
## discipline ACCEPTABLE

The D9 Ruling-B transfer is genuine, not cargo-culted: the structure
is identical — a weather-driven modulation whose per-year
renormalisation would (i) zero the inter-annual band, (ii) sever the
cold-year covariance (more charging energy AND worse heat-pump COP AND
low wind in the same hours), (iii) break horizon composability. The
pinned constant `c` (record-mean of `m(T̄_d)` over 1985–2024 = 1,
computed once from the pinned trace, echoed to outputs) is exactly the
D9 rule-3 mechanism, and `annual_energy_twh` as record-mean quantum is
the right reading. Record-mean of the multiplier = 1 ⇒ record-mean
annual energy = quantum: conservation test 8.1 is well-posed.
The correlation-hazard paragraph is adequate and correctly prominent —
including the key sentence that omitting the derating would be
adequacy-FAVOURABLE at the binding hours. The daily-mean limitation
(cannot move energy within the day) is owned with direction.

Geotab: the quoted study parameters (4,200 EVs, 5.2 M trips, ~115% of
rated range near 21.5 °C, ~20% loss around 0 °C, ~46–54% loss at deep
cold) are consistent with the published Geotab analysis as I know it —
spot-check passes at design level; the data package must verify
against the live pages on its access date. Licence discipline RULED
ACCEPTABLE: the D9 ruling-A pattern (trigger and fallback defined now;
licence verdict recorded BEFORE transcription; corroborating source
named) is the project's established evidence discipline, and the data
package gates on the reviewer anyway. Adoption of the design does not
wait for the licence; transcription does. The consumption multiplier =
1/(range fraction) convention is stated — correct.

## §4 V2G-lite — CONFORMS to ADR-8; Q6 gating COHERENT

Verified against scenario.rs: `StorageKind::Dsr` exists with exactly
the DSR-only fields the draft names (`shift_duration`,
`daily_volume_limit`, scenario.rs:1488–1495), validation already
rejects those fields on non-dsr kinds (ADR-8), and the module doc
states "DSR engine semantics are provisional until Q6" — the draft's
gating (parameter derivation, parsing, validation, echo now; runs
QUOTING the v2g_lite limb wait for Q6) matches the committed posture
and the D-note-before-code discipline. The derivation (power from
participating fleet × export power × minimum plugged-in fraction;
energy from usable battery swing; RTE = charger-in × charger-out;
shift_duration = window convention; participation validated in [0,1])
emits only existing schema fields — ADR-8's pseudo-storage sentence is
implemented, not extended. No ADR amendment needed: correct.

## §5 Schema v8 — additive claim VERIFIED

- `DemandSpec` (v7, scenario.rs:989–1021) carries base_profile,
  column, extra_profiles, annual_scale, extra_demand_gw, heating —
  NO EV field of any kind. Unlike D9's v5, this bump is genuinely
  purely additive; the draft's contrast with v5 is accurate.
- Fixture-freeze precedent verified: `grid-core/tests/fixtures/`
  holds v1…v6 reference scenarios; freezing v7 continues it.
- The `heating-cop.toml` precedent is accurately described: schema
  string (`heating-cop-v1`) probed before the strict parse,
  engine-constant path (no scenario field), pinned regression tests,
  per-entry overrides echoed to outputs. `ev-fleet.toml` /
  `ev-fleet-v1` on that shape is right.
- Per-zone block (the D9 edit-3 ruling), trace reference carried in
  the block, `deny_unknown_fields`, share-sum law: all correct
  transfers.
- Digest discipline (test 8.4) is testable and the precedent is real:
  the v5/v6/v7 version-line migrations left `779d7444…` and the
  multi-zone pins unmoved (stage-5/6 and later review records);
  explicit re-verification in the v8 commit is the right posture.

## §6 Acceptance criteria — red-first-able WITH edits 1–3

Tests 8.1 (conservation on the record trace), 8.4 (digests), 8.5
(pinned characterisation run per the published-number law) are
red-first-able as stated. Test 8.3 becomes a theorem only after
edit 1. Test 8.2 is exact for D10 but ill-posed for the D9 term until
edit 2, and needs edit 3's realised-energy statement so nobody reads
the quantum identity as a claim about the 2024-weather run's realised
total. Tolerance: ruled in edit 5 (fork 6).

## §7 Scope and claim ceiling — PASS

Matches the docs/08 D15 promotion exactly ("D10 promoted into v1 —
needed to run FES EE demand honestly at 2035+, not only Q12"); Q12
full analysis correctly deferred; literal EU-27 transport correctly
out (D15 demotion); the claim ceiling (national-aggregate bracket with
declared conventions, never a forecast; road EVs only, matching the
ED1 component class) is honest and matches the FES headers' named
no-reprofiling gap this overlay exists to close. Non-goals adequate —
distribution-network exclusion correctly pinned to ADR-12 with the
favourable-to-flexibility direction on every smart/V2G quote. The
three-package chain with reviewer gates matches supervisor-mode law.

## §8 Sources — spot-verified; one scope defect

- ED1 EV rows: bit-verified against the committed reference (above).
- TRA0101 car traffic 256.1 bn vehicle-miles (≈412 bn vehicle-km,
  conversion checked: ×1.609) and the ~70 TWh order-of-magnitude
  cross-check (412 bn km × 0.17 kWh/km): arithmetic verified;
  magnitudes consistent with the published DfT series. GB scope —
  correct for TRA0101.
- VEH0101: DEFECT (edit 4) — the draft labels the 41.7 M end-June-2024
  figure "GB" while in the same sentence quoting cars "of UK licensed
  vehicles"; the DfT release headline basis must be pinned (UK vs GB),
  and if a conversion is needed it follows the D9 ONS-population
  precedent (a cited factor, stated, never silent).
- All DfT sources OGL; Geotab per §3; FES already pinned (no new
  fetch) — licence posture clean.

---

## Ordered edits (blocking; apply verbatim in spirit, exact text where
## given)

### Edit 1 — Rule 5 smart allocator: replace greedy cap-fill with the
### min-max (level/water-fill) allocation; machine-check containment
In the `smart` bullet, replace "Allocation: greedy lowest-`R` fill
subject to the segment's aggregate power cap, with the window
convention chosen so the smart feasible set CONTAINS the dumb
allocation (window ⊇ the dumb charging span at the same cap) — this
makes the bracket direction on the residual peak a theorem by
construction (rule 8.3)." with:

> Allocation: the **level (water-fill) allocation** — the day's energy
> is placed within the window so as to MINIMISE the maximum of
> `R(t) + ev(t)`, subject to the segment's aggregate power cap per
> half-hour (fill the lowest-`R` half-hours up to a common level,
> capped; the level is the unique solution of the energy-balance
> equation — deterministic, no tie-breaking needed). Containment is a
> VALIDATED precondition, not an assumption: the smart window must
> contain the dumb allocation's support and the cap must be ≥ the dumb
> allocation's maximum aggregate power, checked at load time with a
> structured error naming the violation. Because the allocator is the
> feasible-set max-minimiser and the feasible set contains the dumb
> allocation, smart's maximum post-EV residual ≤ dumb's is a genuine
> theorem (rule 8.3). A greedy fill AT the cap is explicitly rejected:
> on a flat residual it produces `R + cap` and can exceed the dumb
> peak — the bracket would invert by construction artefact.

Amend rule 8.3's parenthesis to cite the min-max construction. Amend
the rule-11 row's "bracket direction a theorem by construction"
accordingly (it stays true, via this edit).

### Edit 2 — Rule 2 + test 8.2: restate the carve-out in ELECTRICAL
### terms and demote the D9 side to a named open convention
In rule 2, after the carve-out bullet, add:

> The carved quanta are **electrical energies**. For D10 the quantum
> IS electrical (`annual_energy_twh` = the ED1 road-EV component,
> exact). For D9 the scenario input is heat-side
> (`delivered_heat_twh`); its electrical energy is derived
> (`heat/COP`, portfolio-dependent), and the pinned ED1 extraction
> carries only RESIDENTIAL heat markers (18.223/58.505 heat-pump,
> 3.478/14.643 resistive TWh) — no commercial-heat row. The D9 FES
> carve-out therefore needs its own convention (which ED1 rows
> constitute the carved component; the heat-to-electrical inversion
> computed ONCE from pinned inputs on the record mean, never per
> year), adjudicated at the scenario package with the reviewer gate —
> NAMED OPEN, not silently pinned here. What IS pinned here for both
> overlays is the composition mechanism: subtract the overlays'
> electrical quanta from the retarget, add each back with its physics
> shape, one subtraction of the sum (order-independent).

Restate test 8.2's "(+ D9 quantum where applied)" as "(+ the D9
electrical carve-out quantum under its adjudicated convention, where
applied)". Amend the rule-11 row's "carve-out rule pins the
anti-double-count composition for BOTH D9/D10" to "…pins the
composition MECHANISM for both; the D9 component convention is named
open until the scenario package".

### Edit 3 — Rule 2: state the realised-vs-quantum weather wrinkle
Append to the FES-consistency bullet:

> The identity is asserted on QUANTA. A single-weather-year scenario's
> REALISED overlay energy is `quantum × mean(m(T̄_d))` over that year,
> not the quantum: under 2024 weather (mild — 2024 degree-hours are
> ~13% below the record mean) the realised annual total lands BELOW
> the published System Demand Total by the weather modulation of both
> overlays. Direction stated on every FES-reprofiled quote and on any
> scaled-peak-vs-published-peak comparison; the record-mean identity
> is the design claim, the per-year deviation is physics.

### Edit 4 — Rule 3: pin the VEH0101 scope
Correct the stock bullet: the 41.7 M end-June-2024 headline's basis
(UK vs GB) is pinned by the data package from the release itself; if
GB figures are needed, the conversion is a cited factor (the D9
ONS-0.972 precedent), stated next to the number — never a silent
relabel. (As drafted the note calls the same figure GB and UK in one
sentence.)

### Edit 5 — Rule 8.2 (fork 6 RULED): tolerance = 1e-9 relative
The FES-consistency identity (retargeted base energy + overlay
electrical quanta vs the published System Demand Total) is asserted at
**1e-9 relative** — the committed pathway-pin precedent
(stage7-pathways-scenarios-review §B: annual_scale reproduces to the
last digit; run demand < 1e-9 relative). The retarget is a single f64
division of exact TOML decimals, so 1e-9 has orders of margin while
catching any wrong-row or wrong-basis error. The D10-quantum == ED1
component assertion stays EXACT equality of the parsed reference value
(no tolerance), as drafted.

### Edit 6 — Rule 6 (fork 5 hardened): zonal upgrade is MUST, not
### "should"
The per-zone default (demand-share split) is adopted for GB-aggregate
work, but replace "a zone-resolved result quoting EV effects should
upgrade to a cited DfT per-region licensing split" with: any
ZONE-RESOLVED EV claim MUST use the cited per-region licensing split
(VEH0105-class, data-package option); the demand-share default is
quotable only at GB aggregate, and the convention is stated next to
any zonal claim.

### Edit 7 — Rule 5 (forks 1–2 text): carry the fork rulings into the
### dumb/commercial text
(a) The smart window per segment lives in `ev-fleet.toml` (cited or
declared-as-convention), with edit 1's containment check. (b) The
commercial dumb profile: the data package must SEARCH FIRST and record
the result — a cited public depot/commercial charging distribution if
one exists licence-clean; the flat-working-hours convention is the
recorded fallback, labelled as convention with its bias direction
stated. The draft's "(cited if a public source exists; else …)"
becomes an ordered search-and-record deliverable, not an optional
look.

---

## The seven fork rulings (consolidated)

1. **Smart window convention:** per-segment window from
   `ev-fleet.toml`; containment of the dumb allocation MANDATORY and
   machine-checked; allocator = level/water-fill min-max (edit 1).
2. **Commercial daytime profile:** search-first, record the search;
   flat-working-hours is the labelled fallback convention (edit 7b).
3. **Flat daily km:** ACCEPTED for v1 — second-order against profile
   and derating at the binding hours; day-type variant stays the named
   follow-on.
4. **V2G run gating:** ACCEPTED — parse/derivation/validation/echo
   land now; any run QUOTING v2g_lite waits on Q6 activation
   semantics; the dependency binds the analysis package only.
5. **Zonal split default:** demand-share ACCEPTED at GB aggregate;
   per-region licensing split MANDATORY for zone-resolved claims
   (edit 6).
6. **FES-consistency tolerance:** 1e-9 relative; component equality
   exact (edit 5).
7. **kWh/km source order:** ACCEPTED — FES workbook components first
   (pinned pack, licence-clean, internally consistent with the
   quantum), else a named measured real-world study; at-the-meter
   basis mandatory, with the conversion cited if the source is
   at-battery.

## Notes of record (non-blocking)

1. "Quantum ÷ days" leaves the leap-day convention implicit (365 vs
   366): pick one (per-calendar-day of the running year is fine),
   state it in the field docs; conservation test 8.1 will hold either
   way since `c` is computed on the same convention.
2. `R(t)`'s "must-take weather-driven generation" set is the engine's
   own CF-trace must-take convention — acceptable by reference; the
   engine package should name the set in the module doc.
3. The DfT 2017 chargepoint arrival distribution's age (early-adopter
   fleet) is correctly recorded as a stated limitation with direction
   to be discussed in the evidence note — adequate.
4. HGV-inside-commercial fold: stated, extension path priced at one
   version bump — matches the D9 rule-2/Ruling-F law exactly. Correct.
5. The draft correctly proposes docs/08 text without editing docs/08
   (no-silent-drift); rule-11 text needs the edit-1/edit-2 amendments
   before it lands.

## Chain instruction

Apply edits 1–7 to `docs/notes/d10-ev-overlay.md`, amend the rule-11
row per edits 1–2, flip the note's status to ADOPTED citing this
review, update the docs/08 D10 row, commit the pair with the
project-state entry BEFORE the data package is briefed — edits 4, 5,
and 7 change the data package's deliverables; edits 1–3 change the
engine and scenario packages' acceptance tests (8.2/8.3).
