# Q5/Q11 heating-mix analysis package — reviewer adjudication

Reviewer, 2026-07-04. Subject: the UNCOMMITTED heating-mix analysis
package (grid-adequacy/src/heating_mix.rs + lib.rs export,
grid-adequacy/tests/heating_mix.rs, the `sweep heating-mix` subcommand
in grid-cli/src/sweep.rs + main.rs, grid-cli/tests/heating_mix_sweep.rs,
scenarios/royal-society-37y-heated.toml; gitignored artefacts in
runs/q5-heating-mix/). Everything B6-related was committed at 274a750;
the uncommitted tree is exactly this package (scope verified by
`git status`: those six files and nothing else; no Cargo.toml, no
docs/notes, no memory/, no pin edits).

Acceptance contract: D9 rules 6/6b + ruling C
(docs/notes/d9-heating-overlay.md), the binding record items of
docs/notes/q5-heating-engine-review.md (rating stated next to every
storage number; SolveInfeasible reportable, never bumped; the
quote-duty wording), the heating.rs characterisation pins, the Stage 4
attribution machinery and its window-sensitivity publication rule
(docs/notes/stage-4-decomposition-run-report.md), docs/06.

## VERDICT: ACCEPT-WITH-NOTES

Every load-bearing number reproduces under my own runs — the full
66-point sweep artefacts are BIT-IDENTICAL to the delivered
runs/q5-heating-mix/ files except `created_utc` and the (stale, dirty)
embedded engine hash. Gates run by me: `cargo fmt --check` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean; full
`cargo test --workspace --release` exit 0, 48 suites all ok (the
heating_mix acceptance suite 2/2 in 11.07 s). Five conditions, none
requiring redesign.

## Independent reproductions (verify, don't trust)

**1. The cross-check (item 1).** I ran the sweep myself (release,
26.5 s for 66 points + baseline + 4 decompositions, rayon). The
0.70/0.20/0.10 row and the baseline reproduce the heating.rs
characterisation pins bit-identically — the artefact CSV cells are
character-for-character the pin strings: baseline
92.23871490574456 GW / 23,872 GWh; D9 mix 113.4466987983204 GW /
40,224 GWh at 200 GW both endpoints. The bit-identity is not luck: the
loader computes `base×scale + extra + heating` (inputs.rs), which is
left-associative — `(base×scale + extra) + heating` — exactly the
sweep's baseline-plus-overlay construction. Reasoning verified in
source AND by the pins.

**2. Corners (item 2), my run:**
- all-ASHP: 115.68894336087274 GW / 43,488 GWh (×1.8217)
- all-GSHP: 114.39690356751237 GW / 41,248 GWh (×1.7279)
- all-district: 95.85057732206994 GW / 25,872 GWh (×1.0838)
- baseline: 92.23871490574456 GW / 23,872 GWh
All pin-exact. `store_power_gw = 200` is stamped on EVERY row
including the baseline; assumption 1 of the artefact block carries the
engine-review quote-duty wording (both-endpoints phrase, the 100 GW
POWER-BOUND INFEASIBLE finding travelling with ×1.69-class headlines,
"reportable result, never a silently bumped rating") — verbatim in
substance, checked against the review text.

**3. Gradients (item 3), my run:**
- ASHP→district peak: −1.9838366 GW per 10 %, linear to f64 dust
  (max step spread 1.4e-14 GW across all ten steps; same for the
  ASHP→GSHP edge at −0.1292040 GW).
- ASHP→district storage: edge average −1,761.6 GWh per 10 %, with the
  knee: steps −2,816/−2,816/−2,752/−2,752 (shifts 0→0.4), transition
  −1,312 (0.4→0.5), then −1,040/−1,040/−1,024/−1,040/−1,024.
- ASHP→GSHP storage: exactly −224 GWh every step; ×10 = 2,240 =
  43,488 − 41,248. The corner arithmetic is consistent — the small
  gradient IS the corner gap; no inversion.
- **Knee is real, not tolerance.** The bisection quantum here is
  16–32 GWh (SolveOptions default: max(0.1 GWh, 1e-3 × upper
  bracket); all requirements land on 16-GWh multiples, and the
  within-regime wobble −1,024↔−1,040 is exactly one quantum). The
  regime change (~1,700 GWh per step) is ~100× the quantum.

