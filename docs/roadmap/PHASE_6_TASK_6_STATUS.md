# Phase 6, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_6.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `examples/valid/47_http2_stream.rfs` | HTTP/2-like stream FSM (IDLE/OPEN/HALF_CLOSED_REMOTE/HALF_CLOSED_LOCAL/CLOSED, terminal Closed with absorbing self-loops, bit<5> send window with wrap-cap arm, RST from any live state). Safety: `closed_terminal`, `rst_goes_closed`, `grant_only_when_alive`; bounded liveness `closes_after_local_end` [8] backed by the `peer_ends` assumption [7]. Version "1.0" |
| `examples/valid/48_usb_control_transfer.rfs` | USB-like control transfer (SETUP/DATA/STATUS phases, length-4 payload abstraction, bit<2> NAK retry counter saturating into an absorbing Error state that is UNREACHABLE under the `env_progress` assumption [4]). Safety: `no_double_setup`, `status_only_when_complete`, `error_terminal`; tight bounded liveness `completes_after_setup` [16]. Version "1.0"; plan correction 6c removed the `Error on Setup` absorbing arm (see below) |
| `tests/common/mc_json.rs` | Shared helper for the Phase 6 differential suites' MC JSON byte comparisons: `zero_peak_rss` normalizes the informational `peak_rss_kb` field to 0 before cross-backend / engine-vs-ref byte identity checks (the only machine-dependent field in the D-P5-10 document) |
| `examples/valid/49_can_arbitration.rfs` | CAN-like arbitration/error-state machine (TX_REQ/ARB_WIN/ARB_LOSE/AckOk/AckErr/BUS_OFF/RECOVER, bit<5> error counter with +4 abstraction weight and 31-saturation cap arm, concrete never-inspected `tx_id: bit<4>` multiplying the state space by 16). Safety: `data_only_after_win`, `active_needs_threshold`, `passive_band`, `one_tx_at_a_time`, `recover_resets`; bounded liveness `idle_after_ack` [4] backed by `env_bus_idle` [3]. Version "1.0" |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen_mc.rs` | changed | Action 2: `McJsonReport.peak_rss_kb` (statistics, INFORMATIONAL - machine-dependent; cross-backend byte comparisons normalize it to 0) + `JsonCounterexample.ce_steps()` (`counterexample.ce_steps`, deterministic trace length: steps for a trace, prefix+cycle for a lasso, depth for a deadlock config; always the last key of every counterexample object) + new JSON keys + D-P5-10 shape pins re-pinned (all five status/reason documents) |
| `src/backends/rust_mc.rs` | changed | Action 2: generated Rust MC self-reports peak RSS via `/proc/self/status` VmHWM (`peak_rss_kb()`, 0 on read failure) and emits it in the JSON report; `ce_steps` rendered for all three counterexample kinds |
| `src/backends/c_mc.rs` | changed | Action 2: generated C MC self-reports peak RSS via `getrusage(RUSAGE_SELF).ru_maxrss` (kB on Linux) - same field, same position; gcc -Wall clean |
| `src/backends/{c,rust}.rs` | changed | Plan correction 6a: new `c_binding_ident` / `rust_binding_ident` - pattern bindings colliding with identifiers a generated body uses unqualified are renamed `<name>__rfsb` (further suffixed until unique) |
| `src/backends/{c_fsm,rust_fsm}.rs` | changed | Plan correction 6a: binding scopes now pass their per-site `reserved` lists (C MC effect counter + loop vars; C commit call `f`/`in`/`handler`/`user`; Rust equivalents) and render guards/emits through the mangled idents |
| `src/validator.rs` | changed | Plan correction 6a: `Env.binding_idents` map (RFS binding name -> emitted identifier; empty = plain names) shared by both backends' expression rendering |
| `src/codegen_test.rs` | changed | Plan correction 6b: `expect_progress` replaced by `expect_progress_from_initial` - computed exactly as the generated positive drive runs (representative candidate inputs, initial variable values); old `!transitions.is_empty()` heuristic over-approximated and emitted failing drives for specs whose initial arms reject the representative input (spec 47). Lib pin: `plan_expect_progress_respects_guarded_out_initial_arms` |
| `tests/{backend_parity,ref_mc_diff,ref_mc_sweep,frontend}_test.rs`, `tests/common/mod.rs` | changed | Action 2: cross-backend / engine-vs-ref JSON byte comparisons now normalize `peak_rss_kb` to 0 via `common::mc_json::zero_peak_rss`; backend parity gains the three new benchmark specs (pass-complete entries) and its golden document pin re-pinned with `peak_rss_kb = 0` |
| `src/ref_mc.rs` | changed | Action 2: in-process reference MC reports `peak_rss_kb = 0` (documented convention); the two D-P5-10 golden full-document pins re-pinned with the new fields (the final full-suite run caught these two - all other shape pins had been re-pinned in action 2) |
| `examples/valid/48_usb_control_transfer.rfs` | changed | Plan correction 6c: removed the `Error on Setup` absorbing arm (it violated `no_double_setup` once Error became reachable under relaxed assumptions, exposed by the action-3 FAIL variant); header comment documents the RESET-first convention. Pass run under `env_progress` byte-identical (Error unreachable) |
| `tests/benchmark_mc_test.rs` | changed | Action 3: extended suite (details in "Tests add/change/remove") |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `BUILD.md` | changed | New section 7 "Benchmarks in CI" (single command, what it writes/prints, asserted vs informational metric list, GitHub Actions yaml snippet - documented only, no workflow committed); section-3 benchmark row refreshed to the six-spec suite with current pins; examples row corrected to 50 valid / 13 invalid (action 1 consequence); aggregate-crate counts updated (189 modules / 1601 tests - action 1 consequence); trailing paragraph on `tests/benchmark_mc_test.rs` extended. Remaining stale section-3 baseline totals (the "376 pass" cell) stay for task 9 per the plan's file ownership |
| `docs/roadmap/PHASE_6_TASK_6.md` | changed | Exit criteria checked off 5/5; "Other information" carries the measured 47/48/49 counts (the Pins); new "Task corrections" section (6a/6b/6c + 6d design note) |
| `docs/roadmap/PHASE_6_PLAN.md` | changed | "Plan corrections" table: rows 6a (binding shadowing), 6b (expect_progress), 6c (spec-48 arm) appended per the bug-found protocol |

## Changed functionality
- MC JSON report (both generated backends + in-process ref_mc): gains `statistics.peak_rss_kb` (actual peak RSS, self-reported by the binary; 0 on read failure / for ref_mc) and `counterexample.ce_steps` (deterministic counterexample length - a CE QUALITY metric). Key order is part of the byte-parity contract; every cross-backend byte comparison normalizes `peak_rss_kb` to 0.
- Spec 48: SETUP in Error is now an unexpected termination instead of an absorbed no-op (RESET-first recovery, consistent with Stalled and with `no_double_setup`).

## Removed functionality
- None (the spec-48 arm removal is a modeling correction, not a feature loss - the state remains reachable under relaxed assumptions and absorbs all DATA-phase bus activity).

## New functionality
- Three v1.0-versioned benchmark specs covering new protocol families (transport flow control / bus device control / fieldbus error handling), each with safety + bounded-liveness properties and abstraction-stress fields, all exact-MC-tractable far below the ~150k budget (434 / 104 / 240 states).
- Benchmark suite FAIL-variant coverage of a third failure flavor: assumption relaxation (`--assumes none`) - the unmodified spec 48 run whose Error state becomes reachable and whose bounded obligation expires at step 16.
- In-process parse + codegen timing for the eight timed specs (03/04 + the six benchmarks) in the JSON report and a printed table (informational only).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `tests/benchmark_mc_test.rs` - `bench_mc_pass_complete_metrics_parity` | extended | BENCH_SPECS 3 -> 6; fresh Pins for 47/48/49 (434/21857/7/10, 104/340/15/11, 240/448/41/7 - states/transitions/max_depth/monitors_bits) |
| `tests/benchmark_mc_test.rs` - `bench_mc_fail_variants_both_backends` | extended | cases 2 -> 5: existing ring (ce_steps pin 1) + elev (ce_steps pin 2) unchanged except the new field; NEW `http2_rst_fail` (one-line-delta mutation via `mutate_spec`: Open RstStream retargeted off Closed -> `rst_goes_closed` step 2, ce_steps 2), NEW 48 `--assumes none` run of the unmodified spec (reuses the pass binary) -> `completes_after_setup` step 16, states 262/1464/15, ce_steps 16, NEW `can_passive_fail` (P2 band-entry counter update +4 -> +1 -> `passive_band` step 6, states 15/17/6, ce_steps 6). Each case now also runs `--counterexample-format json` and pins `ce_steps` + JSON label; per-backend FAIL table printed. New helpers: `mutate_spec`, `http2_rst_fail_src`, `can_passive_fail_src`, `run_json_mode`, `json_peak_rss_kb`, `json_ce_steps`; `FailPin.ce_steps` |
| `tests/benchmark_mc_test.rs` - `bench_mc_json_report_and_table` | extended | per spec x backend a JSON-report-mode run: `peak_rss_kb` consumed into the report entry + table column with the single order-of-magnitude sanity bound (0 <= rss < 1_000_000; values are informational), text-vs-JSON metric cross-check (D-P5-10 same-data contract); new `codegen_timing` report section from in-process `time_parse_codegen` over TIMED_SPECS (8 specs x both backends) + printed timing table. Clippy: `SpecRuns` type alias for the entries tuple |
| `tests/benchmark_mc_test.rs` - `bench_mc_threads_parity` | extended | gate workloads 5 -> 8 (six benchmark specs + all five FAIL variants, the 48 case carrying its `--assumes none` args through the byte-identity gate) |
| `src/codegen_mc.rs` lib tests | changed | D-P5-10 shape pins re-pinned with `peak_rss_kb` (normalized to 0 in comparisons) + `ce_steps` |
| `src/ref_mc.rs` lib tests | changed | two golden full-document pins re-pinned with the new fields |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, 448) | OK |
| `cargo test --test benchmark_mc_test` (5, ~15 s) | OK - all Pins matched on first run after measurement |
| `cargo generate-code-tests` + `cargo test-generated-code` (1601, 189 modules - regenerated after the spec-48 correction) | OK |
| `cargo clippy --all-targets` | OK for every file this task touched (zero warnings in `tests/benchmark_mc_test.rs`, `src/codegen_mc.rs`, `src/ref_mc.rs`); remaining warnings are the pre-task baseline in untouched files (prod_driver.rs, c_fsm.rs, mc_json.rs, test_cmd.rs, validator.rs, codegen_test.rs, codegen.rs) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- **Plan correction 6a** (real toolchain bug, found in action 1 while bringing spec 48 through the pipeline): pattern binding `n` shadowed C MC's unqualified effect counter - emitted outputs went to `out[payload value]` (out-of-bounds stack write for wide fields) and the counter never incremented, so the C engine silently missed every successor through emitting transitions: a memory-safety + backend-parity bug reachable from fully valid specs. Fixed with `__rfsb` mangling in both backends (reserved lists per generated-body site + shared `Env.binding_idents`). Regression coverage: spec 48 itself is pinned by the benchmark suite (104/340/15 both backends), backend parity and ref_mc_diff ground truth.
- **Plan correction 6b** (real toolchain bug, found in action 1 by spec 47): the test planner's `expect_progress = !transitions.is_empty()` over-approximation emitted a POSITIVE drive that fails at runtime for specs whose initial-state arms all reject the representative candidate input (spec 47: `w > 0` guard vs w = 0) - a generated test failing on a fully valid spec. Fixed with the exact `expect_progress_from_initial`; lib pin added.
- **Plan correction 6c** (spec modeling bug, found by the new action-3 FAIL variant): spec 48's absorbing `Error on Setup` arm contradicted its own `no_double_setup` invariant once Error became reachable under relaxed assumptions - the `--assumes none` run reported `no_double_setup` at step 6 instead of the intended `completes_after_setup`. Fixed by removing the arm (RESET-first convention, matching Stalled); pass behavior unchanged.
- **Task correction 6d** (design note, no code change): RFS priorities fire DESCENDING (SEMANTICS 3.4). The first draft of the 49 FAIL variant relied on a guard overlap and silently passed - the priority-3 arm preempts for counters 0..3. The final variant mutates the band-entry arm's `set` instead (+4 -> +1), which works under either reading; spec 49's disjoint guards make its P1/P2/P3 labels behaviorally correct as-is.
- Two D-P5-10 golden pins in `src/ref_mc.rs` were missed during action 2's re-pin sweep and only surfaced at the final full-suite run - fixed (see changed files).

## Other important findings/information
- Measured counts for 47/48/49 (both backends identical; these are the Pins): 434/21857/7 (10 monitor bits), 104/340/15 (11), 240/448/41 (7) - states/transitions/max_depth. All far below the ~150k budget; no abstraction shrinkage needed.
- FAIL-variant pins: `http2_rst_fail` -> property-violation / `rst_goes_closed` / step 2 / 34 states / ce_steps 2; 48 `--assumes none` -> property-violation (bounded-liveness monitors report online) / `completes_after_setup` / step 16 (trigger 1) / 262 states / ce_steps 16 - canonical CE: Setup, Data(1) x3, Nak x4 into Error, then absorbed until the countdown expires; `can_passive_fail` -> property-violation / `passive_band` / step 6 / 15 states / ce_steps 6.
- peak_rss_kb measured ~2-4 MB on this host for the largest benchmark (44: 26k states) - the 1 GB sanity bound is orders of magnitude away; the field is still informational per the Phase 4 discipline.
- The in-process parse/codegen timings are sub-millisecond to single-digit ms for all eight specs on this host (informational).
- Root main-test count moved 447 -> 448 (+1 net: action 2's shape-pin re-pins replaced existing tests; the new JSON-mode coverage lives inside existing benchmark tests). Final BUILD.md section-3 baseline totals remain task 9's per plan ownership.
