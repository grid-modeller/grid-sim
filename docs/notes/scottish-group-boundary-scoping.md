# Scottish-group boundary — two-zone modelling scoping report

> **ADJUDICATED 2026-07-04: reviewer ADOPT-WITH-EDITS**
> (docs/notes/scottish-group-boundary-design-review.md). Three-zone
> N-Scotland / S-Scotland / E+W (B4 + B6 links) confirmed correct;
> schema v6 supports it, flow.rs untouched. SIX BINDING OBLIGATIONS
> travel into the data/scenario/engine work orders: (1) FORBID
> retuning the N/S demand split or the CF partition to the B4 DA
> series — B4 has no outturn cross-anchor, so it is an unfalsifiable
> tuning surface; pin both splits PRE-run and report the B4 miss
> (incl. the ~19% offshore-commissioning wedge) as a budget, not
> tuned out. (2) The model may quote DIRECTION + PINNED TOTALS under
> stated conventions ONLY — NO "B4 effect proper" %, NO B4-vs-B6
> decomposition (the single-pass rule across two hub-sharing borders
> compounds the equal-depth artefact that inverted the B6 magnitude;
> LP at Stage 7 is the resolver). (3) N=710k proxy adopted, but an
> explicit N<->S sensitivity on Cruachan pumped storage (Argyll
> fringe, feeds the storage headline). (4) State the border-clearing
> order (S-Scotland is the hub — single-pass hub-staleness). (5) the
> forth_tay offshore cluster STRADDLES B4 (a within-cluster split,
> not a clean re-partition), and the REPD-northing fleet split must
> be reconciled with the CF cluster split (the anti-conservative
> failure class B6 was corrected for). (6) GB-internal and
> continental multizone stay SEPARATE scenario families in v1
> (convention mixing, not zone count). B4 is quotable for direction +
> binding frequency; net magnitude carries "DA-only, no outturn
> anchor". ADR-7 gains a clean internal-zone amendment.

Data engineer, 2026-07-04. PRIORITY package (Richard's load-bearing
concern): the committed B6 two-zone model
(`scenarios/gb-2024-2zone.toml`, `docs/notes/b6-two-zone-data-report.md`)
represents only the **B6** Anglo-Scottish boundary, but the real
Scotland→England export restriction is the **group B4+B5+B6**, and in
2024 **B4 bound ~4× harder than B6**. The committed review
(`docs/notes/b6-two-zone-data-review.md` §6) ruled the current model a
**lower bound** on the Scottish constraint phenomenon with the group
(£526m) as *context only*. Richard now wants the *adequate*
(whole-Scottish-group) restriction represented. This note scopes that
fix: it delivers the geography, the per-boundary capability/flow/cost
evidence, and a modelling-convention recommendation with the physical
argument.

**No engine code, no scenario file, no schema, no `memory/`, no
`docs/04` was touched.** All data is fetched-not-committed under the
existing `data/packs/b6/` manifest (`data/packs/b6.sha256`, retrieval
2026-07-04); nothing new was fetched — every number below comes from
sources already in the pack plus two NESO ETYS pages (cited, not
fetched into a pack).

---

## 0. Licence diligence (checked first, per project law / D1 posture)

No new data source is required for this scoping. Every figure derives
from sources already licence-cleared in the B6 package (report §1) or
from the NESO ETYS publication:

| Source | Used for | Licence | Verdict |
|---|---|---|---|
| NESO Day Ahead Constraint Flows and Limits (already in pack, `neso_day_ahead_constraint_flows_limits.csv`) | B4 flow+limit series (`SSE-SP`/`SSE-SP2` rows), B6 (`SCOTEX`) | NESO Open Data Licence (per-dataset CKAN, confirmed report §1) | **Adopt** (already adopted) |
| NESO Thermal Constraint Costs (already in pack) | B4/B6/SSHARN calendar-2024 costs | NESO Open Data Licence | **Adopt** (already adopted) |
| DESNZ REPD Q1-2026 extract (already in pack, `repd_q1_2026.csv`) | site-level X/Y coordinates → north/south-of-B4 split | OGL v3.0 | **Adopt** (already adopted) |
| NESO ETYS "Scottish boundaries" page + "ETYS 2024 GB Transmission System Boundaries" dataset | B4/B5/B6 planning capabilities + limiting circuits | NESO publication; the ETYS boundary *dataset* carries the NESO Open Data Licence | **Adopt** (cited numbers; condition 4 of the review — pin ETYS to a fetchable artefact — remains open and now applies to B4/B5 too) |

