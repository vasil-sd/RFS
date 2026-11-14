# Phase 14, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_5.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none - the change re-partitions how the Rust mc/test artifacts reference the FSM and how the Rust pipelines compile it; no new file.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/backends/rust/fsm.rs` | changed | `Gen` now renders the fsm-core items and the test-section into SEPARATE strings (plus the line-1 header and the inner-attribute prelude, split out so an including crate can supply its own). New `render_fsm_parts` returns those parts; `generate_core` keeps its signature and fuses them in emission order - byte-identical to the pre-task output, preserving the aggregate-crate (1683-test) contract. New `generate_test_reference` returns the BARE referenced fsm file (header + core, NO inner attrs - include! cannot bring `#![...]` into an including crate) in the fsm-code slot and the test section prefixed with its own prelude + `include!("<fsm file>");` in the test-code slot. |
| `src/codegen/backends/rust/mc.rs` | changed | The mc executable no longer inlines the FSM: it carries its own line-1 header + inner-attribute prelude, then `include!("<fsm file>")` (the resolved reference name from `BackendOptions`), then the engine + main. Fills the mc slot and the fsm slot (bare) so the write loop applies take-or-emit. |
| `src/codegen/backends/rust/test.rs` | changed | `generate_unit_tests` (the CLI path) uses `generate_test_reference`: the test artifact is include! + test section only, with the bare fsm file in the fsm-code slot. The `fuzz_target.rs` skeleton KEEPS its crate-wiring model (lib root = production source); its wiring comments are rewritten to the new naming (`BASE-fsm.rs` co-emitted by `rfs codegen rust test <spec> --out BASE`). |
| `src/cli/commands/codegen.rs` | changed | Stamps the resolved fsm reference name into `BackendOptions.fsm_source_name` for the Rust mc/test commands (take-or-emit), so the include! matches where the write loop co-emits / takes the file. Header flags stay rejected on Rust (only `--fsm-file` applies). |
| `src/cli/commands/test.rs` | changed | Rust unit pipeline: writes the fsm .rs + test .rs into the temp dir under their standalone default names and compiles the TEST file (`include!` resolves relative to it). `--print-script` Rust branch: emits a heredoc for the referenced fsm file alongside the test heredoc (the script previously wrote only the test source, so its include! would not resolve) and adds `rfs_fsm.rs` to the cleanup line. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-5 complete, tasks 6-11 pending. |

