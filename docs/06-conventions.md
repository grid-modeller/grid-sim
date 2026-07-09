# 06 — Conventions

Applies to every crate and every coding session.

## Rust

- Edition: latest stable. CI: `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`, benchmark smoke test.
- **No panics in library crates.** Library APIs return
  `Result<T, GridError>`; `unwrap`/`expect` permitted only in tests and
  `grid-cli` top level. `GridError` is a structured enum (`thiserror`), with
  scenario-file errors carrying file/line context for user-facing messages.
- No `unsafe`.
- Newtype units everywhere per ADR-4; raw `f64` for physical quantities does
  not cross a public API boundary.
- No hidden state: no globals, no wall-clock reads, no environment-dependent
  behaviour in library crates (ADR-5).
- Dependencies (pinned in workspace): `serde` + `toml`, `thiserror`, `rayon`,
  `ndarray` (as needed), `argmin`, `good_lp` + HiGHS (Stage 7), `plotters`,
  `arrow`/`parquet`, `clap` (CLI). New dependencies require a one-line
  justification in the PR description.

## Testing

- Unit tests per module; property tests (`proptest`) for conservation laws
  (energy balance, SoC consistency) and schema round-tripping.
- Regression tests: pinned scenario → pinned output hash, one per published
  number (see `05-validation.md`).
- Doc-tests demonstrate unit-type safety (dimensionally wrong code in
  ` ```compile_fail` blocks).

## CLI and outputs

- Subcommands: `run`, `sweep`, `solve`, `validate`, `fetch-data`, `plot`,
  `stability` (with `event` and `inertia` sub-modes; added Stage 6,
  ratified by Richard 2026-07-03). `plot` includes `capacity-credit`
  (added Stage 5, same ratification).
- Tabular outputs: CSV (human) and Parquet (analysis) — both, always.
- Charts: PNG via `plotters`; every chart embeds engine hash + scenario hash
  in metadata and a footer caption.
- All output files carry a header/metadata block: engine git hash, scenario
  hash, data-pack checksum, schema version, timestamp (the one permitted
  wall-clock read, at the CLI layer).
- Exit codes: 0 success, 1 model infeasibility (e.g. unserved energy in a
  solve), 2 usage/scenario error.

## Performance targets (reference: single modern desktop core unless stated)

- Full 40-year half-hourly single-zone run with storage: **< 1 s**
  single-threaded (target ~100 ms; enables interactive WASM later).
- 10⁴-scenario sweep: < 1 min with rayon on 8 cores.
- Stability event simulation: < 10 ms per event.
- Benchmarks in `benches/`, run in CI as smoke tests with generous
  thresholds (2× target) to catch regressions without flakiness.

## Documentation

- Every public item documented; crate-level docs state the crate's ADR
  responsibilities.
- The rule-based dispatch policy's rules are documented **in the code** in
  full prose (it is the most contestable modelling choice — ADR-6).
- CHANGELOG per release; schema changes cross-referenced to
  `03-domain-model.md`.

## Git

- Trunk-based, short-lived branches per stage or sub-task.
- Commit messages reference stage and acceptance test where relevant
  (`stage3: SoC conservation property test`).
- Tags at each stage completion (`stage-1-validated`).
