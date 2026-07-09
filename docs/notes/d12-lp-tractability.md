# D12 LP tractability — the horizon decision (rule 3), pinned with evidence

**Status:** MEASURED 2026-07-04 — the implementer's tractability finding
for the D12 perfect-foresight LP (package 2b PHASE 1), per
`docs/notes/d12-perfect-foresight-lp.md` **rule 3** ("the tractability
decision pinned by the implementer with evidence"). This note records the
benchmark, the decision, and the recommended rolling-horizon design for
supervisor review.

## The open question

Can HiGHS (`good_lp` + HiGHS, ADR-10) solve a **full-horizon 40-year ×
multi-zone** perfect-foresight LP in acceptable time and memory, so the LP
can drop into the bisection sizing loop as the feasibility oracle
(`solve.rs`)? Or is a **rolling-horizon window** needed?

The bisection calls the oracle roughly `log2(size range) ≈ 15–30` times.
So for the oracle to be usable, **one full-horizon LP solve must be on the
order of a few minutes at most**. That is the budget everything below is
judged against.

## The benchmark

`grid-adequacy/tests/tractability_bench.rs` (`#[ignore]`d; run explicitly
under `/usr/bin/time -l`). A representative **3-zone LINE topology**
(A—B—C — the wheeling case rule 3 says matters), each zone a wind
renewable + a hydrogen store (η = 0.40), a thermal backstop in the
southern zone, two finite links (2% loss). Traces are synthetic but
chronologically varying (seasonal × diurnal, zones phase-shifted) so the
LP is a genuine multi-period recharge problem, not a degenerate one. LP
size per period ≈ 20 variables (1 thermal + 3 zones × {charge, discharge,
soc} + 2 links × {fwd, rev} + 3 zones × {unserved, curtailment}) and 6
constraints (3 zone-balance + 3 SoC dynamics). Measured on this machine
(Apple Silicon, release build, single-threaded HiGHS). The binding cost is
problem **size** (variable/constraint count → memory and simplex work) and
the 10-year abort is an internal allocation failure — both are independent
of how near the objective sits to the feasibility boundary, so the
oracle's near-zero-unserved regime scales the same way. The full-GB fleet
(more thermals/stores/links per zone) is larger per period than this
3-zone probe, so these numbers are a **lower bound** on the real cost.

### Default solver (HiGHS `choose` → simplex)

| Horizon | Periods | LP variables | LP solve time | Peak RSS | Result |
|--------:|--------:|-------------:|--------------:|---------:|--------|
| 1 year  |  17,520 | ~0.35 M      | **59.3 s**    | **1.11 GB** | solved |
| 5 years |  87,600 | ~1.75 M      | **570.3 s** (9.5 min) | **3.25 GB** | solved |
| 10 years| 175,200 | ~3.5 M       | **ABORTED @ 1433 s** (23.9 min) | 5.20 GB | **HiGHS crash** |
| 40 years| 701,280 | ~14 M        | not attempted | — | — |

The **10-year solve did not merely slow down — it aborted the process**
with an uncaught C++ exception from inside HiGHS
(`libc++abi: terminating due to uncaught exception of type
std::length_error: vector`), i.e. `std::terminate` → `SIGABRT`, after ~24
minutes and 5.2 GB. This is uncatchable from Rust (it is not a
`Result::Err`, it aborts the whole process). So the default solver fails
**hard** at 10 years, long before 40.

Scaling of the two clean points: time grows as ≈ `O(n^1.41)`
(`ln(570.3/59.3)/ln 5 = 1.41`); memory grows ≈ linearly (~30.6 KB/period
above a ~0.57 GB fixed floor). Extrapolated to the full 40-year horizon
(if it did not crash first): **~3 hours per solve, ~22 GB peak RSS**.

### Interior-point solver (HiGHS `ipm`)

Measured via a temporary env-gated switch in `run_multi_lp` (reverted
before handover — `lp.rs` is byte-identical at handover; the 2a
formulation is unchanged). IPM produced the **same** minimised unserved as
simplex (valid solve). It wins at 1 year but **reverses sharply at 5**:

| Horizon | Periods | LP solve time (IPM) | Peak RSS (IPM) | vs simplex |
|--------:|--------:|--------------------:|---------------:|-----------|
| 1 year  |  17,520 | **42.0 s**          | **0.42 GB**    | 1.4× faster, 2.6× less RSS |
| 5 years |  87,600 | **1409.4 s** (23.5 min) | **4.28 GB** | **2.5× SLOWER, more RSS** |

At 1 year IPM is faster and much leaner; but by 5 years its interior-point
+ crossover phase dominates and it is **2.5× slower than simplex** and uses
**more** memory (4.28 GB vs 3.25 GB). So **IPM does not rescue the
full-horizon solve** — it scales worse in time than the default. Its only
regime of advantage is ~1-year-scale subproblems. (This is worth
remembering for the rolling window below: for windows of ~1 year IPM would
help, but the window must be ≥ ~3 years for correctness, where simplex is
the better choice.)

## The decision — full-horizon is NOT viable; rolling-horizon is required

A single full-horizon (40-year × 3-zone) LP solve is **hours** of wall
time and, with the default solver, **aborts** at 10 years already
(extrapolated ~3 h and ~22 GB if it did not crash first). The
interior-point alternative does **not** help — it scales worse in time and
is already slower than simplex by 5 years. The bisection needs 15–30 such
solves (**tens of hours, likely OOM** on a 16–32 GB desktop, and it does
not even complete at 10 years with the default solver). This is **~30–60×
over** the "few minutes per solve" budget under either solver. **The
full-horizon LP inside the bisection is not viable.**

Per the fork in the work order, PHASE 1 stops here: **the rolling-horizon
machinery is NOT built** — this note records the recommended design for
supervisor review before any build.

## Recommended rolling-horizon design (for supervisor review)

A **receding-horizon** LP with SoC continuity, sized so the window always
spans a full drawdown–recharge cycle (rule 3: short windows and
typical-day / representative-period aggregation are **RULED OUT** — a
window that cannot see a full episode is not perfect foresight and would
understate the requirement).

1. **Window length W and commit step S.** The binding constraint is the
   **longest below-full episode**: the RS-comparable lean sizing shows
   **720.6 days** (2009-12-09 → 2011-11-30, nadir 2011-04-25), with
   runners-up of 442 days (1996–98) and 407 days (1987–88)
   (`docs/notes/stage-3-storage-run-report.md`). A drawdown beginning
   anywhere in a committed block must have its **full recharge tail inside
   the window**, so the look-ahead `W − S ≥ ~721 days ≈ 2.0 years`.
   Recommend **S = 1 year, W ≥ 3 years** (commit 1 year, look ahead ≥ 2
   years) — round up for margin if tractable.

2. **SoC continuity across windows (no reset).** Solve window k over
   `[t_k, t_k + W)`, commit only the first `S` of decisions **including
   the store SoC trajectory**, and carry the committed terminal SoC at
   `t_k + S` as the **fixed initial SoC** of window k+1. This is a warm
   hand-off, not an annual reset (the reset is exactly the "few days of
   storage" error the multi-year tests already guard against). No
   final-SoC constraint inside a window (perfect foresight may draw the
   store down toward the window end within the discarded look-ahead).

3. **Window-sensitivity toward W → ∞.** Report the sizing requirement at
   increasing windows (e.g. W = 2, 3, 4, 5 years) and show convergence
   toward the full-foresight ideal (W → ∞). The **quoted** figure is the
   converged / largest-tractable-window value; shorter windows understate
   and are reported as the trend, never the answer.

4. **Solver: keep the default simplex for the window.** The measured IPM
   result rules out switching: IPM only wins at ~1-year scale and is
   already 2.5× slower and heavier by 5 years, whereas the window must be
   ≥ 3 years. A W = 3-year window sits between the clean 1- and 5-year
   simplex points — extrapolated ~4–5 min and ~2.4 GB, comfortably below
   the 10-year abort threshold. So the default solver (already pinned
   deterministic in the 2a tests, ADR-5) is the right per-window engine;
   the window size is what keeps it safe. (Revisit only if a window is ever
   cut to ~1-year-scale subproblems, where IPM would help.)

5. **Residual cost — flag for the supervisor.** Windowing definitively
   removes the memory wall and the solver abort (each window is a ~3-year,
   ≤5-year-scale solve). But a full 40-year dispatch is then ~38 window
   solves at ~4–5 min each (~2–3 h), and the **bisection multiplies that
   by 15–30** → still on the order of tens of hours per sizing run. Before
   the build, the supervisor should scope one or more of: (a) warm-starting
   the simplex basis across bisection iterations and across adjacent
   windows (successive store sizes and successive windows are close); (b) a
   two-stage approach that first identifies the binding episode(s) cheaply
   and sizes against those; (c) a coarser bisection or a dual-based direct
   sizing. These are design decisions, not measurements, and belong to the
   approved rolling-horizon build — not PHASE 1.

## Determinism and robustness notes

- Every solve above is deterministic (single-threaded HiGHS, ADR-5); the
  2a determinism test already pins this for the default solver.
- **Robustness finding to report:** `run_multi_lp` can **abort the
  process** (uncaught C++ `std::length_error` from HiGHS) on very large
  LPs — a library crate should not abort. Windowing avoids it by keeping
  each solve small; if the full-horizon path is ever exercised directly it
  should carry a size guard. Reported to the supervisor, not fixed here
  (2a is frozen; this is a 2b/robustness follow-up).

## Reproduce

```
cargo test -p grid-adequacy --release --test tractability_bench --no-run
BIN=$(find target/release/deps -name 'tractability_bench-*' -type f -perm +111)
/usr/bin/time -l "$BIN" bench_lp_1yr  --ignored --nocapture --test-threads=1
/usr/bin/time -l "$BIN" bench_lp_5yr  --ignored --nocapture --test-threads=1
/usr/bin/time -l "$BIN" bench_lp_10yr --ignored --nocapture --test-threads=1  # aborts
```

The IPM rows were measured with a temporary `GRID_LP_IPM=1` env switch in
`run_multi_lp` that selects `HighsSolverType::Ipm`; that switch was
**reverted before handover** (recorded here so the finding is reproducible
by re-adding one line).