**4. Curtailment trade-off (item 4), my run:** baseline 18,711.64 →
all-ASHP 14,288.54 → all-district 17,962.90 TWh (horizon totals;
the artefact's horizon-total note is present as the last assumption
line). Direction confirmed: heating REDUCES curtailment, district
relieves LESS than ASHP. I closed the energy balance exactly on the
artefact: ΔCurtailment(all-ASHP) = −4,423.10 TWh =
−(ΔheatE 3,096.55 + Δstore-charge 2,210.91 − Δstore-discharge
884.36) — the drop exceeding the added demand is round-trip-loss
absorption (η = 0.40), not an error.

**5. Decomposition (item 5), my run:** all sixteen band pins
reproduce (baseline 23,872 = 1,088+9,512+13,272+0; all-ASHP 43,488 =
2,848+16,096+24,544+0; all-GSHP 41,248 = 2,880+15,152+23,216+0;
all-district 25,872 = 1,040+9,816+15,016+0). Telescoping verified on
the artefact (bands sum exactly; also asserted by the CLI test on the
written CSV). Deltas: ASHP−district seasonal 9,528 > synoptic 6,280 >
diurnal 1,808 GWh — seasonal loads hardest, as claimed. Windows
24 h/336 h/8,760 h stamped per row, in the CSV comment block, in the
parquet `window_convention` key and in the console — the Stage 4
publication rule is carried everywhere.

**6. Machinery (item 6).** MixShares integer-lattice bit-identity:
unit-tested and true (7/10 is the correctly-rounded f64 = the literal
0.70). Rayon/serial bit-identity asserted on the whole sweep struct.
SolveInfeasible-reported-not-bumped: the 5 GW CLI test verifies the
stamped rating, empty requirement cells, the solver's reason per row,
artefacts on disk, and a loud nonzero exit naming the already-written
artefacts. `--store-power-gw` genuinely required (verified: clap
usage error without it). Sweep artefacts written before the
decomposition, so infeasibility still leaves a reportable artefact
(exercised by the same test).

**7. Conventions (item 7).** CSV+Parquet both, always; the chart
embeds engine hash + scenario sha256 + the rating duty + the
lower-bound caveats; newtypes throughout the library module; no new
dependencies (no Cargo.toml touched); the heated scenario header
corrects the "energy-binding by design" claim with the full
store-rating warning (the engine-review record item). Red-first
where a theorem (district-lowest), measured-then-pinned per ruling C
for corners/decomposition — and the separate directional asserts
WOULD fire on an inversion (checked: the ASHP>GSHP assert and the
district-lowest assert are independent of the pins).

## Rulings requested

**R1 — the knee (item 3): REAL; quote piecewise, never the edge
average alone.** The knee is ~100× the bisection quantum — a genuine
regime change in the binding drawdown, not solver noise. Duty: any
quoted ASHP→district storage gradient states the two limbs
(≈−2,750…−2,816 GWh per 10 % for the first four tenths from
all-ASHP; ≈−1,030 beyond, transition step −1,312) or shows the
curve; the −1,762 edge average may only appear alongside the limbs.
The policy-relevant marginal value from today's ASHP-heavy starting
point is the STEEP limb (~1.6× the edge average) — understating it by
quoting the average is the exact inversion-class error this
programme's reviews exist to catch. All at 200 GW both endpoints;
lower bounds per rule 3.

