# Phase 14, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none - docs-only task.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

(none - the exit criteria's "no source changes" held; the only conditional edit was the TEST_STRUCTURE.md budget header, recorded below.)

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `src/codegen/CODEGEN.md` | changed (output contract + CLI sections) | The output contract now documents the named-slot model (`ArtifactSlot`, `CodegenOutput::filled_slots/slot_content/primary_code/primary`) instead of `{ code, extra_files }`; the full CLI surface is documented: spec-stem BASE default, per-artifact override flags, take-or-emit for referenced fsm slots (partial C pair = usage error; stale line-1 hash warns per D14-4), and `--out -` stdout mode with its multi-slot refusal. Zero `extra_files` mentions remain. |
| `src/codegen/backends/C.md` | changed (artifact layout section) | New "Artifact layout (Phase 14: header/source separation)" section: what lives in the header (mangled uppercased include guard derived from the module name, its own line-1 artifact header, types/enum/constants, function prototypes) vs the source (its own line-1 header, `#include` of its own header, definitions); the no-source-includes-a-.c invariant; the header-reference rule for mc/test/fuzz artifacts. |
| `src/codegen/backends/RUST.md` | changed (artifact layout section) | The include! reference mechanism (D14-5): the bare referenced fsm file (line-1 + core, NO inner attributes - `include!` cannot bring `#![...]` in) plus the test/mc sections prefixed with their own prelude + `include!("<fsm file>")`; the fused-vs-referenced rendering split in rust/fsm.rs (`render_fsm_parts` as the single rendering path); the preserved byte-for-byte fused `generate_core(with_tests=true)` aggregate contract. |
| `docs/dev/cli_commands.md` | changed (via rfs-dev sync-fix) | The codegen command's output prose rewritten from "artifact + extra files next to it / stdout refuses extra-file commands" to the write_artifacts flow: BASE := --out value or spec stem, per-artifact overrides, take-or-emit (partial C pair = usage error, D14-4 stale-hash warning), `--out -` single-slot print with multi-slot refusal. |
| `docs/dev/backend_c.md`, `docs/dev/backend_rust.md`, `docs/dev/DEPENDENCIES.md`, `docs/dev/workspace_crates.md` | changed (via rfs-dev sync-fix) | Pointer/reference re-syncing to the renamed/moved items (slot-model accessors, render_fsm_parts/generate_test_reference, write_artifacts); no prose-model changes beyond what sync-fix listed. |
| `docs/llm/SRC_STRUCTURE.md` | changed (via rfs-dev inventory) | Rows added for the new slot-model functions (`ArtifactSlot::default_name`, `CodegenOutput::{filled_slots,slot_content,primary_code,primary}`, `render_fsm_parts`, `generate_test_reference`, the write-loop helpers) and the removed `ExtraFile`-era rows deleted; Depends columns updated where call edges moved (e.g. rust mc/test now depend on render_fsm_parts/generate_test_reference instead of generate_core). |
| `docs/llm/TEST_STRUCTURE.md` | changed (via rfs-dev inventory + budget header) | Rows added for the new test functions (compile_c_object, build_c_mc_binary, the codegen CLI flag-family/take-or-emit/stdout-mode pins, the no-source-includes-.c and no-fsm-duplication grep bars); removed-helper rows deleted; Depends edges updated. **Budget header raised 1309 -> 1325** (round ceiling with headroom, the Phase 13 way) because the legitimate new rows took the file to 1323 lines - recorded here per action 8.3. |
| `docs/llm/codegen_core.md` | changed | The codegen-core fragment description updated to the slot model (one-line fix). |
| `docs/llm/TASK_MAP.md` | changed | The codegen functionality entry now names the Phase 14 write loop (`write_artifacts`: naming resolution, take-or-emit, stdout mode) alongside the dispatch. |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-8 complete, tasks 9-11 pending. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

(none - `tests/dev_docs_manifest.txt` untouched: no `//! Docs:` pointer lines were added or removed by this docs-only task, and sync-check confirms pointer validity.)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

(none.)

## Tests run status

| Name | Status |
|---|---|
| `rfs-dev sync-check` | OK - zero stale/missing/orphan pointers |
| `rfs-dev inventory audit` | OK - EXACT: SRC 955 fns / 200 types, TEST 976 fns / 34 types, zero per-file diffs |
| `cargo test --test dev_docs_test` | OK - 9 passed / 0 failed (pointer validity, reference resolution, fragment ownership, TASK_MAP coverage, Class B budgets - the raised 1325 budget passes) |
| `cargo test --workspace` | OK - 676 passed / 0 failed (exact task-7 baseline; one rfs-adapters E0514 cache mismatch from the previous session's rustc was cleared with `cargo clean -p rfs-adapters` before the run) |
| `cargo fmt --check` | OK - clean |
| `cargo clippy --workspace` | OK - no NEW warnings: this task changed zero `.rs` files, so the warning set is byte-identical to HEAD's by construction (observed: rfs 12, rfs-adapters 3, async_handler 1, sync_handler 1 - all pre-existing baseline) |

## Found problems/issues/mismatches
- **TEST_STRUCTURE.md was over its Class B budget** (1323 lines vs the 1309 header) when this task's verification started: tasks 1-6's new test functions had been inventoried but the budget header never raised. Fixed by raising it to 1325 (action 8.3's conditional path) - the gate is green again.
- **`nix develop -f shell.nix` fails in this sandbox** on the rfs package build: `ref_mc_diff_test` panics with a CLANG-style warning ("overlapping comparisons always evaluate to true [-Wtautological-overlap-compare]", "1 warning generated.") attributed to `gcc -Wall`. Running the same suite directly against the store's real gcc 16.2.0 passes 4/4, so this is a tool-resolution artifact of the nix build sandbox (clang answering for gcc), not a tree regression - the generated always-true disjunct `(s != B) || (s == B)` is legal and gcc-warning-free. Noted so a future session does not chase it as a codegen bug.

## Other important findings/information
- All three exit-criteria greps verified: `extra_files` appears 0 times in CODEGEN.md, backends/C.md and backends/RUST.md; the C split model and the Rust include! + preserved-fused-contract sections are present.
- CHANGE_IMPACT.md needed no change: its codegen rows name files and test suites (still accurate) and carry no old-output-model prose - confirmed by grep for `extra_files` / `--out FILE` (0 hits).
- The docs describe the FINAL state per the task's dependency on tasks 1-6; every CLI behaviour documented (BASE naming, take-or-emit outcomes, stdout refusal) matches the committed source and the task-6 help text.
