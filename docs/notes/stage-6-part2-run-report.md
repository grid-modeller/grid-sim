# Stage 6 (part 2) — Q8 pathway runner + Module 6 chart: results

Committed record of the Stage 6 part 2 acceptance runs (2026-07-03),
closing the Stage 6 work order (part 1:
`docs/notes/stage-6-stability-run-report.md`, whose §5 publication
rules and §2 no-retuning rule continue to bind). Adversarial review:
`docs/notes/stage-6-part2-review.md`, ACCEPT-WITH-NOTES — every number
below reviewer-reproduced (bit-identical digests), tag
`stage-6-validated` authorised with this report's conditions landed.
Pathway input: `data/reference/fes-pathway.toml` (NESO FES 2025
"Holistic Transition", annual 2024–2050, NESO Open Data Licence —
published outputs must carry "Supported by National Energy SO Open
Data"; 29/29 spot-checks vs the published report).

## 1. Q8 — largest survivable loss vs year, FES 2025 Holistic Transition

Definition (quote against this, not as an operational standard):
single-step loss at t=0, 120 s window, survival floor 48.8 Hz (LFDD
stage 1), no LFDD scheme in the survival sim, dynamic (droop) services
only; bisection on [0, 5,000 MW] to 1 MW. Response holdings and damping
default to the committed 2019 event spec values (drift-guarded,
flagged `*_defaulted` in every output). Dispatch conditions are a band:
φ = 0.15 (min) / 0.35 (mean) of synchronous capacity committed,
anchored to the measured 2024 H-weighted synchronised share (mean
0.344, p5 0.147, p10 0.169 vs the 231.4 GVA·s full-commitment
denominator). Artefact: `runs/stage6-q8-fes2025-ht/` (digest
`fd410616…`).

**Headline numbers (2019-default holdings, flagged):**

- 2024 survivable loss **1,373 MW (min) / 1,573 MW (mean)** — below
  the 1,800 MW SQSS infrequent-infeed standard from the start.
- The curve **rises** along the pathway, crossing ≥1,800 MW in
  **2035 (mean) / 2037 (min)**; it never falls below 1,320 MW.
- Zero-inertia years never occur on this pathway (CCS gas ≥ ~22 GW and
  nuclear up to 14.2 GW persist to 2050).

## 2. THE FINDING THAT CUTS AGAINST THE TIDY HEADLINE (kill-criterion-4
## prominence — read before quoting ANY Q8 number)

**The work order's framing — "the year the grid can no longer ride
through, as a date" — does not exist under these assumptions. The
delivered result is inverted: the standard is unmet at the start
(2024), and the date the curve yields is a RECOVERY year.**

**And the recovery is fragile.** The reviewer's fixed-demand probe
(demand pinned at the 2024 base, everything else identical): the
recovery vanishes — min moves 1,373→1,385 MW and mean 1,573→1,593 MW
over the entire 26 years, and **1,800 MW is never met, 2024–2050**.
The crossing is therefore wholly the demand-growth→damping channel:
FES demand grows ~2.4× by 2050, demand enters the survival model ONLY
as the load-damping base (1.836 %/Hz of demand), so grown demand =
proportionally more frequency-arresting response. That damping
constant is a 2019 load-composition derivation; an electrified 2050
load (heat pumps, EVs, inverter-interfaced everything) plausibly damps
LESS per MW. The fleet-composition contribution to the recovery is
second-order by comparison.

Consequences, pinned:
1. The safe statement is the 2024 one: **"2019-era response holdings
   could not secure 1,800 MW at 2024 inertia"** — consistent with what
   9 Aug 2019 demonstrated. It must NEVER be read as "GB today cannot":
   current NESO holdings are materially larger (dynamic containment
   ~1.4 GW+); holdings are a spec input awaiting a current-holdings
   variant run.
2. The crossing years are quotable only as **conditional recovery
   dates** — conditional on 2019 holdings, on damping holding
   1.836 %/Hz of a 2.4×-grown demand base (the sole driver), and on the
   FES technology-mapping conventions (±1 year on the waste
   convention). Preferred wording: "mid-2030s under the stated
   conventions".
3. The demand→damping channel being the model's ONLY demand pathway is
   a stated limitation of the v1 survival model, not a discovered
   physical mechanism.

## 3. Sensitivity of the crossings to the FES mapping conventions
## (reviewer-run, condition 3)

| Variant | Mean crossing | Min crossing |
|---|---|---|
| As delivered (waste/oil/marine/H2-turbine non-synchronous) | 2035 | 2037 |
| waste → synchronous (~3.2 GW steam plant) | **2034** | 2037 |
| hydrogen turbines → synchronous (26 GW by 2050) | 2035 (+95 MW at 2050) | 2037 |
| all open-set ids synchronous | 2034 | 2037 |

The conventions move the mean crossing by at most one year; hydrogen
turbines — despite 26 GW — move no crossing (they arrive after the
curve is already above the line). Documented at the mapping sites in
`data/reference/fes-pathway.toml`.

φ-denominator note (reviewer condition): the 231.4 GVA·s anchor
denominator excludes pumped hydro while the pathway inertia sum
includes it (incl-PH: 245.4 GVA·s, stats 0.324/0.138/0.160). Immaterial
(≲1 % in survivable loss; the defaults sit inside either reading) —
recorded, not corrected, to keep the pinned anchor stable.

## 4. Module 6 — hours/year below the inertia floors vs renewable share

Artefact: `runs/stage6-module6-2024/` (digest `484be5d9…`; 10 points,
wind+solar scale 0.25–3.0 on the 2024 reference; share = potential
wind+solar energy ÷ underlying demand, D3 denominator; delivered-share
column alongside). Scale 1.0 (36.1 % share) reproduces the part-1 pins
**bit-identically** (15,020 / 13,335 periods below 120 / 102 GVA·s);
scale 2.0 (72.1 %) pinned at **16,601 / 15,968**; monotone throughout.

**The chart's honest message (same UNCONSTRAINED caveat as part 1 §3,
carried in report.toml, chart footer and console):** even at 9 % share
the market-only counts are 4,080.5 h below 120 GVA·s — **most of the
floor-gap is the market-only dispatch convention, not the renewable
share.** The curve shows how renewables deepen a gap that merit-order
dispatch opens on its own; it is not "renewables cause X hours below
the floor". Do not quote scale points without the caveat sentence.

