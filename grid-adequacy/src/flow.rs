//! The Stage 5 interconnector flow rule (docs/04 Stage 5; ADR-7).
//!
//! # The flow rule, in prose (normative — D4 precedent: the contestable
//! # modelling choice is documented at its definition site)
//!
//! Imports and exports **emerge from relative scarcity and price**; no
//! flow is ever assumed. Each half-hourly period, before any storage
//! action (link trades clear ahead of residual balancing, as day-ahead
//! market coupling clears ahead of within-zone balancing — a stated v1
//! convention):
//!
//! 1. **The flow signal.** Every zone gets a signal from its current
//!    net position, using a **common merit ladder** — the fixed
//!    [`crate::MERIT_ORDER`] technology ordering (nuclear < biomass <
//!    hydro < coal < ccgt < ocgt), the engine's documented SRMC proxy
//!    since Stage 1. Two signal forms are selectable per scenario
//!    (`dispatch.flow_signal`, schema v7 — the ADR-6 policy set
//!    generalised to {scarcity-rule, priced-ladder,
//!    perfect-foresight-LP}, all reported):
//!
//!    **1a. The scarcity score** (`"scarcity"`, the default — the
//!    Stage 5 validated behaviour, byte-identical to pre-v7 runs):
//!
//!    - a zone in **surplus** (must-take exceeds demand) has signal
//!      `−surplus` in GW — deeper surplus, stronger exporter (this is
//!      the negative-price region: two surplus zones still trade toward
//!      equal surplus depth, the model's analogue of negative-price
//!      exports shifting curtailment to the cheaper-to-curtail zone);
//!    - a zone dispatching its stack has signal
//!      `ladder index of the marginal technology + fractional
//!      utilisation of that technology's available capacity` — e.g.
//!      "ccgt, 30 % dispatched" reads 4.3. Technologies a zone lacks
//!      are simply absent: its signal jumps across the gap (its next
//!      MW really does cost the next rung). Fractional utilisation is
//!      the tie-breaker *within* a rung: between two gas-marginal
//!      zones, energy flows toward the zone whose gas fleet is
//!      proportionally more stressed — the price proxy for "whose
//!      marginal unit is more expensive right now";
//!    - a zone past its whole stack has signal `6 + unserved` in GW —
//!      unserved energy outbids everything.
//!
//!    The scarcity score is dimensionally a score, not a price:
//!    monotone in stress, comparable across zones **only** through the
//!    shared ladder. That comparability is its central modelling
//!    assumption (equal technology, comparable marginal cost), and it
//!    is wrong exactly where fuel/carbon prices diverge between zones —
//!    the stated boundary the priced ladder (1b) addresses; under the
//!    scarcity signal it remains bounded by the acceptance gates
//!    (docs/04 A1–A4).
//!
//!    **1b. The priced ladder** (`"priced_ladder"`, D11 — selectable
//!    for priced/market questions; requires `[zones.pricing]` on every
//!    zone). The signal becomes **lexicographic: (marginal SRMC £/MWh
//!    primary, the scarcity score secondary)**:
//!
//!    - the primary is the zone's **system marginal price** at its
//!      current residual: the SRMC of the marginal rung (the Stage 2
//!      SRMC recipe evaluated per zone from per-zone fuel/carbon
//!      inputs — the grid-core recipe reused unchanged; the per-zone
//!      application is new plumbing). Conventions carried from the
//!      Stage 2 pricing layer: rungs without an SRMC recipe and the
//!      whole surplus region price at the **£0 must-take floor**
//!      (conventions 1–2), so at the floor the old surplus-depth
//!      scarcity equalisation decides the flow (graceful degradation);
//!      the unserved region prices at the **fleet-SRMC ceiling**
//!      (convention 3, evaluated at run scope: the maximum SRMC over
//!      every zone's priced technologies that period, so unserved
//!      outbids every dispatched rung anywhere — a pinned, consistent
//!      level, NOT a new VoLL proxy, never monetised into adequacy);
//!    - the secondary is the scarcity score of 1a, **retained
//!      everywhere**, not only at the £0 floor: it preserves
//!      [`equalising_flow`]'s positive-`rate` invariant where SRMC is
//!      flat across a band, makes the rule degrade to **exactly the
//!      scarcity behaviour wherever per-zone SRMCs are equal** (the
//!      graceful-degradation guarantee, pinned by test), and leaves
//!      the fix to operate through the primary only where per-zone
//!      prices genuinely diverge;
//!    - **bang-bang on flat bands, deliberately**: when a non-zero
//!      SRMC gap spans a flat band, the equalisation runs to a rung
//!      edge or the cap — the intended merit-order-coupling behaviour
//!      (D11 rule 1), stated here so it is not an implementation
//!      accident. Consequence, owned: an arbitrarily small per-zone
//!      price wedge decides the whole band's direction, so the
//!      ladder's fidelity is bounded by the price data's granularity
//!      (docs/notes/d11-a2a-mismatch-characterisation.md).
//!
//! 2. **Direction.** For each border, the zone with the lower signal
//!    exports to the zone with the higher signal. Equal signals: no
//!    flow (no dead-band beyond exact equality — small differentials
//!    produce small equalising flows, and the 50 MW dead-band of the
//!    A2 direction gate lives in the *validation*, not the physics).
//!
//! 3. **Quantity: signal equalisation.** The flow grows until the two
//!    signals meet — the exporter climbs its supply curve, the importer
//!    walks down its own — or a bound binds first:
//!    - **link capacity × availability** (a deterministic derate, not a
//!      stochastic outage draw — ADR-5), applied at the sending end;
//!    - **the exporter's stack ceiling**: a zone never exports into its
//!      own unserved region, and never out of its storage (D4: storage
//!      discharges only after the local stack, for the local deficit).
//!
//!    Both curves are piecewise linear in the flow, so the crossing is
//!    solved **exactly** by walking the merged breakpoints — no
//!    iteration tolerance, bit-reproducible (ADR-5). One floating-point
//!    degeneracy is owned (R7, docs/08, fixed 2026-07-06): a
//!    boundary-exact step can leave a sub-ULP residual sliver that the
//!    accumulated flow cannot represent; the walk then snaps the probe
//!    past the breakpoint (skipping energy below the representable
//!    resolution — physically zero) instead of silently truncating the
//!    flow at the pass cap, as the pre-fix engine did.
//!
//! 4. **Losses.** The receiving end gets `sent × (1 − loss)` (the HVDC
//!    loss wedge between sending-end ENTSO-E and GB-end NESO metering,
//!    +1.04 TWh on 2024 — pack report §3). Flows are recorded at BOTH
//!    ends; GB-side validation totals use the GB end (the NESO
//!    convention).
//!
//! 5. **Borders, not links.** Links joining the same zone pair are
//!    dispatched **jointly** as one border (combined capacity, the
//!    capacity-weighted mean loss for the equalisation) and the flow is
//!    split across them pro-rata by `capacity × availability`. Without
//!    this, whichever parallel link dispatched first would close the
//!    differential and idle its twins (IFA/IFA2/ElecLink; Nemo/BritNed).
//!    Consequence, adjudicated in D5: Nemo and BritNed — separate
//!    borders in reality but one model zone pair (CONT-NW) — always
//!    carry the same differential sign and, with equal caps, equal
//!    flows; per-border BE/NL validation is therefore annual-energy
//!    only (D5 ruling a).
//!
//! 6. **Border order.** Borders are dispatched sequentially, in first
//!    appearance order of the scenario's `[[links]]` list, each seeing
//!    the zone positions left by its predecessors. A single pass, not a
//!    joint market coupling: later borders can move a hub zone (GB)
//!    after an earlier border equalised against it, leaving the earlier
//!    border marginally over-dispatched. The bias is bounded by the
//!    later borders' capacities; a stated v1 convention (scenario
//!    authors should list the largest border first, as the 5-zone
//!    scenario does).
//!
//! # What this rule deliberately is not
//!
//! - Not an LP / market coupling optimum (Stage 7 territory; ADR-10):
//!   both signal forms are myopic (per-period, no foresight) — the
//!   priced ladder is a **price model**, but a myopic one; the
//!   priced-ladder-vs-LP gap is a reported finding, exactly as the
//!   rule-vs-LP gap is (D11 rule 3). ("Not a price model" was struck
//!   from this list by D11: under `priced_ladder` the per-zone
//!   fuel/carbon asymmetry is a real term of the signal.)
//! - Not a market-coupling institutional model: no day-ahead/intraday
//!   split, no bidding, no strategic behaviour (D11 rule 5).
//! - Not a wheeling model: neighbour-of-neighbour transit is out of
//!   scope (D5; CONT-NW internal copper plate, ruling c).

