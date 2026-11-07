# Phase 11, Task 12 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_12.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | documentation-only task - no source added |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | no source touched |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TEST_STRUCTURE.md` | changed | actualized: the three suite renames rewritten across row identifiers + Depends (33 refs), plus the 1794 stale pre-refactor source-path references the audit's zero-diff criterion forces (inline-test rows re-keyed to post-move host paths, LCG/ident Depends re-pointed); "Proposed layout" -> "Layout" (mermaid verified against the actual tree - new names already drawn by design, no diagram change needed), file-mapping rows + bare Components-table mentions converted |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | task 12 entry appended |

## Changed functionality
- None - documentation-only change; the inventory now describes the ACTUAL
  test tree instead of proposing it.

## Removed functionality
- None.

## New functionality
- None (behavioral). The document's role changed from proposal to living
  inventory of the test tree.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no committed test changes; the test-side string-masked grep audit ran as a session-local script, not a repo test |

## Tests run status

| Name | Status |
|---|---|
| `docs_md_test` (5) | OK |
| `dev_docs_test` (9) | OK |
| test-side string-masked grep audit (session script) | EXACT - 942 fns / 34 types, zero per-file diffs against the refactored test tree; pre-check CLEAN pairwise before any edit (whole-file extraction for tests/**, crates/*/tests/**, fuzz/fuzz_targets/** incl. the four `fuzz_target!` macro entries; cfg(test)-module-only for host files) |
| old-name grep (all three old suite names + all 42 mapping entries' old sides) | ZERO hits in the document |

## Found problems/issues/mismatches
- Scope finding at start: the document carried not just the three old suite
  names (25 hits) but also 1794 stale pre-refactor source-path references -
  inline-test row identifiers for the 26 host files and Depends cells naming
  non-test source identifiers. The audit's zero-per-file-diff criterion forced
  those rewrites too, so the full task-11 mapping was applied here as well.
- Two test-side extractor conventions surfaced during pre-check validation and
  were encoded: plain inline mods in test files contribute their name as an
  identifier segment (`nom_smoke::ident_start`), and the test side never used
  the Type(Trait) marked form (MockFsm's ProtocolFsm impl rows are plain).

## Other important findings/information
- This status file was BACKFILLED: the task executed and committed (fd0a876)
  without its `PHASE_11_TASK_12_STATUS.md` - a PROTOCOL.md section 3 step m gap.
  Content reconstructed from the commit record, the TASKS.md entry of
  2026-11-06 21:35, and the task file's exit criteria (all met at commit time).
- SRC_STRUCTURE.md's audit was re-run EXACT after the shared extractor's
  mod-scope change (a no-op there, as predicted) - both inventories are
  audit-exact against the refactored tree.
