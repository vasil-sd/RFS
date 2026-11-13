# Phase 14, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none - the split re-partitions the existing MC emission in `c/mc.rs`; no new file.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/backends/c/mc.rs` | changed | `generate_model_checker` no longer splices the inlined FSM core: it calls the shared `generate_fsm_pair` (monitors Always) to produce the referenced fsm pair, then assembles the mc SOURCE as line-1 + `#include "<mc header>"` + `#include "<fsm header>"` + stdio/stdlib + the `-Wunused-function` pragma + the engine (`shared_printers` + `exact_codegen_c` + `fuzz_runtime` + `MAIN_DISPATCH_C`) and the mc HEADER as line-1 + include guard `<MODULE>_MC_H` + `#include "<fsm header>"` + the run-entry-point declarations. Fills ALL FOUR slots (mc_header, mc_code, fsm_header, fsm_code) so the write loop applies take-or-emit to the fsm pair. `run_exact`/`run_fuzz` made non-static (they are the header's public API). |
| `src/codegen/backends/c/fsm.rs` | changed | Extracted `generate_fsm_pair(module, options, prop_mode)` from `generate_production` (shared by production and mc). Exposed the engine-facing API the separately compiled MC links against as EXTERNAL symbols: `trace_vars`, `rfs_commit`, `assume_monitors_step`, `prop_monitors_step` (definitions de-staticised in the source) and the label tables `ASSUME_LABELS`/`N_ASSUME_LABELS`/`PROPERTY_LABELS`/`N_PROPERTY_LABELS` (de-staticised, `RFS_UNUSED` dropped - an exported symbol is never -Wall-unused). Their prototypes/externs are emitted directly to the header (bypassing the legacy single buffer, like the public prototypes); `prop_monitors_step` is gated by `prop_emit` because it references prop-gated types. |
| `src/codegen/mod.rs` | changed | `BackendOptions` gains `mc_header_name: Option<String>` (resolved C mc-header name; None = standalone default `rfs_mc.h`). `primary_code()` re-prioritised to mc > test > fsm and its exactly-one assertion dropped: C `mc` now fills BOTH `fsm_code` (the referenced pair) and `mc_code` (its own), so the most derived artifact wins. |
| `src/cli/commands/codegen.rs` | changed | Factored `resolve_fsm_reference(...)` out of the write loop and shared it with the CLI stamping, so the name a generated source `#include`s always matches where the file lands. Stamping extended from C production to C `mc`: stamps `fsm_header_name`/`fsm_source_name` (via the take-or-emit reference resolution) and `mc_header_name` (`--out-mc-header` > BASE-derived). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-3 complete, tasks 4-11 pending. |

