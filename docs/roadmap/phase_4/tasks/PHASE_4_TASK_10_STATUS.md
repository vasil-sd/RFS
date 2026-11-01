# Phase 4, Task 10 - Status

**State:** COMPLETE (all exit criteria met; `--threads` deferred via plan correction)
**Date:** 2026-10-19

## Files added / changed / removed
- `src/backends/rust_mc.rs` (changed): statistics polish - generated code now embeds `MONITORS_BITS` (from `plan.encoding.monitors_bits`, the encoded bit width of all property + assumption monitors) and the exact-mode report prints `  monitor size: N bits` between `first unexpected` and `memory estimate`.
- `src/backends/c_mc.rs` (changed): identical polish - `#define MONITORS_BITS N` + the same `monitor size:` line, same position, same format. Field set of the statistics block is now: machine summary line (status/states/transitions/max_depth [+ reason/label/step]), `unexpected terminations`, `first unexpected` (conditional), `monitor size`, `memory estimate` - identical in both backends.
- `src/codegen.rs` (changed): generic codegen help - `--timeout-sec` documented as wall-clock ONLY, NOT parity-guaranteed (same spec can time out on one machine and not another), never used by parity or benchmark assertions.
- `src/backends/rust.rs`, `src/backends/c.rs` (changed): the per-backend command help strings carry the same `--timeout-sec` documentation (these are where `rfs codegen <backend> --help` actually renders the flag list).
- `tests/benchmark_mc_test.rs` (changed): parses `monitor size:` into `BenchRun`; `Pin` extended with pinned monitor sizes (43: 4 bits, 44: 10 bits, 45: 4 bits) asserted per backend; cross-backend parity of the new field covered by the existing full-field comparison; `monitors_bits` added to the JSON report.

## Action 2 decision: `--max-states` default KEPT at 1,000,000 (no plan correction needed)
Task 9's benchmark data: largest state space is the token ring at **129,985** states; elevator 11,098; sliding window 165. The default sits **7.7x above the largest benchmark** with every spec passing complete far below it - no tuning pressure (task 9's status said the same). `EXACT_DEFAULT_MAX_STATES` stays 1_000_000 (already unit-test-pinned in `codegen_mc.rs`); usage text already shows the correct default, so no code change.

## Action 4 decision: `--threads N` DEFERRED (plan correction)
Deferred per the plan's own escape hatch ("if canonical selection proves fragile ... defer via plan correction"). Evidence:
1. **Data (task 9):** the largest benchmark runs in ~1.1 s (ring, rust -O) / ~0.9 s (C); the other two are <80 ms where threading overhead would dominate. The suite budget is ~30 s per spec - a 27x headroom before parallelism becomes necessary. Best-case gain today: ~0.5 s on one spec.
2. **Engine analysis:** canonical order in exact mode comes from (a) inline violation detection during expansion (first `StepOutcome::Violation` in level x frontier x input x effect order aborts the search) and (b) enqueue-order commit of `next_frontier`, which fixes the next level's canonical order. Preserving both under parallel expansion requires a two-pass speculative design per level (parallel pre-computation of ALL successors, then a sequential canonical replay that commits and detects violations) - which doubles the work on FAIL runs (the common case in tests) and adds race-sensitive code to BOTH backends (C would need pthreads). That complexity/risk profile is not justified by ~0.5 s.
3. **Revisit trigger:** any benchmark spec exceeding ~30 s in exact mode, or an explicit CI time-budget requirement - at that point parallel search becomes necessary and the two-pass design should be prototyped in Rust first with a threads 1 vs 2 parity gate before mirroring to C.

## Bugs found and fixed during development
None in the engine. One self-inflicted clippy warning (`u32 as u32` cast in the C codegen constant) removed before completion.

## Tests
- `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`: all green. `cargo test`: **220/220** (same count as task 9 - this task extended the existing benchmark suite rather than adding tests; record for task 14).
- Monitor size verified identical across backends on all three benchmarks (4/10/4 bits) and pinned in the suite.
- gcc -Wall one-shot sweep over ALL valid examples after the c_mc.rs change: every generated C MC binary compiles with zero warnings (0 skipped).

## Findings / notes for later tasks
- **For task 13 (docs sync):** document (a) the `monitor size` statistic (definition: encoded bit width of all property + assumption monitors; deterministic; feeds CONFIG_TOTAL_BYTES and hence the memory estimate), (b) `--timeout-sec` as wall-clock/non-parity, (c) that MC flag help lives in `src/codegen.rs` (generic) and the per-backend command descriptions (`rust.rs`/`c.rs`) - NOT in `main.rs` usage text (plan deviation recorded below).
- **For task 14 (close-out):** full verification = `cargo test` at **220**; `--threads` is deferred, so no threads-parity gate exists to run; the deferral rationale + revisit trigger are above.

## Plan deviations / corrections (recorded per protocol)
1. **Action 3 file:** plan listed `src/main.rs` for the `--timeout-sec` usage text, but `main.rs`'s `usage()` does not document MC flags at all - the flag help is rendered from `src/codegen.rs` (generic codegen help) and the per-backend command descriptions in `src/backends/rust.rs` / `c.rs`. All three locations were updated; no main.rs change was needed.
2. **Action 4:** `--threads N` deferred via plan correction (rationale + revisit trigger above). The phase file lists parallel search as "where safe"; the data says it is not yet needed and the canonical-selection risk is real.

## Exit criteria
- [x] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green (220/220).
- [x] Benchmark suite asserts parity of every statistics field across backends (incl. new `monitor size`, pinned 4/10/4).
- [x] `--max-states` default: KEPT at 1,000,000, recorded with task 9 data (7.7x headroom over the largest benchmark).
- [x] `--timeout-sec` documented as non-parity in usage text (generic + both per-backend helps).
- [x] `--threads`: deferral recorded as a plan correction with data + engine-analysis rationale and a revisit trigger.
