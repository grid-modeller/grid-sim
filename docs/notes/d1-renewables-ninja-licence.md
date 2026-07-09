# D1 — renewables.ninja licence: fetch-and-build vs. direct ERA5 derivation

Research note resolving open decision D1 (`docs/08-risks-and-decisions.md`)
and risk R3. All sources accessed **2026-07-02**. Quotes are verbatim from
the cited pages.

---

## 1. Findings per source

### 1.1 renewables.ninja — data licence

Source: <https://www.renewables.ninja/about> (accessed 2026-07-02).

Licence: **Creative Commons Attribution-NonCommercial 4.0 International
(CC BY-NC 4.0)**. Operative language from the About page:

> "you are free to copy, redistribute and adapt them for non-commercial
> purposes"

> "If you wish to use the data for commercial purposes, please contact us"

Attribution: for academic/professional use, "cite the papers describing our
methods [1, 2] and, if possible, link to www.renewables.ninja"; otherwise
"cite the papers [1, 2] or link to www.renewables.ninja, as is more
appropriate". Data is "made available as-is and without warranty".

What the NC clause covers — from the CC BY-NC 4.0 legal code, §1(i)
(<https://creativecommons.org/licenses/by-nc/4.0/legalcode>, accessed
2026-07-02):

> "NonCommercial means not primarily intended for or directed towards
> commercial advantage or monetary compensation."

The NC restriction attaches to **any use** of the licensed material, not
only redistribution. grid-sim exists explicitly to produce results for a
commercial book (*The Energy Trap*). Deriving the book's published numbers
from renewables.ninja data is plausibly a commercial use requiring their
explicit permission ("please contact us") — regardless of who runs the
fetch. This is the central problem, and it is not cured by the per-user
fetch-and-build architecture.

### 1.2 renewables.ninja — API terms and 40-year feasibility

Sources: <https://www.renewables.ninja/documentation>,
<https://www.renewables.ninja/documentation/api>,
<https://www.renewables.ninja/downloads> (accessed 2026-07-02).

- Registration: anonymous access allowed; registered accounts get a token
  (`Authorization: Token <...>`).
- Rate limits (documentation page): anonymous users "limited to a maximum
  of 5 requests per day"; registered users 50/hour; burst limit "currently
  set to 6/minute". "If you need a higher limit, e.g. for automated access
  via the API, please contact us."
- Range limits: "Currently, the maximum amount of data available per
  request is 1 year." "Anonymous users have access to a single year.
  Registered users have access to the full range of data available."
- Bulk: 40 years x 3 technologies (onshore wind, offshore wind, PV) at
  country level ≈ 120 requests — mechanically feasible for a registered
  user in ~3 hours within the 50/hour limit, but `grid-cli fetch-data`
  orchestrating this for every user is exactly the "automated access via
  the API" case for which they ask to be contacted first.
- Pre-computed downloads: country-level hourly capacity factors (EU-28 +
  NO/CH), **MERRA-2 / CM-SAF SARAH based, v1.3 extends only through 2019**
  — they do not cover 1985–2024 and are also CC BY-NC 4.0.

**Factual correction to note against `docs/05-validation.md`:** the data
table there says "ERA5 via renewables.ninja". renewables.ninja simulations
are based on **MERRA-2** (wind, PV) and **CM-SAF SARAH** (PV, Europe), not
ERA5. If ninja were adopted, the weather basis would differ from the ERA5
temperature traces used elsewhere in the pack. (Not editing that doc here;
flagging for the maintainer.)

### 1.3 Per-user fetch and hosted-pack redistribution under CC BY-NC

- **Per-user fetch (`grid-cli fetch-data` calling the ninja API):** each
  user accepts CC BY-NC for the data they download. A user's own
  non-commercial experimentation is fine. But the project's own runs that
  feed the book are commercial use (see 1.1), and the tool systematically
  directing users at the API is automated access they ask to be consulted
  about. Compatible only for genuinely non-commercial use, and only with
  their tolerance of orchestrated access.
