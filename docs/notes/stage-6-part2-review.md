# Stage 6 review — part 2 (Q8 pathway runner + Module 6 sweep + FES data pack), 2026-07-03

Adversarial review of the uncommitted Stage 6 part 2 package (reviewer
agent). Verdict: **ACCEPT-WITH-NOTES** — commit is authorised and
`stage-6-validated` tags once the conditions below land (all
supervisor-actionable; condition 1 is the only code-adjacent one and is
pre-quote, not pre-merge). Every delivered number was independently
reproduced; nothing was taken on trust. Excluded from this review:
`scripts/fetch-entsoe/` and the ENTSO-E pack files (separate in-flight
package).

## Independently reproduced (commands and results)

Suite health (reviewer-run):

- `cargo fmt --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — **312 passed, 0 failed** (33 result sets;
  includes all part-1 anchors T1–T4 and Stage 1–4 pins).
- `cargo test -p grid-cli --test regression_2024` — re-passed
  explicitly; dispatch digest `779d7444…` unmoved.

Q8 (`grid-cli stability pathway --pathway data/reference/fes-pathway.toml`),
runtime **0.46 s**, 27 years × 2 conditions:

- 2024: **1,373 MW** (min, φ=0.15, E=42.1 GVA·s) / **1,573 MW** (mean,
  φ=0.35, E=98.2 GVA·s) — both below the 1,800 MW SQSS
  infrequent-infeed standard.
- First year at or above 1,800 MW: **2035 (mean, 1,826 MW)** /
  **2037 (min, 1,811 MW)**; `first_year_below` 1,800 = 2024 (both).
- 1,320 MW normal infeed loss: never unmet (series minimum 1,373 MW,
  at 2024 min).
- 2050: 2,346 / 2,516 MW.
- Determinism: reruns bit-identical, result digest
  `fd410616862f386047137615ce99108fa2dc830d26d93e1bc5e3f27fec84e11a` —
  identical to the delivered `runs/stage6-q8-fes2025-ht` artefact.

Module 6 (`grid-cli stability inertia --scenario
scenarios/gb-2024-reference.toml --renewable-scale …`):

- Scale 1.0 **bit-identical to the part-1 pins**: 15,020 periods below
  120 GVA·s (7,510 h), 13,335 below 102, min inertia 0.
- Scale 2.0 pin reproduced: **16,601 / 15,968** at 72.1 % potential
  share. Monotone in scale (12,241/8,329 at 0.5).
- The delivered 10-point run (0.25–3.0) reproduced digest-identical:
  `484be5d9…`. Runtime ~0.05 s.
- UNCONSTRAINED caveat present in report.toml, chart footer, console
  (test-asserted).

Closed-form gate: L* = E(1−(48.8/50)²)/T = 100 × 0.047424 / 60 =
**79.04 MW** — arithmetic re-verified by hand; bisection lands within
the 0.6 MW test tolerance; hand-fixture inertia 18/0.9 × 5.0 =
100 GVA·s re-verified.

FES data package (`scripts/fes-pathway/validate.py .`): **51/51 checks
pass** (29 report anchors, 18 FLX1 cross-checks, TOML equality,
reconciliation worst |diff| **0.050 MW/yr**, sole exclusion =
non-networked offshore wind). Raw-input checksums re-verified
(`shasum -c data/packs/fes2025.sha256` — 5/5 OK). `build.py` re-run:
regenerated TOML **byte-identical** to the committed file. Anti-
circularity spot-checks read directly from the report PDF by the
reviewer (pdftotext): Table 3 p.45 (offshore 15.5, onshore 14.6, solar
18.8, nuclear 6.1, biomass/BECCS 4.3, unabated gas 39.3, LDES 2.8,
batteries 6.8) and Table 31 (nuclear HT 2.9 GW 2030 / 14.2 GW 2050) —
all match validate.py's transcriptions. Licence claim verified against
the live NESO Open Data Licence page: attribution statement is exactly
**"Supported by National Energy SO Open Data"**, CC BY 4.0 compatible,
redistribution of the derived table permitted.

## Rulings

**(1) The inverted Q8 headline — ARITHMETIC SOUND; MECHANISM REAL BUT
FRAGILE; KC-4 PROMINENCE REQUIRED (condition 2).**

(i) Reviewer probe (demand held at the 2024 base, 33.07 GW, all else
delivered defaults): the recovery **disappears entirely** — min
1,373 → 1,333 (2035) → 1,385 MW (2050); mean 1,573 → 1,511 → 1,593 MW.
Under 2019 holdings at fixed demand, **1,800 MW is never met in any
year 2024–2050**. The entire crossing headline is therefore the
demand-growth-through-damping channel (ED1 demand 289.65 → 705.2 TWh,
2.4×, scaling the 1.836 %/Hz damping base). Rising damping with rising
connected load is a real first-order mechanism, and demand entering
only through damping is a stated convention (pathway module docs §6) —
but the damping *percentage* was derived from the 2019 load
composition, and the electrified 2050 load (heat pumps, EV chargers —
inverter-interfaced) plausibly has a **lower** natural damping share.
Applying a fixed 1.836 %/Hz to 2.4× demand is the single most fragile
assumption in the crossing years, and it is currently documented only
in a Rust source comment. The model also holds the secured-loss
standard fixed while fleet unit sizes grow. This is a model limitation
to be stated at kill-criterion prominence, not an error.

(ii) The 2019-holdings default is an acceptable **flagged** baseline:
drift-guarded by `default_era_assumptions_match_the_committed_2019_spec`,
flagged `responses_defaulted_to_2019` in report.toml, console NOTE, and
chart footer. It must never be read as "GB today cannot secure
1,800 MW": real NESO holdings are materially larger (dynamic
containment ~1.4 GW+), and the 2024 finding is consistent with the
9 Aug 2019 outturn (holdings of that era did not contain a ~1.7 GW
loss). A current-holdings spec variant is the natural follow-up run.

(iii) Publication rules — see the standing rules section below;
adopting them into the part-2 run report is condition 2.

**(2) The dispatch-condition band — ANCHORING SOUND, BAND HONEST, one
documentation note.** The anchor measurement reproduces exactly:
H-weighted synchronised share of the part-1 2024 series ÷ 231.4 GVA·s
gives mean **0.344**, p5 **0.147**, p10 **0.169**, p90 **0.574**
(reviewer-recomputed from `runs/stage6-inertia-2024/inertia.csv`). So
min=0.15 ≈ the low decile, mean=0.35 ≈ the mean — measurement-anchored
as claimed, pinned by `pinned_shape_parses_with_cited_2019_defaults_flagged`.
The market-only zero lower edge is carried in the report.toml band
block, the chart footer, and the module docs. NOTE: the 231.4
denominator **excludes pumped hydro** (with PH it is 245.4 and the
stats read 0.324/0.138/0.160), while `pathway_year_inertia` **includes**
PH under the same φ — a small internal inconsistency, immaterial to
results (≲1 % in survivable loss; the defaults sit inside either
reading) but it should be stated in the run report. Applying
2024-anchored φ to 2050 fleets is defensible-with-caveat: by 2050 the
synchronous residue is CCS gas + nuclear + LDES, whose committed
fraction plausibly exceeds 0.35 (nuclear runs baseload) — the band is a
2024-market-behaviour convention, not a dispatch forecast, and must be
quoted as such (publication rule 4).

**(3) Monotonicity / bisection — VERIFIED.** From `swing.rs`: dynamic
services are active from t=0 with a delivery envelope that is a pure
function of elapsed time (delay/ramp/sustain/rundown) × droop(Δf); the
survival simulation carries **no LFDD and no static services**, so the
dynamics are state-free: f′ = F(t, f; L) with F pointwise decreasing in
L, hence trajectories are ordered by the ODE comparison argument and
survival is monotone non-increasing in L — bisection is sound. Static
latched services would key their envelope to a trajectory-dependent
activation time; they are rejected structurally (no `kind`/`trigger_hz`
fields; `deny_unknown_fields` — test-asserted). Bracket saturation is
reported, not hidden (`bracket_saturated`, test + report field). The
closed-form gate (79.04 MW) re-verified by hand. Zero loss survives
trivially; zero inertia returns the 0 MW FINDING with `zero_inertia`
flagged (RS-lean precedent) — reproduced via the hand fixture's 2036
year through the CLI.

**(4) No-retuning audit — VERIFIED.** `git diff` is empty for
`grid-stability/src/{swing,spec,inertia}.rs`, `grid-core/src/inertia.rs`,
`scenarios/events/`, `data/reference/stability-2019-event.toml`, and
`data/reference/inertia-constants.toml`; `grid-stability/src` gains only
`pathway.rs` plus lib.rs re-exports. The 2019 defaults in `pathway.rs`
are read-only transcriptions, and the drift guard genuinely guards: it
loads the committed event spec at test time and asserts equality of
responses, damping, demand base and f0 — any retuning of either side
breaks it. Part-1 pins re-ran green (T1–T4, dispatch digest
`779d7444…`, inertia counts 15,020/13,335 bit-identical at scale 1.0).

**(5) FES mapping materiality — QUANTIFIED; DOCUMENTATION MET AT DATA
LEVEL; SENSITIVITY LINE REQUIRED IN THE RUN REPORT (condition 3).**
Reviewer sensitivity runs (remapping the non-synchronous-defaulted ids
to their nearest synchronous class, delivered CLI, same defaults):

| Variant | 2024 min/mean (MW) | ≥1,800 year (mean/min) | 2050 min/mean (MW) |
|---|---|---|---|
| Delivered mapping | 1,373 / 1,573 | 2035 / 2037 | 2,346 / 2,516 |
| waste → synchronous (H 4.0) | 1,382 / 1,589 | **2034** / 2037 | 2,351 / 2,526 |
| hydrogen_turbine → synchronous (H 4.0) | 1,373 / 1,573 | 2035 / 2037 | 2,401 / 2,611 |
| all (+ oil) synchronous | 1,387 / 1,596 | **2034** / 2037 | 2,405 / 2,620 |

So: the waste default moves the mean crossing by one year (physically,
energy-from-waste IS synchronous steam plant — the TOML header says so
itself); the 26 GW of hydrogen turbines arrive too late to move the
crossings at all (+95 MW mean by 2050); the min-condition crossing
(2037) is robust to every remap; the "never below 1,320" and
"2024 unmet" findings are robust to all of them. The mapping decisions
are documented decision-by-decision in the TOML header with explicit
per-id warnings and component splits — the data-level documentation
burden is met. The crossing years must be quoted ±1 year on the
waste/H2 conventions (publication rule 3), and the table above goes in
the run report.

**(6) The tag — YES, CLOSES THE WORK ORDER.** Part-1 ruling (c)
required the Q8 pathway runner and the Module 6 chart, reviewed. Both
are delivered, tested, and reproduced: the pathway runner with strict
parsing, structured errors, docs/06 artefact set and the crossing-year
report; the Module 6 sweep with the hours-below vs renewable-share
chart, scale-1.0 endpoint bit-identical to part 1 and the scale-2.0
pin-after-measure. The FES 2025 HT reference table is provenance-,
licence- and checksum-complete with an independent audit script.
`stage-6-validated` tags on the commit once conditions 1–2 land with it.

## Standard duties

- **No library panics**: `pathway.rs` clean (only `unwrap_or` on
  optionals; the single `losses[0]` index is on a locally-constructed
  one-element vec inside a private fn).
- **Newtypes**: raw f64 only in the TOML-facing raw structs (the
  established boundary pattern); converted at one place; public API is
  Power/Inertia/Frequency/Damping/PerUnit/Duration/Energy.
- **Determinism**: Q8 and Module 6 reruns bit-identical (digests
  above); no wall-clock in library code; `now_utc`/elapsed at CLI only.
  (Note: Module 6 report.toml carries `sweep_seconds` — byte-varying
  like `created_utc`, outside the digest; acceptable.)
- **Strict parsing / structured errors**: schema probe, deny_unknown_
  fields, semantic validation with named-field messages; three new
  `GridError` variants; file-path wrapping; CLI exit 2 on bad spec
  (test-asserted).
- **docs/06 outputs**: CSV+Parquet both; metadata blocks with engine
  hash, input sha256s, data-pack file hashes (Module 6), schema/spec
  id, timestamp; PNG footer embeds engine hash + spec hash + caveats
  (reviewer-inspected).
- **Dependencies**: none new; Cargo.toml/lock untouched by this
  package.
- **TDD**: single-tree delivery per the recorded Stage 0 gate
  (acceptance tests + review); test quality consistent with test-first
  (designed-red closed-form gate, structured-error assertions,
  monotonicity property, drift guard, endpoint pins).
- **Data deliverables (docs/05)**: provenance (5 pinned sources with
  URLs + sha256s), licence verified live, checksums committed,
  deterministic stdlib-only rebuild (byte-identical), audit script,
  documented exclusions. `data/packs/fes2025/raw/` correctly
  git-ignored (fetch-not-ship); `scripts/fes-pathway/__pycache__`
  ignored.
- **Scope**: matches the work order. `--renewable-scale` on `inertia`
  is the Module 6 demo artefact, in scope. No docs/ edits by the
  implementers. Excluded ENTSO-E files not reviewed.

## Conditions (supervisor-actionable; 1–2 in the tag commit)

1. **Pin before quote (docs/05 rule 3).** The FES-pathway headline
   numbers (1,373/1,573 MW; crossings 2035/2037; "never below 1,320")
   currently have **no pinned regression test** — the CLI tests pin
   only the hand fixture and the Module 6 endpoints. Before these
   numbers are quoted in the run report or anywhere else, add a pin on
   `data/reference/fes-pathway.toml` (the result digest `fd410616…`
   and/or the 2024 points + crossing years; same pattern as the Module
   6 scale-2.0 pin).
2. **Stage 6 part 2 run report** must carry at KC-4 prominence:
   (i) the ruling-1 mechanism — the recovery is wholly the
   demand→damping channel (fixed-demand probe: 1,800 MW never met
   2024–2050; series flat 1,373→1,385 / 1,573→1,593 MW), the fixed
   1.836 %/Hz on 2.4× electrified demand as the fragile assumption, and
   the 2019-holdings framing from ruling 1(ii); (ii) the ruling-5
   sensitivity table; (iii) the ruling-2 φ-denominator note (231.4
   ex-PH vs 245.4 incl-PH; stats 0.344/0.147 vs 0.324/0.138);
   (iv) the publication rules below.
3. **Sensitivity line in artefact regeneration**: when `runs/` is
   regenerated post-commit for clean hashes (part-1 condition 5), the
   run report must be the carrier of the mapping-sensitivity numbers
   (the artefact code needs no change).
4. Carried from part 1: docs/06 subcommand-list amendment proposal
   (`stability`) recorded in project-state, not silently edited.

## Publication rules (standing; extend the part-1 §5 rules)

1. Q8 numbers are quotable only as a **band with the condition named**
   (min φ=0.15 / mean φ=0.35, 2024-anchored stated fractions), never a
   single line; the market-only lower edge (zero inertia, zero
   survivable loss) must accompany any quoted band.
2. The 2024 finding is **"2019-era response holdings could not secure
   1,800 MW"** — never "GB today cannot": current NESO holdings are
   larger (dynamic containment ~1.4 GW+) and are a spec input awaiting
   a current-holdings variant.
3. The crossing years (2035 mean / 2037 min) are quotable only as
   **conditional recovery dates**: conditional on (a) 2019 holdings,
   (b) load damping holding 1.836 %/Hz of a 2.4×-grown demand base —
   the sole driver of the rise (ruling 1 probe) and the model's only
   demand pathway, and (c) the technology-mapping conventions (±1 year
   on the waste convention). Preferred wording: "mid-2030s under the
   stated conventions".
4. The φ band is a 2024 market-behaviour anchor applied across the
   pathway; it is not a forecast of 2050 dispatch and must be quoted
   as a convention.
5. "Largest survivable loss" means: single-step loss, 120 s window,
   survival floor 48.8 Hz (LFDD stage 1), no LFDD scheme, dynamic
   services only — quote against the definition, not as an operational
   security standard.
6. Part-1 rules (T1–T4 with inertia bound; the 85.49 % caveat; the
   no-retuning rule) continue to bind.

Reviewer reproduction environment: macOS, workspace at
`[local path]`, uncommitted tree as delivered;
part-1 pins re-run green before and during review; FES raw inputs
checksum-verified; NESO licence page fetched live 2026-07-03.
