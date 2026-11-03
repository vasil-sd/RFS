# Phase 10, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_4.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 47 function rows (validator.rs 26, codegen_fsm.rs 6, monitor.rs 15) + 13 type rows (validator.rs ValidationError/ExprType/Env, codegen_fsm.rs VariantInfo/FsmVariants/RepValue, monitor.rs SignalExpr/SignalAtom/AnchorRange/MonNode/MonitorPlan/MonState/ReferenceMonitor) appended to `## All functions` / `## All types`; Component/Functionality still blank |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The validation + monitor-compilation cluster is now dependency-annotated:
  validator.rs's fatal pass (validate_module) references the typing spine
  (type_expr/resolve_path/binary_type/bit_op_type, ExprType::must_bool /
  compatible_with), the pattern/output checkers, bounded_always_error and
  type_formula; module_warnings references the warning helpers
  (collect_names, is_literal_false, contradictory_equalities,
  pattern_variant_name). codegen_fsm.rs references the IR constructor types it
  flattens (ConstructorVariant/FieldDecl/Type) plus Formula for the legacy
  safety-subset helpers. monitor.rs's compiler (compile_formula ->
  compile_check / compile_signal) references the formula atoms it lowers
  (Expr/EventPattern/OutputPattern) and builds the SignalExpr/MonNode/
  AnchorRange plan family; the test-only reference interpreter
  (MonState, initial_state, ReferenceMonitor, step_node, min_trigger_at)
  depends on the same node/state types and is flagged as such in its rows.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green; no unescaped pipes in the new rows |

## Found problems/issues/mismatches

- The task file's Scope table and Planned actions still said "~105 function
  rows" (the pre-correction-2a total INCLUDING the 58 inline `#[cfg(test)]`
  functions), while its own Goal said ~47 fns. Re-verification confirmed
  26+6+15 = 47 non-test fns / 3+3+7 = 13 types, matching plan correction 2a
  ("T4 47/13") and the boundary rule (inline tests belong to task 29).
  Recorded as a task-level correction; no plan correction needed.

## Other important findings/information

- Dependency convention applied (consistent with tasks 2-3): a function row
  lists project functions it calls, types it matches/destructures or
  constructs in its own body, and its self type for inherent methods
  (constructors excepted); opaque pass-through parameters are NOT listed.
  `check_output_pattern_group` therefore has an empty Depends column (it
  ignores both parameters - Phase 1 placeholder).
- monitor.rs's reference interpreter is `pub` but test-only by construction;
  its rows carry a "(test-only reference interpreter)" marker in the
  description so task 29 can cross-link them from TEST_STRUCTURE.md.