- **Hosting a pre-built pack (phase 2):** CC BY-NC permits redistribution
  of copies and adaptations "for non-commercial purposes" with attribution.
  A pack hosted on a teaching site that sits alongside and promotes a
  commercial book is at best a grey area under §1(i)'s "primarily intended
  for or directed towards commercial advantage" test. Safe only with
  written permission from the ninja team.

### 1.4 Copernicus / ERA5 licence

Sources:
- ECMWF announcement, "CC-BY licence to replace Licence to use Copernicus
  Products on 02 July 2025":
  <https://forum.ecmwf.int/t/cc-by-licence-to-replace-licence-to-use-copernicus-products-on-02-july-2025/13464>
  (accessed 2026-07-02).
- Previous licence text (rev. 12):
  <https://cds.climate.copernicus.eu/licences/licence-to-use-copernicus-products>
  (accessed 2026-07-02). (Note: the old static PDF URL
  `cds.climate.copernicus.eu/api/v2/terms/...` now returns 404; the licence
  text is at the URL above.)

**Since 2 July 2025, data in the Climate Data Store (including ERA5) is
licensed CC-BY 4.0**:

> "the License to use Copernicus Products in the Climate Data Store (CDS),
> Atmosphere Data Store (ADS) and the CEMS Early Warning Data Store (EWDS)
> will be replaced with the Creative Commons Attribution License (CC-BY)"

> "Usage of any Copernicus Products through the CDS, ADS and EWDS after the
> above date will be considered as your acceptance of this license."

CC-BY 4.0 permits reproduction, redistribution, adaptation, and
**commercial use**, with attribution. Even the superseded Licence to Use
Copernicus Products (rev. 12) already granted a licence that is "free of
charge, worldwide, non-exclusive, royalty free and perpetual" covering
"reproduction; distribution; communication to the public; adaptation,
modification and combination with other data and information" — i.e.
redistribution of derived capacity-factor packs was permitted under the old
terms too.

Attribution required (rev. 12 clauses 5.1.1/5.1.2, carried into CDS dataset
citation guidance):

> "Generated using Copernicus Climate Change Service information [Year]"

and for modified/derived products:

> "Contains modified Copernicus Climate Change Service information [Year]"

plus a statement that "neither the European Commission nor ECMWF is
responsible for any use" of the information. Under CC-BY, per the
announcement, "please refer to the 'References' section on each dataset
entry" (dataset DOI citation, e.g. Hersbach et al. for ERA5).

Access: free CDS/ECMWF account, `cdsapi` client, licence accepted once per
account. Fully compatible with per-user `grid-cli fetch-data` and with
hosting a pre-built derived pack, commercially, with attribution.

Determinism note: ERA5 final data is stable and versioned (ERA5T
preliminary data only affects the most recent ~3 months, irrelevant for a
1985–2024 pack). Pinned request parameters + recorded checksums give exact
reproducibility. renewables.ninja, by contrast, upgrades its models in
place (see their news posts), so a re-fetch is not guaranteed to reproduce
a checksum.

### 1.5 Effort delta of direct ERA5 derivation

