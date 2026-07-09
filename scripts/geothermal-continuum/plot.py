#!/usr/bin/env python3
"""Geothermal depth continuum — SCOP, peak, and seasonal storage vs depth.

Renders figures/geothermal-scop-continuum.png for the reply to an industry correspondent and book ch. 27. Every number is transcribed from the
PINNED acceptance tests — no re-run, no re-derivation:

  - peak / storage vs depth:  grid-adequacy/tests/geothermal_depth.rs
        continuum_curve_peak_and_storage_vs_depth_pinned
  - SCOP vs depth:            grid-adequacy/tests/geothermal_depth.rs
        scop_vs_depth_pinned  (D16 SCOP read-out, commit bf3256e)
  - RHPP field medians:       data/reference/heating-cop.toml [rhpp]
        ASHP SPFH2 2.65, GSHP SPFH2 2.81

Scenario: Royal Society 37-year wind+solar fleet, all building-heat electrified
on ONE technology, ground-source swept across resource depth on the industry correspondent's
conservative 25 C/km gradient (BGS band 26-35 stated). Physical only: no cost,
NO cooling credit (a separate, larger, still-unmodelled benefit).
"""

import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

# ---- palette (dataviz reference instance, validated blue/orange pair) --------
GROUND = "#2a78d6"   # ground-source continuum — the hero series
AIR = "#eb6834"      # air-source reference (off the depth continuum)
INK = "#0b0b0b"      # text-primary
INK2 = "#52514e"     # text-secondary
GRID = "#e4e3df"     # recessive grid
FIELD = "#0b0b0b"    # RHPP field-median markers (hollow, ink outline)
SURFACE = "#fcfcfb"

# ---- pinned data -------------------------------------------------------------
# GSHP resource-depth grid (metres) and the three pinned series.
depth = [1, 15, 100, 250, 500, 750, 1000, 1250, 1500, 1750, 2000, 3000]
scop = [2.810, 2.970, 3.151, 3.484, 4.024, 4.359,
        5.018, 7.824, 10.667, 10.845, 15.000, 15.000]
# deltas vs the no-heat baseline (peak 92.239 GW; storage 23,872 GWh):
dpeak = [22.158, 19.617, 18.466, 16.660, 14.149, 12.135,
         11.857, 11.751, 4.210, 4.210, 3.612, 3.612]
dstore = [17376, 14432, 12800, 10272, 7600, 6784,
          6656, 5008, 2400, 2400, 2000, 2000]

# air-source reference (no ground loop, no depth) — pinned:
ashp_scop, ashp_dpeak, ashp_dstore = 2.651, 23.450, 19616

# RHPP field medians (independent — the model was NOT fitted to them):
field_ashp, field_gshp = 2.65, 2.81

# source mean temperature on the 25 C/km gradient, T = 10.5 + 0.025*(z-1) [C]
def tsrc(z):
    return 10.5 + 0.025 * (z - 1)


# ---- figure ------------------------------------------------------------------
plt.rcParams.update({
    "font.family": "DejaVu Sans", "font.size": 10,
    "axes.edgecolor": INK2, "axes.linewidth": 0.8,
    "text.color": INK, "axes.labelcolor": INK, "figure.facecolor": SURFACE,
    "axes.facecolor": SURFACE, "savefig.facecolor": SURFACE,
})

fig, (axa, axb, axc) = plt.subplots(
    3, 1, figsize=(8.2, 9.6), sharex=True, constrained_layout=True)
# Titles sit above the canvas (y > 1); savefig bbox_inches="tight" captures
# them, so they never collide with the panel-A source-temperature axis.
fig.text(0.02, 1.075,
         "The air / ground / district trichotomy is one continuum in depth",
         fontsize=14, fontweight="bold", ha="left", va="top", color=INK)
fig.text(0.02, 1.030,
         "Royal Society 37-year wind+solar Britain, all building heat electrified on one technology. "
         "Ground-source swept\nacross resource depth on a conservative 25 °C/km gradient. "
         "SCOP rises and system cost falls as one curve.",
         fontsize=9.2, ha="left", va="top", color=INK2)

for ax in (axa, axb, axc):
    ax.set_xscale("log")
    ax.grid(True, which="major", color=GRID, linewidth=0.8, zorder=0)
    ax.grid(True, which="minor", color=GRID, linewidth=0.4, zorder=0)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)

marker_kw = dict(marker="o", markersize=6, markerfacecolor=GROUND,
                 markeredgecolor=SURFACE, markeredgewidth=1.0)

# --- Panel A: SCOP (the correspondent's lens) --------------------------------------------
axa.plot(depth, scop, color=GROUND, linewidth=2, zorder=3, **marker_kw)
axa.axhline(ashp_scop, color=AIR, linewidth=2, linestyle=(0, (5, 2)), zorder=2)
axa.text(3400, 2.32, "air-source (no ground loop)  SCOP 2.65",
         color=AIR, fontsize=9, ha="right", va="center", fontweight="bold")
