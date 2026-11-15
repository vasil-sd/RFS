# Phase 14 - Implementation status

> Written once when all exit criteria of `PHASE_14.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|

(none - every change is inside an existing file.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/mod.rs` | changed | Slot-model output contract: `CodegenOutput`'s six named artifact slots + `ArtifactSlot` (default names, filled_slots/slot_content/primary_code/primary); `BackendOptions` gains the resolved-name fields; the flag family (`--out BASE`, per-artifact flags, fsm-reference flags, `--out -`) parsed with command-scoped rejections; `ExtraFile` removed. |
| `src/cli/commands/codegen.rs` | changed | The write loop: `write_artifacts` (BASE := --out or spec stem, take-or-emit for referenced fsm slots, D14-4 stale-hash warning, stdout mode), `resolve_fsm_reference`, `check_stale`, `write_fresh`, `OutputArgs`. |
| `src/codegen/backends/c/fsm.rs` | changed | Header/source pair generation: `Piece` routing in `Gen::emit`, `generate_fsm_pair` (include guard, dual line-1 headers, engine-facing externs hoisted to the header). |
| `src/codegen/backends/c/mc.rs` | changed | The mc artifact is a header/source pair referencing the separately generated fsm pair; no inlined FSM core. |
| `src/codegen/backends/c/test.rs` | changed | The test source references the fsm through its header; the fuzz skeleton is a named slot (header-referencing). |
| `src/codegen/backends/rust/fsm.rs` | changed | Fused-vs-referenced rendering split: `FsmParts` + `render_fsm_parts` (single rendering path), `generate_test_reference` (D14-5 include! artifact); the fused `generate_core` contract preserved byte-for-byte. |
| `src/codegen/backends/rust/mc.rs` | changed | The mc artifact is its own header + prelude + `include!("<fsm file>")` + engine; the bare fsm file in the fsm-code slot. |
| `src/codegen/backends/rust/test.rs` | changed | The test artifact is include! + test section only; the fuzz skeleton in its named slot. |
| `src/cli/commands/test.rs` | changed | `rfs test` pipelines over the new layout: C unit = object+link, C fuzz = both sources compiled together, Rust = two-file include!; `build_fuzz_context` reads the slots of the test codegen output (no second production dispatch). |
| `src/cli/args.rs` | changed | Help texts reference only the `rfs` binary; codegen help rewritten to the new CLI surface (23-row examples table). |
| `crates/rfs-build/src/lib.rs` | changed | Pulled-forward fix (task 7): per-slot output flags instead of the retired `--out <exact file>`; one `cargo:include` per generated file. |
| `crates/examples/c_sync_handler/{Makefile,src/handler.c,README.md}` | changed | Header-only include; explicit object files (arq_fsm.o + handler.o) + link; scenario output unchanged. |
| `crates/examples/fuzz_c/{Makefile,README.md}` | changed | New-flag generation (no delete-dance); clang builds the fsm source + skeleton together. |
| `crates/rfs-adapters/tests/nostd_minimal.rs` | changed | Artifact-header pins over the new slot accessors. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `src/codegen/CODEGEN.md` | changed | The artifact model: slot model, flag family, BASE resolution, take-or-emit + partial-pair error + D14-4 warning, `--out -`; no `extra_files` mention remains. |
| `src/codegen/backends/C.md` | changed | Header/source split model (content rules, include guard, dual line-1 headers, pragma placement) + the header-reference rule. |
| `src/codegen/backends/RUST.md` | changed | The include! reference mechanism (D14-5), the fused-vs-referenced rendering split, the preserved `generate_core` aggregate contract. |
| `docs/dev/{cli_commands,backend_c,backend_rust,DEPENDENCIES,workspace_crates}.md` | changed | Pointer/reference re-syncing to the write loop and slot model (via rfs-dev sync-fix). |
| `docs/llm/{SRC_STRUCTURE,TEST_STRUCTURE,codegen_core,TASK_MAP}.md` | changed | Inventory rows for the new fns/types, Depends-edge updates, ExtraFile removals; TEST budget 1309->1325. |
| `KB.md`, `BUILD.md`, `README.md`, `START.md`, `docs/user/*` (six files) | changed | Every `cargo run -- <command>` example -> `rfs <command>`; codegen walkthroughs rewritten to the new CLI and verified by execution. |
| `training/{frontend_cli,codegen_rust,design,project_tooling}/rfs_training.jsonl` | changed | 14 entries prefix-swapped + 7 entries audited to the new output surface (task 10). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `all_rfs_training.jsonl` | regenerated | Aggregated SFT set - 267 lines, unchanged from baseline (no entries added/removed). |
| `docs/roadmap/phase_14/*` | new | Phase plan + 11 task files + 10 task status files + this file. |
| `TASKS.md`, `docs/roadmap/CURRENT_STATE.md` | changed | Per-task entries; phase row -> COMPLETE at close-out. |

## Changed functionality
- Codegen output model: a command's artifacts are its filled named slots (fsm-header/fsm-code/mc-header/mc-code/test-code/fuzz-target) instead of one primary string plus unnamed extras.
- CLI output surface: `--out` takes a BASE name (or `-` for stdout); per-artifact flags override individual names; the spec-file stem is the default BASE when no output flag is given (files are written, not printed).
- `rfs test` build pipelines: C unit tests compile to an object and link with the fsm object; C fuzz compiles both sources together; Rust uses the two-file include! layout.
- rfs-build's codegen invocation: per-slot flags with one `cargo:include` per generated file (consumer-visible paths unchanged).
- The C example projects: c_sync_handler builds explicit objects and links them; fuzz_c generates under the new flags and clang-builds both sources.