**R2 — the curtailment framing (item 4): direction verified;
framing duty.** The correct statement, backed by the exact artefact
identity above: electrified heat absorbs otherwise-curtailed energy,
directly and via extra store cycling; a geothermal/district share
needs ~5.7× less electricity for the same heat and therefore
FORGOES most of that absorption (all-district relieves 749 TWh of
horizon curtailment vs all-ASHP's 4,423). The network value of
geothermal is peak + storage relief MINUS foregone curtailment
absorption — and that netting is a Stage 7 (£) statement; until then
the two sides are quoted in physical units side by side, never
collapsed into one number. Energies are HORIZON totals (the
artefact note exists and stays); curtailment is pooled (Stage 1
convention, no per-source attribution — assumption 5 says so); the
dispatch metrics are at the committed 100 TWh store with the stated
200 GW rating (stamped in dispatch_store_energy_gwh) — state that
next to any curtailment number.

**R3 — inter-annual zero (item 5): a FINDING, not forced — and the
implementer's wording is correct.** Worked through against the Stage 4
construction: the inter-annual band is the bisection requirement of
the 365 d-smoothed residual, and under rule 3's pinned intensity the
heated smoothed series genuinely varies year to year (cold years draw
more heat) — zero is NOT a construction identity. It is zero because
the RS fleet's overbuild dwarfs the inter-annual channel: I computed
per-year potential from the CF packs — worst year 2010 at 951.1 TWh
vs mean 1,097.5 — against a worst-case all-ASHP heated annual demand
of ≈665 TWh (570 tiled + ≈95 cold-year heating electrical): every
365-day window sits ≥~280 TWh/yr (~33 GW mean) in surplus, so the
smoothed residual never approaches deficit. Quote as: "inter-annual
attribution stays zero at every corner ON THIS OVERBUILT FLEET — the
Stage 4 posture (seasonal-scale need, multi-year recovery) survives
electrified heat; a leaner fleet could attribute nonzero." The test's
wording already carries the fleet-contingency; keep it wherever
quoted.

**R4 — the −48 GWh diurnal delta at all-district: legitimate, but
quote as resolution-scale.** Telescoping holds exactly on the
artefact; a negative BAND never occurs, only a negative DELTA of the
diurnal attribution (1,040 vs 1,088). Each band is a difference of
two 16–32 GWh-quantized bisections, so band deltas carry ±2 quanta of
attribution noise — 48 GWh is ~1.5–3 quanta. Duty: present as "≈0, at
the attribution's resolution; the flat pump load shifts which
smoothing level binds" — never as "district reduces diurnal storage
need" (a physical-mechanism claim the resolution cannot support).

## Conditions (numbered; action before/at landing)

1. **Parquet assumption metadata under a duplicate key.** Both new
   parquet writers push all eight assumption lines under the single
   key `assumption` (sweep.rs:1335, 1553); parquet permits duplicate
   KeyValues but dict-based readers (pyarrow's default view) surface
   only ONE of the eight — seven quote-duty lines, including the
   rating duty, are effectively invisible to standard tooling. This
   pattern is new to this package (no prior writer does it). Number
   the keys (assumption_1…assumption_8, and match in both writers)
   and extend heating_mix_sweep.rs to assert the duties ride in the
   PARQUET metadata, not just the CSV (docs/06: outputs carry their
   conventions).
2. **Pin what will be quoted.** The knee (R1) and the curtailment
   trade-off (R2) are headline findings, but the interior
   ASHP→district edge requirements (40,672 / 37,856 / 35,104 /
   32,352 / 31,040 / 30,000 / 28,960 / 27,936 / 26,896 GWh) and the
   corner curtailment totals (18,711.64 / 14,288.54 / 14,569.50 /
   17,962.90 TWh) are pinned nowhere — "every published number gets a
   pinned regression test before it is quoted anywhere" (CLAUDE.md).
   Add the edge-requirement pins and corner-curtailment pins (with
   the direction asserts: heated < baseline curtailment; district
   relieves less than ASHP) to heating_mix.rs before either finding
   is quoted.
3. **u32 overflow panic paths in the library API.**
   `MixShares::new` computes `ashp + gshp + district` and
   `simplex_mixes` computes `(n+1)*(n+2)/2` on raw u32 — adversarial
   but legal inputs (huge numerators; a tiny `--step` like 1e-9
   reaches `simplex_mixes(10^9)` through the CLI) overflow-panic in
   debug and wrap in release. docs/06: no panics in library crates.
   Use checked arithmetic with a structured error (and/or bound the
   denominator sanely at the CLI).
4. **TDD evidence at landing** (the engine review's condition 5,
   restated for this package): the tree is uncommitted, so red-first
   cannot be evidenced by commit order. Land as a commit sequence
   with the acceptance tests preceding the runner/CLI they gate, or
   record the red-run evidence explicitly in the landing record. The
   cross-check values genuinely predate the machinery (heating.rs is
   committed history) — that part of the red-first contract is
   proven.
5. **Regenerate the artefacts at landing.** runs/q5-heating-mix/
   embeds `engine_git_hash = 8342b87f…-dirty` — a pre-B6 dirty tree
   that no commit can reproduce. The data content is proven correct
   (bit-identical to my rerun), but the determinism law is
   `results = f(scenario, pack checksum, engine hash)`: regenerate
   from the landed commit so the header hash is clean and citable.

## Notes of record (non-blocking)

- The acceptance test's cross-check asserts peaks within 1e-12 rather
  than exact equality (requirements are assert_eq). The measured
  values ARE bit-identical; tightening the peak asserts to equality
  would make the test say what the design proves. Suggested, not
  required.
- The CLI test writes under a fixed `std::env::temp_dir()` path — the
  standing concurrent-suite race hygiene item (engine review note),
  inherited, not new.
- Reproduced timing: 66-point 40-year sweep + 4 decompositions in
  26.5 s (rayon, release) — comfortably inside the implementer's
  ~30 s claim; no docs/06 performance target implicated.
- The decomposition parquet carries `simplex_step` metadata it does
  not use — harmless.