Attribution to carry unchanged: "Supported by National Energy SO Open
Data" (NESO); OGL v3.0 (DESNZ); Copernicus (ERA5 traces). Nothing
proprietary; no source substituted silently.

---

## 1. Geography — where Scottish generation sits vs B4 / B5 / B6

### Method

The B6 split is exact by REPD `Country` (Scotland = north of B6;
England+Wales = south of B6). The **within-Scotland** north/south-of-B4
split uses REPD site OSGB northings (`Y-coordinate`, complete for all
574 operational Scottish sites — zero missing). B4 is the SSEN↔SPT
interface **limited by the Tealing–Westfield 275 kV circuit** (NESO
ETYS): Tealing (near Dundee) N≈738.5 k, Westfield (Fife) N≈700 k, so the
B4 line runs roughly E–W at **N≈705–715 k**. A threshold of **N=710 k**
is used; the split is **stable** across 700 k / 710 k / 720 k (onshore
N-of-B4 moves only 4,156→3,900 MW). Named-station validation confirms
the cut: Seagreen (749 k), Moray East/West, Beatrice, Peterhead,
Griffin, Stronelairg → **north of B4**; Whitelee, Clyde, Torness,
Crystal Rig, Kilgallioch, Robin Rigg → **between B4 and B6**.

Conventional plant absent from REPD is placed by known station:
**Torness** nuclear (East Lothian, N≈674 k) → between B4–B6;
**Peterhead** CCGT (Aberdeenshire, N≈846 k) → north of B4.

Caveat (stated): a single horizontal northing line approximates a
boundary that is not perfectly E–W (the SSEN↔SPT interface dips around
Argyll in the west); REPD county names are legacy (`Strathclyde` mixes
SPT-Lanarkshire/Ayrshire with a little SSEN-Argyll). Both are
second-order at the aggregate; the named-station spot-checks bound the
error.

### Geography table — capacity by boundary band (GW)

Shares are REPD within-Scotland fractions applied to the **scenario
`gb-2024-2zone.toml` SCO capacities** (so the bands sum to the SCO zone
already validated); conventional plant placed by station.

| Technology | SCO total (GW) | **North of B4** (SSEN) | **B4–B6** (SPT) | South of B6 (E+W, RGB) | % of SCO north of B4 |
|---|---|---|---|---|---|
| Onshore wind | 10.08 | **4.11** | 5.97 | 4.32 | 41% |
| Offshore wind | 3.07 | **2.89** | 0.19 | 11.63 | 94% |
| Solar PV | 0.50 | 0.35 | 0.15 | 18.20 | 69% (immaterial) |
| Nuclear | 1.19 | 0.00 | **1.19** (Torness) | 4.71 | 0% |
| CCGT | 1.18 | **1.18** (Peterhead) | 0.00 | 28.82 | 100% |
| Hydro | 1.69 | 1.64 | 0.05 | 0.21 | 97% |
| Pumped storage | 0.74 | **0.74** (Cruachan+Foyers) | 0.00 | 2.09 | 100% |
| Battery | 0.68 | 0.33 | 0.35 | 4.02 | 49% |

**The decisive number: 53% of Scottish wind capacity (7.00 of 13.15 GW)
sits NORTH of B4** — 94% of Scottish offshore and 41% of Scottish
onshore. By energy the northern share is slightly higher still
(north-of-B4 is offshore-heavy, CF≈0.35–0.45, vs southern onshore
CF≈0.29).

- North-of-B4 wind **7.00 GW vs B4 capability 4.0 GW = 1.75× oversubscribed** at full output.
- All Scottish wind **13.15 GW vs B6 capability 6.7 GW = 1.96×**.
- North-of-B4 **demand** is small (SSEN ≈ ⅓ of Scottish demand, ~3% of GB), so the northern pool is heavily export-dependent — it must push its surplus across B4 before that energy can even reach B6.

