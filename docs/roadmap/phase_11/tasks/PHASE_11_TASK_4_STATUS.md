# Phase 11, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none new) | codegen/mod.rs is the absorbed codegen.rs - it keeps its existing //! Docs: pointer line (which already names dev:code-map-codegen) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen.rs` | removed (moved) | git mv to src/codegen/mod.rs - ZERO-CHURN absorption: module path `codegen` unchanged; content changes are only the 4 new sibling declarations + doc-comment path updates + its own in-body `crate::codegen_mc::` refs rewritten |
| `src/codegen_fsm.rs` | removed (moved) | git mv to src/codegen/fsm.rs, byte-identical |
| `src/codegen_test.rs` | removed (moved) | git mv to src/codegen/test_plan.rs; 14 changed lines = its own import rewrites + one doc-comment path |
| `src/codegen_mc.rs` | removed (moved) | git mv to src/codegen/mc_concepts.rs, byte-identical |
| `src/monitor.rs` | removed (moved) | git mv to src/codegen/monitor.rs, byte-identical |
| `src/lib.rs` | changed | four `pub mod` declarations removed; `pub mod codegen;` untouched (zero-churn) |
| ~15 source files with import sites | changed | scripted rewrites of the four renamed modules in both `crate::` and `rfs::` forms across src/ + tests/ (fuzz/ untouched - zero-churn); the aliased import `use crate::codegen_mc as mc;` in info_format.rs fixed to `use crate::codegen::mc_concepts as mc;` |
| `src/backends/{rust_test,c_test,c_fsm,c_mc,rust_fsm,rust_mc}.rs` | changed | UNQUALIFIED usages renamed to the new bound names: `codegen_fsm::` -> `fsm::`, `codegen_test::` -> `test_plan::` (the imports bind `self` under the new names); ~15 stale doc-comment mentions of the old module paths updated |
| `src/ref_mc.rs` | changed | the independence_import_audit test's forbidden list updated ("codegen_fsm" -> "codegen::fsm") so the contract stays meaningful after the rename; two doc-comment refs updated |
| `src/mc_ref_cmd.rs`, `src/info_format.rs`, `src/codegen/test_plan.rs`, `tests/{benchmark_mc_test,monitor_integration_test}.rs` | changed | stale doc-comment references to the old module paths updated |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | five entries rewritten: src/codegen.rs -> src/codegen/mod.rs, codegen_fsm -> codegen/fsm, codegen_mc -> codegen/mc_concepts, codegen_test -> codegen/test_plan, monitor -> codegen/monitor (sorted block) |
| `docs/dev/CODE_MAP.md` | changed | all five file entries in the "Codegen core + backends" section rewritten to their src/codegen/... paths (section count stays 18 - every file remains inside the area); subsection "MC engine + monitor (5)" -> "MC engine (4)" (monitor now lives under codegen/) |
| `docs/dev/*.md`, `docs/llm/*.md`, `docs/user/*.md` (all files with refs) | changed | every live-doc reference to the five old paths rewritten to the new ones (per correction 1a - incremental, in this commit); roadmap history untouched by design |
| `docs/llm/TASK_MAP.md` | changed | path rewrites only - the file-list entry for src/codegen/mod.rs came along with the src/codegen.rs rewrite, so gate coverage holds automatically |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - pure move + import-path rewrite; 535/535 green. The zero-churn property held: no fuzz/ file appears in this commit's diff.

## Removed functionality

None.

## New functionality

- `src/codegen/` now holds the whole shared pre-backend layer (mod.rs registry +
  fsm + test_plan + mc_concepts + monitor) - backends/ will nest under it in
  task 5.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `src/ref_mc.rs` independence_import_audit | changed (in-place) | forbidden list updated to the post-rename module fragment so the audit keeps biting |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (after adding the four sibling declarations to mod.rs + renaming unqualified usages) |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline |
| `cargo test --test dev_docs_test` (9) | OK - green on first run after the incremental re-sync |

## Found problems/issues/mismatches

- The absorbed mod.rs needed FOUR new `pub mod` declarations - codegen.rs was a
  standalone file with none, and Rust does not auto-discover siblings. Not
  anticipated by the task file; caught by the compiler (E0432).
- Unqualified usages (`codegen_fsm::build_variants(...)`) were bound through
  `use crate::codegen_fsm::{self, ...}` imports: the scripted path rewrite left
  the OLD bound name behind while binding the new one. Fixed by renaming the
  usages to the new module names (fsm:: / test_plan::). Future absorption tasks
  (info_format in task 7) must check for this form.
- The ref_mc independence audit hard-codes forbidden module NAMES as strings -
  a rename silently vacates an entry unless the list is updated with the move.

## Other important findings/information

- Zero-churn verified: `git diff --name-only` contains no fuzz/ path; every
  `rfs::codegen::*` importer (incl. the detached workspace) compiled untouched.
- tests/liveness_mc_test.rs has a LOCAL helper function named `codegen_mc` -
  deliberately not renamed (it is not a module reference).