use grid_core::GridError;

/// The shared multi-zone merit ladder — the Stage 1 six-rung stack,
/// FROZEN independently of the single-zone dispatch ladder
/// ([`crate::MERIT_ORDER`], extended for the Stage 7 published-pathway
/// scenarios).
///
/// Frozen because the scarcity signal below is NUMERICALLY index-based
/// (`signal = ladder index + fractional position`, prose rule 1): the
/// committed multi-zone digests (2/3/5/8-zone) pin the signal VALUES,
/// so inserting rungs — even rungs no committed scenario names — moves
/// equalising-flow arithmetic and the digests with it (measured on the
/// 2-zone pin during the Stage 7 build). Extending this ladder is
/// therefore a signal-convention change requiring a knowing re-pin of
/// the multi-zone family; the Stage 7 pathway scenarios are
/// single-zone and do not need it (multi-zone pathway variants are
/// post-beta). Until then, a multi-zone scenario naming a
/// dispatch-ladder-only technology is rejected with
/// `UnknownThermalTechnology`.
///
/// Invariant (unit-tested below): this ladder is a subset of
/// [`crate::MERIT_ORDER`] in the same relative order.
pub const FLOW_MERIT_ORDER: [&str; 6] = ["nuclear", "biomass", "hydro", "coal", "ccgt", "ocgt"];

/// Number of rungs in the shared multi-zone merit ladder
/// ([`FLOW_MERIT_ORDER`]).
pub(crate) const LADDER_LEN: usize = FLOW_MERIT_ORDER.len();

/// One zone's supply curve for one period, as the flow rule sees it:
/// merit-ladder segments `(ladder index, available ceiling GW)` in
/// ascending ladder order, zero-ceiling segments dropped.
#[derive(Debug, Clone)]
pub(crate) struct ZoneCurve {
    segments: Vec<(usize, f64)>,
    total_ceiling: f64,
}

