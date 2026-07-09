# Quarantine-lift + boundary-loose-ends package — independent review

Independent reviewer (quarantine-lift gate), 2026-07-06. Subject: the
UNCOMMITTED package in the working tree — 10 modified files +
`docs/notes/high-penetration-multizone-recut.md` — reviewed against the
Stage 7 cost-inputs review conditions 3.i/3.ii/3.iii and 9
(`docs/notes/stage7-cost-inputs-review.md`), the committed battery
evidence record (23676f1 + the costs-gb.toml cross-check block), the b6
engine-review note 6 (`docs/notes/b6-two-zone-engine-review.md`), and
the Q9/pathways review condition-3 discipline. Verification was
adversarial: the NREL primary re-read from the pinned PDF, both
red-first claims re-derived by reverting the fixes in a scratch
worktree and watching the new tests fail, all 14 scenarios re-scanned
independently, and every quoted number in the re-cut note re-checked
against its pinned test constant. Base moved from 33e6570 to 3d7d113
during review (two governance-docs commits, f49d3c8 and 3d7d113 —
docs-only, no effect on the code under review; noted for the record).

## VERDICT: ACCEPT-WITH-CONDITIONS

The lift is exactly the coordinated reviewed act the parser guard
demanded, the evidence chain holds against the primary, no numeric pin
moved anywhere, both red-first stories are genuine (reviewer-reproduced,
not trusted), the re-cut note introduces no new measurement and every
number in it matches its committed pin, and the gates are green
(fmt clean, clippy `-D warnings` clean, workspace suite
**698 passed / 0 failed / 4 ignored** — the 4 are the tractability
benchmarks — summed by the reviewer across all 71 suite results, run
twice). Three conditions attach before commit; none requires
re-assembly.

---

## Verification record (re-derived, not trusted)

**1. The lift is the reviewed act (VERIFIED).**
- The old parser guard (`git show HEAD:grid-core/src/costs_reference.rs`,
  battery block) rejected `quotable = true` with the message "…lifting
  the quarantine requires a reference revision citing the
  re-verification". This revision is that act: the guard is removed
  with a comment block citing condition 3.i's discharge
  (NREL/TP-6A40-93281 via OSTI, `data/packs/costs-evidence.sha256`,
  evidence commit 23676f1), and the successor test
  `battery_quotability_is_byte_pinned_to_the_discharge_citation`
  byte-pins the LIFTED flag line WITH its citation (reviewer probe:
  the exact flag line occurs exactly once in the TOML; the pinned
  string matches the file bytes).
- Evidence chain: `costs-evidence.sha256` verifies on disk (both
  files OK, `shasum -c`). The NREL numbers in the comment were
  re-derived from the pinned PDF itself, not the commit body: Table 2
  (appendix p. 16) 4-hour overnight $/kWh 2030 low/mid/high
  **207/279/354**, 2035 **147/243/339**, 2050 **108/178/307** — all
  match; "lifetime we selected is 15 years", "round-trip efficiency is
  chosen to be 85%", FOM "4% of the $/kW capacity cost" all verified
  verbatim in the report text; the bracket arithmetic recomputed
  (135 + 262/4 = 200.5 £/kWh ≈ $257 at 1.28 → inside [207, 279],
  between low and mid).
- CONFIRMED-not-corrected: the battery numeric pins
  262 / 135 / 12.9 £/kW-yr / 15 yr / 0.85 RTE are byte-unchanged (the
  TOML diff contains exactly two value changes in the whole file: the
  battery `quotable` flag and the NSL estimate — everything else is
  comments).

**2. Quotability flip + the staleness catch (VERIFIED).**
- The four pathway stacks consume no interconnector (the cost spec in
  `acceptance_stage7_pathways.rs` builds `interconnectors: vec![]`,
  `holdings: vec![]` — line 4 is a structural zero), so
  NSL/NeuConnect/Greenlink are never consumed; the renamed test
  `quarantine_declaration_is_affirmatively_empty_and_stacks_are_quotable`
  asserts the affirmatively-EMPTY set on all four scenarios and was
  re-run by the reviewer standalone (pass).