(User-facing / dev architecture docs for the mc pair model are task 8's scope - the phase plan defers the doc re-sync there.)

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/common/prod_driver.rs` | changed | Added `compile_c_object(src, obj, include_dir)` (gcc -Wall -c with the zero-warning bar + `-I`) and `build_c_mc_binary(out, dir, bin)` (writes the four-file set under the standalone default names, compiles both sources to objects, links) - the shared object+link builder every migrated MC leg uses. |

## Changed functionality
- C `mc` codegen: the single self-contained `.c` (FSM inlined) is now a header/source PAIR that references the separately generated fsm pair. The mc source includes its own header and the fsm header; the FSM implementation exists exactly once, in the fsm source. No MC artifact contains a second copy of `fsm_step`.
- The C production fsm header's public surface grew: it now also declares the engine-facing API (`trace_vars`, `rfs_commit`, `assume_monitors_step`, `prop_monitors_step`, the label tables) as external symbols, so a separately compiled consumer (the MC) can link against the fsm object. A standalone production artifact is unaffected in behaviour (the symbols were always defined; they are now exported rather than source-local statics).
- `primary_code()` for C `mc` returns the mc source (not the fsm source); the command's own primary artifact wins over the co-emitted referenced pair.

## Removed functionality
- The inlined `generate_core` splice inside the C MC executable (the FSM core is no longer copied into the mc translation unit).
- The single-file C MC compile shape in every test leg (replaced by object+link).
- The now-dead `compile_c` helpers in `benchmark_mc_test.rs` and `ref_mc_diff_test.rs` (their only C caller migrated to `build_c_mc_binary`).

## New functionality
- `rfs codegen c mc <spec> --out BASE` writes FOUR files: `BASE-fsm.h`, `BASE-fsm.c`, `BASE-mc.h`, `BASE-mc.c`. The fsm pair is co-emitted via take-or-emit (an existing pair is referenced without rewrite; a partial pair is a usage error; a stale-hash pair warns per D14-4).
- `--fsm-file` / `--fsm-file-header`+`--fsm-file-source` name the referenced fsm pair for `mc`; `--out-mc-header`/`--out-mc-code` name the mc pair. The generated sources include their headers by the RESOLVED names.
- `build_c_mc_binary` / `compile_c_object`: the shared test infrastructure for building a C MC binary via object+link under the `gcc -Wall` zero-warning bar.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/common/prod_driver.rs::build_c_mc_binary`, `::compile_c_object` | new | Shared object+link builder + zero-warning `-c` compile for the C MC pair. |
| `tests/backend_parity_test.rs` (`mc_binary`, `liv_mc_binary`, `mc_json_binary`, `mc_json_binary_for`) | changed | C legs build via `build_c_mc_binary`; the `generate_*` helpers return the full `CodegenOutput`. All Rust/C report-parity assertions pass UNMODIFIED (byte-identical MC reports). |
| `tests/benchmark_mc_test.rs` (`bench_binary`) | changed | Same object+link migration; dead `compile_c` removed. |
| `tests/c_compiler_agnostic_test.rs` (Leg 1 mc variant, Leg 3 forced-violation) | changed | Added `generate_full` + `build_mc_pair(cc, ...)`: compiles BOTH sources under gcc AND clang at -O2 -Wall and links; the cross-compiler byte-parity comparison now runs on the LINKED binaries. |
| `tests/codegen_test.rs` (`mc_enum_input_field_domain_qualified_both_backends`) | changed | C engine built via `build_c_mc_binary`; the Rust/C byte-identity assertion is unchanged. |
| `tests/info_format_test.rs` (`ce_mc_binary`, `artifact_header_generated_output_matches_documented_format`) | changed | `ce_mc_binary` C leg migrated; the slot-shape test now expects C mc to fill 4 slots and checks the mc-header AND fsm-header line-1 stamps. |
| `tests/monitor_integration_test.rs` (`mc_buggy_run`, `mc_good_run`) | changed | C legs migrated; added `generate_full`. The C production leg (already a pair from task 2) is untouched. |
| `tests/ref_mc_diff_test.rs` (`mc_json_binary`) | changed | Same migration; dead `compile_c` removed. |
| `tests/toolchain_fuzz_test.rs` (slow-suite mc kind, `c_mc_trigger_step_zero_report_no_crash`) | changed | C mc built via `build_c_mc_binary` into a per-tag/per-test subdir (avoids fixed-name collisions across parallel test threads). |
| `tests/wide_ops_parity_test.rs` (MC parity `run` closure) | changed | C engine migrated to object+link; the pass-complete summary parity assertion is unchanged. |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, all 32 suites) | OK - 541 passed / 0 failed |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets` | OK - at the pre-phase baseline (12 lib warnings, zero new; removed 2 dead `compile_c` + 6 unnecessary `to_string` the migration would otherwise have left) |

## Found problems/issues/mismatches
- The task assumed the MC engine only needed the fsm HEADER. In reality it is coupled to FSM internals that were source-local statics (`rfs_commit`, `trace_vars`, `assume_monitors_step`, `prop_monitors_step`, `ASSUME_LABELS`/`N_ASSUME_LABELS`) - visible only because the FSM core used to be inlined into the same translation unit. The first object+link build failed with implicit-declaration / undeclared-identifier errors until these were exposed as external symbols in the fsm header.
- `prop_monitors_step` references prop-gated types (`prop_monitors_t`, `rfs_property_violation_t`), so its header prototype had to be emitted under the same `#ifdef RFS_PROPERTY_MONITORS` guard (`prop_emit`) - an unconditional prototype broke monitors-OFF production headers with unknown-type errors.
- `primary_code()`'s "exactly one of {fsm_code, test_code, mc_code}" assertion panicked for C mc (which fills both fsm_code and mc_code); re-prioritised to mc > test > fsm.
- The MC source lost the `-Wunused-function` pragma it previously inherited from the inlined FSM core; without re-adding it, non-liveness specs warned on the unconditionally emitted `liv_*` statics. The pragma was moved into the mc source.

## Other important findings/information
- The fsm header's public API is now genuinely shared between standalone production users and the MC: exposing the engine-facing helpers as external symbols (rather than duplicating them) is what makes a true object+link split possible. Task 4 (C `test`) will rely on the same exposed API when it splits the test artifact.
- `build_c_mc_binary` writes the four files under FIXED standalone-default names (`rfs_fsm.h/c`, `rfs_mc.h/c`) and compiles with `-I dir`; callers that share a scratch directory across parallel test threads must either hold a lock throughout the build (the `cached_binary`/`ce_mc_binary` helpers do) or pass a per-build subdir (toolchain_fuzz_test does).
- The SMT-bounded MC mode is unaffected: it returns early with a standalone libz3 binary that references no fsm artifact, so its single-file compile legs were left as-is.
