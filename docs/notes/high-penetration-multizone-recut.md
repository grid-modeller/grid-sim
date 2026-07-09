# High-penetration artefacts — the multi-zone re-cut (results addendum)

**Status:** collation addendum, 2026-07-06 (boundary-loose-ends package;
reviewer-verified as collation-only in the quarantine-lift review,
`docs/notes/quarantine-lift-review.md` §5, committed 99600d3). Closes
the tracked beta item "re-cut
high-penetration artefacts as measured multi-zone results"
(memory/project-state.md, V1/BETA item 2 tail; carried on the
boundary-loose-ends list).

**Scope statement (honest):** the measured multi-zone counterparts of the
single-zone high-penetration records were delivered PROGRESSIVELY by the
committed b6 two-zone, b4 three-zone, D11 tier-2 sweep and D13 composed
packages — each with its own reviewed pins. **This note introduces no new
measurement.** It is the missing collation: the old single-zone records
and their measured multi-zone counterparts at the 60 GW headline point,
side by side, with the boundary attribution stated. Every number below is
a committed pin (post-R7-fix values, 2026-07-06 re-pin; the pre-fix
values are recorded per pin at the named test constants) and carries its
record's mandatory caveat set — quote ONLY through the named source
records, never from this table alone.

## 1. Q2 — curtailment at 60 GW GB wind (Module-1 scaling, 60/29.1)

| Instrument | measured at 60 GW | pin / record |
|---|---|---|
| Single-zone GB copper plate, tier-1 import conventions | frozen 21.85 / zero 17.80 / export 5.33 TWh | `regression_imports_bracket_2024.rs` (Package B); quoting frame: one-sided bound per d11-sweep-review §F(i) |
| 5-zone, endogenous imports (GB still an internal copper plate) | **3.98 TWh** (3.982736889304) | `acceptance_d11_sweep.rs` `PIN_60_CURTAILMENT_TWH`; quote per `d11-sweep-run-report.md` §4, all five caveats (a)–(e) |
| 2-zone GB (B6 at flat 4.1/3.5 GW) | SCO **27.03** vs copper 10.93 TWh; B6-attributable SCO **+16.09** / RGB **−7.24** / system net **+8.85** TWh | `acceptance_b6_2zone.rs` `PIN_60GW_*`; condition-7 three-leg rule (never the SCO delta alone); LOWER BOUND, B6-only slice |
| 3-zone GB (B4+B6) | Scottish (NSCO+SSCO) **31.57** (24.54 + 7.03) vs copper 27.25 TWh; RGB 2.68 vs 3.71 | `acceptance_b4_3zone.rs` `PIN_60_*`; LOWER BOUND (B5 folded), increment DIRECTION only, no B4-vs-B6 decomposition |
| Composed 8-zone (3-zone GB + 5 externals) | LP minimum SYSTEM waste **36.22 TWh** (any dispatch); rule-based GB-curtailment ceiling **29.91 TWh**; deconfounded exceedance **≥ +20.0 TWh** over the tier-2 3.98 (pre-R7: 4.01) central | `acceptance_d13_60gw.rs`; quote ONLY the `d13-run-report.md` §5 E.1 sentence with basis labels (GB curtailment vs system waste) |

**Boundary attribution (the point of the re-cut):** the single-zone
copper-plate curtailment story does not survive the measured boundaries,
in either direction, and the two directions are different mechanisms:

- **Within-GB geometry pushes Scottish curtailment UP.** The B6 boundary
  alone adds +16.09 TWh of SCO curtailment at 60 GW with a −7.24 TWh RGB
  counter-movement (system net +8.85 TWh vs the two-zone copper plate);
  adding B4 tightens the Scottish bound further (31.57 vs the two-zone
  27.03). These are lower bounds on the Scottish constraint phenomenon.
- **Endogenous interconnection pushes GB curtailment DOWN** — the 5-zone
  3.98 TWh central sits below the tier-1 export floor (5.33) — but that
  instrument wheels northern wind south across an internal copper plate
  (caveat (e): landing points south of both boundaries), so its level is
  upper-side on exports/capture and lower-side on curtailment.