impl ZoneCurve {
    /// Build a curve from `(ladder index, ceiling GW)` pairs in
    /// ascending ladder order. Zero (or negative-dust) ceilings are
    /// dropped; a NaN ceiling is a structured error (a corrupted
    /// availability model must not become an unordered comparison).
    pub(crate) fn new(segments: &[(usize, f64)]) -> Result<Self, GridError> {
        let mut kept = Vec::with_capacity(segments.len());
        let mut total = 0.0;
        let mut last_index = None;
        for &(index, ceiling) in segments {
            if ceiling.is_nan() {
                return Err(GridError::InvalidRunInputs {
                    reason: format!("zone supply curve: NaN ceiling at ladder index {index}"),
                });
            }
            if let Some(last) = last_index
                && index <= last
            {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "zone supply curve: ladder index {index} out of order (after {last})"
                    ),
                });
            }
            last_index = Some(index);
            if ceiling > 0.0 {
                kept.push((index, ceiling));
                total += ceiling;
            }
        }
        Ok(Self {
            segments: kept,
            total_ceiling: total,
        })
    }

    /// Total stack ceiling, GW (the export bound: a zone never exports
    /// beyond `residual + q ≤ total_ceiling`).
    pub(crate) fn total_ceiling(&self) -> f64 {
        self.total_ceiling
    }

    /// The scarcity signal at residual demand `r` GW (prose rule 1).
    pub(crate) fn signal(&self, r: f64) -> f64 {
        if r <= 0.0 {
            return r;
        }
        let mut rem = r;
        for &(index, ceiling) in &self.segments {
            if rem <= ceiling {
                return index as f64 + rem / ceiling;
            }
            rem -= ceiling;
        }
        LADDER_LEN as f64 + rem
    }

    /// `(signal, slope per GW, GW to the next breakpoint)` looking UP
    /// the curve from residual `r` (the exporter's view: residual
    /// rising). At an exact segment boundary the next segment's values
    /// are returned (the next MW comes from the next rung).
    fn up_probe(&self, r: f64) -> (f64, f64, f64) {
        if r < 0.0 {
            return (r, 1.0, -r);
        }
        let mut rem = r;
        for &(index, ceiling) in &self.segments {
            if rem < ceiling {
                return (index as f64 + rem / ceiling, 1.0 / ceiling, ceiling - rem);
            }
            rem -= ceiling;
        }
        (LADDER_LEN as f64 + rem, 1.0, f64::INFINITY)
    }

    /// `(signal, slope per GW, GW to the next breakpoint)` looking DOWN
    /// the curve from residual `r` (the importer's view: residual
    /// falling). At an exact segment boundary the segment below is
    /// returned (the marginal MW being displaced sits in it).
    fn down_probe(&self, r: f64) -> (f64, f64, f64) {
        if r <= 0.0 {
            return (r, 1.0, f64::INFINITY);
        }
        let mut rem = r;
        for &(index, ceiling) in &self.segments {
            if rem <= ceiling {
                return (index as f64 + rem / ceiling, 1.0 / ceiling, rem);
            }
            rem -= ceiling;
        }
        (LADDER_LEN as f64 + rem, 1.0, rem)
    }
}

/// Solve one border's flow (prose rules 2–4): the sending-end power
/// from the exporter (`exp`, residual `r_exp`) to the importer (`imp`,
/// residual `r_imp`), with sending-end cap `cap_gw` and loss fraction
/// `loss`. Returns 0 when the importer's signal does not exceed the
/// exporter's. Exact piecewise-linear breakpoint walk — deterministic,
/// no iteration tolerance.
pub(crate) fn equalising_flow(
    exp: &ZoneCurve,
    r_exp: f64,
    imp: &ZoneCurve,
    r_imp: f64,
    cap_gw: f64,
    loss: f64,
) -> f64 {
    let delivered = 1.0 - loss;
    // Rule 3 bounds: link cap, and the exporter's stack ceiling.
    let q_max = cap_gw.min((exp.total_ceiling() - r_exp).max(0.0));
    if q_max <= 0.0 {
        return 0.0;
    }
    let mut q = 0.0;
    // Each pass either terminates or crosses a breakpoint; the segment
    // counts are small, so the 64-pass budget is generous headroom, not
    // a tolerance — EXCEPT in the R7 degenerate class (docs/08): a
    // boundary-exact step can leave a sub-ULP residual sliver whose
    // increment is absorbed below the ULP of `q`, so the probe never
    // crosses the breakpoint and the walk spins. Passes 64.. are the
    // R7 recovery regime: identical arithmetic, but after each
    // breakpoint step the stepped side's probe position is snapped
    // monotonically past its breakpoint (`snap_exp`/`snap_imp`), so
    // every recovery pass provably crosses a breakpoint and the walk
    // completes. The recovery regime only ever runs on walks the
    // pre-R7 code silently truncated at the cap; every walk that
    // terminated within 64 passes is bit-identical to the old code.
    // The skipped slivers are below the representable resolution of
    // `q` — no energy is created or lost.
    let mut snap_exp = f64::NEG_INFINITY;
    let mut snap_imp = f64::INFINITY;
    for pass in 0..96 {
        let recovery = pass >= 64;
        let x_exp = if recovery {
            (r_exp + q).max(snap_exp)
        } else {
            r_exp + q
        };
        let x_imp = if recovery {
            (r_imp - delivered * q).min(snap_imp)
        } else {
            r_imp - delivered * q
        };
        let (s_exp, slope_exp, dist_exp) = exp.up_probe(x_exp);
        let (s_imp, slope_imp, dist_imp) = imp.down_probe(x_imp);
        let gap = s_imp - s_exp;
        if gap <= 0.0 {
            break;
        }
        // Signal closing rate per GW of flow (both slopes positive;
        // `delivered` > 0 because loss < 1 by validation).
        let rate = slope_exp + slope_imp * delivered;
        let d_cross = gap / rate;
        let d_exp = dist_exp;
        let d_imp = dist_imp / delivered;
        let d_lim = q_max - q;
        if d_cross <= d_exp && d_cross <= d_imp && d_cross <= d_lim {
            q += d_cross;
            break;
        }
        if d_lim <= d_exp && d_lim <= d_imp {
            q = q_max;
            break;
        }
        q += d_exp.min(d_imp);
        if recovery {
            // Guarantee the stepped breakpoint is crossed even when
            // the increment was absorbed below the ULP of `q`.
            if d_exp <= d_imp {
                snap_exp = snap_exp.max((x_exp + dist_exp).next_up());
            }
            if d_imp <= d_exp {
                snap_imp = snap_imp.min((x_imp - dist_imp).next_down());
            }
        }
    }
    q.clamp(0.0, q_max)
}

