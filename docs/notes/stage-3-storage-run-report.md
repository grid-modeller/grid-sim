# Stage 3 — 40-year storage runs: results, the Royal Society comparison, and unit honesty

Committed record of the Stage 3 acceptance runs (2026-07-03). All numbers
were produced by the engine against the complete 1985–2024 weather record
(Earthmover-sourced 1985–2023 + Phase A 2024; snapshot
`39TK56WX185WZ1HP9WNG`; manifests `era5-1985-2023.sha256`,
`cf-1985-2024.sha256`). The Royal Society comparison below is anchored to
the published report itself (*Large-scale electricity storage*, Sept
2023, ISBN 978-1-78252-666-7, CC-BY 4.0), read in full by a research
agent the same night — page references in the project transcript.

## 1. Headline results

**The benign-year illusion, quantified (Module 3a vs 3b).** A
renewables-heavy fleet that runs 19.1 GWh short over one benign year —
fully cured by a 12 h / 36 GWh battery — fails on the full 40-year
record in **33 of 40 years**, accumulating **557.4 GWh of unserved
energy over 574 half-hours**. Same fleet, same battery; the only change
is using weather that actually happened. "A few days of storage" is an
artefact of single-benign-year analysis.

**The storage-overbuild curve (RS-style fleet: wind+solar+hydrogen,
570 TWh/yr demand, 100 GW symmetric store power, η_rt = 0.40):**

| Avg supply ÷ demand | Min hydrogen store (engine units*) | Delivered-electricity equivalent |
|---|---|---|
| 1.15× | **INFEASIBLE** — no store size achieves zero unserved (pinned) | — |
| 1.35× | 58.4 TWh (pinned; min SoC 21.7 GWh, 2011-04-25) — **the RS-comparable point**: the RS sized supply at 1.23–1.40× | ≈ 36.9 TWh_e |
| 1.64× | 28.3 TWh (pinned; min SoC 13.4 GWh, 1997-02-01) | ≈ 17.9 TWh_e |
| 1.92× | 23.9 TWh (pinned; min SoC 8.9 GWh, 1989-12-13) | ≈ 15.1 TWh_e |

All four points are regression-pinned
(`grid-adequacy/tests/acceptance_stage3_rs37y.rs`; scenario files
`royal-society-37y{,-lean,-mid}.toml`). Nadirs land in three different
decades (1989, 1997, 2011) — which winter binds depends on fleet
sizing, an argument for the full record no single design-year study can
make.

*Engine store units are the D4 symmetric-√η convention: delivered
electricity = stored units × √0.40 ≈ 0.632. This differs from BOTH
common publication conventions; see §3.

The binding behaviour depends on sizing, and the stage demo artefact is
the RS-comparable lean point at its pinned requirement
(`runs/rs-37y-lean-at-requirement/`): longest below-full episode
**720.6 days (2009-12-09 → 2011-11-30**, nadir 2011-04-25) — the RS
report's own binding window, reproduced — with runner-up episodes of
442 days (1996–98) and 407 days (1987–88), and the store at full only
15 % of all periods. (At the overbuilt 1.92× point the same store
refills within ~8 weeks — overbuild buys recovery speed; the lean/RS
sizing is where multi-year recharge lives. The lean shape is pinned;
the 1.92× contrast is recorded in the pin test's documentation.)
GB's existing grid storage is ~0.03 TWh_e.

## 2. The Royal Society comparison — structure, order, and (at their
supply sizing, in comparable units) magnitude reproduced; their
electrified-heat headline awaits Q5

RS central results (their report, per the primary-source read): storage
60–100 TWh **hydrogen-LHV** (×0.55 → ≈ 33–55 TWh delivered
electricity); supply sizing 1.23–1.40× demand; hard feasibility
threshold at **1.234×** (RTE-dependent); binding weather 2009–2011;
80/20 wind/solar; 74%/55% leg efficiencies (RTE 40.7%); demand = an
AFRY 2050 *electrified-heating* hourly profile.

