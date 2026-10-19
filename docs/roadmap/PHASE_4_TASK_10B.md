# Phase 4, Task 10B - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Level-parallel exact-mode BFS behind `--threads N` with CANONICAL violation
selection (deterministic by construction: byte-identical to `--threads 1` for
any N, in both backends), plus an opt-in `--fast` early-abort mode documented
as non-parity. Revives task 10's deferred action 4 with the corrected design
agreed with the user on 2026-10-19 (plan correction in PHASE_4_PLAN.md).

## Context

- Task 10 deferred `--threads` citing (a) no benchmark needing it (largest run
  ~1.1 s vs the ~30 s budget) and (b) fragility of canonical violation
  selection under parallel expansion. The user proposed a two-mode design with
  a shared-seed RNG controlling thread switching; design review rejected the
  RNG mechanism: identical seeds do NOT make real-time thread interleavings
  reproducible (the same random numbers are applied to different wall-clock
  events), and any decision depending on completion order breaks cross-backend
  determinism.
- The corrected source of determinism is CANONICAL SELECTION with arbitrary
  work distribution: parallelize only WHICH thread evaluates WHICH successor;
  commit successors and detect violations by walking the PRECOMPUTED outcomes
  in canonical order (level x frontier x input x effect) exactly as the
  sequential engine does. The result is then a pure function of the spec -
  independent of thread count, machine, scheduler, and any seed. This is
  stronger than seed-based reproducibility and keeps EVERY existing pin valid
  (no re-pinning anywhere; a broken pin means a bug in the parallel path).
- No re-execution cost: phase A already runs `step_successor` (monitors
  included) for every successor and stores the outcome; phase B is an ordered
  scan of stored results (visited insert, parent/depth/adj push, first
  violation), not a re-run of transition logic.
- Rejected alternatives (recorded so they are not re-proposed):
  1. Shared-seed RNG controlling thread switching - cannot guarantee
     reproducible results (see above).
  2. RNG-driven exploration ORDER (fuzz-like randomized commit order) - would
     change which violation/trace is reported (new pin tables per seed) and
     require a bit-identical PRNG across Rust and C (a second parity surface),
     without parallelizing better than canonical selection.
- `--fast` mode: during phase A, a found violation sets an atomic flag and
  workers stop scheduling new work; the run reports the FIRST-FOUND
  (non-canonical) violation. Soundness: never a false pass (a found violation
  is real; pass still requires full completion). Non-parity: label/trace/
  states-at-abort are run-dependent - documented exactly like `--timeout-sec`.
- Flag surface: `--threads N` (default det behavior when N > 1) and
  `--fast` (only meaningful with `--threads N > 1`). Both are codegen options
  (embedded default) AND runtime args of the generated binary, following the
  `--max-states` pattern. Default N = 1 keeps today's code path and output
  byte-identical.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Exact engine - C | the C runtime being parallelized (pthreads mirror) |
| 7. | Liveness engine - C | FAIL variants exercise liveness counterexamples under threads |
| 9. | Benchmark suite test | the parity gate runs on its benchmark set + FAIL variants |
| 10. | Performance features | this task revives its deferred action 4 with the corrected design |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Docs sync | documents `--threads`/`--fast` semantics in DESIGN/CODEGEN |
| 14. | Close-out | full verification includes the threads parity gate |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/rust_mc.rs` | change | two-phase level-parallel BFS (std::thread), canonical commit, `--threads N`/`--fast` wiring |
| `src/backends/c_mc.rs` | change | pthreads mirror of the same structure; identical canonical commit |
| `src/codegen.rs` | change | `--threads N` / `--fast` option parsing + defaults (generic help) |
| `src/backends/rust.rs`, `src/backends/c.rs` | change | per-backend command help strings |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/benchmark_mc_test.rs` | change | threads 1 vs 2 (vs 4) byte-identity gate on all benchmarks + FAIL variants; `--fast` smoke test |

## Planned actions

### Action 1: Rust deterministic mode (`--threads N`)
Restructure the exact-mode BFS in `rust_mc.rs` into two phases per level:
- Phase A (parallel): partition frontier x input x effect work across N
  `std::thread` workers; each worker owns its slice (no shared mutable state);
  outcomes (Success with full successor node data, or Violation record) stored
  in a preallocated array indexed by canonical position.
