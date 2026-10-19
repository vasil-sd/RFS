# Phase 4, Task 10B - Status

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-19

## Files added / changed / removed
- `src/backends/rust_mc.rs` (changed): two-phase level-parallel exact-mode BFS. New generated-code pieces: `LevelOutcome` enum + pure `expand_config()` (phase A work), the `'outer` loop restructured into phase A (sequential for N=1; `std::thread::scope` workers over disjoint frontier ranges for N>1, capturing only Copy values - pre-bound references + per-worker range tuples) and phase B (sequential canonical commit: visited insert at enqueue, parent/depth/adj pushes, first violation, state limit AT COMMIT, deadlock check - byte-identical order to the pre-task engine). `--threads N` (codegen default via new `DEFAULT_THREADS`) and runtime-only `--fast` (atomic early-abort flag; workers stop scheduling new configs once any finds a violation; the first found violation is reported).
- `src/backends/c_mc.rs` (changed): pthreads mirror - `mc_outcome_t`/`mc_outcomes_t` + pure `mc_expand_range()` + `mc_expand_worker()` (per-worker arg struct, C11 `atomic_int` abort flag with explicit relaxed ordering), same two-phase loop structure; inline fallback if `pthread_create` fails (results stay canonical); per-level outcome storage freed on every exit path. `#include <pthread.h>`/`<stdatomic.h>` added; links without `-pthread` on this toolchain (glibc >= 2.34).
- `src/codegen.rs` (changed): `BackendOptions.mc_threads` (default `EXACT_DEFAULT_THREADS = 1`), `--threads N` codegen option parsing (positive integer), `CodegenRequest::Generate` field, generic help text.
- `src/codegen_mc.rs` (changed): `pub const EXACT_DEFAULT_THREADS: usize = 1`.
- `src/main.rs` (changed): request plumbing for the new field.
- `src/backends/rust.rs`, `src/backends/c.rs` (changed): per-backend help - `--threads N` option line, usage line, runtime-flags line now lists `--threads N, --fast (non-parity early abort)`.
- `tests/benchmark_mc_test.rs` (changed): `run_parsed` refactored over new `parse_run` + `run_args_full(bin, args)` (keeps raw stdout); two new tests - `bench_mc_threads_parity` (all five workloads x both backends: `--threads 2/4` must be BYTE-IDENTICAL to rust `--threads 1`, traces included) and `bench_mc_fast_smoke` (soundness contract only: exit 1, status=fail, genuinely violated label). Suite is now 5 tests.
- `BUILD.md` (changed): factual correction of the suite row this project wrote in task 9 (3 -> 5 pass; threads gate mentioned). Doc sync beyond that stays with task 13.

## Design notes / deviations
- **Determinism by construction:** canonical SELECTION with arbitrary work distribution - results are a pure function of the spec, independent of thread count, machine, and scheduler. No RNG/seed anywhere (rejected in design review; see plan correction 10b).
- **`--fast` is runtime-only** (deviation from the task file's "both are codegen options"): a non-deterministic mode must never be a build-time default that cannot be disabled per invocation. Recorded here per protocol.
- **State limit enforced at canonical commit (phase B), not during phase A** - unknown-state-limit behavior matches threads=1 exactly; phase A may compute work beyond the limit (wasted, never incorrect).

## Verification (all in final state)
- Byte-identity: all five workloads (3 benchmarks + 2 FAIL variants) x {rust, c} x N in {default, 1, 2, 4} produce IDENTICAL stdout (verified manually before the gate existed; now enforced by `bench_mc_threads_parity`). Zero pin changes anywhere.
- `cargo build && cargo test`: **222/222** (was 220 after task 10; +2 new tests). Benchmark suite: 5/5 in ~11 s.
- `cargo clippy --all-targets`: baseline only (11 lib + 1 codegen_test, all pre-existing); zero from this task. `cargo fmt --check` clean.
- gcc -Wall one-shot sweep over ALL valid examples after the c_mc.rs change: every generated C MC binary compiles with zero warnings (0 skipped).
- ASan+UBSan of the threaded C binaries (`--threads 4`, incl. `--fast` on the FAIL variant): **zero memory-safety errors**. The only LeakSanitizer findings are the PRE-EXISTING BFS storage block (nodes/parent/visited/adj, never freed before exit - unchanged by this task); the new per-level outcome + worker-arg allocations are absent from the leak report (freed on every path).
- **TSan could not be run in this environment**: it FATALs with "unexpected memory mapping" even on a minimal 10-line pthread program (environment/kernel limitation, verified before blaming the code). Race-freedom rests on: (a) ASan-clean threaded runs, (b) design - workers write only disjoint per-config outcome lists plus an atomic flag; all shared engine state is read-only in phase A; phase B runs strictly after join (happens-before), (c) the byte-identity gate across repeated runs at different N.
- `cargo generate-code-tests && cargo test-generated-code`: **522/522** (unchanged). `cargo run -- validate --input rfs examples/valid`: exit 0.

## Performance data (informational - 32-core host)
Ring (largest benchmark, rust -O): t1 = 966 ms, t2 = 1003 ms, t4 = 993 ms - **no speedup yet**: per-level thread spawn/join + outcome allocation overhead exceeds the per-level work at these sizes. The capability is in place and gated for larger specs; the default stays N=1 (sequential) because it is still fastest on this phase's workloads. This confirms task 10's original assessment: no tuning pressure today.

## Bugs found and fixed during development
No engine bugs - parity held as soon as the code compiled. Compile-level fixes in the Rust template before completion: inner `collect()` type inference (explicit `Vec<_>`), flatten of per-worker chunks, scoped-thread capture (pre-bound references + Copy range tuples instead of `move` on loop variables), and a non-exhaustive match after flattening phase B's outcome arms.

## Findings / notes for later tasks
- **For task 13 (docs sync):** document `--threads N` (deterministic, byte-identical to N=1) and `--fast` (non-parity early abort, soundness-only) in DESIGN/CODEGEN; note the TSan environment limitation if sanitizer guidance is documented.
- **For task 14 (close-out):** full verification = `cargo test` at **222** (this task adds 2 tests, ~11 s suite runtime); the threads gate compares raw stdout across N and backends on all five workloads.

## Exit criteria
- [x] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green (222/222).
- [x] Det mode: byte-identical stdout for N in {1,2,4} x {rust,c} on all three benchmarks + both FAIL variants; zero pin changes anywhere.
- [x] gcc -Wall full sweep clean after the c_mc.rs change; ASan/UBSan threaded runs clean (TSan unavailable in this environment - recorded with evidence).
- [x] `--fast`: smoke-tested (exit 1, status=fail, genuinely violated label), documented non-parity in all three help locations.
- [x] Default behavior unchanged: without `--threads`, output byte-identical to pre-task on the full suite (N=1 path verified against every pin).
