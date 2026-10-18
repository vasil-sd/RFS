# Phase 4, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_4_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | All work landed in existing files; the new generated runtime is produced by codegen, not stored in the repo. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/rust_mc.rs` | changed | Rewritten: `generate_model_checker` now computes `McPlan`, appends the generated exact-mode runtime (defaults consts, `all_inputs()`, `put_bits`/`config_bytes` from the plan layout, `mc_step_all` via rust_fsm, BFS engine `run_exact`) plus the Phase 3 fuzz runtime (renamed `main` -> `run_fuzz`) and a `--mode` dispatching `main`. |
| `src/backends/rust_fsm.rs` | changed | New `pub(crate) fn render_mc_step_all(module)` - renders the generated `mc_step_all` (every enabled transition's effect in firing order + on_error fallback) from the same IR/helpers as `Fsm::step`; enables both det (first effect) and nondet (all effects). |
| `src/codegen_mc.rs` | changed | Exact-mode shared concepts (D-P4-4/D-P4-7): `ExactStatus`, `FailReason` (+wire names), `exact_summary_line` (pinned format), human status text, exit codes 0/1/2, `EXACT_DEFAULT_MAX_STATES`/`EXACT_DEFAULT_TIMEOUT_SEC`; 2 new unit tests. |
| `src/codegen.rs` | changed | New `McMode { Exact, Fuzz }` (+FromStr); `BackendOptions` + `CodegenRequest::Generate` gain `mc_mode/mc_max_states/mc_nondet/mc_timeout_sec`; CLI parsing of `--mode/--max-states/--nondet/--timeout-sec` (mc only); common help lines. |
| `src/backends/rust.rs` | changed | `MC_INFO` usage/options updated for the new flags and both modes. |
| `src/main.rs` | changed | Passes the four new fields through to `BackendOptions`. |
| `src/mc_plan.rs` | changed | Offset bug fix: var slots now start at `state_bits`; `MonitorStateLayout.offset_bits` (absolute monitor base) assigned by `config_encoding`; task-1 test asserts corrected layout. |
| `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs` | changed | `..BackendOptions::default()` in initializers (mechanical). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_4_TASK_2.md` | changed | "Task corrections" section appended (6 entries: rust_fsm.rs addition, fuzz pinning of Phase 3 suites, struct-update ripple, pinned deadlock/timeout semantics, config-offset bug, clippy sandbox workaround). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs` | changed | 2 new tests: `mc_exact_flags_embedded_as_runtime_defaults` (each flag changes generated output; exact is default), `mc_flag_parsing_roundtrip`; `..` in one pattern. |
| `tests/backend_parity_test.rs`, `tests/monitor_integration_test.rs`, `tests/integration_example_test.rs` | changed | Phase 3 MC invocations pinned to `--mode fuzz`; `..BackendOptions::default()` initializers (mechanical). |

## Changed functionality
- `rfs codegen rust mc` output: the generated binary now defaults to exact mode; Phase 3 fuzz behavior is retained verbatim behind `--mode fuzz` (same-seed summaries verified unchanged).
- New CLI flags on `rfs codegen <backend> mc`: `--mode exact|fuzz`, `--max-states N`, `--nondet`, `--timeout-sec N` (embedded as runtime defaults; the generated binary also accepts them at runtime, overriding the defaults).

## Removed functionality
- None. (The old MC binary's implicit "always fuzz" default is superseded by explicit mode selection.)

## New functionality
- Exact reachable-state engine in generated Rust MC binaries:
  - level-synchronous BFS; config = (state index, packed vars, monitor transition states) bit-packed per the plan layout; visited set checked at enqueue; parent pointers (config id, input index, transition id).
  - canonical input enumeration from the plan (variants/fields declaration order, values ascending); abstracted fields take their representative value.
  - det mode: first enabled transition fires (identical to core `step`); `--nondet`: every enabled transition is a separate successor (verified to explore more states on an overlapping-transitions spec).
  - assumption pruning via the Phase 3 monitors at the correct sample index (depth+1); property violations stop BFS with the canonical first counterexample.
  - deadlock detection (no successor at all) with path counterexample; unexpected-input paths terminate and are counted (`unexpected terminations` + first occurrence).
  - pinned machine line `model-check summary: status=... states=N transitions=M max_depth=D[ reason=... label=... step=S]`; human lines PASS (complete)/FAIL/UNKNOWN (state limit reached); exit codes 0/1/2; stats incl. memory estimate `states * (CONFIG_TOTAL_BYTES + 32)`.
  - counterexample reconstruction replays the parent path on a fresh Fsm and reuses the Phase 3 trace print format.
- Shared exact-mode concepts in `codegen_mc.rs` for backend parity (task 3 mirrors them).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `mc_exact_flags_embedded_as_runtime_defaults` (tests/codegen_test.rs) | new | Each new flag changes generated output; exact is the default; engine + dispatch present. |
| `mc_flag_parsing_roundtrip` (tests/codegen_test.rs) | new | CLI parsing of all four flags + defaults. |
| `exact_summary_line_format`, `exact_status_wire_human_exit` (src/codegen_mc.rs) | new | Pin the D-P4-4 wire format, human text, exit codes. |
| Phase 3 MC suites (backend_parity, monitor_integration) | changed | Pinned to `--mode fuzz`; same-seed behavior verified unchanged. |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (184: lib 98, codegen_test 25, parity 3, monitor_integration 4, examples 3, training 4, library_comparison 4, integration_example 2, + rest) | OK |
| clippy via `RUSTC_WORKSPACE_WRAPPER` workaround (`cargo check --all-targets`) | OK - back to the 12-warning baseline (11 lib + 1 codegen_test), zero from new code |
| `cargo fmt --check` | OK |
| `cargo generate-code-tests` (171 modules) + `cargo test-generated-code` | OK - 513/513 with regenerated MC outputs |
| Manual: spec 37 exact | OK - PASS (complete), exit 0, pinned summary line |
| Manual: buggy spec 04 (emit removed) | OK - FAIL property-violation label=syn_ack step=1, Phase-3-format counterexample, exit 1 |
| Manual: spec 04 exact | OK - FAIL deadlock at Established (genuine sink state; correct per D-P4-5), exit 1 |
| Manual: `--max-states 1` | OK - UNKNOWN (state limit reached), exit 2 |
| Manual: overlapping-transitions spec det vs `--nondet` | OK - 4 vs 5 states (nondet explores both successors) |
| Manual: `--mode fuzz --bound 5 --runs 20 --seed 12345` on spec 04 | OK - Phase 3 summary format, behavior unchanged |

## Found problems/issues/mismatches
- Task 1 config-encoding offset bug (var slots at offset 0; monitors without absolute base) collapsed distinct configurations into one hash - found by the first exact-mode smoke run (states=2 on spec 04), fixed in `src/mc_plan.rs` with test updated.
- `cargo clippy --all-targets` is broken in this sandbox (pre-existing, reproduces on pristine tree): cargo's clippy-driver probe receives foreign error text on stdin. Workaround: `RUSTC_WORKSPACE_WRAPPER=<toolchain>/clippy-driver cargo check --all-targets`.
- Spec 04 (and other terminal-state demos) are genuine deadlocks under exact semantics - expected findings, not bugs; pass-complete demos use spec 37.

## Other important findings/information
- `mc_step_all` first effect == core `step_with_handler` outcome by construction (same IR, same helpers, same firing order); det mode therefore reuses one code path for both modes.
- Sample-0 assumption violations panic in exact mode too (identical to Phase 3 `Fsm::new`) - documented edge case.
- Task 3 (C mirror) must reuse: `codegen_mc` exact concepts, the summary-line format, exit codes, the memory-estimate formula, and the deadlock/timeout semantics pinned in the task-corrections table.