- The staleness catch is genuine: in a scratch worktree with the
  package applied but the OLD `costs.rs` (stamp push gated on
  `!battery.quotable`, old lines ~552-562), the new test
  `lifted_battery_row_is_quotable_and_the_staleness_stamp_still_travels`
  FAILS with "staleness stamps: []" — the old code really would have
  dropped caveat 3.iii the moment the flag flipped. The fix is
  minimal: the push moves outside the quotable branch but stays inside
  the `!spec.batteries.is_empty()` gate, so no stamp attaches to
  battery-free stacks. Caveat 3.ii's derivation-caveat block is still
  in the TOML row.
- The quarantine machinery stays genuinely tested: the
  `requarantined_battery_reference()` string mutation was re-run by
  the reviewer outside the tests (target string occurs exactly once;
  the mutated TOML parses with `quotable = false`; the committed TOML
  parses `true`) — discriminating, not vacuous.

**3. NSL correction (VERIFIED, no engine effect).** 2.0 → 1.6 EUR bn
with the Statnett owner-primary status string; `verified = false` /
`quotable = false` unchanged; the mutation literal in
`quarantined_interconnector_with_a_point_capex_is_rejected` updated to
match. `capex_eur_bn_estimated` is deserialised into a
`#[allow(dead_code)]` field and never surfaced; the parser rejects any
GBP point capex on a quarantined row — the value is dead-code-recorded
as claimed.

**4. Duplicate-TechId (loose end f — VERIFIED).**
- Characterisation checked in code: `BTreeMap<TechId, …>` plain
  `.insert` (last-wins) for CF traces, availability, budgets, SRMC
  (`grid-adequacy/src/inputs.rs:86/91/347/364`, `pricing.rs`);
  dispatch builds one unit PER fleet entry (`dispatch.rs:274`,
  `multizone.rs:834`); result readouts take the FIRST matching series
  (`result.rs:165`). A within-zone duplicate would genuinely mix
  last-wins inputs with both-entries dispatch — silently corrupt.
- Red-first reproduced: with the OLD `scenario.rs` and the new test,
  `duplicate_fleet_technology_within_a_zone_is_a_validation_error`
  fails on `unwrap_err()` of `Ok(())`. Structured
  `GridError::DuplicateFleetTechnology { zone, technology }`;
  cross-zone same-id legality has its own test.
- Scenario scan re-run twice, independently: `grid-cli validate`
  (which calls `Scenario::validate`) on all 14 committed scenarios —
  14/14 OK — and a tomllib per-zone duplicate scan — 0 hits. The full
  suite exercises the rule on every scenario (`load_run_inputs` runs
  `Scenario::validate` first; the acceptance suites collectively load
  all 14).

**5. The re-cut note (loose end g — VERIFIED as collation-only).** No
new measurement: every number checked against its committed post-R7
pin (well beyond the six asked) — 5-zone 3.982736889304 / 0.6975 /
0.6815 / 40.67 / −6.46 / 0.6782 (`acceptance_d11_sweep.rs`); 2-zone
27.03 / 10.93 / +16.09 / −7.24 / +8.85 / ref 1.679 / ~0.0001
(`acceptance_b6_2zone.rs`, old values recorded per pin); 3-zone
31.57 = 24.54 + 7.03 vs copper 27.25, RGB 2.68 vs 3.71, ref 6.914 /
0.061 / 6.871 (`acceptance_b4_3zone.rs`); composed 36.22 / 29.91 /
12.20 / +24.03 (= 36.224 − 12.197) / ≥ +20.0 / +11.70
(`acceptance_d13_60gw.rs`); tier-1 21.85 / 17.80 / 5.33, capture
0.5514–0.6106, potential 0.5348
(`grid-cli/tests/regression_imports_bracket_2024.rs`). All match. The
cited record sections (d11-sweep-run-report §4 caveats (a)–(e),
d13-run-report §5 E.1/E.3/E.4) exist. The named remainder is honest:
the Module-1 multi-zone CHART, the 40 GW point, D14
capture/net-trade, and the correlated-wind sensitivity are genuinely
absent from the pinned record.