## 5. Machinery record

- 312 workspace tests → +pinned FES regression (review condition 1;
  test pins the 2024 band values, the crossing years and digest
  `fd410616…` against the committed FES file). Part-1 pins unmoved
  (dispatch digest `779d7444…`, T1–T4, inertia counts).
- Monotonicity-in-loss is structural: dynamic-only services + no LFDD
  ⇒ state-free dynamics, trajectories ordered in L (reviewer verified
  the ODE comparison argument from the code); static latched services
  are rejected at parse with the reason documented — they would break
  bisection. Closed-form gate L* = E(1−(48.8/50)²)/T = 79.04 MW hit
  within tolerance.
- No-retuning audit clean: part-1 constants (swing/spec/inertia, event
  spec, inertia-constants.toml) verified untouched by diff; the
  2019-defaults drift-guard asserts equality against the committed
  event spec.
- Runtimes: pathway 0.46 s; Module 6 sweep 0.12 s (Stage 4 lean-sweep
  remediation not needed here).

## 6. Publication rules (standing; extend part-1 §5)

1. Q8 numbers only as a **band with the condition named** (min φ=0.15 /
   mean φ=0.35, 2024-anchored); the market-only lower edge (zero
   inertia ⇒ zero survivable loss) accompanies any quoted band.
2. The 2024 finding is "2019-era response holdings could not secure
   1,800 MW" — never "GB today cannot".
3. Crossing years only as conditional recovery dates (§2), "mid-2030s
   under the stated conventions".
4. The φ band is a 2024 market-behaviour convention, not a 2050
   dispatch forecast.
5. "Largest survivable loss" is defined by §1's simulation contract —
   quote against the definition, not as an operational security
   standard.
6. Module 6 scale points never without the market-only caveat (§4).
7. Part-1 rules (T1–T4 with inertia bound; the 85.49 % caveat; the
   no-retuning rule) continue to bind.
