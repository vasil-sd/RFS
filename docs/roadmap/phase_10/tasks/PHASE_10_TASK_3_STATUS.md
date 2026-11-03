# Phase 10, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_3.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 115 function rows (ir.rs 63, printer.rs 20, json_rw.rs 32) + 27 type rows (ir.rs 23 incl. the IrJson trait, printer.rs Fctx, json_rw.rs Json/JsonError/Reader) appended to `## All functions` / `## All types`; Component/Functionality still blank |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The serialization spine is now dependency-annotated: ir.rs's 22 per-type
  IrJson impl pairs reference the json_rw value type + writer constructors and
  the shared field-extraction helpers; printer.rs references the IR types it
  renders plus parser/json_rw entries in its round-trip helpers; json_rw.rs is
  self-contained (Reader -> Json/JsonError only).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run caught 2 unescaped pipes in descriptions (bin_str's operator list, expr_has_boolean_op's `&&/||`); escaped as `\|`, second run green |

## Found problems/issues/mismatches

- ir.rs identifier collision: `Ir` has BOTH inherent pub `to_json`/`from_json`
  wrappers AND the IrJson trait impl pair. Convention adopted (recorded for all
  later chunks): trait impl methods are identified `<file>::<Type>(<Trait>)::<method>`
  (e.g. `src/ir.rs::Ir(IrJson)::to_json`), inherent methods keep the plain
  `<file>::<Type>::<method>` form. The private IrJson trait itself is enumerated
  as a type row with its two method signatures as function rows.
- Two descriptions contained literal pipe characters (operator symbol lists)
  that broke table cell alignment - escaped per the task-2 treatment.

## Other important findings/information

- Running totals after this task: 201 function rows + 31 type rows in
  SRC_STRUCTURE.md (chunks A+B of the ~999/~233 non-test inventory).
