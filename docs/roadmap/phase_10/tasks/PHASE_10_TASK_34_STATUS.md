# Phase 10, Task 34 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_34.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | Two function rows appended to the All functions table (after the fuzz-target rows): crates/examples/async_handler/src/lib.rs::tests::arq_session_completes_over_the_stream and crates/examples/sync_handler/src/lib.rs::tests::arq_session_completes_over_channels - each with Depends on its file's run + ArqReport (both exist in SRC_STRUCTURE.md), Component = Inline unit tests, Example crates, Functionality = behavior pinning, one-line description from the test's own doc comment, Documented-in blank per the pinned inline-row rule. Bookkeeping updated to keep the document exact against a fresh grep: preamble host-file count 25 -> 26 (the 23 under src/ + crates/rfs-build/src/lib.rs + the two example-crate lib.rs files), the Inline unit tests component row (26 host files, 180 + 115 + 2 rows), the behavior-pinning axis row (297 inline rows), and task 33's file-mapping intro note (the two example-crate hosts added to the covered-hosts list). |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The test inventory is now COMPLETE against a fresh string-masked grep: 942 function rows (641 in the 39 test files + 297 inline test-module fns across the 26 host files + 4 fuzz_target! macro entries) and 34 type rows, zero per-file deltas on both documents.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows + bookkeeping were written |

## Found problems/issues/mismatches

- The gap itself is plan correction 34a (recorded in PHASE_10_PLAN.md): the two example-crate inline test fns (Phase 5 task 11's CI records) were assigned to no enumeration task - SRC_STRUCTURE's boundary covers the host files but correction 2a moved inline test fns to the test side, and tasks 29-30's file lists never included the example crates (the same gap class as correction 30c's rfs-build module).
- The preamble's pre-existing "25 such host files under src/" figure was a raw-grep count that included src/backends/rust_test.rs, whose nine #[test] hits are template phantoms inside its generated-test-crate string (correction 30a already established rust_test holds no real test module). The masked true count is 23 hosts under src/; the updated preamble states the exact 26 with the breakdown.

## Other important findings/information

- Close-out fresh-grep audit (step q) results, both documents exact: SRC_STRUCTURE.md 938 fns / 197 types (the two nested types ref_mc.rs::run::St and test_cmd.rs::Guard are real and inventoried); TEST_STRUCTURE.md 942 fns / 34 types after this task. The audit required a character-level masker (comments before strings, raw-string hash counting, char-literal lookahead) - regex-only masking mis-paired quotes inside comments and under/over-counted by dozens.
