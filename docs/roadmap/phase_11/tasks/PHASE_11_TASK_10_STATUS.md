# Phase 11, Task 10 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_10.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | verification-only task per correction 1a - the incremental re-sync of tasks 1-9 left the manifest and doc references already in final state |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | no source changes |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | changed | 5 bare-name stragglers the prefixed-path greps of tasks 6-9 missed: section headings 5.1-5.4 (`codegen_fsm.rs` -> `src/codegen/fsm.rs`, `codegen_test.rs` -> `src/codegen/test_plan.rs`, `codegen_mc.rs` -> `src/codegen/mc_concepts.rs`, `monitor.rs` -> `src/codegen/monitor.rs`) + one prose ref (`json_rw.rs` -> `src/json/json_rw.rs`) - full-path form matching the doc's own file-map table |
| `docs/dev/codegen_core.md` | changed | 1 bare-name straggler: "the SMT path (`smt_plan.rs` / `smt_emit.rs`)" -> full `src/mc/...` paths |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| (none) | - | manifest already final (87 entries, verified against the filesystem scan inside dev_docs_test) |

## Changed functionality

None - docs-only verification sweep.

## Removed functionality

None.

## New functionality

None.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | the two doc gates are the verification instruments, unchanged |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK - 9/9 on first run; manifest needed no edits (correction 1a's per-task re-sync held) |
| `cargo test --test docs_md_test` (5) | OK - 5/5 after the two straggler fixes (re-verified post-edit) |

## Found problems/issues/mismatches

- The task file's expectation "after tasks 1-9, dev_docs_test FAILS and its
  output IS the update list" was pre-correction-1a: every move task already
  re-synced the manifest + doc refs in-commit, so the gate was green before
  this task started. The sweep's real value was the BARE-name check: prefixed
  old paths (`src/...`, `tests/...`) were zero-hit, but bare module names in
  section headings/prose (6 refs across CODEGEN.md + codegen_core.md) survived
  every earlier grep because those patterns required the path prefix.

## Other important findings/information

- Verification method for the record: (1) dev_docs_test 9/9; (2) fresh grep of
  ALL 45 old paths (42 src + 3 test, from the plan's mapping table) under
  docs/ excluding docs/roadmap/ history -> zero hits; (3) bare-name sweep for
  every moved file's old name across docs/dev + docs/llm + docs/user + all root
  *.md + ide/ + the backend .md docs -> the 6 stragglers above, now fixed;
  remaining grep hits are kept-file refs (`tests/codegen_test.rs` - a different,
  unmoved file) and intentional subgraph-relative mermaid labels in
  DEPENDENCIES.md (titles carry the directory context); (4) docs_md_test 5/5.
- TASKS.md and docs/roadmap/** untouched by this task (exit criterion 4).
