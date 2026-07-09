# Stage 7 cost-inputs adjudication — evidence note + DRAFT costs-gb.toml (reviewer)

Reviewer adjudication, 2026-07-03. Subject: the uncommitted
`docs/notes/stage7-cost-inputs-report.md` and
`data/reference/costs-gb.toml` (DRAFT). Reviewed against
`docs/notes/d8-lcoe-methods.md` (rules 1, 4, 8, 9 and the closing
deferral list), the D8 review's non-blocking notes,
`docs/05-validation.md`, the `prices-2024.toml` citation precedent, and
D1/D6 (`docs/08-risks-and-decisions.md`). Verification was adversarial:
primary sources re-fetched this session, conversions recomputed, EAC
statistics recomputed from the pinned snapshot. Scope check: `git
status` shows exactly the two subject files; no docs/04, ADR, schema or
q8-file edits. No code in this package, so TDD/clippy/fmt gates do not
attach; the pinned-regression-test obligation attaches when any number
is first quoted (D8 rule 9 — restated in condition 12).

## VERDICT: ACCEPT-WITH-NOTES

The package is substantially verified: of everything I re-fetched from
primaries this session, **every number matched except one** — but that
one is the Ofgem WACC anchor, and it is mis-stated in a way that
invalidates the proposed low anchor's justification (condition 1,
blocking for the rule-4 WACC pin). A second, smaller defect: the
p/therm conversion factor is internally inconsistent with the package's
own stated therm definition (condition 2, blocking for the SRMC chain).
Everything else is conditions-of-adoption or notes. Nothing here
requires a re-assembly of the package; the supervisor can pin into
docs/04 once conditions 1–6 are discharged.

---

## Spot-check record (re-fetched and recomputed, not trusted)

**Matched exactly:**
1. **Edition.** DESNZ Electricity Generation Costs 2025 published
   14 Jan 2026 (gov.uk publication page, re-fetched); Annex A URL as
   cited; 2025 edition covers the claimed technology list and does NOT
   update nuclear or biomass (report "Nuclear" section: SZC/HPC
   assumptions withheld for commercial sensitivity — the note's
   stale-source framing is correct).
2. **Annex A rows** (all values re-read from the re-fetched xlsx,
   "Technical and Cost Assumptions", 2030 commissioning): CCGT
   (predev [10,20,20] + construction [800,1000,1100]; FOM 16,000;
   ins 2,500; conn 4,400; VOM 5; eff 0.54; life 25; build 3; infra
   £24.4m/1,666 MW; hurdle [.073,.089,.106]), OCGT 299 MW
   ([10,20,20]+[500,600,700]; FOM 11,800; eff 0.35; infra
   £13.2m/299 MW), OCGT 760 MW (FOM 9,100 — see note x2), fixed
   offshore ([110,170,240]+[2100,2500,3100]; FOM 46,000; ins 8,900;
   conn 86,700; life 30; predev 7; LF 48%), onshore
   ([40,80,210]+[1000,1300,1700]; FOM 18,400; conn 17,700; life 35;
   predev 8; infra £16.3m/51.6 MW), solar ([10,30,60]+[400,500,500];
   life 38; infra £6.1m/52 MW), OCHT ([50]+[700,800,900]; FOM 19,400;
   eff 0.25 as published; hurdle [.085,.101,.120]; infra £5.3m/100 MW),
   CCHT ([60,70,80]+[1000,1100,1300]; FOM 14,500; eff 0.51). **Every
   TOML value matches.**