(User-facing / dev architecture docs for the include! reference model are task 8's scope - the phase plan defers the doc re-sync there.)

## Other file changes

| Name | Operation | Description |
|---|---|
| `tests/codegen_test.rs` | changed | Rust mc/test legs re-pointed at the fsm-code slot (the monitor types / ProtocolFsm impl live in the referenced file, not the primary artifact). Added three tests: `cli_rust_mc_writes_fsm_and_references_it` (two-file `--out arq` output; include! present; fsm body absent from the mc; compiles with plain rustc and runs), `cli_rust_test_writes_three_files_and_references_fsm` (three files; test is include! + tests only), `no_rust_mc_test_duplicates_the_fsm_body` (the grep bar: every Rust mc/test command x every valid example references via include! and does not duplicate the Fsm impl/struct). Added `cli_rust_fsm_reference_take_or_emit` (Rust-side take-or-emit: matching hash taken silently, foreign hash warns-but-used-and-untouched, absent co-emitted). |
| `tests/backend_parity_test.rs`, `tests/benchmark_mc_test.rs`, `tests/ref_mc_sweep_test.rs`, `tests/ref_mc_diff_test.rs`, `tests/ref_mc_diag_test.rs` | changed | Rust mc/test compile legs migrated to the two-file layout. Because these suites share ONE scratch dir across many specs (and some run parallel workers), a fixed `rfs_fsm.rs` would collide - each now writes a per-spec/per-tag fsm file and rewrites the include! to point at it (mirrors the C per-tag subdirs). |
| `tests/toolchain_fuzz_test.rs` | changed | The Rust mc/test compile legs write a per-tag/kind fsm file + rewrite the include! (all test functions share one scratch dir and run on parallel threads); the trigger-step-0 leg writes its referenced fsm file alongside. |
| `tests/wide_ops_parity_test.rs`, `tests/liveness_mc_test.rs`, `tests/monitor_integration_test.rs`, `tests/info_format_test.rs` | changed | Rust mc compile legs write the referenced fsm file alongside (per-process / per-name dirs - no collision). |
| `tests/utils_test.rs` | changed | The `--print-script` pin regenerated for the new 5-step Rust script (test heredoc + fsm heredoc + compile + run + cleanup); its cross-check now normalizes the include! reference name (the script uses the standalone default `rfs_fsm.rs`, the `--out gen` cross-check uses `gen-fsm.rs`) and verifies BOTH heredoc bodies against their codegen artifacts. The mc-ref-agreement leg writes a per-spec fsm file + rewrites the include!. |

## Changed functionality
- The Rust `mc` and `test` artifacts reference the separately generated fsm file via `include!("<fsm file>")` instead of inlining a duplicate of the FSM code. The FSM implementation exists exactly once, in the referenced fsm file.
- `rfs test --backend rust`: the unit pipeline compiles the two-file layout (fsm .rs + test .rs); the emitted `--print-script` now writes the referenced fsm file so the script's include! resolves.

## Removed functionality
- The inlined FSM body inside the Rust mc executable and the fused FSM+tests inside the Rust CLI test artifact (both now reference the fsm file via include!).

## New functionality
- `rfs codegen rust mc <spec> --out BASE` writes `BASE-fsm.rs` + `BASE-mc.rs`; the mc includes the fsm by name and compiles with plain `rustc --edition 2021`.
- `rfs codegen rust test <spec> --out BASE` writes three files: `BASE-fsm.rs`, `BASE-test.rs` (include! + tests only), `BASE-fuzz-target.rs`.
- The fused `generate_core` contract is preserved byte-for-byte - the aggregate-crate (1683-test) generated-code suite is unchanged by construction.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `tests/codegen_test.rs::cli_rust_mc_writes_fsm_and_references_it` | new | CLI: `rfs codegen rust mc --out arq` writes arq-fsm.rs + arq-mc.rs; include! present, fsm body absent from the mc; compiles with plain rustc and runs. |
| `tests/codegen_test.rs::cli_rust_test_writes_three_files_and_references_fsm` | new | CLI: `rfs codegen rust test --out arq` writes three files; the test file is include! + tests only. |
| `tests/codegen_test.rs::no_rust_mc_test_duplicates_the_fsm_body` | new | The grep bar: every Rust mc/test command x every valid example references via include! and does not duplicate the Fsm impl/struct. |
| `tests/codegen_test.rs::cli_rust_fsm_reference_take_or_emit` | new | Rust-side take-or-emit: matching hash taken silently, foreign hash warns-but-used-and-untouched, absent co-emitted. |
| `tests/codegen_test.rs` (Rust mc/test legs) | changed | Re-pointed at the fsm-code slot; two-file compile/run. |
| `tests/{backend_parity,benchmark_mc,ref_mc_sweep,ref_mc_diff,ref_mc_diag}_test.rs` (Rust legs) | changed | Per-spec/per-tag fsm file + include! rewrite (shared-dir collision avoidance). |
| `tests/{toolchain_fuzz,wide_ops_parity,liveness_mc,monitor_integration,info_format}_test.rs` (Rust legs) | changed | Write the referenced fsm file alongside. |
| `tests/utils_test.rs` (`--print-script` pin + mc-ref leg) | changed | Regenerated 5-step pin with normalized include! cross-check; per-spec fsm file for the mc-ref leg. |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, all 32 suites) | OK - 0 failed (547 passed; codegen_test 59 passed incl. the four new tests) |
| `cargo test-generated-code` | OK - 1683 passed / 0 failed (fused `generate_core` contract preserved - byte-identical aggregate output) |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets` | OK - at baseline (40 warning sites, IDENTICAL set to HEAD - task 5 adds ZERO new; the three sites that moved line number are pre-existing warnings relocated by this task's insertions) |

## Found problems/issues/mismatches
- A prior session had ALREADY implemented Actions 5.1-5.4 (the `render_fsm_parts` / `generate_test_reference` split, the mc include!, the test-artifact re-render, the unit-pipeline two-file write, and most of the test-leg migrations) - all uncommitted and not status-marked. The remaining gaps found on arrival were: (a) the `--print-script` Rust branch never wrote the referenced fsm file, so the emitted script's include! would not resolve in real use; (b) several shared-dir / parallel-worker test suites (ref_mc_sweep, backend_parity, benchmark_mc, ref_mc_diff, ref_mc_diag, toolchain_fuzz) all wrote the default `rfs_fsm.rs` into ONE dir and collided across specs - fixed with per-spec/per-tag fsm names + include! rewrite; (c) a handful of single-site Rust mc compile legs (wide_ops_parity, the trigger-step-0 leg in toolchain_fuzz, the mc-ref-agreement leg in utils_test) wrote the mc source but not its referenced fsm file; (d) the `--print-script` pin's cross-check assumed `rfs test` and `rfs codegen rust test --out gen` produce identical output, which no longer holds because their include! reference names differ by design - fixed by normalizing that one line and verifying both heredoc bodies.
- The Rust production artifact is flat crate-root items with no `mod` declarations and no inner `include!` (verified against a property-bearing spec), so inlining it at the mc/test crate root via include! cannot collide - the precondition the task file asked to re-check before relying on include! holds.

## Other important findings/information
- The take-or-emit + stale-hash logic lives in the shared CLI write loop (`resolve_fsm_reference` / `check_stale`), so the C test from task 1 already exercised it; this task adds an explicit Rust-side test (`cli_rust_fsm_reference_take_or_emit`) to guard the Rust stamping path against future divergence.
- The grep bar `no_rust_mc_test_duplicates_the_fsm_body` is the standing regression for the no-duplication invariant: any future generator that re-inlines the Fsm impl/struct into a Rust mc/test artifact fails it across every command and example.
