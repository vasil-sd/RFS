# Phase 10, Task 16 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_16.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | Component + Functionality cells set on ALL 197 rows of the `## All types` table (authoritative count per correction 13a). With this, BOTH inventory tables are fully classified - 938 function rows + 197 type rows = 1,135 rows, every one carrying Component + Functionality from the task-14 vocabulary. Base component per row = the file's category; functionality follows what the type CARRIES/ENABLES. Zero multi-component rows: every shared type (wire models in codegen_mc/json_rw, plan models in mc_plan/codegen_test/monitor/smt_plan) already sits in its shared home, so none needed a second component - the task context's "name the shared home, not both backends" rule held by construction and was machine-verified (no row names both backends; every row carries its file's base component) |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_16.md` | changed | Goal's "~233 rows" and Planned actions' "~237 rows" corrected in place to the authoritative 197 per correction 13a (same stale-reference class as tasks 5, 8-10, 13, 14) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The fully classified `## All types` table: 197 rows, zero out-of-vocabulary values (machine-checked against the 32-component / 20-axis vocabulary). Coverage: all 32 components and all 20 axes are referenced by at least one type row; component mentions total 197 (one per row), functionality mentions 293 (83 multi-valued rows).
- Classification judgments recorded for the refactoring consumer, kept mutually consistent with the task-15 function-table picture:
  - Formula-carrying types are tagged PSL translation at every home: ir.rs `Property`/`Formula`, monitor.rs `SignalExpr`/`SignalAtom`, mc_plan's five liveness-decomposition types (D-P4-9), smt_plan `Sf`/`SmtProp`, ref_mc `Sig` + the liveness skeleton/witness types (`Leaf`/`Skel`/`LivProp`/`LivWitness`/`LeafSig`).
  - Monitor-carrying types are tagged property monitoring: both backends' `Gen` structs (verified to hold `assume_plans`/`prop_plans: Vec<MonitorPlan>`), mc_plan's packed monitor-state types, ref_mc `Mon`/`MonSt`, codegen_test `TestPlan`/`InvariantCase`/`MonitorViolationCase`, and codegen.rs `PropertyMonitorMode`.
  - Wire-format types are tagged wire-format serialization: the four codegen_mc JSON report models, ref_mc's CE record/result types (`IoRec`/`StepRec`/`CeData`/`RefResult`), frontend `StructuredError` (the --json document shape), and ir.rs `Ir` additionally carries provenance (it holds ir_version - consistent with the function row `Ir::new`).
  - Fuzz-carrying types are tagged fuzzing: codegen_test's decode-plan trio + `Lcg`, codegen_mc's bounded-run config/status/phase types, codegen.rs `ExtraFile` (the fuzz-skeleton carrier), test_cmd `TestSuite`/`TestOptions`/`FuzzContext`, and specgen `Rng`/`Bounds` (shared constants with the fuzz wire format).
  - CLI-facing option types carry CLI surface plus their domain: codegen.rs option/command enums, test_cmd options, gen-spec/mc-ref/info/diagram option types.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - 12 runs, one after each cluster save (chunks B, A, C, D, E, F+G, H, I+J, K+L), all green first try |

## Found problems/issues/mismatches

- None recorded. No plan correction was needed: the task-14 vocabulary classified every type row without gaps, and the shared-type consistency rule (exit criterion 3) held by construction - verified by machine check that no row names both backends and every row carries its file's base component, matching the sibling function rows' cluster assignments.

## Other important findings/information

- The task file's "~233 rows" / "~237 rows" figures were stale pre-correction numbers (correction 2a era); the authoritative type-row count is 197 per correction 13a - corrected in place in the task file, same stale-reference class as tasks 5, 8-10, 13, 14.
- Type-table physical row ranges by chunk (for tasks 17-18): B 989-1015, A 1016-1019 (lib.rs has no type rows), C 1020-1032, D 1033-1070, E 1071-1099, F 1100-1102, G 1103-1104, H 1105-1123, I 1124-1130, J 1131-1142, K 1143-1173, L 1174-1185.
- Spot-check of 10 random rows against source confirmed the judgment-heavy assignments (both `Gen` structs verified to hold `Vec<MonitorPlan>`; `ExactStatus`'s three-way split mirrors its function rows' wire/human vs exit_code split; `StructuredError` is the --json document shape).
- Both inventory tables are now complete - task 17 (naming scheme) and task 18 (file mapping + mermaid layout) can consume the fully classified picture.