- **The composed measurement adjudicates the level:** with B4/B6 AND the
  2024 external system represented, no dispatcher can waste less than
  36.22 TWh at 60 GW (perfect-foresight floor; composed-anchor baseline
  12.20 TWh; wind-driven increment +24.03 TWh). The caveat-(e)
  curtailment component is RESOLVED AGAINST the tier-2 copper-plate
  level; 3.98 TWh (pre-R7: 4.01) is quotable only alongside that
  record.

## 2. Q10 — capture at 60 GW GB wind

| Instrument | measured at 60 GW | pin / record |
|---|---|---|
| Single-zone tier-1 bracket (copper plate, frozen/zero/export conventions) | delivered 0.5514–0.6106 (design envelope 0.535–0.611); potential 0.5348 | `regression_imports_bracket_2024.rs`; quotable only as the frozen-convention record with the §4(b)(iii) understatement caveat — never as an error bar |
| 5-zone, endogenous imports (scarcity rule central) | delivered **0.6975** / potential 0.6815; gas 40.67 TWh; GB net imports **−6.46 TWh** (exporter); priced-ladder sensitivity 0.6782 | `acceptance_d11_sweep.rs` `PIN_60_*`; the tier-2 central is ABOVE the whole tier-1 envelope — one-sided-bound ruling §F(i) binding, caveats (a)–(e) |
| Composed geometry | **NO composed capture instrument exists** (D13 ruling D / §5 E.4) | 0.698 remains an upper-side estimate on caveat (e)'s capture axis; named resolver = the economic-dispatch (min-cost) LP, D14 — NOT SCHEDULED |

**Boundary attribution:** the bracket-escape DIRECTION (tier-2 above the
tier-1 envelope on every axis) is like-for-like (tier 1 was also
copper-plated) and survives; the ABSOLUTE level 0.698 is upper-side on
both the frozen-2024-externals convention (caveat (a)) and the
uncomposed B4/B6 geometry (caveat (e)). Export survival at the composed
point is OPEN under the asymmetric evidential rule (d13-run-report §5
E.3): the composed rule-based floor reads +11.70 TWh net imports, which
is not evidence of collapse; the copper-plate −6.46 TWh is quotable only
alongside that OPEN status.

## 3. Reference-fleet (anchor) rows, for completeness

Two-zone: SCO constrained 1.679 vs copper ~0.0001 TWh; system net
+1.679 TWh (`acceptance_b6_2zone.rs` `PIN_REF_*`). Three-zone: NSCO
6.914 / SSCO 0.061 constrained vs NSCO copper 6.871
(`acceptance_b4_3zone.rs` `PIN_REF_*` — the NSCO copper value is the
equal-depth stranding artefact family; see the D13 package-1 record
before quoting it against any national anchor). The standing
CF-calibration caveat travels on every high-penetration number: 2024
constraint curtailment is frozen INTO the calibrated CF traces, so
scaled fleets inherit 2024's curtailment rate — error grows with
penetration, always in the flattering direction.

## 4. The remainder, named

1. **The Module-1 chart artefact** (Stage 2 single-zone gas-share /
   capture curve, 10→60 GW) has not been re-cut as a multi-zone CHART.
   The 5-zone sweep machinery (`wind_capacity_sweep_multi`) computes the
   full curve, but only the anchor and 60 GW points are pinned/quotable
   (caveat (d), docs/05 rule 3) — re-cutting the chart is an
   artefact/CLI task plus a quoting ruling on intermediate points, not
   an engine gap.
2. **The 40 GW bracket point** has tier-1 pins
   (`regression_imports_bracket_2024.rs`) and no measured multi-zone
   counterpart (same caveat (d): nothing between the pinned points is
   quotable).
3. **Composed capture and net-trade centrals** remain OPEN — resolver
   D14 (min-cost LP), not scheduled, Richard's call.
4. **The correlated-external-wind sensitivity**
   (d11-sweep-run-report §10, priority 2) is named, not run.
