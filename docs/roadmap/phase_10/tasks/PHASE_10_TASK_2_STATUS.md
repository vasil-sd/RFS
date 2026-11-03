# Phase 10, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

(none - docs-only phase)

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | `## All functions` gained 86 rows (parser.rs 76, error.rs 8, lib.rs 2) and `## All types` gained 4 rows (ModuleItem, TransitionBody, Span, ParseError); every row carries bidirectional Depends (project functions AND types), a one-line description, and Documented-in copied verbatim from the file's `//! Docs:` line; Component/Functionality blank per convention |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 2a recorded (chunk counts re-derived test-aware; gen_spec_cmd.rs + mc_ref_cmd.rs added to task 9; emitter-file counting caveat); Context scale lines and the Planned-tasks rows for tasks 2-13, 15, 16, 30 updated to the corrected numbers |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_{2..13,15,16,30}.md` | changed | Context/Goal counts and file lists updated per correction 2a (task files of not-yet-implemented tasks only) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - no source changed.

## Removed functionality

None.

## New functionality

None executable - the parser-core inventory chunk (entry points, comment stripping,
error construction, the full module/item/enum/event/output/var/transition/pattern/
expression/formula rule set, token helpers, KEYWORDS context) is now dependency-
annotated in SRC_STRUCTURE.md.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - after fixing the table hazards below; first run FAILED (see Found problems) |

## Found problems/issues/mismatches

- Plan correction 2a (recorded in PHASE_10_PLAN.md): the planned chunk counts came
  from raw line-based greps that included inline `#[cfg(test)]` functions (which per
  the boundary rule belong to tasks 29-30) and missed `src/gen_spec_cmd.rs` +
  `src/mc_ref_cmd.rs` entirely. Corrected chunk A scope: 86 fns / 4 types (was
  ~120/~4). All other chunk tasks' counts re-derived the same way.
- Naive counters miscount emitter files: `rust_test.rs` and `test_cmd.rs` have NO
  real test module (their awk-era inline counts were template fns after doc-comment
  mentions of `#[cfg(test)]`); `smt_emit.rs`'s real test module holds 6 fns, not 10.
  Inline test total corrected to ~255 (was 311).
- Markdown table hazards in the inventory rows (caught by docs_md_test on first
  run): literal `|` inside description cells splits the cell and breaks column
  alignment (11 rows affected - e.g. the action/type_parser/pattern descriptions
  using ` | ` as a separator), and bracket-paren text like `always[N](f)` parses
  as a relative markdown link that fails resolution. Fixed by escaping literal
  pipes as `\|` and wrapping formula forms in code spans. Future enumeration tasks
  must apply the same treatment to any description containing pipes or
  `[...](...)` shapes.

## Other important findings/information

- Method note for later chunks: impl-block methods are identified as
  `<file>::<Type>::<method>` (e.g. `src/error.rs::Span::new`) to disambiguate
  same-named constructors across types in one file - an extension of the
  `<file>::<function>` convention, applied consistently from this chunk on.
- Row-count verification: 76 + 8 + 2 = 86 function rows and 4 type rows, matching
  the test-aware per-file counts exactly; all 90 data rows have exactly 6 cells.