## Removed functionality
- `CodegenOutput.extra_files` / `ExtraFile` - replaced by the named slots.
- Single-file `--out FILE` semantics (the flag value was the exact output path; stdout was the default) - replaced by BASE naming with spec-stem default.
- Whole-source `.c` inclusion: no generated C artifact source-includes another `.c`; handler.c's `#include "arq_fsm.c"` pattern is gone.
- Fused Rust mc/test artifacts (FSM core inlined into every dependent file) - replaced by the include! reference; the fused `generate_core(with_tests=true)` contract survives ONLY for the aggregate-crate path.
- The `cargo run -- <command>` invocation pattern from every help surface, living doc, and training entry.

## New functionality
- Take-or-emit for referenced fsm files: an existing file with a matching line-1 spec hash is taken as-is; a missing one is co-emitted; a partial C pair (one of header/source) is a usage error; a foreign-hash taken file warns per D14-4 and is still used.
- `--out -` stdout mode: prints the single filled slot (Rust production); multi-slot commands are refused with a diagnostic naming the spec-stem files a bare invocation would have written.
- C header/source pair for production (and mc): include guard derived from the module name, dual line-1 provenance headers, engine-facing helpers as external symbols - a true object+link split.
- Rust include! reference mechanism (D14-5): the bare referenced fsm file (line-1 + core, no inner attributes) plus dependent artifacts that `include!` it at crate root.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs` (+10 tests) | added | Flag-family parsing, take-or-emit rules (both backends), C pair pins, no-source-includes-a-.c grep bar, Rust include! reference pins, no-FSM-duplication grep bar, the 13-surface help regression. |
| `tests/common/prod_driver.rs` | changed | New shared helpers `compile_c_object` + `build_c_mc_binary` (the object+link building block every migrated MC leg uses). |
| `tests/{backend_parity,benchmark_mc,ref_mc_diff,c_compiler_agnostic,monitor_integration}_test.rs` | changed | Migrated to the named slots and object+link / both-source build shapes; local single-file `compile_c` helpers removed where the shared builder replaced them. |
| `tests/smt_bounded_test.rs` | changed | Close-out correction (task 1): `.primary_code()` now returns `&str` - one `.to_string()` at the helper's return. |
| `tests/info_format_test.rs` | changed | Task-3 mc-pair header pins; close-out correction: expect-message format! hoisted (clippy::expect_fun_call). |

## Tests run status

| Name | Status |
|---|---|
| `cargo build --workspace` | OK |
| `cargo test` (root) | OK - 548 passed / 0 failed. Baseline 535 + 13: Phase 12's c_compiler_agnostic_test (+3, pre-existing to this phase) + this phase's 10 new codegen_test tests. |
| `cargo test --workspace` | OK - 676 passed / 0 failed (baseline 663 + the same 13). |
| `cargo test --features smt` | OK - 550 passed / 0 failed (baseline 537 + the same 13; includes the task-1 E0308 correction). |
| `cargo test-generated-code` | OK - exactly 1683/1683 (the fused generate_core contract is preserved). |
| `cargo clippy --all-targets` | At baseline: 55 warning lines / 34 unique = the pre-phase tree measured with the SAME clippy 1.99.0 (53/32) + 2 new dead-code warnings for the new shared test helpers (same class as the ~10 pre-existing helper dead-code warnings in the baseline). The documented 46/35 predates a clippy release. Three genuine new lints found at close-out were fixed via task corrections (task 1: none; task 3: expect_fun_call; task 5: needless_borrow x2). |
| `cargo fmt --check` | OK - clean. |
| `cargo test --test example_projects_test` | OK - 3/0 (c_sync_handler object+link + scenario parity, fuzz_rust, fuzz_c generation + clang build). |
| `cargo test --test dev_docs_test` | OK - 9/0. |
| `cargo test-training` | OK - 4/0; aggregate 267 lines unchanged. |
| CLI contract spot-checks | OK - zero `cargo run` in any help surface; spec-stem defaults for both backends; take-or-emit take AND emit sides; partial-pair usage error; `--out -` stdout + multi-slot refusal. |

## Found problems/issues/mismatches
- **Task 1's slot-model signature change broke the smt-feature leg** (found at close-out): `tests/smt_bounded_test.rs` expected `primary_code()` to return `String`; the feature-gated test never compiled under tasks 1-10's default-feature runs. Fixed with a task-1 correction (`.to_string()`); the smt leg is green.
- **Three clippy lints in this phase's new test code** (found at close-out): expect_fun_call (task 3, info_format_test.rs) and needless_borrow x2 (task 5, codegen_test.rs). Fixed via task corrections; zero behavioural change.
- **rfs-build was functionally broken by task 1's CLI change** (found at task 7, fixed there): the retired `--out <exact file>` invocation made every rfs-build consumer fail; root `cargo test` never builds the example crates, so tasks 1-5 stayed green while the workspace suite rotted.
- **The task-10 occurrence map was incomplete**: its 14-entry map covered only the `cargo run --` pattern; the audit found 5 more entries teaching the old output model without that prefix (in codegen_rust, design, project_tooling, frontend_cli). All were updated; the training baseline figure in the task file (264) also predates Phase 8 task 15's re-pin to 267.

## Other important findings/information
- Pre-existing path drift in three architecture-teaching training entries (old `src/backends/...` layout, no output-model content) was left untouched as out of scope and recorded in the task-10 status file.
- Environment (not project) findings from this sandbox: `nix develop` fails on a clang-answering-for-gcc tool-resolution artifact in the nix build sandbox; the canonical toolchain is the project-local rustup install at `/workspace/host/rustup` per TOOLCHAIN.md section 1 - all close-out verification ran under it.