The methods are published and tooled. Wind: Staffell & Pfenninger 2016
("Using bias-corrected reanalysis to simulate current and future wind power
output", *Energy* 114) — extrapolate reanalysis wind speeds to hub height,
apply smoothed turbine power curves, aggregate over fleet coordinates, and
bias-correct against national statistics; ninja's own wind code (VWF, R) is
open at <https://github.com/renewables-ninja/vwf>. Solar: Pfenninger &
Staffell 2016 (*Energy* 114); their GSEE Python package is open at
<https://github.com/renewables-ninja/gsee>. The **atlite** package
(<https://github.com/PyPSA/atlite>, MIT licence, verified via GitHub API
2026-07-02) implements the whole pipeline against ERA5 directly: download a
GB cutout via `cdsapi`, convert with turbine power curves / PV panel models,
aggregate over a capacity layout — used in production by PyPSA-Eur. The
genuinely new work is not the physics but the **calibration**: raw
reanalysis-derived capacity factors carry known biases, and matching GB
onshore/offshore/solar fleet CFs requires bias correction against
Elexon/NESO actuals for overlapping years — which this project must do
anyway for Stage 1 validation, using data it already fetches. Realistic
estimate: days-to-a-couple-of-weeks of Python pipeline + validation work on
top of the planned data-assembly effort, versus hours for a ninja fetch.

---

## 2. Decision matrix

Scores 1 (poor) – 5 (good).

| Criterion | (a) fetch-and-build via renewables.ninja | (b) direct ERA5 derivation | (c) hybrid: ninja for validation, ERA5 for shipped tool |
|---|---|---|---|
| Licence risk, commercial-book context | **1** — CC BY-NC; the book is commercial use; needs written permission | **5** — CC-BY 4.0; commercial use explicit; attribution only | **4** — shipped/published numbers all ERA5-derived; ninja used only as an internal cross-check (research use, nothing redistributed or published from it) |
| Hosted-pack feasibility (phase 2) | **1** — redistribution only "for non-commercial purposes"; site adjacent to commercial book is a grey area at best | **5** — redistribution of derived pack permitted, incl. commercially, with attribution | **5** — hosted pack is ERA5-derived only |
| Implementation effort | **4** — API client + ~120 requests; but 1-year-per-request chunking, rate limits, and "contact us" for automated access | **2** — cutout download + atlite/VWF-style conversion + GB bias correction (days–weeks) | **2** — same as (b) plus a cheap ninja comparison script |
| Reproducibility / determinism (checksummable inputs) | **2** — model upgraded in place; re-fetch may not reproduce checksums; MERRA-2 basis diverges from ERA5 temperature traces | **5** — pinned ERA5 requests, stable final data, single weather basis for CFs and temperature | **5** — shipped inputs as (b) |
| **Total** | **8** | **17** | **16** |

---

## 3. Recommendation for D1

**Adopt direct ERA5 derivation for everything the tool ships and every
number the book publishes** — i.e. resolve D1 as option (b), with the
option-(c) refinement that renewables.ninja may be used, unbundled and
unpublished, as an independent cross-check of our derived GB capacity
factors during development.

Rationale:

1. **The NC clause is disqualifying, not merely inconvenient.** CC BY-NC
   restricts use, not just redistribution, and this project's declared
   purpose is a commercial book. Per-user fetch does not launder that.
   Relying on ninja would make every published number contingent on an
   informal permission ("please contact us") — the opposite of the
   project's reproducibility-and-credibility posture (R4).
2. **ERA5 is now CC-BY 4.0** (since 2025-07-02), so direct derivation gives
   unencumbered commercial use *and* the phase-two hosted pack, killing
   risk R3 outright. Even the prior Copernicus licence permitted this.
3. **Determinism.** Pinned ERA5 dataset versions checksum cleanly; ninja's
   in-place model upgrades and 1-year-chunk API do not, and its MERRA-2 /
   CM-SAF weather basis would diverge from the ERA5 temperature traces in
   the same pack.
4. **The extra effort is largely work the project owes anyway.** Bias
   correction against Elexon/NESO actuals is the same exercise as the Stage
   1 validation-pack assembly, and open MIT-licensed tooling (atlite) plus
   ninja's own open-source method code (GSEE, VWF) covers the conversion
   step.

Consequential actions (for the maintainer; not made here):

- Update the D1 row in `docs/08-risks-and-decisions.md` to "Resolved:
  direct ERA5 derivation; ninja for internal cross-validation only, never
  redistributed."
- Correct the `05-validation.md` data table: renewables.ninja is
  MERRA-2/CM-SAF based, not ERA5; the weather-CF row should read "ERA5
  (CC-BY 4.0), direct derivation".
- Data-pack outputs must carry the attribution notice "Contains modified
  Copernicus Climate Change Service information [Year]" plus the
  no-responsibility disclaimer, and cite the ERA5 dataset DOI.
