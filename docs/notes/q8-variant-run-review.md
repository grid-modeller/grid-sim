# Q8 current-holdings variant run — adversarial review

Reviewer, 2026-07-03. Package under review (uncommitted): three pathway
spec files (`data/reference/fes-pathway-current-holdings.toml`, `…-df090.toml`,
`…-2019-speed.toml` — diagnostic, marked NOT QUOTABLE), three drift-guard
tests (`grid-stability/tests/pathway.rs`), two pinned regressions
(`grid-cli/tests/stability.rs`), gitignored artefacts
`runs/q8-current-holdings/{central,df090,diag-2019-speed}`. Baseline
context: `docs/notes/stage-6-part2-run-report.md` (2019-default run, whose
§6 rule 2 named this variant); holdings record `docs/notes/q8-current-holdings.md`
(commit 887d5e4, reviewed ACCEPT-WITH-NOTES).

## Verdict: ACCEPT-WITH-NOTES

The specs, tests, artefacts and headline all verify independently
(bit-identical digests, reviewer's own CLI runs). The package as committed
is sound. **The conditions bind the run report the supervisor writes
next** — the implementer's *mechanism story* for the speed-vs-volume
decomposition is refuted by reviewer probes and must not be quoted as
claimed.

## Conditions

1. **Mechanism correction (blocking for the run report).** The claimed
   volume-leg mechanism — "droop-saturation shape (DM+DR full at 0.2 Hz
   vs 0.5 Hz) plus removal of the 30 s sustain cliff" — is empirically
   nil. Reviewer probes (reproduce with the CLI; specs described in §3):
   removing the 30 s sustain + 10 s rundown from the 2019 defaults
   reproduces the 2019 baseline **digest-identically** (`fd410616…`, all
   27 years × 2 conditions — the sustain cliff never binds at the
   survival boundary); widening the DM/DR saturation from 0.2 to 0.5 Hz
   reproduces the diagnostic **digest-identically** (`bd8f4684…`), and
   moves the central run ≤ 0.61 MW. The volume leg (+684 MW at 2024 min)
   is instead: delivery-factor accounting +37.8 MW (min) / +89.1 (mean)
   (2019 volumes at df 1.0), and a **residual ≈ +646 MW from relocating
   held MW into the sub-second envelope rank** (fast-rank effective MW
   472 → 1,178; slow rank 1,055 → 461). That is a speed effect sitting
   in the "volume" leg. Quote the split only with this attribution.
