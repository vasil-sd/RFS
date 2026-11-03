# Phase 10, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_8.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 73 function rows (c.rs 6, c_fsm.rs 37, c_test.rs 15, c_mc.rs 15) + 2 type rows (Gen, Ctx) appended to `## All functions` / `## All types`; Component/Functionality still blank; preamble identifier bullet extended with the nested-fn form `<file>::<parent_fn>::<child_fn>` |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 8a recorded (chunk G type phantoms: authoritative 73/2, not ~73/~5; non-test src/ totals ~935 fns / ~207 types) |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_8.md` | changed | count figures corrected to the re-verified 73/2 (Goal, cluster notes, Planned actions, Scope table); nested-fn convention note added to Context |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The C backend is now dependency-annotated, mirroring the Rust-backend chunk's
  vocabulary (cross-backend parity invariant): c.rs's mangling/type-mapping
  helpers are the shared identifiers every C emitter row references; c_fsm.rs's
  Gen pipeline (header -> types -> monitors -> fsm impl) cross-references the
  validator typing spine, the monitor plan family, and codegen_fsm flattening
  exactly as rust_fsm.rs's rows do; c_test.rs renders every codegen_test plan
  case type into test source with the SAME stable test names as the Rust
  harness; c_mc.rs's exact_codegen_c is the single row that ties mc_plan
  (encoding/traversal/liveness), smt_plan/smt_emit (SMT mode), printer
  (formula text), and the JSON report contract together - the direct mirror of
  rust_mc.rs::exact_codegen.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green; no unescaped pipes in this chunk's descriptions |

## Found problems/issues/mismatches

- MAJOR type-count mismatch vs correction 2a ("T8 73/5"): c_mc.rs holds NO
  Rust types. Its three `struct` grep hits (`struct rusage ru;`, two
  `struct timespec`) are C declarations inside the generated-code raw strings,
  not toolchain definitions - the same template-phantom class correction 7a
  established for fns, here affecting types. The fn count (73) matched the plan
  exactly: c.rs 6 non-test (+4 inline tests belonging to task 30), c_fsm.rs 37,
  c_test.rs 15, c_mc.rs 15. Authoritative chunk G total: 73 fns / 2 types.
  Plan correction 8a recorded; the non-test src/ type total drops from ~210 to
  ~207 (the fn total ~935 is unchanged).
- MINOR internal inconsistency in the task file itself: Goal said ~73/~5 while
  Planned actions said ~77/~5 (the 77 counted c.rs's 4 inline tests, which per
  the boundary rule belong to task 30). Both figures corrected to 73/2.

## Other important findings/information

- First use of the nested-fn identifier convention: c_fsm.rs's `push` (inside
  collect_signal_patterns) and `walk` (inside collect_liv_signal_patterns) are
  the ONLY truly-nested fns in the non-test source so far - every other
  indented `fn` in inventoried files is an impl method. They get rows as
  `<file>::<parent_fn>::<child_fn>`; the convention is now pinned in the
  SRC_STRUCTURE.md preamble. `src/ebnf_grammar.rs::to_svg` contains one more
  (`to_railroad`) - its chunk task must follow the same form.
- c_mc.rs's embedded C engines (EXACT_ENGINE_C, LIV_ENGINE_C, JSON_WRITER_C,
  EXACT_RUN_C, MAIN_DISPATCH_C, SMT_ENGINE_C) contribute zero inventory rows -
  their template fns are generated-code text, mirroring correction 7a's finding
  for rust_mc.rs's raw strings.
- The C-backend rows complete the cross-backend dependency vocabulary: every
  shared identifier (validator typing, monitor plan, mc_plan encoding,
  codegen_test cases, smt spec) is now referenced from BOTH backends' rows,
  which tasks 15-16 will classify and task 17's file mapping will consume.
