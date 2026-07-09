# 01 — Vision and Positioning

## Purpose

Third tool in a trilogy:

- **subsidyclock.co.uk** — exposes the *cost* of the UK energy settlement.
- **gridmargin.co.uk** — exposes *exposure* to unreliable sources.
- **Grid simulator** — exposes the *dynamics*: how the system works, its
  interdependencies, and its fragilities, by letting people run it themselves.

Two audiences, one engine:

1. **Teaching / public debate.** An interactive "game-like" sandbox where a
   player configures a fleet, picks a weather year, and watches the system
   succeed or fail. The deeper message: how exposed the current trajectory is.
2. **Research.** A CLI/library instrument producing publishable, reproducible
   numbers — above all, required long-duration storage under different
   scenarios, interrogating the claim that a storage-backed
   unreliable-generation grid is achievable.

## The credibility strategy

The tool's limitations become an impediment only if assumptions are
cherry-pickable. Defences, designed in from the start:

1. **Validation against reality.** Run the actual 2024 fleet against 2024
   weather and reproduce actual dispatch, gas burn, and imports within stated
   tolerances. This is the standing answer to every critic.
2. **Opponent's defaults.** Every parameter defaults to NESO FES / published
   official assumptions. Critics argue with NESO, not with us.
3. **Real weather, not synthetic.** Actual historical weather years
   (1985–2024), selectable individually. "Here is your proposed grid, in
   weather that actually happened" — including January 2010 and the
   December 2022 Dunkelflaute.
4. **Open source, deterministic, versioned.** Anyone can reproduce any
   published figure from a scenario file and an engine version.
5. **Contestable choices made pluggable.** The storage dispatch policy — the
   single most attackable modelling choice — is a trait with multiple
   implementations (rule-based, perfect-foresight LP); results are reported
   under both, and the gap between them is itself a finding.
6. **Could-have-come-out-otherwise discipline.** Kill criteria specified up
   front (see `08-risks-and-decisions.md`).

## The two-timescale insight

Adequacy and stability are different physics answering different questions:

- **Adequacy** (half-hourly, years): does the energy balance? Where the
  storage question lives. Tractable, validatable to the decimal place.
- **Stability** (ms–minutes): does the system survive losing its largest
  infeed? Swing-equation event simulation — inertia, RoCoF, frequency nadir —
  not full EMT simulation. Where "fragility" lives.

They are coupled: stability inputs (system inertia) are *derived from
adequacy outputs* (which synchronous plant is dispatched at each hour).
The signature demo: click an hour in the adequacy trace, stress-test the
grid *as dispatched at that moment*.

## The viral mechanic

Scenarios are shareable files. Anyone can publish "their" grid; anyone else
can stress it against the worst weather on record.

## What this is not

- Not a network/power-flow model (a single GB constraint-cost approximation
  covers the B6/Scottish-wind gap).
- Not an electromagnetic transient simulator.
- Not a web app in phase one. Engine first; UI is deliverable two.
