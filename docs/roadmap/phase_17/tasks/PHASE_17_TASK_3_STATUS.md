# Phase 17, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (documentation-only task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed | budget header 1786 -> 1830 (headroom for tasks 6-9's new rows); five line-neutral stale-name fixes: row 598 "test_cmd" -> cli/commands/test.rs, row 1269 component description frontend.rs -> utils.rs (with rename note), row 1416 tree legend, row 1656 PHANTOM file-mapping row renamed to the real src/cli/utils.rs, row 1701 mermaid label |
| `docs/llm/TEST_STRUCTURE.md` | changed | budget header 1379 -> 1430; thirteen line-neutral stale-name fixes: gen_spec_cmd/mc_ref_cmd (rows 1196-1197), cli_log (row 1193), and ten `frontend_test` -> `utils_test` references across rows 1180/1181/1183/1193/1195/1196/1198/1202/1220/1225 + the mermaid node label (row 1314) |
| `docs/llm/CHANGE_IMPACT.md` | changed | four fixes: gen_spec_cmd -> cli/commands/gen_spec.rs (row 35), test_cmd -> cli/commands/test.rs + frontend_test x2 -> utils_test (row 32), frontend_test -> utils_test (rows 21-22) |
| `docs/llm/TASK_MAP.md` | changed | row 31: mc-ref CLI row now names the mode dispatch (D-P16-4) + all three engine entry points (run/run_fuzz/run_smt delegating to ref_mc_exact/fuzz/smt) and links all three engine fragments |
| `docs/llm/cli_commands.md` | changed | row 33: dispatch description now says ALL eight commands are registry plugins (main.rs a pure dispatcher); row 39: exit_code_for described as exact-mode-only with fuzz/smt-bounded codes derived inline in the dispatch |
| `docs/llm/ir_json_rw.md` | changed | two frontend_test -> utils_test fixes (beyond the task file's list - see corrections) |
| `docs/llm/validator.md` | changed | two frontend_test -> utils_test fixes (same) |
| `docs/dev/ir_json_rw.md` | changed | one `cargo test --test frontend_test` -> utils_test fix (same) |
| `docs/dev/validator.md` | changed | one `cargo test --test frontend_test` -> utils_test fix (same) |
| `KB.md` | changed | section 4.2: three NEW rows for src/mc/ref_mc_exact.rs / ref_mc_fuzz.rs / ref_mc_smt.rs (content from PHASE_16_STATUS.md + the sources); the ref_mc.rs row updated to common-core + entry-points (D-P16-1); the cli/commands/mc_ref.rs row updated to three-mode dispatch with per-mode exit codes |

## Other file changes

None.

## Changed functionality

None - documentation only.

## Removed functionality

None.

## New functionality

- Both structure-inventory budgets carry headroom for the example-project tasks
  (SRC 1830, TEST 1430).
- The LLM set + KB.md source map name every Phase 16 file and use only live
  file/test names.

## Tests add/change/remove

None - existing gates re-run.

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK |
| `rfs-dev inventory audit` | EXACT: SRC 998 fns / 210 types, TEST 1031 fns / 35 types - zero per-file diffs |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- The one-directional inventory audit does NOT catch phantom file rows: SRC_STRUCTURE carried a file-mapping row for `src/cli/frontend.rs` (deleted in Phase 13) with no corresponding source file, and it passed the audit. Fixed by renaming the row to the real `src/cli/utils.rs`. Future inventory work should treat file-mapping rows as checkable too.
- The stale-name sweep was substantially wider than the task file listed (15 extra live references across 6 extra files) - recorded as task corrections.

## Other important findings/information
- Intentional rename-history notes in SRC_STRUCTURE (the "X -> Y" form at rows
  1334-1336, 1362, 1387, 1415) were kept - they document the Phase 11 renames
  by design and are not stale references.