/// One zone's supply curve for one period under the priced ladder
/// (prose rule 1b): the scarcity-score curve plus a price per kept
/// segment (£/MWh — the rung's per-zone SRMC, or 0.0 for rungs with no
/// SRMC recipe) and the unserved-region price (the run-scope fleet-SRMC
/// ceiling, Stage 2 convention 3).
#[derive(Debug, Clone)]
pub(crate) struct PricedZoneCurve {
    curve: ZoneCurve,
    /// Price per KEPT segment, aligned with `curve.segments`.
    prices: Vec<f64>,
    /// The unserved-region price (≥ every kept segment price).
    unserved_price: f64,
}

impl PricedZoneCurve {
    /// Build a priced curve from `(ladder index, ceiling GW, price)`
    /// triples in ascending ladder order. Zero-ceiling segments are
    /// dropped (with their prices), matching [`ZoneCurve::new`].
    ///
    /// Price coherence is validated here (structured errors, never a
    /// silent mis-walk): prices must be finite, **non-decreasing along
    /// the merit ladder** (the ladder IS the SRMC-proxy order — an
    /// inversion contradicts its premise and would break the
    /// breakpoint walk's monotonicity), and the unserved price must be
    /// finite and at least the top kept segment's price (unserved
    /// outbids every dispatched rung).
    pub(crate) fn new(
        segments: &[(usize, f64, f64)],
        unserved_price: f64,
    ) -> Result<Self, GridError> {
        let curve = ZoneCurve::new(
            &segments
                .iter()
                .map(|&(index, ceiling, _)| (index, ceiling))
                .collect::<Vec<_>>(),
        )?;
        let mut prices = Vec::with_capacity(curve.segments.len());
        let mut last = 0.0f64; // the £0 floor below the first rung
        for &(index, ceiling, price) in segments {
            if !price.is_finite() {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "priced zone curve: non-finite price {price} at ladder index {index}"
                    ),
                });
            }
            // NaN ceilings were already rejected by `ZoneCurve::new`
            // above, so this matches its `ceiling > 0.0` keep-rule.
            if ceiling <= 0.0 {
                continue; // dropped segment: its price is dropped too
            }
            if price < last {
                return Err(GridError::InvalidRunInputs {
                    reason: format!(
                        "priced zone curve: price {price} at ladder index {index} is below \
                         the previous rung's {last} — segment prices must be non-decreasing \
                         along the merit ladder (the ladder is the SRMC-proxy order)"
                    ),
                });
            }
            last = price;
            prices.push(price);
        }
        if !unserved_price.is_finite() || unserved_price < last {
            return Err(GridError::InvalidRunInputs {
                reason: format!(
                    "priced zone curve: unserved price {unserved_price} must be finite and \
                     at least the top rung's price {last} (Stage 2 convention 3: unserved \
                     outbids every dispatched rung)"
                ),
            });
        }
        Ok(Self {
            curve,
            prices,
            unserved_price,
        })
    }

    /// Total stack ceiling, GW (the export bound — as [`ZoneCurve`]).
    pub(crate) fn total_ceiling(&self) -> f64 {
        self.curve.total_ceiling()
    }

    /// The lexicographic signal at residual `r` (prose rule 1b):
    /// `(primary SRMC £/MWh, secondary scarcity score)`. The secondary
    /// is bit-identical to [`ZoneCurve::signal`].
    pub(crate) fn signal(&self, r: f64) -> (f64, f64) {
        if r <= 0.0 {
            return (0.0, r); // the £0 must-take floor
        }
        let mut rem = r;
        for (&(index, ceiling), &price) in self.curve.segments.iter().zip(&self.prices) {
            if rem <= ceiling {
                return (price, index as f64 + rem / ceiling);
            }
            rem -= ceiling;
        }
        (self.unserved_price, LADDER_LEN as f64 + rem)
    }

    /// `(primary, secondary, secondary slope per GW, GW to the next
    /// breakpoint)` looking UP the curve from residual `r` — the
    /// priced analogue of [`ZoneCurve::up_probe`], with identical
    /// secondary arithmetic.
    fn up_probe(&self, r: f64) -> (f64, f64, f64, f64) {
        if r < 0.0 {
            return (0.0, r, 1.0, -r);
        }
        let mut rem = r;
        for (&(index, ceiling), &price) in self.curve.segments.iter().zip(&self.prices) {
            if rem < ceiling {
                return (
                    price,
                    index as f64 + rem / ceiling,
                    1.0 / ceiling,
                    ceiling - rem,
                );
            }
            rem -= ceiling;
        }
        (
            self.unserved_price,
            LADDER_LEN as f64 + rem,
            1.0,
            f64::INFINITY,
        )
    }

    /// The DOWN-looking priced probe — the analogue of
    /// [`ZoneCurve::down_probe`], with identical secondary arithmetic.
    fn down_probe(&self, r: f64) -> (f64, f64, f64, f64) {
        if r <= 0.0 {
            return (0.0, r, 1.0, f64::INFINITY);
        }
        let mut rem = r;
        for (&(index, ceiling), &price) in self.curve.segments.iter().zip(&self.prices) {
            if rem <= ceiling {
                return (price, index as f64 + rem / ceiling, 1.0 / ceiling, rem);
            }
            rem -= ceiling;
        }
        (self.unserved_price, LADDER_LEN as f64 + rem, 1.0, rem)
    }
}

