# GB Grid Simulator — Documentation Suite

A Rust-based simulator of the GB electricity system with two coupled lenses:
**energy adequacy** (chronological dispatch, hours–decades) and **system
stability** (frequency dynamics, milliseconds–minutes). Built as a research
instrument for *The Energy Trap*, a public teaching tool alongside
subsidyclock.co.uk and gridmargin.co.uk, and a reproducible engine for
published claims about storage requirements, system costs, and grid fragility.

## Document map

| Doc | Purpose | Stability |
|---|---|---|
| `01-vision.md` | Why the tool exists, audiences, credibility strategy | Stable |
| `02-architecture.md` | Architecture Decision Record — crate layout, core invariants | **Immutable** — every coding session cites this |
| `03-domain-model.md` | Scenario schema, types, weather/demand data model | Versioned — changes require schema version bump |
| `04-implementation-plan.md` | Stages 0–7, each with scope, non-goals, acceptance tests, demo artefact | Working document |
| `05-validation.md` | Data sources, licensing, the 2024 validation pack, reproducibility rules | Stable |
| `06-conventions.md` | Coding standards, error handling, outputs, benchmarks | Stable |
| `07-research-syllabus.md` | Modules 1–7 and research questions Q1–Q10 the tool must answer, with spec dependencies | Stable |
| `08-risks-and-decisions.md` | Open decisions, kill criteria, scope boundaries | Working document |

## How to seed a coding session

1. Always include `02-architecture.md` and `06-conventions.md`.
2. Include `03-domain-model.md` for any work touching the scenario schema or `grid-core`.
3. Include the current stage's section from `04-implementation-plan.md` as the work order.
4. Include `05-validation.md` when a stage checkpoint involves validation against real data.
5. `01`, `07`, `08` are context documents — include when design judgement is needed, omit for mechanical work.

## Ground rules (non-negotiable, repeated in the ADR)

- Every published number is regenerable from `scenario file + engine git hash`.
- No stage is complete until its acceptance test passes.
- Real weather and real fleet data; defaults traceable to NESO FES assumptions.
- Engine (library + CLI) is the product for the book. Web UI is a separate, later deliverable.