2. **Pin before quoting the decomposition.** The diagnostic is
   drift-guarded but not pinned. If the run report quotes the ~1/3:~2/3
   split or any diag-derived number (it should — the split is the
   variant's most interesting result), first add a pinned regression on
   the diag run (digest `bd8f46842cf74ca799172b19368e839deb82877284d1d0dd8fa72a5028b4f261`
   plus representative 2024/2035 points), per CLAUDE.md pin-before-quote
   and docs/05. The implementer supplied the digest for exactly this.
3. **Demand→damping carry-over wording.** The "never lost" statement
   must carry the part-2 §2 caveat resolution: reviewer fixed-demand
   probe (all years' `demand_twh` pinned at the 2024 value 289.65)
   gives pathway minimum **2,359.0 MW (min φ, 2035) / 2,622.7 (mean φ,
   2035)** — still above 1,800 everywhere, so the no-crossing claim is
   robust to the demand-growth→damping channel. Later-year *absolute*
   values are not (2050 min 3,419 → 2,452 MW with demand pinned): never
   quote post-2030 absolutes without the caveat.
4. (Note) TDD evidence is the chain record only ("pin tests
   designed-red") — a single uncommitted package leaves no commit-order
   trail. Acceptable for a data+tests package (no library code touched);
   recorded.
5. (Note) The diag artefact files themselves carry no not-quotable
   marking (it lives in the spec header and the guard-test comment).
   Acceptable; the run report must repeat it (rule 9 below).

## 1. Reproduced numbers (reviewer's own release-CLI runs from the specs)

| Run | Digest (reviewer-reproduced) | 2024 min/mean (MW) | Crossings vs 1,800 |
|---|---|---|---|
| central (FY2025 holdings, df 1.0) | `2c8d6997…` ✓ bit-identical | **2,432.86 / 2,700.81** | none — at/above from 2024, both conditions; pathway minimum is the 2024 point |
| df 0.9 sensitivity | `2da63edd…` ✓ | **2,282.71 / 2,527.47** | none |
| diag 2019-speed (NOT QUOTABLE) | `bd8f4684…` ✓ | 2,056.88 / 2,283.33 | — |
| 2019-default baseline | `fd410616…` ✓ (untouched; part-2 pin green) | 1,372.68 / 1,573.49 | 2035 mean / 2037 min |

Headline claim 1 verified: standard met from 2024, never lost, both
dispatch conditions, both df conventions; 2024 band as claimed; 2019
band 1,373/1,573 as committed. Also ≥ 1,320 (normal infeed) throughout.

## 2. Spec fidelity (claim 2) — verified

- Base `fes-pathway.toml` sha256 `3944cf16…` matches the header claim;
  full diff: central = base verbatim + header + ONE marked
  `[[assumptions.responses]]` block. Transcription of the three services
  is exact against `response-holdings-2025.toml` §4 (1,178/416/461 MW;
  droops 0.5/0.2/0.2 Hz; delays 0.5/0.5/2 s; ramps 0.5/0.5/8 s; df 1.0;
  no sustain — contracted durations ≫ 120 s window, stated).
- df090 diff vs central: header text + exactly three
  `delivery_factor = 0.9` lines. 2019-speed diff: header + exactly the
  six delay/ramp lines (0.3/0.7, 2/8, 10/20 by speed rank).
- 2019 baseline untouched: `fes-pathway.toml` clean in git; the part-2
  pin (`q8_fes2025_pathway_reproduces_the_pinned_headline_numbers`,
  digest `fd410616…`) green in the reviewer's run;
  `runs/stage6-q8-fes2025-ht` digest matches.
- Artefacts embed engine hash, spec path + sha256, per docs/06 (the
  `-dirty` engine hash is expected pre-commit; digests cover the CSV
  data section only).

## 3. Decomposition audit (claim 3) — arithmetic PASS, mechanism FAIL

Sequential legs (leg A = central − diag = envelope timing at current
volumes; leg B = diag − 2019 = everything else), reviewer-computed:

| Year, cond | Total gain | Leg A (timing) | Leg B (volume side) |
|---|---|---|---|
| 2024 min | 1,060.2 | 376.0 (35.5 %) | 684.2 (64.5 %) |
| 2024 mean | 1,127.3 | 417.5 (37.0 %) | 709.8 (63.0 %) |
| 2035 min | 1,024.2 | 345.5 (33.7 %) | 678.7 (66.3 %) |
| 2035 mean | 1,114.5 | 408.9 (36.7 %) | 705.6 (63.3 %) |
| 2050 min | 1,073.6 | 382.7 (35.6 %) | 690.9 (64.4 %) |
| 2050 mean | 1,132.2 | 418.7 (37.0 %) | 713.5 (63.0 %) |

~1/3 : ~2/3, stable across years and conditions — the claimed split is
arithmetically correct. Held volume fell (2,055 vs 2,336 MW) and
effective volume rose +159 MW (2,055 vs 1,896) — both verified.

**But the claimed leg-B mechanism is refuted** (probe specs: diag with
all droops at 0.5 Hz; 2019 defaults without sustain/rundown; 2019
defaults at df 1.0; central with droops at 0.5 Hz; 2019 defaults
explicit as sanity — the last reproduces `fd410616…` exactly):

- 30 s sustain cliff removal: **0.00 MW** (digest-identical to baseline,
  every year, both conditions).
- Droop saturation 0.2 vs 0.5 Hz: **0.00 MW** at 2019 timings
  (digest-identical to diag); ≤ 0.61 MW under current timings. At the
  bisected survival boundary the trajectory saturates every droop
  quickly, so saturation width is immaterial (this also empirically
  supports part-1's "linear-droop optimism is second-order at the
  floor" argument).
- Delivery factors → 1.0: +37.8 MW (min) / +89.1 (mean) on 2019 volumes.
- Residual ≈ +646 MW (min 2024): **relocation of held MW into the
  sub-second rank** — the dominant mechanism of leg B.

Leakage ruling: the construction is a *sequential, path-dependent*
decomposition (leg A is measured at current volumes/droops; interactions
are folded into whichever leg is measured second) and leg B mislabels a
speed effect (volume moved into fast products) as "volume/saturation".
With the corrected attribution (condition 1) and the legs quoted by
definition rather than as an exact attribution, the split is quotable.
The honest one-line summary: **essentially the whole improvement is
response speed — one third from faster envelopes per product, roughly
two thirds from the fleet's MW moving into the fastest products; volume
and delivery-factor accounting are small, droop shape and sustain nil.**

## 4. Part-2 reframe (claim 4) — supported, with required wording

"The 2035/2037 recovery was a holdings artefact" is supported at both φ
conditions (which set the inertia bounds on this pathway) and under both
df conventions: under FY2025 holdings there is no crossing to recover
from. The part-2 fixed-demand caveat carries over asymmetrically
(condition 3): the no-crossing claim survives demand pinned at 2024
(pathway min 2,359/2,623 MW); late-year absolutes do not. Required
wording is rule 4 below.

## 5. Test quality (claim 5)

- Drift-guards bind: `assert_same_but_for_responses` compares every
  `PathwaySpec`/`PathwayAssumptions` field (checked against the struct
  definitions — complete), `years` table equality pins the FES table
  verbatim, and the responses vectors are compared exactly against the
  in-test transcription of the reviewed record. Any spec edit fails.
- Pins tight: digest over the whole CSV data section + exact point
  blocks + crossing blocks + `responses_defaulted_to_2019 = false`. A
  df-swap or spec-content-swap changes the digest and the point values —
  fails. Verified green in the reviewer's own workspace run.
- Diagnostic drift-guarded, not pinned, marked not-quotable in spec
  header and test comment: acceptable as committed; pin required before
  the run report quotes decomposition numbers (condition 2).

## 6. Gates (reviewer-run)

- `cargo fmt --check` clean; `cargo clippy --workspace --all-targets --
  -D warnings` clean; `cargo test --workspace` **400 passed / 0 failed**
  (395 + the 5 new tests).
- Stage 1/2 digests (`pinned_2024_reference_result_digest`,
  `pinned_2024_prices_digest`), part-2 FES pin, and the no-retuning
  guard (`default_era_assumptions_match_the_committed_2019_spec`) all
  green; no 2019 constant, scenario or schema file touched (tests +
  reference specs only; no new dependencies; no docs/03 change needed —
  `fes-pathway-v1` schema unchanged, the responses block pre-existed).
- Scope clean: exactly the five package files plus gitignored runs. The
  other worktree items (stage-7 cost inputs, D9) belong to other
  streams. The chain-record's flagged `gen_variants.py` (undefined
  `dc_tc2`) is absent from the tree — resolved by deletion.

## 7. Publication rules for the run report (drafted for the supervisor)

1. Q8 numbers only as a **band with the condition named** (min φ=0.15 /
   mean φ=0.35, 2024-anchored market-behaviour convention, not a 2050
   dispatch forecast); the market-only lower edge (zero inertia ⇒ zero
   survivable loss) accompanies any quoted band. (Part-2 rules 1 and 4
   carried.)
2. Headline form: "Under FY2025 procured response volumes at contract
   delivery factors, the modelled 1,800 MW infrequent-infeed standard is
   met from 2024 and never lost on the FES 2025 Holistic Transition
   pathway — 2024 band 2,433 / 2,701 MW, vs 1,373 / 1,573 under
   2019-era holdings." Always paired with the statement that the
   difference is a holdings *input*, not a physics change, and quoted
   against the §1 simulation-contract definition (single-step loss,
   120 s window, 48.8 Hz floor, dynamic services only, no LFDD) — never
   as an operational security standard.
3. The part-2 crossing years are superseded as: **an artefact of the
   2019-default holdings level** — under current holdings no crossing
   exists to recover from.
4. Demand→damping caveat, asymmetric: the no-crossing claim is robust
   to the channel (fixed-demand probe: pathway minimum 2,359 / 2,623 MW,
   still above 1,800); later-year absolutes are not (2050 min 3,419 →
   2,452 MW with demand pinned) — no post-2030 absolute without this
   caveat. Part-2 §2 consequence 3 (the channel is a stated v1
   limitation) carries over verbatim.
5. Contract-vs-measured asymmetry always stated: 2019 side measured
   delivery factors (0.67–1.0), current side contractual 1.0; the df 0.9
   uniform sensitivity is the quantification (2024 band 2,283 / 2,527 MW;
   headline and no-crossing unchanged). Never an invented central factor.
6. Conservative exclusions named in any comparison: ~202 MW SFFR
   (static — engine-rejected by documented design) and MFR (volumes
   unpublished) are excluded from the current side; the current-holdings
   numbers are understated on this axis.
7. Linear-droop optimism caveat carried on both legs (part-1 family);
   may add that reviewer probes show saturation-width choice moves the
   survival boundary ≤ 0.61 MW — the boundary is saturated, so the
   optimism is second-order here.
8. The speed-vs-volume split only as the **sequential leg decomposition
   with legs defined** (leg A = envelope timing at current volumes,
   ~34–37 %; leg B = remainder, ~63–66 %) and with the corrected
   mechanism (§3: leg B ≈ relocation of held MW into the sub-second
   rank + small delivery-factor term; droop shape and sustain nil). Do
   NOT attribute leg B to droop-saturation shape or the 30 s sustain
   cliff. Quotable only after the diag pin lands (condition 2).
9. The 2019-speed diagnostic is never quotable as a holdings scenario;
   repeat the marking wherever its numbers appear.
10. Every published artefact/chart derived from these runs carries
    "**Supported by National Energy SO Open Data**" — no CLI
    chart-footer hook exists; manual addition required.
11. Volumes quoted as "FY2025 mean procured volumes"; where DM is
    discussed, note the step-up (annual mean 416 vs going-forward
    ~500 MW).
12. Every number in the run report must be covered by a pinned
    regression (the two landed pins, plus the diag pin for
    decomposition numbers); part-1 and part-2 publication rules
    otherwise continue to bind.
