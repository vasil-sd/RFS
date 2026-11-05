# Phase 10, Task 27 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_27.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | 72 function rows + 9 type rows appended to `## All functions` / `## All types` for the monitor/PSL test cluster: 21 fns + 5 types for `tests/monitor_ref_test.rs` (hand-written Rust + C reference monitors for specs 43/44/45 vs generated monitors on random traces + crafted boundary scenarios), 17 fns + 1 type for `tests/monitor_integration_test.rs` (injected bug through the MC in both backends, no-false-positive correct-spec run, debug-gated production monitors at the step API), 34 fns + 3 types for `tests/psl_translation_test.rs` (independent recursive window evaluator vs generated monitors over all property-bearing examples + the RFS_PSL_REF_TOOL hook). Every row carries bidirectional project Depends (mapped to real identifiers across src/{parser,ir,codegen,lib}.rs and the task-20 tests/common/prod_driver.rs helpers - build_drivers/build_sample_drivers/input_script/run_driver/production_artifacts/flattened_inputs/flattened_outputs/state_variants/FlatVariant), a one-line description, and blank Component/Functionality/Documented-in cells. The six Ctx methods get `<file>::<Type>::<method>` identifiers (the first method rows in this document). Also fixed the section-heading glue defect that task 26's splice had introduced (see Found problems). |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | Plan correction 27a: two template phantoms inflated the Goal's per-file figures - see Found problems. |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_27.md` | changed | Task corrections section: row counts 73/10 -> 72/9 with the phantom analysis (mirrors plan correction 27a). |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The monitor/PSL test cluster is now in the test inventory:
  - `tests/monitor_ref_test.rs` (21 fns / 5 types): independent hand-written Rust + C (`gcc -Wall`) reference monitors for specs 43/44/45 vs the generated online monitors - random traces (seeded input script drives the real production FSM, trace parsed back out of the driver stdout) and crafted sample scripts (boundary values, bit-width wraparound, obligation deadlines confirmed exactly at trigger+bound-1, re-trigger after clear, sticky failure, same-sample multi-property confirmation with declaration-order tie-break); generated backends must also agree with each other.
  - `tests/monitor_integration_test.rs` (17 fns / 1 type): the Phase 3 end-to-end proof - MC catches the injected bug (dropped Ack transition) in BOTH backends with cross-backend parity on label/trigger/violation steps (re-pinned fin_acked_bounded, trigger 1, violation 5), the correct spec reports no property violation (no monitor false positives; re-pinned FAIL(deadlock) at depth 3), and production debug monitors surface the violation at the step API in both languages with identical values.
  - `tests/psl_translation_test.rs` (34 fns / 3 types): the independent recursive window evaluator for the monitorable PSL subset (own bit-vector semantics per SEMANTICS 2.2, own obligation scanning - window search, not a countdown automaton) vs the generated monitors over ALL property-bearing examples: first-confirmation sample AND trigger sample must match exactly; the RFS_PSL_REF_TOOL external hook (availability-gated) is cross-checked when set; non-degeneracy floors pinned.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written and the two glued section headings were restored (the new rows are plain-text table cells, no backticked paths or mermaid, so they do not trip the gate's link/mermaid checks) |

## Found problems/issues/mismatches

- PLAN CORRECTION 27a: the Goal's per-file figures were grep counts inflated by two template phantoms (the class of corrections 7a/8a/20a/21a/23a/24a/25a). monitor_ref_test.rs: one of the six type hits is a C `enum {{ KIND_ALWAYS, KIND_OBLIGATION }}` on a line of c_reference_source's format! string template (it starts a line, so the naive counter sees it) - only Sample, RefViolation, RefProp, CTable and SpecDef are real Rust types. monitor_integration_test.rs: one of the eighteen fn hits is `fn main() {` at column 0 inside the RUST_DRIVER raw-string driver template - only 17 fns are real (the single type McViolation is clean). psl_translation_test.rs is clean at 34/3. Authoritative total: 72 fns / 9 types (was 73/10).
- Task 26's splice had introduced the section-heading glue defect AGAIN, and this task's first insertion attempt caught it: the committed file had `...toolchain_fuzz_slow_suite |  |` directly adjacent to `## All types` (and the Run row to `## Components`) with NO blank line. Root cause: task 26's splice `text[:eol+1] + rows + text[eol+1:]` consumed the separating blank line as the terminator of the last inserted row - its assertion `text[eol+1] == "\n"` verified the blank line EXISTED but not that it would SURVIVE (the leading newline of `text[eol+1:]` becomes the last row's own terminator). Comrak parses a heading inside a table cell as cell text, so docs_md_test never caught it. Fixed in this task's insertion pass: the splice now detects the glued state and re-adds the blank line before each heading (all six section headers verified present in order).

## Other important findings/information

- The six Ctx methods (`new`, `n`, `mask`, `var_index`, `enum_variants`, `state_index`) get the `<file>::<Type>::<method>` identifiers `tests/psl_translation_test.rs::Ctx::<method>` per the preamble's method form - the first method rows in this document (the nested-fn form `<file>::<parent_fn>::<child_fn>` was already used by tasks 22-23).
- All 81 rows have blank Component/Functionality (task 32) and Documented-in (none of the three files carries a `//! Docs:` line; all three DO have docs/llm/TASK_MAP.md suite entries, noted per the preamble's rule rather than repeated row by row - same practice as tasks 21-26).
