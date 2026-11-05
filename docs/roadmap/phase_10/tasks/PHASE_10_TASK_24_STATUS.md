# Phase 10, Task 24 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_24.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | 51 function rows + 5 type rows appended to `## All functions` / `## All types` for the three cross-backend parity suites: 41 fns + 4 types for `tests/backend_parity_test.rs` (exact-mode differential runs over all property-bearing examples, re-pinned values, generated-test-executable parity, liveness parity incl. inline variants + truncated-mode soundness, JSON report parity + five status/reason cases + golden document pin, forced-violation parity), 7 fns for `tests/wide_ops_parity_test.rs` (wide-ops boundary drivers byte-identical + pinned, exact-MC status parity), 3 fns + 1 type for `tests/prod_parity_test.rs` (production-FSM cross-backend byte parity over EVERY valid example with identical seeded LCG scripts). Every row carries bidirectional project Depends (mapped to real identifiers across src/{parser,codegen,codegen_mc,json_rw,lib}.rs and the tests/common helpers enumerated in task 20 - prod_driver build_drivers/input_script/run_driver, mc_json zero_peak_rss), a one-line description, and blank Component/Functionality/Documented-in cells. Also removed a stray blank line between the last task-20 row (mc_json.rs::zero_peak_rss) and the first task-21 row that had split the All functions table into two GFM fragments. |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | Plan correction 24a: the cluster notes' "41 fns / 14 types" (backend_parity) and "8 fns" (wide_ops) were grep counts inflated by template phantoms - see Found problems. |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_24.md` | changed | Task corrections section: row counts ~52/~18 -> 51/5 with the phantom analysis (mirrors plan correction 24a). |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The cross-backend parity suites are now in the test inventory:
  - `tests/backend_parity_test.rs` (41 fns / 4 types): the backend-parity principle made executable - exact-mode MC field identity over all property-bearing examples, re-pinned values (14 pass-complete / 9 fail), generated-test-executable agreement (per-test result lists + byte-identical random-trace coverage line), liveness parity (example 43 + ten inline variants + truncated-mode soundness + pinned lasso fields), JSON report parity (byte-identity modulo peak_rss_kb, canonical form, value agreement with the text run, five status/reason cases, golden document pin) and forced-violation parity over all assumption-bearing examples.
  - `tests/wide_ops_parity_test.rs` (7 fns): spec-50 boundary behavior (variable shifts by >= width -> 0, wide multiplication) byte-identical across backends AND matching pinned values independent of either backend; exact-mode MC status parity on the spec.
  - `tests/prod_parity_test.rs` (3 fns / 1 type): EVERY valid example x both backends (monitors ON) driven with identical seeded LCG scripts (seed 43210, 200 steps) -> byte-identical stdout + exit code 0, with the unexpected-event path guaranteed exercised somewhere in the suite.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written and the stray blank line was removed (the new rows are plain-text table cells, no backticked paths or mermaid, so they do not trip the gate's link/mermaid checks) |

## Found problems/issues/mismatches

- PLAN CORRECTION 24a: the cluster notes' "41 fns / 14 types" for backend_parity_test.rs and "8 fns" for wide_ops_parity_test.rs were grep counts inflated by template phantoms (the class corrections 7a/8a/20a/21a/23a established). backend_parity_test.rs: ten of the fourteen type grep hits are RFS spec enums (`enum State` / `enum Color`) inside the LIV_*_SPEC and JSON_GOLDEN_SPEC raw strings - only McOutcome, McExact, Pinned and the nested-in-fn-body JsonCase are real. wide_ops_parity_test.rs: the eighth fn grep hit is `fn main()` on a backslash-continuation line of rust_driver_source's format! string template - unlike smt_emit.rs (correction 6a) it DOES start a line, so the naive counter sees it. Authoritative: 41/4 + 7/0 + 3/1 = 51 fns / 5 types (was ~52/~18).
- Pre-existing defect fixed while appending: a stray blank line between the last task-20 row (`tests/common/mc_json.rs::zero_peak_rss`) and the first task-21 row (`tests/frontend_test.rs::tmp_root`) split the All functions table into two GFM fragments (introduced by task 21's append; docs_md_test did not catch it because comrak parses both fragments as valid tables). Removed so all rows - including this task's - form one contiguous table.

## Other important findings/information

- The nested type JsonCase (declared inside the mc_json_five_status_reason_cases test fn body) gets the `<file>::<parent_fn>::<Type>` identifier `tests/backend_parity_test.rs::mc_json_five_status_reason_cases::JsonCase` per the preamble's nested-type convention - the first such type row in this document.
- All 56 rows have blank Component/Functionality (task 32) and Documented-in (none of the three files carries a `//! Docs:` line; all three DO have docs/llm/TASK_MAP.md suite entries, noted per the preamble's rule rather than repeated row by row - same practice as tasks 21-23).