/// Solve one border's flow under the priced ladder (prose rule 1b with
/// rules 2–4 unchanged): the lexicographic-signal analogue of
/// [`equalising_flow`].
///
/// The walk, in prose: while the importer's `(primary, secondary)`
/// exceeds the exporter's lexicographically —
///
/// - **equal primaries** (both at the £0 floor, both on equally-priced
///   rungs, or ceiling-vs-top-rung ties): the secondary equalisation is
///   the scarcity walk's exact arithmetic (`rate`, `d_cross`), so
///   equal-priced configurations reproduce [`equalising_flow`]
///   byte-for-byte — the graceful-degradation guarantee;
/// - **a primary gap**: the primary is flat within segments, so the gap
///   cannot close inside them — the flow advances to the nearest
///   breakpoint or the cap (bang-bang, prose rule 1b), and the gap is
///   re-evaluated on the new segments.
///
/// Bounds, losses and determinism are as [`equalising_flow`]: link cap
/// and exporter stack ceiling; exact breakpoint walk, no iteration
/// tolerance.
pub(crate) fn equalising_flow_priced(
    exp: &PricedZoneCurve,
    r_exp: f64,
    imp: &PricedZoneCurve,
    r_imp: f64,
    cap_gw: f64,
    loss: f64,
) -> f64 {
    let delivered = 1.0 - loss;
    let q_max = cap_gw.min((exp.total_ceiling() - r_exp).max(0.0));
    if q_max <= 0.0 {
        return 0.0;
    }
    let mut q = 0.0;
    // Each pass either terminates or crosses a breakpoint (the segment
    // counts are small); the 64-pass budget is generous headroom, not a
    // tolerance. Passes 64.. are the R7 recovery regime, exactly as in
    // [`equalising_flow`] (the same sub-ULP boundary-sliver stall
    // exists in both step branches here): every walk that terminated
    // within 64 passes is bit-identical to the pre-R7 code, and the
    // snap keeps the degradation guarantee — equal prices reproduce
    // the scarcity walk byte-for-byte, stall class included.
    let mut snap_exp = f64::NEG_INFINITY;
    let mut snap_imp = f64::INFINITY;
    for pass in 0..96 {
        let recovery = pass >= 64;
        let x_exp = if recovery {
            (r_exp + q).max(snap_exp)
        } else {
            r_exp + q
        };
        let x_imp = if recovery {
            (r_imp - delivered * q).min(snap_imp)
        } else {
            r_imp - delivered * q
        };
        let (p_exp, s_exp, slope_exp, dist_exp) = exp.up_probe(x_exp);
        let (p_imp, s_imp, slope_imp, dist_imp) = imp.down_probe(x_imp);
        // Lexicographic gap test (prose rule 2 on the pair).
        if p_imp < p_exp || (p_imp == p_exp && s_imp - s_exp <= 0.0) {
            break;
        }
        let d_lim = q_max - q;
        if p_imp > p_exp {
            // Primary gap: flat within segments — bang-bang to the
            // nearest breakpoint or the cap.
            let step = dist_exp.min(dist_imp / delivered);
            if d_lim <= step {
                q = q_max;
                break;
            }
            q += step;
        } else {
            // Equal primaries: the scarcity walk's exact arithmetic
            // (byte-identical in the equal-prices case). Both secondary
            // slopes are positive — the preserved invariant.
            let gap = s_imp - s_exp;
            let rate = slope_exp + slope_imp * delivered;
            let d_cross = gap / rate;
            let d_exp = dist_exp;
            let d_imp = dist_imp / delivered;
            if d_cross <= d_exp && d_cross <= d_imp && d_cross <= d_lim {
                q += d_cross;
                break;
            }
            if d_lim <= d_exp && d_lim <= d_imp {
                q = q_max;
                break;
            }
            q += d_exp.min(d_imp);
        }
        if recovery {
            // Guarantee the stepped breakpoint is crossed even when
            // the increment was absorbed below the ULP of `q`.
            let d_exp = dist_exp;
            let d_imp = dist_imp / delivered;
            if d_exp <= d_imp {
                snap_exp = snap_exp.max((x_exp + dist_exp).next_up());
            }
            if d_imp <= d_exp {
                snap_imp = snap_imp.min((x_imp - dist_imp).next_down());
            }
        }
    }
    q.clamp(0.0, q_max)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn curve(segments: &[(usize, f64)]) -> ZoneCurve {
        ZoneCurve::new(segments).unwrap()
    }

    #[test]
    fn flow_ladder_is_a_relative_order_subset_of_the_dispatch_ladder() {
        // The frozen six-rung flow ladder and the Stage 7 extended
        // single-zone dispatch ladder must never disagree on relative
        // order (the two-ladder invariant in the FLOW_MERIT_ORDER docs).
        let position = |tech: &str| {
            crate::MERIT_ORDER
                .iter()
                .position(|t| *t == tech)
                .unwrap_or_else(|| panic!("flow rung {tech} missing from MERIT_ORDER"))
        };
        for pair in FLOW_MERIT_ORDER.windows(2) {
            assert!(
                position(pair[0]) < position(pair[1]),
                "relative order disagreement: {} vs {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn signal_covers_surplus_stack_gaps_and_unserved() {
        // nuclear 10 GW (rung 0), ccgt 20 GW (rung 4): the ladder gap
        // (biomass/hydro/coal absent) is jumped, not interpolated.
        let c = curve(&[(0, 10.0), (4, 20.0)]);
        assert_eq!(c.signal(-3.0), -3.0);
        assert_eq!(c.signal(0.0), 0.0);
        assert!((c.signal(5.0) - 0.5).abs() < 1e-12);
        assert!((c.signal(10.0) - 1.0).abs() < 1e-12);
        // Just past nuclear: straight to the ccgt rung.
        assert!((c.signal(10.0 + 2.0) - (4.0 + 0.1)).abs() < 1e-12);
        assert!((c.signal(30.0) - 5.0).abs() < 1e-12);
        // Past the stack: 6 + unserved GW.
        assert!((c.signal(33.0) - 9.0).abs() < 1e-12);
    }

    #[test]
    fn zero_ceilings_are_dropped_and_empty_curves_are_legal() {
        let c = curve(&[(0, 0.0), (4, 0.0)]);
        assert_eq!(c.total_ceiling(), 0.0);
        // An empty zone jumps from surplus straight to unserved.
        assert_eq!(c.signal(-1.0), -1.0);
        assert!((c.signal(2.0) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn nan_and_out_of_order_ceilings_are_structured_errors() {
        assert!(matches!(
            ZoneCurve::new(&[(0, f64::NAN)]),
            Err(GridError::InvalidRunInputs { .. })
        ));
        assert!(matches!(
            ZoneCurve::new(&[(4, 1.0), (0, 1.0)]),
            Err(GridError::InvalidRunInputs { .. })
        ));
    }

    #[test]
    fn equalisation_within_one_shared_rung_is_exact() {
        // Both on ccgt 10 GW; residuals 2 and 8: q = 3 equalises at 0.5
        // utilisation (hand-derived: (2+q)/10 = (8−q)/10).
        let a = curve(&[(4, 10.0)]);
        let b = curve(&[(4, 10.0)]);
        let q = equalising_flow(&a, 2.0, &b, 8.0, 100.0, 0.0);
        assert!((q - 3.0).abs() < 1e-12, "q = {q}");
    }

    #[test]
    fn equalisation_accounts_for_losses() {
        // With loss λ the importer receives (1−λ)q:
        // (2+q)/10 = (8−0.9q)/10 → q = 6/1.9.
        let a = curve(&[(4, 10.0)]);
        let b = curve(&[(4, 10.0)]);
        let q = equalising_flow(&a, 2.0, &b, 8.0, 100.0, 0.1);
        assert!((q - 6.0 / 1.9).abs() < 1e-12, "q = {q}");
    }

    #[test]
    fn flow_stops_at_the_capacity_cap() {
        let a = curve(&[(0, 10.0)]);
        let b = curve(&[(4, 10.0)]);
        let q = equalising_flow(&a, 5.0, &b, 5.0, 1.5, 0.0);
        assert_eq!(q, 1.5);
    }

    #[test]
    fn flow_stops_at_the_exporters_stack_ceiling() {
        // Exporter has 3 GW of stack against 2 GW of load: 1 GW spare,
        // even though the importer is unserved-deep.
        let a = curve(&[(0, 3.0)]);
        let b = curve(&[]);
        let q = equalising_flow(&a, 2.0, &b, 4.0, 10.0, 0.0);
        assert!((q - 1.0).abs() < 1e-12, "q = {q}");
    }

    #[test]
    fn surplus_flows_toward_the_scarcer_zone_across_the_zero_boundary() {
        // Exporter in 4 GW surplus with no stack; importer on gas.
        // The walk crosses the exporter's r = 0 breakpoint and stops at
        // the equalisation inside the importer's rung... here the cap.
        let a = curve(&[]);
        let b = curve(&[(4, 10.0)]);
        let q = equalising_flow(&a, -4.0, &b, 5.0, 3.0, 0.0);
        assert_eq!(q, 3.0);
    }

    #[test]
    fn two_surplus_zones_split_the_difference() {
        // Negative-price region: signals −6 and −1 equalise at −3.5
        // (q = 2.5), shifting curtailment toward the deeper-surplus
        // zone's counterparty.
        let a = curve(&[]);
        let b = curve(&[]);
        let q = equalising_flow(&a, -6.0, &b, -1.0, 100.0, 0.0);
        assert!((q - 2.5).abs() < 1e-12, "q = {q}");
    }

    #[test]
    fn no_flow_without_a_positive_differential() {
        let a = curve(&[(4, 10.0)]);
        let b = curve(&[(4, 10.0)]);
        assert_eq!(equalising_flow(&a, 5.0, &b, 5.0, 10.0, 0.0), 0.0);
        assert_eq!(equalising_flow(&a, 6.0, &b, 5.0, 10.0, 0.0), 0.0);
    }

    /// R7 (docs/08): the boundary-sliver stall, the minimal
    /// reproduction of the D11 sweep review's §B.4 mechanism.
    ///
    /// The first pass steps exactly `d_imp` onto the importer's
    /// coal/ccgt edge; rounding lands the recomputed residual a
    /// sub-ULP sliver ABOVE the edge (here 8.67e-18 — the review
    /// measured ≈1e-17 on the Moyle/EWIC border), the next increment
    /// `q += d_exp.min(d_imp)` is absorbed below the ULP of `q`
    /// (4.4e-16 at q ≈ 2.02), and the 64-pass cap then binds silently:
    /// the defective walk returns q = 2.020304568527919, truncated
    /// 0.0102 GW short with the signal gap (≈2.9) still open.
    ///
    /// The correct walk displaces the importer's whole residual — the
    /// exporter's nuclear signal (≈0.1) undercuts the importer's
    /// entire stack, so the flow runs to the surplus boundary:
    /// q = r_imp / delivered = 2.0/0.985.
    #[test]
    fn boundary_sliver_stall_does_not_truncate_the_walk() {
        let a = curve(&[(0, 1000.0)]); // nuclear-marginal exporter
        let b = curve(&[(3, 0.01), (4, 5.0)]); // small coal rung under ccgt
        let q = equalising_flow(&a, 100.0, &b, 2.0, 100.0, 0.015);
        let q_true = 2.0 / 0.985;
        assert!(
            (q - q_true).abs() < 1e-12,
            "stall-truncated flow: q = {q:.17}, expected {q_true:.17}"
        );
    }

    #[test]
    fn ladder_gaps_bound_the_equalisation_at_the_jump() {
        // Exporter: nuclear 10 (rung 0). Importer: ccgt 10 (rung 4),
        // residual 0.5. The importer's signal (4.05) exceeds the
        // exporter's top-of-nuclear (1.0) everywhere, so the flow runs
        // until the importer's residual is fully displaced — the
        // crossing lands exactly at the importer's segment bottom.
        let a = curve(&[(0, 10.0)]);
        let b = curve(&[(4, 10.0)]);
        let q = equalising_flow(&a, 2.0, &b, 0.5, 10.0, 0.0);
        assert!((q - 0.5).abs() < 1e-12, "q = {q}");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod priced_tests {
    use super::*;

    fn curve(segments: &[(usize, f64)]) -> ZoneCurve {
        ZoneCurve::new(segments).unwrap()
    }

    fn priced(segments: &[(usize, f64, f64)], unserved_price: f64) -> PricedZoneCurve {
        PricedZoneCurve::new(segments, unserved_price).unwrap()
    }

    /// The lexicographic signal (prose rule 1, priced ladder): primary
    /// £0 in surplus and on unpriced rungs, the rung SRMC on priced
    /// rungs, the fleet-SRMC ceiling past the stack; secondary is
    /// exactly the scarcity score everywhere.
    #[test]
    fn priced_signal_covers_floor_bands_and_unserved() {
        // nuclear 10 GW unpriced (£0), ccgt 20 GW at £80.
        let c = priced(&[(0, 10.0, 0.0), (4, 20.0, 80.0)], 120.0);
        assert_eq!(c.signal(-3.0), (0.0, -3.0));
        assert_eq!(c.signal(0.0), (0.0, 0.0));
        let (p, s) = c.signal(5.0);
        assert_eq!(p, 0.0);
        assert!((s - 0.5).abs() < 1e-12);
        let (p, s) = c.signal(12.0);
        assert_eq!(p, 80.0);
        assert!((s - 4.1).abs() < 1e-12);
        // Past the stack: the ceiling price, secondary 6 + unserved.
        let (p, s) = c.signal(33.0);
        assert_eq!(p, 120.0);
        assert!((s - 9.0).abs() < 1e-12);
    }

    /// THE DEGRADATION GUARANTEE (D11 rule 1 / review §A): with equal
    /// per-zone prices the lexicographic order collapses to the
    /// scarcity order, and the priced walk is the scarcity walk —
    /// byte-identical flows over a dense residual grid, across curve
    /// shapes with shared rungs, ladder gaps, surpluses and unserved.
    #[test]
    fn equal_prices_walk_is_byte_identical_to_the_scarcity_walk() {
        // Shared price table per rung (equal in both zones).
        let price_of = |index: usize| match index {
            4 => 80.0,
            5 => 105.0,
            _ => 0.0,
        };
        let shapes: [&[(usize, f64)]; 4] = [
            &[(0, 10.0), (4, 20.0), (5, 2.0)],
            &[(4, 10.0)],
            &[(2, 5.0), (4, 8.0)],
            &[],
        ];
        let ceiling = 120.0;
        for exp_shape in shapes {
            for imp_shape in shapes {
                let exp = curve(exp_shape);
                let imp = curve(imp_shape);
                let priced_segments = |shape: &[(usize, f64)]| -> Vec<(usize, f64, f64)> {
                    shape.iter().map(|&(i, c)| (i, c, price_of(i))).collect()
                };
                let exp_p = priced(&priced_segments(exp_shape), ceiling);
                let imp_p = priced(&priced_segments(imp_shape), ceiling);
                let mut r = -6.0;
                while r < 40.0 {
                    let mut r2 = -6.0;
                    while r2 < 40.0 {
                        for (cap, loss) in [(100.0, 0.0), (1.5, 0.021), (0.4, 0.1)] {
                            let scarcity = equalising_flow(&exp, r, &imp, r2, cap, loss);
                            let ladder = equalising_flow_priced(&exp_p, r, &imp_p, r2, cap, loss);
                            assert!(
                                scarcity == ladder,
                                "diverged at r_exp={r}, r_imp={r2}, cap={cap}, loss={loss}: \
                                 scarcity {scarcity} vs priced {ladder} \
                                 (exp {exp_shape:?}, imp {imp_shape:?})"
                            );
                        }
                        r2 += 0.7;
                    }
                    r += 0.7;
                }
            }
        }
    }

    /// A non-zero SRMC gap across a flat band runs bang-bang to a rung
    /// edge or the cap — the intended merit-order-coupling behaviour
    /// (D11 rule 1, stated so it is not an implementation accident).
    #[test]
    fn price_gap_runs_bang_bang_to_the_rung_edge_or_cap() {
        // Both gas-marginal at 10 GW; the importer's zone prices gas
        // £1 higher. The scarcity rule would trade toward equal
        // fractional utilisation (q = 3); the priced ladder displaces
        // the importer's whole gas dispatch (q = 8, its rung edge).
        let a = priced(&[(4, 10.0, 79.0)], 120.0);
        let b = priced(&[(4, 10.0, 80.0)], 120.0);
        let q = equalising_flow_priced(&a, 2.0, &b, 8.0, 100.0, 0.0);
        assert!((q - 8.0).abs() < 1e-12, "q = {q}");
        // Same wedge, cap 1.5: the cap binds first.
        let q = equalising_flow_priced(&a, 2.0, &b, 8.0, 1.5, 0.0);
        assert!((q - 1.5).abs() < 1e-12, "q = {q}");
    }

    /// A cheap non-gas margin (primary £0) exports into a gas-marginal
    /// zone until the importer's gas is displaced, the exporter's own
    /// stack binds, or the cap binds — the FR-nuclear/hydro-vs-GB-gas
    /// lever the D11 design names.
    #[test]
    fn zero_priced_margin_exports_into_a_gas_marginal_zone() {
        // Exporter: hydro 10 GW unpriced, residual 2 (8 GW headroom).
        // Importer: ccgt 10 GW at £80, residual 8.
        let a = priced(&[(2, 10.0, 0.0)], 120.0);
        let b = priced(&[(4, 10.0, 80.0)], 120.0);
        let q = equalising_flow_priced(&a, 2.0, &b, 8.0, 100.0, 0.0);
        assert!((q - 8.0).abs() < 1e-12, "q = {q}");
        // The exporter's stack ceiling binds when smaller.
        let q = equalising_flow_priced(&a, 5.0, &b, 8.0, 100.0, 0.0);
        assert!((q - 5.0).abs() < 1e-12, "q = {q}");
    }

    /// The unserved region prices at the fleet-SRMC ceiling (Stage 2
    /// convention 3, evaluated at run scope), so an unserved importer
    /// outbids every dispatched rung anywhere; the £0 floor keeps the
    /// old surplus-depth equalisation (both primaries zero).
    #[test]
    fn unserved_outbids_every_rung_and_the_floor_keeps_surplus_equalisation() {
        // Unserved importer vs an OCGT-marginal exporter: the ceiling
        // (≥ every rung SRMC) pulls the flow until the importer's whole
        // deficit is served...
        let a = priced(&[(5, 4.0, 105.0)], 120.0);
        let b = priced(&[], 120.0);
        let q = equalising_flow_priced(&a, 1.0, &b, 2.0, 100.0, 0.0);
        assert!((q - 2.0).abs() < 1e-12, "q = {q}");
        // ...or the exporter's own stack ceiling binds (never exports
        // into its own unserved region).
        let q = equalising_flow_priced(&a, 1.0, &b, 5.0, 100.0, 0.0);
        assert!((q - 3.0).abs() < 1e-12, "q = {q}");
        // Two surplus zones: both primaries £0 → the scarcity
        // surplus-depth split, exactly as today (graceful degradation).
        let a = priced(&[], 120.0);
        let b = priced(&[], 120.0);
        let q = equalising_flow_priced(&a, -6.0, &b, -1.0, 100.0, 0.0);
        assert!((q - 2.5).abs() < 1e-12, "q = {q}");
    }

    /// R7 (docs/08): the boundary-sliver stall through the priced
    /// walk — the same degenerate class as the scarcity-walk
    /// reproduction (see `boundary_sliver_stall_does_not_truncate_the_walk`),
    /// under equal prices so the equal-primaries branch carries the
    /// walk. The degradation guarantee (equal prices ⇒ byte-identical
    /// to the scarcity walk) must hold in the stall class too.
    #[test]
    fn boundary_sliver_stall_does_not_truncate_the_priced_walk() {
        let a = priced(&[(0, 1000.0, 0.0)], 120.0);
        let b = priced(&[(3, 0.01, 0.0), (4, 5.0, 0.0)], 120.0);
        let q = equalising_flow_priced(&a, 100.0, &b, 2.0, 100.0, 0.015);
        let q_true = 2.0 / 0.985;
        assert!(
            (q - q_true).abs() < 1e-12,
            "stall-truncated priced flow: q = {q:.17}, expected {q_true:.17}"
        );
        // Byte-identity with the scarcity walk on the stall case.
        let a_s = curve(&[(0, 1000.0)]);
        let b_s = curve(&[(3, 0.01), (4, 5.0)]);
        let q_s = equalising_flow(&a_s, 100.0, &b_s, 2.0, 100.0, 0.015);
        assert!(q == q_s, "degradation guarantee broken: {q} vs {q_s}");
    }

    /// Price coherence is validated at construction: NaN prices,
    /// prices decreasing along the merit ladder, and an unserved
    /// ceiling below the top rung are structured errors (the ladder IS
    /// the SRMC-proxy order; an inversion contradicts its premise).
    #[test]
    fn incoherent_prices_are_structured_errors() {
        assert!(matches!(
            PricedZoneCurve::new(&[(4, 1.0, f64::NAN)], 120.0),
            Err(GridError::InvalidRunInputs { .. })
        ));
        assert!(matches!(
            PricedZoneCurve::new(&[(4, 1.0, 80.0), (5, 1.0, 60.0)], 120.0),
            Err(GridError::InvalidRunInputs { .. })
        ));
        assert!(matches!(
            PricedZoneCurve::new(&[(4, 1.0, 80.0)], 50.0),
            Err(GridError::InvalidRunInputs { .. })
        ));
        assert!(matches!(
            PricedZoneCurve::new(&[(4, 1.0, 80.0)], f64::NAN),
            Err(GridError::InvalidRunInputs { .. })
        ));
    }
}
