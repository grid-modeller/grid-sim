# D3 — Embedded-generation convention (resolved 2026-07-02)

**Decision: the total-generation ("gross") convention** — option (b) of
`docs/notes/2024-validation-pack-report.md` §2:

- Modelled demand is **underlying demand**:
  `demand(t) = ND(t) + NESO embedded-wind estimate(t) + NESO embedded-solar
  estimate(t)`. The pack carries this as a built column so the convention
  is reproducible from source data.
- The fleet includes embedded capacity explicitly (18.7 GW solar, 6.6 GW
  embedded onshore wind), with capacity-factor traces covering the whole
  fleet.
- Validation targets are restated in the same convention: total wind
  82.61 TWh (transmission 65.64 + embedded 16.97), solar 13.95 TWh
  (NESO estimate), gas unchanged at 72.79 TWh (embedded output nets out of
  the residual either way). Monthly-mix correlation is computed with model
  and target in this single convention.

## Why (b) and not (a) (ND demand + embedded stripped from the fleet)

1. **Future scenarios are the point of the tool.** Under (a), solar — which
   is effectively 100 % embedded in GB — does not exist in the model, and
   FES-style 2035+ fleets with tens of GW of solar cannot be represented
   without switching convention mid-project. NESO FES itself models gross
   demand and embedded generation explicitly.
2. **Demand overlays apply to underlying demand.** The heating overlay (Q5)
   and `annual_scale` growth act on what consumers use, not on the
   transmission-metered residual of it.
3. **The metric ceiling.** Mixing conventions caps the monthly-mix
   correlation at 0.973 (report §4); a single pinned convention removes
   that wedge.

## Cost, stated

- Solar (and embedded-wind) validation inherits NESO's estimation error —
  and is partly circular, since the NESO estimate is both the input to
  underlying demand and the target. Order 1–2 TWh; already inside the ±5 %
  gas-tolerance evidence (report §7). Documented as a model boundary in
  `docs/05-validation.md`.
- Station transformer load (≈ 0.67 GW mean, 5.86 TWh/yr) remains a supply-
  side wedge under either convention; the Stage 1 validation harness must
  account for it explicitly (report §3).