**6. Quoting records — RULING (condition 2 below).** Post-lift, two
places in the standing record state the pre-lift world as current:
`docs/notes/stage7-run-report.md` §4 (the condition-3 paragraph:
"consumed quarantined rows: storage.battery_li_ion — NON-QUOTABLE…",
naming the now-renamed test) and §8 (the nuclear bracket rule
"satisfied trivially: nothing is quotable… becomes live the moment the
NREL re-verification lifts the battery quarantine" — it is now live).
Nothing else needs treatment: the manual's cost chapter is a skeleton
("numbers pending"; its two "non-quotable" mentions are the permanent
LP-aggregates ruling, unrelated); docs/04's Stage 7 clause ("battery
split until NREL re-verification") reads correctly once the "until" is
satisfied; the dated reviewer adjudications (stage7-pkg1-review,
stage7-q9-review, stage7-cost-inputs-review) are historical records
and stay untouched, per the R7 precedent.

**7. Gates (VERIFIED).** `cargo fmt --check` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean; full
workspace suite green twice, reviewer-summed **698 / 0 / 4** (the 4 =
`tractability_bench` probes). No numeric pin moved: the four
`*_pins_are_exact` pathway tests are byte-untouched by the diff (zero
pin-constant lines changed) and pass; `cost_stack.rs`,
`q9_decomposition.rs`, `costs_reference.rs` (29 tests incl. every
pinned value) and all digest/regression suites green in the same run.
No `Cargo.toml`/`Cargo.lock` change — no new dependencies.

---

## Conditions (ordered; 1–3 discharge before commit)

1. **(Blocking) Amend the docs/03 costs-reference-v1 note.**
   `docs/03-domain-model.md` (reference schema note, semantics
   clause 1) still states "The battery row must remain
   `quotable = false` until the reference is revised citing the NREL
   re-verification (review condition 3) — the parser rejects a silent
   lift." Both statements are false in this revision. Add a dated
   amendment recording the 2026-07-06 lift, the discharge citation,
   and the successor discipline (the lifted flag line is byte-pinned
   with its citation; caveats 3.ii/3.iii remain and propagate).
   RULING: no schema-string bump — the struct shape and field set are
   unchanged and the flip is the reviewed act v1's own note
   anticipated; but the note of record may not contradict the parser.
   (Minor, same edit: docs/03 §4's semantic-validation list may take
   one line for fleet-TechId uniqueness; the list is already
   non-exhaustive, so this is tidiness, not a defect.)
2. **(Blocking) Quoting-record banners, R7 precedent.** Add dated
   status banners (not rewrites) to `docs/notes/stage7-run-report.md`:
   at §4 — the condition-3 declaration is affirmatively EMPTY since
   the 2026-07-06 lift; the asserting test is renamed
   `quarantine_declaration_is_affirmatively_empty_and_stacks_are_quotable`;
   caveats 3.ii/3.iii still travel — and at §8 — the nuclear
   both-variants obligation is no longer satisfied trivially; it is
   LIVE on every quotable nuclear-containing stack, and the metadata
   carries it (asserted in the suite). No other document requires
   treatment (enumeration in item 6 above).
3. **(Pre-commit hygiene) The stray `figures/` files**
   (`rs-37y-storage-trace.csv`, 23 MB, + `.png`, dated 2026-07-04 —
   B4-era artefacts) are not part of this work order and must not
   ride along with the package commit.
4. **(Note) TDD evidence.** The package is uncommitted, so
   commit-order evidence cannot exist yet; red-first was instead
   verified empirically by this review (both probes above watched
   fail against the reverted code). The commit message should state
   the red-first story so the record survives.
5. **(Note, pre-existing)** `acceptance_d13_60gw.rs` header (~line 73)
   still narrates the pre-R7 ceiling 30.175 TWh in a dated 2026-07-05
   "MEASURED" block while the pins carry the re-pinned 29.910 —
   consistent with the old-values-recorded convention, but worth a
   one-word "(pre-R7)" at next touch. Not this package's defect.

## Reviewer-reproduced numbers (appendix)

Suite totals 698/0/4 over 71 result lines (two independent full runs,
second summed line-by-line). Manifest: both costs-evidence files OK.
NREL Table 2 re-derivation as §1. Byte-pin uniqueness: 1 exact match.
Mutation probe: 1 target occurrence, parsed flag flips true→false.
Red probes: staleness test fails "stamps: []" on old costs.rs; duplicate
test fails "unwrap_err on Ok" on old scenario.rs; both green with the
package applied (cost_stack 16/0, scenario suite green in-run).
Scenario scans: grid-cli 14/14 OK; tomllib duplicate scan 0 hits.

— independent reviewer (quarantine-lift gate), 2026-07-06
