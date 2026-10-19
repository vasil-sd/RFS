# Phase 4, Task 9 - Status

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-19

## Files added / changed / removed
- `tests/benchmark_mc_test.rs` (new, ~780 lines): the benchmark MC suite - three tests, one per planned action:
  1. `bench_mc_pass_complete_metrics_parity`: specs 43/44/45 x both backends in exact mode; asserts `pass-complete` + exit 0, metrics equal to the task 6/8 record (165/330/22, 129985/519940/276, 11098/99882/22), and every parsed field IDENTICAL across backends.
  2. `bench_mc_fail_variants_both_backends`: two inline FAIL variants (full modified sources as consts - test inputs, no new example files): **ring_no_grant** (44 with the Tick grant arm removed) -> exit 1, `fail`, reason `property-violation`, label `holder_no_request`, step 1, violation_step 1, states=1/transitions=1/max_depth=0 - safety catches it before liveness; **elev_noup** (45 with the Idle go-up arm removed) -> exit 1, `fail`, reason `liveness-violation`, label `up2_served`, step 0, violation_step 0, **cycle length 3**, states=144/transitions=1296/max_depth=7. Both backends asserted field-identical including the lasso counterexample fields.
  3. `bench_mc_json_report_and_table`: writes `target/mc_benchmarks.json` (per spec x backend: status, states, transitions, max_depth, unexpected_terminations, memory_estimate_bytes, wall_ms), reads it back and asserts on PARSED fields only (never byte-for-byte; wall-clock never asserted), and prints a per-backend summary table to the test output.
  - Harness: codegen from source string or example file via `rfs::codegen` (same BackendOptions as the parity suite), binary cache under `target/rfs_benchmarks/`, Rust MC binaries built with `rustc --edition 2021 -O` (benchmark methodology matches task 8's sizing data; informational wall clock only), C with `gcc -Wall` and the zero-warning hard bar. Parser covers the machine summary line plus `unexpected terminations`, `memory estimate`, `trigger/violation step`, `deadlock depth` and `cycle length` human lines.
- `BUILD.md` (changed): section 3 table row for the suite + one sentence in the "Other suites" paragraph. The pre-existing stale counts in that section (170 pass / 42 valid / 513 generated) were left untouched - bulk update stays with task 13 per its plan.

## Benchmark data (informational - input for task 10)
Wall-clock from the suite's own runs (Rust `-O`, C plain `gcc`; deterministic metrics identical across backends):
- **43_sliding_window**: pass-complete, 165/330/22; ~1 ms both backends; memory estimate 5,610 B; unexpected terminations 0.
- **44_token_ring**: pass-complete, 129985/519940/276; ~1.1 s (rust) / ~0.9 s (c); memory estimate 4,679,460 B (~4.7 MB - confirms task 8's figure); unexpected terminations **129985 = one per config** (design-intended `Request(holder)` rejection - the suite asserts cross-backend identity, never zero).
- **45_elevator**: pass-complete, 11098/99882/22; ~79 ms (rust) / ~52 ms (c); memory estimate 388,430 B; unexpected terminations 0.
- Cold suite run (all 10 binaries codegen'd + compiled from scratch): **~8.5 s** inside `cargo test` - far under the ~30 s per-spec budget; no plan correction needed.
- For task 10: default `--max-states` 1,000,000 sits 7.7x above the largest benchmark (129,985) - no tuning pressure from this phase's data; the ring is the only workload where compile optimization or threading would buy anything (~1 s).

## Bugs found and fixed during development
None in the engine - the suite passed on first run against both backends (the task 7/8 liveness + C-wrap work holds under the benchmark set). One self-inflicted clippy warning (`map_identity` in the draft JSON test) was removed before completion.

## Tests
- `cargo test`: **220/220** (was 217 after task 8; +3 from this suite - record for task 14's full verification).
- `cargo clippy --all-targets`: baseline only (11 lib + 1 codegen_test, all pre-existing); zero from this file.
- `cargo fmt --check` clean.
- gcc -Wall: all five generated C MC binaries (3 specs + 2 variants) compile warning-free (asserted in-suite).

## Findings / notes for later tasks
- **For task 10:** data above; the suite's JSON report (`target/mc_benchmarks.json`) is a ready-made machine-readable source - wall-clock fields are informational and vary with host load, deterministic fields do not.
- **For task 13 (docs sync):** note that benchmark specs' unexpected-termination counts are by design non-zero for the ring (one per config) - any docs table of "expected MC output" must not imply zero. The stale BUILD.md/START.md counts (now 220 tests / 45 valid / 522 generated) still await the task 13 bulk update.
- **For task 14 (close-out):** full verification = `cargo test` at **220** (this suite contributes 3 and runs in ~8.5 s cold); the JSON report is a development artifact under `target/` - never checked in, never asserted byte-for-byte.

## Exit criteria
- [x] `cargo test` green including the new suite (220 total; pass count recorded for task 14).
- [x] All three benchmarks pass-complete in BOTH backends with identical deterministic metrics (pinned to the task 6/8 record).
- [x] FAIL variants: correct status/reason/label + counterexample parity in both backends (safety `holder_no_request` step 1; liveness `up2_served` cycle length 3).
- [x] `target/mc_benchmarks.json` written; per-backend table printed; wall-clock informational only.
- [x] BUILD.md section 3 updated.