3. **June-2025 gas capex revision** — verified verbatim in the report
   ("Capex estimates were adjusted in June 2025 … lower supply of gas
   turbines worldwide … data centres and countries transitioning from
   coal").
4. **Carbon-assumption method change and Table 4** — verified: LCOEs no
   longer converge to the appraisal cost of carbon; Table 4 (93% LF
   CCGT, 2030, 2024 prices): £165/MWh under 2023 carbon assumptions vs
   £109/MWh under 2025 assumptions = **£56/MWh**, as claimed.
5. **Deflator chain** — FFPA 2025 workbook "UK GDP Deflator" sheet
   re-fetched: 2012 ×1.38733, 2014 ×1.34101, 2020 ×1.171447,
   2021 ×1.173107, 2023 ×1.038364, 2025 ×0.969194. All six TOML
   factors match to 4 dp. All derived conversions recomputed (nuclear
   ×1.3410 incl. FOM 72,900→97,760 and connection 500→670; biomass
   ×1.1731 incl. insurance 13,200→15,490 and connection 14,800→17,360;
   battery ×1.3873; electrolyser ×1.1714; cavern ×1.0384): all correct.
6. **Gas trajectory** — FFPA "Gas" sheet: every year of low/central/
   high in the TOML matches scenarios A/B/C exactly; 2024 outturn
   84 p/therm confirmed. (But see condition 2 on the conversion
   factor.)
7. **Traded carbon values 2025** — published table re-fetched: real
   2025 GBP; market 2025 = £44; central 29/50/70/136/197/235; low
   22/25/45/74/139/167; high 37/66/92/178/250/298. Every TOML value
   recomputes exactly as published ×0.9692.
8. **CEPA hurdle rates** — PDF re-fetched: basis confirmed verbatim
   (pre-tax real CPI, vanilla, whole-life, 50% gearing, 31 Dec 2024);
   bands L-M 7.60 / M 8.90 / M-H 10.10 / H 11.40; **all 14 per-tech
   assignments in `wacc.per_tech_sensitivity` match Table E.1**,
   including interconnectors cap-and-floor at L-M 7.60%.
9. **EGC 2016 nuclear** — PDF re-fetched: 3,300 MW PWR FOAK; predev
   [110,240,640]; construction [3,700,4,100,5,100] (2014£); FOM
   72,900; VOM 5; insurance 10,000; connection 500; infra £11.5m;
   life 60; build 8 (phased 5%/5%/20%×3…); predev 5. All match.
10. **Sizewell C** — press release re-fetched: "target construction
    cost of around £38 billion (2024 prices)", ~20% below HPC;
    £38bn/3.2 GW = £11,875/kW as carried.
11. **BEIS/MM 2018 battery** — PDF re-fetched: all costs 2012 prices;
    Li-ion FM (50 MW/50 MWh) medium capex 590.3 (2018) / 286.6 (2030)
    £/kW; Li-ion PL-DA (200 MW/800 MWh) 1,462.1 / 579.7; PL-DA OPEX
    medium 2030 = 9.3 £/kW/yr (→ ×1.3873 = 12.9 ✓); RTE 85%, DoD 80%,
    5,000 cycles, 15-year life, construction 1–2 yr. Differencing
    arithmetic recomputed: correct. (Caveat added — condition 5.)
12. **HPC 2021 annex** — xlsx re-fetched: PEM 2030 capex
    [433.33, 561.76, 1215.93] £/kW_H2 (2020£); electrical efficiency
    1.27 kWhe/kWh_H2 HHV; FOM 33.38; life 30; build 3; hurdle 10%;
    alkaline 670.92 / 1.25095. Per-kWe derivations recomputed: 442.3
    → 518 (2024£) ✓, alkaline 628 ✓. (But see note x3 on VOM.)
13. **H2 T&S 2023** — PDF re-fetched: new salt cavern (CGH2)
    £0.26/kg levelised, 9 cycles/yr, 250 bar; 0.27 £/kg (2024£) /
    39.39 kWh_HHV/kg = 6.9 £/MWh_H2 recomputed ✓.
14. **GHG CF 2024 condensed v1.1** — xlsx re-fetched: coal
    (electricity generation) gross CV 0.31530 kgCO2/kWh CO2-only,
    0.31699 kgCO2e ✓; wood pellets non-CO2 0.01132 kgCO2e/kWh ✓;
    natural-gas factors consistent with prices-2024.toml.
15. **Viking Link** — press release re-fetched: £1.7bn, 1.4 GW,
    475 miles (= 765 km), commercial operations 29 Dec 2023; derived
    intensities recomputed ✓.
16. **EAC FY2025 holding costs** — recomputed independently from the
    pinned CSV (sha256 verified against the committed manifest
    `data/packs/response-holdings.sha256`): all six products, 2,190
    windows each; volume-weighted mean, mean, p5/p50/p95, min, max —
    **bit-identical to the note's table and the TOML** (DCL 3.31,
    DML 6.05, DRL 14.19, DCH 2.06, DMH 1.24, DRH −3.42, and all
    quantiles). £113m/yr sanity product reproduced. Keys join
    `response-holdings-2025.toml` [[services]] names mechanically ✓.