- Phase B (sequential, canonical): walk outcomes in canonical order, committing
  Successes EXACTLY as today (visited insert at enqueue, parent/depth/adj
  push, max_depth update) and stopping at the first Violation. The
  `--max-states` limit is enforced AT COMMIT (phase B), so unknown-state-limit
  behavior matches threads=1; phase A may compute work beyond the limit
  (wasted, never incorrect).
- N = 1 must keep the current single-threaded path byte-identical (no
  regression surface for the default).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | two-phase BFS + flag wiring (codegen option + runtime arg) |
| `src/codegen.rs` | change | `--threads N` parsing, default 1, generic help text |

### Action 2: C mirror
Same two-phase structure in `c_mc.rs` with pthreads; identical canonical
commit order; generated C stays warning-free under `gcc -Wall`. Validate with
a one-off ASan run of the benchmark suite binaries (C side) before commit.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | pthreads mirror; `-pthread` link flag if required by the toolchain |

### Action 3: Parity gate (threads 1 vs 2 vs 4)
Extend `tests/benchmark_mc_test.rs`: for each of the three benchmarks AND both
inline FAIL variants, run both backends at N = 1, 2 and 4 in det mode and
assert BYTE-IDENTICAL stdout across all runs. No pin changes are expected
(canonical order is unchanged) - if any pinned value breaks, that is a bug in
the parallel implementation, not a re-pin.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/benchmark_mc_test.rs` | change | byte-identity gate over N x backend on benchmarks + FAIL variants |

### Action 4: `--fast` non-deterministic mode
Opt-in early abort: phase A sets an atomic flag on first found violation;
workers stop scheduling new work; the run reports the first-found (possibly
non-canonical) violation with its real counterexample. Documented non-parity
(label/trace/states-at-abort run-dependent), soundness = never a false pass.
Smoke test only: FAIL variants exit 1 with `status=fail` and a label that is a
genuinely violated property of the variant.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs`, `src/backends/c_mc.rs` | change | atomic early-abort path behind `--fast` |
| `tests/benchmark_mc_test.rs` | change | `--fast` smoke test (soundness, not parity) |

### Action 5: Help text
`--threads N` / `--fast` in the generic codegen help and both per-backend
command help strings, with the non-parity note for `--fast` (same style as the
task 10 `--timeout-sec` documentation). DESIGN/CODEGEN doc sync stays with
task 13.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen.rs`, `src/backends/rust.rs`, `src/backends/c.rs` | change | flag documentation in all three help locations |

## Task deliverables
- `--threads N` det mode: byte-identical to N = 1 across backends and N values
  (test-gated, zero pin changes).
- `--fast` opt-in early-abort mode, documented non-parity.
- Parity gate in the benchmark suite; help text updated in all locations.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Det mode: byte-identical stdout for N in {1,2,4} x {rust,c} on all three benchmarks + both FAIL variants; zero pin changes anywhere.
- [ ] gcc -Wall full sweep clean after the c_mc.rs change; one-off ASan run of the C benchmark binaries clean.
- [ ] `--fast`: smoke-tested (exit 1, status=fail, genuinely violated label), documented non-parity in help text.
- [ ] Default behavior unchanged: without `--threads`, output byte-identical to pre-task on the full suite.

## Non-goals
- RNG/seed-driven exploration order or seeded thread switching (rejected in
  design review - see Context; would create a new pin surface + cross-backend
  PRNG parity requirement).
- Parallel fuzz mode, SMT mode (task 11), docs beyond help text (task 13).

## Other information
- This is the largest engine change of the phase so far. Fallback: if
  byte-identical canonical commit cannot be achieved after reasonable effort
  in EITHER backend, record a plan correction reverting to the task 10
  deferral with the concrete failure evidence (do not ship a partially-parity
  flag - a Rust-only `--threads` would break cross-backend flag-surface parity).
- The benchmark suite's wall-clock numbers will change once threads are used;
  they remain informational only (never asserted) - no re-recording needed.
- Rust: `std::thread` only (no rayon or other new dependency). C: pthreads;
  verify whether the generated-code compile command needs an explicit
  `-pthread` on this toolchain and add it in the test harness if so.
- Timeout checks stay wall-clock (non-parity) as today; they interact with
  `--fast` only by also bounding phase A.