**Consequence for the current model (the core problem, confirmed):** a
Scotland/rest-of-GB split with the zone boundary at B6 and a
B6-capability link puts all 13.15 GW of Scottish wind in one
copper-plate SCO zone, so northern wind reaches the B6 link with no
B4 gate. But in reality ~7 GW of northern wind is throttled at the 4 GW
B4 wall **before** it can reach B6. The B6-placed zone boundary
therefore **structurally cannot see the binding constraint** — it
misses the boundary that cost 4× as much in 2024.

---

## 2. Per-boundary capability, 2024 flows, and 2024 costs

### Planning capability (NESO ETYS "Scottish boundaries", retrieved 2026-07-04)

| Boundary | Interface | Planning capability | Limiting circuit |
|---|---|---|---|
| **B4** (`SSE-SP`) | SSEN Transmission → SP Transmission | **4.0 GW** | Tealing–Westfield 275 kV |
| **B5** | North → South within SP Transmission | **3.9 GW** | Denny North–Lambhill 275 kV |
| **B6** (`SCOTEX`) | SP Transmission → NGET (Anglo-Scottish) | **6.7 GW** | Harker–Moffat 400 kV |

B4 and B5 are the **tightest** planning boundaries and both sit
**upstream** of (north of) B6, which is the *widest* of the three.

### Observed 2024 operational limits, flows, binding frequency (NESO DA dataset, recomputed this note)

| Boundary | 2024 DA periods | Median limit (MW) | Net DA flow S-ward (TWh) | Binding freq (flow ≥ 99% limit) |
|---|---|---|---|---|
| **B4** (`SSE-SP`+`SSE-SP2` stitched¹) | 17,280 | **1,800** | **15.78** | **35.8%** |
| **B6** (`SCOTEX`) | 17,216 | 4,100 | 22.63 | 23.6% |