17. **NREL retrieval obstacle** — corroborated: nrel.gov and
    atb.nrel.gov do not respond from this environment either.
18. **OCHT ambiguity is real** — Annex A publishes 25% "average fuel
    efficiency"; report footnote 9: "100% hydrogen capable turbines …
    reduced efficiency (35% vs c. 38%)". Both citations verified.
19. **TOML parses** (tomllib), DRAFT-headed, per-number citations
    present, structure joins the Stage 7 implementer's needs
    (per-tech tables, [wacc], trajectory arrays, [holding_costs]
    keyed to the holdings file).

**Did not match — the two defects:**

**(A) The Ofgem RIIO-3 anchor is mis-stated** (evidence note §5,
TOML [sources.ofgem_riio3_fd] and [wacc] comment). The Finance Annex
(re-fetched, Table 16 and ¶1.11) gives, for ET at 55% notional gearing:
cost of equity allowance (CPIH-real) 5.70% ✓ — that part is right — but
the **5.53–5.74% figures (5.64% ET unweighted average) are the
SEMI-NOMINAL WACC** (real equity + 90%-nominal debt; Ofgem's own
term, ¶4.3). The **CPIH-real WACC allowance is 4.46–4.67%**
(Table 16 row "WACC allowance (real)"). The note's "allowed vanilla
WACC ≈ 5.6% CPIH-real" conflates the two bases by ~110 bps — a larger
wedge than the CPI/CPIH and tax caveats it does state. The
regulated-asset financing floor, in the real terms this package uses
everywhere else, is ~4.5%, not 5.6%.

**(B) The p/therm conversion factor is internally inconsistent.**
The package states "1 therm = 29.3071 kWh, gross CV" (the UK statutory
therm — correct for NBP/GB gas) but uses ×0.34130, which is the
US-therm factor (29.30011 kWh). The UK-therm factor is **0.341214**.
Consequence: 84 p/therm = £28.66/MWh, not £28.67 — the "agree to the
penny"/"matches … exactly" reconciliation claim against the pack's
daily SAP mean (£28.67) is overstated; true agreement is within £0.01
(still an excellent reconciliation, but it must be stated as what it
is). FFPA 2025 publishes no therm→kWh factor of its own (checked).

---

## Conditions (numbered; 1–6 discharge before the docs/04 pin)

1. **(Blocking — rule-4 WACC pin.) Re-anchor or re-label the low
   WACC.** Correct §5 and the [wacc] comment: 5.6% is Ofgem's
   semi-nominal allowance; the CPIH-real allowed WACC for ET is
   4.46–4.67%. Then either (a) move the low anchor to ~4.5 real
   (unrounded 4.46–4.67) so "everything financed like a regulated
   network asset" is true in the package's own real terms, or
   (b) keep 5.5 with an honest label (a spread point ~1 pt above the
   regulated real floor, not the floor). The "regulated-asset
   financing floor" language cannot stand over a semi-nominal number.
   Central (7.60 CEPA L-M) and high (10.10 CEPA M-H) anchors are
   verified and stand; unrounded anchors stay in the TOML. The
   CPI-vs-CPIH and pre-tax/vanilla caveats as drafted are adequate
   for D8 rule-4 use ONCE the real/semi-nominal error is fixed — no
   further harmonisation required, because the set is a labelled
   sensitivity spread, not a claimed like-for-like composite; but the
   caveat paragraph must add the real-vs-semi-nominal wedge it missed.