# RHPP field medians — hollow markers proving the model lands on field data
axa.plot([1], [field_gshp], marker="o", markersize=10, markerfacecolor="none",
         markeredgecolor=FIELD, markeredgewidth=1.4, zorder=4)
axa.plot([1.0], [field_ashp], marker="o", markersize=10, markerfacecolor="none",
         markeredgecolor=FIELD, markeredgewidth=1.4, zorder=4)
axa.annotate("RHPP field medians\n(model not fitted to them):\nair 2.65 · shallow ground 2.81",
             xy=(1, field_gshp), xytext=(1.9, 6.1), fontsize=8.4, color=INK2,
             ha="left", va="center",
             arrowprops=dict(arrowstyle="-", color=INK2, linewidth=0.7))
axa.annotate("direct use — no heat pump\n(source ≥ sink)  SCOP 15",
             xy=(2000, 15), xytext=(430, 13.4), fontsize=8.6, color=GROUND,
             ha="left", va="center", fontweight="bold",
             arrowprops=dict(arrowstyle="-", color=GROUND, linewidth=0.7))
axa.text(1.15, 2.81 + 0.15, "shallow loop 2.81", color=GROUND, fontsize=8.4,
         ha="left", va="bottom")
axa.set_ylabel("Seasonal COP\n(heat delivered ÷ electricity)", fontsize=9.5)
axa.set_ylim(2, 16)

# secondary top axis: source temperature at representative depths
axtop = axa.secondary_xaxis("top")
tick_d = [1, 100, 500, 1000, 2000, 3000]
axtop.set_xticks(tick_d)
axtop.xaxis.set_major_formatter(FuncFormatter(lambda z, _: f"{tsrc(z):.0f}"))
axtop.set_xlabel("mean source temperature on the gradient (°C)", fontsize=9, color=INK2)
axtop.tick_params(colors=INK2, labelsize=8.5)

# --- Panel B: peak electricity (Richard's lens 1) ----------------------------
axb.plot(depth, dpeak, color=GROUND, linewidth=2, zorder=3, **marker_kw)
axb.axhline(ashp_dpeak, color=AIR, linewidth=2, linestyle=(0, (5, 2)), zorder=2)
axb.text(2100, ashp_dpeak - 0.2, "air-source  +23.5 GW", color=AIR, fontsize=9,
         ha="right", va="top", fontweight="bold")
axb.annotate("shallow ground\n+22.2 GW", xy=(1, 22.158), xytext=(2.2, 17.5),
             fontsize=8.4, color=GROUND, ha="left", va="center",
             arrowprops=dict(arrowstyle="-", color=GROUND, linewidth=0.7))
axb.annotate("direct use  +3.6 GW", xy=(2000, 3.612), xytext=(430, 6.6),
             fontsize=8.6, color=GROUND, ha="left", va="center", fontweight="bold",
             arrowprops=dict(arrowstyle="-", color=GROUND, linewidth=0.7))
axb.set_ylabel("Added peak electricity\ndemand (GW)", fontsize=9.5)
axb.set_ylim(0, 26)

# --- Panel C: seasonal storage (Richard's lens 2) ----------------------------
axc.plot(depth, dstore, color=GROUND, linewidth=2, zorder=3, **marker_kw)
axc.axhline(ashp_dstore, color=AIR, linewidth=2, linestyle=(0, (5, 2)), zorder=2)
axc.text(2100, ashp_dstore - 400, "air-source  +19,600 GWh", color=AIR,
         fontsize=9, ha="right", va="top", fontweight="bold")
axc.annotate("shallow ground\n+17,400 GWh", xy=(1, 17376), xytext=(2.2, 13200),
             fontsize=8.4, color=GROUND, ha="left", va="center",
             arrowprops=dict(arrowstyle="-", color=GROUND, linewidth=0.7))
axc.annotate("direct use  +2,000 GWh", xy=(2000, 2000), xytext=(430, 5200),
             fontsize=8.6, color=GROUND, ha="left", va="center", fontweight="bold",
             arrowprops=dict(arrowstyle="-", color=GROUND, linewidth=0.7))
axc.set_ylabel("Added seasonal-class\nstorage (GWh)", fontsize=9.5)
axc.set_ylim(0, 21000)
axc.yaxis.set_major_formatter(FuncFormatter(lambda v, _: f"{v:,.0f}"))

# shared x
axc.set_xlim(0.8, 3600)
axc.set_xticks([1, 10, 100, 1000])
axc.xaxis.set_major_formatter(FuncFormatter(lambda z, _: f"{z:g}"))
axc.set_xlabel("geothermal resource depth (metres, log scale)", fontsize=10)

fig.text(0.02, -0.005,
         "Physical only — no cost, and no cooling credit (a separate, larger benefit not yet modelled). "
         "Gradient centre 25 °C/km (BGS band 26–35). "
         "Numbers pinned: grid-adequacy/tests/geothermal_depth.rs.",
         fontsize=7.8, ha="left", color=INK2)

out = "figures/geothermal-scop-continuum.png"
fig.savefig(out, dpi=200, bbox_inches="tight")
print(f"wrote {out}")
