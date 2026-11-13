# Phase 14, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/mod.rs` | changed | `CodegenOutput` restructured from (`code` + `extra_files`) to the six named artifact slots (`fsm_header`, `fsm_code`, `mc_header`, `mc_code`, `test_code`, `fuzz_target`, each `Option<String>`) + unchanged `warnings`; `ExtraFile` deleted; `primary_code()` helper for library consumers. `CodegenRequest::Generate` extended with the flag family: `base` (raw `--out` value, `Some("-")` = stdout marker), six per-artifact output names, three fsm-reference forms (`fsm_file`, `fsm_file_header`, `fsm_file_source`). `parse_codegen_args` implements D14-1..D14-3: per-command flag subsets, header flags rejected on Rust, `--fsm-file*` rejected on production, C form-2 partial-pair usage error. `#[allow(clippy::large_enum_variant)]` on `CodegenRequest` with justification (Generate grew to >=359 bytes from the ten new `Option<String>` fields; boxing would ripple through ~18 construction/match sites for a once-per-process CLI request). |
| `src/cli/commands/codegen.rs` | changed | New `OutputArgs` struct. Write loop rewritten per D14-2/D14-3/D14-4: BASE := `--out` value or spec-file stem; per-artifact default-name table (`BASE-fsm.h/.c`, `BASE-mc.h/.c`, `BASE-test.c`, `BASE-fuzz-target.c`; Rust `.rs` equivalents) with per-artifact flag overrides; fsm-reference resolution (explicit flags > C form-1 STEM expansion to `STEM.h`+`STEM.c` with NO `-fsm` suffix > BASE-derived); take-or-emit for referenced fsm files (C form 1: BOTH must exist, partial pair = usage error exit 1, no overwrite); D14-4 line-1 `spec sha256:<16hex>` mismatch on a taken file = stderr warning, still referenced; fresh writes with `create_dir_all` + `wrote <path>` per file; `--out -` prints the single filled slot, refused (exit 1) when more than one slot would be written. |
| `src/cli/commands/test.rs` | changed | The two `extra_files` reads re-pointed at the `fuzz_target` slot; no pipeline behaviour change. |
| `src/codegen/backends/c/fsm.rs` | changed | `generate_production` returns content in the `fsm_code` slot only; emitted bytes unchanged. |
| `src/codegen/backends/c/test.rs` | changed | `generate_unit_tests`: test content -> `test_code`, fuzz skeleton -> `fuzz_target` (was `extra_files`); bytes unchanged. |
| `src/codegen/backends/c/mc.rs` | changed | `generate_model_checker`: content -> `mc_code`; bytes unchanged. |
| `src/codegen/backends/rust/fsm.rs` | changed | `generate_production`: content -> `fsm_code`; bytes unchanged. |
| `src/codegen/backends/rust/test.rs` | changed | `generate_unit_tests`: test content -> `test_code`, fuzz skeleton -> `fuzz_target`; bytes unchanged. |
| `src/codegen/backends/rust/mc.rs` | changed | `generate_model_checker`: content -> `mc_code`; bytes unchanged. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/dev/codegen_core.md` | changed | "Output" paragraph rewritten for the slot model (named slots + write-loop naming resolution + stdout refusal); the old `ExtraFile` description would have been factually wrong after this task. User-facing docs and help text stay untouched (tasks 6-8). |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row updated: task files written, task 1 complete. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/c_sync_handler/Makefile` | changed | `--out $@` -> `--out-fsm-code $@` (old flag-as-file-name semantics no longer exist). The deeper .o-rule restructure is task 9. |
| `crates/examples/fuzz_c/Makefile` | changed | production leg -> `--out-fsm-code`; test leg -> `--out-test-code` + `--out-fuzz-target-code` (the skeleton was previously the unnamed extra file next to `--out`). |

## Changed functionality