2. **(Blocking — SRMC chain.) Fix the therm factor** to 0.341214
   (UK statutory therm, matching the stated 29.3071 kWh) in the note
   §7 and TOML `p_per_therm_to_gbp_per_mwh_hhv`; restate the FFPA
   reconciliation as agreement within £0.01 (28.66 vs 28.67). If
   instead DESNZ's own conversion convention is shown to be the
   US therm, cite that and fix the stated kWh figure — either way the
   pair must be consistent and the basis named.
3. **Battery rows: non-quotable until ATB re-verification.** Ruling on
   weak row (a): the configuration-differencing derivation is
   arithmetically verified and is an acceptable convention for the
   only OGL GB primary supporting a split — ACCEPTED as the draft pin,
   with: (i) the rows carry `quotable = false` (or equivalent) until
   the NREL 2025 storage-cost bracket is re-verified against the
   primary from an unblocked network; (ii) add to the TOML comment
   that the differencing ignores the two configurations' differing
   infrastructure costs (5.8 vs 34.1 £/kW medium, 2012£) and use-case
   archetypes (FM vs PL-DA) — the split is duration-attribution over
   CAPEX only; (iii) the 2018-projection-vintage staleness stamp
   propagates to any artefact quoting a battery-containing cost.
4. **Nuclear: both-variants rule upgraded to both-variants-quoted.**
   Ruling on weak row (b): carrying `desnz_2016` and
   `sizewell_c_observed` as labelled variants is correct and both are
   licence-clean (OGL). "Never quote one without naming which" is
   necessary but NOT sufficient: any published nuclear-containing
   headline must quote the BRACKET (both variants), because the 2016
   set and the SZC project total are different objects (overnight
   component set vs all-in project cost — the basis caveat in the TOML
   is right and must appear wherever the bracket does). Kill-criterion-4
   prominence discipline applies.
5. **Hydrogen cavern: option (a) — carry levelised-with-convention.**
   Ruling on weak row (c): adopt the levelised 0.27 £/kg (6.9
   £/MWh_H2 throughput) with the 9-cycles convention BINDING — make
   normative in the TOML that this number MUST NOT be applied to a
   store cycling materially below the stated rate (a seasonal ~1
   cycle/yr store re-opens the gap; the row is then unusable and the
   capex gap returns as a named blocker). A derived capex (option b)
   would be an unsourced construction — rejected; holding open
   (option c) blocks Stage 7 hydrogen unnecessarily. If Stage 7
   scenarios need seasonal H2 storage, more evidence must be ordered
   then (DEA remains cross-check-only per D1).
6. **OCHT efficiency: bracket before the SRMC chain consumes it.**
   Ruling on weak row (d): default = 25% as published in Annex A (the
   opponent's-defaults discipline — it is the government's number),
   but the SRMC chain may not consume it un-bracketed: a mandatory
   labelled sensitivity at the report-footnote basis (~35% LHV ≈
   ~29.6% HHV via the H2 LHV/HHV ratio ≈0.846) accompanies every
   OCHT-consuming result, the annex-vs-footnote discrepancy is
   stamped on the artefact, and the Baringa H2P primary must be
   checked (one fetch) to resolve the basis before any
   OCHT-containing number is PUBLISHED. Direction stated: the 25%
   figure is the conservative (higher-cost) end.
7. **Emissions rule-9 closure: ACCEPTED as proposed** (weak row e).
   Coal 0.31530 CO2-only for UKA/CPS pricing (0.31699 CO2e for
   accounting) — verified, consistent with the prices-2024.toml
   CO2-only convention. Biomass biogenic CO2 zero-rated for pricing
   (UK ETS treatment) with the 0.01132 non-CO2 factor carried for
   accounting only — accepted. "Other" zero-priced — accepted WITH the
   proposal's own condition made mandatory: every emissions-priced
   artefact reports the unpriced "other" residual as a named quantity
   (TWh and, where a bounding factor is stated, a bounded tCO2 range).
   The rule-9 precondition for emissions-priced cost lines is
   discharged when this lands in the TOML as a normative comment.
8. **CPS convention: ACCEPTED as proposed.** £18/tCO2 nominal frozen,
   deflated along the pathway in real terms (labelled); real-terms
   continuation as the labelled conservative variant. Factual basis
   (frozen nominal) verified by the SN05927 citation precedent already
   in prices-2024.toml.