What reproduces:
- **The threshold cliff**: ours between 1.15× (infeasible) and 1.35×
  (feasible); theirs at 1.234×. Consistent.
- **The steep overbuild trade-off**: both models halve the store for
  ~0.5× more supply (their published Fig. 12/23 curve; our table above).
- **The order of magnitude**: tens of TWh delivered-electricity, i.e.
  three orders above extant GB storage — the argument-bearing fact.
- **Multi-decade weather dependence**: their p.6 warning ("studies based
  on less than several decades of weather data are liable to very
  seriously underestimate the need for storage") is our Module 3(a)/(b)
  contrast, computed.

What differs, and why (each nameable, none hidden):
1. **Units**: RS headline is hydrogen-side LHV; ours is a symmetric-√η
   store. Headline-to-headline comparison misstates the gap by ×1.15
   (0.632/0.55 at constant delivered energy); the larger trap is
   comparing either headline against charged-electricity figures, which
   spread by 1/RTE ≈ 2.5. Convert to delivered electricity (§3) before
   any comparison.
2. **Demand shape**: RS used a 2050 electrified-heat profile
   (temperature-correlated, winter-peaked); we tile today's 2024 profile
   (Q5's heating overlay is future work). Electrified heat is *the*
   storage amplifier — our flatter profile systematically needs less.
   This is the largest expected contributor and it will be quantified,
   not assumed, when Q5 activates.
3. **Weather source and window**: renewables.ninja/MERRA-2 1980–2016
   (April-to-March years) vs our ERA5-derived 1985–2024 (calendar
   years). Their record includes 1980–84; ours includes 2017–2024
   (incl. the 2021 drought). Their binding 2009–2011 event is inside our
   record too.
4. **Power ratings and mix**: their electrolyser power is optimised
   (89–169 GW); ours fixed at a generous 100 GW symmetric. Their mix
   80/20 wind/solar with 70/30 off/onshore; ours 86% wind.

**Publication rule (kill-criterion discipline):** any quoted comparison
must state the unit convention and the demand-profile difference. On
the delivered-electricity basis (the only safe one, §3), our
RS-comparable lean point — 36.9 TWh_e at 1.35× supply — sits **inside**
the RS delivered band (33–55 TWh_e), at 84 % of their central ≈44.
"The simulator reproduces the Royal Society's storage requirement at
their supply sizing, in comparable units" is supportable. What is NOT
reproduced — and cannot be until Q5 — is their number under their
2050 electrified-heating demand profile; our 2024-shaped profile is
systematically less storage-hungry, so our agreement likely flatters
the comparison, and the residual profile effect is itself a finding
(the storage cost of electrifying heat, to be computed at Q5).

## 3. Store-unit conventions (write once, cite forever)

For a store with round-trip efficiency η holding S "stored units":
- **Engine (D4)**: symmetric legs √η each; delivered = S·√η; charged
  from surplus = S/√η.
- **RS report**: hydrogen-LHV space-side; delivered = S·0.55 with their
  74%/55% legs.
- **Grid-side (deliverable)**: what the grid can actually draw — the
  only convention safe for cross-study comparison. All published
  comparisons convert to this.

## 4. Test/pin status

Acceptance (docs/04 Stage 3): SoC conservation + exclusivity properties
(part 1); benign-year 12 h battery zero-unserved (part 1); RS-style
scenario in the published order of magnitude — **PASS** at the pinned
1.92× point, with the 1.35× lean point and the 1.15× infeasibility
pinned alongside as first-class results; SoC continuous across all 39
year boundaries (reset-proof test). The 2024 CF traces regenerated
byte-identical throughout the source switch; the Stage 1/2 validation
pins were never touched by any of tonight's data work.

## 5. Boundaries carried forward

Rule-based (zero-foresight) policy per D4 — the Stage 7 LP will cycle
storage economically and produce LOWER requirements (kill-criterion 3
gap reporting); no reserve holding; demand tiling has no weather-demand
correlation (heating Q5) and no growth; 100 GW power un-optimised;
hydrogen store is a single aggregate (geology unmodelled, docs/05).
