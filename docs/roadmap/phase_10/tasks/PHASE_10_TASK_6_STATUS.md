# Phase 10, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_6.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 57 function rows (mc_plan.rs 18, smt_plan.rs 11, smt_emit.rs 28) + 29 type rows (mc_plan.rs 16, smt_plan.rs 11, smt_emit.rs 2) appended to `## All functions` / `## All types`; Component/Functionality still blank |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 6a recorded (smt_emit.rs authoritative count 28 non-test fns / 10 test fns / 2 types - correction 2a's "T6 61/29" and its "6 real test fns" claim were wrong; chunk E total 57/29, non-test src/ total ~995) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The MC-planning + SMT cluster is now dependency-annotated: mc_plan.rs's
  plan computation (McPlan::compute -> referenced_fields/monitor_layouts/
  config_encoding/traversal_spec) references codegen_fsm's variant flattening
  + representative values, the monitor compiler (compile_formula, MonNode,
  MonitorPlan), and the codegen AssumptionSelection; its liveness
  decomposition (LivenessDecomposition::compute -> decompose_top /
  boolean_over_atoms/contains_liveness) is self-contained over Formula.
  smt_plan.rs's build_smt_spec consumes the McPlan abstraction table +
  ValueDomain/RepValue and converts IR expressions/formulas with validator-
  order name resolution; se_type/bit_op_type mirror the validator's typing
  rules. smt_emit.rs's dual emitters (emit_rust_spec / emit_c_spec with the
  shared render_se/render_bool_bin/render_bit_bin/render_shift/render_sf
  algorithm pair and RustRender/CRender accumulators) depend only on
  smt_plan's spec + type family - the parity-by-construction boundary.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run caught 1 unescaped pipe in boolean_over_atoms's operator list (`!/&&/||/->`); escaped as `\|\|`, second run green |

## Found problems/issues/mismatches

- Correction 2a's smt_emit.rs count was wrong in BOTH directions it claimed:
  its "real test module holds 6 fns, not 10" phantom theory did not hold up -
  all 10 test-module fns are real, and there are no template phantoms at all
  (the generated `fn` text lives inside format!/push_str string literals and
  never starts a line, so line-based counters see only real definitions).
  Authoritative: 28 non-test fns / 10 test fns / 2 types. Chunk E total is
  57/29, not 61/29 - recorded as plan correction 6a (supersedes correction
  2a's "T6 61/29"; non-test src/ total ~995, was ~999).
- mc_plan.rs's type grep matched 18, but 2 of those are template enums
  (`enum State { A, B }`, `enum Mode { M1, M2, M3 }`) inside string fixtures
  in the test region - not standalone types. Authoritative: 16 non-test
  types (matches the task file's cluster note).
- The same stale-table pattern as tasks 4-5: Scope/Planned-actions said
  "~83 function rows + ~31 type rows" (test-inclusive / pre-correction-2a);
  resolved to 57/29 per the boundary rule + correction 6a.

## Other important findings/information

- smt_emit.rs's inline tests belong to task 30 (backends/MC/CLI cluster), as
  do mc_plan.rs's - neither file is in task 29's frontend/IR list.
- The emitter pair is the cleanest parity boundary in the inventory: every
  `*_c` function depends on exactly the same smt_plan identifiers as its
  `*_rust` mirror (plus its own CRender instead of RustRender) - useful for
  task 14's component categorization.