9. **Interconnector rows: quarantine machine-readably.** Viking Link
   is verified and usable. NSL (secondary-sourced), NeuConnect
   (conflicting figures), Greenlink (unverified) must carry a
   machine-readable non-usable flag (`quotable = false` /
   `verified = false`), not only prose status strings — a committed
   reference file must not let the engine consume them by accident.
   Alternatively move the three rows to the evidence note. CEPA's
   7.60% cap-and-floor rate for interconnector costing is verified.
10. **Holding costs: ACCEPTED.** Volume-weighted means per product,
    p5–p95 ranges, per-product (not pooled — the negative-DRH point is
    correct and material), keys joining response-holdings-2025.toml:
    all verified bit-identical from the pinned snapshot. The
    high-frequency products are recorded-not-consumed, consistent with
    the Q8 exclusion record.
11. **Build phasing: carry it or pin a convention.** Note §1 correctly
    makes escalation-over-the-source's-build-phasing a rule-4
    obligation (verified: DESNZ applies the hurdle rate over published
    phasing, not an IDC line), and the Annex publishes per-tech phasing
    percentages — but the TOML carries only durations. Either
    transcribe the phasing arrays per technology, or pin an explicit
    stated convention (e.g., uniform spend across build years, with
    the approximation named). Without one of these the implementer
    cannot honour §1's consequence and will invent it.
12. **On adoption (not now):** when costs-gb.toml is de-DRAFTed, the
    new reference schema gets its docs/03-domain-model.md note (the
    prices-reference-v1 precedent), the docs/04 Stage 7 TBD-DATA
    slots take the pinned values citing this review, and every number
    later published acquires its pinned regression test (D8 rule 9 /
    docs/05 rule 3). The evidence note's licence table (OGL for all
    DESNZ/BEIS/HMT items; NESO Open Data; Ofgem and National Grid
    quote-with-attribution; DEA cross-check-only; CCC
    quote-with-attribution pending licence clarification; NREL
    flagged-unverified) is APPROVED as D1-compliant: every number IN
    the TOML is licence-clean except the three quarantined
    interconnector rows (condition 9); NREL figures appear only as
    flagged comment-level cross-checks, which is acceptable.

## Notes of record (non-blocking)

- x1. The note's nuclear claim "EGC 2023 uprated 2016 by deflator" was
  not independently verified this session (immaterial: the TOML's
  nuclear numbers are converted directly from the verified 2016
  primary, not via 2023).
- x2. TOML OCGT comment "the 760 MW class is cheaper (GBP 620/kW
  central…)" — imprecise: the 299 MW class central capex is also
  620 £/kW (Annex A: both 20+600); the 760 class is cheaper on FOM
  (9,100 vs 11,800), insurance (1,300 vs 1,500) and high-end capex
  (620 vs 720), not on central capex. Fix wording; the 299 pin itself
  is fine as the peaker archetype.
- x3. Note §2 table's electrolyser "stack repl. in VOM ~0.5 £/MWh_H2"
  does not match the HPC 2021 annex Variable OPEX (0.0031 £/kWh_H2 =
  £3.1/MWh_H2, 2020£, PEM 2030 medium, which per the report includes
  stack replacement). The TOML pins no electrolyser VOM — either pin
  the annex VOM (3.1 → ×1.1714 ≈ 3.6 £/MWh_H2 2024£) or drop the
  ~0.5 claim from the note.
- x4. The TOML CCGT comment cites the June-2025 adjustment to "report
  Section 5"; the passage sits in the unabated-gas part of the
  generation-technologies section — cite it by heading ("Unabated
  gas — Key results") rather than section number.
- x5. FES/CCC cross-check licence treatments are correctly
  conservative; nothing from either is load-bearing in the TOML.
- x6. The fetch-and-build law is honoured (downloaded workbooks used
  for extraction only, URLs recorded); if the supervisor wants
  raw-file provenance pinned like the UKA PDF precedent, that is a
  pack-builder task, not a defect here.
