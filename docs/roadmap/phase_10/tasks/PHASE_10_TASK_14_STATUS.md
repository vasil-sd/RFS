# Phase 10, Task 14 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_14.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | `## Components` table filled (32 categories) + `## Functionality` table filled (20 axes) - every row carries name / depends-on list / description + rationale; all deviations from the seed proposal explained in-row per the exit criteria |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_14.md` | changed | Context's stale "~999 function rows, ~233 type rows per plan correction 2a" corrected in place to the authoritative final inventory (938 + 197 = 1,135 rows per correction 13a) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The classification vocabulary for tasks 15-18: 32 component categories + 20 functionality axes derived from the finished inventory's dependency edges (file-level edge scan over all 1,135 rows). Components: the seed's 30 minus "toolchain fuzz targets" (boundary rule - fuzz/fuzz_targets/*.rs belong to the test inventory, task 30) plus three new: Shared FSM concepts (codegen_fsm.rs had no seed category - its 10 consumer files prove it is a distinct shared layer, not part of Codegen core), gen-spec command and mc-ref command (completing the one-category-per-command-file pattern the seed applied to rfs-test/info). Folds: error.rs into Parser (all 7 of its incoming edges come from parser.rs - it is the parser's diagnostics infrastructure), cli_log.rs into CLI dispatch (zero outgoing, 27 incoming from the command modules), and the two codegen-test bins + lib.rs collectors into Shared test planning (the seed lumped all four bins with the training pipeline; the edges split them - generate_code_tests/test_generated_code consume codegen + rust_test + lib.rs, never training.rs). Rename: "diagram core + plugins" -> "Diagram command + plugins" (diagram.rs is the command module). Functionality: the seed's 16 minus "IR serialization" (renamed wire-format serialization - json_rw serves every wire format, not just IR) plus four new: source rendering (the printer's 21 functions had no axis - rendering is the inverse of parsing and must stay separable for impact analysis), spec generation (specgen's 58 functions + gen-spec), diagram generation (the 53-function diagram stack), external tool integration (an ext_tool change impacts rfs-test and diagram at once, so it is distinct from CLI surface).
- Backend parity invariant respected in the design: Rust backend and C backend stay separate categories with symmetric intended functionality assignments (codegen-production / codegen-tests / model-checking + fuzzing). Two observed dependency asymmetries flagged for the task-18 mapping: the c_* emitters borrow snake/rust_ident from rust.rs (the one deliberate cross-backend coupling - a refactoring should move those utilities to a neutral module), and rust_test's spec-hash stamping has no C-side counterpart.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green |

## Found problems/issues/mismatches

- No plan correction needed: the seed proposal is a starting point by design ("refine after seeing all rows"), and every deviation is explained in the table rows per the exit criteria. The task file's Context cited correction 2a's approximate totals (~999/~233); the authoritative final inventory per correction 13a is 938/197 - corrected in place in the task file (same stale-reference class as tasks 5, 8-10, 13).

## Other important findings/information

- Dependency picture driving the design (file-level edge scan): json_rw.rs is a leaf with 134 incoming edges; ir.rs is the hub with 576 incoming; ext_tool.rs and cli_log.rs are zero-outgoing leaves with 61/27 incoming respectively; smt_emit's 59 outgoing edges all target smt_plan (inseparable - the seed's merge stands); codegen.rs <-> codegen_mc.rs form a real 2-edge cycle (kept separate: dispatch vs wire format); ref_mc's independence contract verified by the edge scan (outgoing only to ir 86, codegen_mc data types 18, json_rw 3 - no mc_plan/monitor/codegen_fsm/backend edges).
- File-coverage walk (exit criterion): all 61 inventory files are assignable to >= 1 component and >= 1 functionality under the new vocabulary; src/backends/mod.rs remains file-mapping-only (0 rows per correction 2a, task 18).
- Both tables' depends-on lists use only defined category names (verified: zero unresolved references) - tasks 15-16 assign rows against this frozen vocabulary, and task 17 justifies the naming scheme against these categories.