- `rfs codegen <backend> <command> <input>` output handling:
  - `--out FILE` no longer means "write the single artifact to exactly this path": it is now the BASE name all default artifact names derive from (D14-3), or `-` for stdout mode.
  - The old refusal of any command with extra files in stdout mode becomes a precise rule: `--out -` is refused only when more than one slot would be written.
  - Fsm-reference flags (`--fsm-file`, `--fsm-file-header/--fsm-file-source`) take-or-emit: an existing file on disk is referenced instead of regenerated; a taken file whose line-1 spec hash differs from the current spec's hash produces a stderr warning (D14-4) but is still referenced.
  - A C form-1 reference where only one of `STEM.h`/`STEM.c` exists is a usage error (exit 1, nothing overwritten).
- Library API: `CodegenOutput.code` / `.extra_files` replaced by the six named slots + `primary_code()`; all library consumers re-pointed.

## Removed functionality

- `ExtraFile` struct and `CodegenOutput.extra_files` / `.code` fields (deleted from `src/`; only pre-task doc mentions remain, owned by tasks 7-8).
- The `--out FILE`-as-exact-file-name CLI semantics.

## New functionality

- Named artifact-slot output model: a command fills exactly the slots its artifacts occupy; "which artifacts does this command produce" is now a property of the filled slots.
- Full output-flag family with per-command subsets (D14-1): `--out-fsm-code`, `--out-fsm-header`, `--out-mc-code`, `--out-mc-header`, `--out-test-code`, `--out-fuzz-target-code`; header flags rejected on the Rust backend; fsm-reference flags rejected on production.
- BASE resolution: `--out` value, else the spec-file stem (extension-agnostic, no directory component); artifacts land in CWD, `--out DIR/BASE` under DIR/; per-artifact flags override BASE-derived names.
- Fsm-reference flag forms (D14-2): Rust `--fsm-file NAME[.rs]` (`.rs` appended when omitted); C form 1 `--fsm-file STEM` expanding to `STEM.h`+`STEM.c` with NO `-fsm` suffix; C form 2 `--fsm-file-header FILE.h --fsm-file-source FILE.c` (both required together).
- `--out -` stdout mode for single-artifact results.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs::cli_parses_output_flag_family` | new | Parse-level coverage of the flag family: spec-stem default naming, per-artifact overrides, `--out -` marker, header-flag rejection on Rust, fsm-reference rejection on production, C form-2 partial-pair usage error. |
| `tests/codegen_test.rs::cli_fsm_reference_take_or_emit_rules` | new | Take-or-emit behaviour: existing file taken (not overwritten), missing file emitted, C form-1 partial pair = exit 1 with no overwrite, D14-4 hash-mismatch warning on a taken file. |
| `tests/codegen_test.rs` (CLI-parsing tests) | changed | Migrated to the new request shapes (`base`/per-artifact fields replace `out`). |
| `tests/codegen_test.rs` (compile legs) | changed | Fixed output names (e.g. `rfs_generated_fsm.c`) now produced via explicit per-artifact flags under the new naming. |
| `tests/utils_test.rs` | changed | Codegen CLI-parsing legs migrated to the new flag family. |
| `tests/common/prod_driver.rs`, `tests/backend_parity_test.rs`, `tests/benchmark_mc_test.rs`, `tests/c_compiler_agnostic_test.rs`, `tests/endpoint_protocol_test.rs`, `tests/info_format_test.rs`, `tests/liveness_mc_test.rs`, `tests/monitor_integration_test.rs`, `tests/ref_mc_diag_test.rs`, `tests/ref_mc_diff_test.rs`, `tests/ref_mc_sweep_test.rs`, `tests/smt_bounded_test.rs`, `tests/toolchain_fuzz_test.rs`, `tests/wide_ops_parity_test.rs`, `crates/rfs-adapters/tests/nostd_minimal.rs` | changed | Library-consumer migration: `.code` reads -> `primary_code()` / slot reads; no behaviour change. |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (540) | OK |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets` | OK - at the pre-phase baseline (12 lib warnings, zero new; the task-1 field additions had tripped `large_enum_variant`, resolved with a justified allow rather than boxing) |

Note: the first full-suite run in this environment failed one unrelated test
(`diagram_render_parity_test::parity_external_mmd_pinned_subset`) because
mmdc's chrome-headless-shell binary was missing from the sandbox; installing
`chrome-headless-shell@156.0.8078.47` into the puppeteer cache path made the
suite fully green. No RFS code is involved in that test.