¹ NESO versions B4 mid-year: `SSE-SP` (Jan 1 → Apr 20) then `SSE-SP2`
(Apr 21 → Dec 31), zero overlap, stitched to 17,280 periods — the same
versioning pattern SCOTEX/B6 carries. Semantics identical to the B6
series: this is the *forecast position after day-ahead scheduling*
(the pack's pre-constraint-action interpretation, review condition 3).

**B4 binds in 35.8% of periods vs B6's 23.6%, at a median operational
limit of 1.8 GW — less than half its 4 GW planning capability** (as
outage-driven as B6). Behind that 1.8 GW wall sits ~7 GW of northern
wind. That is the physical mechanism of the 4× cost gap.

### Calendar-2024 thermal constraint costs (NESO Thermal Constraint Costs, from `b6_report.json`, review-verified)

| Boundary group | Calendar-2024 cost | Note |
|---|---|---|
| **SSE-SP (B4)** | **£366.8 m** | intra-Scotland, north↔south |
| SSHARN (≈B7, N England) | £68.5 m | onward path *south* of B6 |
| **SCOTEX (B6)** | **£90.5 m** | Anglo-Scottish |
| ESTEX | £49.0 m | |
| SEIMP | £4.7 m | |
| SWALEX | £0.04 m | |
| **Scottish group (B4+B6+SSHARN)** | **£525.8 m** | the phenomenon the two-zone model must bound |

**B4 (£366.8 m) = 4.05× B6 (£90.5 m). The binding boundary is B4, and
it binds both harder (4× cost) and more often (35.8% vs 23.6%).** The
£367m / £90.5m / £526m 2024 figures are **confirmed** and reproduce
exactly. (The six-boundary set is only 39% of GB thermal cost; do not
total it as complete — report §7.2.)

**Note — B5 has no separate named cost/flow series.** NESO's cost
dataset names B4 (`SSE-SP`) and B6 (`SCOTEX`) but not B5; B5 congestion
is not separately published. The DA dataset likewise has no clean B5
row. B5 (3.9 GW, within SPT) sits between B4 and B6 and in practice its
gating is bracketed by B4 above and B6 below — a modelling
simplification we can state but not independently anchor.

---

## 3. Recommended modelling convention

### The physical structure: a two-stage cascade, not a single wall

"Scottish wind can't reach England" is a **series cascade**:

```
  North pool (N of B4):  ~7 GW wind + 0.74 GW PS + 1.18 GW Peterhead + ~1.6 GW hydro
        │  B4  = 4.0 GW planning / ~1.8 GW observed   (35.8% binding, £367m)
        ▼
  South-Scotland pool (B4–B6):  ~6 GW wind + 1.19 GW Torness
        │  [B5 = 3.9 GW, internal to SPT, unanchored]
        │  B6  = 6.7 GW planning / ~4.1 GW observed   (23.6% binding, £90.5m)
        ▼
  England + Wales
```

Northern wind must clear **both** B4 and B6 to reach England; southern
wind clears **only** B6. **No single link can represent a two-stage
cascade** where different generation pools see different gates:

- Link = B6 (6.7 GW), zone boundary at Scotland/England (**the current
  model**): northern wind reaches B6 freely → **understates**
  curtailment → the committed lower bound. Misses B4 entirely.
- Link = B4 (4.0 GW), all Scotland in one zone: caps *total* Scottish
  export at 4 GW, but southern wind (6 GW) does **not** cross B4 →
  **overstates** curtailment on the southern pool. Double-counts.

Neither is "adequate"; the group export capability is a two-constraint
network, not a scalar.

### RECOMMENDATION: option (b) — three zones (N-Scotland / S-Scotland / E+W)

Split into three zones joined by two links:

| Link | From → To | Capability | Validation series |
|---|---|---|---|
| **B4** | N-Scotland → S-Scotland | 4.0 GW planning / observed `SSE-SP`+`SSE-SP2` DA limit series (median 1.8 GW) | stitched B4 DA flow (net 15.78 TWh, 35.8% binding) |
| **B6** | S-Scotland → E+W | 6.7 GW planning / observed `SCOTEX` DA limit series (median 4.1 GW) | SCOTEX DA flow (net 22.63 TWh, 23.6% binding) — unchanged from current model |

Zone boundaries: **B4 line at N≈710 k** (Tealing–Westfield), **B6 at
the Scotland/England border** (by `Country`). Zonal fleet split is
already computed (§1 table). This is the **minimal faithful
representation** because it puts the binding constraint (B4) on a link
that gates the correct pool (the 7 GW northern surplus), preserves B6
as the exit gate for the combined flow, and lets both bind
independently as they do in reality. It directly captures "Scottish
wind can't reach England": the majority (northern) wind is
**double-gated**.

**Physical justification for adequacy:** in a high-wind hour the
northern pool presents ~7 GW to a 4 GW B4 → ~3 GW of northern wind is
curtailed at B4 *regardless of B6 headroom*. The current two-zone model
cannot represent that curtailment at all; the three-zone model captures
it as the binding, dominant (£367m) term. B5 (3.9 GW, internal SPT) is
folded into the S-Scotland copper-plate — a stated simplification,
bounded above by B4 and below by B6, and it has no open anchor anyway.

**Cost of option (b):** (i) a zonal fleet re-split into N/S Scotland —
**already delivered** in §1; (ii) a zonal **demand** split (N-Scotland
≈ 3% of GB, S-Scotland ≈ 7%, from the same Energy Trends / DESNZ
subnational basis as the current 10.1% — modest extra work); (iii)
**zonal CF traces** — the `sco` ERA5 cluster must be sub-split into N/S
sub-clusters (the pinned cluster members carry coordinates, so this is
a re-partition of the existing `derive_cf_gb2zone.py` cluster lists, not
new ERA5 fetching — bounded); (iv) two link capability series
(**both already in the pack** — SSE-SP/SSE-SP2 and SCOTEX); (v) an
engine/scenario package with a 3-zone scenario and B4+B6 acceptance
gates (schema v6 already supports per-direction/time-series links — no
new schema needed). This is a **data + scenario** package, not an engine
rebuild.

### Interim / cheaper alternative: option (a) — keep two zones, tighten the link

If three zones exceed budget, do **not** silently retune the current
link. Instead run **two labelled configurations that bracket the
truth**:

- **Lower bound (current, unchanged):** Scotland/England zones, link =
  B6 series. Keeps all committed flow-gate validity (review §6).
- **Tighter "B4-effective" sensitivity:** *either* move the zone
  boundary to B4 (N-Scotland vs everything-south) with link = B4 series
  — captures the tightest boundary on the biggest pool but frees
  southern-Scotland wind (so it is **not** a clean upper bound, and it
  abandons the Scotland/rGB narrative and the delivered 10.1% demand /
  DESNZ fleet splits); *or* keep the Scotland/England geometry and set
  the link to a group-effective capability **strictly as a
  bounding-study sensitivity that claims NO flow-gate validity** (the
  review §6(c) already permits exactly this variant). The physical
  argument favours option (b): option (a) in either form still
  represents only one gate of a two-gate cascade.

**Bottom line:** (b) is the adequate representation; (a) is a bracket,
not a fix. Because B4 is 4× B6 and gates 53% of Scottish wind, the
existing B6-only model omits the *dominant* term — upgrading to the
group is warranted.

---

## 4. Validation anchor for the recommended convention

The current model validates the single link against the **SCOTEX (B6)**
DA flow. A group/three-zone model needs a **group-consistent** anchor
per link:

- **B4 link:** the stitched **`SSE-SP`+`SSE-SP2`** DA limit+flow series
  (NESO DA dataset, already in pack) — net **15.78 TWh** southward,
  **35.8%** binding, median limit 1.8 GW. Gate the modelled
  pre-constraint N→S flow against it (correlation + net + binding
  frequency), exactly the three-gate structure used for B6.
- **B6 link:** unchanged — **`SCOTEX`** DA flow (net 22.63 TWh, 23.6%
  binding).
- **Net Scotland→England cross-anchor (outturn):** Energy Trends
  (18 Dec 2025, OGL) — **Scotland transferred 17 TWh to England in
  2024**. This anchors the **B6 exit only** (net Scotland export), not
  B4; B4 is an *internal* Scottish flow and has no annual-outturn
  cross-anchor (only the DA series). The **~2 TWh DA-vs-outturn wedge**
  identified for B6 (report §6; DA-forecast error + ledger + missing
  periods) **carries over unchanged** onto the B6 exit gate. B4 gates
  carry only the DA-series basis (no independent outturn), so their
  tolerance must be set from first-run wedges alone — the report's
  tolerance-deferral posture stands and now covers two links.

Named limitation carried forward: the ~3 TWh 2024-offshore-commissioning
wedge (report §3) lands **disproportionately on the B4 anchor**, because
94% of Scottish offshore is north of B4 (Moray East/West, Beatrice) — it
must be quantified against the B4 flow gate specifically, not just B6.

---

## 5. Effect on the existing B6 results — direction and rough magnitude

The current B6-only two-zone model is a **lower bound** on the Scottish
restriction (review §6(c)). The group/three-zone model **tightens** it,
because it adds the binding B4 gate that the current model cannot see:

- **Scottish curtailment (Q2/M4): UP.** The current model curtails only
  when total Scotland > B6 (6.7 GW). The three-zone model *additionally*
  curtails northern wind whenever the northern pool > B4 (4.0 GW
  planning, ~1.8 GW observed) — a constraint that binds **35.8%** of
  the time vs B6's 23.6%, on ~7 GW of northern wind. Rough scale: B4
  carried **15.78 TWh** of southward DA flow with binding 35.8% of
  periods and cost **4× B6**; the incremental curtailment the current
  model omits is of the **same order as, and plausibly larger than, the
  B6 term it already captures**. The like-for-like cost gap (£367m B4 vs
  £90.5m B6) is the sharpest available scale for the direction and size.
- **Storage sensitivity: UP.** The committed finding is that
  copper-plate **understates** the storage requirement by **+38% to
  +49%** at 2024 B6 capability (memory POST-STAGE-5 correction;
  `b6-two-zone-engine-review.md` §1d), because inter-drought **recharge**
  is boundary-limited. Adding the tighter, more-frequently-binding B4
  gate upstream of B6 **increases** that sensitivity further: northern
  wind — which is where the recharge energy predominantly sits (94% of
  offshore) — is throttled *earlier and harder*. Direction is
  unambiguous (up); magnitude requires the three-zone run to quantify.
- **Capture/revenue (Q10): DOWN further for Scottish wind**, same
  mechanism (more curtailed/price-separated energy north of B4).
- **Q4/M3 GB adequacy (drought-depth) claims: unaffected in direction.**
  Drought depth is GB-wide (worst year 2010 in both zones); the added
  constraint bites on *recharge between droughts*, consistent with the
  existing correction — it deepens the storage-requirement understatement,
  it does not change the drought-adequacy story.

Net: the group model does not overturn any committed direction; it
**widens the gap between the copper-plate/B6-only numbers and reality in
the already-stated direction**, and it moves the dominant term (B4) from
"structurally invisible" to "modelled."

---

## 6. What could not be sourced openly

- **B5 (3.9 GW) flow, limit, and cost.** No separate NESO named series
  (cost dataset has B4/B6 but not B5; DA dataset has no clean B5 row).
  B5 is folded into the S-Scotland copper-plate in option (b) — a
  stated simplification, bracketed by B4 above / B6 below, unanchored.
- **ETYS B4/B5/B6 capabilities pinned to a fetchable artefact.** Taken
  from the JS-rendered NESO "Scottish boundaries" page (review
  condition 4, already open for B6, now also for B4/B5). The observed DA
  envelopes are consistent, but the scenario package must pin these to
  the ETYS appendix workbook/PDF before hard-coding — condition 4
  extended.
- **Per-boundary curtailed-wind volume (TWh).** Only cost is published
  per boundary; volume is GB-wide only (report §7). Per-unit curtailment
  is computable from Elexon BOALF (BSC open, heavy assembly) — the v2
  path if Q2/Q10 needs curtailed TWh per boundary rather than cost.
- **Sub-Scotland (N/S of B4) demand and CF at metered quality.** The N/S
  demand split is inferable from DESNZ subnational + Energy Trends but
  not metered at half-hourly per-band; Elexon P114 GSP-group
  (`_P` = North Scotland/SSE ≈ north-of-B4 proxy, `_N` = South
  Scotland/SP) is the open half-hourly upgrade (BSC Open Data, free
  Elexon account, review condition 7 verification outstanding) and would
  give the true N/S demand *shape* — recommended if option (b) proceeds.
- **Precise B4 boundary line.** Approximated by a horizontal N=710 k cut
  (Tealing–Westfield); a substation-level SSEN/SPT GSP mapping (from the
  ETYS boundary dataset's circuit list) would tighten the ~few-hundred-MW
  ambiguity around the Argyll/Stirling fringe — a refinement, not a
  blocker (the split is stable ±10 k and named-station-validated).

---

## 7. Reproduction

```
V=~/.local/share/grid-sim/era5-venv/bin/python
# Geography (REPD north/south-of-B4 split, N=710k threshold):
$V - <<'PY'  # coordinates already in data/packs/b6/raw/repd_q1_2026.csv
# see §1: filter Operational & op_date<=2024-12-31 & Country==Scotland,
# split cap by Y-coordinate >= 710000
PY
# B4 flow series (stitch SSE-SP + SSE-SP2 from the DA dataset already in pack):
#   data/packs/b6/raw/neso_day_ahead_constraint_flows_limits.csv
# Costs / capabilities: data/packs/b6/processed/b6_report.json (B4=SSE-SP,
#   B6=SCOTEX) ; ETYS "Scottish boundaries" page (B4 4.0 / B5 3.9 / B6 6.7 GW).
```

No new fetch; the B4 series is a re-slice of the DA file already pinned
by `data/packs/b6.sha256` (retrieval 2026-07-04). If option (b)
proceeds, the fetch-b6 `build.py` should be extended to emit a stitched
`b4_da_flows_limits.{parquet,csv}` (SSE-SP + SSE-SP2, same clock-change
and sentinel handling as the B6 builder) — the only machinery change
needed on the data side.
