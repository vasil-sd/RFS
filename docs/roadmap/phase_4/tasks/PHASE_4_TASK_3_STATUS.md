# Phase 4, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_4_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | All work landed in existing files; the new generated runtime is produced by codegen, not stored in the repo. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/c_mc.rs` | changed | Rewritten: `generate_model_checker` now computes `McPlan`, emits shared printers (`print_vars`/`print_trace`, moved ahead of both runtimes), the generated exact-mode runtime (defaults, `all_inputs()` from the plan abstraction, `put_bits`/`config_bytes` from the plan layout, `mc_step_all` via c_fsm, BFS engine `run_exact` with sorted-vector visited set + binary search, parent pointers, replay, reports), the Phase 3 fuzz runtime (renamed `main` -> `run_fuzz`), and a `--mode` dispatching `main`. |
| `src/backends/c_fsm.rs` | changed | New `pub(crate) fn render_mc_step_all_c(module)` + private `mc_pattern_cond`: renders generated `mc_step_all(state, vars, in, out)` (every enabled transition's effect in firing order + on_error fallback as `transition_id = -1`) from the same IR/helpers as `fsm_step`. Also: zero-initialization of the outputs array in `transition_block_c`/on_error rendering (`= {0}`) to make generated C `gcc -Wall` warning-free (pre-existing `-Wmaybe-uninitialized`). |
| `src/backends/c.rs` | changed | `MC_INFO` usage/options updated for the new flags and both modes. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_4_TASK_3.md` | changed | "Task corrections" section appended (4 entries: c_fsm.rs addition, pre-existing gcc -Wall fix, parity-suite bar note, shared-printers move). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| (none) | - | No test files needed changes: the Phase 3 suites already pin `--mode fuzz` (task 2), and task 3 adds no new Rust-side tests beyond what the parity spot check covers manually. |

## Changed functionality
- `rfs codegen c mc` output: the generated binary now defaults to exact mode; Phase 3 fuzz behavior is retained verbatim behind `--mode fuzz` (same-seed summaries verified unchanged by the existing parity suites).
- Generated C core text: emit-less transitions/on_error now zero-initialize their outputs array (warning fix, no behavior change).

## Removed functionality
- None. (The old MC binary's implicit "always fuzz" default is superseded by explicit mode selection.)

## New functionality
- Exact reachable-state engine in generated C MC binaries (mirror of task 2):
  - level-synchronous BFS; config = (state, packed vars, monitor transition states) bit-packed per the plan layout with fixed-width types (`uint64_t`/`uint32_t`) so bytes are identical to Rust's.
  - visited set = **sorted vector of config byte strings + binary search** (documented simple choice; behavior must not depend on the container - task 10 may switch to open addressing with benchmark data).
  - canonical input enumeration from the plan (variants/fields declaration order, values ascending; `uint64_t` loop counters so bit<8>..bit<64> domains cannot wrap); abstracted fields take their representative value.
  - det mode: first effect of `mc_step_all` (identical to core `fsm_step`); `--nondet`: every enabled transition is a separate successor (verified: overlapping-transitions spec explores 5 states vs 4 in det, same as Rust).
  - assumption pruning via the Phase 3 C monitors at sample index depth+1; property violations stop BFS with the canonical first counterexample.
  - deadlock detection (no successor at all) with path counterexample; unexpected inputs terminate their path and are counted.
  - pinned machine line `model-check summary: status=... states=N transitions=M max_depth=D[ reason=... label=... step=S]`; human lines PASS (complete)/FAIL/UNKNOWN (state limit reached); exit codes 0/1/2; stats incl. memory estimate with the SAME formula as Rust (`states * (CONFIG_TOTAL_BYTES + 32)`).
  - counterexample reconstruction replays the parent path on a fresh `fsm_t` via `rfs_commit` and reuses the Phase 3 C trace print format.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (no new Rust tests) | - | Verification was done via the manual parity spot check + gcc -Wall scan (recorded below); the existing suites (184) all pass unchanged, including the fuzz-parity suites that now exercise `--mode fuzz` on both backends. |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (184: lib 98, codegen_test 25, parity 3, monitor_integration 4, examples 3, training 4, library_comparison 4, integration_example 2, + rest) | OK |
| clippy via `RUSTC_WORKSPACE_WRAPPER` workaround (`cargo check --all-targets`) | OK - back to the 12-warning baseline (11 lib + 1 codegen_test), zero from new code |
| `cargo fmt --check` | OK |
| `gcc -Wall` on all 36 generated C MC outputs | OK - **zero warnings** (after fixing the pre-existing core `-Wmaybe-uninitialized`) |
| Parity spot check (exact mode, all 9 property-bearing examples): status + summary-line fields + counterexample fields Rust vs C | OK - **9/9 identical**, exit codes match (0/1/2 verified separately) |
| `cargo generate-code-tests` (171 modules) + `cargo test-generated-code` | OK - 513/513 with regenerated outputs |
| Manual: overlapping-transitions spec, C det vs `--nondet` | OK - 4 vs 5 states, matching Rust's 4 vs 5 |
| Manual: C `--max-states 1` / fail / pass exit codes | OK - 2 / 1 / 0 |
| Fuzz mode same-seed (existing parity suites, both backends) | OK - unchanged |

## Found problems/issues/mismatches
- Pre-existing `-Wmaybe-uninitialized` warnings in generated C core (emit-less transitions pass an uninitialized outputs array to `rfs_commit`) - surfaced by the task's zero-warning requirement; reproduced on the pristine tree; fixed with `= {0}` initialization.
- First parity run failed to compile: a format-string `{0}` was interpreted as a positional placeholder when zero-initializing (`= 0` instead of `= {0}`) - escaped as `{{0}}`.
- The sorted-vector visited set showed no slowness on any example (largest exact run: 258 states, instant) - no note needed for task 10 beyond the benchmark data itself.

## Other important findings/information
- C state/enum values are declaration-order integers, matching Rust's match indices; config bytes are therefore identical by construction (verified by the parity spot check).
- `mc_step_all` first effect == core `fsm_step` outcome by construction (same IR, same helpers, same firing order); det mode reuses one code path for both modes.
- Sample-0 assumption violations abort in C exact mode too (identical to Phase 3 `fsm_new`) - documented edge case.
- Task 4 (parity suite migration) can now migrate the suites to exact mode: both backends produce byte-identical machine lines and matching counterexample fields on all property-bearing examples.
